/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `autoreplace_cmd.cpp`: deals with autoreplace execution but not the setup. Its two
//! commands, `CmdAutoreplaceVehicle` and `CmdSetAutoReplace`, and every body behind them but
//! `CheckCargoCapacity` and `TransferCargo`'s loops, which `cargo_storage.rs` holds.
//!
//! Lifetime contract: the tables this file reaches through [`Services`] live for the process,
//! and every vehicle a body holds is a live record of the serialized game state, as the
//! original's `Vehicle *` is. Each `CommandCost` is a local of the body that declares it,
//! constructed and destroyed around its use, and the saved random seeds are a local of
//! `cmd_autoreplace_vehicle`. No borrow of a record lives across a command.

use super::{
    EngineRenewRule, add_engine_replacement, add_profit_last_year, cmd_add_vehicle_group,
    engine_replacement, group_get_if_valid, remove_engine_replacement, update_autoreplace,
};
use crate::bitmath_func::{has_at_most_one_bit, has_bit64};
use crate::cargo_type::{CargoType, is_valid_cargo_type};
use crate::command_func::cmd_error;
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company::CompanyRef;
use crate::company_type::{CompanyID, OpenTTDCompany};
use crate::economy_type::ExpensesType;
use crate::engine_capabilities::{
    can_carry_cargo, engine_is_valid_id, get_articulated_refit_masks,
    get_cargo_types_of_articulated_parts, get_cargo_types_of_articulated_vehicle,
    get_union_of_articulated_refit_masks,
};
use crate::engine_type::{AIR_CTOL, EngineID, EngineMiscFlag, EngineMiscFlags, RailVehicleTypes};
use crate::group::{GroupFlag, is_all_group_id, is_default_group_id};
use crate::group_type::GroupID;
use crate::network_type::ClientID;
use crate::news_type::AdviceType;
use crate::order_type::{CloneOptions, VehicleOrderID};
use crate::script_event_types::ScriptEvent;
use crate::services::{CommandCostServices, CommandServices, Cost};
use crate::strings_type::StringParameterData::Integer;
use crate::table::strings::{
    STR_ERROR_AUTOREPLACE_INCOMPATIBLE_CARGO, STR_ERROR_AUTOREPLACE_INCOMPATIBLE_REFIT,
    STR_ERROR_AUTOREPLACE_NOTHING_TO_DO, STR_ERROR_RAIL_VEHICLE_NOT_AVAILABLE,
    STR_ERROR_TRAIN_TOO_LONG, STR_ERROR_TRAIN_TOO_LONG_AFTER_REPLACEMENT,
    STR_NEWS_VEHICLE_AUTORENEW_FAILED,
};
use crate::tile_type::{TILE_SIZE, TileIndex};
use crate::vehicle::{EngineRef, VehicleRef};
use crate::vehicle_base::VehState;
use crate::vehicle_transactions::{TransactionServices, build, rail, refit};
use crate::vehicle_type::{VehicleID, VehicleType};
use crate::vehiclelist::{VehicleListIdentifier, VehicleListType};
use crate::window_type::{WindowClass, WindowNumber};

/// The tables of the autoreplace bodies and the native bodies they call that are not ported.
#[repr(C)]
pub struct Services {
    // Tables.
    /// The vehicle build table, which reaches the refit, common and base transaction tables
    /// and the group table; resolved at the call, since its initialiser reaches this table.
    pub(crate) build: extern "C" fn() -> &'static build::Services,
    /// The engine capabilities table.
    pub(crate) capabilities: &'static crate::engine_capabilities::Services,
    /// The command table.
    pub(crate) commands: *const CommandServices,
    /// The base vehicle table.
    pub(crate) vehicles: &'static crate::vehicle::VehicleServices,

    // Unported native bodies.
    /// `ChangeVehicleNews(from_index, to_index)` (`news_gui.cpp`).
    pub(crate) change_vehicle_news: extern "C" fn(u32, u32),
    /// `ChangeVehicleViewWindow(from_index, to_index)` (`vehicle_gui.cpp`).
    pub(crate) change_vehicle_view_window: extern "C" fn(u32, u32),
    /// `ChangeVehicleViewports(from_index, to_index)` (`window.cpp`).
    pub(crate) change_vehicle_viewports: extern "C" fn(u32, u32),
    /// `this->CopyVehicleConfigAndStatistics(src)`, which writes members the C++ shell stores
    /// and calls `ReleaseUnitNumber`.
    pub(crate) copy_vehicle_config_and_statistics: extern "C" fn(VehicleRef, VehicleRef),
    /// `GetBestFittingSubType(v_from, v_for, dest_cargo_type)` (`vehicle_gui.cpp`).
    pub(crate) get_best_fitting_sub_type: extern "C" fn(VehicleRef, VehicleRef, CargoType) -> u8,
    /// `InvalidateAutoreplaceWindow(e, id_g)` (`autoreplace_gui.cpp`).
    pub(crate) invalidate_autoreplace_window: extern "C" fn(u16, u16),
}

impl Services {
    /// The vehicle build table.
    fn build(&self) -> &'static build::Services {
        (self.build)()
    }
    /// The command table.
    #[allow(
        unsafe_code,
        reason = "the table pointer names the process-lifetime command table"
    )]
    fn commands(&self) -> &CommandServices {
        // SAFETY: The module's table contract.
        unsafe { &*self.commands }
    }
    /// The company table, which names the company identity and engine tables.
    #[allow(
        unsafe_code,
        reason = "the getter returns the process-lifetime company table"
    )]
    fn company(&self) -> &'static crate::company::Services {
        // SAFETY: The module's table contract.
        unsafe { &*(self.commands().company)() }
    }
    /// The engine table.
    #[allow(
        unsafe_code,
        reason = "the getter returns the process-lifetime engine table"
    )]
    fn engines(&self) -> &'static crate::engine::Services {
        // SAFETY: The module's table contract.
        unsafe { &*(self.company().engines)() }
    }
    /// The vehicle resolver table, for `Vehicle::index` and `cargo_type`.
    fn newgrf(&self) -> &'static crate::newgrf_vehicle::Services {
        self.vehicles.newgrf()
    }
    /// `_settings_game.vehicle.max_train_length`, from the settings catalog.
    #[allow(unsafe_code, reason = "the catalog addresses the process-wide setting")]
    pub(crate) fn max_train_length(&self) -> u8 {
        // SAFETY: The catalog addresses the process-wide setting; one field-sized load.
        unsafe {
            self.build()
                .refit
                .common
                .base
                .shared
                .settings()
                .max_train_length
                .read()
        }
    }
    /// `IsLocalCompany`, through the command layer's company and finance owners.
    pub(crate) fn is_local_company(&self) -> bool {
        crate::command::is_local_company(self.commands())
    }
}

/// A `CommandCost` temporary: constructed before `body` and destroyed after it.
#[allow(unsafe_code, reason = "the native result lives on this stack frame")]
pub(crate) fn with_cost<T>(
    cost: &CommandCostServices,
    body: impl for<'a> FnOnce(Cost<'a>) -> T,
) -> T {
    // SAFETY: No body of this file keeps the temporary or its address past its return.
    unsafe { crate::command::with_cost(cost, body) }
}

/// `Command<cmd>::Do(flags, ...)` around the Rust body `body` of `cmd`, its result in `out`.
pub(crate) fn command(
    base: &TransactionServices,
    out: Cost<'_>,
    tile: TileIndex,
    flags: DoCommandFlags,
    mut body: impl FnMut(DoCommandFlags),
) {
    crate::command::nested(base.commands, base.shared, out, tile.0, flags.0, |flags| {
        body(DoCommandFlags(flags));
    });
}

