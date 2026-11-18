/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `roadveh_cmd.cpp`: handling of road vehicles, with the `RoadVehicle` members of
//! `roadveh.h` whose state Rust owns.
//!
//! Each entry receives a live vehicle record and the road owner its shell holds. Both stay
//! live and serialized by the game-state mutex for the synchronous call. Every field is
//! read and written in place at the original's observation point, and no owner borrow spans
//! a service that can reenter or mutate it.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    clippy::verbose_bit_mask,
    clippy::items_after_statements,
    clippy::collapsible_if,
    clippy::if_not_else
)]

use crate::company_type::Owner;
use crate::direction_func::{
    change_dir, diag_dir_to_axis, diag_dir_to_dir, dir_difference, dir_to_diag_dir,
    reverse_diag_dir,
};
use crate::direction_type::{DiagDirection, DirDiff, Direction};
use crate::engine_type::EngineID;
use crate::ground_vehicle::GroundVehicleFlags;
use crate::map_func::{Map, World};
use crate::math::is_inside_mm;
use crate::movement::CachedTable;
use crate::newgrf_callbacks::CBID_VEHICLE_LENGTH;
use crate::newgrf_properties::{
    PROP_ROADVEH_CARGO_AGE_PERIOD, PROP_ROADVEH_RUNNING_COST_FACTOR, PROP_ROADVEH_SHORTEN_FACTOR,
    PROP_ROADVEH_SPEED,
};
use crate::newgrf_sound::VSE_START;
use crate::news_func::{add_tile_news_item, add_vehicle_news_item};
use crate::news_type::NewsType;
use crate::order_type::{
    DestinationID, OrderDepotActionFlags, OrderDepotTypeFlag, OrderDepotTypeFlags,
    OrderNonStopFlag, OrderNonStopFlags, OrderType,
};
use crate::orders::Closest;
use crate::pathfinder_type::FindDepotData;
use crate::raw_field::{read_field, write_field};
use crate::road_type::{RoadBits, RoadTramType, RoadType, RoadTypes};
use crate::roadveh::{
    RDE_NEXT_TILE, RDE_TURNED, RV_OVERTAKE_TIMEOUT, RVC_DEFAULT_START_FRAME, RVC_DEPOT_START_FRAME,
    RVC_DRIVE_THROUGH_STOP_FRAME, RVC_START_FRAME_AFTER_LONG_TRAM, RVC_TURN_AROUND_START_FRAME,
    RVC_TURN_AROUND_START_FRAME_SHORT_TRAM, RVS_DRIVE_SIDE, RVS_ENTERED_STOP, RVS_IN_DT_ROAD_STOP,
    RVSB_DRIVE_SIDE, RVSB_IN_DEPOT, RVSB_IN_DT_ROAD_STOP, RVSB_IN_DT_ROAD_STOP_END,
    RVSB_IN_ROAD_STOP, RVSB_IN_ROAD_STOP_END, RVSB_ROAD_STOP_TRACKDIR_MASK, RVSB_TRACKDIR_MASK,
    RVSB_WORMHOLE,
};
use crate::script_event_types::{CrashReason, ScriptEvent};
use crate::services::{RoadStopType, Services};
use crate::sound_type::{SoundFx, SoundID};
use crate::station_type::{
    StationAnimationTrigger, StationHadVehicleOfType, StationID, StationRandomTrigger,
};
use crate::strings_func::get_encoded_string;
use crate::strings_type::StringParameterData::Integer;
use crate::table::strings::{
    STR_NEWS_FIRST_BUS_ARRIVAL, STR_NEWS_FIRST_CARGO_TRAM_ARRIVAL,
    STR_NEWS_FIRST_PASSENGER_TRAM_ARRIVAL, STR_NEWS_FIRST_TRUCK_ARRIVAL,
    STR_NEWS_ROAD_VEHICLE_CRASH, STR_NEWS_ROAD_VEHICLE_CRASH_DRIVER,
};
use crate::tile_cmd::{VehicleEnterTileState, VehicleEnterTileStates};
use crate::tile_type::{TILE_SIZE, TileIndex, TileType};
use crate::track_func::{
    diag_dir_to_diag_trackdir, diagdir_reaches_trackdirs, is_reversing_road_trackdir,
    is_straight_road_trackdir, track_status_to_red_signals, track_status_to_trackdir_bits,
    trackdir_bits_to_track_bits,
};
use crate::track_type::{TrackBits, TrackStatus, Trackdir, TrackdirBits};
use crate::transport_type::TransportType;
use crate::vehicle::maintenance;
use crate::vehicle::{GroundServices, VehicleRef, VehicleServices, YEAR_TICKS};
use crate::vehicle_base::ClosestDepot;
use crate::vehicle_base::{VehState, VehStates};
use crate::vehicle_type::{AccelerationModel, VEHICLE_LENGTH, VehicleID, VehicleType};
use std::ptr::NonNull;

/// Roadtype flags (`road.h`), as bit numbers, retaining every underlying byte.
/// The other enumerators join with their first Rust user.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoadTypeFlag(pub u8);
#[allow(
    non_upper_case_globals,
    reason = "original road.h RoadTypeFlag enumerators"
)]
impl RoadTypeFlag {
    /// Bit number for setting this roadtype as not house friendly
    pub const NoHouses: Self = Self(2);
    /// Bit number for hidden from construction.
    pub const Hidden: Self = Self(3);
    /// Bit number for allowing towns to build this roadtype.
    pub const TownBuild: Self = Self(4);
}
/// Set of `RoadTypeFlag`s (`road.h`), retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoadTypeFlags(pub u8);
impl RoadTypeFlags {
    /// Set containing the named flag (the element is a bit position).
    #[must_use]
    pub const fn from_element(element: RoadTypeFlag) -> Self {
        Self(1 << element.0)
    }
    /// Native `Test` of one element.
    #[must_use]
    pub const fn test(self, element: RoadTypeFlag) -> bool {
        crate::base_bitset_type::test(self.0 as u64, element.0)
    }
}

/// `RoadTypeSpriteGroup::ROTSG_END`.
pub const ROTSG_END: u8 = 12;

pub(crate) mod availability;
pub(crate) use availability::*;

/// `_road_reverse_table`.
const ROAD_REVERSE_TABLE: [Trackdir; 4] = [
    Trackdir::TRACKDIR_RVREV_NE,
    Trackdir::TRACKDIR_RVREV_SE,
    Trackdir::TRACKDIR_RVREV_SW,
    Trackdir::TRACKDIR_RVREV_NW,
];

crate::abi::shared_records! {
    fn abi_record;
    /// The scalar members of `RoadVehicle`, with the original widths and zero defaults.
    /// This is the first member of the stable allocation; the path stays Rust-private.
    #[derive(Default)]
    "OpenTTDRoadState" ["road_state"] => pub struct OpenTTDRoadState {
        /// @see `RoadVehicleStates`
        pub state: u8,
        pub frame: u8,
        pub blocked_ctr: u16,
        /// Set to `RVSB_DRIVE_SIDE` when overtaking, otherwise 0.
        pub overtaking: u8,
        /// The length of the current overtake attempt.
        pub overtaking_ctr: u8,
        /// Animation counter when the vehicle has crashed. @see `RoadVehIsCrashed`
        pub crashed_ctr: u16,
        pub reverse_ctr: u8,
    }
}

/// Element of the `RoadVehPathCache`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PathElement {
    /// Trackdir for this element.
    pub trackdir: u8,
    /// Tile for this element.
    pub tile: u32,
}
/// The Rust owner of one road vehicle shell: its scalar members and its cached path.
#[repr(C)]
#[derive(Default)]
pub struct State {
    pub fields: OpenTTDRoadState,
    /// Cached path.
    pub(crate) path: Vec<PathElement>,
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_new() -> *mut State {
    Box::into_raw(Box::default())
}
/// # Safety
/// One shell transfers its live owner exactly once, after `PreDestructor`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_destroy(state: *mut State) {
    // SAFETY: As this function's contract.
    unsafe {
        drop(Box::from_raw(state));
    }
}
/// # Safety
/// A live owner, not mutated during this accessor.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_path_size(state: *const State) -> usize {
    // SAFETY: As this function's contract.
    unsafe { (*state).path.len() }
}
/// Write the path element at `index` to the caller-owned `out`.
/// # Safety
/// A live owner, the original vector's index precondition and a writable `out`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_path_get(
    state: *const State,
    index: usize,
    out: *mut PathElement,
) {
    // SAFETY: As this function's contract.
    unsafe { out.write((&(*state).path)[index]) }
}
/// # Safety
/// A live owner and a disjoint call-scoped staging array of `count` elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_path_replace(
    state: *mut State,
    input: *const PathElement,
    count: usize,
) {
    // SAFETY: As this function's contract; no owner borrow spans a C++ call.
    unsafe {
        (*state).path.clear();
        for index in 0..count {
            (*state).path.push(input.add(index).read());
        }
    }
}
/// # Safety
/// Exclusive game-thread access to a live owner.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_path_clear(state: *mut State) {
    // SAFETY: As this function's contract.
    unsafe {
        (*state).path.clear();
    }
}
/// # Safety
/// Exclusive game-thread access to a live owner.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_path_push(state: *mut State, item: PathElement) {
    // SAFETY: As this function's contract.
    unsafe {
        (*state).path.push(item);
    }
}
/// # Safety
/// Exclusive game-thread access to a live owner and the original nonempty-vector precondition.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_path_pop(state: *mut State) {
    // SAFETY: As this function's contract.
    unsafe {
        (*state).path.pop();
    }
}

/// `RoadVehicle::state`.
/// # Safety
/// The serialized native shell owns this live state for the scalar access.
pub(crate) unsafe fn state(s: *const State) -> u8 {
    read_field!(s, fields.state)
}
/// `RoadVehicle::IsInDepot`: reads only `state`.
/// # Safety
/// The serialized native shell owns this live state for the scalar access.
pub(crate) unsafe fn is_in_depot(s: *const State) -> bool {
    unsafe { state(s) == crate::roadveh::RVSB_IN_DEPOT }
}
/// Store `RoadVehicle::state`.
/// # Safety
/// The serialized native shell owns this live state for the scalar access.
pub(crate) unsafe fn set_state(s: *mut State, state: u8) {
    write_field!(s, fields.state, state);
}
/// `RoadVehicle::frame`.
/// # Safety
/// The serialized native shell owns this live state for the scalar access.
pub(crate) unsafe fn frame(s: *const State) -> u8 {
    read_field!(s, fields.frame)
}
/// Store `RoadVehicle::frame`.
/// # Safety
/// The serialized native shell owns this live state for the scalar access.
pub(crate) unsafe fn set_frame(s: *mut State, frame: u8) {
    write_field!(s, fields.frame, frame);
}
/// `RoadVehicle::blocked_ctr`.
/// # Safety
/// The serialized native shell owns this live state for the scalar access.
pub(crate) unsafe fn blocked_ctr(s: *const State) -> u16 {
    read_field!(s, fields.blocked_ctr)
}
/// `RoadVehicle::overtaking`.
/// # Safety
/// The serialized native shell owns this live state for the scalar access.
pub(crate) unsafe fn overtaking(s: *const State) -> u8 {
    read_field!(s, fields.overtaking)
}
/// `RoadVehicle::overtaking_ctr`.
/// # Safety
/// The serialized native shell owns this live state for the scalar access.
pub(crate) unsafe fn overtaking_ctr(s: *const State) -> u8 {
    read_field!(s, fields.overtaking_ctr)
}
/// `RoadVehicle::crashed_ctr`.
/// # Safety
/// The serialized native shell owns this live state for the scalar access.
pub(crate) unsafe fn crashed_ctr(s: *const State) -> u16 {
    read_field!(s, fields.crashed_ctr)
}

/// The services a road vehicle entry calls; the shared tables are borrowed through pointers.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Leaves {
    /// Construct, in the controller's native stack slot, the road command context that
    /// `CanBuildTramTrackOnTile` runs `CmdBuildRoad` in.
    pub build_road_context:
        extern "C" fn(*mut crate::road_cmd::CommandSlot) -> *const crate::road_cmd::CommandContext,
    pub ground: *const GroundServices,
    pub shared: *const Services,
    pub vehicle: *const VehicleServices,
}

/// Count one C++ service call for the road witness; returns the unchanged function pointer.
macro_rules! service {
    ($function:expr) => {
        crate::witness::counted!(before ROAD_SERVICES, $function)
    };
}

