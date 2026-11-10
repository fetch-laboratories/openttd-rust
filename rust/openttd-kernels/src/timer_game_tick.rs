/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Tick durations from `timer_game_tick.h`.
pub const TOWN_GROWTH_TICKS: u16 = 70;
/// `Ticks::DAY_TICKS`.
pub const DAY_TICKS: i32 = 74;
/// `Ticks::TICKS_PER_SECOND`.
pub const TICKS_PER_SECOND: i32 = 37;
/// `Ticks::INDUSTRY_PRODUCE_TICKS`.
pub const INDUSTRY_PRODUCE_TICKS: u32 = 256;

/// `Ticks::STATION_RATING_TICKS`: cycle duration for updating station rating.
pub const STATION_RATING_TICKS: i32 = 185;
/// `Ticks::STATION_ACCEPTANCE_TICKS`: cycle duration for updating station acceptance.
pub const STATION_ACCEPTANCE_TICKS: i32 = 250;
/// `Ticks::STATION_LINKGRAPH_TICKS`: cycle duration for cleaning dead links.
pub const STATION_LINKGRAPH_TICKS: i32 = 504;

/// `Ticks::CARGO_AGING_TICKS`.
pub const CARGO_AGING_TICKS: u16 = 185;

/// Cycle duration for the lumber mill's extra action.
pub const INDUSTRY_CUT_TREE_TICKS: u32 = INDUSTRY_PRODUCE_TICKS * 2;
