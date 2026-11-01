/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete ship controller and canonical private scalars. Shared path stays in `ship_yapf`.
//! Entries keep vehicle shells and their service tables live and serialized through
//! maintenance calls; no owner borrow spans a callback that can reenter or mutate it.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::similar_names,
    clippy::items_after_statements,
    clippy::verbose_bit_mask,
    clippy::needless_late_init,
    clippy::if_not_else
)]
use crate::game_state::GameState;
use crate::order_type::{
    OT_GOTO_DEPOT, OT_GOTO_STATION, OT_GOTO_WAYPOINT, OT_LEAVESTATION, OT_LOADING,
};
use crate::services::Services;
use crate::tile_type::{MP_INDUSTRY, MP_TUNNELBRIDGE};
use crate::vehicle::{VehicleRef, VehicleServices, maintenance};
use crate::vehicle_base::VS_STOPPED;
use crate::water_regions::Patch;
use std::collections::{HashSet, VecDeque};
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_ship_control_reload_rotation() {
    crate::witness::hit(crate::witness::SHIP_ROTATION_RELOAD);
}

const INVALID: u32 = u32::MAX;
const DEPOT: u16 = crate::track_type::TRACK_BIT_DEPOT as u16;
const WORMHOLE: u16 = crate::track_type::TRACK_BIT_WORMHOLE as u16;
// Match FindFirstBit and FindFirstTrack, including their distinct zero results.
fn first_bit(bits: u32) -> u32 {
    if bits == 0 { 0 } else { bits.trailing_zeros() }
}
fn first_track(bits: u32) -> u32 {
    if bits == 0 { 255 } else { first_bit(bits) }
}
/// One private allocation per shell, including indexed load construction.
pub struct State {
    track: u8,
    rotation: u8,
    rotation_x: i16,
    rotation_y: i16,
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_ship_state_new() -> *mut State {
    Box::into_raw(Box::new(State {
        track: 0,
        rotation: 255,
        rotation_x: 0,
        rotation_y: 0,
    }))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_state_destroy(state: *mut State) {
    // SAFETY: The shell transfers its live allocation once after PreDestructor.
    unsafe {
        drop(Box::from_raw(state));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_get_state(state: *const State) -> u8 {
    // SAFETY: Live shell allocation; field-sized read retains no owner reference.
    unsafe { (*state).track }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_set_state(state: *mut State, value: u8) {
    // SAFETY: Serialized game-thread write; no borrow survives a service callback.
    unsafe {
        (*state).track = value;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_get_rotation(state: *const State) -> u8 {
    // SAFETY: Live shell allocation; field-sized read retains no owner reference.
    unsafe { (*state).rotation }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_set_rotation(state: *mut State, value: u8) {
    // SAFETY: Serialized game-thread write; no borrow survives a service callback.
    unsafe {
        (*state).rotation = value;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_get_rotation_x(state: *const State) -> i16 {
    // SAFETY: Live shell allocation; field-sized read retains no owner reference.
    unsafe { (*state).rotation_x }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_set_rotation_x(state: *mut State, value: i16) {
    // SAFETY: Serialized game-thread write; no borrow survives a service callback.
    unsafe {
        (*state).rotation_x = value;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_get_rotation_y(state: *const State) -> i16 {
    // SAFETY: Live shell allocation; field-sized read retains no owner reference.
    unsafe { (*state).rotation_y }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_set_rotation_y(state: *mut State, value: i16) {
    // SAFETY: Serialized game-thread write; no borrow survives a service callback.
    unsafe {
        (*state).rotation_y = value;
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Depot {
    pub id: u32,
    pub tile: u32,
    pub owner: u32,
    pub ship: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub old_tile: u32,
    pub new_tile: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ReverseResult {
    pub reverse: bool,
    pub trackdir: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrackResult {
    pub track: u8,
    pub found: bool,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct DepotResult {
    pub tile: u32,
    pub id: u16,
    pub valid: bool,
}
/// Synchronous typed shared services. No borrow into the owner or world spans a call.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Leaves {
    pub depot_dir: extern "C" fn(u32) -> u32,
    pub depot_axis: extern "C" fn(u32) -> u32,
    pub is_depot: extern "C" fn(u32) -> bool,
    pub depot_index: extern "C" fn(u32) -> u32,
    pub wait_unbunch: extern "C" fn(u32) -> bool,
    pub chain_depot: extern "C" fn(u32) -> bool,
    pub servint: extern "C" fn(u32) -> u32,
    pub max_distance: extern "C" fn() -> u32,
    pub tile_valid: extern "C" fn(u32) -> bool,
    pub tile_type: extern "C" fn(u32) -> u32,
    pub water_class: extern "C" fn(u32) -> u32,
    pub lock_middle: extern "C" fn(u32) -> bool,
    pub lock_dir: extern "C" fn(u32) -> u32,
    pub tile_min_z: extern "C" fn(u32) -> u32,
    pub tile_max_z: extern "C" fn(u32) -> u32,
    pub offset: extern "C" fn(u32) -> u32,
    pub diag_between: extern "C" fn(u32, u32) -> u32,
    pub dist_square: extern "C" fn(u32, u32) -> u32,
    pub dist_manhattan: extern "C" fn(u32, u32) -> u32,
    pub docking: extern "C" fn(u32) -> bool,
    pub dock: extern "C" fn(u32) -> bool,
    pub dock_water: extern "C" fn(u32) -> bool,
    pub station: extern "C" fn(u32) -> u32,
    pub industry_station: extern "C" fn(u32) -> u32,
    pub oilrig: extern "C" fn(u32) -> bool,
    pub station_xy: extern "C" fn(u32) -> u32,
    pub station_dock: extern "C" fn(u32) -> bool,
    pub station_visits: extern "C" fn(u32) -> u32,
    pub visit_set: extern "C" fn(u32, u32),
    pub arrival: extern "C" fn(u32, u32),
    pub leave_unbunch: extern "C" fn(u32),
    pub path_result: extern "C" fn(u32, bool),
    pub order_free: extern "C" fn(u32),
    pub order_dummy: extern "C" fn(u32),
    pub order_depot: extern "C" fn(u32, u32),
    pub order_leave: extern "C" fn(u32),
    pub order_increment: extern "C" fn(u32),
    pub timetable: extern "C" fn(u32),
    pub start_dirty: extern "C" fn(u32),
    pub depot_dirty: extern "C" fn(u32),
    pub depot_invalidate: extern "C" fn(u32),
    pub ships_dirty: extern "C" fn(),
    pub check_orders: extern "C" fn(u32),
    pub cost_divisor: extern "C" fn() -> u32,
    pub pay_running: extern "C" fn(u32, i64),
    pub speed_default: extern "C" fn(u32) -> u32,
    pub age_default: extern "C" fn(u32) -> u32,
    pub speed_frac: extern "C" fn(u32, bool) -> u32,
    pub cache_invalidate: extern "C" fn(u32),
    pub(crate) capabilities: &'static crate::engine_capabilities::Services,
    pub sprite_direction: extern "C" fn(u32),
    pub tile_x: extern "C" fn(u32) -> u32,
    pub tile_y: extern "C" fn(u32) -> u32,
    pub build_flag: extern "C" fn(u32) -> bool,
    pub build_random: extern "C" fn(u32, u16),
    pub new_position: extern "C" fn(u32) -> Position,
    pub exit_dir: extern "C" fn(u32, u32) -> u32,
    pub track_direction: extern "C" fn(u32, u32) -> u32,
    pub tracks_reach: extern "C" fn(u32) -> u32,
    pub path_size: extern "C" fn(u32) -> usize,
    pub path_back: extern "C" fn(u32) -> u32,
    pub path_pop: extern "C" fn(u32),
    pub path_clear: extern "C" fn(u32),
    pub viewport: extern "C" fn(u32, bool, bool),
    pub base_viewport: extern "C" fn(u32),
    pub cache: extern "C" fn(u32),
    pub play_sound: extern "C" fn(u32),
    pub yapf_reverse: extern "C" fn(u32, bool) -> ReverseResult,
    pub yapf_choose: extern "C" fn(u32, u32) -> TrackResult,
    pub update_delta: extern "C" fn(u32),
    pub build_owner: extern "C" fn(u32),
    pub build_z: extern "C" fn(u32),
    pub build_properties: extern "C" fn(u32, u32),
    pub build_dates: extern "C" fn(u32),
    pub build_acceleration: extern "C" fn(u32, u32),
    pub build_prototype: extern "C" fn(u32),
    pub build_interval_percent: extern "C" fn(u32),
    pub build_capacity: extern "C" fn(u32, u32),
    pub set_tile: extern "C" fn(u32, u32),
    pub set_x: extern "C" fn(u32, i32),
    pub set_y: extern "C" fn(u32, i32),
    pub set_z: extern "C" fn(u32, i32),
    pub set_direction: extern "C" fn(u32, u8),
    pub set_speed: extern "C" fn(u32, u16),
    pub set_tick: extern "C" fn(u32, u8),
    pub set_running: extern "C" fn(u32, u8),
    pub set_day: extern "C" fn(u32, u8),
    pub set_order_time: extern "C" fn(u32, i32),
    pub set_progress: extern "C" fn(u32, u8),
    pub set_last_station: extern "C" fn(u32, u16),
    pub set_hidden: extern "C" fn(u32, bool),
    pub set_max_speed: extern "C" fn(u32, u16),
    pub set_cargo_age: extern "C" fn(u32, u16),
    pub set_dest: extern "C" fn(u32, u32),
    pub tile: extern "C" fn(u32) -> u32,
    pub dest: extern "C" fn(u32) -> u32,
    pub x: extern "C" fn(u32) -> u32,
    pub y: extern "C" fn(u32) -> u32,
    pub z: extern "C" fn(u32) -> u32,
    pub direction: extern "C" fn(u32) -> u32,
    pub speed: extern "C" fn(u32) -> u32,
    pub tick: extern "C" fn(u32) -> u32,
    pub running: extern "C" fn(u32) -> u32,
    pub day: extern "C" fn(u32) -> u32,
    pub order_time: extern "C" fn(u32) -> u32,
    pub progress: extern "C" fn(u32) -> u32,
    pub status: extern "C" fn(u32) -> u32,
    pub owner: extern "C" fn(u32) -> u32,
    pub last_station: extern "C" fn(u32) -> u32,
    pub order_destination: extern "C" fn(u32) -> u32,
    pub order_type: extern "C" fn(u32) -> u32,
    pub order_max_speed: extern "C" fn(u32) -> u32,
    pub acceleration: extern "C" fn(u32) -> u32,
    pub max_speed: extern "C" fn(u32) -> u32,
    pub state_owner: extern "C" fn(u32) -> *mut State,
    pub patch: extern "C" fn(u32) -> Patch,
    pub neighbours: extern "C" fn(Patch, *mut Patch) -> usize,
    pub next_depot: extern "C" fn(u32, *mut Depot) -> bool,
    /// Base vehicle table, immutable for the process.
    pub vehicle: *const VehicleServices,
}
/// The original reuses one process-wide BFS set/deque. Field-sized scratch borrows
/// end before water-region callbacks under the game-state mutex.
struct DepotScratch {
    visited: HashSet<(i32, i32, u8)>,
    queue: VecDeque<Patch>,
}
static DEPOT_SCRATCH: GameState<Option<DepotScratch>> = GameState::new(None);
// The caller serializes scratch access; every borrow ends before a service call.
unsafe fn depot_scratch() -> *mut DepotScratch {
    // SAFETY: Serialized initialization; the short borrow ends before returning.
    unsafe {
        std::ptr::from_mut((*DEPOT_SCRATCH.get()).get_or_insert_with(|| DepotScratch {
            visited: HashSet::new(),
            queue: VecDeque::new(),
        }))
    }
}
struct Game<'a> {
    leaves: &'a Leaves,
    services: &'a Services,
    owner: *mut State,
}
impl Game<'_> {
    fn vs(&self) -> &VehicleServices {
        // SAFETY: The native table names the immutable process-wide base table.
        unsafe { &*self.leaves.vehicle }
    }
    fn state(&self) -> u16 {
        // SAFETY: Raw shell pointer stays live; field access ends before any service.
        unsafe { u16::from((*self.owner).track) }
    }
    fn set_state(&self, value: u16) {
        // SAFETY: Serialized field-sized write, no owner borrow spans reentry.
        unsafe {
            (*self.owner).track = value as u8;
        }
    }
    fn rotation(&self) -> u16 {
        // SAFETY: Raw shell pointer stays live; field access ends before any service.
        unsafe { u16::from((*self.owner).rotation) }
    }
    fn set_rotation(&self, value: u16) {
        // SAFETY: Serialized field-sized write, no owner borrow spans reentry.
        unsafe {
            (*self.owner).rotation = value as u8;
        }
    }
    fn set_rotation_x(&self, value: u16) {
        // SAFETY: Serialized field-sized write, no owner borrow spans reentry.
        unsafe {
            (*self.owner).rotation_x = value as i16;
        }
    }
    fn set_rotation_y(&self, value: u16) {
        // SAFETY: Serialized field-sized write, no owner borrow spans reentry.
        unsafe {
            (*self.owner).rotation_y = value as i16;
        }
    }
}
impl Game<'_> {
    fn viewport(&self, id: u32) {
        (self.leaves.viewport)(id, true, true);
    }
    fn path_clear(&self, id: u32) {
        (self.leaves.path_clear)(id);
    }
    fn destination(&self, id: u32, tile: u32) {
        if tile == (self.leaves.dest)(id) {
            return;
        }
        self.path_clear(id);
        (self.leaves.set_dest)(id, tile);
    }
    fn remember_rotation(&self, id: u32) {
        let observed_x = (self.leaves.x)(id);
        let observed_y = (self.leaves.y)(id);
        self.set_rotation_x(observed_x as u16);
        self.set_rotation_y(observed_y as u16);
    }
    fn reverse(&self, id: u32, v: VehicleRef, trackdir: u32) {
        crate::witness::hit(crate::witness::SHIP_REVERSE);
        if trackdir == INVALID {
            (self.leaves.set_direction)(id, (((self.leaves.direction)(id) + 4) & 7) as u8);
        } else {
            const DIRS: [u32; 16] = [1, 3, 2, 2, 4, 4, 255, 255, 5, 7, 6, 6, 0, 0, 255, 255];
            (self.leaves.set_direction)(id, DIRS[trackdir as usize] as u8);
            self.set_state(1 << (trackdir & 7));
        }
        self.remember_rotation(id);
        (self.leaves.set_speed)(id, 0);
        self.path_clear(id);
        // SAFETY: Caller retains the live shell across this index update.
        unsafe {
            crate::vehicle::occupancy::update(self.vs(), v, false);
        }
        self.viewport(id);
    }
    fn depot(&self, id: u32, max_distance: u32) -> Option<Depot> {
        crate::witness::hit(crate::witness::SHIP_DEPOT_SEARCH);
        // SAFETY: The native game-state mutex serializes depot searches.
        let scratch = unsafe { depot_scratch() };
        let observed_tile = (self.leaves.tile)(id);
        let observed_owner = (self.leaves.owner)(id);
        let start = (self.leaves.patch)(observed_tile);
        // SAFETY: The serialized game-thread entry owns scratch access. No reference crosses a service call.
        unsafe {
            (*scratch).visited.clear();
            (*scratch).queue.clear();
        }
        let key = |p: Patch| (p.x, p.y, p.label);
        // SAFETY: Scratch borrows end before the neighbour callback below.
        unsafe {
            (*scratch).visited.insert(key(start));
            (*scratch).queue.push_back(start);
        }
        let max_region = (max_distance / 16 + 1) as i32;
        // SAFETY: Pop copies the patch and ends the scratch borrow before visiting neighbours.
        while let Some(current) = unsafe { (*scratch).queue.pop_front() } {
            let mut neighbours = [Patch {
                x: 0,
                y: 0,
                label: 0,
            }; 320];
            let count = (self.leaves.neighbours)(current, neighbours.as_mut_ptr());
            for patch in &neighbours[..count] {
                if (patch.x - start.x).abs() > max_region || (patch.y - start.y).abs() > max_region
                {
                    continue;
                }
                // SAFETY: Only scratch changes; no reference survives the next callback.
                unsafe {
                    if (*scratch).visited.insert(key(*patch)) {
                        (*scratch).queue.push_back(*patch);
                    }
                }
            }
        }
        let mut best = None;
        let mut best_distance = u32::MAX;
        let mut first = 0;
        let mut depot = Depot::default();
        while (self.leaves.next_depot)(first, &raw mut depot) {
            first = depot.id + 1;
            if depot.ship == 0 || depot.owner != observed_owner {
                continue;
            }
            let distance = (self.leaves.dist_square)(depot.tile, observed_tile);
            if distance < best_distance && distance <= max_distance.wrapping_mul(max_distance) && {
                let patch = (self.leaves.patch)(depot.tile);
                // SAFETY: The callback has returned before taking this short scratch borrow.
                unsafe { (*scratch).visited.contains(&key(patch)) }
            } {
                best_distance = distance;
                best = Some(depot);
            }
        }
        best
    }
    fn service(&self, id: u32, v: VehicleRef) {
        if (self.leaves.servint)(id) == 0
            || !unsafe {
                self.vs()
                    .maintain(v, maintenance::needs_automatic_servicing)
            }
        {
            return;
        }
        if (self.leaves.chain_depot)(id) {
            unsafe { self.vs().maintain(v, maintenance::service_in_depot) };
            return;
        }
        let distance = (self.leaves.max_distance)();
        if let Some(depot) = self.depot(id, distance) {
            crate::witness::hit(crate::witness::SHIP_AUTO_SERVICE);
            (self.leaves.order_depot)(id, depot.id);
            self.destination(id, depot.tile);
            (self.leaves.start_dirty)(id);
        } else if (self.leaves.order_type)(id) == u32::from(OT_GOTO_DEPOT) {
            (self.leaves.order_dummy)(id);
            (self.leaves.start_dirty)(id);
        }
    }
    fn cache(&self, id: u32, v: VehicleRef) {
        let observed_tile = (self.leaves.tile)(id);
        let sea = (self.leaves.water_class)(observed_tile) == 0;
        let speed = (self.leaves.speed_default)(id);
        let raw = (self.vs().property)(
            (self.vs().newgrf)(),
            v,
            crate::newgrf_properties::PROP_SHIP_SPEED,
            speed as i32,
        ) as u32;
        let frac = (self.leaves.speed_frac)(id, sea);
        (self.leaves.set_max_speed)(id, (raw.wrapping_mul(256 - frac) / 256) as u16);
        let period = (self.leaves.age_default)(id);
        let age = (self.vs().property)(
            (self.vs().newgrf)(),
            v,
            crate::newgrf_properties::PROP_SHIP_CARGO_AGE_PERIOD,
            period as i32,
        ) as u32;
        (self.leaves.set_cargo_age)(id, age as u16);
        crate::vehicle_effect::openttd_rust_visual_update(v, true);
    }
    fn stay_depot(&self, id: u32, v: VehicleRef) -> bool {
        if !(self.leaves.chain_depot)(id) {
            return false;
        }
        if (self.leaves.wait_unbunch)(id) {
            return true;
        }
        let observed_tile = (self.leaves.tile)(id);
        let observed_dest = (self.leaves.dest)(id);
        let observed_order_destination = (self.leaves.order_destination)(id);
        let observed_order_type = (self.leaves.order_type)(id);
        if observed_order_type == u32::from(OT_GOTO_DEPOT)
            && (self.leaves.is_depot)(observed_tile)
            && (self.leaves.depot_index)(observed_tile) == observed_order_destination
        {
            self.vs().enter_depot(v);
            return true;
        }
        if observed_dest == 0
            || self.vs().any_on_tile((self.leaves.tile)(id), |u| {
                (self.vs().vehicle_type)(u) == crate::vehicle::VehicleType::SHIP.0
                    && (self.vs().speed)(u) != 0
            })
        {
            return true;
        }
        let mut direction = (self.leaves.depot_dir)(observed_tile) * 2 + 1;
        (self.leaves.set_direction)(id, direction as u8);
        if (self.leaves.yapf_reverse)(id, false).reverse {
            direction = (direction + 4) & 7;
            (self.leaves.set_direction)(id, direction as u8);
        }
        crate::witness::hit(crate::witness::SHIP_DEPOT_LEAVE);
        self.set_state(1 << (self.leaves.depot_axis)(observed_tile));
        self.set_rotation(direction as u16);
        (self.leaves.set_hidden)(id, false);
        (self.leaves.set_speed)(id, 0);
        self.viewport(id);
        (self.leaves.depot_dirty)(id);
        unsafe { self.vs().maintain(v, maintenance::service_in_depot) };
        (self.leaves.leave_unbunch)(id);
        (self.leaves.play_sound)(id);
        (self.leaves.depot_invalidate)(id);
        (self.leaves.ships_dirty)();
        false
    }
    fn lock_delta(&self, id: u32) -> i32 {
        let tile = (self.leaves.tile)(id);
        if !(self.leaves.lock_middle)(tile)
            || (self.leaves.x)(id) & 15 != 8
            || (self.leaves.y)(id) & 15 != 8
        {
            return 0;
        }
        let diag = (self.leaves.lock_dir)(tile);
        if (self.leaves.direction)(id) / 2 == diag {
            i32::from(((self.leaves.z)(id) as i32) < (self.leaves.tile_max_z)(tile) as i32 * 8)
        } else {
            -i32::from(((self.leaves.z)(id) as i32) > (self.leaves.tile_min_z)(tile) as i32 * 8)
        }
    }
    fn move_lock(&self, id: u32, v: VehicleRef) -> bool {
        let delta = self.lock_delta(id);
        if delta > 0 {
            crate::witness::hit(crate::witness::SHIP_LOCK_UP);
        } else if delta < 0 {
            crate::witness::hit(crate::witness::SHIP_LOCK_DOWN);
        }
        if delta == 0 {
            return false;
        }
        let observed_z = (self.leaves.z)(id);
        let observed_speed = (self.leaves.speed)(id);
        let observed_tick = (self.leaves.tick)(id);
        if observed_speed != 0 {
            (self.leaves.set_speed)(id, 0);
            (self.leaves.start_dirty)(id);
        }
        if observed_tick & 7 == 0 {
            (self.leaves.set_z)(id, observed_z.wrapping_add(delta as u32) as i32);
            // SAFETY: Caller retains the live shell across this index update.
            unsafe {
                crate::vehicle::occupancy::update(self.vs(), v, false);
            }
            self.viewport(id);
        }
        true
    }
    fn accelerate(&self, id: u32) -> u32 {
        let observed_direction = (self.leaves.direction)(id);
        let observed_speed = (self.leaves.speed)(id);
        let observed_progress = (self.leaves.progress)(id);
        let observed_order_max_speed = (self.leaves.order_max_speed)(id);
        let observed_acceleration = (self.leaves.acceleration)(id);
        let observed_max_speed = (self.leaves.max_speed)(id);
        let speed = observed_speed
            .wrapping_add(observed_acceleration)
            .min(observed_max_speed)
            .min(observed_order_max_speed.wrapping_mul(2));
        if speed != observed_speed {
            (self.leaves.set_speed)(id, speed as u16);
            (self.leaves.start_dirty)(id);
        }
        let advance = speed.wrapping_mul(3) / 4;
        let distance = if observed_direction & 1 != 0 {
            192
        } else {
            256
        };
        let progress = advance.wrapping_add(observed_progress);
        (self.leaves.set_progress)(id, (progress % distance) as u8);
        progress / distance
    }
    fn choose(&self, id: u32, tile: u32, tracks: u32) -> u32 {
        let found;
        let track;
        if (self.leaves.dest)(id) == 0 {
            let mut choice = first_bit(u32::from(self.state()) & 63);
            if choice >= 2 {
                choice ^= 1;
            }
            if tracks & (1 << choice) == 0 {
                choice = first_track(tracks);
            }
            track = choice;
            found = false;
        } else {
            if (self.leaves.path_size)(id) != 0 {
                crate::witness::hit(crate::witness::SHIP_PATH_CACHE);
                let cached = (self.leaves.path_back)(id) & 7;
                if tracks & (1 << cached) != 0 {
                    (self.leaves.path_pop)(id);
                    return cached;
                }
                self.path_clear(id);
            }
            let result = (self.leaves.yapf_choose)(id, tile);
            track = u32::from(result.track);
            found = result.found;
        }
        (self.leaves.path_result)(id, found);
        track
    }
    fn destination_station(&self, tile: u32, station: u32) -> bool {
        for dir in 0..4 {
            let adjacent = tile.wrapping_add((self.leaves.offset)(dir));
            if !(self.leaves.tile_valid)(adjacent) {
                continue;
            }
            if (self.leaves.dock)(adjacent)
                && (self.leaves.station)(adjacent) == station
                && (self.leaves.dock_water)(adjacent)
            {
                return true;
            }
            if (self.leaves.tile_type)(adjacent) == u32::from(MP_INDUSTRY)
                && (self.leaves.industry_station)(adjacent) == station
            {
                return true;
            }
            if (self.leaves.oilrig)(adjacent) && (self.leaves.station)(adjacent) == station {
                return true;
            }
        }
        false
    }
    fn arrives(&self, id: u32, station: u32) {
        let visits = (self.leaves.station_visits)(station);
        if visits & 32 == 0 {
            (self.leaves.visit_set)(station, 32);
            (self.leaves.arrival)(id, station);
        }
    }
    fn controller(&self, id: u32, v: VehicleRef) {
        let observed_tick = (self.leaves.tick)(id);
        let observed_order_time = (self.leaves.order_time)(id);
        (self.leaves.set_tick)(id, observed_tick.wrapping_add(1) as u8);
        (self.leaves.set_order_time)(id, observed_order_time.wrapping_add(1) as i32);
        let breakdown = unsafe { self.vs().maintain(v, maintenance::handle_breakdown_status) };
        if breakdown.broken {
            return;
        }
        if breakdown.status & VS_STOPPED != 0 {
            return;
        }
        // Reacquire with the source strict Ship::Get at the original orders call.
        // No owner borrow spans the lookup or order mutation/reentry.
        let orders_vehicle = (self.vs().ship_get)(id);
        let orders = self.vs().orders();
        let reverse = unsafe {
            crate::orders::openttd_rust_process_orders(orders_vehicle.0.as_ptr().cast(), orders)
        } != 0;
        if reverse && (self.leaves.yapf_reverse)(id, false).reverse {
            self.reverse(id, v, INVALID);
            return;
        }
        self.vs().handle_loading(v, false, orders);
        if (self.leaves.order_type)(id) == u32::from(OT_LOADING) {
            return;
        }
        if self.stay_depot(id, v) {
            return;
        }
        if crate::vehicle_effect::openttd_rust_visual_eligible(v) {
            let maximum = current_max_speed(
                (self.leaves.max_speed)(id) as u16,
                (self.leaves.order_max_speed)(id) as u16,
            );
            crate::vehicle_effect::openttd_rust_visual_show(v, maximum as u32);
        }
        let observed_direction = (self.leaves.direction)(id);
        let observed_tick = (self.leaves.tick)(id);
        let rotation = u32::from(self.rotation());
        if observed_direction != rotation {
            crate::witness::hit(crate::witness::SHIP_ROTATE);
            if observed_tick & 7 == 0 {
                let diff = observed_direction.wrapping_sub(rotation) & 7;
                self.set_rotation(((rotation + if diff > 4 { 7 } else { 1 }) & 7) as u16);
                (self.leaves.sprite_direction)(id);
                self.viewport(id);
            }
            return;
        }
        if self.move_lock(id, v) {
            return;
        }
        let steps = self.accelerate(id);
        for _ in 0..steps {
            if self.move_lock(id, v) {
                return;
            }
            let position = (self.leaves.new_position)(id);
            let mut x = position.x as u32;
            let mut y = position.y as u32;
            let old_tile = position.old_tile;
            let new_tile = position.new_tile;
            if self.state() != WORMHOLE {
                if old_tile == new_tile {
                    if self.state() == DEPOT {
                        x = (self.leaves.x)(id);
                        y = (self.leaves.y)(id);
                    } else {
                        if u32::from(unsafe {
                            crate::movement::enter(self.services, v, new_tile, x as i32, y as i32)
                        }) & 4
                            != 0
                        {
                            self.reverse(id, v, INVALID);
                            return;
                        }
                        let observed_tile = (self.leaves.tile)(id);
                        let observed_dest = (self.leaves.dest)(id);
                        let observed_direction = (self.leaves.direction)(id);
                        let observed_order_destination = (self.leaves.order_destination)(id);
                        let observed_order_type = (self.leaves.order_type)(id);
                        if observed_order_type == u32::from(OT_LEAVESTATION) {
                            (self.leaves.order_free)(id);
                            (self.leaves.start_dirty)(id);
                            let exit =
                                (self.leaves.exit_dir)(observed_direction, u32::from(self.state()));
                            let tile = observed_tile.wrapping_add((self.leaves.offset)(exit));
                            if crate::movement::water_tracks(self.services, tile, exit as u8) == 0 {
                                self.reverse(id, v, INVALID);
                                return;
                            }
                        } else if observed_dest != 0 {
                            if observed_order_type == u32::from(OT_GOTO_WAYPOINT)
                                && (self.leaves.dist_manhattan)(observed_dest, new_tile) <= 3
                            {
                                crate::witness::hit(crate::witness::SHIP_BUOY);
                                (self.leaves.timetable)(id);
                                (self.leaves.order_increment)(id);
                                (self.leaves.order_dummy)(id);
                            } else if observed_order_type == u32::from(OT_GOTO_DEPOT)
                                && observed_dest == new_tile
                            {
                                if x & 15 == 8 && y & 15 == 8 {
                                    crate::witness::hit(crate::witness::SHIP_DEPOT_ENTER);
                                    self.vs().enter_depot(v);
                                    return;
                                }
                            } else if observed_order_type == u32::from(OT_GOTO_STATION)
                                && (self.leaves.docking)(new_tile)
                            {
                                let station = observed_order_destination;
                                // SAFETY: The shared table is immutable; `Station::Get` names a
                                // live station whose docking area is read in place.
                                let docking = unsafe {
                                    let table = &*(self.services.station_commands)();
                                    let st = ((*table.station).get)(station as u16);
                                    *(table.docking_area)(st)
                                };
                                if docking.contains(new_tile, self.services.map(), |ok| {
                                    crate::movement::native_assert(self.services, ok);
                                }) && self.destination_station(new_tile, station)
                                {
                                    (self.leaves.set_last_station)(id, station as u16);
                                    if (self.leaves.station_dock)(station) {
                                        crate::witness::hit(crate::witness::SHIP_LOADING);
                                        self.arrives(id, station);
                                        crate::vehicle::visits::begin_loading(
                                            v,
                                            self.vs().visits(),
                                        );
                                    } else {
                                        (self.leaves.order_leave)(id);
                                        (self.leaves.order_increment)(id);
                                    }
                                }
                            }
                        }
                    }
                } else {
                    if !(self.leaves.tile_valid)(new_tile) {
                        self.reverse(id, v, INVALID);
                        return;
                    }
                    let diag = (self.leaves.diag_between)(old_tile, new_tile);
                    let tracks = crate::movement::water_tracks(self.services, new_tile, 255)
                        & (self.leaves.tracks_reach)(diag);
                    if tracks == 0 {
                        let result = (self.leaves.yapf_reverse)(id, true);
                        let trackdir = u32::from(result.trackdir);
                        self.reverse(id, v, if trackdir == 255 { INVALID } else { trackdir });
                        return;
                    }
                    let track = self.choose(id, new_tile, tracks);
                    if track == 255 {
                        self.reverse(id, v, INVALID);
                        return;
                    }
                    let (sx, sy, direction) = SUBCOORD[diag as usize][track as usize];
                    x = (x & !15) | u32::from(sx);
                    y = (y & !15) | u32::from(sy);
                    let entered = u32::from(unsafe {
                        crate::movement::enter(self.services, v, new_tile, x as i32, y as i32)
                    });
                    if entered & 4 != 0 {
                        self.reverse(id, v, INVALID);
                        return;
                    }
                    if entered & 2 == 0 {
                        (self.leaves.set_tile)(id, new_tile);
                        self.set_state(1 << track);
                        if (self.leaves.water_class)(old_tile)
                            != (self.leaves.water_class)(new_tile)
                        {
                            crate::witness::hit(crate::witness::SHIP_WATER_CHANGE);
                            (self.leaves.cache)(id);
                        }
                    }
                    let diff = u32::from(direction).wrapping_sub((self.leaves.direction)(id)) & 7;
                    if diff == 0 || diff == 1 || diff == 7 {
                        self.set_rotation(u16::from(direction));
                        (self.leaves.set_direction)(id, direction);
                    } else {
                        (self.leaves.set_speed)(id, 0);
                        (self.leaves.set_direction)(id, direction);
                        self.remember_rotation(id);
                    }
                }
            } else {
                crate::witness::hit(crate::witness::SHIP_AQUEDUCT);
                if (self.leaves.tile_type)(new_tile) != u32::from(MP_TUNNELBRIDGE)
                    || u32::from(unsafe {
                        crate::movement::enter(self.services, v, new_tile, x as i32, y as i32)
                    }) & 2
                        == 0
                {
                    (self.leaves.set_x)(id, x as i32);
                    (self.leaves.set_y)(id, y as i32);
                    // SAFETY: Caller retains the live shell across this index update.
                    unsafe {
                        crate::vehicle::occupancy::update(self.vs(), v, false);
                    }
                    if (self.leaves.status)(id) & 1 == 0 {
                        (self.leaves.base_viewport)(id);
                    }
                    continue;
                }
                if (self.leaves.path_size)(id) != 0 {
                    (self.leaves.path_pop)(id);
                }
            }
            (self.leaves.set_x)(id, x as i32);
            (self.leaves.set_y)(id, y as i32);
            // SAFETY: Caller retains the live shell across this index update.
            unsafe {
                crate::vehicle::occupancy::update(self.vs(), v, false);
            }
            self.viewport(id);
        }
    }
}
const SUBCOORD: [[(u8, u8, u8); 6]; 4] = [
    [
        (15, 8, 1),
        (0, 0, 255),
        (0, 0, 255),
        (15, 8, 2),
        (15, 7, 0),
        (0, 0, 255),
    ],
    [
        (0, 0, 255),
        (8, 0, 3),
        (7, 0, 2),
        (0, 0, 255),
        (8, 0, 4),
        (0, 0, 255),
    ],
    [
        (0, 8, 5),
        (0, 0, 255),
        (0, 7, 6),
        (0, 0, 255),
        (0, 0, 255),
        (0, 8, 4),
    ],
    [
        (0, 0, 255),
        (8, 15, 7),
        (0, 0, 255),
        (8, 15, 6),
        (0, 0, 255),
        (7, 15, 0),
    ],
];
/// The `Game` of an entry's tables; `owner` may read the bound `leaves` table.
macro_rules! entry_game {
    ($leaves:ident, $services:ident, $owner:expr) => {{
        // SAFETY: The service tables remain immutable/live for this entry; owner borrows end before calls.
        let $leaves = unsafe { &*$leaves };
        Game {
            leaves: $leaves,
            services: unsafe { &*$services },
            owner: $owner,
        }
    }};
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_tick(
    id: u32,
    vehicle: VehicleRef,
    leaves: *const Leaves,
    services: *const Services,
) -> bool {
    let game = entry_game!(leaves, services, (leaves.state_owner)(id));
    let observed_running = (game.leaves.running)(id);
    let observed_status = (game.leaves.status)(id);
    if observed_status & 2 == 0 {
        (game.leaves.set_running)(id, observed_running.wrapping_add(1) as u8);
    }
    game.controller(id, vehicle);
    true
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_calendar_day(
    id: u32,
    vehicle: VehicleRef,
    leaves: *const Leaves,
    services: *const Services,
) {
    let game = entry_game!(leaves, services, (leaves.state_owner)(id));
    unsafe { game.vs().maintain(vehicle, maintenance::age_vehicle) };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_economy_day(
    id: u32,
    vehicle: VehicleRef,
    leaves: *const Leaves,
    services: *const Services,
) {
    let game = entry_game!(leaves, services, (leaves.state_owner)(id));
    crate::witness::hit(crate::witness::SHIP_ECONOMY_DAY);
    unsafe {
        game.vs()
            .maintain(vehicle, maintenance::economy_age_vehicle);
    };
    let day = (game.leaves.day)(id).wrapping_add(1);
    (game.leaves.set_day)(id, day as u8);
    if day & 7 == 0 {
        unsafe {
            game.vs()
                .maintain(vehicle, maintenance::decrease_vehicle_value);
        };
    }
    unsafe {
        game.vs()
            .maintain(vehicle, maintenance::check_vehicle_breakdown);
    };
    game.service(id, vehicle);
    (game.leaves.check_orders)(id);
    let ticks = (game.leaves.running)(id);
    if ticks != 0 {
        let cost = unsafe { game.vs().running_cost(vehicle) };
        let cost = cost.saturating_mul(i64::from(ticks)) / i64::from((game.leaves.cost_divisor)());
        (game.leaves.pay_running)(id, cost);
        // SAFETY: The base table names its immutable maintenance table.
        (unsafe { &*game.vs().maintenance }.details_dirty)(vehicle);
        (game.leaves.ships_dirty)();
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_cache(
    v: VehicleRef,
    id: u32,
    leaves: *const Leaves,
    services: *const Services,
) {
    let game = entry_game!(leaves, services, (leaves.state_owner)(id));
    game.cache(id, v);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_destination(
    id: u32,
    tile: u32,
    leaves: *const Leaves,
    services: *const Services,
) {
    let game = entry_game!(leaves, services, (leaves.state_owner)(id));
    game.destination(id, tile);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_find_depot(
    id: u32,
    max_distance: u32,
    leaves: *const Leaves,
    services: *const Services,
) -> DepotResult {
    let game = entry_game!(leaves, services, (leaves.state_owner)(id));
    game.depot(id, max_distance)
        .map_or_else(DepotResult::default, |d| DepotResult {
            tile: d.tile,
            id: d.id as u16,
            valid: true,
        })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_is_destination(
    tile: u32,
    station: u32,
    leaves: *const Leaves,
    services: *const Services,
) -> bool {
    let game = entry_game!(leaves, services, std::ptr::null_mut());
    game.destination_station(tile, station)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_station_destination(
    id: u32,
    station: u32,
    vehicle: *const crate::vehicle::OpenTTDVehicle,
    leaves: *const Leaves,
    services: *const Services,
) -> u32 {
    let game = entry_game!(leaves, services, (leaves.state_owner)(id));
    {
        if station == (game.leaves.last_station)(id) {
            (game.leaves.set_last_station)(id, 65535);
        }
        if unsafe {
            crate::vehicle_func::can_vehicle_use_station(
                vehicle,
                crate::newgrf_airport::station_by_id(
                    station as u16,
                    crate::newgrf_airport::runtime(),
                ),
                crate::newgrf_airport::runtime(),
            )
        } {
            (game.leaves.station_xy)(station)
        } else {
            (game.leaves.order_increment)(id);
            0
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_trackdir(
    id: u32,
    leaves: *const Leaves,
    services: *const Services,
) -> u8 {
    let game = entry_game!(leaves, services, (leaves.state_owner)(id));
    (if (game.leaves.status)(id) & 128 != 0 {
        255
    } else if game.state() == DEPOT {
        let tile = (game.leaves.tile)(id);
        [0_u32, 1, 8, 9][(game.leaves.depot_dir)(tile) as usize]
    } else if game.state() == WORMHOLE {
        [0_u32, 1, 8, 9][((game.leaves.direction)(id) / 2) as usize]
    } else {
        (game.leaves.track_direction)(
            first_track(u32::from(game.state())),
            (game.leaves.direction)(id),
        )
    }) as u8
}
/// Only the Rust vehicle build transaction calls this.
pub(crate) unsafe fn ship_build(
    id: u32,
    v: VehicleRef,
    engine: u32,
    leaves: *const Leaves,
    services: *const Services,
) {
    let game = entry_game!(leaves, services, (leaves.state_owner)(id));
    crate::witness::hit(crate::witness::SHIP_BUILD);
    let tile = (game.leaves.tile)(id);
    let x = (game.leaves.tile_x)(tile).wrapping_mul(16) + 8;
    let y = (game.leaves.tile_y)(tile).wrapping_mul(16) + 8;
    (game.leaves.build_owner)(id);
    (game.leaves.set_x)(id, x as i32);
    (game.leaves.set_y)(id, y as i32);
    (game.leaves.build_z)(id);
    let direction = (game.leaves.depot_dir)(tile) * 2 + 1;
    (game.leaves.set_direction)(id, direction as u8);
    game.set_rotation(direction as u16);
    (game.leaves.update_delta)(id);
    (game.leaves.build_properties)(id, engine);
    let cap = unsafe { (&*(game.vs().newgrf)()).capabilities };
    (game.vs().set_max_age)(v, unsafe {
        crate::engine_capabilities::openttd_rust_engine_life_days(cap, (cap.engine)(engine as u16))
    });
    game.set_state(DEPOT);
    (game.leaves.build_dates)(id);
    (game.leaves.build_random)(id, game.services.random() as u16);
    (game.leaves.build_acceleration)(id, engine);
    (game.leaves.cache)(id);
    if (game.leaves.build_flag)(engine) {
        (game.leaves.build_prototype)(id);
    }
    (game.leaves.build_interval_percent)(id);
    (game.leaves.cache_invalidate)(id);
    // SAFETY: the build transaction retains the new shell across capacity callbacks.
    let capacity = unsafe {
        crate::engine_capabilities::openttd_rust_engine_capacity(
            game.leaves.capabilities,
            (game.vs().engine)(v).0.as_ptr(),
            v.0.as_ptr(),
            std::ptr::null_mut(),
        )
    };
    (game.leaves.build_capacity)(id, capacity);
    (game.leaves.cache_invalidate)(id);
    // SAFETY: The build transaction retains the new shell.
    unsafe {
        crate::vehicle::occupancy::update(game.vs(), v, false);
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDShipPosition" ["probe"] => Position {
        x,
        y,
        old_tile,
        new_tile,
    };
    "OpenTTDShipLeaves" ["probe"] => Leaves {
        depot_dir,
        depot_axis,
        is_depot,
        depot_index,
        wait_unbunch,
        chain_depot,
        servint,
        max_distance,
        tile_valid,
        tile_type,
        water_class,
        lock_middle,
        lock_dir,
        tile_min_z,
        tile_max_z,
        offset,
        diag_between,
        dist_square,
        dist_manhattan,
        docking,
        dock,
        dock_water,
        station,
        industry_station,
        oilrig,
        station_xy,
        station_dock,
        station_visits,
        visit_set,
        arrival,
        leave_unbunch,
        path_result,
        order_free,
        order_dummy,
        order_depot,
        order_leave,
        order_increment,
        timetable,
        start_dirty,
        depot_dirty,
        depot_invalidate,
        ships_dirty,
        check_orders,
        cost_divisor,
        pay_running,
        speed_default,
        age_default,
        speed_frac,
        cache_invalidate,
        capabilities,
        sprite_direction,
        tile_x,
        tile_y,
        build_flag,
        build_random,
        new_position,
        exit_dir,
        track_direction,
        tracks_reach,
        path_size,
        path_back,
        path_pop,
        path_clear,
        viewport,
        base_viewport,
        cache,
        play_sound,
        yapf_reverse,
        yapf_choose,
        update_delta,
        build_owner,
        build_z,
        build_properties,
        build_dates,
        build_acceleration,
        build_prototype,
        build_interval_percent,
        build_capacity,
        set_tile,
        set_x,
        set_y,
        set_z,
        set_direction,
        set_speed,
        set_tick,
        set_running,
        set_day,
        set_order_time,
        set_progress,
        set_last_station,
        set_hidden,
        set_max_speed,
        set_cargo_age,
        set_dest,
        tile,
        dest,
        x,
        y,
        z,
        direction,
        speed,
        tick,
        running,
        day,
        order_time,
        progress,
        status,
        owner,
        last_station,
        order_destination,
        order_type,
        order_max_speed,
        acceleration,
        max_speed,
        state_owner,
        patch,
        neighbours,
        next_depot,
        vehicle,
    };
    "OpenTTDShipReverseResult" ["probe"] => ReverseResult {
        reverse,
        trackdir,
    };
    "OpenTTDShipDepot" ["probe"] => Depot {
        id,
        tile,
        owner,
        ship,
    };
    "OpenTTDShipTrackResult" ["probe"] => TrackResult {
        track,
        found,
    };
    "OpenTTDShipDepotResult" ["probe"] => DepotResult {
        tile,
        id,
        valid,
    };
}

/// `Ship::GetCurrentMaxSpeed`, from `ship.h`.
pub(crate) fn current_max_speed(cached: u16, order: u16) -> i32 {
    i32::from(cached).min(i32::from(order) * 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depot_capacity_is_reused_across_serialized_game_threads() {
        // Spawn/join provides the same ordering as the native game-state mutex.
        let (address, capacity) = std::thread::spawn(|| unsafe {
            let scratch = depot_scratch();
            (*scratch).queue.reserve(64);
            (scratch as usize, (*scratch).queue.capacity())
        })
        .join()
        .unwrap();
        unsafe {
            let scratch = depot_scratch();
            assert_eq!(scratch as usize, address);
            assert_eq!((*scratch).queue.capacity(), capacity);
            *DEPOT_SCRATCH.get() = None;
        }
    }
}
