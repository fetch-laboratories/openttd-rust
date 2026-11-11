/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Common build transaction, including speculative construction and RNG rollback.
use super::{ERROR, construct, mass, rail, refit};
use crate::cargo_type::CargoType;
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::engine_type::EngineID;
use crate::network_type::ClientID;
use crate::services::{CommandCost, Cost, RandomServices};
use crate::tile_type::TileIndex;
use crate::vehicle::{RailVehicleTypes, VehicleType};
use std::ptr;
#[repr(C)]
pub struct Result {
    pub id: u32,
    pub capacity: refit::Capacity,
}
impl Default for Result {
    fn default() -> Self {
        Self {
            id: crate::fleet::INVALID_VEHICLE,
            capacity: refit::Capacity::default(),
        }
    }
}
#[repr(C)]
pub struct Services {
    pub refit: &'static refit::Services,
    pub mass: &'static mass::Services,
    pub random: &'static RandomServices,
    pub depot_kind: extern "C" fn(u32) -> VehicleType,
    pub free_unit: extern "C" fn(VehicleType) -> u16,
    pub construct: &'static construct::Services,
    pub rail: &'static rail::Services,
    pub ship_tile: extern "C" fn(u32) -> u32,
    pub new_ship: extern "C" fn(u32) -> *mut (),
    pub set_purchase: extern "C" fn(*mut (), u16, i64),
    pub is_engine: extern "C" fn(*mut ()) -> bool,
    pub presentation: extern "C" fn(*mut (), VehicleType),
    pub use_unit: extern "C" fn(*mut ()),
    pub too_many: u32,
    pub unavailable: u32,
}

