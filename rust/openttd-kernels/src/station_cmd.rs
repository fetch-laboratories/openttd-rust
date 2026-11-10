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
use crate::airport::{AT_OILRIG, FLYING, NUM_AIRPORTS};
use crate::base_station_base::StationRectMode;
use crate::bitmath_func::find_first_bit;
use crate::bitmath_func::{gb, has_bit};
use crate::cargo_type::NUM_CARGO;
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company_type::CompanyID;
use crate::debug::debug;
use crate::depot_func::can_build_depot_by_tileh;
use crate::direction_func::{axis_to_diag_dir, is_valid_axis, other_axis};
use crate::direction_func::{diag_dir_to_axis, is_valid_diag_direction, reverse_diag_dir};
use crate::direction_type::Direction;
use crate::direction_type::{Axis, DiagDirection, DiagDirections};
use crate::economy_type::Price;
use crate::economy_type::ROAD_STOP_TRACKBIT_FACTOR;
use crate::newgrf_callbacks::{CALLBACK_FAILED, CallbackID};
use crate::newgrf_roadstop::RoadStopClassID;
use crate::newgrf_station::StationClassID;
use crate::order_type::OrderType;
use crate::rail_type::RailType;
use crate::road::{road_type_is_road, road_type_is_tram};
use crate::road_type::{DisallowedRoadDirections, RoadBits, RoadTramType, RoadType};
use crate::slope_type::Slope;
use crate::station_type::StationFacilities;
use crate::station_type::{StationID, StationType};
use crate::strings_type::StringID;
use crate::tile_type::{TileIndex, TileType};
use crate::tilearea::TileArea;
use crate::town::TownCouncilAttitudes;
use crate::track_func::reverse_trackdir;
use crate::track_type::{Track, TrackBits, Trackdir};
use crate::vehicle_type::VehicleType;
use crate::viewport_func::TILE_HEIGHT_STEP;
use crate::water_map::WaterClass;
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
    pub last_train:
        unsafe extern "C" fn(*mut crate::services::Train) -> *mut crate::services::Train,
    pub train: *const crate::train::Leaves,
    pub reservation: *const crate::train_reservation::Leaves,
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

/// `CheckFlatLandRailStation`'s arguments.
struct RailStationLandCheck<'a> {
    cost: Cost<'a>,
    ret: Cost<'a>,
    /// Tile to check.
    tile_cur: TileIndex,
    /// North tile of the area being checked.
    north_tile: TileIndex,
    /// Height allowed for the tile; set to this tile's height if negative.
    allowed_z: &'a mut i32,
    flags: DoCommandFlags,
    /// Rail station axis.
    axis: Axis,
    /// `StationID` to be queried and returned if available.
    station: &'a mut StationID,
    /// The rail type to check for (overbuilding rail stations over rail).
    rt: RailType,
    /// List of trains with PBS reservations on the tiles.
    affected_vehicles: &'a mut Vec<*mut crate::services::Train>,
    statspec: *const RailSpec,
    /// Platform length.
    plat_len: u8,
    /// Number of platforms.
    numtracks: u8,
}

/// `CheckFlatLandRailStation`'s inputs for overbuilding a plain rail tile.
struct RailOverbuild<'a> {
    cost: Cost<'a>,
    ret: Cost<'a>,
    tile_cur: TileIndex,
    rt: RailType,
    invalid_dirs: DiagDirections,
    flags: DoCommandFlags,
    affected_vehicles: &'a mut Vec<*mut crate::services::Train>,
}

/// `CalculateRailStationCost`'s arguments.
struct RailStationCost<'a> {
    cost: Cost<'a>,
    ret: Cost<'a>,
    /// Area to check.
    tile_area: TileArea,
    flags: DoCommandFlags,
    /// Rail station axis.
    axis: Axis,
    /// `StationID` to be queried and returned if available.
    station: &'a mut StationID,
    /// The rail type to check for (overbuilding rail stations over rail).
    rt: RailType,
    /// List of trains with PBS reservations on the tiles.
    affected_vehicles: &'a mut Vec<*mut crate::services::Train>,
    statspec: *const RailSpec,
    /// Platform length.
    plat_len: u8,
    /// Number of platforms.
    numtracks: u8,
}

/// `CheckFlatLandRoadStop`'s arguments.
struct RoadStopLandCheck<'a> {
    cost: Cost<'a>,
    ret: Cost<'a>,
    /// Tile to check.
    cur_tile: TileIndex,
    /// Height allowed for the tile; set to this tile's height if negative.
    allowed_z: &'a mut i32,
    /// Spec of road stop to be built.
    spec: *const RoadSpec,
    flags: DoCommandFlags,
    /// Prohibited directions.
    invalid_dirs: DiagDirections,
    /// True if trying to build a drive-through station.
    is_drive_through: bool,
    /// Station type (bus, truck or road waypoint).
    station_type: StationType,
    /// Axis of a drive-through road stop.
    axis: Axis,
    /// `StationID` to be queried and returned if available.
    station: &'a mut StationID,
    /// Road type to build, may be `INVALID_ROADTYPE` if an existing road is required.
    rt: RoadType,
}

/// `CalculateRoadStopCost`'s arguments.
struct RoadStopCost<'a> {
    cost: Cost<'a>,
    ret: Cost<'a>,
    /// Area to check.
    tile_area: TileArea,
    flags: DoCommandFlags,
    /// True if trying to build a drive-through station.
    is_drive_through: bool,
    /// Station type (bus, truck or road waypoint).
    station_type: StationType,
    /// Spec of road stop being built.
    roadstopspec: *const RoadSpec,
    /// Axis of a drive-through road stop.
    axis: Axis,
    /// Entrance direction for normal stops.
    ddir: DiagDirection,
    /// `StationID` to be queried and returned if available.
    est: &'a mut StationID,
    /// Road type to build, may be `INVALID_ROADTYPE` if an existing road is required.
    rt: RoadType,
    /// The cost to build one road stop of the current type.
    unit_cost: i64,
}

/// `RemoveFromRailBaseStation`'s arguments.
#[derive(Clone, Copy)]
struct RailStationRemoval<'a> {
    total_cost: Cost<'a>,
    ret: Cost<'a>,
    error: Cost<'a>,
    /// The area to clear station tile from.
    ta: TileArea,
    flags: DoCommandFlags,
    /// The cost for removing the tile, including the rail.
    removal_cost: i64,
    /// Whether to keep the rail of the station.
    keep_rail: bool,
    /// Whether to remove from waypoints rather than stations.
    waypoint: bool,
}

/// `RemoveGenericRoadStop`'s arguments.
#[derive(Clone, Copy)]
struct RoadStopRemoval<'a> {
    cost: Cost<'a>,
    ret: Cost<'a>,
    last_error: Cost<'a>,
    flags: DoCommandFlags,
    /// Tile area of road stop or road waypoint tiles to remove.
    roadstop_area: TileArea,
    /// Whether to remove road waypoints or road stops.
    road_waypoint: bool,
    /// Remove roads of drive-through stops?
    remove_road: bool,
}

/// `ChangeTileOwner_Station`'s arguments.
#[derive(Clone, Copy)]
struct StationOwnerChange<'a> {
    ownership: &'a crate::tile_owner::Context,
    cost: Cost<'a>,
    ret: Cost<'a>,
    error: Cost<'a>,
    tile: TileIndex,
    old_owner: CompanyID,
    new_owner: CompanyID,
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
        // SAFETY: The live Rust-owned rectangle, updated in place as the original
        // does. Its callbacks (the spread setting, map reads and the native
        // assertion) never access a station rectangle.
        let ok = unsafe { &mut *self.rect(st) }.before_add_rect(
            TileIndex(area.tile),
            i32::from(area.w),
            i32::from(area.h),
            mode,
            &c,
        );
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
        // SAFETY: As in `before_add_rect`.
        let ok = unsafe { &mut *self.rect(st) }.before_add_tile(tile, mode, &c);
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
        // SAFETY: As in `before_add_rect`; the map scans read only tile types and
        // station indices.
        unsafe { &mut *self.rect(st) }.after_remove_tile(id, tile, &c);
    }
    /// `st->rect.AfterRemoveRect(st, area)`.
    fn after_remove_rect(&self, st: *mut BaseStation, area: TileArea) {
        let c = self.catchment();
        let id = (c.c.id)(st);
        // SAFETY: As in `after_remove_tile`.
        unsafe { &mut *self.rect(st) }.after_remove_rect(id, area, &c);
    }
    /// `st->rect.PtInExtendedRect(TileX(tile), TileY(tile))`.
    #[allow(clippy::cast_possible_wrap)]
    fn rect_contains(&self, st: *mut BaseStation, tile: TileIndex) -> bool {
        let map = self.map();
        let (x, y) = (map.tile_x(tile.0) as i32, map.tile_y(tile.0) as i32);
        // SAFETY: Live owner; the test reads its fields in place without a callback.
        unsafe { &*self.rect(st) }.pt_in_extended_rect(x, y, 0)
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
        if (!allow_steep && is_steep_slope(tileh))
            || (!(self.shared.build_on_slopes)() && tileh != Slope::SLOPE_FLAT)
        {
            return out.error(self.s.strings.flat_land);
        }
        out.set(0);
        let flat_z = z + get_slope_max_z(tileh);
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
            let height_diff = ((self.shared.tile_max_z)(tile.0) + i32::from(height)
                - bridge_height)
                * TILE_HEIGHT_STEP;
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
    c.is_road_station_bridge_above_ok(
        c.c(out),
        TileIndex(tile),
        spec,
        StationType(kind),
        StationGfx(layout),
    );
}

