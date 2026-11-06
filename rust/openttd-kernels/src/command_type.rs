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
/// `Commands::CMD_COMPANY_ALLOW_LIST_CTRL`.
pub const CMD_COMPANY_ALLOW_LIST_CTRL: u8 = 91;
/// `Commands::CMD_RENAME_PRESIDENT`.
pub const CMD_RENAME_PRESIDENT: u8 = 63;
/// `Commands::CMD_PAUSE`.
pub const CMD_PAUSE: u8 = 71;
/// `Commands::CMD_SET_COMPANY_COLOUR`.
pub const CMD_SET_COMPANY_COLOUR: u8 = 54;
/// `CommandType::CompanySetting`.
pub const COMMAND_TYPE_COMPANY_SETTING: u8 = 6;
/// `Commands::CMD_END`.
pub const CMD_END: u8 = 145;

/// `CommandFlag` values as `CommandFlags` bits.

impl CommandFlag {
    /// `CommandFlag::Server`.
    pub const SERVER: u16 = CommandFlags::from_element(Self::Server).0;
    /// `CommandFlag::Spectator`.
    pub const SPECTATOR: u16 = CommandFlags::from_element(Self::Spectator).0;
    /// `CommandFlag::Offline`.
    pub const OFFLINE: u16 = CommandFlags::from_element(Self::Offline).0;
    /// `CommandFlag::Auto`.
    pub const AUTO: u16 = CommandFlags::from_element(Self::Auto).0;
    /// `CommandFlag::AllTiles`.
    pub const ALL_TILES: u16 = CommandFlags::from_element(Self::AllTiles).0;
    /// `CommandFlag::NoWater`.
    pub const NO_WATER: u16 = CommandFlags::from_element(Self::NoWater).0;
    /// `CommandFlag::ClientID`.
    pub const CLIENT_ID: u16 = CommandFlags::from_element(Self::ClientID).0;
    /// `CommandFlag::NoTest`.
    pub const NO_TEST: u16 = CommandFlags::from_element(Self::NoTest).0;
    /// `CommandFlag::Deity`.
    pub const DEITY: u16 = CommandFlags::from_element(Self::Deity).0;
    /// `CommandFlag::NoEst`.
    pub const NO_EST: u16 = CommandFlags::from_element(Self::NoEst).0;
}

/// `CommandType::ServerSetting`.
pub const COMMAND_TYPE_SERVER_SETTING: u8 = 7;

/// `IsCommandAllowedWhilePaused` lookup: `CommandPauseLevel` per `CommandType`.
pub const COMMAND_TYPE_PAUSE_LEVEL: [u8; 9] = [3, 2, 2, 1, 1, 1, 0, 0, 0];

/// `DoCommandFlag` values as `DoCommandFlags` bits.

impl DoCommandFlag {
    /// `DoCommandFlag::Execute`.
    pub const EXECUTE: u16 = DoCommandFlags::from_element(Self::Execute).0;
    /// `DoCommandFlag::Auto`.
    /// `CommandFlag::Auto`.
    pub const AUTO: u16 = DoCommandFlags::from_element(Self::Auto).0;
    /// `DoCommandFlag::QueryCost`.
    pub const QUERY_COST: u16 = DoCommandFlags::from_element(Self::QueryCost).0;
    /// `DoCommandFlag::NoWater`.
    /// `CommandFlag::NoWater`.
    pub const NO_WATER: u16 = DoCommandFlags::from_element(Self::NoWater).0;
    /// `DoCommandFlag::NoTestTownRating`.
    pub const NO_TEST_TOWN_RATING: u16 = DoCommandFlags::from_element(Self::NoTestTownRating).0;
    /// `DoCommandFlag::Bankrupt`.
    pub const BANKRUPT: u16 = DoCommandFlags::from_element(Self::Bankrupt).0;
    /// `DoCommandFlag::AutoReplace`.
    pub const AUTO_REPLACE: u16 = DoCommandFlags::from_element(Self::AutoReplace).0;
    /// `DoCommandFlag::NoCargoCapacityCheck`.
    pub const NO_CARGO_CAPACITY_CHECK: u16 =
        DoCommandFlags::from_element(Self::NoCargoCapacityCheck).0;
    /// `DoCommandFlag::AllTiles`.
    /// `CommandFlag::AllTiles`.
    pub const ALL_TILES: u16 = DoCommandFlags::from_element(Self::AllTiles).0;
    /// `DoCommandFlag::NoModifyTownRating`.
    pub const NO_MODIFY_TOWN_RATING: u16 = DoCommandFlags::from_element(Self::NoModifyTownRating).0;
    /// `DoCommandFlag::ForceClearTile`.
    pub const FORCE_CLEAR_TILE: u16 = DoCommandFlags::from_element(Self::ForceClearTile).0;
}

