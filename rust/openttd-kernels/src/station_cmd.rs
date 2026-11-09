/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Station construction, joining and removal policy on the station service owner
//! (`station_service.rs`). Shells, map, pools, specs and command result slots stay native.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::if_not_else,
    clippy::too_many_lines,
    clippy::too_many_arguments
)]
use crate::base_station_base::StationRectMode;
use crate::cargo_type::NUM_CARGO;
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company_type::{CompanyID, INVALID_OWNER, OWNER_NONE, OWNER_TOWN, OWNER_WATER};
use crate::debug::debug;
use crate::depot_func::can_build_depot_by_tileh;
use crate::direction_type::{DiagDirection, DiagDirections, INVALID_DIAGDIR};
use crate::economy_type::Price;
use crate::newgrf_callbacks::{CALLBACK_FAILED, CallbackID};
use crate::order_type::{OT_GOTO_STATION, OT_GOTO_WAYPOINT};
use crate::road_type::INVALID_ROADTYPE;
use crate::slope_type::Slope;
use crate::station_type::StationFacilities;
use crate::station_type::{
    FACILITY_AIRPORT, FACILITY_BUS, FACILITY_DOCK, FACILITY_TRAIN, FACILITY_TRUCK, StationID,
    StationType,
};
use crate::strings_type::INVALID_STRING_ID;
use crate::strings_type::StringID;
use crate::tile_type::{
    MP_INDUSTRY, MP_RAILWAY, MP_ROAD, MP_STATION, MP_TUNNELBRIDGE, MP_WATER, TileIndex, TileType,
};
use crate::tilearea::TileArea;
mod waypoint;
use crate::newgrf_roadstop::RoadStopAvailabilityType;
use crate::newgrf_station::station_spec::{TileFlag, TileFlags};
use crate::rail_cmd::RailCommandServices;
use crate::road_stop::{
    RoadStopServices, Stop, clear_drive_through, get_by_tile as road_stop_by_tile,
    make_drive_through,
};
use crate::services::{CommandCost, CommandCostServices, CommandServices, Cost, Map, Services};
use crate::slope_func::{get_inclined_slope_direction, get_slope_max_z, is_steep_slope};
use crate::station::BaseStation;
use crate::station_map::{GFX_DOCK_BASE_WATER_PART, GFX_TRUCK_BUS_DRIVETHROUGH_OFFSET, StationGfx};
use crate::station_service::World;
use crate::station_type::{RoadStopType, StationFacility};
use crate::town::AuthorityServices;
use waypoint::WaypointLayout;

/// Native station naming classes local to `station_cmd.cpp`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StationNaming(pub u8);
impl StationNaming {
    pub(crate) const STATIONNAMING_RAIL: Self = Self(0);
    pub(crate) const STATIONNAMING_ROAD: Self = Self(1);
    pub(crate) const STATIONNAMING_AIRPORT: Self = Self(2);
    pub(crate) const STATIONNAMING_OILRIG: Self = Self(3);
    pub(crate) const STATIONNAMING_DOCK: Self = Self(4);
    pub(crate) const STATIONNAMING_HELIPORT: Self = Self(5);
}

impl CommandServicesTable {
    /// `BaseStation::GetTileArea(StationType::Rail)`: a station's
    /// `train_station`, or a waypoint's rail area.
    pub(crate) fn rail_area(&self, st: *const BaseStation) -> TileArea {
        let station = unsafe { &*self.station };
        let station_type = if (station.is_station)(st.cast_mut()) {
            StationType::Rail
        } else {
            StationType::RailWaypoint
        };
        (self.tile_area)(st, station_type.0)
    }
}

#[repr(C)]
pub struct Industry {
    _private: [u8; 0],
}
#[repr(C)]
pub struct RailSpec {
    _private: [u8; 0],
}
#[repr(C)]
pub struct RoadSpec {
    _private: [u8; 0],
}
#[repr(C)]
pub struct BridgeInfo {
    pub height: u8,
    pub pillars: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BridgeSpan {
    pub data: *const BridgeInfo,
    pub size: usize,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SpecIndex {
    pub value: u8,
    pub valid: bool,
}
#[repr(C)]
pub struct RailSpecInfo {
    pub platforms: u8,
    pub lengths: u8,
    pub slope_callback: bool,
    pub availability_callback: bool,
}
/// Only the spec fields needed by one build/clear price calculation.
#[repr(C)]
pub struct RoadSpecCost {
    pub multiplier: u8,
    /// The spec GRF's `price_base_multipliers[price]`, zero without a GRF file.
    pub shift: i32,
}
#[repr(C)]
pub struct RoadSpecInfo {
    pub stop_type: u8,
    pub drive_only: bool,
    pub availability_callback: bool,
}
#[repr(C)]
pub struct AirportSpec {
    _private: [u8; 0],
}
#[repr(C)]
pub struct TileIterator {
    _private: [u8; 0],
}
#[repr(C)]
pub struct AirportSize {
    pub w: u8,
    pub h: u8,
}
#[repr(C)]
pub struct AirportLayout {
    pub size: usize,
    pub rotation: u8,
}
#[repr(C)]
pub struct AirportTile {
    pub tile: u32,
    pub gfx: u8,
}
#[repr(C)]
pub struct Strings {
    pub demolish_bridge: u32,
    pub flat_land: u32,
    pub spread: u32,
    pub bridge_too_low: [u32; 9],
    pub too_many_stations: u32,
    pub adjoins_multiple: u32,
    pub remove_station: u32,
    pub remove_rail_waypoint: u32,
    pub remove_road_waypoint: u32,
    pub remove_road_stop: u32,
    pub name_unique: u32,
    pub unsuitable: u32,
    pub too_many_specs: u32,
    pub disallowed_tracks: u32,
    pub disallowed_length: u32,
    pub remove_signals: u32,
    pub no_station: u32,
    pub drive_direction: u32,
    pub drive_corner: u32,
    pub drive_junction: u32,
    pub drive_town: u32,
    pub no_road: u32,
    pub unsuitable_road: u32,
    pub too_many_truck: u32,
    pub too_many_bus: u32,
    pub refuse_noise: u32,
    pub refuse_airport: u32,
    pub close_airport: u32,
    pub aircraft_in_way: u32,
    pub demolish_railroad: u32,
    pub demolish_building: u32,
    pub demolish_airport: u32,
    pub demolish_cargo_tram: u32,
    pub demolish_truck: u32,
    pub demolish_passenger_tram: u32,
    pub demolish_bus: u32,
    pub buoy_in_way: u32,
    pub demolish_dock: u32,
    pub object_in_way: u32,
    pub oilrig_name: u32,
    pub buoy_in_use: u32,
    pub buoy_name: u32,
    pub waypoint_adjoins_multiple: u32,
    pub waypoint_close: u32,
    pub waypoint_name: u32,
}
#[repr(C)]
pub struct CommandServicesTable {
    pub shared: *const Services,
    pub station: *const World,
    pub rail: *const RailCommandServices,
    pub cost: *const CommandCostServices,
    pub command: *const CommandServices,
    pub authority: *const AuthorityServices,
    pub strings: Strings,
    pub spread: extern "C" fn() -> u8,
    pub default_bridge_info: extern "C" fn(u8) -> BridgeSpan,
    pub rail_bridge_info: unsafe extern "C" fn(*const RailSpec) -> BridgeSpan,
    pub road_bridge_info: unsafe extern "C" fn(*const RoadSpec) -> BridgeSpan,
    pub southern_bridge_end: extern "C" fn(u32) -> u32,
    pub tile_area: extern "C" fn(*const BaseStation, u8) -> TileArea,
    pub rail_spec: extern "C" fn(u32) -> *const RailSpec,
    pub links: *const crate::station_service::Links,
    pub location: extern "C" fn(*const BaseStation) -> u32,
    pub base_move_sign: extern "C" fn(*mut BaseStation, u32),
    pub waypoint_move_sign: extern "C" fn(*mut BaseStation, u32),
    pub virtual_coord: extern "C" fn(*mut BaseStation),
    pub catchment_dirty: extern "C" fn(),
    pub facility_widget: extern "C" fn(*mut BaseStation, u8),
    pub select_invalidate: extern "C" fn(),
    pub station_available: extern "C" fn() -> bool,
    pub new_station: extern "C" fn(u32) -> *mut BaseStation,
    pub set_town: extern "C" fn(*mut BaseStation, *const crate::services::TownHandle),
    pub lifecycle: *const crate::station_lifecycle::Services,
    pub have_rating: extern "C" fn(*mut BaseStation, u8),
    pub road_waypoint: extern "C" fn(*mut BaseStation) -> bool,
    pub name_equals: extern "C" fn(*mut BaseStation, *const u8, usize) -> bool,
    pub set_name: extern "C" fn(*mut BaseStation, *const u8, usize),
    pub flip_airport_closed: extern "C" fn(*mut BaseStation),
    pub closed_widget: extern "C" fn(*mut BaseStation),
    pub distant_join: extern "C" fn() -> bool,
    pub rail_association: *const crate::station_association::Services,
    pub rail_spec_info: extern "C" fn(*const RailSpec) -> RailSpecInfo,
    pub rail_layout: extern "C" fn(*const RailSpec, u16) -> crate::services::ByteSpan,
    pub rail_boolean: extern "C" fn(*const RailSpec, u16, u16) -> bool,
    pub rail_callback_error: extern "C" fn(*const RailSpec, u16, u16),
    pub set_train_area: extern "C" fn(*mut BaseStation, TileArea),
    pub rail_triggers: extern "C" fn(*mut BaseStation, *const RailSpec),
    pub reserved: extern "C" fn(u32) -> bool,
    pub last_train:
        unsafe extern "C" fn(*mut crate::services::Train) -> *mut crate::services::Train,
    pub train: *const crate::train::Leaves,
    pub reservation: *const crate::train_reservation::Leaves,
    pub compatible_tiles: extern "C" fn(u32, u32) -> bool,
    pub custom_rail_index: extern "C" fn(u32) -> u32,
    pub make_rail_station: extern "C" fn(u32, u8, u16, u8, u8, u8),
    pub close_station_inspect: extern "C" fn(u32),
    pub set_custom_rail_index: extern "C" fn(u32, u8),
    pub set_random_bits: extern "C" fn(u32, u8),
    pub set_gfx: extern "C" fn(u32, u8),

    pub tile_flag_count: extern "C" fn(*const RailSpec) -> usize,
    pub tile_flags: extern "C" fn(*const RailSpec, u8) -> u8,
    pub set_tile_flags: extern "C" fn(u32, bool, bool, bool),
    pub station_infrastructure: extern "C" fn(u8, u32),
    pub mark_tiles: extern "C" fn(*mut BaseStation, bool),
    pub reset_facilities: extern "C" fn(*mut BaseStation, u8),
    pub orders_dirty: extern "C" fn(),
    pub vehicle: *const crate::vehicle::VehicleServices,
    pub stop_on_town_road: extern "C" fn() -> bool,
    pub stop_on_competitor_road: extern "C" fn() -> bool,
    pub road_spec_info: extern "C" fn(*const RoadSpec) -> RoadSpecInfo,
    pub road_spec_build_cost: extern "C" fn(*const RoadSpec, u8) -> RoadSpecCost,
    pub road_spec_clear_cost: extern "C" fn(*const RoadSpec, u8) -> RoadSpecCost,
    pub stops_available: extern "C" fn(usize) -> bool,
    pub roadstop_resolver: *const crate::newgrf_roadstop::Services,
    pub road_boolean: extern "C" fn(*const RoadSpec, u16) -> bool,
    pub road_triggers: extern "C" fn(*mut BaseStation, *const RoadSpec),
    pub new_stop: extern "C" fn(u32) -> *mut Stop,
    pub set_stop_head: extern "C" fn(*mut BaseStation, bool, *mut Stop),
    pub set_stop_next: extern "C" fn(*mut Stop, *mut Stop),
    /// `truck_station` or `bus_station`, in place.
    pub stop_area: extern "C" fn(*mut BaseStation, bool) -> *mut TileArea,
    pub clear_stop_area: extern "C" fn(*mut BaseStation, bool),
    pub make_drive_stop: extern "C" fn(u32, u8, u8, u8, u16, u8, u8, u8, u8),
    pub make_bay_stop: extern "C" fn(u32, u8, u16, u8, u8, u8, u8),
    pub delete_stop: extern "C" fn(*mut Stop),
    pub set_custom_road_index: extern "C" fn(u32, u8),
    pub custom_road_index: extern "C" fn(u32) -> u32,

    pub remove_stop_tile_data: extern "C" fn(*mut BaseStation, u32),
    pub close_stop_inspect: extern "C" fn(u32),
    pub set_road_waypoint_area: extern "C" fn(*mut BaseStation, TileArea),
    pub road_vehicle_owner: extern "C" fn(crate::vehicle::VehicleRef) -> *mut crate::road::State,
    pub order_station_location: extern "C" fn(crate::vehicle::VehicleRef, u16) -> u32,
    pub aircraft: *const crate::aircraft::Leaves,
    pub airport_runtime: *const crate::newgrf_airport::Services,
    pub airport_layout_count: extern "C" fn(*const AirportSpec) -> usize,
    pub airport_size: extern "C" fn(*const AirportSpec) -> AirportSize,
    pub airport_layout: extern "C" fn(*const AirportSpec, u8) -> AirportLayout,
    pub airport_layout_tile: extern "C" fn(*const AirportSpec, u8, u32, usize) -> AirportTile,
    pub iterator_tile: extern "C" fn(*mut TileIterator) -> u32,
    pub iterator_next: extern "C" fn(*mut TileIterator),
    pub airport_noise: extern "C" fn(*const AirportSpec) -> u8,
    pub town_max_noise: extern "C" fn(*const crate::services::TownHandle) -> u16,
    pub station_town: extern "C" fn(*mut BaseStation) -> *const crate::services::TownHandle,
    pub airport_tile: extern "C" fn(*const BaseStation) -> u32,
    pub airport_type: extern "C" fn(*const BaseStation) -> u8,
    /// `Station::airport.rotation`.
    pub airport_rotation: extern "C" fn(*const BaseStation) -> u8,
    pub airport_airplanes: extern "C" fn(*const std::ffi::c_void) -> bool,
    pub set_airport_details: extern "C" fn(*mut BaseStation, u8, u8, u8),
    pub make_airport: extern "C" fn(u32, u8, u16, u8, u8),
    /// `airport`, in place.
    pub airport_area: extern "C" fn(*mut BaseStation) -> *mut TileArea,
    pub airport_animated: extern "C" fn(*const crate::animated_tile::AirportSpec) -> bool,

    pub airport_infrastructure: extern "C" fn(u8, u32),
    pub station_view_invalidate: extern "C" fn(*mut BaseStation, i32),
    pub town_view_dirty: extern "C" fn(*const crate::services::TownHandle),
    pub aircraft_vehicle: extern "C" fn(crate::vehicle::VehicleRef) -> crate::aircraft::Vehicle,
    pub hangar_count: extern "C" fn(*mut BaseStation, *const AirportSpec) -> u32,
    pub hangar_tile: extern "C" fn(*const BaseStation, u32, *const AirportSpec) -> u32,
    pub close_hangar: extern "C" fn(u32),
    pub close_airport_tile_inspect: extern "C" fn(u32),
    pub delete_airport_storage: extern "C" fn(*mut BaseStation),
    pub clear_airport: extern "C" fn(*mut BaseStation),
    pub close_airport_inspect: extern "C" fn(*mut BaseStation),
    pub ship: *const crate::ship_control::Leaves,
    pub clear: *const crate::ground::ClearServices,
    pub restoration: *const crate::water::RestorationServices,
    pub set_docking: extern "C" fn(u32, bool),
    pub set_ship_area: extern "C" fn(*mut BaseStation, TileArea),
    /// `ship_station`, in place.
    pub ship_station_area: extern "C" fn(*mut BaseStation) -> *mut TileArea,
    pub docking_clear: extern "C" fn(*mut BaseStation),
    /// `docking_station`, in place.
    pub docking_area: extern "C" fn(*mut BaseStation) -> *mut TileArea,
    pub water_infrastructure: extern "C" fn(u8, u32),
    pub make_dock: extern "C" fn(u32, u8, u16, u8, u8),
    pub neutral_for_tile: extern "C" fn(u32) -> *mut BaseStation,
    pub associate_industry: extern "C" fn(*mut BaseStation, u32),
    pub make_oilrig: extern "C" fn(u32, u16, u8),
    pub set_station_owner: extern "C" fn(*mut BaseStation, u8),
    pub set_airport_kind_rotation: extern "C" fn(*mut BaseStation, u8, u8),
    pub set_facilities: extern "C" fn(*mut BaseStation, u8),
    pub set_build_date: extern "C" fn(*mut BaseStation, i32),
    pub associated_industry: extern "C" fn(*mut BaseStation) -> *mut Industry,
    pub industry_neutral: extern "C" fn(*mut Industry) -> *mut BaseStation,
    pub clear_industry_neutral: extern "C" fn(*mut Industry),
    pub industry_area: extern "C" fn(*mut Industry) -> TileArea,
    pub autoslope_axis: extern "C" fn(u32, i32, u8, u8) -> bool,
    pub station_list_classes_invalidate: extern "C" fn(),
    // Waypoint command leaves; canonical shells and shared map services.
    pub clear_cached_name: extern "C" fn(*mut BaseStation),
    pub clear_name: extern "C" fn(*mut BaseStation),
    pub make_buoy: extern "C" fn(u32, u16, crate::services::WaterClass),
    pub make_rail_waypoint: extern "C" fn(u32, u8, u16, u8, u8, crate::services::RailType),
    pub new_waypoint: extern "C" fn(u32) -> *mut BaseStation,
    pub road_ground: *const crate::ground::RoadServices,
    /// `road_waypoint_area`, in place.
    pub road_waypoint_tile_area: extern "C" fn(*mut BaseStation) -> *mut TileArea,
    pub set_facility_bits: extern "C" fn(*mut BaseStation, u8),
    pub set_location: extern "C" fn(*mut BaseStation, u32),
    pub set_rail_station_reservation: extern "C" fn(u32, bool),
    pub set_road_waypoint: extern "C" fn(*mut BaseStation),
    pub set_string_id: extern "C" fn(*mut BaseStation, u32),
    pub string_id: extern "C" fn(*mut BaseStation) -> u32,
    pub tile_flat: extern "C" fn(u32) -> bool,
    pub waypoint_available: extern "C" fn() -> bool,
    pub waypoint_default_name: extern "C" fn(*mut BaseStation),
    pub waypoint_layout_gfx: extern "C" fn(*mut WaypointLayout, bool) -> u8,
    pub waypoint_layout_init: extern "C" fn(*mut WaypointLayout, *const RailSpec, u8),
    pub waypoint_view_invalidate: extern "C" fn(*mut BaseStation),
}
// Call-local helper contexts contain the original inputs and live native result
// slots. Mutable terrain/station scratch is reborrowed for one helper invocation;
// no world-state borrow is retained across a command or callback.
/// `IsStationBridgeAboveOk`'s arguments.
#[derive(Clone, Copy)]
struct StationBridgeCheck<'a> {
    out: Cost<'a>,
    /// Tile to test.
    tile: TileIndex,
    bridgeable_info: BridgeSpan,
    /// Type of station.
    station_type: StationType,
    /// Layout piece of road station to test.
    layout: StationGfx,
    /// Height of bridge to test.
    bridge_height: i32,
    /// Error message if bridge is disallowed.
    disallowed_msg: StringID,
}

/// `FindJoiningBaseStation`'s arguments.
#[derive(Clone, Copy)]
struct JoiningStation<'a> {
    out: Cost<'a>,
    /// An existing station we build over.
    existing_station: StationID,
    /// The station to join to.
    station_to_join: StationID,
    /// Whether adjacent stations are allowed.
    adjacent: bool,
    /// The area of the newly build station.
    ta: TileArea,
    /// Whether to find a waypoint (`FindJoiningWaypoint`) rather than a station.
    waypoint: bool,
    /// Whether to find a road waypoint.
    road: bool,
    /// The error message when building a station on top of others.
    error_message: StringID,
}

struct RailStationLandCheck<'a> {
    cost: Cost<'a>,
    ret: Cost<'a>,
    tile: u32,
    north: u32,
    allowed: &'a mut i32,
    flags: u16,
    axis: u8,
    est: &'a mut u16,
    rt: u8,
    affected: &'a mut Vec<*mut crate::services::Train>,
    spec: *const RailSpec,
    length: u8,
    tracks: u8,
}

