/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Canonical train-private state, owned by one C++ vehicle shell allocation.
//! Accessors copy scalars and retain no Rust reference across shared services.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

/// Train-private scalars, with exactly the original native widths.
#[derive(Default)]
pub struct State {
    pub(crate) flags: u16,
    pub(crate) crash_anim_pos: u16,
    pub(crate) wait_counter: u16,
    pub(crate) compatible_railtypes: u64,
    pub(crate) railtypes: u64,
    pub(crate) track: u8,
    pub(crate) force_proceed: u8,
    pub(crate) cached_tilt: bool,
    pub(crate) user_def_data: u8,
    pub(crate) cached_curve_speed_mod: i16,
    pub(crate) cached_max_curve_speed: u16,
}

/// Allocate one zero-created canonical owner for a vehicle shell.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_train_state_new() -> *mut State {
    Box::into_raw(Box::default())
}

/// Destroy the shell's canonical owner after its pre-destructor.
///
/// # Safety
/// `state` must be a live allocation from `openttd_rust_train_state_new`, destroyed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_state_destroy(state: *mut State) {
    // SAFETY: The owning shell transfers its allocation exactly once after PreDestructor.
    unsafe {
        drop(Box::from_raw(state));
    }
}

/// Read the shell's canonical `flags` scalar.
/// # Safety
/// The serialized shell owns a live state; no reference escapes this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_state_get_flags(state: *const State) -> u16 {
    // SAFETY: Caller supplies a live owner and this scalar read ends before return.
    unsafe { (*state).flags }
}
/// Write the shell's canonical `flags` scalar.
/// # Safety
/// The serialized shell owns this exclusively accessible live state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_state_set_flags(state: *mut State, value: u16) {
    // SAFETY: Exclusive scalar access, retaining no state borrow.
    unsafe {
        (*state).flags = value;
    }
}
/// Read the shell's canonical `crash_anim_pos` scalar.
/// # Safety
/// The serialized shell owns a live state; no reference escapes this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_state_get_crash_anim_pos(state: *const State) -> u16 {
    // SAFETY: Caller supplies a live owner and this scalar read ends before return.
    unsafe { (*state).crash_anim_pos }
}
/// Write the shell's canonical `crash_anim_pos` scalar.
/// # Safety
/// The serialized shell owns this exclusively accessible live state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_state_set_crash_anim_pos(
    state: *mut State,
    value: u16,
) {
    // SAFETY: Exclusive scalar access, retaining no state borrow.
    unsafe {
        (*state).crash_anim_pos = value;
    }
}
/// Read the shell's canonical `wait_counter` scalar.
/// # Safety
/// The serialized shell owns a live state; no reference escapes this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_state_get_wait_counter(state: *const State) -> u16 {
    // SAFETY: Caller supplies a live owner and this scalar read ends before return.
    unsafe { (*state).wait_counter }
}
/// Write the shell's canonical `wait_counter` scalar.
/// # Safety
/// The serialized shell owns this exclusively accessible live state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_state_set_wait_counter(state: *mut State, value: u16) {
    // SAFETY: Exclusive scalar access, retaining no state borrow.
    unsafe {
        (*state).wait_counter = value;
    }
}
/// Read the shell's canonical `compatible_railtypes` scalar.
/// # Safety
/// The serialized shell owns a live state; no reference escapes this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_state_get_compatible_railtypes(
    state: *const State,
) -> u64 {
    // SAFETY: Caller supplies a live owner and this scalar read ends before return.
    unsafe { (*state).compatible_railtypes }
}
/// Write the shell's canonical `compatible_railtypes` scalar.
/// # Safety
/// The serialized shell owns this exclusively accessible live state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_state_set_compatible_railtypes(
    state: *mut State,
    value: u64,
) {
    // SAFETY: Exclusive scalar access, retaining no state borrow.
    unsafe {
        (*state).compatible_railtypes = value;
    }
}
/// Read the shell's canonical `railtypes` scalar.
/// # Safety
/// The serialized shell owns a live state; no reference escapes this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_state_get_railtypes(state: *const State) -> u64 {
    // SAFETY: Caller supplies a live owner and this scalar read ends before return.
    unsafe { (*state).railtypes }
}
/// Write the shell's canonical `railtypes` scalar.
/// # Safety
/// The serialized shell owns this exclusively accessible live state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_state_set_railtypes(state: *mut State, value: u64) {
    // SAFETY: Exclusive scalar access, retaining no state borrow.
    unsafe {
        (*state).railtypes = value;
    }
}
/// Read the shell's canonical `track` scalar.
/// # Safety
/// The serialized shell owns a live state; no reference escapes this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_state_get_track(state: *const State) -> u8 {
    // SAFETY: Caller supplies a live owner and this scalar read ends before return.
    unsafe { (*state).track }
}
/// Write the shell's canonical `track` scalar.
/// # Safety
/// The serialized shell owns this exclusively accessible live state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_state_set_track(state: *mut State, value: u8) {
    // SAFETY: Exclusive scalar access, retaining no state borrow.
    unsafe {
        (*state).track = value;
    }
}
/// Read the shell's canonical `force_proceed` scalar.
/// # Safety
/// The serialized shell owns a live state; no reference escapes this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_state_get_force_proceed(state: *const State) -> u8 {
    // SAFETY: Caller supplies a live owner and this scalar read ends before return.
    unsafe { (*state).force_proceed }
}
/// Write the shell's canonical `force_proceed` scalar.
/// # Safety
/// The serialized shell owns this exclusively accessible live state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_state_set_force_proceed(state: *mut State, value: u8) {
    // SAFETY: Exclusive scalar access, retaining no state borrow.
    unsafe {
        (*state).force_proceed = value;
    }
}
/// Read the shell's canonical `cached_tilt` scalar.
/// # Safety
/// The serialized shell owns a live state; no reference escapes this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_state_get_cached_tilt(state: *const State) -> u8 {
    // SAFETY: Caller supplies a live owner and this scalar read ends before return.
    unsafe { u8::from((*state).cached_tilt) }
}
/// Write the shell's canonical `cached_tilt` scalar.
/// # Safety
/// The serialized shell owns this exclusively accessible live state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_state_set_cached_tilt(state: *mut State, value: u8) {
    // SAFETY: Exclusive scalar access, retaining no state borrow.
    unsafe {
        (*state).cached_tilt = value != 0;
    }
}
/// Read the shell's canonical `user_def_data` scalar.
/// # Safety
/// The serialized shell owns a live state; no reference escapes this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_state_get_user_def_data(state: *const State) -> u8 {
    // SAFETY: Caller supplies a live owner and this scalar read ends before return.
    unsafe { (*state).user_def_data }
}
/// Read the shell's canonical `cached_curve_speed_mod` scalar.
/// # Safety
/// The serialized shell owns a live state; no reference escapes this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_state_get_cached_curve_speed_mod(
    state: *const State,
) -> i16 {
    // SAFETY: Caller supplies a live owner and this scalar read ends before return.
    unsafe { (*state).cached_curve_speed_mod }
}
/// Write the shell's canonical `cached_curve_speed_mod` scalar.
/// # Safety
/// The serialized shell owns this exclusively accessible live state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_state_set_cached_curve_speed_mod(
    state: *mut State,
    value: i16,
) {
    // SAFETY: Exclusive scalar access, retaining no state borrow.
    unsafe {
        (*state).cached_curve_speed_mod = value;
    }
}
/// Read the shell's canonical `cached_max_curve_speed` scalar.
/// # Safety
/// The serialized shell owns a live state; no reference escapes this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_state_get_cached_max_curve_speed(
    state: *const State,
) -> u16 {
    // SAFETY: Caller supplies a live owner and this scalar read ends before return.
    unsafe { (*state).cached_max_curve_speed }
}
/// Write the shell's canonical `cached_max_curve_speed` scalar.
/// # Safety
/// The serialized shell owns this exclusively accessible live state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_state_set_cached_max_curve_speed(
    state: *mut State,
    value: u16,
) {
    // SAFETY: Exclusive scalar access, retaining no state borrow.
    unsafe {
        (*state).cached_max_curve_speed = value;
    }
}
