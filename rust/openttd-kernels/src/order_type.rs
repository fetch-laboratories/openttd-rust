/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `order_type.h` values. `services.cpp` asserts every value.

/// `OT_NOTHING`.
pub const OT_NOTHING: u8 = OrderType::OT_NOTHING.0;
/// `OT_GOTO_STATION`.
pub const OT_GOTO_STATION: u8 = OrderType::OT_GOTO_STATION.0;
/// `OT_GOTO_DEPOT`.
pub const OT_GOTO_DEPOT: u8 = OrderType::OT_GOTO_DEPOT.0;
/// `OT_LOADING`.
pub const OT_LOADING: u8 = OrderType::OT_LOADING.0;
/// `OT_LEAVESTATION`.
pub const OT_LEAVESTATION: u8 = OrderType::OT_LEAVESTATION.0;
/// `OT_DUMMY`.
pub const OT_DUMMY: u8 = OrderType::OT_DUMMY.0;
/// `OT_GOTO_WAYPOINT`.
pub const OT_GOTO_WAYPOINT: u8 = OrderType::OT_GOTO_WAYPOINT.0;
/// `OT_CONDITIONAL`.
pub const OT_CONDITIONAL: u8 = OrderType::OT_CONDITIONAL.0;
/// `OT_IMPLICIT`.
pub const OT_IMPLICIT: u8 = OrderType::OT_IMPLICIT.0;
/// `INVALID_VEH_ORDER_ID`.
pub const INVALID_VEH_ORDER_ID: u8 = 0xFF;
/// `OrderLoadType::FullLoad` and `FullLoadAny`.
pub const OLFB_FULL_LOAD: u8 = 2;
pub const OLFB_FULL_LOAD_ANY: u8 = 3;
/// `OrderDepotTypeFlag::PartOfOrders`.
pub const ODTF_PART_OF_ORDERS: u8 = 1 << 1;
/// `OrderDepotActionFlag::Halt`.
pub const ODATF_HALT: u8 = 1 << 0;
/// `OrderDepotActionFlag::NearestDepot`.
pub const ODATF_NEAREST_DEPOT: u8 = 1 << 1;
/// `OrderLoadType::NoLoad`.
pub const OLFB_NO_LOAD: u8 = 4;
/// `OrderUnloadType::Unload`.
pub const OUFB_UNLOAD: u8 = 1;
/// `OrderUnloadType::Transfer`.
pub const OUFB_TRANSFER: u8 = 2;
/// `OrderUnloadType::NoUnload`.
pub const OUFB_NO_UNLOAD: u8 = 4;

/// `OrderDepotTypeFlag::Service`.
pub const ODTF_SERVICE: u8 = 1 << 0;
/// `OrderNonStopFlag::NoIntermediate`.
pub const ONSF_NO_INTERMEDIATE: u8 = 1 << 0;

/// Native `OrderType : uint8_t`, including every unnamed byte pattern.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct OrderType(pub u8);

#[allow(non_upper_case_globals)]
impl OrderType {
    /// Native `OT_BEGIN`.
    pub const OT_BEGIN: Self = Self(0);
    /// Native `OT_NOTHING`.
    pub const OT_NOTHING: Self = Self(0);
    /// Native `OT_GOTO_STATION`.
    pub const OT_GOTO_STATION: Self = Self(1);
    /// Native `OT_GOTO_DEPOT`.
    pub const OT_GOTO_DEPOT: Self = Self(2);
    /// Native `OT_LOADING`.
    pub const OT_LOADING: Self = Self(3);
    /// Native `OT_LEAVESTATION`.
    pub const OT_LEAVESTATION: Self = Self(4);
    /// Native `OT_DUMMY`.
    pub const OT_DUMMY: Self = Self(5);
    /// Native `OT_GOTO_WAYPOINT`.
    pub const OT_GOTO_WAYPOINT: Self = Self(6);
    /// Native `OT_CONDITIONAL`.
    pub const OT_CONDITIONAL: Self = Self(7);
    /// Native `OT_IMPLICIT`.
    pub const OT_IMPLICIT: Self = Self(8);
    /// Native `OT_END`.
    pub const OT_END: Self = Self(9);
}

