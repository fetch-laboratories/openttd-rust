/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Road vehicle movement, consist traversal and private counters and cache. Observations are
//! copies taken at the original read points and refreshed after every service that can
//! change them.
//! Entries keep vehicle shells and their service tables live and serialized through
//! maintenance calls; no owner borrow spans a callback that can reenter or mutate it.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::struct_excessive_bools,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::verbose_bit_mask,
    clippy::items_after_statements,
    clippy::if_same_then_else,
    clippy::needless_late_init,
    clippy::manual_is_power_of_two,
    clippy::collapsible_if,
    clippy::if_not_else
)]

use crate::direction_type::{DiagDirection, Direction};
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
    /// Native `Test` of one element.
    #[must_use]
    pub const fn test(self, element: RoadTypeFlag) -> bool {
        self.0 & (1 << element.0) != 0
    }
}
use crate::direction_func::dir_to_diag_dir_raw as diag;
use crate::engine_type::INVALID_ENGINE;
use crate::newgrf_callbacks::CBID_VEHICLE_LENGTH;
use crate::newgrf_sound::VSE_START;
use crate::raw_field::{read_field, write_field};
use crate::road_data;
use crate::roadveh::{
    RVSB_IN_DEPOT as DEPOT, RVSB_IN_DT_ROAD_STOP as DT_STOP, RVSB_IN_ROAD_STOP as STOP,
    RVSB_WORMHOLE,
};
use crate::services::{Map, RoadStopType, Services};
use crate::tile_type::TileIndex;
use crate::track_type::INVALID_TRACKDIR;
use crate::vehicle::maintenance;
use crate::vehicle::{
    AM_REALISTIC, CALLBACK_FAILED, GroundServices, HVOT_BUS, HVOT_TRUCK, INVALID_PRICE, MP_ROAD,
    MP_STATION, MP_TUNNELBRIDGE, OT_GOTO_DEPOT, OT_GOTO_STATION, OT_LEAVESTATION, OT_LOADING,
    SND_DEPARTURE_OLD_RV_1, SND_DEPARTURE_OLD_RV_2, VEHICLE_LENGTH, VETS_CANNOT_ENTER,
    VETS_ENTERED_WORMHOLE, VehicleRef, VehicleServices, YEAR_TICKS, advance_distance,
};
use crate::vehicle_base::{VS_CRASHED, VS_HIDDEN, VS_STOPPED};
use std::ptr::NonNull;