/// `Train::From(v)`'s type assertion, for a vehicle the original takes as a `Train *`.
fn train_from(v: VehicleRef) -> VehicleRef {
    crate::train::Handle::assert_from(|| v.vehicle_type());
    v
}

/// `Train::IsRearDualheaded`.
fn is_rear_dualheaded(v: VehicleRef) -> bool {
    crate::ground_vehicle::is_rear_dualheaded(v.subtype())
}

/// `RailVehInfo(engine_type)->railveh_type == RAILVEH_WAGON`.
fn is_wagon(w: &Services, engine_type: EngineID) -> bool {
    EngineRef::of((w.capabilities.engine)(engine_type.0)).rail_kind()
        == RailVehicleTypes::RAILVEH_WAGON
}

/// `Train::From(v)->gcache.cached_total_length`.
#[allow(
    unsafe_code,
    reason = "a field-sized load of the live train's ground cache"
)]
fn cached_total_length(v: VehicleRef) -> u16 {
    crate::raw_field::read_field!(train_from(v).ground_cache(), cached_total_length)
}

/// `EnginesHaveCargoInCommon`: figure out if two engines got at least one type of cargo in
/// common (refitting if needed). True if they can both carry the same type of cargo (or at
/// least one of them got no capacity at all).
fn engines_have_cargo_in_common(w: &Services, engine_a: EngineID, engine_b: EngineID) -> bool {
    let available_cargoes_a = get_union_of_articulated_refit_masks(w.capabilities, engine_a, true);
    let available_cargoes_b = get_union_of_articulated_refit_masks(w.capabilities, engine_b, true);
    available_cargoes_a.0 == 0
        || available_cargoes_b.0 == 0
        || (available_cargoes_a.0 & available_cargoes_b.0) != 0
}

/// `CheckAutoreplaceValidity`: checks some basic properties whether autoreplace is allowed
/// from engine `from` to engine `to` for `company`.
pub(crate) fn check_autoreplace_validity(
    w: &Services,
    from: EngineID,
    to: EngineID,
    company: CompanyID,
) -> bool {
    let capabilities = w.capabilities;
    crate::services_assert::assert_with(|| {
        engine_is_valid_id(capabilities, from) && engine_is_valid_id(capabilities, to)
    });

    let e_from = EngineRef::of((capabilities.engine)(from.0));
    let e_to = EngineRef::of((capabilities.engine)(to.0));
    let r#type = e_from.kind();

    /* check that the new vehicle type is available to the company and its type is the same as the original one */
    if !crate::engine::is_engine_buildable(w.engines(), to, r#type, company) {
        return false;
    }

    let shared = capabilities.shared;
    match r#type {
        VehicleType::VEH_TRAIN => {
            /* make sure the railtypes are compatible */
            if e_from.compatible_railtypes(shared) & e_to.compatible_railtypes(shared) == 0 {
                return false;
            }

            /* make sure we do not replace wagons with engines or vice versa */
            if (e_from.rail_kind() == RailVehicleTypes::RAILVEH_WAGON)
                != (e_to.rail_kind() == RailVehicleTypes::RAILVEH_WAGON)
            {
                return false;
            }
        }

        VehicleType::VEH_ROAD => {
            /* make sure the roadtypes are compatible */
            if e_from.powered_roadtypes(shared) & e_to.powered_roadtypes(shared) == 0 {
                return false;
            }

            /* make sure that we do not replace a tram with a normal road vehicles or vice versa */
            if EngineMiscFlags(e_from.misc()).test(EngineMiscFlag::RoadIsTram)
                != EngineMiscFlags(e_to.misc()).test(EngineMiscFlag::RoadIsTram)
            {
                return false;
            }
        }

        /* make sure that we do not replace a plane with a helicopter or vice versa */
        VehicleType::VEH_AIRCRAFT
            if (e_from.aircraft_subtype() & AIR_CTOL) != (e_to.aircraft_subtype() & AIR_CTOL) =>
        {
            return false;
        }

        _ => {}
    }

    /* the engines needs to be able to carry the same cargo */
    engines_have_cargo_in_common(w, from, to)
}

/// `TransferCargo`: transfer cargo from a single (articulated) old vehicle `old_veh` to the
/// new vehicle chain headed by `new_head`; `part_of_chain` is whether the vehicle is part of
/// a train. Both consists are neither loading nor unloading, and carry no reserved cargo nor
/// cargo to be unloaded or transferred. `cargo_storage::transfer_cargo` holds the loops.
#[allow(unsafe_code, reason = "the cargo lists of two live consists")]
fn transfer_cargo(w: &Services, old_veh: VehicleRef, new_head: VehicleRef, part_of_chain: bool) {
    // SAFETY: The module's record contract; the original's preconditions on both consists.
    let changed = unsafe {
        crate::cargo_storage::transfer_cargo(old_veh, new_head, part_of_chain, w.newgrf())
    };

    /* Update train weight etc., the old vehicle will be sold anyway */
    if changed {
        let refit = w.build().refit;
        crate::train::consist_changed(
            refit.train,
            refit.common.base.shared,
            crate::train::Handle::from_vehicle(new_head),
            crate::train::CCF_LOADUNLOAD,
        );
    }
}

/// `VerifyAutoreplaceRefitForOrders`: tests whether refit orders that applied to the vehicle
/// `v` to be replaced will also apply to the new vehicle type `engine_type`. True iff all
/// refit orders stay valid.
fn verify_autoreplace_refit_for_orders(w: &Services, v: VehicleRef, engine_type: EngineID) -> bool {
    let union_refit_mask_a =
        get_union_of_articulated_refit_masks(w.capabilities, v.engine_type(), false);
    let union_refit_mask_b =
        get_union_of_articulated_refit_masks(w.capabilities, engine_type, false);

    let u = if v.vehicle_type() == VehicleType::VEH_TRAIN {
        v.first()
    } else {
        v
    };
    let Some(orders) = crate::orders::orders(u, w.vehicles.orders()) else {
        return true;
    };
    for index in 0..orders.len() {
        let o = orders.order(index).view();
        if !o.is_refit() || o.is_auto_refit() {
            continue;
        }
        let cargo_type = o.refit_cargo();

        if !has_bit64(union_refit_mask_a.0, cargo_type.0) {
            continue;
        }
        if !has_bit64(union_refit_mask_b.0, cargo_type.0) {
            return false;
        }
    }

    true
}

/// `GetIncompatibleRefitOrderIdForAutoreplace`: gets the index of the first refit order of
/// the vehicle `v` to be replaced that is incompatible with the requested engine type
/// `engine_type`, or -1 if none were found.
#[allow(clippy::many_single_char_names, reason = "the original's names")]
fn get_incompatible_refit_order_id_for_autoreplace(
    w: &Services,
    v: VehicleRef,
    engine_type: EngineID,
) -> i32 {
    let union_refit_mask = get_union_of_articulated_refit_masks(w.capabilities, engine_type, false);

    let u = if v.vehicle_type() == VehicleType::VEH_TRAIN {
        v.first()
    } else {
        v
    };

    let Some(orders) = crate::orders::order_list(u, w.vehicles.orders()) else {
        return -1;
    };
    let mut i = VehicleOrderID(0);
    while i < orders.get_num_orders() {
        // `GetOrderAt(i)` below `GetNumOrders()`: the stored order `i`.
        let o = orders.order(usize::from(i.0)).view();
        i = VehicleOrderID(i.0 + 1);
        if !o.is_refit() {
            continue;
        }
        if !has_bit64(union_refit_mask.0, o.refit_cargo().0) {
            return i32::from(i.0 - 1);
        }
    }

    -1
}

