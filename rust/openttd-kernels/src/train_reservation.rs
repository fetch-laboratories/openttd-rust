/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Controller path extension, rollback, temporary-order lookahead, and reservations.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::items_after_statements
)]
use crate::train::Handle;
const INVALID: u32 = u32::MAX;
const DEPOT: u8 = 0x80;
const WORMHOLE: u8 = 0x40;
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct TrainReservationFreeRead {
    pub tile: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct TrainReservationFree1Read {
    pub tile: u32,
    pub next: Handle,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct TrainReservationNewRead {
    pub dest: u32,
    pub last_station: u16,
    pub order_index: u8,
    pub suppress: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct TrainReservationChooseRead {
    pub tile: u32,
    pub dest: u32,
    pub destination: u16,
    pub order: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct TrainReservationChoose4Read {
    pub order: u8,
    pub nearest: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct TrainReservationCheckNextRead {
    pub tile: u32,
    pub dest: u32,
    pub destination: u16,
    pub order: u8,
    pub num_orders: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Follow {
    pub old_tile: u32,
    pub new_tile: u32,
    pub skipped: i32,
    pub dirs: u16,
    pub old_td: u8,
    pub exitdir: u8,
    pub tunnel: u8,
    pub bridge: u8,
    pub station: u8,
    pub error: u8,
}
impl Default for Follow {
    fn default() -> Self {
        Self {
            old_tile: INVALID,
            new_tile: INVALID,
            skipped: 0,
            dirs: 0,
            old_td: 0xff,
            exitdir: 0xff,
            tunnel: 0,
            bridge: 0,
            station: 0,
            error: 0,
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Pbs {
    pub tile: u32,
    pub other: Handle,
    pub td: u8,
    pub okay: u8,
}
impl Default for Pbs {
    fn default() -> Self {
        Self {
            tile: INVALID,
            other: Handle::NONE,
            td: 0xff,
            okay: 0,
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Search {
    pub track: u8,
    pub tile: u32,
    pub final_dest: u32,
    pub td: u8,
    pub found: u8,
    pub okay: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Choice {
    pub track: u8,
    pub reserved: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Leaves {
    pub read_tile: extern "C" fn(Handle) -> u32,
    pub read_next: extern "C" fn(Handle) -> Handle,
    pub read_last_station: extern "C" fn(Handle) -> u16,
    pub read_direction: extern "C" fn(Handle) -> u8,
    pub read_order: extern "C" fn(Handle) -> u8,
    pub read_num_orders: extern "C" fn(Handle) -> u8,
    pub read_order_index: extern "C" fn(Handle) -> u8,
    pub read_free: extern "C" fn(Handle) -> TrainReservationFreeRead,
    pub read_free_1: extern "C" fn(Handle) -> TrainReservationFree1Read,
    pub read_new: extern "C" fn(Handle) -> TrainReservationNewRead,
    pub read_choose: extern "C" fn(Handle) -> TrainReservationChooseRead,
    pub read_choose_4: extern "C" fn(Handle) -> TrainReservationChoose4Read,
    pub read_check_next: extern "C" fn(Handle) -> TrainReservationCheckNextRead,

    pub all_compat: extern "C" fn(*mut std::ffi::c_void, Handle) -> u64,
    pub backoff: extern "C" fn(*mut std::ffi::c_void, Handle) -> u64,
    pub bits_track: extern "C" fn(*mut std::ffi::c_void, Handle, u8) -> u64,
    pub blocking: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u8) -> u64,
    pub check_reverse: extern "C" fn(*mut std::ffi::c_void, Handle) -> u64,
    pub compat_station: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u32) -> u64,
    pub conditional: extern "C" fn(*mut std::ffi::c_void, Handle, u8) -> u64,
    pub copy_order: extern "C" fn(*mut std::ffi::c_void, Handle, u8),
    pub cross_dirs: extern "C" fn(*mut std::ffi::c_void, Handle, u8) -> u64,
    pub cross_tracks: extern "C" fn(*mut std::ffi::c_void, Handle, u8) -> u64,
    pub depot_dir: extern "C" fn(*mut std::ffi::c_void, Handle, u32) -> u64,
    pub depot_reserved: extern "C" fn(*mut std::ffi::c_void, Handle, u32) -> u64,
    pub diag_reach_dirs: extern "C" fn(*mut std::ffi::c_void, Handle, u8) -> u64,
    pub diag_track: extern "C" fn(*mut std::ffi::c_void, Handle, u8) -> u64,
    pub enter_td: extern "C" fn(*mut std::ffi::c_void, Handle, u8, u8) -> u64,
    pub exit_dir: extern "C" fn(*mut std::ffi::c_void, Handle, u8) -> u64,
    pub free: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u8) -> u64,
    pub green: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u8) -> u64,
    pub has_pbs: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u8) -> u64,
    pub has_reserved: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u8) -> u64,
    pub has_signal: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u8) -> u64,
    pub increment_order: extern "C" fn(*mut std::ffi::c_void, Handle),
    pub is_bridge: extern "C" fn(*mut std::ffi::c_void, Handle, u32) -> u64,
    pub is_depot: extern "C" fn(*mut std::ffi::c_void, Handle, u32) -> u64,
    pub is_pbs: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u8) -> u64,
    pub is_plain: extern "C" fn(*mut std::ffi::c_void, Handle, u32) -> u64,
    pub is_railway: extern "C" fn(*mut std::ffi::c_void, Handle, u32) -> u64,
    pub is_station: extern "C" fn(*mut std::ffi::c_void, Handle, u32) -> u64,
    pub is_tunnel: extern "C" fn(*mut std::ffi::c_void, Handle, u32) -> u64,
    pub is_waypoint: extern "C" fn(*mut std::ffi::c_void, Handle, u32) -> u64,
    pub line_reverse: extern "C" fn(*mut std::ffi::c_void, Handle) -> u64,
    pub mark_tile: extern "C" fn(*mut std::ffi::c_void, Handle, u32),
    pub needs_service: extern "C" fn(*mut std::ffi::c_void, Handle) -> u64,
    pub oneway: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u8) -> u64,
    pub order_service: extern "C" fn(*mut std::ffi::c_void, Handle, u8) -> u64,
    pub order_stop: extern "C" fn(*mut std::ffi::c_void, Handle, u16) -> u64,
    pub order_type: extern "C" fn(*mut std::ffi::c_void, Handle, u8) -> u64,
    pub other_end: extern "C" fn(*mut std::ffi::c_void, Handle, u32) -> u64,
    pub overlap: extern "C" fn(*mut std::ffi::c_void, Handle, u8) -> u64,
    pub path_result: extern "C" fn(*mut std::ffi::c_void, Handle, u8),
    pub rail90: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u32) -> u64,
    pub reach_dirs: extern "C" fn(*mut std::ffi::c_void, Handle, u8) -> u64,
    pub reach_tracks: extern "C" fn(*mut std::ffi::c_void, Handle, u8) -> u64,
    pub reserved: extern "C" fn(*mut std::ffi::c_void, Handle, u32) -> u64,
    pub reserve_paths: extern "C" fn(*mut std::ffi::c_void, Handle) -> u64,
    pub restore_order: extern "C" fn(*mut std::ffi::c_void, Handle),
    pub save_order: extern "C" fn(*mut std::ffi::c_void, Handle),
    pub service: extern "C" fn(*mut std::ffi::c_void, Handle),
    pub set_depot: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u8),
    pub set_depot_dest: extern "C" fn(*mut std::ffi::c_void, Handle, u32),
    pub set_platform: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u8, u8),
    pub set_signal: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u8, u8),
    pub set_tunnel: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u8),
    pub show_res: extern "C" fn(*mut std::ffi::c_void, Handle) -> u64,
    pub signal_context: extern "C" fn() -> crate::signal::Context,
    pub vehicle: *const crate::vehicle::VehicleServices,
    pub shared: *const crate::services::Services,
    pub start_stop: extern "C" fn(*mut std::ffi::c_void, Handle),
    pub station: extern "C" fn(*mut std::ffi::c_void, Handle, u32) -> u64,
    pub station_train: extern "C" fn(*mut std::ffi::c_void, Handle, u16) -> u64,
    pub station_xy: extern "C" fn(*mut std::ffi::c_void, Handle, u16) -> u64,
    pub stuck: extern "C" fn(*mut std::ffi::c_void, Handle),
    pub tile_add: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u8) -> u64,
    pub tile_offset: extern "C" fn(*mut std::ffi::c_void, Handle, u8) -> u64,
    pub trackdir: extern "C" fn(*mut std::ffi::c_void, Handle) -> u64,
    pub track_status: extern "C" fn(*mut std::ffi::c_void, Handle, u32) -> u64,
    pub try_track: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u8) -> u64,
    pub tunnel_dir: extern "C" fn(*mut std::ffi::c_void, Handle, u32) -> u64,
    pub tunnel_free: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u32) -> u64,
    pub unreserve: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u8),
    pub write_dest: extern "C" fn(*mut std::ffi::c_void, Handle, u32),
    pub write_last: extern "C" fn(*mut std::ffi::c_void, Handle, u16),
    pub write_suppress: extern "C" fn(*mut std::ffi::c_void, Handle, u8),

    pub follow: extern "C" fn(Handle, u64, *mut Follow) -> u8,
    pub origin: extern "C" fn(Handle, u8) -> Pbs,
    pub pathfind: extern "C" fn(Handle, u32, u8, u8, u8, u8) -> Search,
    pub safe_track: extern "C" fn(Handle, u32, u8, u8) -> u8,
    pub process_orders: extern "C" fn(Handle) -> u8,
    pub update_order_dest: extern "C" fn(Handle, u8) -> u8,
}
#[derive(Clone, Copy)]
struct Game<'a> {
    vehicle: *mut crate::vehicle::OpenTTDVehicle,
    leaves: &'a Leaves,
    context: *mut std::ffi::c_void,
}
impl Game<'_> {
    fn read_tile(&self, id: Handle) -> u32 {
        (self.leaves.read_tile)(id)
    }
    fn read_next(&self, id: Handle) -> Handle {
        (self.leaves.read_next)(id)
    }
    fn read_last_station(&self, id: Handle) -> u16 {
        (self.leaves.read_last_station)(id)
    }
    fn read_direction(&self, id: Handle) -> u8 {
        (self.leaves.read_direction)(id)
    }
    fn read_order(&self, id: Handle) -> u8 {
        (self.leaves.read_order)(id)
    }
    fn read_num_orders(&self, id: Handle) -> u8 {
        (self.leaves.read_num_orders)(id)
    }
    fn read_order_index(&self, id: Handle) -> u8 {
        (self.leaves.read_order_index)(id)
    }
    fn read_free(&self, id: Handle) -> TrainReservationFreeRead {
        (self.leaves.read_free)(id)
    }
    fn read_free_1(&self, id: Handle) -> TrainReservationFree1Read {
        (self.leaves.read_free_1)(id)
    }
    fn read_new(&self, id: Handle) -> TrainReservationNewRead {
        (self.leaves.read_new)(id)
    }
    fn read_choose(&self, id: Handle) -> TrainReservationChooseRead {
        (self.leaves.read_choose)(id)
    }
    fn read_choose_4(&self, id: Handle) -> TrainReservationChoose4Read {
        (self.leaves.read_choose_4)(id)
    }
    fn read_check_next(&self, id: Handle) -> TrainReservationCheckNextRead {
        (self.leaves.read_check_next)(id)
    }

    fn svc_all_compat(&self, id: Handle) -> u64 {
        (self.leaves.all_compat)(self.context, id)
    }
    fn svc_backoff(&self, id: Handle) -> u64 {
        (self.leaves.backoff)(self.context, id)
    }
    fn svc_bits_track(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.bits_track)(self.context, id, a as u8)
    }
    fn svc_blocking(&self, id: Handle, a: u64, b: u64) -> u64 {
        (self.leaves.blocking)(self.context, id, a as u32, b as u8)
    }
    fn svc_check_reverse(&self, id: Handle) -> u64 {
        (self.leaves.check_reverse)(self.context, id)
    }
    fn svc_compat_station(&self, id: Handle, a: u64, b: u64) -> u64 {
        (self.leaves.compat_station)(self.context, id, a as u32, b as u32)
    }
    fn svc_conditional(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.conditional)(self.context, id, a as u8)
    }
    fn svc_copy_order(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.copy_order)(self.context, id, a as u8);
        0
    }
    fn svc_cross_dirs(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.cross_dirs)(self.context, id, a as u8)
    }
    fn svc_cross_tracks(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.cross_tracks)(self.context, id, a as u8)
    }
    fn svc_depot_dir(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.depot_dir)(self.context, id, a as u32)
    }
    fn svc_depot_reserved(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.depot_reserved)(self.context, id, a as u32)
    }
    fn svc_diag_reach_dirs(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.diag_reach_dirs)(self.context, id, a as u8)
    }
    fn svc_diag_track(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.diag_track)(self.context, id, a as u8)
    }
    fn svc_enter_td(&self, id: Handle, a: u64, b: u64) -> u64 {
        (self.leaves.enter_td)(self.context, id, a as u8, b as u8)
    }
    fn svc_exit_dir(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.exit_dir)(self.context, id, a as u8)
    }
    fn svc_free(&self, id: Handle, a: u64, b: u64) -> u64 {
        (self.leaves.free)(self.context, id, a as u32, b as u8)
    }
    fn svc_green(&self, id: Handle, a: u64, b: u64) -> u64 {
        (self.leaves.green)(self.context, id, a as u32, b as u8)
    }
    fn svc_has_pbs(&self, id: Handle, a: u64, b: u64) -> u64 {
        (self.leaves.has_pbs)(self.context, id, a as u32, b as u8)
    }
    fn svc_has_reserved(&self, id: Handle, a: u64, b: u64) -> u64 {
        (self.leaves.has_reserved)(self.context, id, a as u32, b as u8)
    }
    fn svc_has_signal(&self, id: Handle, a: u64, b: u64) -> u64 {
        (self.leaves.has_signal)(self.context, id, a as u32, b as u8)
    }
    fn svc_increment_order(&self, id: Handle) -> u64 {
        (self.leaves.increment_order)(self.context, id);
        0
    }
    fn svc_is_bridge(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.is_bridge)(self.context, id, a as u32)
    }
    fn svc_is_depot(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.is_depot)(self.context, id, a as u32)
    }
    fn svc_is_pbs(&self, id: Handle, a: u64, b: u64) -> u64 {
        (self.leaves.is_pbs)(self.context, id, a as u32, b as u8)
    }
    fn svc_is_plain(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.is_plain)(self.context, id, a as u32)
    }
    fn svc_is_railway(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.is_railway)(self.context, id, a as u32)
    }
    fn svc_is_station(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.is_station)(self.context, id, a as u32)
    }
    fn svc_is_tunnel(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.is_tunnel)(self.context, id, a as u32)
    }
    fn svc_is_waypoint(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.is_waypoint)(self.context, id, a as u32)
    }
    fn svc_line_reverse(&self, id: Handle) -> u64 {
        (self.leaves.line_reverse)(self.context, id)
    }
    fn svc_mark_tile(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.mark_tile)(self.context, id, a as u32);
        0
    }
    fn svc_needs_service(&self, id: Handle) -> u64 {
        (self.leaves.needs_service)(self.context, id)
    }
    fn svc_oneway(&self, id: Handle, a: u64, b: u64) -> u64 {
        (self.leaves.oneway)(self.context, id, a as u32, b as u8)
    }
    fn svc_order_service(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.order_service)(self.context, id, a as u8)
    }
    fn svc_order_stop(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.order_stop)(self.context, id, a as u16)
    }
    fn svc_order_type(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.order_type)(self.context, id, a as u8)
    }
    fn svc_other_end(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.other_end)(self.context, id, a as u32)
    }
    fn svc_overlap(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.overlap)(self.context, id, a as u8)
    }
    fn svc_path_result(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.path_result)(self.context, id, a as u8);
        0
    }
    fn svc_rail90(&self, id: Handle, a: u64, b: u64) -> u64 {
        (self.leaves.rail90)(self.context, id, a as u32, b as u32)
    }
    fn svc_reach_dirs(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.reach_dirs)(self.context, id, a as u8)
    }
    fn svc_reach_tracks(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.reach_tracks)(self.context, id, a as u8)
    }
    fn svc_reserved(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.reserved)(self.context, id, a as u32)
    }
    fn svc_reserve_paths(&self, id: Handle) -> u64 {
        (self.leaves.reserve_paths)(self.context, id)
    }
    fn svc_restore_order(&self, id: Handle) -> u64 {
        (self.leaves.restore_order)(self.context, id);
        0
    }
    fn shared(&self) -> &crate::services::Services {
        // SAFETY: Native installs the process-lifetime shared table; tests a live fixture.
        unsafe { &*self.leaves.shared }
    }
    fn svc_save_order(&self, id: Handle) -> u64 {
        (self.leaves.save_order)(self.context, id);
        0
    }
    fn svc_service(&self, id: Handle) -> u64 {
        (self.leaves.service)(self.context, id);
        0
    }
    fn svc_set_depot(&self, id: Handle, a: u64, b: u64) -> u64 {
        (self.leaves.set_depot)(self.context, id, a as u32, b as u8);
        0
    }
    fn svc_set_depot_dest(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.set_depot_dest)(self.context, id, a as u32);
        0
    }
    fn svc_set_platform(&self, id: Handle, a: u64, b: u64, c: u64) -> u64 {
        (self.leaves.set_platform)(self.context, id, a as u32, b as u8, c as u8);
        0
    }
    fn svc_set_signal(&self, id: Handle, a: u64, b: u64, c: u64) -> u64 {
        (self.leaves.set_signal)(self.context, id, a as u32, b as u8, c as u8);
        0
    }
    fn svc_set_tunnel(&self, id: Handle, a: u64, b: u64) -> u64 {
        (self.leaves.set_tunnel)(self.context, id, a as u32, b as u8);
        0
    }
    fn svc_show_res(&self, id: Handle) -> u64 {
        (self.leaves.show_res)(self.context, id)
    }
    fn svc_signal_buffer(&self, _id: Handle, a: u64, b: u64) -> u64 {
        // SAFETY: C++ supplies the adjusted Vehicle base of the entry train.
        // The live shell and immutable table survive this call; no owner borrow
        // spans the canonical shared callback. free() retains that entry train.
        let owner = unsafe {
            let vehicle =
                crate::vehicle::VehicleRef(std::ptr::NonNull::new_unchecked(self.vehicle));
            ((*self.leaves.vehicle).owner)(vehicle)
        };
        crate::signal::count_context_owner_read();
        crate::signal::acquire_context(self.leaves.signal_context)
            .add_side(a as u32, b as u8, owner);
        0
    }
    fn svc_start_stop(&self, id: Handle) -> u64 {
        (self.leaves.start_stop)(self.context, id);
        0
    }
    fn svc_station(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.station)(self.context, id, a as u32)
    }
    fn svc_station_train(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.station_train)(self.context, id, a as u16)
    }
    fn svc_station_xy(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.station_xy)(self.context, id, a as u16)
    }
    fn svc_stuck(&self, id: Handle) -> u64 {
        (self.leaves.stuck)(self.context, id);
        0
    }
    fn svc_tile_add(&self, id: Handle, a: u64, b: u64) -> u64 {
        (self.leaves.tile_add)(self.context, id, a as u32, b as u8)
    }
    fn svc_tile_offset(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.tile_offset)(self.context, id, a as u8)
    }
    fn svc_trackdir(&self, id: Handle) -> u64 {
        (self.leaves.trackdir)(self.context, id)
    }
    fn svc_track_status(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.track_status)(self.context, id, a as u32)
    }
    fn svc_try_track(&self, id: Handle, a: u64, b: u64) -> u64 {
        (self.leaves.try_track)(self.context, id, a as u32, b as u8)
    }
    fn svc_tunnel_dir(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.tunnel_dir)(self.context, id, a as u32)
    }
    fn svc_tunnel_free(&self, id: Handle, a: u64, b: u64) -> u64 {
        (self.leaves.tunnel_free)(self.context, id, a as u32, b as u32)
    }
    fn svc_unreserve(&self, id: Handle, a: u64, b: u64) -> u64 {
        (self.leaves.unreserve)(self.context, id, a as u32, b as u8);
        0
    }
    fn svc_update_buffer(&self, _id: Handle) -> u64 {
        crate::signal::acquire_context(self.leaves.signal_context).flush();
        0
    }
    fn svc_write_dest(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.write_dest)(self.context, id, a as u32);
        0
    }
    fn svc_write_last(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.write_last)(self.context, id, a as u16);
        0
    }
    fn svc_write_suppress(&self, id: Handle, a: u64) -> u64 {
        (self.leaves.write_suppress)(self.context, id, a as u8);
        0
    }

    fn reserve_track(&self, id: Handle, tile: u32, track: u8) -> bool {
        (self.leaves.try_track)(self.context, id, tile, track) != 0
    }
    fn pathfind(
        &self,
        id: Handle,
        tile: u32,
        dir: u8,
        tracks: u8,
        reserve: bool,
        want_final: bool,
    ) -> Search {
        (self.leaves.pathfind)(
            id,
            tile,
            dir,
            tracks,
            u8::from(reserve),
            u8::from(want_final),
        )
    }
    fn safe_track(&self, id: Handle, tile: u32, td: u8, override_types: bool) -> bool {
        (self.leaves.safe_track)(id, tile, td, u8::from(override_types)) != 0
    }

    fn follow(&self, id: Handle, types: u64, ft: &mut Follow, tile: u32, td: u8) -> bool {
        ft.old_tile = tile;
        ft.old_td = td;
        (self.leaves.follow)(id, types, ft) != 0
    }
    fn origin(&self, id: Handle, other: bool) -> Pbs {
        (self.leaves.origin)(id, u8::from(other))
    }
    fn dir(&self, id: Handle, td: u8) -> u8 {
        self.svc_exit_dir(id, u64::from(td)) as u8
    }
    fn add(&self, id: Handle, tile: u32, dir: u8) -> u32 {
        self.svc_tile_add(id, u64::from(tile), u64::from(dir)) as u32
    }
    fn signal(&self, id: Handle, tile: u32, td: u8, green: bool) {
        self.svc_set_signal(id, u64::from(tile), u64::from(td), u64::from(green));
    }
    fn reserve_bounded(&self, id: Handle, tile: u32, td: u8) -> bool {
        self.svc_try_track(id, u64::from(tile), u64::from(td & 7)) != 0
    }

    fn reserve(&self, id: Handle, tile: u32, td: u8) -> bool {
        self.reserve_track(id, tile, td & 7)
    }
    fn unreserve(&self, id: Handle, tile: u32, td: u8) {
        self.svc_unreserve(id, u64::from(tile), u64::from(td & 7));
    }

    fn td(&self, id: Handle) -> u8 {
        self.svc_trackdir(id) as u8
    }
}
fn first(bits: u16) -> u8 {
    if bits == 0 {
        0xff
    } else {
        bits.trailing_zeros() as u8
    }
}
fn single(bits: u16) -> bool {
    bits & bits.wrapping_sub(1) == 0
}
fn tracks(bits: u16) -> u8 {
    (bits | (bits >> 8)) as u8
}
fn dirs(bits: u8) -> u16 {
    u16::from(bits) | (u16::from(bits) << 8)
}
fn pbs(step: Search) -> Pbs {
    Pbs {
        tile: step.tile,
        td: step.td,
        okay: step.okay,
        other: Handle::NONE,
    }
}
fn clear(g: &Game, id: Handle, tile: u32, td: u8) {
    let dir = g.dir(id, td);
    if g.svc_is_tunnel(id, u64::from(tile)) != 0 {
        if g.svc_tunnel_dir(id, u64::from(tile)) as u8 == dir ^ 2 {
            let end = g.svc_other_end(id, u64::from(tile)) as u32;
            if g.svc_tunnel_free(id, u64::from(tile), u64::from(end)) != 0 {
                g.svc_set_tunnel(id, u64::from(tile), 0);
                g.svc_set_tunnel(id, u64::from(end), 0);
                if g.svc_show_res(id) != 0 {
                    if g.svc_is_bridge(id, u64::from(tile)) != 0 {
                        (g.shared().mark_bridge_dirty)(tile);
                    } else {
                        g.svc_mark_tile(id, u64::from(tile));
                        g.svc_mark_tile(id, u64::from(end));
                    }
                }
            }
        }
    } else if g.svc_is_station(id, u64::from(tile)) != 0 {
        let next = g.add(id, tile, dir);
        if g.svc_compat_station(id, u64::from(next), u64::from(tile)) == 0 {
            g.svc_set_platform(id, u64::from(tile), u64::from(dir ^ 2), 0);
        }
    } else {
        g.unreserve(id, tile, td);
    }
}
fn free(g: &Game, id: Handle) {
    crate::witness::hit(crate::witness::TRAIN_FREE_PATH);
    let v = g.read_free(id);
    let mut tile = v.tile;
    let mut td = g.td(id);
    let mut free_tile = !((g.svc_is_station(id, u64::from(tile)) != 0)
        || (g.svc_is_tunnel(id, u64::from(tile)) != 0));
    let station = if g.svc_is_station(id, u64::from(tile)) != 0 {
        g.svc_station(id, u64::from(tile)) as u16
    } else {
        u16::MAX
    };
    if (g.svc_is_depot(id, u64::from(tile)) != 0)
        && g.dir(id, td) != g.svc_depot_dir(id, u64::from(tile)) as u8
    {
        return;
    }
    if id.get_track() == u64::from(DEPOT) {
        let mut u = id;
        while u != Handle::NONE {
            let part = g.read_free_1(u);
            if u.get_track() != u64::from(DEPOT) || part.tile != v.tile {
                return;
            }
            u = part.next;
        }
    }
    let reserved = g.svc_reserved(id, u64::from(tile)) as u8;
    if g.svc_overlap(id, u64::from(reserved | (1 << (td & 7)))) != 0 {
        return;
    }
    let types = g.svc_all_compat(id);
    let mut ft = Follow::default();
    while g.follow(id, types, &mut ft, tile, td) {
        tile = ft.new_tile;
        let bits = ft.dirs & dirs(g.svc_reserved(id, u64::from(tile)) as u8);
        td = first(bits);
        if td == 0xff {
            break;
        }
        if g.svc_is_railway(id, u64::from(tile)) != 0 {
            if (g.svc_has_signal(id, u64::from(tile), u64::from(td)) != 0)
                && (g.svc_is_pbs(id, u64::from(tile), u64::from(td & 7)) == 0)
            {
                g.unreserve(id, tile, td);
                break;
            }
            if g.svc_has_pbs(id, u64::from(tile), u64::from(td)) != 0 {
                if g.svc_green(id, u64::from(tile), u64::from(td)) == 0 {
                    break;
                }
                g.signal(id, tile, td, false);
                g.svc_mark_tile(id, u64::from(tile));
            } else if g.svc_has_pbs(id, u64::from(tile), u64::from(td ^ 8)) != 0 {
                g.svc_signal_buffer(id, u64::from(tile), u64::from(g.dir(id, td ^ 8)));
            } else if (g.svc_has_signal(id, u64::from(tile), u64::from(td ^ 8)) != 0)
                && (g.svc_oneway(id, u64::from(tile), u64::from(td & 7)) != 0)
            {
                break;
            }
        }
        if free_tile
            || (!(ft.station != 0 && g.svc_station(id, u64::from(ft.new_tile)) as u16 == station)
                && ft.tunnel == 0
                && ft.bridge == 0)
        {
            clear(g, id, tile, td);
        }
        free_tile = true;
    }
    g.svc_update_buffer(id);
}
fn extend(g: &Game, id: Handle, new_tracks: &mut u8, enterdir: &mut u8) -> Pbs {
    crate::witness::hit(crate::witness::TRAIN_CONTROLLER_EXTENSION);
    let origin = g.origin(id, false);
    let types = id.get_compatible_railtypes();
    let mut ft = Follow::default();
    let mut red = Vec::new();
    let mut tile = origin.tile;
    let mut td = origin.td;
    while g.follow(id, types, &mut ft, tile, td) {
        if single(ft.dirs)
            && (g.svc_blocking(id, u64::from(ft.new_tile), u64::from(first(ft.dirs))) != 0)
        {
            break;
        }
        if g.svc_rail90(id, u64::from(ft.old_tile), u64::from(ft.new_tile)) != 0 {
            ft.dirs &= !(g.svc_cross_dirs(id, u64::from(ft.old_td)) as u16);
            if ft.dirs == 0 {
                break;
            }
        }
        let target = ft.station != 0
            || ((g.svc_is_railway(id, u64::from(ft.new_tile)) != 0)
                && (g.svc_is_plain(id, u64::from(ft.new_tile)) == 0));
        if target || !single(ft.dirs) {
            if g.svc_has_reserved(
                id,
                u64::from(ft.new_tile),
                u64::from(tracks(g.svc_reach_dirs(id, u64::from(ft.old_td)) as u16)),
            ) != 0
            {
                break;
            }
            if ft.skipped != 0 {
                let off = g.svc_tile_offset(id, u64::from(ft.exitdir)) as i32;
                ft.new_tile = ft
                    .new_tile
                    .wrapping_sub(off.wrapping_mul(ft.skipped) as u32);
            }
            *new_tracks = tracks(ft.dirs);
            *enterdir = ft.exitdir;
            return Pbs {
                tile: ft.new_tile,
                td: ft.old_td,
                okay: 0,
                other: Handle::NONE,
            };
        }
        tile = ft.new_tile;
        td = first(ft.dirs);
        let rev = td ^ 8;
        // SAFETY: The entry keeps the native train live; no owner borrow spans the callback.
        if unsafe {
            (g.shared().safe_waiting_position)(
                id.shell.cast(),
                tile,
                td,
                (g.shared().forbid_90_deg)(),
            )
        } {
            if !((g.svc_free(id, u64::from(tile), u64::from(td)) != 0)
                && g.reserve_bounded(id, tile, td))
            {
                break;
            }
            if (g.svc_has_pbs(id, u64::from(tile), u64::from(rev)) != 0)
                && (g.svc_green(id, u64::from(tile), u64::from(rev)) != 0)
            {
                crate::witness::hit(crate::witness::TRAIN_EXTENSION_OPPOSING_RED);
                red.push((tile, rev));
                g.signal(id, tile, rev, false);
                g.svc_mark_tile(id, u64::from(tile));
            }
            return Pbs {
                tile,
                td,
                okay: 1,
                other: Handle::NONE,
            };
        }
        if !g.reserve_bounded(id, tile, td) {
            break;
        }
        if (g.svc_has_pbs(id, u64::from(tile), u64::from(rev)) != 0)
            && (g.svc_green(id, u64::from(tile), u64::from(rev)) != 0)
        {
            crate::witness::hit(crate::witness::TRAIN_EXTENSION_OPPOSING_RED);
            red.push((tile, rev));
            g.signal(id, tile, rev, false);
            g.svc_mark_tile(id, u64::from(tile));
        }
    }
    if ft.error == 1 || ft.error == 4 {
        return Pbs {
            tile: ft.old_tile,
            td: ft.old_td,
            okay: 1,
            other: Handle::NONE,
        };
    }
    crate::witness::hit(crate::witness::TRAIN_EXTENSION_FAIL);
    tile = origin.tile;
    td = origin.td;
    let stopped = ft.old_tile;
    let stopped_td = ft.old_td;
    while tile != stopped || td != stopped_td {
        if !g.follow(id, types, &mut ft, tile, td) {
            break;
        }
        if g.svc_rail90(id, u64::from(ft.old_tile), u64::from(ft.new_tile)) != 0 {
            ft.dirs &= !(g.svc_cross_dirs(id, u64::from(ft.old_td)) as u16);
        }
        tile = ft.new_tile;
        td = first(ft.dirs);
        crate::witness::hit(crate::witness::TRAIN_EXTENSION_ROLLBACK);
        g.unreserve(id, tile, td);
    }
    for (tile, td) in red {
        crate::witness::hit(crate::witness::TRAIN_EXTENSION_RESTORE);
        g.signal(id, tile, td, true);
    }
    Pbs::default()
}
struct Orders<'a> {
    g: Game<'a>,
    id: Handle,
    dest: u32,
    last: u16,
    suppress: u8,
    index: u8,
    restored: bool,
}
impl<'a> Orders<'a> {
    fn new(g: &Game<'a>, id: Handle) -> Self {
        let v = g.read_new(id);
        g.svc_save_order(id);
        Self {
            g: *g,
            id,
            dest: v.dest,
            last: v.last_station,
            suppress: v.suppress,
            index: v.order_index,
            restored: false,
        }
    }
    fn restore(&mut self) {
        crate::witness::hit(crate::witness::TRAIN_ORDER_RESTORE);
        self.g.svc_restore_order(self.id);
        self.g.svc_write_dest(self.id, u64::from(self.dest));
        self.g.svc_write_last(self.id, u64::from(self.last));
        self.g.svc_write_suppress(self.id, u64::from(self.suppress));
        self.restored = true;
    }
    fn next(&mut self, skip: bool) -> bool {
        crate::witness::hit(crate::witness::TRAIN_ORDER_LOOKAHEAD);
        let g = self.g;
        let id = self.id;
        if g.read_num_orders(id) == 0 {
            return false;
        }
        if skip {
            self.index = self.index.wrapping_add(1);
        }
        let mut depth = 0;
        loop {
            if self.index >= g.read_num_orders(id) {
                self.index = 0;
            }
            let kind = g.svc_order_type(id, u64::from(self.index)) as u8;
            match kind {
                1 | 2 | 6 => {
                    if kind != 2
                        || g.svc_order_service(id, u64::from(self.index)) == 0
                        || g.svc_needs_service(id) != 0
                    {
                        g.svc_copy_order(id, u64::from(self.index));
                        return (g.leaves.update_order_dest)(id, self.index) != 0;
                    }
                }
                7 => {
                    let next = g.svc_conditional(id, u64::from(self.index)) as u8;
                    if next != 0xff {
                        depth += 1;
                        self.index = next;
                        if self.index != g.read_order_index(id)
                            && depth < i32::from(g.read_num_orders(id))
                        {
                            continue;
                        }
                        return false;
                    }
                }
                _ => {}
            }
            self.index = self.index.wrapping_add(1);
            depth += 1;
            if self.index == g.read_order_index(id) || depth >= i32::from(g.read_num_orders(id)) {
                return false;
            }
        }
    }
}
impl Drop for Orders<'_> {
    fn drop(&mut self) {
        if !self.restored {
            self.restore();
        }
    }
}
fn choose(
    g: &Game,
    id: Handle,
    tile: u32,
    dir: u8,
    mut available: u8,
    force: bool,
    mark: bool,
) -> (u8, bool) {
    crate::witness::hit(crate::witness::TRAIN_CHOOSE_TRACK);
    let mut best = 0xff;
    let mut reserve = g.svc_reserve_paths(id) != 0 || force;
    let mut changed = false;
    let mut final_dest = INVALID;
    let mut got = false;
    let res =
        g.svc_reserved(id, u64::from(tile)) as u8 & g.svc_reach_tracks(id, u64::from(dir)) as u8;
    if res != 0 {
        return (first(u16::from(res)), false);
    }
    if single(u16::from(available)) {
        let track = first(u16::from(available));
        if track != 0xff
            && (g.svc_has_pbs(
                id,
                u64::from(tile),
                u64::from(g.svc_enter_td(id, u64::from(track), u64::from(dir)) as u8),
            ) != 0)
        {
            reserve = true;
            changed = true;
            g.signal(
                id,
                tile,
                g.svc_enter_td(id, u64::from(track), u64::from(dir)) as u8,
                true,
            );
        } else if !reserve {
            return (track, false);
        }
        best = track;
    }
    let mut dest = Pbs {
        tile,
        td: 0xff,
        okay: 0,
        other: Handle::NONE,
    };
    let mut dest_dir = dir;
    if reserve {
        dest = extend(g, id, &mut available, &mut dest_dir);
        if dest.tile == INVALID {
            if mark {
                g.svc_stuck(id);
            }
            if changed {
                g.signal(
                    id,
                    tile,
                    g.svc_enter_td(id, u64::from(best), u64::from(dir)) as u8,
                    false,
                );
            }
            return (first(u16::from(available)), false);
        }
        if dest.okay != 0 {
            if changed {
                g.svc_mark_tile(id, u64::from(tile));
            }
            g.reserve(id, g.read_tile(id), g.td(id));
            return (best, true);
        }
        g.svc_service(id);
        if matches!(g.read_order(id), 2 | 5 | 7) {
            (g.leaves.process_orders)(id);
        }
    }
    let mut orders = Orders::new(g, id);
    let v = g.read_choose(id);
    if v.order == 4 {
        orders.next(false);
    } else if v.order == 3
        || (v.order != 2
            && if v.order == 1 {
                (g.svc_is_station(id, u64::from(v.tile)) != 0)
                    && u64::from(v.destination) == g.svc_station(id, u64::from(v.tile))
            } else {
                v.tile == v.dest
            })
    {
        orders.next(true);
    }
    if dest.tile != INVALID && dest.okay == 0 {
        let new_tile = dest.tile;
        let result = g.pathfind(id, new_tile, dest_dir, available, reserve, true);
        dest = pbs(result);
        final_dest = result.final_dest;
        if new_tile == tile {
            best = result.track;
        }
        g.svc_path_result(id, u64::from(result.found));
    }
    if !reserve {
        return (best, false);
    }
    if dest.tile != INVALID && dest.okay == 0 {
        if mark {
            g.svc_stuck(id);
        }
        free(g, id);
        return (best, false);
    }
    if dest.tile == INVALID {
        let origin = g.origin(id, false);
        if g.safe_track(id, origin.tile, origin.td, false) {
            let res = g.svc_reserved(id, u64::from(tile)) as u8
                & g.svc_reach_tracks(id, u64::from(dir)) as u8;
            best = first(u16::from(res));
            g.reserve(id, g.read_tile(id), g.td(id));
            got = true;
            if changed {
                g.svc_mark_tile(id, u64::from(tile));
            }
        } else {
            free(g, id);
            if mark {
                g.svc_stuck(id);
            }
        }
        return (best, got);
    }
    got = true;
    // SAFETY: The entry keeps the native train live through reservation traversal.
    while !unsafe {
        (g.shared().safe_waiting_position)(
            id.shell.cast(),
            dest.tile,
            dest.td,
            (g.shared().forbid_90_deg)(),
        )
    } {
        let exit = g.dir(id, dest.td);
        let next = g.add(id, dest.tile, exit);
        let mut reachable = tracks(g.svc_track_status(id, u64::from(next)) as u16)
            & g.svc_reach_tracks(id, u64::from(exit)) as u8;
        if g.svc_rail90(id, u64::from(dest.tile), u64::from(next)) != 0 {
            reachable &= !(g.svc_cross_tracks(id, u64::from(dest.td & 7)) as u8);
        }
        if orders.next(true) {
            let current = pbs(g.pathfind(id, next, exit, reachable, true, false));
            if current.tile != INVALID {
                dest = current;
                if dest.okay != 0 {
                    continue;
                }
                free(g, id);
                if mark {
                    g.svc_stuck(id);
                }
                got = false;
                changed = false;
                break;
            }
        }
        if !g.safe_track(id, dest.tile, dest.td, true) {
            free(g, id);
            if mark {
                g.svc_stuck(id);
            }
            got = false;
            changed = false;
        }
        break;
    }
    g.reserve(id, g.read_tile(id), g.td(id));
    if changed {
        g.svc_mark_tile(id, u64::from(tile));
    }
    orders.restore();
    let v = g.read_choose_4(id);
    if v.order == 2
        && v.nearest != 0
        && final_dest != INVALID
        && (g.svc_is_depot(id, u64::from(final_dest)) != 0)
    {
        g.svc_set_depot_dest(id, u64::from(final_dest));
        g.svc_write_dest(id, u64::from(final_dest));
        g.svc_start_stop(id);
    }
    (best, got)
}
fn try_path(g: &Game, id: Handle, mark: bool, first_okay: bool) -> bool {
    let v = g.read_free(id);
    if id.get_track() == u64::from(DEPOT) {
        if g.svc_depot_reserved(id, u64::from(v.tile)) != 0 {
            if mark {
                g.svc_stuck(id);
            }
            return false;
        }
        let dir = g.svc_depot_dir(id, u64::from(v.tile)) as u8;
        let next = g.add(id, v.tile, dir);
        if g.svc_has_reserved(id, u64::from(next), g.svc_reach_tracks(id, u64::from(dir))) != 0 {
            return false;
        }
    }
    let origin = g.origin(id, true);
    if origin.other != Handle::NONE && origin.other != id {
        if mark {
            g.svc_stuck(id);
        }
        return false;
    }
    if origin.okay != 0 && (v.tile != origin.tile || first_okay) {
        if id.stuck() {
            g.svc_start_stop(id);
        }
        id.clear_stuck();
        return true;
    }
    if id.get_track() == u64::from(DEPOT) {
        g.svc_set_depot(id, u64::from(v.tile), 1);
        if g.svc_show_res(id) != 0 {
            g.svc_mark_tile(id, u64::from(v.tile));
        }
    }
    let exit = g.dir(id, origin.td);
    let new_tile = g.add(id, origin.tile, exit);
    let mut reachable = tracks(
        g.svc_track_status(id, u64::from(new_tile)) as u16
            & g.svc_diag_reach_dirs(id, u64::from(exit)) as u16,
    );
    if g.svc_rail90(id, u64::from(origin.tile), u64::from(new_tile)) != 0 {
        reachable &= !(g.svc_cross_tracks(id, u64::from(origin.td & 7)) as u8);
    }
    let (_, made) = choose(g, id, new_tile, exit, reachable, true, mark);
    if !made {
        if id.get_track() == u64::from(DEPOT) {
            g.svc_set_depot(id, u64::from(g.read_tile(id)), 0);
        }
        return false;
    }
    if id.stuck() {
        id.set_wait_counter(0);
        g.svc_start_stop(id);
    }
    id.clear_stuck();
    true
}
fn check_next(g: &Game, id: Handle) {
    if g.svc_backoff(id) == 255 || id.get_track() == u64::from(DEPOT) {
        return;
    }
    let v = g.read_check_next(id);
    match v.order {
        2 => {
            if v.tile == v.dest {
                return;
            }
        }
        6 => {
            if (g.svc_is_waypoint(id, u64::from(v.tile)) != 0)
                && g.svc_station(id, u64::from(v.tile)) == u64::from(v.destination)
            {
                (g.leaves.process_orders)(id);
            }
        }
        0 | 3 | 4 if v.num_orders > 0 => return,
        _ => {}
    }
    let tile = g.read_tile(id);
    if (g.svc_is_station(id, u64::from(tile)) != 0)
        && g.svc_order_stop(id, g.svc_station(id, u64::from(tile))) != 0
    {
        return;
    }
    let td = g.td(id);
    if (g.svc_is_railway(id, u64::from(tile)) != 0)
        && (g.svc_has_signal(id, u64::from(tile), u64::from(td)) != 0)
        && (g.svc_is_pbs(id, u64::from(tile), u64::from(td & 7)) == 0)
        && (g.svc_green(id, u64::from(tile), u64::from(td)) == 0)
    {
        return;
    }
    let mut ft = Follow::default();
    if !g.follow(id, id.get_compatible_railtypes(), &mut ft, tile, td) {
        return;
    }
    if g.svc_has_reserved(id, u64::from(ft.new_tile), u64::from(tracks(ft.dirs))) == 0
        && single(ft.dirs)
        && (g.svc_has_pbs(id, u64::from(ft.new_tile), u64::from(first(ft.dirs))) != 0)
    {
        let mut available = tracks(ft.dirs);
        if g.svc_rail90(id, u64::from(ft.old_tile), u64::from(ft.new_tile)) != 0 {
            available &= !(g.svc_cross_tracks(id, u64::from(ft.old_td & 7)) as u8);
        }
        choose(g, id, ft.new_tile, ft.exitdir, available, false, false);
    }
}