/// Native `OrderLoadType`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderLoadType(pub u8);

#[allow(non_upper_case_globals)]
impl OrderLoadType {
    /// Load as long as there is cargo that fits in the train.
    pub const LoadIfPossible: Self = Self(0);
    /// Full load all cargoes of the consist.
    pub const FullLoad: Self = Self(2);
    /// Full load a single cargo of the consist.
    pub const FullLoadAny: Self = Self(3);
    /// Do not load anything.
    pub const NoLoad: Self = Self(4);
}

/// Native `OrderUnloadType`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderUnloadType(pub u8);

#[allow(non_upper_case_globals)]
impl OrderUnloadType {
    /// Unload all cargo that the station accepts.
    pub const UnloadIfPossible: Self = Self(0);
    /// Force unloading all cargo onto the platform, possibly not getting paid.
    pub const Unload: Self = Self(1);
    /// Transfer all cargo onto the platform.
    pub const Transfer: Self = Self(2);
    /// Totally no unloading will be done.
    pub const NoUnload: Self = Self(4);
}

/// Native `OrderNonStopFlag`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderNonStopFlag(pub u8);

#[allow(non_upper_case_globals)]
impl OrderNonStopFlag {
    /// The vehicle will not stop at any stations it passes except the destination, aka non-stop.
    pub const NoIntermediate: Self = Self(0);
    /// The vehicle will stop at any station it passes except the destination, aka via.
    pub const NoDestination: Self = Self(1);
}

/// Native `OrderStopLocation`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderStopLocation(pub u8);

#[allow(non_upper_case_globals)]
impl OrderStopLocation {
    /// Stop at the near end of the platform
    pub const NearEnd: Self = Self(0);
    /// Stop at the middle of the platform
    pub const Middle: Self = Self(1);
    /// Stop at the far end of the platform
    pub const FarEnd: Self = Self(2);
    /// Native `End`.
    pub const End: Self = Self(3);
}

/// Native `OrderDepotTypeFlag`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderDepotTypeFlag(pub u8);

#[allow(non_upper_case_globals)]
impl OrderDepotTypeFlag {
    /// This depot order is because of the servicing limit.
    pub const Service: Self = Self(0);
    /// This depot order is because of a regular order.
    pub const PartOfOrders: Self = Self(1);
}

/// Native `OrderDepotActionFlag`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderDepotActionFlag(pub u8);

#[allow(non_upper_case_globals)]
impl OrderDepotActionFlag {
    /// Service the vehicle and then halt it.
    pub const Halt: Self = Self(0);
    /// Send the vehicle to the nearest depot.
    pub const NearestDepot: Self = Self(1);
    /// Service the vehicle and then unbunch it.
    pub const Unbunch: Self = Self(2);
}

/// Native `OrderNonStopFlags`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderNonStopFlags(pub u8);

impl OrderNonStopFlags {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Set containing the named element (the element is a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: OrderNonStopFlag) -> Self {
        Self((1_u64 << element.0) as u8)
    }
}

/// Native `OrderDepotTypeFlags`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderDepotTypeFlags(pub u8);

impl OrderDepotTypeFlags {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Set containing the named element (the element is a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: OrderDepotTypeFlag) -> Self {
        Self((1_u64 << element.0) as u8)
    }
}

/// Native `OrderDepotActionFlags`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderDepotActionFlags(pub u8);

impl OrderDepotActionFlags {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Set containing the named element (the element is a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: OrderDepotActionFlag) -> Self {
        Self((1_u64 << element.0) as u8)
    }
}

/// Native `OrderListID`, retaining every underlying `u16` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderListID(pub u16);

impl OrderListID {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(64000);
    /// Native `PoolID::Invalid()`.
    pub const INVALID: Self = Self(65535);
}

/// Native `VehicleOrderID`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VehicleOrderID(pub u8);

impl VehicleOrderID {
    /// Invalid order index within a vehicle.
    pub const INVALID_VEH_ORDER_ID: Self = Self(0xFF);
}