struct RoadStopLandCheck<'a> {
    cost: Cost<'a>,
    ret: Cost<'a>,
    tile: u32,
    allowed: &'a mut i32,
    spec: *const RoadSpec,
    flags: u16,
    invalid: u8,
    drive: bool,
    kind: u8,
    axis: u8,
    est: &'a mut u16,
    rt: u8,
}

struct RoadStopCost<'a> {
    cost: Cost<'a>,
    ret: Cost<'a>,
    area: TileArea,
    flags: u16,
    drive: bool,
    kind: u8,
    spec: *const RoadSpec,
    axis: u8,
    dir: u8,
    est: &'a mut u16,
    rt: u8,
    unit: i64,
}

#[derive(Clone, Copy)]
struct RailStationRemoval<'a> {
    cost: Cost<'a>,
    ret: Cost<'a>,
    error: Cost<'a>,
    area: TileArea,
    flags: u16,
    unit: i64,
    keep: bool,
    waypoint: bool,
}

#[derive(Clone, Copy)]
struct RoadStopRemoval<'a> {
    cost: Cost<'a>,
    ret: Cost<'a>,
    error: Cost<'a>,
    flags: u16,
    area: TileArea,
    waypoint: bool,
    remove_road: bool,
}

#[derive(Clone, Copy)]
struct StationOwnerChange<'a> {
    ownership: &'a crate::tile_owner::Context,
    cost: Cost<'a>,
    ret: Cost<'a>,
    error: Cost<'a>,
    tile: u32,
    old: u8,
    new: u8,
}

struct Ctx<'a> {
    s: &'a CommandServicesTable,
    shared: &'a Services,
    cost: &'a CommandCostServices,
    command: &'a CommandServices,
    map: std::cell::Cell<Option<Map>>,
}
impl<'a> Ctx<'a> {
    unsafe fn new(s: *const CommandServicesTable) -> Self {
        // SAFETY: immutable native tables outlive every synchronous entry.
        let s = unsafe { &*s };
        let shared = unsafe { &*s.shared };
        Self {
            s,
            shared,
            cost: unsafe { &*s.cost },
            command: unsafe { &*s.command },
            map: std::cell::Cell::new(None),
        }
    }
    /// The catchment tables for this entry.
    fn catchment(&self) -> crate::station::Ctx<'a> {
        crate::station::Ctx::from_shared(self.shared)
    }
    /// `st->rect`, for immediate access only.
    fn rect(&self, st: *mut BaseStation) -> *mut crate::station::StationRect {
        // SAFETY: Live station owner.
        unsafe { crate::station::rect((self.world().owner)(st)) }
    }
    /// `*out = st->rect.BeforeAddRect(area, mode)`.
    fn before_add_rect(
        &self,
        out: Cost<'_>,
        st: *mut BaseStation,
        area: TileArea,
        mode: StationRectMode,
    ) {
        let c = self.catchment();
        let r = self.rect(st);
        // SAFETY: Local copy; no borrow spans the setting reads.
        let mut rect = unsafe { *r };
        let ok = rect.before_add_rect(
            TileIndex(area.tile),
            i32::from(area.w),
            i32::from(area.h),
            mode,
            &c,
        );
        unsafe { *r = rect };
        if ok {
            out.zero();
        } else {
            out.error(self.s.strings.spread);
        }
    }
    /// `*out = st->rect.BeforeAddTile(tile, mode)`.
    fn before_add_tile(
        &self,
        out: Cost<'_>,
        st: *mut BaseStation,
        tile: TileIndex,
        mode: StationRectMode,
    ) {
        let c = self.catchment();
        let r = self.rect(st);
        // SAFETY: Local copy; no borrow spans the setting reads.
        let mut rect = unsafe { *r };
        let ok = rect.before_add_tile(tile, mode, &c);
        unsafe { *r = rect };
        if ok {
            out.zero();
        } else {
            out.error(self.s.strings.spread);
        }
    }
    /// `st->rect.AfterRemoveTile(st, tile)`.
    fn after_remove_tile(&self, st: *mut BaseStation, tile: TileIndex) {
        let c = self.catchment();
        let id = (c.c.id)(st);
        let r = self.rect(st);
        // SAFETY: Local copy; no borrow spans the map scans.
        let mut rect = unsafe { *r };
        rect.after_remove_tile(id, tile, &c);
        unsafe { *r = rect };
    }
    /// `st->rect.AfterRemoveRect(st, area)`.
    fn after_remove_rect(&self, st: *mut BaseStation, area: TileArea) {
        let c = self.catchment();
        let id = (c.c.id)(st);
        let r = self.rect(st);
        // SAFETY: Local copy; no borrow spans the map scans.
        let mut rect = unsafe { *r };
        rect.after_remove_rect(id, area, &c);
        unsafe { *r = rect };
    }
    /// `st->rect.PtInExtendedRect(TileX(tile), TileY(tile))`.
    #[allow(clippy::cast_possible_wrap)]
    fn rect_contains(&self, st: *mut BaseStation, tile: TileIndex) -> bool {
        let map = self.map();
        // SAFETY: Live owner; immediate copy.
        let rect = unsafe { *self.rect(st) };
        rect.pt_in_extended_rect(map.tile_x(tile.0) as i32, map.tile_y(tile.0) as i32, 0)
    }
    /// `Station::From(st)->RecomputeCatchment(no_clear)`.
    fn recompute_catchment(&self, st: *mut BaseStation, no_clear_nearby_lists: bool) {
        crate::station::recompute_catchment(st, no_clear_nearby_lists, &self.catchment());
    }
    /// Read a station area at this observation point.
    fn area(area: *mut TileArea) -> TileArea {
        // SAFETY: The accessor addresses a live station area.
        unsafe { area.read() }
    }
    /// `TileArea::Add` on a station area resolved in place for this call.
    fn area_add(&self, area: *mut TileArea, tile: TileIndex) {
        // SAFETY: The accessor addresses a live station area; nothing else borrows it.
        unsafe { (*area).add(tile.0, self.map()) };
    }
    fn map(&self) -> Map {
        if let Some(map) = self.map.get() {
            return map;
        }
        let map = self.shared.map();
        self.map.set(Some(map));
        map
    }
    fn c(&self, cost: *mut CommandCost) -> Cost<'a> {
        Cost(cost, self.cost)
    }
    /// `CheckBuildableTile`: checks if the given tile is buildable, flat and has
    /// a certain height. If `allowed_z` is negative, it will be set to the
    /// height of this tile.
    fn check_buildable(
        &self,
        out: Cost<'_>,
        tile: TileIndex,
        invalid_dirs: DiagDirections,
        allowed_z: &mut i32,
        allow_steep: bool,
        check_bridge: bool,
    ) {
        if check_bridge && crate::map_func::World::new(self.shared).bridge_above(tile.0) {
            return out.error(self.s.strings.demolish_bridge);
        }
        // SAFETY: the native output is a live command slot for this entry.
        unsafe { self.command.no_vehicle(out.0, tile.0) };
        if !out.ok() {
            return;
        }
        let tileh = Slope(crate::map_func::World::new(self.shared).tile_slope(tile.0));
        let z = (self.shared.tile_z)(tile.0);
        /* Prohibit building if
         *   1) The tile is "steep" (i.e. stretches two height levels).
         *   2) The tile is non-flat and the build_on_slopes switch is disabled.
         */
        if (!allow_steep && is_steep_slope(tileh.0))
            || (!(self.shared.build_on_slopes)() && tileh != Slope::SLOPE_FLAT)
        {
            return out.error(self.s.strings.flat_land);
        }
        out.set(0);
        let flat_z = z + get_slope_max_z(tileh.0);
        if tileh != Slope::SLOPE_FLAT {
            /* Forbid building if the tile faces a slope in a invalid direction. */
            for dir in DiagDirection::DIAGDIR_BEGIN.0..DiagDirection::DIAGDIR_END.0 {
                let dir = DiagDirection(dir);
                if invalid_dirs.test(dir) && !can_build_depot_by_tileh(dir, tileh) {
                    return out.error(self.s.strings.flat_land);
                }
            }
            out.add_money(crate::company::price(usize::from(
                Price::PR_BUILD_FOUNDATION.0,
            )));
        }
        /* The level of this tile must be equal to allowed_z. */
        if *allowed_z < 0 {
            /* First tile. */
            *allowed_z = flat_z;
        } else if *allowed_z != flat_z {
            out.error(self.s.strings.flat_land);
        }
    }
    /// `GetBridgeHeight`.
    fn bridge_height(&self, tile: TileIndex) -> i32 {
        crate::bridge_map::get_bridge_height(tile.0, self.shared)
    }
    /// `IsStationBridgeAboveOk`: test if a bridge can be built above a station.
    fn is_station_bridge_above_ok(&self, context: StationBridgeCheck<'_>) {
        let StationBridgeCheck {
            out,
            tile,
            bridgeable_info,
            station_type,
            layout,
            bridge_height,
            disallowed_msg,
        } = context;
        let height = if usize::from(layout.0) < bridgeable_info.size {
            // SAFETY: native immutable span, checked index; no references survive.
            unsafe { (&raw const (*bridgeable_info.data.add(usize::from(layout.0))).height).read() }
        } else {
            0
        };
        if height == 0 {
            if disallowed_msg != StringID::INVALID_STRING_ID {
                return out.error(disallowed_msg.0);
            }
            /* Get normal error message associated with clearing the tile. */
            crate::landscape_cmd::clear_command(
                self.command,
                out.0,
                tile.0,
                DoCommandFlags::from_element(DoCommandFlag::Auto).0,
            );
            return;
        }
        if (self.shared.tile_max_z)(tile.0) + i32::from(height) > bridge_height {
            // The original multiplies by `TILE_HEIGHT_STEP` (50); this port has
            // used 8 since it was written. Reported to root; behaviour kept.
            let height_diff =
                ((self.shared.tile_max_z)(tile.0) + i32::from(height) - bridge_height) * 8;
            // `GetBridgeTooLowMessageForStationType`.
            (self.cost.error_param)(
                out.0,
                self.s.strings.bridge_too_low[usize::from(station_type.0)],
                height_diff as u64,
            );
            return;
        }
        out.zero();
    }
    /// The shared body of `IsRailStationBridgeAboveOk`, `IsRoadStationBridgeAboveOk`,
    /// `IsDockBridgeAboveOk` and `IsBuoyBridgeAboveOk` without a custom spec: test
    /// if a station part can be built below a bridge.
    fn default_bridge_above_ok(
        &self,
        out: Cost<'_>,
        tile: TileIndex,
        station_type: StationType,
        layout: StationGfx,
    ) {
        if !crate::map_func::World::new(self.shared).bridge_above(tile.0) {
            return out.zero();
        }
        let rampsouth = TileIndex((self.s.southern_bridge_end)(tile.0));
        self.is_station_bridge_above_ok(StationBridgeCheck {
            out,
            tile,
            bridgeable_info: (self.s.default_bridge_info)(station_type.0),
            station_type,
            layout,
            bridge_height: self.bridge_height(rampsouth),
            disallowed_msg: StringID(self.s.strings.demolish_bridge),
        });
    }
}

/// Check one construction tile, preserving the original mutable height output.
/// # Safety
/// Tables and native output are live for this call; `allowed_z` is exclusively borrowed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_check_buildable(
    s: *const CommandServicesTable,
    out: *mut CommandCost,
    tile: u32,
    invalid: u8,
    allowed_z: *mut i32,
    allow_steep: bool,
    check_bridge: bool,
) {
    let c = unsafe { Ctx::new(s) };
    c.check_buildable(
        c.c(out),
        TileIndex(tile),
        DiagDirections(invalid),
        unsafe { &mut *allowed_z },
        allow_steep,
        check_bridge,
    );
}

/// Check building a road stop below a bridge.
/// # Safety
/// Native tables, optional spec and output are live for this synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_road_bridge_ok(
    s: *const CommandServicesTable,
    out: *mut CommandCost,
    tile: u32,
    spec: *const RoadSpec,
    kind: u8,
    layout: u8,
) {
    let c = unsafe { Ctx::new(s) };
    // `IsRoadStationBridgeAboveOk`.
    let (tile, station_type, layout) = (TileIndex(tile), StationType(kind), StationGfx(layout));
    if !crate::map_func::World::new(c.shared).bridge_above(tile.0) {
        return c.c(out).zero();
    }
    let rampsouth = TileIndex((c.s.southern_bridge_end)(tile.0));
    let bridgeable_info = if spec.is_null() {
        (c.s.default_bridge_info)(station_type.0)
    } else {
        unsafe { (c.s.road_bridge_info)(spec) }
    };
    c.is_station_bridge_above_ok(StationBridgeCheck {
        out: c.c(out),
        tile,
        bridgeable_info,
        station_type,
        layout,
        bridge_height: c.bridge_height(rampsouth),
        disallowed_msg: StringID(c.s.strings.demolish_bridge),
    });
}

/// Source uint8 tolerance-distance truncation and noise reduction.
/// # Safety
/// The immutable native table and authority table are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_airport_noise(
    s: *const CommandServicesTable,
    noise: u8,
    distance: u32,
) -> u8 {
    // `GetAirportNoiseLevelForDistance`.
    /* 0 cannot be accounted, and 1 is the lowest that can be reduced from town.
     * So no need to go any further*/
    if noise < 2 {
        return noise;
    }
    let s = unsafe { &*s };
    let authority = unsafe { &*s.authority };
    /* The steps for measuring noise reduction are based on the "magical" (and arbitrary) 8 base distance
     * adding the town_council_tolerance 4 times, as a way to graduate, depending of the tolerance.
     * Basically, it says that the less tolerant a town is, the bigger the distance before
     * an actual decrease can be granted */
    let town_tolerance_distance =
        8_u8.wrapping_add((authority.council_tolerance)().wrapping_mul(4));
    /* now, we want to have the distance segmented using the distance judged bareable by town
     * This will give us the coefficient of reduction the distance provides. */
    let noise_reduction = distance / u32::from(town_tolerance_distance);
    /* If the noise reduction equals the airport noise itself, don't give it for free.
     * Otherwise, simply reduce the airport's level. */
    if noise_reduction >= u32::from(noise) {
        1
    } else {
        (u32::from(noise) - noise_reduction) as u8
    }
}

/// Check bridge construction above the existing station's actual spec.
/// # Safety
/// Immutable native tables and output slot are live for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_build_above(
    s: *const CommandServicesTable,
    out: *mut CommandCost,
    tile: u32,
    height: i32,
) {
    crate::witness::hit(crate::witness::STATION_CMD_BUILD_ABOVE);
    let c = unsafe { Ctx::new(s) };
    // `CheckBuildAbove_Station`.
    let tile = TileIndex(tile);
    let station_type = StationType((c.shared.station_type)(tile.0).0);
    let mut bridgeable_info = (c.s.default_bridge_info)(station_type.0);
    match station_type {
        StationType::Rail | StationType::RailWaypoint => {
            let spec = (c.s.rail_spec)(tile.0);
            if !spec.is_null() {
                bridgeable_info = unsafe { (c.s.rail_bridge_info)(spec) };
            }
        }
        StationType::Bus | StationType::Truck | StationType::RoadWaypoint => {
            let spec = unsafe {
                crate::newgrf_roadstop::openttd_rust_roadstop_spec(tile.0, c.s.roadstop_resolver)
            };
            if !spec.is_null() {
                bridgeable_info = unsafe { (c.s.road_bridge_info)(spec) };
            }
        }
        _ => (),
    }
    c.is_station_bridge_above_ok(StationBridgeCheck {
        out: c.c(out),
        tile,
        bridgeable_info,
        station_type,
        layout: StationGfx((c.shared.station_gfx)(tile.0)),
        bridge_height: height,
        disallowed_msg: StringID::INVALID_STRING_ID,
    });
}

impl Ctx<'_> {
    fn world(&self) -> &World {
        unsafe { &*self.s.station }
    }
    fn links(&self) -> &crate::station_service::Links {
        unsafe { &*self.s.links }
    }
    /// `Station::GetIfValid`.
    fn full_station(&self, id: StationID) -> *mut BaseStation {
        let st = (self.world().get)(id.0);
        if st.is_null() || !(self.world().is_station)(st) {
            std::ptr::null_mut()
        } else {
            st
        }
    }
    /// `Waypoint::GetIfValid`.
    fn waypoint(&self, id: StationID) -> *mut BaseStation {
        let st = (self.world().get)(id.0);
        if st.is_null() || (self.world().is_station)(st) {
            std::ptr::null_mut()
        } else {
            st
        }
    }
    /// `UpdateStationSignCoord`.
    fn update_station_sign_coord(&self, st: *mut BaseStation) {
        let owner = (self.world().owner)(st);
        // SAFETY: Live owner; immediate reads.
        if unsafe { crate::station::rect_is_empty(owner) } {
            return; // no tiles belong to this station
        }
        let r = unsafe { *crate::station::rect(owner) };
        /* clamp sign coord to be inside the station rect */
        let xy = (self.s.location)(st);
        let new_xy = TileIndex(self.map().tile_xy(
            (self.map().tile_x(xy) as i32).clamp(r.left, r.right) as u32,
            (self.map().tile_y(xy) as i32).clamp(r.top, r.bottom) as u32,
        ));
        crate::station::spatial::move_sign(self.shared, st, new_xy.0);
        if !(self.world().is_station)(st) {
            return;
        }
        for cargo in 0..NUM_CARGO {
            let lg = crate::linkgraph::live::station_graph(st, cargo, self.links().graphs);
            if lg.is_null() {
                continue;
            }
            let node = (self.links().graphs.node)(st, cargo);
            unsafe {
                crate::linkgraph::live::openttd_rust_live_node_location(
                    lg,
                    node,
                    (self.s.location)(st),
                );
            }
        }
    }
    /// `DeleteStationIfEmpty`: this is called right after a station was deleted.
    /// It checks if the whole station is free of substations, and if so, the
    /// station will be deleted after a little while.
    fn delete_station_if_empty(&self, st: *mut BaseStation) {
        let owner = (self.world().owner)(st);
        if !(self.world().in_use)(st) {
            // SAFETY: canonical owner, serialized scalar write ends before UI reentry.
            let fields =
                unsafe { crate::station_service::openttd_rust_station_service_fields(owner) };
            unsafe { (&raw mut (*fields).delete_ctr).write(0) };
            (self.world().station_list_invalidation)(st, 0);
        }
        /* station remains but it probably lost some parts - station sign should stay in the station boundaries */
        self.update_station_sign_coord(st);
    }
    /// `Station::AfterStationTileSetChange`: after adding/removing tiles to
    /// station, update some station-related stuff.
    fn after_station_tile_set_change(
        &self,
        st: *mut BaseStation,
        adding: bool,
        station_type: StationType,
    ) {
        (self.s.virtual_coord)(st);
        (self.command.infrastructure_dirty)((self.world().station_owner)(st));
        if adding {
            self.recompute_catchment(st, false);
            (self.s.catchment_dirty)();
            (self.world().station_list_invalidation)(st, 0);
        } else {
            (self.s.catchment_dirty)();
        }
        (self.s.facility_widget)(st, station_type.0);
        if adding {
            // `UpdateStationAcceptance(st, false)`, owned by `station_service.rs`.
            unsafe {
                crate::station_service::openttd_rust_station_acceptance(st, self.s.station, 0);
            };
            (self.s.select_invalidate)();
        } else {
            self.delete_station_if_empty(st);
            self.recompute_catchment(st, false);
        }
    }
    /// `FindJoiningBaseStation`: find a nearby station that joins this station.
    fn find_joining_base_station(&self, context: JoiningStation<'_>) -> *mut BaseStation {
        let JoiningStation {
            out,
            existing_station,
            station_to_join,
            adjacent,
            ta,
            waypoint,
            road,
            error_message,
        } = context;
        crate::witness::hit(crate::witness::STATION_CMD_JOIN);
        let mut st = std::ptr::null_mut();
        let mut check_surrounding = true;
        let get_if_valid = |id| {
            if waypoint {
                self.waypoint(id)
            } else {
                self.full_station(id)
            }
        };
        let filter = |p| !waypoint || (self.s.road_waypoint)(p) == road;
        if existing_station != StationID::INVALID {
            if adjacent && existing_station != station_to_join {
                /* You can't build an adjacent station over the top of one that
                 * already exists. */
                out.error(error_message.0);
                return st;
            }
            /* Extend the current station, and don't check whether it will
             * be near any other stations. */
            let candidate = get_if_valid(existing_station);
            if !candidate.is_null() && filter(candidate) {
                st = candidate;
            }
            check_surrounding = st.is_null();
        } else {
            /* There's no station here. Don't check the tiles surrounding this
             * one if the company wanted to build an adjacent station. */
            if adjacent {
                check_surrounding = false;
            }
        }
        if check_surrounding {
            /* Make sure there is no more than one other station around us that is owned by us. */
            // `GetStationAround`.
            let mut ta = ta;
            ta.expand(1, self.map(), (self.shared.map_max_y)() + 1);
            let company = CompanyID((self.shared.current_company)());
            let mut closest_station = existing_station;
            /* check around to see if there are any stations there owned by the company */
            for tile_cur in ta.tiles(self.map()) {
                if TileType((self.shared.tile_type)(tile_cur)) != TileType::MP_STATION {
                    continue;
                }
                let t = StationID((self.shared.station_index)(tile_cur));
                let candidate = get_if_valid(t);
                if candidate.is_null()
                    || CompanyID((self.world().station_owner)(candidate)) != company
                    || !filter(candidate)
                {
                    continue;
                }
                if closest_station == StationID::INVALID {
                    closest_station = t;
                } else if closest_station != t {
                    out.error(self.s.strings.adjoins_multiple);
                    return std::ptr::null_mut();
                }
            }
            st = if closest_station == StationID::INVALID {
                std::ptr::null_mut()
            } else {
                get_if_valid(closest_station)
            };
        }
        /* Distant join */
        if st.is_null() && station_to_join != StationID::INVALID {
            st = get_if_valid(station_to_join);
        }
        out.zero();
        st
    }
    /// `GetClosestDeletedStation`: find the closest deleted station of the
    /// current company, or null if too far.
    fn get_closest_deleted_station(&self, tile: TileIndex) -> *mut BaseStation {
        let mut threshold = 8;
        let mut best_station: *mut BaseStation = std::ptr::null_mut();
        crate::station::spatial::radius(self.shared, tile.0, threshold, |st| {
            if (self.world().in_use)(st)
                || CompanyID((self.world().station_owner)(st))
                    != CompanyID((self.shared.current_company)())
            {
                return;
            }
            let cur_dist = self.map().distance_manhattan(tile.0, (self.s.location)(st));
            if cur_dist < threshold {
                threshold = cur_dist;
                best_station = st;
            } else if cur_dist == threshold && !best_station.is_null() {
                /* In case of a tie, lowest station ID wins */
                if (self.links().graphs.station_id)(st)
                    < (self.links().graphs.station_id)(best_station)
                {
                    best_station = st;
                }
            }
        });
        best_station
    }
    /// `BuildStationPart`: common part of building various station parts and
    /// possibly attaching them to an existing one; returns the station to attach to.
    fn build_station_part(
        &self,
        out: Cost<'_>,
        mut st: *mut BaseStation,
        flags: DoCommandFlags,
        reuse: bool,
        area: TileArea,
        name_class: StationNaming,
    ) -> *mut BaseStation {
        /* Find a deleted station close to us */
        if st.is_null() && reuse {
            st = self.get_closest_deleted_station(TileIndex(area.tile));
            if !st.is_null() {
                crate::witness::hit(crate::witness::STATION_CMD_REUSE);
            }
        }
        if !st.is_null() {
            if CompanyID((self.world().station_owner)(st))
                != CompanyID((self.shared.current_company)())
            {
                out.error(StringID::INVALID_STRING_ID.0);
                return st;
            }
            self.before_add_rect(out, st, area, StationRectMode::ADD_TEST);
            if !out.ok() {
                return st;
            }
        } else {
            /* allocate and initialize new station */
            if !(self.s.station_available)() {
                out.error(self.s.strings.too_many_stations);
                return st;
            }
            if flags.test(DoCommandFlag::Execute) {
                st = (self.s.new_station)(area.tile);
                crate::station::spatial::insert(self.shared, st);
                let town = crate::town::spatial::closest(self.shared, area.tile, u32::MAX);
                (self.s.set_town)(st, town);
                (self.s.set_string_id)(
                    st,
                    crate::station_lifecycle::generate_name(st, area.tile, name_class, unsafe {
                        &*self.s.lifecycle
                    }),
                );
                let current = CompanyID((self.shared.current_company)());
                if (self.shared.company_valid)(current.0) {
                    (self.s.have_rating)(st, current.0);
                }
            }
        }
        out.zero();
        st
    }
}