/// Synchronous reservation policy; native order snapshots live on the calling stack.
/// # Safety
/// Leaves, context and shell are live for this call. No state borrow spans a callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reservation_check_next(
    id: Handle,
    vehicle: crate::vehicle::VehicleRef,
    leaves: *const Leaves,
    context: *mut std::ffi::c_void,
) {
    // SAFETY: The native stack owns these immutable services throughout the call.
    let g = Game {
        vehicle: vehicle.0.as_ptr(),
        leaves: unsafe { &*leaves },
        context,
    };
    check_next(&g, id);
}

/// Synchronous reservation policy; native order snapshots live on the calling stack.
/// # Safety
/// Leaves, context and shell are live for this call. No state borrow spans a callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reservation_clear(
    id: Handle,
    vehicle: crate::vehicle::VehicleRef,
    tile: u32,
    td: u8,
    leaves: *const Leaves,
    context: *mut std::ffi::c_void,
) {
    // SAFETY: The native stack owns these immutable services throughout the call.
    let g = Game {
        vehicle: vehicle.0.as_ptr(),
        leaves: unsafe { &*leaves },
        context,
    };
    clear(&g, id, tile, td);
}

/// Synchronous reservation policy; native order snapshots live on the calling stack.
/// # Safety
/// Leaves, context and shell are live for this call. No state borrow spans a callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reservation_free(
    id: Handle,
    vehicle: crate::vehicle::VehicleRef,
    leaves: *const Leaves,
    context: *mut std::ffi::c_void,
) {
    // SAFETY: The native stack owns these immutable services throughout the call.
    let g = Game {
        vehicle: vehicle.0.as_ptr(),
        leaves: unsafe { &*leaves },
        context,
    };
    free(&g, id);
}

