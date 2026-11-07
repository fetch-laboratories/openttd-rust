/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `economy_type.h` `Price` indices. `services.cpp` asserts every value.

/// `PR_BUILD_SIGNALS`.
pub const PR_BUILD_SIGNALS: usize = Price::PR_BUILD_SIGNALS.0 as usize;
/// `PR_BUILD_DEPOT_TRAIN`.
pub const PR_BUILD_DEPOT_TRAIN: usize = Price::PR_BUILD_DEPOT_TRAIN.0 as usize;
/// `PR_BUILD_DEPOT_ROAD`.
pub const PR_BUILD_DEPOT_ROAD: usize = Price::PR_BUILD_DEPOT_ROAD.0 as usize;
/// `PR_BUILD_STATION_BUS`.
pub const PR_BUILD_STATION_BUS: usize = Price::PR_BUILD_STATION_BUS.0 as usize;
/// `PR_BUILD_STATION_RAIL`.
pub const PR_BUILD_STATION_RAIL: usize = Price::PR_BUILD_STATION_RAIL.0 as usize;
/// `PR_BUILD_STATION_RAIL_LENGTH`.
pub const PR_BUILD_STATION_RAIL_LENGTH: usize = Price::PR_BUILD_STATION_RAIL_LENGTH.0 as usize;
/// `PR_CLEAR_ROUGH`.
pub const PR_CLEAR_ROUGH: usize = Price::PR_CLEAR_ROUGH.0 as usize;
/// `PR_CLEAR_SIGNALS`.
pub const PR_CLEAR_SIGNALS: usize = Price::PR_CLEAR_SIGNALS.0 as usize;
/// `PR_CLEAR_DEPOT_TRAIN`.
pub const PR_CLEAR_DEPOT_TRAIN: usize = Price::PR_CLEAR_DEPOT_TRAIN.0 as usize;
/// `PR_CLEAR_WATER`.
pub const PR_CLEAR_WATER: usize = Price::PR_CLEAR_WATER.0 as usize;
/// `PR_BUILD_FOUNDATION`.
pub const PR_BUILD_FOUNDATION: usize = Price::PR_BUILD_FOUNDATION.0 as usize;

/// `PR_BUILD_TREES`.
pub const PR_BUILD_TREES: usize = Price::PR_BUILD_TREES.0 as usize;
/// `PR_TERRAFORM`.
pub const PR_TERRAFORM: usize = Price::PR_TERRAFORM.0 as usize;
/// `PR_CLEAR_CANAL`.
pub const PR_CLEAR_CANAL: usize = Price::PR_CLEAR_CANAL.0 as usize;

/// `PR_CLEAR_TREES`.
pub const PR_CLEAR_TREES: usize = Price::PR_CLEAR_TREES.0 as usize;

/// `PR_BUILD_BRIDGE`.
pub const PR_BUILD_BRIDGE: usize = Price::PR_BUILD_BRIDGE.0 as usize;
/// `PR_BUILD_TUNNEL`.
pub const PR_BUILD_TUNNEL: usize = Price::PR_BUILD_TUNNEL.0 as usize;
/// `PR_CLEAR_BRIDGE`.
pub const PR_CLEAR_BRIDGE: usize = Price::PR_CLEAR_BRIDGE.0 as usize;
/// `PR_CLEAR_TUNNEL`.
pub const PR_CLEAR_TUNNEL: usize = Price::PR_CLEAR_TUNNEL.0 as usize;
/// `PR_BUILD_AQUEDUCT`.
pub const PR_BUILD_AQUEDUCT: usize = Price::PR_BUILD_AQUEDUCT.0 as usize;
/// `PR_CLEAR_AQUEDUCT`.
pub const PR_CLEAR_AQUEDUCT: usize = Price::PR_CLEAR_AQUEDUCT.0 as usize;

/// `PR_TOWN_ACTION`.
pub const PR_TOWN_ACTION: usize = Price::PR_TOWN_ACTION.0 as usize;

/// `EXPENSES_CONSTRUCTION`.
pub const EXPENSES_CONSTRUCTION: u8 = ExpensesType::EXPENSES_CONSTRUCTION.0;
/// `EXPENSES_OTHER`.
pub const EXPENSES_OTHER: u8 = ExpensesType::EXPENSES_OTHER.0;