/// `GetNewCargoTypeForReplace`: find what type of cargo to refit to when autoreplacing the
/// original vehicle `v` with the engine `engine_type`; `part_of_chain` is whether the vehicle
/// is part of a train. `CARGO_NO_REFIT` is returned if no refit is needed, `INVALID_CARGO`
/// when both old and new vehicle got cargo capacity and refitting the new one to the old
/// one's cargo type isn't possible.
fn get_new_cargo_type_for_replace(
    w: &Services,
    v: VehicleRef,
    engine_type: EngineID,
    part_of_chain: bool,
) -> CargoType {
    let (union_mask, available_cargo_types) =
        get_articulated_refit_masks(w.capabilities, engine_type, true);

    if union_mask.0 == 0 {
        return CargoType::CARGO_NO_REFIT; // Don't try to refit an engine with no cargo capacity
    }

    let mut cargo_type = CargoType::INVALID_CARGO;
    let cargo_mask =
        get_cargo_types_of_articulated_vehicle(w.capabilities, v, Some(&mut cargo_type));
    if !has_at_most_one_bit(cargo_mask.0) {
        let new_engine_default_cargoes =
            get_cargo_types_of_articulated_parts(w.capabilities, engine_type);
        if (cargo_mask.0 & new_engine_default_cargoes.0) == cargo_mask.0 {
            return CargoType::CARGO_NO_REFIT; // engine_type is already a mixed cargo type which matches the incoming vehicle by default, no refit required
        }

        return CargoType::INVALID_CARGO; // We cannot refit to mixed cargoes in an automated way
    }

    if !is_valid_cargo_type(cargo_type.0) {
        if v.vehicle_type() != VehicleType::VEH_TRAIN {
            return CargoType::CARGO_NO_REFIT; // If the vehicle does not carry anything at all, every replacement is fine.
        }

        if !part_of_chain {
            return CargoType::CARGO_NO_REFIT;
        }

        /* the old engine didn't have cargo capacity, but the new one does
         * now we will figure out what cargo the train is carrying and refit to fit this */

        let newgrf = w.newgrf();
        let mut part = Some(v.first());
        while let Some(v) = part {
            if can_carry_cargo(w.capabilities, v.engine_type()) {
                /* Now we found a cargo type being carried on the train and we will see if it is possible to carry to this one */
                let cargo_type = v.cargo_type(newgrf);
                if has_bit64(available_cargo_types.0, cargo_type.0) {
                    return cargo_type;
                }
            }
            part = v.next();
        }

        return CargoType::CARGO_NO_REFIT; // We failed to find a cargo type on the old vehicle and we will not refit the new one
    }

    if !has_bit64(available_cargo_types.0, cargo_type.0) {
        return CargoType::INVALID_CARGO; // We can't refit the vehicle to carry the cargo we want
    }

    if part_of_chain && !verify_autoreplace_refit_for_orders(w, v, engine_type) {
        return CargoType::INVALID_CARGO; // Some refit orders lose their effect
    }

    cargo_type
}

/// `GetNewEngineType`: get the `EngineID` of the replacement for the vehicle `v`, whose owner
/// `c` is forwarded because it's faster than refinding it; `always_replace` replaces even if
/// not old. Returns `EngineID::INVALID` if no replacement is found, and writes an error to
/// `cost` if the engine to build is not available.
#[allow(unsafe_code, reason = "the company's live renewal list slot")]
fn get_new_engine_type(
    w: &Services,
    cost: Cost<'_>,
    v: VehicleRef,
    c: *const OpenTTDCompany,
    always_replace: bool,
) -> EngineID {
    crate::services_assert::assert_with(|| {
        v.vehicle_type() != VehicleType::VEH_TRAIN || !v.is_articulated_part()
    });

    let mut e = EngineID::INVALID;

    if v.vehicle_type() == VehicleType::VEH_TRAIN && is_rear_dualheaded(train_from(v)) {
        /* we build the rear ends of multiheaded trains with the front ones */
        cost.zero();
        return e;
    }

    let groups = w.build().refit.common.base.groups;
    let company = w.company();
    let mut replace_when_old = false;
    // `EngineReplacementForCompany(c, ...)`: the replacement in `c->engine_renew_list`.
    // SAFETY: The company's live renewal list slot; one pointer-sized load.
    let engine_renew_list = unsafe { (company.identity.renewal_head)(c).read() };
    e = engine_replacement(
        groups,
        engine_renew_list,
        v.engine_type(),
        v.group_id(groups),
        Some(&mut replace_when_old),
    );
    let needs_autorenewing = |use_renew_setting| {
        crate::vehicle::maintenance::needs_autorenewing_entry::<false>(
            v,
            w.vehicles,
            c,
            use_renew_setting,
        )
    };
    if !always_replace && replace_when_old && !needs_autorenewing(false) {
        e = EngineID::INVALID;
    }

    /* Autoreplace, if engine is available */
    let engines = w.engines();
    if e != EngineID::INVALID
        && crate::engine::is_engine_buildable(
            engines,
            e,
            v.vehicle_type(),
            groups.current_company(),
        )
    {
        cost.zero();
        return e;
    }

    /* Autorenew if needed */
    if needs_autorenewing(true) {
        e = v.engine_type();
    }

    /* Nothing to do or all is fine? */
    if e == EngineID::INVALID
        || crate::engine::is_engine_buildable(
            engines,
            e,
            v.vehicle_type(),
            groups.current_company(),
        )
    {
        cost.zero();
        return e;
    }

    /* The engine we need is not available. Report error to user */
    cost.error(STR_ERROR_RAIL_VEHICLE_NOT_AVAILABLE.0 + u32::from(v.vehicle_type().0));
    e
}

