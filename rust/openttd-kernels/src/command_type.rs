/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `command_type.h` `DoCommandFlag` bits of the 16-bit `DoCommandFlags`.
//! `services.cpp` asserts every value.

/// `DoCommandFlag` values as `DoCommandFlags` bits.
pub struct DoCommandFlag;
impl DoCommandFlag {
    /// `DoCommandFlag::Execute`.
    pub const EXECUTE: u16 = 1 << 0;
    /// `DoCommandFlag::Auto`.
    pub const AUTO: u16 = 1 << 1;
    /// `DoCommandFlag::QueryCost`.
    pub const QUERY_COST: u16 = 1 << 2;
    /// `DoCommandFlag::NoWater`.
    pub const NO_WATER: u16 = 1 << 3;
    /// `DoCommandFlag::NoTestTownRating`.
    pub const NO_TEST_TOWN_RATING: u16 = 1 << 4;
    /// `DoCommandFlag::Bankrupt`.
    pub const BANKRUPT: u16 = 1 << 5;
    /// `DoCommandFlag::NoModifyTownRating`.
    pub const NO_MODIFY_TOWN_RATING: u16 = 1 << 9;
    /// `DoCommandFlag::ForceClearTile`.
    pub const FORCE_CLEAR_TILE: u16 = 1 << 10;
}
