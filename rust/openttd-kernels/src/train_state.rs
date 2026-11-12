/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Canonical train-private state, owned by one native shell allocation.
//! Accesses are serialized and field-sized; no borrow crosses a callback.
#![allow(unsafe_code)]

use crate::rail_type::RailTypes;
use crate::track_type::TrackBits;
use crate::vehicle_base::VehicleRecord;

crate::abi::shared_records! {
    fn abi_record;
    /// Train-private fields with the original widths and zero defaults.
    #[derive(Default)]
    "OpenTTDTrainState" ["train_state"] => pub struct TrainState {
        /// `Train::flags`, a `VehicleRailFlags` set.
        pub flags: u16,
        /// Crash animation counter.
        pub crash_anim_pos: u16,
        /// Ticks waiting in front of a signal, ticks being stuck or a counter for forced
        /// proceeding through signals.
        pub wait_counter: u16,
        /// `Train::compatible_railtypes`.
        pub compatible_railtypes: RailTypes,
        /// `Train::railtypes`.
        pub railtypes: RailTypes,
        /// `Train::track`.
        pub track: TrackBits,
        /// `Train::force_proceed`, a `TrainForceProceeding` mode.
        pub force_proceed: u8,
        /// `TrainCache::cached_tilt`: train can tilt; feature provides a bonus in curves
        pub cached_tilt: bool,
        /// `TrainCache::user_def_data`: Cached property 0x25. Can be set by Callback 0x36.
        pub user_def_data: u8,
        /// `TrainCache::cached_curve_speed_mod`: curve speed modifier of the entire train
        pub cached_curve_speed_mod: i16,
        /// `TrainCache::cached_max_curve_speed`: max consist speed limited by curves
        pub cached_max_curve_speed: u16,
        /// NOSAVE link, rebuilt by the original sequential/bracket matching phase.
        pub other_multiheaded_part: *mut VehicleRecord,
    }
}
/// The canonical owner type the train shell holds.
pub type State = TrainState;

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