/// `BuildReplacementVehicle`: builds and refits a replacement vehicle for the single
/// (articulated/multiheaded) vehicle `old_veh`, writing the cost or error to `cost`, and
/// returns the newly built and refitted vehicle; `part_of_chain` is whether the vehicle is
/// part of a train, `flags` the calling command flags. Important: the old vehicle is still in
/// the original vehicle chain (used for determining the cargo when the old vehicle did not
/// carry anything, but the new one does).
#[allow(unsafe_code, reason = "the typed build, refit and probability bodies")]
#[allow(
    clippy::too_many_lines,
    reason = "the original body, with its news and its three commands"
)]
fn build_replacement_vehicle(
    w: &Services,
    cost: Cost<'_>,
    old_veh: VehicleRef,
    part_of_chain: bool,
    flags: DoCommandFlags,
) -> Option<VehicleRef> {
    /* Shall the vehicle be replaced? */
    let b = w.build();
    let base = b.refit.common.base;
    let groups = base.groups;
    let c = (w.company().identity.company)(groups.current_company());
    let e = get_new_engine_type(w, cost, old_veh, c, true);
    if !cost.ok() {
        return None;
    }
    if e == EngineID::INVALID {
        cost.zero();
        return None; // neither autoreplace is set, nor autorenew is triggered
    }

    /* Does it need to be refitted */
    let refit_cargo = get_new_cargo_type_for_replace(w, old_veh, e, part_of_chain);
    let newgrf = w.newgrf();
    if !is_valid_cargo_type(refit_cargo.0) {
        if !w.is_local_company() || !flags.test(DoCommandFlag::Execute) {
            cost.zero();
            return None;
        }

        let old_veh_id = if old_veh.vehicle_type() == VehicleType::VEH_TRAIN {
            train_from(old_veh).first().index(newgrf)
        } else {
            old_veh.index(newgrf)
        };

        let order_id = get_incompatible_refit_order_id_for_autoreplace(w, old_veh, e);
        let headline = if order_id == -1 {
            /* Current cargo is incompatible with the new vehicle. */
            crate::strings_func::get_encoded_string(
                STR_NEWS_VEHICLE_AUTORENEW_FAILED,
                &[
                    Integer(u64::from(old_veh_id.0)),
                    Integer(u64::from(STR_ERROR_AUTOREPLACE_INCOMPATIBLE_CARGO.0)),
                    Integer(u64::from((base.shared.cargo_name)(
                        old_veh.cargo_type(newgrf).0,
                    ))),
                ],
            )
        } else {
            /* Orders contained a refit order that is incompatible with the new vehicle. */
            crate::strings_func::get_encoded_string(
                STR_NEWS_VEHICLE_AUTORENEW_FAILED,
                &[
                    Integer(u64::from(old_veh_id.0)),
                    Integer(u64::from(STR_ERROR_AUTOREPLACE_INCOMPATIBLE_REFIT.0)),
                    Integer(u64::from(order_id.unsigned_abs() + 1)), // 1-based indexing for display
                ],
            )
        };

        crate::news_func::add_vehicle_advice_news_item(
            AdviceType::AutorenewFailed,
            &headline,
            old_veh_id,
        );
        cost.zero();
        return None;
    }

    /* Build the new vehicle */
    let tile = old_veh.tile();
    let result = with_cost(base.cost, |temporary| {
        with_cost(base.cost, |refit_temporary| {
            let mut result = build::Result::default();
            command(
                base,
                cost,
                tile,
                DoCommandFlags::from_element(DoCommandFlag::Execute)
                    .set(DoCommandFlag::AutoReplace),
                |flags| {
                    // SAFETY: The module's table contract; three distinct live results.
                    result = unsafe {
                        build::cmd_build_vehicle(
                            b,
                            cost.0,
                            temporary.0,
                            refit_temporary.0,
                            flags,
                            build::CmdBuildVehicle {
                                tile,
                                eid: e,
                                use_free_vehicles: true,
                                cargo: CargoType::INVALID_CARGO,
                                client_id: ClientID::INVALID_CLIENT_ID,
                            },
                        )
                    };
                },
            );
            result
        })
    });
    if !cost.ok() {
        return None;
    }

    // `Vehicle::Get(new_veh_id)` of the vehicle the command built: its slot holds it, as the
    // original's dereference of the result requires.
    let pool = groups.vehicles.orders().scheduler().pool();
    let Some(new_veh) = crate::vehicle_base::get_asserted(&pool, VehicleID(result.id)) else {
        crate::services_assert::not_reached()
    };

    /* Refit the vehicle if needed */
    if refit_cargo != CargoType::CARGO_NO_REFIT {
        let subtype = (w.get_best_fitting_sub_type)(old_veh, new_veh, refit_cargo);

        let veh_id = new_veh.index(newgrf);
        with_cost(base.cost, |ret| {
            with_cost(base.cost, |temporary| {
                command(
                    base,
                    ret,
                    TileIndex(0),
                    DoCommandFlags::from_element(DoCommandFlag::Execute),
                    |flags| {
                        // SAFETY: The module's table contract; two distinct live results.
                        unsafe {
                            refit::cmd_refit_vehicle(
                                b.refit,
                                ret.0,
                                temporary.0,
                                flags,
                                refit::CmdRefitVehicle {
                                    veh_id,
                                    new_cargo_type: refit_cargo,
                                    new_subtype: subtype,
                                    auto_refit: false,
                                    only_this: false,
                                    num_vehicles: 0,
                                },
                            );
                        }
                    },
                );
            });
            cost.add(ret);
        });
        crate::services_assert::assert_with(|| cost.ok()); // This should be ensured by GetNewCargoTypeForReplace()
    }

    /* Try to reverse the vehicle, but do not care if it fails as the new type might not be reversible */
    if new_veh.vehicle_type() == VehicleType::VEH_TRAIN
        && crate::train::Handle::from_vehicle(old_veh)
            .flags()
            .test(crate::train::VehicleRailFlag::Flipped)
    {
        /* Only copy the reverse state if neither old or new vehicle implements reverse-on-build probability callback. */
        // SAFETY: The module's record contract; the callback reads the live vehicles.
        if !unsafe { crate::newgrf_vehicle::reverse_probability(w.vehicles, old_veh) }
            && !unsafe { crate::newgrf_vehicle::reverse_probability(w.vehicles, new_veh) }
        {
            let veh_id = new_veh.index(newgrf);
            let scheduler = groups.vehicles.orders().scheduler();
            with_cost(base.cost, |ret| {
                command(
                    base,
                    ret,
                    TileIndex(0),
                    DoCommandFlags::from_element(DoCommandFlag::Execute),
                    |flags| {
                        crate::train::cmd_reverse_train_direction(
                            b,
                            ret.0,
                            flags,
                            crate::train::CmdReverseTrainDirection {
                                veh_id,
                                reverse_single_veh: true,
                            },
                            scheduler.train(),
                            scheduler.shared(),
                        );
                    },
                );
            });
        }
    }

    Some(new_veh)
}

/// `DoCmdStartStopVehicle`: issue a start/stop command for the vehicle `v`, its result in
/// `cost`; `evaluate_callback` is whether the start/stop callback shall be evaluated.
#[allow(unsafe_code, reason = "the typed start/stop body")]
fn do_cmd_start_stop_vehicle(w: &Services, cost: Cost<'_>, v: VehicleRef, evaluate_callback: bool) {
    let common = w.build().refit.common;
    let veh_id = v.index(w.newgrf());
    command(
        common.base,
        cost,
        TileIndex(0),
        DoCommandFlags::from_element(DoCommandFlag::Execute).set(DoCommandFlag::AutoReplace),
        |flags| {
            // SAFETY: The module's table contract; the live result.
            unsafe {
                crate::vehicle_transactions::cmd_start_stop_vehicle(
                    common,
                    cost.0,
                    flags,
                    crate::vehicle_transactions::CmdStartStopVehicle {
                        veh_id,
                        evaluate_startstop_cb: evaluate_callback,
                    },
                );
            }
        },
    );
}

/// `CmdMoveVehicle`: issue a train vehicle move command, its result in `cost`. `v` is the
/// vehicle to move, `after` the vehicle to insert `v` after, or `None` to start a new chain;
/// `whole_chain` moves all vehicles following `v` (true), or only `v` (false).
#[allow(unsafe_code, reason = "the typed rail move body")]
fn cmd_move_vehicle(
    w: &Services,
    cost: Cost<'_>,
    v: VehicleRef,
    after: Option<VehicleRef>,
    flags: DoCommandFlags,
    whole_chain: bool,
) {
    let newgrf = w.newgrf();
    let cmd = rail::CmdMoveRailVehicle {
        src_veh: v.index(newgrf),
        dest_veh: after.map_or(VehicleID::INVALID, |after| after.index(newgrf)),
        move_chain: whole_chain,
    };
    // SAFETY: The module's table contract; the live result.
    unsafe {
        rail::nested_move_rail_vehicle(
            w.build(),
            cost.0,
            flags.set(DoCommandFlag::NoCargoCapacityCheck),
            cmd,
        );
    }
}