/// # Safety
/// Native tables and full station shell/owner are live under the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_tiles_changed(
    s: *const CommandServicesTable,
    st: *mut BaseStation,
    adding: bool,
    kind: u8,
) {
    unsafe { Ctx::new(s) }.after_station_tile_set_change(st, adding, StationType(kind));
}

/// `CmdRenameStation`'s arguments.
#[derive(Clone, Copy)]
struct CmdRenameStation {
    /// Station ID that is to be renamed.
    station_id: StationID,
    /// The new name, or an empty string when resetting to the default.
    text: *const u8,
    /// The byte length of `text`.
    length: usize,
}
/// `CmdMoveStationName`'s arguments.
#[derive(Clone, Copy)]
struct CmdMoveStationName {
    /// Id of the station.
    station_id: StationID,
    /// Tile to move the station name to.
    tile: TileIndex,
}
/// `CmdOpenCloseAirport`'s arguments.
#[derive(Clone, Copy)]
struct CmdOpenCloseAirport {
    /// Station ID of the airport.
    station_id: StationID,
}

impl Ctx<'_> {
    /// `CmdRenameStation`: rename a station.
    fn cmd_rename_station(&self, out: Cost<'_>, flags: DoCommandFlags, args: CmdRenameStation) {
        crate::witness::hit(crate::witness::STATION_CMD_RENAME);
        let st = self.full_station(args.station_id);
        if st.is_null() {
            return out.error(StringID::INVALID_STRING_ID.0);
        }
        unsafe {
            self.command
                .ownership(out.0, (self.world().station_owner)(st), 0);
        }
        if !out.ok() {
            return;
        }
        let reset = args.length == 0;
        if !reset {
            if (self.shared.utf8_length)(args.text, args.length)
                >= crate::station_type::MAX_LENGTH_STATION_NAME_CHARS
            {
                return out.error(StringID::INVALID_STRING_ID.0);
            }
            // `IsUniqueStationName`.
            for index in 0..(self.world().pool_size)() {
                let candidate = self.full_station(StationID(index as u16));
                if !candidate.is_null() && (self.s.name_equals)(candidate, args.text, args.length) {
                    return out.error(self.s.strings.name_unique);
                }
            }
        }
        if flags.test(DoCommandFlag::Execute) {
            (self.s.clear_cached_name)(st);
            if reset {
                (self.s.clear_name)(st);
            } else {
                (self.s.set_name)(st, args.text, args.length);
            }
            (self.s.virtual_coord)(st);
            (self.world().station_list_invalidation)(st, 1);
        }
        out.zero();
    }
    /// `CmdMoveStationName`: move a station name; returns the station ID.
    fn cmd_move_station_name(
        &self,
        out: Cost<'_>,
        flags: DoCommandFlags,
        args: CmdMoveStationName,
    ) -> StationID {
        crate::witness::hit(crate::witness::STATION_CMD_MOVE_NAME);
        let tile = args.tile;
        let st = self.full_station(args.station_id);
        if st.is_null() {
            out.error(StringID::INVALID_STRING_ID.0);
            return StationID::INVALID;
        }
        let owner = CompanyID((self.world().station_owner)(st));
        if owner != CompanyID::NONE {
            unsafe {
                self.command.ownership(out.0, owner.0, 0);
            }
            if !out.ok() {
                return StationID::INVALID;
            }
        }
        if !self.rect_contains(st, tile) {
            out.error(self.s.strings.unsuitable);
            return StationID::INVALID;
        }
        let mut search = SignSearch {
            c: self,
            station: st,
            tile: tile.0,
            other: false,
        };
        /* Check if the tile is the base tile of another station */
        crate::station::spatial::radius(self.shared, tile.0, 0, |st| unsafe {
            sign_visit((&raw mut search).cast(), st);
        });
        if search.other {
            out.error(self.s.strings.unsuitable);
            return StationID::INVALID;
        }
        if flags.test(DoCommandFlag::Execute) {
            crate::station::spatial::move_sign(self.shared, st, tile.0);
            (self.s.virtual_coord)(st);
        }
        out.zero();
        args.station_id
    }
    /// `CmdOpenCloseAirport`: open/close an airport to incoming aircraft.
    fn cmd_open_close_airport(
        &self,
        out: Cost<'_>,
        flags: DoCommandFlags,
        args: CmdOpenCloseAirport,
    ) {
        let st = self.full_station(args.station_id);
        if st.is_null() {
            return out.error(StringID::INVALID_STRING_ID.0);
        }
        let owner = CompanyID((self.world().station_owner)(st));
        if !StationFacilities((self.world().facilities)(st)).test(StationFacility::Airport)
            || owner == CompanyID::NONE
        {
            return out.error(StringID::INVALID_STRING_ID.0);
        }
        unsafe {
            self.command.ownership(out.0, owner.0, 0);
        }
        if !out.ok() {
            return;
        }
        if flags.test(DoCommandFlag::Execute) {
            (self.s.flip_airport_closed)(st);
            (self.s.closed_widget)(st);
        }
        out.zero();
    }
    /// `MakeStationAreaSmaller`, with `TileBelongsToRailStation`,
    /// `TileBelongsToShipStation` or `TileBelongsToRoadWaypointStation` as the
    /// tile test for `station_type`.
    fn make_station_area_smaller(
        &self,
        st: *mut BaseStation,
        mut ta: TileArea,
        station_type: StationType,
    ) -> TileArea {
        let id = (self.links().graphs.station_id)(st);
        // A waypoint's rail tiles are `RailWaypoint` tiles.
        let station_type = if station_type == StationType::Rail && !(self.world().is_station)(st) {
            StationType::RailWaypoint
        } else {
            station_type
        };
        let func = |tile| {
            TileType((self.shared.tile_type)(tile)) == TileType::MP_STATION
                && StationType((self.shared.station_type)(tile).0) == station_type
                && (self.shared.station_index)(tile) == id
        };
        'restart: loop {
            /* too small? */
            if ta.w == 0 || ta.h == 0 {
                return TileArea {
                    tile: TileIndex::INVALID.0,
                    w: 0,
                    h: 0,
                };
            }
            /* check the left side, x = constant, y changes;
             * check the right side, x = constant, y changes;
             * check the upper side, y = constant, x changes;
             * check the lower side, y = constant, x changes */
            for (x, vertical) in [
                (0, true),
                (i32::from(ta.w) - 1, true),
                (0, false),
                (i32::from(ta.h) - 1, false),
            ] {
                let size = if vertical { ta.h } else { ta.w };
                let mut found = false;
                for i in 0..size {
                    let tile = if vertical {
                        ta.tile.wrapping_add(self.map().diff(x, i32::from(i)))
                    } else {
                        ta.tile.wrapping_add(self.map().diff(i32::from(i), x))
                    };
                    if func(tile) {
                        found = true;
                        break;
                    }
                }
                /* the side is unused? */
                if !found {
                    if vertical {
                        if x == 0 {
                            ta.tile = ta.tile.wrapping_add(self.map().diff(1, 0));
                        }
                        ta.w -= 1;
                    } else {
                        if x == 0 {
                            ta.tile = ta.tile.wrapping_add(self.map().diff(0, 1));
                        }
                        ta.h -= 1;
                    }
                    continue 'restart;
                }
            }
            return ta;
        }
    }
}
struct SignSearch<'a> {
    c: &'a Ctx<'a>,
    station: *mut BaseStation,
    tile: u32,
    other: bool,
}
unsafe extern "C" fn sign_visit(context: *mut std::ffi::c_void, st: *mut BaseStation) {
    let s = unsafe { &mut *context.cast::<SignSearch<'_>>() };
    if !st.is_null() && st != s.station && (s.c.s.location)(st) == s.tile {
        s.other = true;
    }
}
/// # Safety
/// Native result, serialized map/pool and borrowed UTF-8 bytes remain live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_rename(
    s: *const CommandServicesTable,
    out: *mut CommandCost,
    flags: u16,
    id: u16,
    text: *const u8,
    length: usize,
) {
    let c = unsafe { Ctx::new(s) };
    c.cmd_rename_station(
        c.c(out),
        DoCommandFlags(flags),
        CmdRenameStation {
            station_id: StationID(id),
            text,
            length,
        },
    );
}
/// # Safety
/// Tables and native result are live for the synchronous serialized entry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_move_name(
    s: *const CommandServicesTable,
    out: *mut CommandCost,
    flags: u16,
    id: u16,
    tile: u32,
) -> u16 {
    let c = unsafe { Ctx::new(s) };
    c.cmd_move_station_name(
        c.c(out),
        DoCommandFlags(flags),
        CmdMoveStationName {
            station_id: StationID(id),
            tile: TileIndex(tile),
        },
    )
    .0
}
/// # Safety
/// Tables and native result are live for the synchronous serialized entry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_open_close_airport(
    s: *const CommandServicesTable,
    out: *mut CommandCost,
    flags: u16,
    id: u16,
) {
    let c = unsafe { Ctx::new(s) };
    c.cmd_open_close_airport(
        c.c(out),
        DoCommandFlags(flags),
        CmdOpenCloseAirport {
            station_id: StationID(id),
        },
    );
}

