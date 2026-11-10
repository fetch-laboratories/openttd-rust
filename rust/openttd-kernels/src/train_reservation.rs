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
use crate::order_type::{
    OT_CONDITIONAL, OT_DUMMY, OT_GOTO_DEPOT, OT_GOTO_STATION, OT_GOTO_WAYPOINT, OT_LEAVESTATION,
    OT_LOADING, OT_NOTHING,
};
use crate::raw_field::read_field;
use crate::tile_map::is_tile_type;
use crate::tile_type::INVALID_TILE;
use crate::tile_type::{MP_RAILWAY, MP_TUNNELBRIDGE};
use crate::track_func::{
    find_first_trackdir as first, track_bits_to_trackdir_bits as dirs,
    trackdir_bits_to_track_bits as tracks,
};
use crate::track_type::{TRACK_BIT_DEPOT as DEPOT, TRACK_BIT_WORMHOLE as WORMHOLE};

use crate::follow_track::State as Follower;
use crate::train::Handle;
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
            tile: INVALID_TILE,
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
    pub read_num_orders: extern "C" fn(Handle) -> u8,

    pub backoff: extern "C" fn() -> u8,
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
    pub green: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u8) -> u64,
    pub has_pbs: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u8) -> u64,
    pub has_signal: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u8) -> u64,
    pub increment_order: extern "C" fn(*mut std::ffi::c_void, Handle),
    pub is_bridge: extern "C" fn(*mut std::ffi::c_void, Handle, u32) -> u64,
    pub is_depot: extern "C" fn(*mut std::ffi::c_void, Handle, u32) -> u64,
    pub is_pbs: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u8) -> u64,
    pub is_plain: extern "C" fn(*mut std::ffi::c_void, Handle, u32) -> u64,
    pub is_station: extern "C" fn(*mut std::ffi::c_void, Handle, u32) -> u64,
    pub is_waypoint: extern "C" fn(*mut std::ffi::c_void, Handle, u32) -> u64,
    pub oneway: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u8) -> u64,
    pub order_service: extern "C" fn(*mut std::ffi::c_void, Handle, u8) -> u64,
    pub order_type: extern "C" fn(*mut std::ffi::c_void, Handle, u8) -> u64,
    pub other_end: extern "C" fn(*mut std::ffi::c_void, Handle, u32) -> u64,
    pub overlap: extern "C" fn(*mut std::ffi::c_void, Handle, u8) -> u64,
    pub path_result: extern "C" fn(*mut std::ffi::c_void, Handle, u8),
    pub rail90: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u32) -> u64,
    pub reach_dirs: extern "C" fn(*mut std::ffi::c_void, Handle, u8) -> u64,
    pub reach_tracks: extern "C" fn(*mut std::ffi::c_void, Handle, u8) -> u64,
    pub reserve_paths: extern "C" fn() -> bool,
    pub service: extern "C" fn(*mut std::ffi::c_void, Handle),
    pub set_depot: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u8),
    pub set_signal: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u8, u8),
    pub set_tunnel: extern "C" fn(*mut std::ffi::c_void, Handle, u32, u8),
    pub show_res: extern "C" fn(*mut std::ffi::c_void, Handle) -> u64,
    pub signal_context: extern "C" fn() -> crate::signal::Context,
    pub vehicle: *const crate::vehicle::VehicleServices,
    pub shared: *const crate::services::Services,
    pub start_stop: extern "C" fn(*mut std::ffi::c_void, Handle),
    pub stuck: extern "C" fn(*mut std::ffi::c_void, Handle),
    pub tile_offset: extern "C" fn(*mut std::ffi::c_void, Handle, u8) -> u64,
    pub trackdir: extern "C" fn(*mut std::ffi::c_void, Handle) -> u64,

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
    /// # Safety
    /// `leaves` is the entry's table, live and immutable for the call.
    unsafe fn entry(
        vehicle: crate::vehicle::VehicleRef,
        leaves: *const Leaves,
        context: *mut std::ffi::c_void,
    ) -> Self {
        Self {
            vehicle: vehicle.0.as_ptr(),
            // SAFETY: The function contract.
            leaves: unsafe { &*leaves },
            context,
        }
    }
    fn read_num_orders(&self, id: Handle) -> u8 {
        (self.leaves.read_num_orders)(id)
    }

    fn pbs(&self) -> crate::pbs::Pbs<'_> {
        crate::pbs::Pbs::acquire(self.shared())
    }
    fn free(&self, id: Handle, tile: u32, td: u8) -> bool {
        self.pbs()
            .free(id, tile, td, (self.shared().forbid_90_deg)())
    }
    fn has_reserved(&self, tile: u32, tracks: u8) -> bool {
        self.pbs().has_reserved_tracks(tile, tracks)
    }
    fn needs_service(&self, id: Handle) -> bool {
        // SAFETY: Live serialized train and immutable table; no owner borrow
        // spans the direct `Vehicle::NeedsServicing` port.
        unsafe { crate::vehicle::maintenance::needs_servicing(id.vehicle(), &*self.leaves.vehicle) }
    }
    fn order_stop(&self, id: Handle, station: u16) -> bool {
        crate::movement::should_stop(self.shared(), id.vehicle(), station)
    }
    fn reserved(&self, tile: u32) -> u8 {
        self.pbs().reserved(tile)
    }
    fn safe(&self, id: Handle, tile: u32, td: u8) -> bool {
        self.pbs()
            .safe(id, tile, td, true, (self.shared().forbid_90_deg)())
    }
    fn shared(&self) -> &crate::services::Services {
        // SAFETY: The entry's shared table lives for the process (or the test fixture's scope).
        unsafe { &*self.leaves.shared }
    }
    fn set_platform(&self, tile: u32, dir: u8, reserved: bool) {
        self.pbs().platform(tile, dir, reserved);
    }
    fn signal_buffer(&self, tile: u32, td: u8) {
        // SAFETY: C++ supplies the adjusted Vehicle base of the entry train.
        // The live shell and immutable table survive this call; no owner borrow
        // spans the shared callback. free() retains that entry train.
        let owner = unsafe {
            let vehicle =
                crate::vehicle::VehicleRef(std::ptr::NonNull::new_unchecked(self.vehicle));
            vehicle.owner()
        };
        crate::signal::count_context_owner_read();
        crate::signal::acquire_context(self.leaves.signal_context).add_side(tile, td, owner);
    }
    fn track_status(&self, tile: u32) -> u16 {
        crate::track_func::track_status_to_trackdir_bits(crate::movement::track_status(
            self.shared(),
            tile,
            0,
            0,
            255,
        ))
    }
    fn tunnel_free(&self, id: Handle, tile: u32, end: u32) -> bool {
        // SAFETY: The current train and all offered shells survive this query.
        unsafe {
            crate::vehicle::occupancy::wormhole_blocker(
                &*self.leaves.vehicle,
                tile,
                end,
                id.vehicle().0.as_ptr(),
            )
            .is_none()
        }
    }
    fn update_buffer(&self) {
        crate::signal::acquire_context(self.leaves.signal_context).flush();
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

    fn follower(&self, id: Handle, types: u64) -> Follower {
        Follower::rail(self.shared(), id, 0, types)
    }
    fn origin(&self, id: Handle, other: bool) -> Pbs {
        self.pbs().follow_train(id, other, Handle::NONE)
    }
    fn dir(&self, id: Handle, td: u8) -> u8 {
        (self.leaves.exit_dir)(self.context, id, td) as u8
    }
    fn signal(&self, id: Handle, tile: u32, td: u8, green: bool) {
        (self.leaves.set_signal)(self.context, id, tile, td, u8::from(green));
    }
    fn reserve_track(&self, tile: u32, track: u8) -> bool {
        self.pbs().try_reserve(tile, track, true)
    }
    /// `TryReserveRailTrack(tile, TrackdirToTrack(td))`.
    fn reserve(&self, tile: u32, td: u8) -> bool {
        self.reserve_track(tile, td & 7)
    }
    /// `UnreserveRailTrack(tile, TrackdirToTrack(td))`.
    fn unreserve(&self, tile: u32, td: u8) {
        self.pbs().unreserve(tile, td & 7);
    }

    fn td(&self, id: Handle) -> u8 {
        (self.leaves.trackdir)(self.context, id) as u8
    }
}
/// `Train::ReserveTrackUnderConsist`: tries to reserve track under whole train consist.
fn reserve_track_under_consist(g: &Game, id: Handle) {
    let mut part = id;
    while part != Handle::NONE {
        let v_tile: u32 = part.vehicle().tile();
        let track = part.get_track() as u8;
        match track {
            WORMHOLE => {
                g.reserve_track(
                    v_tile,
                    (g.leaves.diag_track)(
                        g.context,
                        part,
                        crate::map_func::World::new(g.shared()).tunnel_bridge_dir(v_tile),
                    ) as u8,
                );
            }
            DEPOT => {}
            _ => {
                g.reserve_track(v_tile, (g.leaves.bits_track)(g.context, part, track) as u8);
            }
        }
        part = part.next();
    }
}
fn single(bits: u16) -> bool {
    bits & bits.wrapping_sub(1) == 0
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
    if is_tile_type(g.shared(), tile, MP_TUNNELBRIDGE) {
        if crate::map_func::World::new(g.shared()).tunnel_bridge_dir(tile) == dir ^ 2 {
            let end = (g.leaves.other_end)(g.context, id, tile) as u32;
            if g.tunnel_free(id, tile, end) {
                (g.leaves.set_tunnel)(g.context, id, tile, 0);
                (g.leaves.set_tunnel)(g.context, id, end, 0);
                if (g.leaves.show_res)(g.context, id) != 0 {
                    if (g.leaves.is_bridge)(g.context, id, tile) != 0 {
                        (g.shared().mark_bridge_dirty)(tile);
                    } else {
                        (g.shared().mark_dirty)(tile);
                        (g.shared().mark_dirty)(end);
                    }
                }
            }
        }
    } else if (g.leaves.is_station)(g.context, id, tile) != 0 {
        let next = g.shared().map().tile_add_by_diag_dir_raw(tile, dir);
        if (g.leaves.compat_station)(g.context, id, next, tile) == 0 {
            g.set_platform(tile, dir ^ 2, false);
        }
    } else {
        g.unreserve(tile, td);
    }
}
#[allow(clippy::nonminimal_bool)] // `FreeTrainTrackReservation`'s condition as written.
fn free(g: &Game, id: Handle) {
    crate::witness::hit(crate::witness::TRAIN_FREE_PATH);
    if g.pbs().0.assertions {
        crate::movement::native_assert(g.shared(), id.front() != 0);
    }
    let v_tile: u32 = id.vehicle().tile();
    let mut tile = v_tile;
    let mut td = g.td(id);
    let mut free_tile = !(((g.leaves.is_station)(g.context, id, tile) != 0)
        || (is_tile_type(g.shared(), tile, MP_TUNNELBRIDGE)));
    let station = if (g.leaves.is_station)(g.context, id, tile) != 0 {
        crate::map_func::World::new(g.shared())
            .get_station_index(crate::tile_type::TileIndex(tile))
            .0
    } else {
        u16::MAX
    };
    if ((g.leaves.is_depot)(g.context, id, tile) != 0)
        && g.dir(id, td) != (g.leaves.depot_dir)(g.context, id, tile) as u8
    {
        return;
    }
    if id.get_track() == u64::from(DEPOT) {
        let mut u = id;
        while u != Handle::NONE {
            let part_tile: u32 = u.vehicle().tile();
            let part_next: Handle = u.next();
            if u.get_track() != u64::from(DEPOT) || part_tile != v_tile {
                return;
            }
            u = part_next;
        }
    }
    let reserved = g.reserved(tile);
    if (g.leaves.overlap)(g.context, id, reserved | (1 << (td & 7))) != 0 {
        return;
    }
    let types = (g.shared().all_compatible_railtypes)(id.get_railtypes());
    let mut ft = g.follower(id, types);
    while ft.follow(g.shared(), tile, td) {
        tile = ft.new_tile.0;
        let bits = ft.new_td_bits & dirs(g.reserved(tile));
        td = first(bits);
        crate::movement::native_assert(g.shared(), single(bits));
        if td == 0xff {
            break;
        }
        if is_tile_type(g.shared(), tile, MP_RAILWAY) {
            if ((g.leaves.has_signal)(g.context, id, tile, td) != 0)
                && ((g.leaves.is_pbs)(g.context, id, tile, td & 7) == 0)
            {
                g.unreserve(tile, td);
                break;
            }
            if (g.leaves.has_pbs)(g.context, id, tile, td) != 0 {
                if (g.leaves.green)(g.context, id, tile, td) == 0 {
                    break;
                }
                g.signal(id, tile, td, false);
                (g.shared().mark_dirty)(tile);
            } else if (g.leaves.has_pbs)(g.context, id, tile, td ^ 8) != 0 {
                g.signal_buffer(tile, g.dir(id, td ^ 8));
            } else if ((g.leaves.has_signal)(g.context, id, tile, td ^ 8) != 0)
                && ((g.leaves.oneway)(g.context, id, tile, td & 7) != 0)
            {
                break;
            }
        }
        if free_tile
            || (!(ft.is_station
                && crate::map_func::World::new(g.shared())
                    .get_station_index(crate::tile_type::TileIndex(ft.new_tile.0))
                    .0
                    == station)
                && !ft.is_tunnel
                && !ft.is_bridge)
        {
            clear(g, id, tile, td);
        }
        free_tile = true;
    }
    g.update_buffer();
}
fn extend(g: &Game, id: Handle, new_tracks: &mut u8, enterdir: &mut u8) -> Pbs {
    crate::witness::hit(crate::witness::TRAIN_CONTROLLER_EXTENSION);
    extend_from(g, id, g.origin(id, false), new_tracks, enterdir)
}
/// `ExtendTrainReservation` after its `FollowTrainReservation`.
fn extend_from(g: &Game, id: Handle, origin: Pbs, new_tracks: &mut u8, enterdir: &mut u8) -> Pbs {
    let types = id.get_compatible_railtypes();
    let mut ft = g.follower(id, types);
    let mut red = Vec::new();
    let mut tile = origin.tile;
    let mut td = origin.td;
    while ft.follow(g.shared(), tile, td) {
        if single(ft.new_td_bits)
            && ((g.leaves.blocking)(g.context, id, ft.new_tile.0, first(ft.new_td_bits)) != 0)
        {
            break;
        }
        if (g.leaves.rail90)(g.context, id, ft.old_tile.0, ft.new_tile.0) != 0 {
            ft.new_td_bits &= !((g.leaves.cross_dirs)(g.context, id, ft.old_td) as u16);
            if ft.new_td_bits == 0 {
                break;
            }
        }
        let target = ft.is_station
            || ((is_tile_type(g.shared(), ft.new_tile.0, MP_RAILWAY))
                && ((g.leaves.is_plain)(g.context, id, ft.new_tile.0) == 0));
        if target || !single(ft.new_td_bits) {
            if g.has_reserved(
                ft.new_tile.0,
                tracks((g.leaves.reach_dirs)(g.context, id, ft.old_td) as u16),
            ) {
                break;
            }
            if ft.tiles_skipped != 0 {
                let off = (g.leaves.tile_offset)(g.context, id, ft.exitdir) as i32;
                ft.new_tile.0 = ft
                    .new_tile
                    .0
                    .wrapping_sub(off.wrapping_mul(ft.tiles_skipped) as u32);
            }
            *new_tracks = tracks(ft.new_td_bits);
            *enterdir = ft.exitdir;
            return Pbs {
                tile: ft.new_tile.0,
                td: ft.old_td,
                okay: 0,
                other: Handle::NONE,
            };
        }
        tile = ft.new_tile.0;
        td = first(ft.new_td_bits);
        let rev = td ^ 8;
        // SAFETY: The entry keeps the native train live; no owner borrow spans the callback.
        if g.safe(id, tile, td) {
            if !(g.free(id, tile, td) && g.reserve(tile, td)) {
                break;
            }
            if ((g.leaves.has_pbs)(g.context, id, tile, rev) != 0)
                && ((g.leaves.green)(g.context, id, tile, rev) != 0)
            {
                crate::witness::hit(crate::witness::TRAIN_EXTENSION_OPPOSING_RED);
                red.push((tile, rev));
                g.signal(id, tile, rev, false);
                (g.shared().mark_dirty)(tile);
            }
            return Pbs {
                tile,
                td,
                okay: 1,
                other: Handle::NONE,
            };
        }
        if !g.reserve(tile, td) {
            break;
        }
        if ((g.leaves.has_pbs)(g.context, id, tile, rev) != 0)
            && ((g.leaves.green)(g.context, id, tile, rev) != 0)
        {
            crate::witness::hit(crate::witness::TRAIN_EXTENSION_OPPOSING_RED);
            red.push((tile, rev));
            g.signal(id, tile, rev, false);
            (g.shared().mark_dirty)(tile);
        }
    }
    if ft.err == 1 || ft.err == 4 {
        return Pbs {
            tile: ft.old_tile.0,
            td: ft.old_td,
            okay: 1,
            other: Handle::NONE,
        };
    }
    crate::witness::hit(crate::witness::TRAIN_EXTENSION_FAIL);
    tile = origin.tile;
    td = origin.td;
    let stopped = ft.old_tile.0;
    let stopped_td = ft.old_td;
    while tile != stopped || td != stopped_td {
        if !ft.follow(g.shared(), tile, td) {
            break;
        }
        if (g.leaves.rail90)(g.context, id, ft.old_tile.0, ft.new_tile.0) != 0 {
            ft.new_td_bits &= !((g.leaves.cross_dirs)(g.context, id, ft.old_td) as u16);
            crate::movement::native_assert(g.shared(), ft.new_td_bits != 0);
        }
        crate::movement::native_assert(g.shared(), single(ft.new_td_bits));
        tile = ft.new_tile.0;
        td = first(ft.new_td_bits);
        crate::witness::hit(crate::witness::TRAIN_EXTENSION_ROLLBACK);
        g.unreserve(tile, td);
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
    saved_order: crate::orders::Order,
}
impl<'a> Orders<'a> {
    fn new(g: &Game<'a>, id: Handle) -> Self {
        let v_dest: u32 = id.vehicle().dest_tile();
        let v_last_station: u16 = id.vehicle().last_station();
        let v_order_index: u8 = id.vehicle().order_index();
        let v_suppress: u8 = u8::from(
            read_field!(id.vehicle().ground(), flags)
                & (1 << crate::ground_vehicle::GVF_SUPPRESS_IMPLICIT_ORDERS)
                != 0,
        );
        let saved_order = id.vehicle().current_order();
        Self {
            g: *g,
            id,
            dest: v_dest,
            last: v_last_station,
            suppress: v_suppress,
            index: v_order_index,
            restored: false,
            saved_order,
        }
    }
    fn restore(&mut self) {
        crate::witness::hit(crate::witness::TRAIN_ORDER_RESTORE);
        // SAFETY: The current order and this stack snapshot are live for this copy.
        unsafe {
            self.id
                .vehicle()
                .current_order_pointer()
                .write(self.saved_order);
        };
        self.id.vehicle().write_dest(self.dest);
        self.id.vehicle().write_last_station(self.last);
        self.id.set_suppress_implicit(self.suppress != 0);
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
            let kind = (g.leaves.order_type)(g.context, id, self.index) as u8;
            match kind {
                1 | 2 | 6 => {
                    if kind != 2
                        || (g.leaves.order_service)(g.context, id, self.index) == 0
                        || g.needs_service(id)
                    {
                        (g.leaves.copy_order)(g.context, id, self.index);
                        return (g.leaves.update_order_dest)(id, self.index) != 0;
                    }
                }
                7 => {
                    let next = (g.leaves.conditional)(g.context, id, self.index) as u8;
                    if next != 0xff {
                        depth += 1;
                        self.index = next;
                        if self.index != id.vehicle().order_index()
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
            if self.index == id.vehicle().order_index() || depth >= i32::from(g.read_num_orders(id))
            {
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
    let mut reserve = (g.leaves.reserve_paths)() || force;
    let mut changed = false;
    let mut final_dest = INVALID_TILE;
    let mut got = false;
    let res = g.reserved(tile) & (g.leaves.reach_tracks)(g.context, id, dir) as u8;
    if res != 0 {
        return (first(u16::from(res)), false);
    }
    if single(u16::from(available)) {
        let track = first(u16::from(available));
        if track != 0xff
            && ((g.leaves.has_pbs)(
                g.context,
                id,
                tile,
                (g.leaves.enter_td)(g.context, id, track, dir) as u8,
            ) != 0)
        {
            reserve = true;
            changed = true;
            g.signal(
                id,
                tile,
                (g.leaves.enter_td)(g.context, id, track, dir) as u8,
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
        if dest.tile == INVALID_TILE {
            if mark {
                (g.leaves.stuck)(g.context, id);
            }
            if changed {
                g.signal(
                    id,
                    tile,
                    (g.leaves.enter_td)(g.context, id, best, dir) as u8,
                    false,
                );
            }
            return (first(u16::from(available)), false);
        }
        if dest.okay != 0 {
            if changed {
                (g.shared().mark_dirty)(tile);
            }
            g.reserve(id.vehicle().tile(), g.td(id));
            return (best, true);
        }
        (g.leaves.service)(g.context, id);
        if matches!(
            id.vehicle().current_order().kind(),
            OT_GOTO_DEPOT | OT_DUMMY | OT_CONDITIONAL
        ) {
            (g.leaves.process_orders)(id);
        }
    }
    let mut orders = Orders::new(g, id);
    let v_tile: u32 = id.vehicle().tile();
    let v_dest: u32 = id.vehicle().dest_tile();
    let v_destination: u16 = id.vehicle().current_order().destination;
    let v_order: u8 = id.vehicle().current_order().kind();
    if v_order == OT_LEAVESTATION {
        orders.next(false);
    } else if v_order == OT_LOADING
        || (v_order != OT_GOTO_DEPOT
            && if v_order == OT_GOTO_STATION {
                ((g.leaves.is_station)(g.context, id, v_tile) != 0)
                    && v_destination
                        == crate::map_func::World::new(g.shared())
                            .get_station_index(crate::tile_type::TileIndex(v_tile))
                            .0
            } else {
                v_tile == v_dest
            })
    {
        orders.next(true);
    }
    if dest.tile != INVALID_TILE && dest.okay == 0 {
        let new_tile = dest.tile;
        let result = g.pathfind(id, new_tile, dest_dir, available, reserve, true);
        dest = pbs(result);
        final_dest = result.final_dest;
        if new_tile == tile {
            best = result.track;
        }
        (g.leaves.path_result)(g.context, id, result.found);
    }
    if !reserve {
        return (best, false);
    }
    if dest.tile != INVALID_TILE && dest.okay == 0 {
        if mark {
            (g.leaves.stuck)(g.context, id);
        }
        free(g, id);
        return (best, false);
    }
    if dest.tile == INVALID_TILE {
        let origin = g.origin(id, false);
        if g.safe_track(id, origin.tile, origin.td, false) {
            let res = g.reserved(tile) & (g.leaves.reach_tracks)(g.context, id, dir) as u8;
            best = first(u16::from(res));
            g.reserve(id.vehicle().tile(), g.td(id));
            got = true;
            if changed {
                (g.shared().mark_dirty)(tile);
            }
        } else {
            free(g, id);
            if mark {
                (g.leaves.stuck)(g.context, id);
            }
        }
        return (best, got);
    }
    got = true;
    // SAFETY: The entry keeps the native train live through reservation traversal.
    while !g.safe(id, dest.tile, dest.td) {
        let exit = g.dir(id, dest.td);
        let next = g.shared().map().tile_add_by_diag_dir_raw(dest.tile, exit);
        let mut reachable =
            tracks(g.track_status(next)) & (g.leaves.reach_tracks)(g.context, id, exit) as u8;
        if (g.leaves.rail90)(g.context, id, dest.tile, next) != 0 {
            reachable &= !((g.leaves.cross_tracks)(g.context, id, dest.td & 7) as u8);
        }
        if orders.next(true) {
            let current = pbs(g.pathfind(id, next, exit, reachable, true, false));
            if current.tile != INVALID_TILE {
                dest = current;
                if dest.okay != 0 {
                    continue;
                }
                free(g, id);
                if mark {
                    (g.leaves.stuck)(g.context, id);
                }
                got = false;
                changed = false;
                break;
            }
        }
        if !g.safe_track(id, dest.tile, dest.td, true) {
            free(g, id);
            if mark {
                (g.leaves.stuck)(g.context, id);
            }
            got = false;
            changed = false;
        }
        break;
    }
    g.reserve(id.vehicle().tile(), g.td(id));
    if changed {
        (g.shared().mark_dirty)(tile);
    }
    orders.restore();
    let v_order: u8 = id.vehicle().current_order().kind();
    let v_nearest: u8 = u8::from(
        id.vehicle().current_order().depot_action_flags() & crate::order_type::ODATF_NEAREST_DEPOT
            != 0,
    );
    if v_order == OT_GOTO_DEPOT
        && v_nearest != 0
        && final_dest != INVALID_TILE
        && ((g.leaves.is_depot)(g.context, id, final_dest) != 0)
    {
        let destination = (g.shared().depot_index)(final_dest);
        // SAFETY: The live current order belongs to this serialized train.
        unsafe { (*id.vehicle().current_order_pointer()).destination = destination };
        id.vehicle().write_dest(final_dest);
        (g.leaves.start_stop)(g.context, id);
    }
    (best, got)
}
fn try_path(g: &Game, id: Handle, mark: bool, first_okay: bool) -> bool {
    let v_tile: u32 = id.vehicle().tile();
    if id.get_track() == u64::from(DEPOT) {
        if (g.leaves.depot_reserved)(g.context, id, v_tile) != 0 {
            if mark {
                (g.leaves.stuck)(g.context, id);
            }
            return false;
        }
        let dir = (g.leaves.depot_dir)(g.context, id, v_tile) as u8;
        let next = g.shared().map().tile_add_by_diag_dir_raw(v_tile, dir);
        if g.has_reserved(next, (g.leaves.reach_tracks)(g.context, id, dir) as u8) {
            return false;
        }
    }
    let origin = g.origin(id, true);
    if origin.other != Handle::NONE && origin.other != id {
        if mark {
            (g.leaves.stuck)(g.context, id);
        }
        return false;
    }
    if origin.okay != 0 && (v_tile != origin.tile || first_okay) {
        if id.stuck() {
            (g.leaves.start_stop)(g.context, id);
        }
        id.clear_stuck();
        return true;
    }
    if id.get_track() == u64::from(DEPOT) {
        (g.leaves.set_depot)(g.context, id, v_tile, 1);
        if (g.leaves.show_res)(g.context, id) != 0 {
            (g.shared().mark_dirty)(v_tile);
        }
    }
    let exit = g.dir(id, origin.td);
    let new_tile = g.shared().map().tile_add_by_diag_dir_raw(origin.tile, exit);
    let mut reachable =
        tracks(g.track_status(new_tile) & (g.leaves.diag_reach_dirs)(g.context, id, exit) as u16);
    if (g.leaves.rail90)(g.context, id, origin.tile, new_tile) != 0 {
        reachable &= !((g.leaves.cross_tracks)(g.context, id, origin.td & 7) as u8);
    }
    let (_, made) = choose(g, id, new_tile, exit, reachable, true, mark);
    if !made {
        if id.get_track() == u64::from(DEPOT) {
            (g.leaves.set_depot)(g.context, id, id.vehicle().tile(), 0);
        }
        return false;
    }
    if id.stuck() {
        id.set_wait_counter(0);
        (g.leaves.start_stop)(g.context, id);
    }
    id.clear_stuck();
    true
}
fn check_next(g: &Game, id: Handle) {
    if (g.leaves.backoff)() == 255 || id.get_track() == u64::from(DEPOT) {
        return;
    }
    let v_tile: u32 = id.vehicle().tile();
    let v_dest: u32 = id.vehicle().dest_tile();
    let v_destination: u16 = id.vehicle().current_order().destination;
    let v_order: u8 = id.vehicle().current_order().kind();
    let v_num_orders: u8 = g.read_num_orders(id);
    match v_order {
        OT_GOTO_DEPOT => {
            if v_tile == v_dest {
                return;
            }
        }
        OT_GOTO_WAYPOINT => {
            if ((g.leaves.is_waypoint)(g.context, id, v_tile) != 0)
                && crate::map_func::World::new(g.shared())
                    .get_station_index(crate::tile_type::TileIndex(v_tile))
                    .0
                    == v_destination
            {
                (g.leaves.process_orders)(id);
            }
        }
        OT_NOTHING | OT_LOADING | OT_LEAVESTATION if v_num_orders > 0 => return,
        _ => {}
    }
    let tile = id.vehicle().tile();
    if ((g.leaves.is_station)(g.context, id, tile) != 0)
        && g.order_stop(
            id,
            crate::map_func::World::new(g.shared())
                .get_station_index(crate::tile_type::TileIndex(tile))
                .0,
        )
    {
        return;
    }
    let td = g.td(id);
    if (is_tile_type(g.shared(), tile, MP_RAILWAY))
        && ((g.leaves.has_signal)(g.context, id, tile, td) != 0)
        && ((g.leaves.is_pbs)(g.context, id, tile, td & 7) == 0)
        && ((g.leaves.green)(g.context, id, tile, td) == 0)
    {
        return;
    }
    let mut ft = g.follower(id, id.get_compatible_railtypes());
    if !ft.follow(g.shared(), tile, td) {
        return;
    }
    if !g.has_reserved(ft.new_tile.0, tracks(ft.new_td_bits))
        && single(ft.new_td_bits)
        && ((g.leaves.has_pbs)(g.context, id, ft.new_tile.0, first(ft.new_td_bits)) != 0)
    {
        let mut available = tracks(ft.new_td_bits);
        if (g.leaves.rail90)(g.context, id, ft.old_tile.0, ft.new_tile.0) != 0 {
            available &= !((g.leaves.cross_tracks)(g.context, id, ft.old_td & 7) as u8);
        }
        choose(g, id, ft.new_tile.0, ft.exitdir, available, false, false);
    }
}

/// Synchronous reservation policy; order snapshots live on the Rust stack.
/// # Safety
/// Leaves, context and shell are live for this call. No state borrow spans a callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reservation_check_next(
    id: Handle,
    vehicle: crate::vehicle::VehicleRef,
    leaves: *const Leaves,
    context: *mut std::ffi::c_void,
) {
    let g = unsafe { Game::entry(vehicle, leaves, context) };
    check_next(&g, id);
}

/// Synchronous reservation policy; order snapshots live on the Rust stack.
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
    let g = unsafe { Game::entry(vehicle, leaves, context) };
    clear(&g, id, tile, td);
}

/// Synchronous reservation policy; order snapshots live on the Rust stack.
/// # Safety
/// Leaves, context and shell are live for this call. No state borrow spans a callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reservation_free(
    id: Handle,
    vehicle: crate::vehicle::VehicleRef,
    leaves: *const Leaves,
    context: *mut std::ffi::c_void,
) {
    let g = unsafe { Game::entry(vehicle, leaves, context) };
    free(&g, id);
}

/// Synchronous reservation policy; order snapshots live on the Rust stack.
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
    let g = unsafe { Game::entry(vehicle, leaves, context) };
    let (track, got) = choose(&g, id, tile, dir, tracks, force != 0, mark != 0);
    Choice {
        track,
        reserved: u8::from(got),
    }
}

/// Synchronous reservation policy; order snapshots live on the Rust stack.
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
    let g = unsafe { Game::entry(vehicle, leaves, context) };
    u8::from(try_path(&g, id, mark != 0, first_okay != 0))
}

/// Synchronous reservation policy; order snapshots live on the Rust stack.
/// # Safety
/// Leaves, context and shell are live for this call. No state borrow spans a callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reservation_check_reverse(
    id: Handle,
    vehicle: crate::vehicle::VehicleRef,
    leaves: *const Leaves,
    context: *mut std::ffi::c_void,
) -> u8 {
    let g = unsafe { Game::entry(vehicle, leaves, context) };
    u8::from(check_reverse_train(&g, id))
}

/// `CheckReverseTrain`.
fn check_reverse_train(g: &Game, id: Handle) -> bool {
    // SAFETY: The catalog addresses the process-wide setting; one field-sized load.
    if unsafe { g.shared().settings().line_reverse_mode.read() }
        || id.get_track() == u64::from(DEPOT)
        || id.get_track() == u64::from(WORMHOLE)
        || id.vehicle().direction() & 1 == 0
    {
        return false;
    }
    (g.leaves.check_reverse)(g.context, id) != 0
}

/// Synchronous reservation policy; order snapshots live on the Rust stack.
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
    let g = unsafe { Game::entry(vehicle, leaves, context) };
    if station == id.vehicle().last_station() {
        id.vehicle().write_last_station(u16::MAX);
    }
    if (g.shared().station_facilities)(station) & crate::station_type::FACILITY_TRAIN == 0 {
        (g.leaves.increment_order)(g.context, id);
        0
    } else {
        (g.shared().station_xy)(station)
    }
}

/// Synchronous reservation policy; order snapshots live on the Rust stack.
/// # Safety
/// Leaves, context and shell are live for this call. No state borrow spans a callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reservation_reserve_under(
    id: Handle,
    vehicle: crate::vehicle::VehicleRef,
    leaves: *const Leaves,
    context: *mut std::ffi::c_void,
) {
    let g = unsafe { Game::entry(vehicle, leaves, context) };
    reserve_track_under_consist(&g, id);
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDTrainReservationPbs" ["probe"] => Pbs {
        tile,
        other,
        td,
        okay,
    };
    "OpenTTDTrainReservationSearch" ["probe"] => Search {
        track,
        tile,
        final_dest,
        td,
        found,
        okay,
    };
    "OpenTTDTrainReservationLeaves" ["probe"] => Leaves {
        read_num_orders,
        backoff,
        bits_track,
        blocking,
        check_reverse,
        compat_station,
        conditional,
        copy_order,
        cross_dirs,
        cross_tracks,
        depot_dir,
        depot_reserved,
        diag_reach_dirs,
        diag_track,
        enter_td,
        exit_dir,
        green,
        has_pbs,
        has_signal,
        increment_order,
        is_bridge,
        is_depot,
        is_pbs,
        is_plain,
        is_station,
        is_waypoint,
        oneway,
        order_service,
        order_type,
        other_end,
        overlap,
        path_result,
        rail90,
        reach_dirs,
        reach_tracks,
        reserve_paths,
        service,
        set_depot,
        set_signal,
        set_tunnel,
        show_res,
        signal_context,
        vehicle,
        shared,
        start_stop,
        stuck,
        tile_offset,
        trackdir,
        pathfind,
        safe_track,
        process_orders,
        update_order_dest,
    };
    "OpenTTDTrainReservationChoice" ["probe"] => Choice {
        track,
        reserved,
    };
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::pbs::PbsServices;
    use crate::services::RailMapServices;
    use crate::train_state::State;
    use std::cell::Cell;
    use std::ffi::c_void;

    /// An aborting callee of any fixture slot's signature.
    trait Unused {
        fn unused() -> Self;
    }
    macro_rules! unused_arity {
        ($($arg:ident),*) => {
            impl<R, $($arg),*> Unused for extern "C" fn($($arg),*) -> R {
                fn unused() -> Self {
                    #[allow(improper_ctypes_definitions)]
                    extern "C" fn abort<R, $($arg),*>($(_: $arg),*) -> R {
                        std::process::abort()
                    }
                    abort::<R, $($arg),*>
                }
            }
            impl<R, $($arg),*> Unused for unsafe extern "C" fn($($arg),*) -> R {
                fn unused() -> Self {
                    #[allow(improper_ctypes_definitions)]
                    unsafe extern "C" fn abort<R, $($arg),*>($(_: $arg),*) -> R {
                        std::process::abort()
                    }
                    abort::<R, $($arg),*>
                }
            }
        };
    }
    unused_arity!();
    unused_arity!(A);
    unused_arity!(A, B);
    unused_arity!(A, B, C);
    unused_arity!(A, B, C, D);
    unused_arity!(A, B, C, D, E);
    unused_arity!(A, B, C, D, E, F);
    fn unused<T: Unused>() -> T {
        T::unused()
    }

    fn leaves() -> Leaves {
        Leaves {
            read_num_orders: unused(),
            backoff: unused(),
            bits_track: unused(),
            blocking: unused(),
            check_reverse: unused(),
            compat_station: unused(),
            conditional: unused(),
            copy_order: unused(),
            cross_dirs: unused(),
            cross_tracks: unused(),
            depot_dir: unused(),
            depot_reserved: unused(),
            diag_reach_dirs: unused(),
            diag_track: unused(),
            enter_td: unused(),
            exit_dir: unused(),
            green: unused(),
            has_pbs: unused(),
            has_signal: unused(),
            increment_order: unused(),
            is_bridge: unused(),
            is_depot: unused(),
            is_pbs: unused(),
            is_plain: unused(),
            is_station: unused(),
            is_waypoint: unused(),
            oneway: unused(),
            order_service: unused(),
            order_type: unused(),
            other_end: unused(),
            overlap: unused(),
            path_result: unused(),
            rail90: unused(),
            reach_dirs: unused(),
            reach_tracks: unused(),
            reserve_paths: unused(),
            service: unused(),
            set_depot: unused(),
            set_signal: unused(),
            set_tunnel: unused(),
            show_res: unused(),
            signal_context: unused(),
            vehicle: std::ptr::null(),
            shared: std::ptr::null(),
            start_stop: unused(),
            stuck: unused(),
            tile_offset: unused(),
            trackdir: unused(),
            pathfind: unused(),
            safe_track: unused(),
            process_orders: unused(),
            update_order_dest: unused(),
        }
    }

    impl<T> Unused for *const T {
        fn unused() -> Self {
            std::ptr::null()
        }
    }
    impl<T> Unused for *mut T {
        fn unused() -> Self {
            std::ptr::null_mut()
        }
    }
    impl<T> Unused for Option<T> {
        fn unused() -> Self {
            None
        }
    }
    impl Unused for bool {
        fn unused() -> Self {
            false
        }
    }
    /// Movement table whose every service is unused and which has no map descriptor.
    pub(crate) fn movement_table() -> crate::movement::MovementServices {
        crate::movement::MovementServices {
            shared: unused(),
            orders: unused(),
            train: unused(),
            vehicle: unused(),
            tunnel_bridge: unused(),
            road_compatible: unused(),
            road_type_speed: unused(),
            road_kind: unused(),
            road_vehicle: unused(),
            resolve: unused(),
            depot_invalidate: unused(),
            native_assert: unused(),
            owned_land: unused(),
            object_foundation: unused(),
            house_foundation: unused(),
            industry_foundation: unused(),
            industry_callback: unused(),
        }
    }
    fn vehicle_table() -> crate::vehicle::VehicleServices {
        crate::vehicle::VehicleServices {
            record_from_shell: resolve_record,
            assert_next: unused(),
            tile_links: unused(),
            map_log_x: unused(),
            assertions: unused(),
            train_handle: unused(),
            waiting_for_unbunching: unused(),
            leave_unbunching_depot: unused(),
            reset_depot_unbunching: unused(),
            enter_depot: unused(),
            pathfinding_result: unused(),
            new_position: unused(),
            base_viewport: unused(),
            newgrf: unused(),
            property: unused(),
            callback: unused(),
            set_max_age: unused(),
            length_changed: unused(),
            play_vehicle_sound: unused(),
            play_sound: unused(),
            order_free: unused(),
            order_dummy: unused(),
            start_stop_dirty: unused(),
            destroy: unused(),
            set_dest_tile: unused(),
            engine: unused(),
            order_type: unused(),
            order_destination: unused(),
            order_nonstop: unused(),
            write_profit: unused(),
            write_economy_service: unused(),
            write_calendar_service: unused(),
            write_breakdowns: unused(),
            write_reliability: unused(),
            profit: unused(),
            make_dummy: unused(),
            check_orders_only: unused(),
            maintenance: unused(),
            visits: unused(),
            orders: unused(),
            ship_get: unused(),
            first_sprite: unused(),
            update_delta: unused(),
            can_allocate: unused(),
        }
    }
    extern "C" fn resolve_record(shell: *mut c_void) -> *mut crate::vehicle_base::VehicleRecord {
        // SAFETY: Test handles retain their exclusive stack Mock for each query.
        unsafe { &raw mut *(*shell.cast::<Mock>()).record }
    }
    struct Mock {
        record: Box<crate::vehicle_base::VehicleRecord>,
        state: State,
        stop: u8,
        log: Vec<(&'static str, u32, u8)>,
        ground: crate::ground_vehicle::GroundState,
        consist: crate::orders::Consist,
        orders: crate::orders::VehicleOrders,
    }
    impl Mock {
        fn new(stop: u8) -> Self {
            Self {
                // SAFETY: Transfer the private record allocation to this test owner.
                record: unsafe {
                    Box::from_raw(crate::vehicle_base::openttd_rust_vehicle_record_new(
                        std::ptr::null_mut(),
                        crate::vehicle::VehicleType::TRAIN,
                    ))
                },
                state: State::default(),
                stop,
                log: Vec::new(),
                ground: crate::ground_vehicle::GroundState {
                    flags: 4,
                    ..Default::default()
                },
                consist: crate::orders::Consist {
                    time: 0,
                    lateness: 0,
                    start: 0,
                    last_departure: 0,
                    next_departure: 0,
                    round_trip: 0,
                    real: 254,
                    implicit: 0,
                    flags: 0,
                },
                orders: crate::orders::VehicleOrders {
                    current: std::mem::MaybeUninit::new(crate::orders::Order {
                        kind: 0x12,
                        flags: 0x34,
                        destination: 1,
                        refit: 0x56,
                        wait: 0,
                        travel: 0x789a,
                        speed: 0xbcde,
                    }),
                    orders: std::ptr::null_mut(),
                    next: std::ptr::null_mut(),
                    previous: std::ptr::null_mut(),
                },
            }
        }
        fn set_order_token(&mut self, token: u32) {
            // SAFETY: The fixture initializes this Order before every access.
            let order = unsafe { self.orders.current.assume_init_mut() };
            order.destination = token as u16;
            order.wait = (token >> 16) as u16;
        }
        fn order_token(&self) -> u32 {
            // SAFETY: The fixture initializes this Order before every access.
            let order = unsafe { self.orders.current.assume_init_ref() };
            u32::from(order.destination) | (u32::from(order.wait) << 16)
        }
        fn handle(&mut self) -> Handle {
            self.record.dest_tile = crate::tile_type::TileIndex(9);
            self.record.last_station_visited = crate::station_type::StationID(7);
            self.record.ground = &raw mut self.ground;
            self.record.consist = &raw mut self.consist;
            self.record.orders = &raw mut self.orders;
            self.record.train = &raw mut self.state;
            self.record.owner = crate::company_type::CompanyID(0);
            self.record.shell = std::ptr::from_mut(self).cast();
            Handle {
                record: &raw mut *self.record,
                owner: &raw mut self.state,
            }
        }
    }
    unsafe fn mock<'a>(h: Handle) -> &'a mut Mock {
        // SAFETY: Every test installs this handle to its live exclusive stack mock.
        unsafe { &mut *h.vehicle().shell().cast::<Mock>() }
    }

    // A PBS service has no vehicle argument; each test thread names its world here.
    thread_local! {
        static TRAIN: Cell<Handle> = const { Cell::new(Handle::NONE) };
        static PBS: Cell<*const PbsServices> = const { Cell::new(std::ptr::null()) };
        static WORLD: Cell<*const World> = const { Cell::new(std::ptr::null()) };
    }
    fn current<'a>() -> &'a mut Mock {
        // SAFETY: The installing test keeps this mock live; callbacks end each access.
        unsafe { mock(TRAIN.get()) }
    }
    /// One railway world: every tile is plain unsignalled, unreserved rail.
    struct World {
        shared: crate::services::Services,
        rail: RailMapServices,
        pbs: PbsServices,
        movement: crate::movement::MovementServices,
        vehicle: crate::vehicle::VehicleServices,
        base: Vec<crate::map_func::BaseTileLayout>,
        extended: Vec<crate::map_func::ExtendedTileLayout>,
    }
    extern "C" fn pbs_services(_: *const crate::services::Services) -> *const PbsServices {
        PBS.get()
    }
    extern "C" fn railway(_: u32) -> u8 {
        crate::tile_type::MP_RAILWAY
    }
    extern "C" fn yes(_: u32) -> bool {
        true
    }
    extern "C" fn no_signal(_: u32, _: u8) -> bool {
        false
    }
    extern "C" fn no_reservation(_: u32) -> crate::services::TrackBits {
        crate::services::TrackBits(0)
    }
    extern "C" fn no_show() -> bool {
        false
    }
    extern "C" fn every_type(_: u64) -> u64 {
        1
    }
    extern "C" fn same_types(_: u8, _: u8, _: bool) -> bool {
        false
    }
    extern "C" fn try_track(tile: u32, track: u8) -> bool {
        let m = current();
        m.log.push(("try", ordinal(tile), track));
        ordinal(tile) != 3 || m.stop == 1
    }
    extern "C" fn unreserve_track(tile: u32, track: u8) {
        current().log.push(("unreserve", ordinal(tile), track));
    }
    extern "C" fn no_random(_: *mut c_void) -> u32 {
        std::process::abort()
    }
    extern "C" fn no_industry(_: i32, _: i32, _: *mut u32) -> u32 {
        std::process::abort()
    }
    impl World {
        fn new() -> Box<Self> {
            let mut world = Box::new(Self {
                shared: crate::services::Services {
                    pbs_services,
                    all_compatible_railtypes: every_type,
                    tile_type: railway,
                    is_plain_rail: yes,
                    mark_dirty: mark,
                    ..crate::services::fixture(std::ptr::null_mut(), no_random, no_industry)
                },
                rail: rail_map(),
                pbs: pbs_table(),
                movement: movement_table(),
                vehicle: vehicle_table(),
                base: (0..4096)
                    .map(|_| crate::map_func::BaseTileLayout {
                        kind: 0,
                        height: 0,
                        m2: 0,
                        m1: 0,
                        m3: 0,
                        m4: 0,
                        m5: 0,
                    })
                    .collect(),
                extended: (0..4096)
                    .map(|_| crate::map_func::ExtendedTileLayout {
                        m6: 0,
                        m7: 0,
                        m8: 0,
                    })
                    .collect(),
            });
            for ordinal in 0..=4 {
                world.base[index(ordinal) as usize].kind = crate::tile_type::MP_RAILWAY << 4;
                world.base[index(ordinal) as usize].m5 = crate::track_type::TRACK_BIT_X;
            }
            world.movement.vehicle = &raw const world.vehicle;
            world.movement.shared = &raw const world.shared;
            world.shared.movement = &raw const world.movement;
            world.pbs.shared = &raw const world.shared;
            world.pbs.rail = &raw const world.rail;
            world
        }
        /// Route this thread's PBS queries to the world and to `train`'s mock.
        fn install(&mut self, train: Handle) {
            let stop = unsafe { mock(train) }.stop;
            self.base[index(if stop == 2 { 3 } else { 4 }) as usize].m5 = 0;
            WORLD.set(std::ptr::from_ref(self));
            crate::map_func::fixture::set(
                crate::map_func::TileTypeAllocation {
                    base: self.base.as_ptr(),
                    extended: self.extended.as_ptr(),
                    count: self.base.len() as u32,
                    assertions: false,
                },
                6,
            );
            TRAIN.set(train);
            PBS.set(&raw const self.pbs);
        }
    }
    fn rail_map() -> RailMapServices {
        RailMapServices {
            ground: std::ptr::null(),
            traversal: unused(),
            station_admission: unused(),
            wormhole_admission: unused(),
            set_signal_state: unused(),
            other_tunnel_bridge_end: unused(),
            invalid_rail_type: unused(),
            set_rail_type: unused(),
            set_track_bits: unused(),
            set_present_signals: unused(),
            set_signal_states: unused(),
            set_has_signals: unused(),
            set_signal_type: unused(),
            signal_variant: unused(),
            set_signal_variant: unused(),
            cycle_signal_side: unused(),
            rail_reservation: no_reservation,
            set_track_reservation: unused(),
            depot_reserved: unused(),
            set_depot_exit: unused(),
            make_rail_normal: unused(),
            make_rail_depot: unused(),
            wormhole_reserved: unused(),
        }
    }

    const fn index(ordinal: u32) -> u32 {
        72 - ordinal
    }
    const fn ordinal(tile: u32) -> u32 {
        72 - tile
    }
    fn pbs_table() -> PbsServices {
        PbsServices {
            shared: std::ptr::null(),
            rail: std::ptr::null(),
            train: std::ptr::null(),
            depot_bits: unused(),
            crossing_bits: unused(),
            station_bits: unused(),
            wormhole_bits: unused(),
            set_depot: unused(),
            set_station: unused(),
            set_wormhole: unused(),
            try_track,
            unreserve_track,
            show_reservation: no_show,
            assert_platform: unused(),
            station: unused(),
            is_crossing: unused(),
            read_train_on_res: unused(),
            write_train_on_res: unused(),
            assertions: false,
            handle: unused(),
            compatible: unused(),
            rail90: same_types,
            blocking: no_signal,
            has_pbs: no_signal,
            index: unused(),
        }
    }

    extern "C" fn is_plain(_: *mut c_void, _: Handle, tile: u32) -> u64 {
        u64::from(yes(tile))
    }
    extern "C" fn zero_pair(_: *mut c_void, _: Handle, _: u32, _: u32) -> u64 {
        0
    }
    extern "C" fn zero_track(_: *mut c_void, _: Handle, _: u32, _: u8) -> u64 {
        0
    }
    extern "C" fn one_track(_: *mut c_void, _: Handle, _: u32, _: u8) -> u64 {
        1
    }
    extern "C" fn signal(_: *mut c_void, h: Handle, tile: u32, _: u8, green: u8) {
        // SAFETY: Live exclusive test mock.
        unsafe {
            mock(h).log.push(("signal", ordinal(tile), green));
        }
    }
    extern "C" fn mark(tile: u32) {
        current().log.push(("mark", ordinal(tile), 0));
    }
    fn extension_leaves(shared: &crate::services::Services) -> Leaves {
        Leaves {
            shared,
            blocking: zero_track,
            rail90: zero_pair,
            is_plain,
            has_pbs: one_track,
            green: one_track,
            set_signal: signal,
            ..leaves()
        }
    }
    const ORIGIN: Pbs = Pbs {
        tile: index(0),
        other: Handle::NONE,
        td: 0,
        okay: 0,
    };
    #[test]
    fn controller_extension_rolls_back_only_new_tracks_and_restores_signal_order() {
        let mut m = Mock::new(0);
        let h = m.handle();
        let mut world = World::new();
        world.install(h);
        let ls = extension_leaves(&world.shared);
        let g = Game {
            vehicle: std::ptr::null_mut(),
            leaves: &ls,
            context: std::ptr::null_mut(),
        };
        let mut available = 0;
        let mut dir = 0;
        let result = extend_from(&g, h, ORIGIN, &mut available, &mut dir);
        assert_eq!(result.tile, INVALID_TILE);
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
            let mut world = World::new();
            world.install(h);
            let ls = extension_leaves(&world.shared);
            let g = Game {
                vehicle: std::ptr::null_mut(),
                leaves: &ls,
                context: std::ptr::null_mut(),
            };
            let mut available = 0;
            let mut dir = 0;
            let result = extend_from(&g, h, ORIGIN, &mut available, &mut dir);
            assert_eq!(
                (result.tile, result.td, result.okay),
                (index(if stop == 1 { 3 } else { 2 }), 0, 1)
            );
            assert!(
                !m.log
                    .iter()
                    .any(|&(op, _, value)| op == "unreserve" || op == "signal" && value == 1)
            );
        }
    }
    #[test]
    fn explicit_or_drop_order_restoration_runs_once_with_nested_snapshots() {
        for explicit in [false, true] {
            let mut m = Mock::new(0);
            m.set_order_token(0x89ab_cdef);
            let h = m.handle();
            let ls = leaves();
            let g = Game {
                vehicle: std::ptr::null_mut(),
                leaves: &ls,
                context: std::ptr::null_mut(),
            };
            {
                let mut outer = Orders::new(&g, h);
                m.set_order_token(0x7654_3210);
                m.record.dest_tile = crate::tile_type::TileIndex(42);
                m.record.last_station_visited = crate::station_type::StationID(8);
                m.ground.flags &= !4;
                {
                    let inner_game = Game {
                        vehicle: std::ptr::null_mut(),
                        leaves: &ls,
                        context: std::ptr::null_mut(),
                    };
                    let mut inner = Orders::new(&inner_game, h);
                    m.set_order_token(3);
                    m.record.dest_tile = crate::tile_type::TileIndex(99);
                    inner.restore();
                }
                assert_eq!(
                    (
                        m.record.dest_tile.0,
                        m.record.last_station_visited.0,
                        u8::from(m.ground.flags & 4 != 0),
                        m.order_token()
                    ),
                    (42, 8, 0, 0x7654_3210)
                );
                assert_eq!(outer.index, 254);
                if explicit {
                    outer.restore();
                    m.set_order_token(0x1357_2468);
                }
            }
            assert_eq!(
                (
                    m.record.dest_tile.0,
                    m.record.last_station_visited.0,
                    u8::from(m.ground.flags & 4 != 0),
                    m.order_token()
                ),
                (9, 7, 1, if explicit { 0x1357_2468 } else { 0x89ab_cdef })
            );
        }
    }
    #[test]
    fn direct_station_reservation_can_reenter_the_same_owner() {
        extern "C" fn nested(_: u32, _: u8) -> bool {
            let line_reverse = false;
            let settings = crate::settings_type::SettingAddresses {
                line_reverse_mode: &raw const line_reverse,
                ..crate::settings_type::SettingAddresses::NULL
            };
            let shared = crate::services::Services {
                settings: &raw const settings,
                ..crate::services::fixture(std::ptr::null_mut(), no_random, no_industry)
            };
            let ls = Leaves {
                shared: &raw const shared,
                ..leaves()
            };
            let h = TRAIN.get();
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
            true
        }
        let mut m = Mock::new(0);
        let h = m.handle();
        let mut world = World::new();
        world.pbs.try_track = nested;
        world.install(h);
        let ls = Leaves {
            shared: &raw const world.shared,
            ..leaves()
        };
        let g = Game {
            vehicle: std::ptr::null_mut(),
            leaves: &ls,
            context: std::ptr::null_mut(),
        };
        assert!(g.reserve_track(7, 2));
        assert_eq!(h.get_flags(), 7);
        assert_eq!(m.state.wait_counter, 0);
    }
}