/// `Command<CMD_SELL_VEHICLE>::Do(flags, v->index, false, false, INVALID_CLIENT_ID)`, its
/// result in `cost`.
#[allow(unsafe_code, reason = "the typed sell body")]
fn sell_vehicle(w: &Services, cost: Cost<'_>, flags: DoCommandFlags, v: VehicleRef) {
    let common = w.build().refit.common;
    let v_id = v.index(w.newgrf());
    command(common.base, cost, TileIndex(0), flags, |flags| {
        // SAFETY: The module's table contract; the live result.
        unsafe {
            crate::vehicle_transactions::cmd_sell_vehicle(
                common,
                cost.0,
                flags,
                crate::vehicle_transactions::CmdSellVehicle {
                    v_id,
                    sell_chain: false,
                    backup_order: false,
                    client_id: ClientID::INVALID_CLIENT_ID,
                },
            );
        }
    });
}

/// `CopyHeadSpecificThings`: copy head specific things from the old front vehicle `old_head`
/// (no wagons attached anymore) to the new head `new_head` of the completely replaced
/// vehicle chain after it was successfully constructed, the cost in `cost`.
fn copy_head_specific_things(
    w: &Services,
    cost: Cost<'_>,
    old_head: VehicleRef,
    new_head: VehicleRef,
    flags: DoCommandFlags,
) {
    let common = w.build().refit.common;
    let base = common.base;
    let groups = base.groups;
    let newgrf = w.newgrf();
    cost.zero();

    /* Share orders */
    if cost.ok() && old_head != new_head {
        let veh_dst = new_head.index(newgrf);
        let veh_src = old_head.index(newgrf);
        with_cost(base.cost, |ret| {
            command(
                base,
                ret,
                TileIndex(0),
                DoCommandFlags::from_element(DoCommandFlag::Execute),
                |flags| {
                    crate::orders::cmd_clone_order(
                        common.orders,
                        ret,
                        flags,
                        crate::orders::CmdCloneOrder {
                            action: CloneOptions::CO_SHARE,
                            veh_dst,
                            veh_src,
                        },
                    );
                },
            );
            cost.add(ret);
        });
    }

    /* Copy group membership */
    if cost.ok() && old_head != new_head {
        let group_id = old_head.group_id(groups);
        let veh_id = new_head.index(newgrf);
        with_cost(base.cost, |ret| {
            command(
                base,
                ret,
                TileIndex(0),
                DoCommandFlags::from_element(DoCommandFlag::Execute),
                |flags| {
                    cmd_add_vehicle_group(
                        groups,
                        ret,
                        flags,
                        crate::fleet::CmdAddVehicleGroup {
                            group_id,
                            veh_id,
                            add_shared: false,
                            vli: VehicleListIdentifier::default(),
                        },
                    );
                },
            );
            cost.add(ret);
        });
    }

    /* Perform start/stop check whether the new vehicle suits newgrf restrictions etc. */
    if cost.ok() {
        /* Start the vehicle, might be denied by certain things */
        crate::services_assert::assert_with(|| new_head.vehstatus().test(VehState::Stopped));
        with_cost(base.cost, |ret| {
            do_cmd_start_stop_vehicle(w, ret, new_head, true);
            cost.add(ret);
        });

        /* Stop the vehicle again, but do not care about evil newgrfs allowing starting but not stopping :p */
        if cost.ok() {
            with_cost(base.cost, |ret| {
                do_cmd_start_stop_vehicle(w, ret, new_head, false);
                cost.add(ret);
            });
        }
    }

    /* Last do those things which do never fail (resp. we do not care about), but which are not undo-able */
    if cost.ok() && old_head != new_head && flags.test(DoCommandFlag::Execute) {
        /* Copy other things which cannot be copied by a command and which shall not stay reset from the build vehicle command */
        (w.copy_vehicle_config_and_statistics)(new_head, old_head);
        add_profit_last_year(groups, new_head);

        /* Switch vehicle windows/news to the new vehicle, so they are not closed/deleted when the old vehicle is sold */
        let from_index = old_head.index(newgrf).0;
        let to_index = new_head.index(newgrf).0;
        (w.change_vehicle_viewports)(from_index, to_index);
        (w.change_vehicle_view_window)(from_index, to_index);
        (w.change_vehicle_news)(from_index, to_index);
    }
}

/// `ReplaceFreeUnit`: replace the single unit `single_unit` of a free wagon chain, the cost
/// in `cost`. `nothing_to_do` is set to false when something was done (only valid when not
/// failed).
fn replace_free_unit(
    w: &Services,
    cost: Cost<'_>,
    single_unit: &mut VehicleRef,
    flags: DoCommandFlags,
    nothing_to_do: &mut bool,
) {
    let old_v = train_from(*single_unit);
    crate::services_assert::assert_with(|| {
        !old_v.is_articulated_part() && !is_rear_dualheaded(old_v)
    });
    let base = w.build().refit.common.base;
    let newgrf = w.newgrf();

    cost.expense_money(ExpensesType::EXPENSES_NEW_VEHICLES, 0);

    /* Build and refit replacement vehicle */
    let new_v = with_cost(base.cost, |ret| {
        let new_v = build_replacement_vehicle(w, ret, old_v, false, flags);
        cost.add(ret);
        new_v
    });

    /* Was a new vehicle constructed? */
    if cost.ok()
        && let Some(new_v) = new_v
    {
        *nothing_to_do = false;

        if flags.test(DoCommandFlag::Execute) {
            /* Move the new vehicle behind the old */
            with_cost(base.cost, |ret| {
                cmd_move_vehicle(
                    w,
                    ret,
                    new_v,
                    Some(old_v),
                    DoCommandFlags::from_element(DoCommandFlag::Execute),
                    false,
                );
            });

            /* Take over cargo
             * Note: We do only transfer cargo from the old to the new vehicle.
             *       I.e. we do not transfer remaining cargo to other vehicles.
             *       Else you would also need to consider moving cargo to other free chains,
             *       or doing the same in ReplaceChain(), which would be quite troublesome.
             */
            transfer_cargo(w, old_v, new_v, false);

            *single_unit = new_v;

            crate::ai::new_event(
                old_v.owner(),
                &ScriptEvent::vehicle_auto_replaced(old_v.index(newgrf), new_v.index(newgrf)),
            );
        }

        /* Sell the old vehicle */
        with_cost(base.cost, |ret| {
            sell_vehicle(w, ret, flags, old_v);
            cost.add(ret);
        });

        /* If we are not in DoCommandFlag::Execute undo everything */
        if !flags.test(DoCommandFlag::Execute) {
            with_cost(base.cost, |ret| {
                sell_vehicle(
                    w,
                    ret,
                    DoCommandFlags::from_element(DoCommandFlag::Execute),
                    new_v,
                );
            });
        }
    }
}

/// `ReplaceChainItem`: struct for recording vehicle chain replacement information.
struct ReplaceChainItem {
    /// Old vehicle to replace.
    old_veh: VehicleRef,
    /// Replacement vehicle, or `None` if no replacement.
    new_veh: Option<VehicleRef>,
    /// Cost of buying and refitting replacement.
    cost: i64,
}

impl ReplaceChainItem {
    /// Get vehicle to use for this position: either the new vehicle, or the old vehicle if
    /// there is no replacement.
    fn get_vehicle(&self) -> VehicleRef {
        self.new_veh.unwrap_or(self.old_veh)
    }
}

