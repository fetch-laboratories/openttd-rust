/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Types related to windows (`window_type.h`). Native assertions in `services.cpp` pin the
//! values.

use crate::company_type::CompanyID;
use crate::industry_type::IndustryID;
use crate::league_type::LeagueTableID;
use crate::network_type::DestType;
use crate::station_type::StationID;
use crate::tile_type::TileIndex;
use crate::town_type::TownId;
use crate::vehicle_type::VehicleID;

/// Native `WindowNumberEnum`: window numbers.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowNumberEnum(pub u8);

impl WindowNumberEnum {
    /// Network join status.
    pub const WN_NETWORK_STATUS_WINDOW_JOIN: Self = Self(0);
}

/// Native `WindowClass`, retaining every underlying `u16` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowClass(pub u16);

impl WindowClass {
    /// Main toolbar (the long bar at the top).
    pub const WC_MAIN_TOOLBAR: Self = Self(1);
    /// Popup with a set of buttons, designed to ask the user a question.
    pub const WC_GOAL_QUESTION: Self = Self(13);
    /// Town authority.
    pub const WC_TOWN_AUTHORITY: Self = Self(22);
    /// Vehicle details.
    pub const WC_VEHICLE_DETAILS: Self = Self(23);
    /// Vehicle refit.
    pub const WC_VEHICLE_REFIT: Self = Self(24);
    /// Vehicle orders.
    pub const WC_VEHICLE_ORDERS: Self = Self(25);
    /// Vehicle timetable.
    pub const WC_VEHICLE_TIMETABLE: Self = Self(27);
    /// Company colour selection.
    pub const WC_COMPANY_COLOUR: Self = Self(28);
    /// Select station (when joining stations).
    pub const WC_SELECT_STATION: Self = Self(30);
    /// Town directory.
    pub const WC_TOWN_DIRECTORY: Self = Self(32);
    /// Sign list.
    pub const WC_SIGN_LIST: Self = Self(36);
    /// Goals list.
    pub const WC_GOALS_LIST: Self = Self(38);
    /// Story book.
    pub const WC_STORY_BOOK: Self = Self(39);
    /// Station list.
    pub const WC_STATION_LIST: Self = Self(40);
    /// Trains list.
    pub const WC_TRAINS_LIST: Self = Self(41);
    /// Road vehicle list.
    pub const WC_ROADVEH_LIST: Self = Self(42);
    /// Ships list.
    pub const WC_SHIPS_LIST: Self = Self(43);
    /// Aircraft list.
    pub const WC_AIRCRAFT_LIST: Self = Self(44);
    /// Town view.
    pub const WC_TOWN_VIEW: Self = Self(45);
    /// Vehicle view.
    pub const WC_VEHICLE_VIEW: Self = Self(46);
    /// Station view.
    pub const WC_STATION_VIEW: Self = Self(47);
    /// Depot view.
    pub const WC_VEHICLE_DEPOT: Self = Self(48);
    /// Industry view.
    pub const WC_INDUSTRY_VIEW: Self = Self(50);
    /// Company view.
    pub const WC_COMPANY: Self = Self(51);
    /// Build vehicle.
    pub const WC_BUILD_VEHICLE: Self = Self(54);
    /// Build industry.
    pub const WC_BUILD_INDUSTRY: Self = Self(62);
    /// Client list.
    pub const WC_CLIENT_LIST: Self = Self(68);
    /// Network status window.
    pub const WC_NETWORK_STATUS_WINDOW: Self = Self(69);
    /// Chatbox.
    pub const WC_SEND_NETWORK_MSG: Self = Self(72);
    /// Company league window.
    pub const WC_COMPANY_LEAGUE: Self = Self(81);
    /// Industry production history graph.
    pub const WC_INDUSTRY_PRODUCTION: Self = Self(84);
    /// Town cargo history graph.
    pub const WC_TOWN_CARGO_GRAPH: Self = Self(85);
    /// Engine preview window.
    pub const WC_ENGINE_PREVIEW: Self = Self(88);
    /// Game options window.
    pub const WC_GAME_OPTIONS: Self = Self(91);
}

/// Native `WindowNumber`: the number of a window within its class, constructed from an
/// `int32_t` or from the base of a type convertible through its base.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowNumber(pub i32);

impl From<CompanyID> for WindowNumber {
    fn from(value: CompanyID) -> Self {
        Self(i32::from(value.0))
    }
}

impl From<DestType> for WindowNumber {
    fn from(value: DestType) -> Self {
        Self(i32::from(value.0))
    }
}

impl From<IndustryID> for WindowNumber {
    fn from(value: IndustryID) -> Self {
        Self(i32::from(value.0))
    }
}

impl From<LeagueTableID> for WindowNumber {
    fn from(value: LeagueTableID) -> Self {
        Self(i32::from(value.0))
    }
}

impl From<StationID> for WindowNumber {
    fn from(value: StationID) -> Self {
        Self(i32::from(value.0))
    }
}

impl From<TileIndex> for WindowNumber {
    fn from(value: TileIndex) -> Self {
        Self(value.0.cast_signed())
    }
}

impl From<WindowNumberEnum> for WindowNumber {
    fn from(value: WindowNumberEnum) -> Self {
        Self(i32::from(value.0))
    }
}

impl From<TownId> for WindowNumber {
    fn from(value: TownId) -> Self {
        Self(i32::from(value.0))
    }
}

impl From<VehicleID> for WindowNumber {
    fn from(value: VehicleID) -> Self {
        Self(value.0.cast_signed())
    }
}

/// Native `WidgetID`.
pub type WidgetID = i32;
