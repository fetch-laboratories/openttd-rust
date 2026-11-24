/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `newgrf_properties.h`: properties of `NewGRF` Action 0.

/// `PropertyID`: list of `NewGRF` properties used in Action 0 or Callback 0x36
/// (`CBID_VEHICLE_MODIFY_PROPERTY`). Names are formatted as `PROP_<CLASS>_<NAME>`.
/// The list only contains properties which are used more than once in the code, that is,
/// those available for callback 0x36.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PropertyID(pub u8);

impl PropertyID {
    /// Loading speed.
    pub const PROP_VEHICLE_LOAD_AMOUNT: Self = Self(0x07);
    /// Max. speed: 1 unit = 1/1.6 mph = 1 km-ish/h.
    pub const PROP_TRAIN_SPEED: Self = Self(0x09);
    /// Power in hp (if dualheaded: sum of both vehicles).
    pub const PROP_TRAIN_POWER: Self = Self(0x0b);
    /// Yearly runningcost (if dualheaded: sum of both vehicles).
    pub const PROP_TRAIN_RUNNING_COST_FACTOR: Self = Self(0x0d);
    /// Capacity (if dualheaded: for each single vehicle).
    pub const PROP_TRAIN_CARGO_CAPACITY: Self = Self(0x14);
    /// Weight in t (if dualheaded: for each single vehicle).
    pub const PROP_TRAIN_WEIGHT: Self = Self(0x16);
    /// Purchase cost (if dualheaded: sum of both vehicles).
    pub const PROP_TRAIN_COST_FACTOR: Self = Self(0x17);
    /// Tractive effort coefficient in 1/256.
    pub const PROP_TRAIN_TRACTIVE_EFFORT: Self = Self(0x1f);
    /// Shorter vehicles.
    pub const PROP_TRAIN_SHORTEN_FACTOR: Self = Self(0x21);
    /// User defined data for vehicle variable 0x42.
    pub const PROP_TRAIN_USER_DATA: Self = Self(0x25);
    /// Number of ticks before carried cargo is aged.
    pub const PROP_TRAIN_CARGO_AGE_PERIOD: Self = Self(0x2b);
    /// Modifier to maximum speed in curves.
    pub const PROP_TRAIN_CURVE_SPEED_MOD: Self = Self(0x2e);
    /// Yearly runningcost.
    pub const PROP_ROADVEH_RUNNING_COST_FACTOR: Self = Self(0x09);
    /// Capacity.
    pub const PROP_ROADVEH_CARGO_CAPACITY: Self = Self(0x0f);
    /// Purchase cost.
    pub const PROP_ROADVEH_COST_FACTOR: Self = Self(0x11);
    /// Power in 10 HP.
    pub const PROP_ROADVEH_POWER: Self = Self(0x13);
    /// Weight in 1/4 t.
    pub const PROP_ROADVEH_WEIGHT: Self = Self(0x14);
    /// Max. speed: 1 unit = 1/0.8 mph = 2 km-ish/h.
    pub const PROP_ROADVEH_SPEED: Self = Self(0x15);
    /// Tractive effort coefficient in 1/256.
    pub const PROP_ROADVEH_TRACTIVE_EFFORT: Self = Self(0x18);
    /// Number of ticks before carried cargo is aged.
    pub const PROP_ROADVEH_CARGO_AGE_PERIOD: Self = Self(0x22);
    /// Shorter vehicles.
    pub const PROP_ROADVEH_SHORTEN_FACTOR: Self = Self(0x23);
    /// Purchase cost.
    pub const PROP_SHIP_COST_FACTOR: Self = Self(0x0a);
    /// Max. speed: 1 unit = 1/3.2 mph = 0.5 km-ish/h.
    pub const PROP_SHIP_SPEED: Self = Self(0x0b);
    /// Capacity.
    pub const PROP_SHIP_CARGO_CAPACITY: Self = Self(0x0d);
    /// Yearly runningcost.
    pub const PROP_SHIP_RUNNING_COST_FACTOR: Self = Self(0x0f);
    /// Number of ticks before carried cargo is aged.
    pub const PROP_SHIP_CARGO_AGE_PERIOD: Self = Self(0x1d);
    /// Purchase cost.
    pub const PROP_AIRCRAFT_COST_FACTOR: Self = Self(0x0b);
    /// Max. speed: 1 unit = 8 mph = 12.8 km-ish/h.
    pub const PROP_AIRCRAFT_SPEED: Self = Self(0x0c);
    /// Yearly runningcost.
    pub const PROP_AIRCRAFT_RUNNING_COST_FACTOR: Self = Self(0x0e);
    /// Passenger Capacity.
    pub const PROP_AIRCRAFT_PASSENGER_CAPACITY: Self = Self(0x0f);
    /// Mail Capacity.
    pub const PROP_AIRCRAFT_MAIL_CAPACITY: Self = Self(0x11);
    /// Number of ticks before carried cargo is aged.
    pub const PROP_AIRCRAFT_CARGO_AGE_PERIOD: Self = Self(0x1c);
    /// Aircraft range.
    pub const PROP_AIRCRAFT_RANGE: Self = Self(0x1f);
}

