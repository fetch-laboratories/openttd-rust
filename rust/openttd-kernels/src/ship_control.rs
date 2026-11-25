/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `ship_cmd.cpp`: handling of ships, with the `Ship` members of `ship.h` whose state Rust
//! owns: `state`, `rotation`, the rotation coordinates and the cached `path`.
//!
//! Each entry receives a live ship record, whose `ship` link names the ship owner its shell
//! holds. Both stay live and serialized by the game-state mutex for the synchronous call.
//! Every field is read and written in place at the original's observation point, and no
//! reference into the record or the owner lives across a service.
#![allow(unsafe_code, clippy::verbose_bit_mask, clippy::if_not_else)]

use crate::cargo_type::is_valid_cargo_type;
use crate::command_type::DoCommandFlag;
use crate::direction_func::{
    change_dir, diag_dir_to_dir, dir_difference, dir_to_diag_dir, is_valid_diag_direction,
    reverse_dir,
};
use crate::direction_type::{DiagDirection, DirDiff, Direction};
use crate::economy_type::ExpensesType;
use crate::game_state::GameState;
use crate::map_func::{Map, World};
use crate::movement::CachedTable;
use crate::newgrf_properties::PropertyID;
use crate::newgrf_vehicle::get_vehicle_property;
use crate::news_func::add_vehicle_news_item;
use crate::news_type::NewsType;
use crate::order_type::{
    DestinationID, OrderDepotActionFlags, OrderDepotTypeFlag, OrderDepotTypeFlags,
    OrderNonStopFlag, OrderNonStopFlags, OrderType,
};
use crate::raw_field::{read_field, write_field};
use crate::script_event_types::ScriptEvent;
use crate::services::Services;
use crate::ship_yapf::ShipPathCache;
use crate::slope_func::get_inclined_slope_direction;
use crate::station_type::{StationFacility, StationHadVehicleOfType, StationID};
use crate::strings_func::get_encoded_string;
use crate::strings_type::StringParameterData::Integer;
use crate::table::strings::STR_NEWS_FIRST_SHIP_ARRIVAL;
use crate::tile_cmd::VehicleEnterTileState;
use crate::tile_type::{TILE_HEIGHT, TILE_SIZE, TileIndex, TileType};
use crate::track_func::{
    axis_to_track_bits, diag_dir_to_diag_trackdir, diagdir_reaches_tracks, find_first_track,
    is_diagonal_track, track_bits_to_track, track_direction_to_trackdir,
    track_status_to_track_bits, track_to_opposite_track, track_to_track_bits,
    trackdir_bits_to_track_bits, trackdir_to_exitdir, trackdir_to_track, trackdir_to_trackdir_bits,
    vehicle_exit_dir,
};
use crate::track_type::{Track, TrackBits, Trackdir};
use crate::transport_type::TransportType;
use crate::vehicle::{VehicleRef, VehicleServices, maintenance};
use crate::vehicle_base::{ClosestDepot, VehState, VehStates};
use crate::vehicle_type::VehicleType;
use crate::water_map::WaterClass;
use crate::water_regions::{
    WATER_REGION_EDGE_LENGTH, WaterRegionPatchDesc, WaterVisit, calculate_water_region_patch_hash,
    get_water_region_patch_info, visit_water_region_patch_neighbours,
};
use crate::window_func::{invalidate_window_data, set_window_classes_dirty, set_window_dirty};
use crate::window_type::{WindowClass, WindowNumber};
use std::collections::{HashSet, VecDeque};
use std::ptr::NonNull;

/// Max distance in tiles (as the crow flies) to search for depots when user clicks "go to depot".
const MAX_SHIP_DEPOT_SEARCH_DISTANCE: u32 = 80;

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_ship_control_reload_rotation() {
    crate::witness::hit(crate::witness::SHIP_ROTATION_RELOAD);
}

