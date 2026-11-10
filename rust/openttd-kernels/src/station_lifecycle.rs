/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Station naming, tile evolution and destruction. Native shells retain their pool
//! identities and unmoved fields. All calls run under the game-state mutex; raw
//! field accesses and owner borrows end before a service can reenter an owner.
#![allow(unsafe_code, clippy::cast_possible_truncation, clippy::too_many_lines)]
use crate::cargo_type::{CargoType, NUM_CARGO, is_valid_cargo_type};
use crate::cargotype::{CargoClass, CargoClasses};
use crate::company_type::CompanyID;
use crate::house::HouseZone;
use crate::industry_map::{get_industry_by_tile, get_industry_type};
use crate::industry_type::{IndustryType, NUM_INDUSTRYTYPES};
use crate::industrytype::{IndustryLifeType, IndustryLifeTypes};
use crate::landscape_type::LandscapeType;
use crate::map_func::TileTypeAccess;
use crate::road_map::{ROADSIDE_BARREN, ROADSIDE_GRASS, ROADSIDE_PAVED};
use crate::services::{Map, Services as Shared};
use crate::station::BaseStation;
use crate::station_cmd::StationNaming;
use crate::station_type::{
    AirportAnimationTrigger, INVALID_STATION, StationAnimationTrigger, StationFacilities,
    StationFacility, StationID, StationType,
};
use crate::strings_type::StringID;
use crate::tile_type::{MP_STATION, TileIndex, TileType};
use crate::vehicle::VehicleRef;
use crate::vehicle_type::VehicleID;

use crate::industrytype::{self, IndustrySpec};
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
    names: Names,
    pub industry_type: extern "C" fn(*mut BaseStation) -> u8,
    pub set_industry_type: extern "C" fn(*mut BaseStation, u8),
    pub roadstop: *const crate::newgrf_roadstop::Services,
    pub cleaning_pool: extern "C" fn() -> bool,
    pub close_view: extern "C" fn(StationID),
    pub delete_news: extern "C" fn(StationID),
    pub sign_valid: extern "C" fn(*mut BaseStation) -> bool,
    pub remove_sign: extern "C" fn(StationID),
    pub last_loading: extern "C" fn(VehicleRef) -> StationID,
    pub airport_maintenance: extern "C" fn(*const crate::station_cmd::AirportSpec) -> u16,
    pub maintenance_price: usize,
}
impl Services {
    fn commands(&self) -> &crate::station_cmd::CommandServicesTable {
        unsafe { &*(self.shared.station_commands)() }
    }
    fn vehicle_resolver(&self) -> &crate::newgrf_vehicle::Services {
        unsafe { &*((*self.commands().vehicle).newgrf)() }
    }
    fn check(&self) -> extern "C" fn(bool) {
        crate::movement::table(self.shared).native_assert
    }
    fn map_access(&self) -> TileTypeAccess {
        crate::map_func::World::new(self.shared).types
    }
    /// `GetIndustrySpec`.
    fn spec(kind: IndustryType) -> *const IndustrySpec {
        industrytype::industry(u16::from(kind.0))
    }
    /// `_settings_game.game_creation.landscape`.
    fn landscape() -> LandscapeType {
        LandscapeType(unsafe { ((*crate::animated_tile::services().industries).landscape)() })
    }
}

