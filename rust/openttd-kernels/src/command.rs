/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared ordinary command control. Native costs retain storage and encoded
//! errors; recursion stays native. No owner borrow crosses a native service.
#![allow(unsafe_code)]
use crate::services::{CommandCost, CommandCostServices, CommandServices};

/// Original `InternalDoBefore`, called directly by Rust command owners.
pub(crate) fn before(_s: &CommandServices, top: bool, test: bool) {
    if top {
        crate::objects::clear_areas();
    }
    if test {
        crate::town::openttd_rust_town_rating_test(true);
    }
}
/// Original command money availability, with the original short-circuit order.
pub(crate) fn available_money(s: &CommandServices) -> i64 {
    if (s.infinite_money)() {
        return i64::MAX;
    }
    let f = s.finance;
    let p = (f.company_finances)((f.current_company)());
    // SAFETY: null or live canonical finance owner, serialized by the caller.
    unsafe { crate::company::openttd_rust_company_available_money(p, false) }
}
/// The native result is live throughout this synchronous call.
pub(crate) unsafe fn check_money(
    s: &CommandServices,
    costs: &CommandCostServices,
    out: *mut CommandCost,
) -> bool {
    let amount = (costs.money)(out);
    if amount <= 0 || (s.infinite_money)() {
        return true;
    }
    let f = s.finance;
    let p = (f.company_finances)((f.current_company)());
    // SAFETY: null or live owner; no borrow spans native error formatting.
    if unsafe { crate::company::has_money(p, false, amount) } {
        return true;
    }
    // SAFETY: live native result; native callback retains neither pointer nor borrow.
    unsafe { (s.money_error)(out, amount) };
    false
}
/// Subtract only for a valid company and nonzero command cost.
/// The native result retains the source valid-expense precondition.
pub(crate) unsafe fn subtract_money(
    s: &CommandServices,
    costs: &CommandCostServices,
    out: *const CommandCost,
) {
    let f = s.finance;
    let id = (f.current_company)();
    let p = (f.company_finances)(id);
    if !p.is_null() {
        let amount = (costs.money)(out);
        if amount != 0 {
            crate::company::subtract_money(f, p, id, amount, usize::from((costs.expense)(out).0));
        }
    }
}
/// Original `InternalDoAfter`, called directly by Rust command owners.
/// `out` denotes the live native result for the synchronous entry.
pub(crate) unsafe fn after(
    s: &CommandServices,
    costs: &CommandCostServices,
    out: *mut CommandCost,
    flags: u16,
    top: bool,
    test: bool,
) {
    if test {
        crate::town::openttd_rust_town_rating_test(false);
        if (costs.success)(out) && top && flags & (4 | 32) == 0 {
            // SAFETY: live native result, with no borrow retained across callbacks.
            unsafe { check_money(s, costs, out) };
        }
    } else if (costs.success)(out) && top && flags & 32 == 0 {
        // SAFETY: same result lifetime and original expense precondition.
        unsafe { subtract_money(s, costs, out) };
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_command_before(s: &CommandServices, top: bool, test: bool) {
    before(s, top, test);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_command_available_money(s: &CommandServices) -> i64 {
    available_money(s)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_check_money(
    s: &CommandServices,
    costs: &CommandCostServices,
    out: *mut CommandCost,
) -> bool {
    // SAFETY: native caller provides the synchronous live result.
    unsafe { check_money(s, costs, out) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_subtract_money(
    s: &CommandServices,
    costs: &CommandCostServices,
    out: *const CommandCost,
) {
    // SAFETY: native caller provides the live result and valid expense category.
    unsafe { subtract_money(s, costs, out) };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_after(
    s: &CommandServices,
    costs: &CommandCostServices,
    out: *mut CommandCost,
    flags: u16,
    top: bool,
    test: bool,
) {
    // SAFETY: native caller provides the synchronous live result.
    unsafe { after(s, costs, out, flags, top, test) };
}
