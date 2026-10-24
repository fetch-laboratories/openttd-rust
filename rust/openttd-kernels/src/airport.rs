/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `airport.h` values. `services.cpp` asserts every value.

/// `HANGAR`.
pub const HANGAR: u8 = 1;
/// `AT_OILRIG`.
pub const AT_OILRIG: u8 = 9;

pub const TERM1: u8 = 2;
pub const TERM2: u8 = 3;
pub const TERM3: u8 = 4;
pub const TERM4: u8 = 5;
pub const TERM5: u8 = 6;
pub const TERM6: u8 = 7;
pub const HELIPAD1: u8 = 8;
pub const HELIPAD2: u8 = 9;
pub const TAKEOFF: u8 = 10;
pub const STARTTAKEOFF: u8 = 11;
pub const ENDTAKEOFF: u8 = 12;
pub const HELITAKEOFF: u8 = 13;
pub const FLYING: u8 = 14;
pub const LANDING: u8 = 15;
pub const ENDLANDING: u8 = 16;
pub const HELILANDING: u8 = 17;
pub const HELIENDLANDING: u8 = 18;
pub const TERM7: u8 = 19;
pub const TERM8: u8 = 20;
pub const HELIPAD3: u8 = 21;
pub const AMD_SLOW_TURN: u8 = 2;
pub const AMD_EXACT_POSITION: u8 = 4;
pub const AMD_BRAKE: u8 = 5;
pub const AMD_HELI_RAISE: u8 = 6;
pub const AMD_HELI_LOWER: u8 = 7;
pub const AMD_HOLD: u8 = 8;
pub const ATP_TTDP_LARGE: u8 = 1;
