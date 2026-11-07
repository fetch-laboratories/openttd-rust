/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `command_type.h` command identifiers, flags, types and pause levels.
//! `services.cpp` asserts every value.

/// `Commands::CMD_COMPANY_CTRL`.
pub const CMD_COMPANY_CTRL: u8 = Commands::CMD_COMPANY_CTRL.0;
/// `Commands::CMD_COMPANY_ALLOW_LIST_CTRL`.
pub const CMD_COMPANY_ALLOW_LIST_CTRL: u8 = Commands::CMD_COMPANY_ALLOW_LIST_CTRL.0;
/// `Commands::CMD_RENAME_PRESIDENT`.
pub const CMD_RENAME_PRESIDENT: u8 = Commands::CMD_RENAME_PRESIDENT.0;
/// `Commands::CMD_PAUSE`.
pub const CMD_PAUSE: u8 = Commands::CMD_PAUSE.0;
/// `Commands::CMD_SET_COMPANY_COLOUR`.
pub const CMD_SET_COMPANY_COLOUR: u8 = Commands::CMD_SET_COMPANY_COLOUR.0;
/// `CommandType::CompanySetting`.
pub const COMMAND_TYPE_COMPANY_SETTING: u8 = CommandType::CompanySetting.0;
/// `Commands::CMD_END`.
pub const CMD_END: u8 = Commands::CMD_END.0;

/// Primitive mask adapters retained for existing consumers.
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
pub const COMMAND_TYPE_SERVER_SETTING: u8 = CommandType::ServerSetting.0;

/// `IsCommandAllowedWhilePaused` lookup: `CommandPauseLevel` per `CommandType`.
pub const COMMAND_TYPE_PAUSE_LEVEL: [u8; 9] = [3, 2, 2, 1, 1, 1, 0, 0, 0];

/// Primitive mask adapters retained for existing consumers.
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

impl DoCommandFlags {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: DoCommandFlag) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl CommandFlags {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: CommandFlag) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Native `Commands`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Commands(pub u8);