/// `ReplaceChain`: replace the whole vehicle chain `chain`, the cost in `cost`.
/// `wagon_removal` removes wagons when the resulting chain occupies more tiles than the old
/// did; `nothing_to_do` is set to false when something was done (only valid when not
/// failed).
///
/// The original nulls `it->old_veh` and `old_head` after selling them under `Execute`; only
/// the branches without `Execute` read them afterwards, so the locals keep their vehicles.
#[allow(
    clippy::too_many_lines,
    reason = "the original body, whose train branch builds, rearranges and undoes the chain"
)]
#[allow(unsafe_code, reason = "the cargo lists of the live new consist")]
fn replace_chain(
    w: &Services,
    cost: Cost<'_>,
    chain: &mut VehicleRef,
    flags: DoCommandFlags,
    wagon_removal: bool,
    nothing_to_do: &mut bool,
) {
    let old_head = *chain;
    crate::services_assert::assert_with(|| old_head.is_primary_vehicle());
    let base = w.build().refit.common.base;
    let newgrf = w.newgrf();
    let execute = DoCommandFlags::from_element(DoCommandFlag::Execute);
    let execute_autoreplace = execute.set(DoCommandFlag::AutoReplace);

    cost.expense_money(ExpensesType::EXPENSES_NEW_VEHICLES, 0);

    if old_head.vehicle_type() == VehicleType::VEH_TRAIN {
        /* Store the length of the old vehicle chain, rounded up to whole tiles */
        #[allow(
            clippy::cast_possible_truncation,
            reason = "the original's `uint16_t` store"
        )]
        let old_total_length =
            (u32::from(cached_total_length(old_head)).div_ceil(TILE_SIZE) * TILE_SIZE) as u16;

        let mut replacements: Vec<ReplaceChainItem> = Vec::new();

        /* Collect vehicles and build replacements
         * Note: The replacement vehicles can only successfully build as long as the old vehicles are still in their chain */
        let mut unit = Some(train_from(old_head));
        while let Some(w_unit) = unit {
            replacements.push(ReplaceChainItem {
                old_veh: w_unit,
                new_veh: None,
                cost: 0,
            });
            let index = replacements.len() - 1;

            let failed = with_cost(base.cost, |ret| {
                let replacement = &mut replacements[index];
                replacement.new_veh =
                    build_replacement_vehicle(w, ret, replacement.old_veh, true, flags);
                replacement.cost = ret.money();
                cost.add(ret);
                !cost.ok()
            });
            if failed {
                break;
            }

            if replacements[index].new_veh.is_some() {
                *nothing_to_do = false;
            }
            unit = w_unit.next_unit();
        }
        let new_head = replacements[0].get_vehicle();

        /* Note: When autoreplace has already failed here, old_vehs[] is not completely initialized. But it is also not needed. */
        if cost.ok() {
            /* Separate the head, so we can start constructing the new chain */
            let second = train_from(old_head).next_unit();
            if let Some(second) = second {
                with_cost(base.cost, |ret| {
                    cmd_move_vehicle(w, ret, second, None, execute_autoreplace, true);
                    cost.add(ret);
                });
            }

            crate::services_assert::assert_with(|| train_from(new_head).next_unit().is_none());

            /* Append engines to the new chain
             * We do this from back to front, so that the head of the temporary vehicle chain does not change all the time.
             * That way we also have less trouble when exceeding the unitnumber limit.
             * OTOH the vehicle attach callback is more expensive this way :s */
            let mut last_engine: Option<VehicleRef> = None; // Shall store the last engine unit after this step
            if cost.ok() {
                for it in replacements.iter().rev() {
                    let append = it.get_vehicle();

                    if is_wagon(w, append.engine_type()) {
                        continue;
                    }

                    if it.new_veh.is_some() {
                        /* Move the old engine to a separate row with DoCommandFlag::AutoReplace. Else
                         * moving the wagon in front may fail later due to unitnumber limit.
                         * (We have to attach wagons without DoCommandFlag::AutoReplace.) */
                        with_cost(base.cost, |ret| {
                            cmd_move_vehicle(w, ret, it.old_veh, None, execute_autoreplace, false);
                        });
                    }

                    if last_engine.is_none() {
                        last_engine = Some(append);
                    }
                    with_cost(base.cost, |ret| {
                        cmd_move_vehicle(w, ret, append, Some(new_head), execute, false);
                        cost.add(ret);
                    });
                    if !cost.ok() {
                        break;
                    }
                }
                if last_engine.is_none() {
                    last_engine = Some(new_head);
                }
            }

            /* When wagon removal is enabled and the new engines without any wagons are already longer than the old, we have to fail */
            if cost.ok() && wagon_removal && cached_total_length(new_head) > old_total_length {
                cost.error(STR_ERROR_TRAIN_TOO_LONG_AFTER_REPLACEMENT.0);
            }

            /* Append/insert wagons into the new vehicle chain
             * We do this from back to front, so we can stop when wagon removal or maximum train length (i.e. from mammoth-train setting) is triggered.
             */
            if cost.ok() {
                for it in replacements.iter().rev() {
                    crate::services_assert::assert_with(|| last_engine.is_some());
                    let append = it.get_vehicle();

                    if is_wagon(w, append.engine_type()) {
                        /* Insert wagon after 'last_engine' */
                        let stop = with_cost(base.cost, |res| {
                            cmd_move_vehicle(w, res, append, last_engine, execute, false);

                            /* When we allow removal of wagons, either the move failing due
                             * to the train becoming too long, or the train becoming longer
                             * would move the vehicle to the empty vehicle chain. */
                            if wagon_removal
                                && if res.ok() {
                                    cached_total_length(new_head) > old_total_length
                                } else {
                                    res.message() == STR_ERROR_TRAIN_TOO_LONG.0
                                }
                            {
                                with_cost(base.cost, |ret| {
                                    cmd_move_vehicle(
                                        w,
                                        ret,
                                        append,
                                        None,
                                        execute_autoreplace,
                                        false,
                                    );
                                });
                                return true;
                            }

                            cost.add(res);
                            !cost.ok()
                        });
                        if stop {
                            break;
                        }
                    } else {
                        /* We have reached 'last_engine', continue with the next engine towards the front */
                        crate::services_assert::assert_with(|| last_engine == Some(append));
                        last_engine = last_engine.and_then(|v| train_from(v).previous_unit());
                    }
                }
            }

            /* Sell superfluous new vehicles that could not be inserted. */
            if cost.ok() && wagon_removal {
                crate::services_assert::assert_with(|| {
                    u32::from(cached_total_length(new_head))
                        <= u32::from(w.max_train_length()) * TILE_SIZE
                });
                for it in replacements.iter_mut().skip(1) {
                    let Some(wagon) = it.new_veh else {
                        continue;
                    };
                    if wagon.first() == new_head {
                        break;
                    }

                    crate::services_assert::assert_with(|| is_wagon(w, wagon.engine_type()));

                    /* Sell wagon */
                    with_cost(base.cost, |ret| {
                        sell_vehicle(w, ret, execute, wagon);
                        crate::services_assert::assert_with(|| ret.ok());
                    });
                    it.new_veh = None;

                    /* Revert the money subtraction when the vehicle was built.
                     * This value is different from the sell value, esp. because of refitting */
                    cost.add_money(it.cost.saturating_neg());
                }
            }

            /* The new vehicle chain is constructed, now take over orders and everything... */
            if cost.ok() {
                with_cost(base.cost, |ret| {
                    copy_head_specific_things(w, ret, old_head, new_head, flags);
                    cost.add(ret);
                });
            }

            if cost.ok() {
                /* Success ! */
                if flags.test(DoCommandFlag::Execute) && new_head != old_head {
                    *chain = new_head;
                    crate::ai::new_event(
                        old_head.owner(),
                        &ScriptEvent::vehicle_auto_replaced(
                            old_head.index(newgrf),
                            new_head.index(newgrf),
                        ),
                    );
                }

                /* Transfer cargo of old vehicles and sell them */
                for it in &replacements {
                    let w_old = it.old_veh;
                    /* Is the vehicle again part of the new chain?
                     * Note: We cannot test 'new_vehs[i] != nullptr' as wagon removal might cause to remove both */
                    if w_old.first() == new_head {
                        continue;
                    }

                    if flags.test(DoCommandFlag::Execute) {
                        transfer_cargo(w, w_old, new_head, true);
                    }

                    /* Sell the vehicle.
                     * Note: This might temporarily construct new trains, so use DoCommandFlag::AutoReplace to prevent
                     *       it from failing due to engine limits. */
                    with_cost(base.cost, |ret| {
                        sell_vehicle(w, ret, flags.set(DoCommandFlag::AutoReplace), w_old);
                        cost.add(ret);
                    });
                }

                if flags.test(DoCommandFlag::Execute) {
                    // SAFETY: The module's record contract; the original's preconditions on the
                    // new consist.
                    unsafe { crate::cargo_storage::check_cargo_capacity(Some(new_head), newgrf) };
                }
            }

            /* If we are not in DoCommandFlag::Execute undo everything, i.e. rearrange old vehicles.
             * We do this from back to front, so that the head of the temporary vehicle chain does not change all the time.
             * Note: The vehicle attach callback is disabled here :) */
            if !flags.test(DoCommandFlag::Execute) {
                /* Separate the head, so we can reattach the old vehicles */
                let second = train_from(old_head).next_unit();
                if let Some(second) = second {
                    with_cost(base.cost, |ret| {
                        cmd_move_vehicle(w, ret, second, None, execute_autoreplace, true);
                    });
                }

                crate::services_assert::assert_with(|| train_from(old_head).next_unit().is_none());

                for it in replacements.iter().rev() {
                    with_cost(base.cost, |ret| {
                        cmd_move_vehicle(
                            w,
                            ret,
                            it.old_veh,
                            Some(old_head),
                            execute_autoreplace,
                            false,
                        );
                        crate::services_assert::assert_with(|| ret.ok());
                    });
                }
            }
        }

        /* Finally undo buying of new vehicles */
        if !flags.test(DoCommandFlag::Execute) {
            for it in replacements.iter_mut().rev() {
                if let Some(new_veh) = it.new_veh {
                    with_cost(base.cost, |ret| sell_vehicle(w, ret, execute, new_veh));
                    it.new_veh = None;
                }
            }
        }
    } else {
        /* Build and refit replacement vehicle */
        let new_head = with_cost(base.cost, |ret| {
            let new_head = build_replacement_vehicle(w, ret, old_head, true, flags);
            cost.add(ret);
            new_head
        });

        /* Was a new vehicle constructed? */
        if cost.ok()
            && let Some(new_head) = new_head
        {
            *nothing_to_do = false;

            /* The new vehicle is constructed, now take over orders and everything... */
            with_cost(base.cost, |ret| {
                copy_head_specific_things(w, ret, old_head, new_head, flags);
                cost.add(ret);
            });

            if cost.ok() {
                /* The new vehicle is constructed, now take over cargo */
                if flags.test(DoCommandFlag::Execute) {
                    transfer_cargo(w, old_head, new_head, true);
                    *chain = new_head;

                    crate::ai::new_event(
                        old_head.owner(),
                        &ScriptEvent::vehicle_auto_replaced(
                            old_head.index(newgrf),
                            new_head.index(newgrf),
                        ),
                    );
                }

                /* Sell the old vehicle */
                with_cost(base.cost, |ret| {
                    sell_vehicle(w, ret, flags, old_head);
                    cost.add(ret);
                });
            }

            /* If we are not in DoCommandFlag::Execute undo everything */
            if !flags.test(DoCommandFlag::Execute) {
                with_cost(base.cost, |ret| sell_vehicle(w, ret, execute, new_head));
            }
        }
    }
}