/// The Rust owner of one ship shell: the `Ship` members Rust reads and writes.
pub struct ShipState {
    /// The "track" the ship is following.
    state: TrackBits,
    /// Visible direction.
    rotation: Direction,
    /// NOSAVE: X Position before rotation.
    rotation_x_pos: i16,
    /// NOSAVE: Y Position before rotation.
    rotation_y_pos: i16,
    /// Cached path.
    path: ShipPathCache,
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_ship_state_new() -> *mut ShipState {
    Box::into_raw(Box::new(ShipState {
        state: TrackBits::TRACK_BIT_NONE,
        rotation: Direction::INVALID_DIR,
        rotation_x_pos: 0,
        rotation_y_pos: 0,
        path: ShipPathCache::default(),
    }))
}
/// # Safety
/// One shell transfers its live owner exactly once, after `PreDestructor`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_state_destroy(state: *mut ShipState) {
    // SAFETY: As this function's contract.
    unsafe {
        drop(Box::from_raw(state));
    }
}
/// The owner's cached path, which the C++ `ShipPathCache` view reaches for save/load.
///
/// # Safety
/// A live owner; the path lives as long as the owner.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_state_path(state: *mut ShipState) -> *mut ShipPathCache {
    // SAFETY: As this function's contract; no reference is formed.
    unsafe { std::ptr::addr_of_mut!((*state).path) }
}
// The scalar accessors of the shell and save/load. SAFETY for each: a live owner, accessed
// on the game thread with no Rust reference into it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_get_state(state: *const ShipState) -> u8 {
    read_field!(state, state).0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_set_state(state: *mut ShipState, value: u8) {
    write_field!(state, state, TrackBits(value));
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_get_rotation(state: *const ShipState) -> u8 {
    read_field!(state, rotation).0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_set_rotation(state: *mut ShipState, value: u8) {
    write_field!(state, rotation, Direction(value));
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_get_rotation_x(state: *const ShipState) -> i16 {
    read_field!(state, rotation_x_pos)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_set_rotation_x(state: *mut ShipState, value: i16) {
    write_field!(state, rotation_x_pos, value);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_get_rotation_y(state: *const ShipState) -> i16 {
    read_field!(state, rotation_y_pos)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_set_rotation_y(state: *mut ShipState, value: i16) {
    write_field!(state, rotation_y_pos, value);
}

/// A ship: its live record and the live ship owner its record names.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Ship {
    pub(crate) v: VehicleRef,
    s: NonNull<ShipState>,
}
impl Ship {
    /// The ship whose record is `v`, a live ship's, as a `Ship` member's `this` is.
    pub(crate) fn of(v: VehicleRef) -> Self {
        // SAFETY: Every ship shell's constructor links its non-null owner.
        Self {
            v,
            s: unsafe { NonNull::new_unchecked(v.ship()) },
        }
    }
    /// `Ship::From(v)`: the ship `v` is, with `Ship::From`'s type assertion.
    pub(crate) fn from_vehicle(v: VehicleRef) -> Self {
        crate::services_assert::assert_with(|| v.vehicle_type() == VehicleType::VEH_SHIP);
        Self::of(v)
    }
    /// `Ship::state`.
    pub(crate) fn state(self) -> TrackBits {
        read_field!(self.s.as_ptr(), state)
    }
    /// Store `Ship::state`.
    pub(crate) fn set_state(self, state: TrackBits) {
        write_field!(self.s.as_ptr(), state, state);
    }
    /// `Ship::rotation`.
    fn rotation(self) -> Direction {
        read_field!(self.s.as_ptr(), rotation)
    }
    fn set_rotation(self, rotation: Direction) {
        write_field!(self.s.as_ptr(), rotation, rotation);
    }
    fn set_rotation_x_pos(self, rotation_x_pos: i16) {
        write_field!(self.s.as_ptr(), rotation_x_pos, rotation_x_pos);
    }
    fn set_rotation_y_pos(self, rotation_y_pos: i16) {
        write_field!(self.s.as_ptr(), rotation_y_pos, rotation_y_pos);
    }
    /// `Ship::path`, for a search that holds the only reference to it for its call.
    fn path(self) -> *mut ShipPathCache {
        // SAFETY: A live owner; no reference is formed.
        unsafe { std::ptr::addr_of_mut!((*self.s.as_ptr()).path) }
    }
    /// `path.empty()`.
    fn path_empty(self) -> bool {
        // SAFETY: A live owner; the borrow ends here.
        unsafe { (*self.path()).empty() }
    }
    /// `path.back()`, under the original's nonempty precondition.
    fn path_back(self) -> Trackdir {
        // SAFETY: A live owner; the copy ends the borrow.
        unsafe { (*self.path()).back() }
    }
    /// `path.pop_back()`.
    fn path_pop_back(self) {
        // SAFETY: A live owner on the game thread.
        unsafe { (*self.path()).pop_back() }
    }
    /// `path.clear()`.
    fn path_clear(self) {
        // SAFETY: A live owner on the game thread.
        unsafe { (*self.path()).clear() }
    }
    /// `Ship::IsInDepot`, which is also a ship's `Vehicle::IsChainInDepot`.
    pub(crate) fn is_in_depot(self) -> bool {
        self.state() == TrackBits::TRACK_BIT_DEPOT
    }
}

/// `Depot`, as one step of `Depot::Iterate` returns it.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Depot {
    /// `Depot::index`.
    pub index: u16,
    /// `Depot::xy`.
    pub xy: u32,
}

/// The services a ship entry calls that C++ still owns.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Leaves {
    /// `SpecializedVehicle<Ship>::UpdateViewport(force_update, update_delta)`, which calls the
    /// ship's drawing members.
    pub update_viewport: extern "C" fn(VehicleRef, bool, bool),
    /// One step of `Depot::Iterate(first)`: writes the first depot at or after index `first`
    /// to `out`, or returns false when there is none.
    pub next_depot: extern "C" fn(u32, *mut Depot) -> bool,
    /// Base vehicle table, immutable for the process.
    pub vehicle: *const VehicleServices,
}

/// The tables one ship entry runs in, and its map descriptor.
pub(crate) struct Tables<'a> {
    l: &'a Leaves,
    vs: &'a VehicleServices,
    ss: &'a Services,
    movement: CachedTable<'a>,
}
impl<'a> Tables<'a> {
    /// The tables of an entry. Orders, loading, sound, `NewGRF`, movement, window, news,
    /// script, depot and viewport callbacks cannot load or reset the map or yield the
    /// game-state mutex, so the descriptor holds for the entry; nested entries build their
    /// own.
    pub(crate) fn new(l: &'a Leaves, ss: &'a Services) -> Self {
        Self {
            l,
            // SAFETY: The native table names the immutable process-wide base table.
            vs: unsafe { &*l.vehicle },
            ss,
            // SAFETY: As this function's contract.
            movement: unsafe { CachedTable::audited(ss) },
        }
    }
    /// The entry's map descriptor, loaded at its first use.
    fn world(&self) -> &World<'a> {
        self.movement.world()
    }
    fn map(&self) -> Map {
        self.world().dimensions
    }
    /// The native assertion handler, as a header helper's precondition check.
    fn check(&self) -> impl Fn(bool) + Copy + '_ {
        |ok| crate::movement::native_assert(self.ss, ok)
    }
    /// `v->UpdateViewport(true, true)`.
    fn update_viewport(&self, v: Ship) {
        (self.l.update_viewport)(v.v, true, true);
    }
    /// `v->UpdatePosition()`.
    fn update_position(&self, v: VehicleRef) {
        // SAFETY: The live shell survives the synchronous index update.
        unsafe { crate::vehicle::occupancy::update_position(self.vs, v, self.map()) }
    }
    /// `VehicleEnterTile(v, tile, x, y)`.
    fn vehicle_enter_tile(
        &self,
        v: VehicleRef,
        tile: TileIndex,
        x: i32,
        y: i32,
    ) -> crate::tile_cmd::VehicleEnterTileStates {
        // SAFETY: The same live serialized shell as the original callback; no owner borrow
        // spans it.
        unsafe { self.movement.get().vehicle_enter_tile(v, tile, x, y) }
    }
    /// `TrackStatusToTrackBits(GetTileTrackStatus(tile, TRANSPORT_WATER, 0, side))`.
    fn water_track_bits(&self, tile: TileIndex, side: DiagDirection) -> TrackBits {
        track_status_to_track_bits(self.movement.get().get_tile_track_status_in_world(
            self.world(),
            tile,
            TransportType::TRANSPORT_WATER,
            0,
            side,
        ))
    }
    /// `SetWindowWidgetDirty(WC_VEHICLE_VIEW, v->index, WID_VV_START_STOP)`.
    fn start_stop_dirty(&self, v: VehicleRef) {
        (self.vs.start_stop_dirty)(v);
    }
    /// `v->index`.
    fn index(&self, v: VehicleRef) -> crate::vehicle_type::VehicleID {
        v.index(self.vs.newgrf())
    }
    /// The company table, through base maintenance.
    fn company(&self) -> &crate::company::Services {
        // SAFETY: Base maintenance names the shared static command and company tables.
        unsafe { &*((*(*(*self.vs.maintenance).transaction).commands).company)() }
    }
}

/// `GetEffectiveWaterClass`: determine the effective `WaterClass` for a ship travelling on
/// a tile.
#[inline]
pub(crate) fn get_effective_water_class(world: &World<'_>, tile: TileIndex) -> WaterClass {
    if world.asserted(|m| crate::water_map::has_tile_water_class(m, tile.0)) {
        return WaterClass(world.water_class(tile.0));
    }
    if world.kind(tile) == TileType::MP_TUNNELBRIDGE {
        world.assert_with(|| {
            TransportType(world.tunnel_bridge_transport(tile.0)) == TransportType::TRANSPORT_WATER
        });
        return WaterClass::Canal;
    }
    if world.kind(tile) == TileType::MP_RAILWAY {
        world.assert_with(|| {
            crate::rail_map::RailGroundType(world.get_rail_ground_type_raw(tile.0))
                == crate::rail_map::RailGroundType::HalfTileWater
        });
        return WaterClass::Sea;
    }
    crate::services_assert::not_reached()
}

/// `visited_patch_hashes` and `patches_to_search`, the original's two function statics of
/// `FindClosestShipDepot`.
#[derive(Default)]
struct DepotSearch {
    visited_patch_hashes: HashSet<i32>,
    patches_to_search: VecDeque<WaterRegionPatchDesc>,
}
static DEPOT_SEARCH: GameState<Option<DepotSearch>> = GameState::new(None);
impl DepotSearch {
    /// The statics, out of their slot for one search. The caller holds the game-state mutex.
    fn take() -> Self {
        // SAFETY: Native entries serialize access; the slot borrow ends here.
        unsafe { (*DEPOT_SEARCH.get()).take().unwrap_or_default() }
    }
    /// Return the statics to their slot, keeping their capacity.
    fn put_back(self) {
        // SAFETY: As `take`.
        unsafe { *DEPOT_SEARCH.get() = Some(self) };
    }
}