impl Ctx<'_> {
    fn rail(&self) -> &RailCommandServices {
        unsafe { &*self.s.rail }
    }
    fn rail_map(&self) -> &crate::services::RailMapServices {
        unsafe { &*self.rail().rail }
    }
    fn authority(&self, out: Cost<'_>, tile: u32, flags: u16) {
        unsafe {
            crate::town::openttd_rust_town_authority_station(
                self.s.authority,
                self.s.shared,
                self.s.cost,
                out.0,
                tile,
                flags & DoCommandFlag::NO_TEST_TOWN_RATING != 0,
            );
        }
    }
    fn rail_station(&self, tile: u32) -> bool {
        crate::map_func::World::new(self.shared).rail_station(tile)
    }
    fn station_axis(&self, tile: u32) -> u8 {
        crate::services::Track(crate::map_func::World::new(self.shared).station_track(tile)).0
    }
    fn buildable_area(&self, tile: u32, w: u16, h: u16) -> bool {
        (self.shared.tile_valid)(tile)
            && self.map().tile_add_wrap(
                tile,
                i32::from(w) - 1,
                i32::from(h) - 1,
                (self.shared.map_max_y)() + 1,
                (self.world().environment.freeform_edges)(),
            ) != crate::tile_type::INVALID_TILE
    }
    fn free_reservation(&self, train: *mut crate::services::Train) {
        unsafe {
            let handle = crate::pbs::Pbs::acquire(self.shared).handle(train);
            crate::train_reservation::openttd_rust_train_reservation_free(
                handle,
                handle.vehicle(),
                self.s.reservation,
                std::ptr::null_mut(),
            );
        }
        self.platform_end(train, false, false);
        let last = unsafe { (self.s.last_train)(train) };
        self.platform_end(last, true, false);
    }
    fn restore_reservation(&self, train: *mut crate::services::Train) {
        self.platform_end(train, false, true);
        unsafe {
            let handle = crate::pbs::Pbs::acquire(self.shared).handle(train);
            crate::train_reservation::openttd_rust_train_reservation_try_path(
                handle,
                handle.vehicle(),
                1,
                1,
                self.s.reservation,
                std::ptr::null_mut(),
            );
        }
        let last = unsafe { (self.s.last_train)(train) };
        self.platform_end(last, true, true);
    }
    fn platform_end(&self, train: *mut crate::services::Train, reverse: bool, reserved: bool) {
        let handle = crate::pbs::Pbs::acquire(self.shared).handle(train);
        // SAFETY: The reached platform update always supplies a live train shell.
        let vehicle = handle.vehicle();
        let tile = (vehicle).tile();
        if self.rail_station(tile) {
            // SAFETY: The shell and canonical immutable tables remain live for this call.
            let trackdir = unsafe {
                crate::train::openttd_rust_train_trackdir(handle, self.s.train, self.s.shared)
            };
            let td = trackdir ^ if reverse { 8 } else { 0 };
            crate::pbs::Pbs::acquire(self.shared).platform(
                tile,
                crate::track_func::trackdir_to_exitdir(td),
                reserved,
            );
        }
    }
    fn flat_rail(&self, context: RailStationLandCheck<'_>) {
        let RailStationLandCheck {
            cost,
            ret,
            tile,
            north,
            allowed,
            flags,
            axis,
            est,
            rt,
            affected,
            spec,
            length,
            tracks,
        } = context;
        let invalid = if axis == 0 { 5 } else { 10 };
        let slope_callback = !spec.is_null() && (self.s.rail_spec_info)(spec).slope_callback;
        self.check_buildable(
            ret,
            TileIndex(tile),
            DiagDirections(invalid),
            allowed,
            false,
            false,
        );
        if !ret.ok() {
            return cost.take(ret);
        }
        cost.set(0);
        cost.add_money(ret.money());
        if slope_callback {
            unsafe {
                crate::newgrf_station::openttd_rust_station_slope_check(
                    ret.0, north, tile, spec, axis, length, tracks,
                );
            }
            if !ret.ok() {
                return cost.take(ret);
            }
        }
        if (self.shared.tile_type)(tile) == MP_STATION {
            if !self.rail_station(tile) {
                return self.auto_clear(cost, tile);
            }
            let id = (self.shared.station_index)(tile);
            if *est == StationID::INVALID.0 {
                *est = id;
            } else if *est != id {
                cost.error(self.s.strings.adjoins_multiple);
            }
        } else {
            if rt != (self.rail_map().invalid_rail_type)().0 && (self.shared.is_plain_rail)(tile) {
                if crate::map_func::World::new(self.shared).has_signals(tile) {
                    return cost.error(self.s.strings.remove_signals);
                }
                if (self.rail().powered_rail)(
                    crate::services::RailType(
                        crate::map_func::World::new(self.shared).rail_type(tile),
                    )
                    .0,
                    rt,
                ) {
                    let bits = (self.rail_map().traversal)(tile).tracks.0;
                    let track = bits.trailing_zeros() as u8;
                    if bits & bits.wrapping_sub(1) == 0 && track == axis {
                        if crate::pbs::Pbs::acquire(self.shared).reserved(tile) & (1 << track) != 0
                        {
                            let train = crate::pbs::Pbs::acquire(self.shared)
                                .train_for(tile, track)
                                .shell()
                                .cast::<crate::services::Train>();
                            if !train.is_null() {
                                affected.push(train);
                            }
                        }
                        // Reuse the two native slots across the nested command;
                        // the accumulated station cost is scalar and already successful.
                        let saved = cost.money();
                        self.nested(ret, tile, flags, |flags| unsafe {
                            crate::rail_cmd::openttd_rust_rail_remove_single(
                                self.s.rail,
                                ret.0,
                                cost.0,
                                flags,
                                tile,
                                track,
                            );
                        });
                        cost.set(saved);
                        if !ret.ok() {
                            return cost.take(ret);
                        }
                        cost.add_money(ret.money());
                        return;
                    }
                }
            }
            crate::landscape_cmd::clear_command(self.command, ret.0, tile, flags);
            if !ret.ok() {
                return cost.take(ret);
            }
            cost.add_money(ret.money());
        }
    }
    fn set_rail_flags(&self, tile: u32, spec: *const RailSpec) {
        let gfx = (self.shared.station_gfx)(tile);
        let flags = if spec.is_null() || usize::from(gfx) >= (self.s.tile_flag_count)(spec) {
            u8::from(gfx < 4)
        } else {
            (self.s.tile_flags)(spec, gfx)
        };
        (self.s.set_tile_flags)(
            tile,
            TileFlags(flags).test(TileFlag::Blocked),
            TileFlags(flags).test(TileFlag::Pylons),
            !TileFlags(flags).test(TileFlag::NoWires),
        );
    }
    fn layout_gfx(custom: crate::services::ByteSpan, index: u32, tracks: u8, length: u8) -> u8 {
        if custom.size != 0 {
            return unsafe { custom.data.add(index as usize).read() } & !1;
        }
        let length = u32::from(length);
        let tracks = u32::from(tracks);
        if length == 1 {
            return if index == (tracks - 1) / 2 { 2 } else { 0 };
        }
        if index < length && tracks % 2 == 1 {
            return if index == (length - 1) / 2 { 2 } else { 0 };
        }
        if length > 4 && (index.is_multiple_of(length) || index % length == length - 1) {
            return 0;
        }
        if (index / length) % 2 == tracks % 2 {
            4
        } else {
            6
        }
    }
    fn rail_bridge(
        &self,
        out: Cost<'_>,
        tile: u32,
        spec: *const RailSpec,
        kind: u8,
        gfx: StationGfx,
    ) {
        if !crate::map_func::World::new(self.shared).bridge_above(tile) {
            return out.zero();
        }
        let south = (self.s.southern_bridge_end)(tile);
        let info = if spec.is_null() {
            (self.s.default_bridge_info)(kind)
        } else {
            unsafe { (self.s.rail_bridge_info)(spec) }
        };
        self.is_station_bridge_above_ok(StationBridgeCheck {
            out,
            tile: TileIndex(tile),
            bridgeable_info: info,
            station_type: StationType(kind),
            layout: gfx,
            bridge_height: self.bridge_height(TileIndex(south)),
            disallowed_msg: StringID(self.s.strings.demolish_bridge),
        });
    }
    fn build_rail(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        flags: u16,
        tile: u32,
        rt: u8,
        axis: u8,
        tracks: u8,
        length: u8,
        class: u16,
        index: u16,
        mut target: u16,
        adjacent: bool,
    ) {
        crate::witness::hit(crate::witness::STATION_CMD_BUILD_RAIL);
        self.authority(cost, tile, flags);
        if !cost.ok() {
            return;
        }
        if !crate::rail::val_param_rail_type(self.shared, rt) || axis >= 2 {
            return cost.error(INVALID_STRING_ID);
        }
        if u32::from(class) >= crate::newgrf_class::station::count()
            || crate::newgrf_station::is_waypoint_class(crate::newgrf_class::station_label(class))
            || u32::from(index) >= crate::newgrf_class::station::spec_count(class)
            || length == 0
            || tracks == 0
        {
            return cost.error(INVALID_STRING_ID);
        }
        let (w, h) = if axis == 0 {
            (length, tracks)
        } else {
            (tracks, length)
        };
        if !self.buildable_area(tile, u16::from(w), u16::from(h)) {
            return cost.error(INVALID_STRING_ID);
        }
        let reuse = target != StationID::NEW.0;
        if !reuse {
            target = StationID::INVALID.0;
        }
        if target != StationID::INVALID.0
            && (!(self.s.distant_join)() || self.full_station(StationID(target)).is_null())
        {
            return cost.error(INVALID_STRING_ID);
        }
        let spread = (self.s.spread)();
        if h > spread || w > spread {
            return cost.error(INVALID_STRING_ID);
        }
        let mut area = TileArea {
            tile,
            w: u16::from(w),
            h: u16::from(h),
        };
        let mut est = StationID::INVALID.0;
        // Clearing may affect reservations on existing station tiles.
        let mut affected = Vec::new();
        let spec = crate::newgrf_class::station::spec(class, u32::from(index));
        let mut allowed = -1;
        let mut length_ready = true;
        let mut tracknum = 0_u8;
        // Temporary native slots are reused; ret becomes each tile's result.
        cost.set(0);
        for t in area.tiles(self.map()) {
            // flat_rail needs two command slots; preserve the accumulated amount
            // through the independent native result slot used by this helper.
            let saved = cost.money();
            self.flat_rail(RailStationLandCheck {
                cost,
                ret,
                tile: t,
                north: tile,
                allowed: &mut allowed,
                flags,
                axis,
                est: &mut est,
                rt,
                affected: &mut affected,
                spec,
                length,
                tracks,
            });
            if !cost.ok() {
                return;
            }
            let tile_cost = cost.money();
            cost.set(saved);
            if tracknum == tracks {
                length_ready = true;
                tracknum = 0;
            } else {
                tracknum = tracknum.wrapping_add(1);
            }
            if !self.rail_station(t) || self.station_axis(t) != axis {
                cost.add_money(tile_cost);
                cost.add_money(crate::company::price(usize::from(
                    Price::PR_BUILD_STATION_RAIL.0,
                )));
                cost.add_money((self.rail().rail_build_cost)(rt));
                if length_ready {
                    cost.add_money(crate::company::price(usize::from(
                        Price::PR_BUILD_STATION_RAIL_LENGTH.0,
                    )));
                    length_ready = false;
                }
            }
        }
        let st = self.find_joining_base_station(JoiningStation {
            out: ret,
            existing_station: StationID(est),
            station_to_join: StationID(target),
            adjacent,
            ta: area,
            waypoint: false,
            road: false,
            error_message: StringID(self.s.strings.remove_station),
        });
        if !ret.ok() {
            return cost.take(ret);
        }
        let st = self.build_station_part(
            ret,
            st,
            DoCommandFlags(flags),
            reuse,
            area,
            StationNaming::STATIONNAMING_RAIL,
        );
        if !ret.ok() {
            return cost.take(ret);
        }
        if !st.is_null() && self.s.rail_area(st).tile != u32::MAX {
            area = self.expand_rail(ret, st, area);
            if !ret.ok() {
                return cost.take(ret);
            }
        }
        let spec = crate::newgrf_class::station::spec(class, u32::from(index));
        let delta = if axis == 0 { 1 } else { self.map().size_x() };
        let track_delta = if axis == 0 { self.map().size_x() } else { 1 };
        let layout = (self.s.rail_layout)(
            spec,
            crate::newgrf_station::get_station_layout_key(tracks, length),
        );
        for i in 0..tracks {
            for j in 0..length {
                let t = tile
                    .wrapping_add(u32::from(i).wrapping_mul(track_delta))
                    .wrapping_add(u32::from(j).wrapping_mul(delta));
                // There is no need to check the layout without a bridge above.
                if !crate::map_func::World::new(self.shared).bridge_above(t) {
                    continue;
                }
                let mut gfx = Self::layout_gfx(
                    layout,
                    u32::from(i) * u32::from(length) + u32::from(j),
                    tracks,
                    length,
                )
                .wrapping_add(axis);
                if !spec.is_null() {
                    let info = crate::newgrf_station::platform_info(gfx, length, tracks, j, i);
                    // The station is not completely built yet, so no station
                    // instance exists for this layout callback.
                    let cb = crate::newgrf_station::callback(
                        CallbackID::CBID_STATION_BUILD_TILE_LAYOUT.0,
                        info,
                        0,
                        spec,
                        std::ptr::null_mut(),
                        u32::MAX,
                    );
                    if cb != CALLBACK_FAILED && cb <= 255 {
                        gfx = (cb as u8 & !1).wrapping_add(axis);
                    }
                }
                self.rail_bridge(ret, t, spec, StationType::Rail.0, StationGfx(gfx));
                if !ret.ok() {
                    return cost.take(ret);
                }
            }
        }
        // Check whether a custom station spec can be allocated.
        let specindex = crate::station_association::allocate(self.s.rail_association, spec, st);
        if !specindex.valid {
            return cost.error(self.s.strings.too_many_specs);
        }
        if !spec.is_null() {
            let info = (self.s.rail_spec_info)(spec);
            if info.platforms & (1 << (tracks - 1).min(7)) != 0 {
                return cost.error(self.s.strings.disallowed_tracks);
            }
            if info.lengths & (1 << (length - 1).min(7)) != 0 {
                return cost.error(self.s.strings.disallowed_length);
            }
            if info.availability_callback {
                let cb = crate::newgrf_station::callback(
                    CallbackID::CBID_STATION_AVAILABILITY.0,
                    0,
                    0,
                    spec,
                    std::ptr::null_mut(),
                    u32::MAX,
                );
                if cb != CALLBACK_FAILED
                    && !(self.s.rail_boolean)(spec, CallbackID::CBID_STATION_AVAILABILITY.0, cb)
                {
                    return cost.error(INVALID_STRING_ID);
                }
            }
        }
        if flags & DoCommandFlag::EXECUTE == 0 {
            return;
        }
        (self.s.set_train_area)(st, area);
        crate::station_lifecycle::add_facility_raw(st, StationFacility::Train, area.tile, unsafe {
            &*self.s.lifecycle
        });
        self.before_add_rect(
            ret,
            st,
            TileArea {
                tile,
                w: u16::from(w),
                h: u16::from(h),
            },
            StationRectMode::ADD_TRY,
        );
        crate::station_association::assign(self.s.rail_association, spec, st, specindex.value);
        if !spec.is_null() {
            // Include this spec in the cached animation trigger mask.
            (self.s.rail_triggers)(st, spec);
        }
        let owner = (self.world().station_owner)(st);
        let id = (self.links().graphs.station_id)(st);
        for i in 0..tracks {
            let track_tile = tile.wrapping_add(u32::from(i).wrapping_mul(track_delta));
            for j in 0..length {
                let t = track_tile.wrapping_add(u32::from(j).wrapping_mul(delta));
                if self.rail_station(t) && (self.s.reserved)(t) {
                    let train = crate::pbs::Pbs::acquire(self.shared)
                        .train_for(t, self.station_axis(t))
                        .shell()
                        .cast::<crate::services::Train>();
                    if !train.is_null() {
                        affected.push(train);
                        self.free_reservation(train);
                    }
                }
                if self.rail_station(t) {
                    if !crate::map_func::World::new(self.shared).station_blocked(t) {
                        (self.rail().rail_infrastructure)(
                            owner,
                            crate::services::RailType(
                                crate::map_func::World::new(self.shared).rail_type(t),
                            )
                            .0,
                            u32::MAX,
                        );
                    }
                    (self.s.station_infrastructure)(owner, u32::MAX);
                }
                crate::animated_tile::openttd_rust_animation_delete(t, false);
                let old = if (self.shared.tile_type)(t) == MP_STATION
                    && crate::map_func::World::new(self.shared).station_has_rail(t)
                {
                    (self.s.custom_rail_index)(t) as u8
                } else {
                    0
                };
                let gfx = Self::layout_gfx(
                    layout,
                    u32::from(i) * u32::from(length) + u32::from(j),
                    tracks,
                    length,
                );
                (self.s.make_rail_station)(t, owner, id, axis, gfx, rt);
                crate::station_association::deallocate(self.s.rail_association, st, old);
                if spec.is_null() {
                    (self.s.close_station_inspect)(t);
                }
                (self.s.set_custom_rail_index)(t, specindex.value);
                (self.s.set_random_bits)(t, ((self.shared.random)(self.shared.context) & 15) as u8);
                (self.shared.tile_animation_set_frame)(t, 0);
                if !spec.is_null() {
                    let info = crate::newgrf_station::platform_info(
                        (self.shared.station_gfx)(t),
                        length,
                        tracks,
                        j,
                        i,
                    );
                    let cb = crate::newgrf_station::callback(
                        CallbackID::CBID_STATION_BUILD_TILE_LAYOUT.0,
                        info,
                        0,
                        spec,
                        std::ptr::null_mut(),
                        t,
                    );
                    if cb != CALLBACK_FAILED {
                        if cb <= 255 {
                            (self.s.set_gfx)(t, (cb as u8 & !1).wrapping_add(axis));
                        } else {
                            (self.s.rail_callback_error)(spec, 0x24, cb);
                        }
                    }
                    crate::animated_tile::openttd_rust_trigger_station(
                        st,
                        t,
                        crate::station_type::SAT_BUILT,
                        crate::cargo_type::INVALID_CARGO,
                    );
                }
                self.set_rail_flags(t, spec);
                if !crate::map_func::World::new(self.shared).station_blocked(t) {
                    (self.rail().rail_infrastructure)(owner, rt, 1);
                }
                (self.s.station_infrastructure)(owner, 1);
            }
            let signals = crate::signal::Context {
                shared: self.s.shared,
                rail: self.rail().rail,
                native: self.rail().signal,
                log_x: u32::from(self.map().log_x),
            };
            signals.add_track(track_tile, axis, (self.shared.current_company)());
            (self.rail().layout_changed)(track_tile, axis);
        }
        for train in affected {
            self.restore_reservation(train);
        }
        let update = if axis == 0 {
            TileArea {
                tile,
                w: 1,
                h: u16::from(tracks),
            }
        } else {
            TileArea {
                tile,
                w: u16::from(tracks),
                h: 1,
            }
        };
        for t in update.tiles(self.map()) {
            if crate::map_func::World::new(self.shared).station_blocked(t) {
                continue;
            }
            let dir = 2 - axis;
            let offset = if dir == 2 { 1 } else { self.map().size_x() };
            let mut begin = t;
            let mut end = t;
            let mut next = begin.wrapping_sub(offset);
            while (self.s.compatible_tiles)(next, begin) {
                begin = next;
                next = next.wrapping_sub(offset);
            }
            next = end.wrapping_add(offset);
            while (self.s.compatible_tiles)(next, end) {
                end = next;
                next = next.wrapping_add(offset);
            }
            let mut reservation = false;
            let mut cur = begin;
            while !reservation && cur <= end {
                reservation = (self.s.reserved)(cur);
                cur = cur.wrapping_add(offset);
            }
            if reservation {
                crate::pbs::Pbs::acquire(self.shared).platform(begin, dir, true);
            }
        }
        (self.s.mark_tiles)(st, false);
        self.after_station_tile_set_change(st, true, StationType::Rail);
    }
}
/// # Safety
/// Native tables, cost slots and serialized world remain live for this entry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_build_rail(
    s: *const CommandServicesTable,
    cost: *mut CommandCost,
    ret: *mut CommandCost,
    flags: u16,
    tile: u32,
    rt: u8,
    axis: u8,
    tracks: u8,
    length: u8,
    class: u16,
    index: u16,
    target: u16,
    adjacent: bool,
) {
    let c = unsafe { Ctx::new(s) };
    c.build_rail(
        c.c(cost),
        c.c(ret),
        flags,
        tile,
        rt,
        axis,
        tracks,
        length,
        class,
        index,
        target,
        adjacent,
    );
}
/// # Safety
/// Native tables, tile and optional immutable station spec are valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_set_rail_flags(
    s: *const CommandServicesTable,
    tile: u32,
    spec: *const RailSpec,
) {
    unsafe { Ctx::new(s) }.set_rail_flags(tile, spec);
}

impl Ctx<'_> {
    fn signals(&self) -> crate::signal::Context {
        crate::signal::Context {
            shared: self.s.shared,
            rail: self.rail().rail,
            native: self.rail().signal,
            log_x: u32::from(self.map().log_x),
        }
    }
    fn nested(&self, out: Cost<'_>, tile: u32, flags: u16, body: impl FnMut(u16)) {
        crate::command::nested(self.command, self.shared, out, tile, flags, body);
    }

    fn remove_rail_area(&self, context: RailStationRemoval<'_>) -> Vec<*mut BaseStation> {
        let RailStationRemoval {
            cost,
            ret,
            error,
            area,
            flags,
            unit,
            keep,
            waypoint,
        } = context;
        crate::witness::hit(crate::witness::STATION_CMD_REMOVE_RAIL_AREA);
        let mut quantity = 0_i32;
        let mut affected = Vec::new();
        cost.set(0);
        error.zero();
        for tile in area.tiles(self.map()) {
            if (self.shared.tile_type)(tile) != MP_STATION
                || !crate::map_func::World::new(self.shared).station_has_rail(tile)
            {
                continue;
            }
            unsafe {
                self.command.no_vehicle(ret.0, tile);
            }
            error.add(ret);
            if !error.ok() {
                continue;
            }
            let id = (self.shared.station_index)(tile);
            let st = if waypoint {
                self.waypoint(StationID(id))
            } else {
                self.full_station(StationID(id))
            };
            if st.is_null() {
                continue;
            }
            if (self.shared.current_company)() != OWNER_WATER {
                unsafe {
                    self.command
                        .ownership(ret.0, (self.world().station_owner)(st), 0);
                }
                error.add(ret);
                if !error.ok() {
                    continue;
                }
            }
            quantity += 1;
            if keep || crate::map_func::World::new(self.shared).station_blocked(tile) {
                cost.add_money(
                    crate::company::price(usize::from(Price::PR_CLEAR_RAIL.0)).saturating_neg(),
                );
            }
            if flags & DoCommandFlag::EXECUTE == 0 {
                continue;
            }
            let index = (self.s.custom_rail_index)(tile);
            let track = self.station_axis(tile);
            let owner = (self.shared.tile_owner)(tile);
            let rt =
                crate::services::RailType(crate::map_func::World::new(self.shared).rail_type(tile))
                    .0;
            let mut train = std::ptr::null_mut();
            if (self.s.reserved)(tile) {
                train = crate::pbs::Pbs::acquire(self.shared)
                    .train_for(tile, track)
                    .shell()
                    .cast::<crate::services::Train>();
                if !train.is_null() {
                    self.free_reservation(train);
                }
            }
            let build_rail =
                keep && !crate::map_func::World::new(self.shared).station_blocked(tile);
            if !build_rail && !crate::map_func::World::new(self.shared).station_blocked(tile) {
                (self.rail().rail_infrastructure)(owner, rt, u32::MAX);
            }
            crate::ground::clear_square(tile, self.shared);
            (self.s.close_station_inspect)(tile);
            if build_rail {
                (self.rail_map().make_rail_normal)(tile, owner, 1 << track, rt);
            }
            (self.s.station_infrastructure)(owner, u32::MAX);
            (self.command.infrastructure_dirty)(owner);
            self.after_remove_tile(st, TileIndex(tile));
            self.signals().add_track(tile, track, owner);
            (self.rail().layout_changed)(tile, track);
            crate::station_association::deallocate(self.s.rail_association, st, index as u8);
            if !affected.contains(&st) {
                affected.push(st);
            }
            if !train.is_null() {
                self.restore_reservation(train);
            }
        }
        if quantity == 0 {
            if error.ok() {
                cost.error(self.s.strings.no_station);
            } else {
                cost.take(error);
            }
            return affected;
        }
        if !error.ok() {
            crate::witness::hit(crate::witness::STATION_CMD_PARTIAL_RAIL);
        }
        for &st in &affected {
            let area = self.make_station_area_smaller(st, self.s.rail_area(st), StationType::Rail);
            (self.s.set_train_area)(st, area);
            self.update_station_sign_coord(st);
            if area.tile == u32::MAX {
                (self.s.reset_facilities)(st, FACILITY_TRAIN);
                (self.s.orders_dirty)();
                (self.s.facility_widget)(st, 0);
                (self.s.catchment_dirty)();
                (self.s.virtual_coord)(st);
                self.delete_station_if_empty(st);
            }
        }
        cost.add_money(unit.saturating_mul(i64::from(quantity)));
        affected
    }
    fn remove_rail_command(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        error: Cost<'_>,
        flags: u16,
        start: u32,
        mut end: u32,
        keep: bool,
        waypoint: bool,
    ) {
        if end == 0 {
            end = start;
        }
        let size = ((self.shared.map_max_y)() + 1) << self.map().log_x;
        if start >= size || end >= size {
            return cost.error(INVALID_STRING_ID);
        }
        let affected = self.remove_rail_area(RailStationRemoval {
            cost,
            ret,
            error,
            area: TileArea::between(self.map(), start, end),
            flags,
            unit: crate::company::price(usize::from(
                (if waypoint {
                    Price::PR_CLEAR_WAYPOINT_RAIL
                } else {
                    Price::PR_CLEAR_STATION_RAIL
                })
                .0,
            )),
            keep,
            waypoint,
        });
        if !cost.ok() || waypoint {
            return;
        }
        for st in affected {
            if self.s.rail_area(st).tile == u32::MAX {
                (self.s.facility_widget)(st, 0);
            }
            (self.s.mark_tiles)(st, false);
            (self.s.catchment_dirty)();
            self.recompute_catchment(st, false);
        }
    }
    fn remove_rail_all_inner(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        error: Cost<'_>,
        flags: u16,
        st: *mut BaseStation,
        waypoint: bool,
    ) {
        if (self.shared.current_company)() != OWNER_WATER {
            unsafe {
                self.command
                    .ownership(cost.0, (self.world().station_owner)(st), 0);
            }
            if !cost.ok() {
                return;
            }
        }
        let area = self.s.rail_area(st);
        cost.set(0);
        let id = (self.links().graphs.station_id)(st);
        let kind = if waypoint {
            StationType::RailWaypoint.0
        } else {
            StationType::Rail.0
        };
        for tile in area.tiles(self.map()) {
            if (self.shared.tile_type)(tile) != MP_STATION
                || (self.shared.station_type)(tile).0 != kind
                || (self.shared.station_index)(tile) != id
            {
                continue;
            }
            let total = cost.money();
            self.remove_rail_area(RailStationRemoval {
                cost,
                ret,
                error,
                area: TileArea { tile, w: 1, h: 1 },
                flags,
                unit: crate::company::price(usize::from(
                    (if waypoint {
                        Price::PR_CLEAR_WAYPOINT_RAIL
                    } else {
                        Price::PR_CLEAR_STATION_RAIL
                    })
                    .0,
                )),
                keep: false,
                waypoint,
            });
            if !cost.ok() {
                return;
            }
            let amount = cost.money();
            cost.set(total);
            cost.add_money(amount);
        }
    }
    fn remove_rail_all(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        error: Cost<'_>,
        flags: u16,
        tile: u32,
        waypoint: bool,
    ) {
        if (self.shared.current_company)() == OWNER_WATER {
            return self.nested(cost, tile, DoCommandFlag::EXECUTE, |flags| {
                self.remove_rail_command(cost, ret, error, flags, tile, 0, false, waypoint);
            });
        }
        let id = (self.shared.station_index)(tile);
        let st = if waypoint {
            self.waypoint(StationID(id))
        } else {
            self.full_station(StationID(id))
        };
        self.remove_rail_all_inner(cost, ret, error, flags, st, waypoint);
        if !waypoint && flags & DoCommandFlag::EXECUTE != 0 {
            self.recompute_catchment(st, false);
        }
    }
}
/// # Safety
/// The three distinct native cost slots and all native tables remain live for this entry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_remove_rail_area(
    s: *const CommandServicesTable,
    cost: *mut CommandCost,
    ret: *mut CommandCost,
    error: *mut CommandCost,
    flags: u16,
    start: u32,
    end: u32,
    keep: bool,
    waypoint: bool,
) {
    let c = unsafe { Ctx::new(s) };
    c.remove_rail_command(
        c.c(cost),
        c.c(ret),
        c.c(error),
        flags,
        start,
        end,
        keep,
        waypoint,
    );
}

