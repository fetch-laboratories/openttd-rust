/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Functions related to the vehicle's GUIs (`vehicle_gui.h`).

use crate::vehicle_type::VehicleType;
use crate::window_type::WindowClass;

/// Special values for vehicle-related windows for the data parameter of
/// `InvalidateWindowData`. These values are used to communicate about order changes so
/// windows can do some smart updates.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VehicleInvalidateWindowData(pub i32);

impl VehicleInvalidateWindowData {
    /// Removed / replaced all orders (after deleting / sharing).
    pub const VIWD_REMOVE_ALL_ORDERS: Self = Self(-1);
    /// Other order modifications.
    pub const VIWD_MODIFY_ORDERS: Self = Self(-2);
    /// Vehicle composition was changed.
    pub const VIWD_CONSIST_CHANGED: Self = Self(-3);
}

/// `GetWindowClassForVehicleType`: get the window class for the vehicle list of the given
/// vehicle type. Works only for company buildable vehicle types.
pub(crate) fn get_window_class_for_vehicle_type(vt: VehicleType) -> WindowClass {
    match vt {
        VehicleType::VEH_TRAIN => WindowClass::WC_TRAINS_LIST,
        VehicleType::VEH_ROAD => WindowClass::WC_ROADVEH_LIST,
        VehicleType::VEH_SHIP => WindowClass::WC_SHIPS_LIST,
        VehicleType::VEH_AIRCRAFT => WindowClass::WC_AIRCRAFT_LIST,
        _ => crate::services_assert::not_reached(),
    }
}

#[cfg(test)]
mod tests {
    use super::get_window_class_for_vehicle_type;
    use crate::vehicle_type::VehicleType;
    use crate::window_type::WindowClass;

    #[test]
    fn each_buildable_type_has_its_list_class() {
        for (vt, class) in [
            (VehicleType::VEH_TRAIN, WindowClass::WC_TRAINS_LIST),
            (VehicleType::VEH_ROAD, WindowClass::WC_ROADVEH_LIST),
            (VehicleType::VEH_SHIP, WindowClass::WC_SHIPS_LIST),
            (VehicleType::VEH_AIRCRAFT, WindowClass::WC_AIRCRAFT_LIST),
        ] {
            assert_eq!(get_window_class_for_vehicle_type(vt), class);
        }
    }
}
