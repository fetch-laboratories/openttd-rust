/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared station, goods, airport and road-stop records (#634).
//!
//! One `StationRecord` lives beside every `BaseStation` shell, `Station` and
//! `Waypoint` alike, from before the shell's members until after its base
//! destructor. It is the first member of the canonical Rust station owner, so a
//! record pointer is also the owner's address. One `RoadStopRecord` lives beside
//! every `RoadStop` shell. Both languages access fields in place under the
//! game-state mutex; Rust holds no reference to a record across a callback.
#![allow(unsafe_code)]

use crate::company_type::CompanyID;
use crate::station_type::BaseStation;
use crate::tile_type::TileIndex;

/// Opaque native `Town`.
#[repr(C)]
pub struct Town {
    _private: [u8; 0],
}
/// Opaque native `PersistentStorage`.
#[repr(C)]
pub struct PersistentStorage {
    _private: [u8; 0],
}
/// Opaque native `GoodsEntryData`: the cargo list and flow facades, owned by C++.
#[repr(C)]
pub struct GoodsEntryData {
    _private: [u8; 0],
}
/// Opaque native `RoadStop` shell.
#[repr(C)]
pub struct RoadStop {
    _private: [u8; 0],
}

/// `NUM_CARGO`.
pub const NUM_CARGO: usize = 64;

crate::abi::shared_records! {
    fn abi_record;
    /// Native `OrthogonalTileArea` scalars.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    "OpenTTDTileArea" ["station"] => pub struct TileAreaRecord {
        pub tile: TileIndex,
        pub w: u16,
        pub h: u16,
    }
    /// `StationRect`, the station spread rectangle.
    #[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
    "OpenTTDStationRect" ["station"] => pub struct StationRect {
        pub left: i32,
        pub top: i32,
        pub right: i32,
        pub bottom: i32,
    }
    /// One `GoodsEntry` cargo slot.
    #[derive(Clone, Copy)]
    "OpenTTDGoodsEntry" ["station"] => pub struct GoodsRecord {
        /// `GoodsEntry::data`, owned by the native station shell.
        pub data: *mut GoodsEntryData,
        pub max_waiting_cargo: u32,
        /// `NodeID`.
        pub node: u16,
        /// `LinkGraphID`.
        pub link_graph: u16,
        /// Native `GoodsEntry::States`.
        pub status: u8,
        pub time_since_pickup: u8,
        pub rating: u8,
        pub last_speed: u8,
        pub last_age: u8,
        pub amount_fract: u8,
    }
    /// `Station::airport`.
    "OpenTTDAirport" ["station"] => pub struct AirportRecord {
        /// Native `AirportBlocks`.
        pub blocks: u64,
        pub psa: *mut PersistentStorage,
        pub area: TileAreaRecord,
        pub r#type: u8,
        pub layout: u8,
        /// Native `Direction`.
        pub rotation: u8,
    }
    /// The fields of one `BaseStation`, `Station` or `Waypoint` shell.
    "OpenTTDStationRecord" ["station"] => pub struct StationRecord {
        /// Owning native shell; stable while the record lives.
        pub shell: *mut BaseStation,
        pub town: *mut Town,
        /// NOSAVE `Station::industry`.
        pub industry: *mut crate::industry_type::Industry,
        /// `Station::bus_stops`.
        pub bus_stops: *mut RoadStopRecord,
        /// `Station::truck_stops`.
        pub truck_stops: *mut RoadStopRecord,
        /// NOSAVE `BaseStation::cached_cargo_triggers`.
        pub cached_cargo_triggers: u64,
        /// NOSAVE `BaseStation::cached_roadstop_cargo_triggers`.
        pub cached_roadstop_cargo_triggers: u64,
        pub always_accepted: u64,
        pub airport: AirportRecord,
        pub goods: [GoodsRecord; 64],
        pub rect: StationRect,
        pub train_station: TileAreaRecord,
        pub bus_station: TileAreaRecord,
        pub truck_station: TileAreaRecord,
        pub ship_station: TileAreaRecord,
        pub docking_station: TileAreaRecord,
        /// `Waypoint::road_waypoint_area`.
        pub road_waypoint_area: TileAreaRecord,
        pub xy: TileIndex,
        /// `StringID`.
        pub string_id: u32,
        /// `TimerGameCalendar::Date`.
        pub build_date: i32,
        pub random_bits: u16,
        /// NOSAVE native `StationAnimationTriggers`.
        pub cached_anim_triggers: u16,
        /// NOSAVE native `StationAnimationTriggers`.
        pub cached_roadstop_anim_triggers: u16,
        /// `Waypoint::town_cn`.
        pub town_cn: u16,
        /// `Waypoint::waypoint_flags`.
        pub waypoint_flags: u16,
        pub owner: CompanyID,
        /// Native `StationFacilities`.
        pub facilities: u8,
        /// Native `StationRandomTriggers`.
        pub waiting_random_triggers: u8,
        /// `IndustryType`.
        pub indtype: u8,
        /// Native `StationHadVehicleOfType`.
        pub had_vehicle_of_type: u8,
        pub delete_ctr: u8,
        pub time_since_load: u8,
        pub time_since_unload: u8,
        pub last_vehicle_type: u8,
    }
    /// The fields of one `RoadStop` shell.
    "OpenTTDRoadStopState" ["station"] => pub struct RoadStopRecord {
        /// Owning native shell; stable while the record lives.
        pub shell: *mut RoadStop,
        /// `RoadStop::next`.
        pub next: *mut RoadStopRecord,
        /// Shared entries, owned by the `BaseEntry` holder.
        pub entries: *mut crate::road_stop::Entries,
        pub xy: TileIndex,
        /// Native `RoadStopStatusFlags`.
        pub status: u8,
    }
}