/// Native `DoCommandFlag`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DoCommandFlag(pub u8);

#[allow(non_upper_case_globals)]
impl DoCommandFlag {
    /// execute the given command
    pub const Execute: Self = Self(0);
    /// don't allow building on structures
    pub const Auto: Self = Self(1);
    /// query cost only,  don't build.
    pub const QueryCost: Self = Self(2);
    /// don't allow building on water
    pub const NoWater: Self = Self(3);
    /// town rating does not disallow you from building
    pub const NoTestTownRating: Self = Self(4);
    /// company bankrupts, skip money check, skip vehicle on tile check in some cases
    pub const Bankrupt: Self = Self(5);
    /// autoreplace/autorenew is in progress, this shall disable vehicle limits when building, and ignore certain restrictions when undoing things (like vehicle attach callback)
    pub const AutoReplace: Self = Self(6);
    /// when autoreplace/autorenew is in progress, this shall prevent truncating the amount of cargo in the vehicle to prevent testing the command to remove cargo
    pub const NoCargoCapacityCheck: Self = Self(7);
    /// allow this command also on MP_VOID tiles
    pub const AllTiles: Self = Self(8);
    /// do not change town rating
    pub const NoModifyTownRating: Self = Self(9);
    /// do not only remove the object on the tile, but also clear any water left on it
    pub const ForceClearTile: Self = Self(10);
}

/// Native `CommandFlag`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandFlag(pub u8);

#[allow(non_upper_case_globals)]
impl CommandFlag {
    /// the command can only be initiated by the server
    pub const Server: Self = Self(0);
    /// the command may be initiated by a spectator
    pub const Spectator: Self = Self(1);
    /// the command cannot be executed in a multiplayer game; single-player only
    pub const Offline: Self = Self(2);
    /// set the DoCommandFlag::Auto flag on this command
    pub const Auto: Self = Self(3);
    /// allow this command also on MP_VOID tiles
    pub const AllTiles: Self = Self(4);
    /// the command's output may differ between test and execute due to town rating changes etc.
    pub const NoTest: Self = Self(5);
    /// set the DoCommandFlag::NoWater flag on this command
    pub const NoWater: Self = Self(6);
    /// set p2 with the ClientID of the sending client.
    pub const ClientID: Self = Self(7);
    /// the command may be executed by COMPANY_DEITY
    pub const Deity: Self = Self(8);
    /// the command's string may contain control strings
    pub const StrCtrl: Self = Self(9);
    /// the command is never estimated.
    pub const NoEst: Self = Self(10);
    /// the command has implicit location argument.
    pub const Location: Self = Self(11);
}

/// Native `CommandType`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandType(pub u8);

#[allow(non_upper_case_globals)]
impl CommandType {
    /// Construction and destruction of objects on the map.
    pub const LandscapeConstruction: Self = Self(0);
    /// Construction, modification (incl. refit) and destruction of vehicles.
    pub const VehicleConstruction: Self = Self(1);
    /// Management of money, i.e. loans.
    pub const MoneyManagement: Self = Self(2);
    /// Stopping, starting, sending to depot, turning around, replace orders etc.
    pub const VehicleManagement: Self = Self(3);
    /// Modifications to route management (orders, groups, etc).
    pub const RouteManagement: Self = Self(4);
    /// Renaming stuff, changing company colours, placing signs, etc.
    pub const OtherManagement: Self = Self(5);
    /// Changing settings related to a company.
    pub const CompanySetting: Self = Self(6);
    /// Pausing/removing companies/server settings.
    pub const ServerSetting: Self = Self(7);
    /// A cheat of some sorts.
    pub const Cheat: Self = Self(8);
    /// End marker.
    pub const End: Self = Self(9);
}

/// Native `DoCommandFlags`, retaining every underlying `u16` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DoCommandFlags(pub u16);

impl DoCommandFlags {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Set containing the named element (the element is a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: DoCommandFlag) -> Self {
        Self((1_u64 << element.0) as u16)
    }
}

/// Native `CommandFlags`, retaining every underlying `u16` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandFlags(pub u16);

impl CommandFlags {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Set containing the named element (the element is a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: CommandFlag) -> Self {
        Self((1_u64 << element.0) as u16)
    }
}