/// Synchronous reservation policy; native order snapshots live on the calling stack.
/// # Safety
/// Leaves, context and shell are live for this call. No state borrow spans a callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reservation_choose(
    id: Handle,
    vehicle: crate::vehicle::VehicleRef,
    tile: u32,
    dir: u8,
    tracks: u8,
    force: u8,
    mark: u8,
    leaves: *const Leaves,
    context: *mut std::ffi::c_void,
) -> Choice {
    // SAFETY: The native stack owns these immutable services throughout the call.
    let g = Game {
        vehicle: vehicle.0.as_ptr(),
        leaves: unsafe { &*leaves },
        context,
    };
    let (track, got) = choose(&g, id, tile, dir, tracks, force != 0, mark != 0);
    Choice {
        track,
        reserved: u8::from(got),
    }
}

/// Synchronous reservation policy; native order snapshots live on the calling stack.
/// # Safety
/// Leaves, context and shell are live for this call. No state borrow spans a callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reservation_try_path(
    id: Handle,
    vehicle: crate::vehicle::VehicleRef,
    mark: u8,
    first_okay: u8,
    leaves: *const Leaves,
    context: *mut std::ffi::c_void,
) -> u8 {
    // SAFETY: The native stack owns these immutable services throughout the call.
    let g = Game {
        vehicle: vehicle.0.as_ptr(),
        leaves: unsafe { &*leaves },
        context,
    };
    u8::from(try_path(&g, id, mark != 0, first_okay != 0))
}