/// `GetAirportNoiseLevelForDistance`.
/// # Safety
/// The immutable native table, authority table and airport spec are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_airport_noise(
    s: *const CommandServicesTable,
    spec: *const AirportSpec,
    distance: u32,
) -> u8 {
    unsafe { Ctx::new(s) }.get_airport_noise_level_for_distance(spec, distance)
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
    c.check_build_above_station(c.c(out), TileIndex(tile), height);
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
        let r = unsafe { crate::station::rect(owner) };
        /* clamp sign coord to be inside the station rect */
        let xy = (self.s.location)(st);
        // SAFETY: Field-sized reads of the live rectangle at the clamp; no callback
        // runs between them.
        let new_xy = TileIndex(self.map().tile_xy(
            (self.map().tile_x(xy) as i32).clamp(unsafe { (*r).left }, unsafe { (*r).right })
                as u32,
            (self.map().tile_y(xy) as i32).clamp(unsafe { (*r).top }, unsafe { (*r).bottom })
                as u32,
        ));
        crate::station::spatial::station_move_sign(self.shared, st, new_xy);
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
            let fields = unsafe { crate::station_service::fields(owner) };
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
                crate::station_service::update_station_acceptance(st, false, &*self.s.station);
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
            ta.expand(
                1,
                self.map(),
                crate::map_func::World::new(self.shared).size_y(),
            );
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
        crate::station::spatial::for_all_stations_radius(self.shared, tile, threshold, |st| {
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
                let town = crate::town::spatial::closest_town_from_tile(
                    self.shared,
                    TileIndex(area.tile),
                    u32::MAX,
                );
                (self.s.set_town)(st, town);
                (self.s.set_string_id)(
                    st,
                    crate::station_lifecycle::generate_station_name(
                        st,
                        TileIndex(area.tile),
                        name_class,
                        unsafe { &*self.s.lifecycle },
                    )
                    .0,
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
            tile,
            other: false,
        };
        /* Check if the tile is the base tile of another station */
        crate::station::spatial::for_all_stations_radius(self.shared, tile, 0, |st| unsafe {
            sign_visit((&raw mut search).cast(), st);
        });
        if search.other {
            out.error(self.s.strings.unsuitable);
            return StationID::INVALID;
        }
        if flags.test(DoCommandFlag::Execute) {
            crate::station::spatial::station_move_sign(self.shared, st, tile);
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
    /// `BaseStation::TileBelongsToRailStation` for a station or waypoint shell.
    fn tile_belongs_to_rail_station(&self, st: *mut BaseStation, tile: TileIndex) -> bool {
        crate::station_base::tile_belongs_to_rail_station(
            self.shared,
            !(self.world().is_station)(st),
            StationID((self.links().graphs.station_id)(st)),
            tile,
        )
    }
    /// `TileBelongsToShipStation`.
    fn tile_belongs_to_ship_station(&self, st: *mut BaseStation, tile: TileIndex) -> bool {
        crate::map_func::World::new(self.shared).is_dock_tile(tile)
            && StationID((self.shared.station_index)(tile.0))
                == StationID((self.links().graphs.station_id)(st))
    }
    /// `TileBelongsToRoadWaypointStation`.
    fn tile_belongs_to_road_waypoint_station(&self, st: *mut BaseStation, tile: TileIndex) -> bool {
        crate::station_map::is_road_waypoint_tile(tile, self.shared)
            && StationID((self.shared.station_index)(tile.0))
                == StationID((self.links().graphs.station_id)(st))
    }
    /// `MakeStationAreaSmaller`: shrink `ta` while one of its edges has no
    /// tile for which `func` holds.
    fn make_station_area_smaller(
        &self,
        mut ta: TileArea,
        func: impl Fn(TileIndex) -> bool,
    ) -> TileArea {
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
                    if func(TileIndex(tile)) {
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
/// `CmdMoveStationName` and `CmdMoveWaypointName`'s check for another
/// station whose base tile is `tile`.
struct SignSearch<'a> {
    c: &'a Ctx<'a>,
    station: *mut BaseStation,
    tile: TileIndex,
    other: bool,
}
unsafe extern "C" fn sign_visit(context: *mut std::ffi::c_void, st: *mut BaseStation) {
    let s = unsafe { &mut *context.cast::<SignSearch<'_>>() };
    if !st.is_null() && st != s.station && TileIndex((s.c.s.location)(st)) == s.tile {
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

/// `CmdBuildRailStation`'s arguments.
#[derive(Clone, Copy)]
struct CmdBuildRailStation {
    /// Northern most position of station dragging/placement.
    tile_org: TileIndex,
    /// Railtype.
    rt: RailType,
    /// Orientation (Axis).
    axis: Axis,
    /// Number of tracks.
    numtracks: u8,
    /// Platform length.
    plat_len: u8,
    /// Custom station class.
    spec_class: StationClassID,
    /// Custom station id.
    spec_index: u16,
    /// Station ID to join (`NEW_STATION` if build new one).
    station_to_join: StationID,
    /// Allow stations directly adjacent to other stations.
    adjacent: bool,
}

impl Ctx<'_> {
    fn rail(&self) -> &RailCommandServices {
        unsafe { &*self.s.rail }
    }
    fn rail_map(&self) -> &crate::services::RailMapServices {
        unsafe { &*self.rail().rail }
    }
    /// `CheckIfAuthorityAllowsNewStation`.
    fn check_if_authority_allows_new_station(
        &self,
        out: Cost<'_>,
        tile: TileIndex,
        flags: DoCommandFlags,
    ) {
        unsafe {
            crate::town::check_if_authority_allows_new_station(
                &*self.s.authority,
                &*self.s.shared,
                &*self.s.cost,
                out.0,
                tile,
                flags,
            );
        }
    }
    /// `IsRailStationTile`.
    fn is_rail_station_tile(&self, tile: TileIndex) -> bool {
        crate::map_func::World::new(self.shared).is_rail_station_tile(tile)
    }
    /// `GetRailStationAxis`.
    fn get_rail_station_axis(&self, tile: TileIndex) -> Axis {
        crate::map_func::World::new(self.shared).get_rail_station_axis(tile)
    }
    /// `HasStationReservation`.
    fn has_station_reservation(&self, tile: TileIndex) -> bool {
        crate::map_func::World::new(self.shared).has_station_reservation(tile)
    }
    /// `IsStationTileBlocked`.
    fn is_station_tile_blocked(&self, tile: TileIndex) -> bool {
        crate::map_func::World::new(self.shared).is_station_tile_blocked(tile)
    }
    /// `!IsValidTile(tile) || TileAddWrap(tile, w - 1, h - 1) == INVALID_TILE`
    /// negated: whether the first tile and the last tile are valid.
    fn first_and_last_tile_valid(&self, tile: TileIndex, w: u16, h: u16) -> bool {
        (self.shared.tile_valid)(tile.0)
            && self.map().tile_add_wrap(
                tile.0,
                i32::from(w) - 1,
                i32::from(h) - 1,
                crate::map_func::World::new(self.shared).size_y(),
                (self.world().environment.freeform_edges)(),
            ) != TileIndex::INVALID.0
    }
    /// `FreeTrainReservation`: clear platform reservation during station
    /// building/removing.
    fn free_train_reservation(&self, v: *mut crate::services::Train) {
        unsafe {
            let handle = crate::pbs::Pbs::acquire(self.shared).handle(v);
            crate::train_reservation::openttd_rust_train_reservation_free(
                handle,
                handle.vehicle(),
                self.s.reservation,
                std::ptr::null_mut(),
            );
        }
        self.set_platform_reservation_at(v, false, false);
        let v = unsafe { (self.s.last_train)(v) };
        self.set_platform_reservation_at(v, true, false);
    }
    /// `RestoreTrainReservation`: restore platform reservation during station
    /// building/removing.
    fn restore_train_reservation(&self, v: *mut crate::services::Train) {
        self.set_platform_reservation_at(v, false, true);
        unsafe {
            let handle = crate::pbs::Pbs::acquire(self.shared).handle(v);
            crate::train_reservation::openttd_rust_train_reservation_try_path(
                handle,
                handle.vehicle(),
                1,
                1,
                self.s.reservation,
                std::ptr::null_mut(),
            );
        }
        let v = unsafe { (self.s.last_train)(v) };
        self.set_platform_reservation_at(v, true, true);
    }
    /// `if (IsRailStationTile(v->tile)) SetRailStationPlatformReservation(v->tile,
    /// TrackdirToExitdir(td), b)`, with `td` the vehicle trackdir, reversed when
    /// `reverse`.
    fn set_platform_reservation_at(&self, v: *mut crate::services::Train, reverse: bool, b: bool) {
        let handle = crate::pbs::Pbs::acquire(self.shared).handle(v);
        // SAFETY: The reached platform update always supplies a live train shell.
        let vehicle = handle.vehicle();
        let tile = TileIndex(vehicle.tile());
        if self.is_rail_station_tile(tile) {
            // SAFETY: The shell and canonical immutable tables remain live for this call.
            let td = Trackdir(unsafe {
                crate::train::openttd_rust_train_trackdir(handle, self.s.train, self.s.shared)
            });
            let td = if reverse {
                Trackdir(reverse_trackdir(td.0))
            } else {
                td
            };
            crate::pbs::Pbs::acquire(self.shared).platform(
                tile.0,
                crate::track_func::trackdir_to_exitdir(td.0),
                b,
            );
        }
    }
    /// `CheckFlatLandRailStation`'s overbuilding of an existing plain rail
    /// tile with a valid railtype; true when the check is complete.
    fn overbuild_rail_tile(&self, context: RailOverbuild<'_>) -> bool {
        let RailOverbuild {
            cost,
            ret,
            tile_cur,
            rt,
            invalid_dirs,
            flags,
            affected_vehicles,
        } = context;
        if rt != RailType((self.rail_map().invalid_rail_type)().0)
            && crate::map_func::World::new(self.shared).plain_rail(tile_cur.0)
        {
            /* Don't overbuild signals. */
            if crate::map_func::World::new(self.shared).has_signals(tile_cur.0) {
                cost.error(self.s.strings.remove_signals);
                return true;
            }
            /* The current rail type must have power on the to-be-built type (e.g. convert normal rail to electrified rail). */
            if (self.rail().powered_rail)(
                crate::map_func::World::new(self.shared).rail_type(tile_cur.0),
                rt.0,
            ) {
                let mut tracks =
                    TrackBits(crate::map_func::World::new(self.shared).tracks(tile_cur.0));
                let track = crate::track_func::remove_first_track(&mut tracks, |ok| {
                    crate::movement::native_assert(self.shared, ok);
                });
                let expected_track = if invalid_dirs.test(DiagDirection::DIAGDIR_NE) {
                    Track::TRACK_X
                } else {
                    Track::TRACK_Y
                };
                /* The existing track must align with the desired station axis. */
                if tracks == TrackBits::TRACK_BIT_NONE && track == expected_track {
                    self.overbuild_rail_track(cost, ret, tile_cur, track, flags, affected_vehicles);
                    /* With DoCommandFlags{flags}.Reset(DoCommandFlag::Execute) CmdLandscapeClear would fail since the rail still exists */
                    return true;
                }
            }
        }
        false
    }
    /// `CheckFlatLandRailStation`'s overbuilding of the one aligned track:
    /// note a train reserving it, then `CMD_REMOVE_SINGLE_RAIL` and add its cost.
    fn overbuild_rail_track(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        tile_cur: TileIndex,
        track: Track,
        flags: DoCommandFlags,
        affected_vehicles: &mut Vec<*mut crate::services::Train>,
    ) {
        /* Check for trains having a reservation for this tile. */
        if has_bit(
            crate::pbs::Pbs::acquire(self.shared).reserved(tile_cur.0),
            track.0,
        ) {
            let v = crate::pbs::Pbs::acquire(self.shared)
                .train_for(tile_cur.0, track.0)
                .shell()
                .cast::<crate::services::Train>();
            if !v.is_null() {
                affected_vehicles.push(v);
            }
        }
        // Reuse the two native slots across the nested command;
        // the accumulated station cost is scalar and already successful.
        let saved = cost.money();
        self.nested(ret, tile_cur, flags, |flags| unsafe {
            crate::rail_cmd::openttd_rust_rail_remove_single(
                self.s.rail,
                ret.0,
                cost.0,
                flags.0,
                tile_cur.0,
                track.0,
            );
        });
        cost.set(saved);
        if !ret.ok() {
            return cost.take(ret);
        }
        cost.add_money(ret.money());
    }
    /// `CheckFlatLandRailStation`: checks if a rail station can be built at the
    /// given tile.
    fn check_flat_land_rail_station(&self, context: RailStationLandCheck<'_>) {
        let RailStationLandCheck {
            cost,
            ret,
            tile_cur,
            north_tile,
            allowed_z,
            flags,
            axis,
            station,
            rt,
            affected_vehicles,
            statspec,
            plat_len,
            numtracks,
        } = context;
        let invalid_dirs = crate::direction_func::axis_to_diag_dirs(axis, |ok| {
            crate::movement::native_assert(self.shared, ok);
        });
        let slope_cb = !statspec.is_null() && (self.s.rail_spec_info)(statspec).slope_callback;
        self.check_buildable(ret, tile_cur, invalid_dirs, allowed_z, false, false);
        if !ret.ok() {
            return cost.take(ret);
        }
        cost.set(0);
        cost.add_money(ret.money());
        if slope_cb {
            /* Do slope check if requested. */
            unsafe {
                crate::newgrf_station::openttd_rust_station_slope_check(
                    ret.0,
                    north_tile.0,
                    tile_cur.0,
                    statspec,
                    axis.0,
                    plat_len,
                    numtracks,
                );
            }
            if !ret.ok() {
                return cost.take(ret);
            }
        }
        /* if station is set, then we have special handling to allow building on top of already existing stations.
         * so station points to StationID::Invalid() if we can build on any station.
         * Or it points to a station if we're only allowed to build on exactly that station. */
        let world = crate::map_func::World::new(self.shared);
        if world.kind(tile_cur.0) == TileType::MP_STATION.0 {
            if !world.is_rail_station(tile_cur) {
                return self.auto_clear(cost, tile_cur); // get error message
            }
            let st = world.get_station_index(tile_cur);
            if *station == StationID::INVALID {
                *station = st;
            } else if *station != st {
                cost.error(self.s.strings.adjoins_multiple);
            }
        } else {
            /* If we are building a station with a valid railtype, we may be able to overbuild an existing rail tile. */
            if self.overbuild_rail_tile(RailOverbuild {
                cost,
                ret,
                tile_cur,
                rt,
                invalid_dirs,
                flags,
                affected_vehicles,
            }) {
                return;
            }
            crate::landscape_cmd::clear_command(self.command, ret.0, tile_cur.0, flags.0);
            if !ret.ok() {
                return cost.take(ret);
            }
            cost.add_money(ret.money());
        }
    }
    /// `CalculateRailStationCost`: calculates cost of new rail stations within
    /// the area; `context.cost` is the accumulated cost.
    fn calculate_rail_station_cost(&self, context: RailStationCost<'_>) {
        let RailStationCost {
            cost,
            ret,
            tile_area,
            flags,
            axis,
            station,
            rt,
            affected_vehicles,
            statspec,
            plat_len,
            numtracks,
        } = context;
        cost.set(0);
        let mut length_price_ready = true;
        let mut tracknum = 0_u8;
        let mut allowed_z = -1;
        for cur_tile in tile_area.tiles(self.map()) {
            let cur_tile = TileIndex(cur_tile);
            /* Clear the land below the station. */
            // CheckFlatLandRailStation uses both native slots; its result is
            // left in `cost`, and the accumulated amount is restored after.
            let saved = cost.money();
            self.check_flat_land_rail_station(RailStationLandCheck {
                cost,
                ret,
                tile_cur: cur_tile,
                north_tile: TileIndex(tile_area.tile),
                allowed_z: &mut allowed_z,
                flags,
                axis,
                station: &mut *station,
                rt,
                affected_vehicles: &mut *affected_vehicles,
                statspec,
                plat_len,
                numtracks,
            });
            if !cost.ok() {
                return;
            }
            let ret_cost = cost.money();
            cost.set(saved);
            /* Only add _price[PR_BUILD_STATION_RAIL_LENGTH] once for each valid plat_len. */
            if tracknum == numtracks {
                length_price_ready = true;
                tracknum = 0;
            } else {
                tracknum = tracknum.wrapping_add(1);
            }
            /* AddCost for new or rotated rail stations. */
            if !self.is_rail_station_tile(cur_tile) || self.get_rail_station_axis(cur_tile) != axis
            {
                cost.add_money(ret_cost);
                cost.add_money(crate::company::price(usize::from(
                    Price::PR_BUILD_STATION_RAIL.0,
                )));
                cost.add_money((self.rail().rail_build_cost)(rt.0));
                if length_price_ready {
                    cost.add_money(crate::company::price(usize::from(
                        Price::PR_BUILD_STATION_RAIL_LENGTH.0,
                    )));
                    length_price_ready = false;
                }
            }
        }
    }
    /// `GetStationTileFlags`: get station tile flags for the given `StationGfx`.
    fn get_station_tile_flags(&self, gfx: StationGfx, statspec: *const RailSpec) -> TileFlags {
        /* Default stations do not draw pylons under roofs (gfx >= 4) */
        if statspec.is_null() || usize::from(gfx.0) >= (self.s.tile_flag_count)(statspec) {
            return if gfx.0 < 4 {
                TileFlags(1 << TileFlag::Pylons.0)
            } else {
                TileFlags(0)
            };
        }
        TileFlags((self.s.tile_flags)(statspec, gfx.0))
    }
    /// `SetRailStationTileFlags`: set rail station tile flags for the given tile.
    fn set_rail_station_tile_flags(&self, tile: TileIndex, statspec: *const RailSpec) {
        let flags = self.get_station_tile_flags(
            crate::map_func::World::new(self.shared).get_station_gfx(tile),
            statspec,
        );
        (self.s.set_tile_flags)(
            tile.0,
            flags.test(TileFlag::Blocked),
            flags.test(TileFlag::Pylons),
            !flags.test(TileFlag::NoWires),
        );
    }
    /// `RailStationTileLayout::Iterator::operator*`: the layout piece at
    /// `position` of a station `platforms` tracks by `length` tiles.
    fn rail_station_tile_layout_gfx(
        layout: crate::services::ByteSpan,
        position: u32,
        platforms: u8,
        length: u8,
    ) -> StationGfx {
        /* Use predefined layout if it exists. Mask bit zero which will indicate axis. */
        if layout.size != 0 {
            return StationGfx(unsafe { layout.data.add(position as usize).read() } & !1);
        }
        let length = u32::from(length);
        let platforms = u32::from(platforms);
        if length == 1 {
            /* Special case for 1-long platforms, all bare platforms except one small building. */
            return StationGfx(if position == (platforms - 1) / 2 {
                2
            } else {
                0
            });
        }
        if position < length && platforms % 2 == 1 {
            /* Number of tracks is odd, make the first platform bare with a small building. */
            return StationGfx(if position == (length - 1) / 2 { 2 } else { 0 });
        }
        if length > 4 && (position.is_multiple_of(length) || position % length == length - 1) {
            /* Station is longer than 4 tiles, place bare platforms at either end. */
            return StationGfx(0);
        }
        /* None of the above so must be north or south part of larger station. */
        StationGfx(if (position / length) % 2 == platforms % 2 {
            4
        } else {
            6
        })
    }
    /// `IsRailStationBridgeAboveOk`: test if a rail station can be built below
    /// a bridge.
    fn is_rail_station_bridge_above_ok(
        &self,
        out: Cost<'_>,
        tile: TileIndex,
        spec: *const RailSpec,
        station_type: StationType,
        layout: StationGfx,
    ) {
        if !crate::map_func::World::new(self.shared).bridge_above(tile.0) {
            return out.zero();
        }
        let rampsouth = TileIndex((self.s.southern_bridge_end)(tile.0));
        let bridgeable_info = if spec.is_null() {
            (self.s.default_bridge_info)(station_type.0)
        } else {
            unsafe { (self.s.rail_bridge_info)(spec) }
        };
        self.is_station_bridge_above_ok(StationBridgeCheck {
            out,
            tile,
            bridgeable_info,
            station_type,
            layout,
            bridge_height: self.bridge_height(rampsouth),
            disallowed_msg: StringID(self.s.strings.demolish_bridge),
        });
    }
    /// `CmdBuildRailStation`'s final step: check whether we need to expand the
    /// reservation of trains already on the station.
    fn expand_platform_reservations(&self, tile_org: TileIndex, axis: Axis, numtracks: u8) {
        let update_reservation_area = if axis == Axis::AXIS_X {
            TileArea {
                tile: tile_org.0,
                w: 1,
                h: u16::from(numtracks),
            }
        } else {
            TileArea {
                tile: tile_org.0,
                w: u16::from(numtracks),
                h: 1,
            }
        };
        let world = crate::map_func::World::new(self.shared);
        for tile in update_reservation_area.tiles(self.map()) {
            let tile = TileIndex(tile);
            /* Don't even try to make eye candy parts reserved. */
            if self.is_station_tile_blocked(tile) {
                continue;
            }
            let dir = axis_to_diag_dir(axis, |ok| {
                crate::movement::native_assert(self.shared, ok);
            });
            let tile_offset = self.map().offs_by_diag_dir(dir.0);
            let mut platform_begin = tile;
            let mut platform_end = tile;
            /* We can only account for tiles that are reachable from this tile, so ignore primarily blocked tiles while finding the platform begin and end. */
            let mut next_tile = TileIndex(platform_begin.0.wrapping_sub(tile_offset));
            while world.is_compatible_train_station_tile(next_tile, platform_begin) {
                platform_begin = next_tile;
                next_tile = TileIndex(next_tile.0.wrapping_sub(tile_offset));
            }
            let mut next_tile = TileIndex(platform_end.0.wrapping_add(tile_offset));
            while world.is_compatible_train_station_tile(next_tile, platform_end) {
                platform_end = next_tile;
                next_tile = TileIndex(next_tile.0.wrapping_add(tile_offset));
            }
            /* If there is at least on reservation on the platform, we reserve the whole platform. */
            let mut reservation = false;
            let mut t = platform_begin;
            while !reservation && t.0 <= platform_end.0 {
                reservation = self.has_station_reservation(t);
                t = TileIndex(t.0.wrapping_add(tile_offset));
            }
            if reservation {
                crate::pbs::Pbs::acquire(self.shared).platform(platform_begin.0, dir.0, true);
            }
        }
    }
    /// `CmdBuildRailStation`: build rail station.
    fn cmd_build_rail_station(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        flags: DoCommandFlags,
        args: CmdBuildRailStation,
    ) {
        let CmdBuildRailStation {
            tile_org,
            rt,
            axis,
            numtracks,
            plat_len,
            spec_class,
            spec_index,
            mut station_to_join,
            adjacent,
        } = args;
        crate::witness::hit(crate::witness::STATION_CMD_BUILD_RAIL);
        /* Does the authority allow this? */
        self.check_if_authority_allows_new_station(cost, tile_org, flags);
        if !cost.ok() {
            return;
        }
        if !crate::rail::val_param_rail_type(self.shared, rt.0) || !is_valid_axis(axis) {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        /* Check if the given station class is valid */
        if u32::from(spec_class.0) >= crate::newgrf_class::station::count()
            || crate::newgrf_station::is_waypoint_class(crate::newgrf_class::station_label(
                spec_class.0,
            ))
            || u32::from(spec_index) >= crate::newgrf_class::station::spec_count(spec_class.0)
            || plat_len == 0
            || numtracks == 0
        {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        let (w_org, h_org) = if axis == Axis::AXIS_X {
            (plat_len, numtracks)
        } else {
            (numtracks, plat_len)
        };
        /* Check if the first tile and the last tile are valid */
        if !self.first_and_last_tile_valid(tile_org, u16::from(w_org), u16::from(h_org)) {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        let reuse = station_to_join != StationID::NEW;
        if !reuse {
            station_to_join = StationID::INVALID;
        }
        let distant_join = station_to_join != StationID::INVALID;
        if distant_join
            && (!(self.s.distant_join)() || self.full_station(station_to_join).is_null())
        {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        let station_spread = (self.s.spread)();
        if h_org > station_spread || w_org > station_spread {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        /* these values are those that will be stored in train_tile and station_platforms */
        let mut new_location = TileArea {
            tile: tile_org.0,
            w: u16::from(w_org),
            h: u16::from(h_org),
        };
        /* Make sure the area below consists of clear tiles. (OR tiles belonging to a certain rail station) */
        let mut est = StationID::INVALID;
        let mut affected_vehicles = Vec::new();
        /* Add construction and clearing expenses. */
        let statspec = crate::newgrf_class::station::spec(spec_class.0, u32::from(spec_index));
        self.calculate_rail_station_cost(RailStationCost {
            cost,
            ret,
            tile_area: new_location,
            flags,
            axis,
            station: &mut est,
            rt,
            affected_vehicles: &mut affected_vehicles,
            statspec,
            plat_len,
            numtracks,
        });
        if !cost.ok() {
            return;
        }
        let st = self.find_joining_base_station(JoiningStation {
            out: ret,
            existing_station: est,
            station_to_join,
            adjacent,
            ta: new_location,
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
            flags,
            reuse,
            new_location,
            StationNaming::STATIONNAMING_RAIL,
        );
        if !ret.ok() {
            return cost.take(ret);
        }
        if !st.is_null() && self.s.rail_area(st).tile != TileIndex::INVALID.0 {
            new_location = self.can_expand_rail_station(ret, st, new_location);
            if !ret.ok() {
                return cost.take(ret);
            }
        }
        let statspec = crate::newgrf_class::station::spec(spec_class.0, u32::from(spec_index));
        let tile_delta = self.map().offs_by_axis(axis.0); // offset to go to the next platform tile
        let track_delta = self.map().offs_by_axis(
            other_axis(axis, |ok| {
                crate::movement::native_assert(self.shared, ok);
            })
            .0,
        ); // offset to go to the next track
        let layout = (self.s.rail_layout)(
            statspec,
            crate::newgrf_station::get_station_layout_key(numtracks, plat_len),
        );
        for i in 0..numtracks {
            let tile_track = TileIndex(
                tile_org
                    .0
                    .wrapping_add(u32::from(i).wrapping_mul(track_delta)),
            );
            for j in 0..plat_len {
                let tile = TileIndex(
                    tile_track
                        .0
                        .wrapping_add(u32::from(j).wrapping_mul(tile_delta)),
                );
                /* Don't check the layout if there's no bridge above anyway. */
                if !crate::map_func::World::new(self.shared).bridge_above(tile.0) {
                    continue;
                }
                let position = u32::from(i) * u32::from(plat_len) + u32::from(j);
                let mut gfx = StationGfx(
                    Self::rail_station_tile_layout_gfx(layout, position, numtracks, plat_len)
                        .0
                        .wrapping_add(axis.0),
                );
                if !statspec.is_null() {
                    let platinfo =
                        crate::newgrf_station::platform_info(gfx.0, plat_len, numtracks, j, i);
                    /* As the station is not yet completely finished, the station does not yet exist. */
                    let callback = crate::newgrf_station::callback(
                        CallbackID::CBID_STATION_BUILD_TILE_LAYOUT.0,
                        platinfo,
                        0,
                        statspec,
                        std::ptr::null_mut(),
                        TileIndex::INVALID.0,
                    );
                    if callback != CALLBACK_FAILED && u8::try_from(callback).is_ok() {
                        gfx = StationGfx((callback as u8 & !1).wrapping_add(axis.0));
                    }
                }
                self.is_rail_station_bridge_above_ok(ret, tile, statspec, StationType::Rail, gfx);
                if !ret.ok() {
                    return cost.take(ret);
                }
            }
        }
        /* Check if we can allocate a custom stationspec to this station */
        let specindex = crate::station_association::allocate(self.s.rail_association, statspec, st);
        if !specindex.valid {
            return cost.error(self.s.strings.too_many_specs);
        }
        if !statspec.is_null() {
            /* Perform NewStation checks */
            let info = (self.s.rail_spec_info)(statspec);
            /* Check if the station size is permitted */
            if has_bit(info.platforms, (numtracks - 1).min(7)) {
                return cost.error(self.s.strings.disallowed_tracks);
            }
            if has_bit(info.lengths, (plat_len - 1).min(7)) {
                return cost.error(self.s.strings.disallowed_length);
            }
            /* Check if the station is buildable */
            if info.availability_callback {
                let cb_res = crate::newgrf_station::callback(
                    CallbackID::CBID_STATION_AVAILABILITY.0,
                    0,
                    0,
                    statspec,
                    std::ptr::null_mut(),
                    TileIndex::INVALID.0,
                );
                if cb_res != CALLBACK_FAILED
                    && !(self.s.rail_boolean)(
                        statspec,
                        CallbackID::CBID_STATION_AVAILABILITY.0,
                        cb_res,
                    )
                {
                    return cost.error(StringID::INVALID_STRING_ID.0);
                }
            }
        }
        if !flags.test(DoCommandFlag::Execute) {
            return;
        }
        (self.s.set_train_area)(st, new_location);
        crate::station_lifecycle::add_facility(
            st,
            StationFacility::Train,
            TileIndex(new_location.tile),
            unsafe { &*self.s.lifecycle },
        );
        self.before_add_rect(
            ret,
            st,
            TileArea {
                tile: tile_org.0,
                w: u16::from(w_org),
                h: u16::from(h_org),
            },
            StationRectMode::ADD_TRY,
        );
        crate::station_association::assign(self.s.rail_association, statspec, st, specindex.value);
        if !statspec.is_null() {
            /* Include this station spec's animation trigger bitmask
             * in the station's cached copy. */
            (unsafe { &*self.s.rail_association }.rail_triggers)(st, statspec);
        }
        let track = crate::track_func::axis_to_track(crate::services::Axis(axis.0), |ok| {
            crate::movement::native_assert(self.shared, ok);
        });
        let owner = CompanyID((self.world().station_owner)(st));
        let index = (self.links().graphs.station_id)(st);
        let mut tile_track = tile_org;
        for i in 0..numtracks {
            let mut tile = tile_track;
            for j in 0..plat_len {
                let position = u32::from(i) * u32::from(plat_len) + u32::from(j);
                if self.is_rail_station_tile(tile) && self.has_station_reservation(tile) {
                    /* Check for trains having a reservation for this tile. */
                    let v = crate::pbs::Pbs::acquire(self.shared)
                        .train_for(
                            tile.0,
                            crate::track_func::axis_to_track(
                                crate::services::Axis(self.get_rail_station_axis(tile).0),
                                |ok| crate::movement::native_assert(self.shared, ok),
                            )
                            .0,
                        )
                        .shell()
                        .cast::<crate::services::Train>();
                    if !v.is_null() {
                        affected_vehicles.push(v);
                        self.free_train_reservation(v);
                    }
                }
                /* Railtype can change when overbuilding. */
                if self.is_rail_station_tile(tile) {
                    if !self.is_station_tile_blocked(tile) {
                        (self.rail().rail_infrastructure)(
                            owner.0,
                            crate::map_func::World::new(self.shared).rail_type(tile.0),
                            u32::MAX,
                        );
                    }
                    (self.s.station_infrastructure)(owner.0, u32::MAX);
                }
                /* Remove animation if overbuilding */
                crate::animated_tile::openttd_rust_animation_delete(tile.0, false);
                let old_specindex =
                    if crate::map_func::World::new(self.shared).has_station_tile_rail(tile) {
                        (self.s.custom_rail_index)(tile.0) as u8
                    } else {
                        0
                    };
                let gfx = Self::rail_station_tile_layout_gfx(layout, position, numtracks, plat_len);
                (self.s.make_rail_station)(tile.0, owner.0, index, axis.0, gfx.0, rt.0);
                /* Free the spec if we overbuild something */
                crate::station_association::deallocate(self.s.rail_association, st, old_specindex);
                if statspec.is_null() {
                    (self.s.close_station_inspect)(tile.0);
                }
                (self.s.set_custom_rail_index)(tile.0, specindex.value);
                (self.s.set_random_bits)(
                    tile.0,
                    gb((self.shared.random)(self.shared.context), 0, 4) as u8,
                );
                (self.shared.tile_animation_set_frame)(tile.0, 0);
                if !statspec.is_null() {
                    /* Use a fixed axis for GetPlatformInfo as our platforms / numtracks are always the right way around */
                    let platinfo = crate::newgrf_station::platform_info(
                        crate::map_func::World::new(self.shared)
                            .get_station_gfx(tile)
                            .0,
                        plat_len,
                        numtracks,
                        j,
                        i,
                    );
                    /* As the station is not yet completely finished, the station does not yet exist. */
                    let callback = crate::newgrf_station::callback(
                        CallbackID::CBID_STATION_BUILD_TILE_LAYOUT.0,
                        platinfo,
                        0,
                        statspec,
                        std::ptr::null_mut(),
                        tile.0,
                    );
                    if callback != CALLBACK_FAILED {
                        if u8::try_from(callback).is_ok() {
                            (self.s.set_gfx)(tile.0, (callback as u8 & !1).wrapping_add(axis.0));
                        } else {
                            (self.s.rail_callback_error)(
                                statspec,
                                CallbackID::CBID_STATION_BUILD_TILE_LAYOUT.0,
                                callback,
                            );
                        }
                    }
                    /* Trigger station animation -- after building? */
                    crate::animated_tile::openttd_rust_trigger_station(
                        st,
                        tile.0,
                        crate::station_type::SAT_BUILT,
                        crate::cargo_type::INVALID_CARGO,
                    );
                }
                self.set_rail_station_tile_flags(tile, statspec);
                if !self.is_station_tile_blocked(tile) {
                    (self.rail().rail_infrastructure)(owner.0, rt.0, 1);
                }
                (self.s.station_infrastructure)(owner.0, 1);
                tile = TileIndex(tile.0.wrapping_add(tile_delta));
            }
            self.signals()
                .add_track(tile_track.0, track.0, (self.shared.current_company)());
            (self.rail().layout_changed)(tile_track.0, track.0);
            tile_track = TileIndex(tile_track.0.wrapping_add(track_delta));
        }
        for v in affected_vehicles {
            /* Restore reservations of trains. */
            self.restore_train_reservation(v);
        }
        self.expand_platform_reservations(tile_org, axis, numtracks);
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
    c.cmd_build_rail_station(
        c.c(cost),
        c.c(ret),
        DoCommandFlags(flags),
        CmdBuildRailStation {
            tile_org: TileIndex(tile),
            rt: RailType(rt),
            axis: Axis(axis),
            numtracks: tracks,
            plat_len: length,
            spec_class: StationClassID(class),
            spec_index: index,
            station_to_join: StationID(target),
            adjacent,
        },
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
    unsafe { Ctx::new(s) }.set_rail_station_tile_flags(TileIndex(tile), spec);
}

/// `CmdRemoveFromRailStation` and `CmdRemoveFromRailWaypoint`'s arguments.
#[derive(Clone, Copy)]
struct CmdRemoveFromRailBaseStation {
    /// Tile of station piece to remove.
    start: TileIndex,
    /// Other edge of the rect to remove.
    end: TileIndex,
    /// If set keep the rail.
    keep_rail: bool,
    /// Whether this is `CmdRemoveFromRailWaypoint`.
    waypoint: bool,
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
    /// A nested `Command<...>::Do(flags, tile, ...)` running `body` with the
    /// nested command's flags.
    fn nested(
        &self,
        out: Cost<'_>,
        tile: TileIndex,
        flags: DoCommandFlags,
        mut body: impl FnMut(DoCommandFlags),
    ) {
        crate::command::nested(self.command, self.shared, out, tile.0, flags.0, |flags| {
            body(DoCommandFlags(flags));
        });
    }
    /// `PR_CLEAR_WAYPOINT_RAIL` or `PR_CLEAR_STATION_RAIL`.
    fn rail_removal_cost(waypoint: bool) -> i64 {
        crate::company::price(usize::from(
            (if waypoint {
                Price::PR_CLEAR_WAYPOINT_RAIL
            } else {
                Price::PR_CLEAR_STATION_RAIL
            })
            .0,
        ))
    }
    /// `RemoveFromRailBaseStation`'s execution for one rail station tile.
    /// Returns the train whose reservation was freed, for the caller to restore.
    fn remove_rail_station_tile(
        &self,
        st: *mut BaseStation,
        tile: TileIndex,
        keep_rail: bool,
    ) -> *mut crate::services::Train {
        /* read variables before the station tile is removed */
        let specindex = (self.s.custom_rail_index)(tile.0);
        let track = crate::map_func::World::new(self.shared).get_rail_station_track(tile);
        let owner = CompanyID((self.shared.tile_owner)(tile.0));
        let rt = RailType(crate::map_func::World::new(self.shared).rail_type(tile.0));
        let mut v = std::ptr::null_mut();
        if self.has_station_reservation(tile) {
            v = crate::pbs::Pbs::acquire(self.shared)
                .train_for(tile.0, track.0)
                .shell()
                .cast::<crate::services::Train>();
            if !v.is_null() {
                self.free_train_reservation(v);
            }
        }
        let build_rail = keep_rail && !self.is_station_tile_blocked(tile);
        if !build_rail && !self.is_station_tile_blocked(tile) {
            (self.rail().rail_infrastructure)(owner.0, rt.0, u32::MAX);
        }
        crate::ground::clear_square(tile.0, self.shared);
        (self.s.close_station_inspect)(tile.0);
        if build_rail {
            (self.rail_map().make_rail_normal)(
                tile.0,
                owner.0,
                crate::track_func::track_to_track_bits(track.0, |ok| {
                    crate::movement::native_assert(self.shared, ok);
                }),
                rt.0,
            );
        }
        (self.s.station_infrastructure)(owner.0, u32::MAX);
        (self.command.infrastructure_dirty)(owner.0);
        self.after_remove_tile(st, tile);
        self.signals().add_track(tile.0, track.0, owner.0);
        (self.rail().layout_changed)(tile.0, track.0);
        crate::station_association::deallocate(self.s.rail_association, st, specindex as u8);
        v
    }
    /// `RemoveFromRailBaseStation`: remove a number of tiles from any rail
    /// station within the area; returns the stations affected.
    fn remove_from_rail_base_station(
        &self,
        context: RailStationRemoval<'_>,
    ) -> Vec<*mut BaseStation> {
        let RailStationRemoval {
            total_cost,
            ret,
            error,
            ta,
            flags,
            removal_cost,
            keep_rail,
            waypoint,
        } = context;
        crate::witness::hit(crate::witness::STATION_CMD_REMOVE_RAIL_AREA);
        /* Count of the number of tiles removed */
        let mut quantity = 0_i32;
        let mut affected_stations = Vec::new();
        total_cost.set(0);
        /* Accumulator for the errors seen during clearing. If no errors happen,
         * and the quantity is 0 there is no station. Otherwise it will be one
         * of the other error that got accumulated. */
        error.zero();
        /* Do the action for every tile into the area */
        for tile in ta.tiles(self.map()) {
            let tile = TileIndex(tile);
            /* Make sure the specified tile is a rail station */
            if !crate::map_func::World::new(self.shared).has_station_tile_rail(tile) {
                continue;
            }
            /* If there is a vehicle on ground, do not allow to remove (flood) the tile */
            unsafe {
                self.command.no_vehicle(ret.0, tile.0);
            }
            error.add(ret);
            if !error.ok() {
                continue;
            }
            /* Check ownership of station */
            let id = crate::map_func::World::new(self.shared).get_station_index(tile);
            let st = if waypoint {
                self.waypoint(id)
            } else {
                self.full_station(id)
            };
            if st.is_null() {
                continue;
            }
            if CompanyID((self.shared.current_company)()) != CompanyID::WATER {
                unsafe {
                    self.command
                        .ownership(ret.0, (self.world().station_owner)(st), 0);
                }
                error.add(ret);
                if !error.ok() {
                    continue;
                }
            }
            /* If we reached here, the tile is valid so increase the quantity of tiles we will remove */
            quantity += 1;
            if keep_rail || self.is_station_tile_blocked(tile) {
                /* Don't refund the 'steel' of the track when we keep the
                 *  rail, or when the tile didn't have any rail at all. */
                total_cost.add_money(
                    crate::company::price(usize::from(Price::PR_CLEAR_RAIL.0)).saturating_neg(),
                );
            }
            if !flags.test(DoCommandFlag::Execute) {
                continue;
            }
            let v = self.remove_rail_station_tile(st, tile, keep_rail);
            if !affected_stations.contains(&st) {
                affected_stations.push(st);
            }
            if !v.is_null() {
                self.restore_train_reservation(v);
            }
        }
        if quantity == 0 {
            if error.ok() {
                total_cost.error(self.s.strings.no_station);
            } else {
                total_cost.take(error);
            }
            return affected_stations;
        }
        if !error.ok() {
            crate::witness::hit(crate::witness::STATION_CMD_PARTIAL_RAIL);
        }
        for &st in &affected_stations {
            /* now we need to make the "spanned" area of the railway station smaller
             * if we deleted something at the edges.
             * we also need to adjust train_tile. */
            // `MakeRailStationAreaSmaller`.
            let train_station = self.make_station_area_smaller(self.s.rail_area(st), |tile| {
                self.tile_belongs_to_rail_station(st, tile)
            });
            (self.s.set_train_area)(st, train_station);
            self.update_station_sign_coord(st);
            /* if we deleted the whole station, delete the train facility. */
            if train_station.tile == TileIndex::INVALID.0 {
                (self.s.reset_facilities)(
                    st,
                    StationFacilities::from_element(StationFacility::Train).0,
                );
                (self.s.orders_dirty)();
                (self.s.facility_widget)(st, StationType::Rail.0);
                (self.s.catchment_dirty)();
                (self.s.virtual_coord)(st);
                self.delete_station_if_empty(st);
            }
        }
        total_cost.add_money(removal_cost.saturating_mul(i64::from(quantity)));
        affected_stations
    }
    /// `CmdRemoveFromRailStation` and `CmdRemoveFromRailWaypoint`: remove a
    /// single tile from a rail station or waypoint. This allows for
    /// custom-built stations with holes and weird layouts.
    fn cmd_remove_from_rail_base_station(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        error: Cost<'_>,
        flags: DoCommandFlags,
        args: CmdRemoveFromRailBaseStation,
    ) {
        let CmdRemoveFromRailBaseStation {
            start,
            mut end,
            keep_rail,
            waypoint,
        } = args;
        if end.0 == 0 {
            end = start;
        }
        let map_size = self.shared.map_size();
        if start.0 >= map_size || end.0 >= map_size {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        let affected_stations = self.remove_from_rail_base_station(RailStationRemoval {
            total_cost: cost,
            ret,
            error,
            ta: TileArea::between(self.map(), start.0, end.0),
            flags,
            removal_cost: Self::rail_removal_cost(waypoint),
            keep_rail,
            waypoint,
        });
        if !cost.ok() || waypoint {
            return;
        }
        /* Do all station specific functions here. */
        for st in affected_stations {
            if self.s.rail_area(st).tile == TileIndex::INVALID.0 {
                (self.s.facility_widget)(st, StationType::Rail.0);
            }
            (self.s.mark_tiles)(st, false);
            (self.s.catchment_dirty)();
            self.recompute_catchment(st, false);
        }
        /* Now apply the rail cost to the number that we deleted */
    }
    /// `RemoveRailStation(T *st, ...)`: remove a rail station/waypoint.
    fn remove_rail_station_of(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        error: Cost<'_>,
        flags: DoCommandFlags,
        st: *mut BaseStation,
        waypoint: bool,
    ) {
        /* Current company owns the station? */
        if CompanyID((self.shared.current_company)()) != CompanyID::WATER {
            unsafe {
                self.command
                    .ownership(cost.0, (self.world().station_owner)(st), 0);
            }
            if !cost.ok() {
                return;
            }
        }
        /* determine width and height of platforms */
        let ta = self.s.rail_area(st);
        cost.set(0);
        /* clear all areas of the station */
        for tile in ta.tiles(self.map()) {
            /* only remove tiles that are actually train station tiles */
            if !self.tile_belongs_to_rail_station(st, TileIndex(tile)) {
                continue;
            }
            let total = cost.money();
            self.remove_from_rail_base_station(RailStationRemoval {
                total_cost: cost,
                ret,
                error,
                ta: TileArea { tile, w: 1, h: 1 },
                flags,
                removal_cost: Self::rail_removal_cost(waypoint),
                keep_rail: false,
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
    /// `RemoveRailStation(TileIndex, ...)` and `RemoveRailWaypoint`.
    fn remove_rail_station(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        error: Cost<'_>,
        flags: DoCommandFlags,
        tile: TileIndex,
        waypoint: bool,
    ) {
        /* if there is flooding, remove platforms tile by tile */
        if CompanyID((self.shared.current_company)()) == CompanyID::WATER {
            return self.nested(
                cost,
                tile,
                DoCommandFlags::from_element(DoCommandFlag::Execute),
                |flags| {
                    self.cmd_remove_from_rail_base_station(
                        cost,
                        ret,
                        error,
                        flags,
                        CmdRemoveFromRailBaseStation {
                            start: tile,
                            end: TileIndex(0),
                            keep_rail: false,
                            waypoint,
                        },
                    );
                },
            );
        }
        let id = crate::map_func::World::new(self.shared).get_station_index(tile);
        let st = if waypoint {
            self.waypoint(id)
        } else {
            self.full_station(id)
        };
        self.remove_rail_station_of(cost, ret, error, flags, st, waypoint);
        if !waypoint && flags.test(DoCommandFlag::Execute) {
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
    c.cmd_remove_from_rail_base_station(
        c.c(cost),
        c.c(ret),
        c.c(error),
        DoCommandFlags(flags),
        CmdRemoveFromRailBaseStation {
            start: TileIndex(start),
            end: TileIndex(end),
            keep_rail: keep,
            waypoint,
        },
    );
}

/// `CmdBuildRoadStop`'s arguments.
#[derive(Clone, Copy)]
struct CmdBuildRoadStop {
    /// Northernmost tile of the stop.
    tile: TileIndex,
    /// Width of the road stop.
    width: u8,
    /// Length of the road stop.
    length: u8,
    /// Type of road stop (bus/truck).
    stop_type: RoadStopType,
    /// False for normal stops, true for drive-through.
    is_drive_through: bool,
    /// Entrance direction (`DiagDirection`) for normal stops. Converted to the axis for drive-through stops.
    ddir: DiagDirection,
    /// The roadtype.
    rt: RoadType,
    /// Road stop spec class.
    spec_class: RoadStopClassID,
    /// Road stop spec index.
    spec_index: u16,
    /// Station ID to join (`NEW_STATION` if build new one).
    station_to_join: StationID,
    /// Allow stations directly adjacent to other stations.
    adjacent: bool,
}

impl Ctx<'_> {
    fn vehicle(&self) -> &crate::vehicle::VehicleServices {
        unsafe { &*self.s.vehicle }
    }
    fn stops(&self) -> &RoadStopServices {
        // SAFETY: The shared table returns a static immutable table.
        unsafe { &*(self.shared.road_stops)() }
    }
    /// `GetRoadType(tile, rtt)`.
    fn get_road_type(&self, tile: TileIndex, rtt: RoadTramType) -> RoadType {
        RoadType(
            crate::map_func::World::new(self.shared)
                .road_type(tile.0, rtt == RoadTramType::RTT_TRAM),
        )
    }
    /// `GetRoadOwner(tile, rtt)`.
    fn get_road_owner(&self, tile: TileIndex, rtt: RoadTramType) -> CompanyID {
        CompanyID(
            crate::map_func::World::new(self.shared)
                .road_owner(tile.0, rtt == RoadTramType::RTT_TRAM),
        )
    }
    /// `GetRoadBits(tile, rtt)`.
    fn get_road_bits(&self, tile: TileIndex, rtt: RoadTramType) -> RoadBits {
        RoadBits(
            crate::map_func::World::new(self.shared)
                .road_bits(tile.0, rtt == RoadTramType::RTT_TRAM),
        )
    }
    /// `MayHaveRoad`.
    fn may_have_road(&self, tile: TileIndex) -> bool {
        crate::map_func::World::new(self.shared)
            .asserted(|m| crate::road_map::may_have_road(m, tile))
    }
    /// `UpdateCompanyRoadInfrastructure`.
    fn update_company_road_infrastructure(&self, rt: RoadType, o: CompanyID, count: i32) {
        // SAFETY: The shared road command tables are process-lifetime.
        unsafe {
            crate::road_cmd::update_company_road_infrastructure(
                (self.shared.road_commands)(),
                rt.0,
                o.0,
                count,
            );
        }
    }
    /// `IsRoadStationBridgeAboveOk`: test if a road station can be built below
    /// a bridge.
    fn is_road_station_bridge_above_ok(
        &self,
        out: Cost<'_>,
        tile: TileIndex,
        spec: *const RoadSpec,
        station_type: StationType,
        layout: StationGfx,
    ) {
        if !crate::map_func::World::new(self.shared).bridge_above(tile.0) {
            return out.zero();
        }
        let rampsouth = TileIndex((self.s.southern_bridge_end)(tile.0));
        let bridgeable_info = if spec.is_null() {
            (self.s.default_bridge_info)(station_type.0)
        } else {
            unsafe { (self.s.road_bridge_info)(spec) }
        };
        self.is_station_bridge_above_ok(StationBridgeCheck {
            out,
            tile,
            bridgeable_info,
            station_type,
            layout,
            bridge_height: self.bridge_height(rampsouth),
            disallowed_msg: StringID(self.s.strings.demolish_bridge),
        });
    }
    /// `CheckBuildAbove_Station`.
    fn check_build_above_station(&self, out: Cost<'_>, tile: TileIndex, height: i32) {
        let station_type = StationType((self.shared.station_type)(tile.0).0);
        let mut bridgeable_info = (self.s.default_bridge_info)(station_type.0);
        match station_type {
            StationType::Rail | StationType::RailWaypoint => {
                let spec = (self.s.rail_spec)(tile.0);
                if !spec.is_null() {
                    bridgeable_info = unsafe { (self.s.rail_bridge_info)(spec) };
                }
            }
            StationType::Bus | StationType::Truck | StationType::RoadWaypoint => {
                let spec = unsafe {
                    crate::newgrf_roadstop::openttd_rust_roadstop_spec(
                        tile.0,
                        self.s.roadstop_resolver,
                    )
                };
                if !spec.is_null() {
                    bridgeable_info = unsafe { (self.s.road_bridge_info)(spec) };
                }
            }
            _ => (),
        }
        self.is_station_bridge_above_ok(StationBridgeCheck {
            out,
            tile,
            bridgeable_info,
            station_type,
            layout: StationGfx((self.shared.station_gfx)(tile.0)),
            bridge_height: height,
            disallowed_msg: StringID::INVALID_STRING_ID,
        });
    }
    /// `CheckFlatLandRoadStop`'s check of building a road+tram stop over an
    /// existing normal road tile.
    fn check_build_over_road(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        cur_tile: TileIndex,
        rt: RoadType,
    ) {
        /* There is a road, check if we can build road+tram stop over it. */
        let road_rt = self.get_road_type(cur_tile, RoadTramType::RTT_ROAD);
        if road_rt != RoadType::INVALID_ROADTYPE {
            let road_owner = self.get_road_owner(cur_tile, RoadTramType::RTT_ROAD);
            if road_owner == CompanyID::TOWN {
                if !(self.s.stop_on_town_road)() {
                    return cost.error(self.s.strings.drive_town);
                }
            } else if !(self.s.stop_on_competitor_road)() && road_owner != CompanyID::NONE {
                unsafe {
                    self.command.ownership(ret.0, road_owner.0, 0);
                }
                if !ret.ok() {
                    return cost.take(ret);
                }
            }
            let num_pieces = self
                .get_road_bits(cur_tile, RoadTramType::RTT_ROAD)
                .0
                .count_ones();
            if rt != RoadType::INVALID_ROADTYPE
                && road_type_is_road(rt.0)
                && !(self.shared.road_powered)(rt.0, road_rt.0)
            {
                return cost.error(self.s.strings.unsuitable_road);
            }
            if crate::map_func::World::new(self.shared).disallowed_road_directions(cur_tile.0)
                != DisallowedRoadDirections::DRD_NONE.0
                && road_owner != CompanyID::TOWN
                && road_owner != CompanyID::NONE
            {
                unsafe {
                    self.command.ownership(ret.0, road_owner.0, 0);
                }
                if !ret.ok() {
                    return cost.take(ret);
                }
            }
            cost.add_money(
                crate::road_cost::build_cost(self.shared, road_rt.0)
                    .saturating_mul(i64::from(2_u32.wrapping_sub(num_pieces))),
            );
        } else if rt != RoadType::INVALID_ROADTYPE && road_type_is_road(rt.0) {
            cost.add_money(crate::road_cost::build_cost(self.shared, rt.0).saturating_mul(2));
        }
        /* There is a tram, check if we can build road+tram stop over it. */
        let tram_rt = self.get_road_type(cur_tile, RoadTramType::RTT_TRAM);
        if tram_rt != RoadType::INVALID_ROADTYPE {
            let tram_owner = self.get_road_owner(cur_tile, RoadTramType::RTT_TRAM);
            if (self.shared.company_valid)(tram_owner.0)
                && (!(self.s.stop_on_competitor_road)()
                    /* Disallow breaking end-of-line of someone else
                     * so trams can still reverse on this tile. */
                    || self
                        .get_road_bits(cur_tile, RoadTramType::RTT_TRAM)
                        .0
                        .is_power_of_two())
            {
                unsafe {
                    self.command.ownership(ret.0, tram_owner.0, 0);
                }
                if !ret.ok() {
                    return cost.take(ret);
                }
            }
            let num_pieces = self
                .get_road_bits(cur_tile, RoadTramType::RTT_TRAM)
                .0
                .count_ones();
            if rt != RoadType::INVALID_ROADTYPE
                && road_type_is_tram(rt.0)
                && !(self.shared.road_powered)(rt.0, tram_rt.0)
            {
                return cost.error(self.s.strings.unsuitable_road);
            }
            cost.add_money(
                crate::road_cost::build_cost(self.shared, tram_rt.0)
                    .saturating_mul(i64::from(2_u32.wrapping_sub(num_pieces))),
            );
        } else if rt != RoadType::INVALID_ROADTYPE && road_type_is_tram(rt.0) {
            cost.add_money(crate::road_cost::build_cost(self.shared, rt.0).saturating_mul(2));
        }
    }
    /// `CheckFlatLandRoadStop`: checks if a road stop can be built at the given
    /// tile.
    fn check_flat_land_road_stop(&self, context: RoadStopLandCheck<'_>) {
        let RoadStopLandCheck {
            cost,
            ret,
            cur_tile,
            allowed_z,
            spec,
            flags,
            invalid_dirs,
            is_drive_through,
            station_type,
            axis,
            station,
            rt,
        } = context;
        let world = crate::map_func::World::new(self.shared);
        self.check_buildable(
            ret,
            cur_tile,
            invalid_dirs,
            allowed_z,
            !is_drive_through,
            false,
        );
        if !ret.ok() {
            return cost.take(ret);
        }
        cost.set(0);
        cost.add_money(ret.money());
        self.is_road_station_bridge_above_ok(
            ret,
            cur_tile,
            spec,
            station_type,
            StationGfx(if is_drive_through {
                GFX_TRUCK_BUS_DRIVETHROUGH_OFFSET + axis.0
            } else {
                find_first_bit(u64::from(invalid_dirs.0))
            }),
        );
        if !ret.ok() {
            return cost.take(ret);
        }
        /* If station is set, then we have special handling to allow building on top of already existing stations.
         * Station points to StationID::Invalid() if we can build on any station.
         * Or it points to a station if we're only allowed to build on exactly that station. */
        if TileType((self.shared.tile_type)(cur_tile.0)) == TileType::MP_STATION {
            if !world.any_road_stop_tile(cur_tile.0) {
                return self.auto_clear(cost, cur_tile); // Get error message.
            }
            if station_type != StationType((self.shared.station_type)(cur_tile.0).0)
                || is_drive_through != world.drive_through_stop_tile(cur_tile.0)
            {
                return self.auto_clear(cost, cur_tile); // Get error message.
            }
            /* Drive-through station in the wrong direction. */
            if is_drive_through
                && world.drive_through_stop_tile(cur_tile.0)
                && world.get_drive_through_stop_axis(cur_tile) != axis
            {
                return cost.error(self.s.strings.drive_direction);
            }
            let st = StationID((self.shared.station_index)(cur_tile.0));
            if *station == StationID::INVALID {
                *station = st;
            } else if *station != st {
                cost.error(self.s.strings.adjoins_multiple);
            }
            return;
        }
        let build_over_road = is_drive_through && world.is_normal_road_tile(cur_tile.0);
        /* Road bits in the wrong direction. */
        let rb = if world.is_normal_road_tile(cur_tile.0) {
            RoadBits(world.all_road_bits(cur_tile.0))
        } else {
            RoadBits::ROAD_NONE
        };
        let wrong = if axis == Axis::AXIS_X {
            RoadBits::ROAD_Y
        } else {
            RoadBits::ROAD_X
        };
        if build_over_road && rb & wrong != RoadBits::ROAD_NONE {
            /* Someone was pedantic and *NEEDED* three fracking different error messages. */
            return cost.error(match rb.0.count_ones() {
                1 => self.s.strings.drive_direction,
                2 if rb == RoadBits::ROAD_X || rb == RoadBits::ROAD_Y => {
                    self.s.strings.drive_direction
                }
                2 => self.s.strings.drive_corner,
                _ => self.s.strings.drive_junction, // 3 or 4
            });
        }
        if build_over_road {
            self.check_build_over_road(cost, ret, cur_tile, rt);
        } else if rt == RoadType::INVALID_ROADTYPE {
            cost.error(self.s.strings.no_road);
        } else {
            crate::landscape_cmd::clear_command(self.command, ret.0, cur_tile.0, flags.0);
            if !ret.ok() {
                return cost.take(ret);
            }
            cost.add_money(ret.money());
            cost.add_money(crate::road_cost::build_cost(self.shared, rt.0).saturating_mul(2));
        }
    }
    /// `CalculateRoadStopCost`: calculates cost of new road stops within the
    /// area.
    fn calculate_road_stop_cost(&self, context: RoadStopCost<'_>) {
        let RoadStopCost {
            cost,
            ret,
            tile_area,
            flags,
            is_drive_through,
            station_type,
            roadstopspec,
            axis,
            ddir,
            est,
            rt,
            unit_cost,
        } = context;
        let invalid_dirs = if is_drive_through {
            let dir = axis_to_diag_dir(axis, |ok| {
                crate::movement::native_assert(self.shared, ok);
            });
            DiagDirections::from_element(dir)
                | DiagDirections::from_element(reverse_diag_dir(dir, |ok| {
                    crate::movement::native_assert(self.shared, ok);
                }))
        } else {
            DiagDirections::from_element(ddir)
        };
        /* Check every tile in the area. */
        let mut allowed_z = -1;
        cost.set(0);
        for cur_tile in tile_area.tiles(self.map()) {
            let cur_tile = TileIndex(cur_tile);
            // CheckFlatLandRoadStop uses both native slots; its result is left
            // in `cost`, and the accumulated amount is restored after.
            let total = cost.money();
            self.check_flat_land_road_stop(RoadStopLandCheck {
                cost,
                ret,
                cur_tile,
                allowed_z: &mut allowed_z,
                spec: roadstopspec,
                flags,
                invalid_dirs,
                is_drive_through,
                station_type,
                axis,
                station: &mut *est,
                rt,
            });
            if !cost.ok() {
                return;
            }
            let ret_cost = cost.money();
            cost.set(total);
            let is_preexisting_roadstop = TileType((self.shared.tile_type)(cur_tile.0))
                == TileType::MP_STATION
                && crate::map_func::World::new(self.shared).any_road_stop_tile(cur_tile.0);
            /* Only add costs if a stop doesn't already exist in the location */
            if !is_preexisting_roadstop {
                cost.add_money(ret_cost);
                cost.add_money(unit_cost);
            }
        }
    }
    /// `CmdBuildRoadStop`'s existing road types and owners of a tile, before any
    /// tile clearing: `INVALID_ROADTYPE` and `_current_company` where absent.
    fn existing_road_types_and_owners(
        &self,
        cur_tile: TileIndex,
    ) -> (RoadType, RoadType, CompanyID, CompanyID) {
        let road_rt = if self.may_have_road(cur_tile) {
            self.get_road_type(cur_tile, RoadTramType::RTT_ROAD)
        } else {
            RoadType::INVALID_ROADTYPE
        };
        let tram_rt = if self.may_have_road(cur_tile) {
            self.get_road_type(cur_tile, RoadTramType::RTT_TRAM)
        } else {
            RoadType::INVALID_ROADTYPE
        };
        let road_owner = if road_rt != RoadType::INVALID_ROADTYPE {
            self.get_road_owner(cur_tile, RoadTramType::RTT_ROAD)
        } else {
            CompanyID((self.shared.current_company)())
        };
        let tram_owner = if tram_rt != RoadType::INVALID_ROADTYPE {
            self.get_road_owner(cur_tile, RoadTramType::RTT_TRAM)
        } else {
            CompanyID((self.shared.current_company)())
        };
        (road_rt, tram_rt, road_owner, tram_owner)
    }
    /// `FindRoadStopSpot` and `*currstop = road_stop`: append `road_stop` to the
    /// station's truck or bus stop list.
    fn find_road_stop_spot_insert(
        &self,
        st: *mut BaseStation,
        truck_station: bool,
        road_stop: *mut Stop,
    ) {
        let primary_stop = (self.stops().head)(st, truck_station);
        if primary_stop.is_null() {
            /* we have no roadstop of the type yet, so write a "primary stop" */
            (self.s.set_stop_head)(st, truck_station, road_stop);
        } else {
            /* there are stops already, so append to the end of the list */
            let mut stop = primary_stop;
            while !(self.stops().next)(stop).is_null() {
                stop = (self.stops().next)(stop);
            }
            (self.s.set_stop_next)(stop, road_stop);
        }
    }
    /// `CmdBuildRoadStop`'s company infrastructure update on a normal road tile:
    /// remove the old bits first.
    fn remove_normal_road_infrastructure(
        &self,
        cur_tile: TileIndex,
        road_rt: RoadType,
        road_owner: CompanyID,
        tram_rt: RoadType,
        tram_owner: CompanyID,
    ) {
        self.update_company_road_infrastructure(
            road_rt,
            road_owner,
            -(self
                .get_road_bits(cur_tile, RoadTramType::RTT_ROAD)
                .0
                .count_ones() as i32),
        );
        self.update_company_road_infrastructure(
            tram_rt,
            tram_owner,
            -(self
                .get_road_bits(cur_tile, RoadTramType::RTT_TRAM)
                .0
                .count_ones() as i32),
        );
    }
    /// `CmdBuildRoadStop`'s checks of a custom road stop spec against the stop
    /// type and drive-through flag.
    fn road_stop_spec_allows(
        &self,
        roadstopspec: *const RoadSpec,
        stop_type: RoadStopType,
        is_drive_through: bool,
    ) -> bool {
        let info = (self.s.road_spec_info)(roadstopspec);
        let spec_type = RoadStopAvailabilityType(info.stop_type);
        if stop_type == RoadStopType::Truck
            && spec_type != RoadStopAvailabilityType::ROADSTOPTYPE_FREIGHT
            && spec_type != RoadStopAvailabilityType::ROADSTOPTYPE_ALL
        {
            return false;
        }
        if stop_type == RoadStopType::Bus
            && spec_type != RoadStopAvailabilityType::ROADSTOPTYPE_PASSENGER
            && spec_type != RoadStopAvailabilityType::ROADSTOPTYPE_ALL
        {
            return false;
        }
        is_drive_through || !info.drive_only
    }
    /// `CmdBuildRoadStop`: build a bus or truck stop.
    fn cmd_build_road_stop(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        flags: DoCommandFlags,
        args: CmdBuildRoadStop,
    ) {
        let CmdBuildRoadStop {
            tile,
            width,
            length,
            stop_type,
            is_drive_through,
            ddir,
            rt,
            spec_class,
            spec_index,
            mut station_to_join,
            adjacent,
        } = args;
        crate::witness::hit(crate::witness::STATION_CMD_BUILD_ROAD);
        if !crate::road::val_param_road_type(self.shared, rt.0)
            || !is_valid_diag_direction(ddir)
            || stop_type.0 >= RoadStopType::End.0
        {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        let reuse = station_to_join != StationID::NEW;
        if !reuse {
            station_to_join = StationID::INVALID;
        }
        let distant_join = station_to_join != StationID::INVALID;
        /* Check if the given station class is valid */
        if u32::from(spec_class.0) >= crate::newgrf_class::roadstop::count()
            || crate::newgrf_station::is_waypoint_class(crate::newgrf_class::roadstop_label(
                spec_class.0,
            ))
            || u32::from(spec_index) >= crate::newgrf_class::roadstop::spec_count(spec_class.0)
        {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        let roadstopspec = crate::newgrf_class::roadstop::spec(spec_class.0, u32::from(spec_index));
        if !roadstopspec.is_null()
            && !self.road_stop_spec_allows(roadstopspec, stop_type, is_drive_through)
        {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        /* Check if the requested road stop is too big */
        let station_spread = (self.s.spread)();
        if width > station_spread || length > station_spread {
            return cost.error(self.s.strings.spread);
        }
        /* Check for incorrect width / length. */
        if width == 0 || length == 0 {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        /* Check if the first tile and the last tile are valid */
        if !self.first_and_last_tile_valid(tile, u16::from(width), u16::from(length)) {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        let roadstop_area = TileArea {
            tile: tile.0,
            w: u16::from(width),
            h: u16::from(length),
        };
        if distant_join
            && (!(self.s.distant_join)() || self.full_station(station_to_join).is_null())
        {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        /* Trams only have drive through stops */
        if !is_drive_through && road_type_is_tram(rt.0) {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        let axis = diag_dir_to_axis(ddir, |ok| {
            crate::movement::native_assert(self.shared, ok);
        });
        self.check_if_authority_allows_new_station(cost, tile, flags);
        if !cost.ok() {
            return;
        }
        let is_truck_stop = stop_type != RoadStopType::Bus;
        let station_type = if is_truck_stop {
            StationType::Truck
        } else {
            StationType::Bus
        };
        let price = if is_truck_stop {
            Price::PR_BUILD_STATION_TRUCK
        } else {
            Price::PR_BUILD_STATION_BUS
        };
        /* Total road stop cost. */
        let unit_cost = if roadstopspec.is_null() {
            crate::company::price(usize::from(price.0))
        } else {
            crate::newgrf_roadstop::get_build_cost(self.s, roadstopspec, price)
        };
        let mut est = StationID::INVALID;
        self.calculate_road_stop_cost(RoadStopCost {
            cost,
            ret,
            tile_area: roadstop_area,
            flags,
            is_drive_through,
            station_type,
            roadstopspec,
            axis,
            ddir,
            est: &mut est,
            rt,
            unit_cost,
        });
        if !cost.ok() {
            return;
        }
        // `FindJoiningRoadStop`.
        let st = self.find_joining_base_station(JoiningStation {
            out: ret,
            existing_station: est,
            station_to_join,
            adjacent,
            ta: roadstop_area,
            waypoint: false,
            road: false,
            error_message: StringID(self.s.strings.remove_road_stop),
        });
        if !ret.ok() {
            return cost.take(ret);
        }
        /* Check if this number of road stops can be allocated. */
        if !(self.s.stops_available)(usize::from(roadstop_area.w) * usize::from(roadstop_area.h)) {
            return cost.error(if is_truck_stop {
                self.s.strings.too_many_truck
            } else {
                self.s.strings.too_many_bus
            });
        }
        let st = self.build_station_part(
            ret,
            st,
            flags,
            reuse,
            roadstop_area,
            StationNaming::STATIONNAMING_ROAD,
        );
        if !ret.ok() {
            return cost.take(ret);
        }
        /* Check if we can allocate a custom stationspec to this station */
        let index = unsafe {
            crate::newgrf_roadstop::openttd_rust_roadstop_allocate(
                roadstopspec,
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
        /* Perform NewGRF checks */
        /* Check if the road stop is buildable */
        if !roadstopspec.is_null() && (self.s.road_spec_info)(roadstopspec).availability_callback {
            let cb_res = unsafe {
                crate::newgrf_roadstop::openttd_rust_roadstop_callback(
                    CallbackID::CBID_STATION_AVAILABILITY.0,
                    0,
                    0,
                    roadstopspec,
                    std::ptr::null_mut(),
                    TileIndex::INVALID.0,
                    rt.0,
                    station_type.0,
                    0,
                    std::ptr::null_mut(),
                    0,
                    self.s.roadstop_resolver,
                )
            };
            if cb_res != CALLBACK_FAILED && !(self.s.road_boolean)(roadstopspec, cb_res) {
                return cost.error(StringID::INVALID_STRING_ID.0);
            }
        }
        if !flags.test(DoCommandFlag::Execute) {
            return;
        }
        unsafe {
            crate::newgrf_roadstop::openttd_rust_roadstop_assign(
                roadstopspec,
                st,
                specindex.value,
                self.s.roadstop_resolver,
            );
        }
        /* Check every tile in the area. */
        for cur_tile in roadstop_area.tiles(self.map()) {
            let cur_tile = TileIndex(cur_tile);
            /* Get existing road types and owners before any tile clearing */
            let (mut road_rt, mut tram_rt, road_owner, tram_owner) =
                self.existing_road_types_and_owners(cur_tile);
            if TileType((self.shared.tile_type)(cur_tile.0)) == TileType::MP_STATION
                && matches!(
                    StationType((self.shared.station_type)(cur_tile.0).0),
                    StationType::Truck | StationType::Bus
                )
            {
                self.remove_road_stop(ret, cur_tile, flags, i32::from(specindex.value));
            }
            if !roadstopspec.is_null() {
                /* Include this road stop spec's animation trigger bitmask
                 * in the station's cached copy. */
                (self.s.road_triggers)(st, roadstopspec);
            }
            let road_stop = (self.s.new_stop)(cur_tile.0);
            /* Insert into linked list of RoadStops. */
            self.find_road_stop_spot_insert(st, is_truck_stop, road_stop);
            self.area_add((self.s.stop_area)(st, is_truck_stop), cur_tile);
            /* Initialize an empty station. */
            crate::station_lifecycle::add_facility(
                st,
                if is_truck_stop {
                    StationFacility::TruckStop
                } else {
                    StationFacility::BusStop
                },
                cur_tile,
                unsafe { &*self.s.lifecycle },
            );
            self.before_add_tile(ret, st, cur_tile, StationRectMode::ADD_TRY);
            let owner = CompanyID((self.world().station_owner)(st));
            let index = (self.links().graphs.station_id)(st);
            if is_drive_through {
                /* Update company infrastructure counts. If the current tile is a normal road tile, remove the old
                 * bits first. */
                if crate::map_func::World::new(self.shared).is_normal_road_tile(cur_tile.0) {
                    self.remove_normal_road_infrastructure(
                        cur_tile, road_rt, road_owner, tram_rt, tram_owner,
                    );
                }
                if road_rt == RoadType::INVALID_ROADTYPE && road_type_is_road(rt.0) {
                    road_rt = rt;
                }
                if tram_rt == RoadType::INVALID_ROADTYPE && road_type_is_tram(rt.0) {
                    tram_rt = rt;
                }
                (self.s.make_drive_stop)(
                    cur_tile.0,
                    owner.0,
                    road_owner.0,
                    tram_owner.0,
                    index,
                    station_type.0,
                    road_rt.0,
                    tram_rt.0,
                    axis.0,
                );
                make_drive_through(self.shared, cur_tile, (self.stops().state)(road_stop));
            } else {
                if road_rt == RoadType::INVALID_ROADTYPE && road_type_is_road(rt.0) {
                    road_rt = rt;
                }
                if tram_rt == RoadType::INVALID_ROADTYPE && road_type_is_tram(rt.0) {
                    tram_rt = rt;
                }
                (self.s.make_bay_stop)(
                    cur_tile.0,
                    owner.0,
                    index,
                    stop_type.0,
                    road_rt.0,
                    tram_rt.0,
                    ddir.0,
                );
            }
            self.update_company_road_infrastructure(
                road_rt,
                road_owner,
                ROAD_STOP_TRACKBIT_FACTOR as i32,
            );
            self.update_company_road_infrastructure(
                tram_rt,
                tram_owner,
                ROAD_STOP_TRACKBIT_FACTOR as i32,
            );
            (self.s.station_infrastructure)(owner.0, 1);
            (self.s.set_custom_road_index)(cur_tile.0, specindex.value);
            if !roadstopspec.is_null() {
                unsafe {
                    ((*self.s.roadstop_resolver).set_tile_random)(
                        st,
                        cur_tile.0,
                        gb((self.shared.random)(self.shared.context), 0, 8) as u8,
                    );
                }
                crate::animated_tile::openttd_rust_trigger_roadstop(
                    st,
                    cur_tile.0,
                    crate::station_type::SAT_BUILT,
                    crate::cargo_type::INVALID_CARGO,
                );
            }
            (self.shared.mark_dirty)(cur_tile.0);
        }
        if !st.is_null() {
            self.after_station_tile_set_change(st, true, station_type);
        }
    }
    /// `RemoveRoadStop`: remove a bus station/truck stop. A negative
    /// `replacement_spec_index` means the tile is not being replaced.
    fn remove_road_stop(
        &self,
        cost: Cost<'_>,
        tile: TileIndex,
        flags: DoCommandFlags,
        replacement_spec_index: i32,
    ) {
        crate::witness::hit(crate::witness::STATION_CMD_REMOVE_ROAD_TILE);
        let world = crate::map_func::World::new(self.shared);
        let st = self.full_station(StationID((self.shared.station_index)(tile.0)));
        if CompanyID((self.shared.current_company)()) != CompanyID::WATER {
            unsafe {
                self.command
                    .ownership(cost.0, (self.world().station_owner)(st), 0);
            }
            if !cost.ok() {
                return;
            }
        }
        let is_truck = StationType((self.shared.station_type)(tile.0).0) == StationType::Truck;
        let (cur_stop, cur_stop_state) = road_stop_by_tile(
            self.shared,
            tile,
            if is_truck {
                RoadStopType::Truck
            } else {
                RoadStopType::Bus
            },
        );
        /* don't do the check for drive-through road stops when company bankrupts */
        if world.drive_through_stop_tile(tile.0) && flags.test(DoCommandFlag::Bankrupt) {
            /* remove the 'going through road stop' status from all vehicles on that tile */
            if flags.test(DoCommandFlag::Execute) {
                self.vehicle().for_each_on_tile(tile.0, |v| {
                    if VehicleType(v.vehicle_type()) != VehicleType::VEH_ROAD {
                        return;
                    }
                    /* Okay... we are a road vehicle on a drive through road stop.
                     * But that road stop has just been removed, so we need to make
                     * sure we are in a valid state... however, vehicles can also
                     * turn on road stop tiles, so only clear the 'road stop' state
                     * bits and only when the state was 'in road stop', otherwise
                     * we'll end up clearing the turn around bits. */
                    let rv = (self.s.road_vehicle_owner)(v);
                    // SAFETY: Live road owner; no borrow crosses this access.
                    unsafe { crate::road::clear_removed_drive_stop(rv) };
                });
            }
        } else {
            unsafe {
                self.command.no_vehicle(cost.0, tile.0);
            }
            if !cost.ok() {
                return;
            }
        }
        let spec = unsafe {
            crate::newgrf_roadstop::openttd_rust_roadstop_spec(tile.0, self.s.roadstop_resolver)
        };
        if flags.test(DoCommandFlag::Execute) {
            let primary_stop = (self.stops().head)(st, is_truck);
            let next = (self.stops().next)(cur_stop);
            if primary_stop == cur_stop {
                /* removed the first stop in the list */
                (self.s.set_stop_head)(st, is_truck, next);
                /* removed the only stop? */
                if next.is_null() {
                    (self.s.reset_facilities)(
                        st,
                        StationFacilities::from_element(if is_truck {
                            StationFacility::TruckStop
                        } else {
                            StationFacility::BusStop
                        })
                        .0,
                    );
                    (self.s.orders_dirty)();
                }
            } else {
                /* tell the predecessor in the list to skip this stop */
                let mut pred = primary_stop;
                while (self.stops().next)(pred) != cur_stop {
                    pred = (self.stops().next)(pred);
                }
                (self.s.set_stop_next)(pred, next);
            }
            /* Update company infrastructure counts. */
            for rtt in [RoadTramType::RTT_ROAD, RoadTramType::RTT_TRAM] {
                let rt = self.get_road_type(tile, rtt);
                self.update_company_road_infrastructure(
                    rt,
                    self.get_road_owner(tile, rtt),
                    -(ROAD_STOP_TRACKBIT_FACTOR as i32),
                );
            }
            let owner = CompanyID((self.world().station_owner)(st));
            (self.s.station_infrastructure)(owner.0, u32::MAX);
            (self.command.infrastructure_dirty)(owner.0);
            let specindex = (self.s.custom_road_index)(tile.0);
            (self.s.close_stop_inspect)(tile.0);
            if world.drive_through_stop_tile(tile.0) {
                /* Clears the tile for us */
                clear_drive_through(self.shared, tile, cur_stop_state);
                crate::animated_tile::openttd_rust_animation_delete(tile.0, false);
            } else {
                crate::ground::clear_square(tile.0, self.shared);
            }
            (self.s.delete_stop)(cur_stop);
            /* Make sure no vehicle is going to the old roadstop. Narrow the search to any road vehicles with an order to
             * this station, then look for any currently heading to the tile. */
            let station_id = StationID((self.links().graphs.station_id)(st));
            crate::vehiclelist_func::find_vehicles_with_order(
                self.links(),
                |v| VehicleType(v.vehicle_type()) == VehicleType::VEH_ROAD,
                |order| {
                    order.kind == OrderType::OT_GOTO_STATION && order.destination == station_id.0
                },
                |v| {
                    if OrderType((self.vehicle().order_type)(v)) == OrderType::OT_GOTO_STATION
                        && v.dest_tile() == tile.0
                    {
                        let destination = (self.s.order_station_location)(v, station_id.0);
                        (self.vehicle().set_dest_tile)(v, destination);
                    }
                },
            );
            self.after_remove_tile(st, tile);
            if replacement_spec_index < 0 {
                self.after_station_tile_set_change(
                    st,
                    false,
                    if is_truck {
                        StationType::Truck
                    } else {
                        StationType::Bus
                    },
                );
            }
            (self.s.remove_stop_tile_data)(st, tile.0);
            if specindex as i32 != replacement_spec_index {
                unsafe {
                    crate::newgrf_roadstop::openttd_rust_roadstop_deallocate(
                        st,
                        specindex as u8,
                        self.s.roadstop_resolver,
                    );
                }
            }
            /* Update the tile area of the truck/bus stop */
            (self.s.clear_stop_area)(st, is_truck);
            let mut rs = (self.stops().head)(st, is_truck);
            while !rs.is_null() {
                self.area_add(
                    (self.s.stop_area)(st, is_truck),
                    TileIndex((self.stops().tile)(rs)),
                );
                rs = (self.stops().next)(rs);
            }
        }
        let category = if is_truck {
            Price::PR_CLEAR_STATION_TRUCK
        } else {
            Price::PR_CLEAR_STATION_BUS
        };
        cost.set(if spec.is_null() {
            crate::company::price(usize::from(category.0))
        } else {
            crate::newgrf_roadstop::get_clear_cost(self.s, spec, category)
        });
    }
    /// `RemoveRoadWaypointStop`: remove a road waypoint. A negative
    /// `replacement_spec_index` means the tile is not being replaced.
    fn remove_road_waypoint_stop(
        &self,
        cost: Cost<'_>,
        tile: TileIndex,
        flags: DoCommandFlags,
        replacement_spec_index: i32,
    ) {
        crate::witness::hit(crate::witness::STATION_CMD_REMOVE_ROAD_TILE);
        let wp = self.waypoint(StationID((self.shared.station_index)(tile.0)));
        if CompanyID((self.shared.current_company)()) != CompanyID::WATER {
            unsafe {
                self.command
                    .ownership(cost.0, (self.world().station_owner)(wp), 0);
            }
            if !cost.ok() {
                return;
            }
        }
        /* Ignore vehicles when the company goes bankrupt. The road will remain, any vehicles going to the waypoint will be removed. */
        if !flags.test(DoCommandFlag::Bankrupt) {
            unsafe {
                self.command.no_vehicle(cost.0, tile.0);
            }
            if !cost.ok() {
                return;
            }
        }
        let spec = unsafe {
            crate::newgrf_roadstop::openttd_rust_roadstop_spec(tile.0, self.s.roadstop_resolver)
        };
        if flags.test(DoCommandFlag::Execute) {
            /* Update company infrastructure counts. */
            for rtt in [RoadTramType::RTT_ROAD, RoadTramType::RTT_TRAM] {
                let rt = self.get_road_type(tile, rtt);
                self.update_company_road_infrastructure(
                    rt,
                    self.get_road_owner(tile, rtt),
                    -(ROAD_STOP_TRACKBIT_FACTOR as i32),
                );
            }
            let owner = CompanyID((self.world().station_owner)(wp));
            (self.s.station_infrastructure)(owner.0, u32::MAX);
            (self.command.infrastructure_dirty)(owner.0);
            let specindex = (self.s.custom_road_index)(tile.0);
            (self.s.close_stop_inspect)(tile.0);
            crate::ground::clear_square(tile.0, self.shared);
            self.after_remove_tile(wp, tile);
            (self.s.remove_stop_tile_data)(wp, tile.0);
            if specindex as i32 != replacement_spec_index {
                unsafe {
                    crate::newgrf_roadstop::openttd_rust_roadstop_deallocate(
                        wp,
                        specindex as u8,
                        self.s.roadstop_resolver,
                    );
                }
            }
            if replacement_spec_index < 0 {
                // `MakeRoadWaypointStationAreaSmaller`.
                let road_waypoint_area = self.make_station_area_smaller(
                    Self::area((self.s.road_waypoint_tile_area)(wp)),
                    |tile| self.tile_belongs_to_road_waypoint_station(wp, tile),
                );
                (self.s.set_road_waypoint_area)(wp, road_waypoint_area);
                self.update_station_sign_coord(wp);
                /* if we deleted the whole waypoint, delete the road facility. */
                if road_waypoint_area.tile == TileIndex::INVALID.0 {
                    (self.s.reset_facilities)(
                        wp,
                        (StationFacilities::from_element(StationFacility::BusStop)
                            | StationFacilities::from_element(StationFacility::TruckStop))
                        .0,
                    );
                    (self.s.facility_widget)(wp, StationType::Bus.0);
                    (self.s.virtual_coord)(wp);
                    self.delete_station_if_empty(wp);
                }
            }
        }
        cost.set(if spec.is_null() {
            crate::company::price(usize::from(Price::PR_CLEAR_STATION_TRUCK.0))
        } else {
            crate::newgrf_roadstop::get_clear_cost(self.s, spec, Price::PR_CLEAR_STATION_TRUCK)
        });
    }
    /// `RemoveGenericRoadStop`: remove a tile area of road stop or road
    /// waypoints.
    fn remove_generic_road_stop(&self, context: RoadStopRemoval<'_>) {
        // `road_type[RTT_ROAD]`, `road_type[RTT_TRAM]` and the owner pair.
        const RTT_ROAD: usize = RoadTramType::RTT_ROAD.0 as usize;
        const RTT_TRAM: usize = RoadTramType::RTT_TRAM.0 as usize;
        let RoadStopRemoval {
            cost,
            ret,
            last_error,
            flags,
            roadstop_area,
            road_waypoint,
            remove_road,
        } = context;
        let world = crate::map_func::World::new(self.shared);
        cost.set(0);
        last_error.error(self.s.strings.no_station);
        let mut had_success = false;
        for cur_tile in roadstop_area.tiles(self.map()) {
            let cur_tile = TileIndex(cur_tile);
            /* Make sure the specified tile is a road stop of the correct type */
            if TileType((self.shared.tile_type)(cur_tile.0)) != TileType::MP_STATION
                || !world.any_road_stop_tile(cur_tile.0)
                || (StationType((self.shared.station_type)(cur_tile.0).0)
                    == StationType::RoadWaypoint)
                    != road_waypoint
            {
                continue;
            }
            /* Save information on to-be-restored roads before the stop is removed. */
            let mut road_bits = RoadBits::ROAD_NONE;
            let mut road_type = [RoadType::INVALID_ROADTYPE; 2];
            let mut road_owner = [CompanyID::NONE; 2];
            if world.drive_through_stop_tile(cur_tile.0) {
                for rtt in [RoadTramType::RTT_ROAD, RoadTramType::RTT_TRAM] {
                    let i = usize::from(rtt.0);
                    road_type[i] = self.get_road_type(cur_tile, rtt);
                    if road_type[i] == RoadType::INVALID_ROADTYPE {
                        continue;
                    }
                    road_owner[i] = self.get_road_owner(cur_tile, rtt);
                    /* If we don't want to preserve our roads then restore only roads of others. */
                    if remove_road && road_owner[i] == CompanyID((self.shared.current_company)()) {
                        road_type[i] = RoadType::INVALID_ROADTYPE;
                    }
                }
                road_bits = crate::road_func::axis_to_road_bits(
                    world.get_drive_through_stop_axis(cur_tile),
                    |ok| crate::movement::native_assert(self.shared, ok),
                );
            }
            if road_waypoint {
                self.remove_road_waypoint_stop(ret, cur_tile, flags, -1);
            } else {
                self.remove_road_stop(ret, cur_tile, flags, -1);
            }
            if !ret.ok() {
                last_error.take(ret);
                continue;
            }
            cost.add_money(ret.money());
            had_success = true;
            /* Restore roads. */
            if flags.test(DoCommandFlag::Execute)
                && (road_type[RTT_ROAD] != RoadType::INVALID_ROADTYPE
                    || road_type[RTT_TRAM] != RoadType::INVALID_ROADTYPE)
            {
                let town =
                    crate::town::spatial::closest_town_from_tile(self.shared, cur_tile, u32::MAX);
                let authority = unsafe { &*self.s.authority };
                let town = (authority.town_index)(town);
                (self.rail().make_road)(
                    cur_tile.0,
                    crate::rail_cmd::CrossingRoad {
                        bits: road_bits.0,
                        road_type: road_type[RTT_ROAD].0,
                        tram_type: road_type[RTT_TRAM].0,
                        town,
                        road_owner: road_owner[RTT_ROAD].0,
                        tram_owner: road_owner[RTT_TRAM].0,
                    },
                );
                /* Update company infrastructure counts. */
                let count = road_bits.0.count_ones() as i32;
                self.update_company_road_infrastructure(
                    road_type[RTT_ROAD],
                    road_owner[RTT_ROAD],
                    count,
                );
                self.update_company_road_infrastructure(
                    road_type[RTT_TRAM],
                    road_owner[RTT_TRAM],
                    count,
                );
            }
        }
        if !had_success {
            cost.take(last_error);
        }
    }
    /// `CmdRemoveRoadStop`: remove bus or truck stops.
    fn cmd_remove_road_stop(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        error: Cost<'_>,
        flags: DoCommandFlags,
        args: CmdRemoveRoadStop,
    ) {
        let CmdRemoveRoadStop {
            tile,
            width,
            height,
            stop_type,
            remove_road,
        } = args;
        if stop_type.0 >= RoadStopType::End.0 {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        /* Check for incorrect width / height. */
        if width == 0 || height == 0 {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        /* Check if the first tile and the last tile are valid */
        if !self.first_and_last_tile_valid(tile, u16::from(width), u16::from(height)) {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        /* Bankrupting company is not supposed to remove roads, there may be road vehicles. */
        if remove_road && flags.test(DoCommandFlag::Bankrupt) {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        let roadstop_area = TileArea {
            tile: tile.0,
            w: u16::from(width),
            h: u16::from(height),
        };
        self.remove_generic_road_stop(RoadStopRemoval {
            cost,
            ret,
            last_error: error,
            flags,
            roadstop_area,
            road_waypoint: false,
            remove_road,
        });
    }
    /// `CmdRemoveFromRoadWaypoint`: remove road waypoints.
    fn cmd_remove_from_road_waypoint(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        error: Cost<'_>,
        flags: DoCommandFlags,
        args: CmdRemoveFromRoadWaypoint,
    ) {
        let CmdRemoveFromRoadWaypoint { start, mut end } = args;
        if end.0 == 0 {
            end = start;
        }
        let map_size = self.shared.map_size();
        if start.0 >= map_size || end.0 >= map_size {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        let roadstop_area = TileArea::between(self.map(), start.0, end.0);
        self.remove_generic_road_stop(RoadStopRemoval {
            cost,
            ret,
            last_error: error,
            flags,
            roadstop_area,
            road_waypoint: true,
            remove_road: false,
        });
    }
}

/// `CmdRemoveRoadStop`'s arguments.
#[derive(Clone, Copy)]
struct CmdRemoveRoadStop {
    /// Northernmost tile of the removal area.
    tile: TileIndex,
    /// Width of the removal area.
    width: u8,
    /// Height of the removal area.
    height: u8,
    /// Type of stop (bus/truck).
    stop_type: RoadStopType,
    /// Remove roads of drive-through stops?
    remove_road: bool,
}
/// `CmdRemoveFromRoadWaypoint`'s arguments.
#[derive(Clone, Copy)]
struct CmdRemoveFromRoadWaypoint {
    /// Tile of road waypoint piece to remove.
    start: TileIndex,
    /// Other edge of the rect to remove.
    end: TileIndex,
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
    c.cmd_build_road_stop(
        c.c(cost),
        c.c(ret),
        DoCommandFlags(flags),
        CmdBuildRoadStop {
            tile: TileIndex(tile),
            width,
            length,
            stop_type: RoadStopType(stop_type),
            is_drive_through: drive,
            ddir: DiagDirection(dir),
            rt: RoadType(rt),
            spec_class: RoadStopClassID(class),
            spec_index: index,
            station_to_join: StationID(target),
            adjacent,
        },
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
    let (cost, ret, error, flags) = (c.c(cost), c.c(ret), c.c(error), DoCommandFlags(flags));
    if waypoint {
        c.cmd_remove_from_road_waypoint(
            cost,
            ret,
            error,
            flags,
            CmdRemoveFromRoadWaypoint {
                start: TileIndex(start),
                end: TileIndex(end),
            },
        );
    } else {
        c.cmd_remove_road_stop(
            cost,
            ret,
            error,
            flags,
            CmdRemoveRoadStop {
                tile: TileIndex(start),
                width,
                height,
                stop_type: RoadStopType(stop_type),
                remove_road,
            },
        );
    }
}

/// `CmdBuildAirport`'s arguments.
#[derive(Clone, Copy)]
struct CmdBuildAirport {
    /// Tile where airport will be built.
    tile: TileIndex,
    /// Airport type, @see airport.h.
    airport_type: u8,
    /// Airport layout.
    layout: u8,
    /// Station ID to join (`NEW_STATION` if build new one).
    station_to_join: StationID,
    /// Allow airports directly adjacent to other airports.
    allow_adjacent: bool,
}

impl Ctx<'_> {
    /// `AirportTileTableIterator` over a layout's tiles from `tile`.
    fn airport_tiles(
        &self,
        spec: *const AirportSpec,
        layout: u8,
        tile: TileIndex,
    ) -> impl Iterator<Item = AirportTile> + '_ {
        let count = (self.s.airport_layout)(spec, layout).size;
        (0..count).map(move |index| (self.s.airport_layout_tile)(spec, layout, tile.0, index))
    }
    /// `AirportGetNearestTown`: finds the town nearest to given airport. Based
    /// on minimal manhattan distance to any airport's tile. If two towns have
    /// the same distance, town with lower index is returned. Returns the town
    /// and `mindist`.
    fn airport_get_nearest_town(
        &self,
        spec: *const AirportSpec,
        rotation: Direction,
        tile: TileIndex,
        it: impl Iterator<Item = TileIndex>,
    ) -> (*const crate::services::TownHandle, u32) {
        let map = self.map();
        let mut nearest = std::ptr::null();
        let size = (self.s.airport_size)(spec);
        let (width, height) = if rotation == Direction::DIR_E || rotation == Direction::DIR_W {
            (size.h, size.w)
        } else {
            (size.w, size.h)
        };
        let perimeter_min_x = map.tile_x(tile.0);
        let perimeter_min_y = map.tile_y(tile.0);
        let perimeter_max_x = perimeter_min_x + u32::from(width) - 1;
        let perimeter_max_y = perimeter_min_y + u32::from(height) - 1;
        let mut mindist = u32::MAX - 1; // prevent overflow
        let authority = unsafe { &*self.s.authority };
        for cur_tile in it {
            let (x, y) = (map.tile_x(cur_tile.0), map.tile_y(cur_tile.0));
            if x != perimeter_min_x
                && x != perimeter_max_x
                && y != perimeter_min_y
                && y != perimeter_max_y
            {
                continue;
            }
            let t = crate::town::spatial::calc_closest_town_from_tile(
                self.shared,
                cur_tile,
                mindist.wrapping_add(1),
            );
            if t.is_null() {
                continue;
            }
            let xy = unsafe { crate::town::position((self.shared.town_owner)(t)) }.0;
            let dist = map.distance_manhattan(xy, cur_tile.0);
            if dist == mindist && (authority.town_index)(t) < (authority.town_index)(nearest) {
                nearest = t;
            }
            if dist < mindist {
                nearest = t;
                mindist = dist;
            }
        }
        (nearest, mindist)
    }
    /// `AirportGetNearestTown(const Station *st, uint &mindist)`: finds the
    /// town nearest to given existing airport.
    fn airport_get_nearest_town_of(
        &self,
        st: *mut BaseStation,
    ) -> (*const crate::services::TownHandle, u32) {
        let airport = Self::area((self.s.airport_area)(st));
        let rotation = Direction((self.s.airport_rotation)(st));
        let id = (self.links().graphs.station_id)(st);
        // `AirportTileIterator(st)`: the airport area's tiles that belong to it.
        self.airport_get_nearest_town(
            crate::newgrf_airport::station_spec(st, unsafe { &*self.s.airport_runtime }),
            rotation,
            TileIndex(airport.tile),
            airport.tiles(self.map()).map(TileIndex).filter(|&t| {
                crate::station_base::tile_belongs_to_airport(self.shared, StationID(id), t)
            }),
        )
    }
    /// `GetAirportNoiseLevelForDistance`: get a possible noise reduction factor
    /// based on distance from town center. The further you get, the less noise
    /// you generate.
    fn get_airport_noise_level_for_distance(&self, spec: *const AirportSpec, distance: u32) -> u8 {
        let noise_level = (self.s.airport_noise)(spec);
        /* 0 cannot be accounted, and 1 is the lowest that can be reduced from town.
         * So no need to go any further*/
        if noise_level < 2 {
            return noise_level;
        }
        let authority = unsafe { &*self.s.authority };
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
        if noise_reduction >= u32::from(noise_level) {
            1
        } else {
            (u32::from(noise_level) - noise_reduction) as u8
        }
    }
    /// `UpdateAirportsNoise`: recalculate the noise generated by the airports
    /// of each town.
    fn update_airports_noise(&self) {
        let mut from = 0;
        loop {
            let id = (self.shared.town_next_id)(from);
            if id == u32::MAX {
                break;
            }
            from = id + 1;
            let owner = (self.shared.town_owner_by_id)(id);
            let fields = unsafe { crate::town::openttd_rust_town_simulation(owner) };
            unsafe {
                (&raw mut (*fields).noise_reached).write(0);
            }
        }
        for id in 0..(self.world().pool_size)() {
            let st = self.full_station(StationID(id as u16));
            if st.is_null() {
                continue;
            }
            if (self.s.airport_tile)(st) != TileIndex::INVALID.0
                && (self.s.airport_type)(st) != AT_OILRIG
            {
                let (nearest, dist) = self.airport_get_nearest_town_of(st);
                let spec =
                    crate::newgrf_airport::station_spec(st, unsafe { &*self.s.airport_runtime });
                let noise = self.get_airport_noise_level_for_distance(spec, dist);
                let noise_reached = self.noise_reached(nearest);
                unsafe {
                    noise_reached.write(noise_reached.read().wrapping_add(u16::from(noise)));
                }
            }
        }
    }
    /// `t->noise_reached`, for immediate access only.
    fn noise_reached(&self, t: *const crate::services::TownHandle) -> *mut u16 {
        let owner = (self.shared.town_owner)(t);
        let fields = unsafe { crate::town::openttd_rust_town_simulation(owner) };
        unsafe { &raw mut (*fields).noise_reached }
    }
    /// `CmdBuildAirport`'s check whether the local authority would allow a new
    /// airport; `STR_NULL` when it would, otherwise the refusal and its town.
    fn check_local_authority_allows_airport(
        &self,
        tile: TileIndex,
        nearest: *const crate::services::TownHandle,
        noise_reached: *mut u16,
        newnoise_level: u32,
    ) -> (StringID, *const crate::services::TownHandle) {
        let mut authority_refuse_message = StringID::STR_NULL;
        let authority = unsafe { &*self.s.authority };
        let mut authority_refuse_town = std::ptr::null();
        if unsafe { (*self.s.shared).settings().station_noise.read() } {
            /* do not allow to build a new airport if this raise the town noise over the maximum allowed by town */
            if u32::from(unsafe { noise_reached.read() }) + newnoise_level
                > u32::from((self.s.town_max_noise)(nearest))
            {
                authority_refuse_message = StringID(self.s.strings.refuse_noise);
                authority_refuse_town = nearest;
            }
        } else if TownCouncilAttitudes((authority.council_tolerance)())
            != TownCouncilAttitudes::TOWN_COUNCIL_PERMISSIVE
        {
            let t = crate::town::spatial::closest_town_from_tile(self.shared, tile, u32::MAX);
            let mut num = 0_u32;
            for id in 0..(self.world().pool_size)() {
                let st = self.full_station(StationID(id as u16));
                if st.is_null() {
                    continue;
                }
                if (self.s.station_town)(st) == t
                    && StationFacilities((self.world().facilities)(st))
                        .test(StationFacility::Airport)
                    && (self.s.airport_type)(st) != AT_OILRIG
                {
                    num += 1;
                }
            }
            if num >= 2 {
                authority_refuse_message = StringID(self.s.strings.refuse_airport);
                authority_refuse_town = t;
            }
        }
        (authority_refuse_message, authority_refuse_town)
    }
    /// `CmdBuildAirport`'s execution over the layout's tiles: make each
    /// airport tile, then trigger the built animation.
    fn make_airport_tiles(
        &self,
        st: *mut BaseStation,
        spec: *const AirportSpec,
        layout: u8,
        tile: TileIndex,
        (owner, index): (CompanyID, StationID),
    ) {
        for iter in self.airport_tiles(spec, layout, tile) {
            (self.s.make_airport)(
                iter.tile,
                owner.0,
                index.0,
                iter.gfx,
                crate::water_map::WaterClass::Invalid.0,
            );
            (self.s.set_random_bits)(
                iter.tile,
                gb((self.shared.random)(self.shared.context), 0, 4) as u8,
            );
            self.area_add((self.s.airport_area)(st), TileIndex(iter.tile));
            if (self.s.airport_animated)(crate::newgrf_airport::tile_spec_by_gfx(
                iter.gfx,
                unsafe { &*self.s.airport_runtime },
            )) {
                crate::animated_tile::openttd_rust_animation_add(iter.tile, true);
            }
        }
        /* Only call the animation trigger after all tiles have been built */
        for iter in self.airport_tiles(spec, layout, tile) {
            crate::animated_tile::airport_built(st, iter.tile);
        }
    }
    /// `CmdBuildAirport`: place an Airport.
    fn cmd_build_airport(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        flags: DoCommandFlags,
        args: CmdBuildAirport,
    ) {
        let CmdBuildAirport {
            tile,
            airport_type,
            layout,
            mut station_to_join,
            allow_adjacent,
        } = args;
        crate::witness::hit(crate::witness::STATION_CMD_BUILD_AIRPORT);
        let reuse = station_to_join != StationID::NEW;
        if !reuse {
            station_to_join = StationID::INVALID;
        }
        let distant_join = station_to_join != StationID::INVALID;
        if distant_join
            && (!(self.s.distant_join)() || self.full_station(station_to_join).is_null())
        {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        if u16::from(airport_type) >= NUM_AIRPORTS {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        self.check_if_authority_allows_new_station(cost, tile, flags);
        if !cost.ok() {
            return;
        }
        /* Check if a valid, buildable airport was chosen for construction */
        let runtime = unsafe { &*self.s.airport_runtime };
        let spec = crate::newgrf_airport::get(airport_type, runtime);
        if !crate::newgrf_airport::available(spec, runtime)
            || usize::from(layout) >= (self.s.airport_layout_count)(spec)
            || !crate::newgrf_airport::in_bounds(spec, layout, tile.0, runtime)
        {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        let rotation = Direction((self.s.airport_layout)(spec, layout).rotation);
        let size = (self.s.airport_size)(spec);
        let (w, h) = if rotation == Direction::DIR_E || rotation == Direction::DIR_W {
            (size.h, size.w)
        } else {
            (size.w, size.h)
        };
        let airport_area = TileArea {
            tile: tile.0,
            w: u16::from(w),
            h: u16::from(h),
        };
        let station_spread = (self.s.spread)();
        if w > station_spread || h > station_spread {
            return cost.error(self.s.strings.spread);
        }
        // `CheckFlatLandAirport`: checks if an airport can be built at the
        // given location and clear the area.
        let mut allowed_z = -1;
        cost.set(0);
        for tile_iter in self.airport_tiles(spec, layout, tile) {
            self.check_buildable(
                ret,
                TileIndex(tile_iter.tile),
                DiagDirections::EMPTY,
                &mut allowed_z,
                true,
                true,
            );
            if !ret.ok() {
                return cost.take(ret);
            }
            cost.add_money(ret.money());
            crate::landscape_cmd::clear_command(self.command, ret.0, tile_iter.tile, flags.0);
            if !ret.ok() {
                return cost.take(ret);
            }
            cost.add_money(ret.money());
        }
        /* The noise level is the noise from the airport and reduce it to account for the distance to the town center. */
        let (nearest, dist) = self.airport_get_nearest_town(
            spec,
            rotation,
            tile,
            self.airport_tiles(spec, layout, tile)
                .map(|t| TileIndex(t.tile)),
        );
        let newnoise_level = u32::from(self.get_airport_noise_level_for_distance(spec, dist));
        let noise_reached = self.noise_reached(nearest);
        let authority = unsafe { &*self.s.authority };
        /* Check if local auth would allow a new airport */
        let (authority_refuse_message, authority_refuse_town) =
            self.check_local_authority_allows_airport(tile, nearest, noise_reached, newnoise_level);
        if authority_refuse_message != StringID::STR_NULL {
            (self.cost.error_param)(
                cost.0,
                authority_refuse_message.0,
                u64::from((authority.town_index)(authority_refuse_town)),
            );
            return;
        }
        let mut st = self.find_joining_base_station(JoiningStation {
            out: ret,
            existing_station: StationID::INVALID,
            station_to_join,
            adjacent: allow_adjacent,
            ta: airport_area,
            waypoint: false,
            road: false,
            error_message: StringID(self.s.strings.remove_station),
        });
        if !ret.ok() {
            return cost.take(ret);
        }
        /* Distant join */
        if st.is_null() && distant_join {
            st = self.full_station(station_to_join);
        }
        let name_class =
            if (self.s.airport_airplanes)(crate::airport::get_airport(airport_type, unsafe {
                &*self.s.airport_runtime
            })) {
                StationNaming::STATIONNAMING_AIRPORT
            } else {
                StationNaming::STATIONNAMING_HELIPORT
            };
        st = self.build_station_part(ret, st, flags, reuse, airport_area, name_class);
        if !ret.ok() {
            return cost.take(ret);
        }
        if !st.is_null() && (self.s.airport_tile)(st) != TileIndex::INVALID.0 {
            return cost.error(self.s.strings.close_airport);
        }
        for _ in self.airport_tiles(spec, layout, tile) {
            cost.add_money(crate::company::price(usize::from(
                Price::PR_BUILD_STATION_AIRPORT.0,
            )));
        }
        if !flags.test(DoCommandFlag::Execute) {
            return;
        }
        /* Always add the noise, so there will be no need to recalculate when option toggles */
        unsafe {
            noise_reached.write(noise_reached.read().wrapping_add(newnoise_level as u16));
        }
        crate::station_lifecycle::add_facility(st, StationFacility::Airport, tile, unsafe {
            &*self.s.lifecycle
        });
        (self.s.set_airport_details)(st, airport_type, layout, rotation.0);
        self.before_add_rect(ret, st, airport_area, StationRectMode::ADD_TRY);
        let owner = CompanyID((self.world().station_owner)(st));
        let index = StationID((self.links().graphs.station_id)(st));
        self.make_airport_tiles(st, spec, layout, tile, (owner, index));
        // `UpdateAirplanesOnNewStation`.
        unsafe {
            crate::aircraft::openttd_rust_aircraft_replacement(self.s.aircraft, index.0);
        }
        (self.s.airport_infrastructure)(owner.0, 1);
        self.after_station_tile_set_change(st, true, StationType::Airport);
        (self.s.station_view_invalidate)(st, -1);
        if unsafe { (*self.s.shared).settings().station_noise.read() } {
            (self.s.town_view_dirty)(nearest);
        }
    }
    /// `RemoveAirport`: remove an airport.
    fn remove_airport(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        tile: TileIndex,
        flags: DoCommandFlags,
    ) {
        crate::witness::hit(crate::witness::STATION_CMD_REMOVE_AIRPORT);
        let st = self.full_station(StationID((self.shared.station_index)(tile.0)));
        if CompanyID((self.shared.current_company)()) != CompanyID::WATER {
            unsafe {
                self.command
                    .ownership(cost.0, (self.world().station_owner)(st), 0);
            }
            if !cost.ok() {
                return;
            }
        }
        cost.set(0);
        let index = (self.links().graphs.station_id)(st);
        let mut from = 0;
        while let Some(v) = (self.shared.vehicle_next)(from) {
            let a = (self.s.aircraft_vehicle)(v);
            from = a.id + 1;
            // `Aircraft::Iterate()`.
            if VehicleType(v.vehicle_type()) != VehicleType::VEH_AIRCRAFT
                || !crate::aircraft::is_normal_aircraft(v.subtype())
            {
                continue;
            }
            let targetairport = unsafe { (&raw const (*a.state).targetairport).read() };
            let state = unsafe { (&raw const (*a.state).state).read() };
            if targetairport == index && state != FLYING {
                return cost.error(self.s.strings.aircraft_in_way);
            }
        }
        if flags.test(DoCommandFlag::Execute) {
            let spec =
                || crate::newgrf_airport::station_spec(st, unsafe { &*self.s.airport_runtime });
            for i in 0..(self.s.hangar_count)(st, spec()) {
                let tile_cur = (self.s.hangar_tile)(st, i, spec());
                (self.s.close_hangar)(tile_cur);
            }
            /* The noise level is the noise from the airport and reduce it to account for the distance to the town center.
             * And as for construction, always remove it, even if the setting is not set, in order to avoid the
             * need of recalculation */
            let (nearest, dist) = self.airport_get_nearest_town_of(st);
            let noise = self.get_airport_noise_level_for_distance(spec(), dist);
            let noise_reached = self.noise_reached(nearest);
            unsafe {
                noise_reached.write(noise_reached.read().wrapping_sub(u16::from(noise)));
            }
            if unsafe { (*self.s.shared).settings().station_noise.read() } {
                (self.s.town_view_dirty)(nearest);
            }
        }
        let airport = Self::area((self.s.airport_area)(st));
        for tile_cur in airport.tiles(self.map()) {
            if !crate::station_base::tile_belongs_to_airport(
                self.shared,
                StationID(index),
                TileIndex(tile_cur),
            ) {
                continue;
            }
            unsafe {
                self.command.no_vehicle(ret.0, tile_cur);
            }
            if !ret.ok() {
                return cost.take(ret);
            }
            cost.add_money(crate::company::price(usize::from(
                Price::PR_CLEAR_STATION_AIRPORT.0,
            )));
            if flags.test(DoCommandFlag::Execute) {
                crate::ground::clear_square(tile_cur, self.shared);
                (self.s.close_airport_tile_inspect)(tile_cur);
            }
        }
        if flags.test(DoCommandFlag::Execute) {
            /* Clear the persistent storage. */
            (self.s.delete_airport_storage)(st);
            self.after_remove_rect(st, airport);
            (self.s.clear_airport)(st);
            (self.s.reset_facilities)(
                st,
                StationFacilities::from_element(StationFacility::Airport).0,
            );
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
    let it = std::iter::from_fn(|| {
        if started {
            (c.s.iterator_next)(iterator);
        }
        started = true;
        let tile = (c.s.iterator_tile)(iterator);
        if tile == TileIndex::INVALID.0 {
            None
        } else {
            Some(TileIndex(tile))
        }
    });
    let (town, mindist) =
        c.airport_get_nearest_town(spec, Direction(rotation), TileIndex(origin), it);
    unsafe {
        distance.write(mindist);
    }
    town
}
/// `UpdateAirportsNoise`: recalculate the noise generated by the airports of
/// each town.
/// # Safety
/// Tables and serialized station/town pools remain live throughout this entry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_update_airport_noise(s: *const CommandServicesTable) {
    unsafe { Ctx::new(s) }.update_airports_noise();
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
    c.cmd_build_airport(
        c.c(cost),
        c.c(ret),
        DoCommandFlags(flags),
        CmdBuildAirport {
            tile: TileIndex(tile),
            airport_type: airport,
            layout,
            station_to_join: StationID(target),
            allow_adjacent: adjacent,
        },
    );
}

/// `_dock_tileoffs_chkaround`.
const DOCK_TILEOFFS_CHKAROUND: [(i32, i32); 4] = [(-1, 0), (0, 0), (0, 0), (0, -1)];
/// `_dock_w_chk`.
const DOCK_W_CHK: [u16; 4] = [2, 1, 2, 1];
/// `_dock_h_chk`.
const DOCK_H_CHK: [u16; 4] = [1, 2, 1, 2];

/// `CmdBuildDock`'s arguments.
#[derive(Clone, Copy)]
struct CmdBuildDock {
    /// Tile where dock will be built.
    tile: TileIndex,
    /// Station ID to join (`NEW_STATION` if build new one).
    station_to_join: StationID,
    /// Allow docks directly adjacent to other docks.
    adjacent: bool,
}

impl Ctx<'_> {
    /// `t + TileOffsByDiagDir(d)`.
    fn tile_add_by_diag_dir(&self, t: TileIndex, d: DiagDirection) -> TileIndex {
        TileIndex(self.map().add_diag(t.0, d.0))
    }
    /// `IsPossibleDockingTile`.
    fn is_possible_docking_tile(&self, t: TileIndex) -> bool {
        match TileType((self.shared.tile_type)(t.0)) {
            TileType::MP_WATER => {
                // `IsLock(t) && GetLockPart(t) == LockPart::Middle`.
                if (self.shared.water_middle_lock)(t.0) {
                    return false;
                }
            }
            TileType::MP_RAILWAY | TileType::MP_STATION | TileType::MP_TUNNELBRIDGE => (),
            _ => return false,
        }
        crate::movement::water_tracks(self.shared, t.0, DiagDirection::INVALID_DIAGDIR.0)
            != u32::from(TrackBits::TRACK_BIT_NONE.0)
    }
    /// `CheckForDockingTile`.
    fn check_for_docking_tile(&self, t: TileIndex) {
        crate::witness::hit(crate::witness::STATION_CMD_CHECK_DOCKING);
        for d in DiagDirection::DIAGDIR_BEGIN.0..DiagDirection::DIAGDIR_END.0 {
            let tile = self.tile_add_by_diag_dir(t, DiagDirection(d));
            if !(self.shared.tile_valid)(tile.0) {
                continue;
            }
            let world = crate::map_func::World::new(self.shared);
            if world.is_dock_tile(tile) && world.is_dock_water_part(tile) {
                let st = self.full_station(StationID((self.shared.station_index)(tile.0)));
                self.area_add((self.s.docking_area)(st), t);
                (self.s.set_docking)(t.0, true);
            }
            if TileType((self.shared.tile_type)(tile.0)) == TileType::MP_INDUSTRY {
                let st = (self.s.neutral_for_tile)(tile.0);
                if !st.is_null() {
                    self.area_add((self.s.docking_area)(st), t);
                    (self.s.set_docking)(t.0, true);
                }
            }
            if TileType((self.shared.tile_type)(tile.0)) == TileType::MP_STATION
                && crate::map_func::World::new(self.shared).is_oil_rig(tile)
            {
                let st = self.full_station(StationID((self.shared.station_index)(tile.0)));
                self.area_add((self.s.docking_area)(st), t);
                (self.s.set_docking)(t.0, true);
            }
        }
    }
    /// `UpdateStationDockingTiles`.
    fn update_station_docking_tiles(&self, st: *mut BaseStation) {
        (self.s.docking_clear)(st);
        /* For neutral stations, start with the industry area instead of dock area */
        let industry = (self.s.associated_industry)(st);
        let area = if industry.is_null() {
            Self::area((self.s.ship_station_area)(st))
        } else {
            (self.s.industry_area)(industry)
        };
        if area.tile == TileIndex::INVALID.0 {
            return;
        }
        let map = self.map();
        let x = map.tile_x(area.tile);
        let y = map.tile_y(area.tile);
        /* Expand the area by a tile on each side while
         * making sure that we remain inside the map. */
        let x2 = (x + u32::from(area.w) + 1).min(map.size_x());
        let x1 = x.saturating_sub(1);
        let y2 = (y + u32::from(area.h) + 1).min(crate::map_func::World::new(self.shared).size_y());
        let y1 = y.saturating_sub(1);
        let ta = TileArea::between(map, map.tile_xy(x1, y1), map.tile_xy(x2 - 1, y2 - 1));
        for tile in ta.tiles(map) {
            let tile = TileIndex(tile);
            if (self.shared.tile_valid)(tile.0) && self.is_possible_docking_tile(tile) {
                self.check_for_docking_tile(tile);
            }
        }
    }
    /// `RemoveDockingTile`.
    fn remove_docking_tile(&self, t: TileIndex) {
        for d in DiagDirection::DIAGDIR_BEGIN.0..DiagDirection::DIAGDIR_END.0 {
            let tile = self.tile_add_by_diag_dir(t, DiagDirection(d));
            if !(self.shared.tile_valid)(tile.0) {
                continue;
            }
            match TileType((self.shared.tile_type)(tile.0)) {
                TileType::MP_STATION => {
                    let st = self.full_station(StationID((self.shared.station_index)(tile.0)));
                    if !st.is_null() {
                        self.update_station_docking_tiles(st);
                    }
                }
                TileType::MP_INDUSTRY => {
                    let neutral = (self.s.neutral_for_tile)(tile.0);
                    if !neutral.is_null() {
                        self.update_station_docking_tiles(neutral);
                    }
                }
                _ => (),
            }
        }
    }
    /// `ClearDockingTilesCheckingNeighbours`: clear docking tile status from
    /// tiles around a removed dock, if the tile has no neighbours which would
    /// keep it as a docking tile.
    fn clear_docking_tiles_checking_neighbours(&self, tile: TileIndex) {
        /* Clear and maybe re-set docking tile */
        for d in DiagDirection::DIAGDIR_BEGIN.0..DiagDirection::DIAGDIR_END.0 {
            let docking_tile = self.tile_add_by_diag_dir(tile, DiagDirection(d));
            if !(self.shared.tile_valid)(docking_tile.0) {
                continue;
            }
            if self.is_possible_docking_tile(docking_tile) {
                (self.s.set_docking)(docking_tile.0, false);
                self.check_for_docking_tile(docking_tile);
            }
        }
    }
    /// `CmdBuildDock`'s execution: add the dock to the station and make the
    /// dock tiles.
    fn make_dock_station(
        &self,
        ret: Cost<'_>,
        st: *mut BaseStation,
        tile: TileIndex,
        direction: DiagDirection,
        wc: WaterClass,
        dock_area: TileArea,
    ) {
        let world = crate::map_func::World::new(self.shared);
        self.area_add((self.s.ship_station_area)(st), tile);
        let flat_tile = self.tile_add_by_diag_dir(tile, direction);
        self.area_add((self.s.ship_station_area)(st), flat_tile);
        crate::station_lifecycle::add_facility(st, StationFacility::Dock, tile, unsafe {
            &*self.s.lifecycle
        });
        self.before_add_rect(ret, st, dock_area, StationRectMode::ADD_TRY);
        let owner = CompanyID((self.world().station_owner)(st));
        /* If the water part of the dock is on a canal, update infrastructure counts.
         * This is needed as we've cleared that tile before.
         * Clearing object tiles may result in water tiles which are already accounted for in the water infrastructure total.
         * See: MakeWaterKeepingClass() */
        if wc == WaterClass::Canal
            && !((self.shared.has_water_class)(flat_tile.0)
                && WaterClass(world.water_class(flat_tile.0)) == WaterClass::Canal
                && (self.shared.tile_owner)(flat_tile.0) == (self.shared.current_company)())
        {
            (self.s.water_infrastructure)(owner.0, 1);
        }
        (self.s.station_infrastructure)(owner.0, 2);
        (self.s.make_dock)(
            tile.0,
            owner.0,
            (self.links().graphs.station_id)(st),
            direction.0,
            wc.0,
        );
        self.update_station_docking_tiles(st);
        self.after_station_tile_set_change(st, true, StationType::Dock);
    }
    /// `CmdBuildDock`: build a dock/haven.
    fn cmd_build_dock(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        flags: DoCommandFlags,
        args: CmdBuildDock,
    ) {
        let CmdBuildDock {
            tile,
            mut station_to_join,
            adjacent,
        } = args;
        crate::witness::hit(crate::witness::STATION_CMD_BUILD_DOCK);
        let world = crate::map_func::World::new(self.shared);
        let reuse = station_to_join != StationID::NEW;
        if !reuse {
            station_to_join = StationID::INVALID;
        }
        let distant_join = station_to_join != StationID::INVALID;
        if distant_join
            && (!(self.s.distant_join)() || self.full_station(station_to_join).is_null())
        {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        let direction = get_inclined_slope_direction(Slope(world.tile_slope(tile.0)));
        if direction == DiagDirection::INVALID_DIAGDIR {
            return cost.error(self.s.strings.unsuitable);
        }
        let direction = reverse_diag_dir(direction, |ok| {
            crate::movement::native_assert(self.shared, ok);
        });
        /* Docks cannot be placed on rapids */
        if (self.shared.has_water_ground)(tile.0) {
            return cost.error(self.s.strings.unsuitable);
        }
        self.check_if_authority_allows_new_station(cost, tile, flags);
        if !cost.ok() {
            return;
        }
        // `IsDockBridgeAboveOk`.
        self.default_bridge_above_ok(cost, tile, StationType::Dock, StationGfx(direction.0));
        if !cost.ok() {
            return;
        }
        cost.set(crate::company::price(usize::from(
            Price::PR_BUILD_STATION_DOCK.0,
        )));
        crate::landscape_cmd::clear_command(self.command, ret.0, tile.0, flags.0);
        if !ret.ok() {
            return cost.take(ret);
        }
        cost.add_money(ret.money());
        let tile_cur = self.tile_add_by_diag_dir(tile, direction);
        if !(self.shared.has_water_ground)(tile_cur.0) || !world.tile_flat(tile_cur.0) {
            return cost.error(self.s.strings.unsuitable);
        }
        self.default_bridge_above_ok(
            ret,
            tile_cur,
            StationType::Dock,
            StationGfx(
                GFX_DOCK_BASE_WATER_PART
                    + diag_dir_to_axis(direction, |ok| {
                        crate::movement::native_assert(self.shared, ok);
                    })
                    .0,
            ),
        );
        if !ret.ok() {
            return cost.take(ret);
        }
        /* Get the water class of the water tile before it is cleared.*/
        let wc = WaterClass(world.water_class(tile_cur.0));
        let add_cost = !(self.shared.is_water_tile)(tile_cur.0);
        crate::landscape_cmd::clear_command(self.command, ret.0, tile_cur.0, flags.0);
        if !ret.ok() {
            return cost.take(ret);
        }
        if add_cost {
            cost.add_money(ret.money());
        }
        let tile_cur = self.tile_add_by_diag_dir(tile_cur, direction);
        if TileType((self.shared.tile_type)(tile_cur.0)) != TileType::MP_WATER
            || !world.tile_flat(tile_cur.0)
        {
            return cost.error(self.s.strings.unsuitable);
        }
        let (dx, dy) = DOCK_TILEOFFS_CHKAROUND[usize::from(direction.0)];
        let dock_area = TileArea {
            tile: tile.0.wrapping_add(self.map().diff(dx, dy)),
            w: DOCK_W_CHK[usize::from(direction.0)],
            h: DOCK_H_CHK[usize::from(direction.0)],
        };
        /* middle */
        let mut st = self.find_joining_base_station(JoiningStation {
            out: ret,
            existing_station: StationID::INVALID,
            station_to_join,
            adjacent,
            ta: dock_area,
            waypoint: false,
            road: false,
            error_message: StringID(self.s.strings.remove_station),
        });
        if !ret.ok() {
            return cost.take(ret);
        }
        /* Distant join */
        if st.is_null() && distant_join {
            st = self.full_station(station_to_join);
        }
        st = self.build_station_part(
            ret,
            st,
            flags,
            reuse,
            dock_area,
            StationNaming::STATIONNAMING_DOCK,
        );
        if !ret.ok() {
            return cost.take(ret);
        }
        if !flags.test(DoCommandFlag::Execute) {
            return;
        }
        self.make_dock_station(ret, st, tile, direction, wc, dock_area);
    }
    /// `FindDockLandPart`: find the part of a dock that is land-based.
    fn find_dock_land_part(&self, t: TileIndex) -> TileIndex {
        let world = crate::map_func::World::new(self.shared);
        world.check(world.is_dock_tile(t));
        let gfx = world.get_station_gfx(t);
        if gfx.0 < GFX_DOCK_BASE_WATER_PART {
            return t;
        }
        for d in DiagDirection::DIAGDIR_BEGIN.0..DiagDirection::DIAGDIR_END.0 {
            let tile = self.tile_add_by_diag_dir(t, DiagDirection(d));
            if !(self.shared.tile_valid)(tile.0) {
                continue;
            }
            if !world.is_dock_tile(tile) {
                continue;
            }
            if world.get_station_gfx(tile).0 < GFX_DOCK_BASE_WATER_PART
                && self.tile_add_by_diag_dir(tile, world.get_dock_direction(tile)) == t
            {
                return tile;
            }
        }
        TileIndex::INVALID
    }
    /// `RemoveDock`'s update of all ships going to the station's removed dock.
    fn update_ships_for_removed_dock(&self, st: *mut BaseStation) {
        let index = (self.links().graphs.station_id)(st);
        let mut from = 0;
        while let Some(s) = (self.shared.vehicle_next)(from) {
            let identity = (self.s.aircraft_vehicle)(s);
            from = identity.id + 1;
            /* Find all ships going to our dock. */
            if VehicleType(s.vehicle_type()) != VehicleType::VEH_SHIP
                || (self.vehicle().order_destination)(s) != index
            {
                continue;
            }
            /* Find ships that are marked as "loading" but are no longer on a
             * docking tile. Force them to leave the station (as they were loading
             * on the removed dock). */
            if OrderType((self.vehicle().order_type)(s)) == OrderType::OT_LOADING {
                let tile = s.tile();
                if !((self.shared.docking_tile)(tile)
                    && unsafe {
                        crate::ship_control::openttd_rust_ship_is_destination_abi(
                            tile,
                            u32::from(index),
                            self.s.ship,
                            self.s.shared,
                        ) != 0
                    })
                {
                    crate::vehicle::visits::leave_station(s, self.vehicle().visits());
                }
            }
            /* If we no longer have a dock, mark the order as invalid and send
             * the ship to the next order (or, if there is none, make it
             * wander the world). */
            if OrderType((self.vehicle().order_type)(s)) == OrderType::OT_GOTO_STATION
                && !StationFacilities((self.world().facilities)(st)).test(StationFacility::Dock)
            {
                let dest = unsafe {
                    crate::ship_control::openttd_rust_ship_station_destination(
                        identity.id,
                        u32::from(index),
                        s.0.as_ptr(),
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
    /// `RemoveDock`: remove a dock.
    fn remove_dock(&self, cost: Cost<'_>, tile: TileIndex, flags: DoCommandFlags) {
        crate::witness::hit(crate::witness::STATION_CMD_REMOVE_DOCK);
        let st = self.full_station(StationID((self.shared.station_index)(tile.0)));
        let owner = CompanyID((self.world().station_owner)(st));
        unsafe {
            self.command.ownership(cost.0, owner.0, 0);
        }
        if !cost.ok() {
            return;
        }
        if !crate::map_func::World::new(self.shared).is_dock_tile(tile) {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        let tile1 = self.find_dock_land_part(tile);
        if tile1 == TileIndex::INVALID {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        let tile2 = self.tile_add_by_diag_dir(
            tile1,
            crate::map_func::World::new(self.shared).get_dock_direction(tile1),
        );
        unsafe {
            self.command.no_vehicle(cost.0, tile1.0);
        }
        if cost.ok() {
            unsafe {
                self.command.no_vehicle(cost.0, tile2.0);
            }
        }
        if !cost.ok() {
            return;
        }
        if flags.test(DoCommandFlag::Execute) {
            crate::ground::clear_square(tile1.0, self.shared);
            (self.shared.mark_dirty)(tile1.0);
            // `MakeWaterKeepingClass`.
            unsafe {
                crate::water::restore(
                    tile2.0,
                    owner.0,
                    self.shared,
                    &*(self.shared.water_services)(),
                    &*self.s.restoration,
                );
            }
            self.after_remove_tile(st, tile1);
            self.after_remove_tile(st, tile2);
            // `MakeShipStationAreaSmaller`.
            let ship_station = self
                .make_station_area_smaller(Self::area((self.s.ship_station_area)(st)), |tile| {
                    self.tile_belongs_to_ship_station(st, tile)
                });
            (self.s.set_ship_area)(st, ship_station);
            self.update_station_docking_tiles(st);
            if ship_station.tile == TileIndex::INVALID.0 {
                (self.s.set_ship_area)(
                    st,
                    TileArea {
                        tile: TileIndex::INVALID.0,
                        w: 0,
                        h: 0,
                    },
                );
                (self.s.docking_clear)(st);
                (self.s.reset_facilities)(
                    st,
                    StationFacilities::from_element(StationFacility::Dock).0,
                );
                (self.s.orders_dirty)();
            }
            (self.s.station_infrastructure)(owner.0, 2_u32.wrapping_neg());
            self.after_station_tile_set_change(st, false, StationType::Dock);
            self.clear_docking_tiles_checking_neighbours(tile1);
            self.clear_docking_tiles_checking_neighbours(tile2);
            self.update_ships_for_removed_dock(st);
        }
        cost.set(crate::company::price(usize::from(
            Price::PR_CLEAR_STATION_DOCK.0,
        )));
    }
    /// `BuildOilRig`.
    fn build_oil_rig(&self, ret: Cost<'_>, tile: TileIndex) {
        crate::witness::hit(crate::witness::STATION_CMD_BUILD_OILRIG);
        if !(self.s.station_available)() {
            debug!(
                misc,
                0, "Can't allocate station for oilrig at 0x{:X}, reverting to oilrig only", tile.0
            );
            return;
        }
        let st = (self.s.new_station)(tile.0);
        crate::station::spatial::insert(self.shared, st);
        (self.s.set_town)(
            st,
            crate::town::spatial::closest_town_from_tile(self.shared, tile, u32::MAX),
        );
        (self.s.set_string_id)(
            st,
            crate::station_lifecycle::generate_station_name(
                st,
                tile,
                StationNaming::STATIONNAMING_OILRIG,
                unsafe { &*self.s.lifecycle },
            )
            .0,
        );
        /* Mark industry as associated both ways */
        (self.s.associate_industry)(st, tile.0);
        crate::animated_tile::openttd_rust_animation_delete(tile.0, false);
        (self.s.make_oilrig)(
            tile.0,
            (self.links().graphs.station_id)(st),
            crate::map_func::World::new(self.shared).water_class(tile.0),
        );
        (self.s.set_station_owner)(st, CompanyID::NONE.0);
        (self.s.set_airport_kind_rotation)(st, AT_OILRIG, Direction::DIR_N.0);
        self.area_add((self.s.airport_area)(st), tile);
        self.area_add((self.s.ship_station_area)(st), tile);
        (self.s.set_facilities)(
            st,
            (StationFacilities::from_element(StationFacility::Airport)
                | StationFacilities::from_element(StationFacility::Dock))
            .0,
        );
        (self.s.set_build_date)(st, crate::timer_game_common::calendar_date().0);
        self.update_station_docking_tiles(st);
        self.before_add_tile(ret, st, tile, StationRectMode::ADD_FORCE);
        (self.s.virtual_coord)(st);
        /* An industry tile has now been replaced with a station tile, this may change the overlap between station catchments and industry tiles.
         * Recalculate the station catchment for all stations currently in the industry's nearby list.
         * Clear the industry's station nearby list first because Station::RecomputeCatchment cannot remove nearby industries in this case. */
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
                    crate::station_service::update_station_acceptance(
                        near.station,
                        true,
                        &*self.s.station,
                    );
                }
            }
        }
        self.recompute_catchment(st, false);
        unsafe {
            crate::station_service::update_station_acceptance(st, false, &*self.s.station);
        }
    }
    /// `DeleteOilRig`.
    fn delete_oil_rig(&self, tile: TileIndex) {
        crate::witness::hit(crate::witness::STATION_CMD_DELETE_OILRIG);
        let st = self.full_station(StationID((self.shared.station_index)(tile.0)));
        // `MakeWaterKeepingClass(tile, OWNER_NONE)`.
        unsafe {
            crate::water::restore(
                tile.0,
                CompanyID::NONE.0,
                self.shared,
                &*(self.shared.water_services)(),
                &*self.s.restoration,
            );
        }
        /* The oil rig station is not supposed to be shared with anything else */
        let expected_facility = StationFacilities::from_element(StationFacility::Airport)
            | StationFacilities::from_element(StationFacility::Dock);
        assert!(
            StationFacilities((self.world().facilities)(st)) == expected_facility
                && (self.s.airport_type)(st) == AT_OILRIG
        );
        let industry = (self.s.associated_industry)(st);
        if !industry.is_null() && (self.s.industry_neutral)(industry) == st {
            /* Don't leave dangling neutral station pointer */
            (self.s.clear_industry_neutral)(industry);
        }
        (self.world().delete_station)(st);
    }
    /// `HasStationInUse`: tests whether the company's vehicles have this
    /// station in orders. If `include_company`, only check vehicles of
    /// `company`, otherwise only check vehicles of other companies.
    fn has_station_in_use(
        &self,
        station: StationID,
        include_company: bool,
        company: CompanyID,
    ) -> bool {
        for id in 0..(self.links().order_pool_size)() {
            let orderlist = (self.links().order_list)(id as u16);
            if orderlist.is_null() {
                continue;
            }
            let v = unsafe { (self.links().order_vehicle)(orderlist).unwrap_unchecked() };
            if (CompanyID(v.owner()) == company) != include_company {
                continue;
            }
            for index in 0..(self.links().order_count)(orderlist) {
                let order = (self.links().order_destination)(orderlist, index);
                if order.destination == station.0
                    && (order.kind == OrderType::OT_GOTO_STATION
                        || order.kind == OrderType::OT_GOTO_WAYPOINT)
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
    c.cmd_build_dock(
        c.c(cost),
        c.c(ret),
        DoCommandFlags(flags),
        CmdBuildDock {
            tile: TileIndex(tile),
            station_to_join: StationID(target),
            adjacent,
        },
    );
}

/// # Safety
/// Tables and the station shell remain live; no owner borrow spans reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_update_docking(
    s: *const CommandServicesTable,
    st: *mut BaseStation,
) {
    unsafe { Ctx::new(s) }.update_station_docking_tiles(st);
}
/// # Safety
/// Tables and serialized map/pools remain live for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_remove_docking_tile(
    s: *const CommandServicesTable,
    tile: u32,
) {
    unsafe { Ctx::new(s) }.remove_docking_tile(TileIndex(tile));
}
/// # Safety
/// Tables and the original valid tile remain live for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_clear_docking_neighbours(
    s: *const CommandServicesTable,
    tile: u32,
) {
    unsafe { Ctx::new(s) }.clear_docking_tiles_checking_neighbours(TileIndex(tile));
}
/// Shared direct Rust entry for the original guarded docking registration.
/// # Safety
/// Services and the original valid tile remain live for the synchronous call.
pub(crate) unsafe fn check_possible_docking(s: *const CommandServicesTable, tile: TileIndex) {
    // SAFETY: the caller supplies the process-lifetime tables under the game-state mutex.
    let c = unsafe { Ctx::new(s) };
    if c.is_possible_docking_tile(tile) {
        c.check_for_docking_tile(tile);
    }
}
/// Raw adapter for `water.rs` and `rail_cmd.rs`, outside this pass; delegates
/// to `check_possible_docking`.
/// # Safety
/// As `check_possible_docking`.
pub(crate) unsafe fn check_possible_docking_raw(s: *const CommandServicesTable, tile: u32) {
    unsafe { check_possible_docking(s, TileIndex(tile)) };
}
/// # Safety
/// Tables and the original valid tile remain live for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_possible_docking_abi(
    s: *const CommandServicesTable,
    tile: u32,
) -> u8 {
    u8::from(unsafe { Ctx::new(s) }.is_possible_docking_tile(TileIndex(tile)))
}
/// # Safety
/// Tables and serialized map/pools remain live for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_check_docking(
    s: *const CommandServicesTable,
    tile: u32,
) {
    unsafe { Ctx::new(s) }.check_for_docking_tile(TileIndex(tile));
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
    c.build_oil_rig(c.c(ret), TileIndex(tile));
}
/// # Safety
/// Tables remain live; the canonical oilrig shell is destroyed during this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_delete_oilrig(
    s: *const CommandServicesTable,
    tile: u32,
) {
    unsafe { Ctx::new(s) }.delete_oil_rig(TileIndex(tile));
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
    u8::from(unsafe { Ctx::new(s) }.has_station_in_use(
        StationID(station),
        include_company,
        CompanyID(company),
    ))
}

impl Ctx<'_> {
    /// `ClearTile_Station`'s `DoCommandFlag::Auto` errors.
    fn auto_clear(&self, cost: Cost<'_>, tile: TileIndex) {
        let has_tram = || {
            crate::map_func::World::new(self.shared).asserted(|m| {
                crate::road_map::has_tile_road_type(
                    m,
                    tile.0,
                    crate::services::RoadTramType(RoadTramType::RTT_TRAM.0),
                )
            })
        };
        let error = match StationType((self.shared.station_type)(tile.0).0) {
            StationType::Rail => self.s.strings.demolish_railroad,
            StationType::RailWaypoint | StationType::RoadWaypoint => {
                self.s.strings.demolish_building
            }
            StationType::Airport => self.s.strings.demolish_airport,
            StationType::Truck => {
                if has_tram() {
                    self.s.strings.demolish_cargo_tram
                } else {
                    self.s.strings.demolish_truck
                }
            }
            StationType::Bus => {
                if has_tram() {
                    self.s.strings.demolish_passenger_tram
                } else {
                    self.s.strings.demolish_bus
                }
            }
            StationType::Buoy => self.s.strings.buoy_in_way,
            StationType::Dock => self.s.strings.demolish_dock,
            StationType::Oilrig => {
                (self.cost.error_param)(
                    cost.0,
                    self.s.strings.object_in_way,
                    u64::from(self.s.strings.oilrig_name),
                );
                return;
            }
            _ => StringID::INVALID_STRING_ID.0,
        };
        cost.error(error);
    }
    /// `CanRemoveRoadWithStop`: check if a drive-through road stop tile can be
    /// cleared. Road stops built on town-owned roads check the conditions that
    /// would allow clearing of the original road.
    fn can_remove_road_with_stop(&self, cost: Cost<'_>, tile: TileIndex, flags: DoCommandFlags) {
        /* Water flooding can always clear road stops. */
        if CompanyID((self.shared.current_company)()) == CompanyID::WATER {
            return cost.zero();
        }
        if self.get_road_type(tile, RoadTramType::RTT_TRAM) != RoadType::INVALID_ROADTYPE {
            let tram_owner = self.get_road_owner(tile, RoadTramType::RTT_TRAM);
            if tram_owner != CompanyID::NONE {
                unsafe {
                    self.command.ownership(cost.0, tram_owner.0, 0);
                }
                if !cost.ok() {
                    return;
                }
            }
        }
        if self.get_road_type(tile, RoadTramType::RTT_ROAD) != RoadType::INVALID_ROADTYPE {
            let road_owner = self.get_road_owner(tile, RoadTramType::RTT_ROAD);
            if road_owner == CompanyID::TOWN {
                let bits =
                    (self.shared.road_bits)(tile.0, u8::from(RoadTramType::RTT_ROAD.0), false);
                unsafe {
                    crate::road_cmd::openttd_rust_road_allow_remove(
                        (self.shared.road_commands)(),
                        cost.0,
                        tile.0,
                        bits,
                        CompanyID::TOWN.0,
                        u8::from(RoadTramType::RTT_ROAD.0),
                        flags.0,
                        true,
                    );
                }
                if !cost.ok() {
                    return;
                }
            } else if road_owner != CompanyID::NONE {
                unsafe {
                    self.command.ownership(cost.0, road_owner.0, 0);
                }
                if !cost.ok() {
                    return;
                }
            }
        }
        cost.zero();
    }
    /// `ClearTile_Station`: clear a single tile of a station.
    fn clear_tile_station(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        error: Cost<'_>,
        tile: TileIndex,
        flags: DoCommandFlags,
    ) {
        crate::witness::hit(crate::witness::STATION_CMD_CLEAR);
        if flags.test(DoCommandFlag::Auto) {
            return self.auto_clear(cost, tile);
        }
        match StationType((self.shared.station_type)(tile.0).0) {
            StationType::Rail => self.remove_rail_station(cost, ret, error, flags, tile, false),
            StationType::RailWaypoint => {
                self.remove_rail_station(cost, ret, error, flags, tile, true);
            }
            StationType::Airport => self.remove_airport(cost, ret, tile, flags),
            StationType::Truck | StationType::Bus => {
                if crate::map_func::World::new(self.shared).drive_through_stop_tile(tile.0) {
                    self.can_remove_road_with_stop(cost, tile, flags);
                    if !cost.ok() {
                        return;
                    }
                }
                self.remove_road_stop(cost, tile, flags, -1);
            }
            StationType::RoadWaypoint => {
                self.can_remove_road_with_stop(cost, tile, flags);
                if cost.ok() {
                    self.remove_road_waypoint_stop(cost, tile, flags, -1);
                }
            }
            StationType::Buoy => self.remove_buoy(cost, tile, flags),
            StationType::Dock => self.remove_dock(cost, tile, flags),
            _ => cost.error(StringID::INVALID_STRING_ID.0),
        }
    }
    /// `TerraformTile_Station`.
    fn terraform_tile_station(
        &self,
        cost: Cost<'_>,
        tile: TileIndex,
        flags: DoCommandFlags,
        z_new: i32,
        tileh_new: Slope,
    ) {
        crate::witness::hit(crate::witness::STATION_CMD_TERRAFORM);
        let world = crate::map_func::World::new(self.shared);
        if (self.shared.build_on_slopes)()
            && (self.shared.autoslope)()
            /* TODO: If you implement newgrf callback 149 'land slope check', you have to decide what to do with it here.
             *       TTDP does not call it.
             */
            && (self.shared.tile_max_z)(tile.0) == z_new.wrapping_add(get_slope_max_z(tileh_new))
        {
            let allowed = match StationType((self.shared.station_type)(tile.0).0) {
                StationType::RailWaypoint | StationType::Rail => (self.s.autoslope_axis)(
                    tile.0,
                    z_new,
                    tileh_new.0,
                    self.get_rail_station_axis(tile).0,
                ),
                StationType::Airport => true,
                StationType::Truck | StationType::Bus | StationType::RoadWaypoint => {
                    if world.drive_through_stop_tile(tile.0) {
                        (self.s.autoslope_axis)(
                            tile.0,
                            z_new,
                            tileh_new.0,
                            world.get_drive_through_stop_axis(tile).0,
                        )
                    } else {
                        (self.rail().entrance_edge)(
                            tile.0,
                            z_new,
                            tileh_new.0,
                            world.bay_dir(tile.0),
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
        crate::landscape_cmd::clear_command(self.command, cost.0, tile.0, flags.0);
    }
    /// `ChangeTileOwner_Station`'s removal of a drive-through road stop for a
    /// bankrupt owner: `CMD_REMOVE_FROM_ROAD_WAYPOINT` or `CMD_REMOVE_ROAD_STOP`.
    fn remove_drive_through_stop_for_bankrupt(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        error: Cost<'_>,
        tile: TileIndex,
    ) {
        let station_type = StationType((self.shared.station_type)(tile.0).0);
        self.nested(
            cost,
            tile,
            DoCommandFlags::from_element(DoCommandFlag::Execute)
                | DoCommandFlags::from_element(DoCommandFlag::Bankrupt),
            |flags| {
                if station_type == StationType::RoadWaypoint {
                    self.cmd_remove_from_road_waypoint(
                        cost,
                        ret,
                        error,
                        flags,
                        CmdRemoveFromRoadWaypoint {
                            start: tile,
                            end: tile,
                        },
                    );
                } else {
                    self.cmd_remove_road_stop(
                        cost,
                        ret,
                        error,
                        flags,
                        CmdRemoveRoadStop {
                            tile,
                            width: 1,
                            height: 1,
                            stop_type: if station_type == StationType::Truck {
                                RoadStopType::Truck
                            } else {
                                RoadStopType::Bus
                            },
                            remove_road: false,
                        },
                    );
                }
            },
        );
    }
    /// `ChangeTileOwner_Station`.
    fn change_tile_owner_station(&self, context: StationOwnerChange<'_>) {
        let StationOwnerChange {
            ownership,
            cost,
            ret,
            error,
            tile,
            old_owner,
            new_owner,
        } = context;
        crate::witness::hit(crate::witness::STATION_CMD_CHANGE_OWNER);
        let world = crate::map_func::World::new(self.shared);
        if world.any_road_stop_tile(tile.0) {
            for rtt in [RoadTramType::RTT_ROAD, RoadTramType::RTT_TRAM] {
                /* Update all roadtypes, no matter if they are present */
                if self.get_road_owner(tile, rtt) == old_owner {
                    let rt = self.get_road_type(tile, rtt);
                    if rt != RoadType::INVALID_ROADTYPE {
                        /* A drive-through road-stop has always two road bits. No need to dirty windows here, we'll redraw the whole screen anyway. */
                        (self.rail().road_infrastructure)(old_owner.0, rt.0, 2_u32.wrapping_neg());
                        if new_owner != CompanyID::INVALID {
                            (self.rail().road_infrastructure)(new_owner.0, rt.0, 2);
                        }
                    }
                    (self.shared.set_road_owner)(
                        tile.0,
                        u8::from(rtt.0),
                        if new_owner == CompanyID::INVALID {
                            CompanyID::NONE.0
                        } else {
                            new_owner.0
                        },
                    );
                }
            }
        }
        if !crate::tile_map::is_tile_owner(self.shared, tile.0, old_owner.0) {
            return;
        }
        if new_owner != CompanyID::INVALID {
            /* Update company infrastructure counts. Only do it here
             * if the new owner is valid as otherwise the clear
             * command will do it for us. No need to dirty windows
             * here, we'll redraw the whole screen anyway.*/
            let station_type = StationType((self.shared.station_type)(tile.0).0);
            /* Update counts for underlying infrastructure. */
            match station_type {
                StationType::Rail | StationType::RailWaypoint => {
                    if !self.is_station_tile_blocked(tile) {
                        let rt = world.rail_type(tile.0);
                        (self.rail().rail_infrastructure)(old_owner.0, rt, u32::MAX);
                        (self.rail().rail_infrastructure)(new_owner.0, rt, 1);
                    }
                }
                /* Road stops were already handled above. */
                StationType::Buoy | StationType::Dock
                    if world.water_class(tile.0) == crate::water_map::WaterClass::Canal.0 =>
                {
                    (self.s.water_infrastructure)(old_owner.0, u32::MAX);
                    (self.s.water_infrastructure)(new_owner.0, 1);
                }
                _ => (),
            }
            /* Update station tile count. */
            if !world.is_buoy(tile) && !world.is_airport(tile) {
                (self.s.station_infrastructure)(old_owner.0, u32::MAX);
                (self.s.station_infrastructure)(new_owner.0, 1);
            }
            /* for buoys, owner of tile is owner of water, st->owner == OWNER_NONE */
            (self.shared.set_tile_owner)(tile.0, new_owner.0);
            (self.s.station_list_classes_invalidate)();
        } else if world.drive_through_stop_tile(tile.0) {
            /* Remove the drive-through road stop */
            self.remove_drive_through_stop_for_bankrupt(cost, ret, error, tile);
            assert_eq!(TileType((self.shared.tile_type)(tile.0)), TileType::MP_ROAD);
            /* Change owner of tile and all roadtypes */
            // SAFETY: Nested removal has returned; the native slots can be reused.
            unsafe {
                crate::tile_owner::change_tile_owner(ownership, tile, old_owner, new_owner);
            };
        } else {
            crate::landscape_cmd::clear_command(
                self.command,
                cost.0,
                tile.0,
                (DoCommandFlags::from_element(DoCommandFlag::Execute)
                    | DoCommandFlags::from_element(DoCommandFlag::Bankrupt))
                .0,
            );
            /* Set tile owner of water under (now removed) buoy and dock to OWNER_NONE.
             * Update owner of buoy if it was not removed (was in orders).
             * Do not update when owned by OWNER_WATER (sea and rivers). */
            if (TileType((self.shared.tile_type)(tile.0)) == TileType::MP_WATER
                || world.is_buoy_tile(tile))
                && crate::tile_map::is_tile_owner(self.shared, tile.0, old_owner.0)
            {
                (self.shared.set_tile_owner)(tile.0, CompanyID::NONE.0);
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
    c.clear_tile_station(
        c.c(cost),
        c.c(ret),
        c.c(error),
        TileIndex(tile),
        DoCommandFlags(flags),
    );
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
    c.terraform_tile_station(
        c.c(cost),
        TileIndex(tile),
        DoCommandFlags(flags),
        z,
        Slope(slope),
    );
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
    c.change_tile_owner_station(StationOwnerChange {
        ownership,
        cost: c.c(ownership.cost),
        ret: c.c(ownership.ret),
        error: c.c(ownership.error),
        tile: TileIndex(tile),
        old_owner: CompanyID(old),
        new_owner: CompanyID(new),
    });
}

impl Ctx<'_> {
    /// `CanExpandRailStation`: check whether we can expand the rail part of
    /// the given station; returns the expanded `new_ta`.
    fn can_expand_rail_station(
        &self,
        out: Cost<'_>,
        st: *const BaseStation,
        new_ta: TileArea,
    ) -> TileArea {
        let cur_ta = self.s.rail_area(st);
        let map = self.map();
        /* determine new size of train station region.. */
        let x = map.tile_x(cur_ta.tile).min(map.tile_x(new_ta.tile));
        let y = map.tile_y(cur_ta.tile).min(map.tile_y(new_ta.tile));
        let new_ta = TileArea {
            tile: map.tile_xy(x, y),
            w: (map
                .tile_x(cur_ta.tile)
                .wrapping_add(u32::from(cur_ta.w))
                .max(map.tile_x(new_ta.tile).wrapping_add(u32::from(new_ta.w)))
                .wrapping_sub(x)) as u16,
            h: (map
                .tile_y(cur_ta.tile)
                .wrapping_add(u32::from(cur_ta.h))
                .max(map.tile_y(new_ta.tile).wrapping_add(u32::from(new_ta.h)))
                .wrapping_sub(y)) as u16,
        };
        /* make sure the final size is not too big. */
        let station_spread = (self.s.spread)();
        if new_ta.w > u16::from(station_spread) || new_ta.h > u16::from(station_spread) {
            out.error(self.s.strings.spread);
        } else {
            out.zero();
        }
        new_ta
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
        last_train,
        train,
        reservation,
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