impl Ctx<'_> {
    fn vehicle(&self) -> &crate::vehicle::VehicleServices {
        unsafe { &*self.s.vehicle }
    }
    fn stops(&self) -> &RoadStopServices {
        // SAFETY: The shared table returns a static immutable table.
        unsafe { &*(self.shared.road_stops)() }
    }
    fn road_type(&self, tile: u32, tram: bool) -> u8 {
        if tram {
            (self.shared.tram_type)(tile).0
        } else {
            (self.shared.road_type)(tile)
        }
    }
    fn road_owner(&self, tile: u32, tram: bool) -> u8 {
        if tram {
            crate::map_func::World::new(self.shared).road_owner(tile, true)
        } else {
            crate::map_func::World::new(self.shared).road_owner(tile, false)
        }
    }
    fn may_have_road(&self, tile: u32) -> bool {
        match (self.shared.tile_type)(tile) {
            MP_ROAD => true,
            MP_STATION => crate::map_func::World::new(self.shared).any_road_stop_tile(tile),
            MP_TUNNELBRIDGE => {
                crate::map_func::World::new(self.shared).tunnel_bridge_transport(tile) == 1
            }
            _ => false,
        }
    }
    fn road_infrastructure(&self, rt: u8, owner: u8, delta: i32) {
        if rt == 63 || !(self.shared.company_valid)(owner) {
            return;
        }
        (self.rail().road_infrastructure)(owner, rt, delta as u32);
        (self.command.infrastructure_dirty)(owner);
    }
    fn road_bridge(
        &self,
        out: Cost<'_>,
        tile: u32,
        spec: *const RoadSpec,
        kind: u8,
        gfx: StationGfx,
    ) {
        if !crate::map_func::World::new(self.shared).bridge_above(tile) {
            return out.zero();
        }
        let south = (self.s.southern_bridge_end)(tile);
        let info = if spec.is_null() {
            (self.s.default_bridge_info)(kind)
        } else {
            unsafe { (self.s.road_bridge_info)(spec) }
        };
        self.is_station_bridge_above_ok(StationBridgeCheck {
            out,
            tile: TileIndex(tile),
            bridgeable_info: info,
            station_type: StationType(kind),
            layout: gfx,
            bridge_height: self.bridge_height(TileIndex(south)),
            disallowed_msg: StringID(self.s.strings.demolish_bridge),
        });
    }
    fn flat_road(&self, context: RoadStopLandCheck<'_>) {
        let RoadStopLandCheck {
            cost,
            ret,
            tile,
            allowed,
            spec,
            flags,
            invalid,
            drive,
            kind,
            axis,
            est,
            rt,
        } = context;
        self.check_buildable(
            ret,
            TileIndex(tile),
            DiagDirections(invalid),
            allowed,
            !drive,
            false,
        );
        if !ret.ok() {
            return cost.take(ret);
        }
        cost.set(0);
        cost.add_money(ret.money());
        self.road_bridge(
            ret,
            tile,
            spec,
            kind,
            StationGfx(if drive {
                GFX_TRUCK_BUS_DRIVETHROUGH_OFFSET + axis
            } else {
                invalid.trailing_zeros() as u8
            }),
        );
        if !ret.ok() {
            return cost.take(ret);
        }
        if (self.shared.tile_type)(tile) == MP_STATION {
            if !crate::map_func::World::new(self.shared).any_road_stop_tile(tile)
                || kind != (self.shared.station_type)(tile).0
                || drive != crate::map_func::World::new(self.shared).drive_through_stop_tile(tile)
            {
                return self.auto_clear(cost, tile);
            }
            if drive
                && crate::map_func::World::new(self.shared).drive_through_stop_tile(tile)
                && crate::map_func::World::new(self.shared).drive_through_axis(tile) != axis
            {
                return cost.error(self.s.strings.drive_direction);
            }
            let id = (self.shared.station_index)(tile);
            if *est == StationID::INVALID.0 {
                *est = id;
            } else if *est != id {
                cost.error(self.s.strings.adjoins_multiple);
            }
        } else {
            let normal = crate::map_func::World::new(self.shared).is_normal_road_tile(tile);
            let build_over = drive && normal;
            let rb = if normal {
                crate::map_func::World::new(self.shared).road_bits(tile, false)
                    | crate::map_func::World::new(self.shared).road_bits(tile, true)
            } else {
                0
            };
            if build_over && rb & (if axis == 0 { 5 } else { 10 }) != 0 {
                return cost.error(match rb.count_ones() {
                    1 => self.s.strings.drive_direction,
                    2 if rb == 5 || rb == 10 => self.s.strings.drive_direction,
                    2 => self.s.strings.drive_corner,
                    _ => self.s.strings.drive_junction,
                });
            }
            if build_over {
                let road = self.road_type(tile, false);
                if road != 63 {
                    let owner = self.road_owner(tile, false);
                    if owner == OWNER_TOWN {
                        if !(self.s.stop_on_town_road)() {
                            return cost.error(self.s.strings.drive_town);
                        }
                    } else if !(self.s.stop_on_competitor_road)() && owner != OWNER_NONE {
                        unsafe {
                            self.command.ownership(ret.0, owner, 0);
                        }
                        if !ret.ok() {
                            return cost.take(ret);
                        }
                    }
                    let pieces = crate::map_func::World::new(self.shared)
                        .road_bits(tile, false)
                        .count_ones();
                    if rt != 63
                        && crate::road::road_type_is_road(rt)
                        && !(self.shared.road_powered)(rt, road)
                    {
                        return cost.error(self.s.strings.unsuitable_road);
                    }
                    if crate::map_func::World::new(self.shared).disallowed_road_directions(tile)
                        != 0
                        && owner != 0x0f
                        && owner != OWNER_NONE
                    {
                        unsafe {
                            self.command.ownership(ret.0, owner, 0);
                        }
                        if !ret.ok() {
                            return cost.take(ret);
                        }
                    }
                    cost.add_money(
                        crate::road_cost::build_cost(self.shared, road)
                            .saturating_mul(i64::from(2_u32.wrapping_sub(pieces))),
                    );
                } else if rt != 63 && crate::road::road_type_is_road(rt) {
                    cost.add_money(crate::road_cost::build_cost(self.shared, rt).saturating_mul(2));
                }
                let tram = self.road_type(tile, true);
                if tram != 63 {
                    let owner = self.road_owner(tile, true);
                    let bits = crate::map_func::World::new(self.shared).road_bits(tile, true);
                    if (self.shared.company_valid)(owner)
                        && (!(self.s.stop_on_competitor_road)() || bits.is_power_of_two())
                    {
                        unsafe {
                            self.command.ownership(ret.0, owner, 0);
                        }
                        if !ret.ok() {
                            return cost.take(ret);
                        }
                    }
                    let pieces = crate::map_func::World::new(self.shared)
                        .road_bits(tile, true)
                        .count_ones();
                    if rt != 63
                        && crate::road::get_road_tram_type(rt).0
                        && !(self.shared.road_powered)(rt, tram)
                    {
                        return cost.error(self.s.strings.unsuitable_road);
                    }
                    cost.add_money(
                        crate::road_cost::build_cost(self.shared, tram)
                            .saturating_mul(i64::from(2_u32.wrapping_sub(pieces))),
                    );
                } else if rt != 63 && crate::road::get_road_tram_type(rt).0 {
                    cost.add_money(crate::road_cost::build_cost(self.shared, rt).saturating_mul(2));
                }
            } else if rt == 63 {
                cost.error(self.s.strings.no_road);
            } else {
                crate::landscape_cmd::clear_command(self.command, ret.0, tile, flags);
                if !ret.ok() {
                    return cost.take(ret);
                }
                cost.add_money(ret.money());
                cost.add_money(crate::road_cost::build_cost(self.shared, rt).saturating_mul(2));
            }
        }
    }
    fn road_cost(&self, context: RoadStopCost<'_>) {
        let RoadStopCost {
            cost,
            ret,
            area,
            flags,
            drive,
            kind,
            spec,
            axis,
            dir,
            est,
            rt,
            unit,
        } = context;
        let invalid = if drive {
            if axis == 0 { 5 } else { 10 }
        } else {
            1 << dir
        };
        let mut allowed = -1;
        cost.set(0);
        for tile in area.tiles(self.map()) {
            let total = cost.money();
            self.flat_road(RoadStopLandCheck {
                cost,
                ret,
                tile,
                allowed: &mut allowed,
                spec,
                flags,
                invalid,
                drive,
                kind,
                axis,
                est,
                rt,
            });
            if !cost.ok() {
                return;
            }
            let tile_cost = cost.money();
            cost.set(total);
            if (self.shared.tile_type)(tile) != MP_STATION
                || !crate::map_func::World::new(self.shared).any_road_stop_tile(tile)
            {
                cost.add_money(tile_cost);
                cost.add_money(unit);
            }
        }
    }
    fn build_road(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        flags: u16,
        tile: u32,
        width: u8,
        length: u8,
        stop_type: u8,
        drive: bool,
        dir: u8,
        rt: u8,
        class: u16,
        index: u16,
        mut target: u16,
        adjacent: bool,
    ) {
        crate::witness::hit(crate::witness::STATION_CMD_BUILD_ROAD);
        if !crate::road::val_param_road_type(self.shared, rt) || dir >= 4 || stop_type >= 2 {
            return cost.error(INVALID_STRING_ID);
        }
        let reuse = target != StationID::NEW.0;
        if !reuse {
            target = StationID::INVALID.0;
        }
        let distant = target != StationID::INVALID.0;
        if u32::from(class) >= crate::newgrf_class::roadstop::count()
            || crate::newgrf_station::is_waypoint_class(crate::newgrf_class::roadstop_label(class))
            || u32::from(index) >= crate::newgrf_class::roadstop::spec_count(class)
        {
            return cost.error(INVALID_STRING_ID);
        }
        let spec = crate::newgrf_class::roadstop::spec(class, u32::from(index));
        if !spec.is_null() {
            let info = (self.s.road_spec_info)(spec);
            if (RoadStopType(stop_type) == RoadStopType::Truck
                && RoadStopAvailabilityType(info.stop_type)
                    != RoadStopAvailabilityType::ROADSTOPTYPE_FREIGHT
                && RoadStopAvailabilityType(info.stop_type)
                    != RoadStopAvailabilityType::ROADSTOPTYPE_ALL)
                || (RoadStopType(stop_type) == RoadStopType::Bus
                    && RoadStopAvailabilityType(info.stop_type)
                        != RoadStopAvailabilityType::ROADSTOPTYPE_PASSENGER
                    && RoadStopAvailabilityType(info.stop_type)
                        != RoadStopAvailabilityType::ROADSTOPTYPE_ALL)
                || (!drive && info.drive_only)
            {
                return cost.error(INVALID_STRING_ID);
            }
        }
        let spread = (self.s.spread)();
        if width > spread || length > spread {
            return cost.error(self.s.strings.spread);
        }
        if width == 0
            || length == 0
            || !self.buildable_area(tile, u16::from(width), u16::from(length))
        {
            return cost.error(INVALID_STRING_ID);
        }
        let area = TileArea {
            tile,
            w: u16::from(width),
            h: u16::from(length),
        };
        if distant && (!(self.s.distant_join)() || self.full_station(StationID(target)).is_null()) {
            return cost.error(INVALID_STRING_ID);
        }
        // Trams only have drive through stops.
        if !drive && !crate::road::road_type_is_road(rt) {
            return cost.error(INVALID_STRING_ID);
        }
        let axis = dir & 1;
        self.authority(cost, tile, flags);
        if !cost.ok() {
            return;
        }
        let truck = RoadStopType(stop_type) != RoadStopType::Bus;
        let kind = if truck {
            StationType::Truck.0
        } else {
            StationType::Bus.0
        };
        let category = if truck {
            Price::PR_BUILD_STATION_TRUCK
        } else {
            Price::PR_BUILD_STATION_BUS
        };
        let unit = if spec.is_null() {
            crate::company::price(usize::from(category.0))
        } else {
            {
                let factor = (self.s.road_spec_build_cost)(spec, category.0);
                crate::economy_func::get_price(
                    u32::from(category.0),
                    u32::from(factor.multiplier),
                    factor.shift,
                    -4,
                )
            }
        };
        let mut est = StationID::INVALID.0;
        self.road_cost(RoadStopCost {
            cost,
            ret,
            area,
            flags,
            drive,
            kind,
            spec,
            axis,
            dir,
            est: &mut est,
            rt,
            unit,
        });
        if !cost.ok() {
            return;
        }
        let st = self.find_joining_base_station(JoiningStation {
            out: ret,
            existing_station: StationID(est),
            station_to_join: StationID(target),
            adjacent,
            ta: area,
            waypoint: false,
            road: false,
            error_message: StringID(self.s.strings.remove_road_stop),
        });
        if !ret.ok() {
            return cost.take(ret);
        }
        if !(self.s.stops_available)(usize::from(area.w) * usize::from(area.h)) {
            return cost.error(if truck {
                self.s.strings.too_many_truck
            } else {
                self.s.strings.too_many_bus
            });
        }
        let st = self.build_station_part(
            ret,
            st,
            DoCommandFlags(flags),
            reuse,
            area,
            StationNaming::STATIONNAMING_ROAD,
        );
        if !ret.ok() {
            return cost.take(ret);
        }
        let index = unsafe {
            crate::newgrf_roadstop::openttd_rust_roadstop_allocate(
                spec,
                st,
                self.s.roadstop_resolver,
            )
        };
        let specindex = SpecIndex {
            value: index as u8,
            valid: index >= 0,
        };
        if !specindex.valid {
            return cost.error(self.s.strings.too_many_specs);
        }
        if !spec.is_null() && (self.s.road_spec_info)(spec).availability_callback {
            let cb = unsafe {
                crate::newgrf_roadstop::openttd_rust_roadstop_callback(
                    crate::newgrf_callbacks::CBID_STATION_AVAILABILITY,
                    0,
                    0,
                    spec,
                    std::ptr::null_mut(),
                    crate::tile_type::INVALID_TILE,
                    rt,
                    kind,
                    0,
                    std::ptr::null_mut(),
                    0,
                    self.s.roadstop_resolver,
                )
            };
            if cb != CALLBACK_FAILED && !(self.s.road_boolean)(spec, cb) {
                return cost.error(INVALID_STRING_ID);
            }
        }
        if flags & DoCommandFlag::EXECUTE == 0 {
            return;
        }
        unsafe {
            crate::newgrf_roadstop::openttd_rust_roadstop_assign(
                spec,
                st,
                specindex.value,
                self.s.roadstop_resolver,
            );
        }
        for tile in area.tiles(self.map()) {
            let mut road = if self.may_have_road(tile) {
                self.road_type(tile, false)
            } else {
                63
            };
            let mut tram = if self.may_have_road(tile) {
                self.road_type(tile, true)
            } else {
                63
            };
            let road_owner = if road != 63 {
                self.road_owner(tile, false)
            } else {
                (self.shared.current_company)()
            };
            let tram_owner = if tram != 63 {
                self.road_owner(tile, true)
            } else {
                (self.shared.current_company)()
            };
            if (self.shared.tile_type)(tile) == MP_STATION
                && matches!((self.shared.station_type)(tile).0, 2 | 3)
            {
                self.remove_road_tile(ret, ret, tile, flags, i32::from(specindex.value), false);
            }
            if !spec.is_null() {
                (self.s.road_triggers)(st, spec);
            }
            let stop = (self.s.new_stop)(tile);
            let head = (self.stops().head)(st, truck);
            if head.is_null() {
                (self.s.set_stop_head)(st, truck, stop);
            } else {
                let mut tail = head;
                while !(self.stops().next)(tail).is_null() {
                    tail = (self.stops().next)(tail);
                }
                (self.s.set_stop_next)(tail, stop);
            }
            self.area_add((self.s.stop_area)(st, truck), TileIndex(tile));
            crate::station_lifecycle::add_facility_raw(
                st,
                if truck {
                    StationFacility::TruckStop
                } else {
                    StationFacility::BusStop
                },
                tile,
                unsafe { &*self.s.lifecycle },
            );
            self.before_add_tile(ret, st, TileIndex(tile), StationRectMode::ADD_TRY);
            let owner = (self.world().station_owner)(st);
            let id = (self.links().graphs.station_id)(st);
            if drive {
                if crate::map_func::World::new(self.shared).is_normal_road_tile(tile) {
                    self.road_infrastructure(
                        road,
                        road_owner,
                        -(crate::map_func::World::new(self.shared)
                            .road_bits(tile, false)
                            .count_ones() as i32),
                    );
                    self.road_infrastructure(
                        tram,
                        tram_owner,
                        -(crate::map_func::World::new(self.shared)
                            .road_bits(tile, true)
                            .count_ones() as i32),
                    );
                }
                if road == 63 && crate::road::road_type_is_road(rt) {
                    road = rt;
                }
                if tram == 63 && crate::road::get_road_tram_type(rt).0 {
                    tram = rt;
                }
                (self.s.make_drive_stop)(
                    tile, owner, road_owner, tram_owner, id, kind, road, tram, axis,
                );
                make_drive_through(self.shared, TileIndex(tile), (self.stops().state)(stop));
            } else {
                if road == 63 && crate::road::road_type_is_road(rt) {
                    road = rt;
                }
                if tram == 63 && crate::road::get_road_tram_type(rt).0 {
                    tram = rt;
                }
                (self.s.make_bay_stop)(tile, owner, id, stop_type, road, tram, dir);
            }
            self.road_infrastructure(road, road_owner, 2);
            self.road_infrastructure(tram, tram_owner, 2);
            (self.s.station_infrastructure)(owner, 1);
            (self.s.set_custom_road_index)(tile, specindex.value);
            if !spec.is_null() {
                unsafe {
                    ((*self.s.roadstop_resolver).set_tile_random)(
                        st,
                        tile,
                        ((self.shared.random)(self.shared.context) & 255) as u8,
                    );
                }
                crate::animated_tile::openttd_rust_trigger_roadstop(
                    st,
                    tile,
                    crate::station_type::SAT_BUILT,
                    crate::cargo_type::INVALID_CARGO,
                );
            }
            (self.shared.mark_dirty)(tile);
        }
        if !st.is_null() {
            self.after_station_tile_set_change(st, true, StationType(kind));
        }
    }
    fn retarget_road_vehicles(&self, station: u16, tile: u32) {
        for id in 0..(self.links().order_pool_size)() {
            let list = (self.links().order_list)(id as u16);
            if list.is_null() {
                continue;
            }
            // A live order list has a first shared vehicle, as in the original helper.
            let mut vehicle = unsafe { (self.links().order_vehicle)(list).unwrap_unchecked() };
            if vehicle.vehicle_type() != 1 {
                continue;
            }
            for index in 0..(self.links().order_count)(list) {
                let order = (self.links().order_destination)(list, index);
                if order.kind.0 != 1 || order.destination != station {
                    continue;
                }
                loop {
                    if (self.vehicle().order_type)(vehicle) == 1 && (vehicle).dest_tile() == tile {
                        let destination = (self.s.order_station_location)(vehicle, station);
                        (self.vehicle().set_dest_tile)(vehicle, destination);
                    }
                    let next = (self.links().next_shared)(vehicle);
                    if let Some(v) = next {
                        vehicle = v;
                    } else {
                        break;
                    }
                }
                break;
            }
        }
    }
    fn remove_road_tile(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        tile: u32,
        flags: u16,
        replacement: i32,
        waypoint: bool,
    ) {
        crate::witness::hit(crate::witness::STATION_CMD_REMOVE_ROAD_TILE);
        let id = (self.shared.station_index)(tile);
        let st = if waypoint {
            self.waypoint(StationID(id))
        } else {
            self.full_station(StationID(id))
        };
        if (self.shared.current_company)() != OWNER_WATER {
            unsafe {
                self.command
                    .ownership(cost.0, (self.world().station_owner)(st), 0);
            }
            if !cost.ok() {
                return;
            }
        }
        let truck = !waypoint && (self.shared.station_type)(tile) == StationType::Truck;
        let (stop, stop_state) = if waypoint {
            (std::ptr::null_mut(), std::ptr::null_mut())
        } else {
            road_stop_by_tile(self.shared, TileIndex(tile), RoadStopType(u8::from(truck)))
        };
        if waypoint {
            if flags & DoCommandFlag::BANKRUPT == 0 {
                unsafe {
                    self.command.no_vehicle(cost.0, tile);
                }
                if !cost.ok() {
                    return;
                }
            }
        } else if crate::map_func::World::new(self.shared).drive_through_stop_tile(tile)
            && flags & DoCommandFlag::BANKRUPT != 0
        {
            // Bankrupt companies skip the drive-through vehicle-presence check.
            if flags & DoCommandFlag::EXECUTE != 0 {
                self.vehicle().for_each_on_tile(tile, |vehicle| {
                    if vehicle.vehicle_type() == 1 {
                        let state = (self.s.road_vehicle_owner)(vehicle);
                        // SAFETY: Live road owner; no borrow crosses this access.
                        // Vehicles may also turn on this tile. Clear only a
                        // drive-through stop state, preserving turn-around bits.
                        unsafe { crate::road::clear_removed_drive_stop(state) };
                    }
                });
            }
        } else {
            unsafe {
                self.command.no_vehicle(cost.0, tile);
            }
            if !cost.ok() {
                return;
            }
        }
        let spec = unsafe {
            crate::newgrf_roadstop::openttd_rust_roadstop_spec(tile, self.s.roadstop_resolver)
        };
        if flags & DoCommandFlag::EXECUTE != 0 {
            if !waypoint {
                let head = (self.stops().head)(st, truck);
                let next = (self.stops().next)(stop);
                if head == stop {
                    (self.s.set_stop_head)(st, truck, next);
                    if next.is_null() {
                        (self.s.reset_facilities)(
                            st,
                            if truck { FACILITY_TRUCK } else { FACILITY_BUS },
                        );
                        (self.s.orders_dirty)();
                    }
                } else {
                    // Tell the predecessor to skip the removed stop.
                    let mut pred = head;
                    while (self.stops().next)(pred) != stop {
                        pred = (self.stops().next)(pred);
                    }
                    (self.s.set_stop_next)(pred, next);
                }
            }
            for tram in [false, true] {
                self.road_infrastructure(
                    self.road_type(tile, tram),
                    self.road_owner(tile, tram),
                    -2,
                );
            }
            let owner = (self.world().station_owner)(st);
            (self.s.station_infrastructure)(owner, u32::MAX);
            (self.command.infrastructure_dirty)(owner);
            let specindex = (self.s.custom_road_index)(tile);
            (self.s.close_stop_inspect)(tile);
            if !waypoint && crate::map_func::World::new(self.shared).drive_through_stop_tile(tile) {
                // ClearDriveThrough clears the tile for us.
                clear_drive_through(self.shared, TileIndex(tile), stop_state);
                crate::animated_tile::openttd_rust_animation_delete(tile, false);
            } else {
                crate::ground::clear_square(tile, self.shared);
            }
            if !waypoint {
                (self.s.delete_stop)(stop);
                self.retarget_road_vehicles(id, tile);
            }
            self.after_remove_tile(st, TileIndex(tile));
            if !waypoint && replacement < 0 {
                self.after_station_tile_set_change(
                    st,
                    false,
                    if truck {
                        StationType::Truck
                    } else {
                        StationType::Bus
                    },
                );
            }
            (self.s.remove_stop_tile_data)(st, tile);
            if specindex as i32 != replacement {
                unsafe {
                    crate::newgrf_roadstop::openttd_rust_roadstop_deallocate(
                        st,
                        specindex as u8,
                        self.s.roadstop_resolver,
                    );
                }
            }
            if waypoint {
                if replacement < 0 {
                    let area = self.make_station_area_smaller(
                        st,
                        Self::area((self.s.road_waypoint_tile_area)(st)),
                        StationType::RoadWaypoint,
                    );
                    (self.s.set_road_waypoint_area)(st, area);
                    self.update_station_sign_coord(st);
                    if area.tile == u32::MAX {
                        (self.s.reset_facilities)(st, FACILITY_TRUCK | FACILITY_BUS);
                        (self.s.facility_widget)(st, 3);
                        (self.s.virtual_coord)(st);
                        self.delete_station_if_empty(st);
                    }
                }
            } else {
                (self.s.clear_stop_area)(st, truck);
                let mut next = (self.stops().head)(st, truck);
                while !next.is_null() {
                    self.area_add(
                        (self.s.stop_area)(st, truck),
                        TileIndex((self.stops().tile)(next)),
                    );
                    next = (self.stops().next)(next);
                }
            }
        }
        let category = if waypoint || truck {
            Price::PR_CLEAR_STATION_TRUCK
        } else {
            Price::PR_CLEAR_STATION_BUS
        };
        cost.set(if spec.is_null() {
            crate::company::price(usize::from(category.0))
        } else {
            {
                let factor = (self.s.road_spec_clear_cost)(spec, category.0);
                crate::economy_func::get_price(
                    u32::from(category.0),
                    u32::from(factor.multiplier),
                    factor.shift,
                    -4,
                )
            }
        });
        let _ = ret;
    }
    fn remove_generic_road(&self, context: RoadStopRemoval<'_>) {
        let RoadStopRemoval {
            cost,
            ret,
            error,
            flags,
            area,
            waypoint,
            remove_road,
        } = context;
        cost.set(0);
        error.error(self.s.strings.no_station);
        let mut success = false;
        for tile in area.tiles(self.map()) {
            if (self.shared.tile_type)(tile) != MP_STATION
                || !crate::map_func::World::new(self.shared).any_road_stop_tile(tile)
                || ((self.shared.station_type)(tile) == StationType::RoadWaypoint) != waypoint
            {
                continue;
            }
            let mut types = [63_u8; 2];
            let mut owners = [OWNER_NONE; 2];
            let mut bits = 0;
            if crate::map_func::World::new(self.shared).drive_through_stop_tile(tile) {
                for tram in [false, true] {
                    let i = usize::from(tram);
                    types[i] = self.road_type(tile, tram);
                    if types[i] == 63 {
                        continue;
                    }
                    owners[i] = self.road_owner(tile, tram);
                    if remove_road && owners[i] == (self.shared.current_company)() {
                        types[i] = 63;
                    }
                }
                bits = if crate::map_func::World::new(self.shared).drive_through_axis(tile) == 0 {
                    10
                } else {
                    5
                };
            }
            self.remove_road_tile(ret, ret, tile, flags, -1, waypoint);
            if !ret.ok() {
                error.take(ret);
                continue;
            }
            cost.add_money(ret.money());
            success = true;
            if flags & DoCommandFlag::EXECUTE != 0
                && (types[0] != INVALID_ROADTYPE || types[1] != INVALID_ROADTYPE)
            {
                let town = crate::town::spatial::closest(self.shared, tile, u32::MAX);
                let authority = unsafe { &*self.s.authority };
                let town = (authority.town_index)(town);
                (self.rail().make_road)(
                    tile,
                    crate::rail_cmd::CrossingRoad {
                        bits,
                        road_type: types[0],
                        tram_type: types[1],
                        town,
                        road_owner: owners[0],
                        tram_owner: owners[1],
                    },
                );
                let count = bits.count_ones() as i32;
                self.road_infrastructure(types[0], owners[0], count);
                self.road_infrastructure(types[1], owners[1], count);
            }
        }
        if !success {
            cost.take(error);
        }
    }
    fn remove_road_command(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        error: Cost<'_>,
        flags: u16,
        start: u32,
        mut end: u32,
        width: u8,
        height: u8,
        stop_type: u8,
        remove_road: bool,
        waypoint: bool,
    ) {
        let area = if waypoint {
            if end == 0 {
                end = start;
            }
            let size = ((self.shared.map_max_y)() + 1) << self.map().log_x;
            if start >= size || end >= size {
                return cost.error(INVALID_STRING_ID);
            }
            TileArea::between(self.map(), start, end)
        } else {
            if stop_type >= 2
                || width == 0
                || height == 0
                || !self.buildable_area(start, u16::from(width), u16::from(height))
                || (remove_road && flags & DoCommandFlag::BANKRUPT != 0)
            {
                return cost.error(INVALID_STRING_ID);
            }
            TileArea {
                tile: start,
                w: u16::from(width),
                h: u16::from(height),
            }
        };
        self.remove_generic_road(RoadStopRemoval {
            cost,
            ret,
            error,
            flags,
            area,
            waypoint,
            remove_road,
        });
    }
}

