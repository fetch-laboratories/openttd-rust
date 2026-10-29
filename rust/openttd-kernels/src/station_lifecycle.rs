/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Station naming, tile evolution and destruction. Native shells retain their pool
//! identities and unmoved fields. All calls run under the game-state mutex; raw
//! field accesses and owner borrows end before a service can reenter an owner.
#![allow(unsafe_code, clippy::cast_possible_truncation, clippy::too_many_lines)]
use crate::cargo_type::{NUM_CARGO, is_valid_cargo_type};
use crate::industry_type::{IT_INVALID, NUM_INDUSTRYTYPES};
use crate::map_func::TileTypeAccess;
use crate::road_map::{ROADSIDE_BARREN, ROADSIDE_GRASS, ROADSIDE_PAVED};
use crate::services::{Map, Services as Shared};
use crate::station::BaseStation;
use crate::station_type::*;
use crate::tile_type::{INVALID_TILE, MP_INDUSTRY, MP_STATION, MP_TREES, MP_WATER};
use crate::vehicle::VehicleRef;
use std::ptr::NonNull;

#[repr(C)]
pub struct RoadStopTileData {
    _private: [u8; 0],
}
#[repr(C)]
pub struct IndustrySpec {
    _private: [u8; 0],
}
/// Process-lifetime string constants from `table/strings.h`.
#[repr(C)]
pub struct Names {
    base: u32,
    central: u32,
    airport: u32,
    oilfield: u32,
    docks: u32,
    heliport: u32,
    mines: u32,
    lakeside: u32,
    woods: u32,
    forest: u32,
    valley: u32,
    heights: u32,
    north: u32,
    south: u32,
    east: u32,
    west: u32,
    transfer: u32,
    halt: u32,
    exchange: u32,
    annexe: u32,
    sidings: u32,
    branch: u32,
    upper: u32,
    lower: u32,
    fallback: u32,
    undefined: u32,
    null: u32,
}
#[repr(C)]
pub struct Services {
    pub shared: &'static Shared,
    pub catchment: &'static crate::station::CatchmentServices,
    pub commands: extern "C" fn() -> *const crate::station_cmd::CommandServicesTable,
    pub visits: extern "C" fn() -> *const crate::vehicle::visits::Services,
    pub aircraft: extern "C" fn() -> *const crate::aircraft::Leaves,
    pub storage: extern "C" fn() -> *const crate::cargo_storage::Services,
    pub allocation: extern "C" fn() -> crate::map_func::TileTypeAllocation,
    pub check: extern "C" fn(bool),
    pub climate: extern "C" fn() -> u8,
    names: Names,
    pub town: extern "C" fn(*mut BaseStation) -> *const crate::services::TownHandle,
    pub industry_type: extern "C" fn(*mut BaseStation) -> u8,
    pub set_industry_type: extern "C" fn(*mut BaseStation, u8),
    pub spec: extern "C" fn(u8) -> *const IndustrySpec,
    pub spec_name: extern "C" fn(*const IndustrySpec) -> u32,
    pub spec_enabled: extern "C" fn(*const IndustrySpec) -> bool,
    pub spec_life: extern "C" fn(*const IndustrySpec) -> u8,
    pub set_random_bits: extern "C" fn(*mut BaseStation, u16),
    pub freeform: extern "C" fn() -> bool,
    pub cargo_classes: extern "C" fn(u8) -> u16,
    pub forest: extern "C" fn(u32) -> bool,
    pub stop_data: extern "C" fn(*mut BaseStation, usize) -> *mut RoadStopTileData,
    pub stop_count: extern "C" fn(*mut BaseStation) -> usize,
    pub stop_tile: extern "C" fn(*const RoadStopTileData) -> u32,
    pub stop_random: extern "C" fn(*const RoadStopTileData) -> u8,
    pub stop_frame: extern "C" fn(*const RoadStopTileData) -> u8,
    pub stop_set_random: extern "C" fn(*mut RoadStopTileData, u8),
    pub stop_set_frame: extern "C" fn(*mut RoadStopTileData, u8),
    pub stop_append: extern "C" fn(*mut BaseStation, u32, u8, u8),
    pub stop_swap_pop: extern "C" fn(*mut BaseStation, usize),
    pub by_tile: extern "C" fn(u32) -> *mut BaseStation,
    pub airport_animation: extern "C" fn(*mut BaseStation, u32, u8),
    pub rail_animation: extern "C" fn(*mut BaseStation, u32, u8),
    pub road_animation: extern "C" fn(*mut BaseStation, u32, u8),
    pub cleaning_pool: extern "C" fn() -> bool,
    pub cargo: extern "C" fn(*mut BaseStation, u8) -> *const crate::cargo_storage::List,
    pub flows: extern "C" fn(*mut BaseStation, u8) -> *mut crate::cargo_flow::FlowMap,
    pub delete_airport_storage: extern "C" fn(*mut BaseStation),
    pub invalidate_list: extern "C" fn(*mut BaseStation, i32),
    pub invalidate_lists: extern "C" fn(),
    pub close_view: extern "C" fn(StationID),
    pub remove_orders: extern "C" fn(StationID),
    pub delete_news: extern "C" fn(StationID),
    pub remove_index: extern "C" fn(StationID),
    pub sign_valid: extern "C" fn(*mut BaseStation) -> bool,
    pub remove_sign: extern "C" fn(StationID),
    pub next_vehicle: extern "C" fn(u32) -> Option<VehicleRef>,
    pub vehicle_id: extern "C" fn(VehicleRef) -> u32,
    pub last_loading: extern "C" fn(VehicleRef) -> StationID,
    pub airport_maintenance: extern "C" fn(*const crate::station_cmd::AirportSpec) -> u16,
    pub maintenance_price: usize,
}
impl Services {
    fn commands(&self) -> &crate::station_cmd::CommandServicesTable {
        unsafe { &*(self.commands)() }
    }
    fn map_access(&self) -> TileTypeAccess {
        unsafe { TileTypeAccess::raw((self.allocation)(), self.shared.tile_type) }
    }
}

