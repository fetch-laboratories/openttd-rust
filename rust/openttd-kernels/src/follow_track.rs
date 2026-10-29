/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete stack-owned `CFollowTrackT` rail, road and water follower.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_arguments
)]
use crate::map_func::World;
use crate::pbs::Pbs;
use crate::services::Services;
use crate::track_func::*;
use crate::train_reservation::Follow;
use crate::transport_type::{
    TRANSPORT_RAIL as RAIL, TRANSPORT_ROAD as ROAD, TRANSPORT_WATER as WATER,
};
use crate::vehicle::{OpenTTDVehicle, VehicleRef};
const EC_NONE: u8 = 0;
const EC_OWNER: u8 = 1;
const EC_RAIL_ROAD_TYPE: u8 = 2;
const EC_90DEG: u8 = 3;
const EC_NO_WAY: u8 = 4;
const EC_RESERVED: u8 = 5;

/// The complete follower state; C++ facades alias this stack record in place.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct State {
    pub veh: *mut OpenTTDVehicle,
    pub railtypes: u64,
    pub old_tile: u32,
    pub new_tile: u32,
    pub tiles_skipped: i32,
    pub new_td_bits: u16,
    pub veh_owner: u8,
    pub old_td: u8,
    pub exitdir: u8,
    pub is_tunnel: bool,
    pub is_bridge: bool,
    pub is_station: bool,
    pub err: u8,
    pub mode: u8,
    pub allow90: bool,
    pub masking: bool,
}
impl State {
    pub(crate) fn new(
        s: &Services,
        mode: u8,
        v: Option<VehicleRef>,
        owner: u8,
        railtypes: u64,
        allow90: bool,
        masking: bool,
    ) -> Self {
        let m = crate::movement::table(s);
        m.native_assert_call(mode != ROAD || v.is_some());
        m.native_assert_call(mode != RAIL || railtypes != u64::MAX);
        Self {
            veh: v.map_or(std::ptr::null_mut(), |v| v.0.as_ptr()),
            railtypes,
            old_tile: crate::tile_type::INVALID_TILE,
            new_tile: crate::tile_type::INVALID_TILE,
            tiles_skipped: 0,
            new_td_bits: 0,
            veh_owner: owner,
            old_td: crate::track_type::INVALID_TRACKDIR,
            exitdir: crate::direction_type::INVALID_DIAGDIR,
            is_tunnel: false,
            is_bridge: false,
            is_station: false,
            err: EC_NONE,
            mode,
            allow90,
            masking,
        }
    }
    pub(crate) fn vehicle(
        s: &Services,
        mode: u8,
        v: Option<VehicleRef>,
        types: u64,
        allow90: bool,
        masking: bool,
    ) -> Self {
        let m = crate::movement::table(s);
        m.native_assert_call(
            mode != RAIL
                || v.is_some_and(|v| {
                    (m.vehicle().vehicle_type)(v) == crate::vehicle_type::VEH_TRAIN
                }),
        );
        let owner = v.map_or(crate::company_type::INVALID_OWNER, |v| {
            (m.vehicle().owner)(v)
        });
        let types = if mode == RAIL && types == u64::MAX {
            (m.vehicle().train_handle)(v.unwrap().0.as_ptr()).get_compatible_railtypes()
        } else {
            types
        };
        Self::new(s, mode, v, owner, types, allow90, masking)
    }
    fn vehicle_ref(&self) -> VehicleRef {
        VehicleRef(std::ptr::NonNull::new(self.veh).expect("live vehicle"))
    }
    fn tram(&self, w: &World<'_>) -> bool {
        self.mode == ROAD && (crate::movement::table(w.shared).road_kind)(self.vehicle_ref()).0
    }
    fn single_tram(&self, w: &World<'_>, tile: u32) -> u8 {
        w.check(self.tram(w));
        if w.normal_road(tile) {
            use crate::road_type::*;
            return match w.road_bits(tile, true) {
                ROAD_NW => 3,
                ROAD_SW => 2,
                ROAD_SE => 1,
                ROAD_NE => 0,
                _ => crate::direction_type::INVALID_DIAGDIR,
            };
        }
        crate::direction_type::INVALID_DIAGDIR
    }
    fn query(&mut self, w: &World<'_>) -> bool {
        self.new_td_bits = if self.mode == RAIL && w.plain_rail(self.new_tile) {
            u16::from(w.tracks(self.new_tile)) * 0x101
        } else if self.mode == ROAD {
            crate::pathfinder_func::road_trackdirs(
                crate::movement::table(w.shared),
                self.new_tile,
                self.tram(w),
            )
        } else {
            track_status_to_trackdir_bits(crate::movement::track_status(
                w.shared,
                self.new_tile,
                self.mode,
                0,
                crate::direction_type::INVALID_DIAGDIR,
            ))
        };
        self.new_td_bits != 0
    }
    fn reverse_state(&mut self, dir: u8) {
        self.new_tile = self.old_tile;
        self.new_td_bits = trackdir_to_trackdir_bits(reverse_trackdir(self.old_td));
        self.exitdir = dir;
        self.tiles_skipped = 0;
        self.is_tunnel = false;
        self.is_bridge = false;
        self.is_station = false;
    }
    fn forced_reverse(&mut self, w: &World<'_>) -> bool {
        if self.mode != WATER && w.depot(self.old_tile, self.mode) {
            let dir = if self.mode == RAIL {
                w.rail_depot_dir(self.old_tile)
            } else {
                w.road_depot_dir(self.old_tile)
            };
            if dir != self.exitdir {
                self.reverse_state(dir);
                return true;
            }
        }
        if self.mode == ROAD
            && ((self.tram(w) && self.single_tram(w, self.old_tile) == (self.exitdir ^ 2))
                || (w.bay_stop(self.old_tile) && w.bay_dir(self.old_tile) == (self.exitdir ^ 2)))
        {
            self.reverse_state(self.exitdir ^ 2);
            return true;
        }
        false
    }
    fn try_reverse(&mut self, w: &World<'_>) -> bool {
        if self.mode == ROAD && !self.tram(w) {
            self.exitdir ^= 2;
            self.new_tile = self.old_tile;
            self.query(w);
            self.new_td_bits &= diagdir_reaches_trackdirs(self.exitdir);
            if self.new_td_bits != 0 {
                return true;
            }
        }
        self.err = EC_NO_WAY;
        false
    }
    fn can_exit(&mut self, w: &World<'_>) -> bool {
        if self.mode == ROAD
            && w.bay_stop(self.old_tile)
            && w.bay_dir(self.old_tile) != self.exitdir
        {
            self.err = EC_NO_WAY;
            return false;
        }
        if self.tram(w) {
            let dir = self.single_tram(w, self.old_tile);
            if dir != crate::direction_type::INVALID_DIAGDIR && dir != self.exitdir {
                self.err = EC_NO_WAY;
                return false;
            }
        }
        if self.mode == ROAD
            && w.depot(self.old_tile, self.mode)
            && w.road_depot_dir(self.old_tile) != self.exitdir
        {
            self.err = EC_NO_WAY;
            return false;
        }
        true
    }
    fn tile_exit(&mut self, w: &World<'_>) {
        self.is_station = false;
        self.is_bridge = false;
        self.is_tunnel = false;
        self.tiles_skipped = 0;
        if w.kind(self.old_tile) == crate::tile_type::MP_TUNNELBRIDGE {
            let dir = w.tunnel_bridge_dir(self.old_tile);
            if dir == self.exitdir {
                if w.tunnel(self.old_tile) {
                    self.is_tunnel = true;
                    self.new_tile = (w.shared.other_tunnel_end)(self.old_tile);
                } else {
                    self.is_bridge = true;
                    self.new_tile =
                        (crate::movement::table(w.shared).other_bridge_end)(self.old_tile);
                }
                self.tiles_skipped = crate::tunnelbridge::get_tunnel_bridge_length(
                    w.dimensions,
                    self.new_tile,
                    self.old_tile,
                ) as i32;
                return;
            }
            w.check((dir ^ 2) == self.exitdir);
        }
        self.new_tile = w.dimensions.add_diag(self.old_tile, self.exitdir);
        self.is_station = (self.mode == RAIL && w.has_station_rail(self.new_tile))
            || (self.mode == ROAD && w.station_road_stop(self.new_tile));
    }
    fn can_enter(&mut self, w: &World<'_>) -> bool {
        if self.mode == ROAD
            && w.bay_stop(self.new_tile)
            && (w.bay_dir(self.new_tile) ^ 2) != self.exitdir
        {
            self.err = EC_NO_WAY;
            return false;
        }
        if self.tram(w) {
            let dir = self.single_tram(w, self.new_tile);
            if dir != crate::direction_type::INVALID_DIAGDIR && dir != (self.exitdir ^ 2) {
                self.err = EC_NO_WAY;
                return false;
            }
        }
        if self.mode == ROAD && w.depot(self.new_tile, self.mode) {
            if (w.road_depot_dir(self.new_tile) ^ 2) != self.exitdir {
                self.err = EC_NO_WAY;
                return false;
            }
            if w.owner(self.new_tile) != self.veh_owner {
                self.err = EC_OWNER;
                return false;
            }
        }
        if self.mode == RAIL
            && w.depot(self.new_tile, self.mode)
            && (w.rail_depot_dir(self.new_tile) ^ 2) != self.exitdir
        {
            self.err = EC_NO_WAY;
            return false;
        }
        if self.mode == RAIL && w.owner(self.new_tile) != self.veh_owner {
            self.err = EC_NO_WAY;
            return false;
        }
        if self.mode == RAIL
            && self.railtypes != 0
            && self.railtypes & (1_u64 << w.tile_rail_type(self.new_tile)) == 0
        {
            self.err = EC_RAIL_ROAD_TYPE;
            return false;
        }
        if self.mode == ROAD {
            let rt = w.road_type(self.new_tile, self.tram(w));
            if (crate::movement::table(w.shared).road_compatible)(self.vehicle_ref())
                & (1_u64 << rt)
                == 0
            {
                self.err = EC_RAIL_ROAD_TYPE;
                return false;
            }
        }
        if w.kind(self.new_tile) == crate::tile_type::MP_TUNNELBRIDGE {
            if w.tunnel(self.new_tile) {
                if !self.is_tunnel && w.tunnel_bridge_dir(self.new_tile) != self.exitdir {
                    self.err = EC_NO_WAY;
                    return false;
                }
            } else if !self.is_bridge && w.tunnel_bridge_dir(self.new_tile) != self.exitdir {
                self.err = EC_NO_WAY;
                return false;
            }
        }
        if self.mode == RAIL && self.is_station {
            let length = platform_length(w, self.new_tile, trackdir_to_exitdir(self.old_td));
            self.tiles_skipped = length.wrapping_sub(1) as i32;
            self.new_tile = self.new_tile.wrapping_add(
                w.dimensions
                    .offs_by_diag_dir(self.exitdir)
                    .wrapping_mul(self.tiles_skipped as u32),
            );
        }
        true
    }
    pub(crate) fn follow(&mut self, s: &Services, tile: u32, td: u8) -> bool {
        let w = World::new(s);
        self.old_tile = tile;
        self.old_td = td;
        self.err = EC_NONE;
        if w.types.assertions() {
            w.check(
                (self.tram(&w)
                    && self.single_tram(&w, self.old_tile)
                        != crate::direction_type::INVALID_DIAGDIR)
                    || {
                        let sub =
                            u32::from(self.mode == ROAD && !self.veh.is_null() && self.tram(&w));
                        track_status_to_trackdir_bits(crate::movement::track_status(
                            s,
                            self.old_tile,
                            self.mode,
                            sub,
                            crate::direction_type::INVALID_DIAGDIR,
                        )) & trackdir_to_trackdir_bits(self.old_td)
                            != 0
                    },
            );
        }
        self.exitdir = trackdir_to_exitdir(self.old_td);
        if self.forced_reverse(&w) {
            return true;
        }
        if !self.can_exit(&w) {
            return false;
        }
        self.tile_exit(&w);
        if !self.query(&w) {
            return self.try_reverse(&w);
        }
        self.new_td_bits &= diagdir_reaches_trackdirs(self.exitdir);
        if self.new_td_bits == 0 || !self.can_enter(&w) {
            if self.mode == ROAD && !self.tram(&w) && self.try_reverse(&w) {
                return true;
            }
            if self.new_td_bits == 0 {
                self.err = EC_NO_WAY;
            }
            return false;
        }
        if (self.mode != RAIL && !self.allow90)
            || (self.mode == RAIL
                && (Pbs::acquire(s).0.rail90)(
                    w.tile_rail_type(self.old_tile),
                    w.tile_rail_type(self.new_tile),
                    !self.allow90,
                ))
        {
            self.new_td_bits &= !trackdir_crosses_trackdirs(self.old_td);
            if self.new_td_bits == 0 {
                self.err = EC_90DEG;
                return false;
            }
        }
        true
    }
    pub(crate) fn output(&self) -> Follow {
        Follow {
            old_tile: self.old_tile,
            new_tile: self.new_tile,
            skipped: self.tiles_skipped,
            dirs: self.new_td_bits,
            old_td: self.old_td,
            exitdir: self.exitdir,
            tunnel: u8::from(self.is_tunnel),
            bridge: u8::from(self.is_bridge),
            station: u8::from(self.is_station),
            error: self.err,
        }
    }
    pub(crate) fn restore(&mut self, f: Follow) {
        self.old_tile = f.old_tile;
        self.new_tile = f.new_tile;
        self.tiles_skipped = f.skipped;
        self.new_td_bits = f.dirs;
        self.old_td = f.old_td;
        self.exitdir = f.exitdir;
        self.is_tunnel = f.tunnel != 0;
        self.is_bridge = f.bridge != 0;
        self.is_station = f.station != 0;
        self.err = f.error;
    }
}
/// Virtual `BaseStation::GetPlatformLength(tile, dir)`: waypoints return one.
pub(crate) fn platform_length(w: &World<'_>, tile: u32, dir: u8) -> u32 {
    if w.station_kind(tile) == crate::station_type::RAIL_WAYPOINT {
        return 1;
    }
    platform_length_station(w, tile, dir)
}
pub(crate) fn platform_length_station(w: &World<'_>, mut tile: u32, dir: u8) -> u32 {
    let start = tile;
    let mut length: u32 = 0;
    w.check(w.rail_station(tile));
    w.check(dir < 4);
    loop {
        length = length.wrapping_add(1);
        tile = w.dimensions.add_diag(tile, dir);
        if !w.compatible_station(tile, start) {
            return length;
        }
    }
}
pub(crate) fn speed_limit(s: &Services, mode: u8, v: Option<VehicleRef>, tile: u32) -> i32 {
    let w = World::new(s);
    let m = crate::movement::table(s);
    let mut speed = i32::MAX;
    if mode != WATER && w.kind(tile) == crate::tile_type::MP_TUNNELBRIDGE && w.bridge(tile) {
        speed = i32::from((m.bridge_type_speed)((w.byte(w.types.m6(tile)) >> 2) & 15));
        if mode == ROAD {
            speed *= 2;
        }
    }
    let limit = if mode == RAIL {
        (m.train().physics.rail_speed)(crate::services::RailType(w.rail_type(tile)))
    } else if mode == ROAD {
        (m.road_type_speed)(crate::services::RoadType(
            w.road_type(tile, (m.road_kind)(v.unwrap()).0),
        ))
    } else {
        0
    };
    if limit > 0 {
        speed = speed.min(i32::from(limit));
    }
    speed
}