const REVERSE: [u8; 4] = [6, 7, 14, 15];
crate::abi::shared_records! {
    fn abi_record;
    /// Canonical road fields, with the original widths and zero defaults.
    /// This is the first member of the stable allocation; the path stays Rust-private.
    #[derive(Default)]
    "OpenTTDRoadState" ["road_state"] => pub struct OpenTTDRoadState {
        /// `RoadVehicleStates`.
        pub state: u8,
        pub frame: u8,
        pub blocked_ctr: u16,
        /// `RVSB_DRIVE_SIDE` when overtaking, otherwise zero.
        pub overtaking: u8,
        /// Length of the current overtake attempt.
        pub overtaking_ctr: u8,
        /// Crash animation counter; see `RoadVehIsCrashed`.
        pub crashed_ctr: u16,
        pub reverse_ctr: u8,
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PathElement {
    pub trackdir: u8,
    pub tile: u32,
}
/// One allocation per ordinary/indexed-load C++ shell.
#[repr(C)]
#[derive(Default)]
pub struct State {
    pub fields: OpenTTDRoadState,
    pub(crate) path: Vec<PathElement>,
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_new() -> *mut State {
    Box::into_raw(Box::default())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_destroy(state: *mut State) {
    // SAFETY: One shell transfers its live owner exactly once after PreDestructor.
    unsafe {
        drop(Box::from_raw(state));
    }
}
/// Original `RoadVehicle::SetState`, over the canonical field.
/// # Safety
/// The serialized native shell owns this live state for the whole scalar access.
pub(crate) unsafe fn set_state(state: *mut State, value: u8) {
    write_field!(state, fields.state, value);
}
/// Original `RoadVehicle::GetState`.
/// # Safety
/// The serialized native shell owns this live state for the whole scalar access.
pub(crate) unsafe fn get_state(state: *const State) -> u8 {
    read_field!(state, fields.state)
}
/// Original `RoadVehicle::SetState(RVSB_IN_DEPOT)`.
/// # Safety
/// A live serialized road owner; the scalar access ends before any callback.
pub(crate) unsafe fn set_depot_state(state: *mut State) {
    unsafe { set_state(state, DEPOT) };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_path_size(state: *const State) -> usize {
    // SAFETY: Live owner, no mutation during this accessor.
    unsafe { (*state).path.len() }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_path_get(
    state: *const State,
    index: usize,
) -> PathElement {
    // SAFETY: Original vector index precondition, live owner.
    unsafe { (&(*state).path)[index] }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_path_replace(
    state: *mut State,
    input: *const PathElement,
    count: usize,
) {
    // SAFETY: Disjoint call-scoped staging array, no owner borrow during C++ calls.
    unsafe {
        (*state).path.clear();
        for index in 0..count {
            (*state).path.push(input.add(index).read());
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_path_clear(state: *mut State) {
    // SAFETY: Exclusive game-thread owner access.
    unsafe {
        (*state).path.clear();
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_path_push(state: *mut State, item: PathElement) {
    // SAFETY: Exclusive game-thread owner access.
    unsafe {
        (*state).path.push(item);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_path_pop(state: *mut State) {
    // SAFETY: Original nonempty vector precondition, exclusive owner access.
    unsafe {
        (*state).path.pop();
    }
}

/// Separate ground owner, movement settings and map size for one movement entry.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Entry {
    pub ground: *mut crate::ground_vehicle::GroundState,
    pub acceleration_model: u8,
    pub road_side: u8,
    pub queue: bool,
    pub map_log_x: u8,
}

/// `GetCurrentMaxSpeed` inputs of the front part.
#[derive(Clone, Copy)]
struct MaxSpeedRead {
    pub next: Option<VehicleRef>,
    pub next_state: *mut State,
    pub tile: u32,
    pub max_track_speed: u16,
    pub order_max_speed: u16,
    pub direction: u8,
    pub status: u8,
}

/// Speed and first-step inputs observed after visual effects return.
#[derive(Clone, Copy)]
struct SpeedRead {
    pub max: MaxSpeedRead,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub length: u8,
    pub tram: bool,
}

/// One further consist part for speed, viewport and slope walks.
#[derive(Clone, Copy)]
struct SpeedPart {
    pub next: Option<VehicleRef>,
    pub next_state: *mut State,
    pub tile: u32,
    pub direction: u8,
    pub status: u8,
}

/// Consist links and the next shell's separate ground owner, refreshed after callbacks.
#[derive(Clone, Copy)]
struct Link {
    pub next: Option<VehicleRef>,
    pub next_state: *mut State,
    pub next_ground: *mut crate::ground_vehicle::GroundState,
}
/// Consist links and tile for depot traversal.
#[derive(Clone, Copy)]
struct LinkTile {
    pub link: Link,
    pub tile: u32,
}

/// Movement inputs of one part at the start of `IndividualRoadVehicleController`.
#[derive(Clone, Copy)]
struct Step {
    pub tile: u32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub cur_speed: u16,
    pub direction: u8,
    pub length: u8,
    pub front: bool,
    pub tram: bool,
}

/// One part for `RoadVehCheckTrainCrash`.
#[derive(Clone, Copy)]
struct CrashPart {
    pub next: Option<VehicleRef>,
    pub next_state: *mut State,
    pub z: i32,
    pub crossing: bool,
}

/// `VehicleEnterTile` result, then the fields it may have changed.
#[derive(Clone, Copy)]
struct Entered {
    pub tile: u32,
    pub cur_speed: u16,
    pub flags: u8,
    pub direction: u8,
    pub order_type: u8,
}

/// One road vehicle near the probe, other than the probing consist.
#[derive(Clone, Copy)]
struct Candidate {
    pub state: *mut State,
    pub first: VehicleRef,
    pub first_state: *mut State,
    pub first_ground: *mut crate::ground_vehicle::GroundState,
    pub index: u32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub first_speed: u16,
    pub direction: u8,
}

/// Generic part read: link, tile, height and road/tram type.
#[derive(Clone, Copy)]
struct PartRead {
    pub next: Option<VehicleRef>,
    pub next_state: *mut State,
    pub tile: u32,
    pub z: i32,
    pub tram: bool,
}

/// `RoadFindPathToDest` vehicle inputs.
#[derive(Clone, Copy)]
struct PathRead {
    pub dest: u32,
    pub owner: u8,
    pub articulated: bool,
}

/// `RoadVehCheckOvertake` inputs of either vehicle.
#[derive(Clone, Copy)]
struct OvertakeRead {
    pub tile: u32,
    pub cur_speed: u16,
    pub direction: u8,
    pub status: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrackChoice {
    pub trackdir: u8,
    pub found: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DepotResult {
    pub tile: u32,
    pub length: u32,
}

#[derive(Clone, Copy)]
struct ArrivalRead {
    pub had_vehicle_of_type: u8,
}

#[derive(Clone, Copy)]
struct SoundRead {
    pub sound: u16,
    pub tick: u8,
}

#[derive(Clone, Copy)]
struct ServiceRead {
    pub tile: u32,
    pub servint: u16,
    pub cur_speed: u16,
}

#[derive(Clone, Copy)]
struct TurnRead {
    pub tile: u32,
    pub breakdown_ctr: u8,
    pub direction: u8,
    pub order_type: u8,
    pub status: u8,
}

#[derive(Clone, Copy)]
struct TrackdirRead {
    pub tile: u32,
    pub direction: u8,
    pub status: u8,
}

/// Road-specific services; the shared tables are borrowed through pointers.
#[repr(C)]
pub struct Leaves {
    pub vehicle: *const VehicleServices,
    pub ground: *const GroundServices,
    pub shared: *const Services,
    pub any_road_bits: extern "C" fn(VehicleRef, u32, bool) -> u8,
    pub is_road_depot_tile: extern "C" fn(u32) -> bool,
    pub depot_direction: extern "C" fn(u32) -> u8,
    pub disallowed_directions: extern "C" fn(u32) -> u8,
    pub is_bay_stop: extern "C" fn(u32) -> bool,
    pub bay_direction: extern "C" fn(u32) -> u8,
    pub is_station_road_stop: extern "C" fn(u32) -> bool,
    pub choose_track: extern "C" fn(VehicleRef, u32, u8, u16) -> TrackChoice,
    pub find_depot: extern "C" fn(VehicleRef, i32) -> DepotResult,
    pub tram_build_read:
        extern "C" fn(VehicleRef, *mut crate::road_cmd::CommandSlot) -> TramBuildRead,
    pub disconnect: extern "C" fn(),
    pub should_stop: extern "C" fn(VehicleRef, u32) -> bool,
    pub arrival_news: extern "C" fn(VehicleRef, u16, u8),
    /// The road stop arrival randomisation and animation triggers.
    pub depot_exit_dirty: extern "C" fn(VehicleRef),
    pub crash_news: extern "C" fn(VehicleRef, u32),
    pub pay_running: extern "C" fn(VehicleRef, i64),
}

/// A consist part: live vehicle handle and its live Rust owner.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Part {
    v: VehicleRef,
    s: NonNull<State>,
}
impl Part {
    fn new(v: VehicleRef, s: *mut State) -> Self {
        // SAFETY: Every road vehicle shell owns a non-null state.
        Self {
            v,
            s: unsafe { NonNull::new_unchecked(s) },
        }
    }
    fn link(v: Option<VehicleRef>, s: *mut State) -> Option<Self> {
        v.map(|v| Self::new(v, s))
    }
    fn ground(
        self,
        state: *mut crate::ground_vehicle::GroundState,
    ) -> crate::ground_vehicle::Owner {
        crate::ground_vehicle::Owner::for_road(state, self.s.as_ptr())
    }
    fn state(self) -> u8 {
        read_field!(self.s.as_ptr(), fields.state)
    }
    fn frame(self) -> u8 {
        read_field!(self.s.as_ptr(), fields.frame)
    }
    fn overtaking(self) -> u8 {
        read_field!(self.s.as_ptr(), fields.overtaking)
    }
    fn path_len(self) -> usize {
        // SAFETY: Live owner, short accessor only.
        unsafe { (*self.s.as_ptr()).path.len() }
    }
    fn path_back(self) -> PathElement {
        let last = self.path_len() - 1;
        // SAFETY: Caller establishes a nonempty path; no service in this scope.
        unsafe { (&(*self.s.as_ptr()).path)[last] }
    }
    fn path_clear(self) {
        // SAFETY: Live owner, game thread.
        unsafe { (*self.s.as_ptr()).path.clear() }
    }
    fn path_pop(self) {
        // SAFETY: Live owner, game thread.
        unsafe {
            (*self.s.as_ptr()).path.pop();
        }
    }
    fn path_push(self, trackdir: u8, tile: u32) {
        // SAFETY: Live owner, game thread.
        unsafe { (*self.s.as_ptr()).path.push(PathElement { trackdir, tile }) }
    }
}
/// Read the state scalar of a part known only by its owner pointer.
fn state_of(s: *mut State) -> u8 {
    // SAFETY: Live owner supplied by a native traversal for this dispatch only.
    read_field!(s, fields.state)
}

/// Fields consumed together by `CanBuildTramTrackOnTile`.
#[repr(C)]
pub struct TramBuildRead {
    pub road_type: u8,
    /// Freshly constructed in the entry's slot for this one build.
    pub road: *const crate::road_cmd::CommandContext,
}
/// Count one actual typed outgoing call; returns the unchanged function pointer.
macro_rules! service {
    ($function:expr) => {
        crate::witness::counted!(before ROAD_SERVICES, $function)
    };
}

struct Game<'a> {
    road: *mut crate::road_cmd::CommandSlot,
    l: &'a Leaves,
    vs: &'a VehicleServices,
    gs: &'a GroundServices,
    ss: &'a Services,
    movement: crate::movement::CachedTable<'a>,
    acceleration_model: u8,
    road_side: u8,
    queue: bool,
    // Entry dimensions last only for this synchronous controller call.
    // Orders, sound/GRF, movement, crash and viewport callbacks cannot load/reset
    // the map or yield the game-state mutex; nested entries have their own Game.
    map: std::cell::OnceCell<Map>,
    map_log_x: u32,
}

fn in_range(value: u8, start: u8, end: u8) -> bool {
    value >= start && value < end
}
fn reversing(dir: u8) -> bool {
    dir & 7 >= 6
}
fn straight(dir: u8) -> bool {
    dir & 6 == 0
}
/// `RoadVehicle::GetCurrentMaxSpeed`.
#[expect(
    clippy::inline_always,
    reason = "Specialize the existing loop at each speed observation."
)]
#[inline(always)]
fn max_speed(front: Part, r: MaxSpeedRead, acceleration_model: u8, l: &Leaves) -> i32 {
    let mut speed = i32::from(r.max_track_speed);
    let mut u = SpeedPart {
        next: r.next,
        next_state: r.next_state,
        tile: r.tile,
        direction: r.direction,
        status: r.status,
    };
    let mut part = front;
    loop {
        if acceleration_model == AM_REALISTIC {
            let state = front.state();
            if state <= 15 && reversing(state) {
                speed = i32::from(r.max_track_speed) / 2;
                break;
            } else if u.direction & 1 == 0 {
                speed = i32::from(r.max_track_speed) * 3 / 4;
            }
        }
        if part.state() == RVSB_WORMHOLE && u.status & VS_HIDDEN == 0 {
            // SAFETY: The static shared table outlives this synchronous query.
            speed = speed.min(i32::from(service!(unsafe { (*l.shared).bridge_speed })(u.tile)) * 2);
        }
        let Some(next) = Part::link(u.next, u.next_state) else {
            break;
        };
        part = next;
        u = SpeedPart {
            next: next.v.next(),
            next_state: next.v.next().map_or(std::ptr::null_mut(), |n| {
                crate::raw_field::read_field!(n.0.as_ptr(), road).cast()
            }),
            tile: next.v.tile(),
            direction: next.v.direction(),
            status: next.v.status(),
        };
    }
    speed.min(i32::from(r.order_max_speed) * 2)
}

impl Game<'_> {
    /// # Safety
    /// Tables outlive the synchronous entry and every nested entry.
    unsafe fn new<'a>(leaves: *const Leaves, entry: Option<&Entry>) -> Game<'a> {
        // SAFETY: Immutable static tables supplied by the C++ facade.
        let l = unsafe { &*leaves };
        let (acceleration_model, road_side, queue, log_x) = entry.map_or((0, 0, false, 0), |e| {
            (e.acceleration_model, e.road_side, e.queue, e.map_log_x)
        });
        // SAFETY: As above; the shared tables are static.
        unsafe {
            Game {
                road: std::ptr::null_mut(),
                l,
                vs: &*l.vehicle,
                gs: &*l.ground,
                ss: &*l.shared,
                movement: crate::movement::CachedTable::audited(&*l.shared),
                acceleration_model,
                road_side,
                queue,
                map: std::cell::OnceCell::new(),
                map_log_x: u32::from(log_x),
            }
        }
    }
    fn map(&self) -> Map {
        *self.map.get_or_init(|| {
            let mut map = self.ss.map();
            map.log_x = self.map_log_x;
            map
        })
    }
    fn can_build_tram(&self, v: VehicleRef, tile: u32, bits: u8) -> bool {
        let owner = v.owner();
        let read = service!(self.l.tram_build_read)(v, self.road);
        // SAFETY: the read built live native stack scratch and results in the slot.
        unsafe {
            let context = &*read.road;
            let town = &*(*context.services).town;
            let old = (town.exchange_company)(u32::from(owner));
            let success = context.build(8, tile, bits, read.road_type, u16::MAX);
            (town.exchange_company)(old);
            crate::witness::hit(if success {
                crate::witness::ROAD_TRAM_BUILD
            } else {
                crate::witness::ROAD_TRAM_REFUSED
            });
            success
        }
    }
    fn realistic(&self) -> bool {
        self.acceleration_model == AM_REALISTIC
    }
    /// `== AM_ORIGINAL`; the original tests either constant at each site.
    fn original(&self) -> bool {
        self.acceleration_model == 0
    }
    fn side(&self) -> usize {
        usize::from(self.road_side) << 4
    }
    fn random(&self) -> u32 {
        service!(self.ss.random)(self.ss.context)
    }
    fn has_road(&self, vehicle: VehicleRef, tile: u32) -> bool {
        let types = (self.movement.get().road_compatible)(vehicle);
        let world = crate::map_func::World::new(self.ss);
        crate::road_map::has_tile_any_road_type(&world, tile, types)
    }
    fn tile_xy(&self, tile: u32, pos: (u8, u8)) -> (i32, i32) {
        (
            (self.map().tile_x(TileIndex(tile)) * 16 + u32::from(pos.0)) as i32,
            (self.map().tile_y(TileIndex(tile)) * 16 + u32::from(pos.1)) as i32,
        )
    }
    fn set_speed(v: Part, s: &mut Step, speed: u16) {
        (v.v).write_speed(speed);
        s.cur_speed = speed;
    }
    fn observe(s: &mut Step, e: Entered) {
        s.tile = e.tile;
        s.cur_speed = e.cur_speed;
        s.direction = e.direction;
    }

    fn visual_effect_and_speed(&self, v: Part) -> SpeedRead {
        if crate::vehicle_effect::openttd_rust_visual_eligible_abi(v.v) != 0 {
            let r = MaxSpeedRead {
                next: v.v.next(),
                next_state: v.v.next().map_or(std::ptr::null_mut(), |n| {
                    crate::raw_field::read_field!(n.0.as_ptr(), road).cast()
                }),
                tile: v.v.tile(),
                max_track_speed: crate::raw_field::read_field!(
                    v.v.ground_cache(),
                    cached_max_track_speed
                ),
                order_max_speed: v.v.current_order().speed,
                direction: v.v.direction(),
                status: v.v.status(),
            };
            let speed = max_speed(v, r, self.acceleration_model, self.l);
            crate::vehicle_effect::show_visual_effect(v.v, speed as u32);
        }
        SpeedRead {
            max: MaxSpeedRead {
                next: v.v.next(),
                next_state: v.v.next().map_or(std::ptr::null_mut(), |n| {
                    crate::raw_field::read_field!(n.0.as_ptr(), road).cast()
                }),
                tile: v.v.tile(),
                max_track_speed: crate::raw_field::read_field!(
                    v.v.ground_cache(),
                    cached_max_track_speed
                ),
                order_max_speed: v.v.current_order().speed,
                direction: v.v.direction(),
                status: v.v.status(),
            },
            x: v.v.x(),
            y: v.v.y(),
            z: v.v.z(),
            length: crate::raw_field::read_field!(v.v.ground_cache(), cached_veh_length),
            tram: (crate::movement::table(self.ss).road_kind)(v.v).0,
        }
    }
    /// `RoadVehicle::UpdateSpeed`, returning the distance and stored speed.
    fn update_speed(
        &self,
        front: Part,
        ground: crate::ground_vehicle::Owner,
        r: &SpeedRead,
    ) -> (i32, u16) {
        let over = front.overtaking() != 0;
        let (accel, min) = if self.realistic() {
            (
                crate::ground_vehicle::get_acceleration_owned(self.gs.physics, front.v, ground)
                    .wrapping_add(if over { 256 } else { 0 }) as u32,
                if r.max.status & VS_STOPPED != 0 { 0 } else { 4 },
            )
        } else {
            (if over { 512 } else { 256 }, 0)
        };
        let distance = crate::ground_vehicle::do_update_speed(
            front.v,
            accel,
            min,
            max_speed(front, r.max, self.acceleration_model, self.l),
        );
        (distance as i32, front.v.speed())
    }
    /// `RoadVehFindCloseTo`; `v` is always the front part.
    fn close(&self, v: Part, z: i32, x: i32, y: i32, dir: u8, update: bool) -> Option<Candidate> {
        if read_field!(v.s.as_ptr(), fields.reverse_ctr) != 0 {
            return None;
        }
        let mut best: Option<Candidate> = None;
        let mut best_diff = u32::MAX;
        let wormhole = v.state() == RVSB_WORMHOLE;
        let mut offer = |vehicle: VehicleRef| {
            if vehicle.vehicle_type() != crate::vehicle_type::VEH_ROAD || vehicle.first() == v.v {
                return false;
            }
            let first = vehicle.first();
            let candidate = Candidate {
                state: crate::raw_field::read_field!(vehicle.0.as_ptr(), road).cast(),
                first,
                first_state: crate::raw_field::read_field!(first.0.as_ptr(), road).cast(),
                first_ground: first.ground(),
                // SAFETY: The spatial cursor offers a live record and the shared resolver table is static.
                index: unsafe { ((*((self.vs.newgrf)())).id)(vehicle.0.as_ptr()) },
                x: vehicle.x(),
                y: vehicle.y(),
                z: vehicle.z(),
                first_speed: first.speed(),
                direction: vehicle.direction(),
            };
            let u = &candidate;
            crate::witness::hit(crate::witness::ROAD_VISITORS);
            let mut consider = || {
                // FindClosestBlockingRoadVeh after type/consist admission.
                if state_of(u.state) == DEPOT {
                    return true;
                }
                if u.z.wrapping_sub(z).wrapping_abs() >= 6 || u.direction != dir {
                    return true;
                }
                let x_diff = u.x.wrapping_sub(x);
                let y_diff = u.y.wrapping_sub(y);
                let diff = x_diff.wrapping_abs().wrapping_add(y_diff.wrapping_abs()) as u32;
                if diff > best_diff
                    || (diff == best_diff && best.as_ref().is_some_and(|b| u.index > b.index))
                {
                    return true;
                }
                const DX: [i32; 8] = [-4, -8, -4, -1, 4, 8, 4, 1];
                const DY: [i32; 8] = [-4, -1, 4, 8, 4, 1, -4, -8];
                let axis = |dist: i32, diff: i32| {
                    if dist < 0 {
                        diff > dist && diff <= 0
                    } else {
                        diff < dist && diff >= 0
                    }
                };
                let d = usize::from(u.direction);
                if axis(DX[d], x_diff) && axis(DY[d], y_diff) {
                    best = Some(*u);
                    best_diff = diff;
                }
                true
            };
            consider();
            false
        };
        if wormhole {
            let tile = (v.v).tile();
            self.vs.any_on_tile(tile, &mut offer);
            self.vs
                .any_on_tile(service!(self.ss.other_tunnel_end)(tile), &mut offer);
        } else {
            self.vs.any_near(x, y, 8, offer);
        }
        if best_diff == u32::MAX {
            write_field!(v.s.as_ptr(), fields.blocked_ctr, 0);
            return None;
        }
        if update {
            let counter = read_field!(v.s.as_ptr(), fields.blocked_ctr).wrapping_add(1);
            write_field!(v.s.as_ptr(), fields.blocked_ctr, counter);
            if counter > 1480 {
                return None;
            }
        }
        best
    }
    /// `CheckRoadBlockedForOvertaking`.
    fn overtake_blocked(&self, v: Part, u: VehicleRef, tile: u32, trackdir: u8) -> bool {
        if !self.has_road(v.v, tile) {
            return true;
        }
        let ts = self.movement.road_tracks(v.v, tile);
        let trackdirs = ts & 0x3f3f;
        let red = (ts >> 16) & 0x3f3f;
        let trackbits = (trackdirs | (trackdirs >> 8)) & 0x3f;
        if trackdirs & (1 << trackdir) == 0 || trackbits & !3 != 0 || red != 0 {
            return true;
        }
        self.vs.any_on_tile(tile, |w| {
            w.vehicle_type() == crate::vehicle::VehicleType::ROAD.0
                && w.first() == w
                && w != u
                && w != v.v
        })
    }
    /// `RoadVehCheckOvertake(v, u)`; pure predicates are evaluated cheapest first.
    fn overtake(&self, v: Part, s: &Step, u: Part, ground: crate::ground_vehicle::Owner) {
        if s.tram {
            return;
        }
        let ur = OvertakeRead {
            tile: u.v.tile(),
            cur_speed: u.v.speed(),
            direction: u.v.direction(),
            status: u.v.status(),
        };
        if s.direction != ur.direction || s.direction & 1 == 0 {
            return;
        }
        let state = v.state();
        if state >= STOP || !straight(state & 15) {
            return;
        }
        if service!(self.ss.tile_type)(s.tile) == MP_STATION
            || service!(self.ss.tile_type)(ur.tile) == MP_STATION
        {
            return;
        }
        if service!(self.gs.has_articulated)(v.v) {
            return;
        }
        let u_speed = if self.original()
            || crate::ground_vehicle::get_acceleration_owned(self.gs.physics, u.v, ground) > 0
        {
            max_speed(
                u,
                MaxSpeedRead {
                    next: u.v.next(),
                    next_state: u.v.next().map_or(std::ptr::null_mut(), |n| {
                        crate::raw_field::read_field!(n.0.as_ptr(), road).cast()
                    }),
                    tile: u.v.tile(),
                    max_track_speed: crate::raw_field::read_field!(
                        u.v.ground_cache(),
                        cached_max_track_speed
                    ),
                    order_max_speed: u.v.current_order().speed,
                    direction: u.v.direction(),
                    status: u.v.status(),
                },
                self.acceleration_model,
                self.l,
            )
        } else {
            i32::from(ur.cur_speed)
        };
        if u_speed
            >= max_speed(
                v,
                MaxSpeedRead {
                    next: v.v.next(),
                    next_state: v.v.next().map_or(std::ptr::null_mut(), |n| {
                        crate::raw_field::read_field!(n.0.as_ptr(), road).cast()
                    }),
                    tile: v.v.tile(),
                    max_track_speed: crate::raw_field::read_field!(
                        v.v.ground_cache(),
                        cached_max_track_speed
                    ),
                    order_max_speed: v.v.current_order().speed,
                    direction: v.v.direction(),
                    status: v.v.status(),
                },
                self.acceleration_model,
                self.l,
            )
            && ur.status & VS_STOPPED == 0
            && ur.cur_speed != 0
        {
            return;
        }
        let trackdir = crate::track_func::diag_dir_to_diag_trackdir(
            diag(s.direction, |ok| {
                crate::movement::native_assert(self.ss, ok);
            }),
            |ok| crate::movement::native_assert(self.ss, ok),
        );
        if self.overtake_blocked(v, u.v, s.tile, trackdir) {
            return;
        }
        let tile = (TileIndex(s.tile)
            + self.map().tile_offs_by_diag_dir(
                DiagDirection(diag(s.direction, |ok| {
                    crate::movement::native_assert(self.ss, ok);
                })),
                crate::map_func::native_check,
            ))
        .0;
        if self.overtake_blocked(v, u.v, tile, trackdir) {
            return;
        }
        let overtaking_ctr = if ur.cur_speed == 0 || ur.status & VS_STOPPED != 0 {
            17
        } else {
            0
        };
        write_field!(v.s.as_ptr(), fields.overtaking_ctr, overtaking_ctr);
        write_field!(v.s.as_ptr(), fields.overtaking, 16);
    }
    /// `RoadVehGetSlidingDirection`.
    fn sliding(s: &Step, x: i32, y: i32) -> u8 {
        const NEW_DIR: [u8; 11] = [0, 7, 6, 255, 1, 0, 5, 255, 2, 3, 4];
        let xd = x.wrapping_sub(s.x).wrapping_add(1);
        let yd = y.wrapping_sub(s.y).wrapping_add(1);
        let new = if xd as u32 > 2 || yd as u32 > 2 {
            s.direction
        } else {
            NEW_DIR[(yd * 4 + xd) as usize]
        };
        if new == s.direction {
            return s.direction;
        }
        let delta = if new.wrapping_sub(s.direction) & 7 > 4 {
            7
        } else {
            1
        };
        (s.direction + delta) & 7
    }
    /// Direction store with the original-acceleration curve slowdown.
    fn set_direction(&self, v: Part, s: &mut Step, dir: u8) {
        (v.v).write_direction(dir);
        s.direction = dir;
        if self.original() {
            Self::set_speed(v, s, s.cur_speed - (s.cur_speed >> 2));
        }
    }
    /// x/y store, `UpdatePosition`, `UpdateInclination` and `RoadZPosAffectSpeed`.
    fn move_to(
        &self,
        v: Part,
        ground: crate::ground_vehicle::Owner,
        s: &mut Step,
        pos: (i32, i32),
        new_tile: bool,
        delta: bool,
    ) {
        let m = move_incline(
            self.gs,
            &self.movement,
            self.map(),
            v.v,
            ground,
            pos,
            new_tile,
            delta,
        );
        s.x = pos.0;
        s.y = pos.1;
        s.z = m.z;
        s.cur_speed = m.cur_speed;
        if m.old_z == m.z || !self.original() {
            return;
        }
        if m.old_z < m.z {
            Self::set_speed(v, s, (u32::from(m.cur_speed) * 232 / 256) as u16);
        } else {
            let speed = m.cur_speed.wrapping_add(2);
            if speed <= m.max_track_speed {
                Self::set_speed(v, s, speed);
            }
        }
    }
    /// `StartRoadVehSound`.
    fn start_sound(&self, v: Part) {
        if !service!(self.vs.play_vehicle_sound)(v.v, VSE_START) {
            let r = SoundRead {
                sound: (self.vs.engine)(v.v).road_sound(),
                tick: v.v.tick(),
            };
            let mut sound = r.sound;
            if sound == SND_DEPARTURE_OLD_RV_1 && r.tick & 3 == 0 {
                sound = SND_DEPARTURE_OLD_RV_2;
            }
            service!(self.vs.play_sound)(v.v, sound);
        }
    }
    /// `RoadVehicle::Crash`.
    fn crash(&self, v: Part, flooded: bool) -> u32 {
        let mut victims = crate::ground_vehicle::crash(self.gs.physics, v.v, flooded);
        if crate::vehicle_base::is_front_engine(true, (v.v).subtype()) {
            victims = victims.wrapping_add(1);
            if in_range(v.state(), DT_STOP, DT_STOP + 16) {
                // SAFETY: Live front owner; no owner access spans the stop update.
                unsafe { crate::road_stop::leave_vehicle(self.ss, v.v, v.s.as_ptr()) };
            }
        }
        write_field!(
            v.s.as_ptr(),
            fields.crashed_ctr,
            if flooded { 2000 } else { 1 }
        );
        victims
    }
    /// `RoadVehCheckTrainCrash`, starting with the front part's observation.
    #[expect(
        clippy::inline_always,
        reason = "Reject noncrossing parts in the controller frame."
    )]
    #[inline(always)]
    fn train_crash(&self, v: Part, first: CrashPart) -> bool {
        let mut part = v;
        let mut u = first;
        loop {
            if part.state() != RVSB_WORMHOLE && u.crossing {
                let z = u.z;
                let x = v.v.x();
                let y = v.v.y();
                let hit = self.vs.any_near(x, y, 4, |t| {
                    t.vehicle_type() == crate::vehicle::VehicleType::TRAIN.0
                        && (t).z().wrapping_sub(z).wrapping_abs() <= 6
                });
                if hit {
                    let victims = self.crash(v, false);
                    service!(self.l.crash_news)(v.v, victims);
                    crate::station::spatial::modify_rating_around(
                        self.ss,
                        (v.v).tile(),
                        (v.v).owner(),
                        -160,
                        22,
                    );
                    if (self.ss.disaster_sound)() {
                        (self.vs.play_sound)(v.v, crate::sound_type::SND_12_EXPLOSION);
                    }
                    return true;
                }
            }
            let Some(next) = Part::link(u.next, u.next_state) else {
                return false;
            };
            part = next;
            u = CrashPart {
                next: next.v.next(),
                next_state: next.v.next().map_or(std::ptr::null_mut(), |n| {
                    crate::raw_field::read_field!(n.0.as_ptr(), road).cast()
                }),
                z: next.v.z(),
                crossing: crate::map_func::World::new(self.ss).level_crossing_tile(next.v.tile()),
            };
        }
    }
    /// `RoadVehIsCrashed` with this tick's counter.
    fn crashed(&self, v: Part, tick: u8) -> bool {
        let counter = read_field!(v.s.as_ptr(), fields.crashed_ctr).wrapping_add(1);
        write_field!(v.s.as_ptr(), fields.crashed_ctr, counter);
        if counter == 2 {
            crate::effect::openttd_rust_effect_create_rel(
                v.v.0.as_ptr(),
                4,
                4,
                8,
                crate::effectvehicle_func::EV_EXPLOSION_LARGE,
            );
        } else if counter <= 45 {
            if tick & 7 == 0 {
                const DELTA: [u8; 4] = [7, 0, 0, 1];
                let mut u = Some(v.v);
                while let Some(part) = u {
                    let random = self.random();
                    let direction = (part).direction();
                    part.write_direction((direction + DELTA[(random & 3) as usize]) & 7);
                    (self.gs.physics.viewport)(part, true, true);
                    u = (part).next();
                }
            }
        } else if counter >= 2220 && tick & 31 == 0 {
            let first = Link {
                next: v.v.next(),
                next_state: v.v.next().map_or(std::ptr::null_mut(), |n| {
                    crate::raw_field::read_field!(n.0.as_ptr(), road).cast()
                }),
                next_ground: v.v.next().map_or(std::ptr::null_mut(), VehicleRef::ground),
            };
            let alive = first.next.is_some();
            let mut previous = v;
            let mut last = v;
            let mut next = Part::link(first.next, first.next_state);
            while let Some(part) = next {
                previous = last;
                last = part;
                let r = Link {
                    next: part.v.next(),
                    next_state: part.v.next().map_or(std::ptr::null_mut(), |n| {
                        crate::raw_field::read_field!(n.0.as_ptr(), road).cast()
                    }),
                    next_ground: part
                        .v
                        .next()
                        .map_or(std::ptr::null_mut(), VehicleRef::ground),
                };
                next = Part::link(r.next, r.next_state);
            }
            // SAFETY: The crash chain stays live through unlinking and station copy.
            unsafe {
                self.vs
                    .set_next_record(previous.v.0.as_ptr(), std::ptr::null_mut());
            };
            let station = (v.v).last_station();
            (last.v).write_last_station(station);
            if in_range(last.state(), STOP, STOP + 16) {
                // SAFETY: Live last part owner; no owner access spans the stop update.
                unsafe {
                    crate::road_stop::leave_vehicle(self.ss, last.v, last.s.as_ptr());
                };
            }
            service!(self.vs.destroy)(last.v);
            return alive;
        }
        true
    }
    /// `RoadVehUpdateCache`.
    fn update_cache(&self, v: Part, same_length: bool) {
        // SAFETY: Serialized live records; invalidation finishes before the length reset.
        unsafe {
            crate::newgrf_vehicle::openttd_rust_vehicle_grf_invalidate(
                &*(self.vs.newgrf)(),
                v.v.0.as_ptr(),
                true,
            );
        }
        crate::raw_field::write_field!(v.v.ground_cache(), cached_total_length, 0);
        let mut total: u16 = 0;
        let mut u = v.v;
        loop {
            let first_engine = if u == v.v {
                INVALID_ENGINE
            } else {
                (v.v).engine_id()
            };
            crate::raw_field::write_field!(u.ground_cache(), first_engine, first_engine);
            let engine = service!(self.vs.engine)(u);
            let mut length = VEHICLE_LENGTH;
            let mut factor = if engine.grf_version(self.ss) >= 8 {
                let value = service!(self.vs.property)(
                    (self.vs.newgrf)(),
                    u,
                    0x23,
                    i32::from(CALLBACK_FAILED),
                ) as u16;
                if value != CALLBACK_FAILED && value >= u16::from(VEHICLE_LENGTH) {
                    self.vs
                        .unknown_callback_result(engine, CBID_VEHICLE_LENGTH, value);
                }
                value
            } else {
                service!(self.vs.callback)((self.vs.newgrf)(), u, CBID_VEHICLE_LENGTH)
            };
            if factor == CALLBACK_FAILED {
                factor = u16::from(engine.road_shorten());
            }
            if factor != 0 {
                length -= factor.min(u16::from(VEHICLE_LENGTH - 1)) as u8;
            }
            if same_length
                && length != crate::raw_field::read_field!(u.ground_cache(), cached_veh_length)
            {
                service!(self.vs.length_changed)(u);
            }
            total = total.wrapping_add(u16::from(length));
            crate::raw_field::write_field!(u.ground_cache(), cached_veh_length, length);
            crate::raw_field::write_field!(v.v.ground_cache(), cached_total_length, total);
            crate::vehicle_effect::openttd_rust_visual_update(u, true);
            let period = service!(self.vs.engine)(u).cargo_age();
            let age = service!(self.vs.property)(
                (self.vs.newgrf)(),
                u,
                crate::newgrf_properties::PROP_ROADVEH_CARGO_AGE_PERIOD,
                i32::from(period),
            );
            (u).write_cargo_age(age as u16);
            let Some(next) = (u).next() else {
                break;
            };
            u = next;
        }
        let speed = service!(self.vs.property)((self.vs.newgrf)(), v.v, 0x15, 0) as u32;
        (v.v).write_max_speed(if speed != 0 {
            speed.wrapping_mul(4)
        } else {
            u32::from(service!(self.vs.engine)(v.v).road_speed())
        } as u16);
    }
    /// `RoadFindPathToDest`.
    fn path(&self, v: Part, s: &Step, tile: u32, entry: u8) -> u8 {
        let ts = self.movement.road_tracks(v.v, tile);
        let red = (ts >> 16) & 0x3f3f;
        let mut trackdirs = ts & 0x3f3f;
        let p = PathRead {
            dest: v.v.dest_tile(),
            owner: v.v.owner(),
            articulated: v.v.next().is_some_and(|next| {
                crate::vehicle_base::is_articulated_part(
                    crate::vehicle_base::is_ground_vehicle(next.vehicle_type()),
                    next.subtype(),
                )
            }),
        };
        let bus = crate::road_stop::is_bus(self.ss, v.v);
        let kind = service!(self.ss.tile_type)(tile);
        if kind == MP_ROAD {
            if service!(self.l.is_road_depot_tile)(tile)
                && (service!(self.ss.tile_owner)(tile) != p.owner
                    || service!(self.l.depot_direction)(tile) == entry)
            {
                trackdirs = 0;
            }
        } else if kind == MP_STATION && service!(self.l.is_bay_stop)(tile) {
            if service!(self.ss.tile_owner)(tile) != p.owner
                || service!(self.l.bay_direction)(tile) == entry
                || p.articulated
            {
                trackdirs = 0;
            } else if crate::map_func::World::new(self.ss).road_stop_type(tile) != u8::from(!bus)
                || (!self.queue
                    && !crate::road_stop::has_free_bay(
                        crate::road_stop::by_tile_raw(self.ss, tile, RoadStopType(u8::from(!bus)))
                            .1,
                    ))
            {
                trackdirs = 0;
            }
        }
        trackdirs &= u32::from(crate::track_func::diagdir_reaches_trackdirs(entry));
        let best = if trackdirs == 0 {
            if v.path_len() != 0 {
                v.path_clear();
            }
            REVERSE[usize::from(entry)]
        } else {
            let mut forced = None;
            if read_field!(v.s.as_ptr(), fields.reverse_ctr) != 0 {
                let mut reverse = true;
                if s.tram {
                    let bits = service!(self.l.any_road_bits)(v.v, tile, false);
                    let straight = if entry & 1 == 0 { 10 } else { 5 };
                    reverse = (bits & straight) == straight || bits == (8 >> entry);
                }
                if reverse {
                    write_field!(v.s.as_ptr(), fields.reverse_ctr, 0);
                    if s.tile != tile {
                        forced = Some(REVERSE[usize::from(entry)]);
                    }
                }
            }
            if let Some(dir) = forced {
                dir
            } else if p.dest == 0 {
                let count = trackdirs.count_ones();
                let mut n = crate::random_func::scale_to_limit(self.random(), count);
                let mut bits = trackdirs;
                let mut bit = 0;
                loop {
                    if bits & 1 != 0 {
                        if n == 0 {
                            break;
                        }
                        n -= 1;
                    }
                    bits >>= 1;
                    bit += 1;
                }
                bit
            } else if trackdirs & (trackdirs - 1) == 0 {
                if v.path_len() != 0 && v.path_back().tile == tile {
                    v.path_clear();
                }
                trackdirs.trailing_zeros() as u8
            } else {
                let mut cached = None;
                if v.path_len() != 0 {
                    let item = v.path_back();
                    if item.tile != tile {
                        v.path_clear();
                    } else if trackdirs & (1 << item.trackdir) != 0 {
                        v.path_pop();
                        cached = Some(item.trackdir);
                    } else {
                        v.path_clear();
                    }
                }
                if let Some(dir) = cached {
                    dir
                } else {
                    let r = service!(self.l.choose_track)(v.v, tile, entry, trackdirs as u16);
                    service!(self.vs.pathfinding_result)(v.v, r.found);
                    r.trackdir
                }
            }
        };
        if red & (1 << best) != 0 {
            INVALID_TRACKDIR
        } else {
            best
        }
    }
    /// `RoadVehLeaveDepot`.
    fn leave_depot(&self, v: Part, ground: crate::ground_vehicle::Owner, first: bool) -> bool {
        if v.state() != DEPOT {
            return false;
        }
        let d = PartRead {
            next: v.v.next(),
            next_state: v.v.next().map_or(std::ptr::null_mut(), |n| {
                crate::raw_field::read_field!(n.0.as_ptr(), road).cast()
            }),
            tile: v.v.tile(),
            z: v.v.z(),
            tram: (crate::movement::table(self.ss).road_kind)(v.v).0,
        };
        let mut next = Part::link(d.next, d.next_state);
        while let Some(u) = next {
            if u.state() != DEPOT {
                return false;
            }
            let r = LinkTile {
                link: Link {
                    next: u.v.next(),
                    next_state: u.v.next().map_or(std::ptr::null_mut(), |n| {
                        crate::raw_field::read_field!(n.0.as_ptr(), road).cast()
                    }),
                    next_ground: u.v.next().map_or(std::ptr::null_mut(), VehicleRef::ground),
                },
                tile: u.v.tile(),
            };
            if r.tile != d.tile {
                return false;
            }
            next = Part::link(r.link.next, r.link.next_state);
        }
        let dir = service!(self.l.depot_direction)(d.tile);
        let direction = dir * 2 + 1;
        (v.v).write_direction(direction);
        let tdir = crate::track_func::diag_dir_to_diag_trackdir(dir, |ok| {
            crate::movement::native_assert(self.ss, ok);
        });
        let rd = road_data::entry(d.tram, self.side() + usize::from(tdir), 6);
        let (x, y) = self.tile_xy(d.tile, (rd.0 & 15, rd.1 & 15));
        if first {
            if service!(self.vs.order_type)(v.v) == OT_GOTO_DEPOT && d.tile == (v.v).dest_tile() {
                self.vs.enter_depot(v.v);
                return true;
            }
            if self.close(v, d.z, x, y, direction, false).is_some() {
                return true;
            }
            unsafe { self.vs.maintain(v.v, maintenance::service_in_depot) };
            service!(self.vs.leave_unbunching_depot)(v.v);
            self.start_sound(v);
            (v.v).write_speed(0);
        }
        v.v.set_hidden(false);
        write_field!(v.s.as_ptr(), fields.state, tdir);
        write_field!(v.s.as_ptr(), fields.frame, 6);
        v.v.write_x(x);
        v.v.write_y(y);
        // SAFETY: The newly visible shell survives depot exit.
        unsafe {
            crate::vehicle::occupancy::update_position(self.vs, v.v, self.map());
        }
        crate::ground_vehicle::update_inclination_owned(
            self.gs.physics,
            &self.movement,
            v.v,
            ground,
            true,
            true,
        );
        service!(self.l.depot_exit_dirty)(v.v);
        true
    }
    /// `FollowPreviousRoadVehicle`.
    fn follow(&self, v: Part, s: &Step, prev: Part, tile: u32, entry: u8, reversed: bool) -> u8 {
        let prev_tile = (prev.v).tile();
        if prev_tile == s.tile && !reversed {
            return REVERSE[usize::from(entry)];
        }
        let state = prev.state();
        let dir;
        if state == RVSB_WORMHOLE || state == DEPOT {
            let diagonal = if service!(self.ss.tile_type)(tile) == MP_TUNNELBRIDGE {
                service!(self.ss.tunnel_bridge_direction)(tile)
            } else if service!(self.l.is_road_depot_tile)(tile) {
                service!(self.l.depot_direction)(tile) ^ 2
            } else {
                return INVALID_TRACKDIR;
            };
            dir = crate::track_func::diag_dir_to_diag_trackdir(diagonal, |ok| {
                crate::movement::native_assert(self.ss, ok);
            });
        } else if reversed && (prev_tile != tile || (state < 16 && reversing(state))) {
            let north = if prev_tile != tile {
                prev_tile < tile
            } else {
                state == 15 || state == 6
            };
            dir = [[10, 13, 12, 2], [5, 11, 3, 4]][usize::from(!north)][usize::from(entry ^ 2)];
        } else if state & 64 != 0 {
            dir = state & 9;
        } else if state < 16 {
            dir = state;
        } else {
            return INVALID_TRACKDIR;
        }
        const REQUIRED: [u8; 8] = [10, 5, 9, 6, 3, 12, 10, 5];
        let bits = service!(self.l.any_road_bits)(v.v, tile, true);
        if REQUIRED[usize::from(dir & 7)] & bits == 0 {
            INVALID_TRACKDIR
        } else {
            dir
        }
    }
    /// `RoadVehArrivesAt`, `BeginLoading` and the road stop arrival triggers.
    fn arrive_load(&self, v: Part, s: &Step, station: u16) {
        let r = ArrivalRead {
            had_vehicle_of_type: service!(self.ss.station_had_vehicle_of_type)(station),
        };
        let bus = crate::road_stop::is_bus(self.ss, v.v);
        let bit = if bus { HVOT_BUS } else { HVOT_TRUCK };
        // Check if station was ever visited before
        if r.had_vehicle_of_type & bit == 0 {
            let headline = if bus { 0 } else { 2 } + u8::from(s.tram);
            service!(self.ss.station_add_vehicle_of_type)(station, bit);
            service!(self.l.arrival_news)(v.v, station, headline);
        }
        crate::vehicle::visits::begin_loading(v.v, self.vs.visits());
        crate::animated_tile::road_event(
            crate::animated_tile::station_base(station),
            (v.v).tile(),
            crate::station_type::SAT_VEHICLE_ARRIVES,
            crate::station_type::SRT_VEHICLE_ARRIVES,
        );
    }
    /// Drive-through stop test shared by the blocked and stop-frame paths.
    fn stops_here(&self, v: Part, tile: u32) -> bool {
        let owner = (v.v).owner();
        let bus = crate::road_stop::is_bus(self.ss, v.v);
        service!(self.l.should_stop)(v.v, tile)
            && owner == service!(self.ss.tile_owner)(tile)
            && crate::map_func::World::new(self.ss).road_stop_type(tile) == u8::from(!bus)
    }
    /// `IndividualRoadVehicleController`; `s` is a fresh observation of `v`.
    fn individual(
        &self,
        v: Part,
        ground: crate::ground_vehicle::Owner,
        prev: Option<Part>,
        s: &mut Step,
    ) -> bool {
        if v.overtaking() != 0 {
            if service!(self.ss.tile_type)(s.tile) == MP_STATION {
                write_field!(v.s.as_ptr(), fields.overtaking, 0);
            } else {
                let counter = read_field!(v.s.as_ptr(), fields.overtaking_ctr).wrapping_add(1);
                write_field!(v.s.as_ptr(), fields.overtaking_ctr, counter);
                if counter >= 35 && v.state() < STOP && straight(v.state()) {
                    write_field!(v.s.as_ptr(), fields.overtaking, 0);
                }
            }
        }
        if v.state() == DEPOT {
            return true;
        }
        if v.state() == RVSB_WORMHOLE {
            let gp = service!(self.vs.new_position)(v.v);
            if s.front {
                if let Some(u) = self.close(v, s.z, gp.x, gp.y, s.direction, true) {
                    Self::set_speed(v, s, u.first_speed);
                    return false;
                }
            }
            if service!(self.ss.tile_type)(gp.tile) == MP_TUNNELBRIDGE {
                let e = self.movement_enter(v.v, gp.tile, gp.x, gp.y);
                Self::observe(s, e);
                if e.flags & VETS_ENTERED_WORMHOLE != 0 {
                    self.move_without_speed(v, ground, s, gp.x, gp.y);
                    return true;
                }
            }
            service!(v.v).write_xy(gp.x, gp.y);
            // SAFETY: The live road shell survives the synchronous index update.
            unsafe {
                crate::vehicle::occupancy::update_position(self.vs, v.v, self.map());
            }
            let status = (v.v).status();
            s.x = gp.x;
            s.y = gp.y;
            if status & VS_HIDDEN == 0 {
                service!(self.vs.base_viewport)(v.v);
            }
            return true;
        }
        let state = v.state();
        let movement = if state & 64 != 0 { state & 9 } else { state };
        let rd = road_data::entry(
            s.tram,
            (usize::from(movement) + self.side()) ^ usize::from(v.overtaking()),
            usize::from(v.frame()) + 1,
        );
        let entry = rd.0 & 3;
        if rd.0 & 128 != 0 {
            let mut tile = (TileIndex(s.tile)
                + self
                    .map()
                    .tile_offs_by_diag_dir(DiagDirection(entry), crate::map_func::native_check))
            .0;
            let mut dir = if s.front {
                if self.has_road(v.v, tile) {
                    self.path(v, s, tile, entry)
                } else {
                    REVERSE[usize::from(entry)]
                }
            } else {
                // Non-front parts are only driven by the consist loop, which supplies prev.
                prev.map_or(INVALID_TRACKDIR, |p| {
                    self.follow(v, s, p, tile, entry, false)
                })
            };
            if dir == INVALID_TRACKDIR {
                if !s.front {
                    service!(self.l.disconnect)();
                }
                Self::set_speed(v, s, 0);
                return false;
            }
            loop {
                let mut start_frame = 0;
                if reversing(dir) {
                    write_field!(v.s.as_ptr(), fields.overtaking, 0);
                    if s.tram {
                        let needed: u8 = [2, 1, 8, 4][usize::from(((dir >> 2) & 2) | (dir & 1))];
                        let big = v
                            .v
                            .previous()
                            .is_some_and(|previous| previous.tile() == tile)
                            || (s.front
                                && crate::map_func::World::new(self.ss).is_normal_road_tile(tile)
                                && !crate::map_func::World::new(self.ss).has_road_works(tile)
                                && self.has_road(v.v, tile)
                                && needed
                                    & crate::map_func::World::new(self.ss).road_bits(tile, true)
                                    != 0);
                        if !big {
                            if !s.front
                                || !self.can_build_tram(v.v, tile, needed)
                                || (!needed & service!(self.l.any_road_bits)(v.v, s.tile, false))
                                    == 0
                            {
                                tile = s.tile;
                                start_frame = 16;
                            } else {
                                Self::set_speed(v, s, 0);
                                return false;
                            }
                        }
                    } else if crate::map_func::World::new(self.ss).is_normal_road_tile(s.tile)
                        && service!(self.l.disallowed_directions)(s.tile) != 0
                    {
                        Self::set_speed(v, s, 0);
                        return false;
                    } else {
                        tile = s.tile;
                    }
                }
                let pos = road_data::entry(
                    s.tram,
                    (usize::from(dir) + self.side()) ^ usize::from(v.overtaking()),
                    start_frame.into(),
                );
                let (x, y) = self.tile_xy(tile, pos);
                let new_dir = Self::sliding(s, x, y);
                if s.front {
                    if let Some(u) = self.close(v, s.z, x, y, new_dir, true) {
                        Self::set_speed(v, s, u.first_speed);
                        v.path_push(dir, tile);
                        return false;
                    }
                }
                let e = self.movement_enter(v.v, tile, x, y);
                Self::observe(s, e);
                if e.flags & VETS_CANNOT_ENTER != 0 {
                    if service!(self.ss.tile_type)(tile) != MP_TUNNELBRIDGE {
                        Self::set_speed(v, s, 0);
                        return false;
                    }
                    dir = REVERSE[usize::from(entry)];
                    continue;
                }
                let state = v.state();
                if in_range(state, STOP, DT_STOP + 16)
                    && service!(self.ss.tile_type)(s.tile) == MP_STATION
                {
                    if reversing(dir) && in_range(state, STOP, STOP + 16) {
                        Self::set_speed(v, s, 0);
                        return false;
                    }
                    if crate::map_func::World::new(self.ss).drive_through_stop_tile(s.tile)
                        && crate::road_stop::continuation(self.ss, s.tile, tile)
                        && s.tile != tile
                    {
                        dir = v.state();
                    } else if service!(self.l.is_station_road_stop)(s.tile) {
                        // SAFETY: Live front owner; no owner access spans the stop update.
                        unsafe {
                            crate::road_stop::leave_vehicle(self.ss, v.v, v.s.as_ptr());
                        };
                    }
                }
                if e.flags & VETS_ENTERED_WORMHOLE == 0 {
                    let old_tile = v.v.tile();
                    v.v.write_tile(tile);
                    let tram = (self.movement.get().road_kind)(v.v).0;
                    let world = crate::map_func::World::new(self.ss);
                    let before = world.road_type(old_tile, tram);
                    let after = world.road_type(tile, tram);
                    s.tile = tile;
                    write_field!(v.s.as_ptr(), fields.state, dir);
                    write_field!(v.s.as_ptr(), fields.frame, start_frame);
                    if before != after {
                        if s.front {
                            self.update_cache(v, false);
                        }
                        crate::ground_vehicle::cargo_changed(self.gs.physics, v.v.first());
                    }
                }
                if new_dir != s.direction {
                    self.set_direction(v, s, new_dir);
                }
                self.move_to(v, ground, s, (x, y), true, true);
                return true;
            }
        }
        if rd.0 & 64 != 0 {
            let mut start_frame = 1;
            let dir = if s.tram && !service!(self.l.is_road_depot_tile)(s.tile) && {
                let bits = service!(self.l.any_road_bits)(v.v, s.tile, true);
                bits != 0 && bits & (bits - 1) == 0
            } {
                start_frame = 21;
                [14, 15, 6, 7][usize::from(entry)]
            } else if s.front {
                self.path(v, s, s.tile, entry)
            } else {
                prev.map_or(INVALID_TRACKDIR, |p| {
                    self.follow(v, s, p, s.tile, entry, true)
                })
            };
            if dir == INVALID_TRACKDIR {
                Self::set_speed(v, s, 0);
                return false;
            }
            let pos = road_data::entry(s.tram, self.side() + usize::from(dir), start_frame.into());
            let (x, y) = self.tile_xy(s.tile, pos);
            let new_dir = Self::sliding(s, x, y);
            if s.front {
                if let Some(u) = self.close(v, s.z, x, y, new_dir, true) {
                    Self::set_speed(v, s, u.first_speed);
                    v.path_push(dir, s.tile);
                    return false;
                }
            }
            let e = self.movement_enter(v.v, s.tile, x, y);
            Self::observe(s, e);
            if e.flags & VETS_CANNOT_ENTER != 0 {
                Self::set_speed(v, s, 0);
                return false;
            }
            write_field!(v.s.as_ptr(), fields.state, dir);
            write_field!(v.s.as_ptr(), fields.frame, start_frame);
            if new_dir != s.direction {
                self.set_direction(v, s, new_dir);
            }
            self.move_to(v, ground, s, (x, y), true, true);
            return true;
        }
        let link = Link {
            next: v.v.next(),
            next_state: v.v.next().map_or(std::ptr::null_mut(), |n| {
                crate::raw_field::read_field!(n.0.as_ptr(), road).cast()
            }),
            next_ground: v.v.next().map_or(std::ptr::null_mut(), VehicleRef::ground),
        };
        if let Some(next) = Part::link(link.next, link.next_state) {
            if service!(self.l.is_road_depot_tile)(s.tile)
                && u16::from(v.frame()) == u16::from(s.length) + 6
            {
                self.leave_depot(next, next.ground(link.next_ground), false);
            }
        }
        let x = (s.x & !15) + i32::from(rd.0 & 15);
        let y = (s.y & !15) + i32::from(rd.1 & 15);
        let new_dir = Self::sliding(s, x, y);
        if s.front && !in_range(v.state(), STOP, STOP + 16) {
            if let Some(c) = self.close(v, s.z, x, y, new_dir, true) {
                let u = Part::new(c.first, c.first_state);
                if v.overtaking() == 0 {
                    self.overtake(v, s, u, u.ground(c.first_ground));
                }
                if v.overtaking() == 0 {
                    Self::set_speed(v, s, c.first_speed);
                }
                if s.cur_speed == 0
                    && in_range(v.state(), DT_STOP, DT_STOP + 16)
                    && service!(self.l.should_stop)(v.v, s.tile)
                {
                    let owner = (v.v).owner();
                    let bus = crate::road_stop::is_bus(self.ss, v.v);
                    if owner == service!(self.ss.tile_owner)(s.tile)
                        && service!(self.vs.order_type)(v.v) != OT_LEAVESTATION
                        && crate::map_func::World::new(self.ss).road_stop_type(s.tile)
                            == u8::from(!bus)
                    {
                        let station = service!(self.ss.station_index)(s.tile);
                        (v.v).write_last_station(station);
                        self.arrive_load(v, s, station);
                    }
                }
                return false;
            }
        }
        if new_dir != s.direction {
            if self.original() {
                Self::set_speed(v, s, s.cur_speed - (s.cur_speed >> 2));
            }
            v.v.write_direction(new_dir);
            (self.gs.physics.viewport)(v.v, true, true);
            s.direction = new_dir;
            return true;
        }
        let state = v.state();
        let frame = v.frame();
        // The frame test is pure and decides most drive-through passes without a read.
        if s.front
            && ((in_range(state, STOP, STOP + 16)
                && road_data::_ROAD_STOP_STOP_FRAME[usize::from(state - STOP) + self.side()]
                    == frame)
                || (in_range(state, DT_STOP, DT_STOP + 16)
                    && frame == 11
                    && self.stops_here(v, s.tile)))
        {
            let rs = crate::road_stop::at_raw(self.ss, s.tile);
            let station = service!(self.ss.station_index)(s.tile);
            if state & 4 == 0 {
                if crate::map_func::World::new(self.ss).drive_through_stop_tile(s.tile) {
                    let next = self
                        .map()
                        .tile_add_by_dir(
                            TileIndex(s.tile),
                            Direction(s.direction),
                            crate::map_func::native_check,
                        )
                        .0;
                    if crate::road_stop::continuation(self.ss, s.tile, next)
                        && self.has_road(v.v, next)
                    {
                        write_field!(v.s.as_ptr(), fields.frame, frame.wrapping_add(1));
                        self.move_to(v, ground, s, (x, y), true, false);
                        return true;
                    }
                }
                crate::road_stop::set_entrance_busy(rs, false);
                write_field!(v.s.as_ptr(), fields.state, state | 4);
                (v.v).write_last_station(station);
                if crate::map_func::World::new(self.ss).drive_through_stop_tile(s.tile)
                    || (service!(self.vs.order_type)(v.v) == OT_GOTO_STATION
                        && service!(self.vs.order_destination)(v.v) == station)
                {
                    self.arrive_load(v, s, station);
                    return false;
                }
            } else {
                if crate::road_stop::is_entrance_busy(rs) {
                    Self::set_speed(v, s, 0);
                    return false;
                }
                if service!(self.vs.order_type)(v.v) == OT_LEAVESTATION {
                    service!(self.vs.order_free)(v.v);
                }
            }
            if service!(self.l.is_bay_stop)(s.tile) {
                crate::road_stop::set_entrance_busy(rs, true);
            }
            self.start_sound(v);
            service!(self.vs.start_stop_dirty)(v.v);
        }
        let e = self.movement_enter(v.v, s.tile, x, y);
        Self::observe(s, e);
        if e.flags & VETS_CANNOT_ENTER != 0 {
            Self::set_speed(v, s, 0);
            return false;
        }
        if e.order_type == OT_LEAVESTATION
            && crate::map_func::World::new(self.ss).drive_through_stop_tile(s.tile)
        {
            service!(self.vs.order_free)(v.v);
        }
        if e.flags & VETS_ENTERED_WORMHOLE == 0 {
            write_field!(v.s.as_ptr(), fields.frame, v.frame().wrapping_add(1));
        }
        self.move_to(v, ground, s, (x, y), false, true);
        true
    }
    /// Wormhole entry: x/y, `UpdatePosition`, `UpdateInclination(true, true)`.
    fn move_without_speed(
        &self,
        v: Part,
        ground: crate::ground_vehicle::Owner,
        s: &mut Step,
        x: i32,
        y: i32,
    ) {
        let m = move_incline(
            self.gs,
            &self.movement,
            self.map(),
            v.v,
            ground,
            (x, y),
            true,
            true,
        );
        s.x = x;
        s.y = y;
        s.z = m.z;
        s.cur_speed = m.cur_speed;
    }
    /// One `IndividualRoadVehicleController` pass over the consist.
    fn consist_step(
        &self,
        v: Part,
        ground: crate::ground_vehicle::Owner,
        s: &mut Step,
    ) -> Option<bool> {
        if !self.individual(v, ground, None, s) {
            return None;
        }
        let mut prev = v;
        let mut link = Link {
            next: v.v.next(),
            next_state: v.v.next().map_or(std::ptr::null_mut(), |n| {
                crate::raw_field::read_field!(n.0.as_ptr(), road).cast()
            }),
            next_ground: v.v.next().map_or(std::ptr::null_mut(), VehicleRef::ground),
        };
        let mut next = Part::link(link.next, link.next_state);
        let traversed_tail = next.is_some();
        while let Some(u) = next {
            let mut us = Step {
                tile: u.v.tile(),
                x: u.v.x(),
                y: u.v.y(),
                z: u.v.z(),
                cur_speed: u.v.speed(),
                direction: u.v.direction(),
                length: crate::raw_field::read_field!(u.v.ground_cache(), cached_veh_length),
                front: crate::vehicle_base::is_front_engine(
                    crate::vehicle_base::is_ground_vehicle(u.v.vehicle_type()),
                    u.v.subtype(),
                ),
                tram: (crate::movement::table(self.ss).road_kind)(u.v).0,
            };
            if !self.individual(u, u.ground(link.next_ground), Some(prev), &mut us) {
                return None;
            }
            prev = u;
            link = Link {
                next: u.v.next(),
                next_state: u.v.next().map_or(std::ptr::null_mut(), |n| {
                    crate::raw_field::read_field!(n.0.as_ptr(), road).cast()
                }),
                next_ground: u.v.next().map_or(std::ptr::null_mut(), VehicleRef::ground),
            };
            next = Part::link(link.next, link.next_state);
        }
        Some(traversed_tail)
    }
    /// `RoadVehController`.
    fn controller(
        &self,
        v: Part,
        ground: crate::ground_vehicle::Owner,
        status: u8,
        tick: u8,
    ) -> bool {
        let reverse = read_field!(v.s.as_ptr(), fields.reverse_ctr);
        if reverse != 0 {
            write_field!(v.s.as_ptr(), fields.reverse_ctr, reverse - 1);
        }
        if status & VS_CRASHED != 0
            || self.train_crash(
                v,
                CrashPart {
                    next: v.v.next(),
                    next_state: v.v.next().map_or(std::ptr::null_mut(), |n| {
                        crate::raw_field::read_field!(n.0.as_ptr(), road).cast()
                    }),
                    z: v.v.z(),
                    crossing: crate::map_func::World::new(self.ss).level_crossing_tile(v.v.tile()),
                },
            )
        {
            return self.crashed(v, tick);
        }
        let b = if crate::witness::enabled() {
            unsafe { maintenance::handle_breakdown::<true>(v.v, self.vs) }
        } else {
            unsafe { maintenance::handle_breakdown::<false>(v.v, self.vs) }
        };
        if b.broken {
            return true;
        }
        if b.status & VS_STOPPED != 0 {
            crate::ground_vehicle::set_last_speed_owned(self.gs.physics, v.v, ground);
            return true;
        }
        // SAFETY: The getter returns the immutable process-wide orders table.
        let orders = unsafe { &*service!(self.vs.orders)() };
        // SAFETY: The original live shell survives orders; no owner borrow spans reentry.
        unsafe { crate::orders::openttd_rust_process_orders(v.v.shell().cast(), orders) };
        self.vs.handle_loading(v.v, false, orders);
        if service!(self.vs.order_type)(v.v) == OT_LOADING {
            return true;
        }
        if v.state() == DEPOT {
            if service!(self.vs.waiting_for_unbunching)(v.v) {
                return true;
            }
            if self.leave_depot(v, ground, true) {
                return true;
            }
        }
        let r = self.visual_effect_and_speed(v);
        let mut s = Step {
            tile: r.max.tile,
            x: r.x,
            y: r.y,
            z: r.z,
            cur_speed: 0,
            direction: r.max.direction,
            length: r.length,
            front: true,
            tram: r.tram,
        };
        let (mut j, speed) = self.update_speed(v, ground, &r);
        s.cur_speed = speed;
        let mut adv = advance_distance(s.direction);
        let mut blocked = false;
        while j >= adv {
            j -= adv;
            let Some(traversed_tail) = self.consist_step(v, ground, &mut s) else {
                blocked = true;
                break;
            };
            if traversed_tail {
                // A traversed tail may have changed the front, including by detaching itself.
                s = Step {
                    tile: v.v.tile(),
                    x: v.v.x(),
                    y: v.v.y(),
                    z: v.v.z(),
                    cur_speed: v.v.speed(),
                    direction: v.v.direction(),
                    length: crate::raw_field::read_field!(v.v.ground_cache(), cached_veh_length),
                    front: crate::vehicle_base::is_front_engine(
                        crate::vehicle_base::is_ground_vehicle(v.v.vehicle_type()),
                        v.v.subtype(),
                    ),
                    tram: (crate::movement::table(self.ss).road_kind)(v.v).0,
                };
            }
            adv = advance_distance(s.direction);
            if j >= adv {
                let crossing = v.state() != RVSB_WORMHOLE
                    && crate::map_func::World::new(self.ss).level_crossing_tile(s.tile);
                let link = Link {
                    next: v.v.next(),
                    next_state: v.v.next().map_or(std::ptr::null_mut(), |n| {
                        crate::raw_field::read_field!(n.0.as_ptr(), road).cast()
                    }),
                    next_ground: v.v.next().map_or(std::ptr::null_mut(), VehicleRef::ground),
                };
                let first = CrashPart {
                    next: link.next,
                    next_state: link.next_state,
                    z: s.z,
                    crossing,
                };
                if self.train_crash(v, first) {
                    break;
                }
            }
        }
        crate::ground_vehicle::set_last_speed_owned(self.gs.physics, v.v, ground);
        let mut part = Some(v.v);
        while let Some(u) = part {
            if u.status() & VS_HIDDEN == 0 {
                service!(self.gs.update_viewport)(u);
            }
            part = u.next();
        }
        let progress = v.v.progress();
        if progress == 0 {
            (v.v).write_progress(if blocked { adv - 1 } else { j } as u8);
        }
        true
    }
    /// `RoadVehicle::Tick`.
    fn tick(&mut self, v: Part, entry: &Entry) -> bool {
        let tick = v.v.tick().wrapping_add(1);
        if !crate::vehicle_base::is_front_engine(
            crate::vehicle_base::is_ground_vehicle(v.v.vehicle_type()),
            v.v.subtype(),
        ) {
            v.v.write_tick(tick);
            return true;
        }
        self.acceleration_model = entry.acceleration_model;
        self.road_side = entry.road_side;
        self.queue = entry.queue;
        self.map_log_x = u32::from(entry.map_log_x);
        self.map.take();
        let running = if v.v.status() & VS_STOPPED == 0 {
            v.v.running().wrapping_add(1)
        } else {
            v.v.running()
        };
        let order_time = v.v.order_time().wrapping_add(1);
        v.v.write_tick(tick);
        v.v.write_running(running);
        v.v.write_order_time(order_time);
        self.controller(v, v.ground(entry.ground), v.v.status(), tick)
    }
    /// `RoadVehicle::SetDestTile`.
    fn set_dest(v: Part, tile: u32) {
        if tile == v.v.dest_tile() {
            return;
        }
        v.path_clear();
        (v.v).write_dest(tile);
    }
    /// `RoadVehicle::GetRunningCost`.
    fn running_cost(&self, v: Part) -> i64 {
        let cap = unsafe { (&*(self.vs.newgrf)()).capabilities };
        let e = (self.vs.engine)(v.v).0.as_ptr();
        if crate::vehicle::EngineRef::of(e).road_price() == INVALID_PRICE {
            return 0;
        }
        let factor = service!(self.vs.property)(
            (self.vs.newgrf)(),
            v.v,
            crate::newgrf_properties::PROP_ROADVEH_RUNNING_COST_FACTOR,
            i32::from(crate::vehicle::EngineRef::of(e).road_running()),
        ) as u32;
        if factor == 0 {
            return 0;
        }
        crate::engine_capabilities::price(
            cap,
            e,
            u32::from(crate::vehicle::EngineRef::of(e).road_price()),
            factor,
            0,
        )
    }
    /// `CheckIfRoadVehNeedsService`; `IsChainInDepot` and `FindClosestRoadDepot` inline.
    fn service(&self, v: Part) {
        let r = ServiceRead {
            tile: v.v.tile(),
            servint: {
                // SAFETY: Base maintenance names the shared static command and company tables.
                let command = unsafe { &*(*(*self.vs.maintenance).transaction).commands };
                let company = unsafe { &*(command.company)() };
                let settings = (company.settings)();
                let defaults = (settings.company_vehicle_defaults)(crate::company_type::CompanyID(
                    v.v.owner(),
                ));
                crate::raw_field::read_field!(defaults, servint_roadveh)
            },
            cur_speed: v.v.speed(),
        };
        if r.servint == 0
            || !unsafe {
                self.vs
                    .maintain(v.v, maintenance::needs_automatic_servicing)
            }
        {
            return;
        }
        let depot_tile = service!(self.l.is_road_depot_tile)(r.tile);
        let mut all_in_depot = depot_tile && r.cur_speed == 0;
        if all_in_depot && v.state() != DEPOT {
            all_in_depot = false;
        }
        if all_in_depot {
            let mut next = {
                let p = Link {
                    next: v.v.next(),
                    next_state: v.v.next().map_or(std::ptr::null_mut(), |n| {
                        crate::raw_field::read_field!(n.0.as_ptr(), road).cast()
                    }),
                    next_ground: v.v.next().map_or(std::ptr::null_mut(), VehicleRef::ground),
                };
                Part::link(p.next, p.next_state)
            };
            while let Some(u) = next {
                let p = LinkTile {
                    link: Link {
                        next: u.v.next(),
                        next_state: u.v.next().map_or(std::ptr::null_mut(), |n| {
                            crate::raw_field::read_field!(n.0.as_ptr(), road).cast()
                        }),
                        next_ground: u.v.next().map_or(std::ptr::null_mut(), VehicleRef::ground),
                    },
                    tile: u.v.tile(),
                };
                if u.state() != DEPOT || p.tile != r.tile {
                    all_in_depot = false;
                    break;
                }
                next = Part::link(p.link.next, p.link.next_state);
            }
        }
        if all_in_depot {
            unsafe { self.vs.maintain(v.v, maintenance::service_in_depot) };
            return;
        }
        // SAFETY: The catalog addresses the process-wide setting; one field-sized load.
        let penalty = unsafe { self.ss.settings().maximum_go_to_depot_penalty.read() };
        let depot = if depot_tile {
            DepotResult {
                tile: r.tile,
                length: 0,
            }
        } else {
            service!(self.l.find_depot)(v.v, penalty as i32)
        };
        if depot.length == u32::MAX || depot.length > penalty {
            if service!(self.vs.order_type)(v.v) == OT_GOTO_DEPOT {
                service!(self.vs.order_dummy)(v.v);
            }
            return;
        }
        let depot_id = service!(self.ss.depot_index)(depot.tile);
        if service!(self.vs.order_type)(v.v) == OT_GOTO_DEPOT
            && service!(self.vs.order_nonstop)(v.v) & 1 != 0
            && !crate::random_func::chance16_i(1, 20, self.random())
        {
            return;
        }
        let ground = v.v.ground();
        let flags = crate::raw_field::read_field!(ground, flags);
        crate::raw_field::write_field!(
            ground,
            flags,
            flags | (1 << crate::ground_vehicle::GVF_SUPPRESS_IMPLICIT_ORDERS)
        );
        // SAFETY: The current order belongs to this live serialized record.
        unsafe {
            crate::orders::openttd_rust_order_depot(
                v.v.current_order_pointer(),
                depot_id,
                crate::order_type::ODTF_SERVICE,
                crate::order_type::ONSF_NO_INTERMEDIATE,
                0,
                crate::cargo_type::CARGO_NO_REFIT,
            );
        }
        Self::set_dest(v, depot.tile);
        service!(self.vs.start_stop_dirty)(v.v);
    }
    /// `RoadVehicle::OnNewEconomyDay`.
    fn economy_day(&self, v: Part) {
        unsafe { self.vs.maintain(v.v, maintenance::economy_age_vehicle) };
        let day = (v.v).day().wrapping_add(1);
        (v.v).write_day(day);
        if day & 7 == 0 {
            unsafe { self.vs.maintain(v.v, maintenance::decrease_vehicle_value) };
        }
        if read_field!(v.s.as_ptr(), fields.blocked_ctr) == 0 {
            unsafe { self.vs.maintain(v.v, maintenance::check_vehicle_breakdown) };
        }
        self.service(v);
        service!(self.vs.check_orders_only)(v.v);
        let running = v.v.running();
        if running == 0 {
            return;
        }
        let cost = self.running_cost(v).saturating_mul(i64::from(running)) / YEAR_TICKS;
        let profit = service!(self.vs.profit)(v.v).saturating_sub(cost);
        service!(self.vs.write_profit)(v.v, profit);
        v.v.write_running(0);
        service!(self.l.pay_running)(v.v, cost);
    }
    /// `CmdTurnRoadVeh` after the ownership checks.
    fn turn(&self, v: Part, execute: bool) -> bool {
        let r = TurnRead {
            tile: v.v.tile(),
            breakdown_ctr: crate::raw_field::read_field!(v.v.0.as_ptr(), breakdown_ctr),
            direction: v.v.direction(),
            order_type: v.v.current_order().kind(),
            status: v.v.status(),
        };
        if r.status & (VS_STOPPED | VS_CRASHED) != 0
            || r.breakdown_ctr != 0
            || v.overtaking() != 0
            || v.state() == RVSB_WORMHOLE
            || v.state() == DEPOT
            || r.order_type == OT_LOADING
        {
            return false;
        }
        if crate::map_func::World::new(self.ss).is_normal_road_tile(r.tile)
            && service!(self.l.disallowed_directions)(r.tile) != 0
        {
            return false;
        }
        if service!(self.ss.tile_type)(r.tile) == MP_TUNNELBRIDGE
            && diag(r.direction, |ok| {
                crate::movement::native_assert(self.ss, ok);
            }) == service!(self.ss.tunnel_bridge_direction)(r.tile)
        {
            return false;
        }
        if execute {
            write_field!(v.s.as_ptr(), fields.reverse_ctr, 180);
            service!(self.vs.reset_depot_unbunching)(v.v);
        }
        true
    }
    /// `RoadVehicle::GetVehicleTrackdir`.
    fn trackdir(&self, v: Part) -> u8 {
        let r = TrackdirRead {
            tile: v.v.tile(),
            direction: v.v.direction(),
            status: v.v.status(),
        };
        if r.status & VS_CRASHED != 0 {
            return INVALID_TRACKDIR;
        }
        if v.state() == DEPOT {
            return crate::track_func::diag_dir_to_diag_trackdir(
                service!(self.l.depot_direction)(r.tile),
                |ok| crate::movement::native_assert(self.ss, ok),
            );
        }
        if service!(self.l.is_bay_stop)(r.tile) {
            return crate::track_func::diag_dir_to_diag_trackdir(
                service!(self.l.bay_direction)(r.tile),
                |ok| crate::movement::native_assert(self.ss, ok),
            );
        }
        let state = v.state();
        if state > 15 {
            return crate::track_func::diag_dir_to_diag_trackdir(
                diag(r.direction, |ok| {
                    crate::movement::native_assert(self.ss, ok);
                }),
                |ok| crate::movement::native_assert(self.ss, ok),
            );
        }
        if reversing(state) { state - 6 } else { state }
    }
    /// `RoadVehicle::HasToUseGetSlopePixelZ`.
    fn slope_pixel(v: VehicleRef, first: Part, direction: u8) -> bool {
        let state = first.state();
        if state <= 15 && reversing(state) {
            return true;
        }
        let mut u = first.v;
        while u != v {
            if direction != (u).direction() {
                return true;
            }
            // The walk reaches `v` before the chain ends, as in the original.
            let Some(next) = (u).next() else {
                return false;
            };
            u = next;
        }
        false
    }
}

macro_rules! entry {
    ($leaves:expr, $entry:expr) => {{
        crate::witness::hit(crate::witness::ROAD_ENTRIES);
        // SAFETY: Immutable tables outlive this synchronous call and nested entries;
        // owner places are accessed only through raw pointers between services.
        unsafe { Game::new($leaves, $entry) }
    }};
}

/// The native position mutation ends before the Rust inclination update.
pub(crate) struct Moved {
    pub old_z: i32,
    pub z: i32,
    pub cur_speed: u16,
    pub max_track_speed: u16,
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn move_incline(
    gs: &crate::vehicle::GroundServices,
    movement: &crate::movement::CachedTable<'_>,
    map: crate::map_func::Map,
    v: VehicleRef,
    o: crate::ground_vehicle::Owner,
    pos: (i32, i32),
    new_tile: bool,
    delta: bool,
) -> Moved {
    // Road supplies dimensions for its synchronous no-reset/no-yield interval.
    let s = gs.physics;
    (v).write_xy(pos.0, pos.1);
    // SAFETY: Live shell; index writes end before inclination callbacks.
    unsafe {
        crate::vehicle::occupancy::update_position(s.vehicle, v, map);
    }
    let old_z = crate::ground_vehicle::update_inclination_owned(s, movement, v, o, new_tile, delta);
    Moved {
        old_z,
        z: (v).z(),
        cur_speed: (v).speed(),
        max_track_speed: read_field!(o.state, cache.cached_max_track_speed),
    }
}

/// # Safety
/// Live handle/owner pair, entry record and tables for this synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_tick_abi(
    v: VehicleRef,
    state: *mut State,
    entry: *const Entry,
    leaves: *const Leaves,
    road: *mut crate::road_cmd::CommandSlot,
) -> u8 {
    crate::witness::hit(crate::witness::ROAD_TICKS);
    // SAFETY: Records are borrowed for this call only.
    let e = unsafe { &*entry };
    let mut g = entry!(leaves, None);
    g.road = road;
    u8::from(g.tick(Part::new(v, state), e))
}
/// # Safety
/// As `openttd_rust_road_tick`; `prev` is null or a live pair.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_individual_abi(
    v: VehicleRef,
    state: *mut State,
    prev: Option<VehicleRef>,
    prev_state: *mut State,
    entry: *const Entry,
    leaves: *const Leaves,
    road: *mut crate::road_cmd::CommandSlot,
) -> u8 {
    // SAFETY: Entry record borrowed for this call only.
    let e = unsafe { &*entry };
    let mut g = entry!(leaves, Some(e));
    g.road = road;
    let mut s = Step {
        tile: v.tile(),
        x: v.x(),
        y: v.y(),
        z: v.z(),
        cur_speed: v.speed(),
        direction: v.direction(),
        length: crate::raw_field::read_field!(v.ground_cache(), cached_veh_length),
        front: crate::vehicle_base::is_front_engine(
            crate::vehicle_base::is_ground_vehicle(v.vehicle_type()),
            v.subtype(),
        ),
        tram: (crate::movement::table(g.ss).road_kind)(v).0,
    };
    let v = Part::new(v, state);
    u8::from(g.individual(v, v.ground(e.ground), Part::link(prev, prev_state), &mut s))
}
/// # Safety
/// As `openttd_rust_road_tick`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_leave_depot_abi(
    v: VehicleRef,
    state: *mut State,
    first: bool,
    entry: *const Entry,
    leaves: *const Leaves,
) -> u8 {
    // SAFETY: Entry record borrowed for this call only.
    let e = unsafe { &*entry };
    let v = Part::new(v, state);
    u8::from(entry!(leaves, Some(e)).leave_depot(v, v.ground(e.ground), first))
}
/// # Safety
/// As `openttd_rust_road_tick`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_crash(
    v: VehicleRef,
    state: *mut State,
    flooded: bool,
    leaves: *const Leaves,
) -> u32 {
    entry!(leaves, None).crash(Part::new(v, state), flooded)
}
/// # Safety
/// As `openttd_rust_road_tick`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_update_cache(
    v: VehicleRef,
    state: *mut State,
    same_length: bool,
    leaves: *const Leaves,
) {
    entry!(leaves, None).update_cache(Part::new(v, state), same_length);
}
/// # Safety
/// As `openttd_rust_road_tick`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_calendar_day(
    v: VehicleRef,
    front: bool,
    leaves: *const Leaves,
) {
    crate::witness::hit(crate::witness::ROAD_ENTRIES);
    if front {
        // SAFETY: Static tables and a live serialized shell.
        unsafe { maintenance::age_vehicle(v, &*(*leaves).vehicle) };
    }
}
/// # Safety
/// As `openttd_rust_road_tick`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_economy_day(
    v: VehicleRef,
    state: *mut State,
    front: bool,
    leaves: *const Leaves,
) {
    crate::witness::hit(crate::witness::ROAD_ENTRIES);
    if front {
        // SAFETY: Static tables borrowed for this call only; no owner borrow crosses callbacks.
        unsafe { Game::new(leaves, None) }.economy_day(Part::new(v, state));
    }
}
/// # Safety
/// As `openttd_rust_road_tick`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_running_cost(
    v: VehicleRef,
    state: *mut State,
    leaves: *const Leaves,
) -> i64 {
    entry!(leaves, None).running_cost(Part::new(v, state))
}
/// # Safety
/// As `openttd_rust_road_tick`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_max_speed(
    v: VehicleRef,
    state: *mut State,
    acceleration_model: u8,
    leaves: *const Leaves,
) -> i32 {
    crate::witness::hit(crate::witness::ROAD_ENTRIES);
    // SAFETY: Records borrowed for this call only.
    let r = MaxSpeedRead {
        next: v.next(),
        next_state: v.next().map_or(std::ptr::null_mut(), |n| {
            crate::raw_field::read_field!(n.0.as_ptr(), road).cast()
        }),
        tile: v.tile(),
        max_track_speed: crate::raw_field::read_field!(v.ground_cache(), cached_max_track_speed),
        order_max_speed: v.current_order().speed,
        direction: v.direction(),
        status: v.status(),
    };
    // SAFETY: The static table and live state outlive this synchronous query.
    max_speed(Part::new(v, state), r, acceleration_model, unsafe {
        &*leaves
    })
}
/// # Safety
/// As `openttd_rust_road_tick`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_set_dest(v: VehicleRef, state: *mut State, tile: u32) {
    Game::set_dest(Part::new(v, state), tile);
}
/// # Safety
/// As `openttd_rust_road_tick`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_turn_abi(
    v: VehicleRef,
    state: *mut State,
    execute: bool,
    leaves: *const Leaves,
) -> u8 {
    u8::from(entry!(leaves, None).turn(Part::new(v, state), execute))
}
/// # Safety
/// As `openttd_rust_road_tick`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_trackdir(
    v: VehicleRef,
    state: *mut State,
    leaves: *const Leaves,
) -> u8 {
    entry!(leaves, None).trackdir(Part::new(v, state))
}
/// # Safety
/// As `openttd_rust_road_tick`; `first` is the live first part of `v`'s consist.
pub(crate) unsafe fn slope_pixel(
    v: VehicleRef,
    first: VehicleRef,
    first_state: *mut State,
    direction: u8,
) -> bool {
    Game::slope_pixel(v, Part::new(first, first_state), direction)
}
/// Shared modern/legacy afterload road-stop frame query, retaining original indexes.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_stop_frame(index: u32) -> u8 {
    road_data::_ROAD_STOP_STOP_FRAME[index as usize]
}

