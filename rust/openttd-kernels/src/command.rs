/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared ordinary command preparation and result processing. Native stack costs
//! and company owners remain live for synchronous calls under the game mutex.
//! No owner borrow crosses a callback; panics and native exceptions abort.
#![allow(unsafe_code)]
use crate::company;
use crate::services::{CommandCost, CommandCostServices, CommandServices};

pub(crate) fn before(s: &CommandServices, top_level: bool, test: bool) {
    if top_level {
        (s.clear_object_areas)();
    }
    if test {
        crate::town::openttd_rust_town_rating_test(true);
    }
}

pub(crate) fn available_money(s: &CommandServices) -> i64 {
    let infinite = (s.infinite_money)();
    if infinite {
        return i64::MAX;
    }
    let finance = s.finance;
    let owner = (finance.company_finances)((finance.current_company)());
    // SAFETY: Native resolution returns null or the live canonical finance owner.
    unsafe { company::openttd_rust_company_available_money(owner, infinite) }
}

/// `cost` is a live native result; all native callbacks are synchronous.
/// # Safety
/// `cost` remains writable and exclusively accessed for the call.
pub(crate) unsafe fn check_money(
    s: &CommandServices,
    costs: &CommandCostServices,
    cost: *mut CommandCost,
) -> bool {
    let amount = (costs.money)(cost);
    if amount <= 0 || (s.infinite_money)() {
        return true;
    }
    let finance = s.finance;
    let owner = (finance.company_finances)((finance.current_company)());
    // SAFETY: Native resolution returns null or the live canonical finance owner.
    if unsafe { company::has_money(owner, false, amount) } {
        return true;
    }
    // SAFETY: The error formatter mutates the same live cost without reentry.
    unsafe { (s.money_error)(cost, amount) };
    false
}

/// Complete `InternalDoAfter`; read native result fields only on reached paths.
/// # Safety
/// `cost` remains writable and exclusively accessed for the call.
pub(crate) unsafe fn after(
    s: &CommandServices,
    costs: &CommandCostServices,
    cost: *mut CommandCost,
    flags: u16,
    top_level: bool,
    test: bool,
) {
    const QUERY_COST: u16 = 1 << 2;
    const BANKRUPT: u16 = 1 << 5;
    if test {
        crate::town::openttd_rust_town_rating_test(false);
        if (costs.success)(cost) && top_level && flags & (QUERY_COST | BANKRUPT) == 0 {
            // SAFETY: The caller owns this live native result.
            unsafe { check_money(s, costs, cost) };
        }
    } else if (costs.success)(cost) && top_level && flags & BANKRUPT == 0 {
        // SAFETY: The caller owns this live native result.
        unsafe { subtract_money(s, costs, cost) };
    }
}

/// `SubtractMoneyFromCompany`, preserving the valid-company then zero-cost order.
/// # Safety
/// `cost` remains live and exclusively accessed for the call.
pub(crate) unsafe fn subtract_money(
    s: &CommandServices,
    costs: &CommandCostServices,
    cost: *const CommandCost,
) {
    let finance = s.finance;
    let id = (finance.current_company)();
    let owner = (finance.company_finances)(id);
    if !owner.is_null() {
        let amount = (costs.money)(cost);
        if amount != 0 {
            let expense = (costs.expense)(cost).0;
            // SAFETY: The resolved finance owner stays live; subtraction ends
            // its field accesses before the invalidation callback.
            unsafe {
                company::subtract_money(finance, owner, id, amount, expense);
            };
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_command_before(s: &CommandServices, top_level: bool, test: bool) {
    before(s, top_level, test);
}
/// # Safety
/// `cost` points to a live writable native stack result throughout the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_after(
    s: &CommandServices,
    costs: &CommandCostServices,
    cost: *mut CommandCost,
    flags: u16,
    top_level: bool,
    test: bool,
) {
    // SAFETY: Forwarded caller-owned result.
    unsafe { after(s, costs, cost, flags, top_level, test) };
}
/// # Safety
/// `cost` points to a live writable native stack result throughout the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_check_money(
    s: &CommandServices,
    costs: &CommandCostServices,
    cost: *mut CommandCost,
) -> bool {
    // SAFETY: Forwarded caller-owned result.
    unsafe { check_money(s, costs, cost) }
}

/// # Safety
/// `cost` points to a live native stack result throughout the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_subtract_money(
    s: &CommandServices,
    costs: &CommandCostServices,
    cost: *const CommandCost,
) {
    // SAFETY: Forwarded caller-owned result.
    unsafe { subtract_money(s, costs, cost) };
}
