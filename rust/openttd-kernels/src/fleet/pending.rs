/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `vehicle.cpp`'s `_vehicles_to_autoreplace` and the autoreplace loop that ends
//! `CallVehicleTicks`.
//!
//! Lifetime contract: the map is a process-wide static of the serialized game state, read and
//! written only on the game thread, and no borrow of it lives across a command or any other
//! call that can insert into it. The tables are the process-lifetime tables of
//! [`super::transactions`].

use super::GroupServices;
use super::transactions::{CmdAutoreplaceVehicle, Services as TransactionServices};
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company_type::CompanyID;
use crate::economy_type::ExpensesType;
use crate::game_state::GameState;
use crate::news_type::AdviceType;
use crate::strings_type::INVALID_STRING_ID;
use crate::strings_type::StringParameterData::{Empty, Integer};
use crate::table::strings::{
    STR_ERROR_AUTOREPLACE_MONEY_LIMIT, STR_ERROR_AUTOREPLACE_NOTHING_TO_DO,
    STR_ERROR_NOT_ENOUGH_CASH_REQUIRES_CURRENCY, STR_ERROR_TRAIN_TOO_LONG_AFTER_REPLACEMENT,
    STR_NEWS_VEHICLE_AUTORENEW_FAILED,
};
use crate::tile_type::TileIndex;
use crate::vehicle_base::VehState;
use crate::vehicle_type::VehicleID;
use std::collections::BTreeMap;

/// `SetCurrentCompany`, through the command table's finance owner.
#[allow(
    unsafe_code,
    reason = "the command table pointer names the process-lifetime table"
)]
fn set_current_company(t: &TransactionServices, company: CompanyID) {
    // SAFETY: The command table is immutable for the process lifetime.
    (unsafe { &*t.commands }.finance.set_current_company)(company.0);
}

/// `_vehicles_to_autoreplace`: the vehicles to autoreplace at the end of the tick, each with
/// whether to start it again after it was stopped on entering the depot.
static VEHICLES_TO_AUTOREPLACE: GameState<BTreeMap<VehicleID, bool>> =
    GameState::new(BTreeMap::new());
/// `_vehicles_to_autoreplace.clear()`, for `InitializeVehicles` and `CallVehicleTicks`.
/// # Safety
/// Serial game-thread access; no map reference is active during this call.
#[allow(
    unsafe_code,
    reason = "the process-wide pending map, under the module contract"
)]
pub(crate) unsafe fn clear() {
    unsafe { (&mut *VEHICLES_TO_AUTOREPLACE.get()).clear() };
}
/// [`clear`], for `InitializeVehicles`.
/// # Safety
/// As [`clear`].
#[allow(
    unsafe_code,
    reason = "the process-wide pending map, under the module contract"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_pending_clear() {
    unsafe { clear() };
}
/// `_vehicles_to_autoreplace[index] = leave_depot`.
/// # Safety
/// Serial game thread; map accesses end before returning.
#[allow(
    unsafe_code,
    reason = "the process-wide pending map, under the module contract"
)]
pub(crate) unsafe fn add(index: VehicleID, leave_depot: bool) {
    unsafe { (&mut *VEHICLES_TO_AUTOREPLACE.get()).insert(index, leave_depot) };
}
/// `for (auto &it : _vehicles_to_autoreplace)`: the walk of the live map, copying one entry
/// before `action` runs, so `action` may insert or overwrite entries as `std::map`'s
/// iteration allows; an entry inserted after the cursor is visited.
/// # Safety
/// Serialized access; no map reference is retained across `action`.
#[allow(
    unsafe_code,
    reason = "the process-wide pending map, under the module contract"
)]
unsafe fn for_each_pending(mut action: impl FnMut(VehicleID, bool)) {
    let mut from = VehicleID(0);
    loop {
        let item = unsafe {
            (&*VEHICLES_TO_AUTOREPLACE.get())
                .range(from..)
                .next()
                .map(|(&id, &leave)| (id, leave))
        };
        let Some((id, leave)) = item else {
            break;
        };
        from = VehicleID(id.0 + 1);
        action(id, leave);
    }
}

/// The tables an entry of [`drain`] reaches, resolved at the first entry.
struct Tables {
    /// The vehicle build table, for the base transaction table.
    build: &'static crate::vehicle_transactions::build::Services,
    /// The company table, for the company pool and its settings.
    company: &'static crate::company::Services,
    /// The vehicle pool.
    pool: crate::vehicle_base::Pool,
}