/// A road vehicle part: its live record and the live road owner its shell holds.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Part {
    v: VehicleRef,
    s: NonNull<State>,
}
/// `prev` for a part after the front, which `IndividualRoadVehicleController` dereferences
/// without a test (`roadveh_cmd.cpp:1217,1392`). Its callers, `RoadVehController`'s chain walk
/// and `AfterLoadGame`'s frame replay, pass null only with the front engine; every later part
/// receives the part before it.
fn previous_part(prev: Option<Part>) -> Part {
    // SAFETY: Only the non-front branches call this, and the callers above pass each non-front
    // part its predecessor, as the original's unchecked dereference requires.
    unsafe { prev.unwrap_unchecked() }
}
impl Part {
    fn new(v: VehicleRef, s: *mut State) -> Self {
        // SAFETY: Every road vehicle shell owns a non-null road owner.
        Self {
            v,
            s: unsafe { NonNull::new_unchecked(s) },
        }
    }
    /// The road vehicle record `v` with the road owner its record names.
    #[inline]
    fn of(v: VehicleRef) -> Self {
        Self::new(v, v.road())
    }
    /// `RoadVehicle::Next`.
    #[inline]
    fn next(self) -> Option<Self> {
        self.v.next().map(Self::of)
    }
    /// `RoadVehicle::First`.
    #[inline]
    fn first(self) -> Self {
        Self::of(self.v.first())
    }
    /// The ground vehicle owners of this part.
    #[inline]
    fn ground(self) -> crate::ground_vehicle::Owner {
        crate::ground_vehicle::Owner::for_road(self.v.ground(), self.s.as_ptr())
    }
    /// `RoadVehicle::state`, through the shared field accessor.
    fn state(self) -> u8 {
        // SAFETY: A live serialized road owner; one scalar read.
        unsafe { state(self.s.as_ptr()) }
    }
    /// `RoadVehicle::IsInDepot`, over the road owner alone.
    fn is_in_depot(self) -> bool {
        // SAFETY: A live serialized road owner; one scalar read.
        unsafe { is_in_depot(self.s.as_ptr()) }
    }
    fn set_state(self, value: u8) {
        // SAFETY: A live serialized road owner; one scalar store.
        unsafe { set_state(self.s.as_ptr(), value) }
    }
    fn frame(self) -> u8 {
        // SAFETY: A live serialized road owner; one scalar read.
        unsafe { frame(self.s.as_ptr()) }
    }
    fn set_frame(self, value: u8) {
        // SAFETY: A live serialized road owner; one scalar store.
        unsafe { set_frame(self.s.as_ptr(), value) }
    }
    fn blocked_ctr(self) -> u16 {
        // SAFETY: A live serialized road owner; one scalar read.
        unsafe { blocked_ctr(self.s.as_ptr()) }
    }
    fn set_blocked_ctr(self, blocked_ctr: u16) {
        write_field!(self.s.as_ptr(), fields.blocked_ctr, blocked_ctr);
    }
    fn overtaking(self) -> u8 {
        // SAFETY: A live serialized road owner; one scalar read.
        unsafe { overtaking(self.s.as_ptr()) }
    }
    fn set_overtaking(self, overtaking: u8) {
        write_field!(self.s.as_ptr(), fields.overtaking, overtaking);
    }
    fn overtaking_ctr(self) -> u8 {
        // SAFETY: A live serialized road owner; one scalar read.
        unsafe { overtaking_ctr(self.s.as_ptr()) }
    }
    fn set_overtaking_ctr(self, overtaking_ctr: u8) {
        write_field!(self.s.as_ptr(), fields.overtaking_ctr, overtaking_ctr);
    }
    fn crashed_ctr(self) -> u16 {
        // SAFETY: A live serialized road owner; one scalar read.
        unsafe { crashed_ctr(self.s.as_ptr()) }
    }
    fn set_crashed_ctr(self, crashed_ctr: u16) {
        write_field!(self.s.as_ptr(), fields.crashed_ctr, crashed_ctr);
    }
    fn reverse_ctr(self) -> u8 {
        read_field!(self.s.as_ptr(), fields.reverse_ctr)
    }
    fn set_reverse_ctr(self, reverse_ctr: u8) {
        write_field!(self.s.as_ptr(), fields.reverse_ctr, reverse_ctr);
    }
    /// `path.empty()`.
    fn path_empty(self) -> bool {
        // SAFETY: A live owner; the length read ends here.
        unsafe { (*self.s.as_ptr()).path.is_empty() }
    }
    /// `path.back()`, under the original's nonempty precondition.
    fn path_back(self) -> PathElement {
        // SAFETY: A live owner; the copy ends the borrow.
        unsafe { (&(*self.s.as_ptr()).path)[(*self.s.as_ptr()).path.len() - 1] }
    }
    /// `path.clear()`.
    fn path_clear(self) {
        // SAFETY: A live owner on the game thread.
        unsafe { (*self.s.as_ptr()).path.clear() }
    }
    /// `path.pop_back()`.
    fn path_pop_back(self) {
        // SAFETY: A live owner on the game thread.
        unsafe {
            (*self.s.as_ptr()).path.pop();
        }
    }
    /// `path.emplace_back(trackdir, tile)`.
    fn path_emplace_back(self, trackdir: Trackdir, tile: TileIndex) {
        // SAFETY: A live owner on the game thread.
        unsafe {
            (*self.s.as_ptr()).path.push(PathElement {
                trackdir: trackdir.0,
                tile: tile.0,
            });
        }
    }
}

/// A `Station *` from `Station::GetByTile`, carried through the arrival and its
/// triggers. Its `index` is read at each original use.
#[derive(Clone, Copy)]
struct StationHandle {
    st: *mut crate::station::Station,
}
impl StationHandle {
    /// `st->index`, through the shared station index reader.
    fn index(self) -> StationID {
        crate::newgrf_airport::runtime().station_index(self.base())
    }
    /// The station as its `BaseStation`.
    fn base(self) -> *mut crate::station::BaseStation {
        self.st.cast()
    }
}

/// `Station::GetByTile(tile)`.
#[inline]
fn station_get_by_tile(tile: TileIndex) -> StationHandle {
    StationHandle {
        st: (crate::newgrf_airport::runtime().by_tile)(tile.0),
    }
}

/// `IndividualRoadVehicleController`: the road bits a tram needs on its tile to turn
/// around on the reversing trackdir `dir` using the 'big' corner loop. The caller has
/// tested `IsReversingRoadTrackdir(dir)`, so the default is unreachable.
fn tram_turn_around_needed(dir: Trackdir) -> RoadBits {
    match dir {
        Trackdir::TRACKDIR_RVREV_NE => RoadBits::ROAD_SW,
        Trackdir::TRACKDIR_RVREV_SE => RoadBits::ROAD_NW,
        Trackdir::TRACKDIR_RVREV_SW => RoadBits::ROAD_NE,
        Trackdir::TRACKDIR_RVREV_NW => RoadBits::ROAD_SE,
        _ => crate::services_assert::not_reached(),
    }
}

/// `IndividualRoadVehicleController`: the reversing trackdir that shows a tram's turn
/// around the long corner, from the drive data's `x` direction bits. Two bits name
/// every `DiagDirection`, so the default is unreachable.
fn tram_long_corner_trackdir(x: u8) -> Trackdir {
    match DiagDirection(x & 0x3) {
        DiagDirection::DIAGDIR_NW => Trackdir::TRACKDIR_RVREV_SE,
        DiagDirection::DIAGDIR_NE => Trackdir::TRACKDIR_RVREV_SW,
        DiagDirection::DIAGDIR_SE => Trackdir::TRACKDIR_RVREV_NW,
        DiagDirection::DIAGDIR_SW => Trackdir::TRACKDIR_RVREV_NE,
        _ => crate::services_assert::not_reached(),
    }
}

/// `RoadVehFindData`.
struct RoadVehFindData {
    x: i32,
    y: i32,
    veh: VehicleRef,
    best: Option<VehicleRef>,
    best_diff: u32,
    dir: Direction,
}

/// `OvertakeData`.
struct OvertakeData {
    u: VehicleRef,
    v: Part,
    tile: TileIndex,
    trackdir: Trackdir,
}

/// The context one road vehicle entry runs in: the service tables, the map descriptor and
/// the native command slot.
struct Game<'a> {
    l: &'a Leaves,
    vs: &'a VehicleServices,
    gs: &'a GroundServices,
    ss: &'a Services,
    movement: CachedTable<'a>,
    /// The controller's native stack slot for `CanBuildTramTrackOnTile`; null elsewhere.
    road: *mut crate::road_cmd::CommandSlot,
}

