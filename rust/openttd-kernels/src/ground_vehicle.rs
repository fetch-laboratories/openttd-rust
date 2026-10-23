/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `ground_vehicle.hpp` subtype helpers over a copied `Vehicle::subtype`.
//! Setters return the new value; callers store it at the original point.
use crate::vehicle_base::{GVSF_ENGINE, GVSF_FREE_WAGON, GVSF_FRONT, GVSF_MULTIHEADED, GVSF_WAGON};

const fn has(subtype: u8, flag: u8) -> bool {
    subtype & (1 << flag) != 0
}
/// `SetFrontEngine`.
pub(crate) const fn set_front_engine(subtype: u8) -> u8 {
    subtype | 1 << GVSF_FRONT
}
/// `ClearFrontEngine`.
pub(crate) const fn clear_front_engine(subtype: u8) -> u8 {
    subtype & !(1 << GVSF_FRONT)
}
/// `SetEngine`.
pub(crate) const fn set_engine(subtype: u8) -> u8 {
    subtype | 1 << GVSF_ENGINE
}
/// `SetFreeWagon`.
pub(crate) const fn set_free_wagon(subtype: u8) -> u8 {
    subtype | 1 << GVSF_FREE_WAGON
}
/// `ClearFreeWagon`.
pub(crate) const fn clear_free_wagon(subtype: u8) -> u8 {
    subtype & !(1 << GVSF_FREE_WAGON)
}
/// `SetMultiheaded`.
pub(crate) const fn set_multiheaded(subtype: u8) -> u8 {
    subtype | 1 << GVSF_MULTIHEADED
}
/// `IsFreeWagon`.
pub(crate) const fn is_free_wagon(subtype: u8) -> bool {
    has(subtype, GVSF_FREE_WAGON)
}
/// `IsEngine`.
pub(crate) const fn is_engine(subtype: u8) -> bool {
    has(subtype, GVSF_ENGINE)
}
/// `IsWagon`.
pub(crate) const fn is_wagon(subtype: u8) -> bool {
    has(subtype, GVSF_WAGON)
}
/// `IsMultiheaded`.
pub(crate) const fn is_multiheaded(subtype: u8) -> bool {
    has(subtype, GVSF_MULTIHEADED)
}
/// `IsRearDualheaded`.
pub(crate) const fn is_rear_dualheaded(subtype: u8) -> bool {
    is_multiheaded(subtype) && !is_engine(subtype)
}

/// Native cached incline flag bits.
pub const GVF_GOINGUP_BIT: u8 = 0;
pub const GVF_GOINGDOWN_BIT: u8 = 1;
