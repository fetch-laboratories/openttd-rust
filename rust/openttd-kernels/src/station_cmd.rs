/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Station construction, joining and removal policy on the canonical #125 owner.
//! Native shells, map/pools, specs and command result slots remain native. Every
//! callback is synchronous and noexcept; no owner borrow crosses a callback.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::if_not_else,
    clippy::too_many_lines,
    clippy::too_many_arguments
)]
use crate::direction_type::INVALID_DIAGDIR;
use crate::tile_type::{MP_INDUSTRY, MP_RAILWAY, MP_ROAD, MP_STATION, MP_TUNNELBRIDGE, MP_WATER};
mod waypoint;
use crate::rail_cmd::RailCommandServices;
use crate::road_stop::{RoadStopServices, Stop};
use crate::services::{CommandCost, CommandCostServices, CommandServices, Cost, Map, Services};
use crate::slope_func::{get_inclined_slope_direction, get_slope_max_z, is_steep_slope};
use crate::station::BaseStation;
use crate::station_service::World;
use crate::town::AuthorityServices;
use waypoint::WaypointLayout;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Area {
    pub tile: u32,
    pub w: u16,
    pub h: u16,
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
pub struct Bounds {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
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
pub struct AirportGeometry {
    pub area: Area,
    pub rotation: u8,
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
    pub train_area: unsafe extern "C" fn(*const BaseStation) -> Area,
    pub rail_spec: extern "C" fn(u32) -> *const RailSpec,
    pub links: *const crate::station_service::Links,
    pub bounds: extern "C" fn(*mut BaseStation) -> Bounds,
    pub location: extern "C" fn(*const BaseStation) -> u32,
    pub move_sign: extern "C" fn(*mut BaseStation, u32),
    pub virtual_coord: extern "C" fn(*mut BaseStation),
    pub catchment: extern "C" fn(*mut BaseStation, bool),
    pub catchment_dirty: extern "C" fn(),
    pub facility_widget: extern "C" fn(*mut BaseStation, u8),
    pub select_invalidate: extern "C" fn(),
    pub radius: unsafe extern "C" fn(
        u32,
        u32,
        *mut std::ffi::c_void,
        unsafe extern "C" fn(*mut std::ffi::c_void, *mut BaseStation),
    ),
    pub station_available: extern "C" fn() -> bool,
    pub new_station: extern "C" fn(u32) -> *mut BaseStation,
    pub index_insert: extern "C" fn(*mut BaseStation),
    pub set_town: extern "C" fn(*mut BaseStation, *const crate::services::TownHandle),
    pub default_name: extern "C" fn(*mut BaseStation, u32, u8),
    pub have_rating: extern "C" fn(*mut BaseStation, u8),
    pub before_add_rect: extern "C" fn(*mut CommandCost, *mut BaseStation, Area, u8),
    pub road_waypoint: extern "C" fn(*mut BaseStation) -> bool,
    pub name_equals: extern "C" fn(*mut BaseStation, *const u8, usize) -> bool,
    pub set_name: extern "C" fn(*mut BaseStation, *const u8, usize),
    pub rect_contains: extern "C" fn(*mut BaseStation, u32) -> bool,
    pub flip_airport_closed: extern "C" fn(*mut BaseStation),
    pub closed_widget: extern "C" fn(*mut BaseStation),
    pub distant_join: extern "C" fn() -> bool,
    pub rail_class_count: extern "C" fn() -> u32,
    pub rail_waypoint_class: extern "C" fn(u16) -> bool,
    pub rail_spec_count: extern "C" fn(u16) -> u32,
    pub class_rail_spec: extern "C" fn(u16, u16) -> *const RailSpec,
    pub rail_spec_info: extern "C" fn(*const RailSpec) -> RailSpecInfo,
    pub rail_layout: extern "C" fn(*const RailSpec, u8, u8) -> crate::services::ByteSpan,
    pub allocate_rail_spec: extern "C" fn(*const RailSpec, *mut BaseStation) -> SpecIndex,
    pub assign_rail_spec: extern "C" fn(*const RailSpec, *mut BaseStation, u8),
    pub deallocate_rail_spec: extern "C" fn(*mut BaseStation, u32),
    pub rail_boolean: extern "C" fn(*const RailSpec, u16, u16) -> bool,
    pub rail_callback_error: extern "C" fn(*const RailSpec, u16, u16),
    pub set_train_area: extern "C" fn(*mut BaseStation, Area),
    pub add_facility: extern "C" fn(*mut BaseStation, u8, u32),
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
    pub after_remove_tile: extern "C" fn(*mut BaseStation, u32),
    pub reset_facilities: extern "C" fn(*mut BaseStation, u8),
    pub orders_dirty: extern "C" fn(),
    pub vehicle: *const crate::vehicle::VehicleServices,
    pub stop_on_town_road: extern "C" fn() -> bool,
    pub stop_on_competitor_road: extern "C" fn() -> bool,
    pub road_class_count: extern "C" fn() -> u32,
    pub road_waypoint_class: extern "C" fn(u16) -> bool,
    pub road_spec_count: extern "C" fn(u16) -> u32,
    pub class_road_spec: extern "C" fn(u16, u16) -> *const RoadSpec,
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
    pub add_stop_area: extern "C" fn(*mut BaseStation, bool, u32),
    pub clear_stop_area: extern "C" fn(*mut BaseStation, bool),
    pub before_add_tile: extern "C" fn(*mut CommandCost, *mut BaseStation, u32, u8),
    pub make_drive_stop: extern "C" fn(u32, u8, u8, u8, u16, u8, u8, u8, u8),
    pub make_bay_stop: extern "C" fn(u32, u8, u16, u8, u8, u8, u8),
    pub delete_stop: extern "C" fn(*mut Stop),
    pub set_custom_road_index: extern "C" fn(u32, u8),
    pub custom_road_index: extern "C" fn(u32) -> u32,
    pub stop_random_bits: extern "C" fn(*mut BaseStation, u32, u8),

    pub remove_stop_tile_data: extern "C" fn(*mut BaseStation, u32),
    pub close_stop_inspect: extern "C" fn(u32),
    pub road_waypoint_area: extern "C" fn(*mut BaseStation) -> Area,
    pub set_road_waypoint_area: extern "C" fn(*mut BaseStation, Area),
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
    pub noise_enabled: extern "C" fn() -> bool,
    pub town_max_noise: extern "C" fn(*const crate::services::TownHandle) -> u16,
    pub station_town: extern "C" fn(*mut BaseStation) -> *const crate::services::TownHandle,
    pub airport_tile: extern "C" fn(*const BaseStation) -> u32,
    pub airport_type: extern "C" fn(*const BaseStation) -> u8,
    pub airport_geometry: extern "C" fn(*mut BaseStation) -> AirportGeometry,
    pub airport_airplanes: extern "C" fn(*const std::ffi::c_void) -> bool,
    pub set_airport_details: extern "C" fn(*mut BaseStation, u8, u8, u8),
    pub make_airport: extern "C" fn(u32, u8, u16, u8, u8),
    pub airport_add: extern "C" fn(*mut BaseStation, u32),
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
    pub after_remove_rect: extern "C" fn(*mut BaseStation, Area),
    pub clear_airport: extern "C" fn(*mut BaseStation),
    pub close_airport_inspect: extern "C" fn(*mut BaseStation),
    pub ship: *const crate::ship_control::Leaves,
    pub clear: *const crate::ground::ClearServices,
    pub restoration: *const crate::water::RestorationServices,
    pub set_docking: extern "C" fn(u32, bool),
    pub ship_area: extern "C" fn(*mut BaseStation) -> Area,
    pub set_ship_area: extern "C" fn(*mut BaseStation, Area),
    pub ship_area_add: extern "C" fn(*mut BaseStation, u32),
    pub docking_clear: extern "C" fn(*mut BaseStation),
    pub docking_add: extern "C" fn(*mut BaseStation, u32),
    pub water_infrastructure: extern "C" fn(u8, u32),
    pub make_dock: extern "C" fn(u32, u8, u16, u8, u8),
    pub neutral_for_tile: extern "C" fn(u32) -> *mut BaseStation,
    pub oilrig_unavailable: extern "C" fn(u32),
    pub associate_industry: extern "C" fn(*mut BaseStation, u32),
    pub make_oilrig: extern "C" fn(u32, u16, u8),
    pub set_station_owner: extern "C" fn(*mut BaseStation, u8),
    pub set_airport_kind_rotation: extern "C" fn(*mut BaseStation, u8, u8),
    pub set_facilities: extern "C" fn(*mut BaseStation, u8),
    pub set_build_date: extern "C" fn(*mut BaseStation, i32),
    pub with_nearby: unsafe extern "C" fn(
        *mut BaseStation,
        *mut std::ffi::c_void,
        unsafe extern "C" fn(*mut std::ffi::c_void, *mut BaseStation),
    ),
    pub associated_industry: extern "C" fn(*mut BaseStation) -> *mut Industry,
    pub industry_neutral: extern "C" fn(*mut Industry) -> *mut BaseStation,
    pub clear_industry_neutral: extern "C" fn(*mut Industry),
    pub industry_area: extern "C" fn(*mut Industry) -> Area,
    pub autoslope_axis: extern "C" fn(u32, i32, u8, u8) -> bool,
    pub station_list_classes_invalidate: extern "C" fn(),
    // Waypoint command leaves; canonical shells and shared map services.
    pub clear_cached_name: extern "C" fn(*mut BaseStation),
    pub clear_name: extern "C" fn(*mut BaseStation),
    pub make_buoy: extern "C" fn(u32, u16, crate::services::WaterClass),
    pub make_rail_waypoint: extern "C" fn(u32, u8, u16, u8, u8, crate::services::RailType),
    pub new_waypoint: extern "C" fn(u32) -> *mut BaseStation,
    pub rail_axis: extern "C" fn(u32) -> crate::services::Axis,
    pub road_ground: *const crate::ground::RoadServices,
    pub road_waypoint_area_add: extern "C" fn(*mut BaseStation, u32),
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
    fn map(&self) -> Map {
        if let Some(map) = self.map.get() {
            return map;
        }
        let map = Map {
            log_x: (self.shared.map_log_x)() as u8,
        };
        self.map.set(Some(map));
        map
    }
    fn c(&self, cost: *mut CommandCost) -> Cost<'a> {
        Cost(cost, self.cost)
    }
    fn check_buildable(
        &self,
        out: Cost<'_>,
        tile: u32,
        invalid: u8,
        allowed_z: &mut i32,
        allow_steep: bool,
        check_bridge: bool,
    ) {
        if check_bridge && (self.shared.bridge_above)(tile) {
            return out.error(self.s.strings.demolish_bridge);
        }
        // SAFETY: the native output is a live command slot for this entry.
        unsafe { self.command.no_vehicle(out.0, tile) };
        if !out.ok() {
            return;
        }
        let slope = (self.shared.tile_slope)(tile);
        let z = (self.shared.tile_z)(tile);
        if (!allow_steep && is_steep_slope(slope))
            || (!(self.shared.build_on_slopes)() && slope != 0)
        {
            return out.error(self.s.strings.flat_land);
        }
        out.set(0);
        let flat_z = z + get_slope_max_z(slope);
        if slope != 0 {
            const INCLINED: [u8; 4] = [12, 6, 3, 9];
            for (dir, corners) in INCLINED.into_iter().enumerate() {
                if invalid & (1 << dir) != 0
                    && (if is_steep_slope(slope) {
                        slope & corners != corners
                    } else {
                        slope & corners == 0
                    })
                {
                    return out.error(self.s.strings.flat_land);
                }
            }
            out.add_money(crate::company::price(57));
        }
        if *allowed_z < 0 {
            *allowed_z = flat_z;
        } else if *allowed_z != flat_z {
            out.error(self.s.strings.flat_land);
        }
    }
    fn bridge_height(&self, tile: u32) -> i32 {
        crate::bridge_map::get_bridge_height(tile, self.shared)
    }
    fn bridge_ok(
        &self,
        out: Cost<'_>,
        tile: u32,
        info: BridgeSpan,
        kind: u8,
        layout: u8,
        bridge_height: i32,
        disallowed: u32,
    ) {
        let height = if usize::from(layout) < info.size {
            // SAFETY: native immutable span, checked index; no references survive.
            unsafe { (&raw const (*info.data.add(usize::from(layout))).height).read() }
        } else {
            0
        };
        if height == 0 {
            if disallowed == 0xffff {
                crate::landscape_cmd::clear_command(self.command, out.0, tile, 2);
            } else {
                out.error(disallowed);
            }
            return;
        }
        if (self.shared.tile_max_z)(tile) + i32::from(height) > bridge_height {
            let difference =
                ((self.shared.tile_max_z)(tile) + i32::from(height) - bridge_height) * 8;
            (self.cost.error_param)(
                out.0,
                self.s.strings.bridge_too_low[usize::from(kind)],
                difference as u64,
            );
            return;
        }
        out.zero();
    }
    fn default_bridge_ok(&self, out: Cost<'_>, tile: u32, kind: u8, layout: u8) {
        if !(self.shared.bridge_above)(tile) {
            return out.zero();
        }
        let south = (self.s.southern_bridge_end)(tile);
        self.bridge_ok(
            out,
            tile,
            (self.s.default_bridge_info)(kind),
            kind,
            layout,
            self.bridge_height(south),
            self.s.strings.demolish_bridge,
        );
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
        tile,
        invalid,
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
    if !(c.shared.bridge_above)(tile) {
        return c.c(out).zero();
    }
    let south = (c.s.southern_bridge_end)(tile);
    let info = if spec.is_null() {
        (c.s.default_bridge_info)(kind)
    } else {
        unsafe { (c.s.road_bridge_info)(spec) }
    };
    c.bridge_ok(
        c.c(out),
        tile,
        info,
        kind,
        layout,
        c.bridge_height(south),
        c.s.strings.demolish_bridge,
    );
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
    if noise < 2 {
        return noise;
    }
    let s = unsafe { &*s };
    let authority = unsafe { &*s.authority };
    let tolerance = 8_u8.wrapping_add((authority.council_tolerance)().wrapping_mul(4));
    let reduction = distance / u32::from(tolerance);
    if reduction >= u32::from(noise) {
        1
    } else {
        (u32::from(noise) - reduction) as u8
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
    let kind = (c.shared.station_type)(tile).0;
    let mut info = (c.s.default_bridge_info)(kind);
    match kind {
        0 | 7 => {
            let spec = (c.s.rail_spec)(tile);
            if !spec.is_null() {
                info = unsafe { (c.s.rail_bridge_info)(spec) };
            }
        }
        2 | 3 | 8 => {
            let spec = unsafe {
                crate::newgrf_roadstop::openttd_rust_roadstop_spec(tile, c.s.roadstop_resolver)
            };
            if !spec.is_null() {
                info = unsafe { (c.s.road_bridge_info)(spec) };
            }
        }
        _ => (),
    }
    c.bridge_ok(
        c.c(out),
        tile,
        info,
        kind,
        (c.shared.station_gfx)(tile),
        height,
        0xffff,
    );
}

impl Ctx<'_> {
    fn world(&self) -> &World {
        unsafe { &*self.s.station }
    }
    fn links(&self) -> &crate::station_service::Links {
        unsafe { &*self.s.links }
    }
    fn full_station(&self, id: u16) -> *mut BaseStation {
        let st = (self.world().get)(id);
        if st.is_null() || !(self.world().is_station)(st) {
            std::ptr::null_mut()
        } else {
            st
        }
    }
    fn waypoint(&self, id: u16) -> *mut BaseStation {
        let st = (self.world().get)(id);
        if st.is_null() || (self.world().is_station)(st) {
            std::ptr::null_mut()
        } else {
            st
        }
    }
    fn sign_coord(&self, st: *mut BaseStation) {
        if (self.world().rectangle_empty)(st) {
            return;
        }
        let b = (self.s.bounds)(st);
        let xy = (self.s.location)(st);
        let x = (self.map().tile_x(xy) as i32).clamp(b.left, b.right) as u32;
        let y = (self.map().tile_y(xy) as i32).clamp(b.top, b.bottom) as u32;
        (self.s.move_sign)(st, (y << self.map().log_x) + x);
        if !(self.world().is_station)(st) {
            return;
        }
        for cargo in 0..64 {
            let graph = crate::linkgraph::live::station_graph(st, cargo, self.links().graphs);
            if graph.is_null() {
                continue;
            }
            let node = (self.links().graphs.node)(st, cargo);
            unsafe {
                crate::linkgraph::live::openttd_rust_live_node_location(
                    crate::linkgraph::live::resolve(graph, self.links().graphs),
                    node,
                    (self.s.location)(st),
                );
            }
        }
    }
    fn delete_if_empty(&self, st: *mut BaseStation) {
        let owner = (self.world().owner)(st);
        if !(self.world().in_use)(st) {
            // SAFETY: canonical owner, serialized scalar write ends before UI reentry.
            let fields =
                unsafe { crate::station_service::openttd_rust_station_service_fields(owner) };
            unsafe { (&raw mut (*fields).delete_ctr).write(0) };
            (self.world().station_list_invalidation)(st, 0);
        }
        self.sign_coord(st);
    }
    fn tiles_changed(&self, st: *mut BaseStation, adding: bool, kind: u8) {
        (self.s.virtual_coord)(st);
        (self.command.infrastructure_dirty)((self.world().station_owner)(st));
        if adding {
            (self.s.catchment)(st, false);
            (self.s.catchment_dirty)();
            (self.world().station_list_invalidation)(st, 0);
        } else {
            (self.s.catchment_dirty)();
        }
        (self.s.facility_widget)(st, kind);
        if adding {
            // Direct Rust call: acceptance is already owned by #125.
            unsafe {
                crate::station_service::openttd_rust_station_acceptance(st, self.s.station, 0);
            };
            (self.s.select_invalidate)();
        } else {
            self.delete_if_empty(st);
            (self.s.catchment)(st, false);
        }
    }
    fn join(
        &self,
        out: Cost<'_>,
        existing: u16,
        target: u16,
        adjacent: bool,
        area: Area,
        waypoint: bool,
        road: bool,
        overbuild_error: u32,
    ) -> *mut BaseStation {
        crate::witness::hit(crate::witness::STATION_CMD_JOIN);
        let mut st = std::ptr::null_mut();
        let mut check = true;
        let get = |id| {
            if waypoint {
                self.waypoint(id)
            } else {
                self.full_station(id)
            }
        };
        let filter = |p| !waypoint || (self.s.road_waypoint)(p) == road;
        if existing != 0xffff {
            if adjacent && existing != target {
                out.error(overbuild_error);
                return st;
            }
            let candidate = get(existing);
            if !candidate.is_null() && filter(candidate) {
                st = candidate;
            }
            check = st.is_null();
        } else if adjacent {
            check = false;
        }
        if check {
            // TileArea::Expand(1), clamped at the map perimeter.
            let x = self.map().tile_x(area.tile).saturating_sub(1);
            let y = self.map().tile_y(area.tile).saturating_sub(1);
            let right =
                (self.map().tile_x(area.tile) + u32::from(area.w) + 1).min(self.map().size_x());
            let bottom = (self.map().tile_y(area.tile) + u32::from(area.h) + 1)
                .min((self.shared.map_max_y)() + 1);
            let current = (self.shared.current_company)();
            let mut closest = existing;
            for ty in y..bottom {
                for tx in x..right {
                    let tile = (ty << self.map().log_x) + tx;
                    if (self.shared.tile_type)(tile) != MP_STATION {
                        continue;
                    }
                    let id = (self.shared.station_index)(tile);
                    let candidate = get(id);
                    if candidate.is_null()
                        || (self.world().station_owner)(candidate) != current
                        || !filter(candidate)
                    {
                        continue;
                    }
                    if closest == 0xffff {
                        closest = id;
                    } else if closest != id {
                        out.error(self.s.strings.adjoins_multiple);
                        return std::ptr::null_mut();
                    }
                }
            }
            st = if closest == 0xffff {
                std::ptr::null_mut()
            } else {
                get(closest)
            };
        }
        if st.is_null() && target != 0xffff {
            st = get(target);
        }
        out.zero();
        st
    }
}
struct DeletedSearch<'a> {
    c: &'a Ctx<'a>,
    tile: u32,
    threshold: u32,
    best: *mut BaseStation,
}
unsafe extern "C" fn deleted_visit(context: *mut std::ffi::c_void, st: *mut BaseStation) {
    // SAFETY: radius invokes this visitor synchronously with this stack context.
    let search = unsafe { &mut *context.cast::<DeletedSearch<'_>>() };
    let c = search.c;
    if (c.world().in_use)(st) || (c.world().station_owner)(st) != (c.shared.current_company)() {
        return;
    }
    let xy = (c.s.location)(st);
    let distance = c.map().tile_x(xy).abs_diff(c.map().tile_x(search.tile))
        + c.map().tile_y(xy).abs_diff(c.map().tile_y(search.tile));
    if distance < search.threshold {
        search.threshold = distance;
        search.best = st;
    } else if distance == search.threshold
        && !search.best.is_null()
        && (c.links().graphs.station_id)(st) < (c.links().graphs.station_id)(search.best)
    {
        search.best = st;
    }
}
impl Ctx<'_> {
    fn build_part(
        &self,
        out: Cost<'_>,
        mut st: *mut BaseStation,
        flags: u16,
        reuse: bool,
        area: Area,
        name: u8,
    ) -> *mut BaseStation {
        if st.is_null() && reuse {
            let mut search = DeletedSearch {
                c: self,
                tile: area.tile,
                threshold: 8,
                best: std::ptr::null_mut(),
            };
            unsafe { (self.s.radius)(area.tile, 8, (&raw mut search).cast(), deleted_visit) };
            st = search.best;
            if !st.is_null() {
                crate::witness::hit(crate::witness::STATION_CMD_REUSE);
            }
        }
        if !st.is_null() {
            if (self.world().station_owner)(st) != (self.shared.current_company)() {
                out.error(0xffff);
                return st;
            }
            (self.s.before_add_rect)(out.0, st, area, 0);
            if !out.ok() {
                return st;
            }
        } else {
            if !(self.s.station_available)() {
                out.error(self.s.strings.too_many_stations);
                return st;
            }
            if flags & 1 != 0 {
                st = (self.s.new_station)(area.tile);
                (self.s.index_insert)(st);
                let town = (self.shared.closest_town)(area.tile, u32::MAX);
                (self.s.set_town)(st, town);
                (self.s.default_name)(st, area.tile, name);
                let current = (self.shared.current_company)();
                if (self.shared.company_valid)(current) {
                    (self.s.have_rating)(st, current);
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
    unsafe { Ctx::new(s) }.tiles_changed(st, adding, kind);
}

impl Ctx<'_> {
    fn rename(&self, out: Cost<'_>, flags: u16, id: u16, text: *const u8, length: usize) {
        crate::witness::hit(crate::witness::STATION_CMD_RENAME);
        let st = self.full_station(id);
        if st.is_null() {
            return out.error(0xffff);
        }
        unsafe {
            (self.command.ownership)(out.0, (self.world().station_owner)(st), 0);
        }
        if !out.ok() {
            return;
        }
        if length != 0 {
            if (self.shared.utf8_length)(text, length)
                >= crate::station_type::MAX_LENGTH_STATION_NAME_CHARS
            {
                return out.error(0xffff);
            }
            for index in 0..(self.world().pool_size)() {
                let candidate = self.full_station(index as u16);
                if !candidate.is_null() && (self.s.name_equals)(candidate, text, length) {
                    return out.error(self.s.strings.name_unique);
                }
            }
        }
        if flags & 1 != 0 {
            (self.s.clear_cached_name)(st);
            if length == 0 {
                (self.s.clear_name)(st);
            } else {
                (self.s.set_name)(st, text, length);
            }
            (self.s.virtual_coord)(st);
            (self.world().station_list_invalidation)(st, 1);
        }
        out.zero();
    }
    fn move_name(&self, out: Cost<'_>, flags: u16, id: u16, tile: u32) -> u16 {
        crate::witness::hit(crate::witness::STATION_CMD_MOVE_NAME);
        let st = self.full_station(id);
        if st.is_null() {
            out.error(0xffff);
            return 0xffff;
        }
        let owner = (self.world().station_owner)(st);
        if owner != 0x10 {
            unsafe {
                (self.command.ownership)(out.0, owner, 0);
            }
            if !out.ok() {
                return 0xffff;
            }
        }
        if !(self.s.rect_contains)(st, tile) {
            out.error(self.s.strings.unsuitable);
            return 0xffff;
        }
        let mut search = SignSearch {
            c: self,
            station: st,
            tile,
            other: false,
        };
        unsafe {
            (self.s.radius)(tile, 0, (&raw mut search).cast(), sign_visit);
        }
        if search.other {
            out.error(self.s.strings.unsuitable);
            return 0xffff;
        }
        if flags & 1 != 0 {
            (self.s.move_sign)(st, tile);
            (self.s.virtual_coord)(st);
        }
        out.zero();
        id
    }
    fn shrink(&self, st: *mut BaseStation, mut area: Area, kind: u8) -> Area {
        let id = (self.links().graphs.station_id)(st);
        let kind = if kind == 0 && !(self.world().is_station)(st) {
            7
        } else {
            kind
        };
        let belongs = |tile| {
            (self.shared.tile_type)(tile) == MP_STATION
                && (self.shared.station_type)(tile).0 == kind
                && (self.shared.station_index)(tile) == id
        };
        'restart: loop {
            if area.w == 0 || area.h == 0 {
                return Area {
                    tile: u32::MAX,
                    w: 0,
                    h: 0,
                };
            }
            for (x, vertical) in [
                (0, true),
                (i32::from(area.w) - 1, true),
                (0, false),
                (i32::from(area.h) - 1, false),
            ] {
                let size = if vertical { area.h } else { area.w };
                let mut found = false;
                for i in 0..size {
                    let tile = if vertical {
                        area.tile.wrapping_add(self.map().diff(x, i32::from(i)))
                    } else {
                        area.tile.wrapping_add(self.map().diff(i32::from(i), x))
                    };
                    if belongs(tile) {
                        found = true;
                        break;
                    }
                }
                if !found {
                    if vertical {
                        if x == 0 {
                            area.tile = area.tile.wrapping_add(1);
                        }
                        area.w -= 1;
                    } else {
                        if x == 0 {
                            area.tile = area.tile.wrapping_add(self.map().size_x());
                        }
                        area.h -= 1;
                    }
                    continue 'restart;
                }
            }
            return area;
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
    c.rename(c.c(out), flags, id, text, length);
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
    c.move_name(c.c(out), flags, id, tile)
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
    let out = c.c(out);
    let st = c.full_station(id);
    if st.is_null() {
        return out.error(0xffff);
    }
    let owner = (c.world().station_owner)(st);
    if (c.world().facilities)(st) & 8 == 0 || owner == 0x10 {
        return out.error(0xffff);
    }
    unsafe {
        (c.command.ownership)(out.0, owner, 0);
    }
    if !out.ok() {
        return;
    }
    if flags & 1 != 0 {
        (c.s.flip_airport_closed)(st);
        (c.s.closed_widget)(st);
    }
    out.zero();
}

impl Area {
    fn tiles(self, map: Map) -> impl Iterator<Item = u32> {
        (0..self.h).flat_map(move |y| {
            (0..self.w).map(move |x| self.tile.wrapping_add(map.diff(i32::from(x), i32::from(y))))
        })
    }
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
                flags & (1 << 4) != 0,
            );
        }
    }
    fn rail_station(&self, tile: u32) -> bool {
        (self.rail_map().is_rail_station)(tile)
    }
    fn station_axis(&self, tile: u32) -> u8 {
        (self.rail_map().station_track)(tile).0
    }
    fn buildable_area(&self, tile: u32, w: u16, h: u16) -> bool {
        (self.shared.tile_valid)(tile)
            && (self.shared.tile_add_wrap)(tile, i32::from(w) - 1, i32::from(h) - 1) != u32::MAX
    }
    fn free_reservation(&self, train: *mut crate::services::Train) {
        unsafe {
            let handle = crate::pbs::Pbs::acquire(self.shared).handle(train);
            crate::train_reservation::openttd_rust_train_reservation_free(
                handle,
                crate::vehicle::VehicleRef(std::ptr::NonNull::new_unchecked(handle.shell.cast())),
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
                crate::vehicle::VehicleRef(std::ptr::NonNull::new_unchecked(handle.shell.cast())),
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
        let vehicle = unsafe {
            crate::vehicle::VehicleRef(std::ptr::NonNull::new_unchecked(handle.shell.cast()))
        };
        let tile = (self.vehicle().tile)(vehicle);
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
    fn flat_rail(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        tile: u32,
        north: u32,
        allowed: &mut i32,
        flags: u16,
        axis: u8,
        est: &mut u16,
        rt: u8,
        affected: &mut Vec<*mut crate::services::Train>,
        spec: *const RailSpec,
        length: u8,
        tracks: u8,
    ) {
        let invalid = if axis == 0 { 5 } else { 10 };
        let slope_callback = !spec.is_null() && (self.s.rail_spec_info)(spec).slope_callback;
        self.check_buildable(ret, tile, invalid, allowed, false, false);
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
            if *est == 0xffff {
                *est = id;
            } else if *est != id {
                cost.error(self.s.strings.adjoins_multiple);
            }
        } else {
            if rt != (self.rail_map().invalid_rail_type)().0 && (self.shared.is_plain_rail)(tile) {
                if (self.rail_map().has_signals)(tile) {
                    return cost.error(self.s.strings.remove_signals);
                }
                if (self.rail().powered_rail)((self.rail_map().rail_type)(tile).0, rt) {
                    let bits = (self.rail_map().traversal)(tile).tracks.0;
                    let track = bits.trailing_zeros() as u8;
                    if bits & bits.wrapping_sub(1) == 0 && track == axis {
                        if crate::pbs::Pbs::acquire(self.shared).reserved(tile) & (1 << track) != 0
                        {
                            let train = crate::pbs::Pbs::acquire(self.shared)
                                .train_for(tile, track)
                                .shell
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
        (self.s.set_tile_flags)(tile, flags & 4 != 0, flags & 1 != 0, flags & 2 == 0);
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
    fn rail_bridge(&self, out: Cost<'_>, tile: u32, spec: *const RailSpec, kind: u8, gfx: u8) {
        if !(self.shared.bridge_above)(tile) {
            return out.zero();
        }
        let south = (self.s.southern_bridge_end)(tile);
        let info = if spec.is_null() {
            (self.s.default_bridge_info)(kind)
        } else {
            unsafe { (self.s.rail_bridge_info)(spec) }
        };
        self.bridge_ok(
            out,
            tile,
            info,
            kind,
            gfx,
            self.bridge_height(south),
            self.s.strings.demolish_bridge,
        );
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
        if !(self.rail().rail_type_valid)(rt) || axis >= 2 {
            return cost.error(0xffff);
        }
        if u32::from(class) >= (self.s.rail_class_count)()
            || (self.s.rail_waypoint_class)(class)
            || u32::from(index) >= (self.s.rail_spec_count)(class)
            || length == 0
            || tracks == 0
        {
            return cost.error(0xffff);
        }
        let (w, h) = if axis == 0 {
            (length, tracks)
        } else {
            (tracks, length)
        };
        if !self.buildable_area(tile, u16::from(w), u16::from(h)) {
            return cost.error(0xffff);
        }
        let reuse = target != 0xfffd;
        if !reuse {
            target = 0xffff;
        }
        if target != 0xffff && (!(self.s.distant_join)() || self.full_station(target).is_null()) {
            return cost.error(0xffff);
        }
        let spread = (self.s.spread)();
        if h > spread || w > spread {
            return cost.error(0xffff);
        }
        let mut area = Area {
            tile,
            w: u16::from(w),
            h: u16::from(h),
        };
        let mut est = 0xffff;
        let mut affected = Vec::new();
        let spec = (self.s.class_rail_spec)(class, index);
        let mut allowed = -1;
        let mut length_ready = true;
        let mut tracknum = 0_u8;
        // Temporary native slots are reused; ret becomes each tile's result.
        cost.set(0);
        for t in area.tiles(self.map()) {
            // flat_rail needs two command slots; preserve the accumulated amount
            // through the independent native result slot used by this helper.
            let saved = cost.money();
            self.flat_rail(
                cost,
                ret,
                t,
                tile,
                &mut allowed,
                flags,
                axis,
                &mut est,
                rt,
                &mut affected,
                spec,
                length,
                tracks,
            );
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
                cost.add_money(crate::company::price(9));
                cost.add_money((self.rail().rail_build_cost)(rt));
                if length_ready {
                    cost.add_money(crate::company::price(10));
                    length_ready = false;
                }
            }
        }
        let st = self.join(
            ret,
            est,
            target,
            adjacent,
            area,
            false,
            false,
            self.s.strings.remove_station,
        );
        if !ret.ok() {
            return cost.take(ret);
        }
        let st = self.build_part(ret, st, flags, reuse, area, 0);
        if !ret.ok() {
            return cost.take(ret);
        }
        if !st.is_null() && unsafe { (self.s.train_area)(st) }.tile != u32::MAX {
            area = self.expand_rail(ret, st, area);
            if !ret.ok() {
                return cost.take(ret);
            }
        }
        let spec = (self.s.class_rail_spec)(class, index);
        let delta = if axis == 0 { 1 } else { self.map().size_x() };
        let track_delta = if axis == 0 { self.map().size_x() } else { 1 };
        let layout = (self.s.rail_layout)(spec, tracks, length);
        for i in 0..tracks {
            for j in 0..length {
                let t = tile
                    .wrapping_add(u32::from(i).wrapping_mul(track_delta))
                    .wrapping_add(u32::from(j).wrapping_mul(delta));
                if !(self.shared.bridge_above)(t) {
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
                    let cb = crate::newgrf_station::callback(
                        0x24,
                        info,
                        0,
                        spec,
                        std::ptr::null_mut(),
                        u32::MAX,
                    );
                    if cb != 0xffff && cb <= 255 {
                        gfx = (cb as u8 & !1).wrapping_add(axis);
                    }
                }
                self.rail_bridge(ret, t, spec, 0, gfx);
                if !ret.ok() {
                    return cost.take(ret);
                }
            }
        }
        let specindex = (self.s.allocate_rail_spec)(spec, st);
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
                    0x13,
                    0,
                    0,
                    spec,
                    std::ptr::null_mut(),
                    u32::MAX,
                );
                if cb != 0xffff && !(self.s.rail_boolean)(spec, 0x13, cb) {
                    return cost.error(0xffff);
                }
            }
        }
        if flags & 1 == 0 {
            return;
        }
        (self.s.set_train_area)(st, area);
        (self.s.add_facility)(st, 0, area.tile);
        (self.s.before_add_rect)(
            ret.0,
            st,
            Area {
                tile,
                w: u16::from(w),
                h: u16::from(h),
            },
            1,
        );
        (self.s.assign_rail_spec)(spec, st, specindex.value);
        if !spec.is_null() {
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
                        .shell
                        .cast::<crate::services::Train>();
                    if !train.is_null() {
                        affected.push(train);
                        self.free_reservation(train);
                    }
                }
                if self.rail_station(t) {
                    if !(self.rail_map().station_blocked)(t) {
                        (self.rail().rail_infrastructure)(
                            owner,
                            (self.rail_map().rail_type)(t).0,
                            u32::MAX,
                        );
                    }
                    (self.s.station_infrastructure)(owner, u32::MAX);
                }
                crate::animated_tile::openttd_rust_animation_delete(t, false);
                let old = if (self.shared.tile_type)(t) == MP_STATION
                    && (self.rail_map().has_station_rail)(t)
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
                (self.s.deallocate_rail_spec)(st, u32::from(old));
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
                        0x24,
                        info,
                        0,
                        spec,
                        std::ptr::null_mut(),
                        t,
                    );
                    if cb != 0xffff {
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
                if !(self.rail_map().station_blocked)(t) {
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
            Area {
                tile,
                w: 1,
                h: u16::from(tracks),
            }
        } else {
            Area {
                tile,
                w: u16::from(tracks),
                h: 1,
            }
        };
        for t in update.tiles(self.map()) {
            if (self.rail_map().station_blocked)(t) {
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
        self.tiles_changed(st, true, 0);
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

impl Area {
    fn between(map: Map, a: u32, b: u32) -> Self {
        let x = map.tile_x(a).min(map.tile_x(b));
        let y = map.tile_y(a).min(map.tile_y(b));
        Self {
            tile: (y << map.log_x) + x,
            w: (map.tile_x(a).max(map.tile_x(b)) - x + 1) as u16,
            h: (map.tile_y(a).max(map.tile_y(b)) - y + 1) as u16,
        }
    }
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

    fn remove_rail_area(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        error: Cost<'_>,
        area: Area,
        flags: u16,
        unit: i64,
        keep: bool,
        waypoint: bool,
    ) -> Vec<*mut BaseStation> {
        crate::witness::hit(crate::witness::STATION_CMD_REMOVE_RAIL_AREA);
        let mut quantity = 0_i32;
        let mut affected = Vec::new();
        cost.set(0);
        error.zero();
        for tile in area.tiles(self.map()) {
            if (self.shared.tile_type)(tile) != MP_STATION
                || !(self.rail_map().has_station_rail)(tile)
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
                self.waypoint(id)
            } else {
                self.full_station(id)
            };
            if st.is_null() {
                continue;
            }
            if (self.shared.current_company)() != 0x11 {
                unsafe {
                    (self.command.ownership)(ret.0, (self.world().station_owner)(st), 0);
                }
                error.add(ret);
                if !error.ok() {
                    continue;
                }
            }
            quantity += 1;
            if keep || (self.rail_map().station_blocked)(tile) {
                cost.add_money(crate::company::price(27).saturating_neg());
            }
            if flags & 1 == 0 {
                continue;
            }
            let index = (self.s.custom_rail_index)(tile);
            let track = self.station_axis(tile);
            let owner = (self.shared.tile_owner)(tile);
            let rt = (self.rail_map().rail_type)(tile).0;
            let mut train = std::ptr::null_mut();
            if (self.s.reserved)(tile) {
                train = crate::pbs::Pbs::acquire(self.shared)
                    .train_for(tile, track)
                    .shell
                    .cast::<crate::services::Train>();
                if !train.is_null() {
                    self.free_reservation(train);
                }
            }
            let build_rail = keep && !(self.rail_map().station_blocked)(tile);
            if !build_rail && !(self.rail_map().station_blocked)(tile) {
                (self.rail().rail_infrastructure)(owner, rt, u32::MAX);
            }
            crate::ground::clear_square(tile, self.shared);
            (self.s.close_station_inspect)(tile);
            if build_rail {
                (self.rail_map().make_rail_normal)(tile, owner, 1 << track, rt);
            }
            (self.s.station_infrastructure)(owner, u32::MAX);
            (self.command.infrastructure_dirty)(owner);
            (self.s.after_remove_tile)(st, tile);
            self.signals().add_track(tile, track, owner);
            (self.rail().layout_changed)(tile, track);
            (self.s.deallocate_rail_spec)(st, index);
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
            let area = self.shrink(st, unsafe { (self.s.train_area)(st) }, 0);
            (self.s.set_train_area)(st, area);
            self.sign_coord(st);
            if area.tile == u32::MAX {
                (self.s.reset_facilities)(st, 1);
                (self.s.orders_dirty)();
                (self.s.facility_widget)(st, 0);
                (self.s.catchment_dirty)();
                (self.s.virtual_coord)(st);
                self.delete_if_empty(st);
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
            return cost.error(0xffff);
        }
        let affected = self.remove_rail_area(
            cost,
            ret,
            error,
            Area::between(self.map(), start, end),
            flags,
            crate::company::price(if waypoint { 53 } else { 35 }),
            keep,
            waypoint,
        );
        if !cost.ok() || waypoint {
            return;
        }
        for st in affected {
            if unsafe { (self.s.train_area)(st) }.tile == u32::MAX {
                (self.s.facility_widget)(st, 0);
            }
            (self.s.mark_tiles)(st, false);
            (self.s.catchment_dirty)();
            (self.s.catchment)(st, false);
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
        if (self.shared.current_company)() != 0x11 {
            unsafe {
                (self.command.ownership)(cost.0, (self.world().station_owner)(st), 0);
            }
            if !cost.ok() {
                return;
            }
        }
        let area = unsafe { (self.s.train_area)(st) };
        cost.set(0);
        let id = (self.links().graphs.station_id)(st);
        let kind = if waypoint { 7 } else { 0 };
        for tile in area.tiles(self.map()) {
            if (self.shared.tile_type)(tile) != MP_STATION
                || (self.shared.station_type)(tile).0 != kind
                || (self.shared.station_index)(tile) != id
            {
                continue;
            }
            let total = cost.money();
            self.remove_rail_area(
                cost,
                ret,
                error,
                Area { tile, w: 1, h: 1 },
                flags,
                crate::company::price(if waypoint { 53 } else { 35 }),
                false,
                waypoint,
            );
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
        if (self.shared.current_company)() == 0x11 {
            return self.nested(cost, tile, 1, |flags| {
                self.remove_rail_command(cost, ret, error, flags, tile, 0, false, waypoint);
            });
        }
        let id = (self.shared.station_index)(tile);
        let st = if waypoint {
            self.waypoint(id)
        } else {
            self.full_station(id)
        };
        self.remove_rail_all_inner(cost, ret, error, flags, st, waypoint);
        if !waypoint && flags & 1 != 0 {
            (self.s.catchment)(st, false);
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
            (self.shared.tram_owner)(tile)
        } else {
            (self.shared.road_owner)(tile)
        }
    }
    fn may_have_road(&self, tile: u32) -> bool {
        match (self.shared.tile_type)(tile) {
            MP_ROAD => true,
            MP_STATION => (self.shared.is_any_road_stop)(tile),
            MP_TUNNELBRIDGE => (self.shared.tunnel_transport)(tile) == 1,
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
    fn road_bridge(&self, out: Cost<'_>, tile: u32, spec: *const RoadSpec, kind: u8, gfx: u8) {
        if !(self.shared.bridge_above)(tile) {
            return out.zero();
        }
        let south = (self.s.southern_bridge_end)(tile);
        let info = if spec.is_null() {
            (self.s.default_bridge_info)(kind)
        } else {
            unsafe { (self.s.road_bridge_info)(spec) }
        };
        self.bridge_ok(
            out,
            tile,
            info,
            kind,
            gfx,
            self.bridge_height(south),
            self.s.strings.demolish_bridge,
        );
    }
    fn flat_road(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        tile: u32,
        allowed: &mut i32,
        spec: *const RoadSpec,
        flags: u16,
        invalid: u8,
        drive: bool,
        kind: u8,
        axis: u8,
        est: &mut u16,
        rt: u8,
    ) {
        self.check_buildable(ret, tile, invalid, allowed, !drive, false);
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
            if drive {
                4 + axis
            } else {
                invalid.trailing_zeros() as u8
            },
        );
        if !ret.ok() {
            return cost.take(ret);
        }
        if (self.shared.tile_type)(tile) == MP_STATION {
            if !(self.shared.is_any_road_stop)(tile)
                || kind != (self.shared.station_type)(tile).0
                || drive != (self.shared.is_drive_through_stop)(tile)
            {
                return self.auto_clear(cost, tile);
            }
            if drive
                && (self.shared.is_drive_through_stop)(tile)
                && (self.shared.drive_through_axis)(tile) != axis
            {
                return cost.error(self.s.strings.drive_direction);
            }
            let id = (self.shared.station_index)(tile);
            if *est == 0xffff {
                *est = id;
            } else if *est != id {
                cost.error(self.s.strings.adjoins_multiple);
            }
        } else {
            let normal = (self.shared.is_normal_road)(tile);
            let build_over = drive && normal;
            let rb = if normal {
                (self.shared.normal_road_bits)(tile, 0).0
                    | (self.shared.normal_road_bits)(tile, 1).0
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
                    if owner == 0x0f {
                        if !(self.s.stop_on_town_road)() {
                            return cost.error(self.s.strings.drive_town);
                        }
                    } else if !(self.s.stop_on_competitor_road)() && owner != 0x10 {
                        unsafe {
                            (self.command.ownership)(ret.0, owner, 0);
                        }
                        if !ret.ok() {
                            return cost.take(ret);
                        }
                    }
                    let pieces = (self.shared.normal_road_bits)(tile, 0).0.count_ones();
                    if rt != 63
                        && (self.shared.road_type_is_road)(rt)
                        && !(self.shared.road_powered)(rt, road)
                    {
                        return cost.error(self.s.strings.unsuitable_road);
                    }
                    if (self.shared.disallowed_road_directions)(tile) != 0
                        && owner != 0x0f
                        && owner != 0x10
                    {
                        unsafe {
                            (self.command.ownership)(ret.0, owner, 0);
                        }
                        if !ret.ok() {
                            return cost.take(ret);
                        }
                    }
                    cost.add_money(
                        crate::road_cost::build_cost(self.shared, road)
                            .saturating_mul(i64::from(2_u32.wrapping_sub(pieces))),
                    );
                } else if rt != 63 && (self.shared.road_type_is_road)(rt) {
                    cost.add_money(crate::road_cost::build_cost(self.shared, rt).saturating_mul(2));
                }
                let tram = self.road_type(tile, true);
                if tram != 63 {
                    let owner = self.road_owner(tile, true);
                    let bits = (self.shared.normal_road_bits)(tile, 1).0;
                    if (self.shared.company_valid)(owner)
                        && (!(self.s.stop_on_competitor_road)() || bits.is_power_of_two())
                    {
                        unsafe {
                            (self.command.ownership)(ret.0, owner, 0);
                        }
                        if !ret.ok() {
                            return cost.take(ret);
                        }
                    }
                    let pieces = (self.shared.normal_road_bits)(tile, 1).0.count_ones();
                    if rt != 63
                        && (self.shared.road_tram_type)(rt).0
                        && !(self.shared.road_powered)(rt, tram)
                    {
                        return cost.error(self.s.strings.unsuitable_road);
                    }
                    cost.add_money(
                        crate::road_cost::build_cost(self.shared, tram)
                            .saturating_mul(i64::from(2_u32.wrapping_sub(pieces))),
                    );
                } else if rt != 63 && (self.shared.road_tram_type)(rt).0 {
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
    fn road_cost(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        area: Area,
        flags: u16,
        drive: bool,
        kind: u8,
        spec: *const RoadSpec,
        axis: u8,
        dir: u8,
        est: &mut u16,
        rt: u8,
        unit: i64,
    ) {
        let invalid = if drive {
            if axis == 0 { 5 } else { 10 }
        } else {
            1 << dir
        };
        let mut allowed = -1;
        cost.set(0);
        for tile in area.tiles(self.map()) {
            let total = cost.money();
            self.flat_road(
                cost,
                ret,
                tile,
                &mut allowed,
                spec,
                flags,
                invalid,
                drive,
                kind,
                axis,
                est,
                rt,
            );
            if !cost.ok() {
                return;
            }
            let tile_cost = cost.money();
            cost.set(total);
            if (self.shared.tile_type)(tile) != MP_STATION || !(self.shared.is_any_road_stop)(tile)
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
        if !(self.shared.road_type_valid)(rt) || dir >= 4 || stop_type >= 2 {
            return cost.error(0xffff);
        }
        let reuse = target != 0xfffd;
        if !reuse {
            target = 0xffff;
        }
        let distant = target != 0xffff;
        if u32::from(class) >= (self.s.road_class_count)()
            || (self.s.road_waypoint_class)(class)
            || u32::from(index) >= (self.s.road_spec_count)(class)
        {
            return cost.error(0xffff);
        }
        let spec = (self.s.class_road_spec)(class, index);
        if !spec.is_null() {
            let info = (self.s.road_spec_info)(spec);
            if (stop_type == 1 && info.stop_type != 1 && info.stop_type != 2)
                || (stop_type == 0 && info.stop_type != 0 && info.stop_type != 2)
                || (!drive && info.drive_only)
            {
                return cost.error(0xffff);
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
            return cost.error(0xffff);
        }
        let area = Area {
            tile,
            w: u16::from(width),
            h: u16::from(length),
        };
        if distant && (!(self.s.distant_join)() || self.full_station(target).is_null()) {
            return cost.error(0xffff);
        }
        if !drive && !(self.shared.road_type_is_road)(rt) {
            return cost.error(0xffff);
        }
        let axis = dir & 1;
        self.authority(cost, tile, flags);
        if !cost.ok() {
            return;
        }
        let truck = stop_type != 0;
        let kind = if truck { 2 } else { 3 };
        let category = if truck { 13 } else { 12 };
        let unit = if spec.is_null() {
            crate::company::price(usize::from(category))
        } else {
            {
                let factor = (self.s.road_spec_build_cost)(spec, category);
                crate::economy_func::get_price(
                    u32::from(category),
                    u32::from(factor.multiplier),
                    factor.shift,
                    -4,
                )
            }
        };
        let mut est = 0xffff;
        self.road_cost(
            cost, ret, area, flags, drive, kind, spec, axis, dir, &mut est, rt, unit,
        );
        if !cost.ok() {
            return;
        }
        let st = self.join(
            ret,
            est,
            target,
            adjacent,
            area,
            false,
            false,
            self.s.strings.remove_road_stop,
        );
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
        let st = self.build_part(ret, st, flags, reuse, area, 1);
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
            if cb != 0xffff && !(self.s.road_boolean)(spec, cb) {
                return cost.error(0xffff);
            }
        }
        if flags & 1 == 0 {
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
            (self.s.add_stop_area)(st, truck, tile);
            (self.s.add_facility)(st, if truck { 1 } else { 2 }, tile);
            (self.s.before_add_tile)(ret.0, st, tile, 1);
            let owner = (self.world().station_owner)(st);
            let id = (self.links().graphs.station_id)(st);
            if drive {
                if (self.shared.is_normal_road)(tile) {
                    self.road_infrastructure(
                        road,
                        road_owner,
                        -((self.shared.normal_road_bits)(tile, 0).0.count_ones() as i32),
                    );
                    self.road_infrastructure(
                        tram,
                        tram_owner,
                        -((self.shared.normal_road_bits)(tile, 1).0.count_ones() as i32),
                    );
                }
                if road == 63 && (self.shared.road_type_is_road)(rt) {
                    road = rt;
                }
                if tram == 63 && (self.shared.road_tram_type)(rt).0 {
                    tram = rt;
                }
                (self.s.make_drive_stop)(
                    tile, owner, road_owner, tram_owner, id, kind, road, tram, axis,
                );
                crate::road_stop::make_drive_through(self.shared, tile, (self.stops().state)(stop));
            } else {
                if road == 63 && (self.shared.road_type_is_road)(rt) {
                    road = rt;
                }
                if tram == 63 && (self.shared.road_tram_type)(rt).0 {
                    tram = rt;
                }
                (self.s.make_bay_stop)(tile, owner, id, stop_type, road, tram, dir);
            }
            self.road_infrastructure(road, road_owner, 2);
            self.road_infrastructure(tram, tram_owner, 2);
            (self.s.station_infrastructure)(owner, 1);
            (self.s.set_custom_road_index)(tile, specindex.value);
            if !spec.is_null() {
                (self.s.stop_random_bits)(
                    st,
                    tile,
                    ((self.shared.random)(self.shared.context) & 255) as u8,
                );
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
            self.tiles_changed(st, true, kind);
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
            if (self.vehicle().vehicle_type)(vehicle) != 1 {
                continue;
            }
            for index in 0..(self.links().order_count)(list) {
                let order = (self.links().order_destination)(list, index);
                if order.kind.0 != 1 || order.destination != station {
                    continue;
                }
                loop {
                    if (self.vehicle().order_type)(vehicle) == 1
                        && (self.vehicle().dest_tile)(vehicle) == tile
                    {
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
            self.waypoint(id)
        } else {
            self.full_station(id)
        };
        if (self.shared.current_company)() != 0x11 {
            unsafe {
                (self.command.ownership)(cost.0, (self.world().station_owner)(st), 0);
            }
            if !cost.ok() {
                return;
            }
        }
        let truck = !waypoint && (self.shared.station_type)(tile).0 == 2;
        let (stop, stop_state) = if waypoint {
            (std::ptr::null_mut(), std::ptr::null_mut())
        } else {
            crate::road_stop::get_by_tile(
                self.shared,
                tile,
                crate::services::RoadStopType(u8::from(truck)),
            )
        };
        if waypoint {
            if flags & 32 == 0 {
                unsafe {
                    self.command.no_vehicle(cost.0, tile);
                }
                if !cost.ok() {
                    return;
                }
            }
        } else if (self.shared.is_drive_through_stop)(tile) && flags & 32 != 0 {
            if flags & 1 != 0 {
                self.vehicle().for_each_on_tile(tile, |vehicle| {
                    if (self.vehicle().vehicle_type)(vehicle) == 1 {
                        let state = (self.s.road_vehicle_owner)(vehicle);
                        // SAFETY: Live road owner; no borrow crosses this access.
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
        if flags & 1 != 0 {
            if !waypoint {
                let head = (self.stops().head)(st, truck);
                let next = (self.stops().next)(stop);
                if head == stop {
                    (self.s.set_stop_head)(st, truck, next);
                    if next.is_null() {
                        (self.s.reset_facilities)(st, if truck { 2 } else { 4 });
                        (self.s.orders_dirty)();
                    }
                } else {
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
            if !waypoint && (self.shared.is_drive_through_stop)(tile) {
                crate::road_stop::clear_drive_through(self.shared, tile, stop_state);
                crate::animated_tile::openttd_rust_animation_delete(tile, false);
            } else {
                crate::ground::clear_square(tile, self.shared);
            }
            if !waypoint {
                (self.s.delete_stop)(stop);
                self.retarget_road_vehicles(id, tile);
            }
            (self.s.after_remove_tile)(st, tile);
            if !waypoint && replacement < 0 {
                self.tiles_changed(st, false, if truck { 2 } else { 3 });
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
                    let area = self.shrink(st, (self.s.road_waypoint_area)(st), 8);
                    (self.s.set_road_waypoint_area)(st, area);
                    self.sign_coord(st);
                    if area.tile == u32::MAX {
                        (self.s.reset_facilities)(st, 6);
                        (self.s.facility_widget)(st, 3);
                        (self.s.virtual_coord)(st);
                        self.delete_if_empty(st);
                    }
                }
            } else {
                (self.s.clear_stop_area)(st, truck);
                let mut next = (self.stops().head)(st, truck);
                while !next.is_null() {
                    (self.s.add_stop_area)(st, truck, (self.stops().tile)(next));
                    next = (self.stops().next)(next);
                }
            }
        }
        let category = if waypoint || truck { 38 } else { 37 };
        cost.set(if spec.is_null() {
            crate::company::price(usize::from(category))
        } else {
            {
                let factor = (self.s.road_spec_clear_cost)(spec, category);
                crate::economy_func::get_price(
                    u32::from(category),
                    u32::from(factor.multiplier),
                    factor.shift,
                    -4,
                )
            }
        });
        let _ = ret;
    }
    fn remove_generic_road(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        error: Cost<'_>,
        flags: u16,
        area: Area,
        waypoint: bool,
        remove_road: bool,
    ) {
        cost.set(0);
        error.error(self.s.strings.no_station);
        let mut success = false;
        for tile in area.tiles(self.map()) {
            if (self.shared.tile_type)(tile) != MP_STATION
                || !(self.shared.is_any_road_stop)(tile)
                || ((self.shared.station_type)(tile).0 == 8) != waypoint
            {
                continue;
            }
            let mut types = [63_u8; 2];
            let mut owners = [0x10_u8; 2];
            let mut bits = 0;
            if (self.shared.is_drive_through_stop)(tile) {
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
                bits = if (self.shared.drive_through_axis)(tile) == 0 {
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
            if flags & 1 != 0 && (types[0] != 63 || types[1] != 63) {
                let town = (self.shared.closest_town)(tile, u32::MAX);
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
                return cost.error(0xffff);
            }
            Area::between(self.map(), start, end)
        } else {
            if stop_type >= 2
                || width == 0
                || height == 0
                || !self.buildable_area(start, u16::from(width), u16::from(height))
                || (remove_road && flags & 32 != 0)
            {
                return cost.error(0xffff);
            }
            Area {
                tile: start,
                w: u16::from(width),
                h: u16::from(height),
            }
        };
        self.remove_generic_road(cost, ret, error, flags, area, waypoint, remove_road);
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
            let town = (self.shared.calc_closest_town)(tile, distance.wrapping_add(1));
            if town.is_null() {
                continue;
            }
            let owner = (self.shared.town_owner)(town);
            let xy = unsafe { crate::town::position(owner) };
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
        let geometry = (self.s.airport_geometry)(st);
        let id = (self.links().graphs.station_id)(st);
        self.nearest_town(
            crate::newgrf_airport::station_spec(st, unsafe { &*self.s.airport_runtime }),
            geometry.rotation,
            geometry.area.tile,
            geometry.area.tiles(self.map()).filter(|&t| {
                (self.shared.tile_type)(t) == MP_STATION
                    && (self.shared.station_type)(t).0 == 1
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
        let reuse = target != 0xfffd;
        if !reuse {
            target = 0xffff;
        }
        let distant = target != 0xffff;
        if distant && (!(self.s.distant_join)() || self.full_station(target).is_null()) {
            return cost.error(0xffff);
        }
        if airport >= 128 {
            return cost.error(0xffff);
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
            return cost.error(0xffff);
        }
        let rotation = (self.s.airport_layout)(spec, layout).rotation;
        let size = (self.s.airport_size)(spec);
        let (w, h) = if rotation == 2 || rotation == 6 {
            (size.h, size.w)
        } else {
            (size.w, size.h)
        };
        let area = Area {
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
            self.check_buildable(ret, t.tile, 0, &mut allowed, true, true);
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
        if (self.s.noise_enabled)() {
            if u32::from(unsafe { noise_ptr.read() }) + noise
                > u32::from((self.s.town_max_noise)(nearest))
            {
                message = self.s.strings.refuse_noise;
                refused = nearest;
            }
        } else if (authority.council_tolerance)() != 3 {
            let town = (self.shared.closest_town)(tile, u32::MAX);
            let mut count = 0_u32;
            for id in 0..(self.world().pool_size)() {
                let st = self.full_station(id as u16);
                if st.is_null() {
                    continue;
                }
                if (self.s.station_town)(st) == town
                    && (self.world().facilities)(st) & 8 != 0
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
        let mut st = self.join(
            ret,
            0xffff,
            target,
            adjacent,
            area,
            false,
            false,
            self.s.strings.remove_station,
        );
        if !ret.ok() {
            return cost.take(ret);
        }
        if st.is_null() && distant {
            st = self.full_station(target);
        }
        let name = if (self.s.airport_airplanes)(crate::airport::get_airport(airport, unsafe {
            &*self.s.airport_runtime
        })) {
            2
        } else {
            5
        };
        st = self.build_part(ret, st, flags, reuse, area, name);
        if !ret.ok() {
            return cost.take(ret);
        }
        if !st.is_null() && (self.s.airport_tile)(st) != u32::MAX {
            return cost.error(self.s.strings.close_airport);
        }
        for _ in self.airport_tiles(spec, layout, tile) {
            cost.add_money(crate::company::price(11));
        }
        if flags & 1 == 0 {
            return;
        }
        unsafe {
            noise_ptr.write(noise_ptr.read().wrapping_add(noise as u16));
        }
        (self.s.add_facility)(st, 3, tile);
        (self.s.set_airport_details)(st, airport, layout, rotation);
        (self.s.before_add_rect)(ret.0, st, area, 1);
        let owner = (self.world().station_owner)(st);
        let id = (self.links().graphs.station_id)(st);
        for t in self.airport_tiles(spec, layout, tile) {
            (self.s.make_airport)(t.tile, owner, id, t.gfx, 3);
            (self.s.set_random_bits)(
                t.tile,
                ((self.shared.random)(self.shared.context) & 15) as u8,
            );
            (self.s.airport_add)(st, t.tile);
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
        self.tiles_changed(st, true, 1);
        (self.s.station_view_invalidate)(st, -1);
        if (self.s.noise_enabled)() {
            (self.s.town_view_dirty)(nearest);
        }
    }
    fn remove_airport(&self, cost: Cost<'_>, ret: Cost<'_>, tile: u32, flags: u16) {
        crate::witness::hit(crate::witness::STATION_CMD_REMOVE_AIRPORT);
        let st = self.full_station((self.shared.station_index)(tile));
        if (self.shared.current_company)() != 0x11 {
            unsafe {
                (self.command.ownership)(cost.0, (self.world().station_owner)(st), 0);
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
            if (self.vehicle().vehicle_type)(v) != 3 || (self.vehicle().subtype)(v) > 2 {
                continue;
            }
            let target = unsafe { (&raw const (*identity.state).targetairport).read() };
            let state = unsafe { (&raw const (*identity.state).state).read() };
            if target == id && state != 14 {
                return cost.error(self.s.strings.aircraft_in_way);
            }
        }
        if flags & 1 != 0 {
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
            if (self.s.noise_enabled)() {
                (self.s.town_view_dirty)(nearest);
            }
        }
        let area = (self.s.airport_geometry)(st).area;
        for tile in area.tiles(self.map()) {
            if (self.shared.tile_type)(tile) != MP_STATION
                || (self.shared.station_type)(tile).0 != 1
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
            cost.add_money(crate::company::price(36));
            if flags & 1 != 0 {
                crate::ground::clear_square(tile, self.shared);
                (self.s.close_airport_tile_inspect)(tile);
            }
        }
        if flags & 1 != 0 {
            (self.s.delete_airport_storage)(st);
            (self.s.after_remove_rect)(st, area);
            (self.s.clear_airport)(st);
            (self.s.reset_facilities)(st, 8);
            (self.s.orders_dirty)();
            (self.s.station_view_invalidate)(st, -1);
            (self.s.airport_infrastructure)((self.world().station_owner)(st), u32::MAX);
            self.tiles_changed(st, false, 1);
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
        let st = c.full_station(id as u16);
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
        (self.shared.tile_type)(tile) == MP_STATION && (self.shared.station_type)(tile).0 == 5
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
            if self.dock(next) && (self.shared.station_gfx)(next) >= 4 {
                let st = self.full_station((self.shared.station_index)(next));
                (self.s.docking_add)(st, tile);
                (self.s.set_docking)(tile, true);
            }
            if (self.shared.tile_type)(next) == MP_INDUSTRY {
                let st = (self.s.neutral_for_tile)(next);
                if !st.is_null() {
                    (self.s.docking_add)(st, tile);
                    (self.s.set_docking)(tile, true);
                }
            }
            if (self.shared.tile_type)(next) == MP_STATION
                && (self.shared.station_type)(next).0 == 4
            {
                let st = self.full_station((self.shared.station_index)(next));
                (self.s.docking_add)(st, tile);
                (self.s.set_docking)(tile, true);
            }
        }
    }
    fn update_docking(&self, st: *mut BaseStation) {
        (self.s.docking_clear)(st);
        let industry = (self.s.associated_industry)(st);
        let area = if industry.is_null() {
            (self.s.ship_area)(st)
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
        let area = Area {
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
                let st = self.full_station((self.shared.station_index)(next));
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
        let reuse = target != 0xfffd;
        if !reuse {
            target = 0xffff;
        }
        let distant = target != 0xffff;
        if distant && (!(self.s.distant_join)() || self.full_station(target).is_null()) {
            return cost.error(0xffff);
        }
        let slope = (self.shared.tile_slope)(tile);
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
        self.default_bridge_ok(cost, tile, 5, dir);
        if !cost.ok() {
            return;
        }
        cost.set(crate::company::price(14));
        crate::landscape_cmd::clear_command(self.command, ret.0, tile, flags);
        if !ret.ok() {
            return cost.take(ret);
        }
        cost.add_money(ret.money());
        let flat = self.map().add_diag(tile, dir);
        if !(self.shared.has_water_ground)(flat) || (self.shared.tile_slope)(flat) != 0 {
            return cost.error(self.s.strings.unsuitable);
        }
        self.default_bridge_ok(ret, flat, 5, 4 + (dir & 1));
        if !ret.ok() {
            return cost.take(ret);
        }
        let water = (unsafe { &*self.s.clear }.water_class)(flat).0;
        let add_cost = !(self.shared.is_water_tile)(flat);
        crate::landscape_cmd::clear_command(self.command, ret.0, flat, flags);
        if !ret.ok() {
            return cost.take(ret);
        }
        if add_cost {
            cost.add_money(ret.money());
        }
        let beyond = self.map().add_diag(flat, dir);
        if (self.shared.tile_type)(beyond) != MP_WATER || (self.shared.tile_slope)(beyond) != 0 {
            return cost.error(self.s.strings.unsuitable);
        }
        let (dx, dy) = match dir {
            0 => (-1, 0),
            3 => (0, -1),
            _ => (0, 0),
        };
        let area = Area {
            tile: tile.wrapping_add(self.map().diff(dx, dy)),
            w: if dir & 1 == 0 { 2 } else { 1 },
            h: if dir & 1 == 0 { 1 } else { 2 },
        };
        let mut st = self.join(
            ret,
            0xffff,
            target,
            adjacent,
            area,
            false,
            false,
            self.s.strings.remove_station,
        );
        if !ret.ok() {
            return cost.take(ret);
        }
        if st.is_null() && distant {
            st = self.full_station(target);
        }
        st = self.build_part(ret, st, flags, reuse, area, 4);
        if !ret.ok() {
            return cost.take(ret);
        }
        if flags & 1 != 0 {
            (self.s.ship_area_add)(st, tile);
            (self.s.ship_area_add)(st, flat);
            (self.s.add_facility)(st, 4, tile);
            (self.s.before_add_rect)(ret.0, st, area, 1);
            let owner = (self.world().station_owner)(st);
            if water == 1
                && !((self.shared.has_water_class)(flat)
                    && (unsafe { &*self.s.clear }.water_class)(flat).0 == 1
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
            self.tiles_changed(st, true, 5);
        }
    }
    fn dock_land(&self, tile: u32) -> u32 {
        if (self.shared.station_gfx)(tile) < 4 {
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
        let st = self.full_station((self.shared.station_index)(tile));
        let owner = (self.world().station_owner)(st);
        unsafe {
            (self.command.ownership)(cost.0, owner, 0);
        }
        if !cost.ok() {
            return;
        }
        if !self.dock(tile) {
            return cost.error(0xffff);
        }
        let land = self.dock_land(tile);
        if land == u32::MAX {
            return cost.error(0xffff);
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
        if flags & 1 != 0 {
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
            (self.s.after_remove_tile)(st, land);
            (self.s.after_remove_tile)(st, water);
            let area = self.shrink(st, (self.s.ship_area)(st), 5);
            (self.s.set_ship_area)(st, area);
            self.update_docking(st);
            if area.tile == u32::MAX {
                (self.s.set_ship_area)(
                    st,
                    Area {
                        tile: u32::MAX,
                        w: 0,
                        h: 0,
                    },
                );
                (self.s.docking_clear)(st);
                (self.s.reset_facilities)(st, 16);
                (self.s.orders_dirty)();
            }
            (self.s.station_infrastructure)(owner, u32::MAX - 1);
            self.tiles_changed(st, false, 5);
            self.clear_docking_neighbours(land);
            self.clear_docking_neighbours(water);
            let id = (self.links().graphs.station_id)(st);
            let mut from = 0;
            while let Some(v) = (self.shared.vehicle_next)(from) {
                let identity = (self.s.aircraft_vehicle)(v);
                from = identity.id + 1;
                if (self.vehicle().vehicle_type)(v) != 2
                    || (self.vehicle().order_destination)(v) != id
                {
                    continue;
                }
                if (self.vehicle().order_type)(v) == 3 {
                    let tile = (self.vehicle().tile)(v);
                    if !((self.shared.docking_tile)(tile)
                        && unsafe {
                            crate::ship_control::openttd_rust_ship_is_destination(
                                tile,
                                u32::from(id),
                                self.s.ship,
                                self.s.shared,
                            )
                        })
                    {
                        crate::vehicle::visits::leave_station(v, self.vehicle().visits());
                    }
                }
                if (self.vehicle().order_type)(v) == 1 && (self.world().facilities)(st) & 16 == 0 {
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
        cost.set(crate::company::price(39));
        let _ = ret;
    }
    fn build_oilrig(&self, ret: Cost<'_>, tile: u32) {
        crate::witness::hit(crate::witness::STATION_CMD_BUILD_OILRIG);
        if !(self.s.station_available)() {
            (self.s.oilrig_unavailable)(tile);
            return;
        }
        let st = (self.s.new_station)(tile);
        (self.s.index_insert)(st);
        (self.s.set_town)(st, (self.shared.closest_town)(tile, u32::MAX));
        (self.s.default_name)(st, tile, 3);
        (self.s.associate_industry)(st, tile);
        crate::animated_tile::openttd_rust_animation_delete(tile, false);
        (self.s.make_oilrig)(
            tile,
            (self.links().graphs.station_id)(st),
            (unsafe { &*self.s.clear }.water_class)(tile).0,
        );
        (self.s.set_station_owner)(st, 0x10);
        (self.s.set_airport_kind_rotation)(st, 9, 0);
        (self.s.airport_add)(st, tile);
        (self.s.ship_area_add)(st, tile);
        (self.s.set_facilities)(st, 24);
        (self.s.set_build_date)(st, (self.shared.calendar_date)());
        self.update_docking(st);
        (self.s.before_add_tile)(ret.0, st, tile, 2);
        (self.s.virtual_coord)(st);
        if (self.shared.station_serve_neutral)() {
            let mut context = OilrigNearby { c: self };
            unsafe {
                (self.s.with_nearby)(st, (&raw mut context).cast(), oilrig_nearby_visit);
            }
        }
        (self.s.catchment)(st, false);
        unsafe {
            crate::station_service::openttd_rust_station_acceptance(st, self.s.station, 0);
        }
    }
    fn delete_oilrig(&self, tile: u32) {
        crate::witness::hit(crate::witness::STATION_CMD_DELETE_OILRIG);
        let st = self.full_station((self.shared.station_index)(tile));
        unsafe {
            crate::water::restore(
                tile,
                0x10,
                self.shared,
                &*(self.shared.water_services)(),
                &*self.s.restoration,
            );
        }
        assert!((self.world().facilities)(st) == 24 && (self.s.airport_type)(st) == 9);
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
            if ((self.vehicle().owner)(v) == company) != include_company {
                continue;
            }
            for index in 0..(self.links().order_count)(list) {
                let order = (self.links().order_destination)(list, index);
                if order.destination == station && matches!(order.kind.0, 1 | 6) {
                    return true;
                }
            }
        }
        false
    }
}
struct OilrigNearby<'a> {
    c: &'a Ctx<'a>,
}
unsafe extern "C" fn oilrig_nearby_visit(context: *mut std::ffi::c_void, st: *mut BaseStation) {
    let c = unsafe { (*context.cast::<OilrigNearby<'_>>()).c };
    (c.s.catchment)(st, true);
    unsafe {
        crate::station_service::openttd_rust_station_acceptance(st, c.s.station, 1);
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
pub unsafe extern "C" fn openttd_rust_station_possible_docking(
    s: *const CommandServicesTable,
    tile: u32,
) -> bool {
    unsafe { Ctx::new(s) }.possible_docking(tile)
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
pub unsafe extern "C" fn openttd_rust_station_in_orders(
    s: *const CommandServicesTable,
    station: u16,
    include_company: bool,
    company: u8,
) -> bool {
    unsafe { Ctx::new(s) }.in_orders(station, include_company, company)
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
        if (self.shared.current_company)() == 0x11 {
            return cost.zero();
        }
        if self.road_type(tile, true) != 63 {
            let owner = self.road_owner(tile, true);
            if owner != 0x10 {
                unsafe {
                    (self.command.ownership)(cost.0, owner, 0);
                }
                if !cost.ok() {
                    return;
                }
            }
        }
        if self.road_type(tile, false) != 63 {
            let owner = self.road_owner(tile, false);
            if owner == 0x0f {
                let bits = (self.shared.road_bits)(tile, 0, false);
                unsafe {
                    crate::road_cmd::openttd_rust_road_allow_remove(
                        (self.shared.road_commands)(),
                        cost.0,
                        tile,
                        bits,
                        0x0f,
                        0,
                        flags,
                        true,
                    );
                }
                if !cost.ok() {
                    return;
                }
            } else if owner != 0x10 {
                unsafe {
                    (self.command.ownership)(cost.0, owner, 0);
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
        if flags & 2 != 0 {
            return self.auto_clear(cost, tile);
        }
        match (self.shared.station_type)(tile).0 {
            0 => self.remove_rail_all(cost, ret, error, flags, tile, false),
            7 => self.remove_rail_all(cost, ret, error, flags, tile, true),
            1 => self.remove_airport(cost, ret, tile, flags),
            2 | 3 => {
                if (self.shared.is_drive_through_stop)(tile) {
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
            6 => self.remove_buoy(cost, tile, flags),
            5 => self.remove_dock(cost, ret, tile, flags),
            _ => cost.error(0xffff),
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
                    if (self.shared.is_drive_through_stop)(tile) {
                        (self.s.autoslope_axis)(
                            tile,
                            z,
                            slope,
                            (self.shared.drive_through_axis)(tile),
                        )
                    } else {
                        (self.rail().entrance_edge)(
                            tile,
                            z,
                            slope,
                            (self.shared.bay_stop_direction)(tile),
                        )
                    }
                }
                _ => false,
            };
            if allowed {
                return cost.set(crate::company::price(57));
            }
        }
        crate::landscape_cmd::clear_command(self.command, cost.0, tile, flags);
    }
    fn change_owner(
        &self,
        ownership: &crate::tile_owner::Context,
        cost: Cost<'_>,
        ret: Cost<'_>,
        error: Cost<'_>,
        tile: u32,
        old: u8,
        new: u8,
    ) {
        crate::witness::hit(crate::witness::STATION_CMD_CHANGE_OWNER);
        if (self.shared.is_any_road_stop)(tile) {
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
                        if new == 255 { 0x10 } else { new },
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
                    if !(self.rail_map().station_blocked)(tile) {
                        let rt = (self.rail_map().rail_type)(tile).0;
                        (self.rail().rail_infrastructure)(old, rt, u32::MAX);
                        (self.rail().rail_infrastructure)(new, rt, 1);
                    }
                }
                5 | 6 if (unsafe { &*self.s.clear }.water_class)(tile).0 == 1 => {
                    (self.s.water_infrastructure)(old, u32::MAX);
                    (self.s.water_infrastructure)(new, 1);
                }
                _ => (),
            }
            if kind != 6 && kind != 1 {
                (self.s.station_infrastructure)(old, u32::MAX);
                (self.s.station_infrastructure)(new, 1);
            }
            (self.shared.set_tile_owner)(tile, new);
            (self.s.station_list_classes_invalidate)();
        } else if (self.shared.is_drive_through_stop)(tile) {
            let kind = (self.shared.station_type)(tile).0;
            self.nested(cost, tile, 33, |flags| {
                self.remove_road_command(
                    cost,
                    ret,
                    error,
                    flags,
                    tile,
                    tile,
                    1,
                    1,
                    u8::from(kind == 2),
                    false,
                    kind == 8,
                );
            });
            assert_eq!((self.shared.tile_type)(tile), 2);
            // SAFETY: Nested removal has returned; the native slots can be reused.
            unsafe { crate::tile_owner::openttd_rust_tile_change_owner(ownership, tile, old, new) };
        } else {
            crate::landscape_cmd::clear_command(self.command, cost.0, tile, 33);
            let kind = (self.shared.tile_type)(tile);
            if (kind == 6 || (kind == 5 && (self.shared.station_type)(tile).0 == 6))
                && (self.shared.tile_owner)(tile) == old
            {
                (self.shared.set_tile_owner)(tile, 0x10);
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
    c.change_owner(
        ownership,
        c.c(ownership.cost),
        c.c(ownership.ret),
        c.c(ownership.error),
        tile,
        old,
        new,
    );
}

impl Ctx<'_> {
    fn expand_rail(&self, out: Cost<'_>, st: *const BaseStation, new_area: Area) -> Area {
        let old = unsafe { (self.s.train_area)(st) };
        let x = self
            .map()
            .tile_x(old.tile)
            .min(self.map().tile_x(new_area.tile));
        let y = self
            .map()
            .tile_y(old.tile)
            .min(self.map().tile_y(new_area.tile));
        let area = Area {
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
        train_area,
        rail_spec,
        links,
        bounds,
        location,
        move_sign,
        virtual_coord,
        catchment,
        catchment_dirty,
        facility_widget,
        select_invalidate,
        radius,
        station_available,
        new_station,
        index_insert,
        set_town,
        default_name,
        have_rating,
        before_add_rect,
        road_waypoint,
        name_equals,
        set_name,
        rect_contains,
        flip_airport_closed,
        closed_widget,
        distant_join,
        rail_class_count,
        rail_waypoint_class,
        rail_spec_count,
        class_rail_spec,
        rail_spec_info,
        rail_layout,
        allocate_rail_spec,
        assign_rail_spec,
        deallocate_rail_spec,
        rail_boolean,
        rail_callback_error,
        set_train_area,
        add_facility,
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
        after_remove_tile,
        reset_facilities,
        orders_dirty,
        vehicle,
        stop_on_town_road,
        stop_on_competitor_road,
        road_class_count,
        road_waypoint_class,
        road_spec_count,
        class_road_spec,
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
        add_stop_area,
        clear_stop_area,
        before_add_tile,
        make_drive_stop,
        make_bay_stop,
        delete_stop,
        set_custom_road_index,
        custom_road_index,
        stop_random_bits,

        remove_stop_tile_data,
        close_stop_inspect,
        road_waypoint_area,
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
        noise_enabled,
        town_max_noise,
        station_town,
        airport_tile,
        airport_type,
        airport_geometry,
        airport_airplanes,
        set_airport_details,
        make_airport,
        airport_add,
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
        after_remove_rect,
        clear_airport,
        close_airport_inspect,
        ship,
        clear,
        restoration,
        set_docking,
        ship_area,
        set_ship_area,
        ship_area_add,
        docking_clear,
        docking_add,
        water_infrastructure,
        make_dock,
        neutral_for_tile,
        oilrig_unavailable,
        associate_industry,
        make_oilrig,
        set_station_owner,
        set_airport_kind_rotation,
        set_facilities,
        set_build_date,
        with_nearby,
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
        rail_axis,
        road_ground,
        road_waypoint_area_add,
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
    "OpenTTDStationArea" ["probe"] => Area {
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
    "OpenTTDStationBounds" ["probe"] => Bounds {
        left,
        top,
        right,
        bottom,
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
    "OpenTTDAirportGeometry" ["probe"] => AirportGeometry {
        area,
        rotation,
    };
}
