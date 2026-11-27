/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared definitions from `vehiclelist.h`.

use crate::company_type::{CompanyID, Owner};
use crate::group_type::GroupID;
use crate::order_type::DestinationID;
use crate::services_assert::assert_with;
use crate::station_type::StationID;
use crate::vehicle_type::{VehicleID, VehicleType};
use crate::window_type::WindowNumber;

/// Vehicle list type flags, retaining every underlying `uint8_t` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VehicleListType(pub u8);

impl VehicleListType {
    /// Index is the company.
    pub const VL_STANDARD: Self = Self(0);
    /// Index is the first vehicle of the shared orders.
    pub const VL_SHARED_ORDERS: Self = Self(1);
    /// Index is the station.
    pub const VL_STATION_LIST: Self = Self(2);
    /// Index is the destination (station for hangar of aircraft, depot for others).
    pub const VL_DEPOT_LIST: Self = Self(3);
    /// Index is the group.
    pub const VL_GROUP_LIST: Self = Self(4);
    /// Native `VLT_END`.
    pub const VLT_END: Self = Self(5);
}

/// Native `VehicleListIdentifier`: the information about a vehicle list.
///
/// A struct literal stands for the original's constructors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VehicleListIdentifier {
    /// The type of vehicle list.
    pub r#type: VehicleListType,
    /// The vehicle type associated with this list.
    pub vtype: VehicleType,
    /// The company associated with this list.
    pub company: CompanyID,
    /// A vehicle list type specific index.
    pub index: u32,
}

impl Default for VehicleListIdentifier {
    /// The value-initialisation `VehicleListIdentifier{}`: every member zero.
    fn default() -> Self {
        Self {
            r#type: VehicleListType(0),
            vtype: VehicleType(0),
            company: CompanyID(0),
            index: 0,
        }
    }
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "the `To*ID` accessors convert the `uint32_t` index as the original's constructors do"
)]
impl VehicleListIdentifier {
    /// `VehicleListIdentifier::ToWindowNumber`: pack a `VehicleListIdentifier` in 32 bits so
    /// it can be used as unique `WindowNumber`.
    #[must_use]
    #[allow(
        clippy::cast_possible_wrap,
        reason = "the original's conversion to `WindowNumber`"
    )]
    pub fn to_window_number(&self) -> WindowNumber {
        const _: () = assert!(VehicleListType::VLT_END.0 <= (1 << 3));
        let c: u8 = if self.company == Owner::NONE {
            0xF
        } else {
            self.company.0
        };
        assert_with(|| c < (1 << 4));
        assert_with(|| self.vtype.0 < (1 << 2));
        assert_with(|| self.index < (1 << 20));
        assert_with(|| self.r#type < VehicleListType::VLT_END);

        // `c << 28 | type << 23 | vtype << 26 | index` in `uint`, as the `int` window number.
        WindowNumber(
            (u32::from(c) << 28
                | u32::from(self.r#type.0) << 23
                | u32::from(self.vtype.0) << 26
                | self.index) as i32,
        )
    }
    /// `VehicleListIdentifier::Valid`.
    #[must_use]
    pub fn valid(&self) -> bool {
        self.r#type < VehicleListType::VLT_END
    }
    /// `VehicleListIdentifier::ToCompanyID`.
    #[must_use]
    pub fn to_company_id(&self) -> CompanyID {
        assert_with(|| self.r#type == VehicleListType::VL_STANDARD);
        CompanyID(self.index as u8)
    }
    /// `VehicleListIdentifier::ToDestinationID`.
    #[must_use]
    pub fn to_destination_id(&self) -> DestinationID {
        assert_with(|| self.r#type == VehicleListType::VL_DEPOT_LIST);
        DestinationID(self.index as u16)
    }
    /// `VehicleListIdentifier::ToGroupID`.
    #[must_use]
    pub fn to_group_id(&self) -> GroupID {
        assert_with(|| self.r#type == VehicleListType::VL_GROUP_LIST);
        GroupID(self.index as u16)
    }
    /// `VehicleListIdentifier::ToStationID`.
    #[must_use]
    pub fn to_station_id(&self) -> StationID {
        assert_with(|| self.r#type == VehicleListType::VL_STATION_LIST);
        StationID(self.index as u16)
    }
    /// `VehicleListIdentifier::ToVehicleID`.
    #[must_use]
    pub fn to_vehicle_id(&self) -> VehicleID {
        assert_with(|| self.r#type == VehicleListType::VL_SHARED_ORDERS);
        VehicleID(self.index)
    }
}
