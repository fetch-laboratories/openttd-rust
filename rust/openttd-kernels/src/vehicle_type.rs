/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared `vehicle_type.h` transport identities.
/// `VEH_TRAIN`.
pub const VEH_TRAIN: u8 = VehicleType::VEH_TRAIN.0;
/// `VEH_ROAD`.
pub const VEH_ROAD: u8 = VehicleType::VEH_ROAD.0;
/// `VEH_SHIP`.
pub const VEH_SHIP: u8 = VehicleType::VEH_SHIP.0;
/// `VEH_AIRCRAFT`.
pub const VEH_AIRCRAFT: u8 = VehicleType::VEH_AIRCRAFT.0;
/// Acceleration due to gravity: 9.8 m/s² in native units.
pub const GROUND_ACCELERATION: u32 = 9800;
/// `AccelerationModel::AM_ORIGINAL`.
pub const AM_ORIGINAL: u8 = 0;

/// End of vehicle types buildable by companies.
pub const VEH_COMPANY_END: u8 = VehicleType::VEH_COMPANY_END.0;
/// Native `VEH_INVALID`.
pub const VEH_INVALID: u8 = VehicleType::VEH_INVALID.0;

/// A vehicle's shared record (`VehicleRef`'s target); live while its native shell lives.
pub use crate::vehicle_base::VehicleRecord as OpenTTDVehicle;

/// Native `VehicleType`, including unnamed uint8 values.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VehicleType(pub u8);

impl VehicleType {
    /// Native `VEH_TRAIN`.
    pub const TRAIN: Self = Self::VEH_TRAIN;
    /// Native `VEH_ROAD`.
    pub const ROAD: Self = Self::VEH_ROAD;
    /// Native `VEH_SHIP`.
    pub const SHIP: Self = Self::VEH_SHIP;
    /// Native `VEH_AIRCRAFT`.
    pub const AIRCRAFT: Self = Self::VEH_AIRCRAFT;
    /// Native `VEH_COMPANY_END`.
    pub const COMPANY_END: Self = Self::VEH_COMPANY_END;
    /// Native `VEH_EFFECT`.
    pub const EFFECT: Self = Self::VEH_EFFECT;
    /// Native `VEH_DISASTER`.
    pub const DISASTER: Self = Self::VEH_DISASTER;
}

/// Native `VehicleID` with its full original integer domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VehicleID(pub u32);
impl VehicleID {
    /// Native invalid sentinel.
    pub const INVALID: Self = Self(0xFFFFF);
}

impl VehicleType {
    /// Native `VEH_BEGIN`.
    pub const VEH_BEGIN: Self = Self(0);
    /// %Train vehicle type.
    pub const VEH_TRAIN: Self = Self(0);
    /// Road vehicle type.
    pub const VEH_ROAD: Self = Self(1);
    /// %Ship vehicle type.
    pub const VEH_SHIP: Self = Self(2);
    /// %Aircraft vehicle type.
    pub const VEH_AIRCRAFT: Self = Self(3);
    /// Last company-ownable type.
    pub const VEH_COMPANY_END: Self = Self(4);
    /// Effect vehicle type (smoke, explosions, sparks, bubbles)
    pub const VEH_EFFECT: Self = Self(4);
    /// Disaster vehicle type.
    pub const VEH_DISASTER: Self = Self(5);
    /// Native `VEH_END`.
    pub const VEH_END: Self = Self(6);
    /// Non-existing type of vehicle.
    pub const VEH_INVALID: Self = Self(255);
}

/// Native `DepotCommandFlag`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DepotCommandFlag(pub u8);

#[allow(non_upper_case_globals)]
impl DepotCommandFlag {
    /// The vehicle will leave the depot right after arrival (service only)
    pub const Service: Self = Self(0);
    /// Tells that it's a mass send to depot command (type in VLW flag)
    pub const MassSend: Self = Self(1);
    /// Don't cancel current goto depot command if any
    pub const DontCancel: Self = Self(2);
}

/// Native `VehicleRandomTrigger`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VehicleRandomTrigger(pub u8);

#[allow(non_upper_case_globals)]
impl VehicleRandomTrigger {
    /// Affected vehicle only: Vehicle is loaded with cargo, after it was empty.
    pub const NewCargo: Self = Self(0);
    /// Front vehicle only: Consist arrived in depot.
    pub const Depot: Self = Self(1);
    /// Front vehicle only: Entire consist is empty.
    pub const Empty: Self = Self(2);
    /// All vehicles in consist: Any vehicle in the consist received new cargo.
    pub const AnyNewCargo: Self = Self(3);
    /// All vehicles in consist: 32 day callback requested rerandomisation
    pub const Callback32: Self = Self(4);
}

/// Native `DepotCommandFlags`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DepotCommandFlags(pub u8);

impl DepotCommandFlags {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Set containing the named element (the element is a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: DepotCommandFlag) -> Self {
        Self((1_u64 << element.0) as u8)
    }
}

/// Native `VehicleRandomTriggers`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VehicleRandomTriggers(pub u8);

impl VehicleRandomTriggers {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Set containing the named element (the element is a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: VehicleRandomTrigger) -> Self {
        Self((1_u64 << element.0) as u8)
    }
}

impl VehicleID {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(1_044_480);
}

impl DepotCommandFlags {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: DepotCommandFlag) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl VehicleRandomTriggers {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: VehicleRandomTrigger) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

crate::base_bitset_type::impl_bitset_ops!(DepotCommandFlags);
crate::base_bitset_type::impl_bitset_ops!(VehicleRandomTriggers);
