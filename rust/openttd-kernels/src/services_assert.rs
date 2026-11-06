/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Canonical native invariant check, following the native build assertion policy.
#![allow(unsafe_code)]

unsafe extern "C" {
    /// `RustAssertCondition` from `services_assert.h`; it does not mutate Rust state.
    pub safe fn RustAssertCondition(condition: bool);
}