/// `PR_BUILD_STATION_TRUCK`.
pub const PR_BUILD_STATION_TRUCK: usize = Price::PR_BUILD_STATION_TRUCK.0 as usize;
/// `PR_BUILD_WAYPOINT_RAIL`.
pub const PR_BUILD_WAYPOINT_RAIL: usize = Price::PR_BUILD_WAYPOINT_RAIL.0 as usize;
/// `PR_BUILD_WAYPOINT_BUOY`.
pub const PR_BUILD_WAYPOINT_BUOY: usize = Price::PR_BUILD_WAYPOINT_BUOY.0 as usize;
/// `PR_CLEAR_WAYPOINT_BUOY`.
pub const PR_CLEAR_WAYPOINT_BUOY: usize = Price::PR_CLEAR_WAYPOINT_BUOY.0 as usize;
/// `ROAD_STOP_TRACKBIT_FACTOR`.
pub const ROAD_STOP_TRACKBIT_FACTOR: u32 = 2;

/// `PR_END`.
pub const PR_END: u8 = Price::PR_END.0;
/// `INVALID_PRICE`.
pub const INVALID_PRICE: u8 = Price::INVALID_PRICE.0;

/// `PR_BUILD_INDUSTRY`.
pub const PR_BUILD_INDUSTRY: usize = Price::PR_BUILD_INDUSTRY.0 as usize;
/// `PR_CLEAR_INDUSTRY`.
pub const PR_CLEAR_INDUSTRY: usize = Price::PR_CLEAR_INDUSTRY.0 as usize;
/// `PR_BUILD_INDUSTRY_RAW`.
pub const PR_BUILD_INDUSTRY_RAW: usize = Price::PR_BUILD_INDUSTRY_RAW.0 as usize;
/// `PR_CLEAR_HOUSE`.
pub const PR_CLEAR_HOUSE: usize = Price::PR_CLEAR_HOUSE.0 as usize;

/// `PR_CLEAR_GRASS`.
pub const PR_CLEAR_GRASS: usize = Price::PR_CLEAR_GRASS.0 as usize;
/// `PR_CLEAR_ROCKS`.
pub const PR_CLEAR_ROCKS: usize = Price::PR_CLEAR_ROCKS.0 as usize;
/// `PR_CLEAR_FIELDS`.
pub const PR_CLEAR_FIELDS: usize = Price::PR_CLEAR_FIELDS.0 as usize;
/// `PR_BUILD_VEHICLE_TRAIN`.
pub const PR_BUILD_VEHICLE_TRAIN: u32 = Price::PR_BUILD_VEHICLE_TRAIN.0 as u32;
/// `PR_BUILD_VEHICLE_WAGON`.
pub const PR_BUILD_VEHICLE_WAGON: u32 = Price::PR_BUILD_VEHICLE_WAGON.0 as u32;
/// `PR_BUILD_VEHICLE_AIRCRAFT`.
pub const PR_BUILD_VEHICLE_AIRCRAFT: u32 = Price::PR_BUILD_VEHICLE_AIRCRAFT.0 as u32;
/// `PR_BUILD_VEHICLE_ROAD`.
pub const PR_BUILD_VEHICLE_ROAD: u32 = Price::PR_BUILD_VEHICLE_ROAD.0 as u32;
/// `PR_BUILD_VEHICLE_SHIP`.
pub const PR_BUILD_VEHICLE_SHIP: u32 = Price::PR_BUILD_VEHICLE_SHIP.0 as u32;
/// `PR_RUNNING_AIRCRAFT`.
pub const PR_RUNNING_AIRCRAFT: u32 = Price::PR_RUNNING_AIRCRAFT.0 as u32;
/// `PR_RUNNING_SHIP`.
pub const PR_RUNNING_SHIP: u32 = Price::PR_RUNNING_SHIP.0 as u32;
/// `PR_BUILD_DEPOT_SHIP`.
pub const PR_BUILD_DEPOT_SHIP: usize = Price::PR_BUILD_DEPOT_SHIP.0 as usize;
/// `PR_BUILD_STATION_DOCK`.
pub const PR_BUILD_STATION_DOCK: usize = Price::PR_BUILD_STATION_DOCK.0 as usize;
/// `PR_BUILD_CANAL`.
pub const PR_BUILD_CANAL: usize = Price::PR_BUILD_CANAL.0 as usize;
/// `PR_BUILD_LOCK`.
pub const PR_BUILD_LOCK: usize = Price::PR_BUILD_LOCK.0 as usize;

