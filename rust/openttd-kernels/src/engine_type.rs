/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `engine_type.h` values. `services.cpp` asserts every value.

/// `EngineID::Invalid()`.
pub const INVALID_ENGINE: u16 = 0xFFFF;
/// `AIR_CTOL`.
pub const AIR_CTOL: u8 = 1;

/// `VehicleAccelerationModel::Normal`.
pub const ACCELERATION_NORMAL: u8 = 0;
/// `VehicleAccelerationModel::Maglev`.
pub const ACCELERATION_MAGLEV: u8 = 2;