/// `INVALID_TILE`.
const INVALID_TILE: TileIndex = TileIndex(u32::MAX);
/// An empty `TileArea` at `INVALID_TILE`.
pub const EMPTY_AREA: TileAreaRecord = TileAreaRecord {
    tile: INVALID_TILE,
    w: 0,
    h: 0,
};

impl Default for GoodsRecord {
    /// The original `GoodsEntry` member defaults.
    fn default() -> Self {
        Self {
            data: std::ptr::null_mut(),
            max_waiting_cargo: 0,
            node: u16::MAX,
            link_graph: u16::MAX,
            status: 0,
            time_since_pickup: 255,
            rating: crate::station_base::INITIAL_STATION_RATING,
            last_speed: 0,
            last_age: 255,
            amount_fract: 0,
        }
    }
}

impl StationRecord {
    /// The original `BaseStation`, `Station` and `Waypoint` member defaults.
    pub(crate) fn new(shell: *mut BaseStation, xy: TileIndex) -> Self {
        Self {
            shell,
            town: std::ptr::null_mut(),
            industry: std::ptr::null_mut(),
            bus_stops: std::ptr::null_mut(),
            truck_stops: std::ptr::null_mut(),
            cached_cargo_triggers: 0,
            cached_roadstop_cargo_triggers: 0,
            always_accepted: 0,
            airport: AirportRecord {
                blocks: 0,
                psa: std::ptr::null_mut(),
                area: EMPTY_AREA,
                r#type: 0,
                layout: 0,
                rotation: u8::MAX, // INVALID_DIR
            },
            goods: [GoodsRecord::default(); NUM_CARGO],
            rect: StationRect::default(),
            train_station: EMPTY_AREA,
            bus_station: EMPTY_AREA,
            truck_station: EMPTY_AREA,
            ship_station: EMPTY_AREA,
            docking_station: EMPTY_AREA,
            road_waypoint_area: EMPTY_AREA,
            xy,
            string_id: u32::from(u16::MAX),
            build_date: 0,
            random_bits: 0,
            cached_anim_triggers: 0,
            cached_roadstop_anim_triggers: 0,
            town_cn: 0,
            waypoint_flags: 0,
            owner: CompanyID(u8::MAX),
            facilities: 0,
            waiting_random_triggers: 0,
            indtype: u8::MAX,
            had_vehicle_of_type: 0,
            delete_ctr: 0,
            time_since_load: 255,
            time_since_unload: 255,
            last_vehicle_type: 255,
        }
    }
}
