/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Road vehicles own their movement, consist traversal and private counters/cache.
//! Each entry receives a live vehicle handle and its Rust owner; consist links
//! carry both. Owner scalars are read and written in place through the raw owner
//! pointer, so no reference into an owner survives a service call. Observations
//! are copies taken at the original read points and refreshed after every service
//! that can change them.
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
use crate::services::{Map, Services};
use crate::vehicle::{
    AM_REALISTIC, CALLBACK_FAILED, GroundServices, HVOT_BUS, HVOT_TRUCK, INVALID_ENGINE,
    INVALID_PRICE, INVALID_TRACKDIR, MP_ROAD, MP_STATION, MP_TUNNELBRIDGE, OT_GOTO_DEPOT,
    OT_GOTO_STATION, OT_LEAVESTATION, OT_LOADING, SND_DEPARTURE_OLD_RV_1, SND_DEPARTURE_OLD_RV_2,
    VEHICLE_LENGTH, VETS_CANNOT_ENTER, VETS_ENTERED_WORMHOLE, VS_CRASHED, VS_HIDDEN, VS_STOPPED,
    VehicleRef, VehicleServices, YEAR_TICKS, advance_distance,
};
use std::ffi::c_void;
use std::ptr::NonNull;

const DEPOT: u8 = 254;
const WORMHOLE: u8 = 255;
const STOP: u8 = 32;
const DT_STOP: u8 = 64;
const REVERSE: [u8; 4] = [6, 7, 14, 15];
const DIAG_TRACK: [u8; 4] = [0, 1, 8, 9];
/// Owner scalar selectors, shared with `openttd_rust_road_get/set`.
const STATE: usize = 0;
const FRAME: usize = 1;
const BLOCKED: usize = 2;
const OVERTAKING: usize = 3;
const OVERTAKING_CTR: usize = 4;
const CRASHED: usize = 5;
const REVERSE_CTR: usize = 6;

/// Exact scalar path entry; padding is never read.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PathElement {
    pub trackdir: u8,
    pub tile: u32,
}
/// One allocation per ordinary/indexed-load C++ shell.
#[derive(Default)]
pub struct State {
    scalars: [u16; 7],
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_get(state: *const State, field: u8) -> u16 {
    // SAFETY: Game-thread caller supplies a live owner and selector 0..6.
    unsafe { (*state).scalars[field as usize] }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_set(state: *mut State, field: u8, value: u16) {
    // SAFETY: Exclusive game-thread access, with no retained owner reference.
    unsafe {
        (*state).scalars[field as usize] = narrow(field as usize, value);
    }
}
/// Fields 2 and 5 are `uint16_t`; the others are `uint8_t`.
fn narrow(field: usize, value: u16) -> u16 {
    if field == BLOCKED || field == CRASHED {
        value
    } else {
        u16::from(value as u8)
    }
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

/// Movement settings and map size, read once only by movement entries.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Entry {
    pub acceleration_model: u8,
    pub road_side: u8,
    pub queue: bool,
    pub map_log_x: u8,
}

/// The tick/front gate; non-front entries consume only these fields.
#[repr(C)]
pub struct TickRead {
    pub tick: u8,
    pub front: bool,
}
/// Counters/status and movement settings consumed only after the Rust front gate.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FrontTickRead {
    pub movement: Entry,
    pub order_time: i32,
    pub running: u8,
    pub status: u8,
}

/// `GetCurrentMaxSpeed` inputs of the front part.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MaxSpeedRead {
    pub next: Option<VehicleRef>,
    pub next_state: *mut State,
    pub tile: u32,
    pub max_track_speed: u16,
    pub order_max_speed: u16,
    pub direction: u8,
    pub status: u8,
}

/// `ShowVisualEffect()`, then the speed and first-step inputs it leaves.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SpeedRead {
    pub max: MaxSpeedRead,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub acceleration: i32,
    pub length: u8,
    pub tram: bool,
}

/// One further consist part for speed, viewport and slope walks.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SpeedPart {
    pub next: Option<VehicleRef>,
    pub next_state: *mut State,
    pub tile: u32,
    pub direction: u8,
    pub status: u8,
}

/// Consist links, read after callbacks which can change the chain.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Link {
    pub next: Option<VehicleRef>,
    pub next_state: *mut State,
}
/// Consist links and tile for depot traversal.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinkTile {
    pub link: Link,
    pub tile: u32,
}

/// Movement inputs of one part at the start of `IndividualRoadVehicleController`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Step {
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
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CrashPart {
    pub next: Option<VehicleRef>,
    pub next_state: *mut State,
    pub tile: u32,
    pub z: i32,
    pub crossing: bool,
}

/// `VehicleEnterTile` result, then the fields it may have changed.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Entered {
    pub tile: u32,
    pub cur_speed: u16,
    pub flags: u8,
    pub direction: u8,
    pub order_type: u8,
}

/// One road vehicle near the probe, other than the probing consist.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Candidate {
    pub vehicle: VehicleRef,
    pub state: *mut State,
    pub first: VehicleRef,
    pub first_state: *mut State,
    pub index: u32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub first_speed: u16,
    pub direction: u8,
}

/// Road vehicle on a tile checked for overtaking.
#[repr(C)]
pub struct TileVehicle {
    pub vehicle: VehicleRef,
    pub first: VehicleRef,
}

/// Generic part read: link, tile, height and road/tram type.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PartRead {
    pub next: Option<VehicleRef>,
    pub next_state: *mut State,
    pub tile: u32,
    pub z: i32,
    pub tram: bool,
}

/// `RoadFindPathToDest` vehicle inputs.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PathRead {
    pub dest: u32,
    pub owner: u8,
    pub articulated: bool,
    pub bus: bool,
}

/// Road stop arrival and departure inputs.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct StopRead {
    pub owner: u8,
    pub bus: bool,
}