/// `FindClosestShipDepot`.
fn find_closest_ship_depot(t: &Tables<'_>, v: VehicleRef, max_distance: u32) -> Option<Depot> {
    crate::witness::hit(crate::witness::SHIP_DEPOT_SEARCH);
    let world = t.world();
    let map = world.dimensions;
    // `int max_region_distance = (max_distance / WATER_REGION_EDGE_LENGTH) + 1`, in `uint`.
    let max_region_distance = (max_distance / WATER_REGION_EDGE_LENGTH.cast_unsigned())
        .wrapping_add(1)
        .cast_signed();
    // SAFETY: The shared table returns the static water table, whose service returns the
    // current map's region owner. `&mut` is formed only for each `water_regions` call.
    let regions = unsafe { ((*(t.ss.water_services)()).water_regions)() };
    let mut search = DepotSearch::take();
    search.visited_patch_hashes.clear();
    search.patches_to_search.clear();

    /* Step 1: find a set of reachable Water Region Patches using BFS. */
    // SAFETY: As `regions`.
    let start_patch = get_water_region_patch_info(unsafe { &mut *regions }, world, t.ss, v.tile());
    search.patches_to_search.push_back(start_patch);
    search
        .visited_patch_hashes
        .insert(calculate_water_region_patch_hash(map, start_patch));

    while let Some(current_node) = search.patches_to_search.pop_front() {
        /* Remove first patch from the queue and make it the current patch. */

        /* Add neighbours of the current patch to the search queue. */
        let mut visit = WaterVisit::new();
        // SAFETY: As `regions`.
        while let Some(water_region_patch) = visit_water_region_patch_neighbours(
            unsafe { &mut *regions },
            world,
            t.ss,
            &mut visit,
            current_node,
        ) {
            /* Note that we check the max distance per axis, not the total distance. */
            if (water_region_patch.x - start_patch.x).abs() > max_region_distance
                || (water_region_patch.y - start_patch.y).abs() > max_region_distance
            {
                continue;
            }

            let hash = calculate_water_region_patch_hash(map, water_region_patch);
            if !search.visited_patch_hashes.contains(&hash) {
                search.visited_patch_hashes.insert(hash);
                search.patches_to_search.push_back(water_region_patch);
            }
        }
    }

    /* Step 2: Find the closest depot within the reachable Water Region Patches. */
    let mut best_depot = None;
    let mut best_dist_sq = u32::MAX;
    let mut first = 0;
    let mut depot = Depot::default();
    while (t.l.next_depot)(first, &raw mut depot) {
        first = u32::from(depot.index) + 1;
        let tile = TileIndex(depot.xy);
        if world.is_ship_depot_tile(tile) && world.is_tile_owner(tile, v.owner()) {
            let dist_sq = map.distance_square(tile, v.tile());
            if dist_sq < best_dist_sq
                && dist_sq <= max_distance.wrapping_mul(max_distance)
                && search
                    .visited_patch_hashes
                    .contains(&calculate_water_region_patch_hash(
                        map,
                        // SAFETY: As `regions`.
                        get_water_region_patch_info(unsafe { &mut *regions }, world, t.ss, tile),
                    ))
            {
                best_dist_sq = dist_sq;
                best_depot = Some(depot);
            }
        }
    }
    search.put_back();

    best_depot
}

/// `CheckIfShipNeedsService`.
fn check_if_ship_needs_service(t: &Tables<'_>, v: Ship) {
    let defaults = ((t.company().settings)().company_vehicle_defaults)(v.v.owner());
    if read_field!(defaults, servint_ships) == 0
        || !maintenance::needs_automatic_servicing(v.v, t.vs)
    {
        return;
    }
    if v.is_in_depot() {
        maintenance::vehicle_service_in_depot(v.v, t.vs);
        return;
    }

    // SAFETY: The catalog addresses the process-wide setting; one field-sized load.
    let max_distance = unsafe { t.ss.settings().maximum_go_to_depot_penalty.read() }
        / crate::pathfinder_type::YAPF_TILE_LENGTH.cast_unsigned();

    let Some(depot) = find_closest_ship_depot(t, v.v, max_distance) else {
        if v.v.current_order().is_type(OrderType::OT_GOTO_DEPOT) {
            // SAFETY: The current order belongs to this live serialized record.
            unsafe { crate::orders::make_dummy(v.v.current_order_pointer()) };
            t.start_stop_dirty(v.v);
        }
        return;
    };
    crate::witness::hit(crate::witness::SHIP_AUTO_SERVICE);

    // SAFETY: The current order belongs to this live serialized record.
    unsafe {
        crate::orders::make_go_to_depot(
            v.v.current_order_pointer(),
            DestinationID(depot.index),
            OrderDepotTypeFlags::from_element(OrderDepotTypeFlag::Service),
            OrderNonStopFlags::from_element(OrderNonStopFlag::NoIntermediate),
            OrderDepotActionFlags::EMPTY,
            crate::cargo_type::CargoType::CARGO_NO_REFIT,
        );
    }
    set_dest_tile(v, TileIndex(depot.xy));
    t.start_stop_dirty(v.v);
}

/// `Ship::UpdateCache`: update the caches of this ship.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "UpdateCache's uint raw_speed and the uint16_t cache members of GetVehicleProperty's int"
)]
pub(crate) fn update_cache(t: &Tables<'_>, v: Ship) {
    let svi = (t.vs.engine)(v.v);

    /* Get speed fraction for the current water type. Aqueducts are always canals. */
    let is_ocean = get_effective_water_class(t.world(), v.v.tile()) == WaterClass::Sea;
    let raw_speed = get_vehicle_property(
        t.vs.newgrf(),
        v.v,
        PropertyID::PROP_SHIP_SPEED,
        i32::from(svi.ship_speed()),
    ) as u32;
    v.v.set_cached_max_speed(crate::engine_type::apply_water_class_speed_frac(
        raw_speed,
        is_ocean,
        svi.ship_speed_fraction(true),
        svi.ship_speed_fraction(false),
    ) as u16);

    /* Update cargo aging period. */
    v.v.set_cached_cargo_age_period(get_vehicle_property(
        t.vs.newgrf(),
        v.v,
        PropertyID::PROP_SHIP_CARGO_AGE_PERIOD,
        i32::from((t.vs.engine)(v.v).cargo_age()),
    ) as u16);

    crate::vehicle_effect::update_visual_effect(t.vs, t.ss, v.v, true);
}

/// `Ship::OnNewCalendarDay`: calendar day handler.
pub(crate) fn on_new_calendar_day(v: VehicleRef, leaves: &Leaves) {
    // SAFETY: The native table names the immutable process-wide base table.
    maintenance::age_vehicle(v, unsafe { &*leaves.vehicle });
}

/// `Ship::OnNewEconomyDay`: economy day handler.
pub(crate) fn on_new_economy_day(v: VehicleRef, leaves: &Leaves, shared: &Services) {
    let t = Tables::new(leaves, shared);
    let v = Ship::of(v);
    crate::witness::hit(crate::witness::SHIP_ECONOMY_DAY);
    maintenance::economy_age_vehicle(v.v, t.vs);

    v.v.set_day_counter(v.v.day_counter().wrapping_add(1));
    if (v.v.day_counter() & 7) == 0 {
        maintenance::decrease_vehicle_value(v.v, t.vs);
    }

    maintenance::check_vehicle_breakdown(v.v, t.vs);
    check_if_ship_needs_service(&t, v);

    crate::orders::check_orders(v.v, t.vs.orders());

    if v.v.running_ticks() == 0 {
        return;
    }

    // SAFETY: The live ship and its engine; the cost callbacks retain no reference.
    let cost = unsafe { t.vs.running_cost(v.v) }.saturating_mul(i64::from(v.v.running_ticks()))
        / i64::from(crate::timer_game_common::DAYS_IN_YEAR * crate::timer_game_tick::DAY_TICKS);

    v.v.set_profit_this_year(t.vs, v.v.profit_this_year(t.vs).saturating_sub(cost));
    v.v.set_running_ticks(0);

    crate::company::subtract_money_from_company_fract(
        t.company().finance,
        v.v.owner(),
        cost,
        ExpensesType::EXPENSES_SHIP_RUN,
    );

    set_window_dirty(
        WindowClass::WC_VEHICLE_DETAILS,
        WindowNumber::from(t.index(v.v)),
    );
    /* we need this for the profit */
    set_window_classes_dirty(WindowClass::WC_SHIPS_LIST);
}