/// `StationNameInformation`, with default-name and industry-type sets independent.
struct NameInformation {
    used: [bool; 256],
    industries: [bool; NUM_INDUSTRYTYPES as usize],
}
impl NameInformation {
    fn available(&self, name: u32, s: &Services) -> bool {
        (s.check)(name >= s.names.base && name < s.names.fallback);
        !self.used[(name - s.names.base) as usize]
    }
    fn set_used(&mut self, name: u32, s: &Services) {
        (s.check)(name >= s.names.base && name < s.names.fallback);
        self.used[(name - s.names.base) as usize] = true;
    }
}
fn count_around(
    tile: u32,
    map: Map,
    size_y: u32,
    freeform: bool,
    mut matches: impl FnMut(u32) -> bool,
) -> i32 {
    let mut count = 0;
    for dx in -3..=3 {
        for dy in -3..=3 {
            let t = map.tile_add_wrap(tile, dx, dy, size_y, freeform);
            if t != INVALID_TILE && matches(t) {
                count += 1;
            }
        }
    }
    count
}
fn industry_owner(tile: u32, m: &TileTypeAccess, s: &Services) -> *mut crate::industry::Industry {
    (s.check)(m.read(tile) == MP_INDUSTRY);
    let shell = (s.catchment.industry_by_id)(crate::industry_type::IndustryID(m.m2(tile).unwrap()));
    (s.check)(!shell.is_null());
    (s.catchment.industry_owner)(shell)
}
fn get_industry_type(tile: u32, m: &TileTypeAccess, s: &Services) -> u8 {
    let owner = industry_owner(tile, m, s);
    unsafe { (*crate::industry::openttd_rust_industry_metadata(owner)).kind }
}
fn mine(tile: u32, m: &TileTypeAccess, s: &Services) -> bool {
    if m.read(tile) != MP_INDUSTRY {
        return false;
    }
    let owner = industry_owner(tile, m, s);
    let kind = unsafe { (*crate::industry::openttd_rust_industry_metadata(owner)).kind };
    if (s.spec_life)((s.spec)(kind)) & crate::industry_type::LIFE_EXTRACTIVE == 0 {
        return false;
    }
    let mut result = false;
    crate::industry::for_each_produced_cargo(owner, |cargo| {
        if !result
            && is_valid_cargo_type(cargo)
            && (s.cargo_classes)(cargo)
                & (crate::cargotype::CC_LIQUID
                    | crate::cargotype::PASSENGERS
                    | crate::cargotype::CC_MAIL)
                == 0
        {
            result = true;
        }
    });
    result
}
/// Complete `GenerateStationName`, including industry aliases and fallback exhaustion.
pub(crate) fn generate_name(st: *mut BaseStation, tile: u32, class: u8, s: &Services) -> u32 {
    let m = s.map_access();
    let map = s.shared.map();
    let size_y = (s.shared.map_max_y)() + 1;
    let freeform = (s.freeform)();
    let n = &s.names;
    let town = (s.town)(st);
    let mut info = NameInformation {
        used: [false; 256],
        industries: [false; NUM_INDUSTRYTYPES as usize],
    };
    let mut from = 0;
    loop {
        let other = (s.catchment.next_station)(from);
        if other.is_null() {
            break;
        }
        from = u32::from((s.catchment.id)(other).0) + 1;
        if other == st || (s.town)(other) != town {
            continue;
        }
        let kind = (s.industry_type)(other);
        if kind != IT_INVALID {
            info.industries[usize::from(kind)] = true;
            let name = (s.spec_name)((s.spec)(kind));
            if name != n.undefined {
                for it in 0..NUM_INDUSTRYTYPES {
                    let spec = (s.spec)(it);
                    if (s.spec_enabled)(spec) && (s.spec_name)(spec) == name {
                        info.industries[usize::from(it)] = true;
                    }
                }
            }
            continue;
        }
        let name = (s.commands().string_id)(other);
        if name >= n.base && name < n.fallback {
            info.set_used(if name == n.forest { n.woods } else { name }, s);
        }
    }
    let mut spiral =
        crate::spiral::square(map.tile_x(tile), map.tile_y(tile), 7, map.size_x(), size_y);
    while !spiral.is_end() {
        let at = map.tile_xy(spiral.x, spiral.y);
        spiral = spiral.advance(map.size_x(), size_y);
        if m.read(at) != MP_INDUSTRY {
            continue;
        }
        let kind = get_industry_type(at, &m, s);
        let spec = (s.spec)(kind);
        if (s.spec_name)(spec) == n.undefined {
            continue;
        }
        info.set_used(n.oilfield, s);
        info.set_used(n.mines, s);
        if info.industries[usize::from(kind)] {
            continue;
        }
        if (s.spec_name)(spec) != n.null {
            (s.set_industry_type)(st, kind);
            return n.fallback;
        }
        break;
    }
    let special = match class {
        2 => Some(n.airport),
        3 => Some(n.oilfield),
        4 => Some(n.docks),
        5 => Some(n.heliport),
        _ => None,
    };
    if let Some(name) = special {
        if info.available(name, s) {
            return name;
        }
    }
    if info.available(n.mines, s)
        && count_around(tile, map, size_y, freeform, |t| mine(t, &m, s)) >= 2
    {
        return n.mines;
    }
    let town_tile = unsafe { crate::town::position((s.shared.town_owner)(town)) };
    if map.distance_max(tile, town_tile) < 8 {
        if info.available(n.base, s) {
            return n.base;
        }
        if info.available(n.central, s) {
            return n.central;
        }
    }
    if info.available(n.lakeside, s)
        && map.distance_from_edge(tile, size_y) < 20
        && count_around(tile, map, size_y, freeform, |t| {
            m.read(t) == MP_WATER && crate::water_map::is_water(&m, t).unwrap()
        }) >= 5
    {
        return n.lakeside;
    }
    if info.available(n.woods, s)
        && (count_around(tile, map, size_y, freeform, |t| m.read(t) == MP_TREES) >= 8
            || count_around(tile, map, size_y, freeform, |t| (s.forest)(t)) >= 2)
    {
        return if (s.climate)() == crate::landscape_type::TROPIC {
            n.forest
        } else {
            n.woods
        };
    }
    let z = crate::tile_map::get_tile_z(&m, tile, map.size_x(), size_y).unwrap();
    let town_z = crate::tile_map::get_tile_z(&m, town_tile, map.size_x(), size_y).unwrap();
    if z < town_z && info.available(n.valley, s) {
        return n.valley;
    }
    if z > town_z && info.available(n.heights, s) {
        return n.heights;
    }
    for name in if map.tile_x(tile) < map.tile_x(town_tile) {
        [n.south, n.west]
    } else {
        [n.north, n.east]
    } {
        info.set_used(name, s);
    }
    for name in if map.tile_y(tile) < map.tile_y(town_tile) {
        [n.south, n.east]
    } else {
        [n.north, n.west]
    } {
        info.set_used(name, s);
    }
    for name in [
        n.north, n.south, n.east, n.west, n.transfer, n.halt, n.exchange, n.annexe, n.sidings,
        n.branch, n.upper, n.lower,
    ] {
        if info.available(name, s) {
            return name;
        }
    }
    n.fallback
}
/// `Station::AddFacility`; callers pass the original enum position.
pub(crate) fn add_facility(st: *mut BaseStation, facility: u8, tile: u32, s: &Services) {
    let c = s.commands();
    let world = unsafe { &*c.station };
    if (world.facilities)(st) == 0 {
        (c.move_sign)(st, tile);
        (s.set_random_bits)(st, (s.shared.random)(std::ptr::null_mut()) as u16);
    }
    (c.set_facility_bits)(st, 1 << facility);
    (c.set_station_owner)(st, (s.shared.current_company)());
    (c.set_build_date)(st, crate::timer_game_common::calendar_date().0);
    (c.orders_dirty)();
}

