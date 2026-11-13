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
pub const ACCELERATION_NORMAL: u8 = VehicleAccelerationModel::Normal.0;
/// `VehicleAccelerationModel::Maglev`.
pub const ACCELERATION_MAGLEV: u8 = VehicleAccelerationModel::Maglev.0;
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
pub const EC_STEAM: u8 = EngineClass::EC_STEAM.0;
/// `EC_MONORAIL`.
pub const EC_MONORAIL: u8 = EngineClass::EC_MONORAIL.0;

/// `VE_OFFSET_COUNT`.
pub const VE_OFFSET_COUNT: u8 = 4;
/// `VE_TYPE_COUNT`.
pub const VE_TYPE_COUNT: u8 = 2;

/// `EngineMiscFlag::NoDefaultCargoMultiplier`.
pub const NO_DEFAULT_CARGO_MULTIPLIER: u8 =
    EngineMiscFlags::from_element(EngineMiscFlag::NoDefaultCargoMultiplier).0;

/// `AIR_HELI`.
pub const AIR_HELI: u8 = 0;
/// `EC_DIESEL`.
pub const EC_DIESEL: u8 = EngineClass::EC_DIESEL.0;
/// `EC_ELECTRIC`.
pub const EC_ELECTRIC: u8 = EngineClass::EC_ELECTRIC.0;
/// `EC_MAGLEV`.
pub const EC_MAGLEV: u8 = EngineClass::EC_MAGLEV.0;
/// `EngineMiscFlag::Uses2CC`.
pub const USES_2CC: u8 = EngineMiscFlags::from_element(EngineMiscFlag::Uses2CC).0;

/// Canonical engine record; native shell identity is kept only for remaining native consumers.
pub use crate::engine::Lifecycle as OpenTTDEngine;