/// `Ship::GetVehicleTrackdir`.
pub(crate) fn get_vehicle_trackdir(world: &World<'_>, v: Ship) -> Trackdir {
    let check = |ok| world.check(ok);
    if v.v.vehstatus().test(VehState::Crashed) {
        return Trackdir::INVALID_TRACKDIR;
    }

    if v.is_in_depot() {
        /* We'll assume the ship is facing outwards */
        return diag_dir_to_diag_trackdir(world.get_ship_depot_direction(v.v.tile()), check);
    }

    if v.state() == TrackBits::TRACK_BIT_WORMHOLE {
        /* ship on aqueduct, so just use its direction and assume a diagonal track */
        return diag_dir_to_diag_trackdir(dir_to_diag_dir(v.v.direction(), check), check);
    }

    track_direction_to_trackdir(find_first_track(v.state()), v.v.direction(), check)
}

/// `Ship::GetOrderStationLocation`.
pub(crate) fn get_order_station_location(
    vs: &VehicleServices,
    ss: &Services,
    v: Ship,
    station: StationID,
) -> TileIndex {
    if station == v.v.last_station_visited() {
        v.v.set_last_station_visited(StationID::INVALID);
    }

    let runtime = crate::newgrf_airport::runtime();
    let st = crate::newgrf_airport::station_by_id(station.0, runtime);
    if crate::vehicle_func::can_vehicle_use_station(v.v, st, runtime) {
        TileIndex((ss.station_xy)(station.0))
    } else {
        crate::orders::increment_real_order_index(v.v, vs.orders());
        TileIndex(0)
    }
}

/// `CheckReverseShip`.
fn check_reverse_ship(t: &Tables<'_>, v: Ship, trackdir: Option<&mut Trackdir>) -> bool {
    /* Ask pathfinder for best direction */
    crate::ship_yapf::yapf_ship_check_reverse(t.ss, v, trackdir)
}

/// `CheckShipStayInDepot`: checks whether a ship should stay in the depot. True if the ship
/// should stay in the depot, false if it has to leave.
fn check_ship_stay_in_depot(t: &Tables<'_>, v: Ship) -> bool {
    let world = t.world();
    let check = t.check();
    if !v.is_in_depot() {
        return false;
    }

    /* Check if we should wait here for unbunching. */
    if crate::vehicle::is_waiting_for_unbunching(v.v, t.vs.orders(), || v.is_in_depot()) {
        return true;
    }

    /* We are leaving a depot, but have to go to the exact same one; re-enter */
    if v.v.current_order().is_type(OrderType::OT_GOTO_DEPOT)
        && world.is_ship_depot_tile(v.v.tile())
        && DestinationID((t.ss.depot_index)(v.v.tile().0)) == v.v.current_order().destination()
    {
        crate::vehicle::visits::enter_depot(v.v, t.vs.visits());
        return true;
    }

    /* Don't leave depot if no destination set */
    if v.v.dest_tile() == TileIndex(0) {
        return true;
    }

    /* Don't leave depot if another vehicle is already entering/leaving */
    /* This helps avoid CPU load if many ships are set to start at the same time */
    if crate::vehicle_func::has_vehicle_on_tile(t.vs, t.map(), v.v.tile(), |u| {
        u.vehicle_type() == VehicleType::VEH_SHIP && u.cur_speed() != 0
    }) {
        return true;
    }

    world.assert_with(|| {
        get_vehicle_trackdir(world, v) == Trackdir::TRACKDIR_X_NE
            || get_vehicle_trackdir(world, v) == Trackdir::TRACKDIR_Y_NW
    });
    v.v.set_direction(diag_dir_to_dir(
        trackdir_to_exitdir(get_vehicle_trackdir(world, v), check),
        check,
    ));
    if check_reverse_ship(t, v, None) {
        v.v.set_direction(reverse_dir(v.v.direction(), check));
    }
    crate::witness::hit(crate::witness::SHIP_DEPOT_LEAVE);

    v.set_state(axis_to_track_bits(
        world.get_ship_depot_axis(v.v.tile()),
        check,
    ));
    v.set_rotation(v.v.direction());
    v.v.set_hidden(false);
    v.v.set_cur_speed(0);
    t.update_viewport(v);
    set_window_dirty(
        WindowClass::WC_VEHICLE_DEPOT,
        WindowNumber::from(v.v.tile()),
    );

    maintenance::vehicle_service_in_depot(v.v, t.vs);
    crate::orders::VisitOrders::new(t.vs.orders()).leave_unbunching_depot(v.v);
    (t.vs.play_leave_station_sound)(v.v);
    invalidate_window_data(
        WindowClass::WC_VEHICLE_DEPOT,
        WindowNumber::from(v.v.tile()),
        0,
        false,
    );
    set_window_classes_dirty(WindowClass::WC_SHIPS_LIST);

    false
}

/// `ShipAccelerate`: accelerates the ship towards its target speed. Returns the number of
/// steps to move the ship.
#[allow(
    clippy::cast_possible_truncation,
    reason = "ShipAccelerate's uint speed stored to cur_speed, and static_cast<uint8_t>(remainder)"
)]
fn ship_accelerate(t: &Tables<'_>, v: VehicleRef) -> u32 {
    let mut speed: u32;
    speed = (u32::from(v.cur_speed()) + u32::from(v.acceleration()))
        .min(u32::from(v.cached_max_speed()));
    speed = speed.min(u32::from(v.current_order().max_speed()) * 2);

    /* updates statusbar only if speed have changed to save CPU time */
    if speed != u32::from(v.cur_speed()) {
        v.set_cur_speed(speed as u16);
        t.start_stop_dirty(v);
    }

    let advance_speed = VehicleRef::get_advance_speed(speed);
    let number_of_steps =
        advance_speed.wrapping_add(u32::from(v.progress())) / v.get_advance_distance();
    let remainder = advance_speed.wrapping_add(u32::from(v.progress())) % v.get_advance_distance();
    t.world().assert_with(|| u8::try_from(remainder).is_ok());
    v.set_progress(remainder as u8);
    number_of_steps
}

/// `ShipArrivesAt`: ship arrives at a dock. If it is the first time, send out a news item.
fn ship_arrives_at(t: &Tables<'_>, v: VehicleRef, st: *mut crate::station::BaseStation) {
    let index = crate::newgrf_airport::runtime().station_index(st);
    // The live station's field, resolved in this serialized entry; byte-sized loads and
    // stores below.
    let had_vehicle_of_type = (t.ss.station_had_vehicle_of_type_field)(st.cast());
    /* Check if station was ever visited before */
    // SAFETY: As `had_vehicle_of_type`.
    if unsafe { *had_vehicle_of_type } & StationHadVehicleOfType::HVOT_SHIP.0 == 0 {
        // SAFETY: As `had_vehicle_of_type`.
        unsafe { *had_vehicle_of_type |= StationHadVehicleOfType::HVOT_SHIP.0 };

        add_vehicle_news_item(
            &get_encoded_string(STR_NEWS_FIRST_SHIP_ARRIVAL, &[Integer(u64::from(index.0))]),
            if v.owner().0 == (t.ss.local_company)() {
                NewsType::ArrivalCompany
            } else {
                NewsType::ArrivalOther
            },
            t.index(v),
            index,
        );
        crate::ai::new_event(
            v.owner(),
            &ScriptEvent::station_first_vehicle(index, t.index(v)),
        );
        crate::game::new_event(&ScriptEvent::station_first_vehicle(index, t.index(v)));
    }
}