/// `BaseStation::SetRoadStopTileData`, over the canonical native container.
pub(crate) fn set_stop_data(
    st: *mut BaseStation,
    tile: u32,
    data: u8,
    animation: bool,
    s: &Services,
) -> bool {
    for i in 0..(s.stop_count)(st) {
        let entry = (s.stop_data)(st, i);
        if (s.stop_tile)(entry) != tile {
            continue;
        }
        let old = if animation {
            (s.stop_frame)(entry)
        } else {
            (s.stop_random)(entry)
        };
        if old == data {
            return false;
        }
        if animation {
            (s.stop_set_frame)(entry, data);
        } else {
            (s.stop_set_random)(entry, data);
        }
        return true;
    }
    (s.stop_append)(
        st,
        tile,
        if animation { 0 } else { data },
        if animation { data } else { 0 },
    );
    data != 0
}
pub(crate) fn remove_stop_data(st: *mut BaseStation, tile: u32, s: &Services) {
    for i in 0..(s.stop_count)(st) {
        if (s.stop_tile)((s.stop_data)(st, i)) == tile {
            (s.stop_swap_pop)(st, i);
            return;
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_stop_data(
    s: &Services,
    st: *mut BaseStation,
    tile: u32,
    data: u8,
    animation: bool,
) -> bool {
    set_stop_data(st, tile, data, animation, s)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_remove_stop_data(
    s: &Services,
    st: *mut BaseStation,
    tile: u32,
) {
    remove_stop_data(st, tile, s);
}

/// `TileLoop_Station`. The map descriptor remains valid through the original tile
/// callbacks under the game-loop interval in world-state.md.
pub(crate) unsafe fn tile_loop(tile: u32, call: *mut crate::ground::Call, s: &Services) {
    let m = s.map_access();
    let map = s.shared.map();
    let size_y = (s.shared.map_max_y)() + 1;
    let st = (s.by_tile)(tile);
    match crate::station_map::get_station_type(&m, tile)
        .unwrap_or_else(|| (s.shared.station_type)(tile).0)
    {
        STATION_AIRPORT => (s.airport_animation)(st, tile, AAT_TILE_LOOP),
        STATION_RAIL | RAIL_WAYPOINT => (s.rail_animation)(st, tile, SAT_TILE_LOOP),
        STATION_DOCK | STATION_OILRIG | BUOY => {
            if crate::station_map::get_station_type(&m, tile) == Some(STATION_DOCK)
                && crate::tile_map::get_tile_slope(&m, tile, map.size_x(), size_y).unwrap()
                    != crate::slope_func::SLOPE_FLAT
            {
                return;
            }
            unsafe {
                crate::water::water_tile_loop(
                    tile,
                    MP_STATION,
                    false,
                    map.size_x(),
                    size_y,
                    call,
                    (s.shared.water_services)(),
                    s.shared,
                    &m,
                );
            }
        }
        STATION_TRUCK | STATION_BUS => (s.road_animation)(st, tile, SAT_TILE_LOOP),
        ROAD_WAYPOINT => {
            let snow = || crate::station_map::road_waypoint_snow(&m, tile).unwrap();
            let change = match (s.climate)() {
                crate::landscape_type::ARCTIC => {
                    snow()
                        != (crate::tile_map::get_tile_z(&m, tile, map.size_x(), size_y).unwrap()
                            > i32::from((s.shared.snow_line)()))
                }
                crate::landscape_type::TROPIC => {
                    crate::tile_map::get_tropic_zone(&m, tile).unwrap()
                        == crate::tile_type::TROPICZONE_DESERT
                        && !snow()
                }
                _ => false,
            };
            if change {
                crate::station_map::toggle_road_waypoint_snow(&m, tile).unwrap();
                (s.shared.mark_dirty)(tile);
            }
            let town = (s.shared.closest_town)(tile, u32::MAX);
            let zone = if town.is_null() {
                0
            } else {
                unsafe { crate::town::get_radius_group((s.shared.town_owner)(town), tile, map) }
            };
            let new = if zone != 0 {
                ROADSIDE_PAVED
            } else {
                ROADSIDE_GRASS
            };
            let current = crate::station_map::road_waypoint_roadside(&m, tile).unwrap();
            if current != new {
                crate::station_map::set_road_waypoint_roadside(
                    &m,
                    tile,
                    if current == ROADSIDE_BARREN {
                        new
                    } else {
                        ROADSIDE_BARREN
                    },
                )
                .unwrap();
                (s.shared.mark_dirty)(tile);
            }
            (s.road_animation)(st, tile, SAT_TILE_LOOP);
        }
        _ => {}
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_tile_loop(
    s: &Services,
    tile: u32,
    call: *mut crate::ground::Call,
) {
    unsafe {
        tile_loop(tile, call, s);
    }
}

/// `Station::~Station`; the native destructor retains only member destruction.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_destruct(s: &Services, st: *mut BaseStation) {
    let c = s.commands();
    let w = unsafe { &*c.station };
    let state = (w.owner)(st);
    if (s.cleaning_pool)() {
        for cargo in 0..NUM_CARGO {
            let list = (s.cargo)(st, cargo);
            if !list.is_null() {
                unsafe {
                    crate::cargo_storage::openttd_rust_cargo_list_clear(list.cast_mut());
                }
            }
        }
        return;
    }
    let visits = unsafe { &*(s.visits)() };
    loop {
        let front = unsafe { crate::station_service::openttd_rust_station_queue_first(state) };
        if front.is_null() {
            break;
        }
        let vehicle = VehicleRef(
            NonNull::new(
                unsafe { crate::station_service::openttd_rust_station_queue_value(front) }.cast(),
            )
            .unwrap(),
        );
        crate::vehicle::visits::leave_station(vehicle, visits);
    }
    let id = (s.catchment.id)(st);
    unsafe {
        crate::aircraft::openttd_rust_aircraft_invalidate_target((s.aircraft)(), id.0);
    }
    let graphs = w.graphs;
    for cargo in 0..NUM_CARGO {
        let graph = (graphs.get_if_valid)((graphs.graph_id)(st, cargo));
        if graph.is_null() {
            continue;
        }
        let owner = (graphs.owner)(graph);
        let mut node = 0;
        while node < unsafe { &*owner }.size() {
            let other_id = unsafe { &*owner }.nodes[usize::from(node)].station;
            let other = (w.get)(other_id);
            let flows = (s.flows)(other, cargo);
            if !flows.is_null() {
                unsafe {
                    crate::cargo_flow::openttd_rust_flow_map_erase(flows, id.0);
                }
                let destination = (graphs.node)(st, cargo);
                let edge_live = unsafe { &*owner }.nodes[usize::from(node)]
                    .edge(destination)
                    .is_some_and(|e| e.last_update() != crate::timer_game_common::INVALID_DATE);
                if edge_live {
                    unsafe {
                        (&mut *flows).deleted(id.0);
                        crate::station_service::openttd_rust_station_reroute(
                            other, cargo, id.0, other_id, w, c.links,
                        );
                    }
                }
            }
            node += 1;
        }
        unsafe {
            crate::linkgraph::live::openttd_rust_live_graph_remove(
                graph,
                (graphs.node)(st, cargo),
                graphs,
            );
        }
        if unsafe { &*owner }.size() == 0 {
            crate::linkgraph_schedule::openttd_rust_schedule_dequeue(graph);
            (graphs.delete_graph)(graph);
        }
    }
    let vehicles = unsafe { &*c.vehicle };
    let mut from = 0;
    while let Some(v) = (s.next_vehicle)(from) {
        from = (s.vehicle_id)(v).wrapping_add(1);
        if (vehicles.last_station)(v) == id.0 {
            (vehicles.write_last_station)(v, INVALID_STATION);
        }
        if (s.last_loading)(v) == id {
            (visits.set_last_loading)(v, INVALID_STATION);
        }
    }
    unsafe {
        crate::station::openttd_rust_station_remove_from_nearby_lists(st, s.catchment);
    }
    (s.delete_airport_storage)(st);
    if (w.station_owner)(st) == crate::company_type::OWNER_NONE {
        (s.invalidate_lists)();
    } else {
        (s.invalidate_list)(st, 0);
    }
    (s.close_view)(id);
    (s.remove_orders)(id);
    (s.delete_news)(id);
    let storage = (s.storage)();
    for cargo in 0..NUM_CARGO {
        let list = (s.cargo)(st, cargo);
        if !list.is_null() {
            unsafe {
                crate::cargo_storage::truncate_station(
                    list.cast_mut(),
                    *storage,
                    u32::MAX,
                    std::ptr::null_mut(),
                );
            }
        }
    }
    unsafe {
        crate::cargo_storage::openttd_rust_cargo_invalidate(storage, id.0, u8::MAX);
    }
    (s.remove_index)(id);
    if (s.sign_valid)(st) {
        (s.remove_sign)(id);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_airport_maintenance(s: &Services, owner: u8) -> i64 {
    airport_maintenance(s, owner)
}
pub(crate) fn airport_maintenance(s: &Services, owner: u8) -> i64 {
    let c = s.commands();
    let w = unsafe { &*c.station };
    let mut total: i64 = 0;
    let mut from = 0;
    loop {
        let st = (s.catchment.next_station)(from);
        if st.is_null() {
            break;
        }
        from = u32::from((s.catchment.id)(st).0) + 1;
        if (w.station_owner)(st) == owner && (w.facilities)(st) & FACILITY_AIRPORT != 0 {
            let spec = crate::newgrf_airport::station_spec(st, crate::newgrf_airport::runtime());
            total = total.saturating_add(
                crate::company::price(s.maintenance_price)
                    .saturating_mul(i64::from((s.airport_maintenance)(spec))),
            );
        }
    }
    total >> 3
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDStationNames" ["probe"] => Names { base, central, airport, oilfield, docks, heliport, mines, lakeside, woods, forest, valley, heights, north, south, east, west, transfer, halt, exchange, annexe, sidings, branch, upper, lower, fallback, undefined, null };
    "OpenTTDStationLifecycleServices" ["probe"] => Services { shared, catchment, commands, visits, aircraft, storage, allocation, check, climate, names, town, industry_type, set_industry_type, spec, spec_name, spec_enabled, spec_life, set_random_bits, freeform, cargo_classes, forest, stop_data, stop_count, stop_tile, stop_random, stop_frame, stop_set_random, stop_set_frame, stop_append, stop_swap_pop, by_tile, airport_animation, rail_animation, road_animation, cleaning_pool, cargo, flows, delete_airport_storage, invalidate_list, invalidate_lists, close_view, remove_orders, delete_news, remove_index, sign_valid, remove_sign, next_vehicle, vehicle_id, last_loading, airport_maintenance, maintenance_price };
}
