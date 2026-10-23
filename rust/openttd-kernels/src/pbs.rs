/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete PBS policy over synchronous typed native leaves. No world reference
//! survives a call; map arrays and native shells remain canonical. Panics abort.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::collapsible_match
)]
use crate::services::{RailMapServices, Services};
use crate::tile_type::{MP_RAILWAY, MP_ROAD, MP_STATION, MP_TUNNELBRIDGE};
use crate::transport_type::TRANSPORT_RAIL;
#[repr(C)]
pub struct PbsServices {
    pub shared: *const Services,
    pub rail: *const RailMapServices,
    pub train: *const crate::train::Leaves,
    pub depot_bits: extern "C" fn(u32) -> crate::services::TrackBits,
    pub crossing_bits: extern "C" fn(u32) -> crate::services::TrackBits,
    pub station_bits: extern "C" fn(u32) -> crate::services::TrackBits,
    pub wormhole_bits: extern "C" fn(u32) -> crate::services::TrackBits,
    pub set_depot: extern "C" fn(u32, bool) -> (),
    pub set_station: extern "C" fn(u32, bool) -> (),
    pub set_wormhole: extern "C" fn(u32, bool) -> (),
    pub try_track: extern "C" fn(u32, u8) -> bool,
    pub unreserve_track: extern "C" fn(u32, u8) -> (),
    pub station_reserved: extern "C" fn(u32) -> bool,
    pub compatible_station: extern "C" fn(u32, u32) -> bool,
    pub show_reservation: extern "C" fn() -> bool,
    pub assert_track: extern "C" fn(u32, u8) -> (),
    pub assert_platform: extern "C" fn(u32, u8) -> (),
    pub station: extern "C" fn(u32) -> *mut std::ffi::c_void,
    pub randomisation: extern "C" fn(*mut std::ffi::c_void, u32) -> (),
    pub animation: extern "C" fn(*mut std::ffi::c_void, u32) -> (),
    pub is_crossing: extern "C" fn(u32) -> bool,
    pub read_train_on_res: extern "C" fn(*const TrainOutput) -> Handle,
    pub write_train_on_res: extern "C" fn(*mut TrainOutput, Handle),
    pub assertions: bool,
    pub handle: extern "C" fn(*const crate::services::Train) -> crate::train::Handle,
    pub all_compatible: extern "C" fn(u64) -> u64,
    pub compatible: extern "C" fn(u8) -> u64,
    pub tile_rail_type: extern "C" fn(u32) -> crate::services::RailType,
    pub rail90: extern "C" fn(u8, u8, bool) -> bool,
    pub blocking: extern "C" fn(u32, u8) -> bool,
    pub has_pbs: extern "C" fn(u32, u8) -> bool,
    pub index: extern "C" fn(*const std::ffi::c_void) -> u32,
    pub follow: unsafe extern "C" fn(
        crate::train::Handle,
        u8,
        u64,
        *mut crate::train_reservation::Follow,
    ) -> bool,
}
#[derive(Clone, Copy)]
pub(crate) struct Pbs<'a>(pub &'a PbsServices);
impl<'a> Pbs<'a> {
    pub(crate) fn acquire(shared: &'a Services) -> Self {
        // SAFETY: accessor returns the immutable services for this synchronous native entry.
        Self(unsafe { &*(shared.pbs_services)(shared) })
    }
    pub(crate) fn handle(self, v: *const crate::services::Train) -> Handle {
        (self.0.handle)(v)
    }