/// `ChooseShipTrack`: runs the pathfinder to choose a track to continue along on `tile`,
/// the tile the ship is about to enter, from the available track choices `tracks`. Returns
/// the track to choose, or `INVALID_TRACK` when to reverse.
fn choose_ship_track(t: &Tables<'_>, v: Ship, tile: TileIndex, tracks: TrackBits) -> Track {
    let check = t.check();
    let mut path_found = true;
    let mut track;

    if v.v.dest_tile() == TileIndex(0) {
        /* No destination, don't invoke pathfinder. */
        track = track_bits_to_track(v.state(), check);
        if !is_diagonal_track(track, check) {
            track = track_to_opposite_track(track, check);
        }
        if !crate::bitmath_func::has_bit(tracks.0, track.0) {
            track = find_first_track(tracks);
        }
        path_found = false;
    } else {
        /* Attempt to follow cached path. */
        if !v.path_empty() {
            crate::witness::hit(crate::witness::SHIP_PATH_CACHE);
            track = trackdir_to_track(v.path_back(), check);

            if crate::bitmath_func::has_bit(tracks.0, track.0) {
                v.path_pop_back();
                /* HandlePathfindResult() is not called here because this is not a new pathfinder result. */
                return track;
            }

            /* Cached path is invalid so continue with pathfinder. */
            v.path_clear();
        }

        // SAFETY: The search holds the only reference to the path for its call: it calls no
        // service that reaches the controller or a ship's owner.
        track = crate::ship_yapf::yapf_ship_choose_track(t.ss, v, tile, &mut path_found, unsafe {
            &mut *v.path()
        });
    }

    (t.vs.pathfinding_result)(v.v, path_found);
    track
}

/// `GetAvailShipTracks`: get the available water tracks on `tile`, the tile about to enter,
/// for a ship entering it in direction `dir`.
#[inline]
fn get_avail_ship_tracks(t: &Tables<'_>, tile: TileIndex, dir: DiagDirection) -> TrackBits {
    t.water_track_bits(tile, DiagDirection::INVALID_DIAGDIR)
        & diagdir_reaches_tracks(dir, t.check())
}

/// `ShipSubcoordData`: ship sub-coordinate data for moving into a new tile via a `Diagdir`
/// onto a `Track`.
#[derive(Clone, Copy)]
struct ShipSubcoordData {
    /// New X sub-coordinate on the new tile.
    x_subcoord: u8,
    /// New Y sub-coordinate on the new tile.
    y_subcoord: u8,
    /// New Direction to move in on the new track.
    dir: Direction,
}
const fn subcoord(x_subcoord: u8, y_subcoord: u8, dir: Direction) -> ShipSubcoordData {
    ShipSubcoordData {
        x_subcoord,
        y_subcoord,
        dir,
    }
}
/// Ship sub-coordinate data for moving into a new tile via a `Diagdir` onto a `Track`.
/// Array indexes are `Diagdir`, `Track`.
/// There will always be three possible tracks going into an adjacent tile via a `Diagdir`,
/// so each `Diagdir` sub-array will have three valid and three invalid structures per `Track`.
#[rustfmt::skip]
const SHIP_SUBCOORD: [[ShipSubcoordData; Track::TRACK_END.0 as usize];
    DiagDirection::DIAGDIR_END.0 as usize] = {
    use Direction as D;
    [
        /* DIAGDIR_NE */
        [
            subcoord(15,  8, D::DIR_NE),      // TRACK_X
            subcoord( 0,  0, D::INVALID_DIR), // TRACK_Y
            subcoord( 0,  0, D::INVALID_DIR), // TRACK_UPPER
            subcoord(15,  8, D::DIR_E),       // TRACK_LOWER
            subcoord(15,  7, D::DIR_N),       // TRACK_LEFT
            subcoord( 0,  0, D::INVALID_DIR), // TRACK_RIGHT
        ],
        /* DIAGDIR_SE */
        [
            subcoord( 0,  0, D::INVALID_DIR), // TRACK_X
            subcoord( 8,  0, D::DIR_SE),      // TRACK_Y
            subcoord( 7,  0, D::DIR_E),       // TRACK_UPPER
            subcoord( 0,  0, D::INVALID_DIR), // TRACK_LOWER
            subcoord( 8,  0, D::DIR_S),       // TRACK_LEFT
            subcoord( 0,  0, D::INVALID_DIR), // TRACK_RIGHT
        ],
        /* DIAGDIR_SW */
        [
            subcoord( 0,  8, D::DIR_SW),      // TRACK_X
            subcoord( 0,  0, D::INVALID_DIR), // TRACK_Y
            subcoord( 0,  7, D::DIR_W),       // TRACK_UPPER
            subcoord( 0,  0, D::INVALID_DIR), // TRACK_LOWER
            subcoord( 0,  0, D::INVALID_DIR), // TRACK_LEFT
            subcoord( 0,  8, D::DIR_S),       // TRACK_RIGHT
        ],
        /* DIAGDIR_NW */
        [
            subcoord( 0,  0, D::INVALID_DIR), // TRACK_X
            subcoord( 8, 15, D::DIR_NW),      // TRACK_Y
            subcoord( 0,  0, D::INVALID_DIR), // TRACK_UPPER
            subcoord( 8, 15, D::DIR_W),       // TRACK_LOWER
            subcoord( 0,  0, D::INVALID_DIR), // TRACK_LEFT
            subcoord( 7, 15, D::DIR_N),       // TRACK_RIGHT
        ],
    ]
};

/// `ShipTestUpDownOnLock`: test if a ship is in the centre of a lock and should move up or
/// down. Returns 0 if ship is not moving in lock, or -1 to move down, 1 to move up.
#[allow(
    clippy::cast_possible_wrap,
    reason = "ShipTestUpDownOnLock's (int)TILE_HEIGHT"
)]
fn ship_test_up_down_on_lock(t: &Tables<'_>, v: VehicleRef) -> i32 {
    let world = t.world();
    /* Suitable tile? */
    if !(t.ss.water_middle_lock)(v.tile().0) {
        return 0;
    }

    /* Must be at the centre of the lock */
    if (v.x_pos() & 0xF) != 8 || (v.y_pos() & 0xF) != 8 {
        return 0;
    }

    let diagdir = get_inclined_slope_direction(world.tile_slope(v.tile()));
    world.assert_with(|| is_valid_diag_direction(diagdir));

    if dir_to_diag_dir(v.direction(), t.check()) == diagdir {
        /* Move up */
        i32::from(v.z_pos() < world.tile_max_z(v.tile()) * TILE_HEIGHT as i32)
    } else {
        /* Move down */
        -i32::from(v.z_pos() > world.tile_z(v.tile()) * TILE_HEIGHT as i32)
    }
}

