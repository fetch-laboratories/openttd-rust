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
pub const INVALID_VEH_ORDER_ID: u8 = VehicleOrderID::INVALID_VEH_ORDER_ID.0;
/// `OrderLoadType::FullLoad` and `FullLoadAny`.
pub const OLFB_FULL_LOAD: u8 = OrderLoadType::FullLoad.0;
/// Native `OLFB_FULL_LOAD_ANY`.
pub const OLFB_FULL_LOAD_ANY: u8 = OrderLoadType::FullLoadAny.0;
/// `OrderDepotTypeFlag::PartOfOrders`.
pub const ODTF_PART_OF_ORDERS: u8 =
    OrderDepotTypeFlags::from_element(OrderDepotTypeFlag::PartOfOrders).0;
/// `OrderDepotActionFlag::Halt`.
pub const ODATF_HALT: u8 = OrderDepotActionFlags::from_element(OrderDepotActionFlag::Halt).0;
/// `OrderDepotActionFlag::NearestDepot`.
pub const ODATF_NEAREST_DEPOT: u8 =
    OrderDepotActionFlags::from_element(OrderDepotActionFlag::NearestDepot).0;
/// `OrderLoadType::NoLoad`.
pub const OLFB_NO_LOAD: u8 = OrderLoadType::NoLoad.0;
/// `OrderUnloadType::Unload`.
pub const OUFB_UNLOAD: u8 = OrderUnloadType::Unload.0;
/// `OrderUnloadType::Transfer`.
pub const OUFB_TRANSFER: u8 = OrderUnloadType::Transfer.0;
/// `OrderUnloadType::NoUnload`.
pub const OUFB_NO_UNLOAD: u8 = OrderUnloadType::NoUnload.0;

/// `OrderDepotTypeFlag::Service`.
pub const ODTF_SERVICE: u8 = OrderDepotTypeFlags::from_element(OrderDepotTypeFlag::Service).0;
/// `OrderNonStopFlag::NoIntermediate`.
pub const ONSF_NO_INTERMEDIATE: u8 =
    OrderNonStopFlags::from_element(OrderNonStopFlag::NoIntermediate).0;

/// Native `OrderType : uint8_t`, including every unnamed byte pattern.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderType(pub u8);

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

/// `CloneOptions`: the action `CmdCloneOrder` performs, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CloneOptions(pub u8);

impl CloneOptions {
    /// Share the source's order list.
    pub const CO_SHARE: Self = Self(0);
    /// Copy the source's order list.
    pub const CO_COPY: Self = Self(1);
    /// Stop sharing the order list.
    pub const CO_UNSHARE: Self = Self(2);
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

/// Native `DestinationID`: the station or depot an order heads for, retaining
/// every underlying `u16` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DestinationID(pub u16);

impl DestinationID {
    /// `DestinationID::ToDepotID`.
    #[must_use]
    pub const fn to_depot_id(self) -> crate::depot_type::DepotID {
        crate::depot_type::DepotID(self.0)
    }
    /// `DestinationID::ToStationID`.
    #[must_use]
    pub const fn to_station_id(self) -> crate::station_type::StationID {
        crate::station_type::StationID(self.0)
    }
}

/// Native `VehicleOrderID`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VehicleOrderID(pub u8);

impl VehicleOrderID {
    /// Invalid order index within a vehicle.
    pub const INVALID_VEH_ORDER_ID: Self = Self(0xFF);
}

impl OrderNonStopFlags {
    /// Native `BaseBitSet::Any`: whether any element is set.
    #[must_use]
    pub const fn any(self) -> bool {
        self.0 != 0
    }
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: OrderNonStopFlag) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl OrderDepotTypeFlags {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: OrderDepotTypeFlag) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl OrderDepotActionFlags {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: OrderDepotActionFlag) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl OrderDepotTypeFlags {
    /// Native `BaseBitSet::Any`: whether any element is set.
    #[must_use]
    pub const fn any(self) -> bool {
        self.0 != 0
    }
    /// Native `BaseBitSet::Set` of one element, as a new set.
    #[must_use]
    pub const fn set(self, element: OrderDepotTypeFlag) -> Self {
        Self(self.0 | (1 << element.0))
    }
    /// Native `BaseBitSet::Reset` of one element, as a new set.
    #[must_use]
    pub const fn reset(self, element: OrderDepotTypeFlag) -> Self {
        Self(self.0 & !(1 << element.0))
    }
}

impl OrderDepotActionFlags {
    /// Native `BaseBitSet::Any`: whether any element is set.
    #[must_use]
    pub const fn any(self) -> bool {
        self.0 != 0
    }
    /// Native `BaseBitSet::Any(other)`: whether any element of `other` is set.
    #[must_use]
    pub const fn any_of(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
    /// Native `BaseBitSet::Set` of one element, as a new set.
    #[must_use]
    pub const fn set(self, element: OrderDepotActionFlag) -> Self {
        Self(self.0 | (1 << element.0))
    }
    /// Native `BaseBitSet::Reset` of one element, as a new set.
    #[must_use]
    pub const fn reset(self, element: OrderDepotActionFlag) -> Self {
        Self(self.0 & !(1 << element.0))
    }
    /// Native `BaseBitSet::Reset` of a set, as a new set.
    #[must_use]
    pub const fn reset_all(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }
}

/// `MAX_VEH_ORDER_ID`: the maximum number of orders a vehicle can have.
pub const MAX_VEH_ORDER_ID: VehicleOrderID =
    VehicleOrderID(VehicleOrderID::INVALID_VEH_ORDER_ID.0 - 1);

/// `IMPLICIT_ORDER_ONLY_CAP`: maximum number of orders in implicit-only lists before
/// implicit orders are no longer added to them.
pub const IMPLICIT_ORDER_ONLY_CAP: u32 = 32;

/// `OrderConditionVariable`: variables (of a vehicle) to 'cause' skipping on.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderConditionVariable(pub u8);

#[allow(
    non_upper_case_globals,
    reason = "the original's `enum class` member names"
)]
impl OrderConditionVariable {
    /// Skip based on the amount of load.
    pub const LoadPercentage: Self = Self(0);
    /// Skip based on the reliability.
    pub const Reliability: Self = Self(1);
    /// Skip based on the maximum speed.
    pub const MaxSpeed: Self = Self(2);
    /// Skip based on the age.
    pub const Age: Self = Self(3);
    /// Skip when the vehicle requires service.
    pub const RequiresService: Self = Self(4);
    /// Always skip.
    pub const Unconditionally: Self = Self(5);
    /// Skip based on the remaining lifetime.
    pub const RemainingLifetime: Self = Self(6);
    /// Skip based on the maximum reliability.
    pub const MaxReliability: Self = Self(7);
    /// `End`.
    pub const End: Self = Self(8);
}