// Raw values for the files no pass has finished; #734's closure removes them.
pub const PROP_VEHICLE_LOAD_AMOUNT: u8 = PropertyID::PROP_VEHICLE_LOAD_AMOUNT.0;
pub const PROP_TRAIN_SPEED: u8 = PropertyID::PROP_TRAIN_SPEED.0;
pub const PROP_TRAIN_POWER: u8 = PropertyID::PROP_TRAIN_POWER.0;
pub const PROP_TRAIN_RUNNING_COST_FACTOR: u8 = PropertyID::PROP_TRAIN_RUNNING_COST_FACTOR.0;
pub const PROP_TRAIN_CARGO_CAPACITY: u8 = PropertyID::PROP_TRAIN_CARGO_CAPACITY.0;
pub const PROP_TRAIN_WEIGHT: u8 = PropertyID::PROP_TRAIN_WEIGHT.0;
pub const PROP_TRAIN_COST_FACTOR: u8 = PropertyID::PROP_TRAIN_COST_FACTOR.0;
pub const PROP_TRAIN_TRACTIVE_EFFORT: u8 = PropertyID::PROP_TRAIN_TRACTIVE_EFFORT.0;
pub const PROP_TRAIN_SHORTEN_FACTOR: u8 = PropertyID::PROP_TRAIN_SHORTEN_FACTOR.0;
pub const PROP_TRAIN_USER_DATA: u8 = PropertyID::PROP_TRAIN_USER_DATA.0;
pub const PROP_TRAIN_CARGO_AGE_PERIOD: u8 = PropertyID::PROP_TRAIN_CARGO_AGE_PERIOD.0;
pub const PROP_TRAIN_CURVE_SPEED_MOD: u8 = PropertyID::PROP_TRAIN_CURVE_SPEED_MOD.0;
pub const PROP_ROADVEH_RUNNING_COST_FACTOR: u8 = PropertyID::PROP_ROADVEH_RUNNING_COST_FACTOR.0;
pub const PROP_ROADVEH_CARGO_CAPACITY: u8 = PropertyID::PROP_ROADVEH_CARGO_CAPACITY.0;
pub const PROP_ROADVEH_COST_FACTOR: u8 = PropertyID::PROP_ROADVEH_COST_FACTOR.0;
pub const PROP_ROADVEH_POWER: u8 = PropertyID::PROP_ROADVEH_POWER.0;
pub const PROP_ROADVEH_WEIGHT: u8 = PropertyID::PROP_ROADVEH_WEIGHT.0;
pub const PROP_ROADVEH_SPEED: u8 = PropertyID::PROP_ROADVEH_SPEED.0;
pub const PROP_ROADVEH_TRACTIVE_EFFORT: u8 = PropertyID::PROP_ROADVEH_TRACTIVE_EFFORT.0;
pub const PROP_ROADVEH_CARGO_AGE_PERIOD: u8 = PropertyID::PROP_ROADVEH_CARGO_AGE_PERIOD.0;
pub const PROP_ROADVEH_SHORTEN_FACTOR: u8 = PropertyID::PROP_ROADVEH_SHORTEN_FACTOR.0;
pub const PROP_SHIP_COST_FACTOR: u8 = PropertyID::PROP_SHIP_COST_FACTOR.0;
pub const PROP_SHIP_SPEED: u8 = PropertyID::PROP_SHIP_SPEED.0;
pub const PROP_SHIP_CARGO_CAPACITY: u8 = PropertyID::PROP_SHIP_CARGO_CAPACITY.0;
pub const PROP_SHIP_RUNNING_COST_FACTOR: u8 = PropertyID::PROP_SHIP_RUNNING_COST_FACTOR.0;
pub const PROP_SHIP_CARGO_AGE_PERIOD: u8 = PropertyID::PROP_SHIP_CARGO_AGE_PERIOD.0;
pub const PROP_AIRCRAFT_COST_FACTOR: u8 = PropertyID::PROP_AIRCRAFT_COST_FACTOR.0;
pub const PROP_AIRCRAFT_SPEED: u8 = PropertyID::PROP_AIRCRAFT_SPEED.0;
pub const PROP_AIRCRAFT_RUNNING_COST_FACTOR: u8 = PropertyID::PROP_AIRCRAFT_RUNNING_COST_FACTOR.0;
pub const PROP_AIRCRAFT_PASSENGER_CAPACITY: u8 = PropertyID::PROP_AIRCRAFT_PASSENGER_CAPACITY.0;
pub const PROP_AIRCRAFT_MAIL_CAPACITY: u8 = PropertyID::PROP_AIRCRAFT_MAIL_CAPACITY.0;
pub const PROP_AIRCRAFT_CARGO_AGE_PERIOD: u8 = PropertyID::PROP_AIRCRAFT_CARGO_AGE_PERIOD.0;
pub const PROP_AIRCRAFT_RANGE: u8 = PropertyID::PROP_AIRCRAFT_RANGE.0;