/// Construction removal clears only drive-through stop state bits. The raw
/// field access ends before any callback can reenter this canonical owner.
/// # Safety
/// `state` is live and serialized by the game-state mutex.
pub(crate) unsafe fn clear_removed_drive_stop(state: *mut State) {
    let value = read_field!(state, fields.state);
    if value & DT_STOP != 0 {
        write_field!(state, fields.state, value & 9);
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRoadTramBuildRead" ["probe"] => TramBuildRead { road_type, road, };
    "OpenTTDRoadPathElement" ["probe"] => PathElement {
        trackdir,
        tile,
    };
    "OpenTTDRoadEntry" ["probe"] => Entry {
        ground,
        acceleration_model,
        road_side,
        queue,
        map_log_x,
    };
    "OpenTTDRoadTrackChoice" ["probe"] => TrackChoice {
        trackdir,
        found,
    };
    "OpenTTDRoadDepotResult" ["probe"] => DepotResult {
        tile,
        length,
    };
    "OpenTTDRoadLeaves" ["probe"] => Leaves {
        vehicle,
        ground,
        shared,
        any_road_bits,
        is_road_depot_tile,
        depot_direction,
        disallowed_directions,
        is_bay_stop,
        bay_direction,
        is_station_road_stop,
        choose_track,
        find_depot,
        tram_build_read,
        disconnect,
        should_stop,
        arrival_news,

        crash_news,
        pay_running,
        depot_exit_dirty,
    };
}

/// Shared `RoadVehicle::IsInDepot` over the canonical private owner.
pub(crate) fn is_in_depot(state: *mut State) -> bool {
    // SAFETY: caller holds this live shell under the game-state mutex.
    unsafe { get_state(state) == DEPOT }
}

/// `RoadVehicle::GetFrame` on the canonical owner.
/// # Safety
/// The serialized live shell owns this state for this scalar access.
pub(crate) unsafe fn get_frame(s: *const State) -> u8 {
    read_field!(s, fields.frame)
}
/// `RoadVehicle::SetFrame` on the canonical owner.
/// # Safety
/// The serialized live shell owns this state for this scalar access.
pub(crate) unsafe fn set_frame(s: *mut State, frame: u8) {
    write_field!(s, fields.frame, frame);
}

impl Game<'_> {
    fn movement_enter(&self, v: VehicleRef, tile: u32, x: i32, y: i32) -> Entered {
        // SAFETY: Same live serialized shell as the original callback; no owner borrow spans it.
        let flags = unsafe {
            crate::movement::openttd_rust_movement_enter(self.movement.get(), v, tile, x, y)
        };
        Entered {
            tile: v.tile(),
            cur_speed: v.speed(),
            flags,
            direction: v.direction(),
            order_type: v.current_order().kind(),
        }
    }
}