/// `CmdAutoreplaceVehicle`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdAutoreplaceVehicle {
    /// Index of vehicle.
    pub(crate) veh_id: VehicleID,
}

/// `CmdAutoreplaceVehicle`: autoreplaces a vehicle; trains are replaced as a whole chain,
/// free wagons in depot are replaced on their own. The cost of this operation or an error is
/// written to `cost`.
#[allow(unsafe_code, reason = "the random table's save and restore leaves")]
#[allow(clippy::too_many_lines, reason = "the original command body")]
#[allow(clippy::many_single_char_names, reason = "the original's names")]
pub(crate) fn cmd_autoreplace_vehicle(
    w: &Services,
    cost: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdAutoreplaceVehicle,
) {
    let CmdAutoreplaceVehicle { veh_id } = args;
    let b = w.build();
    let base = b.refit.common.base;
    let groups = base.groups;
    let pool = groups.vehicles.orders().scheduler().pool();
    let Some(mut v) = crate::vehicle_base::get_if_valid(&pool, veh_id) else {
        return cmd_error(cost);
    };

    cost.ownership(w.commands(), v.owner().0, 0);
    if !cost.ok() {
        return;
    }

    if v.vehstatus().test(VehState::Crashed) {
        return cmd_error(cost);
    }

    let mut free_wagon = false;
    if v.vehicle_type() == VehicleType::VEH_TRAIN {
        let t = train_from(v);
        if t.is_articulated_part() || is_rear_dualheaded(t) {
            return cmd_error(cost);
        }
        free_wagon = !t.is_front_engine();
        if free_wagon && t.first().is_front_engine() {
            return cmd_error(cost);
        }
    } else if !v.is_primary_vehicle() {
        return cmd_error(cost);
    }
    let physics = b.refit.train.physics;
    if !crate::vehicle_base::is_chain_in_depot(v, physics) {
        return cmd_error(cost);
    }

    let identity = w.company().identity;
    let c = (identity.company)(groups.current_company());
    let mut wagon_removal = (identity.renew_keep_length)(c);

    if let Some(g) = group_get_if_valid(groups, v.group_id(groups)) {
        wagon_removal = g.flags().test(GroupFlag::ReplaceWagonRemoval);
    }

    /* Test whether any replacement is set, before issuing a whole lot of commands that would end in nothing changed */
    let mut unit = Some(v);
    let mut any_replacements = false;
    while let Some(u) = unit {
        let e = with_cost(base.cost, |ret| {
            let e = get_new_engine_type(w, ret, u, c, false);
            if ret.ok() {
                Some(e)
            } else {
                cost.take(ret);
                None
            }
        });
        let Some(e) = e else {
            return;
        };
        any_replacements |= e != EngineID::INVALID;
        unit = if !free_wagon && u.vehicle_type() == VehicleType::VEH_TRAIN {
            train_from(u).next_unit()
        } else {
            None
        };
    }

    cost.expense_money(ExpensesType::EXPENSES_NEW_VEHICLES, 0);
    let mut nothing_to_do = true;

    if any_replacements {
        let was_stopped = free_wagon || v.vehstatus().test(VehState::Stopped);

        /* Stop the vehicle */
        if !was_stopped {
            with_cost(base.cost, |ret| {
                do_cmd_start_stop_vehicle(w, ret, v, true);
                cost.add(ret);
            });
        }
        if !cost.ok() {
            return;
        }

        crate::services_assert::assert_with(|| {
            free_wagon || crate::vehicle_base::is_stopped_in_depot(v, physics)
        });

        /* We have to construct the new vehicle chain to test whether it is valid.
         * Vehicle construction needs random bits, so we have to save the random seeds
         * to prevent desyncs and to replay newgrf callbacks during DoCommandFlag::Execute */
        // SAFETY: The random table saves both streams without a draw.
        let saved_seeds = unsafe { (b.random.save)() };
        with_cost(base.cost, |ret| {
            if free_wagon {
                replace_free_unit(
                    w,
                    ret,
                    &mut v,
                    flags.reset(DoCommandFlag::Execute),
                    &mut nothing_to_do,
                );
            } else {
                replace_chain(
                    w,
                    ret,
                    &mut v,
                    flags.reset(DoCommandFlag::Execute),
                    wagon_removal,
                    &mut nothing_to_do,
                );
            }
            cost.add(ret);
        });
        // SAFETY: As the save's.
        unsafe {
            (b.random.restore)(saved_seeds);
        }

        if cost.ok() && flags.test(DoCommandFlag::Execute) {
            with_cost(base.cost, |ret| {
                if free_wagon {
                    replace_free_unit(w, ret, &mut v, flags, &mut nothing_to_do);
                } else {
                    replace_chain(w, ret, &mut v, flags, wagon_removal, &mut nothing_to_do);
                }
                crate::services_assert::assert_with(|| ret.ok() && ret.money() == cost.money());
            });
        }

        /* Restart the vehicle */
        if !was_stopped {
            with_cost(base.cost, |ret| {
                do_cmd_start_stop_vehicle(w, ret, v, false);
                cost.add(ret);
            });
        }
    }

    if cost.ok() && nothing_to_do {
        cost.error(STR_ERROR_AUTOREPLACE_NOTHING_TO_DO.0);
    }
}