/// Native identity and formatting shell, released with its last native consumer.
#[repr(C)]
pub struct NativeEngine {
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

/// `EngineMiscFlag::RailIsMU`.
pub const RAIL_IS_MU: u8 = EngineMiscFlags::from_element(EngineMiscFlag::RailIsMU).0;
/// `EngineMiscFlag::RoadIsTram`.
pub const ROAD_IS_TRAM: u8 = EngineMiscFlags::from_element(EngineMiscFlag::RoadIsTram).0;
/// `AIR_FAST`.
pub const AIR_FAST: u8 = 2;

/// `EngineMiscFlag::AutoRefit`.
pub const AUTO_REFIT: u8 = EngineMiscFlags::from_element(EngineMiscFlag::AutoRefit).0;

/// `EngineMiscFlag::RailTilts`.
pub const RAIL_TILTS: u8 = EngineMiscFlags::from_element(EngineMiscFlag::RailTilts).0;

/// `EngineMiscFlag::NoBreakdownSmoke`.
pub const NO_BREAKDOWN_SMOKE: u8 =
    EngineMiscFlags::from_element(EngineMiscFlag::NoBreakdownSmoke).0;

/// Native `RailVehicleTypes : uint8_t`, with the same unrestricted byte domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RailVehicleTypes(pub u8);

impl RailVehicleTypes {
    /// Native `RAILVEH_SINGLEHEAD`.
    pub const SINGLEHEAD: Self = Self::RAILVEH_SINGLEHEAD;
    /// Native `RAILVEH_MULTIHEAD`.
    pub const MULTIHEAD: Self = Self::RAILVEH_MULTIHEAD;
    /// Native `RAILVEH_WAGON`.
    pub const WAGON: Self = Self::RAILVEH_WAGON;
}

impl RailVehicleTypes {
    /// indicates a "standalone" locomotive
    pub const RAILVEH_SINGLEHEAD: Self = Self(0);
    /// indicates a combination of two locomotives
    pub const RAILVEH_MULTIHEAD: Self = Self(1);
    /// simple wagon, not motorized
    pub const RAILVEH_WAGON: Self = Self(2);
}

/// Native `EngineClass`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EngineClass(pub u8);

impl EngineClass {
    /// Steam rail engine.
    pub const EC_STEAM: Self = Self(0);
    /// Diesel rail engine.
    pub const EC_DIESEL: Self = Self(1);
    /// Electric rail engine.
    pub const EC_ELECTRIC: Self = Self(2);
    /// Mono rail engine.
    pub const EC_MONORAIL: Self = Self(3);
    /// Maglev engine.
    pub const EC_MAGLEV: Self = Self(4);
}

/// Native `VehicleAccelerationModel`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VehicleAccelerationModel(pub u8);

#[allow(non_upper_case_globals)]
impl VehicleAccelerationModel {
    /// Default acceleration model.
    pub const Normal: Self = Self(0);
    /// Monorail acceleration model.
    pub const Monorail: Self = Self(1);
    /// Maglev acceleration model.
    pub const Maglev: Self = Self(2);
}

/// Native `ExtraEngineFlag`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExtraEngineFlag(pub u8);

#[allow(non_upper_case_globals)]
impl ExtraEngineFlag {
    /// No 'new vehicle' news will be generated.
    pub const NoNews: Self = Self(0);
    /// No exclusive preview will be offered.
    pub const NoPreview: Self = Self(1);
    /// Engine will join exclusive preview with variant parent.
    pub const JoinPreview: Self = Self(2);
    /// Engine reliability will be synced with variant parent.
    pub const SyncReliability: Self = Self(3);
}

/// Native `EngineMiscFlag`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EngineMiscFlag(pub u8);

#[allow(non_upper_case_globals)]
impl EngineMiscFlag {
    /// Rail vehicle tilts in curves
    pub const RailTilts: Self = Self(0);
    /// Road vehicle is a tram/light rail vehicle
    pub const RoadIsTram: Self = Self(0);
    /// Vehicle uses two company colours
    pub const Uses2CC: Self = Self(1);
    /// Rail vehicle is a multiple-unit (DMU/EMU)
    pub const RailIsMU: Self = Self(2);
    /// Rail vehicle has old depot-flip handling
    pub const RailFlips: Self = Self(3);
    /// Automatic refitting is allowed
    pub const AutoRefit: Self = Self(4);
    /// Use the new capacity algorithm. The default cargotype of the vehicle does not affect capacity multipliers. CB 15 is also called in purchase list.
    pub const NoDefaultCargoMultiplier: Self = Self(5);
    /// Do not show black smoke during a breakdown.
    pub const NoBreakdownSmoke: Self = Self(6);
    /// Draw vehicle by stacking multiple sprites.
    pub const SpriteStack: Self = Self(7);
}

/// Native `EngineFlag`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EngineFlag(pub u8);

#[allow(non_upper_case_globals)]
impl EngineFlag {
    /// This vehicle is available to everyone.
    pub const Available: Self = Self(0);
    /// This vehicle is in the exclusive preview stage, either being used or being offered to a company.
    pub const ExclusivePreview: Self = Self(1);
}

/// Native `ExtraEngineFlags`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExtraEngineFlags(pub u8);

impl ExtraEngineFlags {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Set containing the named element (the element is a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: ExtraEngineFlag) -> Self {
        Self((1_u64 << element.0) as u8)
    }
}

/// Native `EngineMiscFlags`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EngineMiscFlags(pub u8);

impl EngineMiscFlags {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Set containing the named element (the element is a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: EngineMiscFlag) -> Self {
        Self((1_u64 << element.0) as u8)
    }
}

/// Native `EngineFlags`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EngineFlags(pub u8);

impl EngineFlags {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Set containing the named element (the element is a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: EngineFlag) -> Self {
        Self((1_u64 << element.0) as u8)
    }
}

impl EngineID {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(64000);
}

impl ExtraEngineFlags {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: ExtraEngineFlag) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl EngineMiscFlags {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: EngineMiscFlag) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl EngineFlags {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: EngineFlag) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

crate::base_bitset_type::impl_bitset_ops!(ExtraEngineFlags);
crate::base_bitset_type::impl_bitset_ops!(EngineMiscFlags);
crate::base_bitset_type::impl_bitset_ops!(EngineFlags);

/// The maximum length of an engine name in characters including '\0'.
pub const MAX_LENGTH_ENGINE_NAME_CHARS: u32 = 32;

/// `ShipVehicleInfo::ApplyWaterClassSpeedFrac`: apply ocean/canal speed fraction
/// to a velocity, given the info's `ocean_speed_frac` and `canal_speed_frac`.
pub(crate) const fn apply_water_class_speed_frac(
    raw_speed: u32,
    is_ocean: bool,
    ocean_speed_frac: u8,
    canal_speed_frac: u8,
) -> u32 {
    /* speed_frac == 0 means no reduction while 0xFF means reduction to 1/256. */
    let speed_frac = if is_ocean {
        ocean_speed_frac
    } else {
        canal_speed_frac
    };
    raw_speed.wrapping_mul(256 - speed_frac as u32) / 256
}