impl<'a> Game<'a> {
    /// # Safety
    /// The tables outlive the synchronous entry and every nested entry.
    unsafe fn new(leaves: *const Leaves) -> Self {
        // SAFETY: Immutable static tables supplied by the C++ facade.
        unsafe {
            let l = &*leaves;
            Game {
                l,
                vs: &*l.vehicle,
                gs: &*l.ground,
                ss: &*l.shared,
                movement: CachedTable::audited(&*l.shared),
                road: std::ptr::null_mut(),
            }
        }
    }
    /// The native assertion handler, as a header helper's precondition check.
    fn check(&self) -> impl Fn(bool) + Copy + '_ {
        |ok| crate::movement::native_assert(self.ss, ok)
    }
    /// `assert(condition)`: evaluated only where the native build asserts.
    fn assert(&self, condition: impl FnOnce() -> bool) {
        if self.vs.assertions {
            crate::movement::native_assert(self.ss, condition());
        }
    }
    /// The entry's map descriptor, loaded at its first map use and shared with the movement
    /// table's track queries. Orders, sound, `NewGRF`, movement, crash and viewport callbacks
    /// cannot load or reset the map or yield the game-state mutex; nested entries have their
    /// own `Game`.
    fn world(&self) -> &World<'a> {
        self.movement.world()
    }
    /// The map dimensions of [`Self::world`].
    fn map(&self) -> Map {
        self.world().dimensions
    }
    /// `_settings_game.pf.roadveh_queue`.
    fn roadveh_queue(&self) -> bool {
        // SAFETY: The catalog addresses the process-wide setting; one field-sized load.
        unsafe { self.ss.settings().roadveh_queue.read() }
    }
    /// `v->roadtype`.
    fn roadtype(&self, v: VehicleRef) -> RoadType {
        // SAFETY: A live record and the immutable binding; one field read.
        unsafe { ((*(self.vs.newgrf)()).roadtype)(v.0.as_ptr()) }
    }
    /// `v->compatible_roadtypes`.
    fn compatible_roadtypes(&self, v: VehicleRef) -> RoadTypes {
        RoadTypes((self.movement.get().road_compatible)(v))
    }
    /// `v->index`.
    fn index(&self, v: VehicleRef) -> VehicleID {
        // SAFETY: A live record and the immutable binding.
        VehicleID(unsafe { ((*(self.vs.newgrf)()).id)(v.0.as_ptr()) })
    }
    /// `HasTileAnyRoadType(tile, types)`.
    fn has_tile_any_road_type(&self, tile: TileIndex, types: RoadTypes) -> bool {
        crate::road_map::has_tile_any_road_type(self.world(), tile.0, types.0)
    }
    /// `GetTileTrackStatus(tile, TRANSPORT_ROAD, GetRoadTramType(v->roadtype))`.
    fn get_road_vehicle_track_status(&self, v: VehicleRef, tile: TileIndex) -> TrackStatus {
        let rtt = get_road_tram_type(self.roadtype(v));
        self.movement.get().get_tile_track_status_in_world(
            self.world(),
            tile,
            TransportType::TRANSPORT_ROAD,
            u32::from(rtt.0),
            DiagDirection::INVALID_DIAGDIR,
        )
    }
    /// `IsTileType(tile, type)`.
    fn is_tile_type(&self, tile: TileIndex, kind: TileType) -> bool {
        self.world().is_tile_type(tile, kind)
    }
    /// `IsRoadDepotTile(tile)`.
    fn is_road_depot_tile(&self, tile: TileIndex) -> bool {
        self.world().road_depot(tile.0)
    }
    /// `GetRoadDepotDirection(tile)`.
    fn get_road_depot_direction(&self, tile: TileIndex) -> DiagDirection {
        DiagDirection(self.world().road_depot_dir(tile.0))
    }
    /// `GetTunnelBridgeDirection(tile)`.
    fn get_tunnel_bridge_direction(&self, tile: TileIndex) -> DiagDirection {
        DiagDirection(self.world().tunnel_bridge_dir(tile.0))
    }
    /// The `RoadDriveEntry` row `_road_drive_data[GetRoadTramType(v->roadtype)][index]`,
    /// at `frame`. The arguments are evaluated in the subscripts' order: callers read and
    /// classify `v->roadtype` before computing the row index and the frame.
    fn road_drive_entry(rtt: RoadTramType, index: usize, frame: usize) -> (u8, u8) {
        crate::road_data::entry(rtt == RoadTramType::RTT_TRAM, index, frame)
    }
    /// `_settings_game.vehicle.road_side << RVS_DRIVE_SIDE`.
    fn drive_side(&self) -> usize {
        usize::from(self.ss.settings().road_side()) << RVS_DRIVE_SIDE
    }
    /// `v->UpdatePosition()`.
    fn update_position(&self, v: VehicleRef) {
        // SAFETY: The live shell survives the synchronous index update.
        unsafe { crate::vehicle::occupancy::update_position(self.vs, v, self.map()) }
    }
    /// `v->UpdateInclination(new_tile, update_delta)`.
    fn update_inclination(&self, v: Part, new_tile: bool, update_delta: bool) -> i32 {
        crate::ground_vehicle::update_inclination_owned(
            self.gs.physics,
            &self.movement,
            v.v,
            v.ground(),
            new_tile,
            update_delta,
        )
    }
    /// `VehicleEnterTile(v, tile, x, y)`.
    fn vehicle_enter_tile(
        &self,
        v: VehicleRef,
        tile: TileIndex,
        x: i32,
        y: i32,
    ) -> VehicleEnterTileStates {
        // SAFETY: The same live serialized shell as the original callback; no owner borrow
        // spans it.
        unsafe { self.movement.get().vehicle_enter_tile(v, tile, x, y) }
    }
    /// `v->GetAcceleration()`.
    fn get_acceleration(&self, v: Part) -> i32 {
        crate::ground_vehicle::get_acceleration_owned(self.gs.physics, v.v, v.ground())
    }

    /// Get length of a road vehicle.
    #[inline]
    fn get_road_veh_length(&self, v: VehicleRef) -> u32 {
        let e = service!(self.vs.engine)(v);
        let mut length = VEHICLE_LENGTH;

        let mut veh_len;
        if e.grf_version(self.ss) >= 8 {
            /* Use callback 36 */
            veh_len = service!(self.vs.property)(
                (self.vs.newgrf)(),
                v,
                PROP_ROADVEH_SHORTEN_FACTOR,
                i32::from(crate::newgrf_callbacks::CALLBACK_FAILED),
            ) as u16;
            if veh_len != crate::newgrf_callbacks::CALLBACK_FAILED
                && u32::from(veh_len) >= VEHICLE_LENGTH
            {
                self.vs
                    .unknown_callback_result(e, CBID_VEHICLE_LENGTH, veh_len);
            }
        } else {
            /* Use callback 11 */
            veh_len = service!(self.vs.callback)((self.vs.newgrf)(), v, CBID_VEHICLE_LENGTH);
        }
        if veh_len == crate::newgrf_callbacks::CALLBACK_FAILED {
            veh_len = u16::from(e.road_shorten());
        }
        if veh_len != 0 {
            length -= crate::math::clamp(
                i32::from(veh_len),
                0,
                (VEHICLE_LENGTH - 1) as i32,
                self.check(),
            ) as u32;
        }

        length
    }

    /// Update the cache of a road vehicle. `v` must be the first road vehicle;
    /// `same_length` says whether the length of the vehicles should stay the same.
    #[inline]
    fn road_veh_update_cache(&self, v: Part, same_length: bool) {
        self.assert(|| v.v.vehicle_type() == VehicleType::VEH_ROAD);
        self.assert(|| v.v.is_front_engine());

        // SAFETY: Serialized live records; the invalidation ends before the length reset.
        unsafe {
            crate::newgrf_vehicle::invalidate_newgrf_cache_of_chain(&*(self.vs.newgrf)(), v.v);
        }

        write_field!(v.v.ground_cache(), cached_total_length, 0);

        let mut part = Some(v.v);
        while let Some(u) = part {
            /* Check the v->first cache. */
            self.assert(|| u.first_link() == Some(v.v));

            /* Update the 'first engine' */
            let first_engine = if v.v == u {
                EngineID::INVALID
            } else {
                v.v.engine_type()
            };
            write_field!(u.ground_cache(), first_engine, first_engine.0);

            /* Update the length of the vehicle. */
            let veh_len = self.get_road_veh_length(u);
            /* Verify length hasn't changed. */
            if same_length && veh_len != u32::from(read_field!(u.ground_cache(), cached_veh_length))
            {
                service!(self.vs.length_changed)(u);
            }

            write_field!(u.ground_cache(), cached_veh_length, veh_len as u8);
            let total = read_field!(v.v.ground_cache(), cached_total_length)
                .wrapping_add(u16::from(read_field!(u.ground_cache(), cached_veh_length)));
            write_field!(v.v.ground_cache(), cached_total_length, total);

            /* Update visual effect */
            crate::vehicle_effect::update_visual_effect(u, true);

            /* Update cargo aging period. */
            let cargo_age_period = service!(self.vs.engine)(u).cargo_age();
            let period = service!(self.vs.property)(
                (self.vs.newgrf)(),
                u,
                PROP_ROADVEH_CARGO_AGE_PERIOD,
                i32::from(cargo_age_period),
            );
            u.set_cached_cargo_age_period(period as u16);
            part = u.next();
        }

        let max_speed =
            service!(self.vs.property)((self.vs.newgrf)(), v.v, PROP_ROADVEH_SPEED, 0) as u32;
        v.v.set_cached_max_speed(if max_speed != 0 {
            max_speed.wrapping_mul(4) as u16
        } else {
            service!(self.vs.engine)(v.v).road_speed()
        });
    }

    /// `FindClosestRoadDepot`.
    #[inline]
    fn find_closest_road_depot(&self, v: Part, max_distance: i32) -> FindDepotData {
        if self.is_road_depot_tile(v.v.tile()) {
            return FindDepotData::new(v.v.tile(), 0, false);
        }

        let trackdir = self.get_vehicle_trackdir(v);
        // SAFETY: The live road vehicle; no service reenters it during the search.
        unsafe {
            crate::road_yapf::yapf_road_vehicle_find_nearest_depot(
                self.ss,
                v.v,
                trackdir,
                max_distance,
            )
        }
    }

    /// `RoadVehicle::FindClosestDepot`.
    #[inline]
    fn find_closest_depot(&self, v: Part) -> ClosestDepot {
        let rfdd = self.find_closest_road_depot(v, 0);
        if rfdd.best_length == u32::MAX {
            return ClosestDepot::default();
        }

        ClosestDepot::new(
            rfdd.tile,
            DestinationID(service!(self.ss.depot_index)(rfdd.tile.0)),
            false,
        )
    }

    /// `CmdTurnRoadVeh` after its vehicle, primary and ownership checks: turn a road
    /// vehicle around. Returns false for `CMD_ERROR`.
    #[inline]
    fn cmd_turn_road_veh(&self, v: Part, flags: crate::command_type::DoCommandFlags) -> bool {
        if v.v.vehstatus().any(
            VehStates::from_element(VehState::Stopped)
                .with(VehStates::from_element(VehState::Crashed)),
        ) || v.v.breakdown_ctr() != 0
            || v.overtaking() != 0
            || v.state() == RVSB_WORMHOLE
            || v.is_in_depot()
            || v.v.current_order().is_type(OrderType::OT_LOADING)
        {
            return false;
        }

        if self.world().is_normal_road_tile(v.v.tile().0)
            && self.world().disallowed_road_directions(v.v.tile().0)
                != crate::road_type::DisallowedRoadDirections::DRD_NONE.0
        {
            return false;
        }

        if self.is_tile_type(v.v.tile(), TileType::MP_TUNNELBRIDGE)
            && dir_to_diag_dir(v.v.direction(), self.check())
                == self.get_tunnel_bridge_direction(v.v.tile())
        {
            return false;
        }

        if flags.test(crate::command_type::DoCommandFlag::Execute) {
            v.set_reverse_ctr(180);

            /* Unbunching data is no longer valid. */
            service!(self.vs.reset_depot_unbunching)(v.v);
        }

        true
    }

    /// Delete last vehicle of a chain road vehicles. `v` is the first road vehicle.
    #[inline]
    fn delete_last_road_veh(&self, mut v: Part) {
        let first = v.first();
        let mut u = v;
        while let Some(next) = v.next() {
            u = v;
            v = next;
        }
        // SAFETY: The crashed chain stays live through the unlinking.
        unsafe { crate::vehicle_base::set_next(u.v.0.as_ptr(), std::ptr::null_mut()) };
        v.v.set_last_station_visited(first.v.last_station_visited()); // for PreDestructor

        /* Only leave the road stop when we're really gone. */
        if is_inside_mm(
            usize::from(v.state()),
            usize::from(RVSB_IN_ROAD_STOP),
            usize::from(RVSB_IN_ROAD_STOP_END),
        ) {
            // SAFETY: A live last-part owner; no owner access spans the stop update.
            unsafe { crate::road_stop::leave_vehicle(self.ss, v.v, v.s.as_ptr()) };
        }

        service!(self.vs.destroy)(v.v);
    }

    /// `RoadVehSetRandomDirection`.
    #[inline]
    fn road_veh_set_random_direction(&self, v: VehicleRef) {
        const DELTA: [DirDiff; 4] = [
            DirDiff::DIRDIFF_45LEFT,
            DirDiff::DIRDIFF_SAME,
            DirDiff::DIRDIFF_SAME,
            DirDiff::DIRDIFF_45RIGHT,
        ];

        let mut part = Some(v);
        while let Some(v) = part {
            let r = self.ss.random();

            v.set_direction(change_dir(
                v.direction(),
                DELTA[(r & 3) as usize],
                self.check(),
            ));
            (self.gs.physics.viewport)(v, true, true);
            part = v.next();
        }
    }

    /// Road vehicle chain has crashed. `v` is the first road vehicle; returns whether the
    /// chain still exists.
    #[inline]
    fn road_veh_is_crashed(&self, v: Part) -> bool {
        v.set_crashed_ctr(v.crashed_ctr().wrapping_add(1));
        if v.crashed_ctr() == 2 {
            crate::effect::create_effect_vehicle_rel(
                v.v,
                4,
                4,
                8,
                crate::effectvehicle_func::EffectVehicleType::EV_EXPLOSION_LARGE,
            );
        } else if v.crashed_ctr() <= 45 {
            if (v.v.tick_counter() & 7) == 0 {
                self.road_veh_set_random_direction(v.v);
            }
        } else if v.crashed_ctr() >= 2220 && (v.v.tick_counter() & 0x1F) == 0 {
            let ret = v.next().is_some();
            self.delete_last_road_veh(v);
            return ret;
        }

        true
    }

    /// `RoadVehicle::Crash`.
    #[inline]
    fn crash(&self, v: Part, flooded: bool) -> u32 {
        let mut victims = crate::ground_vehicle::crash(self.gs.physics, v.v, flooded);
        if v.v.is_front_engine() {
            victims = victims.wrapping_add(1); // driver

            /* If we're in a drive through road stop we ought to leave it */
            if is_inside_mm(
                usize::from(v.state()),
                usize::from(RVSB_IN_DT_ROAD_STOP),
                usize::from(RVSB_IN_DT_ROAD_STOP_END),
            ) {
                // SAFETY: A live front owner; no owner access spans the stop update.
                unsafe { crate::road_stop::leave_vehicle(self.ss, v.v, v.s.as_ptr()) };
            }
        }
        v.set_crashed_ctr(if flooded { 2000 } else { 1 }); // max 2220, disappear pretty fast when flooded
        victims
    }

    /// `RoadVehCrash`.
    #[inline]
    fn road_veh_crash(&self, v: Part) {
        let victims = self.crash(v, false);

        let crashed = || {
            ScriptEvent::vehicle_crashed(
                self.index(v.v),
                v.v.tile(),
                CrashReason::CRASH_RV_LEVEL_CROSSING,
                victims,
                v.v.owner(),
            )
        };
        crate::ai::new_event(v.v.owner(), &crashed());
        crate::game::new_event(&crashed());

        let headline = if victims == 1 {
            get_encoded_string(STR_NEWS_ROAD_VEHICLE_CRASH_DRIVER, &[])
        } else {
            get_encoded_string(STR_NEWS_ROAD_VEHICLE_CRASH, &[Integer(u64::from(victims))])
        };
        let newstype = if v.v.owner().0 == service!(self.ss.local_company)() {
            NewsType::Accident
        } else {
            NewsType::AccidentOther
        };

        add_tile_news_item(&headline, newstype, v.v.tile(), StationID::INVALID);

        crate::station::spatial::modify_station_rating_around(
            self.ss,
            v.v.tile(),
            v.v.owner(),
            -160,
            22,
        );
        if service!(self.ss.disaster_sound)() {
            service!(self.vs.play_sound)(v.v, SoundID::from(SoundFx::SND_12_EXPLOSION).0);
        }
    }

    /// `RoadVehCheckTrainCrash`.
    #[expect(
        clippy::inline_always,
        reason = "Reject noncrossing parts in the controller frame."
    )]
    #[inline(always)]
    fn road_veh_check_train_crash(&self, v: Part) -> bool {
        let mut part = Some(v);
        while let Some(u) = part {
            'next: {
                if u.state() == RVSB_WORMHOLE {
                    break 'next;
                }

                let tile = u.v.tile();

                if !self.world().level_crossing_tile(tile.0) {
                    break 'next;
                }

                if self.vs.any_near(v.v.x_pos(), v.v.y_pos(), 4, |t| {
                    t.vehicle_type() == VehicleType::VEH_TRAIN
                        && t.z_pos().wrapping_sub(u.v.z_pos()).wrapping_abs() <= 6
                }) {
                    self.road_veh_crash(v);
                    return true;
                }
            }
            part = u.next();
        }

        false
    }

    /// `StartRoadVehSound`.
    #[inline]
    fn start_road_veh_sound(&self, v: VehicleRef) {
        if !service!(self.vs.play_vehicle_sound)(v, VSE_START) {
            let mut s = SoundID(service!(self.vs.engine)(v).road_sound());
            if s == SoundID::from(SoundFx::SND_19_DEPARTURE_OLD_RV_1) && (v.tick_counter() & 3) == 0
            {
                s = SoundID::from(SoundFx::SND_1A_DEPARTURE_OLD_RV_2);
            }
            service!(self.vs.play_sound)(v, s.0);
        }
    }

    /// `FindClosestBlockingRoadVeh`.
    #[inline]
    fn find_closest_blocking_road_veh(&self, v: VehicleRef, rvf: &mut RoadVehFindData) {
        const DIST_X: [i8; 8] = [-4, -8, -4, -1, 4, 8, 4, 1];
        const DIST_Y: [i8; 8] = [-4, -1, 4, 8, 4, 1, -4, -8];
        crate::witness::hit(crate::witness::ROAD_VISITORS);

        let x_diff = v.x_pos().wrapping_sub(rvf.x);
        let y_diff = v.y_pos().wrapping_sub(rvf.y);

        /* Not a close Road vehicle when it's not a road vehicle, in the depot, or ourself. */
        if v.vehicle_type() != VehicleType::VEH_ROAD
            || Part::of(v).is_in_depot()
            || rvf.veh.first() == v.first()
        {
            return;
        }

        /* Not close when at a different height or when going in a different direction. */
        if v.z_pos().wrapping_sub(rvf.veh.z_pos()).wrapping_abs() >= 6 || v.direction() != rvf.dir {
            return;
        }

        /* We 'return' the closest vehicle, in distance and then VehicleID as tie-breaker. */
        let diff = x_diff.wrapping_abs().wrapping_add(y_diff.wrapping_abs()) as u32;
        if diff > rvf.best_diff
            || (diff == rvf.best_diff
                && rvf
                    .best
                    .is_some_and(|best| self.index(v) > self.index(best)))
        {
            return;
        }

        let is_close_on_axis = |dist: i32, diff: i32| {
            if dist < 0 {
                return diff > dist && diff <= 0;
            }
            diff < dist && diff >= 0
        };

        if is_close_on_axis(i32::from(DIST_X[usize::from(v.direction().0)]), x_diff)
            && is_close_on_axis(i32::from(DIST_Y[usize::from(v.direction().0)]), y_diff)
        {
            rvf.best = Some(v);
            rvf.best_diff = diff;
        }
    }

    /// `RoadVehFindCloseTo`.
    #[inline]
    fn road_veh_find_close_to(
        &self,
        v: Part,
        x: i32,
        y: i32,
        dir: Direction,
        update_blocked_ctr: bool,
    ) -> Option<VehicleRef> {
        let front = v.first();

        if front.reverse_ctr() != 0 {
            return None;
        }

        let mut rvf = RoadVehFindData {
            x,
            y,
            dir,
            veh: v.v,
            best: None,
            best_diff: u32::MAX,
        };

        if front.state() == RVSB_WORMHOLE {
            self.vs.for_each_on_tile(v.v.tile().0, |u| {
                self.find_closest_blocking_road_veh(u, &mut rvf);
            });
            self.vs
                .for_each_on_tile(service!(self.ss.other_tunnel_end)(v.v.tile().0), |u| {
                    self.find_closest_blocking_road_veh(u, &mut rvf);
                });
        } else {
            self.vs.for_each_near(x, y, 8, |u| {
                self.find_closest_blocking_road_veh(u, &mut rvf);
            });
        }

        /* This code protects a roadvehicle from being blocked for ever
         * If more than 1480 / 74 days a road vehicle is blocked, it will
         * drive just through it. The ultimate backup-code of TTD.
         * It can be disabled. */
        if rvf.best_diff == u32::MAX {
            front.set_blocked_ctr(0);
            return None;
        }

        if update_blocked_ctr {
            front.set_blocked_ctr(front.blocked_ctr().wrapping_add(1));
            if front.blocked_ctr() > 1480 {
                return None;
            }
        }

        /* RoadVehicle::From */
        let best = rvf.best;
        self.assert(|| best.is_some_and(|best| best.vehicle_type() == VehicleType::VEH_ROAD));
        best
    }

    /// `st->had_vehicle_of_type`, through the station's native field.
    #[inline]
    fn had_vehicle_of_type(&self, st: StationHandle) -> StationHadVehicleOfType {
        // SAFETY: A live station resolved in this serialized entry; one byte-sized load.
        StationHadVehicleOfType(unsafe {
            *service!(self.ss.station_had_vehicle_of_type_field)(st.st)
        })
    }

    /// `st->had_vehicle_of_type |= type`, through the station's native field.
    #[inline]
    fn add_had_vehicle_of_type(&self, st: StationHandle, kind: StationHadVehicleOfType) {
        // SAFETY: As `had_vehicle_of_type`; one byte-sized read-modify-write.
        unsafe {
            *service!(self.ss.station_had_vehicle_of_type_field)(st.st) |= kind.0;
        }
    }

    /// A road vehicle arrives at a station. If it is the first time, create a news item.
    #[inline]
    fn road_veh_arrives_at(&self, v: Part, st: StationHandle) {
        if crate::road_stop::is_bus(self.ss, v.v) {
            /* Check if station was ever visited before */
            if self.had_vehicle_of_type(st).0 & StationHadVehicleOfType::HVOT_BUS.0 == 0 {
                self.add_had_vehicle_of_type(st, StationHadVehicleOfType::HVOT_BUS);
                add_vehicle_news_item(
                    &get_encoded_string(
                        if road_type_is_road(self.roadtype(v.v)) {
                            STR_NEWS_FIRST_BUS_ARRIVAL
                        } else {
                            STR_NEWS_FIRST_PASSENGER_TRAM_ARRIVAL
                        },
                        &[Integer(u64::from(st.index().0))],
                    ),
                    if v.v.owner().0 == service!(self.ss.local_company)() {
                        NewsType::ArrivalCompany
                    } else {
                        NewsType::ArrivalOther
                    },
                    self.index(v.v),
                    st.index(),
                );
                crate::ai::new_event(
                    v.v.owner(),
                    &ScriptEvent::station_first_vehicle(st.index(), self.index(v.v)),
                );
                crate::game::new_event(&ScriptEvent::station_first_vehicle(
                    st.index(),
                    self.index(v.v),
                ));
            }
        } else {
            /* Check if station was ever visited before */
            if self.had_vehicle_of_type(st).0 & StationHadVehicleOfType::HVOT_TRUCK.0 == 0 {
                self.add_had_vehicle_of_type(st, StationHadVehicleOfType::HVOT_TRUCK);
                add_vehicle_news_item(
                    &get_encoded_string(
                        if road_type_is_road(self.roadtype(v.v)) {
                            STR_NEWS_FIRST_TRUCK_ARRIVAL
                        } else {
                            STR_NEWS_FIRST_CARGO_TRAM_ARRIVAL
                        },
                        &[Integer(u64::from(st.index().0))],
                    ),
                    if v.v.owner().0 == service!(self.ss.local_company)() {
                        NewsType::ArrivalCompany
                    } else {
                        NewsType::ArrivalOther
                    },
                    self.index(v.v),
                    st.index(),
                );
                crate::ai::new_event(
                    v.v.owner(),
                    &ScriptEvent::station_first_vehicle(st.index(), self.index(v.v)),
                );
                crate::game::new_event(&ScriptEvent::station_first_vehicle(
                    st.index(),
                    self.index(v.v),
                ));
            }
        }
    }

    /// This function looks at the vehicle and updates its speed (`cur_speed` and
    /// `subspeed`) variables. Furthermore, it returns the distance that the vehicle can drive
    /// this tick. `Vehicle::GetAdvanceDistance()` determines the distance to drive before
    /// moving a step on the map.
    #[inline]
    fn update_speed(&self, v: Part) -> i32 {
        match self.ss.settings().roadveh_acceleration_model() {
            AccelerationModel::AM_ORIGINAL => crate::ground_vehicle::do_update_speed(
                v.v,
                if v.overtaking() != 0 { 512 } else { 256 },
                0,
                get_current_max_speed(self.ss, || self.world(), v),
            ) as i32,

            AccelerationModel::AM_REALISTIC => {
                let accel = self
                    .get_acceleration(v)
                    .wrapping_add(if v.overtaking() != 0 { 256 } else { 0 });
                let min_speed =
                    if crate::ground_vehicle::get_acceleration_status_owned(v.v, v.ground())
                        == crate::ground_vehicle::AccelStatus::AS_BRAKE
                    {
                        0
                    } else {
                        4
                    };
                crate::ground_vehicle::do_update_speed(
                    v.v,
                    accel as u32,
                    min_speed,
                    get_current_max_speed(self.ss, || self.world(), v),
                ) as i32
            }
            _ => crate::services_assert::not_reached(),
        }
    }

    /// `RoadVehGetNewDirection`.
    #[inline]
    fn road_veh_get_new_direction(v: VehicleRef, x: i32, y: i32) -> Direction {
        const ROADVEH_NEW_DIR: [Direction; 11] = [
            Direction::DIR_N,
            Direction::DIR_NW,
            Direction::DIR_W,
            Direction::INVALID_DIR,
            Direction::DIR_NE,
            Direction::DIR_N,
            Direction::DIR_SW,
            Direction::INVALID_DIR,
            Direction::DIR_E,
            Direction::DIR_SE,
            Direction::DIR_S,
        ];

        let x = x.wrapping_sub(v.x_pos()).wrapping_add(1);
        let y = y.wrapping_sub(v.y_pos()).wrapping_add(1);

        if x as u32 > 2 || y as u32 > 2 {
            return v.direction();
        }
        ROADVEH_NEW_DIR[(y * 4 + x) as usize]
    }

    /// `RoadVehGetSlidingDirection`.
    #[inline]
    fn road_veh_get_sliding_direction(&self, v: VehicleRef, x: i32, y: i32) -> Direction {
        let new_dir = Self::road_veh_get_new_direction(v, x, y);
        let old_dir = v.direction();

        if new_dir == old_dir {
            return old_dir;
        }
        let delta = if dir_difference(new_dir, old_dir, self.check()) > DirDiff::DIRDIFF_REVERSE {
            DirDiff::DIRDIFF_45LEFT
        } else {
            DirDiff::DIRDIFF_45RIGHT
        };
        change_dir(old_dir, delta, self.check())
    }

    /// Check if overtaking is possible on a piece of track. Returns true if we have to
    /// abort overtaking.
    #[inline]
    fn check_road_blocked_for_overtaking(&self, od: &OvertakeData) -> bool {
        if !self.has_tile_any_road_type(od.tile, self.compatible_roadtypes(od.v.v)) {
            return true;
        }
        let ts = self.get_road_vehicle_track_status(od.v.v, od.tile);
        let trackdirbits = TrackdirBits(track_status_to_trackdir_bits(ts));
        let red_signals = track_status_to_red_signals(ts); // barred level crossing
        let trackbits = TrackBits(trackdir_bits_to_track_bits(trackdirbits.0));

        /* Track does not continue along overtaking direction || track has junction || levelcrossing is barred */
        if !crate::bitmath_func::has_bit(trackdirbits.0, od.trackdir.0)
            || (trackbits.0 & !TrackBits::TRACK_BIT_CROSS.0) != 0
            || red_signals != TrackdirBits::TRACKDIR_BIT_NONE
        {
            return true;
        }

        /* Are there more vehicles on the tile except the two vehicles involved in overtaking */
        self.vs.any_on_tile(od.tile.0, |v| {
            v.vehicle_type() == VehicleType::VEH_ROAD && v.first() == v && v != od.u && v != od.v.v
        })
    }

    /// `RoadVehCheckOvertake`.
    #[inline]
    fn road_veh_check_overtake(&self, v: Part, u: Part) {
        /* Trams can't overtake other trams */
        if road_type_is_tram(self.roadtype(v.v)) {
            return;
        }

        /* Don't overtake in stations */
        if self.is_tile_type(v.v.tile(), TileType::MP_STATION)
            || self.is_tile_type(u.v.tile(), TileType::MP_STATION)
        {
            return;
        }

        /* For now, articulated road vehicles can't overtake anything. */
        if v.v.has_articulated_part() {
            return;
        }

        /* Vehicles are not driving in same direction || direction is not a diagonal direction */
        if v.v.direction() != u.v.direction() || (v.v.direction().0 & 1) == 0 {
            return;
        }

        /* Check if vehicle is in a road stop, depot, tunnel or bridge or not on a straight road */
        if v.state() >= RVSB_IN_ROAD_STOP
            || !is_straight_road_trackdir(Trackdir(v.state() & RVSB_TRACKDIR_MASK), self.check())
        {
            return;
        }

        /* Can't overtake a vehicle that is moving faster than us. If the vehicle in front is
         * accelerating, take the maximum speed for the comparison, else the current speed.
         * Original acceleration always accelerates, so always use the maximum speed. */
        let u_speed = if self.ss.settings().roadveh_acceleration_model()
            == AccelerationModel::AM_ORIGINAL
            || self.get_acceleration(u) > 0
        {
            get_current_max_speed(self.ss, || self.world(), u)
        } else {
            i32::from(u.v.cur_speed())
        };
        if u_speed >= get_current_max_speed(self.ss, || self.world(), v)
            && !u.v.vehstatus().test(VehState::Stopped)
            && u.v.cur_speed() != 0
        {
            return;
        }

        let trackdir = Trackdir(diag_dir_to_diag_trackdir(
            dir_to_diag_dir(v.v.direction(), self.check()).0,
            self.check(),
        ));

        /* Are the current and the next tile suitable for overtaking?
         *  - Does the track continue along od.trackdir
         *  - No junctions
         *  - No barred levelcrossing
         *  - No other vehicles in the way
         */
        let mut od = OvertakeData {
            u: u.v,
            v,
            tile: v.v.tile(),
            trackdir,
        };
        if self.check_road_blocked_for_overtaking(&od) {
            return;
        }

        od.tile = v.v.tile()
            + self.map().tile_offs_by_diag_dir(
                dir_to_diag_dir(v.v.direction(), self.check()),
                self.check(),
            );
        if self.check_road_blocked_for_overtaking(&od) {
            return;
        }

        /* When the vehicle in front of us is stopped we may only take
         * half the time to pass it than when the vehicle is moving. */
        v.set_overtaking_ctr(
            if od.u.cur_speed() == 0 || od.u.vehstatus().test(VehState::Stopped) {
                RV_OVERTAKE_TIMEOUT / 2
            } else {
                0
            },
        );
        v.set_overtaking(RVSB_DRIVE_SIDE);
    }

    /// `RoadZPosAffectSpeed`.
    #[inline]
    fn road_z_pos_affect_speed(&self, v: Part, old_z: i32) {
        if old_z == v.v.z_pos()
            || self.ss.settings().roadveh_acceleration_model() != AccelerationModel::AM_ORIGINAL
        {
            return;
        }

        if old_z < v.v.z_pos() {
            v.v.set_cur_speed((i32::from(v.v.cur_speed()) * 232 / 256) as u16); // slow down by ~10%
        } else {
            let spd = v.v.cur_speed().wrapping_add(2);
            if spd <= read_field!(v.v.ground_cache(), cached_max_track_speed) {
                v.v.set_cur_speed(spd);
            }
        }
    }

    /// `PickRandomBit`.
    #[inline]
    fn pick_random_bit(&self, mut bits: u32) -> u32 {
        let mut i = 0;
        let mut num = self.ss.random_range(bits.count_ones());

        while (bits & 1) == 0 || {
            num = num.wrapping_sub(1);
            num as i32 >= 0
        } {
            bits >>= 1;
            i += 1;
        }
        i
    }

    /// Returns direction to for a road vehicle to take or `INVALID_TRACKDIR` if the
    /// direction is currently blocked. `v` is the vehicle to do the pathfinding for, `tile`
    /// the tile where to start the pathfinding and `enterdir` the direction the vehicle
    /// enters the tile from.
    #[inline]
    fn road_find_path_to_dest(
        &self,
        v: Part,
        tile: TileIndex,
        enterdir: DiagDirection,
    ) -> Trackdir {
        let mut path_found = true;

        let ts = self.get_road_vehicle_track_status(v.v, tile);
        let red_signals = track_status_to_red_signals(ts); // crossing
        let mut trackdirs = TrackdirBits(track_status_to_trackdir_bits(ts));

        if self.is_tile_type(tile, TileType::MP_ROAD) {
            if self.world().is_road_depot(tile)
                && (!self.world().is_tile_owner(tile, v.v.owner())
                    || self.get_road_depot_direction(tile) == enterdir)
            {
                /* Road depot owned by another company or with the wrong orientation */
                trackdirs = TrackdirBits::TRACKDIR_BIT_NONE;
            }
        } else if self.is_tile_type(tile, TileType::MP_STATION)
            && self.world().is_bay_road_stop_tile(tile)
        {
            /* Standard road stop (drive-through stops are treated as normal road) */

            if !self.world().is_tile_owner(tile, v.v.owner())
                || self.world().get_bay_road_stop_dir(tile) == enterdir
                || v.v.has_articulated_part()
            {
                /* different station owner or wrong orientation or the vehicle has articulated parts */
                trackdirs = TrackdirBits::TRACKDIR_BIT_NONE;
            } else {
                /* Our station */
                let rstype = if crate::road_stop::is_bus(self.ss, v.v) {
                    RoadStopType::Bus
                } else {
                    RoadStopType::Truck
                };

                if self.world().get_road_stop_type(tile) != rstype {
                    /* Wrong station type */
                    trackdirs = TrackdirBits::TRACKDIR_BIT_NONE;
                } else {
                    /* Proper station type, check if there is free loading bay */
                    if !self.roadveh_queue()
                        && self.world().is_bay_road_stop_tile(tile)
                        && !crate::road_stop::has_free_bay(
                            crate::road_stop::get_by_tile(self.ss, tile, rstype).1,
                        )
                    {
                        /* Station is full and RV queuing is off */
                        trackdirs = TrackdirBits::TRACKDIR_BIT_NONE;
                    }
                }
            }
        }
        /* The above lookups should be moved to GetTileTrackStatus in the
         * future, but that requires more changes to the pathfinder and other
         * stuff, probably even more arguments to GTTS.
         */

        /* Remove tracks unreachable from the enter dir */
        trackdirs.0 &= diagdir_reaches_trackdirs(enterdir.0);
        let best_track = 'found_best_track: {
            if trackdirs == TrackdirBits::TRACKDIR_BIT_NONE {
                /* If vehicle expected a path, it no longer exists, so invalidate it. */
                if !v.path_empty() {
                    v.path_clear();
                }
                /* No reachable tracks, so we'll reverse */
                break 'found_best_track ROAD_REVERSE_TABLE[usize::from(enterdir.0)];
            }

            if v.reverse_ctr() != 0 {
                let mut reverse = true;
                if road_type_is_tram(self.roadtype(v.v)) {
                    /* Trams may only reverse on a tile if it contains at least the straight
                     * trackbits or when it is a valid turning tile (i.e. one roadbit) */
                    let rb = RoadBits(service!(self.ss.road_bits)(
                        tile.0,
                        u8::from(RoadTramType::RTT_TRAM.0),
                        false,
                    ));
                    let straight = crate::road_func::axis_to_road_bits(
                        diag_dir_to_axis(enterdir, self.check()),
                        self.check(),
                    );
                    reverse = ((rb.0 & straight.0) == straight.0)
                        || (rb == crate::road_func::diag_dir_to_road_bits(enterdir, self.check()));
                }
                if reverse {
                    v.set_reverse_ctr(0);
                    if v.v.tile() != tile {
                        break 'found_best_track ROAD_REVERSE_TABLE[usize::from(enterdir.0)];
                    }
                }
            }

            let desttile = v.v.dest_tile();
            if desttile == TileIndex(0) {
                /* We've got no destination, pick a random track */
                break 'found_best_track Trackdir(
                    self.pick_random_bit(u32::from(trackdirs.0)) as u8
                );
            }

            /* Only one track to choose between? */
            if crate::bitmath_func::kill_first_bit_u16(trackdirs.0)
                == TrackdirBits::TRACKDIR_BIT_NONE.0
            {
                if !v.path_empty() && v.path_back().tile == tile.0 {
                    /* Vehicle expected a choice here, invalidate its path. */
                    v.path_clear();
                }
                break 'found_best_track Trackdir(crate::bitmath_func::find_first_bit(u64::from(
                    trackdirs.0,
                )));
            }

            /* Attempt to follow cached path. */
            if !v.path_empty() {
                if v.path_back().tile != tile.0 {
                    /* Vehicle didn't expect a choice here, invalidate its path. */
                    v.path_clear();
                } else {
                    let trackdir = Trackdir(v.path_back().trackdir);

                    if crate::bitmath_func::has_bit(trackdirs.0, trackdir.0) {
                        v.path_pop_back();
                        break 'found_best_track trackdir;
                    }

                    /* Vehicle expected a choice which is no longer available. */
                    v.path_clear();
                }
            }

            // SAFETY: The live road vehicle and its road owner; no service reenters either.
            let best_track = unsafe {
                crate::road_yapf::yapf_road_vehicle_choose_track(
                    self.ss,
                    v.v,
                    v.s.as_ptr(),
                    tile,
                    enterdir,
                    trackdirs,
                    &mut path_found,
                )
            };

            service!(self.vs.pathfinding_result)(v.v, path_found);

            best_track
        };

        if crate::bitmath_func::has_bit(red_signals.0, best_track.0) {
            return Trackdir::INVALID_TRACKDIR;
        }

        best_track
    }

    /// `RoadVehLeaveDepot`.
    #[inline]
    fn road_veh_leave_depot(&self, v: Part, first: bool) -> bool {
        /* Don't leave unless v and following wagons are in the depot. */
        let mut part = Some(v);
        while let Some(u) = part {
            if u.state() != RVSB_IN_DEPOT || u.v.tile() != v.v.tile() {
                return false;
            }
            part = u.next();
        }

        let dir = self.get_road_depot_direction(v.v.tile());
        v.v.set_direction(diag_dir_to_dir(dir, self.check()));

        let tdir = Trackdir(diag_dir_to_diag_trackdir(dir.0, self.check()));
        let rdp = Self::road_drive_entry(
            get_road_tram_type(self.roadtype(v.v)),
            self.drive_side() + usize::from(tdir.0),
            usize::from(RVC_DEPOT_START_FRAME),
        );

        let x = (self.map().tile_x(v.v.tile()) * TILE_SIZE + u32::from(rdp.0 & 0xF)) as i32;
        let y = (self.map().tile_y(v.v.tile()) * TILE_SIZE + u32::from(rdp.1 & 0xF)) as i32;

        if first {
            /* We are leaving a depot, but have to go to the exact same one; re-enter */
            if v.v.current_order().is_type(OrderType::OT_GOTO_DEPOT)
                && v.v.tile() == v.v.dest_tile()
            {
                crate::vehicle::visits::enter_depot(v.v, self.vs.visits());
                return true;
            }

            if self
                .road_veh_find_close_to(v, x, y, v.v.direction(), false)
                .is_some()
            {
                return true;
            }

            // SAFETY: A live front shell; no owner borrow spans the servicing.
            unsafe { self.vs.maintain(v.v, maintenance::service_in_depot) };
            service!(self.vs.leave_unbunching_depot)(v.v);

            self.start_road_veh_sound(v.v);

            /* Vehicle is about to leave a depot */
            v.v.set_cur_speed(0);
        }

        v.v.set_vehstatus(v.v.vehstatus().reset(VehState::Hidden));
        v.set_state(tdir.0);
        v.set_frame(RVC_DEPOT_START_FRAME);

        v.v.set_x_pos(x);
        v.v.set_y_pos(y);
        self.update_position(v.v);
        self.update_inclination(v, true, true);

        (self.movement.get().depot_invalidate)(v.v);

        true
    }

    /// `FollowPreviousRoadVehicle`.
    #[inline]
    fn follow_previous_road_vehicle(
        &self,
        v: Part,
        prev: Part,
        tile: TileIndex,
        entry_dir: DiagDirection,
        already_reversed: bool,
    ) -> Trackdir {
        if prev.v.tile() == v.v.tile() && !already_reversed {
            /* If the previous vehicle is on the same tile as this vehicle is
             * then it must have reversed. */
            return ROAD_REVERSE_TABLE[usize::from(entry_dir.0)];
        }

        let prev_state = prev.state();
        let mut dir;

        if prev_state == RVSB_WORMHOLE || prev_state == RVSB_IN_DEPOT {
            let mut diag_dir = DiagDirection::INVALID_DIAGDIR;

            if self.is_tile_type(tile, TileType::MP_TUNNELBRIDGE) {
                diag_dir = self.get_tunnel_bridge_direction(tile);
            } else if self.is_road_depot_tile(tile) {
                diag_dir = reverse_diag_dir(self.get_road_depot_direction(tile), self.check());
            }

            if diag_dir == DiagDirection::INVALID_DIAGDIR {
                return Trackdir::INVALID_TRACKDIR;
            }
            dir = Trackdir(diag_dir_to_diag_trackdir(diag_dir.0, self.check()));
        } else if already_reversed
            && (prev.v.tile() != tile
                || (prev_state < Trackdir::TRACKDIR_END.0
                    && is_reversing_road_trackdir(prev_state, self.check())))
        {
            /*
             * The vehicle has reversed, but did not go straight back.
             * It immediately turn onto another tile. This means that
             * the roadstate of the previous vehicle cannot be used
             * as the direction we have to go with this vehicle.
             *
             * Next table is build in the following way:
             *  - first row for when the vehicle in front went to the northern or
             *    western tile, second for southern and eastern.
             *  - columns represent the entry direction.
             *  - cell values are determined by the Trackdir one has to take from
             *    the entry dir (column) to the tile in north or south by only
             *    going over the trackdirs used for turning 90 degrees, i.e.
             *    TRACKDIR_{UPPER,RIGHT,LOWER,LEFT}_{N,E,S,W}.
             */
            let north = if prev.v.tile() != tile {
                prev.v.tile() < tile
            } else {
                prev_state == Trackdir::TRACKDIR_RVREV_NW.0
                    || prev_state == Trackdir::TRACKDIR_RVREV_NE.0
            };
            const REVERSED_TURN_LOOKUP: [[Trackdir; 4]; 2] = [
                [
                    Trackdir::TRACKDIR_UPPER_W,
                    Trackdir::TRACKDIR_RIGHT_N,
                    Trackdir::TRACKDIR_LEFT_N,
                    Trackdir::TRACKDIR_UPPER_E,
                ],
                [
                    Trackdir::TRACKDIR_RIGHT_S,
                    Trackdir::TRACKDIR_LOWER_W,
                    Trackdir::TRACKDIR_LOWER_E,
                    Trackdir::TRACKDIR_LEFT_S,
                ],
            ];
            dir = REVERSED_TURN_LOOKUP[usize::from(!north)]
                [usize::from(reverse_diag_dir(entry_dir, self.check()).0)];
        } else if crate::bitmath_func::has_bit(prev_state, RVS_IN_DT_ROAD_STOP) {
            dir = Trackdir(prev_state & RVSB_ROAD_STOP_TRACKDIR_MASK);
        } else if prev_state < Trackdir::TRACKDIR_END.0 {
            dir = Trackdir(prev_state);
        } else {
            return Trackdir::INVALID_TRACKDIR;
        }

        /* Do some sanity checking. */
        const REQUIRED_ROADBITS: [RoadBits; 8] = [
            RoadBits::ROAD_X,
            RoadBits::ROAD_Y,
            RoadBits(RoadBits::ROAD_NW.0 | RoadBits::ROAD_NE.0),
            RoadBits(RoadBits::ROAD_SW.0 | RoadBits::ROAD_SE.0),
            RoadBits(RoadBits::ROAD_NW.0 | RoadBits::ROAD_SW.0),
            RoadBits(RoadBits::ROAD_NE.0 | RoadBits::ROAD_SE.0),
            RoadBits::ROAD_X,
            RoadBits::ROAD_Y,
        ];
        let required = REQUIRED_ROADBITS[usize::from(dir.0 & 0x07)];

        if (required.0
            & RoadBits(service!(self.ss.road_bits)(
                tile.0,
                u8::from(get_road_tram_type(self.roadtype(v.v)).0),
                true,
            ))
            .0)
            == RoadBits::ROAD_NONE.0
        {
            dir = Trackdir::INVALID_TRACKDIR;
        }

        dir
    }

    /// Can a tram track build without destruction on the given tile? `c` is the company
    /// that would be building the tram tracks, `t` the tile to build on, `rt` the tram type
    /// to build and `r` the road bits needed.
    #[inline]
    fn can_build_tram_track_on_tile(
        &self,
        c: Owner,
        t: TileIndex,
        rt: RoadType,
        r: RoadBits,
    ) -> bool {
        let road = service!(self.l.build_road_context)(self.road);
        // SAFETY: The leaf constructed a live native stack context and results in the slot.
        unsafe {
            let context = &*road;
            let town = &*(*context.services).town;
            /* The 'current' company is not necessarily the owner of the vehicle. */
            let cur_company = (town.exchange_company)(u32::from(c.0));

            context.do_build(
                crate::command_type::DoCommandFlags::from_element(
                    crate::command_type::DoCommandFlag::NoWater,
                ),
                t,
                r,
                rt,
                crate::road_type::DisallowedRoadDirections::DRD_NONE,
                crate::town_type::TownID::INVALID,
            );

            (town.exchange_company)(cur_company);
            let succeeded = context.succeeded();
            crate::witness::hit(if succeeded {
                crate::witness::ROAD_TRAM_BUILD
            } else {
                crate::witness::ROAD_TRAM_REFUSED
            });
            succeeded
        }
    }

    /// `IndividualRoadVehicleController`.
    #[inline]
    fn individual_road_vehicle_controller(&self, v: Part, prev: Option<Part>) -> bool {
        if v.overtaking() != 0 {
            if self.is_tile_type(v.v.tile(), TileType::MP_STATION) {
                /* Force us to be not overtaking! */
                v.set_overtaking(0);
            } else {
                v.set_overtaking_ctr(v.overtaking_ctr().wrapping_add(1));
                if v.overtaking_ctr() >= RV_OVERTAKE_TIMEOUT {
                    /* If overtaking just aborts at a random moment, we can have a out-of-bound problem,
                     *  if the vehicle started a corner. To protect that, only allow an abort of
                     *  overtake if we are on straight roads */
                    if v.state() < RVSB_IN_ROAD_STOP
                        && is_straight_road_trackdir(Trackdir(v.state()), self.check())
                    {
                        v.set_overtaking(0);
                    }
                }
            }
        }

        /* If this vehicle is in a depot and we've reached this point it must be
         * one of the articulated parts. It will stay in the depot until activated
         * by the previous vehicle in the chain when it gets to the right place. */
        if v.is_in_depot() {
            return true;
        }

        if v.state() == RVSB_WORMHOLE {
            /* Vehicle is entering a depot or is on a bridge or in a tunnel */
            let gp = service!(self.vs.new_position)(v.v);

            if v.v.is_front_engine() {
                if let Some(u) = self.road_veh_find_close_to(v, gp.x, gp.y, v.v.direction(), true) {
                    v.v.set_cur_speed(u.first().cur_speed());
                    return false;
                }
            }

            if self.is_tile_type(TileIndex(gp.tile), TileType::MP_TUNNELBRIDGE)
                && self
                    .vehicle_enter_tile(v.v, TileIndex(gp.tile), gp.x, gp.y)
                    .test(VehicleEnterTileState::EnteredWormhole)
            {
                /* Vehicle has just entered a bridge or tunnel */
                v.v.set_x_pos(gp.x);
                v.v.set_y_pos(gp.y);
                self.update_position(v.v);
                self.update_inclination(v, true, true);
                return true;
            }

            v.v.set_x_pos(gp.x);
            v.v.set_y_pos(gp.y);
            self.update_position(v.v);
            if !v.v.vehstatus().test(VehState::Hidden) {
                service!(self.vs.base_viewport)(v.v);
            }
            return true;
        }

        /* Get move position data for next frame.
         * For a drive-through road stop use 'straight road' move data.
         * In this case v->state is masked to give the road stop entry direction. */
        let rd = Self::road_drive_entry(
            get_road_tram_type(self.roadtype(v.v)),
            (usize::from(
                if crate::bitmath_func::has_bit(v.state(), RVS_IN_DT_ROAD_STOP) {
                    v.state() & RVSB_ROAD_STOP_TRACKDIR_MASK
                } else {
                    v.state()
                },
            ) + self.drive_side())
                ^ usize::from(v.overtaking()),
            usize::from(v.frame()) + 1,
        );

        if rd.0 & RDE_NEXT_TILE != 0 {
            let mut tile = v.v.tile()
                + self
                    .map()
                    .tile_offs_by_diag_dir(DiagDirection(rd.0 & 3), self.check());
            let mut dir;

            if v.v.is_front_engine() {
                /* If this is the front engine, look for the right path. */
                if self.has_tile_any_road_type(tile, self.compatible_roadtypes(v.v)) {
                    dir = self.road_find_path_to_dest(v, tile, DiagDirection(rd.0 & 3));
                } else {
                    dir = ROAD_REVERSE_TABLE[usize::from(rd.0 & 3)];
                }
            } else {
                dir = self.follow_previous_road_vehicle(
                    v,
                    previous_part(prev),
                    tile,
                    DiagDirection(rd.0 & 3),
                    false,
                );
            }

            if dir == Trackdir::INVALID_TRACKDIR {
                if !v.v.is_front_engine() {
                    crate::error_func::fatal_error(self.ss, "Disconnecting road vehicle.");
                }
                v.v.set_cur_speed(0);
                return false;
            }

            loop {
                // again:
                let mut start_frame = RVC_DEFAULT_START_FRAME;
                if is_reversing_road_trackdir(dir.0, self.check()) {
                    /* When turning around we can't be overtaking. */
                    v.set_overtaking(0);

                    /* Turning around */
                    if road_type_is_tram(self.roadtype(v.v)) {
                        /* Determine the road bits the tram needs to be able to turn around
                         * using the 'big' corner loop. */
                        let needed = tram_turn_around_needed(dir);
                        if v.v
                            .previous()
                            .is_some_and(|previous| previous.tile() == tile)
                            || (v.v.is_front_engine()
                                && self.world().is_normal_road_tile(tile.0)
                                && !self.world().has_road_works(tile.0)
                                && self
                                    .has_tile_any_road_type(tile, self.compatible_roadtypes(v.v))
                                && (needed.0
                                    & self.world().road_bits(tile.0, RoadTramType::RTT_TRAM.0))
                                    != RoadBits::ROAD_NONE.0)
                        {
                            /*
                             * Taking the 'big' corner for trams only happens when:
                             * - The previous vehicle in this (articulated) tram chain is
                             *   already on the 'next' tile, we just follow them regardless of
                             *   anything. When it is NOT on the 'next' tile, the tram started
                             *   doing a reversing turn when the piece of tram track on the next
                             *   tile did not exist yet. Do not use the big tram loop as that is
                             *   going to cause the tram to split up.
                             * - Or the front of the tram can drive over the next tile.
                             */
                        } else if !v.v.is_front_engine()
                            || !self.can_build_tram_track_on_tile(
                                v.v.owner(),
                                tile,
                                self.roadtype(v.v),
                                needed,
                            )
                            || ((!needed.0
                                & RoadBits(service!(self.ss.road_bits)(
                                    v.v.tile().0,
                                    u8::from(RoadTramType::RTT_TRAM.0),
                                    false,
                                ))
                                .0)
                                == RoadBits::ROAD_NONE.0)
                        {
                            /*
                             * Taking the 'small' corner for trams only happens when:
                             * - We are not the from vehicle of an articulated tram.
                             * - Or when the company cannot build on the next tile.
                             *
                             * The 'small' corner means that the vehicle is on the end of a
                             * tram track and needs to start turning there. To do this properly
                             * the tram needs to start at an offset in the tram turning 'code'
                             * for 'big' corners. It furthermore does not go to the next tile,
                             * so that needs to be fixed too.
                             */
                            tile = v.v.tile();
                            start_frame = RVC_TURN_AROUND_START_FRAME_SHORT_TRAM;
                        } else {
                            /* The company can build on the next tile, so wait till they do. */
                            v.v.set_cur_speed(0);
                            return false;
                        }
                    } else if self.world().is_normal_road_tile(v.v.tile().0)
                        && self.world().disallowed_road_directions(v.v.tile().0)
                            != crate::road_type::DisallowedRoadDirections::DRD_NONE.0
                    {
                        v.v.set_cur_speed(0);
                        return false;
                    } else {
                        tile = v.v.tile();
                    }
                }

                /* Get position data for first frame on the new tile */
                let rdp = Self::road_drive_entry(
                    get_road_tram_type(self.roadtype(v.v)),
                    (usize::from(dir.0) + self.drive_side()) ^ usize::from(v.overtaking()),
                    usize::from(start_frame),
                );

                let x = (self.map().tile_x(tile) * TILE_SIZE + u32::from(rdp.0)) as i32;
                let y = (self.map().tile_y(tile) * TILE_SIZE + u32::from(rdp.1)) as i32;

                let new_dir = self.road_veh_get_sliding_direction(v.v, x, y);
                if v.v.is_front_engine() {
                    if let Some(u) = self.road_veh_find_close_to(v, x, y, new_dir, true) {
                        v.v.set_cur_speed(u.first().cur_speed());
                        /* We might be blocked, prevent pathfinding rerun as we already know where we are heading to. */
                        v.path_emplace_back(dir, tile);
                        return false;
                    }
                }

                let vets = self.vehicle_enter_tile(v.v, tile, x, y);
                if vets.test(VehicleEnterTileState::CannotEnter) {
                    if !self.is_tile_type(tile, TileType::MP_TUNNELBRIDGE) {
                        v.v.set_cur_speed(0);
                        return false;
                    }
                    /* Try an about turn to re-enter the previous tile */
                    dir = ROAD_REVERSE_TABLE[usize::from(rd.0 & 3)];
                    continue;
                }

                if is_inside_mm(
                    usize::from(v.state()),
                    usize::from(RVSB_IN_ROAD_STOP),
                    usize::from(RVSB_IN_DT_ROAD_STOP_END),
                ) && self.is_tile_type(v.v.tile(), TileType::MP_STATION)
                {
                    if is_reversing_road_trackdir(dir.0, self.check())
                        && is_inside_mm(
                            usize::from(v.state()),
                            usize::from(RVSB_IN_ROAD_STOP),
                            usize::from(RVSB_IN_ROAD_STOP_END),
                        )
                    {
                        /* New direction is trying to turn vehicle around.
                         * We can't turn at the exit of a road stop so wait.*/
                        v.v.set_cur_speed(0);
                        return false;
                    }

                    /* If we are a drive through road stop and the next tile is of
                     * the same road stop and the next tile isn't this one (i.e. we
                     * are not reversing), then keep the reservation and state.
                     * This way we will not be shortly unregister from the road
                     * stop. It also makes it possible to load when on the edge of
                     * two road stops; otherwise you could get vehicles that should
                     * be loading but are not actually loading. */
                    if self.world().is_drive_through_stop_tile(v.v.tile())
                        && crate::road_stop::is_drive_through_road_stop_continuation(
                            self.ss,
                            v.v.tile(),
                            tile,
                        )
                        && v.v.tile() != tile
                    {
                        /* So, keep 'our' state */
                        dir = Trackdir(v.state());
                    } else if self.world().is_station_road_stop_tile(v.v.tile()) {
                        /* We're not continuing our drive through road stop, so leave. */
                        // SAFETY: A live front owner; no owner access spans the stop update.
                        unsafe { crate::road_stop::leave_vehicle(self.ss, v.v, v.s.as_ptr()) };
                    }
                }

                if !vets.test(VehicleEnterTileState::EnteredWormhole) {
                    let old_tile = v.v.tile();

                    v.v.set_tile(tile);
                    v.set_state(dir.0);
                    v.set_frame(start_frame);
                    let rtt = get_road_tram_type(self.roadtype(v.v));
                    if self.world().road_type(old_tile.0, rtt.0)
                        != self.world().road_type(tile.0, rtt.0)
                    {
                        if v.v.is_front_engine() {
                            self.road_veh_update_cache(v, false);
                        }
                        crate::ground_vehicle::cargo_changed(self.gs.physics, v.v.first());
                    }
                }
                if new_dir != v.v.direction() {
                    v.v.set_direction(new_dir);
                    if self.ss.settings().roadveh_acceleration_model()
                        == AccelerationModel::AM_ORIGINAL
                    {
                        v.v.set_cur_speed(v.v.cur_speed() - (v.v.cur_speed() >> 2));
                    }
                }
                v.v.set_x_pos(x);
                v.v.set_y_pos(y);
                self.update_position(v.v);
                self.road_z_pos_affect_speed(v, self.update_inclination(v, true, true));
                return true;
            }
        }

        if rd.0 & RDE_TURNED != 0 {
            /* Vehicle has finished turning around, it will now head back onto the same tile */
            let dir;
            let mut turn_around_start_frame = RVC_TURN_AROUND_START_FRAME;

            if road_type_is_tram(self.roadtype(v.v))
                && !self.is_road_depot_tile(v.v.tile())
                && crate::bitmath_func::has_exactly_one_bit(
                    RoadBits(service!(self.ss.road_bits)(
                        v.v.tile().0,
                        u8::from(RoadTramType::RTT_TRAM.0),
                        true,
                    ))
                    .0,
                )
            {
                /*
                 * The tram is turning around with one tram 'roadbit'. This means that
                 * it is using the 'big' corner 'drive data'. However, to support the
                 * trams to take a small corner, there is a 'turned' marker in the middle
                 * of the turning 'drive data'. When the tram took the long corner, we
                 * will still use the 'big' corner drive data, but we advance it one
                 * frame. We furthermore set the driving direction so the turning is
                 * going to be properly shown.
                 */
                turn_around_start_frame = RVC_START_FRAME_AFTER_LONG_TRAM;
                dir = tram_long_corner_trackdir(rd.0);
            } else if v.v.is_front_engine() {
                /* If this is the front engine, look for the right path. */
                dir = self.road_find_path_to_dest(v, v.v.tile(), DiagDirection(rd.0 & 3));
            } else {
                dir = self.follow_previous_road_vehicle(
                    v,
                    previous_part(prev),
                    v.v.tile(),
                    DiagDirection(rd.0 & 3),
                    true,
                );
            }

            if dir == Trackdir::INVALID_TRACKDIR {
                v.v.set_cur_speed(0);
                return false;
            }

            let rdp = Self::road_drive_entry(
                get_road_tram_type(self.roadtype(v.v)),
                self.drive_side() + usize::from(dir.0),
                usize::from(turn_around_start_frame),
            );

            let x = (self.map().tile_x(v.v.tile()) * TILE_SIZE + u32::from(rdp.0)) as i32;
            let y = (self.map().tile_y(v.v.tile()) * TILE_SIZE + u32::from(rdp.1)) as i32;

            let new_dir = self.road_veh_get_sliding_direction(v.v, x, y);
            if v.v.is_front_engine() {
                if let Some(u) = self.road_veh_find_close_to(v, x, y, new_dir, true) {
                    v.v.set_cur_speed(u.first().cur_speed());
                    /* We might be blocked, prevent pathfinding rerun as we already know where we are heading to. */
                    v.path_emplace_back(dir, v.v.tile());
                    return false;
                }
            }

            let vets = self.vehicle_enter_tile(v.v, v.v.tile(), x, y);
            if vets.test(VehicleEnterTileState::CannotEnter) {
                v.v.set_cur_speed(0);
                return false;
            }

            v.set_state(dir.0);
            v.set_frame(turn_around_start_frame);

            if new_dir != v.v.direction() {
                v.v.set_direction(new_dir);
                if self.ss.settings().roadveh_acceleration_model() == AccelerationModel::AM_ORIGINAL
                {
                    v.v.set_cur_speed(v.v.cur_speed() - (v.v.cur_speed() >> 2));
                }
            }

            v.v.set_x_pos(x);
            v.v.set_y_pos(y);
            self.update_position(v.v);
            self.road_z_pos_affect_speed(v, self.update_inclination(v, true, true));
            return true;
        }

        /* This vehicle is not in a wormhole and it hasn't entered a new tile. If
         * it's on a depot tile, check if it's time to activate the next vehicle in
         * the chain yet. */
        if v.v.next().is_some() && self.is_road_depot_tile(v.v.tile()) {
            if u32::from(v.frame())
                == u32::from(read_field!(v.v.ground_cache(), cached_veh_length))
                    + u32::from(RVC_DEPOT_START_FRAME)
            {
                if let Some(next) = v.next() {
                    self.road_veh_leave_depot(next, false);
                }
            }
        }

        /* Calculate new position for the vehicle */
        let x = (v.v.x_pos() & !15) + i32::from(rd.0 & 15);
        let y = (v.v.y_pos() & !15) + i32::from(rd.1 & 15);

        let new_dir = self.road_veh_get_sliding_direction(v.v, x, y);

        if v.v.is_front_engine()
            && !is_inside_mm(
                usize::from(v.state()),
                usize::from(RVSB_IN_ROAD_STOP),
                usize::from(RVSB_IN_ROAD_STOP_END),
            )
        {
            /* Vehicle is not in a road stop.
             * Check for another vehicle to overtake */
            if let Some(u) = self.road_veh_find_close_to(v, x, y, new_dir, true) {
                let u = Part::of(u.first());
                /* There is a vehicle in front overtake it if possible */
                if v.overtaking() == 0 {
                    self.road_veh_check_overtake(v, u);
                }
                if v.overtaking() == 0 {
                    v.v.set_cur_speed(u.v.cur_speed());
                }

                /* In case an RV is stopped in a road stop, why not try to load? */
                if v.v.cur_speed() == 0
                    && is_inside_mm(
                        usize::from(v.state()),
                        usize::from(RVSB_IN_DT_ROAD_STOP),
                        usize::from(RVSB_IN_DT_ROAD_STOP_END),
                    )
                    && crate::movement::should_stop_at_station(
                        self.ss,
                        v.v,
                        self.world().get_station_index(v.v.tile()),
                    )
                    && v.v.owner() == self.world().owner(v.v.tile())
                    && !v.v.current_order().is_type(OrderType::OT_LEAVESTATION)
                    && self.world().get_road_stop_type(v.v.tile())
                        == (if crate::road_stop::is_bus(self.ss, v.v) {
                            RoadStopType::Bus
                        } else {
                            RoadStopType::Truck
                        })
                {
                    let st = station_get_by_tile(v.v.tile());
                    v.v.set_last_station_visited(st.index());
                    self.road_veh_arrives_at(v, st);
                    crate::vehicle::visits::begin_loading(v.v, self.vs.visits());
                    crate::animated_tile::road_event(
                        st.base(),
                        v.v.tile().0,
                        StationAnimationTrigger::VehicleArrives.0,
                        StationRandomTrigger::VehicleArrives.0,
                    );
                }
                return false;
            }
        }

        let old_dir = v.v.direction();
        if new_dir != old_dir {
            v.v.set_direction(new_dir);
            if self.ss.settings().roadveh_acceleration_model() == AccelerationModel::AM_ORIGINAL {
                v.v.set_cur_speed(v.v.cur_speed() - (v.v.cur_speed() >> 2));
            }

            /* Delay the vehicle in curves by making it require one additional frame per turning direction (two in total).
             * A vehicle has to spend at least 9 frames on a tile, so the following articulated part can follow.
             * (The following part may only be one tile behind, and the front part is moved before the following ones.)
             * The short (inner) curve has 8 frames, this elongates it to 10. */
            (self.gs.physics.viewport)(v.v, true, true);
            return true;
        }

        /* If the vehicle is in a normal road stop and the frame equals the stop frame OR
         * if the vehicle is in a drive-through road stop and this is the destination station
         * and it's the correct type of stop (bus or truck) and the frame equals the stop frame...
         * (the station test and stop type test ensure that other vehicles, using the road stop as
         * a through route, do not stop) */
        if v.v.is_front_engine()
            && ((is_inside_mm(
                usize::from(v.state()),
                usize::from(RVSB_IN_ROAD_STOP),
                usize::from(RVSB_IN_ROAD_STOP_END),
            ) && crate::road_data::_ROAD_STOP_STOP_FRAME
                [usize::from(v.state() - RVSB_IN_ROAD_STOP) + self.drive_side()]
                == v.frame())
                || (is_inside_mm(
                    usize::from(v.state()),
                    usize::from(RVSB_IN_DT_ROAD_STOP),
                    usize::from(RVSB_IN_DT_ROAD_STOP_END),
                ) && crate::movement::should_stop_at_station(
                    self.ss,
                    v.v,
                    self.world().get_station_index(v.v.tile()),
                ) && v.v.owner() == self.world().owner(v.v.tile())
                    && self.world().get_road_stop_type(v.v.tile())
                        == (if crate::road_stop::is_bus(self.ss, v.v) {
                            RoadStopType::Bus
                        } else {
                            RoadStopType::Truck
                        })
                    && v.frame() == RVC_DRIVE_THROUGH_STOP_FRAME))
        {
            let rs = crate::road_stop::at(self.ss, v.v.tile());
            let st = station_get_by_tile(v.v.tile());

            /* Vehicle is at the stop position (at a bay) in a road stop.
             * Note, if vehicle is loading/unloading it has already been handled,
             * so if we get here the vehicle has just arrived or is just ready to leave. */
            if !crate::bitmath_func::has_bit(v.state(), RVS_ENTERED_STOP) {
                /* Vehicle has arrived at a bay in a road stop */

                if self.world().is_drive_through_stop_tile(v.v.tile()) {
                    let next_tile =
                        self.map()
                            .tile_add_by_dir(v.v.tile(), v.v.direction(), self.check());

                    /* Check if next inline bay is free and has compatible road. */
                    if crate::road_stop::is_drive_through_road_stop_continuation(
                        self.ss,
                        v.v.tile(),
                        next_tile,
                    ) && self.has_tile_any_road_type(next_tile, self.compatible_roadtypes(v.v))
                    {
                        v.set_frame(v.frame().wrapping_add(1));
                        v.v.set_x_pos(x);
                        v.v.set_y_pos(y);
                        self.update_position(v.v);
                        self.road_z_pos_affect_speed(v, self.update_inclination(v, true, false));
                        return true;
                    }
                }

                crate::road_stop::set_entrance_busy(rs, false);
                let mut state = v.state();
                crate::bitmath_func::set_bit8(&mut state, RVS_ENTERED_STOP);
                v.set_state(state);

                v.v.set_last_station_visited(st.index());

                if self.world().is_drive_through_stop_tile(v.v.tile())
                    || (v.v.current_order().is_type(OrderType::OT_GOTO_STATION)
                        && v.v.current_order().destination() == DestinationID(st.index().0))
                {
                    self.road_veh_arrives_at(v, st);
                    crate::vehicle::visits::begin_loading(v.v, self.vs.visits());
                    crate::animated_tile::road_event(
                        st.base(),
                        v.v.tile().0,
                        StationAnimationTrigger::VehicleArrives.0,
                        StationRandomTrigger::VehicleArrives.0,
                    );
                    return false;
                }
            } else {
                /* Vehicle is ready to leave a bay in a road stop */
                if crate::road_stop::is_entrance_busy(rs) {
                    /* Road stop entrance is busy, so wait as there is nowhere else to go */
                    v.v.set_cur_speed(0);
                    return false;
                }
                if v.v.current_order().is_type(OrderType::OT_LEAVESTATION) {
                    crate::orders::free(v.v);
                }
            }

            if self.world().is_bay_road_stop_tile(v.v.tile()) {
                crate::road_stop::set_entrance_busy(rs, true);
            }

            self.start_road_veh_sound(v.v);
            service!(self.vs.start_stop_dirty)(v.v);
        }

        /* Check tile position conditions - i.e. stop position in depot,
         * entry onto bridge or into tunnel */
        let vets = self.vehicle_enter_tile(v.v, v.v.tile(), x, y);
        if vets.test(VehicleEnterTileState::CannotEnter) {
            v.v.set_cur_speed(0);
            return false;
        }

        if v.v.current_order().is_type(OrderType::OT_LEAVESTATION)
            && self.world().is_drive_through_stop_tile(v.v.tile())
        {
            crate::orders::free(v.v);
        }

        /* Move to next frame unless vehicle arrived at a stop position
         * in a depot or entered a tunnel/bridge */
        if !vets.test(VehicleEnterTileState::EnteredWormhole) {
            v.set_frame(v.frame().wrapping_add(1));
        }
        v.v.set_x_pos(x);
        v.v.set_y_pos(y);
        self.update_position(v.v);
        self.road_z_pos_affect_speed(v, self.update_inclination(v, false, true));
        true
    }

    /// `RoadVehController`.
    #[inline]
    fn road_veh_controller(&self, v: Part) -> bool {
        /* decrease counters */
        v.v.set_current_order_time(v.v.current_order_time().wrapping_add(1));
        if v.reverse_ctr() != 0 {
            v.set_reverse_ctr(v.reverse_ctr() - 1);
        }

        /* handle crashed */
        if v.v.vehstatus().test(VehState::Crashed) || self.road_veh_check_train_crash(v) {
            return self.road_veh_is_crashed(v);
        }

        /* road vehicle has broken down? */
        // SAFETY: A live front shell; no owner borrow spans the breakdown services.
        let broken = if crate::witness::enabled() {
            unsafe { maintenance::handle_breakdown::<true>(v.v, self.vs) }
        } else {
            unsafe { maintenance::handle_breakdown::<false>(v.v, self.vs) }
        };
        if broken.broken {
            return true;
        }
        if v.v.vehstatus().test(VehState::Stopped) {
            crate::ground_vehicle::set_last_speed_owned(self.gs.physics, v.v, v.ground());
            return true;
        }

        let orders = self.vs.orders();
        crate::orders::process_orders(v.v, orders);
        self.vs.handle_loading(v.v, false, orders);

        if v.v.current_order().is_type(OrderType::OT_LOADING) {
            return true;
        }

        if v.is_in_depot() {
            /* Check if we should wait here for unbunching. */
            if service!(self.vs.waiting_for_unbunching)(v.v) {
                return true;
            }
            if self.road_veh_leave_depot(v, true) {
                return true;
            }
        }

        /* Vehicle::ShowVisualEffect */
        if crate::vehicle_effect::show_visual_effect_allowed(v.v) {
            /* Use the speed as limited by underground and orders. */
            let max_speed = get_current_max_speed(self.ss, || self.world(), v) as u32;
            crate::vehicle_effect::show_visual_effect(v.v, max_speed);
        }

        /* Check how far the vehicle needs to proceed */
        let mut j = self.update_speed(v);

        let mut adv_spd = crate::vehicle::advance_distance(v.v.direction().0);
        let mut blocked = false;
        while j >= adv_spd {
            j -= adv_spd;

            let mut u = Some(v);
            let mut prev = None;
            while let Some(part) = u {
                if !self.individual_road_vehicle_controller(part, prev) {
                    blocked = true;
                    break;
                }
                prev = Some(part);
                u = part.next();
            }
            if blocked {
                break;
            }

            /* Determine distance to next map position */
            adv_spd = crate::vehicle::advance_distance(v.v.direction().0);

            /* Test for a collision, but only if another movement will occur. */
            if j >= adv_spd && self.road_veh_check_train_crash(v) {
                break;
            }
        }

        crate::ground_vehicle::set_last_speed_owned(self.gs.physics, v.v, v.ground());

        let mut part = Some(v.v);
        while let Some(u) = part {
            if !u.vehstatus().test(VehState::Hidden) {
                (self.gs.physics.viewport)(u, false, false);
            }
            part = u.next();
        }

        /* If movement is blocked, set 'progress' to its maximum, so the roadvehicle does
         * not accelerate again before it can actually move. I.e. make sure it tries to advance again
         * on next tick to discover whether it is still blocked. */
        if v.v.progress() == 0 {
            v.v.set_progress(if blocked { adv_spd - 1 } else { j } as u8);
        }

        true
    }

    /// `RoadVehicle::GetRunningCost`.
    #[inline]
    fn get_running_cost(&self, v: Part) -> i64 {
        // SAFETY: The vehicle table names the immutable process-lifetime NewGRF table.
        let capabilities = unsafe { (*(self.vs.newgrf)()).capabilities };
        let e = service!(self.vs.engine)(v.v);
        if e.road_price() == crate::economy_type::INVALID_PRICE {
            return 0;
        }

        let cost_factor = service!(self.vs.property)(
            (self.vs.newgrf)(),
            v.v,
            PROP_ROADVEH_RUNNING_COST_FACTOR,
            i32::from(e.road_running()),
        ) as u32;
        if cost_factor == 0 {
            return 0;
        }

        crate::engine_capabilities::price(
            capabilities,
            e.0.as_ptr(),
            u32::from(e.road_price()),
            cost_factor,
            0,
        )
    }

    /// `RoadVehicle::Tick`.
    #[inline]
    fn tick(&self, v: Part) -> bool {
        v.v.set_tick_counter(v.v.tick_counter().wrapping_add(1));

        if v.v.is_front_engine() {
            if !v.v.vehstatus().test(VehState::Stopped) {
                v.v.set_running_ticks(v.v.running_ticks().wrapping_add(1));
            }
            return self.road_veh_controller(v);
        }

        true
    }

    /// `CheckIfRoadVehNeedsService`.
    #[inline]
    fn check_if_road_veh_needs_service(&self, v: Part) {
        // SAFETY: Base maintenance names the shared static command and company tables.
        let company = unsafe { &*((*(*(*self.vs.maintenance).transaction).commands).company)() };
        let defaults = ((company.settings)().company_vehicle_defaults)(v.v.owner());
        /* If we already got a slot at a stop, use that FIRST, and go to a depot later */
        if read_field!(defaults, servint_roadveh) == 0
            // SAFETY: A live front shell; no owner borrow spans the maintenance services.
            || !unsafe { self.vs.maintain(v.v, maintenance::needs_automatic_servicing) }
        {
            return;
        }
        if crate::ground_vehicle::is_chain_in_depot(self.gs.physics, v.v) {
            // SAFETY: As above.
            unsafe { self.vs.maintain(v.v, maintenance::service_in_depot) };
            return;
        }

        // SAFETY: The catalog addresses the process-wide setting; one field-sized load.
        let max_penalty = unsafe { self.ss.settings().maximum_go_to_depot_penalty.read() };

        let rfdd = self.find_closest_road_depot(v, max_penalty as i32);
        /* Only go to the depot if it is not too far out of our way. */
        if rfdd.best_length == u32::MAX || rfdd.best_length > max_penalty {
            if v.v.current_order().is_type(OrderType::OT_GOTO_DEPOT) {
                /* If we were already heading for a depot but it has
                 * suddenly moved farther away, we continue our normal
                 * schedule? */
                // SAFETY: The current order belongs to this live serialized record.
                unsafe { crate::orders::make_dummy(v.v.current_order_pointer()) };
                service!(self.vs.start_stop_dirty)(v.v);
            }
            return;
        }

        let depot = service!(self.ss.depot_index)(rfdd.tile.0);

        if v.v.current_order().is_type(OrderType::OT_GOTO_DEPOT)
            && v.v
                .current_order()
                .non_stop()
                .test(OrderNonStopFlag::NoIntermediate)
            && !crate::random_func::chance16(self.ss, 1, 20)
        {
            return;
        }

        let mut gv_flags = read_field!(v.v.ground(), flags);
        crate::bitmath_func::set_bit16(
            &mut gv_flags,
            GroundVehicleFlags::GVF_SUPPRESS_IMPLICIT_ORDERS.0,
        );
        write_field!(v.v.ground(), flags, gv_flags);
        // SAFETY: The current order belongs to this live serialized record.
        unsafe {
            crate::orders::make_go_to_depot(
                v.v.current_order_pointer(),
                DestinationID(depot),
                OrderDepotTypeFlags::from_element(OrderDepotTypeFlag::Service),
                OrderNonStopFlags::from_element(OrderNonStopFlag::NoIntermediate),
                OrderDepotActionFlags::EMPTY,
                crate::cargo_type::CargoType::CARGO_NO_REFIT,
            );
        }
        set_dest_tile(v, rfdd.tile);
        service!(self.vs.start_stop_dirty)(v.v);
    }

    /// Calendar day handler.
    #[inline]
    fn on_new_calendar_day(&self, v: Part) {
        if !v.v.is_front_engine() {
            return;
        }
        // SAFETY: A live front shell; no owner borrow spans the ageing services.
        unsafe { self.vs.maintain(v.v, maintenance::age_vehicle) };
    }

    /// Economy day handler.
    #[inline]
    fn on_new_economy_day(&self, v: Part) {
        if !v.v.is_front_engine() {
            return;
        }
        // SAFETY: A live front shell; no owner borrow spans the maintenance services.
        unsafe { self.vs.maintain(v.v, maintenance::economy_age_vehicle) };

        v.v.set_day_counter(v.v.day_counter().wrapping_add(1));
        if (v.v.day_counter() & 7) == 0 {
            // SAFETY: As above.
            unsafe { self.vs.maintain(v.v, maintenance::decrease_vehicle_value) };
        }
        if v.blocked_ctr() == 0 {
            // SAFETY: As above.
            unsafe { self.vs.maintain(v.v, maintenance::check_vehicle_breakdown) };
        }

        self.check_if_road_veh_needs_service(v);

        crate::orders::check_orders(v.v, self.vs.orders());

        if v.v.running_ticks() == 0 {
            return;
        }

        let cost = self
            .get_running_cost(v)
            .saturating_mul(i64::from(v.v.running_ticks()))
            / YEAR_TICKS;

        service!(self.vs.write_profit)(v.v, service!(self.vs.profit)(v.v).saturating_sub(cost));
        v.v.set_running_ticks(0);

        // SAFETY: Base maintenance names the shared static command and company tables.
        let company = unsafe { &*((*(*(*self.vs.maintenance).transaction).commands).company)() };
        crate::company::subtract_money_from_company_fract(
            company.finance,
            v.v.owner(),
            cost,
            crate::economy_type::ExpensesType::EXPENSES_ROADVEH_RUN,
        );

        // SAFETY: The static maintenance table outlives the call.
        (unsafe { &*self.vs.maintenance }.details_dirty)(v.v);
        crate::window_func::set_window_classes_dirty(
            crate::window_type::WindowClass::WC_ROADVEH_LIST,
        );
    }

    /// `RoadVehicle::GetVehicleTrackdir`.
    #[inline]
    fn get_vehicle_trackdir(&self, v: Part) -> Trackdir {
        if v.v.vehstatus().test(VehState::Crashed) {
            return Trackdir::INVALID_TRACKDIR;
        }

        if v.is_in_depot() {
            /* We'll assume the road vehicle is facing outwards */
            return Trackdir(diag_dir_to_diag_trackdir(
                self.get_road_depot_direction(v.v.tile()).0,
                self.check(),
            ));
        }

        if self.world().is_bay_road_stop_tile(v.v.tile()) {
            /* We'll assume the road vehicle is facing outwards */
            return Trackdir(diag_dir_to_diag_trackdir(
                self.world().get_bay_road_stop_dir(v.v.tile()).0,
                self.check(),
            )); // Road vehicle in a station
        }

        /* Drive through road stops / wormholes (tunnels) */
        if v.state() > RVSB_TRACKDIR_MASK {
            return Trackdir(diag_dir_to_diag_trackdir(
                dir_to_diag_dir(v.v.direction(), self.check()).0,
                self.check(),
            ));
        }

        /* If vehicle's state is a valid track direction (vehicle is not turning around) return it,
         * otherwise transform it into a valid track direction */
        Trackdir(if is_reversing_road_trackdir(v.state(), self.check()) {
            v.state() - 6
        } else {
            v.state()
        })
    }
}