/// `CmdSetAutoReplace`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdSetAutoReplace {
    /// Engine group.
    pub(crate) id_g: GroupID,
    /// Old engine type.
    pub(crate) old_engine_type: EngineID,
    /// New engine type.
    pub(crate) new_engine_type: EngineID,
    /// Replace when engine gets old?
    pub(crate) when_old: bool,
}

/// `CmdSetAutoReplace`: change engine renewal parameters. The cost of this operation or an
/// error is written to `cost`.
pub(crate) fn cmd_set_auto_replace(
    w: &Services,
    cost: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdSetAutoReplace,
) {
    let CmdSetAutoReplace {
        id_g,
        old_engine_type,
        new_engine_type,
        when_old,
    } = args;
    let groups = w.build().refit.common.base.groups;
    let identity = w.company().identity;
    let current_company = groups.current_company();
    let Some(c) = CompanyRef::get_if_valid(identity, current_company) else {
        return cmd_error(cost);
    };

    let g = group_get_if_valid(groups, id_g);
    if match g {
        Some(g) => g.owner() != current_company,
        None => !is_all_group_id(id_g) && !is_default_group_id(id_g),
    } {
        return cmd_error(cost);
    }
    let capabilities = w.capabilities;
    if !engine_is_valid_id(capabilities, old_engine_type) {
        return cmd_error(cost);
    }
    let old_engine = EngineRef::of((capabilities.engine)(old_engine_type.0));
    if let Some(g) = g
        && g.vehicle_type() != old_engine.kind()
    {
        return cmd_error(cost);
    }

    let engine_renew_list = (identity.renewal_head)(c.company);
    if new_engine_type == EngineID::INVALID {
        remove_engine_replacement(
            groups,
            cost,
            engine_renew_list,
            old_engine_type,
            id_g,
            flags,
        );
    } else {
        if !engine_is_valid_id(capabilities, new_engine_type) {
            return cmd_error(cost);
        }
        if !check_autoreplace_validity(w, old_engine_type, new_engine_type, current_company) {
            return cmd_error(cost);
        }

        add_engine_replacement(
            groups,
            cost,
            engine_renew_list,
            EngineRenewRule {
                old_engine: old_engine_type,
                new_engine: new_engine_type,
                group: id_g,
                replace_when_old: when_old,
            },
            flags,
        );
    }

    if flags.test(DoCommandFlag::Execute) {
        update_autoreplace(groups, current_company);
        if w.is_local_company() {
            crate::window_func::set_window_dirty(
                WindowClass::WC_REPLACE_VEHICLE,
                WindowNumber(i32::from(old_engine.kind().0)),
            );
        }

        let vt = old_engine.kind();
        crate::window_func::set_window_dirty(
            crate::vehicle_gui::get_window_class_for_vehicle_type(vt),
            VehicleListIdentifier {
                r#type: VehicleListType::VL_GROUP_LIST,
                vtype: vt,
                company: current_company,
                index: 0,
            }
            .to_window_number(),
        );
    }
    if flags.test(DoCommandFlag::Execute) && w.is_local_company() {
        (w.invalidate_autoreplace_window)(old_engine_type.0, id_g.0);
    }
}

/// [`check_autoreplace_validity`], for `CheckAutoreplaceValidity`'s C++ callers.
/// # Safety
/// `w` is the live transaction table.
#[allow(unsafe_code, reason = "the C++ facade's entry")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_valid_abi(
    w: *const Services,
    from: u16,
    to: u16,
    company: u8,
) -> u8 {
    u8::from(check_autoreplace_validity(
        unsafe { &*w },
        EngineID(from),
        EngineID(to),
        CompanyID(company),
    ))
}

/// [`cmd_autoreplace_vehicle`], for `CmdAutoreplaceVehicle`'s C++ facade; `cost` is the
/// facade's result.
/// # Safety
/// `w` is the live transaction table and `cost` the caller's live `CommandCost`.
#[allow(unsafe_code, reason = "the C++ facade's entry")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_autoreplace(
    w: *const Services,
    cost: *mut crate::services::CommandCost,
    flags: u16,
    veh_id: u32,
) {
    let w = unsafe { &*w };
    cmd_autoreplace_vehicle(
        w,
        Cost(cost, w.build().refit.common.base.cost),
        DoCommandFlags(flags),
        CmdAutoreplaceVehicle {
            veh_id: VehicleID(veh_id),
        },
    );
}

/// [`cmd_set_auto_replace`], for `CmdSetAutoReplace`'s C++ facade; `cost` is the facade's
/// result.
/// # Safety
/// `w` is the live transaction table and `cost` the caller's live `CommandCost`.
#[allow(unsafe_code, reason = "the C++ facade's entry")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_set_rule(
    w: *const Services,
    cost: *mut crate::services::CommandCost,
    flags: u16,
    id_g: u16,
    old_engine_type: u16,
    new_engine_type: u16,
    when_old: u8,
) {
    let w = unsafe { &*w };
    cmd_set_auto_replace(
        w,
        Cost(cost, w.build().refit.common.base.cost),
        DoCommandFlags(flags),
        CmdSetAutoReplace {
            id_g: GroupID(id_g),
            old_engine_type: EngineID(old_engine_type),
            new_engine_type: EngineID(new_engine_type),
            when_old: when_old != 0,
        },
    );
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDFleetTransactionServices" ["fleet"] => Services {
        build,
        capabilities,
        commands,
        vehicles,
        change_vehicle_news,
        change_vehicle_view_window,
        change_vehicle_viewports,
        copy_vehicle_config_and_statistics,
        get_best_fitting_sub_type,
        invalidate_autoreplace_window,
    };
}

#[cfg(test)]
mod tests;