/// `ShipMoveUpDownOnLock`: test and move a ship up or down in a lock. Returns true iff the
/// ship is moving up or down in a lock.
fn ship_move_up_down_on_lock(t: &Tables<'_>, v: Ship) -> bool {
    /* Moving up/down through lock */
    let dz = ship_test_up_down_on_lock(t, v.v);
    if dz == 0 {
        return false;
    }
    if dz > 0 {
        crate::witness::hit(crate::witness::SHIP_LOCK_UP);
    } else {
        crate::witness::hit(crate::witness::SHIP_LOCK_DOWN);
    }

    if v.v.cur_speed() != 0 {
        v.v.set_cur_speed(0);
        t.start_stop_dirty(v.v);
    }

    if (v.v.tick_counter() & 7) == 0 {
        v.v.set_z_pos(v.v.z_pos().wrapping_add(dz));
        t.update_position(v.v);
        t.update_viewport(v);
    }

    true
}

/// `IsShipDestinationTile`: test if `tile`, a docking tile, is a docking tile for `station`.
/// True iff the docking tile is next to the station.
pub(crate) fn is_ship_destination_tile(
    world: &World<'_>,
    tile: TileIndex,
    station: StationID,
) -> bool {
    world.assert_with(|| world.is_docking_tile(tile));
    /* Check each tile adjacent to docking tile. */
    for d in DiagDirection::DIAGDIR_BEGIN.0..DiagDirection::DIAGDIR_END.0 {
        let t = tile
            + world
                .dimensions
                .tile_offs_by_diag_dir(DiagDirection(d), |ok| world.check(ok));
        if !world.is_valid_tile(t) {
            continue;
        }
        if world.is_dock_tile(t)
            && world.get_station_index(t) == station
            && world.is_dock_water_part(t)
        {
            return true;
        }
        if world.kind(t) == TileType::MP_INDUSTRY {
            let runtime = crate::newgrf_airport::runtime();
            let neutral_station = (runtime.commands().neutral_for_tile)(t.0);
            if !neutral_station.is_null() && runtime.station_index(neutral_station) == station {
                return true;
            }
        }
        if world.kind(t) == TileType::MP_STATION
            && world.is_oil_rig(t)
            && world.get_station_index(t) == station
        {
            return true;
        }
    }
    false
}

/// `ReverseShipIntoTrackdir`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "the int16_t rotation_x_pos and rotation_y_pos of the int positions"
)]
fn reverse_ship_into_trackdir(t: &Tables<'_>, v: Ship, trackdir: Trackdir) {
    use Direction as D;
    const TRACKDIR_TO_DIRECTION: [Direction; 16] = [
        D::DIR_NE,
        D::DIR_SE,
        D::DIR_E,
        D::DIR_E,
        D::DIR_S,
        D::DIR_S,
        D::INVALID_DIR,
        D::INVALID_DIR,
        D::DIR_SW,
        D::DIR_NW,
        D::DIR_W,
        D::DIR_W,
        D::DIR_N,
        D::DIR_N,
        D::INVALID_DIR,
        D::INVALID_DIR,
    ];
    crate::witness::hit(crate::witness::SHIP_REVERSE);

    v.v.set_direction(TRACKDIR_TO_DIRECTION[usize::from(trackdir.0)]);
    t.world()
        .assert_with(|| v.v.direction() != Direction::INVALID_DIR);
    v.set_state(trackdir_bits_to_track_bits(trackdir_to_trackdir_bits(
        trackdir,
        t.check(),
    )));

    /* Remember our current location to avoid movement glitch */
    v.set_rotation_x_pos(v.v.x_pos() as i16);
    v.set_rotation_y_pos(v.v.y_pos() as i16);
    v.v.set_cur_speed(0);
    v.path_clear();

    t.update_position(v.v);
    t.update_viewport(v);
}

/// `ReverseShip`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "the int16_t rotation_x_pos and rotation_y_pos of the int positions"
)]
fn reverse_ship(t: &Tables<'_>, v: Ship) {
    crate::witness::hit(crate::witness::SHIP_REVERSE);
    v.v.set_direction(reverse_dir(v.v.direction(), t.check()));

    /* Remember our current location to avoid movement glitch */
    v.set_rotation_x_pos(v.v.x_pos() as i16);
    v.set_rotation_y_pos(v.v.y_pos() as i16);
    v.v.set_cur_speed(0);
    v.path_clear();

    t.update_position(v.v);
    t.update_viewport(v);
}

/// `Ship::GetCurrentMaxSpeed`.
pub(crate) fn get_current_max_speed(v: VehicleRef) -> i32 {
    i32::from(v.cached_max_speed()).min(i32::from(v.current_order().max_speed()) * 2)
}

