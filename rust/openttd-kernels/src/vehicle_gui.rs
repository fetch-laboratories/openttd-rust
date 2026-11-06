/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Pure vehicle-window classification from `vehicle_gui.h`.
use crate::services::Services;
use crate::window_type::{WC_AIRCRAFT_LIST, WC_ROADVEH_LIST, WC_SHIPS_LIST, WC_TRAINS_LIST};

pub(crate) fn list_window(kind: u8, shared: &Services) -> u16 {
    match kind {
        crate::vehicle_type::VEH_TRAIN => WC_TRAINS_LIST,
        crate::vehicle_type::VEH_ROAD => WC_ROADVEH_LIST,
        crate::vehicle_type::VEH_SHIP => WC_SHIPS_LIST,
        crate::vehicle_type::VEH_AIRCRAFT => WC_AIRCRAFT_LIST,
        _ => {
            (shared.not_reached)();
            std::process::abort();
        }
    }
}
