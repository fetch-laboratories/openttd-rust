/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `command_type.h` command identifiers, flags, types and pause levels.
//! `services.cpp` asserts every value.

/// `Commands::CMD_COMPANY_CTRL`.
pub const CMD_COMPANY_CTRL: u8 = 90;

/// `CommandFlag` values as `CommandFlags` bits.
pub struct CommandFlag;
impl CommandFlag {
    /// `CommandFlag::Server`.
    pub const SERVER: u16 = 1 << 0;
    /// `CommandFlag::Spectator`.
    pub const SPECTATOR: u16 = 1 << 1;
    /// `CommandFlag::Auto`.
    pub const AUTO: u16 = 1 << 3;
    /// `CommandFlag::AllTiles`.
    pub const ALL_TILES: u16 = 1 << 4;
    /// `CommandFlag::NoTest`.
    pub const NO_TEST: u16 = 1 << 5;
    /// `CommandFlag::NoWater`.
    pub const NO_WATER: u16 = 1 << 6;
    /// `CommandFlag::ClientID`.
    pub const CLIENT_ID: u16 = 1 << 7;
    /// `CommandFlag::Deity`.
    pub const DEITY: u16 = 1 << 8;
    /// `CommandFlag::NoEst`.
    pub const NO_EST: u16 = 1 << 10;
}

/// `CommandType::ServerSetting`.
pub const COMMAND_TYPE_SERVER_SETTING: u8 = 7;

/// `IsCommandAllowedWhilePaused` lookup: `CommandPauseLevel` per `CommandType`.
pub const COMMAND_TYPE_PAUSE_LEVEL: [u8; 9] = [3, 2, 2, 1, 1, 1, 0, 0, 0];

/// `CommandFlagsToDCFlags`.
#[must_use]
pub const fn command_flags_to_dc_flags(flags: u16) -> u16 {
    let mut out = 0;
    if flags & CommandFlag::NO_WATER != 0 {
        out |= DoCommandFlag::NO_WATER;
    }
    if flags & CommandFlag::AUTO != 0 {
        out |= DoCommandFlag::AUTO;
    }
    if flags & CommandFlag::ALL_TILES != 0 {
        out |= DoCommandFlag::ALL_TILES;
    }
    out
}

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
    /// `DoCommandFlag::AutoReplace`.
    pub const AUTO_REPLACE: u16 = 1 << 6;
    /// `DoCommandFlag::NoCargoCapacityCheck`.
    pub const NO_CARGO_CAPACITY_CHECK: u16 = 1 << 7;
    /// `DoCommandFlag::AllTiles`.
    pub const ALL_TILES: u16 = 1 << 8;
    /// `DoCommandFlag::NoModifyTownRating`.
    pub const NO_MODIFY_TOWN_RATING: u16 = 1 << 9;
    /// `DoCommandFlag::ForceClearTile`.
    pub const FORCE_CLEAR_TILE: u16 = 1 << 10;
}