/// `ShipController`.
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    reason = "the original's one body; the int16_t rotation_x_pos and rotation_y_pos of the int positions"
)]
fn ship_controller(t: &Tables<'_>, v: Ship) {
    let check = t.check();
    v.v.set_tick_counter(v.v.tick_counter().wrapping_add(1));
    v.v.set_current_order_time(v.v.current_order_time().wrapping_add(1));

    if maintenance::handle_breakdown(v.v, t.vs) {
        return;
    }

    if v.v.vehstatus().test(VehState::Stopped) {
        return;
    }

    if crate::orders::process_orders(v.v, t.vs.orders()) && check_reverse_ship(t, v, None) {
        return reverse_ship(t, v);
    }

    t.vs.handle_loading(v.v, false);

    if v.v.current_order().is_type(OrderType::OT_LOADING) {
        return;
    }

    if check_ship_stay_in_depot(t, v) {
        return;
    }

    crate::vehicle_effect::show_visual_effect(t.vs, t.ss, v.v, || get_current_max_speed(v.v));

    /* Rotating on spot */
    if v.v.direction() != v.rotation() {
        crate::witness::hit(crate::witness::SHIP_ROTATE);
        if (v.v.tick_counter() & 7) == 0 {
            let diff = dir_difference(v.v.direction(), v.rotation(), check);
            v.set_rotation(change_dir(
                v.rotation(),
                if diff > DirDiff::DIRDIFF_REVERSE {
                    DirDiff::DIRDIFF_45LEFT
                } else {
                    DirDiff::DIRDIFF_45RIGHT
                },
                check,
            ));
            /* Invalidate the sprite cache direction to force recalculation of viewport */
            v.v.set_sprite_cache_last_direction(Direction::INVALID_DIR);
            t.update_viewport(v);
        }
        return;
    }

    if ship_move_up_down_on_lock(t, v) {
        return;
    }

    let number_of_steps = ship_accelerate(t, v.v);
    for _ in 0..number_of_steps {
        if ship_move_up_down_on_lock(t, v) {
            return;
        }

        let mut gp = crate::vehicle::get_new_vehicle_pos(v.v, t.map());
        if v.state() != TrackBits::TRACK_BIT_WORMHOLE {
            /* Not on a bridge */
            if gp.old_tile == gp.new_tile {
                /* Staying in tile */
                if v.is_in_depot() {
                    gp.x = v.v.x_pos();
                    gp.y = v.v.y_pos();
                } else {
                    /* Not inside depot */
                    let vets = t.vehicle_enter_tile(v.v, gp.new_tile, gp.x, gp.y);
                    if vets.test(VehicleEnterTileState::CannotEnter) {
                        return reverse_ship(t, v);
                    }

                    /* A leave station order only needs one tick to get processed, so we can
                     * always skip ahead. */
                    if v.v.current_order().is_type(OrderType::OT_LEAVESTATION) {
                        crate::orders::free(v.v);
                        t.start_stop_dirty(v.v);
                        /* Test if continuing forward would lead to a dead-end, moving into the dock. */
                        let exitdir = vehicle_exit_dir(v.v.direction(), v.state(), check);
                        let tile = t.map().tile_add_by_diag_dir(v.v.tile(), exitdir, check);
                        if t.water_track_bits(tile, exitdir) == TrackBits::TRACK_BIT_NONE {
                            return reverse_ship(t, v);
                        }
                    } else if v.v.dest_tile() != TileIndex(0) {
                        /* We have a target, let's see if we reached it... */
                        if v.v.current_order().is_type(OrderType::OT_GOTO_WAYPOINT)
                            && t.map().distance_manhattan(v.v.dest_tile(), gp.new_tile) <= 3
                        {
                            /* We got within 3 tiles of our target buoy, so let's skip to our
                             * next order */
                            crate::witness::hit(crate::witness::SHIP_BUOY);
                            crate::orders::VisitOrders::new(t.vs.orders())
                                .update_timetable(v.v, true);
                            crate::orders::increment_real_order_index(v.v, t.vs.orders());
                            // SAFETY: The current order belongs to this live serialized record.
                            unsafe { crate::orders::make_dummy(v.v.current_order_pointer()) };
                        } else if v.v.current_order().is_type(OrderType::OT_GOTO_DEPOT)
                            && v.v.dest_tile() == gp.new_tile
                        {
                            /* Depot orders really need to reach the tile */
                            if (gp.x & 0xF) == 8 && (gp.y & 0xF) == 8 {
                                crate::witness::hit(crate::witness::SHIP_DEPOT_ENTER);
                                crate::vehicle::visits::enter_depot(v.v, t.vs.visits());
                                return;
                            }
                        } else if v.v.current_order().is_type(OrderType::OT_GOTO_STATION)
                            && t.world().is_docking_tile(gp.new_tile)
                        {
                            /* Process station in the orderlist. */
                            let runtime = crate::newgrf_airport::runtime();
                            let st = crate::newgrf_airport::station_by_id(
                                v.v.current_order().destination().to_station_id().0,
                                runtime,
                            );
                            let index = runtime.station_index(st);
                            // SAFETY: The live station's docking area, read in place.
                            let docking_station = unsafe { *(runtime.commands().docking_area)(st) };
                            if docking_station.contains(gp.new_tile, t.map(), check)
                                && is_ship_destination_tile(t.world(), gp.new_tile, index)
                            {
                                v.v.set_last_station_visited(index);
                                if runtime
                                    .commands()
                                    .facilities(st)
                                    .test(StationFacility::Dock)
                                {
                                    // ugly, ugly workaround for problem with ships able to drop off cargo at wrong stations
                                    crate::witness::hit(crate::witness::SHIP_LOADING);
                                    ship_arrives_at(t, v.v, st);
                                    crate::vehicle::visits::begin_loading(v.v, t.vs.visits());
                                } else {
                                    // leave stations without docks right away
                                    crate::orders::VisitOrders::new(t.vs.orders())
                                        .make_leave_station(v.v);
                                    crate::orders::increment_real_order_index(v.v, t.vs.orders());
                                }
                            }
                        }
                    }
                }
            } else {
                /* New tile */
                if !t.world().is_valid_tile(gp.new_tile) {
                    return reverse_ship(t, v);
                }

                let diagdir = t.map().diagdir_between_tiles(gp.old_tile, gp.new_tile);
                t.world()
                    .assert_with(|| diagdir != DiagDirection::INVALID_DIAGDIR);
                let tracks = get_avail_ship_tracks(t, gp.new_tile, diagdir);
                if tracks == TrackBits::TRACK_BIT_NONE {
                    let mut trackdir = Trackdir::INVALID_TRACKDIR;
                    check_reverse_ship(t, v, Some(&mut trackdir));
                    if trackdir == Trackdir::INVALID_TRACKDIR {
                        return reverse_ship(t, v);
                    }
                    return reverse_ship_into_trackdir(t, v, trackdir);
                }

                /* Choose a direction, and continue if we find one */
                let track = choose_ship_track(t, v, gp.new_tile, tracks);
                if track == Track::INVALID_TRACK {
                    return reverse_ship(t, v);
                }

                let b = SHIP_SUBCOORD[usize::from(diagdir.0)][usize::from(track.0)];

                gp.x = (gp.x & !0xF) | i32::from(b.x_subcoord);
                gp.y = (gp.y & !0xF) | i32::from(b.y_subcoord);

                /* Call the landscape function and tell it that the vehicle entered the tile */
                let vets = t.vehicle_enter_tile(v.v, gp.new_tile, gp.x, gp.y);
                if vets.test(VehicleEnterTileState::CannotEnter) {
                    return reverse_ship(t, v);
                }

                if !vets.test(VehicleEnterTileState::EnteredWormhole) {
                    v.v.set_tile(gp.new_tile);
                    v.set_state(track_to_track_bits(track, check));

                    /* Update ship cache when the water class changes. Aqueducts are always canals. */
                    if get_effective_water_class(t.world(), gp.old_tile)
                        != get_effective_water_class(t.world(), gp.new_tile)
                    {
                        crate::witness::hit(crate::witness::SHIP_WATER_CHANGE);
                        update_cache(t, v);
                    }
                }

                let new_direction = b.dir;
                let diff = dir_difference(new_direction, v.v.direction(), check);
                match diff {
                    DirDiff::DIRDIFF_SAME | DirDiff::DIRDIFF_45RIGHT | DirDiff::DIRDIFF_45LEFT => {
                        /* Continue at speed */
                        v.v.set_direction(new_direction);
                        v.set_rotation(new_direction);
                    }

                    _ => {
                        /* Stop for rotation */
                        v.v.set_cur_speed(0);
                        v.v.set_direction(new_direction);
                        /* Remember our current location to avoid movement glitch */
                        v.set_rotation_x_pos(v.v.x_pos() as i16);
                        v.set_rotation_y_pos(v.v.y_pos() as i16);
                    }
                }
            }
        } else {
            /* On a bridge */
            crate::witness::hit(crate::witness::SHIP_AQUEDUCT);
            if !t
                .world()
                .is_tile_type(gp.new_tile, TileType::MP_TUNNELBRIDGE)
                || !t
                    .vehicle_enter_tile(v.v, gp.new_tile, gp.x, gp.y)
                    .test(VehicleEnterTileState::EnteredWormhole)
            {
                v.v.set_x_pos(gp.x);
                v.v.set_y_pos(gp.y);
                t.update_position(v.v);
                if !v.v.vehstatus().test(VehState::Hidden) {
                    (t.vs.base_viewport)(v.v);
                }
                continue;
            }

            /* Ship is back on the bridge head, we need to consume its path
             * cache entry here as we didn't have to choose a ship track. */
            if !v.path_empty() {
                v.path_pop_back();
            }
        }

        /* update image of ship, as well as delta XY */
        v.v.set_x_pos(gp.x);
        v.v.set_y_pos(gp.y);

        t.update_position(v.v);
        t.update_viewport(v);
    }
}

/// `Ship::Tick`, without its `PerformanceAccumulator`, which the C++ shell keeps.
pub(crate) fn tick(t: &Tables<'_>, v: Ship) -> bool {
    if !v.v.vehstatus().test(VehState::Stopped) {
        v.v.set_running_ticks(v.v.running_ticks().wrapping_add(1));
    }

    ship_controller(t, v);

    true
}

/// `Ship::SetDestTile`.
pub(crate) fn set_dest_tile(v: Ship, tile: TileIndex) {
    if tile == v.v.dest_tile() {
        return;
    }
    v.path_clear();
    v.v.set_dest_tile(tile);
}

/// Arguments of `CmdBuildShip`.
#[derive(Clone, Copy)]
pub(crate) struct CmdBuildShip {
    pub tile: TileIndex,
    pub e: *const crate::engine_type::OpenTTDEngine,
}