    fn shared(self) -> &'a Services {
        // SAFETY: tables have process lifetime; they contain operations, no world borrows.
        unsafe { &*self.0.shared }
    }
    fn rail(self) -> &'a RailMapServices {
        // SAFETY: same table contract.
        unsafe { &*self.0.rail }
    }
    pub(crate) fn reserved(self, tile: u32) -> u8 {
        let s = self.shared();
        let r = self.rail();
        match (s.tile_type)(tile) {
            MP_RAILWAY => {
                if (r.is_rail_depot)(tile) {
                    return (self.0.depot_bits)(tile).0;
                }
                if (s.is_plain_rail)(tile) {
                    return (r.rail_reservation)(tile).0;
                }
            }
            MP_ROAD => {
                if (self.0.is_crossing)(tile) {
                    return (self.0.crossing_bits)(tile).0;
                }
            }
            MP_STATION => {
                if (r.has_station_rail)(tile) {
                    return (self.0.station_bits)(tile).0;
                }
            }
            MP_TUNNELBRIDGE => {
                if (s.tunnel_transport)(tile) == TRANSPORT_RAIL {
                    return (self.0.wormhole_bits)(tile).0;
                }
            }
            _ => {}
        }
        0
    }
    pub(crate) fn platform(self, start: u32, dir: u8, reserved: bool) {
        (self.0.assert_platform)(start, dir);
        let map = crate::map_func::Map {
            log_x: (self.shared().map_log_x)() as u8,
        };
        let diff = map.offs_by_diag_dir(dir);
        let mut tile = start;
        loop {
            (self.0.set_station)(tile, reserved);
            (self.shared().mark_dirty)(tile);
            tile = tile.wrapping_add(diff);
            if !(self.0.compatible_station)(tile, start) {
                break;
            }
        }
    }
    fn dirty(self, tile: u32) {
        if (self.0.show_reservation)() {
            if (self.shared().is_bridge)(tile) {
                (self.shared().mark_bridge_dirty)(tile);
            } else {
                (self.shared().mark_dirty)(tile);
            }
        }
    }
    fn crossing(self, tile: u32, sound: bool) {
        // SAFETY: synchronous integrated owner, static service tables. No owner borrow crosses it.
        unsafe {
            crate::train::openttd_rust_train_update_crossing(
                tile,
                u8::from(sound),
                0,
                self.0.train,
                self.0.shared,
            );
        }
    }
    pub(crate) fn try_reserve(self, tile: u32, track: u8, trigger: bool) -> bool {
        (self.0.assert_track)(tile, track);
        self.dirty(tile);
        let s = self.shared();
        let r = self.rail();
        match (s.tile_type)(tile) {
            MP_RAILWAY => {
                if (s.is_plain_rail)(tile) {
                    return (self.0.try_track)(tile, track);
                }
                if (r.is_rail_depot)(tile) && !(r.depot_reserved)(tile) {
                    (self.0.set_depot)(tile, true);
                    (s.mark_dirty)(tile);
                    return true;
                }
            }
            MP_ROAD => {
                if (self.0.is_crossing)(tile) && !(s.crossing_reserved)(tile) {
                    (s.set_crossing_reserved)(tile, true);
                    self.crossing(tile, false);
                    return true;
                }
            }
            MP_STATION => {
                if (r.has_station_rail)(tile) && !(self.0.station_reserved)(tile) {
                    (self.0.set_station)(tile, true);
                    if trigger {
                        let station = (self.0.station)(tile);
                        (self.0.randomisation)(station, tile);
                        (self.0.animation)(station, tile);
                    }
                    (s.mark_dirty)(tile);
                    return true;
                }
            }
            MP_TUNNELBRIDGE => {
                if (s.tunnel_transport)(tile) == TRANSPORT_RAIL
                    && (self.0.wormhole_bits)(tile).0 == 0
                {
                    (self.0.set_wormhole)(tile, true);
                    return true;
                }
            }
            _ => {}
        }
        false
    }
    pub(crate) fn unreserve(self, tile: u32, track: u8) {
        (self.0.assert_track)(tile, track);
        self.dirty(tile);
        let s = self.shared();
        let r = self.rail();
        match (s.tile_type)(tile) {
            MP_RAILWAY => {
                if (r.is_rail_depot)(tile) {
                    (self.0.set_depot)(tile, false);
                    (s.mark_dirty)(tile);
                } else if (s.is_plain_rail)(tile) {
                    (self.0.unreserve_track)(tile, track);
                }
            }
            MP_ROAD => {
                if (self.0.is_crossing)(tile) {
                    (s.set_crossing_reserved)(tile, false);
                    self.crossing(tile, true);
                }
            }
            MP_STATION => {
                if (r.has_station_rail)(tile) {
                    (self.0.set_station)(tile, false);
                    (s.mark_dirty)(tile);
                }
            }
            MP_TUNNELBRIDGE => {
                if (s.tunnel_transport)(tile) == TRANSPORT_RAIL {
                    (self.0.set_wormhole)(tile, false);
                }
            }
            _ => {}
        }
    }
}
/// # Safety
/// The service tables and native world remain live for this synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pbs_reserved(s: *const PbsServices, tile: u32) -> u8 {
    Pbs(unsafe { &*s }).reserved(tile)
}
/// # Safety
/// Same synchronous service/world contract; platform has the source preconditions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pbs_platform(
    s: *const PbsServices,
    tile: u32,
    dir: u8,
    b: bool,
) {
    Pbs(unsafe { &*s }).platform(tile, dir, b);
}
/// # Safety
/// Same synchronous service/world contract; track has the source preconditions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pbs_try_reserve(
    s: *const PbsServices,
    tile: u32,
    track: u8,
    trigger: bool,
) -> bool {
    Pbs(unsafe { &*s }).try_reserve(tile, track, trigger)
}
/// # Safety
/// Same synchronous service/world contract; track has the source preconditions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pbs_unreserve(s: *const PbsServices, tile: u32, track: u8) {
    Pbs(unsafe { &*s }).unreserve(tile, track);
}