/// `OrderConditionComparator`: comparator for the skip reasoning.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderConditionComparator(pub u8);

#[allow(
    non_upper_case_globals,
    reason = "the original's `enum class` member names"
)]
impl OrderConditionComparator {
    /// Skip if both values are equal.
    pub const Equal: Self = Self(0);
    /// Skip if both values are not equal.
    pub const NotEqual: Self = Self(1);
    /// Skip if the value is less than the limit.
    pub const LessThan: Self = Self(2);
    /// Skip if the value is less or equal to the limit.
    pub const LessThanOrEqual: Self = Self(3);
    /// Skip if the value is more than the limit.
    pub const MoreThan: Self = Self(4);
    /// Skip if the value is more or equal to the limit.
    pub const MoreThanOrEqual: Self = Self(5);
    /// Skip if the variable is true.
    pub const IsTrue: Self = Self(6);
    /// Skip if the variable is false.
    pub const IsFalse: Self = Self(7);
    /// `End`.
    pub const End: Self = Self(8);
}

/// `ModifyOrderFlags`: enumeration for the data to set in `CmdModifyOrder`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModifyOrderFlags(pub u8);

impl ModifyOrderFlags {
    /// Passes an `OrderNonStopFlags`.
    pub const MOF_NON_STOP: Self = Self(0);
    /// Passes an `OrderStopLocation`.
    pub const MOF_STOP_LOCATION: Self = Self(1);
    /// Passes an `OrderUnloadType`.
    pub const MOF_UNLOAD: Self = Self(2);
    /// Passes an `OrderLoadType`.
    pub const MOF_LOAD: Self = Self(3);
    /// Selects the `OrderDepotAction`.
    pub const MOF_DEPOT_ACTION: Self = Self(4);
    /// A conditional variable changes.
    pub const MOF_COND_VARIABLE: Self = Self(5);
    /// A comparator changes.
    pub const MOF_COND_COMPARATOR: Self = Self(6);
    /// The value to set the condition to.
    pub const MOF_COND_VALUE: Self = Self(7);
    /// Change the destination of a conditional order.
    pub const MOF_COND_DESTINATION: Self = Self(8);
    /// `MOF_END`.
    pub const MOF_END: Self = Self(9);
}

/// `OrderDepotAction`: depot action to switch to when doing a `MOF_DEPOT_ACTION`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderDepotAction(pub u8);

#[allow(
    non_upper_case_globals,
    reason = "the original's `enum class` member names"
)]
impl OrderDepotAction {
    /// Always go to the depot.
    pub const AlwaysGo: Self = Self(0);
    /// Service only if needed.
    pub const Service: Self = Self(1);
    /// Go to the depot and stop there.
    pub const Stop: Self = Self(2);
    /// Go to the depot and unbunch.
    pub const Unbunch: Self = Self(3);
    /// `End`.
    pub const End: Self = Self(4);
}

/// `ModifyTimetableFlags`: enumeration for the data to set in `CmdChangeTimetable`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModifyTimetableFlags(pub u8);

impl ModifyTimetableFlags {
    /// Set wait time.
    pub const MTF_WAIT_TIME: Self = Self(0);
    /// Set travel time.
    pub const MTF_TRAVEL_TIME: Self = Self(1);
    /// Set max travel speed.
    pub const MTF_TRAVEL_SPEED: Self = Self(2);
    /// `MTF_END`.
    pub const MTF_END: Self = Self(3);
}

crate::base_bitset_type::impl_bitset_ops!(OrderNonStopFlags);
crate::base_bitset_type::impl_bitset_ops!(OrderDepotTypeFlags);
crate::base_bitset_type::impl_bitset_ops!(OrderDepotActionFlags);