/// Synchronous reservation policy; native order snapshots live on the calling stack.
/// # Safety
/// Leaves, context and shell are live for this call. No state borrow spans a callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reservation_check_reverse(
    id: Handle,
    vehicle: crate::vehicle::VehicleRef,
    leaves: *const Leaves,
    context: *mut std::ffi::c_void,
) -> u8 {
    // SAFETY: The native stack owns these immutable services throughout the call.
    let g = Game {
        vehicle: vehicle.0.as_ptr(),
        leaves: unsafe { &*leaves },
        context,
    };
    if g.svc_line_reverse(id) == 0
        && id.get_track() != u64::from(DEPOT)
        && id.get_track() != u64::from(WORMHOLE)
        && g.read_direction(id) & 1 != 0
    {
        g.svc_check_reverse(id) as u8
    } else {
        0
    }
}

/// Synchronous reservation policy; native order snapshots live on the calling stack.
/// # Safety
/// Leaves, context and shell are live for this call. No state borrow spans a callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reservation_station_location(
    id: Handle,
    vehicle: crate::vehicle::VehicleRef,
    station: u16,
    leaves: *const Leaves,
    context: *mut std::ffi::c_void,
) -> u32 {
    // SAFETY: The native stack owns these immutable services throughout the call.
    let g = Game {
        vehicle: vehicle.0.as_ptr(),
        leaves: unsafe { &*leaves },
        context,
    };
    if station == g.read_last_station(id) {
        g.svc_write_last(id, u64::from(u16::MAX));
    }
    if g.svc_station_train(id, u64::from(station)) == 0 {
        g.svc_increment_order(id);
        0
    } else {
        g.svc_station_xy(id, u64::from(station)) as u32
    }
}