use crate::track_func::{
    diag_dir_to_diag_trackdir, diagdir_reaches_trackdirs, find_first_trackdir, reverse_trackdir,
    track_bits_to_trackdir_bits, track_overlaps_tracks, track_to_track_bits, track_to_trackdir,
    trackdir_bits_to_track_bits, trackdir_crosses_trackdirs, trackdir_to_exitdir,
    trackdir_to_track,
};
use crate::train::Handle;
use crate::train_reservation::{Follow, Pbs as TileInfo};
impl<'a> Pbs<'a> {
    pub(crate) fn has_reserved_tracks(self, tile: u32, tracks: u8) -> bool {
        self.reserved(tile) & tracks != 0
    }
    fn follow(self, v: Handle, owner: u8, types: u64, ft: &mut Follow, tile: u32, td: u8) -> bool {
        ft.old_tile = tile;
        ft.old_td = td;
        // SAFETY: live shell (or no vehicle), typed stack output; follower does not retain it.
        unsafe { (self.0.follow)(v, owner, types, ft) }
    }
    fn is_depot(self, tile: u32) -> bool {
        crate::rail_map::is_rail_depot_tile(self.shared(), self.rail(), tile)
    }
    fn block_signal(self, tile: u32, td: u8) -> bool {
        (self.shared().tile_type)(tile) == MP_RAILWAY
            && (self.rail().has_signal_trackdir)(tile, td)
            && !crate::rail_map::is_pbs_signal(
                (self.rail().signal_type)(tile, trackdir_to_track(td)).0,
            )
    }
    fn map(self) -> crate::map_func::Map {
        crate::map_func::Map {
            log_x: (self.shared().map_log_x)() as u8,
        }
    }
    fn follow_reservation(
        self,
        owner: u8,
        types: u64,
        mut tile: u32,
        mut td: u8,
        ignore_oneway: bool,
    ) -> TileInfo {
        let (mut start_tile, mut start_td) = (tile, td);
        let mut first_loop = true;
        if !self.has_reserved_tracks(tile, track_to_track_bits(trackdir_to_track(td))) {
            crate::witness::hit(crate::witness::PBS_START_UNRESERVED);
            return TileInfo {
                tile,
                td,
                okay: 0,
                other: Handle::NONE,
            };
        }
        let mut ft = Follow::default();
        while self.follow(Handle::NONE, owner, types, &mut ft, tile, td) {
            let reserved = ft.dirs & track_bits_to_trackdir_bits(self.reserved(ft.new_tile));
            if reserved == 0 {
                if ft.station != 0 {
                    let diff = self.map().offs_by_diag_dir(ft.exitdir);
                    while {
                        let skipped = ft.skipped;
                        ft.skipped = ft.skipped.wrapping_sub(1);
                        skipped > 0
                    } {
                        ft.new_tile = ft.new_tile.wrapping_sub(diff);
                        if (self.0.station_reserved)(ft.new_tile) {
                            tile = ft.new_tile;
                            td = diag_dir_to_diag_trackdir(ft.exitdir);
                            crate::witness::hit(crate::witness::PBS_PLATFORM_INTERIOR);
                            break;
                        }
                    }
                }
                break;
            }
            let new_td = find_first_trackdir(reserved);
            if !ignore_oneway && (self.0.blocking)(ft.new_tile, new_td) {
                crate::witness::hit(crate::witness::PBS_ONEWAY_STOP);
                break;
            }
            tile = ft.new_tile;
            td = new_td;
            if first_loop {
                start_tile = tile;
                start_td = td;
                first_loop = false;
            } else if tile == start_tile && td == start_td {
                crate::witness::hit(crate::witness::PBS_LOOP);
                break;
            }
            if self.is_depot(tile) {
                break;
            }
            if self.block_signal(tile, td) {
                break;
            }
        }
        crate::witness::hit(crate::witness::PBS_FOLLOW);
        TileInfo {
            tile,
            td,
            okay: 0,
            other: Handle::NONE,
        }
    }
    fn train(self) -> &'a crate::train::Leaves {
        unsafe { &*self.0.train }
    }
    fn check_trains(self, res: &TileInfo, tile: u32, best: &mut Handle) {
        struct Check<'a> {
            pbs: Pbs<'a>,
            td: u8,
            best: &'a mut Handle,
        }
        extern "C" fn visit(context: *mut std::ffi::c_void, t: Handle) -> u8 {
            // SAFETY: visitor and call-local selection state live through synchronous enumeration.
            let c = unsafe { &mut *context.cast::<Check<'_>>() };
            if (c.pbs.train().read_status)(t) & 128 != 0 {
                return 0;
            }
            let track = t.track();
            if track == crate::track_type::TRACK_BIT_WORMHOLE
                || track & track_to_track_bits(trackdir_to_track(c.td)) != 0
            {
                let t = (c.pbs.train().read_first)(t);
                if c.best.shell.is_null()
                    || (c.pbs.0.index)(t.shell) < (c.pbs.0.index)(c.best.shell)
                {
                    if !c.best.shell.is_null() {
                        crate::witness::hit(crate::witness::PBS_LOWER_ID);
                    }
                    *c.best = t;
                }
            }
            0
        }
        let mut check = Check {
            pbs: self,
            td: res.td,
            best,
        };
        (self.train().visit_tile)(tile, std::ptr::from_mut(&mut check).cast(), visit);
    }
    pub(crate) fn follow_train(self, v: Handle, check_other: bool, initial: Handle) -> TileInfo {
        let mut other = initial;
        let mut res = self.follow_train_output(v, check_other, &mut other);
        res.other = other;
        res
    }
    fn follow_train_output<O: Output>(
        self,
        v: Handle,
        check_other: bool,
        other: &mut O,
    ) -> TileInfo {
        let tile = (self.train().read_tile)(v);
        let td =
            unsafe { crate::train::openttd_rust_train_trackdir(v, self.0.train, self.0.shared) };
        if self.is_depot(tile) && (self.0.depot_bits)(tile).0 == 0 {
            return TileInfo {
                tile,
                td,
                okay: 0,
                other: Handle::NONE,
            };
        }
        let mut res = self.follow_reservation(
            (self.train().read_owner)(v),
            (self.0.all_compatible)(v.get_railtypes()),
            tile,
            td,
            false,
        );
        res.okay = u8::from(self.safe(v, res.tile, res.td, true, (self.shared().forbid_90_deg)()));
        if check_other {
            let mut best = Handle::NONE;
            self.check_trains(&res, res.tile, &mut best);
            if !best.shell.is_null() {
                other.write((self.train().read_first)(best));
            }
            if other.read().shell.is_null() && (self.rail().is_rail_station)(res.tile) {
                let diff = self
                    .map()
                    .offs_by_diag_dir(trackdir_to_exitdir(reverse_trackdir(res.td)));
                let mut st_tile = res.tile.wrapping_add(diff);
                while other.read().shell.is_null() && (self.0.compatible_station)(st_tile, res.tile)
                {
                    self.check_trains(&res, st_tile, &mut best);
                    if !best.shell.is_null() {
                        other.write((self.train().read_first)(best));
                    }
                    st_tile = st_tile.wrapping_add(diff);
                }
            }
            if other.read().shell.is_null()
                && (self.shared().tile_type)(res.tile) == MP_TUNNELBRIDGE
            {
                self.check_trains(
                    &res,
                    (self.rail().other_tunnel_bridge_end)(res.tile),
                    &mut best,
                );
                if !best.shell.is_null() {
                    other.write((self.train().read_first)(best));
                    crate::witness::hit(crate::witness::PBS_OTHER_END_TRAIN);
                }
            }
        }
        res
    }
    pub(crate) fn train_for(self, tile: u32, track: u8) -> Handle {
        if self.0.assertions {
            assert!(self.has_reserved_tracks(tile, track_to_track_bits(track)));
        }
        let mut td = track_to_trackdir(track);
        let types = (self.0.compatible)((self.0.tile_rail_type)(tile).0);
        for direction in 0..2 {
            if direction == 1 {
                td = reverse_trackdir(td);
                crate::witness::hit(crate::witness::PBS_SECOND_DIRECTION);
            }
            if (self.0.blocking)(tile, reverse_trackdir(td)) && !(self.0.has_pbs)(tile, td) {
                crate::witness::hit(crate::witness::PBS_SEARCH_ONEWAY_SKIP);
                continue;
            }
            let res =
                self.follow_reservation((self.shared().tile_owner)(tile), types, tile, td, true);
            let mut best = Handle::NONE;
            self.check_trains(&res, res.tile, &mut best);
            if !best.shell.is_null() {
                return best;
            }
            if (self.rail().is_rail_station)(res.tile) {
                let diff = self
                    .map()
                    .offs_by_diag_dir(trackdir_to_exitdir(reverse_trackdir(res.td)));
                let mut st_tile = res.tile.wrapping_add(diff);
                while (self.0.compatible_station)(st_tile, res.tile) {
                    self.check_trains(&res, st_tile, &mut best);
                    if !best.shell.is_null() {
                        crate::witness::hit(crate::witness::PBS_PLATFORM_TRAIN);
                        return best;
                    }
                    st_tile = st_tile.wrapping_add(diff);
                }
            }
            if (self.shared().tile_type)(res.tile) == MP_TUNNELBRIDGE {
                self.check_trains(
                    &res,
                    (self.rail().other_tunnel_bridge_end)(res.tile),
                    &mut best,
                );
                if !best.shell.is_null() {
                    crate::witness::hit(crate::witness::PBS_OTHER_END_TRAIN);
                    return best;
                }
            }
        }
        crate::witness::hit(crate::witness::PBS_STRAY);
        Handle::NONE
    }
    fn reachable(self, ft: &mut Follow, td: u8, no90: bool) {
        ft.dirs &= diagdir_reaches_trackdirs(ft.exitdir);
        if (self.0.rail90)(
            (self.0.tile_rail_type)(ft.old_tile).0,
            (self.0.tile_rail_type)(ft.new_tile).0,
            no90,
        ) {
            ft.dirs &= !trackdir_crosses_trackdirs(td);
        }
    }
    pub(crate) fn safe(self, v: Handle, tile: u32, td: u8, include_end: bool, no90: bool) -> bool {
        if self.is_depot(tile) {
            return true;
        }
        if self.block_signal(tile, td) {
            return true;
        }
        let mut ft = Follow::default();
        let types = (self.0.all_compatible)(v.get_railtypes());
        if !self.follow(v, 0, types, &mut ft, tile, td) && include_end {
            crate::witness::hit(crate::witness::PBS_SAFE_LINE_END);
            return true;
        }
        self.reachable(&mut ft, td, no90);
        if ft.dirs == 0 {
            return include_end;
        }
        if ft.dirs & ft.dirs.wrapping_sub(1) == 0 {
            let next_td = find_first_trackdir(ft.dirs);
            if (self.0.has_pbs)(ft.new_tile, next_td) {
                crate::witness::hit(crate::witness::PBS_SAFE_PBS);
                return true;
            }
            if (self.shared().tile_type)(ft.new_tile) == MP_RAILWAY
                && (self.rail().has_signal_trackdir)(ft.new_tile, reverse_trackdir(next_td))
                && (self.rail().signal_type)(ft.new_tile, trackdir_to_track(next_td)).0
                    == crate::signal_type::SIGTYPE_PBS_ONEWAY
            {
                crate::witness::hit(crate::witness::PBS_SAFE_OPPOSING_PBS);
                return include_end;
            }
        }
        false
    }
    pub(crate) fn free(self, v: Handle, tile: u32, td: u8, no90: bool) -> bool {
        if track_overlaps_tracks(self.reserved(tile), trackdir_to_track(td)) {
            crate::witness::hit(crate::witness::PBS_OVERLAP);
            return false;
        }
        if self.is_depot(tile) || self.block_signal(tile, td) {
            return true;
        }
        let mut ft = Follow::default();
        if !self.follow(v, 0, 0, &mut ft, tile, td) {
            return true;
        }
        self.reachable(&mut ft, td, no90);
        !self.has_reserved_tracks(ft.new_tile, trackdir_bits_to_track_bits(ft.dirs))
    }
}
/// # Safety
/// Live serialized train and service tables outlive the synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pbs_follow_train(
    s: *const PbsServices,
    v: Handle,
    output: *mut TrainOutput,
) -> TileInfo {
    let pbs = Pbs(unsafe { &*s });
    pbs.follow_train_output(v, !output.is_null(), &mut NativeOutput { pbs, output })
}
/// # Safety
/// Live world and tables; source requires a reserved track on this tile.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pbs_train_for(
    s: *const PbsServices,
    tile: u32,
    track: u8,
) -> Handle {
    Pbs(unsafe { &*s }).train_for(tile, track)
}
/// # Safety
/// Live train and tables; original tile and trackdir preconditions apply.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pbs_safe(
    s: *const PbsServices,
    v: Handle,
    tile: u32,
    td: u8,
    end: bool,
    no90: bool,
) -> bool {
    Pbs(unsafe { &*s }).safe(v, tile, td, end, no90)
}
/// # Safety
/// Live train and tables; original tile and trackdir preconditions apply.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pbs_free(
    s: *const PbsServices,
    v: Handle,
    tile: u32,
    td: u8,
    no90: bool,
) -> bool {
    Pbs(unsafe { &*s }).free(v, tile, td, no90)
}

/// Transient endpoint observation for the default unchanged-body gap check.
/// # Safety
/// Live serialized world/tables and original track follower preconditions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pbs_follow(
    s: *const PbsServices,
    owner: u8,
    types: u64,
    tile: u32,
    td: u8,
) -> TileInfo {
    Pbs(unsafe { &*s }).follow_reservation(owner, types, tile, td, false)
}

/// Opaque native output slot, never dereferenced or borrowed by Rust.
#[repr(C)]
pub struct TrainOutput {
    _private: [u8; 0],
}
trait Output {
    fn read(&self) -> Handle;
    fn write(&mut self, value: Handle);
}
impl Output for Handle {
    fn read(&self) -> Handle {
        *self
    }
    fn write(&mut self, value: Handle) {
        *self = value;
    }
}
struct NativeOutput<'a> {
    pbs: Pbs<'a>,
    output: *mut TrainOutput,
}
impl Output for NativeOutput<'_> {
    fn read(&self) -> Handle {
        (self.pbs.0.read_train_on_res)(self.output)
    }
    fn write(&mut self, value: Handle) {
        (self.pbs.0.write_train_on_res)(self.output, value);
    }
}