/// `StationNameInformation`: information to handle station action 0 property 24
/// correctly, with default-name and industry-type sets independent.
struct NameInformation {
    /// Used default station suffixes. The original's bitset spans
    /// `STR_SV_STNAME_FALLBACK - STR_SV_STNAME`; the native names bound it below 256.
    used_names: [bool; 256],
    /// Bit set indicating when an industry type has been found.
    indtypes: [bool; NUM_INDUSTRYTYPES as usize],
}
impl NameInformation {
    fn is_available(&self, str: StringID, s: &Services) -> bool {
        (s.check())(str.0 >= s.names.base && str.0 < s.names.fallback);
        !self.used_names[(str.0 - s.names.base) as usize]
    }
    fn set_used(&mut self, str: StringID, s: &Services) {
        (s.check())(str.0 >= s.names.base && str.0 < s.names.fallback);
        self.used_names[(str.0 - s.names.base) as usize] = true;
    }
}
/// `CountMapSquareAround`: count the tiles matching `cmp` in the area around
/// the center tile.
fn count_map_square_around(
    tile: TileIndex,
    map: Map,
    size_y: u32,
    freeform: bool,
    mut cmp: impl FnMut(TileIndex) -> bool,
) -> i32 {
    let mut num = 0;
    for dx in -3..=3 {
        for dy in -3..=3 {
            let t = TileIndex(map.tile_add_wrap(tile.0, dx, dy, size_y, freeform));
            if t != TileIndex::INVALID && cmp(t) {
                num += 1;
            }
        }
    }
    num
}
/// `CMSAMine`: whether the tile is a mine.
fn cmsa_mine(tile: TileIndex, m: &TileTypeAccess, s: &Services) -> bool {
    /* No industry */
    if TileType(m.read(tile.0)) != TileType::MP_INDUSTRY {
        return false;
    }
    let shell = get_industry_by_tile(m, tile, s.catchment.industry_by_id, s.check());
    let owner = (s.catchment.industry_owner)(shell);
    let kind =
        IndustryType(unsafe { (*crate::industry::openttd_rust_industry_metadata(owner)).kind });
    /* No extractive industry */
    if !IndustryLifeTypes(unsafe { (*Services::spec(kind)).life_type })
        .test(IndustryLifeType::Extractive)
    {
        return false;
    }
    let mut result = false;
    crate::industry::for_each_produced_cargo(owner, |cargo| {
        /* The industry extracts something non-liquid, i.e. no oil or plastic, so it is a mine.
         * Also the production of passengers and mail is ignored. */
        if !result
            && is_valid_cargo_type(cargo)
            && !CargoClasses(unsafe { (s.vehicle_resolver().cargo_classes)(cargo) }).any(
                CargoClasses::from_element(CargoClass::Liquid)
                    .with(CargoClasses::from_element(CargoClass::Passengers))
                    .with(CargoClasses::from_element(CargoClass::Mail)),
            )
        {
            result = true;
        }
    });
    result
}
/// `CMSAWater`: whether the tile is a water tile.
fn cmsa_water(tile: TileIndex, m: &TileTypeAccess) -> bool {
    TileType(m.read(tile.0)) == TileType::MP_WATER && crate::water_map::is_water(m, tile.0).unwrap()
}
/// `CMSATree`: whether the tile is a tree tile.
fn cmsa_tree(tile: TileIndex, m: &TileTypeAccess) -> bool {
    TileType(m.read(tile.0)) == TileType::MP_TREES
}
/// `GenerateStationName`'s scan of the other stations of the same town: their
/// industry types and used default names.
fn station_name_information(
    st: *mut BaseStation,
    town: *const crate::services::TownHandle,
    s: &Services,
) -> NameInformation {
    let n = &s.names;
    let mut sni = NameInformation {
        used_names: [false; 256],
        indtypes: [false; NUM_INDUSTRYTYPES as usize],
    };
    let mut from = 0;
    loop {
        let other = (s.catchment.next_station)(from);
        if other.is_null() {
            break;
        }
        from = u32::from((s.catchment.id)(other).0) + 1;
        if other == st || (s.commands().station_town)(other) != town {
            continue;
        }
        let indtype = IndustryType((s.industry_type)(other));
        if indtype != IndustryType::IT_INVALID {
            sni.indtypes[usize::from(indtype.0)] = true;
            let name = unsafe { (*Services::spec(indtype)).station_name };
            if name != n.undefined {
                /* Filter for other industrytypes with the same name */
                for it in 0..NUM_INDUSTRYTYPES {
                    let indsp = Services::spec(IndustryType(it));
                    if unsafe { (*indsp).enabled && (*indsp).station_name == name } {
                        sni.indtypes[usize::from(it)] = true;
                    }
                }
            }
            continue;
        }
        let str = (s.commands().string_id)(other);
        if str >= n.base && str < n.fallback {
            sni.set_used(StringID(if str == n.forest { n.woods } else { str }), s);
        }
    }
    sni
}
/// `GenerateStationName`, including industry aliases and fallback exhaustion.
pub(crate) fn generate_station_name(
    st: *mut BaseStation,
    tile: TileIndex,
    name_class: StationNaming,
    s: &Services,
) -> StringID {
    let m = s.map_access();
    let map = s.shared.map();
    let size_y = (s.shared.map_max_y)() + 1;
    let freeform = (unsafe { &*s.catchment.station }.environment.freeform_edges)();
    let n = &s.names;
    let town = (s.commands().station_town)(st);
    let mut sni = station_name_information(st, town, s);
    let mut spiral = crate::spiral::square(
        map.tile_x(tile.0),
        map.tile_y(tile.0),
        7,
        map.size_x(),
        size_y,
    );
    while !spiral.is_end() {
        let indtile = TileIndex(map.tile_xy(spiral.x, spiral.y));
        spiral = spiral.advance(map.size_x(), size_y);
        if TileType(m.read(indtile.0)) != TileType::MP_INDUSTRY {
            continue;
        }
        /* If the station name is undefined it means that it doesn't name a station */
        let indtype = get_industry_type(
            &m,
            indtile,
            s.catchment.industry_by_id,
            s.catchment.industry_owner,
            s.check(),
        );
        let indsp = Services::spec(indtype);
        if unsafe { (*indsp).station_name } == n.undefined {
            continue;
        }
        /* In all cases if an industry that provides a name is found two of
         * the standard names will be disabled. */
        sni.set_used(StringID(n.oilfield), s);
        sni.set_used(StringID(n.mines), s);
        if sni.indtypes[usize::from(indtype.0)] {
            continue;
        }
        /* STR_NULL means it only disables oil rig/mines */
        if unsafe { (*indsp).station_name } != n.null {
            (s.set_industry_type)(st, indtype.0);
            return StringID(n.fallback);
        }
        break;
    }
    /* check default names
     * Oil rigs/mines name could be marked not free by looking for a near by industry. */
    let class_name = match name_class {
        StationNaming::STATIONNAMING_AIRPORT => Some(n.airport),
        StationNaming::STATIONNAMING_OILRIG => Some(n.oilfield),
        StationNaming::STATIONNAMING_DOCK => Some(n.docks),
        StationNaming::STATIONNAMING_HELIPORT => Some(n.heliport),
        _ => None,
    };
    if let Some(name) = class_name
        && sni.is_available(StringID(name), s)
    {
        return StringID(name);
    }
    /* check mine? */
    if sni.is_available(StringID(n.mines), s)
        && count_map_square_around(tile, map, size_y, freeform, |at| cmsa_mine(at, &m, s)) >= 2
    {
        return StringID(n.mines);
    }
    /* check close enough to town to get central as name? */
    let town_xy = unsafe { crate::town::position((s.shared.town_owner)(town)) };
    if map.distance_max(tile.0, town_xy.0) < 8 {
        if sni.is_available(StringID(n.base), s) {
            return StringID(n.base);
        }
        if sni.is_available(StringID(n.central), s) {
            return StringID(n.central);
        }
    }
    /* Check lakeside */
    if sni.is_available(StringID(n.lakeside), s)
        && map.distance_from_edge(tile.0, size_y) < 20
        && count_map_square_around(tile, map, size_y, freeform, |at| cmsa_water(at, &m)) >= 5
    {
        return StringID(n.lakeside);
    }
    /* Check woods */
    if sni.is_available(StringID(n.woods), s)
        && (count_map_square_around(tile, map, size_y, freeform, |at| cmsa_tree(at, &m)) >= 8
            || count_map_square_around(tile, map, size_y, freeform, |at| {
                crate::industry::forest(at.0, unsafe {
                    &*crate::animated_tile::services().industries
                })
            }) >= 2)
    {
        return StringID(if Services::landscape() == LandscapeType::Tropic {
            n.forest
        } else {
            n.woods
        });
    }
    /* check elevation compared to town */
    let z = crate::tile_map::get_tile_z(&m, tile.0, map.size_x(), size_y).unwrap();
    let z2 = crate::tile_map::get_tile_z(&m, town_xy.0, map.size_x(), size_y).unwrap();
    if z < z2 && sni.is_available(StringID(n.valley), s) {
        return StringID(n.valley);
    }
    if z > z2 && sni.is_available(StringID(n.heights), s) {
        return StringID(n.heights);
    }
    /* check direction compared to town */
    for name in if map.tile_x(tile.0) < map.tile_x(town_xy.0) {
        [n.south, n.west]
    } else {
        [n.north, n.east]
    } {
        sni.set_used(StringID(name), s);
    }
    for name in if map.tile_y(tile.0) < map.tile_y(town_xy.0) {
        [n.south, n.east]
    } else {
        [n.north, n.west]
    } {
        sni.set_used(StringID(name), s);
    }
    /* Remaining station names that can be used when a more specific name has not been used. */
    let fallback_names = [
        n.north, n.south, n.east, n.west, n.transfer, n.halt, n.exchange, n.annexe, n.sidings,
        n.branch, n.upper, n.lower,
    ];
    for str in fallback_names {
        if sni.is_available(StringID(str), s) {
            return StringID(str);
        }
    }
    StringID(n.fallback)
}
/// `Station::AddFacility`: called when a new facility is built on the station. If
/// it is the first facility it also initializes the `xy` and `random_bits` members.
pub(crate) fn add_facility(
    st: *mut BaseStation,
    new_facility_bit: StationFacility,
    facil_xy: TileIndex,
    s: &Services,
) {
    let c = s.commands();
    let world = unsafe { &*c.station };
    if StationFacilities((world.facilities)(st)) == StationFacilities::EMPTY {
        crate::station::spatial::station_move_sign(s.shared, st, facil_xy);
        unsafe {
            ((*s.roadstop).set_random_bits)(st, (s.shared.random)(std::ptr::null_mut()) as u16);
        }
    }
    (c.set_facility_bits)(st, StationFacilities::from_element(new_facility_bit).0);
    (c.set_station_owner)(st, (s.shared.current_company)());
    (c.set_build_date)(st, crate::timer_game_common::calendar_date().0);
    (c.orders_dirty)();
}