/// # Safety
/// Tables, native cost slots and serialized map/pools remain live for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_build_road(
    s: *const CommandServicesTable,
    cost: *mut CommandCost,
    ret: *mut CommandCost,
    flags: u16,
    tile: u32,
    width: u8,
    length: u8,
    stop_type: u8,
    drive: bool,
    dir: u8,
    rt: u8,
    class: u16,
    index: u16,
    target: u16,
    adjacent: bool,
) {
    let c = unsafe { Ctx::new(s) };
    c.build_road(
        c.c(cost),
        c.c(ret),
        flags,
        tile,
        width,
        length,
        stop_type,
        drive,
        dir,
        rt,
        class,
        index,
        target,
        adjacent,
    );
}

/// # Safety
/// Tables and three native cost slots are live for the serialized synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_remove_road_area(
    s: *const CommandServicesTable,
    cost: *mut CommandCost,
    ret: *mut CommandCost,
    error: *mut CommandCost,
    flags: u16,
    start: u32,
    end: u32,
    width: u8,
    height: u8,
    stop_type: u8,
    remove_road: bool,
    waypoint: bool,
) {
    let c = unsafe { Ctx::new(s) };
    c.remove_road_command(
        c.c(cost),
        c.c(ret),
        c.c(error),
        flags,
        start,
        end,
        width,
        height,
        stop_type,
        remove_road,
        waypoint,
    );
}

impl Ctx<'_> {
    fn airport_tiles(
        &self,
        spec: *const AirportSpec,
        layout: u8,
        origin: u32,
    ) -> impl Iterator<Item = AirportTile> + '_ {
        let count = (self.s.airport_layout)(spec, layout).size;
        (0..count).map(move |index| (self.s.airport_layout_tile)(spec, layout, origin, index))
    }
    fn nearest_town(
        &self,
        spec: *const AirportSpec,
        rotation: u8,
        origin: u32,
        tiles: impl Iterator<Item = u32>,
    ) -> (*const crate::services::TownHandle, u32) {
        let size = (self.s.airport_size)(spec);
        let (width, height) = if rotation == 2 || rotation == 6 {
            (size.h, size.w)
        } else {
            (size.w, size.h)
        };
        let min_x = self.map().tile_x(origin);
        let min_y = self.map().tile_y(origin);
        let max_x = min_x + u32::from(width) - 1;
        let max_y = min_y + u32::from(height) - 1;
        let mut nearest = std::ptr::null();
        let mut distance = u32::MAX - 1;
        let authority = unsafe { &*self.s.authority };
        for tile in tiles {
            let x = self.map().tile_x(tile);
            let y = self.map().tile_y(tile);
            if x != min_x && x != max_x && y != min_y && y != max_y {
                continue;
            }
            let town =
                crate::town::spatial::calc_closest(self.shared, tile, distance.wrapping_add(1));
            if town.is_null() {
                continue;
            }
            let owner = (self.shared.town_owner)(town);
            let xy = unsafe { crate::town::position_raw(owner) };
            let dist = self.map().tile_x(xy).abs_diff(x) + self.map().tile_y(xy).abs_diff(y);
            if dist == distance && (authority.town_index)(town) < (authority.town_index)(nearest) {
                nearest = town;
            }
            if dist < distance {
                nearest = town;
                distance = dist;
            }
        }
        (nearest, distance)
    }
    fn nearest_existing_airport(
        &self,
        st: *mut BaseStation,
    ) -> (*const crate::services::TownHandle, u32) {
        let area = Self::area((self.s.airport_area)(st));
        let rotation = (self.s.airport_rotation)(st);
        let id = (self.links().graphs.station_id)(st);
        self.nearest_town(
            crate::newgrf_airport::station_spec(st, unsafe { &*self.s.airport_runtime }),
            rotation,
            area.tile,
            area.tiles(self.map()).filter(|&t| {
                (self.shared.tile_type)(t) == MP_STATION
                    && (self.shared.station_type)(t) == StationType::Airport
                    && (self.shared.station_index)(t) == id
            }),
        )
    }
    fn noise(&self, spec: *const AirportSpec, distance: u32) -> u8 {
        unsafe {
            openttd_rust_station_airport_noise(self.s, (self.s.airport_noise)(spec), distance)
        }
    }
    fn noise_pointer(&self, town: *const crate::services::TownHandle) -> *mut u16 {
        let owner = (self.shared.town_owner)(town);
        let fields = unsafe { crate::town::openttd_rust_town_simulation(owner) };
        unsafe { &raw mut (*fields).noise_reached }
    }
    fn build_airport(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        flags: u16,
        tile: u32,
        airport: u8,
        layout: u8,
        mut target: u16,
        adjacent: bool,
    ) {
        crate::witness::hit(crate::witness::STATION_CMD_BUILD_AIRPORT);
        let reuse = target != StationID::NEW.0;
        if !reuse {
            target = StationID::INVALID.0;
        }
        let distant = target != StationID::INVALID.0;
        if distant && (!(self.s.distant_join)() || self.full_station(StationID(target)).is_null()) {
            return cost.error(INVALID_STRING_ID);
        }
        if airport >= 128 {
            return cost.error(INVALID_STRING_ID);
        }
        self.authority(cost, tile, flags);
        if !cost.ok() {
            return;
        }
        let runtime = unsafe { &*self.s.airport_runtime };
        let spec = crate::newgrf_airport::get(airport, runtime);
        if !crate::newgrf_airport::available(spec, runtime)
            || usize::from(layout) >= (self.s.airport_layout_count)(spec)
            || !crate::newgrf_airport::in_bounds(spec, layout, tile, runtime)
        {
            return cost.error(INVALID_STRING_ID);
        }
        let rotation = (self.s.airport_layout)(spec, layout).rotation;
        let size = (self.s.airport_size)(spec);
        let (w, h) = if rotation == 2 || rotation == 6 {
            (size.h, size.w)
        } else {
            (size.w, size.h)
        };
        let area = TileArea {
            tile,
            w: u16::from(w),
            h: u16::from(h),
        };
        let spread = (self.s.spread)();
        if w > spread || h > spread {
            return cost.error(self.s.strings.spread);
        }
        let mut allowed = -1;
        cost.set(0);
        for t in self.airport_tiles(spec, layout, tile) {
            self.check_buildable(
                ret,
                TileIndex(t.tile),
                DiagDirections::EMPTY,
                &mut allowed,
                true,
                true,
            );
            if !ret.ok() {
                return cost.take(ret);
            }
            cost.add_money(ret.money());
            crate::landscape_cmd::clear_command(self.command, ret.0, t.tile, flags);
            if !ret.ok() {
                return cost.take(ret);
            }
            cost.add_money(ret.money());
        }
        let (nearest, distance) = self.nearest_town(
            spec,
            rotation,
            tile,
            self.airport_tiles(spec, layout, tile).map(|t| t.tile),
        );
        let noise = u32::from(self.noise(spec, distance));
        let noise_ptr = self.noise_pointer(nearest);
        let authority = unsafe { &*self.s.authority };
        let mut refused = std::ptr::null();
        let mut message = 0;
        if unsafe { (*self.s.shared).settings().station_noise.read() } {
            if u32::from(unsafe { noise_ptr.read() }) + noise
                > u32::from((self.s.town_max_noise)(nearest))
            {
                message = self.s.strings.refuse_noise;
                refused = nearest;
            }
        } else if (authority.council_tolerance)() != 3 {
            let town = crate::town::spatial::closest(self.shared, tile, u32::MAX);
            let mut count = 0_u32;
            for id in 0..(self.world().pool_size)() {
                let st = self.full_station(StationID(id as u16));
                if st.is_null() {
                    continue;
                }
                if (self.s.station_town)(st) == town
                    && (self.world().facilities)(st) & FACILITY_AIRPORT != 0
                    && (self.s.airport_type)(st) != 9
                {
                    count += 1;
                }
            }
            if count >= 2 {
                message = self.s.strings.refuse_airport;
                refused = town;
            }
        }
        if message != 0 {
            (self.cost.error_param)(cost.0, message, u64::from((authority.town_index)(refused)));
            return;
        }
        let mut st = self.find_joining_base_station(JoiningStation {
            out: ret,
            existing_station: StationID::INVALID,
            station_to_join: StationID(target),
            adjacent,
            ta: area,
            waypoint: false,
            road: false,
            error_message: StringID(self.s.strings.remove_station),
        });
        if !ret.ok() {
            return cost.take(ret);
        }
        if st.is_null() && distant {
            st = self.full_station(StationID(target));
        }
        let name = if (self.s.airport_airplanes)(crate::airport::get_airport(airport, unsafe {
            &*self.s.airport_runtime
        })) {
            StationNaming::STATIONNAMING_AIRPORT
        } else {
            StationNaming::STATIONNAMING_HELIPORT
        };
        st = self.build_station_part(ret, st, DoCommandFlags(flags), reuse, area, name);
        if !ret.ok() {
            return cost.take(ret);
        }
        if !st.is_null() && (self.s.airport_tile)(st) != u32::MAX {
            return cost.error(self.s.strings.close_airport);
        }
        for _ in self.airport_tiles(spec, layout, tile) {
            cost.add_money(crate::company::price(usize::from(
                Price::PR_BUILD_STATION_AIRPORT.0,
            )));
        }
        if flags & DoCommandFlag::EXECUTE == 0 {
            return;
        }
        unsafe {
            noise_ptr.write(noise_ptr.read().wrapping_add(noise as u16));
        }
        crate::station_lifecycle::add_facility_raw(st, StationFacility::Airport, tile, unsafe {
            &*self.s.lifecycle
        });
        (self.s.set_airport_details)(st, airport, layout, rotation);
        self.before_add_rect(ret, st, area, StationRectMode::ADD_TRY);
        let owner = (self.world().station_owner)(st);
        let id = (self.links().graphs.station_id)(st);
        for t in self.airport_tiles(spec, layout, tile) {
            (self.s.make_airport)(t.tile, owner, id, t.gfx, 3);
            (self.s.set_random_bits)(
                t.tile,
                ((self.shared.random)(self.shared.context) & 15) as u8,
            );
            self.area_add((self.s.airport_area)(st), TileIndex(t.tile));
            if (self.s.airport_animated)(crate::newgrf_airport::tile_spec_by_gfx(t.gfx, unsafe {
                &*self.s.airport_runtime
            })) {
                crate::animated_tile::openttd_rust_animation_add(t.tile, true);
            }
        }
        for t in self.airport_tiles(spec, layout, tile) {
            crate::animated_tile::airport_built(st, t.tile);
        }
        unsafe {
            crate::aircraft::openttd_rust_aircraft_replacement(self.s.aircraft, id);
        }
        (self.s.airport_infrastructure)(owner, 1);
        self.after_station_tile_set_change(st, true, StationType::Airport);
        (self.s.station_view_invalidate)(st, -1);
        if unsafe { (*self.s.shared).settings().station_noise.read() } {
            (self.s.town_view_dirty)(nearest);
        }
    }
    fn remove_airport(&self, cost: Cost<'_>, ret: Cost<'_>, tile: u32, flags: u16) {
        crate::witness::hit(crate::witness::STATION_CMD_REMOVE_AIRPORT);
        let st = self.full_station(StationID((self.shared.station_index)(tile)));
        if (self.shared.current_company)() != OWNER_WATER {
            unsafe {
                self.command
                    .ownership(cost.0, (self.world().station_owner)(st), 0);
            }
            if !cost.ok() {
                return;
            }
        }
        cost.set(0);
        let id = (self.links().graphs.station_id)(st);
        let mut from = 0;
        while let Some(v) = (self.shared.vehicle_next)(from) {
            let identity = (self.s.aircraft_vehicle)(v);
            from = identity.id + 1;
            if v.vehicle_type() != 3 || v.subtype() > 2 {
                continue;
            }
            let target = unsafe { (&raw const (*identity.state).targetairport).read() };
            let state = unsafe { (&raw const (*identity.state).state).read() };
            if target == id && state != 14 {
                return cost.error(self.s.strings.aircraft_in_way);
            }
        }
        if flags & DoCommandFlag::EXECUTE != 0 {
            for index in 0..(self.s.hangar_count)(
                st,
                crate::newgrf_airport::station_spec(st, unsafe { &*self.s.airport_runtime }),
            ) {
                (self.s.close_hangar)((self.s.hangar_tile)(
                    st,
                    index,
                    crate::newgrf_airport::station_spec(st, unsafe { &*self.s.airport_runtime }),
                ));
            }
            let (nearest, distance) = self.nearest_existing_airport(st);
            let noise = self.noise(
                crate::newgrf_airport::station_spec(st, unsafe { &*self.s.airport_runtime }),
                distance,
            );
            let ptr = self.noise_pointer(nearest);
            unsafe {
                ptr.write(ptr.read().wrapping_sub(u16::from(noise)));
            }
            if unsafe { (*self.s.shared).settings().station_noise.read() } {
                (self.s.town_view_dirty)(nearest);
            }
        }
        let area = Self::area((self.s.airport_area)(st));
        for tile in area.tiles(self.map()) {
            if (self.shared.tile_type)(tile) != MP_STATION
                || (self.shared.station_type)(tile) != StationType::Airport
                || (self.shared.station_index)(tile) != id
            {
                continue;
            }
            unsafe {
                self.command.no_vehicle(ret.0, tile);
            }
            if !ret.ok() {
                return cost.take(ret);
            }
            cost.add_money(crate::company::price(usize::from(
                Price::PR_CLEAR_STATION_AIRPORT.0,
            )));
            if flags & DoCommandFlag::EXECUTE != 0 {
                crate::ground::clear_square(tile, self.shared);
                (self.s.close_airport_tile_inspect)(tile);
            }
        }
        if flags & DoCommandFlag::EXECUTE != 0 {
            (self.s.delete_airport_storage)(st);
            self.after_remove_rect(st, area);
            (self.s.clear_airport)(st);
            (self.s.reset_facilities)(st, FACILITY_AIRPORT);
            (self.s.orders_dirty)();
            (self.s.station_view_invalidate)(st, -1);
            (self.s.airport_infrastructure)((self.world().station_owner)(st), u32::MAX);
            self.after_station_tile_set_change(st, false, StationType::Airport);
            (self.s.close_airport_inspect)(st);
        }
    }
}
/// # Safety
/// The native iterator, spec, tables and distance output remain live for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_airport_nearest(
    s: *const CommandServicesTable,
    spec: *const AirportSpec,
    rotation: u8,
    origin: u32,
    iterator: *mut TileIterator,
    distance: *mut u32,
) -> *const crate::services::TownHandle {
    let c = unsafe { Ctx::new(s) };
    let mut started = false;
    let tiles = std::iter::from_fn(|| {
        if started {
            (c.s.iterator_next)(iterator);
        }
        started = true;
        let tile = (c.s.iterator_tile)(iterator);
        if tile == u32::MAX { None } else { Some(tile) }
    });
    let (town, dist) = c.nearest_town(spec, rotation, origin, tiles);
    unsafe {
        distance.write(dist);
    }
    town
}
/// # Safety
/// Tables and serialized station/town pools remain live throughout this entry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_update_airport_noise(s: *const CommandServicesTable) {
    let c = unsafe { Ctx::new(s) };
    let mut from = 0;
    loop {
        let id = (c.shared.town_next_id)(from);
        if id == u32::MAX {
            break;
        }
        from = id + 1;
        let owner = (c.shared.town_owner_by_id)(id);
        let fields = unsafe { crate::town::openttd_rust_town_simulation(owner) };
        unsafe {
            (&raw mut (*fields).noise_reached).write(0);
        }
    }
    for id in 0..(c.world().pool_size)() {
        let st = c.full_station(StationID(id as u16));
        if st.is_null() {
            continue;
        }
        if (c.s.airport_tile)(st) != u32::MAX && (c.s.airport_type)(st) != 9 {
            let (nearest, distance) = c.nearest_existing_airport(st);
            let spec = crate::newgrf_airport::station_spec(st, unsafe { &*c.s.airport_runtime });
            let noise = c.noise(spec, distance);
            let ptr = c.noise_pointer(nearest);
            unsafe {
                ptr.write(ptr.read().wrapping_add(u16::from(noise)));
            }
        }
    }
}
/// # Safety
/// Native tables and distinct result slots remain live for the serialized command.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_build_airport(
    s: *const CommandServicesTable,
    cost: *mut CommandCost,
    ret: *mut CommandCost,
    flags: u16,
    tile: u32,
    airport: u8,
    layout: u8,
    target: u16,
    adjacent: bool,
) {
    let c = unsafe { Ctx::new(s) };
    c.build_airport(
        c.c(cost),
        c.c(ret),
        flags,
        tile,
        airport,
        layout,
        target,
        adjacent,
    );
}

