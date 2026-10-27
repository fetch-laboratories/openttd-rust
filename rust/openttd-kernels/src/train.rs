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
use crate::order_type::{
    OT_CONDITIONAL, OT_GOTO_DEPOT, OT_GOTO_STATION, OT_GOTO_WAYPOINT, OT_LEAVESTATION, OT_LOADING,
    OT_NOTHING,
};
use crate::raw_field::{read_field, write_field};
use crate::services::Services;
use crate::tile_map::is_tile_type;
use crate::tile_type::{MP_RAILWAY, MP_TUNNELBRIDGE};
use crate::train_state::State;
use crate::vehicle::{VehicleRef, VehicleServices, maintenance};
/// Native identity and canonical scalar owner, resolved together once per entry or link.
/// Neither pointer permits a Rust borrow to survive a callback. Deletion invalidates both.
/// Each entry keeps `owner` live and serialized for the `raw_field` accesses below.
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
    /// The base `Vehicle` of this live `Train` shell: `Vehicle` is the primary,
    /// non-virtual base of `Train`, so the native upcast does not adjust it.
    pub(crate) fn vehicle(self) -> VehicleRef {
        // SAFETY: Callers pass the live serialized train of this entry.
        VehicleRef(unsafe { std::ptr::NonNull::new_unchecked(self.shell.cast()) })
    }
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
        u64::from(read_field!(self.owner, flags))
    }
    pub(crate) fn set_flags(self, value: u64) {
        write_field!(self.owner, flags, value as u16);
    }
    pub(crate) fn get_crash_anim_pos(self) -> u64 {
        u64::from(read_field!(self.owner, crash_anim_pos))
    }
    pub(crate) fn set_crash_anim_pos(self, value: u64) {
        write_field!(self.owner, crash_anim_pos, value as u16);
    }
    pub(crate) fn get_wait_counter(self) -> u64 {
        u64::from(read_field!(self.owner, wait_counter))
    }
    pub(crate) fn set_wait_counter(self, value: u64) {
        write_field!(self.owner, wait_counter, value as u16);
    }
    pub(crate) fn get_compatible_railtypes(self) -> u64 {
        read_field!(self.owner, compatible_railtypes)
    }
    pub(crate) fn set_compatible_railtypes(self, value: u64) {
        write_field!(self.owner, compatible_railtypes, value);
    }
    pub(crate) fn get_railtypes(self) -> u64 {
        read_field!(self.owner, railtypes)
    }
    pub(crate) fn set_railtypes(self, value: u64) {
        write_field!(self.owner, railtypes, value);
    }
    pub(crate) fn get_track(self) -> u64 {
        u64::from(read_field!(self.owner, track))
    }
    pub(crate) fn set_track(self, value: u64) {
        write_field!(self.owner, track, value as u8);
    }
    pub(crate) fn get_force_proceed(self) -> u64 {
        u64::from(read_field!(self.owner, force_proceed))
    }
    pub(crate) fn set_force_proceed(self, value: u64) {
        write_field!(self.owner, force_proceed, value as u8);
    }
    pub(crate) fn get_cached_tilt(self) -> u64 {
        u64::from(read_field!(self.owner, cached_tilt))
    }
    pub(crate) fn set_cached_tilt(self, value: u64) {
        write_field!(self.owner, cached_tilt, value != 0);
    }
    pub(crate) fn get_user_def_data(self) -> u64 {
        u64::from(read_field!(self.owner, user_def_data))
    }
    pub(crate) fn set_user_def_data(self, value: u64) {
        write_field!(self.owner, user_def_data, value as u8);
    }
    pub(crate) fn get_cached_curve_speed_mod(self) -> u64 {
        u64::from(read_field!(self.owner, cached_curve_speed_mod) as u16)
    }
    pub(crate) fn set_cached_curve_speed_mod(self, value: u64) {
        write_field!(self.owner, cached_curve_speed_mod, value as i16);
    }
    pub(crate) fn get_cached_max_curve_speed(self) -> u64 {
        u64::from(read_field!(self.owner, cached_max_curve_speed))
    }
    pub(crate) fn set_cached_max_curve_speed(self, value: u64) {
        write_field!(self.owner, cached_max_curve_speed, value as u16);
    }
    pub(crate) fn set_flag(self, bit: u8, value: bool) {
        let mask = 1 << bit;
        let old = self.get_flags();
        self.set_flags(if value { old | mask } else { old & !mask });
    }
}
/// Native `VehicleRailFlag::Flipped`.
pub(crate) const FLIPPED: u8 = 4;
/// `CCF_ARRANGE`: `ConsistChangeFlag::Length` and `ConsistChangeFlag::Capacity`.
pub(crate) const CCF_ARRANGE: u8 = 3;
use crate::tile_type::INVALID_TILE;
use crate::track_type::{TRACK_BIT_DEPOT as DEPOT, TRACK_BIT_WORMHOLE as WORMHOLE};
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
    pub physics: &'static crate::ground_vehicle::Services,
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

    pub acc_model: extern "C" fn(Handle) -> u64,
    pub advance_distance: extern "C" fn(Handle) -> u64,
    pub all_powered: extern "C" fn(Handle, u64) -> u64,
    pub ambient_sound: extern "C" fn(Handle) -> u64,
    pub arrival_news: extern "C" fn(Handle, u16),

    pub axis_diag: extern "C" fn(Handle, u8) -> u64,
    pub backoff: extern "C" fn() -> u8,
    pub base_viewport: extern "C" fn(Handle, u8),
    pub bridge_speed: extern "C" fn(Handle, u32) -> u64,
    pub(crate) capabilities: &'static crate::engine_capabilities::Services,
    pub capacity_error: extern "C" fn(Handle),
    pub cargo_age_default: extern "C" fn(Handle) -> u64,
    pub check_next: extern "C" fn(Handle),
    pub check_orders: extern "C" fn(Handle),
    pub check_reverse: extern "C" fn(Handle) -> u64,
    pub choose_track: extern "C" fn(Handle, u32, u8, u8) -> u64,
    pub clear_reservation: extern "C" fn(Handle, u32, u8),
    pub consist_windows: extern "C" fn(Handle),
    pub cost_class: extern "C" fn(Handle) -> u64,
    pub cost_default: extern "C" fn(Handle) -> u64,
    pub cost_divisor: extern "C" fn(Handle) -> u64,
    pub count_chain: extern "C" fn(Handle) -> u64,
    pub crash_event: extern "C" fn(Handle, u32),
    pub crash_news: extern "C" fn(Handle, u32),
    pub crash_rating: extern "C" fn(Handle),
    pub crash_sound: extern "C" fn(Handle),
    pub crossing_barred: extern "C" fn(Handle, u32) -> u64,
    pub crossing_rail_axis: extern "C" fn(Handle, u32) -> u64,
    pub crossing_road_axis: extern "C" fn(Handle, u32) -> u64,
    pub crossing_sound: extern "C" fn(Handle, u32),
    pub curve_advantage: extern "C" fn(Handle) -> u64,
    pub curve_mod_default: extern "C" fn(Handle) -> i16,
    pub day_ticks: extern "C" fn(Handle) -> u64,
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
    pub disconnect: extern "C" fn(Handle),
    pub engine_power: extern "C" fn(Handle) -> u64,
    pub find_depot: extern "C" fn(Handle, i32) -> Depot,
    pub first_track: extern "C" fn(Handle, u8) -> u64,
    pub grf_version: extern "C" fn(Handle) -> u64,
    pub has_depot_res: extern "C" fn(Handle, u32) -> u64,
    pub has_signal: extern "C" fn(Handle, u32, u8) -> u64,
    pub has_signals: extern "C" fn(Handle, u32) -> u64,
    pub has_signal_td: extern "C" fn(Handle, u32, u8) -> u64,
    pub hide_fill: extern "C" fn(Handle),
    pub invalidate_grf: extern "C" fn(Handle),
    pub invalid_price: extern "C" fn(Handle) -> u64,
    pub is_bridge: extern "C" fn(Handle, u32) -> u64,
    pub is_crossing: extern "C" fn(Handle, u32) -> u64,
    pub is_depot: extern "C" fn(Handle, u32) -> u64,
    pub is_plain_rail: extern "C" fn(Handle, u32) -> u64,
    pub is_station: extern "C" fn(Handle, u32) -> u64,
    pub is_station_any: extern "C" fn(Handle, u32) -> u64,
    pub leave_sound: extern "C" fn(Handle),
    pub leave_unbunch: extern "C" fn(Handle),
    pub length_callback: extern "C" fn(Handle) -> u64,
    pub length_default: extern "C" fn(Handle) -> u64,
    pub local_company: extern "C" fn(Handle) -> u64,
    pub lost_warn: extern "C" fn(Handle) -> u64,
    pub map_size: extern "C" fn(Handle) -> u64,
    pub max_depot_penalty: extern "C" fn(Handle) -> u64,
    pub no_90: extern "C" fn(Handle, u8, u8) -> u64,
    pub oneway_blocking: extern "C" fn(Handle, u32, u8) -> u64,
    pub order_depot_service: extern "C" fn(Handle, u16),
    pub order_dummy: extern "C" fn(Handle),
    pub order_free: extern "C" fn(Handle),
    pub order_max_speed: extern "C" fn(Handle) -> u64,
    pub other_end: extern "C" fn(Handle, u32) -> u64,
    pub pay_running: extern "C" fn(Handle, i64),
    pub pbs_signal_type: extern "C" fn(Handle) -> u64,
    pub platform_ahead: extern "C" fn(Handle, u16, u32) -> u64,
    pub platform_length: extern "C" fn(Handle, u16, u32) -> u64,
    pub pow_wag_power: extern "C" fn(Handle) -> u64,
    pub price: extern "C" fn(Handle, u32) -> u64,
    pub railveh_wagon: extern "C" fn(Handle) -> u64,
    pub rail_tilt: extern "C" fn(Handle) -> u64,
    pub rail_type: extern "C" fn(Handle, u32) -> u64,
    pub rail_types: extern "C" fn(Handle) -> u64,
    pub reserve_paths: extern "C" fn() -> bool,
    pub reserve_under: extern "C" fn(Handle),
    pub reset_unbunch: extern "C" fn(Handle),
    pub reverse_at_signals: extern "C" fn(Handle) -> u64,
    pub reverse_single_blocked: extern "C" fn(Handle) -> u64,
    pub reverse_windows: extern "C" fn(Handle),
    pub running_windows: extern "C" fn(Handle),
    pub servint: extern "C" fn(Handle) -> u64,
    pub set_depot_res: extern "C" fn(Handle, u32, u8),
    pub set_next: extern "C" fn(Handle, Handle),
    pub set_signal_state: extern "C" fn(Handle, u32, u8, u8),
    pub set_tunnel_res: extern "C" fn(Handle, u32, u8),
    pub show_reservation: extern "C" fn(Handle) -> u64,
    pub signal_context: extern "C" fn() -> crate::signal::Context,
    pub signal_has_pbs: extern "C" fn(Handle, u32, u8) -> u64,
    pub signal_pbs: extern "C" fn(Handle, u8) -> u64,
    pub signal_type: extern "C" fn(Handle, u32, u8) -> u64,
    pub sigseg_full: extern "C" fn(Handle) -> u64,
    pub sigseg_pbs: extern "C" fn(Handle) -> u64,
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
    pub tile_add_diag: extern "C" fn(Handle, u32, u8) -> u64,
    pub tile_offset_axis: extern "C" fn(Handle, u8) -> u64,
    pub tile_offset_diag: extern "C" fn(Handle, u8) -> u64,
    pub tile_owner: extern "C" fn(Handle, u32) -> u64,
    pub tile_rail_type: extern "C" fn(Handle, u32) -> u64,
    pub tile_virt: extern "C" fn(i32, i32) -> u32,
    pub trackdir_exit: extern "C" fn(Handle, u8) -> u64,
    pub trackdir_reaches: extern "C" fn(Handle, u8) -> u64,
    pub track_bits: extern "C" fn(Handle, u32) -> u64,
    pub track_crosses: extern "C" fn(Handle, u8) -> u64,
    pub track_direction: extern "C" fn(Handle, u8, u8) -> u64,
    pub train_list: extern "C" fn(Handle),
    pub train_visit: extern "C" fn(Handle) -> u64,
    pub truncate_cargo: extern "C" fn(Handle, u32),
    pub tunnel_dir: extern "C" fn(Handle, u32) -> u64,
    pub update_delta: extern "C" fn(Handle),
    pub user_default: extern "C" fn(Handle) -> u64,
    pub veh_exit_dir: extern "C" fn(Handle, u8, u8) -> u64,
    pub viewport: extern "C" fn(Handle, u8, u8),
    pub visit_type: extern "C" fn(Handle, u16) -> u64,
    pub wagon_override: extern "C" fn(Handle) -> u64,
    pub wagon_speed_limits: extern "C" fn(Handle) -> u64,
    pub wait_oneway: extern "C" fn(Handle) -> u64,
    pub wait_pbs: extern "C" fn(Handle) -> u64,
    pub wait_twoway: extern "C" fn(Handle) -> u64,
    pub wait_unbunch: extern "C" fn(Handle) -> u64,
    pub write_crossing_bar: extern "C" fn(Handle, u32, u8),
    pub write_visit_type: extern "C" fn(Handle, u16),

    /// Base vehicle table, immutable for the process.
    pub vehicle: *const VehicleServices,
}
/// The `Game` view of an entry's `leaves` and `services` tables.
macro_rules! entry_game {
    ($leaves:expr, $services:expr) => {
        // SAFETY: The entry's contract keeps both immutable tables live until it returns.
        Game::new(unsafe { &*$leaves }, unsafe { &*$services })
    };
}
#[derive(Clone)]
struct Game<'a> {
    leaves: &'a Leaves,
    services: &'a Services,
    movement: crate::movement::CachedTable<'a>,
}
impl<'a> Game<'a> {
    fn new(leaves: &'a Leaves, services: &'a Services) -> Self {
        Self {
            leaves,
            services,
            movement: crate::movement::CachedTable::new(services),
        }
    }
    fn vs(&self) -> &VehicleServices {
        // SAFETY: The native table names the immutable process-wide base table.
        unsafe { &*self.leaves.vehicle }
    }
    /// Run one `vehicle::maintenance` entry on this entry's shell.
    /// # Safety
    /// The caller must satisfy `entry`'s preconditions for this shell and table.
    unsafe fn maintain<R>(
        &self,
        id: Handle,
        entry: unsafe fn(VehicleRef, &VehicleServices) -> R,
    ) -> R {
        // SAFETY: The caller meets the maintenance entry contract.
        unsafe { self.vs().maintain(id.vehicle(), entry) }
    }
    /// `TryPathReserve`.
    fn try_path_reserve(&self, id: Handle, mark_as_reserved: bool, first_tile_okay: bool) -> bool {
        // SAFETY: id.shell is the live Train for this serialized entry; no owner borrow spans the call.
        unsafe {
            (self.services.try_path_reserve)(id.shell.cast(), mark_as_reserved, first_tile_okay)
        }
    }
    /// `FreeTrainTrackReservation`.
    fn free_train_reservation(&self, id: Handle) {
        // SAFETY: id.shell is the live Train for this serialized entry; no owner borrow spans the call.
        unsafe { (self.services.free_train_reservation)(id.shell.cast()) };
    }
    /// `UpdateVehicleTileHash` of this shell, which stays in place.
    fn update_occupancy(&self, id: Handle) {
        // SAFETY: The controller retains this live shell through the index update.
        unsafe { crate::vehicle::occupancy::update(self.vs(), id.vehicle(), false) };
    }
    fn handle_breakdown(&self, id: Handle) -> bool {
        // SAFETY: The live train and unborrowed owner satisfy `vehicle_handle_breakdown`.
        unsafe { self.maintain(id, maintenance::vehicle_handle_breakdown) }
    }
    fn needs_automatic_servicing(&self, id: Handle) -> bool {
        // SAFETY: The live train and unborrowed owner satisfy `needs_automatic_servicing`.
        unsafe { self.maintain(id, maintenance::needs_automatic_servicing) }
    }
    fn service_in_depot(&self, id: Handle) {
        // SAFETY: The live train and unborrowed owner satisfy `service_in_depot`.
        unsafe { self.maintain(id, maintenance::service_in_depot) };
    }
    /// `SetWindowDirty(WC_VEHICLE_VIEW, v->index)`.
    fn view_dirty(&self, id: Handle) {
        // SAFETY: The base table names its immutable maintenance table.
        (unsafe { &*self.vs().maintenance }.view_dirty)(id.vehicle());
    }
    fn visit_tile<F: FnMut(Handle) -> bool>(&self, tile: u32, mut visitor: F) -> bool {
        let vs = self.vs();
        vs.any_on_tile(tile, |v| {
            (vs.vehicle_type)(v) == crate::vehicle::VehicleType::TRAIN.0
                && visitor((vs.train_handle)(v.0.as_ptr()))
        })
    }
    fn visit_near<F: FnMut(Handle) -> bool>(&self, x: i32, y: i32, mut visitor: F) -> bool {
        let vs = self.vs();
        vs.any_near(x, y, 7, |v| {
            (vs.vehicle_type)(v) == crate::vehicle::VehicleType::TRAIN.0
                && visitor((vs.train_handle)(v.0.as_ptr()))
        })
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
impl Game<'_> {
    fn property(&self, id: Handle, prop: u32, default: u64) -> u64 {
        (self.vs().property)(
            (self.vs().newgrf)(),
            id.vehicle(),
            prop as u8,
            default as i32,
        ) as u64
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
            u.set_user_def_data(self.property(
                u,
                u32::from(crate::newgrf_properties::PROP_TRAIN_USER_DATA),
                u.get_user_def_data(),
            ));
            (self.leaves.invalidate_grf)(id);
            (self.leaves.invalidate_grf)(u);
            u = (self.leaves.read_next)(u);
        }
        u = id;
        while u != Handle::NONE {
            if (self.leaves.rail_tilt)(u) == 0 {
                tilt = false;
            }
            // SAFETY: the current shell survives property evaluation; no owner borrow is held.
            min_curve_mod = min_curve_mod.min(unsafe {
                crate::newgrf_vehicle::signed_property(
                    self.vs(),
                    u.vehicle(),
                    crate::newgrf_properties::PROP_TRAIN_CURVE_SPEED_MOD,
                    i32::from((self.leaves.curve_mod_default)(u)),
                )
            } as i16);
            // SAFETY: live serialized shell; evaluation retains no owner reference.
            unsafe { crate::newgrf_vehicle::cache_override(self.vs(), u.vehicle()) };
            (self.leaves.write_colourmap)(u, 0);
            crate::vehicle_effect::openttd_rust_visual_update(u.vehicle(), true);
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
                    let speed = self.property(
                        u,
                        u32::from(crate::newgrf_properties::PROP_TRAIN_SPEED),
                        (self.leaves.speed_default)(u),
                    ) as u16;
                    if speed != 0 {
                        max_speed = max_speed.min(speed);
                    }
                }
            }
            // SAFETY: the current consist retains this shell during capacity callbacks.
            let cap = unsafe {
                crate::engine_capabilities::openttd_rust_engine_capacity(
                    self.leaves.capabilities,
                    (self.vs().engine)(u.vehicle()).0.as_ptr(),
                    u.shell.cast(),
                    std::ptr::null_mut(),
                )
            } as u16;
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
                    u32::from(crate::newgrf_properties::PROP_TRAIN_CARGO_AGE_PERIOD),
                    (self.leaves.cargo_age_default)(u),
                )) as u16,
            );
            let mut length = u16::MAX;
            if (self.leaves.grf_version)(u) >= 8 {
                length = self.property(
                    u,
                    u32::from(crate::newgrf_properties::PROP_TRAIN_SHORTEN_FACTOR),
                    u64::from(u16::MAX),
                ) as u16;
                if length != u16::MAX && length >= 8 {
                    let vs = self.vs();
                    (vs.unknown_callback_result)(
                        (vs.engine)(u.vehicle()),
                        crate::newgrf_callbacks::CBID_VEHICLE_LENGTH,
                        length,
                    );
                }
            } else if (self.leaves.length_callback)(u) != 0 {
                length = (self.vs().callback)(
                    (self.vs().newgrf)(),
                    u.vehicle(),
                    crate::newgrf_callbacks::CBID_VEHICLE_LENGTH,
                );
            }
            if length == u16::MAX {
                length = (self.leaves.length_default)(u) as u16;
            }
            length = 8 - length.min(7);
            if allowed & 1 != 0 {
                (self.leaves.write_length)(u, (u64::from(length)) as u8);
            } else if length != (self.leaves.read_length)(u) {
                (self.vs().length_changed)(u.vehicle());
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
        crate::ground_vehicle::cargo_changed(self.leaves.physics, id.vehicle());
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
        } else if v.order == OT_GOTO_STATION && v.order_destination == station {
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
            if crate::movement::should_stop(self.services, id.vehicle(), station) {
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
            if model == 1 && u.get_track() == u64::from(DEPOT) {
                max_speed = max_speed.min(61);
                break;
            }
            if u.get_track() == u64::from(WORMHOLE) && (self.leaves.read_status)(u) & 1 == 0 {
                let tile = (self.leaves.read_tile)(u);
                max_speed = max_speed.min((self.leaves.bridge_speed)(u, tile) as i32);
            }
            u = (self.leaves.read_next)(u);
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
            crate::ground_vehicle::update_speed(
                self.leaves.physics,
                id.vehicle(),
                (i64::from(i32::from(v.acceleration) * if braking { -4 } else { 2 }) as u64) as u32,
                0,
                (max as u64) as i32,
            )
            .distance as i32
        } else {
            crate::ground_vehicle::update_speed(
                self.leaves.physics,
                id.vehicle(),
                crate::ground_vehicle::acceleration(self.leaves.physics, id.vehicle()) as u32,
                (u64::from(!braking) * 2) as i32,
                (max as u64) as i32,
            )
            .distance as i32
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
        crate::ground_vehicle::cargo_changed(self.leaves.physics, id.vehicle());
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
                    u32::from(crate::newgrf_properties::PROP_TRAIN_RUNNING_COST_FACTOR),
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
        crate::tile_map::is_tile_owner(self.services, tile, (self.leaves.read_owner)(id))
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
        if is_tile_type(self.services, v.tile, MP_TUNNELBRIDGE)
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
            return INVALID_TILE;
        }
        let base = (self.leaves.read_tile)(id);
        let dir = self.exitdir(id);
        let tile = base.wrapping_add((self.leaves.tile_offset_diag)(id, dir) as u32);
        if (self.leaves.is_crossing)(Handle::NONE, tile) == 0
            || (self.leaves.diag_axis)(id, dir)
                == (self.leaves.crossing_road_axis)(Handle::NONE, tile)
            || !self.compatible(id, tile)
        {
            return INVALID_TILE;
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
        (self.services.crossing_reserved)(tile)
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
            (self.services.set_crossing_reserved)(tile, true);
            self.update_crossing(tile, true, false);
        }
    }
    fn mark_stuck(&self, id: Handle) {
        if !id.flag(8) {
            id.set_flag(8, true);
            id.set_wait_counter(0);
            (self.leaves.write_speed)(id, 0_u16);
            (self.leaves.write_subspeed)(id, 0_u8);
            crate::ground_vehicle::last_speed(self.leaves.physics, id.vehicle());
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
            self.movement_enter(id, v.tile, (v.x as u64) as i32, (v.y as u64) as i32);
        } else {
            let vt = (self.leaves.tile_virt)((v.x as u64) as i32, (v.y as u64) as i32);
            if is_tile_type(self.services, vt, MP_TUNNELBRIDGE) {
                self.movement_enter(id, vt, (v.x as u64) as i32, (v.y as u64) as i32);
                if id.track() != WORMHOLE
                    && (self.leaves.is_bridge)(Handle::NONE, (self.leaves.read_tile)(id)) != 0
                {
                    self.update_occupancy(id);
                    crate::ground_vehicle::inclination(
                        self.leaves.physics,
                        &self.movement,
                        id.vehicle(),
                        true,
                        true,
                    );
                    return;
                }
            }
        }
        self.update_occupancy(id);
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
            let d = crate::movement::depot_ticks(self.services, dep);
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
            self.free_train_reservation(id);
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
        if crossing != INVALID_TILE {
            self.update_crossing(crossing, true, false);
        }
        let crossing = self.approaching_crossing(id);
        if crossing != INVALID_TILE {
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
            || is_tile_type(self.services, tile, MP_TUNNELBRIDGE)
        {
            dir = u8::MAX;
        }
        if self.signal_segment(id, tile, dir) == (self.leaves.sigseg_pbs)(id)
            || (self.leaves.reserve_paths)()
        {
            let td = self.trackdir(id);
            let mut okay = !(is_tile_type(self.services, tile, MP_RAILWAY)
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
                crate::pbs::Pbs::acquire(self.services).platform(
                    tile,
                    (self.leaves.trackdir_exit)(id, td) as u8,
                    true,
                );
            }
            if self.try_path_reserve(id, false, okay) {
                (self.leaves.check_next)(id);
            } else if (self.leaves.read_order)(id) != OT_LOADING {
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
        let ts = self.movement_tracks(tile, dir ^ 2) as u32;
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
        self.view_dirty(id);
        crate::vehicle::visits::begin_loading(id.vehicle(), self.vs().visits());
        crate::animated_tile::rail_event(
            crate::animated_tile::station_base(station),
            (self.vs().tile)(id.vehicle()),
            crate::station_type::SAT_VEHICLE_ARRIVES,
            crate::station_type::SRT_VEHICLE_ARRIVES,
        );
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
        if is_tile_type(self.services, tile, MP_RAILWAY)
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
        let new = (self.leaves.tile_virt)((x as u64) as i32, (y as u64) as i32);
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
                        self.movement_enter(id, new, (x as u64) as i32, (y as u64) as i32) as u8;
                    if vets & 4 != 0 {
                        return (2, false);
                    }
                    if vets & 1 != 0 {
                        self.enter_station(id, (self.leaves.station)(Handle::NONE, new) as u16);
                    }
                }
            } else {
                entered = (self.leaves.diag_between)(id, old, new) as u8;
                let ts = self.movement_tracks(new, entered ^ 2) as u32;
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
                            self.view_dirty(id);
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
                    crate::pbs::Pbs::acquire(self.services).try_reserve(
                        new,
                        chosen.trailing_zeros() as u8,
                        false,
                    );
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
                let vets = self.movement_enter(id, new, (x as u64) as i32, (y as u64) as i32) as u8;
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
                        let small = if u64::from(crate::ground_vehicle::acceleration_type(
                            self.leaves.physics,
                            id.vehicle(),
                        )) == 2
                        {
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
                    if crossing != INVALID_TILE
                        && (self.services.crossing_reserved)(crossing)
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
            if is_tile_type(self.services, new, MP_TUNNELBRIDGE)
                && self.movement_enter(id, new, (x as u64) as i32, (y as u64) as i32) & 2 != 0
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
                self.update_occupancy(id);
                if (self.leaves.read_status)(id) & 1 == 0 {
                    (self.leaves.base_viewport)(id, 1);
                }
                return (0, false);
            }
        }
        (self.leaves.update_delta)(id);
        (self.leaves.write_x)(id, (x as u64) as i32);
        (self.leaves.write_y)(id, (y as u64) as i32);
        self.update_occupancy(id);
        let old_z = crate::ground_vehicle::inclination(
            self.leaves.physics,
            &self.movement,
            id.vehicle(),
            new != old,
            false,
        );
        if prev == Handle::NONE {
            self.speed_z(id, old_z);
        }
        if signals {
            if (self.leaves.read_front)(id) != 0 && self.moved_signals(id, new, entered) {
                if (!crate::pbs::Pbs::acquire(self.services).has_reserved_tracks(new, id.track())
                    && self.reserve_track(id, new, u64::from(id.track().trailing_zeros())) == 0)
                    || !self.try_path_reserve(id, false, false)
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
            && u64::from((self.leaves.read_tick)(id)) % u64::from((self.leaves.backoff)()) == 0
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
    fn reserve_track(&self, _id: Handle, tile: u32, track: u64) -> u64 {
        u64::from(crate::pbs::Pbs::acquire(self.services).try_reserve(tile, track as u8, true))
    }
    fn crash(&self, id: Handle, flooded: bool) -> u32 {
        let mut victims = 0_u32;
        if (self.leaves.read_front)(id) != 0 {
            victims += 2;
            if !id.flag(8) {
                self.free_train_reservation(id);
            }
            let mut u = id;
            while u != Handle::NONE {
                let tile = (self.leaves.read_tile)(u);
                (self.leaves.clear_reservation)(u, tile, self.trackdir(u));
                if is_tile_type(self.services, tile, MP_TUNNELBRIDGE) {
                    (self.leaves.set_tunnel_res)(
                        u,
                        (self.leaves.other_end)(Handle::NONE, tile) as u32,
                        0,
                    );
                }
                u = (self.leaves.read_next)(u);
            }
            let crossing = self.approaching_crossing(id);
            if crossing != INVALID_TILE {
                self.update_crossing(crossing, true, false);
            }
            (self.leaves.hide_fill)(id);
        }
        victims = victims.wrapping_add(crate::ground_vehicle::crash(
            self.leaves.physics,
            id.vehicle(),
            flooded,
        ));
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
        if (self.services.disaster_sound)() {
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
        if crate::pbs::Pbs::acquire(self.services).has_reserved_tracks(tile, tracks) {
            crate::pbs::Pbs::acquire(self.services).unreserve(tile, track);
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
            crate::pbs::Pbs::acquire(self.services).platform(tile, dir, occupied);
            crate::pbs::Pbs::acquire(self.services).platform(tile, dir ^ 2, occupied);
        }
        if is_tile_type(self.services, tile, MP_TUNNELBRIDGE)
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
                    self.update_occupancy(id);
                    crate::ground_vehicle::inclination(
                        self.leaves.physics,
                        &self.movement,
                        id.vehicle(),
                        false,
                        true,
                    );
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
            crate::effect::openttd_rust_effect_create_rel(
                id.vehicle().0.as_ptr(),
                4,
                4,
                8,
                crate::effectvehicle_func::EV_EXPLOSION_LARGE,
            );
        }
        if state <= 200 {
            let mut r = self.services.random();
            if crate::random_func::chance16_i(1, 7, r) {
                let mut index = (r.wrapping_mul(10) >> 16) as i32;
                let mut u = id;
                loop {
                    index -= 1;
                    if index < 0 {
                        r = self.services.random();
                        crate::effect::openttd_rust_effect_create_rel(
                            u.vehicle().0.as_ptr(),
                            (((r >> 8) & 7) + 2) as i32,
                            (((r >> 16) & 7) + 2) as i32,
                            ((r & 7) + 5) as i32,
                            crate::effectvehicle_func::EV_EXPLOSION_SMALL,
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
        let segment = if (self.leaves.reserve_paths)() {
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
        if (self.leaves.read_order)(id) == OT_GOTO_DEPOT
            && (self.leaves.read_tile)(id) == (self.leaves.read_dest)(id)
        {
            if (self.leaves.has_depot_res)(Handle::NONE, (self.leaves.read_tile)(id)) == 0 {
                crate::witness::hit(crate::witness::TRAIN_DEPOT_REENTRY);
                self.vs().enter_depot(id.vehicle());
            }
            return true;
        }
        if segment == (self.leaves.sigseg_pbs)(id)
            && !self.try_path_reserve(id, false, false)
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
        self.service_in_depot(id);
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
        self.update_occupancy(id);
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
        if self.handle_breakdown(id) {
            return true;
        }
        if id.flag(0) && (self.leaves.read_speed)(id) == 0 {
            self.reverse(id);
        }
        let v = (self.leaves.read_loco)(id);
        if v.status & 2 != 0 && v.speed == 0 {
            return true;
        }
        let valid_order = v.order != OT_NOTHING && v.order != OT_CONDITIONAL;
        // SAFETY: The original live train shell survives orders; no owner borrow spans reentry.
        if unsafe {
            crate::orders::openttd_rust_process_orders(id.shell.cast(), self.vs().orders())
        } != 0
            && (self.leaves.check_reverse)(id) != 0
        {
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
                || is_tile_type(self.services, tile, MP_TUNNELBRIDGE)
            {
                dir = u8::MAX;
            }
            if self.signal_segment(id, tile, dir) == (self.leaves.sigseg_pbs)(id)
                || (self.leaves.reserve_paths)()
            {
                self.try_path_reserve(id, true, true);
            }
            id.set_flag(9, false);
        }
        self.vs().handle_loading(id.vehicle(), mode);
        if (self.leaves.read_order)(id) == OT_LOADING || self.stay_depot(id) {
            return true;
        }
        if !mode {
            if crate::vehicle_effect::openttd_rust_visual_eligible(id.vehicle()) {
                crate::vehicle_effect::openttd_rust_visual_show(
                    id.vehicle(),
                    self.current_max_speed(id) as u32,
                );
            }
        }
        if !valid_order && (self.leaves.read_order)(id) != OT_NOTHING {
            (self.leaves.check_next)(id);
        }
        if !mode && id.flag(8) {
            crate::witness::hit(crate::witness::TRAIN_STUCK_RETRY);
            let wait = u64::from(id.wait_inc());
            let turn = wait % ((self.leaves.wait_pbs)(id) * (self.leaves.day_ticks)(id)) == 0
                && (self.leaves.reverse_at_signals)(id) != 0;
            if !turn
                && wait % u64::from((self.leaves.backoff)()) != 0
                && id.get_force_proceed() == 0
            {
                return true;
            }
            if !self.try_path_reserve(id, false, false) {
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
        if (self.leaves.read_order)(id) == OT_LEAVESTATION {
            (self.leaves.order_free)(id);
            (self.leaves.start_stop_dirty)(id);
            return true;
        }
        let mut distance = self.update_speed(id);
        if (self.leaves.read_speed)(id) == 0 && (self.leaves.read_status)(id) & 2 != 0 {
            id.set_force_proceed(0);
            self.view_dirty(id);
        }
        let mut advance = (self.leaves.advance_distance)(id) as i32;
        if distance < advance {
            if (self.leaves.read_speed)(id) == 0 {
                crate::ground_vehicle::last_speed(self.leaves.physics, id.vehicle());
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
                if (v.order == OT_GOTO_WAYPOINT || v.order == OT_GOTO_STATION)
                    && v.nonstop & 2 != 0
                    && (self.leaves.is_station_any)(Handle::NONE, v.tile) != 0
                    && u64::from(v.order_destination) == (self.leaves.station)(Handle::NONE, v.tile)
                {
                    // SAFETY: The orders owner is resolved at this source call; no owner borrow remains.
                    unsafe {
                        crate::orders::openttd_rust_process_orders(
                            id.shell.cast(),
                            self.vs().orders(),
                        )
                    };
                }
            }
            crate::ground_vehicle::last_speed(self.leaves.physics, id.vehicle());
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
        if (self.leaves.servint)(id) == 0 || !self.needs_automatic_servicing(id) {
            return;
        }
        if u64::from(crate::ground_vehicle::chain_in_depot(
            self.leaves.physics,
            id.vehicle(),
        )) != 0
        {
            crate::witness::hit(crate::witness::TRAIN_DEPOT_SERVICE);
            self.service_in_depot(id);
            return;
        }
        let max = (self.leaves.max_depot_penalty)(id) as u32;
        let depot = (self.leaves.find_depot)(id, max as i32);
        let length = depot.length;
        if length == u32::MAX || length > max {
            if (self.leaves.read_order)(id) == OT_GOTO_DEPOT {
                (self.leaves.order_dummy)(id);
                (self.leaves.start_stop_dirty)(id);
            }
            return;
        }
        let tile = depot.tile;
        let depot_id = (self.leaves.depot_index)(Handle::NONE, tile) as u16;
        let v = (self.leaves.read_needs_service)(id);
        if v.order == OT_GOTO_DEPOT
            && v.order_destination != depot_id
            && !crate::random_func::chance16_i(3, 16, self.services.random())
        {
            return;
        }
        (self.leaves.suppress_implicit)(id);
        (self.leaves.order_depot_service)(id, depot_id);
        (self.leaves.write_dest)(id, (u64::from(tile)) as u32);
        (self.leaves.start_stop_dirty)(id);
    }
    fn economy_day(&self, id: Handle) {
        // SAFETY: The live train and unborrowed owner satisfy `economy_age_vehicle`.
        unsafe { self.maintain(id, maintenance::economy_age_vehicle) };
        let day = (self.leaves.read_day)(id).wrapping_add(1);
        (self.leaves.write_day)(id, (u64::from(day)) as u8);
        if day & 7 == 0 {
            // SAFETY: The live train and unborrowed owner satisfy `decrease_vehicle_value`.
            unsafe { self.maintain(id, maintenance::decrease_vehicle_value) };
        }
        if (self.leaves.read_front)(id) != 0 {
            // SAFETY: The live train and unborrowed owner satisfy `check_vehicle_breakdown`.
            unsafe { self.maintain(id, maintenance::check_vehicle_breakdown) };
            self.needs_service(id);
            (self.leaves.check_orders)(id);
            if (self.leaves.read_order)(id) == OT_GOTO_STATION {
                let tile = (self.leaves.station_dest)(id) as u32;
                if tile != INVALID_TILE {
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
            return if u64::from(crate::ground_vehicle::chain_in_depot(
                self.leaves.physics,
                id.vehicle(),
            )) != 0
            {
                1
            } else {
                2
            };
        }
        let dir = (self.leaves.trackdir_exit)(id, self.trackdir(id)) as u8;
        let tile = self.tile_diag(v.tile, dir);
        if tile == INVALID_TILE
            || !is_tile_type(self.services, tile, MP_RAILWAY)
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
                if (self.leaves.read_order)(id) == OT_LOADING {
                    let last = (self.leaves.read_tile)((self.leaves.read_last)(id));
                    if (self.leaves.is_station_any)(Handle::NONE, last) == 0
                        || (self.leaves.station)(Handle::NONE, last)
                            != (self.leaves.station)(Handle::NONE, (self.leaves.read_tile)(id))
                    {
                        crate::vehicle::visits::leave_station(id.vehicle(), self.vs().visits());
                    }
                }
                id.set_force_proceed(0);
                self.view_dirty(id);
                if (self.leaves.acc_model)(id) != 0 && (self.leaves.read_speed)(id) != 0 {
                    id.set_flag(0, !id.flag(0));
                } else {
                    (self.leaves.write_speed)(id, 0_u16);
                    crate::ground_vehicle::last_speed(self.leaves.physics, id.vehicle());
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
            self.view_dirty(id);
            (self.leaves.reset_unbunch)(id);
        }
        true
    }
}

/// `VehicleRailFlag::LeavingStation`.
pub(crate) const VRF_LEAVING_STATION: u64 = 1 << 9;
/// `VehicleRailFlag::Reversed`.
pub const VRF_REVERSED: u64 = 1 << 7;

/// The train part of `VehicleEnterDepot`: clear the depot reservation,
/// update the segment's signals and arrange the consist.
pub(crate) fn enter_depot(id: Handle, leaves: &Leaves, services: &Services) {
    let g = Game::new(leaves, services);
    let tile = (leaves.read_tile)(id);
    (leaves.set_depot_res)(id, tile, 0);
    if (leaves.show_reservation)(id) != 0 {
        (leaves.dirty_tile)(Handle::NONE, tile);
    }
    g.signal_segment(id, tile, crate::direction_type::INVALID_DIAGDIR);
    id.set_wait_counter(0);
    id.set_force_proceed(0);
    id.set_flags(id.get_flags() & !VRF_REVERSED);
    g.consist_changed(id, CCF_ARRANGE);
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
    let g = entry_game!(leaves, services);
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
    let g = entry_game!(leaves, services);
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
    let g = entry_game!(leaves, services);
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
    let g = entry_game!(leaves, services);
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
    let g = entry_game!(leaves, services);
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
    let g = entry_game!(leaves, services);
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
    let g = entry_game!(leaves, services);
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
    let g = entry_game!(leaves, services);
    // SAFETY: The live train and unborrowed owner satisfy `age_vehicle`.
    unsafe { g.maintain(id, maintenance::age_vehicle) };
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
    let g = entry_game!(leaves, services);
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
    let g = entry_game!(leaves, services);
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
    let g = entry_game!(leaves, services);
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
    let g = entry_game!(leaves, services);
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
    let g = entry_game!(leaves, services);
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
    let g = entry_game!(leaves, services);
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
    let g = entry_game!(leaves, services);
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
    let g = entry_game!(leaves, services);
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
    let g = entry_game!(leaves, services);
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
    let g = entry_game!(leaves, services);
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
    let g = entry_game!(leaves, services);
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
    let g = entry_game!(leaves, services);
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
    let g = entry_game!(leaves, services);
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
    let g = entry_game!(leaves, services);
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
    let g = entry_game!(leaves, services);
    u8::from(g.force_command(id, execute != 0))
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDTrainHandle" ["probe"] => Handle {
        shell,
        owner,
    };
    "OpenTTDTrainConsistChangedRead" ["probe"] => TrainConsistChangedRead {
        engine,
        front,
    };
    "OpenTTDTrainConsistChanged1Read" ["probe"] => TrainConsistChanged1Read {
        engine,
        engine_part,
    };
    "OpenTTDTrainConsistChanged2Read" ["probe"] => TrainConsistChanged2Read {
        cargo_cap,
    };
    "OpenTTDTrainCurveLimitRead" ["probe"] => TrainCurveLimitRead {
        next,
        direction,
    };
    "OpenTTDTrainStopLocationRead" ["probe"] => TrainStopLocationRead {
        length,
        total_length,
        order_destination,
        order,
    };
    "OpenTTDTrainCurrentMaxSpeedRead" ["probe"] => TrainCurrentMaxSpeedRead {
        tile,
        max_track_speed,
        speed,
    };
    "OpenTTDTrainUpdateAccelerationRead" ["probe"] => TrainUpdateAccelerationRead {
        power,
        weight,
    };
    "OpenTTDTrainUpdateSpeedRead" ["probe"] => TrainUpdateSpeedRead {
        status,
        acceleration,
    };
    "OpenTTDTrainTrackdirRead" ["probe"] => TrainTrackdirRead {
        tile,
        direction,
        status,
    };
    "OpenTTDTrainCanLeaveRead" ["probe"] => TrainCanLeaveRead {
        tile,
        direction,
    };
    "OpenTTDTrainCrossingApproachRead" ["probe"] => TrainCrossingApproachRead {
        status,
        front,
    };
    "OpenTTDTrainNextOffsetRead" ["probe"] => TrainNextOffsetRead {
        next,
        length,
    };
    "OpenTTDTrainAfterSwapRead" ["probe"] => TrainAfterSwapRead {
        tile,
        x,
        y,
    };
    "OpenTTDTrainReverseSwapRead" ["probe"] => TrainReverseSwapRead {
        tile,
        x,
        y,
        z,
        direction,
        status,
    };
    "OpenTTDTrainApproachingEndRead" ["probe"] => TrainApproachingEndRead {
        x,
        y,
        length,
        speed,
        direction,
    };
    "OpenTTDTrainLineEndsRead" ["probe"] => TrainLineEndsRead {
        tile,
        speed,
        breakdown,
    };
    "OpenTTDTrainSpeedZRead" ["probe"] => TrainSpeedZRead {
        z,
        max_track_speed,
        speed,
    };
    "OpenTTDTrainMoveVehicleRead" ["probe"] => TrainMoveVehicleRead {
        tile,
        x,
        y,
        direction,
        front,
        articulated,
    };
    "OpenTTDTrainMoveVehicle20Read" ["probe"] => TrainMoveVehicle20Read {
        speed,
        status,
        front,
    };
    "OpenTTDTrainCollisionOneRead" ["probe"] => TrainCollisionOneRead {
        first,
        x,
        y,
        z,
        length,
        owner,
    };
    "OpenTTDTrainCollisionOne22Read" ["probe"] => TrainCollisionOne22Read {
        x,
        y,
        z,
        length,
        owner,
    };
    "OpenTTDTrainDeleteLastRead" ["probe"] => TrainDeleteLastRead {
        tile,
        owner,
    };
    "OpenTTDTrainStayDepotRead" ["probe"] => TrainStayDepotRead {
        tile,
        power,
    };
    "OpenTTDTrainLocoRead" ["probe"] => TrainLocoRead {
        speed,
        status,
        order,
    };
    "OpenTTDTrainLoco26Read" ["probe"] => TrainLoco26Read {
        tile,
        order_destination,
        order,
        nonstop,
    };
    "OpenTTDTrainTickRead" ["probe"] => TrainTickRead {
        speed,
        status,
        running,
        front,
        free_wagon,
    };
    "OpenTTDTrainNeedsServiceRead" ["probe"] => TrainNeedsServiceRead {
        order_destination,
        order,
    };
    "OpenTTDTrainNextForceRead" ["probe"] => TrainNextForceRead {
        tile,
        status,
    };
    "OpenTTDTrainReverseCommandRead" ["probe"] => TrainReverseCommandRead {
        status,
        breakdown,
        front,
    };
    "OpenTTDTrainServices" ["probe"] => Leaves {
        read_first,
        read_next,
        read_previous,
        read_next_unit,
        read_last,
        read_tile,
        read_dest,
        read_order_time,
        read_length,
        read_total_length,
        read_max_track_speed,
        read_speed,
        read_gv_flags,
        read_refit_cap,
        read_last_station,
        read_direction,
        read_status,
        read_tick,
        read_running,
        read_day,
        read_progress,
        read_order,
        read_front,
        read_articulated,
        read_multiheaded,
        read_owner,
        read_vis_effect,
        read_consist_changed,
        read_consist_changed_1,
        read_consist_changed_2,
        read_curve_limit,
        read_stop_location,
        read_current_max_speed,
        read_update_acceleration,
        read_update_speed,
        read_trackdir,
        read_can_leave,
        read_crossing_approach,
        read_next_offset,
        read_after_swap,
        read_reverse_swap,
        read_approaching_end,
        read_line_ends,
        read_speed_z,
        read_move_vehicle,
        read_move_vehicle_20,
        read_collision_one,
        read_collision_one_22,
        read_delete_last,
        read_stay_depot,
        read_loco,
        read_loco_26,
        read_tick_state,
        read_needs_service,
        read_next_force,
        read_reverse_command,
        write_tile,
        write_dest,
        write_x,
        write_y,
        write_z,
        write_direction,
        write_speed,
        write_tick,
        write_running,
        write_day,
        write_order_time,
        write_progress,
        write_subspeed,
        write_gv_flags,
        write_acceleration,
        write_length,
        write_total_length,
        write_first_engine,
        write_max_speed,
        write_cargo_cap,
        write_refit_cap,
        write_cargo_age,
        write_last_station,
        write_colourmap,
        write_status,
        acc_model,
        advance_distance,
        all_powered,
        ambient_sound,
        arrival_news,

        axis_diag,
        backoff,
        base_viewport,
        bridge_speed,
        capabilities,
        capacity_error,
        cargo_age_default,
        check_next,
        check_orders,
        check_reverse,
        choose_track,
        clear_reservation,
        consist_windows,
        cost_class,
        cost_default,
        cost_divisor,
        count_chain,
        crash_event,
        crash_news,
        crash_rating,
        crash_sound,
        crossing_barred,
        crossing_rail_axis,
        crossing_road_axis,
        crossing_sound,
        curve_advantage,
        curve_mod_default,
        day_ticks,
        delete_vehicle,
        depot_dir,
        depot_dirty,
        depot_index,
        depot_track,
        depot_window,
        diag_axis,
        diag_between,
        diag_reaches_tracks,
        diag_trackdir,
        dirty_tile,
        dir_diag,
        disconnect,
        engine_power,
        find_depot,
        first_track,
        grf_version,
        has_depot_res,
        has_signal,
        has_signals,
        has_signal_td,
        hide_fill,
        invalidate_grf,
        invalid_price,
        is_bridge,
        is_crossing,
        is_depot,
        is_plain_rail,
        is_station,
        is_station_any,
        leave_sound,
        leave_unbunch,
        length_callback,
        length_default,
        local_company,
        lost_warn,
        map_size,
        max_depot_penalty,
        no_90,
        oneway_blocking,
        order_depot_service,
        order_dummy,
        order_free,
        order_max_speed,
        other_end,
        pay_running,
        pbs_signal_type,
        platform_ahead,
        platform_length,
        pow_wag_power,
        price,
        railveh_wagon,
        rail_tilt,
        rail_type,
        rail_types,
        reserve_paths,
        reserve_under,
        reset_unbunch,
        reverse_at_signals,
        reverse_single_blocked,
        reverse_windows,
        running_windows,
        servint,
        set_depot_res,
        set_next,
        set_signal_state,
        set_tunnel_res,
        show_reservation,
        signal_context,
        signal_has_pbs,
        signal_pbs,
        signal_type,
        sigseg_full,
        sigseg_pbs,
        speed_default,
        start_stop_dirty,
        station,
        station_axis,
        station_compatible,
        station_dest,
        stopped_in_depot,
        stop_location,
        stuck_news,
        suppress_implicit,
        tile_add_diag,
        tile_offset_axis,
        tile_offset_diag,
        tile_owner,
        tile_rail_type,
        tile_virt,
        trackdir_exit,
        trackdir_reaches,
        track_bits,
        track_crosses,
        track_direction,
        train_list,
        train_visit,
        truncate_cargo,
        tunnel_dir,
        update_delta,
        user_default,
        veh_exit_dir,
        viewport,
        visit_type,
        wagon_override,
        wagon_speed_limits,
        wait_oneway,
        wait_pbs,
        wait_twoway,
        wait_unbunch,
        write_crossing_bar,
        write_visit_type,
        vehicle,
        physics,
    };
    "OpenTTDTrainDepot" ["probe"] => Depot {
        tile,
        length,
    };
}
/// `VehicleRailFlag::PoweredWagon`, from `train.h`.
pub(crate) const FLAG_POWERED_WAGON: u64 = 1 << 3;
/// `VehicleRailFlag::Reversing`, from `train.h`.
pub(crate) const FLAG_REVERSING: u64 = 1 << 0;
/// `VehicleRailFlag::Stuck`, from `train.h`.
pub(crate) const FLAG_STUCK: u64 = 1 << 8;

/// Same owned stop-location callee for tile admission and train speed control.
pub(crate) fn stop_location(
    leaves: &Leaves,
    services: &Services,
    id: Handle,
    station: u16,
    tile: u32,
) -> (i32, i32, i32) {
    Game::new(leaves, services).stop_location(id, station, tile)
}
/// `Train::CalcNextVehicleOffset`, shared with depot movement policy.
pub(crate) fn next_offset(leaves: &Leaves, services: &Services, id: Handle) -> i32 {
    Game::new(leaves, services).next_offset(id)
}

impl Game<'_> {
    fn movement_enter(&self, id: Handle, tile: u32, x: i32, y: i32) -> u64 {
        // SAFETY: Same live train as the original synchronous native callback.
        u64::from(unsafe {
            crate::movement::openttd_rust_movement_enter(
                self.movement.get(),
                id.vehicle(),
                tile,
                x,
                y,
            )
        })
    }
    fn movement_tracks(&self, tile: u32, side: u8) -> u64 {
        u64::from(self.movement.get().track_status(tile, 0, 0, side))
    }
}

pub const VRF_POWERED_WAGON: u64 = 1 << 3;
pub const VRF_FLIPPED: u64 = 1 << 4;