/// Raw adapter for `tile_loop.rs`, outside this pass; delegates to `tile_loop_station`.
/// SAFETY: As `tile_loop_station`.
pub(crate) unsafe fn tile_loop(
    tile: u32,
    call: *mut crate::ground::Call,
    s: &Services,
    m: &TileTypeAccess,
    size_x: u32,
    size_y: u32,
) {
    unsafe { tile_loop_station(TileIndex(tile), call, s, m, size_x, size_y) }
}
/// `TileLoop_Station`. The scheduler's map descriptor remains valid through the
/// original tile callbacks under the game-loop interval in world-state.md.
pub(crate) unsafe fn tile_loop_station(
    tile: TileIndex,
    call: *mut crate::ground::Call,
    s: &Services,
    m: &TileTypeAccess,
    size_x: u32,
    size_y: u32,
) {
    let map = crate::map_func::Map::from_log_x(size_x.trailing_zeros());
    let st = unsafe { ((*s.roadstop).by_tile)(tile.0) };
    let trigger_road_stop_animation = || {
        crate::animated_tile::openttd_rust_trigger_roadstop(
            st,
            tile.0,
            StationAnimationTrigger::TileLoop.0,
            CargoType::INVALID_CARGO.0,
        );
    };
    match crate::station_map::get_station_type(m, tile)
        .unwrap_or_else(|| (s.shared.station_type)(tile.0))
    {
        StationType::Airport => {
            crate::animated_tile::openttd_rust_trigger_airport_tile_abi(
                (crate::animated_tile::services().as_station)(st),
                tile.0,
                AirportAnimationTrigger::TileLoop.0,
            );
        }
        StationType::Rail | StationType::RailWaypoint => {
            crate::animated_tile::openttd_rust_trigger_station(
                st,
                tile.0,
                StationAnimationTrigger::TileLoop.0,
                CargoType::INVALID_CARGO.0,
            );
        }
        StationType::Dock | StationType::Oilrig | StationType::Buoy => {
            // StationType::Dock: only handle the water part; then fall through.
            if crate::station_map::get_station_type(m, tile)
                .is_some_and(|kind| kind == StationType::Dock)
                && !crate::tile_map::is_tile_flat(m, tile.0, size_x, size_y).unwrap()
            {
                return;
            }
            // StationType::Oilrig (station part), StationType::Buoy.
            unsafe {
                crate::water::water_tile_loop(
                    tile.0,
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
        StationType::Truck | StationType::Bus => trigger_road_stop_animation(),
        StationType::RoadWaypoint => {
            let on_snow_or_desert =
                || crate::station_map::is_road_waypoint_on_snow_or_desert(m, tile).unwrap();
            let change = match Services::landscape() {
                LandscapeType::Arctic => {
                    on_snow_or_desert()
                        != (crate::tile_map::get_tile_z(m, tile.0, size_x, size_y).unwrap()
                            > i32::from(crate::snow_line::get(s.shared)))
                }
                LandscapeType::Tropic => {
                    crate::tile_map::get_tropic_zone(m, tile.0).unwrap()
                        == crate::tile_type::TROPICZONE_DESERT
                        && !on_snow_or_desert()
                }
                _ => false,
            };
            if change {
                crate::station_map::toggle_road_waypoint_on_snow_or_desert(m, tile).unwrap();
                (s.shared.mark_dirty)(tile.0);
            }
            let t = crate::town::spatial::closest(s.shared, tile.0, u32::MAX);
            let new_zone = if t.is_null() {
                HouseZone::TownEdge
            } else {
                unsafe { crate::town::get_town_radius_group((s.shared.town_owner)(t), tile, map) }
            };
            /* Adjust road ground type depending on 'new_zone' */
            let new_rs = if new_zone == HouseZone::TownEdge {
                ROADSIDE_GRASS
            } else {
                ROADSIDE_PAVED
            };
            let cur_rs = crate::station_map::get_road_waypoint_roadside(m, tile).unwrap();
            if new_rs != cur_rs {
                crate::station_map::set_road_waypoint_roadside(
                    m,
                    tile,
                    if cur_rs == ROADSIDE_BARREN {
                        new_rs
                    } else {
                        ROADSIDE_BARREN
                    },
                )
                .unwrap();
                (s.shared.mark_dirty)(tile.0);
            }
            trigger_road_stop_animation();
        }
        _ => {}
    }
}
/// `Station::~Station`; the native destructor retains only member destruction.
/// Aircraft-hangar orders need special treatment, as the hangars are part of a
/// station (tile type `MP_STATION`) but the order type is `OT_GOTO_DEPOT`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_destruct(s: &Services, st: *mut BaseStation) {
    let c = s.commands();
    let w = unsafe { &*c.station };
    let links = unsafe { &*c.links };
    let state = (w.owner)(st);
    if (s.cleaning_pool)() {
        for cargo in 0..NUM_CARGO {
            let list = links.station_cargo(st, cargo);
            if !list.is_null() {
                unsafe {
                    crate::cargo_storage::openttd_rust_cargo_list_clear(list.cast_mut());
                }
            }
        }
        return;
    }
    let visits = unsafe { &*s.catchment.visits };
    // Every loading vehicle leaves the station.
    loop {
        let front = unsafe { crate::station_service::openttd_rust_station_queue_first(state) };
        if front.is_null() {
            break;
        }
        let vehicle = unsafe {
            VehicleRef::from_shell(
                crate::station_service::openttd_rust_station_queue_value(front),
                visits.vehicles(),
            )
        };
        crate::vehicle::visits::leave_station(vehicle, visits);
    }
    let id = (s.catchment.id)(st);
    // Normal aircraft targeting this airport lose their target.
    unsafe {
        crate::aircraft::invalidate_target(c.aircraft, id.0);
    }
    let graphs = w.graphs;
    let storage = links.storage();
    for cargo in 0..NUM_CARGO {
        let graph = graphs.get(crate::linkgraph_type::LinkGraphID((graphs.graph_id)(
            st, cargo,
        )));
        if graph.is_null() {
            continue;
        }
        let owner = graph;
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
    let resolver = s.vehicle_resolver();
    let mut from = VehicleID(0);
    while let Some(v) = (s.shared.vehicle_next)(from.0) {
        from = VehicleID(unsafe { (resolver.id)(v.0.as_ptr()) }.wrapping_add(1));
        /* Forget about this station if this station is removed */
        if v.last_station() == id.0 {
            v.write_last_station(INVALID_STATION);
        }
        if (s.last_loading)(v) == id {
            (visits.set_last_loading)(v, INVALID_STATION);
        }
    }
    /* Remove station from industries and towns that reference it. */
    unsafe {
        crate::station::openttd_rust_station_remove_from_nearby_lists(st, s.catchment);
    }
    /* Clear the persistent storage. */
    (c.delete_airport_storage)(st);
    if CompanyID((w.station_owner)(st)) == CompanyID::NONE {
        /* Invalidate all in case of oil rigs. */
        (c.station_list_classes_invalidate)();
    } else {
        (w.station_list_invalidation)(st, 0);
    }
    (s.close_view)(id);
    /* Now delete all orders that go to the station */
    unsafe {
        crate::orders::openttd_rust_remove_destination(
            crate::order_type::OT_GOTO_STATION,
            id.0,
            0,
            &*(vehicles.orders)(),
        );
    }
    /* Remove all news items */
    (s.delete_news)(id);
    for cargo in 0..NUM_CARGO {
        let list = links.station_cargo(st, cargo);
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
    // `CargoPacket::InvalidateAllFrom(StationID)`: the `u8::MAX` source kind selects
    // the station form of the shared invalidation entry.
    unsafe {
        crate::cargo_storage::openttd_rust_cargo_invalidate(storage, id.0, u8::MAX);
    }
    crate::station::spatial::remove(s.shared, id);
    if (s.sign_valid)(st) {
        (s.remove_sign)(id);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_airport_maintenance(s: &Services, owner: u8) -> i64 {
    airport_maintenance_cost(s, CompanyID(owner))
}
/// Raw adapter for `company.rs`, outside this pass; delegates to `airport_maintenance_cost`.
pub(crate) fn airport_maintenance(s: &Services, owner: u8) -> i64 {
    airport_maintenance_cost(s, CompanyID(owner))
}
/// `AirportMaintenanceCost`: the maintenance cost of all airports of a company.
pub(crate) fn airport_maintenance_cost(s: &Services, owner: CompanyID) -> i64 {
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
        if CompanyID((w.station_owner)(st)) == owner
            && StationFacilities((w.facilities)(st)).test(StationFacility::Airport)
        {
            let spec = crate::newgrf_airport::station_spec(st, crate::newgrf_airport::runtime());
            total = total.saturating_add(
                crate::company::price(s.maintenance_price)
                    .saturating_mul(i64::from((s.airport_maintenance)(spec))),
            );
        }
    }
    /* 3 bits fraction for the maintenance cost factor. */
    total >> 3
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDStationNames" ["probe"] => Names { base, central, airport, oilfield, docks, heliport, mines, lakeside, woods, forest, valley, heights, north, south, east, west, transfer, halt, exchange, annexe, sidings, branch, upper, lower, fallback, undefined, null };
    "OpenTTDStationLifecycleServices" ["probe"] => Services { shared, catchment, names, industry_type, set_industry_type, roadstop, cleaning_pool, close_view, delete_news, sign_valid, remove_sign, last_loading, airport_maintenance, maintenance_price };
}

/// Test-only table for `crate::test_tables`; its private fields keep it in this module.
#[cfg(test)]
pub(crate) const TEST_TABLE: crate::station_lifecycle::Services =
    crate::station_lifecycle::Services {
        shared: &crate::test_tables::SHARED,
        catchment: &crate::test_tables::CATCHMENT,
        names: crate::station_lifecycle::Names {
            base: 0,
            central: 0,
            airport: 0,
            oilfield: 0,
            docks: 0,
            heliport: 0,
            mines: 0,
            lakeside: 0,
            woods: 0,
            forest: 0,
            valley: 0,
            heights: 0,
            north: 0,
            south: 0,
            east: 0,
            west: 0,
            transfer: 0,
            halt: 0,
            exchange: 0,
            annexe: 0,
            sidings: 0,
            branch: 0,
            upper: 0,
            lower: 0,
            fallback: 0,
            undefined: 0,
            null: 0,
        },
        industry_type: crate::test_tables::unreachable1,
        set_industry_type: crate::test_tables::unreachable2,
        roadstop: std::ptr::null(),
        cleaning_pool: crate::test_tables::unreachable0,
        close_view: crate::test_tables::unreachable1,
        delete_news: crate::test_tables::unreachable1,
        sign_valid: crate::test_tables::unreachable1,
        remove_sign: crate::test_tables::unreachable1,
        last_loading: crate::test_tables::unreachable1,
        airport_maintenance: crate::test_tables::unreachable1,
        maintenance_price: 0,
    };
