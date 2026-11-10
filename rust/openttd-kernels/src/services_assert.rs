/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Canonical native invariant check, following the native build assertion policy.
#![allow(unsafe_code)]

use std::sync::atomic::{AtomicPtr, Ordering};

extern "C" fn unbound(_: bool) {
    std::process::abort();
}
static SHARED: AtomicPtr<crate::services::Services> = AtomicPtr::new(std::ptr::null_mut());
static CHECK: AtomicPtr<()> = AtomicPtr::new(unbound as *mut ());

/// Bind the native invariant policy before game/tool entries. A null shared
/// pointer replaces only the assertion callback, preserving the fatal binding.
///
/// # Safety
/// A non-null `shared` points to the canonical process-lifetime shared table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_assert_bind(
    check: extern "C" fn(bool),
    shared: *const crate::services::Services,
) {
    if !shared.is_null() {
        SHARED.store(shared.cast_mut(), Ordering::Release);
    }
    CHECK.store(check as *mut (), Ordering::Release);
}

/// `RustAssertCondition`; this leaf does not mutate Rust state.
pub extern "C" fn check(condition: bool) {
    // SAFETY: Both the initial function and every bound function have this signature.
    let check: extern "C" fn(bool) = unsafe { std::mem::transmute(CHECK.load(Ordering::Acquire)) };
    check(condition);
}

/// The canonical native `NOT_REACHED` operation, independent of assertion policy.
pub(crate) fn not_reached() -> ! {
    let shared = SHARED.load(Ordering::Acquire);
    if !shared.is_null() {
        // SAFETY: Installed only by the startup binding's process-lifetime contract.
        (unsafe { &*shared }.not_reached)();
    }
    std::process::abort()
}

/// Replace only the assertion callback for a Rust test; retain fatal dispatch.
#[cfg(test)]
pub(crate) fn bind_test_check(check: extern "C" fn(bool)) {
    // SAFETY: A null table changes no pointer lifetime contract.
    unsafe {
        openttd_rust_assert_bind(check, std::ptr::null());
    }
}