/// `CmdBuildVehicle`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdBuildVehicle {
    /// Tile of depot where the vehicle is built.
    pub(crate) tile: TileIndex,
    /// Vehicle type being built.
    pub(crate) eid: EngineID,
    /// Use free vehicles when building the vehicle.
    pub(crate) use_free_vehicles: bool,
    /// Refit cargo type.
    pub(crate) cargo: CargoType,
    /// User.
    pub(crate) client_id: ClientID,
}
/// Raw adapter: `CmdBuildVehicle` over base values, for `fleet/transactions.rs`
/// until its pass.
///
/// # Safety
/// As `cmd_build_vehicle`.
pub(crate) unsafe fn build(
    s: &'static Services,
    out: *mut CommandCost,
    temporary: *mut CommandCost,
    refit_temporary: *mut CommandCost,
    flags: u16,
    tile: u32,
    engine: u16,
    use_free: bool,
    cargo: u8,
    client: u32,
) -> Result {
    unsafe {
        cmd_build_vehicle(
            s,
            out,
            temporary,
            refit_temporary,
            DoCommandFlags(flags),
            CmdBuildVehicle {
                tile: TileIndex(tile),
                eid: EngineID(engine),
                use_free_vehicles: use_free,
                cargo: CargoType(cargo),
                client_id: ClientID(client),
            },
        )
    }
}
/// `CmdBuildVehicle`: build a vehicle. Returns the new vehicle ID and the
/// refitted capacity, mail capacity (aircraft) and cargo capacities; the cost
/// or error is in `out`.
///
/// # Safety
/// All tables and three distinct constructed native costs live through the call.
/// Native allocations produce live shells. Every owner access ends before any
/// callback; speculative destruction leaves only the original returned ID value.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
pub(crate) unsafe fn cmd_build_vehicle(
    s: &'static Services,
    out: *mut CommandCost,
    temporary: *mut CommandCost,
    refit_temporary: *mut CommandCost,
    flags: DoCommandFlags,
    cmd: CmdBuildVehicle,
) -> Result {
    let tile = cmd.tile.0;
    let engine = cmd.eid.0;
    let cargo = cmd.cargo.0;
    let r = s.refit;
    let c = r.common;
    let b = c.base;
    let value = Cost(out, b.cost);
    let company = (b.shared.current_company)();
    /* Elementary check for valid location. */
    if !(s.mass.is_depot)(tile)
        || !crate::tile_map::is_tile_owner(
            b.shared,
            TileIndex(tile),
            crate::company_type::CompanyID(company),
        )
    {
        value.error(ERROR);
        return Result::default();
    }
    let kind = (s.depot_kind)(tile);
    /* Validate the engine type. */
    if !unsafe { (c.transactions.engine_buildable)(engine, kind.0, company) } {
        value.error(s.unavailable.wrapping_add(u32::from(kind.0)));
        return Result::default();
    }
    /* Validate the cargo type. */
    if cargo >= c.strings.cargo_count && (c.valid_cargo)(cargo) {
        value.error(ERROR);
        return Result::default();
    }
    let e = (r.engine)(engine);
    (b.cost.expense_money)(out, c.strings.new_vehicle_expense, unsafe {
        crate::engine_capabilities::openttd_rust_engine_cost(r.capabilities, e)
    });
    /* Engines without valid cargo should not be available */
    let default_cargo = crate::vehicle::EngineRef::of(e).default_cargo();
    if !(c.valid_cargo)(default_cargo) {
        value.error(ERROR);
        return Result::default();
    }
    let refitting = (c.valid_cargo)(cargo) && cargo != default_cargo;
    /* Check whether the number of vehicles we need to build can be built according to pool space. */
    let count = match kind {
        VehicleType::TRAIN => {
            u32::from(crate::vehicle::EngineRef::of(e).rail_kind() == RailVehicleTypes::MULTIHEAD)
                + 1
                + unsafe {
                    crate::engine_capabilities::openttd_rust_articulated_count(
                        r.capabilities,
                        engine,
                        false,
                    )
                }
        }
        VehicleType::ROAD => {
            1 + unsafe {
                crate::engine_capabilities::openttd_rust_articulated_count(
                    r.capabilities,
                    engine,
                    false,
                )
            }
        }
        VehicleType::SHIP => 1,
        VehicleType::AIRCRAFT => {
            if crate::vehicle::EngineRef::of(e).plane() {
                2
            } else {
                3
            }
        }
        _ => {
            (b.shared.not_reached)(); // Safe due to IsDepotTile()
            std::process::abort()
        }
    };
    if !(s.refit.common.vehicles.can_allocate)(count as usize) {
        value.error(s.too_many);
        return Result::default();
    }
    /* Check whether we can allocate a unit number. Autoreplace does not allocate
     * an unit number as it will (always) reuse the one of the replaced vehicle
     * and (train) wagons don't have an unit number in any scenario. */
    let unit = if flags.test(DoCommandFlag::QueryCost)
        || flags.test(DoCommandFlag::AutoReplace)
        || (kind == VehicleType::TRAIN
            && crate::vehicle::EngineRef::of(e).rail_kind() == RailVehicleTypes::WAGON)
    {
        0
    } else {
        (s.free_unit)(kind)
    };
    if unit == u16::MAX {
        value.error(s.too_many);
        return Result::default();
    }
    /* If we are refitting we need to temporarily purchase the vehicle to be able to
     * test it. */
    let subflags = if refitting && !flags.test(DoCommandFlag::Execute) {
        flags
            .with(DoCommandFlags::from_element(DoCommandFlag::Execute))
            .with(DoCommandFlags::from_element(DoCommandFlag::AutoReplace))
    } else {
        flags
    };
    /* Vehicle construction needs random bits, so we have to save the random
     * seeds to prevent desyncs. */
    let seeds = unsafe { (s.random.save)() };
    let v = match kind {
        VehicleType::TRAIN => unsafe {
            rail::cmd_build_rail_vehicle(
                s,
                temporary,
                subflags,
                rail::CmdBuildRailVehicle { tile: cmd.tile, e },
            )
        },
        VehicleType::ROAD => unsafe {
            construct::cmd_build_road_vehicle(
                s,
                temporary,
                subflags,
                construct::CmdBuildRoadVehicle { tile: cmd.tile, e },
            )
        },
        VehicleType::SHIP => {
            let north = (s.ship_tile)(tile);
            let v = if subflags.test(DoCommandFlag::Execute) {
                let v = (s.new_ship)(north);
                let id = unsafe { (b.groups.vehicle_id)(v) };
                unsafe {
                    crate::ship_control::ship_build(
                        id,
                        super::vehicle_ref(v, r.common.vehicles),
                        u32::from(engine),
                        r.ship,
                        b.shared,
                    );
                };
                v
            } else {
                ptr::null_mut()
            };
            (b.cost.zero)(temporary);
            v
        }
        VehicleType::AIRCRAFT => unsafe {
            construct::cmd_build_aircraft(
                s,
                temporary,
                subflags,
                construct::CmdBuildAircraft { tile: cmd.tile, e },
            )
        },
        _ => {
            (b.shared.not_reached)(); // Safe due to IsDepotTile()
            std::process::abort()
        }
    };
    value.add(Cost(temporary, b.cost));
    let mut result = Result::default();
    // This is deliberately one success test, before refitting. A failed refit
    // still follows the original accounting, temporary sale and restoration.
    if value.ok() {
        if subflags.test(DoCommandFlag::Execute) {
            (s.set_purchase)(v, unit, value.money());
            result.id = unsafe { (b.groups.vehicle_id)(v) };
        }
        if refitting {
            /* Refit only one vehicle. If we purchased an engine, it may have gained free wagons. */
            result.capacity = unsafe {
                refit::cmd_refit_vehicle(
                    r,
                    temporary,
                    refit_temporary,
                    flags,
                    refit::CmdRefitVehicle {
                        veh_id: crate::vehicle_type::VehicleID((b.groups.vehicle_id)(v)),
                        new_cargo_type: cmd.cargo,
                        new_subtype: 0,
                        auto_refit: false,
                        only_this: false,
                        num_vehicles: 1,
                    },
                )
            };
            value.add(Cost(temporary, b.cost));
        } else if kind == VehicleType::TRAIN || kind == VehicleType::ROAD {
            /* Fill in non-refitted capacities */
            unsafe {
                crate::engine_capabilities::openttd_rust_articulated_capacity(
                    r.capabilities,
                    engine,
                    result.capacity.cargo.as_mut_ptr(),
                );
            };
            result.capacity.total = result.capacity.cargo[usize::from(default_cargo)];
        } else {
            result.capacity.total = unsafe {
                crate::engine_capabilities::display_default_capacity(
                    r.capabilities,
                    e,
                    &raw mut result.capacity.mail,
                )
            };
            result.capacity.cargo[usize::from(default_cargo)] = result.capacity.total;
            let mail = crate::cargo_registry::get_cargo_type_by_label(
                crate::cargo_type::CargoLabel::CT_MAIL,
            )
            .0;
            if (c.valid_cargo)(mail) {
                result.capacity.cargo[usize::from(mail)] = u32::from(result.capacity.mail);
            }
        }
        if flags.test(DoCommandFlag::Execute) {
            if kind == VehicleType::TRAIN
                && cmd.use_free_vehicles
                && !flags.test(DoCommandFlag::AutoReplace)
                && (s.is_engine)(v)
            {
                /* Move any free wagons to the new vehicle. */
                // The speculative-build result in `temporary` was consumed above.
                unsafe { rail::normalize_train_veh_in_depot(s, temporary, v) };
            }
            (s.presentation)(v, kind);
        }
        if subflags.test(DoCommandFlag::Execute) {
            unsafe {
                crate::fleet::openttd_rust_fleet_count_engine(b.groups, v, 1);
                crate::fleet::openttd_rust_fleet_update_autoreplace(
                    b.groups,
                    (b.shared.current_company)(),
                );
            }
            if unsafe { (b.groups.primary)(v) } {
                unsafe { crate::fleet::openttd_rust_fleet_count_vehicle(b.groups, v, 1) };
                if !subflags.test(DoCommandFlag::AutoReplace) {
                    unsafe {
                        crate::orders::openttd_rust_backup_restore(
                            v.cast(),
                            cmd.client_id.0,
                            c.orders,
                        );
                    };
                }
            }
            (s.use_unit)(v);
        }
        /* If we are not in DoCommandFlag::Execute undo everything */
        if flags != subflags {
            let id = unsafe { (b.groups.vehicle_id)(v) };
            crate::command::nested(
                b.commands,
                b.shared,
                Cost(temporary, b.cost),
                0,
                DoCommandFlag::EXECUTE,
                |nested_flags| unsafe {
                    super::cmd_sell_vehicle(
                        c,
                        temporary,
                        DoCommandFlags(nested_flags),
                        super::CmdSellVehicle {
                            v_id: crate::vehicle_type::VehicleID(id),
                            sell_chain: false,
                            backup_order: false,
                            client_id: ClientID(c.strings.invalid_client),
                        },
                    );
                },
            );
        }
    }
    /* Only restore if we actually did some refitting */
    if flags != subflags {
        crate::witness::hit(crate::witness::VT_RESTORE);
        unsafe { (s.random.restore)(seeds) };
    }
    result
}

/// # Safety
/// As `cmd_build_vehicle`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_build(
    s: &'static Services,
    out: *mut CommandCost,
    temporary: *mut CommandCost,
    refit_temporary: *mut CommandCost,
    flags: u16,
    tile: u32,
    engine: u16,
    use_free: bool,
    cargo: u8,
    client: u32,
) -> Result {
    unsafe {
        cmd_build_vehicle(
            s,
            out,
            temporary,
            refit_temporary,
            DoCommandFlags(flags),
            CmdBuildVehicle {
                tile: TileIndex(tile),
                eid: EngineID(engine),
                use_free_vehicles: use_free,
                cargo: CargoType(cargo),
                client_id: ClientID(client),
            },
        )
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDVehicleBuildResult" ["probe"] => Result { id, capacity, };
    "OpenTTDVehicleBuildServices" ["probe"] => Services {
        refit, mass, random, depot_kind,
        free_unit, construct, rail, ship_tile, new_ship, set_purchase,
        is_engine, presentation, use_unit, too_many, unavailable,
    };
}
