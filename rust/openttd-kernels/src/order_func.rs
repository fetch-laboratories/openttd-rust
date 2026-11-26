/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! The service interval defaults and limits of `order_func.h`, each a `static const uint`.

/// Default service interval of trains, in days.
pub const DEF_SERVINT_DAYS_TRAINS: u32 = 150;
/// Default service interval of road vehicles, in days.
pub const DEF_SERVINT_DAYS_ROADVEH: u32 = 150;
/// Default service interval of aircraft, in days.
pub const DEF_SERVINT_DAYS_AIRCRAFT: u32 = 100;
/// Default service interval of ships, in days.
pub const DEF_SERVINT_DAYS_SHIPS: u32 = 360;
/// Shortest service interval in days.
pub const MIN_SERVINT_DAYS: u32 = 30;
/// Longest service interval in days.
pub const MAX_SERVINT_DAYS: u32 = 800;

/// Default service interval of trains, in minutes.
pub const DEF_SERVINT_MINUTES_TRAINS: u32 = 5;
/// Default service interval of road vehicles, in minutes.
pub const DEF_SERVINT_MINUTES_ROADVEH: u32 = 5;
/// Default service interval of aircraft, in minutes.
pub const DEF_SERVINT_MINUTES_AIRCRAFT: u32 = 3;
/// Default service interval of ships, in minutes.
pub const DEF_SERVINT_MINUTES_SHIPS: u32 = 12;
/// Shortest service interval in minutes.
pub const MIN_SERVINT_MINUTES: u32 = 1;
/// Longest service interval in minutes.
pub const MAX_SERVINT_MINUTES: u32 = 30;

/// Default service interval in percent.
pub const DEF_SERVINT_PERCENT: u32 = 50;
/// Smallest service interval in percent.
pub const MIN_SERVINT_PERCENT: u32 = 5;
/// Largest service interval in percent.
pub const MAX_SERVINT_PERCENT: u32 = 90;
