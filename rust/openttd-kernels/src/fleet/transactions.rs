/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Whole autoreplace transactions. Native costs/seeds live in the caller's stack.
use super::{
    ALL_GROUP, DEFAULT_GROUP, ERROR, GroupFields, GroupServices, INVALID_GROUP, add_profit,
    add_replacement, group, remove_replacement, replacement, update_autoreplace,
};
/// `EngineID::Invalid()`; shares the 0xFFFF encoding with `GroupID::Invalid()`.
const INVALID_ENGINE: u16 = INVALID_GROUP;
use std::ptr;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Costs {
    pub result: *mut (),
    pub replace: *mut (),
    pub build: *mut (),
    pub copy: *mut (),
    pub temporary: *mut (),
    pub seeds: *mut (),
}
#[repr(C)]
pub struct Services {
    pub(crate) cost_zero: unsafe extern "C" fn(*mut ()),
    pub(crate) cost_vehicles: unsafe extern "C" fn(*mut ()),
    pub(crate) cost_error: unsafe extern "C" fn(*mut (), u32),
    pub(crate) cost_add: unsafe extern "C" fn(*mut (), *mut ()),
    pub(crate) cost_move: unsafe extern "C" fn(*mut (), *mut ()),
    pub(crate) cost_amount: unsafe extern "C" fn(*mut (), i64),
    pub(crate) success: unsafe extern "C" fn(*mut ()) -> bool,
    pub(crate) error: unsafe extern "C" fn(*mut ()) -> u32,
    pub(crate) money: unsafe extern "C" fn(*mut ()) -> i64,
    pub(crate) commands: *const crate::services::CommandServices,
    pub(crate) rear: unsafe extern "C" fn(*mut ()) -> bool,
    pub(crate) articulated: unsafe extern "C" fn(*mut ()) -> bool,
    pub(crate) crashed: unsafe extern "C" fn(*mut ()) -> bool,
    pub(crate) stopped: unsafe extern "C" fn(*mut ()) -> bool,
    pub(crate) chain_depot: unsafe extern "C" fn(*mut ()) -> bool,
    pub(crate) first: unsafe extern "C" fn(*mut ()) -> *mut (),
    pub(crate) next_unit: unsafe extern "C" fn(*mut ()) -> *mut (),
    pub(crate) prev_unit: unsafe extern "C" fn(*mut ()) -> *mut (),
    pub(crate) length: unsafe extern "C" fn(*mut ()) -> u16,
    pub(crate) flipped: unsafe extern "C" fn(*mut ()) -> bool,
    pub(crate) cargo_type: unsafe extern "C" fn(*mut ()) -> u8,
    pub(crate) can_carry: unsafe extern "C" fn(*mut ()) -> bool,
    pub(crate) stopped_in_depot: unsafe extern "C" fn(*mut ()) -> bool,
    pub(crate) max_length: unsafe extern "C" fn() -> u8,
    pub(crate) check: unsafe extern "C" fn(bool),
    pub(crate) needs_renew: unsafe extern "C" fn(*mut (), bool) -> bool,
    pub(crate) engine_valid: unsafe extern "C" fn(u16) -> bool,
    pub(crate) company_valid: unsafe extern "C" fn(u8) -> bool,
    pub(crate) engine_buildable: unsafe extern "C" fn(u16, u8, u8) -> bool,
    pub(crate) rail_compatible: unsafe extern "C" fn(u16) -> u64,
    pub(crate) road_powered: unsafe extern "C" fn(u16) -> u64,
    pub(crate) wagon: unsafe extern "C" fn(u16) -> bool,
    pub(crate) tram: unsafe extern "C" fn(u16) -> bool,
    pub(crate) plane: unsafe extern "C" fn(u16) -> u8,
    pub(crate) refit_mask: unsafe extern "C" fn(u16, bool) -> u64,
    pub(crate) refit_masks: unsafe extern "C" fn(u16, *mut u64, *mut u64),
    pub(crate) vehicle_cargo: unsafe extern "C" fn(*mut (), *mut u8) -> u64,
    pub(crate) default_cargo: unsafe extern "C" fn(u16) -> u64,
    pub(crate) orders: unsafe extern "C" fn(*mut ()) -> *mut (),
    pub(crate) order_count: unsafe extern "C" fn(*mut ()) -> usize,
    pub(crate) order_count_id: unsafe extern "C" fn(*mut ()) -> u8,
    pub(crate) order_at: unsafe extern "C" fn(*mut (), usize) -> *mut (),
    pub(crate) order_refit: unsafe extern "C" fn(*mut ()) -> bool,
    pub(crate) order_auto: unsafe extern "C" fn(*mut ()) -> bool,
    pub(crate) order_cargo: unsafe extern "C" fn(*mut ()) -> u8,
    pub(crate) local: unsafe extern "C" fn() -> bool,
    pub(crate) refit_news: unsafe extern "C" fn(*mut (), i32),
    pub(crate) build: unsafe extern "C" fn(*mut (), *mut (), u16) -> *mut (),
    pub(crate) refit: unsafe extern "C" fn(*mut (), *mut (), u8, u8),
    pub(crate) subtype: unsafe extern "C" fn(*mut (), *mut (), u8) -> u8,
    pub(crate) reverse_probability: unsafe extern "C" fn(*mut ()) -> bool,
    pub(crate) reverse: unsafe extern "C" fn(*mut ()),
    pub(crate) start_stop: unsafe extern "C" fn(*mut (), *mut (), bool),
    pub(crate) move_vehicle: unsafe extern "C" fn(*mut (), *mut (), *mut (), u32, bool),
    pub(crate) sell: unsafe extern "C" fn(*mut (), *mut (), u32),
    pub(crate) clone_order: unsafe extern "C" fn(*mut (), *mut (), *mut ()),
    pub(crate) copy_group: unsafe extern "C" fn(*mut (), *mut (), *mut ()),
    pub(crate) copy_configuration: unsafe extern "C" fn(*mut (), *mut ()),
    pub(crate) viewports: unsafe extern "C" fn(*mut (), *mut ()),
    pub(crate) view_window: unsafe extern "C" fn(*mut (), *mut ()),
    pub(crate) news: unsafe extern "C" fn(*mut (), *mut ()),
    pub(crate) transfer_cargo: unsafe extern "C" fn(*mut (), *mut (), bool),
    pub(crate) capacity: unsafe extern "C" fn(*mut ()),
    pub(crate) event: unsafe extern "C" fn(*mut (), *mut ()),
    pub(crate) save_rng: unsafe extern "C" fn(*mut ()),
    pub(crate) restore_rng: unsafe extern "C" fn(*mut ()),
    pub(crate) rule_window: unsafe extern "C" fn(u16, u16),
    pub(crate) assertions: bool,
    pub(crate) unavailable: u32,
    pub(crate) too_long: u32,
    pub(crate) too_long_replacement: u32,
    pub(crate) nothing: u32,
}