/// Synchronous reservation policy; native order snapshots live on the calling stack.
/// # Safety
/// Leaves, context and shell are live for this call. No state borrow spans a callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reservation_reserve_under(
    id: Handle,
    vehicle: crate::vehicle::VehicleRef,
    leaves: *const Leaves,
    context: *mut std::ffi::c_void,
) {
    // SAFETY: The native stack owns these immutable services throughout the call.
    let g = Game {
        vehicle: vehicle.0.as_ptr(),
        leaves: unsafe { &*leaves },
        context,
    };
    let mut part = id;
    while part != Handle::NONE {
        let v = g.read_free(part);
        let track = part.get_track() as u8;
        match track {
            WORMHOLE => {
                g.reserve_track(
                    part,
                    v.tile,
                    g.svc_diag_track(part, g.svc_tunnel_dir(part, u64::from(v.tile))) as u8,
                );
            }
            DEPOT => {}
            _ => {
                g.reserve_track(part, v.tile, g.svc_bits_track(part, u64::from(track)) as u8);
            }
        }
        part = g.read_next(part);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::train_state::State;
    macro_rules! unused {
        (($($ty:ty),*); $ret:ty) => {{
            extern "C" fn abort($(_: $ty),*) -> $ret { std::process::abort() }
            abort
        }};
    }
    fn leaves() -> Leaves {
        Leaves {
            read_tile: unused!((Handle); u32),
            read_next: unused!((Handle); Handle),
            read_last_station: unused!((Handle); u16),
            read_direction: unused!((Handle); u8),
            read_order: unused!((Handle); u8),
            read_num_orders: unused!((Handle); u8),
            read_order_index: unused!((Handle); u8),
            read_free: unused!((Handle); TrainReservationFreeRead),
            read_free_1: unused!((Handle); TrainReservationFree1Read),
            read_new: unused!((Handle); TrainReservationNewRead),
            read_choose: unused!((Handle); TrainReservationChooseRead),
            read_choose_4: unused!((Handle); TrainReservationChoose4Read),
            read_check_next: unused!((Handle); TrainReservationCheckNextRead),
            all_compat: unused!((*mut std::ffi::c_void, Handle); u64),
            backoff: unused!((*mut std::ffi::c_void, Handle); u64),
            bits_track: unused!((*mut std::ffi::c_void, Handle, u8); u64),
            blocking: unused!((*mut std::ffi::c_void, Handle, u32, u8); u64),
            check_reverse: unused!((*mut std::ffi::c_void, Handle); u64),
            compat_station: unused!((*mut std::ffi::c_void, Handle, u32, u32); u64),
            conditional: unused!((*mut std::ffi::c_void, Handle, u8); u64),
            copy_order: unused!((*mut std::ffi::c_void, Handle, u8); ()),
            cross_dirs: unused!((*mut std::ffi::c_void, Handle, u8); u64),
            cross_tracks: unused!((*mut std::ffi::c_void, Handle, u8); u64),
            depot_dir: unused!((*mut std::ffi::c_void, Handle, u32); u64),
            depot_reserved: unused!((*mut std::ffi::c_void, Handle, u32); u64),
            diag_reach_dirs: unused!((*mut std::ffi::c_void, Handle, u8); u64),
            diag_track: unused!((*mut std::ffi::c_void, Handle, u8); u64),
            enter_td: unused!((*mut std::ffi::c_void, Handle, u8, u8); u64),
            exit_dir: unused!((*mut std::ffi::c_void, Handle, u8); u64),
            free: unused!((*mut std::ffi::c_void, Handle, u32, u8); u64),
            green: unused!((*mut std::ffi::c_void, Handle, u32, u8); u64),
            has_pbs: unused!((*mut std::ffi::c_void, Handle, u32, u8); u64),
            has_reserved: unused!((*mut std::ffi::c_void, Handle, u32, u8); u64),
            has_signal: unused!((*mut std::ffi::c_void, Handle, u32, u8); u64),
            increment_order: unused!((*mut std::ffi::c_void, Handle); ()),
            is_bridge: unused!((*mut std::ffi::c_void, Handle, u32); u64),
            is_depot: unused!((*mut std::ffi::c_void, Handle, u32); u64),
            is_pbs: unused!((*mut std::ffi::c_void, Handle, u32, u8); u64),
            is_plain: unused!((*mut std::ffi::c_void, Handle, u32); u64),
            is_railway: unused!((*mut std::ffi::c_void, Handle, u32); u64),
            is_station: unused!((*mut std::ffi::c_void, Handle, u32); u64),
            is_tunnel: unused!((*mut std::ffi::c_void, Handle, u32); u64),
            is_waypoint: unused!((*mut std::ffi::c_void, Handle, u32); u64),
            line_reverse: unused!((*mut std::ffi::c_void, Handle); u64),
            mark_tile: unused!((*mut std::ffi::c_void, Handle, u32); ()),
            needs_service: unused!((*mut std::ffi::c_void, Handle); u64),
            oneway: unused!((*mut std::ffi::c_void, Handle, u32, u8); u64),
            order_service: unused!((*mut std::ffi::c_void, Handle, u8); u64),
            order_stop: unused!((*mut std::ffi::c_void, Handle, u16); u64),
            order_type: unused!((*mut std::ffi::c_void, Handle, u8); u64),
            other_end: unused!((*mut std::ffi::c_void, Handle, u32); u64),
            overlap: unused!((*mut std::ffi::c_void, Handle, u8); u64),
            path_result: unused!((*mut std::ffi::c_void, Handle, u8); ()),
            rail90: unused!((*mut std::ffi::c_void, Handle, u32, u32); u64),
            reach_dirs: unused!((*mut std::ffi::c_void, Handle, u8); u64),
            reach_tracks: unused!((*mut std::ffi::c_void, Handle, u8); u64),
            reserved: unused!((*mut std::ffi::c_void, Handle, u32); u64),
            reserve_paths: unused!((*mut std::ffi::c_void, Handle); u64),
            restore_order: unused!((*mut std::ffi::c_void, Handle); ()),
            save_order: unused!((*mut std::ffi::c_void, Handle); ()),
            service: unused!((*mut std::ffi::c_void, Handle); ()),
            set_depot: unused!((*mut std::ffi::c_void, Handle, u32, u8); ()),
            set_depot_dest: unused!((*mut std::ffi::c_void, Handle, u32); ()),
            set_platform: unused!((*mut std::ffi::c_void, Handle, u32, u8, u8); ()),
            set_signal: unused!((*mut std::ffi::c_void, Handle, u32, u8, u8); ()),
            set_tunnel: unused!((*mut std::ffi::c_void, Handle, u32, u8); ()),
            show_res: unused!((*mut std::ffi::c_void, Handle); u64),
            signal_context: unused!(() ; crate::signal::Context),
            vehicle: std::ptr::null(),
            shared: std::ptr::null(),
            start_stop: unused!((*mut std::ffi::c_void, Handle); ()),
            station: unused!((*mut std::ffi::c_void, Handle, u32); u64),
            station_train: unused!((*mut std::ffi::c_void, Handle, u16); u64),
            station_xy: unused!((*mut std::ffi::c_void, Handle, u16); u64),
            stuck: unused!((*mut std::ffi::c_void, Handle); ()),
            tile_add: unused!((*mut std::ffi::c_void, Handle, u32, u8); u64),
            tile_offset: unused!((*mut std::ffi::c_void, Handle, u8); u64),
            trackdir: unused!((*mut std::ffi::c_void, Handle); u64),
            track_status: unused!((*mut std::ffi::c_void, Handle, u32); u64),
            try_track: unused!((*mut std::ffi::c_void, Handle, u32, u8); u64),
            tunnel_dir: unused!((*mut std::ffi::c_void, Handle, u32); u64),
            tunnel_free: unused!((*mut std::ffi::c_void, Handle, u32, u32); u64),
            unreserve: unused!((*mut std::ffi::c_void, Handle, u32, u8); ()),
            write_dest: unused!((*mut std::ffi::c_void, Handle, u32); ()),
            write_last: unused!((*mut std::ffi::c_void, Handle, u16); ()),
            write_suppress: unused!((*mut std::ffi::c_void, Handle, u8); ()),
            follow: unused!((Handle, u64, *mut Follow); u8),
            origin: unused!((Handle, u8); Pbs),
            pathfind: unused!((Handle, u32, u8, u8, u8, u8); Search),
            safe_track: unused!((Handle, u32, u8, u8); u8),
            process_orders: unused!((Handle); u8),
            update_order_dest: unused!((Handle, u8); u8),
        }
    }
    struct Mock {
        state: State,
        stop: u8,
        log: Vec<(&'static str, u32, u8)>,
        order_token: u32,
        dest: u32,
        last: u16,
        suppress: u8,
        restores: usize,
    }
    impl Mock {
        fn new(stop: u8) -> Self {
            Self {
                state: State::default(),
                stop,
                log: Vec::new(),
                order_token: 1,
                dest: 9,
                last: 7,
                suppress: 1,
                restores: 0,
            }
        }
        fn handle(&mut self) -> Handle {
            Handle {
                shell: std::ptr::from_mut(self).cast(),
                owner: &raw mut self.state,
            }
        }
    }
    unsafe fn mock<'a>(h: Handle) -> &'a mut Mock {
        // SAFETY: Every test installs this handle to its live exclusive thread-local stack mock.
        unsafe { &mut *h.shell.cast::<Mock>() }
    }
    extern "C" fn zero_tile(_: *mut std::ffi::c_void, _: Handle, _: u32) -> u64 {
        0
    }
    extern "C" fn zero_pair(_: *mut std::ffi::c_void, _: Handle, _: u32, _: u32) -> u64 {
        0
    }
    extern "C" fn zero_track(_: *mut std::ffi::c_void, _: Handle, _: u32, _: u8) -> u64 {
        0
    }
    extern "C" fn one_track(_: *mut std::ffi::c_void, _: Handle, _: u32, _: u8) -> u64 {
        1
    }
    extern "C" fn origin(_: Handle, _: u8) -> Pbs {
        Pbs {
            tile: 0,
            other: Handle::NONE,
            td: 0,
            okay: 0,
        }
    }
    extern "C" fn follow(h: Handle, _: u64, out: *mut Follow) -> u8 {
        // SAFETY: The mock and output live throughout this synchronous callback; no recursive callback here.
        unsafe {
            let m = mock(h);
            let ft = &mut *out;
            if ft.old_tile == 3 && m.stop == 1 {
                ft.error = 4;
                return 0;
            }
            ft.new_tile = ft.old_tile + 1;
            ft.dirs = 1;
            ft.error = 0;
            1
        }
    }
    extern "C" fn safe(v: *const crate::services::Train, tile: u32, _: u8, _: bool) -> bool {
        // SAFETY: The train shell is the live mock for this synchronous scalar callback.
        unsafe { (*v.cast::<Mock>()).stop == 2 && tile == 2 }
    }
    extern "C" fn no_random(_: *mut std::ffi::c_void) -> u32 {
        std::process::abort()
    }
    extern "C" fn no_industry(_: i32, _: i32, _: *mut u32) -> u32 {
        std::process::abort()
    }
    fn extension_shared() -> crate::services::Services {
        crate::services::Services {
            safe_waiting_position: safe,
            ..crate::services::fixture(std::ptr::null_mut(), no_random, no_industry)
        }
    }
    extern "C" fn try_track(_: *mut std::ffi::c_void, h: Handle, tile: u32, track: u8) -> u64 {
        // SAFETY: The callback ends this mock access before returning to policy.
        unsafe {
            let m = mock(h);
            m.log.push(("try", tile, track));
            u64::from(tile != 3 || m.stop == 1)
        }
    }
    extern "C" fn signal(_: *mut std::ffi::c_void, h: Handle, tile: u32, _: u8, green: u8) {
        // SAFETY: Live exclusive test mock.
        unsafe {
            mock(h).log.push(("signal", tile, green));
        }
    }
    extern "C" fn unreserve(_: *mut std::ffi::c_void, h: Handle, tile: u32, track: u8) {
        // SAFETY: Live exclusive test mock.
        unsafe {
            mock(h).log.push(("unreserve", tile, track));
        }
    }
    extern "C" fn mark(_: *mut std::ffi::c_void, h: Handle, tile: u32) {
        // SAFETY: Live exclusive test mock.
        unsafe {
            mock(h).log.push(("mark", tile, 0));
        }
    }
    fn extension_leaves(shared: &crate::services::Services) -> Leaves {
        Leaves {
            shared,
            blocking: zero_track,
            rail90: zero_pair,
            is_railway: zero_tile,
            free: one_track,
            has_pbs: one_track,
            green: one_track,
            try_track,
            set_signal: signal,
            unreserve,
            mark_tile: mark,
            origin,
            follow,
            ..leaves()
        }
    }
    #[test]
    fn controller_extension_rolls_back_only_new_tracks_and_restores_signal_order() {
        let mut m = Mock::new(0);
        let h = m.handle();
        let shared = extension_shared();
        let ls = extension_leaves(&shared);
        let g = Game {
            vehicle: std::ptr::null_mut(),
            leaves: &ls,
            context: std::ptr::null_mut(),
        };
        let mut available = 0;
        let mut dir = 0;
        assert_eq!(extend(&g, h, &mut available, &mut dir).tile, INVALID);
        assert_eq!(
            m.log,
            vec![
                ("try", 1, 0),
                ("signal", 1, 0),
                ("mark", 1, 0),
                ("try", 2, 0),
                ("signal", 2, 0),
                ("mark", 2, 0),
                ("try", 3, 0),
                ("unreserve", 1, 0),
                ("unreserve", 2, 0),
                ("signal", 1, 1),
                ("signal", 2, 1)
            ]
        );
    }
    #[test]
    fn controller_extension_accepts_safe_position_or_end_of_line_without_rollback() {
        for stop in [1, 2] {
            let mut m = Mock::new(stop);
            let h = m.handle();
            let shared = extension_shared();
            let ls = extension_leaves(&shared);
            let g = Game {
                vehicle: std::ptr::null_mut(),
                leaves: &ls,
                context: std::ptr::null_mut(),
            };
            let mut available = 0;
            let mut dir = 0;
            let result = extend(&g, h, &mut available, &mut dir);
            assert_eq!(
                (result.tile, result.td, result.okay),
                (if stop == 1 { 3 } else { 2 }, 0, 1)
            );
            assert!(
                !m.log
                    .iter()
                    .any(|&(op, _, value)| op == "unreserve" || op == "signal" && value == 1)
            );
        }
    }
    extern "C" fn read_order(h: Handle) -> TrainReservationNewRead {
        // SAFETY: Live test mock, no reference escapes.
        unsafe {
            let m = mock(h);
            TrainReservationNewRead {
                dest: m.dest,
                last_station: m.last,
                order_index: 254,
                suppress: m.suppress,
            }
        }
    }
    struct OrderContext {
        saved: u32,
    }
    extern "C" fn save_order(context: *mut std::ffi::c_void, h: Handle) {
        // SAFETY: Each entry receives a live distinct stack context and this snapshot borrow ends here.
        unsafe {
            (*context.cast::<OrderContext>()).saved = mock(h).order_token;
        }
    }
    extern "C" fn restore_order(context: *mut std::ffi::c_void, h: Handle) {
        // SAFETY: The corresponding native stack snapshot and mock are live; no borrow spans a callback.
        unsafe {
            let token = (*context.cast::<OrderContext>()).saved;
            let m = mock(h);
            m.order_token = token;
            m.restores += 1;
        }
    }
    extern "C" fn write_dest(_: *mut std::ffi::c_void, h: Handle, dest: u32) {
        // SAFETY: Live exclusive mock.
        unsafe {
            mock(h).dest = dest;
        }
    }
    extern "C" fn write_last(_: *mut std::ffi::c_void, h: Handle, last: u16) {
        // SAFETY: Live exclusive mock.
        unsafe {
            mock(h).last = last;
        }
    }
    extern "C" fn write_suppress(_: *mut std::ffi::c_void, h: Handle, suppress: u8) {
        // SAFETY: Live exclusive mock.
        unsafe {
            mock(h).suppress = suppress;
        }
    }
    fn order_leaves() -> Leaves {
        Leaves {
            read_new: read_order,
            save_order,
            restore_order,
            write_dest,
            write_last,
            write_suppress,
            ..leaves()
        }
    }
    #[test]
    fn explicit_or_drop_order_restoration_runs_once_with_nested_stack_contexts() {
        for explicit in [false, true] {
            let mut m = Mock::new(0);
            let h = m.handle();
            let ls = order_leaves();
            let mut outer_context = OrderContext { saved: 0 };
            let g = Game {
                vehicle: std::ptr::null_mut(),
                leaves: &ls,
                context: std::ptr::from_mut(&mut outer_context).cast(),
            };
            {
                let mut outer = Orders::new(&g, h);
                m.order_token = 2;
                m.dest = 42;
                m.last = 8;
                m.suppress = 0;
                {
                    let mut inner_context = OrderContext { saved: 0 };
                    let inner_game = Game {
                        vehicle: std::ptr::null_mut(),
                        leaves: &ls,
                        context: std::ptr::from_mut(&mut inner_context).cast(),
                    };
                    let mut inner = Orders::new(&inner_game, h);
                    m.order_token = 3;
                    m.dest = 99;
                    inner.restore();
                }
                assert_eq!((m.dest, m.last, m.suppress, m.order_token), (42, 8, 0, 2));
                assert_eq!(outer.index, 254);
                if explicit {
                    outer.restore();
                }
            }
            assert_eq!((m.dest, m.last, m.suppress, m.order_token), (9, 7, 1, 1));
            assert_eq!(m.restores, 2);
        }
    }
    #[test]
    fn direct_station_reservation_can_reenter_the_same_owner() {
        extern "C" fn nested(_: *mut std::ffi::c_void, h: Handle, _: u32, _: u8) -> u64 {
            extern "C" fn line_reverse(_: *mut std::ffi::c_void, _: Handle) -> u64 {
                0
            }
            let ls = Leaves {
                line_reverse,
                ..leaves()
            };
            h.set_track(u64::from(DEPOT));
            // SAFETY: Nested direct entry receives the same live owner and its own native stack context.
            assert_eq!(
                unsafe {
                    openttd_rust_train_reservation_check_reverse(
                        h,
                        crate::vehicle::VehicleRef(std::ptr::NonNull::dangling()),
                        &raw const ls,
                        std::ptr::null_mut(),
                    )
                },
                0
            );
            h.set_flags(1 << 8);
            h.clear_stuck();
            h.set_wait_counter(0x1_0000);
            h.set_flags(7);
            1
        }
        let mut m = Mock::new(0);
        let h = m.handle();
        let ls = Leaves {
            try_track: nested,
            ..leaves()
        };
        let g = Game {
            vehicle: std::ptr::null_mut(),
            leaves: &ls,
            context: std::ptr::null_mut(),
        };
        assert!(g.reserve_track(h, 7, 2));
        assert_eq!(h.get_flags(), 7);
        assert_eq!(m.state.wait_counter, 0);
    }
}
