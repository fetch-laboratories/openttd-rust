/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared named constants from `subsidy_base.h`.
/// Duration of an unawarded offer in months.
pub const SUBSIDY_OFFER_MONTHS: u16 = 12;
/// Minimum passenger-source and destination population.
pub const SUBSIDY_PAX_MIN_POPULATION: u32 = 400;
/// Minimum town cargo-source population.
pub const SUBSIDY_CARGO_MIN_POPULATION: u32 = 900;
/// Largest transported percentage allowed for an offer.
pub const SUBSIDY_MAX_PCT_TRANSPORTED: u8 = 42;
/// Inclusive Manhattan route-distance limit.
pub const SUBSIDY_MAX_DISTANCE: u32 = 70;
/// Town centre scan radius.
pub const SUBSIDY_TOWN_CARGO_RADIUS: i32 = 6;
