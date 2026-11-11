/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Station helpers and rectangle modes from `base_station_base.h`.

/// Native `StationRect::StationRectMode`, retaining the original byte domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StationRectMode(pub u8);
impl StationRectMode {
    /// Test the spread without changing the rectangle.
    pub const ADD_TEST: Self = Self(0);
    /// Try to extend the rectangle within the station spread limit.
    pub const ADD_TRY: Self = Self(1);
    /// Extend the rectangle regardless of the station spread limit.
    pub const ADD_FORCE: Self = Self(2);
}

/// `BaseStation::IsInUse`: check whether the base station currently is in use;
/// in use means that it is not scheduled for deletion and that it still has
/// some facilities left.
pub(crate) fn is_in_use(facilities: crate::station_type::StationFacilities) -> bool {
    use crate::station_type::{StationFacilities, StationFacility};
    facilities.any(
        StationFacilities::from_element(StationFacility::Train)
            .with(StationFacilities::from_element(StationFacility::TruckStop))
            .with(StationFacilities::from_element(StationFacility::BusStop))
            .with(StationFacilities::from_element(StationFacility::Airport))
            .with(StationFacilities::from_element(StationFacility::Dock)),
    )
}
