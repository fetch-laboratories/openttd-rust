/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `base_consist.h` constants shared by simulation owners.

/// `VehicleFlag::LoadingFinished`.
pub const VF_LOADING_FINISHED: u16 = 1 << 0;
/// `VehicleFlag::StopLoading`.
pub const VF_STOP_LOADING: u16 = 1 << 6;

/// `VehicleFlag::CargoUnloading`.
pub const VF_CARGO_UNLOADING: u16 = 1 << 1;
/// `VehicleFlag::BuiltAsPrototype`.
pub const VF_BUILT_AS_PROTOTYPE: u16 = 1 << 2;