const EXECUTE: u32 = 1;
const AUTOREPLACE: u32 = 1 << 6;
const INVALID_CARGO: u8 = 255;
const NO_REFIT: u8 = 254;
const TILE_SIZE: u16 = 16;

/// Original `assert` site: the condition is evaluated only in builds whose C++
/// `assert` is active, and the native wrapper terminates exactly like it.
macro_rules! check {
    ($w:expr, $condition:expr) => {
        if $w.assertions {
            let condition = $condition;
            unsafe { ($w.check)(condition) };
        }
    };
}

unsafe fn valid(w: &Services, gw: &GroupServices, from: u16, to: u16, company: u8) -> bool {
    let kind = unsafe { (gw.engine_type)(from) };
    if !unsafe { (w.engine_buildable)(to, kind, company) } {
        return false;
    }
    match kind {
        0 => {
            if unsafe { (w.rail_compatible)(from) & (w.rail_compatible)(to) } == 0 {
                return false;
            }
            if unsafe { (w.wagon)(from) != (w.wagon)(to) } {
                return false;
            }
        }
        1 => {
            if unsafe { (w.road_powered)(from) & (w.road_powered)(to) } == 0 {
                return false;
            }
            if unsafe { (w.tram)(from) != (w.tram)(to) } {
                return false;
            }
        }
        3 if unsafe { (w.plane)(from) != (w.plane)(to) } => {
            return false;
        }
        _ => (),
    }
    let a = unsafe { (w.refit_mask)(from, true) };
    let b = unsafe { (w.refit_mask)(to, true) };
    a == 0 || b == 0 || a & b != 0
}
unsafe fn order_vehicle(w: &Services, gw: &GroupServices, v: *mut ()) -> *mut () {
    if unsafe { (gw.vehicle_type)(v) } == 0 {
        unsafe { (w.first)(v) }
    } else {
        v
    }
}
unsafe fn verify_refits(w: &Services, gw: &GroupServices, v: *mut (), engine: u16) -> bool {
    let old_engine = unsafe { (gw.vehicle_engine)(v) };
    let old_mask = unsafe { (w.refit_mask)(old_engine, false) };
    let new_mask = unsafe { (w.refit_mask)(engine, false) };
    let u = unsafe { order_vehicle(w, gw, v) };
    let orders = unsafe { (w.orders)(u) };
    if orders.is_null() {
        return true;
    }
    let size = unsafe { (w.order_count)(orders) };
    for i in 0..size {
        let order = unsafe { (w.order_at)(orders, i) };
        if !unsafe { (w.order_refit)(order) } || unsafe { (w.order_auto)(order) } {
            continue;
        }
        let cargo = unsafe { (w.order_cargo)(order) };
        // Source shifts through HasBit; cargo values admitted here are <64.
        let mask = 1_u64.wrapping_shl(u32::from(cargo));
        if old_mask & mask == 0 {
            continue;
        }
        if new_mask & mask == 0 {
            return false;
        }
    }
    true
}
unsafe fn incompatible_refit(w: &Services, gw: &GroupServices, v: *mut (), engine: u16) -> i32 {
    let mask = unsafe { (w.refit_mask)(engine, false) };
    let u = unsafe { order_vehicle(w, gw, v) };
    let orders = unsafe { (w.orders)(u) };
    if orders.is_null() {
        return -1;
    }
    let size = unsafe { (w.order_count_id)(orders) };
    for i in 0..size {
        let order = unsafe { (w.order_at)(orders, usize::from(i)) };
        if !unsafe { (w.order_refit)(order) } {
            continue;
        }
        let cargo = unsafe { (w.order_cargo)(order) };
        if mask & 1_u64.wrapping_shl(u32::from(cargo)) == 0 {
            return i32::from(i);
        }
    }
    -1
}
unsafe fn new_cargo(
    w: &Services,
    gw: &GroupServices,
    mut v: *mut (),
    engine: u16,
    chain: bool,
) -> u8 {
    let mut union = 0;
    let mut available = 0;
    unsafe { (w.refit_masks)(engine, &raw mut union, &raw mut available) };
    if union == 0 {
        return NO_REFIT;
    }
    let mut cargo = INVALID_CARGO;
    let mask = unsafe { (w.vehicle_cargo)(v, &raw mut cargo) };
    if mask & mask.wrapping_sub(1) != 0 {
        let defaults = unsafe { (w.default_cargo)(engine) };
        if mask & defaults == mask {
            return NO_REFIT;
        }
        return INVALID_CARGO;
    }
    if cargo == INVALID_CARGO {
        if unsafe { (gw.vehicle_type)(v) } != 0 || !chain {
            return NO_REFIT;
        }
        v = unsafe { (w.first)(v) };
        while !v.is_null() {
            if unsafe { (w.can_carry)(v) } {
                let cargo = unsafe { (w.cargo_type)(v) };
                if available & 1_u64.wrapping_shl(u32::from(cargo)) != 0 {
                    return cargo;
                }
            }
            v = unsafe { (gw.next_part)(v) };
        }
        NO_REFIT
    } else {
        if available & 1_u64.wrapping_shl(u32::from(cargo)) == 0 {
            return INVALID_CARGO;
        }
        if chain && !unsafe { verify_refits(w, gw, v, engine) } {
            return INVALID_CARGO;
        }
        cargo
    }
}
unsafe fn new_engine(
    w: &Services,
    gw: &GroupServices,
    v: *mut (),
    always: bool,
    cost: *mut (),
) -> u16 {
    let kind = unsafe { (gw.vehicle_type)(v) };
    check!(w, kind != 0 || !unsafe { (w.articulated)(v) });
    unsafe { (w.cost_zero)(cost) };
    if kind == 0 && unsafe { (w.rear)(v) } {
        return INVALID_ENGINE;
    }
    let company = unsafe { (gw.current_company)() };
    let head = unsafe { (gw.head)(company).cast::<*mut ()>().read() };
    let from = unsafe { (gw.vehicle_engine)(v) };
    let id = unsafe { (gw.vehicle_group)(v) };
    let mut old = false;
    let mut engine = unsafe { replacement(gw, head, from, id, &raw mut old) };
    if !always && old && !unsafe { (w.needs_renew)(v, false) } {
        engine = INVALID_ENGINE;
    }
    if engine != INVALID_ENGINE && unsafe { (w.engine_buildable)(engine, kind, company) } {
        return engine;
    }
    if unsafe { (w.needs_renew)(v, true) } {
        engine = from;
    }
    if engine == INVALID_ENGINE || unsafe { (w.engine_buildable)(engine, kind, company) } {
        return engine;
    }
    unsafe { (w.cost_error)(cost, w.unavailable + u32::from(kind)) };
    engine
}
unsafe fn build(
    w: &Services,
    gw: &GroupServices,
    costs: Costs,
    old: *mut (),
    chain: bool,
    flags: u32,
) -> *mut () {
    let cost = costs.build;
    let engine = unsafe { new_engine(w, gw, old, true, cost) };
    if !unsafe { (w.success)(cost) } || engine == INVALID_ENGINE {
        return ptr::null_mut();
    }
    let cargo = unsafe { new_cargo(w, gw, old, engine, chain) };
    if cargo == INVALID_CARGO {
        if unsafe { (w.local)() } && flags & EXECUTE != 0 {
            let order = unsafe { incompatible_refit(w, gw, old, engine) };
            unsafe { (w.refit_news)(old, order) };
        }
        return ptr::null_mut();
    }
    let new = unsafe { (w.build)(cost, old, engine) };
    if !unsafe { (w.success)(cost) } {
        return ptr::null_mut();
    }
    if cargo != NO_REFIT {
        let subtype = unsafe { (w.subtype)(old, new, cargo) };
        unsafe {
            (w.refit)(costs.temporary, new, cargo, subtype);
            (w.cost_add)(cost, costs.temporary);
        }
        check!(w, unsafe { (w.success)(cost) });
    }
    if unsafe { (gw.vehicle_type)(new) } == 0
        && unsafe { (w.flipped)(old) }
        && !unsafe { (w.reverse_probability)(old) }
        && !unsafe { (w.reverse_probability)(new) }
    {
        unsafe { (w.reverse)(new) };
    }
    new
}
unsafe fn copy_head(
    w: &Services,
    gw: &GroupServices,
    costs: Costs,
    old: *mut (),
    new: *mut (),
    flags: u32,
) {
    let cost = costs.copy;
    unsafe { (w.cost_zero)(cost) };
    if old != new {
        unsafe {
            (w.clone_order)(costs.temporary, old, new);
            (w.cost_add)(cost, costs.temporary);
        }
    }
    if unsafe { (w.success)(cost) } && old != new {
        unsafe {
            (w.copy_group)(costs.temporary, old, new);
            (w.cost_add)(cost, costs.temporary);
        }
    }
    if unsafe { (w.success)(cost) } {
        check!(w, unsafe { (w.stopped)(new) });
        unsafe {
            (w.start_stop)(costs.temporary, new, true);
            (w.cost_add)(cost, costs.temporary);
        }
        if unsafe { (w.success)(cost) } {
            unsafe {
                (w.start_stop)(costs.temporary, new, false);
                (w.cost_add)(cost, costs.temporary);
            };
        }
    }
    if unsafe { (w.success)(cost) } && old != new && flags & EXECUTE != 0 {
        unsafe {
            (w.copy_configuration)(old, new);
            add_profit(gw, new);
            (w.viewports)(old, new);
            (w.view_window)(old, new);
            (w.news)(old, new);
        }
    }
}
unsafe fn replace_free(
    w: &Services,
    gw: &GroupServices,
    costs: Costs,
    unit: &mut *mut (),
    flags: u32,
    nothing: &mut bool,
) {
    let old = *unit;
    check!(
        w,
        !unsafe { (w.articulated)(old) } && !unsafe { (w.rear)(old) }
    );
    let cost = costs.replace;
    unsafe { (w.cost_vehicles)(cost) };
    let new = unsafe { build(w, gw, costs, old, false, flags) };
    unsafe { (w.cost_add)(cost, costs.build) };
    if unsafe { (w.success)(cost) } && !new.is_null() {
        *nothing = false;
        if flags & EXECUTE != 0 {
            unsafe {
                (w.move_vehicle)(costs.temporary, new, old, EXECUTE, false);
                (w.transfer_cargo)(old, new, false);
            }
            *unit = new;
            unsafe { (w.event)(old, new) };
        }
        unsafe {
            (w.sell)(costs.temporary, old, flags);
            (w.cost_add)(cost, costs.temporary);
        }
        if flags & EXECUTE == 0 {
            unsafe { (w.sell)(costs.temporary, new, EXECUTE) };
        }
    }
}
struct ChainItem {
    old: *mut (),
    new: *mut (),
    cost: i64,
}
impl ChainItem {
    fn vehicle(&self) -> *mut () {
        if self.new.is_null() {
            self.old
        } else {
            self.new
        }
    }
}
unsafe fn replace_chain(
    w: &Services,
    gw: &GroupServices,
    costs: Costs,
    chain: &mut *mut (),
    flags: u32,
    wagon_removal: bool,
    nothing: &mut bool,
) {
    let mut old_head = *chain;
    check!(w, unsafe { (gw.primary)(old_head) });
    let cost = costs.replace;
    unsafe { (w.cost_vehicles)(cost) };
    if unsafe { (gw.vehicle_type)(old_head) } == 0 {
        let length = unsafe { (w.length)(old_head) };
        let old_total_length =
            (u32::from(length).div_ceil(u32::from(TILE_SIZE)) * u32::from(TILE_SIZE)) as u16;
        let mut replacements = Vec::<ChainItem>::new();
        let mut unit = old_head;
        while !unit.is_null() {
            replacements.push(ChainItem {
                old: unit,
                new: ptr::null_mut(),
                cost: 0,
            });
            let new = unsafe { build(w, gw, costs, unit, true, flags) };
            let money = unsafe { (w.money)(costs.build) };
            // The item exists even on failure, preserving partial initialization.
            let item = replacements.last_mut().unwrap();
            item.new = new;
            item.cost = money;
            unsafe { (w.cost_add)(cost, costs.build) };
            if !unsafe { (w.success)(cost) } {
                break;
            }
            if !new.is_null() {
                *nothing = false;
            }
            unit = unsafe { (w.next_unit)(unit) };
        }
        let new_head = replacements[0].vehicle();
        if unsafe { (w.success)(cost) } {
            let second = unsafe { (w.next_unit)(old_head) };
            if !second.is_null() {
                unsafe {
                    (w.move_vehicle)(
                        costs.temporary,
                        second,
                        ptr::null_mut(),
                        EXECUTE | AUTOREPLACE,
                        true,
                    );
                    (w.cost_add)(cost, costs.temporary);
                }
            }
            check!(w, unsafe { (w.next_unit)(new_head) }.is_null());
            let mut last_engine: *mut () = ptr::null_mut();
            if unsafe { (w.success)(cost) } {
                for item in replacements.iter().rev() {
                    let append = item.vehicle();
                    let engine = unsafe { (gw.vehicle_engine)(append) };
                    if unsafe { (w.wagon)(engine) } {
                        continue;
                    }
                    if !item.new.is_null() {
                        unsafe {
                            (w.move_vehicle)(
                                costs.temporary,
                                item.old,
                                ptr::null_mut(),
                                EXECUTE | AUTOREPLACE,
                                false,
                            );
                        };
                    }
                    if last_engine.is_null() {
                        last_engine = append;
                    }
                    unsafe {
                        (w.move_vehicle)(costs.temporary, append, new_head, EXECUTE, false);
                        (w.cost_add)(cost, costs.temporary);
                    }
                    if !unsafe { (w.success)(cost) } {
                        break;
                    }
                }
                if last_engine.is_null() {
                    last_engine = new_head;
                }
            }
            if unsafe { (w.success)(cost) }
                && wagon_removal
                && unsafe { (w.length)(new_head) } > old_total_length
            {
                unsafe { (w.cost_error)(cost, w.too_long_replacement) };
            }
            if unsafe { (w.success)(cost) } {
                for item in replacements.iter().rev() {
                    check!(w, !last_engine.is_null());
                    let append = item.vehicle();
                    let engine = unsafe { (gw.vehicle_engine)(append) };
                    if unsafe { (w.wagon)(engine) } {
                        unsafe {
                            (w.move_vehicle)(costs.temporary, append, last_engine, EXECUTE, false);
                        };
                        if wagon_removal
                            && if unsafe { (w.success)(costs.temporary) } {
                                (unsafe { (w.length)(new_head) }) > old_total_length
                            } else {
                                (unsafe { (w.error)(costs.temporary) }) == w.too_long
                            }
                        {
                            unsafe {
                                (w.move_vehicle)(
                                    costs.temporary,
                                    append,
                                    ptr::null_mut(),
                                    EXECUTE | AUTOREPLACE,
                                    false,
                                );
                            };
                            break;
                        }
                        unsafe { (w.cost_add)(cost, costs.temporary) };
                        if !unsafe { (w.success)(cost) } {
                            break;
                        }
                    } else {
                        check!(w, append == last_engine);
                        last_engine = unsafe { (w.prev_unit)(last_engine) };
                    }
                }
            }
            if unsafe { (w.success)(cost) } && wagon_removal {
                check!(
                    w,
                    u32::from(unsafe { (w.length)(new_head) })
                        <= u32::from(unsafe { (w.max_length)() }) * u32::from(TILE_SIZE)
                );
                for item in replacements.iter_mut().skip(1) {
                    let wagon = item.new;
                    if wagon.is_null() {
                        continue;
                    }
                    if unsafe { (w.first)(wagon) } == new_head {
                        break;
                    }
                    check!(w, unsafe { (w.wagon)((gw.vehicle_engine)(wagon)) });
                    unsafe { (w.sell)(costs.temporary, wagon, EXECUTE) };
                    check!(w, unsafe { (w.success)(costs.temporary) });
                    item.new = ptr::null_mut();
                    unsafe { (w.cost_amount)(cost, item.cost.saturating_neg()) };
                }
            }
            if unsafe { (w.success)(cost) } {
                unsafe {
                    copy_head(w, gw, costs, old_head, new_head, flags);
                    (w.cost_add)(cost, costs.copy);
                }
            }
            if unsafe { (w.success)(cost) } {
                if flags & EXECUTE != 0 && new_head != old_head {
                    *chain = new_head;
                    unsafe { (w.event)(old_head, new_head) };
                }
                for (i, item) in replacements.iter_mut().enumerate() {
                    let old = item.old;
                    if unsafe { (w.first)(old) } == new_head {
                        continue;
                    }
                    if flags & EXECUTE != 0 {
                        unsafe { (w.transfer_cargo)(old, new_head, true) };
                    }
                    unsafe {
                        (w.sell)(costs.temporary, old, flags | AUTOREPLACE);
                        (w.cost_add)(cost, costs.temporary);
                    }
                    if flags & EXECUTE != 0 {
                        item.old = ptr::null_mut();
                        if i == 0 {
                            old_head = ptr::null_mut();
                        }
                    }
                }
                if flags & EXECUTE != 0 {
                    unsafe { (w.capacity)(new_head) };
                }
            }
            if flags & EXECUTE == 0 {
                let second = unsafe { (w.next_unit)(old_head) };
                if !second.is_null() {
                    unsafe {
                        (w.move_vehicle)(
                            costs.temporary,
                            second,
                            ptr::null_mut(),
                            EXECUTE | AUTOREPLACE,
                            true,
                        );
                    };
                }
                check!(w, unsafe { (w.next_unit)(old_head) }.is_null());
                for item in replacements.iter().rev() {
                    unsafe {
                        (w.move_vehicle)(
                            costs.temporary,
                            item.old,
                            old_head,
                            EXECUTE | AUTOREPLACE,
                            false,
                        );
                    };
                    check!(w, unsafe { (w.success)(costs.temporary) });
                }
            }
        }
        if flags & EXECUTE == 0 {
            for item in replacements.iter_mut().rev() {
                if !item.new.is_null() {
                    unsafe { (w.sell)(costs.temporary, item.new, EXECUTE) };
                    item.new = ptr::null_mut();
                }
            }
        }
    } else {
        let new_head = unsafe { build(w, gw, costs, old_head, true, flags) };
        unsafe { (w.cost_add)(cost, costs.build) };
        if unsafe { (w.success)(cost) } && !new_head.is_null() {
            *nothing = false;
            unsafe {
                copy_head(w, gw, costs, old_head, new_head, flags);
                (w.cost_add)(cost, costs.copy);
            }
            if unsafe { (w.success)(cost) } {
                if flags & EXECUTE != 0 {
                    unsafe { (w.transfer_cargo)(old_head, new_head, true) };
                    *chain = new_head;
                    unsafe { (w.event)(old_head, new_head) };
                }
                unsafe {
                    (w.sell)(costs.temporary, old_head, flags);
                    (w.cost_add)(cost, costs.temporary);
                }
            }
            if flags & EXECUTE == 0 {
                unsafe { (w.sell)(costs.temporary, new_head, EXECUTE) };
            }
        }
    }
}
unsafe fn autoreplace(w: &Services, gw: &GroupServices, costs: Costs, flags: u32, id: u32) {
    let cost = costs.result;
    let mut v = unsafe { (gw.vehicle)(id) };
    if v.is_null() {
        unsafe { (w.cost_error)(cost, ERROR) };
        return;
    }
    let owner = unsafe { (gw.vehicle_owner)(v) };
    // Native cost is live caller-owned CommandCost storage; no owner borrow spans the call.
    unsafe { ((*w.commands).ownership)(cost.cast(), owner) };
    if !unsafe { (w.success)(cost) } {
        return;
    }
    if unsafe { (w.crashed)(v) } {
        unsafe { (w.cost_error)(cost, ERROR) };
        return;
    }
    let kind = unsafe { (gw.vehicle_type)(v) };
    let mut free = false;
    if kind == 0 {
        if unsafe { (w.articulated)(v) || (w.rear)(v) } {
            unsafe { (w.cost_error)(cost, ERROR) };
            return;
        }
        free = !unsafe { (gw.front)(v) };
        let first = unsafe { (w.first)(v) };
        if free && unsafe { (gw.front)(first) } {
            unsafe { (w.cost_error)(cost, ERROR) };
            return;
        }
    } else if !unsafe { (gw.primary)(v) } {
        unsafe { (w.cost_error)(cost, ERROR) };
        return;
    }
    if !unsafe { (w.chain_depot)(v) } {
        unsafe { (w.cost_error)(cost, ERROR) };
        return;
    }
    let company = unsafe { (gw.current_company)() };
    let mut wagon_removal = unsafe { (gw.keep_length)(company) };
    let group_id = unsafe { (gw.vehicle_group)(v) };
    let g = unsafe { group(gw, group_id) };
    if !g.is_null() {
        wagon_removal = field!(g, GroupFields, flags) & 2 != 0;
    }
    let mut unit = v;
    let mut any = false;
    while !unit.is_null() {
        let engine = unsafe { new_engine(w, gw, unit, false, costs.temporary) };
        if !unsafe { (w.success)(costs.temporary) } {
            unsafe { (w.cost_move)(cost, costs.temporary) };
            return;
        }
        any |= engine != INVALID_ENGINE;
        unit = if !free && unsafe { (gw.vehicle_type)(unit) } == 0 {
            unsafe { (w.next_unit)(unit) }
        } else {
            ptr::null_mut()
        };
    }
    unsafe { (w.cost_vehicles)(cost) };
    let mut nothing = true;
    if any {
        let stopped = free || unsafe { (w.stopped)(v) };
        if !stopped {
            unsafe {
                (w.start_stop)(costs.temporary, v, true);
                (w.cost_add)(cost, costs.temporary);
            };
        }
        if !unsafe { (w.success)(cost) } {
            return;
        }
        check!(w, free || unsafe { (w.stopped_in_depot)(v) });
        unsafe { (w.save_rng)(costs.seeds) };
        if free {
            unsafe { replace_free(w, gw, costs, &mut v, flags & !EXECUTE, &mut nothing) };
        } else {
            unsafe {
                replace_chain(
                    w,
                    gw,
                    costs,
                    &mut v,
                    flags & !EXECUTE,
                    wagon_removal,
                    &mut nothing,
                );
            };
        }
        unsafe {
            (w.cost_add)(cost, costs.replace);
            (w.restore_rng)(costs.seeds);
        }
        if unsafe { (w.success)(cost) } && flags & EXECUTE != 0 {
            if free {
                unsafe { replace_free(w, gw, costs, &mut v, flags, &mut nothing) };
            } else {
                unsafe { replace_chain(w, gw, costs, &mut v, flags, wagon_removal, &mut nothing) };
            }
            // Source ret is intentionally not AddCost'ed: the test pass owns the cost.
            check!(
                w,
                unsafe { (w.success)(costs.replace) }
                    && unsafe { (w.money)(costs.replace) == (w.money)(cost) }
            );
        }
        if !stopped {
            unsafe {
                (w.start_stop)(costs.temporary, v, false);
                (w.cost_add)(cost, costs.temporary);
            };
        }
    }
    if unsafe { (w.success)(cost) } && nothing {
        unsafe { (w.cost_error)(cost, w.nothing) };
    }
}
unsafe fn set_rule(
    w: &Services,
    gw: &GroupServices,
    flags: u32,
    id: u16,
    from: u16,
    to: u16,
    old: bool,
) -> u32 {
    let company = unsafe { (gw.current_company)() };
    if !unsafe { (w.company_valid)(company) } {
        return ERROR;
    }
    let g = unsafe { group(gw, id) };
    if if g.is_null() {
        id != ALL_GROUP && id != DEFAULT_GROUP
    } else {
        field!(g, GroupFields, owner) != company
    } {
        return ERROR;
    }
    if !unsafe { (w.engine_valid)(from) } {
        return ERROR;
    }
    let kind = unsafe { (gw.engine_type)(from) };
    if !g.is_null() && field!(g, GroupFields, vehicle_type) != kind {
        return ERROR;
    }
    let head = unsafe { (gw.head)(company).cast::<*mut ()>() };
    let cost = if to == INVALID_ENGINE {
        unsafe { remove_replacement(gw, head, from, id, flags) }
    } else {
        if !unsafe { (w.engine_valid)(to) } || !unsafe { valid(w, gw, from, to, company) } {
            return ERROR;
        }
        unsafe { add_replacement(gw, head, from, to, id, old, flags) }
    };
    if flags & EXECUTE != 0 {
        unsafe { update_autoreplace(gw, company) };
        if unsafe { (w.local)() } {
            unsafe { (gw.replace_dirty)(kind) };
        }
        unsafe { (gw.list_set_dirty)(company, kind) };
    }
    if flags & EXECUTE != 0 && unsafe { (w.local)() } {
        unsafe { (w.rule_window)(from, id) };
    }
    cost
}
/// Original eligibility policy using direct typed shared-world services.
/// # Safety
/// Live static noexcept services, valid engines and serial game-thread access.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_valid(
    w: *const Services,
    gw: *const GroupServices,
    from: u16,
    to: u16,
    company: u8,
) -> bool {
    unsafe { valid(&*w, &*gw, from, to, company) }
}
/// Entire speculative/execute autoreplacement and rollback with native cost lifetimes.
/// # Safety
/// Costs/seeds are unique native stack objects for this call. Services are noexcept;
/// no canonical Rust borrow spans commands, `NewGRF` resolution, or destruction.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_autoreplace(
    w: *const Services,
    gw: *const GroupServices,
    costs: *const Costs,
    flags: u32,
    id: u32,
) {
    unsafe { autoreplace(&*w, &*gw, *costs, flags, id) }
}
/// Entire renewal-rule command, including original failure-side GUI invalidation.
/// # Safety
/// Static noexcept services and serial game thread, no Rust field borrow across calls.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_set_rule(
    w: *const Services,
    gw: *const GroupServices,
    flags: u32,
    id: u16,
    from: u16,
    to: u16,
    old: bool,
) -> u32 {
    unsafe { set_rule(&*w, &*gw, flags, id, from, to, old) }
}