/// `MaskReservedTracks` for a rail follower with `DoTrackMasking()` enabled.
/// The caller invokes it only after native Follow succeeds. All accesses are
/// call-local scalars and typed services; no world borrow spans a service.
#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
pub(crate) fn mask_reserved_tracks(pbs: Pbs<'_>, ft: &mut Follow) -> bool {
    if ft.station != 0 {
        let diff = pbs.map().offs_by_diag_dir(ft.exitdir);
        let mut tile = ft
            .new_tile
            .wrapping_sub(diff.wrapping_mul(ft.skipped as u32));
        while tile != ft.new_tile {
            if World::new(pbs.shared()).station_reserved(tile) {
                ft.dirs = 0;
                ft.error = EC_RESERVED;
                return false;
            }
            tile = tile.wrapping_add(diff);
        }
    }
    let reserved = pbs.reserved(ft.new_tile);
    ft.dirs &= !track_bits_to_trackdir_bits(reserved);
    let mut tracks = trackdir_bits_to_track_bits(ft.dirs);
    while tracks != 0 {
        let track = tracks.trailing_zeros() as u8;
        tracks &= tracks.wrapping_sub(1);
        if tracks_overlap(
            reserved
                | track_to_track_bits(track, |ok| crate::movement::native_assert(pbs.shared(), ok)),
        ) {
            ft.dirs &= !track_to_trackdir_bits(track);
        }
    }
    if ft.dirs == 0 {
        ft.error = EC_RESERVED;
        return false;
    }
    true
}