/// Native `ExpensesType : uint8_t` carrier.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExpensesType(pub u8);

/// Native `Price`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Price(pub u8);

impl Price {
    /// Native `PR_BEGIN`.
    pub const PR_BEGIN: Self = Self(0);
    /// Native `PR_STATION_VALUE`.
    pub const PR_STATION_VALUE: Self = Self(0);
    /// Native `PR_BUILD_RAIL`.
    pub const PR_BUILD_RAIL: Self = Self(1);
    /// Native `PR_BUILD_ROAD`.
    pub const PR_BUILD_ROAD: Self = Self(2);
    /// Native `PR_BUILD_SIGNALS`.
    pub const PR_BUILD_SIGNALS: Self = Self(3);
    /// Native `PR_BUILD_BRIDGE`.
    pub const PR_BUILD_BRIDGE: Self = Self(4);
    /// Native `PR_BUILD_DEPOT_TRAIN`.
    pub const PR_BUILD_DEPOT_TRAIN: Self = Self(5);
    /// Native `PR_BUILD_DEPOT_ROAD`.
    pub const PR_BUILD_DEPOT_ROAD: Self = Self(6);
    /// Native `PR_BUILD_DEPOT_SHIP`.
    pub const PR_BUILD_DEPOT_SHIP: Self = Self(7);
    /// Native `PR_BUILD_TUNNEL`.
    pub const PR_BUILD_TUNNEL: Self = Self(8);
    /// Native `PR_BUILD_STATION_RAIL`.
    pub const PR_BUILD_STATION_RAIL: Self = Self(9);
    /// Native `PR_BUILD_STATION_RAIL_LENGTH`.
    pub const PR_BUILD_STATION_RAIL_LENGTH: Self = Self(10);
    /// Native `PR_BUILD_STATION_AIRPORT`.
    pub const PR_BUILD_STATION_AIRPORT: Self = Self(11);
    /// Native `PR_BUILD_STATION_BUS`.
    pub const PR_BUILD_STATION_BUS: Self = Self(12);
    /// Native `PR_BUILD_STATION_TRUCK`.
    pub const PR_BUILD_STATION_TRUCK: Self = Self(13);
    /// Native `PR_BUILD_STATION_DOCK`.
    pub const PR_BUILD_STATION_DOCK: Self = Self(14);
    /// Native `PR_BUILD_VEHICLE_TRAIN`.
    pub const PR_BUILD_VEHICLE_TRAIN: Self = Self(15);
    /// Native `PR_BUILD_VEHICLE_WAGON`.
    pub const PR_BUILD_VEHICLE_WAGON: Self = Self(16);
    /// Native `PR_BUILD_VEHICLE_AIRCRAFT`.
    pub const PR_BUILD_VEHICLE_AIRCRAFT: Self = Self(17);
    /// Native `PR_BUILD_VEHICLE_ROAD`.
    pub const PR_BUILD_VEHICLE_ROAD: Self = Self(18);
    /// Native `PR_BUILD_VEHICLE_SHIP`.
    pub const PR_BUILD_VEHICLE_SHIP: Self = Self(19);
    /// Native `PR_BUILD_TREES`.
    pub const PR_BUILD_TREES: Self = Self(20);
    /// Native `PR_TERRAFORM`.
    pub const PR_TERRAFORM: Self = Self(21);
    /// Native `PR_CLEAR_GRASS`.
    pub const PR_CLEAR_GRASS: Self = Self(22);
    /// Native `PR_CLEAR_ROUGH`.
    pub const PR_CLEAR_ROUGH: Self = Self(23);
    /// Native `PR_CLEAR_ROCKS`.
    pub const PR_CLEAR_ROCKS: Self = Self(24);
    /// Native `PR_CLEAR_FIELDS`.
    pub const PR_CLEAR_FIELDS: Self = Self(25);
    /// Native `PR_CLEAR_TREES`.
    pub const PR_CLEAR_TREES: Self = Self(26);
    /// Native `PR_CLEAR_RAIL`.
    pub const PR_CLEAR_RAIL: Self = Self(27);
    /// Native `PR_CLEAR_SIGNALS`.
    pub const PR_CLEAR_SIGNALS: Self = Self(28);
    /// Native `PR_CLEAR_BRIDGE`.
    pub const PR_CLEAR_BRIDGE: Self = Self(29);
    /// Native `PR_CLEAR_DEPOT_TRAIN`.
    pub const PR_CLEAR_DEPOT_TRAIN: Self = Self(30);
    /// Native `PR_CLEAR_DEPOT_ROAD`.
    pub const PR_CLEAR_DEPOT_ROAD: Self = Self(31);
    /// Native `PR_CLEAR_DEPOT_SHIP`.
    pub const PR_CLEAR_DEPOT_SHIP: Self = Self(32);
    /// Native `PR_CLEAR_TUNNEL`.
    pub const PR_CLEAR_TUNNEL: Self = Self(33);
    /// Native `PR_CLEAR_WATER`.
    pub const PR_CLEAR_WATER: Self = Self(34);
    /// Native `PR_CLEAR_STATION_RAIL`.
    pub const PR_CLEAR_STATION_RAIL: Self = Self(35);
    /// Native `PR_CLEAR_STATION_AIRPORT`.
    pub const PR_CLEAR_STATION_AIRPORT: Self = Self(36);
    /// Native `PR_CLEAR_STATION_BUS`.
    pub const PR_CLEAR_STATION_BUS: Self = Self(37);
    /// Native `PR_CLEAR_STATION_TRUCK`.
    pub const PR_CLEAR_STATION_TRUCK: Self = Self(38);
    /// Native `PR_CLEAR_STATION_DOCK`.
    pub const PR_CLEAR_STATION_DOCK: Self = Self(39);
    /// Native `PR_CLEAR_HOUSE`.
    pub const PR_CLEAR_HOUSE: Self = Self(40);
    /// Native `PR_CLEAR_ROAD`.
    pub const PR_CLEAR_ROAD: Self = Self(41);
    /// Native `PR_RUNNING_TRAIN_STEAM`.
    pub const PR_RUNNING_TRAIN_STEAM: Self = Self(42);
    /// Native `PR_RUNNING_TRAIN_DIESEL`.
    pub const PR_RUNNING_TRAIN_DIESEL: Self = Self(43);
    /// Native `PR_RUNNING_TRAIN_ELECTRIC`.
    pub const PR_RUNNING_TRAIN_ELECTRIC: Self = Self(44);
    /// Native `PR_RUNNING_AIRCRAFT`.
    pub const PR_RUNNING_AIRCRAFT: Self = Self(45);
    /// Native `PR_RUNNING_ROADVEH`.
    pub const PR_RUNNING_ROADVEH: Self = Self(46);
    /// Native `PR_RUNNING_SHIP`.
    pub const PR_RUNNING_SHIP: Self = Self(47);
    /// Native `PR_BUILD_INDUSTRY`.
    pub const PR_BUILD_INDUSTRY: Self = Self(48);
    /// Native `PR_CLEAR_INDUSTRY`.
    pub const PR_CLEAR_INDUSTRY: Self = Self(49);
    /// Native `PR_BUILD_OBJECT`.
    pub const PR_BUILD_OBJECT: Self = Self(50);
    /// Native `PR_CLEAR_OBJECT`.
    pub const PR_CLEAR_OBJECT: Self = Self(51);
    /// Native `PR_BUILD_WAYPOINT_RAIL`.
    pub const PR_BUILD_WAYPOINT_RAIL: Self = Self(52);
    /// Native `PR_CLEAR_WAYPOINT_RAIL`.
    pub const PR_CLEAR_WAYPOINT_RAIL: Self = Self(53);
    /// Native `PR_BUILD_WAYPOINT_BUOY`.
    pub const PR_BUILD_WAYPOINT_BUOY: Self = Self(54);
    /// Native `PR_CLEAR_WAYPOINT_BUOY`.
    pub const PR_CLEAR_WAYPOINT_BUOY: Self = Self(55);
    /// Native `PR_TOWN_ACTION`.
    pub const PR_TOWN_ACTION: Self = Self(56);
    /// Native `PR_BUILD_FOUNDATION`.
    pub const PR_BUILD_FOUNDATION: Self = Self(57);
    /// Native `PR_BUILD_INDUSTRY_RAW`.
    pub const PR_BUILD_INDUSTRY_RAW: Self = Self(58);
    /// Native `PR_BUILD_TOWN`.
    pub const PR_BUILD_TOWN: Self = Self(59);
    /// Native `PR_BUILD_CANAL`.
    pub const PR_BUILD_CANAL: Self = Self(60);
    /// Native `PR_CLEAR_CANAL`.
    pub const PR_CLEAR_CANAL: Self = Self(61);
    /// Native `PR_BUILD_AQUEDUCT`.
    pub const PR_BUILD_AQUEDUCT: Self = Self(62);
    /// Native `PR_CLEAR_AQUEDUCT`.
    pub const PR_CLEAR_AQUEDUCT: Self = Self(63);
    /// Native `PR_BUILD_LOCK`.
    pub const PR_BUILD_LOCK: Self = Self(64);
    /// Native `PR_CLEAR_LOCK`.
    pub const PR_CLEAR_LOCK: Self = Self(65);
    /// Native `PR_INFRASTRUCTURE_RAIL`.
    pub const PR_INFRASTRUCTURE_RAIL: Self = Self(66);
    /// Native `PR_INFRASTRUCTURE_ROAD`.
    pub const PR_INFRASTRUCTURE_ROAD: Self = Self(67);
    /// Native `PR_INFRASTRUCTURE_WATER`.
    pub const PR_INFRASTRUCTURE_WATER: Self = Self(68);
    /// Native `PR_INFRASTRUCTURE_STATION`.
    pub const PR_INFRASTRUCTURE_STATION: Self = Self(69);
    /// Native `PR_INFRASTRUCTURE_AIRPORT`.
    pub const PR_INFRASTRUCTURE_AIRPORT: Self = Self(70);
    /// Native `PR_END`.
    pub const PR_END: Self = Self(71);
    /// Native `INVALID_PRICE`.
    pub const INVALID_PRICE: Self = Self(255);
}

