/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared vehicle property IDs from `newgrf_properties.h`.
pub const PROP_TRAIN_POWER: u8 = 0x0b;
pub const PROP_TRAIN_WEIGHT: u8 = 0x16;
pub const PROP_TRAIN_TRACTIVE_EFFORT: u8 = 0x1f;
pub const PROP_ROADVEH_POWER: u8 = 0x13;
pub const PROP_ROADVEH_WEIGHT: u8 = 0x14;
pub const PROP_ROADVEH_TRACTIVE_EFFORT: u8 = 0x18;

pub const PROP_VEHICLE_LOAD_AMOUNT: u8 = 0x07;
pub const PROP_TRAIN_CURVE_SPEED_MOD: u8 = 0x2e;
pub const PROP_ROADVEH_CARGO_AGE_PERIOD: u8 = 0x22;
pub const PROP_SHIP_SPEED: u8 = 0x0b;
pub const PROP_SHIP_CARGO_AGE_PERIOD: u8 = 0x1d;
pub const PROP_AIRCRAFT_CARGO_AGE_PERIOD: u8 = 0x1c;
pub const PROP_AIRCRAFT_RANGE: u8 = 0x1f;

pub const PROP_TRAIN_CARGO_CAPACITY: u8 = 0x14;
pub const PROP_ROADVEH_CARGO_CAPACITY: u8 = 0x0f;
pub const PROP_SHIP_CARGO_CAPACITY: u8 = 0x0d;
pub const PROP_AIRCRAFT_PASSENGER_CAPACITY: u8 = 0x0f;
pub const PROP_AIRCRAFT_MAIL_CAPACITY: u8 = 0x11;

pub const PROP_TRAIN_SPEED: u8 = 0x09;
pub const PROP_TRAIN_RUNNING_COST_FACTOR: u8 = 0x0d;
pub const PROP_TRAIN_COST_FACTOR: u8 = 0x17;
pub const PROP_TRAIN_SHORTEN_FACTOR: u8 = 0x21;
pub const PROP_TRAIN_USER_DATA: u8 = 0x25;
pub const PROP_TRAIN_CARGO_AGE_PERIOD: u8 = 0x2b;

pub const PROP_ROADVEH_RUNNING_COST_FACTOR: u8 = 0x09;
pub const PROP_ROADVEH_COST_FACTOR: u8 = 0x11;
pub const PROP_ROADVEH_SPEED: u8 = 0x15;
pub const PROP_ROADVEH_SHORTEN_FACTOR: u8 = 0x23;
pub const PROP_SHIP_COST_FACTOR: u8 = 0x0a;
pub const PROP_SHIP_RUNNING_COST_FACTOR: u8 = 0x0f;
pub const PROP_AIRCRAFT_COST_FACTOR: u8 = 0x0b;
pub const PROP_AIRCRAFT_SPEED: u8 = 0x0c;
pub const PROP_AIRCRAFT_RUNNING_COST_FACTOR: u8 = 0x0e;