/// # Safety
/// The output is writable; services and optional native vehicle remain live and serialized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_follower_init(
    out: *mut State,
    s: *const Services,
    mode: u8,
    v: *mut OpenTTDVehicle,
    owner: u8,
    types: u64,
    allow90: bool,
    masking: bool,
    from_vehicle: bool,
) {
    let s = unsafe { &*s };
    let v = std::ptr::NonNull::new(v).map(VehicleRef);
    let state = if from_vehicle {
        State::vehicle(s, mode, v, types, allow90, masking)
    } else {
        State::new(s, mode, v, owner, types, allow90, masking)
    };
    unsafe {
        out.write(state);
    }
}
/// # Safety
/// The stack follower and shared game world remain live and serialized throughout the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_follower_follow(
    s: *const Services,
    state: *mut State,
    tile: u32,
    td: u8,
) -> bool {
    unsafe { (&mut *state).follow(&*s, tile, td) }
}
/// # Safety
/// Same stack follower and serialized world contract as Follow.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_follower_mask(s: *const Services, state: *mut State) -> bool {
    let s = unsafe { &*s };
    let state = unsafe { &mut *state };
    if !state.masking {
        return true;
    }
    let mut out = state.output();
    let result = mask_reserved_tracks(Pbs::acquire(s), &mut out);
    state.restore(out);
    result
}
/// # Safety
/// Shared services and optional vehicle stay live under the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_follower_speed(
    s: *const Services,
    mode: u8,
    v: *mut OpenTTDVehicle,
    tile: u32,
) -> i32 {
    speed_limit(
        unsafe { &*s },
        mode,
        std::ptr::NonNull::new(v).map(VehicleRef),
        tile,
    )
}
/// # Safety
/// Station tile, services and direction satisfy the original method's preconditions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_platform_length(
    s: *const Services,
    station: u16,
    tile: u32,
    dir: u8,
    directional: bool,
) -> u32 {
    let w = World::new(unsafe { &*s });
    if directional {
        return platform_length_station(&w, tile, dir);
    }
    w.check(w.rail_station(tile) && w.word(w.types.m2(tile)) == station);
    let delta = w.dimensions.offs_by_axis(w.station_axis(tile));
    let mut t = tile;
    let mut len: u32 = 0;
    loop {
        t = t.wrapping_sub(delta);
        len = len.wrapping_add(1);
        if !w.compatible_station(t, tile) {
            break;
        }
    }
    t = tile;
    loop {
        t = t.wrapping_add(delta);
        len = len.wrapping_add(1);
        if !w.compatible_station(t, tile) {
            break;
        }
    }
    len.wrapping_sub(1)
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDFollowTrackState" ["probe"] => crate::follow_track::State { veh, railtypes, old_tile, new_tile, tiles_skipped, new_td_bits, veh_owner, old_td, exitdir, is_tunnel, is_bridge, is_station, err, mode, allow90, masking, };
}

pub(crate) fn rail(
    s: &Services,
    v: crate::train::Handle,
    owner: u8,
    types: u64,
    out: &mut Follow,
    tile: u32,
    td: u8,
) -> bool {
    let vehicle = std::ptr::NonNull::new(v.shell.cast()).map(VehicleRef);
    let mut state = if vehicle.is_some() {
        State::vehicle(s, RAIL, vehicle, types, true, false)
    } else {
        State::new(s, RAIL, None, owner, types, true, false)
    };
    state.restore(*out);
    let result = state.follow(s, tile, td);
    *out = state.output();
    result
}
pub(crate) fn water(s: &Services, tile: u32, td: u8) -> (State, bool) {
    let mut state = State::vehicle(s, WATER, None, u64::MAX, true, false);
    let ok = state.follow(s, tile, td);
    (state, ok)
}
