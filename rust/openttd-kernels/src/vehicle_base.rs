/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `vehicle_base.h` subtype values and inline predicates over a copied
//! `Vehicle::subtype`. `services.cpp` asserts every value.

/// `GroundVehicleSubtypeFlags::GVSF_FRONT`.
pub const GVSF_FRONT: u8 = 0;
/// `GroundVehicleSubtypeFlags::GVSF_ARTICULATED_PART`.
pub const GVSF_ARTICULATED_PART: u8 = 1;
/// `GroundVehicleSubtypeFlags::GVSF_WAGON`.
pub const GVSF_WAGON: u8 = 2;
/// `GroundVehicleSubtypeFlags::GVSF_ENGINE`.
pub const GVSF_ENGINE: u8 = 3;
/// `GroundVehicleSubtypeFlags::GVSF_FREE_WAGON`.
pub const GVSF_FREE_WAGON: u8 = 4;
/// `GroundVehicleSubtypeFlags::GVSF_MULTIHEADED`.
pub const GVSF_MULTIHEADED: u8 = 5;

/// `HasBit(subtype, flag)`.
const fn has(subtype: u8, flag: u8) -> bool {
    subtype & (1 << flag) != 0
}
/// `Vehicle::IsFrontEngine`; `ground` is `IsGroundVehicle()`.
pub(crate) const fn is_front_engine(ground: bool, subtype: u8) -> bool {
    ground && has(subtype, GVSF_FRONT)
}
/// `Vehicle::IsArticulatedPart`; `ground` is `IsGroundVehicle()`.
pub(crate) const fn is_articulated_part(ground: bool, subtype: u8) -> bool {
    ground && has(subtype, GVSF_ARTICULATED_PART)
}

/// `Vehicle::IsGroundVehicle`.
pub(crate) const fn is_ground_vehicle(kind: u8) -> bool {
    kind == crate::vehicle_type::VEH_TRAIN || kind == crate::vehicle_type::VEH_ROAD
}

/// `VehStatus::TrainSlowing`.
pub const VS_TRAIN_SLOWING: u8 = 1 << 4;

/// `NCVV_POSITION_CONSIST_LENGTH`.
pub const NCVV_POSITION_CONSIST_LENGTH: u8 = 0;
/// `NCVV_POSITION_SAME_ID_LENGTH`.
pub const NCVV_POSITION_SAME_ID_LENGTH: u8 = 1;
/// `NCVV_CONSIST_CARGO_INFORMATION`.
pub const NCVV_CONSIST_CARGO_INFORMATION: u8 = 2;
/// `NCVV_COMPANY_INFORMATION`.
pub const NCVV_COMPANY_INFORMATION: u8 = 3;
/// `NCVV_POSITION_IN_VEHICLE`.
pub const NCVV_POSITION_IN_VEHICLE: u8 = 4;
/// `NCVV_END`.
pub const NCVV_END: u8 = 5;