impl Ctx<'_> {
    fn dock(&self, tile: u32) -> bool {
        (self.shared.tile_type)(tile) == MP_STATION
            && (self.shared.station_type)(tile) == StationType::Dock
    }
    fn possible_docking(&self, tile: u32) -> bool {
        match (self.shared.tile_type)(tile) {
            MP_WATER => {
                if (self.shared.water_middle_lock)(tile) {
                    return false;
                }
            }
            MP_RAILWAY | MP_STATION | MP_TUNNELBRIDGE => (),
            _ => return false,
        }
        crate::movement::water_tracks(self.shared, tile, 255) != 0
    }
    fn check_docking(&self, tile: u32) {
        crate::witness::hit(crate::witness::STATION_CMD_CHECK_DOCKING);
        for dir in 0..4 {
            let next = self.map().add_diag(tile, dir);
            if !(self.shared.tile_valid)(next) {
                continue;
            }
            if self.dock(next) && (self.shared.station_gfx)(next) >= GFX_DOCK_BASE_WATER_PART {
                let st = self.full_station(StationID((self.shared.station_index)(next)));
                self.area_add((self.s.docking_area)(st), TileIndex(tile));
                (self.s.set_docking)(tile, true);
            }
            if (self.shared.tile_type)(next) == MP_INDUSTRY {
                let st = (self.s.neutral_for_tile)(next);
                if !st.is_null() {
                    self.area_add((self.s.docking_area)(st), TileIndex(tile));
                    (self.s.set_docking)(tile, true);
                }
            }
            if (self.shared.tile_type)(next) == MP_STATION
                && (self.shared.station_type)(next) == StationType::Oilrig
            {
                let st = self.full_station(StationID((self.shared.station_index)(next)));
                self.area_add((self.s.docking_area)(st), TileIndex(tile));
                (self.s.set_docking)(tile, true);
            }
        }
    }
    fn update_docking(&self, st: *mut BaseStation) {
        (self.s.docking_clear)(st);
        let industry = (self.s.associated_industry)(st);
        let area = if industry.is_null() {
            Self::area((self.s.ship_station_area)(st))
        } else {
            (self.s.industry_area)(industry)
        };
        if area.tile == u32::MAX {
            return;
        }
        let x = self.map().tile_x(area.tile);
        let y = self.map().tile_y(area.tile);
        let x1 = x.saturating_sub(1);
        let y1 = y.saturating_sub(1);
        let x2 = (x + u32::from(area.w) + 1).min(self.map().size_x());
        let y2 = (y + u32::from(area.h) + 1).min((self.shared.map_max_y)() + 1);
        let area = TileArea {
            tile: (y1 << self.map().log_x) + x1,
            w: (x2 - x1) as u16,
            h: (y2 - y1) as u16,
        };
        for tile in area.tiles(self.map()) {
            if (self.shared.tile_valid)(tile) && self.possible_docking(tile) {
                self.check_docking(tile);
            }
        }
    }
    fn remove_docking_tile(&self, tile: u32) {
        for dir in 0..4 {
            let next = self.map().add_diag(tile, dir);
            if !(self.shared.tile_valid)(next) {
                continue;
            }
            if (self.shared.tile_type)(next) == MP_STATION {
                let st = self.full_station(StationID((self.shared.station_index)(next)));
                if !st.is_null() {
                    self.update_docking(st);
                }
            } else if (self.shared.tile_type)(next) == MP_INDUSTRY {
                let st = (self.s.neutral_for_tile)(next);
                if !st.is_null() {
                    self.update_docking(st);
                }
            }
        }
    }
    fn clear_docking_neighbours(&self, tile: u32) {
        for dir in 0..4 {
            let next = self.map().add_diag(tile, dir);
            if !(self.shared.tile_valid)(next) {
                continue;
            }
            if self.possible_docking(next) {
                (self.s.set_docking)(next, false);
                self.check_docking(next);
            }
        }
    }
    fn build_dock(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        flags: u16,
        tile: u32,
        mut target: u16,
        adjacent: bool,
    ) {
        crate::witness::hit(crate::witness::STATION_CMD_BUILD_DOCK);
        let reuse = target != StationID::NEW.0;
        if !reuse {
            target = StationID::INVALID.0;
        }
        let distant = target != StationID::INVALID.0;
        if distant && (!(self.s.distant_join)() || self.full_station(StationID(target)).is_null()) {
            return cost.error(INVALID_STRING_ID);
        }
        let slope = crate::map_func::World::new(self.shared).tile_slope(tile);
        let dir = get_inclined_slope_direction(slope);
        if dir == INVALID_DIAGDIR {
            return cost.error(self.s.strings.unsuitable);
        }
        // ReverseDiagDir.
        let dir = dir ^ 2;
        if (self.shared.has_water_ground)(tile) {
            return cost.error(self.s.strings.unsuitable);
        }
        self.authority(cost, tile, flags);
        if !cost.ok() {
            return;
        }
        self.default_bridge_above_ok(cost, TileIndex(tile), StationType::Dock, StationGfx(dir));
        if !cost.ok() {
            return;
        }
        cost.set(crate::company::price(usize::from(
            Price::PR_BUILD_STATION_DOCK.0,
        )));
        crate::landscape_cmd::clear_command(self.command, ret.0, tile, flags);
        if !ret.ok() {
            return cost.take(ret);
        }
        cost.add_money(ret.money());
        let flat = self.map().add_diag(tile, dir);
        if !(self.shared.has_water_ground)(flat)
            || crate::map_func::World::new(self.shared).tile_slope(flat) != 0
        {
            return cost.error(self.s.strings.unsuitable);
        }
        self.default_bridge_above_ok(
            ret,
            TileIndex(flat),
            StationType::Dock,
            StationGfx(GFX_DOCK_BASE_WATER_PART + (dir & 1)),
        );
        if !ret.ok() {
            return cost.take(ret);
        }
        let water = crate::map_func::World::new(self.shared).water_class(flat);
        let add_cost = !(self.shared.is_water_tile)(flat);
        crate::landscape_cmd::clear_command(self.command, ret.0, flat, flags);
        if !ret.ok() {
            return cost.take(ret);
        }
        if add_cost {
            cost.add_money(ret.money());
        }
        let beyond = self.map().add_diag(flat, dir);
        if (self.shared.tile_type)(beyond) != MP_WATER
            || crate::map_func::World::new(self.shared).tile_slope(beyond) != 0
        {
            return cost.error(self.s.strings.unsuitable);
        }
        let (dx, dy) = match dir {
            0 => (-1, 0),
            3 => (0, -1),
            _ => (0, 0),
        };
        let area = TileArea {
            tile: tile.wrapping_add(self.map().diff(dx, dy)),
            w: if dir & 1 == 0 { 2 } else { 1 },
            h: if dir & 1 == 0 { 1 } else { 2 },
        };
        let mut st = self.find_joining_base_station(JoiningStation {
            out: ret,
            existing_station: StationID::INVALID,
            station_to_join: StationID(target),
            adjacent,
            ta: area,
            waypoint: false,
            road: false,
            error_message: StringID(self.s.strings.remove_station),
        });
        if !ret.ok() {
            return cost.take(ret);
        }
        if st.is_null() && distant {
            st = self.full_station(StationID(target));
        }
        st = self.build_station_part(
            ret,
            st,
            DoCommandFlags(flags),
            reuse,
            area,
            StationNaming::STATIONNAMING_DOCK,
        );
        if !ret.ok() {
            return cost.take(ret);
        }
        if flags & DoCommandFlag::EXECUTE != 0 {
            self.area_add((self.s.ship_station_area)(st), TileIndex(tile));
            self.area_add((self.s.ship_station_area)(st), TileIndex(flat));
            crate::station_lifecycle::add_facility_raw(st, StationFacility::Dock, tile, unsafe {
                &*self.s.lifecycle
            });
            self.before_add_rect(ret, st, area, StationRectMode::ADD_TRY);
            let owner = (self.world().station_owner)(st);
            if water == 1
                && !((self.shared.has_water_class)(flat)
                    && crate::map_func::World::new(self.shared).water_class(flat) == 1
                    && (self.shared.tile_owner)(flat) == (self.shared.current_company)())
            {
                (self.s.water_infrastructure)(owner, 1);
            }
            (self.s.station_infrastructure)(owner, 2);
            (self.s.make_dock)(
                tile,
                owner,
                (self.links().graphs.station_id)(st),
                dir,
                water,
            );
            self.update_docking(st);
            self.after_station_tile_set_change(st, true, StationType::Dock);
        }
    }
    fn dock_land(&self, tile: u32) -> u32 {
        if (self.shared.station_gfx)(tile) < GFX_DOCK_BASE_WATER_PART {
            return tile;
        }
        for dir in 0..4 {
            let next = self.map().add_diag(tile, dir);
            if !(self.shared.tile_valid)(next) || !self.dock(next) {
                continue;
            }
            let gfx = (self.shared.station_gfx)(next);
            if gfx < 4 && self.map().add_diag(next, gfx) == tile {
                return next;
            }
        }
        u32::MAX
    }
    fn remove_dock(&self, cost: Cost<'_>, ret: Cost<'_>, tile: u32, flags: u16) {
        crate::witness::hit(crate::witness::STATION_CMD_REMOVE_DOCK);
        let st = self.full_station(StationID((self.shared.station_index)(tile)));
        let owner = (self.world().station_owner)(st);
        unsafe {
            self.command.ownership(cost.0, owner, 0);
        }
        if !cost.ok() {
            return;
        }
        if !self.dock(tile) {
            return cost.error(INVALID_STRING_ID);
        }
        let land = self.dock_land(tile);
        if land == u32::MAX {
            return cost.error(INVALID_STRING_ID);
        }
        let water = self.map().add_diag(land, (self.shared.station_gfx)(land));
        unsafe {
            self.command.no_vehicle(cost.0, land);
        }
        if cost.ok() {
            unsafe {
                self.command.no_vehicle(cost.0, water);
            }
        }
        if !cost.ok() {
            return;
        }
        if flags & DoCommandFlag::EXECUTE != 0 {
            crate::ground::clear_square(land, self.shared);
            (self.shared.mark_dirty)(land);
            unsafe {
                crate::water::restore(
                    water,
                    owner,
                    self.shared,
                    &*(self.shared.water_services)(),
                    &*self.s.restoration,
                );
            }
            self.after_remove_tile(st, TileIndex(land));
            self.after_remove_tile(st, TileIndex(water));
            let area = self.make_station_area_smaller(
                st,
                Self::area((self.s.ship_station_area)(st)),
                StationType::Dock,
            );
            (self.s.set_ship_area)(st, area);
            self.update_docking(st);
            if area.tile == u32::MAX {
                (self.s.set_ship_area)(
                    st,
                    TileArea {
                        tile: u32::MAX,
                        w: 0,
                        h: 0,
                    },
                );
                (self.s.docking_clear)(st);
                (self.s.reset_facilities)(st, FACILITY_DOCK);
                (self.s.orders_dirty)();
            }
            (self.s.station_infrastructure)(owner, u32::MAX - 1);
            self.after_station_tile_set_change(st, false, StationType::Dock);
            self.clear_docking_neighbours(land);
            self.clear_docking_neighbours(water);
            let id = (self.links().graphs.station_id)(st);
            let mut from = 0;
            while let Some(v) = (self.shared.vehicle_next)(from) {
                let identity = (self.s.aircraft_vehicle)(v);
                from = identity.id + 1;
                if v.vehicle_type() != 2 || (self.vehicle().order_destination)(v) != id {
                    continue;
                }
                if (self.vehicle().order_type)(v) == 3 {
                    let tile = (v).tile();
                    if !((self.shared.docking_tile)(tile)
                        && unsafe {
                            crate::ship_control::openttd_rust_ship_is_destination_abi(
                                tile,
                                u32::from(id),
                                self.s.ship,
                                self.s.shared,
                            ) != 0
                        })
                    {
                        crate::vehicle::visits::leave_station(v, self.vehicle().visits());
                    }
                }
                if (self.vehicle().order_type)(v) == 1
                    && (self.world().facilities)(st) & FACILITY_DOCK == 0
                {
                    let dest = unsafe {
                        crate::ship_control::openttd_rust_ship_station_destination(
                            identity.id,
                            u32::from(id),
                            v.0.as_ptr(),
                            self.s.ship,
                            self.s.shared,
                        )
                    };
                    unsafe {
                        crate::ship_control::openttd_rust_ship_destination(
                            identity.id,
                            dest,
                            self.s.ship,
                            self.s.shared,
                        );
                    }
                }
            }
        }
        cost.set(crate::company::price(usize::from(
            Price::PR_CLEAR_STATION_DOCK.0,
        )));
        let _ = ret;
    }
    fn build_oilrig(&self, ret: Cost<'_>, tile: u32) {
        crate::witness::hit(crate::witness::STATION_CMD_BUILD_OILRIG);
        if !(self.s.station_available)() {
            debug!(
                misc,
                0, "Can't allocate station for oilrig at 0x{:X}, reverting to oilrig only", tile
            );
            return;
        }
        let st = (self.s.new_station)(tile);
        crate::station::spatial::insert(self.shared, st);
        (self.s.set_town)(
            st,
            crate::town::spatial::closest(self.shared, tile, u32::MAX),
        );
        (self.s.set_string_id)(
            st,
            crate::station_lifecycle::generate_name(
                st,
                tile,
                StationNaming::STATIONNAMING_OILRIG,
                unsafe { &*self.s.lifecycle },
            ),
        );
        (self.s.associate_industry)(st, tile);
        crate::animated_tile::openttd_rust_animation_delete(tile, false);
        (self.s.make_oilrig)(
            tile,
            (self.links().graphs.station_id)(st),
            crate::map_func::World::new(self.shared).water_class(tile),
        );
        (self.s.set_station_owner)(st, OWNER_NONE);
        (self.s.set_airport_kind_rotation)(st, 9, 0);
        self.area_add((self.s.airport_area)(st), TileIndex(tile));
        self.area_add((self.s.ship_station_area)(st), TileIndex(tile));
        (self.s.set_facilities)(st, FACILITY_AIRPORT | FACILITY_DOCK);
        (self.s.set_build_date)(st, crate::timer_game_common::calendar_date().0);
        self.update_docking(st);
        self.before_add_tile(ret, st, TileIndex(tile), StationRectMode::ADD_FORCE);
        (self.s.virtual_coord)(st);
        // The industry tile became a station tile, which may change neighbouring
        // catchments. RecomputeCatchment cannot remove nearby industries here,
        // so the industry's set is moved out first.
        if (self.shared.station_serve_neutral)() {
            let c = self.catchment();
            let industry = (c.c.associated_industry)(st);
            let list = crate::industry::stations_near((c.c.industry_owner)(industry));
            // SAFETY: Live industry owner; the set is moved into a local before the visits.
            let nearby = unsafe { std::mem::take(&mut *list) };
            unsafe { (*list).clear() };
            for near in nearby.entries() {
                self.recompute_catchment(near.station, true);
                unsafe {
                    crate::station_service::openttd_rust_station_acceptance(
                        near.station,
                        self.s.station,
                        1,
                    );
                }
            }
        }
        self.recompute_catchment(st, false);
        unsafe {
            crate::station_service::openttd_rust_station_acceptance(st, self.s.station, 0);
        }
    }
    fn delete_oilrig(&self, tile: u32) {
        crate::witness::hit(crate::witness::STATION_CMD_DELETE_OILRIG);
        let st = self.full_station(StationID((self.shared.station_index)(tile)));
        unsafe {
            crate::water::restore(
                tile,
                0x10,
                self.shared,
                &*(self.shared.water_services)(),
                &*self.s.restoration,
            );
        }
        assert!(
            (self.world().facilities)(st) == (FACILITY_AIRPORT | FACILITY_DOCK)
                && (self.s.airport_type)(st) == 9
        );
        let industry = (self.s.associated_industry)(st);
        if !industry.is_null() && (self.s.industry_neutral)(industry) == st {
            (self.s.clear_industry_neutral)(industry);
        }
        (self.world().delete_station)(st);
    }
    fn in_orders(&self, station: u16, include_company: bool, company: u8) -> bool {
        for id in 0..(self.links().order_pool_size)() {
            let list = (self.links().order_list)(id as u16);
            if list.is_null() {
                continue;
            }
            let v = unsafe { (self.links().order_vehicle)(list).unwrap_unchecked() };
            if (v.owner() == company) != include_company {
                continue;
            }
            for index in 0..(self.links().order_count)(list) {
                let order = (self.links().order_destination)(list, index);
                if order.destination == station
                    && matches!(order.kind.0, OT_GOTO_STATION | OT_GOTO_WAYPOINT)
                {
                    return true;
                }
            }
        }
        false
    }
}
/// # Safety
/// Tables and distinct native result slots are live for this serialized command.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_build_dock(
    s: *const CommandServicesTable,
    cost: *mut CommandCost,
    ret: *mut CommandCost,
    flags: u16,
    tile: u32,
    target: u16,
    adjacent: bool,
) {
    let c = unsafe { Ctx::new(s) };
    c.build_dock(c.c(cost), c.c(ret), flags, tile, target, adjacent);
}