#[allow(non_upper_case_globals)]
impl Commands {
    /// build a rail track
    pub const CMD_BUILD_RAILROAD_TRACK: Self = Self(0);
    /// remove a rail track
    pub const CMD_REMOVE_RAILROAD_TRACK: Self = Self(1);
    /// build a single rail track
    pub const CMD_BUILD_SINGLE_RAIL: Self = Self(2);
    /// remove a single rail track
    pub const CMD_REMOVE_SINGLE_RAIL: Self = Self(3);
    /// demolish a tile
    pub const CMD_LANDSCAPE_CLEAR: Self = Self(4);
    /// build a bridge
    pub const CMD_BUILD_BRIDGE: Self = Self(5);
    /// build a rail station
    pub const CMD_BUILD_RAIL_STATION: Self = Self(6);
    /// build a train depot
    pub const CMD_BUILD_TRAIN_DEPOT: Self = Self(7);
    /// build a signal
    pub const CMD_BUILD_SINGLE_SIGNAL: Self = Self(8);
    /// remove a signal
    pub const CMD_REMOVE_SINGLE_SIGNAL: Self = Self(9);
    /// terraform a tile
    pub const CMD_TERRAFORM_LAND: Self = Self(10);
    /// build an object
    pub const CMD_BUILD_OBJECT: Self = Self(11);
    /// build an area of objects
    pub const CMD_BUILD_OBJECT_AREA: Self = Self(12);
    /// build a tunnel
    pub const CMD_BUILD_TUNNEL: Self = Self(13);
    /// remove a (rectangle of) tiles from a rail station
    pub const CMD_REMOVE_FROM_RAIL_STATION: Self = Self(14);
    /// convert a rail type
    pub const CMD_CONVERT_RAIL: Self = Self(15);
    /// build a waypoint
    pub const CMD_BUILD_RAIL_WAYPOINT: Self = Self(16);
    /// rename a waypoint
    pub const CMD_RENAME_WAYPOINT: Self = Self(17);
    /// move a waypoint name
    pub const CMD_MOVE_WAYPOINT_NAME: Self = Self(18);
    /// remove a (rectangle of) tiles from a rail waypoint
    pub const CMD_REMOVE_FROM_RAIL_WAYPOINT: Self = Self(19);
    /// build a road waypoint
    pub const CMD_BUILD_ROAD_WAYPOINT: Self = Self(20);
    /// remove a (rectangle of) tiles from a road waypoint
    pub const CMD_REMOVE_FROM_ROAD_WAYPOINT: Self = Self(21);
    /// build a road stop
    pub const CMD_BUILD_ROAD_STOP: Self = Self(22);
    /// remove a road stop
    pub const CMD_REMOVE_ROAD_STOP: Self = Self(23);
    /// build a complete road (not a "half" one)
    pub const CMD_BUILD_LONG_ROAD: Self = Self(24);
    /// remove a complete road (not a "half" one)
    pub const CMD_REMOVE_LONG_ROAD: Self = Self(25);
    /// build a "half" road
    pub const CMD_BUILD_ROAD: Self = Self(26);
    /// build a road depot
    pub const CMD_BUILD_ROAD_DEPOT: Self = Self(27);
    /// convert a road type
    pub const CMD_CONVERT_ROAD: Self = Self(28);
    /// build an airport
    pub const CMD_BUILD_AIRPORT: Self = Self(29);
    /// build a dock
    pub const CMD_BUILD_DOCK: Self = Self(30);
    /// build a ship depot
    pub const CMD_BUILD_SHIP_DEPOT: Self = Self(31);
    /// build a buoy
    pub const CMD_BUILD_BUOY: Self = Self(32);
    /// plant a tree
    pub const CMD_PLANT_TREE: Self = Self(33);
    /// build a vehicle
    pub const CMD_BUILD_VEHICLE: Self = Self(34);
    /// sell a vehicle
    pub const CMD_SELL_VEHICLE: Self = Self(35);
    /// refit the cargo space of a vehicle
    pub const CMD_REFIT_VEHICLE: Self = Self(36);
    /// send a vehicle to a depot
    pub const CMD_SEND_VEHICLE_TO_DEPOT: Self = Self(37);
    /// hide or unhide a vehicle in the build vehicle and autoreplace GUIs
    pub const CMD_SET_VEHICLE_VISIBILITY: Self = Self(38);
    /// move a rail vehicle (in the depot)
    pub const CMD_MOVE_RAIL_VEHICLE: Self = Self(39);
    /// proceed a train to pass a red signal
    pub const CMD_FORCE_TRAIN_PROCEED: Self = Self(40);
    /// turn a train around
    pub const CMD_REVERSE_TRAIN_DIRECTION: Self = Self(41);
    /// clear the order backup of a given user/tile
    pub const CMD_CLEAR_ORDER_BACKUP: Self = Self(42);
    /// modify an order (like set full-load)
    pub const CMD_MODIFY_ORDER: Self = Self(43);
    /// skip an order to the next of specific one
    pub const CMD_SKIP_TO_ORDER: Self = Self(44);
    /// delete an order
    pub const CMD_DELETE_ORDER: Self = Self(45);
    /// insert a new order
    pub const CMD_INSERT_ORDER: Self = Self(46);
    /// change the server interval of a vehicle
    pub const CMD_CHANGE_SERVICE_INT: Self = Self(47);
    /// build a new industry
    pub const CMD_BUILD_INDUSTRY: Self = Self(48);
    /// change industry control flags
    pub const CMD_INDUSTRY_SET_FLAGS: Self = Self(49);
    /// change industry exclusive consumer/supplier
    pub const CMD_INDUSTRY_SET_EXCLUSIVITY: Self = Self(50);
    /// change additional text for the industry
    pub const CMD_INDUSTRY_SET_TEXT: Self = Self(51);
    /// change industry production
    pub const CMD_INDUSTRY_SET_PRODUCTION: Self = Self(52);
    /// set the manager's face of the company
    pub const CMD_SET_COMPANY_MANAGER_FACE: Self = Self(53);
    /// set the colour of the company
    pub const CMD_SET_COMPANY_COLOUR: Self = Self(54);
    /// increase the loan from the bank
    pub const CMD_INCREASE_LOAN: Self = Self(55);
    /// decrease the loan from the bank
    pub const CMD_DECREASE_LOAN: Self = Self(56);
    /// sets the max loan for the company
    pub const CMD_SET_COMPANY_MAX_LOAN: Self = Self(57);
    /// confirm the preview of an engine
    pub const CMD_WANT_ENGINE_PREVIEW: Self = Self(58);
    /// control availability of the engine for companies
    pub const CMD_ENGINE_CTRL: Self = Self(59);
    /// rename a whole vehicle
    pub const CMD_RENAME_VEHICLE: Self = Self(60);
    /// rename a engine (in the engine list)
    pub const CMD_RENAME_ENGINE: Self = Self(61);
    /// change the company name
    pub const CMD_RENAME_COMPANY: Self = Self(62);
    /// change the president name
    pub const CMD_RENAME_PRESIDENT: Self = Self(63);
    /// rename a station
    pub const CMD_RENAME_STATION: Self = Self(64);
    /// move a station name
    pub const CMD_MOVE_STATION_NAME: Self = Self(65);
    /// rename a depot
    pub const CMD_RENAME_DEPOT: Self = Self(66);
    /// place a sign
    pub const CMD_PLACE_SIGN: Self = Self(67);
    /// rename a sign
    pub const CMD_RENAME_SIGN: Self = Self(68);
    /// move a sign
    pub const CMD_MOVE_SIGN: Self = Self(69);
    /// turn a road vehicle around
    pub const CMD_TURN_ROADVEH: Self = Self(70);
    /// pause the game
    pub const CMD_PAUSE: Self = Self(71);
    /// buy a company which is bankrupt
    pub const CMD_BUY_COMPANY: Self = Self(72);
    /// found a town
    pub const CMD_FOUND_TOWN: Self = Self(73);
    /// rename a town
    pub const CMD_RENAME_TOWN: Self = Self(74);
    /// do a action from the town detail window (like advertises or bribe)
    pub const CMD_DO_TOWN_ACTION: Self = Self(75);
    /// set the goal of a cargo for a town
    pub const CMD_TOWN_CARGO_GOAL: Self = Self(76);
    /// set the town growth rate
    pub const CMD_TOWN_GROWTH_RATE: Self = Self(77);
    /// set rating of a company in a town
    pub const CMD_TOWN_RATING: Self = Self(78);
    /// set the custom text of a town
    pub const CMD_TOWN_SET_TEXT: Self = Self(79);
    /// expand a town
    pub const CMD_EXPAND_TOWN: Self = Self(80);
    /// delete a town
    pub const CMD_DELETE_TOWN: Self = Self(81);
    /// place a house
    pub const CMD_PLACE_HOUSE: Self = Self(82);
    /// change the refit information of an order (for "goto depot" )
    pub const CMD_ORDER_REFIT: Self = Self(83);
    /// clone (and share) an order
    pub const CMD_CLONE_ORDER: Self = Self(84);
    /// clear an area
    pub const CMD_CLEAR_AREA: Self = Self(85);
    /// do the money cheat
    pub const CMD_MONEY_CHEAT: Self = Self(86);
    /// change bank balance to charge costs or give money from a GS
    pub const CMD_CHANGE_BANK_BALANCE: Self = Self(87);
    /// build a canal
    pub const CMD_BUILD_CANAL: Self = Self(88);
    /// create a new subsidy
    pub const CMD_CREATE_SUBSIDY: Self = Self(89);
    /// used in multiplayer to create a new companies etc.
    pub const CMD_COMPANY_CTRL: Self = Self(90);
    /// Used in multiplayer to add/remove a client's public key to/from the company's allow list.
    pub const CMD_COMPANY_ALLOW_LIST_CTRL: Self = Self(91);
    /// create a custom news message
    pub const CMD_CUSTOM_NEWS_ITEM: Self = Self(92);
    /// create a new goal
    pub const CMD_CREATE_GOAL: Self = Self(93);
    /// remove a goal
    pub const CMD_REMOVE_GOAL: Self = Self(94);
    /// update goal destination of a goal
    pub const CMD_SET_GOAL_DESTINATION: Self = Self(95);
    /// update goal text of a goal
    pub const CMD_SET_GOAL_TEXT: Self = Self(96);
    /// update goal progress text of a goal
    pub const CMD_SET_GOAL_PROGRESS: Self = Self(97);
    /// update goal completed status of a goal
    pub const CMD_SET_GOAL_COMPLETED: Self = Self(98);
    /// ask a goal related question
    pub const CMD_GOAL_QUESTION: Self = Self(99);
    /// answer(s) to CMD_GOAL_QUESTION
    pub const CMD_GOAL_QUESTION_ANSWER: Self = Self(100);
    /// create a new story page
    pub const CMD_CREATE_STORY_PAGE: Self = Self(101);
    /// create a new story page element
    pub const CMD_CREATE_STORY_PAGE_ELEMENT: Self = Self(102);
    /// update a story page element
    pub const CMD_UPDATE_STORY_PAGE_ELEMENT: Self = Self(103);
    /// update title of a story page
    pub const CMD_SET_STORY_PAGE_TITLE: Self = Self(104);
    /// update date of a story page
    pub const CMD_SET_STORY_PAGE_DATE: Self = Self(105);
    /// show a story page
    pub const CMD_SHOW_STORY_PAGE: Self = Self(106);
    /// remove a story page
    pub const CMD_REMOVE_STORY_PAGE: Self = Self(107);
    /// remove a story page element
    pub const CMD_REMOVE_STORY_PAGE_ELEMENT: Self = Self(108);
    /// scroll main viewport of players
    pub const CMD_SCROLL_VIEWPORT: Self = Self(109);
    /// selection via story page button
    pub const CMD_STORY_PAGE_BUTTON: Self = Self(110);
    /// level land
    pub const CMD_LEVEL_LAND: Self = Self(111);
    /// build a lock
    pub const CMD_BUILD_LOCK: Self = Self(112);
    /// add signals along a track (by dragging)
    pub const CMD_BUILD_SIGNAL_TRACK: Self = Self(113);
    /// remove signals along a track (by dragging)
    pub const CMD_REMOVE_SIGNAL_TRACK: Self = Self(114);
    /// give money to another company
    pub const CMD_GIVE_MONEY: Self = Self(115);
    /// change a setting
    pub const CMD_CHANGE_SETTING: Self = Self(116);
    /// change a company setting
    pub const CMD_CHANGE_COMPANY_SETTING: Self = Self(117);
    /// set an autoreplace entry
    pub const CMD_SET_AUTOREPLACE: Self = Self(118);
    /// clone a vehicle
    pub const CMD_CLONE_VEHICLE: Self = Self(119);
    /// start or stop a vehicle
    pub const CMD_START_STOP_VEHICLE: Self = Self(120);
    /// start/stop all vehicles (in a depot)
    pub const CMD_MASS_START_STOP: Self = Self(121);
    /// replace/renew a vehicle while it is in a depot
    pub const CMD_AUTOREPLACE_VEHICLE: Self = Self(122);
    /// sell all vehicles which are in a given depot
    pub const CMD_DEPOT_SELL_ALL_VEHICLES: Self = Self(123);
    /// force the autoreplace to take action in a given depot
    pub const CMD_DEPOT_MASS_AUTOREPLACE: Self = Self(124);
    /// create a new group
    pub const CMD_CREATE_GROUP: Self = Self(125);
    /// delete a group
    pub const CMD_DELETE_GROUP: Self = Self(126);
    /// alter a group
    pub const CMD_ALTER_GROUP: Self = Self(127);
    /// add a vehicle to a group
    pub const CMD_ADD_VEHICLE_GROUP: Self = Self(128);
    /// add all other shared vehicles to a group which are missing
    pub const CMD_ADD_SHARED_VEHICLE_GROUP: Self = Self(129);
    /// remove all vehicles from a group
    pub const CMD_REMOVE_ALL_VEHICLES_GROUP: Self = Self(130);
    /// set/clear a flag for a group
    pub const CMD_SET_GROUP_FLAG: Self = Self(131);
    /// set the livery for a group
    pub const CMD_SET_GROUP_LIVERY: Self = Self(132);
    /// move an order
    pub const CMD_MOVE_ORDER: Self = Self(133);
    /// change the timetable for a vehicle
    pub const CMD_CHANGE_TIMETABLE: Self = Self(134);
    /// change the timetable for all orders of a vehicle
    pub const CMD_BULK_CHANGE_TIMETABLE: Self = Self(135);
    /// set the vehicle on time feature (timetable)
    pub const CMD_SET_VEHICLE_ON_TIME: Self = Self(136);
    /// autofill the timetable
    pub const CMD_AUTOFILL_TIMETABLE: Self = Self(137);
    /// set the date that a timetable should start
    pub const CMD_SET_TIMETABLE_START: Self = Self(138);
    /// open/close an airport to incoming aircraft
    pub const CMD_OPEN_CLOSE_AIRPORT: Self = Self(139);
    /// create a new league table
    pub const CMD_CREATE_LEAGUE_TABLE: Self = Self(140);
    /// create a new element in a league table
    pub const CMD_CREATE_LEAGUE_TABLE_ELEMENT: Self = Self(141);
    /// update the data fields of a league table element
    pub const CMD_UPDATE_LEAGUE_TABLE_ELEMENT_DATA: Self = Self(142);
    /// update the score of a league table element
    pub const CMD_UPDATE_LEAGUE_TABLE_ELEMENT_SCORE: Self = Self(143);
    /// remove a league table element
    pub const CMD_REMOVE_LEAGUE_TABLE_ELEMENT: Self = Self(144);
    /// Must ALWAYS be on the end of this list!! (period)
    pub const CMD_END: Self = Self(145);
}

/// Native `CommandPauseLevel`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandPauseLevel(pub u8);

#[allow(non_upper_case_globals)]
impl CommandPauseLevel {
    /// No user actions may be executed.
    pub const NoActions: Self = Self(0);
    /// No construction actions may be executed.
    pub const NoConstruction: Self = Self(1);
    /// No landscaping actions may be executed.
    pub const NoLandscaping: Self = Self(2);
    /// All actions may be executed.
    pub const AllActions: Self = Self(3);
}

crate::base_bitset_type::impl_bitset_ops!(DoCommandFlags);
crate::base_bitset_type::impl_bitset_ops!(CommandFlags);