/// Calculates the maximum speed of the vehicle under its current conditions.
#[expect(
    clippy::inline_always,
    reason = "Specialize the existing loop at each speed observation."
)]
#[inline(always)]
fn get_current_max_speed<'w, 'a: 'w>(
    ss: &Services,
    world: impl Fn() -> &'w World<'a>,
    v: Part,
) -> i32 {
    let cached_max_track_speed =
        || i32::from(read_field!(v.v.ground_cache(), cached_max_track_speed));
    let mut max_speed = cached_max_track_speed();

    /* Limit speed to 50% while reversing, 75% in curves. */
    let mut part = Some(v);
    while let Some(u) = part {
        if ss.settings().roadveh_acceleration_model() == AccelerationModel::AM_REALISTIC {
            if v.state() <= RVSB_TRACKDIR_MASK
                && is_reversing_road_trackdir(v.state(), |ok| {
                    crate::movement::native_assert(ss, ok);
                })
            {
                max_speed = cached_max_track_speed() / 2;
                break;
            } else if (u.v.direction().0 & 1) == 0 {
                max_speed = cached_max_track_speed() * 3 / 4;
            }
        }

        /* Vehicle is on the middle part of a bridge. */
        if u.state() == RVSB_WORMHOLE && !u.v.vehstatus().test(VehState::Hidden) {
            let bridge_type = world().bridge_type(u.v.tile().0);
            let speed = service!(crate::movement::table(ss).tunnel_bridge().speed)(bridge_type);
            max_speed = max_speed.min(i32::from(speed) * 2);
        }
        part = u.next();
    }

    max_speed.min(i32::from(v.v.current_order().max_speed()) * 2)
}

