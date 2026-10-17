/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete train control. Canonical private fields are accessed only for one scalar
//! operation; no Rust reference into an owner or world object crosses a callback.
#![warn(clippy::undocumented_unsafe_blocks)]
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::items_after_statements,
    clippy::if_not_else,
    clippy::collapsible_if,
    clippy::verbose_bit_mask
)]
use crate::services::Services;
use crate::train_state::State;
/// Native identity and canonical scalar owner, resolved together once per entry or link.
/// Neither pointer permits a Rust borrow to survive a callback. Deletion invalidates both.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Handle {
    pub shell: *mut std::ffi::c_void,
    pub owner: *mut State,
}
impl PartialEq for Handle {
    fn eq(&self, other: &Self) -> bool {
        self.shell == other.shell
    }
}
impl Eq for Handle {}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Depot {
    pub tile: u32,
    pub length: u32,
}
impl Handle {
    pub(crate) const NONE: Self = Self {
        shell: std::ptr::null_mut(),
        owner: std::ptr::null_mut(),
    };
}
impl Default for Handle {
    fn default() -> Self {
        Self::NONE
    }
}
impl Handle {
    pub(crate) fn clear_stuck(self) {
        self.set_flags(self.get_flags() & !(1 << 8));
    }
    pub(crate) fn stuck(self) -> bool {
        self.get_flags() & (1 << 8) != 0
    }

    pub(crate) fn track(self) -> u8 {
        self.get_track() as u8
    }
    pub(crate) fn wait_inc(self) -> u16 {
        let next = (self.get_wait_counter() as u16).wrapping_add(1);
        self.set_wait_counter(u64::from(next));
        next
    }

    fn flag(self, bit: u8) -> bool {
        self.get_flags() & (1 << bit) != 0
    }

