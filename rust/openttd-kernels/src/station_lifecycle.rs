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
use crate::station_type::{
    AAT_TILE_LOOP, BUOY, FACILITY_AIRPORT, INVALID_STATION, RAIL_WAYPOINT, ROAD_WAYPOINT,
    SAT_TILE_LOOP, STATION_AIRPORT, STATION_BUS, STATION_DOCK, STATION_OILRIG, STATION_RAIL,
    STATION_TRUCK, StationID,
};
use crate::tile_type::TileIndex;
use crate::tile_type::{INVALID_TILE, MP_INDUSTRY, MP_STATION, MP_TREES, MP_WATER};
use crate::vehicle::VehicleRef;
use crate::vehicle_type::VehicleID;
use std::ptr::NonNull;

use crate::industry::SpecHandle as IndustrySpec;
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
    pub cargo_classes: extern "C" fn(u8) -> u16,
    pub forest: extern "C" fn(u32) -> bool,
    pub roadstop: *const crate::newgrf_roadstop::Services,
    pub as_station: extern "C" fn(*mut BaseStation) -> *mut crate::station::Station,
    pub cleaning_pool: extern "C" fn() -> bool,
    pub close_view: extern "C" fn(StationID),
    pub delete_news: extern "C" fn(StationID),
    pub remove_index: extern "C" fn(StationID),
    pub sign_valid: extern "C" fn(*mut BaseStation) -> bool,
    pub remove_sign: extern "C" fn(StationID),
    pub vehicle_id: extern "C" fn(VehicleRef) -> VehicleID,
    pub last_loading: extern "C" fn(VehicleRef) -> StationID,
    pub airport_maintenance: extern "C" fn(*const crate::station_cmd::AirportSpec) -> u16,
    pub maintenance_price: usize,
}
impl Services {
    fn commands(&self) -> &crate::station_cmd::CommandServicesTable {
        unsafe { &*(self.shared.station_commands)() }
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
fn mine(tile: u32, m: &TileTypeAccess, s: &Services) -> bool {
    if m.read(tile) != MP_INDUSTRY {
        return false;
    }
    let shell =
        crate::industry_map::get_industry_by_tile(m, tile, s.catchment.industry_by_id, s.check);
    let owner = (s.catchment.industry_owner)(shell);
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
    let freeform = (unsafe { &*s.catchment.station }.environment.freeform_edges)();
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
        let kind = crate::industry_map::get_industry_type(
            &m,
            at,
            s.catchment.industry_by_id,
            s.catchment.industry_owner,
            s.check,
        );
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
    if let Some(name) = special
        && info.available(name, s)
    {
        return name;
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

/// `TileLoop_Station`. The scheduler's map descriptor remains valid through the
/// original tile callbacks under the game-loop interval in world-state.md.
pub(crate) unsafe fn tile_loop(
    tile: u32,
    call: *mut crate::ground::Call,
    s: &Services,
    m: &TileTypeAccess,
    size_x: u32,
    size_y: u32,
) {
    let map = crate::map_func::Map::from_log_x(size_x.trailing_zeros());
    let st = unsafe { ((*s.roadstop).by_tile)(tile) };
    match crate::station_map::get_station_type(m, TileIndex(tile))
        .unwrap_or_else(|| (s.shared.station_type)(tile))
        .0
    {
        STATION_AIRPORT => {
            crate::animated_tile::openttd_rust_trigger_airport_tile(
                (s.as_station)(st),
                tile,
                AAT_TILE_LOOP,
            );
        }
        STATION_RAIL | RAIL_WAYPOINT => crate::animated_tile::openttd_rust_trigger_station(
            st,
            tile,
            SAT_TILE_LOOP,
            crate::cargo_type::INVALID_CARGO,
        ),
        STATION_DOCK | STATION_OILRIG | BUOY => {
            if crate::station_map::get_station_type(m, TileIndex(tile))
                .is_some_and(|kind| kind.0 == STATION_DOCK)
                && !crate::tile_map::is_tile_flat(m, tile, size_x, size_y)
                    .unwrap()
            {
                return;
            }
            unsafe {
                crate::water::water_tile_loop(
                    tile,
                    MP_STATION,
                    false,
                    size_x,
                    size_y,
                    call,
                    (s.shared.water_services)(),
                    s.shared,
                    m,
                );
            }
        }
        STATION_TRUCK | STATION_BUS => crate::animated_tile::openttd_rust_trigger_roadstop(
            st,
            tile,
            SAT_TILE_LOOP,
            crate::cargo_type::INVALID_CARGO,
        ),
        ROAD_WAYPOINT => {
            let snow = || crate::station_map::road_waypoint_snow(m, tile).unwrap();
            let change = match (s.climate)() {
                crate::landscape_type::ARCTIC => {
                    snow()
                        != (crate::tile_map::get_tile_z(m, tile, size_x, size_y).unwrap()
                            > i32::from((s.shared.snow_line)()))
                }
                crate::landscape_type::TROPIC => {
                    crate::tile_map::get_tropic_zone(m, tile).unwrap()
                        == crate::tile_type::TROPICZONE_DESERT
                        && !snow()
                }
                _ => false,
            };
            if change {
                crate::station_map::toggle_road_waypoint_snow(m, tile).unwrap();
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
            let current = crate::station_map::road_waypoint_roadside(m, tile).unwrap();
            if current != new {
                crate::station_map::set_road_waypoint_roadside(
                    m,
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
            crate::animated_tile::openttd_rust_trigger_roadstop(
                st,
                tile,
                SAT_TILE_LOOP,
                crate::cargo_type::INVALID_CARGO,
            );
        }
        _ => {}
    }
}
/// `Station::~Station`; the native destructor retains only member destruction.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_destruct(s: &Services, st: *mut BaseStation) {
    let c = s.commands();
    let w = unsafe { &*c.station };
    let links = unsafe { &*c.links };
    let state = (w.owner)(st);
    if (s.cleaning_pool)() {
        for cargo in 0..NUM_CARGO {
            let list = (links.station_cargo)(st, cargo);
            if !list.is_null() {
                unsafe {
                    crate::cargo_storage::openttd_rust_cargo_list_clear(list.cast_mut());
                }
            }
        }
        return;
    }
    let visits = unsafe { &*s.catchment.visits };
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
        crate::aircraft::invalidate_target(c.aircraft, id.0);
    }
    let graphs = w.graphs;
    let storage = unsafe { &*(links.storage)() };
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
            let goods = (links.goods_entry)(other, cargo);
            let flows = (storage.flow_map)(goods.cast());
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
    let mut from = VehicleID(0);
    while let Some(v) = (s.shared.vehicle_next)(from.0) {
        from = VehicleID((s.vehicle_id)(v).0.wrapping_add(1));
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
    (c.delete_airport_storage)(st);
    if (w.station_owner)(st) == crate::company_type::OWNER_NONE {
        (c.station_list_classes_invalidate)();
    } else {
        (w.station_list_invalidation)(st, 0);
    }
    (s.close_view)(id);
    unsafe {
        crate::orders::openttd_rust_remove_destination(
            crate::order_type::OT_GOTO_STATION,
            id.0,
            0,
            &*(vehicles.orders)(),
        );
    }
    (s.delete_news)(id);
    for cargo in 0..NUM_CARGO {
        let list = (links.station_cargo)(st, cargo);
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
    "OpenTTDStationLifecycleServices" ["probe"] => Services { shared, catchment, allocation, check, climate, names, town, industry_type, set_industry_type, spec, spec_name, spec_enabled, spec_life, set_random_bits, cargo_classes, forest, roadstop, as_station, cleaning_pool, close_view, delete_news, remove_index, sign_valid, remove_sign, vehicle_id, last_loading, airport_maintenance, maintenance_price };
}
