/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `engine_type.h` values. `services.cpp` asserts every value.

/// `EngineID::Invalid()`.
pub const INVALID_ENGINE: u16 = EngineID::INVALID.0;
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

/// `AIR_HELI`.
pub const AIR_HELI: u8 = 0;
/// `EC_DIESEL`.
pub const EC_DIESEL: u8 = 1;
/// `EC_ELECTRIC`.
pub const EC_ELECTRIC: u8 = 2;
/// `EC_MAGLEV`.
pub const EC_MAGLEV: u8 = 4;
/// `EngineMiscFlag::Uses2CC`.
pub const USES_2CC: u8 = 1 << 1;

/// Opaque native `Engine`; valid only while its native pool shell lives.
#[repr(C)]
pub struct OpenTTDEngine {
    _private: [u8; 0],
}

/// Native `EngineID` with its full original integer domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EngineID(pub u16);
impl EngineID {
    /// Native invalid sentinel.
    pub const INVALID: Self = Self(u16::MAX);
}

/// EngineMiscFlag::RailIsMU.
pub const RAIL_IS_MU: u8 = 1 << 2;
/// EngineMiscFlag::RoadIsTram.
pub const ROAD_IS_TRAM: u8 = 1;
/// AIR_FAST.
pub const AIR_FAST: u8 = 2;