/// # Safety
/// Tables and the station shell remain live; no owner borrow spans reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_update_docking(
    s: *const CommandServicesTable,
    st: *mut BaseStation,
) {
    unsafe { Ctx::new(s) }.update_docking(st);
}
/// # Safety
/// Tables and serialized map/pools remain live for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_remove_docking_tile(
    s: *const CommandServicesTable,
    tile: u32,
) {
    unsafe { Ctx::new(s) }.remove_docking_tile(tile);
}
/// # Safety
/// Tables and the original valid tile remain live for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_clear_docking_neighbours(
    s: *const CommandServicesTable,
    tile: u32,
) {
    unsafe { Ctx::new(s) }.clear_docking_neighbours(tile);
}
/// Shared direct Rust entry for the original guarded docking registration.
/// # Safety
/// Services and the original valid tile remain live for the synchronous call.
pub(crate) unsafe fn check_possible_docking(s: *const CommandServicesTable, tile: u32) {
    // SAFETY: the caller supplies the process-lifetime tables under the game-state mutex.
    let c = unsafe { Ctx::new(s) };
    if c.possible_docking(tile) {
        c.check_docking(tile);
    }
}
/// # Safety
/// Tables and the original valid tile remain live for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_possible_docking_abi(
    s: *const CommandServicesTable,
    tile: u32,
) -> u8 {
    u8::from(unsafe { Ctx::new(s) }.possible_docking(tile))
}
/// # Safety
/// Tables and serialized map/pools remain live for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_check_docking(
    s: *const CommandServicesTable,
    tile: u32,
) {
    unsafe { Ctx::new(s) }.check_docking(tile);
}
/// # Safety
/// Tables and caller-owned ignored result slot remain live for this entry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_build_oilrig(
    s: *const CommandServicesTable,
    ret: *mut CommandCost,
    tile: u32,
) {
    let c = unsafe { Ctx::new(s) };
    c.build_oilrig(c.c(ret), tile);
}
/// # Safety
/// Tables remain live; the canonical oilrig shell is destroyed during this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_delete_oilrig(
    s: *const CommandServicesTable,
    tile: u32,
) {
    unsafe { Ctx::new(s) }.delete_oilrig(tile);
}
/// # Safety
/// Tables and order/vehicle pools remain live under the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_in_orders_abi(
    s: *const CommandServicesTable,
    station: u16,
    include_company: bool,
    company: u8,
) -> u8 {
    u8::from(unsafe { Ctx::new(s) }.in_orders(station, include_company, company))
}

impl Ctx<'_> {
    fn auto_clear(&self, cost: Cost<'_>, tile: u32) {
        let error = match (self.shared.station_type)(tile).0 {
            0 => self.s.strings.demolish_railroad,
            7 | 8 => self.s.strings.demolish_building,
            1 => self.s.strings.demolish_airport,
            2 => {
                if self.road_type(tile, true) != 63 {
                    self.s.strings.demolish_cargo_tram
                } else {
                    self.s.strings.demolish_truck
                }
            }
            3 => {
                if self.road_type(tile, true) != 63 {
                    self.s.strings.demolish_passenger_tram
                } else {
                    self.s.strings.demolish_bus
                }
            }
            6 => self.s.strings.buoy_in_way,
            5 => self.s.strings.demolish_dock,
            4 => {
                (self.cost.error_param)(
                    cost.0,
                    self.s.strings.object_in_way,
                    u64::from(self.s.strings.oilrig_name),
                );
                return;
            }
            _ => 0xffff,
        };
        cost.error(error);
    }
    fn can_remove_road_stop(&self, cost: Cost<'_>, tile: u32, flags: u16) {
        if (self.shared.current_company)() == OWNER_WATER {
            return cost.zero();
        }
        if self.road_type(tile, true) != 63 {
            let owner = self.road_owner(tile, true);
            if owner != OWNER_NONE {
                unsafe {
                    self.command.ownership(cost.0, owner, 0);
                }
                if !cost.ok() {
                    return;
                }
            }
        }
        if self.road_type(tile, false) != 63 {
            let owner = self.road_owner(tile, false);
            if owner == OWNER_TOWN {
                let bits = (self.shared.road_bits)(tile, 0, false);
                unsafe {
                    crate::road_cmd::openttd_rust_road_allow_remove(
                        (self.shared.road_commands)(),
                        cost.0,
                        tile,
                        bits,
                        OWNER_TOWN,
                        0,
                        flags,
                        true,
                    );
                }
                if !cost.ok() {
                    return;
                }
            } else if owner != OWNER_NONE {
                unsafe {
                    self.command.ownership(cost.0, owner, 0);
                }
                if !cost.ok() {
                    return;
                }
            }
        }
        cost.zero();
    }
    fn clear(&self, cost: Cost<'_>, ret: Cost<'_>, error: Cost<'_>, tile: u32, flags: u16) {
        crate::witness::hit(crate::witness::STATION_CMD_CLEAR);
        if flags & DoCommandFlag::AUTO != 0 {
            return self.auto_clear(cost, tile);
        }
        match (self.shared.station_type)(tile).0 {
            0 => self.remove_rail_all(cost, ret, error, flags, tile, false),
            7 => self.remove_rail_all(cost, ret, error, flags, tile, true),
            1 => self.remove_airport(cost, ret, tile, flags),
            2 | 3 => {
                if crate::map_func::World::new(self.shared).drive_through_stop_tile(tile) {
                    self.can_remove_road_stop(cost, tile, flags);
                    if !cost.ok() {
                        return;
                    }
                }
                self.remove_road_tile(cost, ret, tile, flags, -1, false);
            }
            8 => {
                self.can_remove_road_stop(cost, tile, flags);
                if cost.ok() {
                    self.remove_road_tile(cost, ret, tile, flags, -1, true);
                }
            }
            6 => self.remove_buoy_raw(cost, tile, flags),
            5 => self.remove_dock(cost, ret, tile, flags),
            _ => cost.error(INVALID_STRING_ID),
        }
    }
    fn terraform(&self, cost: Cost<'_>, tile: u32, flags: u16, z: i32, slope: u8) {
        crate::witness::hit(crate::witness::STATION_CMD_TERRAFORM);
        if (self.shared.build_on_slopes)()
            && (self.shared.autoslope)()
            && (self.shared.tile_max_z)(tile) == z.wrapping_add(get_slope_max_z(slope))
        {
            let allowed = match (self.shared.station_type)(tile).0 {
                0 | 7 => (self.s.autoslope_axis)(tile, z, slope, self.station_axis(tile)),
                1 => true,
                2 | 3 | 8 => {
                    if crate::map_func::World::new(self.shared).drive_through_stop_tile(tile) {
                        (self.s.autoslope_axis)(
                            tile,
                            z,
                            slope,
                            crate::map_func::World::new(self.shared).drive_through_axis(tile),
                        )
                    } else {
                        (self.rail().entrance_edge)(
                            tile,
                            z,
                            slope,
                            crate::map_func::World::new(self.shared).bay_dir(tile),
                        )
                    }
                }
                _ => false,
            };
            if allowed {
                return cost.set(crate::company::price(usize::from(
                    Price::PR_BUILD_FOUNDATION.0,
                )));
            }
        }
        crate::landscape_cmd::clear_command(self.command, cost.0, tile, flags);
    }
    fn change_owner(&self, context: StationOwnerChange<'_>) {
        let StationOwnerChange {
            ownership,
            cost,
            ret,
            error,
            tile,
            old,
            new,
        } = context;
        crate::witness::hit(crate::witness::STATION_CMD_CHANGE_OWNER);
        if crate::map_func::World::new(self.shared).any_road_stop_tile(tile) {
            for tram in [false, true] {
                if self.road_owner(tile, tram) == old {
                    let rt = self.road_type(tile, tram);
                    if rt != 63 {
                        (self.rail().road_infrastructure)(old, rt, u32::MAX - 1);
                        if new != 255 {
                            (self.rail().road_infrastructure)(new, rt, 2);
                        }
                    }
                    (self.shared.set_road_owner)(
                        tile,
                        u8::from(tram),
                        if new == INVALID_OWNER {
                            OWNER_NONE
                        } else {
                            new
                        },
                    );
                }
            }
        }
        if (self.shared.tile_owner)(tile) != old {
            return;
        }
        if new != 255 {
            let kind = (self.shared.station_type)(tile).0;
            match kind {
                0 | 7 => {
                    if !crate::map_func::World::new(self.shared).station_blocked(tile) {
                        let rt = crate::services::RailType(
                            crate::map_func::World::new(self.shared).rail_type(tile),
                        )
                        .0;
                        (self.rail().rail_infrastructure)(old, rt, u32::MAX);
                        (self.rail().rail_infrastructure)(new, rt, 1);
                    }
                }
                5 | 6 if crate::map_func::World::new(self.shared).water_class(tile) == 1 => {
                    (self.s.water_infrastructure)(old, u32::MAX);
                    (self.s.water_infrastructure)(new, 1);
                }
                _ => (),
            }
            if kind != StationType::Buoy.0 && kind != StationType::Airport.0 {
                (self.s.station_infrastructure)(old, u32::MAX);
                (self.s.station_infrastructure)(new, 1);
            }
            (self.shared.set_tile_owner)(tile, new);
            (self.s.station_list_classes_invalidate)();
        } else if crate::map_func::World::new(self.shared).drive_through_stop_tile(tile) {
            let kind = (self.shared.station_type)(tile).0;
            self.nested(
                cost,
                tile,
                DoCommandFlag::EXECUTE | DoCommandFlag::BANKRUPT,
                |flags| {
                    self.remove_road_command(
                        cost,
                        ret,
                        error,
                        flags,
                        tile,
                        tile,
                        1,
                        1,
                        u8::from(kind == StationType::Truck.0),
                        false,
                        kind == StationType::RoadWaypoint.0,
                    );
                },
            );
            assert_eq!((self.shared.tile_type)(tile), MP_ROAD);
            // SAFETY: Nested removal has returned; the native slots can be reused.
            unsafe { crate::tile_owner::openttd_rust_tile_change_owner(ownership, tile, old, new) };
        } else {
            crate::landscape_cmd::clear_command(self.command, cost.0, tile, 33);
            let kind = (self.shared.tile_type)(tile);
            if (kind == MP_WATER
                || (kind == MP_STATION && (self.shared.station_type)(tile) == StationType::Buoy))
                && (self.shared.tile_owner)(tile) == old
            {
                (self.shared.set_tile_owner)(tile, OWNER_NONE);
            }
        }
    }
}
/// # Safety
/// Tables, three distinct native result slots and serialized map/pools remain live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_clear(
    s: *const CommandServicesTable,
    cost: *mut CommandCost,
    ret: *mut CommandCost,
    error: *mut CommandCost,
    tile: u32,
    flags: u16,
) {
    let c = unsafe { Ctx::new(s) };
    c.clear(c.c(cost), c.c(ret), c.c(error), tile, flags);
}
/// # Safety
/// Tables and native output slot remain live for the serialized tile proc.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_terraform(
    s: *const CommandServicesTable,
    cost: *mut CommandCost,
    tile: u32,
    flags: u16,
    z: i32,
    slope: u8,
) {
    let c = unsafe { Ctx::new(s) };
    c.terraform(c.c(cost), tile, flags, z, slope);
}
/// # Safety
/// Tables, native result slots and serialized map/pools remain live.
pub(crate) unsafe fn openttd_rust_station_change_owner(
    s: *const CommandServicesTable,
    ownership: &crate::tile_owner::Context,
    tile: u32,
    old: u8,
    new: u8,
) {
    let c = unsafe { Ctx::new(s) };
    c.change_owner(StationOwnerChange {
        ownership,
        cost: c.c(ownership.cost),
        ret: c.c(ownership.ret),
        error: c.c(ownership.error),
        tile,
        old,
        new,
    });
}

impl Ctx<'_> {
    fn expand_rail(&self, out: Cost<'_>, st: *const BaseStation, new_area: TileArea) -> TileArea {
        let old = self.s.rail_area(st);
        let x = self
            .map()
            .tile_x(old.tile)
            .min(self.map().tile_x(new_area.tile));
        let y = self
            .map()
            .tile_y(old.tile)
            .min(self.map().tile_y(new_area.tile));
        let area = TileArea {
            tile: (y << self.map().log_x) + x,
            w: (self
                .map()
                .tile_x(old.tile)
                .wrapping_add(u32::from(old.w))
                .max(
                    self.map()
                        .tile_x(new_area.tile)
                        .wrapping_add(u32::from(new_area.w)),
                )
                .wrapping_sub(x)) as u16,
            h: (self
                .map()
                .tile_y(old.tile)
                .wrapping_add(u32::from(old.h))
                .max(
                    self.map()
                        .tile_y(new_area.tile)
                        .wrapping_add(u32::from(new_area.h)),
                )
                .wrapping_sub(y)) as u16,
        };
        let spread = (self.s.spread)();
        if area.w > u16::from(spread) || area.h > u16::from(spread) {
            out.error(self.s.strings.spread);
        } else {
            out.zero();
        }
        area
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDStationCommandServices" ["probe"] => CommandServicesTable {
        shared,
        station,
        rail,
        cost,
        command,
        authority,
        strings,
        spread,
        default_bridge_info,
        rail_bridge_info,
        road_bridge_info,
        southern_bridge_end,
        tile_area,
        rail_spec,
        links,
        location,
        base_move_sign,
        waypoint_move_sign,
        virtual_coord,
        catchment_dirty,
        facility_widget,
        select_invalidate,
        station_available,
        new_station,
        set_town,
        lifecycle,
        have_rating,
        road_waypoint,
        name_equals,
        set_name,
        flip_airport_closed,
        closed_widget,
        distant_join,
        rail_association,
        rail_spec_info,
        rail_layout,
        rail_boolean,
        rail_callback_error,
        set_train_area,
        rail_triggers,
        reserved,
        last_train,
        train,
        reservation,
        compatible_tiles,
        custom_rail_index,
        make_rail_station,
        close_station_inspect,
        set_custom_rail_index,
        set_random_bits,
        set_gfx,

        tile_flag_count,
        tile_flags,
        set_tile_flags,
        station_infrastructure,
        mark_tiles,
        reset_facilities,
        orders_dirty,
        vehicle,
        stop_on_town_road,
        stop_on_competitor_road,
        road_spec_info,
        road_spec_build_cost,
        road_spec_clear_cost,
        stops_available,
        roadstop_resolver,
        road_boolean,
        road_triggers,
        new_stop,
        set_stop_head,
        set_stop_next,
        stop_area,
        clear_stop_area,
        make_drive_stop,
        make_bay_stop,
        delete_stop,
        set_custom_road_index,
        custom_road_index,

        remove_stop_tile_data,
        close_stop_inspect,
        set_road_waypoint_area,
        road_vehicle_owner,
        order_station_location,
        aircraft,
        airport_runtime,
        airport_layout_count,
        airport_size,
        airport_layout,
        airport_layout_tile,
        iterator_tile,
        iterator_next,
        airport_noise,
        town_max_noise,
        station_town,
        airport_tile,
        airport_type,
        airport_rotation,
        airport_airplanes,
        set_airport_details,
        make_airport,
        airport_area,
        airport_animated,

        airport_infrastructure,
        station_view_invalidate,
        town_view_dirty,
        aircraft_vehicle,
        hangar_count,
        hangar_tile,
        close_hangar,
        close_airport_tile_inspect,
        delete_airport_storage,
        clear_airport,
        close_airport_inspect,
        ship,
        clear,
        restoration,
        set_docking,
        set_ship_area,
        ship_station_area,
        docking_clear,
        docking_area,
        water_infrastructure,
        make_dock,
        neutral_for_tile,
        associate_industry,
        make_oilrig,
        set_station_owner,
        set_airport_kind_rotation,
        set_facilities,
        set_build_date,
        associated_industry,
        industry_neutral,
        clear_industry_neutral,
        industry_area,
        autoslope_axis,
        station_list_classes_invalidate,
        clear_cached_name,
        clear_name,
        make_buoy,
        make_rail_waypoint,
        new_waypoint,
        road_ground,
        road_waypoint_tile_area,
        set_facility_bits,
        set_location,
        set_rail_station_reservation,
        set_road_waypoint,
        set_string_id,
        string_id,
        tile_flat,
        waypoint_available,
        waypoint_default_name,
        waypoint_layout_gfx,
        waypoint_layout_init,
        waypoint_view_invalidate,
    };
    "OpenTTDStationCommandStrings" ["probe"] => Strings {
        demolish_bridge,
        flat_land,
        spread,
        bridge_too_low,
        too_many_stations,
        adjoins_multiple,
        remove_station,
        remove_rail_waypoint,
        remove_road_waypoint,
        remove_road_stop,
        name_unique,
        unsuitable,
        too_many_specs,
        disallowed_tracks,
        disallowed_length,
        remove_signals,
        no_station,
        drive_direction,
        drive_corner,
        drive_junction,
        drive_town,
        no_road,
        unsuitable_road,
        too_many_truck,
        too_many_bus,
        refuse_noise,
        refuse_airport,
        close_airport,
        aircraft_in_way,
        demolish_railroad,
        demolish_building,
        demolish_airport,
        demolish_cargo_tram,
        demolish_truck,
        demolish_passenger_tram,
        demolish_bus,
        buoy_in_way,
        demolish_dock,
        object_in_way,
        oilrig_name,
        buoy_in_use,
        buoy_name,
        waypoint_adjoins_multiple,
        waypoint_close,
        waypoint_name,
    };
    "OpenTTDStationArea" ["probe"] => TileArea {
        tile,
        w,
        h,
    };
    "BridgeableTileInfo" ["probe"] => BridgeInfo {
        height,
        pillars => disallowed_pillars,
    };
    "OpenTTDStationBridgeInfo" ["probe"] => BridgeSpan {
        data,
        size,
    };
    "OpenTTDStationSpecIndex" ["probe"] => SpecIndex {
        value,
        valid,
    };
    "OpenTTDRailStationSpecInfo" ["probe"] => RailSpecInfo {
        platforms,
        lengths,
        slope_callback,
        availability_callback,
    };
    "OpenTTDRoadStationSpecCost" ["probe"] => RoadSpecCost {
        multiplier,
        shift,
    };
    "OpenTTDRoadStationSpecInfo" ["probe"] => RoadSpecInfo {
        stop_type,
        drive_only,
        availability_callback,
    };
    "OpenTTDAirportSize" ["probe"] => AirportSize {
        w,
        h,
    };
    "OpenTTDAirportLayout" ["probe"] => AirportLayout {
        size,
        rotation,
    };
    "OpenTTDAirportLayoutTile" ["probe"] => AirportTile {
        tile,
        gfx,
    };
}
