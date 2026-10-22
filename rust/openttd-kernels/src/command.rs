/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Ordinary command pre/post control, shared by native and Rust command owners.
#![allow(unsafe_code)]
use crate::command_type::DoCommandFlag;
use crate::services::{CommandCost, CommandCostServices, CommandServices};

pub(crate) fn before(_s: &CommandServices, top_level: bool, test: bool) {
    if top_level {
        crate::objects::clear_areas();
    }
    if test {
        crate::town::openttd_rust_town_rating_test(true);
    }
}

pub(crate) fn available_money(s: &CommandServices) -> i64 {
    if (s.infinite_money)() {
        return i64::MAX;
    }
    let id = (s.finance.current_company)();
    let owner = (s.finance.company_finances)(id);
    // SAFETY: the canonical resolver returns null or the live finance owner.
    unsafe { crate::company::openttd_rust_company_available_money(owner, false) }
}

/// # Safety
/// `result` is a live native result, exclusively mutated for this call.
pub(crate) unsafe fn check_money(
    s: &CommandServices,
    c: &CommandCostServices,
    result: *mut CommandCost,
) -> bool {
    let amount = (c.money)(result);
    if amount <= 0 || (s.infinite_money)() {
        return true;
    }
    let owner = (s.finance.company_finances)((s.finance.current_company)());
    // SAFETY: the canonical resolver returns null or a live finance owner.
    if unsafe { crate::company::has_money(owner, false, amount) } {
        return true;
    }
    // SAFETY: the caller supplies exclusive live native result storage.
    unsafe {
        (s.money_error)(result, amount);
    }
    false
}

/// # Safety
/// `result` is live caller-owned native command storage for the whole call.
pub(crate) unsafe fn after(
    s: &CommandServices,
    c: &CommandCostServices,
    result: *mut CommandCost,
    flags: u16,
    top_level: bool,
    test: bool,
) {
    if test {
        crate::town::openttd_rust_town_rating_test(false);
        if (c.success)(result)
            && top_level
            && flags & (DoCommandFlag::QUERY_COST | DoCommandFlag::BANKRUPT) == 0
        {
            // SAFETY: caller guarantees live exclusive storage.
            unsafe {
                check_money(s, c, result);
            }
        }
    } else if (c.success)(result) && top_level && flags & DoCommandFlag::BANKRUPT == 0 {
        // SAFETY: caller guarantees live native result storage.
        unsafe {
            subtract_money(s, c, result);
        }
    }
}

/// # Safety
/// `result` is a live native result for this call.
pub(crate) unsafe fn subtract_money(
    s: &CommandServices,
    c: &CommandCostServices,
    result: *const CommandCost,
) {
    let id = (s.finance.current_company)();
    let owner = (s.finance.company_finances)(id);
    if owner.is_null() {
        return;
    }
    let amount = (c.money)(result);
    if amount == 0 {
        return;
    }
    let expense = (c.expense)(result);
    // SAFETY: resolver returned the live owner; no borrow crosses invalidation.
    unsafe {
        crate::company::subtract_money(s.finance, owner, id, amount, expense.0);
    }
}

/// # Safety
/// `result` is a live native result for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_subtract_money(
    s: &CommandServices,
    c: &CommandCostServices,
    result: *const CommandCost,
) {
    unsafe {
        subtract_money(s, c, result);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_command_before(s: &CommandServices, top_level: bool, test: bool) {
    before(s, top_level, test);
}
/// # Safety
/// `result` is a live exclusive native command result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_after(
    s: &CommandServices,
    c: &CommandCostServices,
    result: *mut CommandCost,
    flags: u16,
    top_level: bool,
    test: bool,
) {
    unsafe {
        after(s, c, result, flags, top_level, test);
    }
}
/// # Safety
/// `result` is a live exclusive native command result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_check_money(
    s: &CommandServices,
    c: &CommandCostServices,
    result: *mut CommandCost,
) -> bool {
    unsafe { check_money(s, c, result) }
}

/// Ordinary `CommandHelper::Do` lifecycle around a Rust-owned command body.
/// Native callbacks operate on caller-owned stack results and serialized state.
pub(crate) fn nested<T: Default>(
    s: &CommandServices,
    shared: &crate::services::Services,
    out: crate::services::Cost<'_>,
    tile: u32,
    flags: u16,
    mut body: impl FnMut(u16) -> T,
) -> T {
    struct Guard<'a>(&'a CommandServices);
    impl Drop for Guard<'_> {
        fn drop(&mut self) {
            (self.0.leave)();
        }
    }
    let size = ((shared.map_max_y)() + 1) << (shared.map_log_x)();
    if tile != 0 && (tile >= size || (!(shared.tile_valid)(tile) && flags & 256 == 0)) {
        out.error(0xffff);
        return T::default();
    }
    let top = (s.enter)();
    let _guard = Guard(s);
    if top || flags & 1 == 0 {
        before(s, top, true);
        let extra = body(flags & !1);
        // SAFETY: the command caller supplies live exclusive native result storage.
        unsafe {
            after(s, out.1, out.0, flags, top, true);
        }
        if !out.ok() || flags & 1 == 0 {
            return extra;
        }
    }
    before(s, top, false);
    let extra = body(flags);
    // SAFETY: the command caller supplies live exclusive native result storage.
    unsafe {
        after(s, out.1, out.0, flags, top, false);
    }
    extra
}