/// `CmdBuildShip`: build a ship at the depot `tile`; returns the new ship, or null when
/// only testing.
///
/// # Safety
/// Live native cost and engine; the new shell stays live, and no owner access spans a
/// callback.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    reason = "CmdBuildShip's int x and y of TileX and TileY, Random()'s uint16_t random_bits and DetermineCapacity's uint stored to cargo_cap"
)]
pub(crate) unsafe fn cmd_build_ship(
    s: &crate::vehicle_transactions::build::Services,
    out: *mut crate::services::CommandCost,
    flags: crate::command_type::DoCommandFlags,
    cmd: CmdBuildShip,
) -> *mut () {
    let CmdBuildShip { tile, e } = cmd;
    let construct = s.construct;
    let articulated = construct.articulated;
    let refit = s.refit;
    let base = refit.common.base;
    let mut shell = std::ptr::null_mut();
    let tile = TileIndex((s.ship_tile)(tile.0));
    if flags.test(DoCommandFlag::Execute) {
        let svi = crate::vehicle::EngineRef::of(e);

        shell = (s.new_ship)();
        // SAFETY: The new shell's record.
        let v = Ship::of(unsafe { VehicleRef::from_shell(shell.cast(), refit.common.vehicles) });
        let tables = Tables::new(refit.ship, base.shared);
        let check = tables.check();
        crate::witness::hit(crate::witness::SHIP_BUILD);

        let company = crate::company_type::CompanyID((base.shared.current_company)());
        v.v.set_owner(company);
        v.v.set_tile(tile);
        let x = (tables.map().tile_x(tile) * TILE_SIZE + TILE_SIZE / 2) as i32;
        let y = (tables.map().tile_y(tile) * TILE_SIZE + TILE_SIZE / 2) as i32;
        v.v.set_x_pos(x);
        v.v.set_y_pos(y);
        v.v.set_z_pos(crate::movement::get_slope_pixel_z(base.shared, x, y, false));

        v.v.set_direction(diag_dir_to_dir(
            tables.world().get_ship_depot_direction(tile),
            check,
        ));

        /* UpdateDeltaXY() requires rotation to be initialised as well. */
        v.set_rotation(v.v.direction());
        (tables.vs.update_delta)(v.v);

        v.v.set_vehstatus(
            VehStates::from_element(VehState::Hidden)
                .with(VehStates::from_element(VehState::Stopped))
                .with(VehStates::from_element(VehState::DefaultPalette)),
        );

        (tables.vs.write_spritenum)(v.v, svi.ship_image());
        v.v.set_cargo_type(tables.vs, crate::cargo_type::CargoType(svi.default_cargo()));
        crate::services_assert::assert_with(|| {
            is_valid_cargo_type(v.v.cargo_type(tables.vs.newgrf()).0)
        });
        v.v.set_cargo_cap(svi.ship_capacity());
        v.v.set_refit_cap(0);

        v.v.set_last_station_visited(StationID::INVALID);
        v.v.set_last_loading_station(tables.vs.visits(), StationID::INVALID);
        v.v.set_engine_type(crate::engine_type::EngineID((refit.capabilities.id)(e)));

        v.v.set_reliability(tables.vs, svi.reliability());
        // SAFETY: The base table names the static maintenance table.
        v.v.set_reliability_spd_dec(unsafe { &*tables.vs.maintenance }, svi.reliability_decay());
        (tables.vs.set_max_age)(
            v.v,
            crate::engine_capabilities::get_life_length_in_days(refit.capabilities, e),
        );

        v.set_state(TrackBits::TRACK_BIT_DEPOT);

        let defaults = ((tables.company().settings)().company_vehicle_defaults)(company);
        (construct.service_dates)(shell, read_field!(defaults, servint_ships));
        (construct.build_stamp)(shell);
        (articulated.random_bits)(shell, base.shared.random() as u16);

        v.v.set_acceleration(svi.ship_acceleration());
        update_cache(&tables, v);

        (construct.service_flags)(
            shell,
            svi.exclusive_preview(),
            base.interval_is_percent(company),
        );

        // SAFETY: The live new shell; no reference spans the invalidation.
        unsafe { crate::newgrf_vehicle::invalidate_newgrf_cache_of_chain(tables.vs.newgrf(), v.v) };

        // SAFETY: The build transaction retains the new shell across the capacity callbacks.
        v.v.set_cargo_cap(unsafe {
            crate::engine_capabilities::determine_capacity(
                refit.capabilities,
                e,
                v.v.0.as_ptr(),
                std::ptr::null_mut(),
            )
        } as u16);

        // SAFETY: As above.
        unsafe { crate::newgrf_vehicle::invalidate_newgrf_cache_of_chain(tables.vs.newgrf(), v.v) };

        tables.update_position(v.v);
    }

    (base.cost.zero)(out);
    shell
}

/// `Ship::FindClosestDepot`.
pub(crate) fn find_closest_depot(t: &Tables<'_>, v: Ship) -> ClosestDepot {
    let Some(depot) = find_closest_ship_depot(t, v.v, MAX_SHIP_DEPOT_SEARCH_DISTANCE) else {
        return ClosestDepot::default();
    };

    ClosestDepot::new(TileIndex(depot.xy), DestinationID(depot.index), false)
}

// The C++ shell's entries. SAFETY for each: the caller holds the game-state mutex and passes
// a live ship record and the immutable process-lifetime tables.

/// `Ship::Tick`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_tick_abi(
    v: VehicleRef,
    leaves: *const Leaves,
    shared: *const Services,
) -> u8 {
    // SAFETY: As the entries' contract.
    let t = unsafe { Tables::new(&*leaves, &*shared) };
    u8::from(tick(&t, Ship::of(v)))
}
/// `Ship::UpdateCache`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_cache(
    v: VehicleRef,
    leaves: *const Leaves,
    shared: *const Services,
) {
    // SAFETY: As the entries' contract.
    let t = unsafe { Tables::new(&*leaves, &*shared) };
    update_cache(&t, Ship::of(v));
}
/// `Ship::SetDestTile`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_destination(v: VehicleRef, tile: u32) {
    set_dest_tile(Ship::of(v), TileIndex(tile));
}
/// `Ship::FindClosestDepot`, written to `closest`.
///
/// # Safety
/// As the entries' contract; `closest` is a writable record.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_find_depot(
    v: VehicleRef,
    leaves: *const Leaves,
    shared: *const Services,
    closest: *mut crate::orders::Closest,
) {
    // SAFETY: As this function's contract.
    let t = unsafe { Tables::new(&*leaves, &*shared) };
    let result = find_closest_depot(&t, Ship::of(v));
    // SAFETY: As this function's contract.
    unsafe {
        closest.write(crate::orders::Closest {
            tile: result.location.0,
            destination: result.destination.0,
            reverse: u8::from(result.reverse),
            found: u8::from(result.found),
        });
    };
}
/// `Ship::GetOrderStationLocation`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_station_destination(
    v: VehicleRef,
    station: u16,
    leaves: *const Leaves,
    shared: *const Services,
) -> u32 {
    // SAFETY: As the entries' contract.
    let t = unsafe { Tables::new(&*leaves, &*shared) };
    get_order_station_location(t.vs, t.ss, Ship::of(v), StationID(station)).0
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDShipLeaves" ["probe"] => Leaves {
        update_viewport,
        next_depot,
        vehicle,
    };
    "OpenTTDShipDepot" ["probe"] => Depot {
        index,
        xy,
    };
}

#[cfg(test)]
mod tests;