/// `RoadVehicle::GetCurrentMaxSpeed` of the road vehicle record `v`.
pub(crate) fn current_max_speed(ss: &Services, v: VehicleRef) -> i32 {
    let world = std::cell::OnceCell::new();
    get_current_max_speed(ss, || world.get_or_init(|| World::new(ss)), Part::of(v))
}

/// `RoadVehicle::SetDestTile`.
fn set_dest_tile(v: Part, tile: TileIndex) {
    if tile == v.v.dest_tile() {
        return;
    }
    v.path_clear();
    v.v.set_dest_tile(tile);
}

/// Construction removal clears only drive-through stop state bits. The raw field access ends
/// before any callback can reenter this owner.
/// # Safety
/// `rv` is live and serialized by the game-state mutex.
pub(crate) unsafe fn clear_removed_drive_stop(rv: *mut State) {
    // SAFETY: As this function's contract.
    unsafe {
        if crate::bitmath_func::has_bit(state(rv), RVS_IN_DT_ROAD_STOP) {
            set_state(rv, state(rv) & RVSB_ROAD_STOP_TRACKDIR_MASK);
        }
    }
}

/// `RoadVehUpdateCache`.
/// # Safety
/// A live serialized front road vehicle and immutable tables for the call.
pub(crate) unsafe fn road_veh_update_cache(leaves: &Leaves, v: VehicleRef, same_length: bool) {
    // SAFETY: As this function's contract.
    unsafe { Game::new(leaves) }.road_veh_update_cache(Part::of(v), same_length);
}

