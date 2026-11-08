/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared named constants from `subsidy_base.h`.
//!
//! The original declares each constant `uint`. Each Rust width is the width of the value it
//! initialises or is compared with, so every use keeps the original's result.
/// Duration of subsidy offer
pub const SUBSIDY_OFFER_MONTHS: u16 = 12;
/// Min. population of towns for subsidised pax route
pub const SUBSIDY_PAX_MIN_POPULATION: u32 = 400;
/// Min. population of destination town for cargo route
pub const SUBSIDY_CARGO_MIN_POPULATION: u32 = 900;
/// Subsidy will be created only for towns/industries with less % transported
pub const SUBSIDY_MAX_PCT_TRANSPORTED: u8 = 42;
/// Max. length of subsidised route (DistanceManhattan)
pub const SUBSIDY_MAX_DISTANCE: u32 = 70;
/// Extent of a tile area around town center when scanning for town cargo acceptance and
/// production (6 ~= min catchment + min station / 2)
pub const SUBSIDY_TOWN_CARGO_RADIUS: i32 = 6;