/// The autoreplace loop at the end of `CallVehicleTicks`, over the vehicles that entered a
/// depot this tick in ascending `VehicleID`; `w` is the transaction table and `g` the group
/// table, which reads `_current_company`.
/// # Safety
/// Serial game thread; no pool, record or map borrow survives into the fleet commands.
#[allow(
    unsafe_code,
    reason = "the process-wide pending map and the command table"
)]
#[allow(clippy::many_single_char_names, reason = "the original's names")]
pub(crate) unsafe fn drain(w: &TransactionServices, g: &GroupServices) {
    let cur_company = g.current_company();
    let mut tables: Option<Tables> = None;
    let visit = |id: VehicleID, leave_depot: bool| {
        let t = tables.get_or_insert_with(|| {
            let build = (w.build)();
            Tables {
                build,
                // SAFETY: The command table names the process-lifetime company table.
                company: unsafe { &*((*w.commands).company)() },
                pool: g.vehicles.orders().scheduler().pool(),
            }
        });
        let base = t.build.refit.common.base;
        // `Vehicle::Get(it.first)` of a pending vehicle: its slot holds it, as the original's
        // dereference of the result requires.
        let Some(v) = crate::vehicle_base::get_asserted(&t.pool, id) else {
            crate::services_assert::not_reached()
        };
        /* Autoreplace needs the current company set as the vehicle owner */
        set_current_company(w, v.owner());

        /* Start vehicle if we stopped them in VehicleEnteredDepotThisTick()
         * We need to stop them between VehicleEnteredDepotThisTick() and here or we risk that
         * they are already leaving the depot again before being replaced. */
        if leave_depot {
            v.set_vehstatus(v.vehstatus().reset(VehState::Stopped));
        }

        /* Store the position of the effect as the vehicle pointer will become invalid later */
        let x = v.x_pos();
        let y = v.y_pos();
        let z = v.z_pos();

        let identity = t.company.identity;
        let c = (identity.company)(g.current_company());
        let finance = unsafe { &*w.commands }.finance;
        crate::company::subtract_money_from_company(
            base.shared,
            finance,
            i64::from((identity.engine_renew_money)(c)),
            ExpensesType::EXPENSES_NEW_VEHICLES,
        );
        // `v->index` is the map's key: the pool stores each vehicle at its index.
        crate::fleet::transactions::with_cost(base.cost, |res| {
            crate::fleet::transactions::command(
                base,
                res,
                TileIndex(0),
                DoCommandFlags::from_element(DoCommandFlag::Execute),
                |flags| {
                    super::transactions::cmd_autoreplace_vehicle(
                        w,
                        res,
                        flags,
                        CmdAutoreplaceVehicle { veh_id: id },
                    );
                },
            );
            crate::company::subtract_money_from_company(
                base.shared,
                finance,
                -i64::from((identity.engine_renew_money)(c)),
                ExpensesType::EXPENSES_NEW_VEHICLES,
            );

            if !w.is_local_company() {
                return;
            }

            if res.ok() {
                (base.shared.cost_animation)(x, y, z, res.money());
                return;
            }

            let mut error_message = res.message();
            if error_message == STR_ERROR_AUTOREPLACE_NOTHING_TO_DO.0
                || error_message == INVALID_STRING_ID
            {
                return;
            }

            if error_message == STR_ERROR_NOT_ENOUGH_CASH_REQUIRES_CURRENCY.0 {
                error_message = STR_ERROR_AUTOREPLACE_MONEY_LIMIT.0;
            }

            // The vehicle may be sold by now; `v->index` is still the map's key.
            let headline = if error_message == STR_ERROR_TRAIN_TOO_LONG_AFTER_REPLACEMENT.0 {
                crate::strings_func::get_encoded_string(
                    crate::strings_type::StringID(error_message),
                    &[Integer(u64::from(id.0))],
                )
            } else {
                crate::strings_func::get_encoded_string(
                    STR_NEWS_VEHICLE_AUTORENEW_FAILED,
                    &[
                        Integer(u64::from(id.0)),
                        Integer(u64::from(error_message)),
                        Empty,
                    ],
                )
            };

            crate::news_func::add_vehicle_advice_news_item(
                AdviceType::AutorenewFailed,
                &headline,
                id,
            );
        });
    };
    unsafe { for_each_pending(visit) };
    set_current_company(w, cur_company);
}

/// Serializes the tests that touch the process-wide pending map.
#[cfg(test)]
pub(crate) static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(unsafe_code)] // The process-wide pending map, under the module contract.
    fn pending_iteration_observes_later_insertions_and_overwrites() {
        let _guard = TEST_LOCK.lock().unwrap();
        // This test retains no pending-map entry while reentering insertion.
        unsafe { openttd_rust_fleet_pending_clear() };
        for (id, leave) in [(5, true), (3, false), (7, true)] {
            unsafe { add(VehicleID(id), leave) };
        }
        let mut trace = Vec::new();
        unsafe {
            for_each_pending(|id, leave| {
                trace.push((id.0, leave));
                if id == VehicleID(5) {
                    add(VehicleID(9), true);
                    add(VehicleID(2), true);
                    add(VehicleID(5), false);
                }
                if id == VehicleID(9) {
                    add(VehicleID::INVALID, false);
                }
            });
        }
        assert_eq!(
            trace,
            [
                (3, false),
                (5, true),
                (7, true),
                (9, true),
                (0xFFFFF, false)
            ]
        );
        // Earlier/current keys remain stored, but are not revisited this tick.
        unsafe {
            assert_eq!(
                (&*VEHICLES_TO_AUTOREPLACE.get()).get(&VehicleID(2)),
                Some(&true)
            );
            assert_eq!(
                (&*VEHICLES_TO_AUTOREPLACE.get()).get(&VehicleID(5)),
                Some(&false)
            );
            openttd_rust_fleet_pending_clear();
        }
    }
}
