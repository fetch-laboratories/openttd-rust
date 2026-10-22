/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Depot and list command dispatch with retained, transaction-local selection.
#![allow(clippy::too_many_lines)]
use super::{CommonServices, ERROR, SelectionServices, depot_list, generate_list};
use crate::command_type::DoCommandFlag;
use crate::services::{CommandCost, Cost};
use std::cell::UnsafeCell;

#[repr(C)]
pub struct ListId {
    pub kind: u8,
    pub vehicle_type: u8,
    pub company: u8,
    pub index: u32,
    pub valid: bool,
}
#[repr(C)]
pub struct Services {
    pub common: &'static CommonServices,
    pub selection: &'static SelectionServices,
    pub is_depot: extern "C" fn(u32) -> bool,
    pub send: extern "C" fn(*mut CommandCost, *mut (), u16, u8),
    pub service: u8,
    pub mass_send: u8,
    pub dont_cancel: u8,
    pub invalid_client: u32,
}
struct Pool(UnsafeCell<Vec<Vec<*mut ()>>>);
// SAFETY: Game-state mutex serializes commands. Active selections are removed
// from this process-wide pool, and no pool access spans a native callback.
unsafe impl Sync for Pool {}
static POOL: Pool = Pool(UnsafeCell::new(Vec::new()));
struct List(Vec<*mut ()>);
impl List {
    fn new() -> Self {
        // SAFETY: Serialized access; nested commands cannot see this active frame.
        Self(unsafe { (&mut *POOL.0.get()).pop() }.unwrap_or_default())
    }
}
impl Drop for List {
    fn drop(&mut self) {
        self.0.clear();
        // SAFETY: Cleared pointers have no destructors or callbacks.
        unsafe {
            (&mut *POOL.0.get()).push(std::mem::take(&mut self.0));
        }
    }
}
unsafe fn generate(s: &Services, list: &mut List, id: &ListId) -> bool {
    unsafe {
        generate_list(
            s.selection,
            id.kind,
            id.vehicle_type,
            id.company,
            id.index,
            |v| list.0.push(v),
        )
    }
}
fn depot(s: &Services, tile: u32) -> bool {
    (s.is_depot)(tile)
        && crate::tile_map::is_tile_owner(
            s.common.base.shared,
            tile,
            (s.common.base.shared.current_company)(),
        )
}
unsafe fn nested_start(s: &Services, cost: *mut CommandCost, flags: u16, id: u32) {
    let b = s.common.base;
    crate::command::nested(
        b.commands,
        b.shared,
        Cost(cost, b.cost),
        0,
        flags,
        |flags| unsafe {
            super::startstop(s.common, cost, flags, id, false);
        },
    );
}
unsafe fn nested_sell(s: &Services, cost: *mut CommandCost, flags: u16, id: u32) {
    let b = s.common.base;
    crate::command::nested(
        b.commands,
        b.shared,
        Cost(cost, b.cost),
        0,
        flags,
        |flags| unsafe {
            super::sell(s.common, cost, flags, id, true, false, s.invalid_client);
        },
    );
}
/// # Safety
/// Live immutable services and distinct caller-owned stack costs. Selection is
/// frozen before commands; no list or owner borrow survives command reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_mass_start(
    s: &Services,
    cost: *mut CommandCost,
    item: *mut CommandCost,
    flags: u16,
    tile: u32,
    start: bool,
    window: bool,
    id: &ListId,
) {
    let b = s.common.base;
    let mut list = List::new();
    if !id.valid || !unsafe { (b.groups.buildable_type)(id.vehicle_type) } {
        (b.cost.error)(cost, ERROR);
        return;
    }
    if window {
        if !unsafe { generate(s, &mut list, id) } {
            (b.cost.error)(cost, ERROR);
            return;
        }
    } else {
        if !depot(s, tile) {
            (b.cost.error)(cost, ERROR);
            return;
        }
        unsafe {
            depot_list(s.selection, id.vehicle_type, tile, false, false, |v, _| {
                list.0.push(v);
            });
        }
    }
    for i in 0..list.0.len() {
        let v = list.0[i];
        if unsafe { (s.common.transactions.stopped)(v) } != start {
            continue;
        }
        if !window && !unsafe { (s.common.transactions.chain_depot)(v) } {
            continue;
        }
        unsafe {
            nested_start(s, item, flags, (b.groups.vehicle_id)(v));
        }
    }
    (b.cost.zero)(cost);
}
/// # Safety
/// Native stack costs and selection services remain live through deletion.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_depot_sell(
    s: &Services,
    cost: *mut CommandCost,
    item: *mut CommandCost,
    last_error: *mut CommandCost,
    flags: u16,
    tile: u32,
    kind: u8,
) {
    let b = s.common.base;
    let mut list = List::new();
    (b.cost.expense_money)(cost, s.common.strings.new_vehicle_expense, 0);
    if !unsafe { (b.groups.buildable_type)(kind) } || !depot(s, tile) {
        (b.cost.error)(cost, ERROR);
        return;
    }
    unsafe {
        depot_list(s.selection, kind, tile, true, false, |v, _| list.0.push(v));
    }
    (b.cost.error)(last_error, ERROR);
    let mut success = false;
    for i in 0..list.0.len() {
        let v = list.0[i];
        unsafe {
            nested_sell(s, item, flags, (b.groups.vehicle_id)(v));
        }
        if (b.cost.success)(item) {
            (b.cost.amount)(cost, (b.cost.money)(item));
            success = true;
        } else {
            (b.cost.move_cost)(last_error, item);
        }
    }
    if !success {
        (b.cost.move_cost)(cost, last_error);
    }
}
/// # Safety
/// Fleet cost slots/seeds are caller-stack storage, distinct from the aggregate.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_depot_replace(
    s: &Services,
    cost: *mut CommandCost,
    costs: &crate::fleet::transactions::Costs,
    flags: u16,
    tile: u32,
    kind: u8,
) {
    let b = s.common.base;
    let mut list = List::new();
    (b.cost.expense_money)(cost, s.common.strings.new_vehicle_expense, 0);
    if !unsafe { (b.groups.buildable_type)(kind) } || !depot(s, tile) {
        (b.cost.error)(cost, ERROR);
        return;
    }
    unsafe {
        depot_list(s.selection, kind, tile, true, true, |v, _| list.0.push(v));
    }
    for i in 0..list.0.len() {
        let v = list.0[i];
        if !unsafe { (s.common.transactions.chain_depot)(v) } {
            continue;
        }
        let id = unsafe { (b.groups.vehicle_id)(v) };
        crate::command::nested(
            b.commands,
            b.shared,
            Cost(costs.result.cast(), b.cost),
            0,
            flags,
            |flags| unsafe {
                crate::fleet::transactions::openttd_rust_fleet_autoreplace(
                    s.common.transactions,
                    b.groups,
                    costs,
                    u32::from(flags),
                    id,
                );
            },
        );
        if (b.cost.success)(costs.result.cast()) {
            (b.cost.amount)(cost, (b.cost.money)(costs.result.cast()));
        }
    }
}
unsafe fn send_one(s: &Services, cost: *mut CommandCost, flags: u16, id: u32, command: u8) {
    let b = s.common.base;
    let v = unsafe { (b.groups.vehicle)(id) };
    if v.is_null() || !unsafe { (b.groups.primary)(v) } {
        (b.cost.error)(cost, ERROR);
        return;
    }
    (s.send)(cost, v, flags, command);
}
/// # Safety
/// The native virtual `SendToDepot` callee retains its original owner. All Rust
/// selection borrows end before calling it or the canonical nested protocol.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_send(
    s: &Services,
    cost: *mut CommandCost,
    item: *mut CommandCost,
    flags: u16,
    id: u32,
    command: u8,
    list_id: &ListId,
) {
    if command & s.mass_send == 0 {
        unsafe {
            send_one(s, cost, flags, id, command);
        }
        return;
    }
    let b = s.common.base;
    if !list_id.valid {
        (b.cost.error)(cost, ERROR);
        return;
    }
    let mut list = List::new();
    if !unsafe { generate(s, &mut list, list_id) } {
        (b.cost.error)(cost, ERROR);
        return;
    }
    let mut success = false;
    let command = (if command & s.service != 0 {
        s.service
    } else {
        0
    }) | s.dont_cancel;
    for i in 0..list.0.len() {
        let id = unsafe { (b.groups.vehicle_id)(list.0[i]) };
        crate::command::nested(
            b.commands,
            b.shared,
            Cost(item, b.cost),
            0,
            flags,
            |flags| unsafe {
                send_one(s, item, flags, id, command);
            },
        );
        if (b.cost.success)(item) {
            success = true;
            if flags & DoCommandFlag::EXECUTE == 0 {
                break;
            }
        }
    }
    if success {
        (b.cost.zero)(cost);
    } else {
        (b.cost.error)(cost, ERROR);
    }
}
