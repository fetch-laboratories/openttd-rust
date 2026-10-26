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
/// `VE_OFFSET_START`.
pub const VE_OFFSET_START: u8 = 0;
/// `VE_OFFSET_CENTRE`.
pub const VE_OFFSET_CENTRE: u8 = 8;
/// `VE_TYPE_START`.
pub const VE_TYPE_START: u8 = 4;
/// `VE_TYPE_DEFAULT`.
pub const VE_TYPE_DEFAULT: u8 = 0;
/// `VE_TYPE_STEAM`.
pub const VE_TYPE_STEAM: u8 = 1;
/// `VE_DISABLE_EFFECT`.
pub const VE_DISABLE_EFFECT: u8 = 6;
/// `VE_ADVANCED_EFFECT`.
pub const VE_ADVANCED_EFFECT: u8 = 6;
/// `VE_DISABLE_WAGON_POWER`.
pub const VE_DISABLE_WAGON_POWER: u8 = 7;
/// `VE_DEFAULT`.
pub const VE_DEFAULT: u8 = 255;
/// `EC_STEAM`.
pub const EC_STEAM: u8 = 0;
/// `EC_MONORAIL`.
pub const EC_MONORAIL: u8 = 3;

/// `VE_OFFSET_COUNT`.
pub const VE_OFFSET_COUNT: u8 = 4;
/// `VE_TYPE_COUNT`.
pub const VE_TYPE_COUNT: u8 = 2;

/// `EngineMiscFlag::NoDefaultCargoMultiplier`.
pub const NO_DEFAULT_CARGO_MULTIPLIER: u8 = 1 << 5;
