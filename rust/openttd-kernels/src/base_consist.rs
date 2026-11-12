/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `base_consist.h` values shared by simulation owners.

/// Native `VehicleFlag`: Bit numbers in `Vehicle::vehicle_flags`. The values Rust names.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VehicleFlag(pub u8);
impl VehicleFlag {
    /// Vehicle has finished loading.
    pub const LOADING_FINISHED: Self = Self(0);
    /// Vehicle is unloading cargo.
    pub const CARGO_UNLOADING: Self = Self(1);
    /// Vehicle is a prototype (accepted as exclusive preview).
    pub const BUILT_AS_PROTOTYPE: Self = Self(2);
    /// Whether the vehicle has started running on the timetable yet.
    pub const TIMETABLE_STARTED: Self = Self(3);
    /// Don't load anymore during the next load cycle.
    pub const STOP_LOADING: Self = Self(6);
}

/// `VehicleFlags` mask of `VehicleFlag::LoadingFinished`.
pub const VF_LOADING_FINISHED: u16 = 1 << VehicleFlag::LOADING_FINISHED.0;
/// `VehicleFlags` mask of `VehicleFlag::CargoUnloading`.
pub const VF_CARGO_UNLOADING: u16 = 1 << VehicleFlag::CARGO_UNLOADING.0;
/// `VehicleFlags` mask of `VehicleFlag::BuiltAsPrototype`.
pub const VF_BUILT_AS_PROTOTYPE: u16 = 1 << VehicleFlag::BUILT_AS_PROTOTYPE.0;
/// `VehicleFlags` mask of `VehicleFlag::TimetableStarted`.
pub const VF_TIMETABLE_STARTED: u16 = 1 << VehicleFlag::TIMETABLE_STARTED.0;
/// `VehicleFlags` mask of `VehicleFlag::StopLoading`.
pub const VF_STOP_LOADING: u16 = 1 << VehicleFlag::STOP_LOADING.0;