    pub(crate) fn get_flags(self) -> u64 {
        // SAFETY: A live serialized owner, scalar access ends before any callback.
        unsafe { u64::from((*self.owner).flags) }
    }
    pub(crate) fn set_flags(self, value: u64) {
        // SAFETY: Exclusive serialized scalar write, retaining no owner borrow.
        unsafe {
            (*self.owner).flags = value as u16;
        }
    }
    pub(crate) fn get_crash_anim_pos(self) -> u64 {
        // SAFETY: A live serialized owner, scalar access ends before any callback.
        unsafe { u64::from((*self.owner).crash_anim_pos) }
    }
    pub(crate) fn set_crash_anim_pos(self, value: u64) {
        // SAFETY: Exclusive serialized scalar write, retaining no owner borrow.
        unsafe {
            (*self.owner).crash_anim_pos = value as u16;
        }
    }
    pub(crate) fn get_wait_counter(self) -> u64 {
        // SAFETY: A live serialized owner, scalar access ends before any callback.
        unsafe { u64::from((*self.owner).wait_counter) }
    }
    pub(crate) fn set_wait_counter(self, value: u64) {
        // SAFETY: Exclusive serialized scalar write, retaining no owner borrow.
        unsafe {
            (*self.owner).wait_counter = value as u16;
        }
    }
    pub(crate) fn get_compatible_railtypes(self) -> u64 {
        // SAFETY: A live serialized owner, scalar access ends before any callback.
        unsafe { (*self.owner).compatible_railtypes }
    }
    pub(crate) fn set_compatible_railtypes(self, value: u64) {
        // SAFETY: Exclusive serialized scalar write, retaining no owner borrow.
        unsafe {
            (*self.owner).compatible_railtypes = value;
        }
    }
    pub(crate) fn get_railtypes(self) -> u64 {
        // SAFETY: A live serialized owner, scalar access ends before any callback.
        unsafe { (*self.owner).railtypes }
    }
    pub(crate) fn set_railtypes(self, value: u64) {
        // SAFETY: Exclusive serialized scalar write, retaining no owner borrow.
        unsafe {
            (*self.owner).railtypes = value;
        }
    }
    pub(crate) fn get_track(self) -> u64 {
        // SAFETY: A live serialized owner, scalar access ends before any callback.
        unsafe { u64::from((*self.owner).track) }
    }
    pub(crate) fn set_track(self, value: u64) {
        // SAFETY: Exclusive serialized scalar write, retaining no owner borrow.
        unsafe {
            (*self.owner).track = value as u8;
        }
    }
    pub(crate) fn get_force_proceed(self) -> u64 {
        // SAFETY: A live serialized owner, scalar access ends before any callback.
        unsafe { u64::from((*self.owner).force_proceed) }
    }
    pub(crate) fn set_force_proceed(self, value: u64) {
        // SAFETY: Exclusive serialized scalar write, retaining no owner borrow.
        unsafe {
            (*self.owner).force_proceed = value as u8;
        }
    }
    pub(crate) fn get_cached_tilt(self) -> u64 {
        // SAFETY: A live serialized owner, scalar access ends before any callback.
        unsafe { u64::from((*self.owner).cached_tilt) }
    }
    pub(crate) fn set_cached_tilt(self, value: u64) {
        // SAFETY: Exclusive serialized scalar write, retaining no owner borrow.
        unsafe {
            (*self.owner).cached_tilt = value != 0;
        }
    }
    pub(crate) fn get_user_def_data(self) -> u64 {
        // SAFETY: A live serialized owner, scalar access ends before any callback.
        unsafe { u64::from((*self.owner).user_def_data) }
    }
    pub(crate) fn set_user_def_data(self, value: u64) {
        // SAFETY: Exclusive serialized scalar write, retaining no owner borrow.
        unsafe {
            (*self.owner).user_def_data = value as u8;
        }
    }
    pub(crate) fn get_cached_curve_speed_mod(self) -> u64 {
        // SAFETY: A live serialized owner, scalar access ends before any callback.
        unsafe { u64::from((*self.owner).cached_curve_speed_mod as u16) }
    }
    pub(crate) fn set_cached_curve_speed_mod(self, value: u64) {
        // SAFETY: Exclusive serialized scalar write, retaining no owner borrow.
        unsafe {
            (*self.owner).cached_curve_speed_mod = value as i16;
        }
    }
    pub(crate) fn get_cached_max_curve_speed(self) -> u64 {
        // SAFETY: A live serialized owner, scalar access ends before any callback.
        unsafe { u64::from((*self.owner).cached_max_curve_speed) }
    }
    pub(crate) fn set_cached_max_curve_speed(self, value: u64) {
        // SAFETY: Exclusive serialized scalar write, retaining no owner borrow.
        unsafe {
            (*self.owner).cached_max_curve_speed = value as u16;
        }
    }
    pub(crate) fn set_flag(self, bit: u8, value: bool) {
        let mask = 1 << bit;
        let old = self.get_flags();
        self.set_flags(if value { old | mask } else { old & !mask });
    }
}
const INVALID: u32 = u32::MAX;
const DEPOT: u8 = 0x80;
const WORMHOLE: u8 = 0x40;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainConsistChangedRead {
    pub engine: u16,
    pub front: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainConsistChanged1Read {
    pub engine: u16,
    pub engine_part: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainConsistChanged2Read {
    pub cargo_cap: u16,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainCurveLimitRead {
    pub next: Handle,
    pub direction: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainStopLocationRead {
    pub length: u16,
    pub total_length: u16,
    pub order_destination: u16,
    pub order: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainCurrentMaxSpeedRead {
    pub tile: u32,
    pub max_track_speed: u16,
    pub speed: u16,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainCurrentMaxSpeed6Read {
    pub next: Handle,
    pub tile: u32,
    pub status: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainUpdateAccelerationRead {
    pub power: u32,
    pub weight: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainUpdateSpeedRead {
    pub status: u8,
    pub acceleration: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainTrackdirRead {
    pub tile: u32,
    pub direction: u8,
    pub status: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainCanLeaveRead {
    pub tile: u32,
    pub direction: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainCrossingApproachRead {
    pub status: u8,
    pub front: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainNextOffsetRead {
    pub next: Handle,
    pub length: u16,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainAfterSwapRead {
    pub tile: u32,
    pub x: i32,
    pub y: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainReverseSwapRead {
    pub tile: u32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub direction: u8,
    pub status: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainApproachingEndRead {
    pub x: i32,
    pub y: i32,
    pub length: u16,
    pub speed: u16,
    pub direction: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainLineEndsRead {
    pub tile: u32,
    pub speed: u16,
    pub breakdown: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainSpeedZRead {
    pub z: i32,
    pub max_track_speed: u16,
    pub speed: u16,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainMoveVehicleRead {
    pub tile: u32,
    pub x: i32,
    pub y: i32,
    pub direction: u8,
    pub front: u8,
    pub articulated: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainMoveVehicle20Read {
    pub speed: u16,
    pub status: u8,
    pub front: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainCollisionOneRead {
    pub first: Handle,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub length: u16,
    pub owner: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainCollisionOne22Read {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub length: u16,
    pub owner: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainDeleteLastRead {
    pub tile: u32,
    pub owner: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainStayDepotRead {
    pub tile: u32,
    pub power: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainLocoRead {
    pub speed: u16,
    pub status: u8,
    pub order: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainLoco26Read {
    pub tile: u32,
    pub order_destination: u16,
    pub order: u8,
    pub nonstop: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainTickRead {
    pub speed: u16,
    pub status: u8,
    pub running: u8,
    pub front: u8,
    pub free_wagon: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainNeedsServiceRead {
    pub order_destination: u16,
    pub order: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainNextForceRead {
    pub tile: u32,
    pub status: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainReverseCommandRead {
    pub status: u8,
    pub breakdown: u8,
    pub front: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Leaves {
    pub read_first: extern "C" fn(Handle) -> Handle,
    pub read_next: extern "C" fn(Handle) -> Handle,
    pub read_previous: extern "C" fn(Handle) -> Handle,
    pub read_next_unit: extern "C" fn(Handle) -> Handle,
    pub read_last: extern "C" fn(Handle) -> Handle,
    pub read_tile: extern "C" fn(Handle) -> u32,
    pub read_dest: extern "C" fn(Handle) -> u32,
    pub read_order_time: extern "C" fn(Handle) -> i32,
    pub read_length: extern "C" fn(Handle) -> u16,
    pub read_total_length: extern "C" fn(Handle) -> u16,
    pub read_max_track_speed: extern "C" fn(Handle) -> u16,
    pub read_speed: extern "C" fn(Handle) -> u16,
    pub read_gv_flags: extern "C" fn(Handle) -> u16,
    pub read_refit_cap: extern "C" fn(Handle) -> u16,
    pub read_last_station: extern "C" fn(Handle) -> u16,
    pub read_direction: extern "C" fn(Handle) -> u8,
    pub read_status: extern "C" fn(Handle) -> u8,
    pub read_tick: extern "C" fn(Handle) -> u8,
    pub read_running: extern "C" fn(Handle) -> u8,
    pub read_day: extern "C" fn(Handle) -> u8,
    pub read_progress: extern "C" fn(Handle) -> u8,
    pub read_order: extern "C" fn(Handle) -> u8,
    pub read_front: extern "C" fn(Handle) -> u8,
    pub read_articulated: extern "C" fn(Handle) -> u8,
    pub read_multiheaded: extern "C" fn(Handle) -> u8,
    pub read_owner: extern "C" fn(Handle) -> u8,
    pub read_vis_effect: extern "C" fn(Handle) -> u8,
    pub read_consist_changed: extern "C" fn(Handle) -> TrainConsistChangedRead,
    pub read_consist_changed_1: extern "C" fn(Handle) -> TrainConsistChanged1Read,
    pub read_consist_changed_2: extern "C" fn(Handle) -> TrainConsistChanged2Read,
    pub read_curve_limit: extern "C" fn(Handle) -> TrainCurveLimitRead,
    pub read_stop_location: extern "C" fn(Handle) -> TrainStopLocationRead,
    pub read_current_max_speed: extern "C" fn(Handle) -> TrainCurrentMaxSpeedRead,
    pub read_current_max_speed_6: extern "C" fn(Handle) -> TrainCurrentMaxSpeed6Read,
    pub read_update_acceleration: extern "C" fn(Handle) -> TrainUpdateAccelerationRead,
    pub read_update_speed: extern "C" fn(Handle) -> TrainUpdateSpeedRead,
    pub read_trackdir: extern "C" fn(Handle) -> TrainTrackdirRead,
    pub read_can_leave: extern "C" fn(Handle) -> TrainCanLeaveRead,
    pub read_crossing_approach: extern "C" fn(Handle) -> TrainCrossingApproachRead,
    pub read_next_offset: extern "C" fn(Handle) -> TrainNextOffsetRead,
    pub read_after_swap: extern "C" fn(Handle) -> TrainAfterSwapRead,
    pub read_reverse_swap: extern "C" fn(Handle) -> TrainReverseSwapRead,
    pub read_approaching_end: extern "C" fn(Handle) -> TrainApproachingEndRead,
    pub read_line_ends: extern "C" fn(Handle) -> TrainLineEndsRead,
    pub read_speed_z: extern "C" fn(Handle) -> TrainSpeedZRead,
    pub read_move_vehicle: extern "C" fn(Handle) -> TrainMoveVehicleRead,
    pub read_move_vehicle_20: extern "C" fn(Handle) -> TrainMoveVehicle20Read,
    pub read_collision_one: extern "C" fn(Handle) -> TrainCollisionOneRead,
    pub read_collision_one_22: extern "C" fn(Handle) -> TrainCollisionOne22Read,
    pub read_delete_last: extern "C" fn(Handle) -> TrainDeleteLastRead,
    pub read_stay_depot: extern "C" fn(Handle) -> TrainStayDepotRead,
    pub read_loco: extern "C" fn(Handle) -> TrainLocoRead,
    pub read_loco_26: extern "C" fn(Handle) -> TrainLoco26Read,
    pub read_tick_state: extern "C" fn(Handle) -> TrainTickRead,
    pub read_needs_service: extern "C" fn(Handle) -> TrainNeedsServiceRead,
    pub read_next_force: extern "C" fn(Handle) -> TrainNextForceRead,
    pub read_reverse_command: extern "C" fn(Handle) -> TrainReverseCommandRead,

    pub write_tile: extern "C" fn(Handle, u32),
    pub write_dest: extern "C" fn(Handle, u32),
    pub write_x: extern "C" fn(Handle, i32),
    pub write_y: extern "C" fn(Handle, i32),
    pub write_z: extern "C" fn(Handle, i32),
    pub write_direction: extern "C" fn(Handle, u8),
    pub write_speed: extern "C" fn(Handle, u16),
    pub write_tick: extern "C" fn(Handle, u8),
    pub write_running: extern "C" fn(Handle, u8),
    pub write_day: extern "C" fn(Handle, u8),
    pub write_order_time: extern "C" fn(Handle, i32),
    pub write_progress: extern "C" fn(Handle, u8),
    pub write_subspeed: extern "C" fn(Handle, u8),
    pub write_gv_flags: extern "C" fn(Handle, u16),
    pub write_acceleration: extern "C" fn(Handle, u8),
    pub write_length: extern "C" fn(Handle, u8),
    pub write_total_length: extern "C" fn(Handle, u16),
    pub write_first_engine: extern "C" fn(Handle, u16),
    pub write_max_speed: extern "C" fn(Handle, u16),
    pub write_cargo_cap: extern "C" fn(Handle, u16),
    pub write_refit_cap: extern "C" fn(Handle, u16),
    pub write_cargo_age: extern "C" fn(Handle, u16),
    pub write_last_station: extern "C" fn(Handle, u16),
    pub write_colourmap: extern "C" fn(Handle, u64),
    pub write_status: extern "C" fn(Handle, u8),

    pub acceleration: extern "C" fn(Handle) -> u64,
    pub acc_model: extern "C" fn(Handle) -> u64,
    pub acc_type: extern "C" fn(Handle) -> u64,
    pub advance_distance: extern "C" fn(Handle) -> u64,
    pub age: extern "C" fn(Handle),
    pub all_powered: extern "C" fn(Handle, u64) -> u64,
    pub ambient_sound: extern "C" fn(Handle) -> u64,
    pub arrival_news: extern "C" fn(Handle, u16),
    pub arrival_triggers: extern "C" fn(Handle, u16),
    pub axis_diag: extern "C" fn(Handle, u8) -> u64,
    pub backoff: extern "C" fn(Handle) -> u64,
    pub base_viewport: extern "C" fn(Handle, u8),
    pub begin_loading: extern "C" fn(Handle),
    pub bridge_speed: extern "C" fn(Handle, u32) -> u64,
    pub cache_override: extern "C" fn(Handle),
    pub callback_length: extern "C" fn(Handle) -> u64,
    pub capacity: extern "C" fn(Handle) -> u64,
    pub capacity_error: extern "C" fn(Handle),
    pub cargo_age_default: extern "C" fn(Handle) -> u64,
    pub cargo_changed: extern "C" fn(Handle),
    pub chain_depot: extern "C" fn(Handle) -> u64,
    pub check_breakdown: extern "C" fn(Handle),
    pub check_next: extern "C" fn(Handle),
    pub check_orders: extern "C" fn(Handle),
    pub check_reverse: extern "C" fn(Handle) -> u64,
    pub choose_track: extern "C" fn(Handle, u32, u8, u8) -> u64,
    pub clear_reservation: extern "C" fn(Handle, u32, u8),
    pub compatible_rail_owner: extern "C" fn(Handle, u32) -> u64,
    pub consist_windows: extern "C" fn(Handle),
    pub cost_class: extern "C" fn(Handle) -> u64,
    pub cost_default: extern "C" fn(Handle) -> u64,
    pub cost_divisor: extern "C" fn(Handle) -> u64,
    pub count_chain: extern "C" fn(Handle) -> u64,
    pub crash_event: extern "C" fn(Handle, u32),
    pub crash_ground: extern "C" fn(Handle, u8) -> u64,
    pub crash_news: extern "C" fn(Handle, u32),
    pub crash_rating: extern "C" fn(Handle),
    pub crash_sound: extern "C" fn(Handle),
    pub crossing_barred: extern "C" fn(Handle, u32) -> u64,
    pub crossing_rail_axis: extern "C" fn(Handle, u32) -> u64,
    pub crossing_reserved: extern "C" fn(Handle, u32) -> u64,
    pub crossing_road_axis: extern "C" fn(Handle, u32) -> u64,
    pub crossing_sound: extern "C" fn(Handle, u32),
    pub curve_advantage: extern "C" fn(Handle) -> u64,
    pub curve_mod: extern "C" fn(Handle) -> u64,
    pub day_ticks: extern "C" fn(Handle) -> u64,
    pub decrease_value: extern "C" fn(Handle),
    pub delete_vehicle: extern "C" fn(Handle),
    pub depot_dir: extern "C" fn(Handle, u32) -> u64,
    pub depot_dirty: extern "C" fn(Handle),
    pub depot_index: extern "C" fn(Handle, u32) -> u64,
    pub depot_track: extern "C" fn(Handle, u32) -> u64,
    pub depot_window: extern "C" fn(Handle, u32),
    pub diag_axis: extern "C" fn(Handle, u8) -> u64,
    pub diag_between: extern "C" fn(Handle, u32, u32) -> u64,
    pub diag_reaches_tracks: extern "C" fn(Handle, u8) -> u64,
    pub diag_trackdir: extern "C" fn(Handle, u8) -> u64,
    pub dirty_tile: extern "C" fn(Handle, u32),
    pub dir_diag: extern "C" fn(Handle, u8) -> u64,
    pub disaster_sound: extern "C" fn(Handle) -> u64,
    pub disconnect: extern "C" fn(Handle),
    pub economy_age: extern "C" fn(Handle),
    pub engine_power: extern "C" fn(Handle) -> u64,
    pub enter_depot: extern "C" fn(Handle),
    pub enter_tile: extern "C" fn(Handle, u32, i32, i32) -> u64,
    pub find_depot: extern "C" fn(Handle, i32) -> Depot,
    pub first_track: extern "C" fn(Handle, u8) -> u64,
    pub grf_version: extern "C" fn(Handle) -> u64,
    pub handle_breakdown: extern "C" fn(Handle) -> u64,
    pub has_depot_res: extern "C" fn(Handle, u32) -> u64,
    pub has_reserved: extern "C" fn(Handle, u32, u8) -> u64,
    pub has_signal: extern "C" fn(Handle, u32, u8) -> u64,
    pub has_signals: extern "C" fn(Handle, u32) -> u64,
    pub has_signal_td: extern "C" fn(Handle, u32, u8) -> u64,
    pub hide_fill: extern "C" fn(Handle),
    pub inclination: extern "C" fn(Handle, u8, u8) -> u64,
    pub invalidate_grf: extern "C" fn(Handle),
    pub invalid_price: extern "C" fn(Handle) -> u64,
    pub is_bridge: extern "C" fn(Handle, u32) -> u64,
    pub is_crossing: extern "C" fn(Handle, u32) -> u64,
    pub is_depot: extern "C" fn(Handle, u32) -> u64,
    pub is_plain_rail: extern "C" fn(Handle, u32) -> u64,
    pub is_railway: extern "C" fn(Handle, u32) -> u64,
    pub is_station: extern "C" fn(Handle, u32) -> u64,
    pub is_station_any: extern "C" fn(Handle, u32) -> u64,
    pub is_tunnelbridge: extern "C" fn(Handle, u32) -> u64,
    pub large_explosion: extern "C" fn(Handle),
    pub last_speed: extern "C" fn(Handle),
    pub leave_sound: extern "C" fn(Handle),
    pub leave_station: extern "C" fn(Handle),
    pub leave_unbunch: extern "C" fn(Handle),
    pub length_callback: extern "C" fn(Handle) -> u64,
    pub length_changed: extern "C" fn(Handle),
    pub length_default: extern "C" fn(Handle) -> u64,
    pub length_error: extern "C" fn(Handle, u16),
    pub loading: extern "C" fn(Handle, u8),
    pub local_company: extern "C" fn(Handle) -> u64,
    pub lost_warn: extern "C" fn(Handle) -> u64,
    pub map_size: extern "C" fn(Handle) -> u64,
    pub max_depot_penalty: extern "C" fn(Handle) -> u64,
    pub needs_service: extern "C" fn(Handle) -> u64,
    pub no_90: extern "C" fn(Handle, u8, u8) -> u64,
    pub oneway_blocking: extern "C" fn(Handle, u32, u8) -> u64,
    pub order_depot_service: extern "C" fn(Handle, u16),
    pub order_dummy: extern "C" fn(Handle),
    pub order_free: extern "C" fn(Handle),
    pub order_max_speed: extern "C" fn(Handle) -> u64,
    pub order_stop: extern "C" fn(Handle, u16) -> u64,
    pub other_end: extern "C" fn(Handle, u32) -> u64,
    pub pay_running: extern "C" fn(Handle, i64),
    pub pbs_signal_type: extern "C" fn(Handle) -> u64,
    pub platform_ahead: extern "C" fn(Handle, u16, u32) -> u64,
    pub platform_length: extern "C" fn(Handle, u16, u32) -> u64,
    pub position: extern "C" fn(Handle),
    pub pow_wag_power: extern "C" fn(Handle) -> u64,
    pub price: extern "C" fn(Handle, u32) -> u64,
    pub process_orders: extern "C" fn(Handle) -> u64,
    pub property: extern "C" fn(Handle, u8, u32) -> u64,
    pub railveh_wagon: extern "C" fn(Handle) -> u64,
    pub rail_tilt: extern "C" fn(Handle) -> u64,
    pub rail_type: extern "C" fn(Handle, u32) -> u64,
    pub rail_types: extern "C" fn(Handle) -> u64,
    pub reserve_paths: extern "C" fn(Handle) -> u64,
    pub reserve_under: extern "C" fn(Handle),
    pub reset_unbunch: extern "C" fn(Handle),
    pub reverse_at_signals: extern "C" fn(Handle) -> u64,
    pub reverse_single_blocked: extern "C" fn(Handle) -> u64,
    pub reverse_windows: extern "C" fn(Handle),
    pub running_windows: extern "C" fn(Handle),
    pub service: extern "C" fn(Handle),
    pub servint: extern "C" fn(Handle) -> u64,
    pub set_depot_res: extern "C" fn(Handle, u32, u8),
    pub set_next: extern "C" fn(Handle, Handle),
    pub set_platform_res: extern "C" fn(Handle, u32, u8, u8),
    pub set_signal_state: extern "C" fn(Handle, u32, u8, u8),
    pub set_tunnel_res: extern "C" fn(Handle, u32, u8),
    pub show_effect: extern "C" fn(Handle),
    pub show_reservation: extern "C" fn(Handle) -> u64,
    pub signal_context: extern "C" fn() -> crate::signal::Context,
    pub signal_has_pbs: extern "C" fn(Handle, u32, u8) -> u64,
    pub signal_pbs: extern "C" fn(Handle, u8) -> u64,
    pub signal_type: extern "C" fn(Handle, u32, u8) -> u64,
    pub sigseg_full: extern "C" fn(Handle) -> u64,
    pub sigseg_pbs: extern "C" fn(Handle) -> u64,
    pub small_explosion: extern "C" fn(Handle, i32, i32, i32),
    pub speed_default: extern "C" fn(Handle) -> u64,
    pub start_stop_dirty: extern "C" fn(Handle),
    pub station: extern "C" fn(Handle, u32) -> u64,
    pub station_axis: extern "C" fn(Handle, u32) -> u64,
    pub station_compatible: extern "C" fn(Handle, u32, u32) -> u64,
    pub station_dest: extern "C" fn(Handle) -> u64,
    pub stopped_in_depot: extern "C" fn(Handle) -> u64,
    pub stop_location: extern "C" fn(Handle) -> u64,
    pub stuck_news: extern "C" fn(Handle),
    pub suppress_implicit: extern "C" fn(Handle),
    pub ticks_leave_depot: extern "C" fn(Handle) -> u64,
    pub tile_add_diag: extern "C" fn(Handle, u32, u8) -> u64,
    pub tile_offset_axis: extern "C" fn(Handle, u8) -> u64,
    pub tile_offset_diag: extern "C" fn(Handle, u8) -> u64,
    pub tile_owner: extern "C" fn(Handle, u32) -> u64,
    pub tile_rail_type: extern "C" fn(Handle, u32) -> u64,
    pub tile_virt: extern "C" fn(Handle, i32, i32) -> u64,
    pub trackdir_exit: extern "C" fn(Handle, u8) -> u64,
    pub trackdir_reaches: extern "C" fn(Handle, u8) -> u64,
    pub track_bits: extern "C" fn(Handle, u32) -> u64,
    pub track_crosses: extern "C" fn(Handle, u8) -> u64,
    pub track_direction: extern "C" fn(Handle, u8, u8) -> u64,
    pub track_status: extern "C" fn(Handle, u32, u8) -> u64,
    pub train_list: extern "C" fn(Handle),
    pub train_visit: extern "C" fn(Handle) -> u64,
    pub truncate_cargo: extern "C" fn(Handle, u32),
    pub try_reserve: extern "C" fn(Handle, u32, u8, u8) -> u64,
    pub tunnel_dir: extern "C" fn(Handle, u32) -> u64,
    pub unreserve: extern "C" fn(Handle, u32, u8),
    pub update_delta: extern "C" fn(Handle),
    pub update_speed: extern "C" fn(Handle, u32, i32, i32) -> u64,
    pub user_default: extern "C" fn(Handle) -> u64,
    pub veh_exit_dir: extern "C" fn(Handle, u8, u8) -> u64,
    pub viewport: extern "C" fn(Handle, u8, u8),
    pub view_window: extern "C" fn(Handle),
    pub visit_type: extern "C" fn(Handle, u16) -> u64,
    pub vis_effect: extern "C" fn(Handle, u8),
    pub wagon_override: extern "C" fn(Handle) -> u64,
    pub wagon_speed_limits: extern "C" fn(Handle) -> u64,
    pub wait_oneway: extern "C" fn(Handle) -> u64,
    pub wait_pbs: extern "C" fn(Handle) -> u64,
    pub wait_twoway: extern "C" fn(Handle) -> u64,
    pub wait_unbunch: extern "C" fn(Handle) -> u64,
    pub write_crossing_bar: extern "C" fn(Handle, u32, u8),
    pub write_crossing_res: extern "C" fn(Handle, u32, u8),
    pub write_visit_type: extern "C" fn(Handle, u16),

    pub visit_tile: extern "C" fn(
        u32,
        *mut std::ffi::c_void,
        extern "C" fn(*mut std::ffi::c_void, Handle) -> u8,
    ) -> u8,
    pub visit_near: extern "C" fn(
        i32,
        i32,
        *mut std::ffi::c_void,
        extern "C" fn(*mut std::ffi::c_void, Handle) -> u8,
    ) -> u8,
}
#[derive(Clone)]
struct Game<'a> {
    leaves: &'a Leaves,
    services: &'a Services,
}
extern "C" fn visit<F: FnMut(Handle) -> bool>(context: *mut std::ffi::c_void, part: Handle) -> u8 {
    // SAFETY: Native visitors call synchronously with this live stack closure; no borrow of world state is held.
    u8::from(unsafe { (&mut *context.cast::<F>())(part) })
}
impl Game<'_> {
    fn visit_tile<F: FnMut(Handle) -> bool>(&self, tile: u32, mut visitor: F) -> bool {
        (self.leaves.visit_tile)(tile, std::ptr::from_mut(&mut visitor).cast(), visit::<F>) != 0
    }
    fn visit_near<F: FnMut(Handle) -> bool>(&self, x: i32, y: i32, mut visitor: F) -> bool {
        (self.leaves.visit_near)(x, y, std::ptr::from_mut(&mut visitor).cast(), visit::<F>) != 0
    }

    fn signal_context(&self) -> crate::signal::Context {
        crate::signal::acquire_context(self.leaves.signal_context)
    }
    fn signal_segment(&self, id: Handle, tile: u32, side: u8) -> u64 {
        let owner = (self.leaves.read_owner)(id);
        crate::signal::count_context_owner_read();
        u64::from(self.signal_context().segment(tile, side, owner))
    }
}
const PROP_TRAIN_USER_DATA: u32 = 37;
const PROP_TRAIN_SPEED: u32 = 9;
const PROP_TRAIN_CARGO_AGE_PERIOD: u32 = 43;
const PROP_TRAIN_SHORTEN_FACTOR: u32 = 33;
const PROP_TRAIN_RUNNING_COST_FACTOR: u32 = 13;
impl Game<'_> {
    fn property(&self, id: Handle, prop: u32, default: u64) -> u64 {
        (self.leaves.property)(id, prop as u8, default as u32)
    }
    fn consist_changed(&self, id: Handle, allowed: u8) {
        let mut max_speed = u16::MAX;
        let head = (self.leaves.read_consist_changed)(id);
        let mut first_engine = if head.front != 0 {
            head.engine
        } else {
            u16::MAX
        };
        (self.leaves.write_total_length)(id, 0_u16);
        id.set_compatible_railtypes(0);
        let mut tilt = true;
        let mut min_curve_mod = i16::MAX;
        let mut u = id;
        while u != Handle::NONE {
            let part = (self.leaves.read_consist_changed_1)(u);
            (self.leaves.write_first_engine)(
                u,
                (u64::from(if u == id { u16::MAX } else { first_engine })) as u16,
            );
            u.set_railtypes((self.leaves.rail_types)(u));
            if part.engine_part != 0 {
                first_engine = part.engine;
            }
            u.set_user_def_data((self.leaves.user_default)(u));
            (self.leaves.invalidate_grf)(id);
            (self.leaves.invalidate_grf)(u);
            u = (self.leaves.read_next)(u);
        }
        u = id;
        while u != Handle::NONE {
            u.set_user_def_data(self.property(u, PROP_TRAIN_USER_DATA, u.get_user_def_data()));
            (self.leaves.invalidate_grf)(id);
            (self.leaves.invalidate_grf)(u);
            u = (self.leaves.read_next)(u);
        }
        u = id;
        while u != Handle::NONE {
            if (self.leaves.rail_tilt)(u) == 0 {
                tilt = false;
            }
            min_curve_mod = min_curve_mod.min((self.leaves.curve_mod)(u) as i16);
            (self.leaves.cache_override)(u);
            (self.leaves.write_colourmap)(u, 0);
            (self.leaves.vis_effect)(u, 1);
            let powered = (self.leaves.pow_wag_power)(id) != 0
                && (self.leaves.railveh_wagon)(u) != 0
                && (self.leaves.wagon_override)(u) != 0
                && (self.leaves.read_vis_effect)(u) & 0x80 == 0;
            u.set_flag(3, powered);
            if (self.leaves.read_articulated)(u) == 0 {
                if (self.leaves.engine_power)(u) > 0 {
                    id.set_compatible_railtypes(
                        id.get_compatible_railtypes()
                            | (self.leaves.all_powered)(u, u.get_railtypes()),
                    );
                }
                if u.flag(6) {
                    u.set_railtypes(u.get_railtypes() | 1);
                    u.set_compatible_railtypes(u.get_compatible_railtypes() | 1);
                }
                if ((self.leaves.railveh_wagon)(u) == 0 || (self.leaves.wagon_speed_limits)(u) != 0)
                    && (self.leaves.wagon_override)(u) == 0
                {
                    let speed =
                        self.property(u, PROP_TRAIN_SPEED, (self.leaves.speed_default)(u)) as u16;
                    if speed != 0 {
                        max_speed = max_speed.min(speed);
                    }
                }
            }
            let cap = (self.leaves.capacity)(u) as u16;
            let part = (self.leaves.read_consist_changed_2)(u);
            if allowed & 2 != 0 {
                if part.cargo_cap > cap {
                    (self.leaves.truncate_cargo)(u, u32::from(cap));
                }
                (self.leaves.write_refit_cap)(
                    u,
                    (u64::from(cap.min((self.leaves.read_refit_cap)(u)))) as u16,
                );
                (self.leaves.write_cargo_cap)(u, (u64::from(cap)) as u16);
            } else if cap != part.cargo_cap {
                (self.leaves.capacity_error)(u);
            }
            (self.leaves.write_cargo_age)(
                u,
                (self.property(
                    u,
                    PROP_TRAIN_CARGO_AGE_PERIOD,
                    (self.leaves.cargo_age_default)(u),
                )) as u16,
            );
            let mut length = u16::MAX;
            if (self.leaves.grf_version)(u) >= 8 {
                length = self.property(u, PROP_TRAIN_SHORTEN_FACTOR, u64::from(u16::MAX)) as u16;
                if length != u16::MAX && length >= 8 {
                    (self.leaves.length_error)(u, length);
                }
            } else if (self.leaves.length_callback)(u) != 0 {
                length = (self.leaves.callback_length)(u) as u16;
            }
            if length == u16::MAX {
                length = (self.leaves.length_default)(u) as u16;
            }
            length = 8 - length.min(7);
            if allowed & 1 != 0 {
                (self.leaves.write_length)(u, (u64::from(length)) as u8);
            } else if length != (self.leaves.read_length)(u) {
                (self.leaves.length_changed)(u);
            }
            (self.leaves.write_total_length)(
                id,
                (u64::from(
                    (self.leaves.read_total_length)(id).wrapping_add((self.leaves.read_length)(u)),
                )) as u16,
            );
            (self.leaves.invalidate_grf)(id);
            (self.leaves.invalidate_grf)(u);
            u = (self.leaves.read_next)(u);
        }
        (self.leaves.write_max_speed)(id, (u64::from(max_speed)) as u16);
        id.set_cached_tilt(u64::from(tilt));
        id.set_cached_curve_speed_mod(u64::from(min_curve_mod as u16));
        id.set_cached_max_curve_speed(u64::from(self.curve_limit(id)));
        (self.leaves.cargo_changed)(id);
        if (self.leaves.read_front)(id) != 0 {
            self.update_acceleration(id);
            (self.leaves.consist_windows)(id);
        }
    }
    fn curve_limit(&self, id: Handle) -> u16 {
        let mut max_speed = i32::from(u16::MAX);
        if (self.leaves.acc_model)(id) == 0 {
            return max_speed as u16;
        }
        let mut curves = [0_i32; 2];
        let (mut numcurve, mut sum, mut pos, mut lastpos) = (0_i32, 0_i32, 0_i32, -1_i32);
        let mut u = id;
        loop {
            let part = (self.leaves.read_curve_limit)(u);
            if part.next == Handle::NONE {
                break;
            }
            let diff = part
                .direction
                .wrapping_sub((self.leaves.read_direction)(part.next))
                & 7;
            if diff != 0 {
                if diff == 7 {
                    curves[0] = curves[0].wrapping_add(1);
                }
                if diff == 1 {
                    curves[1] = curves[1].wrapping_add(1);
                }
                if diff == 7 || diff == 1 {
                    if lastpos != -1 {
                        numcurve = numcurve.wrapping_add(1);
                        sum = sum.wrapping_add(pos.wrapping_sub(lastpos));
                        if pos.wrapping_sub(lastpos) <= 8 && max_speed > 88 {
                            max_speed = 88;
                        }
                    }
                    lastpos = pos;
                }
                if diff == 6 || diff == 2 {
                    max_speed = 61;
                }
            }
            u = part.next;
            pos = pos.wrapping_add(i32::from((self.leaves.read_length)(u)));
        }
        if numcurve > 0 && max_speed > 88 {
            if curves == [1, 1] {
                max_speed = i32::from(u16::MAX);
            } else {
                sum = sum.wrapping_add(7) / 8;
                sum /= numcurve;
                let d = 13 - sum.clamp(1, 12);
                max_speed = 232 - d * d;
            }
        }
        if max_speed != i32::from(u16::MAX) {
            max_speed = max_speed.wrapping_add(
                (max_speed / 2).wrapping_mul((self.leaves.curve_advantage)(id) as i32),
            );
            if id.get_cached_tilt() != 0 {
                max_speed = max_speed.wrapping_add(max_speed / 5);
            }
            max_speed = max_speed.wrapping_add(
                max_speed.wrapping_mul(i32::from(id.get_cached_curve_speed_mod() as i16)) / 256,
            );
            max_speed = max_speed.clamp(2, i32::from(u16::MAX));
        }
        max_speed as u16
    }
    fn stop_location(&self, id: Handle, station: u16, tile: u32) -> (i32, i32, i32) {
        let ahead = ((self.leaves.platform_ahead)(id, station, tile) as i32).wrapping_mul(16);
        let length = ((self.leaves.platform_length)(id, station, tile) as i32).wrapping_mul(16);
        let v = (self.leaves.read_stop_location)(id);
        let mut loc = 1;
        if i32::from(v.total_length) >= length {
            loc = 2;
        } else if v.order == 1 && v.order_destination == station {
            loc = (self.leaves.stop_location)(id);
        }
        let stop = match loc {
            0 => i32::from(v.total_length),
            1 => length.wrapping_sub(length.wrapping_sub(i32::from(v.total_length)) / 2),
            _ => length,
        };
        (
            stop.wrapping_sub((i32::from(v.length) + 1) / 2),
            ahead,
            length,
        )
    }
    fn current_max_speed(&self, id: Handle) -> i32 {
        let v = (self.leaves.read_current_max_speed)(id);
        let model = (self.leaves.acc_model)(id);
        let mut max_speed = if model == 0 {
            i32::from(v.max_track_speed)
        } else {
            id.get_cached_max_curve_speed() as i32
        };
        if model == 1 && (self.leaves.is_station)(id, v.tile) != 0 {
            let station = (self.leaves.station)(id, v.tile) as u16;
            if (self.leaves.order_stop)(id, station) != 0 {
                let (stop, ahead, length) = self.stop_location(id, station, v.tile);
                let distance = ahead / 16 - length.wrapping_sub(stop) / 16;
                if distance > 0 {
                    let mut limit = 120;
                    let delta = i32::from(v.speed) / distance.wrapping_add(1);
                    if max_speed > i32::from(v.speed) - delta {
                        limit = i32::from(v.speed) - delta / 10;
                    }
                    limit = limit.max(25_i32.wrapping_mul(distance));
                    max_speed = max_speed.min(limit);
                }
            }
        }
        let mut u = id;
        while u != Handle::NONE {
            let part = (self.leaves.read_current_max_speed_6)(u);
            if model == 1 && u.get_track() == u64::from(DEPOT) {
                max_speed = max_speed.min(61);
                break;
            }
            if u.get_track() == u64::from(WORMHOLE) && part.status & 1 == 0 {
                max_speed = max_speed.min((self.leaves.bridge_speed)(u, part.tile) as i32);
            }
            u = part.next;
        }
        max_speed = max_speed.min((self.leaves.order_max_speed)(id) as i32);
        max_speed.min(i32::from((self.leaves.read_max_track_speed)(id)))
    }
    fn update_acceleration(&self, id: Handle) {
        let v = (self.leaves.read_update_acceleration)(id);
        // Clamp(int, int, int): the uint product converts to int first.
        let accel = ((v.power / v.weight).wrapping_mul(4) as i32).clamp(1, 255);
        (self.leaves.write_acceleration)(id, accel as u8);
    }
    fn update_speed(&self, id: Handle) -> i32 {
        let v = (self.leaves.read_update_speed)(id);
        let braking = v.status & 2 != 0 || id.flag(0) || id.flag(8);
        let max = self.current_max_speed(id);
        if (self.leaves.acc_model)(id) == 0 {
            (self.leaves.update_speed)(
                id,
                (i64::from(i32::from(v.acceleration) * if braking { -4 } else { 2 }) as u64) as u32,
                0,
                (max as u64) as i32,
            ) as i32
        } else {
            (self.leaves.update_speed)(
                id,
                (self.leaves.acceleration)(id) as u32,
                (u64::from(!braking) * 2) as i32,
                (max as u64) as i32,
            ) as i32
        }
    }
    fn mark_dirty(&self, id: Handle) {
        let mut u = id;
        loop {
            (self.leaves.write_colourmap)(u, 0);
            (self.leaves.viewport)(u, 1, 0);
            u = (self.leaves.read_next)(u);
            if u == Handle::NONE {
                break;
            }
        }
        (self.leaves.cargo_changed)(id);
        self.update_acceleration(id);
    }
    fn trackdir(&self, id: Handle) -> u8 {
        let v = (self.leaves.read_trackdir)(id);
        if v.status & 128 != 0 {
            return u8::MAX;
        }
        let track = id.get_track() as u8;
        if track == DEPOT {
            return (self.leaves.diag_trackdir)(id, (self.leaves.depot_dir)(id, v.tile) as u8)
                as u8;
        }
        if track == WORMHOLE {
            return (self.leaves.diag_trackdir)(id, (self.leaves.dir_diag)(id, v.direction) as u8)
                as u8;
        }
        (self.leaves.track_direction)(id, (self.leaves.first_track)(id, track) as u8, v.direction)
            as u8
    }
    fn running_cost(&self, id: Handle) -> i64 {
        let mut cost = 0_i64;
        let mut u = id;
        loop {
            if (self.leaves.cost_class)(u) != (self.leaves.invalid_price)(u) {
                let mut factor = self.property(
                    u,
                    PROP_TRAIN_RUNNING_COST_FACTOR,
                    (self.leaves.cost_default)(u),
                ) as u32;
                if factor != 0 {
                    if (self.leaves.read_multiheaded)(u) != 0 {
                        factor /= 2;
                    }
                    cost = cost.saturating_add((self.leaves.price)(u, factor) as i64);
                }
            }
            u = (self.leaves.read_next_unit)(u);
            if u == Handle::NONE {
                break;
            }
        }
        cost
    }
}
/// Train branch witness `index` (`crate::witness::NAMES` train order).
const BREAKDOWN_SPEEDS: [u16; 16] = [
    225, 210, 195, 180, 165, 150, 135, 120, 105, 90, 75, 60, 45, 30, 15, 15,
];
impl Game<'_> {
    fn write_status(&self, id: Handle, mask: u8, value: bool) {
        let old = (self.leaves.read_status)(id);
        (self.leaves.write_status)(
            id,
            (u64::from(if value { old | mask } else { old & !mask })) as u8,
        );
    }

    fn tile_diag(&self, tile: u32, dir: u8) -> u32 {
        (self.leaves.tile_add_diag)(Handle::NONE, tile, dir) as u32
    }
    fn exitdir(&self, id: Handle) -> u8 {
        (self.leaves.veh_exit_dir)(id, (self.leaves.read_direction)(id), id.track()) as u8
    }
    fn compatible(&self, id: Handle, tile: u32) -> bool {
        (self.leaves.compatible_rail_owner)(id, tile) != 0
            && ((self.leaves.read_front)(id) == 0
                || id.get_compatible_railtypes()
                    & (1 << (self.leaves.rail_type)(Handle::NONE, tile))
                    != 0)
    }
    fn can_leave(&self, id: Handle) -> bool {
        let v = (self.leaves.read_can_leave)(id);
        let track = id.track();
        if track == WORMHOLE || track == DEPOT {
            return false;
        }
        if (self.leaves.is_tunnelbridge)(Handle::NONE, v.tile) != 0
            && (self.leaves.tunnel_dir)(Handle::NONE, v.tile) as u8 * 2 + 1 == v.direction
        {
            return false;
        }
        if (self.leaves.is_depot)(Handle::NONE, v.tile) != 0
            && ((self.leaves.depot_dir)(Handle::NONE, v.tile) as u8 ^ 2) * 2 + 1 == v.direction
        {
            return false;
        }
        true
    }
    fn approaching_crossing(&self, id: Handle) -> u32 {
        if !self.can_leave(id) {
            return INVALID;
        }
        let base = (self.leaves.read_tile)(id);
        let dir = self.exitdir(id);
        let tile = base.wrapping_add((self.leaves.tile_offset_diag)(id, dir) as u32);
        if (self.leaves.is_crossing)(Handle::NONE, tile) == 0
            || (self.leaves.diag_axis)(id, dir)
                == (self.leaves.crossing_road_axis)(Handle::NONE, tile)
            || !self.compatible(id, tile)
        {
            return INVALID;
        }
        tile
    }
    fn train_on_tile(&self, tile: u32) -> bool {
        self.visit_tile(tile, |_| true)
    }
    fn crossing_approach(&self, tile: u32) -> bool {
        let dir = (self.leaves.axis_diag)(
            Handle::NONE,
            (self.leaves.crossing_rail_axis)(Handle::NONE, tile) as u8,
        ) as u8;
        for d in [dir, dir ^ 2] {
            let from = tile.wrapping_add((self.leaves.tile_offset_diag)(Handle::NONE, d) as u32);
            if self.visit_tile(from, |id| {
                let v = (self.leaves.read_crossing_approach)(id);
                if v.status & 128 == 0 && v.front != 0 && self.approaching_crossing(id) == tile {
                    return true;
                }

                false
            }) {
                return true;
            }
        }
        false
    }
    fn check_crossing(&self, tile: u32) -> bool {
        (self.leaves.crossing_reserved)(Handle::NONE, tile) != 0
            || self.train_on_tile(tile)
            || self.crossing_approach(tile)
    }
    fn update_crossing_tile(&self, tile: u32, sound: bool, force: bool) {
        let barred = force || self.check_crossing(tile);
        if barred != ((self.leaves.crossing_barred)(Handle::NONE, tile) != 0) {
            crate::witness::hit(if barred {
                crate::witness::TRAIN_CROSS_BAR
            } else {
                crate::witness::TRAIN_CROSS_UNBAR
            });
            if barred && sound && (self.leaves.ambient_sound)(Handle::NONE) != 0 {
                (self.leaves.crossing_sound)(Handle::NONE, tile);
            }
            (self.leaves.write_crossing_bar)(Handle::NONE, tile, u8::from(barred));
            (self.leaves.dirty_tile)(Handle::NONE, tile);
        }
    }
    fn update_crossing(&self, tile: u32, sound: bool, force: bool) {
        if (self.leaves.is_crossing)(Handle::NONE, tile) == 0 {
            return;
        }
        let mut state = force;
        let axis = (self.leaves.crossing_road_axis)(Handle::NONE, tile);
        let dir = (self.leaves.axis_diag)(Handle::NONE, axis as u8) as u8;
        for d in [dir, dir ^ 2] {
            let mut t = tile;
            while !state
                && u64::from(t) < (self.leaves.map_size)(Handle::NONE)
                && (self.leaves.is_crossing)(Handle::NONE, t) != 0
                && (self.leaves.crossing_road_axis)(Handle::NONE, t) == axis
            {
                state |= self.check_crossing(t);
                t = self.tile_diag(t, d);
            }
        }
        self.update_crossing_tile(tile, sound, state);
        for d in [dir, dir ^ 2] {
            let mut t = self.tile_diag(tile, d);
            while u64::from(t) < (self.leaves.map_size)(Handle::NONE)
                && (self.leaves.is_crossing)(Handle::NONE, t) != 0
                && (self.leaves.crossing_road_axis)(Handle::NONE, t) == axis
            {
                self.update_crossing_tile(t, sound, state);
                t = self.tile_diag(t, d);
            }
        }
    }
    fn adjacent_crossing_dirty(&self, tile: u32, axis: u64) {
        let dir = (self.leaves.axis_diag)(Handle::NONE, axis as u8) as u8;
        for d in [dir, dir ^ 2] {
            let t = self.tile_diag(tile, d);
            if u64::from(t) < (self.leaves.map_size)(Handle::NONE)
                && (self.leaves.is_crossing)(Handle::NONE, t) != 0
                && (self.leaves.crossing_road_axis)(Handle::NONE, t) == axis
            {
                (self.leaves.dirty_tile)(Handle::NONE, t);
            }
        }
    }
    fn crossing_removed(&self, tile: u32, axis: u64) {
        let dir = (self.leaves.axis_diag)(Handle::NONE, axis as u8) as u8;
        for d in [dir, dir ^ 2] {
            let diff = (self.leaves.tile_offset_diag)(Handle::NONE, d) as u32;
            let mut occupied = false;
            let mut t = tile.wrapping_add(diff);
            while u64::from(t) < (self.leaves.map_size)(Handle::NONE)
                && (self.leaves.is_crossing)(Handle::NONE, t) != 0
                && (self.leaves.crossing_road_axis)(Handle::NONE, t) == axis
            {
                occupied |= self.check_crossing(t);
                t = t.wrapping_add(diff);
            }
            if occupied {
                t = tile.wrapping_add(diff);
                if u64::from(t) < (self.leaves.map_size)(Handle::NONE)
                    && (self.leaves.is_crossing)(Handle::NONE, t) != 0
                    && (self.leaves.crossing_road_axis)(Handle::NONE, t) == axis
                {
                    (self.leaves.dirty_tile)(Handle::NONE, t);
                }
            } else {
                t = tile.wrapping_add(diff);
                while u64::from(t) < (self.leaves.map_size)(Handle::NONE)
                    && (self.leaves.is_crossing)(Handle::NONE, t) != 0
                    && (self.leaves.crossing_road_axis)(Handle::NONE, t) == axis
                {
                    if (self.leaves.crossing_barred)(Handle::NONE, t) != 0 {
                        (self.leaves.write_crossing_bar)(Handle::NONE, t, 0);
                        (self.leaves.dirty_tile)(Handle::NONE, t);
                    } else {
                        (self.leaves.dirty_tile)(Handle::NONE, t);
                        break;
                    }
                    t = t.wrapping_add(diff);
                }
            }
        }
    }
    fn bar_crossing(&self, tile: u32) {
        if (self.leaves.crossing_barred)(Handle::NONE, tile) == 0 {
            (self.leaves.write_crossing_res)(Handle::NONE, tile, 1);
            self.update_crossing(tile, true, false);
        }
    }
    fn mark_stuck(&self, id: Handle) {
        if !id.flag(8) {
            id.set_flag(8, true);
            id.set_wait_counter(0);
            (self.leaves.write_speed)(id, 0_u16);
            (self.leaves.write_subspeed)(id, 0_u8);
            (self.leaves.last_speed)(id);
            (self.leaves.start_stop_dirty)(id);
        }
    }
    fn whole_in_depot(&self, id: Handle) -> bool {
        let tile = (self.leaves.read_tile)(id);
        let mut u = id;
        while u != Handle::NONE {
            if u.track() != DEPOT || (self.leaves.read_tile)(u) != tile {
                return false;
            }
            u = (self.leaves.read_next)(u);
        }
        true
    }
    fn next_offset(&self, id: Handle) -> i32 {
        let v = (self.leaves.read_next_offset)(id);
        i32::from(v.length) / 2
            + if v.next == Handle::NONE {
                0
            } else {
                (i32::from((self.leaves.read_length)(v.next)) + 1) / 2
            }
    }
    fn after_swap(&self, id: Handle) {
        if id.track() == WORMHOLE {
            crate::witness::hit(crate::witness::TRAIN_WORMHOLE_SWAP);
        }
        if id.track() != DEPOT {
            (self.leaves.write_direction)(
                id,
                (u64::from((self.leaves.read_direction)(id) ^ 4)) as u8,
            );
        }
        let v = (self.leaves.read_after_swap)(id);
        if id.track() != WORMHOLE {
            (self.leaves.enter_tile)(id, v.tile, (v.x as u64) as i32, (v.y as u64) as i32);
        } else {
            let vt = (self.leaves.tile_virt)(id, (v.x as u64) as i32, (v.y as u64) as i32) as u32;
            if (self.leaves.is_tunnelbridge)(Handle::NONE, vt) != 0 {
                (self.leaves.enter_tile)(id, vt, (v.x as u64) as i32, (v.y as u64) as i32);
                if id.track() != WORMHOLE
                    && (self.leaves.is_bridge)(Handle::NONE, (self.leaves.read_tile)(id)) != 0
                {
                    (self.leaves.position)(id);
                    (self.leaves.inclination)(id, 1, 1);
                    return;
                }
            }
        }
        (self.leaves.position)(id);
        (self.leaves.viewport)(id, 1, 1);
    }
    fn swap_ground_flags(&self, a: Handle, b: Handle) {
        let fa = (self.leaves.read_gv_flags)(a);
        let fb = (self.leaves.read_gv_flags)(b);
        (self.leaves.write_gv_flags)(a, (u64::from(fa & !3)) as u16);
        (self.leaves.write_gv_flags)(b, (u64::from((self.leaves.read_gv_flags)(b) & !3)) as u16);
        if fa & 1 != 0 {
            (self.leaves.write_gv_flags)(b, (u64::from((self.leaves.read_gv_flags)(b) | 2)) as u16);
        } else if fa & 2 != 0 {
            (self.leaves.write_gv_flags)(b, (u64::from((self.leaves.read_gv_flags)(b) | 1)) as u16);
        }
        if fb & 1 != 0 {
            (self.leaves.write_gv_flags)(a, (u64::from((self.leaves.read_gv_flags)(a) | 2)) as u16);
        } else if fb & 2 != 0 {
            (self.leaves.write_gv_flags)(a, (u64::from((self.leaves.read_gv_flags)(a) | 1)) as u16);
        }
    }
    fn reverse_swap(&self, id: Handle, l: i32, r: i32) {
        let (mut a, mut b) = (id, id);
        for _ in 0..l {
            a = (self.leaves.read_next)(a);
        }
        for _ in 0..r {
            b = (self.leaves.read_next)(b);
        }
        if a != b {
            let va = (self.leaves.read_reverse_swap)(a);
            let vb = (self.leaves.read_reverse_swap)(b);
            self.write_status(b, 1, va.status & 1 != 0);
            self.write_status(a, 1, vb.status & 1 != 0);
            let ta = a.track();
            a.set_track(u64::from(b.track()));
            b.set_track(u64::from(ta));
            (self.leaves.write_direction)(a, (u64::from(vb.direction)) as u8);
            (self.leaves.write_direction)(b, (u64::from(va.direction)) as u8);
            (self.leaves.write_x)(a, (vb.x as u64) as i32);
            (self.leaves.write_x)(b, (va.x as u64) as i32);
            (self.leaves.write_y)(a, (vb.y as u64) as i32);
            (self.leaves.write_y)(b, (va.y as u64) as i32);
            (self.leaves.write_tile)(a, (u64::from(vb.tile)) as u32);
            (self.leaves.write_tile)(b, (u64::from(va.tile)) as u32);
            (self.leaves.write_z)(a, (vb.z as u64) as i32);
            (self.leaves.write_z)(b, (va.z as u64) as i32);
            self.swap_ground_flags(a, b);
            self.after_swap(a);
            self.after_swap(b);
        } else {
            self.swap_ground_flags(a, a);
            self.after_swap(a);
        }
    }
    fn advance_before_swap(&self, id: Handle) {
        let (mut base, mut first, mut last) = (id, id, (self.leaves.read_last)(id));
        let mut length = (self.leaves.count_chain)(id) as u32;
        while length > 2 {
            last = (self.leaves.read_previous)(last);
            first = (self.leaves.read_next)(first);
            let diff = self.next_offset(base) - self.next_offset(last);
            for _ in 0..diff {
                crate::witness::hit(crate::witness::TRAIN_UNEQUAL_BEFORE);
                self.controller(first, (self.leaves.read_next)(last), true);
            }
            base = first;
            length -= 2;
        }
    }
    fn advance_after_swap(&self, id: Handle) {
        let mut dep = id;
        while (self.leaves.read_next)(dep) != Handle::NONE
            && (dep.track() == DEPOT || (self.leaves.read_next)(dep).track() != DEPOT)
        {
            dep = (self.leaves.read_next)(dep);
        }
        let leave = (self.leaves.read_next)(dep);
        if leave != Handle::NONE {
            let d = (self.leaves.ticks_leave_depot)(dep) as i32;
            if d <= 0 {
                self.write_status(leave, 1, false);
                leave.set_track(
                    1 << (self.leaves.depot_track)(Handle::NONE, (self.leaves.read_tile)(leave)),
                );
                for _ in d..=0 {
                    self.controller(leave, Handle::NONE, true);
                }
            }
        } else {
            dep = Handle::NONE;
        }
        let (mut base, mut first, mut last) = (id, id, (self.leaves.read_last)(id));
        let mut length = (self.leaves.count_chain)(id) as u32;
        let mut nomove = dep == Handle::NONE;
        while length > 2 {
            if base == dep {
                break;
            }
            if last == dep {
                nomove = true;
            }
            last = (self.leaves.read_previous)(last);
            first = (self.leaves.read_next)(first);
            let diff = self.next_offset(last) - self.next_offset(base);
            for _ in 0..diff {
                crate::witness::hit(crate::witness::TRAIN_UNEQUAL_AFTER);
                self.controller(
                    first,
                    if nomove {
                        (self.leaves.read_next)(last)
                    } else {
                        Handle::NONE
                    },
                    true,
                );
            }
            base = first;
            length -= 2;
        }
    }
    fn reverse(&self, id: Handle) {
        crate::witness::hit(crate::witness::TRAIN_REVERSAL);
        if (self.leaves.is_depot)(Handle::NONE, (self.leaves.read_tile)(id)) != 0 {
            if self.whole_in_depot(id) {
                return;
            }
            (self.leaves.depot_dirty)(id);
        }
        if !id.flag(8) {
            // SAFETY: id.shell is the live Train for this serialized entry; no owner borrow spans the call.
            unsafe { (self.services.free_train_reservation)(id.shell.cast()) };
        }
        let crossing = self.approaching_crossing(id);
        let mut r = (self.leaves.count_chain)(id) as i32 - 1;
        self.advance_before_swap(id);
        let mut l = 0;
        loop {
            self.reverse_swap(id, l, r);
            l += 1;
            r -= 1;
            if l > r {
                break;
            }
        }
        self.advance_after_swap(id);
        if (self.leaves.is_depot)(Handle::NONE, (self.leaves.read_tile)(id)) != 0 {
            (self.leaves.depot_dirty)(id);
        }
        id.set_flag(7, !id.flag(7));
        id.set_flag(0, false);
        self.consist_changed(id, 0);
        let mut u = id;
        while u != Handle::NONE {
            (self.leaves.viewport)(u, 0, 0);
            u = (self.leaves.read_next)(u);
        }
        if crossing != INVALID {
            self.update_crossing(crossing, true, false);
        }
        let crossing = self.approaching_crossing(id);
        if crossing != INVALID {
            self.bar_crossing(crossing);
        }
        if id.track() == DEPOT {
            if id.flag(8) {
                (self.leaves.start_stop_dirty)(id);
            }
            id.set_flag(8, false);
            return;
        }
        let tile = (self.leaves.read_tile)(id);
        let mut dir = self.exitdir(id);
        if (self.leaves.is_depot)(Handle::NONE, tile) != 0
            || (self.leaves.is_tunnelbridge)(Handle::NONE, tile) != 0
        {
            dir = u8::MAX;
        }
        if self.signal_segment(id, tile, dir) == (self.leaves.sigseg_pbs)(id)
            || (self.leaves.reserve_paths)(id) != 0
        {
            let td = self.trackdir(id);
            let mut okay = !((self.leaves.is_railway)(Handle::NONE, tile) != 0
                && (self.leaves.has_signal_td)(id, tile, td) != 0
                && (self.leaves.signal_pbs)(
                    id,
                    (self.leaves.signal_type)(id, tile, id.track().trailing_zeros() as u8) as u8,
                ) == 0);
            if (self.leaves.is_depot)(Handle::NONE, tile) != 0
                && (self.leaves.trackdir_exit)(id, td)
                    == (self.leaves.depot_dir)(Handle::NONE, tile)
            {
                okay = false;
            }
            if (self.leaves.is_station)(Handle::NONE, tile) != 0 {
                (self.leaves.set_platform_res)(
                    id,
                    tile,
                    (self.leaves.trackdir_exit)(id, td) as u8,
                    1,
                );
            }
            // SAFETY: id.shell is the live Train for this serialized entry; no owner borrow spans the call.
            if unsafe { (self.services.try_path_reserve)(id.shell.cast(), false, okay) } {
                (self.leaves.check_next)(id);
            } else if (self.leaves.read_order)(id) != 3 {
                self.mark_stuck(id);
            }
        } else if id.flag(8) {
            id.set_flag(8, false);
            id.set_wait_counter(0);
        }
    }
    fn approaching_end(&self, id: Handle, signal: bool, reverse: bool) -> bool {
        let v = (self.leaves.read_approaching_end)(id);
        let mut x = v.x as u32 & 15;
        let y = v.y as u32 & 15;
        match v.direction {
            0 => x = (!x).wrapping_add(!y).wrapping_add(25),
            7 => x = (!y).wrapping_add(16),
            1 => x = (!x).wrapping_add(16),
            2 => x = (!x).wrapping_add(y).wrapping_add(9),
            3 => x = y,
            4 => x = x.wrapping_add(y).wrapping_sub(7),
            6 => x = (!y).wrapping_add(x).wrapping_add(9),
            _ => {}
        }
        if !signal
            && x.wrapping_add(
                u32::from(v.length + 1) / 2 * if v.direction & 1 != 0 { 1 } else { 2 },
            ) >= 16
        {
            (self.leaves.write_speed)(id, 0_u16);
            if reverse {
                self.reverse(id);
            }
            return false;
        }
        self.write_status(id, 16, true);
        let speed = BREAKDOWN_SPEEDS[(x & 15) as usize];
        if speed < v.speed {
            (self.leaves.write_speed)(id, (u64::from(speed)) as u16);
        }
        true
    }
    fn line_ends(&self, id: Handle, reverse: bool) -> bool {
        let v = (self.leaves.read_line_ends)(id);
        let t = v.breakdown;
        if t > 1 {
            self.write_status(id, 16, true);
            let speed = BREAKDOWN_SPEEDS[usize::from((!t >> 4) & 15)];
            if speed < v.speed {
                (self.leaves.write_speed)(id, (u64::from(speed)) as u16);
            }
        } else {
            self.write_status(id, 16, false);
        }
        if !self.can_leave(id) {
            return true;
        }
        let dir = self.exitdir(id);
        let tile = v
            .tile
            .wrapping_add((self.leaves.tile_offset_diag)(id, dir) as u32);
        let ts = (self.leaves.track_status)(id, tile, dir ^ 2) as u32;
        let reaches = (self.leaves.trackdir_reaches)(id, dir) as u16;
        let tds = ts as u16 & reaches;
        let reds = (ts >> 16) as u16 & reaches;
        let mut bits = ((tds | tds >> 8) & 63) as u8;
        if (self.leaves.no_90)(
            id,
            (self.leaves.tile_rail_type)(Handle::NONE, v.tile) as u8,
            (self.leaves.tile_rail_type)(Handle::NONE, tile) as u8,
        ) != 0
        {
            bits &= !((self.leaves.track_crosses)(id, id.track().trailing_zeros() as u8) as u8);
        }
        if bits == 0 || !self.compatible(id, tile) {
            return self.approaching_end(id, false, reverse);
        }
        if tds & reds != 0 {
            return self.approaching_end(id, true, reverse);
        }
        if (self.leaves.is_crossing)(Handle::NONE, tile) != 0 {
            self.bar_crossing(tile);
        }
        true
    }
}
const INITIAL_SUBCOORD: [[[u8; 3]; 4]; 6] = [
    [[15, 8, 1], [0, 0, 0], [0, 8, 5], [0, 0, 0]],
    [[0, 0, 0], [8, 0, 3], [0, 0, 0], [8, 15, 7]],
    [[0, 0, 0], [7, 0, 2], [0, 7, 6], [0, 0, 0]],
    [[15, 8, 2], [0, 0, 0], [0, 0, 0], [8, 15, 6]],
    [[15, 7, 0], [8, 0, 4], [0, 0, 0], [0, 0, 0]],
    [[0, 0, 0], [0, 0, 0], [0, 8, 4], [7, 15, 0]],
];
impl Game<'_> {
    fn enter_station(&self, id: Handle, station: u16) {
        (self.leaves.write_last_station)(id, (u64::from(station)) as u16);
        if (self.leaves.visit_type)(id, station) & (self.leaves.train_visit)(id) == 0 {
            (self.leaves.write_visit_type)(id, station);
            (self.leaves.arrival_news)(id, station);
        }
        id.set_force_proceed(0);
        (self.leaves.view_window)(id);
        (self.leaves.begin_loading)(id);
        (self.leaves.arrival_triggers)(id, station);
    }
    fn speed_z(&self, id: Handle, old: i32) {
        let v = (self.leaves.read_speed_z)(id);
        if old == v.z || (self.leaves.acc_model)(id) != 0 {
            return;
        }
        if old < v.z {
            (self.leaves.write_speed)(
                id,
                (u64::from(v.speed - ((u32::from(v.speed) * 64) >> 8) as u16)) as u16,
            );
        } else {
            let speed = v.speed.wrapping_add(2);
            if speed <= v.max_track_speed {
                (self.leaves.write_speed)(id, (u64::from(speed)) as u16);
            }
        }
    }
    fn moved_signals(&self, id: Handle, tile: u32, dir: u8) -> bool {
        if (self.leaves.is_railway)(Handle::NONE, tile) != 0
            && (self.leaves.has_signals)(Handle::NONE, tile) != 0
        {
            let tracks = (self.leaves.track_bits)(Handle::NONE, tile) as u16;
            let tds = (tracks | tracks << 8) & (self.leaves.trackdir_reaches)(id, dir) as u16;
            let td = tds.trailing_zeros() as u8;
            let exit = (self.leaves.trackdir_exit)(id, td);
            // Tile owner is used here, rather than the moving train's owner.
            if u64::from(self.signal_context().segment(
                tile,
                exit as u8,
                (self.leaves.tile_owner)(Handle::NONE, tile) as u8,
            )) == (self.leaves.sigseg_pbs)(id)
                && (self.leaves.has_signal_td)(id, tile, td) != 0
            {
                if (self.leaves.signal_pbs)(id, (self.leaves.signal_type)(id, tile, td & 7) as u8)
                    == 0
                {
                    return true;
                }
            }
        }
        false
    }
    // Result: 0 moved, 1 no move/early stop, 2 invalid rail, 3 requested reversal.
    fn move_vehicle(&self, id: Handle, prev: Handle, reverse: bool) -> (u8, bool) {
        let v = (self.leaves.read_move_vehicle)(id);
        if v.articulated != 0 {
            crate::witness::hit(crate::witness::TRAIN_ARTICULATED_MOVE);
        }
        let delta = [
            (-1, -1),
            (-1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
            (1, 0),
            (1, -1),
            (0, -1),
        ];
        let (dx, dy) = delta[usize::from(v.direction)];
        let (mut x, mut y) = (v.x.wrapping_add(dx), v.y.wrapping_add(dy));
        let mut old = v.tile;
        let new = (self.leaves.tile_virt)(id, (x as u64) as i32, (y as u64) as i32) as u32;
        let mut entered = 0_u8;
        let mut signals = false;
        let mut changed = false;
        if id.track() != WORMHOLE {
            if old == new {
                if id.track() == DEPOT {
                    x = v.x;
                    y = v.y;
                } else {
                    if v.front != 0 && !self.line_ends(id, reverse) {
                        return (1, false);
                    }
                    let vets =
                        (self.leaves.enter_tile)(id, new, (x as u64) as i32, (y as u64) as i32)
                            as u8;
                    if vets & 4 != 0 {
                        return (2, false);
                    }
                    if vets & 1 != 0 {
                        self.enter_station(id, (self.leaves.station)(Handle::NONE, new) as u16);
                    }
                }
            } else {
                entered = (self.leaves.diag_between)(id, old, new) as u8;
                let ts = (self.leaves.track_status)(id, new, entered ^ 2) as u32;
                let reaches = (self.leaves.trackdir_reaches)(id, entered) as u16;
                let tds = ts as u16 & reaches;
                let reds = (((ts >> 16) as u16 & reaches) | (((ts >> 16) as u16 & reaches) >> 8))
                    as u8
                    & 63;
                let mut bits = ((tds | tds >> 8) & 63) as u8;
                if (self.leaves.no_90)(
                    id,
                    (self.leaves.tile_rail_type)(Handle::NONE, old) as u8,
                    (self.leaves.tile_rail_type)(Handle::NONE, new) as u8,
                ) != 0
                    && prev == Handle::NONE
                {
                    bits &=
                        !((self.leaves.track_crosses)(id, id.track().trailing_zeros() as u8) as u8);
                }
                if bits == 0 || !self.compatible(id, new) {
                    return (2, false);
                }
                let mut chosen;
                if prev == Handle::NONE {
                    chosen = 1_u8 << (self.leaves.choose_track)(id, new, entered, bits);
                    if id.get_force_proceed() != 0
                        && (self.leaves.is_plain_rail)(Handle::NONE, new) != 0
                        && (self.leaves.has_signals)(Handle::NONE, new) != 0
                    {
                        let td = tds.trailing_zeros() as u8;
                        if (self.leaves.has_signal_td)(id, new, td) != 0
                            || ((self.leaves.has_signal_td)(id, new, td ^ 8) != 0
                                && (self.leaves.signal_type)(id, new, td & 7)
                                    != (self.leaves.pbs_signal_type)(id))
                        {
                            crate::witness::hit(crate::witness::TRAIN_FORCE_SIGNAL);
                            id.set_force_proceed(u64::from(id.get_force_proceed() == 2));
                            (self.leaves.view_window)(id);
                        }
                    }
                    if reds & chosen != 0 && id.get_force_proceed() == 0 {
                        let td = tds.trailing_zeros() as u8;
                        if id.flag(8) {
                            return (1, false);
                        }
                        if (self.leaves.has_signal_td)(id, new, td ^ 8) == 0 {
                            (self.leaves.write_speed)(id, 0_u16);
                            (self.leaves.write_subspeed)(id, 0_u8);
                            (self.leaves.write_progress)(id, 255_u8);
                            crate::witness::hit(crate::witness::TRAIN_RED_ONEWAY);
                            if (self.leaves.reverse_at_signals)(id) == 0
                                || u64::from(id.wait_inc())
                                    < (self.leaves.wait_oneway)(id)
                                        * (self.leaves.day_ticks)(id)
                                        * 2
                            {
                                return (1, false);
                            }
                        } else if (self.leaves.has_signal_td)(id, new, td) != 0 {
                            (self.leaves.write_speed)(id, 0_u16);
                            (self.leaves.write_subspeed)(id, 0_u8);
                            (self.leaves.write_progress)(id, 255_u8);
                            crate::witness::hit(crate::witness::TRAIN_RED_TWOWAY);
                            if (self.leaves.reverse_at_signals)(id) == 0
                                || u64::from(id.wait_inc())
                                    < (self.leaves.wait_twoway)(id)
                                        * (self.leaves.day_ticks)(id)
                                        * 2
                            {
                                let dir = (self.leaves.trackdir_exit)(id, td) as u8;
                                let tile = self.tile_diag(new, dir);
                                let mut waiting = false;
                                self.visit_tile(tile, |u| {
                                    let other = (self.leaves.read_move_vehicle_20)(u);
                                    if other.status & 128 == 0
                                        && other.front != 0
                                        && u.track() & 63 != 0
                                        && other.speed <= 5
                                        && self.exitdir(u) == dir ^ 2
                                    {
                                        waiting = true;
                                        return true;
                                    }

                                    false
                                });
                                if !waiting {
                                    return (1, false);
                                }
                            }
                        }
                        if (self.leaves.reverse_at_signals)(id) == 0
                            && (self.leaves.oneway_blocking)(id, new, td) == 0
                            && self.signal_segment(id, (self.leaves.read_tile)(id), entered)
                                == (self.leaves.sigseg_pbs)(id)
                        {
                            id.set_wait_counter(0);
                            return (1, false);
                        }
                        return (3, false);
                    }
                    (self.leaves.try_reserve)(id, new, chosen.trailing_zeros() as u8, 0);
                } else {
                    let before = (self.leaves.read_tile)(prev);
                    if before == new {
                        chosen = if prev.track() == WORMHOLE {
                            bits
                        } else {
                            prev.track()
                        };
                    } else {
                        let exit = (self.leaves.diag_between)(id, new, before) as usize;
                        let connecting =
                            [[1, 8, 0, 16], [4, 2, 16, 0], [0, 32, 1, 4], [32, 0, 8, 2]];
                        chosen = connecting[usize::from(entered)][exit];
                    }
                    chosen &= bits;
                }
                let sub = INITIAL_SUBCOORD[chosen.trailing_zeros() as usize][usize::from(entered)];
                x = (x & !15) | i32::from(sub[0]);
                y = (y & !15) | i32::from(sub[1]);
                let dir = sub[2];
                let vets =
                    (self.leaves.enter_tile)(id, new, (x as u64) as i32, (y as u64) as i32) as u8;
                if vets & 4 != 0 {
                    return (2, false);
                }
                if vets & 2 == 0 {
                    if (self.leaves.read_front)(id) != 0 {
                        crate::witness::hit(crate::witness::TRAIN_FIRST_TILE_RAIL);
                    }
                    let td =
                        (self.leaves.track_direction)(id, chosen.trailing_zeros() as u8, dir) as u8;
                    if (self.leaves.read_front)(id) != 0
                        && (self.leaves.signal_has_pbs)(id, new, td) != 0
                    {
                        (self.leaves.set_signal_state)(id, new, td, 0);
                        (self.leaves.dirty_tile)(Handle::NONE, new);
                    }
                    if (self.leaves.read_next)(id) == Handle::NONE {
                        (self.leaves.clear_reservation)(
                            id,
                            (self.leaves.read_tile)(id),
                            self.trackdir(id),
                        );
                    }
                    (self.leaves.write_tile)(id, (u64::from(new)) as u32);
                    if (self.leaves.tile_rail_type)(Handle::NONE, new)
                        != (self.leaves.tile_rail_type)(Handle::NONE, old)
                    {
                        self.consist_changed((self.leaves.read_first)(id), 0);
                    }
                    id.set_track(u64::from(chosen));
                }
                signals = true;
                if dir != (self.leaves.read_direction)(id) {
                    if prev == Handle::NONE && (self.leaves.acc_model)(id) == 0 {
                        let diff = (self.leaves.read_direction)(id).wrapping_sub(dir) & 7;
                        let small = if (self.leaves.acc_type)(id) == 2 {
                            0
                        } else {
                            64
                        };
                        let fraction = if diff == 1 || diff == 7 { small } else { 128 };
                        let speed = (self.leaves.read_speed)(id);
                        (self.leaves.write_speed)(
                            id,
                            (u64::from(speed - ((u32::from(speed) * fraction) >> 8) as u16)) as u16,
                        );
                    }
                    changed = true;
                    (self.leaves.write_direction)(id, (u64::from(dir)) as u8);
                }
                if (self.leaves.read_front)(id) != 0 {
                    id.set_wait_counter(0);
                    let crossing = self.approaching_crossing(id);
                    if crossing != INVALID
                        && (self.leaves.crossing_reserved)(Handle::NONE, crossing) != 0
                        && (self.leaves.ambient_sound)(id) != 0
                    {
                        (self.leaves.crossing_sound)(Handle::NONE, crossing);
                    }
                    (self.leaves.check_next)(id);
                }
                if vets & 1 != 0 {
                    self.enter_station(id, (self.leaves.station)(Handle::NONE, new) as u16);
                }
            }
        } else {
            if (self.leaves.is_tunnelbridge)(Handle::NONE, new) != 0
                && (self.leaves.enter_tile)(id, new, (x as u64) as i32, (y as u64) as i32) & 2 != 0
            {
                if (self.leaves.read_front)(id) != 0 {
                    crate::witness::hit(crate::witness::TRAIN_WORMHOLE_EXIT);
                    self.reserve_track(id, new, (self.leaves.tunnel_dir)(Handle::NONE, new) & 1);
                    (self.leaves.check_next)(id);
                }
                if old == new {
                    old = (self.leaves.other_end)(Handle::NONE, old) as u32;
                }
            } else {
                (self.leaves.write_x)(id, (x as u64) as i32);
                (self.leaves.write_y)(id, (y as u64) as i32);
                (self.leaves.position)(id);
                if (self.leaves.read_status)(id) & 1 == 0 {
                    (self.leaves.base_viewport)(id, 1);
                }
                return (0, false);
            }
        }
        (self.leaves.update_delta)(id);
        (self.leaves.write_x)(id, (x as u64) as i32);
        (self.leaves.write_y)(id, (y as u64) as i32);
        (self.leaves.position)(id);
        let old_z = (self.leaves.inclination)(id, u8::from(new != old), 0) as i32;
        if prev == Handle::NONE {
            self.speed_z(id, old_z);
        }
        if signals {
            if (self.leaves.read_front)(id) != 0 && self.moved_signals(id, new, entered) {
                if ((self.leaves.has_reserved)(id, new, id.track()) == 0
                    && self.reserve_track(id, new, u64::from(id.track().trailing_zeros())) == 0)
                    // SAFETY: id.shell is the live Train for this serialized entry; no owner borrow spans the call.
                    || !unsafe { (self.services.try_path_reserve)(id.shell.cast(), false, false) }
                {
                    self.mark_stuck(id);
                }
            }
            if (self.leaves.read_next)(id) == Handle::NONE {
                self.moved_signals(id, old, entered ^ 2);
                if (self.leaves.is_crossing)(Handle::NONE, old) != 0 {
                    self.update_crossing(old, true, false);
                }
            }
        }
        if (self.leaves.read_front)(id) != 0
            && u64::from((self.leaves.read_tick)(id)) % (self.leaves.backoff)(id) == 0
        {
            (self.leaves.check_next)(id);
        }
        (0, changed)
    }
    fn controller(&self, mut id: Handle, nomove: Handle, reverse: bool) -> bool {
        let first = (self.leaves.read_first)(id);
        let mut prev = (self.leaves.read_previous)(id);
        let mut changed = false;
        while id != nomove {
            let (outcome, direction) = self.move_vehicle(id, prev, reverse);
            changed |= direction;
            if outcome == 1 {
                return false;
            }
            if outcome == 2 && prev != Handle::NONE {
                (self.leaves.disconnect)(id);
            }
            if outcome >= 2 {
                if reverse {
                    id.set_wait_counter(0);
                    (self.leaves.write_speed)(id, 0_u16);
                    (self.leaves.write_subspeed)(id, 0_u8);
                    self.reverse(id);
                }
                return false;
            }
            prev = id;
            id = (self.leaves.read_next)(id);
        }
        if changed {
            first.set_cached_max_curve_speed(u64::from(self.curve_limit(first)));
        }
        true
    }
}

impl Game<'_> {
    /// `TryReserveRailTrack(tile, track)` with its default station triggers.
    fn reserve_track(&self, id: Handle, tile: u32, track: u64) -> u64 {
        (self.leaves.try_reserve)(id, tile, track as u8, 1)
    }
    fn crash(&self, id: Handle, flooded: bool) -> u32 {
        let mut victims = 0_u32;
        if (self.leaves.read_front)(id) != 0 {
            victims += 2;
            if !id.flag(8) {
                // SAFETY: id.shell is the live Train for this serialized entry; no owner borrow spans the call.
                unsafe { (self.services.free_train_reservation)(id.shell.cast()) };
            }
            let mut u = id;
            while u != Handle::NONE {
                let tile = (self.leaves.read_tile)(u);
                (self.leaves.clear_reservation)(u, tile, self.trackdir(u));
                if (self.leaves.is_tunnelbridge)(Handle::NONE, tile) != 0 {
                    (self.leaves.set_tunnel_res)(
                        u,
                        (self.leaves.other_end)(Handle::NONE, tile) as u32,
                        0,
                    );
                }
                u = (self.leaves.read_next)(u);
            }
            let crossing = self.approaching_crossing(id);
            if crossing != INVALID {
                self.update_crossing(crossing, true, false);
            }
            (self.leaves.hide_fill)(id);
        }
        victims = victims.wrapping_add((self.leaves.crash_ground)(id, u8::from(flooded)) as u32);
        id.set_crash_anim_pos(if flooded { 4000 } else { 1 });
        victims
    }
    fn crashed(&self, id: Handle) -> u32 {
        let mut victims = 0;
        if (self.leaves.read_status)(id) & 128 == 0 {
            victims = self.crash(id, false);
            (self.leaves.crash_event)(id, victims);
        }
        (self.leaves.reserve_under)(id);
        victims
    }
    fn collision_one(&self, other: Handle, id: Handle) -> u32 {
        if other.track() == DEPOT {
            return 0;
        }
        let v = (self.leaves.read_collision_one)(other);
        let t = (self.leaves.read_collision_one_22)(id);
        if v.owner != t.owner || v.first == id {
            return 0;
        }
        let x = v.x.wrapping_sub(t.x);
        let y = v.y.wrapping_sub(t.y);
        if ((y.wrapping_add(7) | x.wrapping_add(7)) as u32) & !15 != 0 {
            return 0;
        }
        let min = (i32::from(v.length) + 1) / 2 + (i32::from(t.length) + 1) / 2 - 1;
        if x.wrapping_mul(x).wrapping_add(y.wrapping_mul(y)) > min.wrapping_mul(min)
            || v.z.wrapping_sub(t.z).wrapping_abs() > 5
        {
            return 0;
        }
        let victims = self.crashed(id);
        victims.wrapping_add(self.crashed(v.first))
    }
    fn collision(&self, id: Handle) -> bool {
        if id.track() == DEPOT {
            return false;
        }
        let v = (self.leaves.read_after_swap)(id);
        let mut victims = 0_u32;
        if id.track() == WORMHOLE {
            self.visit_tile(v.tile, |u| {
                victims = victims.wrapping_add(self.collision_one(u, id));

                false
            });
            self.visit_tile((self.leaves.other_end)(Handle::NONE, v.tile) as u32, |u| {
                victims = victims.wrapping_add(self.collision_one(u, id));

                false
            });
        } else {
            self.visit_near(v.x, v.y, |u| {
                victims = victims.wrapping_add(self.collision_one(u, id));

                false
            });
        }
        if victims == 0 {
            return false;
        }
        crate::witness::hit(crate::witness::TRAIN_COLLISION);
        (self.leaves.crash_news)(id, victims);
        (self.leaves.crash_rating)(id);
        if (self.leaves.disaster_sound)(id) != 0 {
            (self.leaves.crash_sound)(id);
        }
        true
    }
    fn platform_occupied(&self, tile: u32) -> bool {
        let delta = (self.leaves.tile_offset_axis)(
            Handle::NONE,
            (self.leaves.station_axis)(Handle::NONE, tile) as u8,
        ) as u32;
        let mut t = tile;
        while (self.leaves.station_compatible)(Handle::NONE, t, tile) != 0 {
            if self.train_on_tile(t) {
                return true;
            }
            t = t.wrapping_sub(delta);
        }
        t = tile.wrapping_add(delta);
        while (self.leaves.station_compatible)(Handle::NONE, t, tile) != 0 {
            if self.train_on_tile(t) {
                return true;
            }
            t = t.wrapping_add(delta);
        }
        false
    }
    fn delete_last(&self, mut id: Handle) {
        crate::witness::hit(crate::witness::TRAIN_CRASH_DELETE);
        let first = (self.leaves.read_first)(id);
        let mut last = id;
        while (self.leaves.read_next)(id) != Handle::NONE {
            last = id;
            id = (self.leaves.read_next)(id);
        }
        (self.leaves.set_next)(last, Handle::NONE);
        if first != id {
            self.consist_changed(first, 3);
            if first.track() == DEPOT {
                (self.leaves.depot_window)(first, (self.leaves.read_tile)(first));
            }
            (self.leaves.write_last_station)(
                id,
                (u64::from((self.leaves.read_last_station)(first))) as u16,
            );
        }
        let mut tracks = id.track();
        let v = (self.leaves.read_delete_last)(id);
        let tile = v.tile;
        let owner = v.owner;
        (self.leaves.delete_vehicle)(id);
        if tracks == WORMHOLE {
            tracks = 1 << ((self.leaves.tunnel_dir)(Handle::NONE, tile) & 1);
        }
        let track = tracks.trailing_zeros() as u8;
        if (self.leaves.has_reserved)(Handle::NONE, tile, tracks) != 0 {
            (self.leaves.unreserve)(Handle::NONE, tile, track);
            let mut remaining = 0_u8;
            self.visit_tile(tile, |u| {
                if (self.leaves.read_status)(u) & 128 == 0 {
                    return false;
                }
                let t = u.track();
                if t == WORMHOLE {
                    remaining |= 1
                        << ((self.leaves.tunnel_dir)(Handle::NONE, (self.leaves.read_tile)(u)) & 1);
                } else if t != DEPOT {
                    remaining |= t;
                }

                false
            });
            for t in 0..6 {
                if remaining & (1 << t) != 0 {
                    self.reserve_track(Handle::NONE, tile, t);
                }
            }
        }
        if (self.leaves.is_crossing)(Handle::NONE, tile) != 0 {
            self.update_crossing(tile, true, false);
        }
        if (self.leaves.is_station)(Handle::NONE, tile) != 0 {
            let occupied = self.platform_occupied(tile);
            let dir = (self.leaves.axis_diag)(
                Handle::NONE,
                (self.leaves.station_axis)(Handle::NONE, tile) as u8,
            ) as u8;
            (self.leaves.set_platform_res)(Handle::NONE, tile, dir, u8::from(occupied));
            (self.leaves.set_platform_res)(Handle::NONE, tile, dir ^ 2, u8::from(occupied));
        }
        if (self.leaves.is_tunnelbridge)(Handle::NONE, tile) != 0
            || (self.leaves.is_depot)(Handle::NONE, tile) != 0
        {
            self.signal_context().segment(tile, u8::MAX, owner);
        } else {
            self.signal_context().both(tile, track, owner);
        }
    }
    fn change_dir_randomly(&self, mut id: Handle) {
        loop {
            if (self.leaves.read_status)(id) & 1 == 0 {
                let delta = [7, 0, 0, 1][(self.services.random() & 3) as usize];
                (self.leaves.write_direction)(
                    id,
                    (u64::from((self.leaves.read_direction)(id).wrapping_add(delta) & 7)) as u8,
                );
                if id.track() != WORMHOLE {
                    (self.leaves.position)(id);
                    (self.leaves.inclination)(id, 0, 1);
                } else {
                    (self.leaves.viewport)(id, 0, 1);
                }
            }
            id = (self.leaves.read_next)(id);
            if id == Handle::NONE {
                break;
            }
        }
    }
    fn handle_crashed(&self, id: Handle) -> bool {
        let state = (id.get_crash_anim_pos() as u16).wrapping_add(1);
        id.set_crash_anim_pos(u64::from(state));
        if state == 4 && (self.leaves.read_status)(id) & 1 == 0 {
            (self.leaves.large_explosion)(id);
        }
        if state <= 200 {
            let mut r = self.services.random();
            if crate::services::chance16_i(1, 7, r) {
                let mut index = (r.wrapping_mul(10) >> 16) as i32;
                let mut u = id;
                loop {
                    index -= 1;
                    if index < 0 {
                        r = self.services.random();
                        (self.leaves.small_explosion)(
                            u,
                            (((r >> 8) & 7) + 2) as i32,
                            (((r >> 16) & 7) + 2) as i32,
                            ((r & 7) + 5) as i32,
                        );
                        break;
                    }
                    u = (self.leaves.read_next)(u);
                    if u == Handle::NONE {
                        break;
                    }
                }
            }
        }
        let tick = (self.leaves.read_tick)(id);
        if state <= 240 && tick & 3 == 0 {
            self.change_dir_randomly(id);
        }
        if state >= 4440 && tick & 31 == 0 {
            let remains = (self.leaves.read_next)(id) != Handle::NONE;
            self.delete_last(id);
            return remains;
        }
        true
    }
    fn stay_depot(&self, id: Handle) -> bool {
        if !self.whole_in_depot(id) {
            return false;
        }
        let v = (self.leaves.read_stay_depot)(id);
        if v.power == 0 {
            self.write_status(id, 2, true);
            (self.leaves.depot_window)(id, v.tile);
            return true;
        }
        if (self.leaves.wait_unbunch)(id) != 0 {
            return true;
        }
        if id.get_force_proceed() == 0 {
            if id.wait_inc() < 37 {
                (self.leaves.train_list)(id);
                return true;
            }
            id.set_wait_counter(0);
        }
        let segment = if (self.leaves.reserve_paths)(id) != 0 {
            (self.leaves.sigseg_pbs)(id)
        } else {
            self.signal_segment(id, v.tile, u8::MAX)
        };
        if id.get_force_proceed() == 0
            && (segment == (self.leaves.sigseg_full)(id)
                || (self.leaves.has_depot_res)(Handle::NONE, v.tile) != 0)
        {
            (self.leaves.train_list)(id);
            return true;
        }
        if (self.leaves.read_order)(id) == 2
            && (self.leaves.read_tile)(id) == (self.leaves.read_dest)(id)
        {
            if (self.leaves.has_depot_res)(Handle::NONE, (self.leaves.read_tile)(id)) == 0 {
                crate::witness::hit(crate::witness::TRAIN_DEPOT_REENTRY);
                (self.leaves.enter_depot)(id);
            }
            return true;
        }
        if segment == (self.leaves.sigseg_pbs)(id)
            // SAFETY: id.shell is the live Train for this serialized entry; no owner borrow spans the call.
            && !unsafe { (self.services.try_path_reserve)(id.shell.cast(), false, false) }
            && id.get_force_proceed() == 0
        {
            (self.leaves.train_list)(id);
            self.mark_stuck(id);
            return true;
        }
        (self.leaves.set_depot_res)(id, (self.leaves.read_tile)(id), 1);
        if (self.leaves.show_reservation)(id) != 0 {
            (self.leaves.dirty_tile)(Handle::NONE, (self.leaves.read_tile)(id));
        }
        crate::witness::hit(crate::witness::TRAIN_DEPOT_START);
        (self.leaves.service)(id);
        (self.leaves.leave_unbunch)(id);
        (self.leaves.leave_sound)(id);
        (self.leaves.train_list)(id);
        id.set_track(if (self.leaves.read_direction)(id) & 2 != 0 {
            2
        } else {
            1
        });
        self.write_status(id, 1, false);
        (self.leaves.write_speed)(id, 0_u16);
        (self.leaves.viewport)(id, 1, 1);
        (self.leaves.position)(id);
        self.signal_segment(id, (self.leaves.read_tile)(id), u8::MAX);
        self.update_acceleration(id);
        (self.leaves.depot_dirty)(id);
        false
    }
    fn loco(&self, id: Handle, mode: bool) -> bool {
        crate::witness::hit(if mode {
            crate::witness::TRAIN_TICK_SECOND
        } else {
            crate::witness::TRAIN_TICK_FIRST
        });
        if (self.leaves.read_status)(id) & 128 != 0 {
            return if mode { true } else { self.handle_crashed(id) };
        }
        if id.get_force_proceed() != 0 {
            id.set_flag(8, false);
            (self.leaves.start_stop_dirty)(id);
        }
        if (self.leaves.handle_breakdown)(id) != 0 {
            return true;
        }
        if id.flag(0) && (self.leaves.read_speed)(id) == 0 {
            self.reverse(id);
        }
        let v = (self.leaves.read_loco)(id);
        if v.status & 2 != 0 && v.speed == 0 {
            return true;
        }
        let valid_order = v.order != 0 && v.order != 7;
        if (self.leaves.process_orders)(id) != 0 && (self.leaves.check_reverse)(id) != 0 {
            id.set_wait_counter(0);
            (self.leaves.write_speed)(id, 0_u16);
            (self.leaves.write_subspeed)(id, 0_u8);
            id.set_flag(9, false);
            self.reverse(id);
            return true;
        } else if id.flag(9) {
            let tile = (self.leaves.read_tile)(id);
            let mut dir = self.exitdir(id);
            if (self.leaves.is_depot)(Handle::NONE, tile) != 0
                || (self.leaves.is_tunnelbridge)(Handle::NONE, tile) != 0
            {
                dir = u8::MAX;
            }
            if self.signal_segment(id, tile, dir) == (self.leaves.sigseg_pbs)(id)
                || (self.leaves.reserve_paths)(id) != 0
            {
                // SAFETY: id.shell is the live Train for this serialized entry; no owner borrow spans the call.
                unsafe { (self.services.try_path_reserve)(id.shell.cast(), true, true) };
            }
            id.set_flag(9, false);
        }
        (self.leaves.loading)(id, u8::from(mode));
        if (self.leaves.read_order)(id) == 3 || self.stay_depot(id) {
            return true;
        }
        if !mode {
            (self.leaves.show_effect)(id);
        }
        if !valid_order && (self.leaves.read_order)(id) != 0 {
            (self.leaves.check_next)(id);
        }
        if !mode && id.flag(8) {
            crate::witness::hit(crate::witness::TRAIN_STUCK_RETRY);
            let wait = u64::from(id.wait_inc());
            let turn = wait % ((self.leaves.wait_pbs)(id) * (self.leaves.day_ticks)(id)) == 0
                && (self.leaves.reverse_at_signals)(id) != 0;
            if !turn && wait % (self.leaves.backoff)(id) != 0 && id.get_force_proceed() == 0 {
                return true;
            }
            // SAFETY: id.shell is the live Train for this serialized entry; no owner borrow spans the call.
            if !unsafe { (self.services.try_path_reserve)(id.shell.cast(), false, false) } {
                if turn {
                    crate::witness::hit(crate::witness::TRAIN_STUCK_REVERSE);
                    self.reverse(id);
                }
                if id.flag(8)
                    && id.get_wait_counter()
                        > 2 * (self.leaves.wait_pbs)(id) * (self.leaves.day_ticks)(id)
                {
                    if (self.leaves.lost_warn)(id) != 0
                        && u64::from((self.leaves.read_owner)(id))
                            == (self.leaves.local_company)(id)
                    {
                        (self.leaves.stuck_news)(id);
                    }
                    id.set_wait_counter(0);
                }
                if id.get_force_proceed() == 0 {
                    return true;
                }
                id.set_flag(8, false);
                id.set_wait_counter(0);
                (self.leaves.start_stop_dirty)(id);
            }
        }
        if (self.leaves.read_order)(id) == 4 {
            (self.leaves.order_free)(id);
            (self.leaves.start_stop_dirty)(id);
            return true;
        }
        let mut distance = self.update_speed(id);
        if (self.leaves.read_speed)(id) == 0 && (self.leaves.read_status)(id) & 2 != 0 {
            id.set_force_proceed(0);
            (self.leaves.view_window)(id);
        }
        let mut advance = (self.leaves.advance_distance)(id) as i32;
        if distance < advance {
            if (self.leaves.read_speed)(id) == 0 {
                (self.leaves.last_speed)(id);
            }
        } else {
            self.line_ends(id, true);
            loop {
                distance = distance.wrapping_sub(advance);
                self.controller(id, Handle::NONE, true);
                if self.collision(id) {
                    break;
                }
                advance = (self.leaves.advance_distance)(id) as i32;
                if distance < advance || (self.leaves.read_speed)(id) == 0 {
                    break;
                }
                let v = (self.leaves.read_loco_26)(id);
                if (v.order == 6 || v.order == 1)
                    && v.nonstop & 2 != 0
                    && (self.leaves.is_station_any)(Handle::NONE, v.tile) != 0
                    && u64::from(v.order_destination) == (self.leaves.station)(Handle::NONE, v.tile)
                {
                    (self.leaves.process_orders)(id);
                }
            }
            (self.leaves.last_speed)(id);
        }
        let mut u = id;
        while u != Handle::NONE {
            if (self.leaves.read_status)(u) & 1 == 0 {
                (self.leaves.viewport)(u, 0, 0);
            }
            u = (self.leaves.read_next)(u);
        }
        if (self.leaves.read_progress)(id) == 0 {
            (self.leaves.write_progress)(id, (distance as u64) as u8);
        }
        true
    }
    fn tick(&self, id: Handle) -> bool {
        (self.leaves.write_tick)(
            id,
            (u64::from((self.leaves.read_tick)(id).wrapping_add(1))) as u8,
        );
        let v = (self.leaves.read_tick_state)(id);
        if v.front != 0 {
            if v.status & 2 == 0 || v.speed > 0 {
                (self.leaves.write_running)(id, (u64::from(v.running.wrapping_add(1))) as u8);
            }
            (self.leaves.write_order_time)(
                id,
                ((self.leaves.read_order_time)(id).wrapping_add(1) as u64) as i32,
            );
            if !self.loco(id, false) {
                return false;
            }
            return self.loco(id, true);
        } else if v.free_wagon != 0 && v.status & 128 != 0 {
            let state = (id.get_crash_anim_pos() as u16).wrapping_add(1);
            id.set_crash_anim_pos(u64::from(state));
            if state >= 4400 {
                crate::witness::hit(crate::witness::TRAIN_FREE_WAGON_DELETE);
                (self.leaves.delete_vehicle)(id);
                return false;
            }
        }
        true
    }
    fn needs_service(&self, id: Handle) {
        if (self.leaves.servint)(id) == 0 || (self.leaves.needs_service)(id) == 0 {
            return;
        }
        if (self.leaves.chain_depot)(id) != 0 {
            crate::witness::hit(crate::witness::TRAIN_DEPOT_SERVICE);
            (self.leaves.service)(id);
            return;
        }
        let max = (self.leaves.max_depot_penalty)(id) as u32;
        let depot = (self.leaves.find_depot)(id, max as i32);
        let length = depot.length;
        if length == u32::MAX || length > max {
            if (self.leaves.read_order)(id) == 2 {
                (self.leaves.order_dummy)(id);
                (self.leaves.start_stop_dirty)(id);
            }
            return;
        }
        let tile = depot.tile;
        let depot_id = (self.leaves.depot_index)(Handle::NONE, tile) as u16;
        let v = (self.leaves.read_needs_service)(id);
        if v.order == 2
            && v.order_destination != depot_id
            && !crate::services::chance16_i(3, 16, self.services.random())
        {
            return;
        }
        (self.leaves.suppress_implicit)(id);
        (self.leaves.order_depot_service)(id, depot_id);
        (self.leaves.write_dest)(id, (u64::from(tile)) as u32);
        (self.leaves.start_stop_dirty)(id);
    }
    fn economy_day(&self, id: Handle) {
        (self.leaves.economy_age)(id);
        let day = (self.leaves.read_day)(id).wrapping_add(1);
        (self.leaves.write_day)(id, (u64::from(day)) as u8);
        if day & 7 == 0 {
            (self.leaves.decrease_value)(id);
        }
        if (self.leaves.read_front)(id) != 0 {
            (self.leaves.check_breakdown)(id);
            self.needs_service(id);
            (self.leaves.check_orders)(id);
            if (self.leaves.read_order)(id) == 1 {
                let tile = (self.leaves.station_dest)(id) as u32;
                if tile != INVALID {
                    (self.leaves.write_dest)(id, (u64::from(tile)) as u32);
                }
            }
            let running = (self.leaves.read_running)(id);
            if running != 0 {
                let cost = self.running_cost(id).saturating_mul(i64::from(running))
                    / ((self.leaves.cost_divisor)(id) as i64);
                (self.leaves.pay_running)(id, (cost as u64) as i64);
                (self.leaves.running_windows)(id);
            }
        }
    }
}
impl Game<'_> {
    fn next_force(&self, id: Handle) -> u8 {
        let v = (self.leaves.read_next_force)(id);
        if v.status & 128 != 0 || id.get_force_proceed() == 2 {
            return 0;
        }
        if !id.flag(8) {
            return if (self.leaves.chain_depot)(id) != 0 {
                1
            } else {
                2
            };
        }
        let dir = (self.leaves.trackdir_exit)(id, self.trackdir(id)) as u8;
        let tile = self.tile_diag(v.tile, dir);
        if tile == INVALID
            || (self.leaves.is_railway)(Handle::NONE, tile) == 0
            || (self.leaves.has_signals)(Handle::NONE, tile) == 0
        {
            return 1;
        }
        let tracks = (self.leaves.diag_reaches_tracks)(id, dir) as u8
            & (self.leaves.track_bits)(Handle::NONE, tile) as u8;
        if tracks != 0 && (self.leaves.has_signal)(id, tile, tracks.trailing_zeros() as u8) != 0 {
            2
        } else {
            1
        }
    }
    fn reverse_command(&self, id: Handle, execute: bool, single: bool) -> u8 {
        if single {
            if (self.leaves.read_multiheaded)(id) != 0
                || (self.leaves.reverse_single_blocked)(id) != 0
            {
                return 1;
            }
            let first = (self.leaves.read_first)(id);
            if (self.leaves.stopped_in_depot)(first) == 0 {
                return 2;
            }
            if execute {
                id.set_flag(4, !id.flag(4));
                self.consist_changed(first, 3);
                (self.leaves.reverse_windows)(first);
            }
        } else {
            let v = (self.leaves.read_reverse_command)(id);
            if v.front == 0 || v.status & 128 != 0 || v.breakdown != 0 {
                return 3;
            }
            if execute {
                if (self.leaves.read_order)(id) == 3 {
                    let last = (self.leaves.read_tile)((self.leaves.read_last)(id));
                    if (self.leaves.is_station_any)(Handle::NONE, last) == 0
                        || (self.leaves.station)(Handle::NONE, last)
                            != (self.leaves.station)(Handle::NONE, (self.leaves.read_tile)(id))
                    {
                        (self.leaves.leave_station)(id);
                    }
                }
                id.set_force_proceed(0);
                (self.leaves.view_window)(id);
                if (self.leaves.acc_model)(id) != 0 && (self.leaves.read_speed)(id) != 0 {
                    id.set_flag(0, !id.flag(0));
                } else {
                    (self.leaves.write_speed)(id, 0_u16);
                    (self.leaves.last_speed)(id);
                    (self.leaves.hide_fill)(id);
                    self.reverse(id);
                }
                (self.leaves.reset_unbunch)(id);
            }
        }
        0
    }
    fn force_command(&self, id: Handle, execute: bool) -> bool {
        if (self.leaves.read_front)(id) == 0 {
            return false;
        }
        if execute {
            id.set_force_proceed(u64::from(self.next_force(id)));
            (self.leaves.view_window)(id);
            (self.leaves.reset_unbunch)(id);
        }
        true
    }
}

/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_consist_changed(
    id: Handle,
    allowed: u8,
    leaves: *const Leaves,
    services: *const Services,
) {
    let g = Game {
        // SAFETY: The immutable callback table is live for this call.
        leaves: unsafe { &*leaves },
        // SAFETY: The immutable services table is live for this call.
        services: unsafe { &*services },
    };
    g.consist_changed(id, allowed);
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_curve_limit(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) -> u16 {
    let g = Game {
        // SAFETY: The immutable callback table is live for this call.
        leaves: unsafe { &*leaves },
        // SAFETY: The immutable services table is live for this call.
        services: unsafe { &*services },
    };
    g.curve_limit(id)
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_current_max_speed(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) -> i32 {
    let g = Game {
        // SAFETY: The immutable callback table is live for this call.
        leaves: unsafe { &*leaves },
        // SAFETY: The immutable services table is live for this call.
        services: unsafe { &*services },
    };
    g.current_max_speed(id)
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_update_acceleration(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) {
    let g = Game {
        // SAFETY: The immutable callback table is live for this call.
        leaves: unsafe { &*leaves },
        // SAFETY: The immutable services table is live for this call.
        services: unsafe { &*services },
    };
    g.update_acceleration(id);
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_update_speed(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) -> i32 {
    let g = Game {
        // SAFETY: The immutable callback table is live for this call.
        leaves: unsafe { &*leaves },
        // SAFETY: The immutable services table is live for this call.
        services: unsafe { &*services },
    };
    g.update_speed(id)
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_mark_dirty(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) {
    let g = Game {
        // SAFETY: The immutable callback table is live for this call.
        leaves: unsafe { &*leaves },
        // SAFETY: The immutable services table is live for this call.
        services: unsafe { &*services },
    };
    g.mark_dirty(id);
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_tick(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) -> u8 {
    let g = Game {
        // SAFETY: The immutable callback table is live for this call.
        leaves: unsafe { &*leaves },
        // SAFETY: The immutable services table is live for this call.
        services: unsafe { &*services },
    };
    u8::from(g.tick(id))
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_calendar_day(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) {
    let g = Game {
        // SAFETY: The immutable callback table is live for this call.
        leaves: unsafe { &*leaves },
        // SAFETY: The immutable services table is live for this call.
        services: unsafe { &*services },
    };
    (g.leaves.age)(id);
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_economy_day(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) {
    let g = Game {
        // SAFETY: The immutable callback table is live for this call.
        leaves: unsafe { &*leaves },
        // SAFETY: The immutable services table is live for this call.
        services: unsafe { &*services },
    };
    g.economy_day(id);
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_running_cost(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) -> i64 {
    let g = Game {
        // SAFETY: The immutable callback table is live for this call.
        leaves: unsafe { &*leaves },
        // SAFETY: The immutable services table is live for this call.
        services: unsafe { &*services },
    };
    g.running_cost(id)
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_trackdir(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) -> u8 {
    let g = Game {
        // SAFETY: The immutable callback table is live for this call.
        leaves: unsafe { &*leaves },
        // SAFETY: The immutable services table is live for this call.
        services: unsafe { &*services },
    };
    g.trackdir(id)
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_mark_stuck(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) {
    let g = Game {
        // SAFETY: The immutable callback table is live for this call.
        leaves: unsafe { &*leaves },
        // SAFETY: The immutable services table is live for this call.
        services: unsafe { &*services },
    };
    g.mark_stuck(id);
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reverse_swap(
    id: Handle,
    left: i32,
    right: i32,
    leaves: *const Leaves,
    services: *const Services,
) {
    let g = Game {
        // SAFETY: The immutable callback table is live for this call.
        leaves: unsafe { &*leaves },
        // SAFETY: The immutable services table is live for this call.
        services: unsafe { &*services },
    };
    g.reverse_swap(id, left, right);
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_train_on_tile(
    tile: u32,
    leaves: *const Leaves,
    services: *const Services,
) -> u8 {
    let g = Game {
        // SAFETY: The immutable callback table is live for this call.
        leaves: unsafe { &*leaves },
        // SAFETY: The immutable services table is live for this call.
        services: unsafe { &*services },
    };
    u8::from(g.train_on_tile(tile))
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_update_crossing(
    tile: u32,
    sound: u8,
    force_bar: u8,
    leaves: *const Leaves,
    services: *const Services,
) {
    let g = Game {
        // SAFETY: The immutable callback table is live for this call.
        leaves: unsafe { &*leaves },
        // SAFETY: The immutable services table is live for this call.
        services: unsafe { &*services },
    };
    g.update_crossing(tile, sound != 0, force_bar != 0);
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_adjacent_crossing_dirty(
    tile: u32,
    axis: u8,
    leaves: *const Leaves,
    services: *const Services,
) {
    let g = Game {
        // SAFETY: The immutable callback table is live for this call.
        leaves: unsafe { &*leaves },
        // SAFETY: The immutable services table is live for this call.
        services: unsafe { &*services },
    };
    g.adjacent_crossing_dirty(tile, u64::from(axis));
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_crossing_removed(
    tile: u32,
    axis: u8,
    leaves: *const Leaves,
    services: *const Services,
) {
    let g = Game {
        // SAFETY: The immutable callback table is live for this call.
        leaves: unsafe { &*leaves },
        // SAFETY: The immutable services table is live for this call.
        services: unsafe { &*services },
    };
    g.crossing_removed(tile, u64::from(axis));
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reverse(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) {
    let g = Game {
        // SAFETY: The immutable callback table is live for this call.
        leaves: unsafe { &*leaves },
        // SAFETY: The immutable services table is live for this call.
        services: unsafe { &*services },
    };
    g.reverse(id);
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_controller(
    id: Handle,
    nomove: Handle,
    reverse: u8,
    leaves: *const Leaves,
    services: *const Services,
) -> u8 {
    let g = Game {
        // SAFETY: The immutable callback table is live for this call.
        leaves: unsafe { &*leaves },
        // SAFETY: The immutable services table is live for this call.
        services: unsafe { &*services },
    };
    u8::from(g.controller(id, nomove, reverse != 0))
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_crash(
    id: Handle,
    flooded: u8,
    leaves: *const Leaves,
    services: *const Services,
) -> u32 {
    let g = Game {
        // SAFETY: The immutable callback table is live for this call.
        leaves: unsafe { &*leaves },
        // SAFETY: The immutable services table is live for this call.
        services: unsafe { &*services },
    };
    g.crash(id, flooded != 0)
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_needs_service(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) {
    let g = Game {
        // SAFETY: The immutable callback table is live for this call.
        leaves: unsafe { &*leaves },
        // SAFETY: The immutable services table is live for this call.
        services: unsafe { &*services },
    };
    g.needs_service(id);
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reverse_command(
    id: Handle,
    execute: u8,
    single: u8,
    leaves: *const Leaves,
    services: *const Services,
) -> u8 {
    let g = Game {
        // SAFETY: The immutable callback table is live for this call.
        leaves: unsafe { &*leaves },
        // SAFETY: The immutable services table is live for this call.
        services: unsafe { &*services },
    };
    g.reverse_command(id, execute != 0, single != 0)
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_force_command(
    id: Handle,
    execute: u8,
    leaves: *const Leaves,
    services: *const Services,
) -> u8 {
    let g = Game {
        // SAFETY: The immutable callback table is live for this call.
        leaves: unsafe { &*leaves },
        // SAFETY: The immutable services table is live for this call.
        services: unsafe { &*services },
    };
    u8::from(g.force_command(id, execute != 0))
}