macro_rules! entry {
    ($leaves:expr) => {{
        crate::witness::hit(crate::witness::ROAD_ENTRIES);
        // SAFETY: Immutable tables outlive this synchronous call and nested entries;
        // owners are accessed only through raw pointers between services.
        unsafe { Game::new($leaves) }
    }};
}

/// `RoadVehicle::Tick`.
/// # Safety
/// A live record and road owner pair, the static tables and the native command slot for
/// this synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_tick_abi(
    v: VehicleRef,
    state: *mut State,
    leaves: *const Leaves,
    road: *mut crate::road_cmd::CommandSlot,
) -> u8 {
    crate::witness::hit(crate::witness::ROAD_TICKS);
    let mut g = entry!(leaves);
    g.road = road;
    u8::from(g.tick(Part::new(v, state)))
}
/// `IndividualRoadVehicleController`.
/// # Safety
/// As [`openttd_rust_road_tick_abi`]; `prev` is null for the front engine and otherwise the
/// live pair of the part before `v`, which the non-front controller path dereferences.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_individual_abi(
    v: VehicleRef,
    state: *mut State,
    prev: Option<VehicleRef>,
    prev_state: *mut State,
    leaves: *const Leaves,
    road: *mut crate::road_cmd::CommandSlot,
) -> u8 {
    let mut g = entry!(leaves);
    g.road = road;
    let prev = prev.map(|prev| Part::new(prev, prev_state));
    u8::from(g.individual_road_vehicle_controller(Part::new(v, state), prev))
}
/// `RoadVehLeaveDepot`.
/// # Safety
/// As [`openttd_rust_road_tick_abi`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_leave_depot_abi(
    v: VehicleRef,
    state: *mut State,
    first: bool,
    leaves: *const Leaves,
) -> u8 {
    u8::from(entry!(leaves).road_veh_leave_depot(Part::new(v, state), first))
}
/// `RoadVehicle::Crash`.
/// # Safety
/// As [`openttd_rust_road_tick_abi`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_crash(
    v: VehicleRef,
    state: *mut State,
    flooded: bool,
    leaves: *const Leaves,
) -> u32 {
    entry!(leaves).crash(Part::new(v, state), flooded)
}
/// `RoadVehUpdateCache`.
/// # Safety
/// As [`openttd_rust_road_tick_abi`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_update_cache(
    v: VehicleRef,
    state: *mut State,
    same_length: bool,
    leaves: *const Leaves,
) {
    entry!(leaves).road_veh_update_cache(Part::new(v, state), same_length);
}
/// `RoadVehicle::OnNewCalendarDay`.
/// # Safety
/// As [`openttd_rust_road_tick_abi`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_calendar_day(
    v: VehicleRef,
    state: *mut State,
    leaves: *const Leaves,
) {
    entry!(leaves).on_new_calendar_day(Part::new(v, state));
}
/// `RoadVehicle::OnNewEconomyDay`.
/// # Safety
/// As [`openttd_rust_road_tick_abi`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_economy_day(
    v: VehicleRef,
    state: *mut State,
    leaves: *const Leaves,
) {
    entry!(leaves).on_new_economy_day(Part::new(v, state));
}
/// `RoadVehicle::GetRunningCost`.
/// # Safety
/// As [`openttd_rust_road_tick_abi`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_running_cost(
    v: VehicleRef,
    state: *mut State,
    leaves: *const Leaves,
) -> i64 {
    entry!(leaves).get_running_cost(Part::new(v, state))
}
/// `RoadVehicle::GetCurrentMaxSpeed`.
/// # Safety
/// As [`openttd_rust_road_tick_abi`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_max_speed(
    v: VehicleRef,
    state: *mut State,
    leaves: *const Leaves,
) -> i32 {
    let ss = entry!(leaves).ss;
    let world = std::cell::OnceCell::new();
    get_current_max_speed(
        ss,
        || world.get_or_init(|| World::new(ss)),
        Part::new(v, state),
    )
}
/// `RoadVehicle::SetDestTile`.
/// # Safety
/// A live record and road owner pair.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_set_dest(v: VehicleRef, state: *mut State, tile: u32) {
    set_dest_tile(Part::new(v, state), TileIndex(tile));
}
/// The body of `CmdTurnRoadVeh` after its vehicle, primary and ownership checks; 0 for
/// `CMD_ERROR`.
/// # Safety
/// As [`openttd_rust_road_tick_abi`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_turn_abi(
    v: VehicleRef,
    state: *mut State,
    flags: u16,
    leaves: *const Leaves,
) -> u8 {
    u8::from(entry!(leaves).cmd_turn_road_veh(
        Part::new(v, state),
        crate::command_type::DoCommandFlags(flags),
    ))
}
/// `RoadVehicle::GetVehicleTrackdir`.
/// # Safety
/// As [`openttd_rust_road_tick_abi`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_trackdir(
    v: VehicleRef,
    state: *mut State,
    leaves: *const Leaves,
) -> u8 {
    entry!(leaves).get_vehicle_trackdir(Part::new(v, state)).0
}
/// `RoadVehicle::FindClosestDepot`, written to `closest`.
/// # Safety
/// As [`openttd_rust_road_tick_abi`]; `closest` is a writable record.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_find_closest_depot(
    v: VehicleRef,
    state: *mut State,
    leaves: *const Leaves,
    closest: *mut Closest,
) {
    let result = entry!(leaves).find_closest_depot(Part::new(v, state));
    // SAFETY: As this function's contract.
    unsafe {
        closest.write(Closest {
            tile: result.location.0,
            destination: result.destination.0,
            reverse: u8::from(result.reverse),
            found: u8::from(result.found),
        });
    };
}
/// `_road_stop_stop_frame[index]`, for the afterload of old saves.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_stop_frame(index: u32) -> u8 {
    crate::road_data::_ROAD_STOP_STOP_FRAME[index as usize]
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRoadPathElement" ["probe"] => PathElement {
        trackdir,
        tile,
    };
    "OpenTTDRoadLeaves" ["probe"] => Leaves {
        build_road_context,
        ground,
        shared,
        vehicle,
    };
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod map_context_tests {
    use super::*;
    use crate::test_tables::{GROUND_VEHICLE, ROAD_LEAVES, SHARED, VEHICLE, unreachable1};

    /// Pinned `RoadVehicle::Tick` returns for an articulated part after wrapping
    /// `tick_counter`. The native ABI fixture leaves map services unused here.
    #[test]
    fn articulated_tick_keeps_early_return_without_map_metadata() {
        let mut shared = SHARED;
        shared.map = std::ptr::null();
        let vehicle = VEHICLE;
        let ground = GroundServices {
            physics: &GROUND_VEHICLE,
            has_articulated: unreachable1,
        };
        let mut leaves = ROAD_LEAVES;
        leaves.ground = &raw const ground;
        leaves.shared = &raw const shared;
        leaves.vehicle = &raw const vehicle;
        let record = crate::vehicle_base::openttd_rust_vehicle_record_new(
            std::ptr::null_mut(),
            crate::vehicle_type::VehicleType::VEH_ROAD,
        );
        let mut state = State::default();
        // SAFETY: The record, owner and tables remain live throughout this call. The
        // original articulated branch reads no orders, ground or map.
        unsafe {
            (*record).road = std::ptr::from_mut(&mut state.fields);
            (*record).tick_counter = u8::MAX;
            (*record).running_ticks = 7;
            let part = Part::new(VehicleRef::of(record), std::ptr::from_mut(&mut state));
            let game = Game::new(&raw const leaves);
            assert!(game.tick(part));
            assert_eq!(((*record).tick_counter, (*record).running_ticks), (0, 7));
            crate::vehicle_base::openttd_rust_vehicle_record_delete(record);
        }
    }
}