/// `RoadVehCheckOvertake` inputs of either vehicle.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OvertakeRead {
    pub tile: u32,
    pub cur_speed: u16,
    pub direction: u8,
    pub status: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RoadTypes {
    pub before: u8,
    pub after: u8,
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

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArrivalRead {
    pub had_vehicle_of_type: u8,
    pub bus: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SoundRead {
    pub sound: u16,
    pub tick: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CostRead {
    pub engine: crate::vehicle::EngineRef,
    pub cost_class: u8,
    pub cost_factor: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ServiceRead {
    pub tile: u32,
    pub servint: u16,
    pub cur_speed: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TurnRead {
    pub tile: u32,
    pub breakdown_ctr: u8,
    pub direction: u8,
    pub order_type: u8,
    pub status: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrackdirRead {
    pub tile: u32,
    pub direction: u8,
    pub status: u8,
}

pub type CloseVisitor = extern "C" fn(*mut c_void, *const Candidate) -> bool;
pub type TrainVisitor = extern "C" fn(*mut c_void, *const i32) -> bool;
pub type TileVisitor = extern "C" fn(*mut c_void, *const TileVehicle) -> bool;

/// Road-specific services; the shared tables are borrowed through pointers.
#[repr(C)]
pub struct Leaves {
    pub vehicle: *const VehicleServices,
    pub ground: *const GroundServices,
    pub shared: *const Services,
    pub tick_front: extern "C" fn(VehicleRef) -> FrontTickRead,
    pub max_depot_penalty: extern "C" fn() -> u32,
    pub process_orders_then_loading: extern "C" fn(VehicleRef) -> u8,
    pub visual_then_speed: extern "C" fn(VehicleRef, bool) -> SpeedRead,
    pub max_speed_read: extern "C" fn(VehicleRef) -> MaxSpeedRead,
    pub speed_part: extern "C" fn(VehicleRef) -> SpeedPart,
    pub step: extern "C" fn(VehicleRef) -> Step,
    pub crash_part: extern "C" fn(VehicleRef) -> CrashPart,
    pub part: extern "C" fn(VehicleRef) -> PartRead,
    pub link: extern "C" fn(VehicleRef) -> Link,
    pub link_tile: extern "C" fn(VehicleRef) -> LinkTile,
    pub enter_tile: extern "C" fn(VehicleRef, u32, i32, i32) -> Entered,
    pub visit_close: extern "C" fn(VehicleRef, i32, i32, bool, CloseVisitor, *mut c_void),
    pub visit_trains: extern "C" fn(VehicleRef, TrainVisitor, *mut c_void),
    pub visit_tile: extern "C" fn(u32, TileVisitor, *mut c_void),
    pub has_road: extern "C" fn(VehicleRef, u32) -> bool,
    pub track_status: extern "C" fn(VehicleRef, u32) -> u32,
    pub any_road_bits: extern "C" fn(VehicleRef, u32, bool) -> u8,
    pub tram_bits: extern "C" fn(u32) -> u8,
    pub is_road_depot_tile: extern "C" fn(u32) -> bool,
    pub depot_direction: extern "C" fn(u32) -> u8,
    pub is_normal_road: extern "C" fn(u32) -> bool,
    pub has_road_works: extern "C" fn(u32) -> bool,
    pub disallowed_directions: extern "C" fn(u32) -> u8,
    pub is_bay_stop: extern "C" fn(u32) -> bool,
    pub bay_direction: extern "C" fn(u32) -> u8,
    pub is_drive_through: extern "C" fn(u32) -> bool,
    pub is_station_road_stop: extern "C" fn(u32) -> bool,
    pub stop_type: extern "C" fn(u32) -> u8,
    pub has_free_bay: extern "C" fn(u32) -> bool,
    pub continuation: extern "C" fn(u32, u32) -> bool,
    pub move_tile: extern "C" fn(VehicleRef, u32) -> RoadTypes,
    pub path_read: extern "C" fn(VehicleRef) -> PathRead,
    pub choose_track: extern "C" fn(VehicleRef, u32, u8, u16) -> TrackChoice,
    pub find_depot: extern "C" fn(VehicleRef, i32) -> DepotResult,
    pub can_build_tram: extern "C" fn(VehicleRef, u32, u8) -> bool,
    pub disconnect: extern "C" fn(),
    pub stop_read: extern "C" fn(VehicleRef) -> StopRead,
    pub should_stop: extern "C" fn(VehicleRef, u32) -> bool,
    pub stop_leave: extern "C" fn(VehicleRef),
    pub stop_entrance: extern "C" fn(VehicleRef, bool),
    pub stop_entrance_busy: extern "C" fn(VehicleRef) -> bool,
    pub arrival_read: extern "C" fn(VehicleRef, u16) -> ArrivalRead,
    pub arrival_news: extern "C" fn(VehicleRef, u16, u8, u8),
    pub begin_loading_at: extern "C" fn(VehicleRef, u16),
    pub overtake_read: extern "C" fn(VehicleRef) -> OvertakeRead,
    pub sound_read: extern "C" fn(VehicleRef) -> SoundRead,
    pub depot_exit: extern "C" fn(VehicleRef, i32, i32),
    pub crash_news: extern "C" fn(VehicleRef, u32),
    pub shorten_factor: extern "C" fn(crate::vehicle::EngineRef) -> u8,
    pub default_max_speed: extern "C" fn(VehicleRef) -> u16,
    pub update_cargo_age: extern "C" fn(VehicleRef),
    pub cost_read: extern "C" fn(VehicleRef) -> CostRead,
    pub price: extern "C" fn(crate::vehicle::EngineRef, u32) -> i64,
    pub pay_running: extern "C" fn(VehicleRef, i64),
    pub service_read: extern "C" fn(VehicleRef) -> ServiceRead,
    pub turn_read: extern "C" fn(VehicleRef) -> TurnRead,
    pub trackdir_read: extern "C" fn(VehicleRef) -> TrackdirRead,
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
    fn get(self, field: usize) -> u16 {
        // SAFETY: Live owner; the place read ends before any service call.
        unsafe { (*self.s.as_ptr()).scalars[field] }
    }
    fn set(self, field: usize, value: u16) {
        // SAFETY: Live owner, game thread; no reference outlives this store.
        unsafe { (*self.s.as_ptr()).scalars[field] = narrow(field, value) }
    }
    fn state(self) -> u8 {
        self.get(STATE) as u8
    }
    fn frame(self) -> u8 {
        self.get(FRAME) as u8
    }
    fn overtaking(self) -> u8 {
        self.get(OVERTAKING) as u8
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
    unsafe { (*s).scalars[STATE] as u8 }
}

/// Synchronously visit native candidates with a Rust closure.
fn visit<T, F: FnMut(&T) -> bool>(
    call: impl FnOnce(extern "C" fn(*mut c_void, *const T) -> bool, *mut c_void),
    mut visitor: F,
) {
    extern "C" fn dispatch<T, F: FnMut(&T) -> bool>(context: *mut c_void, item: *const T) -> bool {
        // SAFETY: Native traversal calls synchronously and once at a time; item is
        // borrowed for this dispatch only.
        {
            crate::witness::hit(crate::witness::ROAD_VISITORS);
            unsafe { (*context.cast::<F>())(&*item) }
        }
    }
    call(dispatch::<T, F>, (&raw mut visitor).cast());
}

/// Count one actual typed outgoing call; returns the unchanged function pointer.
macro_rules! service {
    ($function:expr) => {{
        crate::witness::hit(crate::witness::ROAD_SERVICES);
        $function
    }};
}

struct Game<'a> {
    l: &'a Leaves,
    vs: &'a VehicleServices,
    gs: &'a GroundServices,
    ss: &'a Services,
    acceleration_model: u8,
    road_side: u8,
    queue: bool,
    map: Map,
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
fn diag(dir: u8) -> u8 {
    (dir >> 1) & 3
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
                l,
                vs: &*l.vehicle,
                gs: &*l.ground,
                ss: &*l.shared,
                acceleration_model,
                road_side,
                queue,
                map: Map { log_x },
            }
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
    fn tile_xy(&self, tile: u32, pos: (u8, u8)) -> (i32, i32) {
        (
            (self.map.tile_x(tile) * 16 + u32::from(pos.0)) as i32,
            (self.map.tile_y(tile) * 16 + u32::from(pos.1)) as i32,
        )
    }
    fn set_speed(&self, v: Part, s: &mut Step, speed: u16) {
        service!(self.vs.write_speed)(v.v, speed);
        s.cur_speed = speed;
    }
    fn observe(s: &mut Step, e: Entered) {
        s.tile = e.tile;
        s.cur_speed = e.cur_speed;
        s.direction = e.direction;
    }

    /// `RoadVehicle::GetCurrentMaxSpeed`.
    fn max_speed(&self, front: Part, r: MaxSpeedRead) -> i32 {
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
            if self.realistic() {
                let state = front.state();
                if state <= 15 && reversing(state) {
                    speed = i32::from(r.max_track_speed) / 2;
                    break;
                } else if u.direction & 1 == 0 {
                    speed = i32::from(r.max_track_speed) * 3 / 4;
                }
            }
            if part.state() == WORMHOLE && u.status & VS_HIDDEN == 0 {
                speed = speed.min(i32::from(service!(self.ss.bridge_speed)(u.tile)) * 2);
            }
            let Some(next) = Part::link(u.next, u.next_state) else {
                break;
            };
            part = next;
            u = service!(self.l.speed_part)(next.v);
        }
        speed.min(i32::from(r.order_max_speed) * 2)
    }
    /// `RoadVehicle::UpdateSpeed`, returning the distance and stored speed.
    fn update_speed(&self, front: Part, r: &SpeedRead) -> (i32, u16) {
        let over = front.overtaking() != 0;
        let (accel, min) = if self.realistic() {
            (
                r.acceleration.wrapping_add(if over { 256 } else { 0 }) as u32,
                if r.max.status & VS_STOPPED != 0 { 0 } else { 4 },
            )
        } else {
            (if over { 512 } else { 256 }, 0)
        };
        let speed =
            service!(self.gs.do_update_speed)(front.v, accel, min, self.max_speed(front, r.max));
        (speed.distance, speed.cur_speed)
    }
    /// `RoadVehFindCloseTo`; `v` is always the front part.
    fn close(&self, v: Part, z: i32, x: i32, y: i32, dir: u8, update: bool) -> Option<Candidate> {
        if v.get(REVERSE_CTR) != 0 {
            return None;
        }
        let mut best: Option<Candidate> = None;
        let mut best_diff = u32::MAX;
        let wormhole = v.state() == WORMHOLE;
        visit(
            |dispatch, context| {
                service!(self.l.visit_close)(v.v, x, y, wormhole, dispatch, context);
            },
            |u: &Candidate| {
                // FindClosestBlockingRoadVeh; C++ already skipped other types and our consist.
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
            },
        );
        if best_diff == u32::MAX {
            v.set(BLOCKED, 0);
            return None;
        }
        if update {
            let counter = v.get(BLOCKED).wrapping_add(1);
            v.set(BLOCKED, counter);
            if counter > 1480 {
                return None;
            }
        }
        best
    }
    /// `CheckRoadBlockedForOvertaking`.
    fn overtake_blocked(&self, v: Part, u: VehicleRef, tile: u32, trackdir: u8) -> bool {
        if !service!(self.l.has_road)(v.v, tile) {
            return true;
        }
        let ts = service!(self.l.track_status)(v.v, tile);
        let trackdirs = ts & 0x3f3f;
        let red = (ts >> 16) & 0x3f3f;
        let trackbits = (trackdirs | (trackdirs >> 8)) & 0x3f;
        if trackdirs & (1 << trackdir) == 0 || trackbits & !3 != 0 || red != 0 {
            return true;
        }
        let mut blocked = false;
        visit(
            |dispatch, context| service!(self.l.visit_tile)(tile, dispatch, context),
            |w: &TileVehicle| {
                blocked = w.first == w.vehicle && w.vehicle != u && w.vehicle != v.v;
                !blocked
            },
        );
        blocked
    }
    /// `RoadVehCheckOvertake(v, u)`; pure predicates are evaluated cheapest first.
    fn overtake(&self, v: Part, s: &Step, u: Part) {
        if s.tram {
            return;
        }
        let ur = service!(self.l.overtake_read)(u.v);
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
        let u_speed = if self.original() || service!(self.gs.acceleration)(u.v) > 0 {
            self.max_speed(u, service!(self.l.max_speed_read)(u.v))
        } else {
            i32::from(ur.cur_speed)
        };
        if u_speed >= self.max_speed(v, service!(self.l.max_speed_read)(v.v))
            && ur.status & VS_STOPPED == 0
            && ur.cur_speed != 0
        {
            return;
        }
        let trackdir = DIAG_TRACK[usize::from(diag(s.direction))];
        if self.overtake_blocked(v, u.v, s.tile, trackdir) {
            return;
        }
        let tile = self.map.add_diag(s.tile, diag(s.direction));
        if self.overtake_blocked(v, u.v, tile, trackdir) {
            return;
        }
        v.set(
            OVERTAKING_CTR,
            if ur.cur_speed == 0 || ur.status & VS_STOPPED != 0 {
                17
            } else {
                0
            },
        );
        v.set(OVERTAKING, 16);
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
        service!(self.vs.write_direction)(v.v, dir);
        s.direction = dir;
        if self.original() {
            self.set_speed(v, s, s.cur_speed - (s.cur_speed >> 2));
        }
    }
    /// x/y store, `UpdatePosition`, `UpdateInclination` and `RoadZPosAffectSpeed`.
    fn move_to(&self, v: Part, s: &mut Step, x: i32, y: i32, new_tile: bool, delta: bool) {
        let m = service!(self.gs.move_incline)(v.v, x, y, new_tile, delta);
        s.x = x;
        s.y = y;
        s.z = m.z;
        s.cur_speed = m.cur_speed;
        if m.old_z == m.z || !self.original() {
            return;
        }
        if m.old_z < m.z {
            self.set_speed(v, s, (u32::from(m.cur_speed) * 232 / 256) as u16);
        } else {
            let speed = m.cur_speed.wrapping_add(2);
            if speed <= m.max_track_speed {
                self.set_speed(v, s, speed);
            }
        }
    }
    /// `StartRoadVehSound`.
    fn start_sound(&self, v: Part) {
        if !service!(self.vs.play_start_sound)(v.v) {
            let r = service!(self.l.sound_read)(v.v);
            let mut sound = r.sound;
            if sound == SND_DEPARTURE_OLD_RV_1 && r.tick & 3 == 0 {
                sound = SND_DEPARTURE_OLD_RV_2;
            }
            service!(self.vs.play_sound)(v.v, sound);
        }
    }
    /// `RoadVehicle::Crash`.
    fn crash(&self, v: Part, flooded: bool) -> u32 {
        let c = service!(self.gs.crash)(v.v, flooded);
        let mut victims = c.victims;
        if c.front {
            victims = victims.wrapping_add(1);
            if in_range(v.state(), DT_STOP, DT_STOP + 16) {
                service!(self.l.stop_leave)(v.v);
            }
        }
        v.set(CRASHED, if flooded { 2000 } else { 1 });
        victims
    }
    /// `RoadVehCheckTrainCrash`, starting with the front part's observation.
    fn train_crash(&self, v: Part, first: CrashPart) -> bool {
        let mut part = v;
        let mut u = first;
        loop {
            if part.state() != WORMHOLE && u.crossing {
                let z = u.z;
                let mut hit = false;
                visit(
                    |dispatch, context| service!(self.l.visit_trains)(v.v, dispatch, context),
                    |tz: &i32| {
                        hit = tz.wrapping_sub(z).wrapping_abs() <= 6;
                        !hit
                    },
                );
                if hit {
                    let victims = self.crash(v, false);
                    service!(self.l.crash_news)(v.v, victims);
                    return true;
                }
            }
            let Some(next) = Part::link(u.next, u.next_state) else {
                return false;
            };
            part = next;
            u = service!(self.l.crash_part)(next.v);
        }
    }
    /// `RoadVehIsCrashed` with this tick's counter.
    fn crashed(&self, v: Part, tick: u8) -> bool {
        let counter = v.get(CRASHED).wrapping_add(1);
        v.set(CRASHED, counter);
        if counter == 2 {
            service!(self.vs.large_explosion)(v.v);
        } else if counter <= 45 {
            if tick & 7 == 0 {
                const DELTA: [u8; 4] = [7, 0, 0, 1];
                let mut u = Some(v.v);
                while let Some(part) = u {
                    let random = self.random();
                    let direction = service!(self.vs.direction)(part);
                    service!(self.gs.turn)(part, (direction + DELTA[(random & 3) as usize]) & 7);
                    u = service!(self.vs.next)(part);
                }
            }
        } else if counter >= 2220 && tick & 31 == 0 {
            let first = service!(self.l.link)(v.v);
            let alive = first.next.is_some();
            let mut previous = v;
            let mut last = v;
            let mut next = Part::link(first.next, first.next_state);
            while let Some(part) = next {
                previous = last;
                last = part;
                let r = service!(self.l.link)(part.v);
                next = Part::link(r.next, r.next_state);
            }
            service!(self.vs.detach_last)(previous.v, last.v, v.v);
            if in_range(last.state(), STOP, STOP + 16) {
                service!(self.l.stop_leave)(last.v);
            }
            service!(self.vs.destroy)(last.v);
            return alive;
        }
        true
    }
    /// `RoadVehUpdateCache`.
    fn update_cache(&self, v: Part, same_length: bool) {
        service!(self.gs.cache_begin)(v.v);
        let mut total: u16 = 0;
        let mut u = v.v;
        loop {
            let first_engine = if u == v.v {
                INVALID_ENGINE
            } else {
                service!(self.vs.engine_id)(v.v)
            };
            service!(self.gs.write_first_engine)(u, first_engine);
            let engine = service!(self.vs.engine_for_length)(u);
            let mut length = VEHICLE_LENGTH;
            let mut factor = if engine.grf_version >= 8 {
                let value = service!(self.vs.property)(u, 0x23, i32::from(CALLBACK_FAILED)) as u16;
                if value != CALLBACK_FAILED && value >= u16::from(VEHICLE_LENGTH) {
                    service!(self.vs.unknown_length_result)(engine.engine, value);
                }
                value
            } else {
                service!(self.vs.length_callback)(u)
            };
            if factor == CALLBACK_FAILED {
                factor = u16::from(service!(self.l.shorten_factor)(engine.engine));
            }
            if factor != 0 {
                length -= factor.min(u16::from(VEHICLE_LENGTH - 1)) as u8;
            }
            if same_length && length != service!(self.gs.cached_length)(u) {
                service!(self.vs.length_changed)(u);
            }
            total = total.wrapping_add(u16::from(length));
            service!(self.gs.write_length)(u, length, v.v, total);
            service!(self.l.update_cargo_age)(u);
            let Some(next) = service!(self.vs.next)(u) else {
                break;
            };
            u = next;
        }
        let speed = service!(self.vs.property)(v.v, 0x15, 0) as u32;
        service!(self.vs.write_max_speed)(
            v.v,
            if speed != 0 {
                speed.wrapping_mul(4)
            } else {
                u32::from(service!(self.l.default_max_speed)(v.v))
            } as u16,
        );
    }
    /// `RoadFindPathToDest`.
    fn path(&self, v: Part, s: &Step, tile: u32, entry: u8) -> u8 {
        let ts = service!(self.l.track_status)(v.v, tile);
        let red = (ts >> 16) & 0x3f3f;
        let mut trackdirs = ts & 0x3f3f;
        let p = service!(self.l.path_read)(v.v);
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
            } else if service!(self.l.stop_type)(tile) != u8::from(!p.bus)
                || (!self.queue && !service!(self.l.has_free_bay)(tile))
            {
                trackdirs = 0;
            }
        }
        trackdirs &= [0x1009, 0x0016, 0x0520, 0x2a00][usize::from(entry)];
        let best = if trackdirs == 0 {
            if v.path_len() != 0 {
                v.path_clear();
            }
            REVERSE[usize::from(entry)]
        } else {
            let mut forced = None;
            if v.get(REVERSE_CTR) != 0 {
                let mut reverse = true;
                if s.tram {
                    let bits = service!(self.l.any_road_bits)(v.v, tile, false);
                    let straight = if entry & 1 == 0 { 10 } else { 5 };
                    reverse = (bits & straight) == straight || bits == (8 >> entry);
                }
                if reverse {
                    v.set(REVERSE_CTR, 0);
                    if s.tile != tile {
                        forced = Some(REVERSE[usize::from(entry)]);
                    }
                }
            }
            if let Some(dir) = forced {
                dir
            } else if p.dest == 0 {
                let count = trackdirs.count_ones();
                let mut n = ((u64::from(self.random()) * u64::from(count)) >> 32) as u32;
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
    fn leave_depot(&self, v: Part, first: bool) -> bool {
        if v.state() != DEPOT {
            return false;
        }
        let d = service!(self.l.part)(v.v);
        let mut next = Part::link(d.next, d.next_state);
        while let Some(u) = next {
            if u.state() != DEPOT {
                return false;
            }
            let r = service!(self.l.link_tile)(u.v);
            if r.tile != d.tile {
                return false;
            }
            next = Part::link(r.link.next, r.link.next_state);
        }
        let dir = service!(self.l.depot_direction)(d.tile);
        let direction = dir * 2 + 1;
        service!(self.vs.write_direction)(v.v, direction);
        let tdir = DIAG_TRACK[usize::from(dir)];
        let rd = crate::road_data::entry(d.tram, self.side() + usize::from(tdir), 6);
        let (x, y) = self.tile_xy(d.tile, (rd.0 & 15, rd.1 & 15));
        if first {
            if service!(self.vs.order_type)(v.v) == OT_GOTO_DEPOT
                && d.tile == service!(self.vs.dest_tile)(v.v)
            {
                service!(self.vs.enter_depot)(v.v);
                return true;
            }
            if self.close(v, d.z, x, y, direction, false).is_some() {
                return true;
            }
            service!(self.vs.service_in_depot)(v.v);
            service!(self.vs.leave_unbunching_depot)(v.v);
            self.start_sound(v);
            service!(self.vs.write_speed)(v.v, 0);
        }
        v.set(STATE, u16::from(tdir));
        v.set(FRAME, 6);
        service!(self.l.depot_exit)(v.v, x, y);
        true
    }
    /// `FollowPreviousRoadVehicle`.
    fn follow(&self, v: Part, s: &Step, prev: Part, tile: u32, entry: u8, reversed: bool) -> u8 {
        let prev_tile = service!(self.vs.tile)(prev.v);
        if prev_tile == s.tile && !reversed {
            return REVERSE[usize::from(entry)];
        }
        let state = prev.state();
        let dir;
        if state == WORMHOLE || state == DEPOT {
            let diagonal = if service!(self.ss.tile_type)(tile) == MP_TUNNELBRIDGE {
                service!(self.ss.tunnel_bridge_direction)(tile)
            } else if service!(self.l.is_road_depot_tile)(tile) {
                service!(self.l.depot_direction)(tile) ^ 2
            } else {
                return INVALID_TRACKDIR;
            };
            dir = DIAG_TRACK[usize::from(diagonal)];
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
        let r = service!(self.l.arrival_read)(v.v, station);
        let bit = if r.bus { HVOT_BUS } else { HVOT_TRUCK };
        if r.had_vehicle_of_type & bit == 0 {
            let headline = if r.bus { 0 } else { 2 } + u8::from(s.tram);
            service!(self.l.arrival_news)(v.v, station, bit, headline);
        }
        service!(self.l.begin_loading_at)(v.v, station);
    }
    /// Drive-through stop test shared by the blocked and stop-frame paths.
    fn stops_here(&self, v: Part, tile: u32) -> bool {
        let r = service!(self.l.stop_read)(v.v);
        service!(self.l.should_stop)(v.v, tile)
            && r.owner == service!(self.ss.tile_owner)(tile)
            && service!(self.l.stop_type)(tile) == u8::from(!r.bus)
    }
    /// `IndividualRoadVehicleController`; `s` is a fresh observation of `v`.
    fn individual(&self, v: Part, prev: Option<Part>, s: &mut Step) -> bool {
        if v.overtaking() != 0 {
            if service!(self.ss.tile_type)(s.tile) == MP_STATION {
                v.set(OVERTAKING, 0);
            } else {
                let counter = v.get(OVERTAKING_CTR).wrapping_add(1) as u8;
                v.set(OVERTAKING_CTR, u16::from(counter));
                if counter >= 35 && v.state() < STOP && straight(v.state()) {
                    v.set(OVERTAKING, 0);
                }
            }
        }
        if v.state() == DEPOT {
            return true;
        }
        if v.state() == WORMHOLE {
            let gp = service!(self.vs.new_position)(v.v);
            if s.front {
                if let Some(u) = self.close(v, s.z, gp.x, gp.y, s.direction, true) {
                    self.set_speed(v, s, u.first_speed);
                    return false;
                }
            }
            if service!(self.ss.tile_type)(gp.tile) == MP_TUNNELBRIDGE {
                let e = service!(self.l.enter_tile)(v.v, gp.tile, gp.x, gp.y);
                Self::observe(s, e);
                if e.flags & VETS_ENTERED_WORMHOLE != 0 {
                    self.move_without_speed(v, s, gp.x, gp.y);
                    return true;
                }
            }
            let status = service!(self.vs.move_then_status)(v.v, gp.x, gp.y);
            s.x = gp.x;
            s.y = gp.y;
            if status & VS_HIDDEN == 0 {
                service!(self.vs.base_viewport)(v.v);
            }
            return true;
        }
        let state = v.state();
        let movement = if state & 64 != 0 { state & 9 } else { state };
        let rd = crate::road_data::entry(
            s.tram,
            (usize::from(movement) + self.side()) ^ usize::from(v.overtaking()),
            usize::from(v.frame()) + 1,
        );
        let entry = rd.0 & 3;
        if rd.0 & 128 != 0 {
            let mut tile = self.map.add_diag(s.tile, entry);
            let mut dir = if s.front {
                if service!(self.l.has_road)(v.v, tile) {
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
                self.set_speed(v, s, 0);
                return false;
            }
            loop {
                let mut start_frame = 0;
                if reversing(dir) {
                    v.set(OVERTAKING, 0);
                    if s.tram {
                        let needed: u8 = [2, 1, 8, 4][usize::from(((dir >> 2) & 2) | (dir & 1))];
                        let previous = service!(self.vs.previous_tile)(v.v);
                        let big = (previous.exists && previous.tile == tile)
                            || (s.front
                                && service!(self.l.is_normal_road)(tile)
                                && !service!(self.l.has_road_works)(tile)
                                && service!(self.l.has_road)(v.v, tile)
                                && needed & service!(self.l.tram_bits)(tile) != 0);
                        if !big {
                            if !s.front
                                || !service!(self.l.can_build_tram)(v.v, tile, needed)
                                || (!needed & service!(self.l.any_road_bits)(v.v, s.tile, false))
                                    == 0
                            {
                                tile = s.tile;
                                start_frame = 16;
                            } else {
                                self.set_speed(v, s, 0);
                                return false;
                            }
                        }
                    } else if service!(self.l.is_normal_road)(s.tile)
                        && service!(self.l.disallowed_directions)(s.tile) != 0
                    {
                        self.set_speed(v, s, 0);
                        return false;
                    } else {
                        tile = s.tile;
                    }
                }
                let pos = crate::road_data::entry(
                    s.tram,
                    (usize::from(dir) + self.side()) ^ usize::from(v.overtaking()),
                    start_frame,
                );
                let (x, y) = self.tile_xy(tile, pos);
                let new_dir = Self::sliding(s, x, y);
                if s.front {
                    if let Some(u) = self.close(v, s.z, x, y, new_dir, true) {
                        self.set_speed(v, s, u.first_speed);
                        v.path_push(dir, tile);
                        return false;
                    }
                }
                let e = service!(self.l.enter_tile)(v.v, tile, x, y);
                Self::observe(s, e);
                if e.flags & VETS_CANNOT_ENTER != 0 {
                    if service!(self.ss.tile_type)(tile) != MP_TUNNELBRIDGE {
                        self.set_speed(v, s, 0);
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
                        self.set_speed(v, s, 0);
                        return false;
                    }
                    if service!(self.l.is_drive_through)(s.tile)
                        && service!(self.l.continuation)(s.tile, tile)
                        && s.tile != tile
                    {
                        dir = v.state();
                    } else if service!(self.l.is_station_road_stop)(s.tile) {
                        service!(self.l.stop_leave)(v.v);
                    }
                }
                if e.flags & VETS_ENTERED_WORMHOLE == 0 {
                    let types = service!(self.l.move_tile)(v.v, tile);
                    s.tile = tile;
                    v.set(STATE, u16::from(dir));
                    v.set(FRAME, start_frame as u16);
                    if types.before != types.after {
                        if s.front {
                            self.update_cache(v, false);
                        }
                        service!(self.gs.first_cargo_changed)(v.v);
                    }
                }
                if new_dir != s.direction {
                    self.set_direction(v, s, new_dir);
                }
                self.move_to(v, s, x, y, true, true);
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
                self.set_speed(v, s, 0);
                return false;
            }
            let pos = crate::road_data::entry(s.tram, self.side() + usize::from(dir), start_frame);
            let (x, y) = self.tile_xy(s.tile, pos);
            let new_dir = Self::sliding(s, x, y);
            if s.front {
                if let Some(u) = self.close(v, s.z, x, y, new_dir, true) {
                    self.set_speed(v, s, u.first_speed);
                    v.path_push(dir, s.tile);
                    return false;
                }
            }
            let e = service!(self.l.enter_tile)(v.v, s.tile, x, y);
            Self::observe(s, e);
            if e.flags & VETS_CANNOT_ENTER != 0 {
                self.set_speed(v, s, 0);
                return false;
            }
            v.set(STATE, u16::from(dir));
            v.set(FRAME, start_frame as u16);
            if new_dir != s.direction {
                self.set_direction(v, s, new_dir);
            }
            self.move_to(v, s, x, y, true, true);
            return true;
        }
        let link = service!(self.l.link)(v.v);
        if let Some(next) = Part::link(link.next, link.next_state) {
            if service!(self.l.is_road_depot_tile)(s.tile)
                && u16::from(v.frame()) == u16::from(s.length) + 6
            {
                self.leave_depot(next, false);
            }
        }
        let x = (s.x & !15) + i32::from(rd.0 & 15);
        let y = (s.y & !15) + i32::from(rd.1 & 15);
        let new_dir = Self::sliding(s, x, y);
        if s.front && !in_range(v.state(), STOP, STOP + 16) {
            if let Some(c) = self.close(v, s.z, x, y, new_dir, true) {
                let u = Part::new(c.first, c.first_state);
                if v.overtaking() == 0 {
                    self.overtake(v, s, u);
                }
                if v.overtaking() == 0 {
                    self.set_speed(v, s, c.first_speed);
                }
                if s.cur_speed == 0
                    && in_range(v.state(), DT_STOP, DT_STOP + 16)
                    && service!(self.l.should_stop)(v.v, s.tile)
                {
                    let r = service!(self.l.stop_read)(v.v);
                    if r.owner == service!(self.ss.tile_owner)(s.tile)
                        && service!(self.vs.order_type)(v.v) != OT_LEAVESTATION
                        && service!(self.l.stop_type)(s.tile) == u8::from(!r.bus)
                    {
                        let station = service!(self.ss.station_index)(s.tile);
                        service!(self.vs.write_last_station)(v.v, station);
                        self.arrive_load(v, s, station);
                    }
                }
                return false;
            }
        }
        if new_dir != s.direction {
            if self.original() {
                self.set_speed(v, s, s.cur_speed - (s.cur_speed >> 2));
            }
            service!(self.gs.turn)(v.v, new_dir);
            s.direction = new_dir;
            return true;
        }
        let state = v.state();
        let frame = v.frame();
        // The frame test is pure and decides most drive-through passes without a read.
        if s.front
            && ((in_range(state, STOP, STOP + 16)
                && crate::road_data::_ROAD_STOP_STOP_FRAME
                    [usize::from(state - STOP) + self.side()]
                    == frame)
                || (in_range(state, DT_STOP, DT_STOP + 16)
                    && frame == 11
                    && self.stops_here(v, s.tile)))
        {
            let station = service!(self.ss.station_index)(s.tile);
            if state & 4 == 0 {
                if service!(self.l.is_drive_through)(s.tile) {
                    let next = self.map.add_dir(s.tile, s.direction);
                    if service!(self.l.continuation)(s.tile, next)
                        && service!(self.l.has_road)(v.v, next)
                    {
                        v.set(FRAME, u16::from(frame) + 1);
                        self.move_to(v, s, x, y, true, false);
                        return true;
                    }
                }
                service!(self.l.stop_entrance)(v.v, false);
                v.set(STATE, u16::from(state | 4));
                service!(self.vs.write_last_station)(v.v, station);
                if service!(self.l.is_drive_through)(s.tile)
                    || (service!(self.vs.order_type)(v.v) == OT_GOTO_STATION
                        && service!(self.vs.order_destination)(v.v) == station)
                {
                    self.arrive_load(v, s, station);
                    return false;
                }
            } else {
                if service!(self.l.stop_entrance_busy)(v.v) {
                    self.set_speed(v, s, 0);
                    return false;
                }
                if service!(self.vs.order_type)(v.v) == OT_LEAVESTATION {
                    service!(self.vs.order_free)(v.v);
                }
            }
            if service!(self.l.is_bay_stop)(s.tile) {
                service!(self.l.stop_entrance)(v.v, true);
            }
            self.start_sound(v);
            service!(self.vs.start_stop_dirty)(v.v);
        }
        let e = service!(self.l.enter_tile)(v.v, s.tile, x, y);
        Self::observe(s, e);
        if e.flags & VETS_CANNOT_ENTER != 0 {
            self.set_speed(v, s, 0);
            return false;
        }
        if e.order_type == OT_LEAVESTATION && service!(self.l.is_drive_through)(s.tile) {
            service!(self.vs.order_free)(v.v);
        }
        if e.flags & VETS_ENTERED_WORMHOLE == 0 {
            v.set(FRAME, u16::from(v.frame()) + 1);
        }
        self.move_to(v, s, x, y, false, true);
        true
    }
    /// Wormhole entry: x/y, `UpdatePosition`, `UpdateInclination(true, true)`.
    fn move_without_speed(&self, v: Part, s: &mut Step, x: i32, y: i32) {
        let m = service!(self.gs.move_incline)(v.v, x, y, true, true);
        s.x = x;
        s.y = y;
        s.z = m.z;
        s.cur_speed = m.cur_speed;
    }
    /// One `IndividualRoadVehicleController` pass over the consist.
    fn consist_step(&self, v: Part, s: &mut Step) -> Option<bool> {
        if !self.individual(v, None, s) {
            return None;
        }
        let mut prev = v;
        let link = service!(self.l.link)(v.v);
        let mut next = Part::link(link.next, link.next_state);
        let traversed_tail = next.is_some();
        while let Some(u) = next {
            let mut us = service!(self.l.step)(u.v);
            if !self.individual(u, Some(prev), &mut us) {
                return None;
            }
            prev = u;
            let link = service!(self.l.link)(u.v);
            next = Part::link(link.next, link.next_state);
        }
        Some(traversed_tail)
    }
    /// `RoadVehController`.
    fn controller(&self, v: Part, status: u8, tick: u8) -> bool {
        let reverse = v.get(REVERSE_CTR);
        if reverse != 0 {
            v.set(REVERSE_CTR, reverse - 1);
        }
        if status & VS_CRASHED != 0 || self.train_crash(v, service!(self.l.crash_part)(v.v)) {
            return self.crashed(v, tick);
        }
        let b = if crate::witness::enabled() {
            unsafe { crate::vehicle::maintenance::handle_breakdown::<true>(v.v, self.vs) }
        } else {
            unsafe { crate::vehicle::maintenance::handle_breakdown::<false>(v.v, self.vs) }
        };
        if b.broken {
            return true;
        }
        if b.status & VS_STOPPED != 0 {
            service!(self.gs.set_last_speed)(v.v);
            return true;
        }
        if service!(self.l.process_orders_then_loading)(v.v) == OT_LOADING {
            return true;
        }
        if v.state() == DEPOT {
            if service!(self.vs.waiting_for_unbunching)(v.v) {
                return true;
            }
            if self.leave_depot(v, true) {
                return true;
            }
        }
        let r = service!(self.l.visual_then_speed)(v.v, self.realistic());
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
        let (mut j, speed) = self.update_speed(v, &r);
        s.cur_speed = speed;
        let mut adv = advance_distance(s.direction);
        let mut blocked = false;
        while j >= adv {
            j -= adv;
            let Some(traversed_tail) = self.consist_step(v, &mut s) else {
                blocked = true;
                break;
            };
            if traversed_tail {
                // A traversed tail may have changed the front, including by detaching itself.
                s = service!(self.l.step)(v.v);
            }
            adv = advance_distance(s.direction);
            if j >= adv {
                let crossing = v.state() != WORMHOLE && service!(self.ss.is_level_crossing)(s.tile);
                let link = service!(self.l.link)(v.v);
                let first = CrashPart {
                    next: link.next,
                    next_state: link.next_state,
                    tile: s.tile,
                    z: s.z,
                    crossing,
                };
                if self.train_crash(v, first) {
                    break;
                }
            }
        }
        let last = service!(self.gs.set_last_speed)(v.v);
        let mut final_progress = None;
        let mut next = if last.status & VS_HIDDEN == 0 {
            let r = service!(self.gs.update_viewport_then_next)(v.v, v.v);
            if r.next.is_none() {
                final_progress = Some(r.final_progress);
            }
            r.next
        } else {
            service!(self.vs.next)(v.v)
        };
        while let Some(u) = next {
            next = if service!(self.vs.status)(u) & VS_HIDDEN == 0 {
                let r = service!(self.gs.update_viewport_then_next)(u, v.v);
                if r.next.is_none() {
                    final_progress = Some(r.final_progress);
                }
                r.next
            } else {
                service!(self.vs.next)(u)
            };
        }
        // A hidden final part has no viewport observation; read progress now.
        let progress = final_progress.unwrap_or_else(|| service!(self.vs.progress)(v.v));
        if progress == 0 {
            service!(self.vs.write_progress)(v.v, if blocked { adv - 1 } else { j } as u8);
        }
        true
    }
    /// `RoadVehicle::Tick`.
    fn tick(&mut self, v: Part, t: &TickRead) -> bool {
        let tick = t.tick.wrapping_add(1);
        if !t.front {
            service!(self.vs.write_tick)(v.v, tick);
            return true;
        }
        let front = service!(self.l.tick_front)(v.v);
        self.acceleration_model = front.movement.acceleration_model;
        self.road_side = front.movement.road_side;
        self.queue = front.movement.queue;
        self.map.log_x = front.movement.map_log_x;
        let running = if front.status & VS_STOPPED == 0 {
            front.running.wrapping_add(1)
        } else {
            front.running
        };
        service!(self.vs.write_counters)(v.v, tick, running, front.order_time.wrapping_add(1));
        self.controller(v, front.status, tick)
    }
    /// `RoadVehicle::SetDestTile`.
    fn set_dest(&self, v: Part, tile: u32, current: u32) {
        if tile == current {
            return;
        }
        v.path_clear();
        service!(self.vs.write_dest)(v.v, tile);
    }
    /// `RoadVehicle::GetRunningCost`.
    fn running_cost(&self, v: Part) -> i64 {
        let r = service!(self.l.cost_read)(v.v);
        if r.cost_class == INVALID_PRICE {
            return 0;
        }
        let factor = service!(self.vs.property)(v.v, 9, i32::from(r.cost_factor)) as u32;
        if factor == 0 {
            return 0;
        }
        service!(self.l.price)(r.engine, factor)
    }
    /// `CheckIfRoadVehNeedsService`; `IsChainInDepot` and `FindClosestRoadDepot` inline.
    fn service(&self, v: Part) {
        let r = service!(self.l.service_read)(v.v);
        if r.servint == 0 || !service!(self.vs.needs_automatic_servicing)(v.v) {
            return;
        }
        let depot_tile = service!(self.l.is_road_depot_tile)(r.tile);
        let mut all_in_depot = depot_tile && r.cur_speed == 0;
        if all_in_depot && v.state() != DEPOT {
            all_in_depot = false;
        }
        if all_in_depot {
            let mut next = {
                let p = service!(self.l.link)(v.v);
                Part::link(p.next, p.next_state)
            };
            while let Some(u) = next {
                let p = service!(self.l.link_tile)(u.v);
                if u.state() != DEPOT || p.tile != r.tile {
                    all_in_depot = false;
                    break;
                }
                next = Part::link(p.link.next, p.link.next_state);
            }
        }
        if all_in_depot {
            service!(self.vs.service_in_depot)(v.v);
            return;
        }
        let penalty = service!(self.l.max_depot_penalty)();
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
            && !crate::services::chance16_i(1, 20, self.random())
        {
            return;
        }
        service!(self.gs.go_to_depot_service)(v.v, depot_id);
        self.set_dest(v, depot.tile, service!(self.vs.dest_tile)(v.v));
        service!(self.vs.start_stop_dirty)(v.v);
    }
    /// `RoadVehicle::OnNewEconomyDay`.
    fn economy_day(&self, v: Part) {
        let day = service!(self.vs.economy_age)(v.v).wrapping_add(1);
        service!(self.vs.write_day)(v.v, day);
        if day & 7 == 0 {
            service!(self.vs.decrease_value)(v.v);
        }
        if v.get(BLOCKED) == 0 {
            service!(self.vs.check_breakdown)(v.v);
        }
        self.service(v);
        let running = service!(self.vs.check_orders)(v.v);
        if running == 0 {
            return;
        }
        let cost = self.running_cost(v).saturating_mul(i64::from(running)) / YEAR_TICKS;
        service!(self.l.pay_running)(v.v, cost);
    }
    /// `CmdTurnRoadVeh` after the ownership checks.
    fn turn(&self, v: Part, execute: bool) -> bool {
        let r = service!(self.l.turn_read)(v.v);
        if r.status & (VS_STOPPED | VS_CRASHED) != 0
            || r.breakdown_ctr != 0
            || v.overtaking() != 0
            || v.state() == WORMHOLE
            || v.state() == DEPOT
            || r.order_type == OT_LOADING
        {
            return false;
        }
        if service!(self.l.is_normal_road)(r.tile)
            && service!(self.l.disallowed_directions)(r.tile) != 0
        {
            return false;
        }
        if service!(self.ss.tile_type)(r.tile) == MP_TUNNELBRIDGE
            && diag(r.direction) == service!(self.ss.tunnel_bridge_direction)(r.tile)
        {
            return false;
        }
        if execute {
            v.set(REVERSE_CTR, 180);
            service!(self.vs.reset_depot_unbunching)(v.v);
        }
        true
    }
    /// `RoadVehicle::GetVehicleTrackdir`.
    fn trackdir(&self, v: Part) -> u8 {
        let r = service!(self.l.trackdir_read)(v.v);
        if r.status & VS_CRASHED != 0 {
            return INVALID_TRACKDIR;
        }
        if v.state() == DEPOT {
            return DIAG_TRACK[usize::from(service!(self.l.depot_direction)(r.tile))];
        }
        if service!(self.l.is_bay_stop)(r.tile) {
            return DIAG_TRACK[usize::from(service!(self.l.bay_direction)(r.tile))];
        }
        let state = v.state();
        if state > 15 {
            return DIAG_TRACK[usize::from(diag(r.direction))];
        }
        if reversing(state) { state - 6 } else { state }
    }
    /// `RoadVehicle::HasToUseGetSlopePixelZ`.
    fn slope_pixel(&self, v: VehicleRef, first: Part, direction: u8) -> bool {
        let state = first.state();
        if state <= 15 && reversing(state) {
            return true;
        }
        let mut u = first.v;
        while u != v {
            if direction != service!(self.vs.direction)(u) {
                return true;
            }
            // The walk reaches `v` before the chain ends, as in the original.
            let Some(next) = service!(self.vs.next)(u) else {
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

/// # Safety
/// Live handle/owner pair, entry record and tables for this synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_tick(
    v: VehicleRef,
    state: *mut State,
    read: *const TickRead,
    leaves: *const Leaves,
) -> bool {
    crate::witness::hit(crate::witness::ROAD_TICKS);
    // SAFETY: Records are borrowed for this call only.
    let t = unsafe { &*read };
    entry!(leaves, None).tick(Part::new(v, state), t)
}
/// # Safety
/// As `openttd_rust_road_tick`; `prev` is null or a live pair.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_individual(
    v: VehicleRef,
    state: *mut State,
    prev: Option<VehicleRef>,
    prev_state: *mut State,
    entry: *const Entry,
    leaves: *const Leaves,
) -> bool {
    // SAFETY: Entry record borrowed for this call only.
    let e = unsafe { &*entry };
    let g = entry!(leaves, Some(e));
    let mut s = service!(g.l.step)(v);
    g.individual(Part::new(v, state), Part::link(prev, prev_state), &mut s)
}
/// # Safety
/// As `openttd_rust_road_tick`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_leave_depot(
    v: VehicleRef,
    state: *mut State,
    first: bool,
    entry: *const Entry,
    leaves: *const Leaves,
) -> bool {
    // SAFETY: Entry record borrowed for this call only.
    let e = unsafe { &*entry };
    entry!(leaves, Some(e)).leave_depot(Part::new(v, state), first)
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
        // SAFETY: Static tables.
        unsafe { service!((*(*leaves).vehicle).age)(v) };
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
    read: *const MaxSpeedRead,
    leaves: *const Leaves,
) -> i32 {
    // SAFETY: Records borrowed for this call only.
    let r = unsafe { *read };
    let mut game = entry!(leaves, None);
    game.acceleration_model = acceleration_model;
    game.max_speed(Part::new(v, state), r)
}
/// # Safety
/// As `openttd_rust_road_tick`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_set_dest(
    v: VehicleRef,
    state: *mut State,
    tile: u32,
    current: u32,
    leaves: *const Leaves,
) {
    entry!(leaves, None).set_dest(Part::new(v, state), tile, current);
}
/// # Safety
/// As `openttd_rust_road_tick`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_turn(
    v: VehicleRef,
    state: *mut State,
    execute: bool,
    leaves: *const Leaves,
) -> bool {
    entry!(leaves, None).turn(Part::new(v, state), execute)
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_slope_pixel(
    v: VehicleRef,
    first: VehicleRef,
    first_state: *mut State,
    direction: u8,
    leaves: *const Leaves,
) -> bool {
    entry!(leaves, None).slope_pixel(v, Part::new(first, first_state), direction)
}
/// Shared modern/legacy afterload road-stop frame query, retaining original indexes.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_stop_frame(index: u32) -> u8 {
    crate::road_data::_ROAD_STOP_STOP_FRAME[index as usize]
}

/// Narrow unchanged-reference table comparison for the corpus's tram-table gap.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_drive_entry(tram: u8, state: u8, frame: u8) -> u16 {
    let (x, y) = crate::road_data::entry(tram != 0, state as usize, frame as usize);
    u16::from(x) | (u16::from(y) << 8)
}
