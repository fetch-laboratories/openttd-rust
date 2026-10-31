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
/// Tick-end-only services; vehicle, owner, locality and cost reads reuse the
/// group and transaction tables.
#[repr(C)]
pub struct Services {
    pub(crate) set_current: unsafe extern "C" fn(u8),
    pub(crate) restart: unsafe extern "C" fn(*mut ()),
    pub(crate) x: unsafe extern "C" fn(*mut ()) -> i32,
    pub(crate) y: unsafe extern "C" fn(*mut ()) -> i32,
    pub(crate) z: unsafe extern "C" fn(*mut ()) -> i32,
    pub(crate) company: *const crate::company_identity::Services,
    pub(crate) subtract: unsafe extern "C" fn(i64),
    pub(crate) command: unsafe extern "C" fn(*mut (), u32),
    pub(crate) animation: unsafe extern "C" fn(i32, i32, i32, i64),
    pub(crate) length_news: unsafe extern "C" fn(u32),
    pub(crate) failed_news: unsafe extern "C" fn(u32, u32),
    pub(crate) cash: u32,
    pub(crate) limit: u32,
}
/// `INVALID_STRING_ID`.
const INVALID_STRING: u32 = 0xFFFF;

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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_pending_add(id: u32, leave: bool) {
    unsafe { (&mut *PENDING.get()).insert(id, leave) };
}
/// Drain in ascending IDs; newly inserted later keys remain visible this tick.
/// # Safety
/// Static noexcept services, serial game thread and a native caller-owned cost.
/// The map is accessed only to copy one current item before calling any service.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_pending_drain(
    services: *const Services,
    transaction: *const TransactionServices,
    group: *const GroupServices,
    cost: *mut (),
) {
    let w = unsafe { &*services };
    let t = unsafe { &*transaction };
    let g = unsafe { &*group };
    let previous = unsafe { (g.current_company)() };
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
        let v = unsafe { (g.vehicle)(id) };
        let owner = unsafe { (g.vehicle_owner)(v) };
        unsafe { (w.set_current)(owner) };
        if leave {
            unsafe { (w.restart)(v) };
        }
        let x = unsafe { (w.x)(v) };
        let y = unsafe { (w.y)(v) };
        let z = unsafe { (w.z)(v) };
        let company = unsafe { &*w.company };
        let reserve = i64::from((company.renew_reserve)((company.company)(
            crate::company_type::CompanyID(owner),
        )));
        unsafe {
            (w.subtract)(reserve);
            (w.command)(cost, id);
            (w.subtract)(reserve.wrapping_neg());
        }
        if !unsafe { (t.local)() } {
            continue;
        }
        if unsafe { (t.success)(cost) } {
            let money = unsafe { (t.money)(cost) };
            unsafe { (w.animation)(x, y, z, money) };
            continue;
        }
        let mut error = unsafe { (t.error)(cost) };
        if error == t.nothing || error == INVALID_STRING {
            continue;
        }
        if error == w.cash {
            error = w.limit;
        }
        if error == t.too_long_replacement {
            unsafe { (w.length_news)(id) };
        } else {
            unsafe { (w.failed_news)(id, error) };
        }
    }
    unsafe { (w.set_current)(previous) };
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDFleetPendingServices" ["fleet"] => Services {
        set_current,
        restart,
        x,
        y,
        z,
        company,
        subtract,
        command,
        animation,
        length_news,
        failed_news,
        cash,
        limit,
    };
}