/// `RoadVehicle::GetBlockedCounter`; live serialized owner, no borrow spans reentry.
pub(crate) unsafe fn blocked_counter(state: *const State) -> u16 {
    read_field!(state, fields.blocked_ctr)
}
/// `RoadVehicle::GetOvertaking`; live serialized owner, no borrow spans reentry.
pub(crate) unsafe fn overtaking(state: *const State) -> u8 {
    read_field!(state, fields.overtaking)
}
/// `RoadVehicle::GetOvertakingCounter`; live serialized owner, no borrow spans reentry.
pub(crate) unsafe fn overtaking_counter(state: *const State) -> u8 {
    read_field!(state, fields.overtaking_ctr)
}
/// `RoadVehicle::GetCrashedCounter`; live serialized owner, no borrow spans reentry.
pub(crate) unsafe fn crashed_counter(state: *const State) -> u16 {
    read_field!(state, fields.crashed_ctr)
}

/// `RoadVehicle::GetCurrentMaxSpeed`.
/// # Safety
/// Live serialized shell/private owner; immutable services, no borrow spans callbacks.
pub(crate) unsafe fn current_max_speed(
    v: VehicleRef,
    state: *mut State,
    model: u8,
    leaves: &Leaves,
) -> i32 {
    max_speed(
        Part::new(v, state),
        MaxSpeedRead {
            next: v.next(),
            next_state: v.next().map_or(std::ptr::null_mut(), |n| {
                crate::raw_field::read_field!(n.0.as_ptr(), road).cast()
            }),
            tile: v.tile(),
            max_track_speed: crate::raw_field::read_field!(
                v.ground_cache(),
                cached_max_track_speed
            ),
            order_max_speed: v.current_order().speed,
            direction: v.direction(),
            status: v.status(),
        },
        model,
        leaves,
    )
}

