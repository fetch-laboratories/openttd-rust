/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `order_type.h` values. `services.cpp` asserts every value.

/// `OT_NOTHING`.
pub const OT_NOTHING: u8 = 0;
/// `OT_GOTO_STATION`.
pub const OT_GOTO_STATION: u8 = 1;
/// `OT_GOTO_DEPOT`.
pub const OT_GOTO_DEPOT: u8 = 2;
/// `OT_LOADING`.
pub const OT_LOADING: u8 = 3;
/// `OT_LEAVESTATION`.
pub const OT_LEAVESTATION: u8 = 4;
/// `OT_DUMMY`.
pub const OT_DUMMY: u8 = 5;
/// `OT_GOTO_WAYPOINT`.
pub const OT_GOTO_WAYPOINT: u8 = 6;
/// `OT_CONDITIONAL`.
pub const OT_CONDITIONAL: u8 = 7;
/// `OT_IMPLICIT`.
pub const OT_IMPLICIT: u8 = 8;
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
