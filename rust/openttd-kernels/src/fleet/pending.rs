/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Ascending `VehicleID` tick-end autoreplacement with original overwrite semantics.
use super::GroupServices;
use super::transactions::Services as TransactionServices;
use crate::game_state::GameState;
use std::collections::BTreeMap;
/// Tick-end-only services; vehicle, locality and cost reads reuse the group and
/// transaction tables.
#[repr(C)]
pub struct Services {
    pub(crate) company: *const crate::company_identity::Services,
    pub(crate) subtract: unsafe extern "C" fn(i64),
    pub(crate) command: unsafe extern "C" fn(*mut (), u32),
    pub(crate) length_news: unsafe extern "C" fn(u32),
    pub(crate) failed_news: unsafe extern "C" fn(u32, u32),
    pub(crate) cash: u32,
    pub(crate) limit: u32,
}
/// `INVALID_STRING_ID`.
const INVALID_STRING: u32 = 0xFFFF;

/// `SetCurrentCompany`, through the command table's finance owner.
fn set_current_company(t: &TransactionServices, company: u8) {
    // SAFETY: The command table is immutable for the process lifetime.
    (unsafe { &*t.commands }.finance.set_current_company)(company);
}

static PENDING: GameState<BTreeMap<u32, bool>> = GameState::new(BTreeMap::new());
/// Clear pending replacements at the original initialization/tick-start point.
/// # Safety
/// Serial game-thread access; no map reference is active during this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_pending_clear() {
    unsafe { (&mut *PENDING.get()).clear() };
}
/// Record or overwrite the original `VehicleID` key's leave-depot flag.
/// # Safety
/// Serial game thread; map accesses end before returning/reentry.
pub(crate) unsafe fn add(id: u32, leave: bool) {
    unsafe { (&mut *PENDING.get()).insert(id, leave) };
}
/// Iterate the live pending map without retaining an entry across the action.
/// # Safety
/// Serialized access; the action may insert or overwrite entries synchronously.
unsafe fn for_each_pending(mut action: impl FnMut(u32, bool)) {
    let mut from = 0;
    loop {
        let item = unsafe {
            (&*PENDING.get())
                .range(from..)
                .next()
                .map(|(&id, &leave)| (id, leave))
        };
        let Some((id, leave)) = item else {
            break;
        };
        from = id + 1;
        action(id, leave);
    }
}

/// Drain in ascending IDs; newly inserted later keys remain visible this tick.
/// # Safety
/// Static noexcept services, serial game thread and a native caller-owned cost.
/// The map is accessed only to copy one current item before calling any service.
pub(crate) unsafe fn drain(
    services: *const Services,
    transaction: *const TransactionServices,
    group: *const GroupServices,
    cost: *mut (),
) {
    let w = unsafe { &*services };
    let t = unsafe { &*transaction };
    let g = unsafe { &*group };
    let previous = g.current_company();
    let visit = |id, leave| {
        let v = crate::fleet::record(g, unsafe { (g.vehicle)(id) });
        let owner = v.owner();
        set_current_company(t, owner);
        if leave {
            v.write_status(v.status() & !crate::vehicle_base::VS_STOPPED);
        }
        let x = v.x();
        let y = v.y();
        let z = v.z();
        let company = unsafe { &*w.company };
        let reserve = i64::from((company.renew_reserve)((company.company)(
            crate::company_type::CompanyID(owner),
        )));
        unsafe {
            (w.subtract)(reserve);
            (w.command)(cost, id);
            (w.subtract)(reserve.wrapping_neg());
        }
        if !crate::command::is_local_company(unsafe { &*t.commands }) {
            return;
        }
        if unsafe { (t.success)(cost) } {
            let money = unsafe { (t.money)(cost) };
            unsafe { ((*g.shared).cost_animation)(x, y, z, money) };
            return;
        }
        let mut error = unsafe { (t.error)(cost) };
        if error == t.nothing || error == INVALID_STRING {
            return;
        }
        if error == w.cash {
            error = w.limit;
        }
        if error == t.too_long_replacement {
            unsafe { (w.length_news)(id) };
        } else {
            unsafe { (w.failed_news)(id, error) };
        }
    };
    unsafe { for_each_pending(visit) };
    set_current_company(t, previous);
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDFleetPendingServices" ["fleet"] => Services {
        company,
        subtract,
        command,
        length_news,
        failed_news,
        cash,
        limit,
    };
}

/// Serializes the tests that touch the process-wide pending map.
#[cfg(test)]
pub(crate) static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_iteration_observes_later_insertions_and_overwrites() {
        let _guard = TEST_LOCK.lock().unwrap();
        // This test retains no pending-map entry while reentering insertion.
        unsafe { openttd_rust_fleet_pending_clear() };
        for (id, leave) in [(5, true), (3, false), (7, true)] {
            unsafe { add(id, leave) };
        }
        let mut trace = Vec::new();
        unsafe {
            for_each_pending(|id, leave| {
                trace.push((id, leave));
                if id == 5 {
                    add(9, true);
                    add(2, true);
                    add(5, false);
                }
                if id == 9 {
                    add(crate::vehicle_type::VehicleID::INVALID.0, false);
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
            assert_eq!((&*PENDING.get()).get(&2), Some(&true));
            assert_eq!((&*PENDING.get()).get(&5), Some(&false));
            openttd_rust_fleet_pending_clear();
        }
    }
}