pub(crate) mod availability;
pub(crate) use availability::*;

/// `RoadTypeSpriteGroup::ROTSG_END`.
pub const ROTSG_END: u8 = 12;

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
            update_viewport: unreachable1,
            has_articulated: unreachable1,
        };
        let mut leaves = ROAD_LEAVES;
        leaves.shared = &raw const shared;
        leaves.vehicle = &raw const vehicle;
        leaves.ground = &raw const ground;
        let record = crate::vehicle_base::openttd_rust_vehicle_record_new(
            std::ptr::null_mut(),
            crate::vehicle_type::VehicleType::VEH_ROAD,
        );
        let mut state = State::default();
        // SAFETY: The record, owner, entry and tables remain live throughout this
        // call. The original articulated branch reads no orders, ground or map.
        unsafe {
            (*record).road = std::ptr::from_mut(&mut state.fields);
            (*record).tick_counter = u8::MAX;
            (*record).running_ticks = 7;
            let part = Part::new(VehicleRef::of(record), std::ptr::from_mut(&mut state));
            let entry = Entry {
                ground: std::ptr::null_mut(),
                acceleration_model: 0,
                road_side: 0,
                queue: false,
                map_log_x: 8,
            };
            let mut game = Game::new(&raw const leaves, None);
            assert!(game.tick(part, &entry));
            assert_eq!(((*record).tick_counter, (*record).running_ticks), (0, 7));
            crate::vehicle_base::openttd_rust_vehicle_record_delete(record);
        }
    }
}
