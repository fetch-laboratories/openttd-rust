/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared `vehicle_type.h` transport identities.
/// `VEH_TRAIN`.
pub const VEH_TRAIN: u8 = VehicleType::TRAIN.0;
/// `VEH_ROAD`.
pub const VEH_ROAD: u8 = VehicleType::ROAD.0;
/// `VEH_SHIP`.
pub const VEH_SHIP: u8 = VehicleType::SHIP.0;
/// `VEH_AIRCRAFT`.
pub const VEH_AIRCRAFT: u8 = VehicleType::AIRCRAFT.0;
pub const GROUND_ACCELERATION: u32 = 9800;
/// `AccelerationModel::AM_ORIGINAL`.
pub const AM_ORIGINAL: u8 = 0;

/// End of vehicle types buildable by companies.
pub const VEH_COMPANY_END: u8 = VehicleType::COMPANY_END.0;
/// Native `VEH_INVALID`.
pub const VEH_INVALID: u8 = u8::MAX;

/// A vehicle's shared record (`VehicleRef`'s target); live while its native shell lives.
pub use crate::vehicle_base::VehicleRecord as OpenTTDVehicle;

/// Native `VehicleType`, including unnamed uint8 values.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct VehicleType(pub u8);

impl VehicleType {
    /// Native `VEH_TRAIN`.
    pub const TRAIN: Self = Self(0);
    /// Native `VEH_ROAD`.
    pub const ROAD: Self = Self(1);
    /// Native `VEH_SHIP`.
    pub const SHIP: Self = Self(2);
    /// Native `VEH_AIRCRAFT`.
    pub const AIRCRAFT: Self = Self(3);
    /// Native `VEH_COMPANY_END`.
    pub const COMPANY_END: Self = Self(4);
    /// Native `VEH_EFFECT`.
    pub const EFFECT: Self = Self::COMPANY_END;
    /// Native `VEH_DISASTER`.
    pub const DISASTER: Self = Self(5);
}

/// Native `VehicleID` with its full original integer domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VehicleID(pub u32);
impl VehicleID {
    /// Native invalid sentinel.
    pub const INVALID: Self = Self(0xFFFFF);
}
