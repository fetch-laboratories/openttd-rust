/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Canonical native invariant check, following the native build assertion policy.
#![allow(unsafe_code)]

use std::sync::atomic::{AtomicPtr, Ordering};

extern "C" fn unbound(_: bool) {
    std::process::abort();
}
static CHECK: AtomicPtr<()> = AtomicPtr::new(unbound as *mut ());

/// Bind `RustAssertCondition` during native startup, before any native assertion call.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_assert_bind(check: extern "C" fn(bool)) {
    CHECK.store(check as *mut (), Ordering::Release);
}

/// `RustAssertCondition`; this leaf does not mutate Rust state.
pub extern "C" fn check(condition: bool) {
    // SAFETY: Both the initial function and every bound function have this signature.
    let check: extern "C" fn(bool) = unsafe { std::mem::transmute(CHECK.load(Ordering::Acquire)) };
    check(condition);
}