impl ExpensesType {
    /// Construction costs.
    pub const EXPENSES_CONSTRUCTION: Self = Self(0);
    /// New vehicles.
    pub const EXPENSES_NEW_VEHICLES: Self = Self(1);
    /// Running costs trains.
    pub const EXPENSES_TRAIN_RUN: Self = Self(2);
    /// Running costs road vehicles.
    pub const EXPENSES_ROADVEH_RUN: Self = Self(3);
    /// Running costs aircraft.
    pub const EXPENSES_AIRCRAFT_RUN: Self = Self(4);
    /// Running costs ships.
    pub const EXPENSES_SHIP_RUN: Self = Self(5);
    /// Property costs.
    pub const EXPENSES_PROPERTY: Self = Self(6);
    /// Revenue from trains.
    pub const EXPENSES_TRAIN_REVENUE: Self = Self(7);
    /// Revenue from road vehicles.
    pub const EXPENSES_ROADVEH_REVENUE: Self = Self(8);
    /// Revenue from aircraft.
    pub const EXPENSES_AIRCRAFT_REVENUE: Self = Self(9);
    /// Revenue from ships.
    pub const EXPENSES_SHIP_REVENUE: Self = Self(10);
    /// Interest payments over the loan.
    pub const EXPENSES_LOAN_INTEREST: Self = Self(11);
    /// Other expenses.
    pub const EXPENSES_OTHER: Self = Self(12);
    /// Number of expense types.
    pub const EXPENSES_END: Self = Self(13);
    /// Invalid expense type.
    pub const INVALID_EXPENSES: Self = Self(255);
}

/// Native `CargoPaymentID`, retaining every underlying `u32` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CargoPaymentID(pub u32);

impl CargoPaymentID {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(1_044_480);
    /// Native `PoolID::Invalid()`.
    pub const INVALID: Self = Self(1_048_575);
}

impl Price {
    /// Index into native price arrays.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}
