/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! PBS support routines (`pbs.cpp`, `pbs.h`). Map arrays and native shells stay
//! canonical; the native leaves in [`PbsServices`] reach the fields still stored
//! in C++. No world reference survives a call.
// The `switch` cases keep the original's `if` inside each `case`.
#![allow(unsafe_code, clippy::collapsible_match)]
use crate::bitmath_func::{has_bit, kill_first_bit_u32};
use crate::company_type::Owner;
use crate::direction_func::diag_dir_to_axis;
use crate::direction_type::DiagDirection;
use crate::follow_track::CFollowTrackT;
use crate::map_func::{World, native_check};
use crate::rail_type::RailTypes;
use crate::services::{RailMapServices, Services};
use crate::signal_type::SignalType;
use crate::tile_map::is_tile_type;
use crate::tile_type::{TileIndex, TileType};
use crate::track_func::{
    diag_dir_to_diag_trackdir, diagdir_reaches_trackdirs, find_first_trackdir, has_track,
    reverse_trackdir, track_bits_to_trackdir_bits, track_overlaps_tracks,
    track_status_to_track_bits, track_to_track_bits, track_to_trackdir,
    trackdir_bits_to_track_bits, trackdir_crosses_trackdirs, trackdir_to_exitdir,
    trackdir_to_track,
};
use crate::track_type::{Track, TrackBits, Trackdir, TrackdirBits};
use crate::train::Handle;
use crate::transport_type::TransportType;
use crate::vehicle_base::{VehState, VehStates};
use crate::vehicle_type::VEH_TRAIN;

#[repr(C)]
pub struct PbsServices {
    pub shared: *const Services,
    pub rail: *const RailMapServices,
    pub train: *const crate::train::Leaves,
    pub depot_bits: extern "C" fn(u32) -> TrackBits,
    pub crossing_bits: extern "C" fn(u32) -> TrackBits,
    pub station_bits: extern "C" fn(u32) -> TrackBits,
    pub wormhole_bits: extern "C" fn(u32) -> TrackBits,
    pub set_depot: extern "C" fn(u32, bool) -> (),
    pub set_station: extern "C" fn(u32, bool) -> (),
    pub set_wormhole: extern "C" fn(u32, bool) -> (),
    pub try_track: extern "C" fn(u32, u8) -> bool,
    pub unreserve_track: extern "C" fn(u32, u8) -> (),
    pub show_reservation: extern "C" fn() -> bool,
    pub station: extern "C" fn(u32) -> *mut crate::station::BaseStation,

    pub is_crossing: extern "C" fn(u32) -> bool,
    pub read_train_on_res: extern "C" fn(*const TrainOutput) -> Handle,
    pub write_train_on_res: extern "C" fn(*mut TrainOutput, Handle),
    pub assertions: bool,
    pub handle: extern "C" fn(*const crate::services::Train) -> crate::train::Handle,
    pub compatible: extern "C" fn(u8) -> u64,
    pub rail90: extern "C" fn(u8, u8, bool) -> bool,
    pub blocking: extern "C" fn(u32, u8) -> bool,
    pub has_pbs: extern "C" fn(u32, u8) -> bool,
    pub index: extern "C" fn(*const std::ffi::c_void) -> u32,
}

/// The end of a followed reservation (`pbs.h`).
#[derive(Clone, Copy)]
pub(crate) struct PBSTileInfo {
    /// Tile the path ends, `INVALID_TILE` if no valid path was found.
    pub tile: TileIndex,
    /// The reserved trackdir on the tile.
    pub trackdir: Trackdir,
    /// True if tile is a safe waiting position, false otherwise.
    pub okay: bool,
}

impl PBSTileInfo {
    /// The FFI record of this result, carrying `other` beside it.
    fn to_ffi(self, other: Handle) -> crate::train_reservation::Pbs {
        crate::train_reservation::Pbs {
            tile: self.tile.0,
            other,
            td: self.trackdir.0,
            okay: u8::from(self.okay),
        }
    }
}

/// Helper struct for finding the best matching vehicle on a specific track.
struct FindTrainOnTrackInfo {
    /// Information about the track.
    res: PBSTileInfo,
    /// The currently "best" vehicle we have found.
    best: Handle,
}

/// Context of the PBS routines: the service table of one synchronous native
/// entry. The tables it names have process lifetime and hold operations, no
/// world borrows.
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
    pub(crate) fn shared(self) -> &'a Services {
        // SAFETY: tables have process lifetime; they contain operations, no world borrows.
        unsafe { &*self.0.shared }
    }
    fn rail(self) -> &'a RailMapServices {
        // SAFETY: same table contract.
        unsafe { &*self.0.rail }
    }
    fn train(self) -> &'a crate::train::Leaves {
        // SAFETY: same table contract.
        unsafe { &*self.0.train }
    }
    pub(crate) fn map(self) -> crate::map_func::Map {
        self.shared().map()
    }
    /// The native assertion on a failed condition.
    fn check(self, condition: bool) {
        crate::movement::native_assert(self.shared(), condition);
    }
    /// `assert(condition)`: evaluates the condition only where the native build asserts.
    fn assert_with(self, condition: impl FnOnce() -> bool) {
        if self.0.assertions {
            self.check(condition());
        }
    }

    /// Get the reserved trackbits for any tile, regardless of type.
    /// Returns `TRACK_BIT_NONE` on nothing reserved or a tile without rail.
    pub(crate) fn get_reserved_trackbits(self, t: TileIndex) -> TrackBits {
        let w = World::new(self.shared());
        match w.kind(t) {
            TileType::MP_RAILWAY => {
                if w.is_rail_depot(t.0) {
                    return (self.0.depot_bits)(t.0);
                }
                if w.is_plain_rail(t.0) {
                    return (self.rail().rail_reservation)(t.0);
                }
            }
            TileType::MP_ROAD => {
                if (self.0.is_crossing)(t.0) {
                    return (self.0.crossing_bits)(t.0);
                }
            }
            TileType::MP_STATION => {
                if w.has_station_rail(t) {
                    return (self.0.station_bits)(t.0);
                }
            }
            TileType::MP_TUNNELBRIDGE => {
                if TransportType(w.tunnel_bridge_transport(t.0)) == TransportType::TRANSPORT_RAIL {
                    return (self.0.wormhole_bits)(t.0);
                }
            }
            _ => {}
        }
        TrackBits::TRACK_BIT_NONE
    }

    /// `HasReservedTracks`: check whether some of tracks is reserved on a tile.
    pub(crate) fn has_reserved_tracks(self, tile: TileIndex, tracks: TrackBits) -> bool {
        self.get_reserved_trackbits(tile) & tracks != TrackBits::TRACK_BIT_NONE
    }

    /// Set the reservation for a complete station platform, starting at `start`
    /// (a rail station tile) and following `dir`.
    pub(crate) fn set_rail_station_platform_reservation(
        self,
        start: TileIndex,
        dir: DiagDirection,
        b: bool,
    ) {
        let s = self.shared();
        let w = World::new(s);
        let mut tile = start;
        let diff = self.map().tile_offs_by_diag_dir(dir, native_check);

        self.assert_with(|| w.is_rail_station_tile(start));
        self.assert_with(|| {
            w.get_rail_station_axis(start) == diag_dir_to_axis(dir, |ok| self.check(ok))
        });

        loop {
            (self.0.set_station)(tile.0, b);
            (s.mark_dirty)(tile.0);
            tile = self.map().tile_add(tile, diff, native_check);
            if !w.is_compatible_train_station_tile(tile, start) {
                break;
            }
        }
    }

    /// `assert(HasTrack(TrackStatusToTrackBits(GetTileTrackStatus(tile, TRANSPORT_RAIL, 0)), t))`,
    /// which opens `TryReserveRailTrack` and `UnreserveRailTrack`.
    fn assert_has_track(self, tile: TileIndex, t: Track) {
        self.assert_with(|| {
            let bits = track_status_to_track_bits(crate::movement::get_tile_track_status(
                self.shared(),
                tile,
                TransportType::TRANSPORT_RAIL,
                0,
                DiagDirection::INVALID_DIAGDIR,
            ));
            has_track(bits, t.0, |ok| self.check(ok))
        });
    }

    /// `if (_settings_client.gui.show_track_reservation)`: the redraw that
    /// `TryReserveRailTrack` and `UnreserveRailTrack` make before changing a reservation.
    fn mark_reservation_dirty(self, tile: TileIndex) {
        if (self.0.show_reservation)() {
            if (self.shared().is_bridge)(tile.0) {
                (self.shared().mark_bridge_dirty)(tile.0);
            } else {
                (self.shared().mark_dirty)(tile.0);
            }
        }
    }

    /// Try to reserve a specific track on a tile; `trigger_stations` says whether
    /// to call the station randomisation trigger. Returns true if reservation was
    /// successful, i.e. the track was free and didn't cross any other reserved tracks.
    pub(crate) fn try_reserve_rail_track(
        self,
        tile: TileIndex,
        t: Track,
        trigger_stations: bool,
    ) -> bool {
        self.assert_has_track(tile, t);

        /* show the reserved rail if needed */
        self.mark_reservation_dirty(tile);

        let s = self.shared();
        let w = World::new(s);
        match w.kind(tile) {
            TileType::MP_RAILWAY => {
                if w.is_plain_rail(tile.0) {
                    return (self.0.try_track)(tile.0, t.0);
                }
                if w.is_rail_depot(tile.0) && !(self.rail().depot_reserved)(tile.0) {
                    (self.0.set_depot)(tile.0, true);
                    (s.mark_dirty)(tile.0); // some GRFs change their appearance when tile is reserved
                    return true;
                }
            }
            TileType::MP_ROAD => {
                if (self.0.is_crossing)(tile.0) && !(s.crossing_reserved)(tile.0) {
                    (s.set_crossing_reserved)(tile.0, true);
                    crate::train::update_level_crossing(self.train(), s, tile, false, false);
                    return true;
                }
            }
            TileType::MP_STATION => {
                if w.has_station_rail(tile) && !w.has_station_reservation(tile) {
                    (self.0.set_station)(tile.0, true);
                    if trigger_stations {
                        let st = (self.0.station)(tile.0);
                        crate::animated_tile::rail_event(
                            st,
                            tile.0,
                            crate::station_type::SAT_PATH_RESERVATION,
                            crate::station_type::SRT_PATH_RESERVATION,
                        );
                    }
                    (s.mark_dirty)(tile.0); // some GRFs need redraw after reserving track
                    return true;
                }
            }
            TileType::MP_TUNNELBRIDGE => {
                if TransportType(w.tunnel_bridge_transport(tile.0)) == TransportType::TRANSPORT_RAIL
                    && (self.0.wormhole_bits)(tile.0) == TrackBits::TRACK_BIT_NONE
                {
                    (self.0.set_wormhole)(tile.0, true);
                    return true;
                }
            }
            _ => {}
        }
        false
    }

    /// Lift the reservation of a specific track on a tile.
    pub(crate) fn unreserve_rail_track(self, tile: TileIndex, t: Track) {
        self.assert_has_track(tile, t);
        self.mark_reservation_dirty(tile);

        let s = self.shared();
        let w = World::new(s);
        match w.kind(tile) {
            TileType::MP_RAILWAY => {
                if w.is_rail_depot(tile.0) {
                    (self.0.set_depot)(tile.0, false);
                    (s.mark_dirty)(tile.0);
                } else if w.is_plain_rail(tile.0) {
                    (self.0.unreserve_track)(tile.0, t.0);
                }
            }
            TileType::MP_ROAD => {
                if (self.0.is_crossing)(tile.0) {
                    (s.set_crossing_reserved)(tile.0, false);
                    crate::train::update_level_crossing(self.train(), s, tile, true, false);
                }
            }
            TileType::MP_STATION => {
                if w.has_station_rail(tile) {
                    (self.0.set_station)(tile.0, false);
                    (s.mark_dirty)(tile.0);
                }
            }
            TileType::MP_TUNNELBRIDGE => {
                if TransportType(w.tunnel_bridge_transport(tile.0)) == TransportType::TRANSPORT_RAIL
                {
                    (self.0.set_wormhole)(tile.0, false);
                }
            }
            _ => {}
        }
    }

    /// `IsTileType(tile, MP_RAILWAY) && HasSignalOnTrackdir(tile, trackdir) &&
    /// !IsPbsSignal(GetSignalType(tile, TrackdirToTrack(trackdir)))`: a non-PBS
    /// signal on the trackdir.
    fn has_non_pbs_signal(self, tile: TileIndex, trackdir: Trackdir) -> bool {
        let w = World::new(self.shared());
        is_tile_type(self.shared(), tile, TileType::MP_RAILWAY)
            && w.has_signal_on_trackdir(tile.0, trackdir.0)
            && !crate::rail_map::is_pbs_signal(w.signal_type(tile.0, trackdir_to_track(trackdir.0)))
    }

    /// Follow a reservation starting from a specific tile to the end.
    fn follow_reservation(
        self,
        o: Owner,
        rts: RailTypes,
        mut tile: TileIndex,
        mut trackdir: Trackdir,
        ignore_oneway: bool,
    ) -> PBSTileInfo {
        let s = self.shared();
        let w = World::new(s);
        let mut start_tile = tile;
        let mut start_trackdir = trackdir;
        let mut first_loop = true;

        /* Start track not reserved? This can happen if two trains
         * are on the same tile. The reservation on the next tile
         * is not ours in this case, so exit. */
        if !self.has_reserved_tracks(
            tile,
            TrackBits(track_to_track_bits(trackdir_to_track(trackdir.0), |ok| {
                self.check(ok);
            })),
        ) {
            crate::witness::hit(crate::witness::PBS_START_UNRESERVED);
            return PBSTileInfo {
                tile,
                trackdir,
                okay: false,
            };
        }

        /* Do not disallow 90 deg turns as the setting might have changed between reserving and now. */
        let mut ft = CFollowTrackT::rail(s, Handle::NONE, o, rts);
        while ft.follow(s, tile, trackdir) {
            let reserved = ft.new_td_bits
                & TrackdirBits(track_bits_to_trackdir_bits(
                    self.get_reserved_trackbits(ft.new_tile).0,
                ));

            /* No reservation --> path end found */
            if reserved == TrackdirBits::TRACKDIR_BIT_NONE {
                if ft.is_station {
                    /* Check skipped station tiles as well, maybe our reservation ends inside the station. */
                    let diff = self.map().tile_offs_by_diag_dir(ft.exitdir, native_check);
                    while {
                        let skipped = ft.tiles_skipped;
                        ft.tiles_skipped = ft.tiles_skipped.wrapping_sub(1);
                        skipped > 0
                    } {
                        ft.new_tile = ft.new_tile - diff;
                        if w.has_station_reservation(ft.new_tile) {
                            tile = ft.new_tile;
                            trackdir = Trackdir(diag_dir_to_diag_trackdir(ft.exitdir.0, |ok| {
                                self.check(ok);
                            }));
                            crate::witness::hit(crate::witness::PBS_PLATFORM_INTERIOR);
                            break;
                        }
                    }
                }
                break;
            }

            /* Can't have more than one reserved trackdir */
            let new_trackdir = Trackdir(find_first_trackdir(reserved.0));

            /* One-way signal against us. The reservation can't be ours as it is not
             * a safe position from our direction and we can never pass the signal. */
            if !ignore_oneway && (self.0.blocking)(ft.new_tile.0, new_trackdir.0) {
                crate::witness::hit(crate::witness::PBS_ONEWAY_STOP);
                break;
            }

            tile = ft.new_tile;
            trackdir = new_trackdir;

            if first_loop {
                /* Update the start tile after we followed the track the first
                 * time. This is necessary because the track follower can skip
                 * tiles (in stations for example) which means that we might
                 * never visit our original starting tile again. */
                start_tile = tile;
                start_trackdir = trackdir;
                first_loop = false;
            } else if tile == start_tile && trackdir == start_trackdir {
                /* Loop encountered? */
                crate::witness::hit(crate::witness::PBS_LOOP);
                break;
            }
            /* Depot tile? Can't continue. */
            if w.rail_depot(tile.0) {
                break;
            }
            /* Non-pbs signal? Reservation can't continue. */
            if self.has_non_pbs_signal(tile, trackdir) {
                break;
            }
        }

        crate::witness::hit(crate::witness::PBS_FOLLOW);
        PBSTileInfo {
            tile,
            trackdir,
            okay: false,
        }
    }

    /// Find the best matching vehicle on a tile.
    fn check_trains_on_track(self, info: &mut FindTrainOnTrackInfo, tile: TileIndex) {
        // SAFETY: the serialized query keeps shells and service tables live.
        let vs = unsafe { &*self.train().vehicle };
        vs.any_on_tile(tile.0, |v| {
            if v.vehicle_type() != VEH_TRAIN || VehStates(v.status()).test(VehState::Crashed) {
                return false;
            }

            let t = (vs.train_handle)(v.0.as_ptr());
            if t.track() == TrackBits::TRACK_BIT_WORMHOLE.0
                || has_bit(t.track(), trackdir_to_track(info.res.trackdir.0))
            {
                let t = t.first();

                /* ALWAYS return the lowest ID (anti-desync!) */
                if info.best.record.is_null()
                    || (self.0.index)(t.vehicle().shell())
                        < (self.0.index)(info.best.vehicle().shell())
                {
                    if !info.best.record.is_null() {
                        crate::witness::hit(crate::witness::PBS_LOWER_ID);
                    }
                    info.best = t;
                }
            }
            false
        });
    }

    /// Follow a train reservation to the last tile. `train_on_res` is set to a
    /// train we might encounter. Returns the last tile of the reservation or the
    /// current train tile if no reservation present.
    pub(crate) fn follow_train_reservation(
        self,
        v: Handle,
        train_on_res: Option<&mut impl TrainOnRes>,
    ) -> PBSTileInfo {
        let s = self.shared();
        let w = World::new(s);
        self.assert_with(|| v.vehicle().vehicle_type() == VEH_TRAIN);

        let tile = TileIndex(v.vehicle().tile());
        let trackdir = crate::train::get_vehicle_trackdir(self.train(), s, v);

        if w.rail_depot(tile.0) && (self.0.depot_bits)(tile.0) == TrackBits::TRACK_BIT_NONE {
            return PBSTileInfo {
                tile,
                trackdir,
                okay: false,
            };
        }

        let mut ftoti = FindTrainOnTrackInfo {
            res: self.follow_reservation(
                crate::company_type::CompanyID(v.vehicle().owner()),
                RailTypes((s.all_compatible_railtypes)(v.get_railtypes())),
                tile,
                trackdir,
                false,
            ),
            best: Handle::NONE,
        };
        ftoti.res.okay = self.is_safe_waiting_position(
            v,
            ftoti.res.tile,
            ftoti.res.trackdir,
            true,
            (s.forbid_90_deg)(),
        );
        if let Some(train_on_res) = train_on_res {
            let res_tile = ftoti.res.tile;
            self.check_trains_on_track(&mut ftoti, res_tile);
            if !ftoti.best.record.is_null() {
                train_on_res.write(ftoti.best.first());
            }
            if train_on_res.read().record.is_null() && w.is_rail_station_tile(res_tile) {
                /* The target tile is a rail station. The track follower
                 * has stopped on the last platform tile where we haven't
                 * found a train. Also check all previous platform tiles
                 * for a possible train. */
                let diff = self.map().tile_offs_by_diag_dir(
                    DiagDirection(trackdir_to_exitdir(reverse_trackdir(ftoti.res.trackdir.0))),
                    native_check,
                );
                let mut st_tile = res_tile + diff;
                while train_on_res.read().record.is_null()
                    && w.is_compatible_train_station_tile(st_tile, res_tile)
                {
                    self.check_trains_on_track(&mut ftoti, st_tile);
                    if !ftoti.best.record.is_null() {
                        train_on_res.write(ftoti.best.first());
                    }
                    st_tile += diff;
                }
            }
            if train_on_res.read().record.is_null()
                && is_tile_type(s, res_tile, TileType::MP_TUNNELBRIDGE)
            {
                /* The target tile is a bridge/tunnel, also check the other end tile. */
                self.check_trains_on_track(
                    &mut ftoti,
                    TileIndex((self.rail().other_tunnel_bridge_end)(res_tile.0)),
                );
                if !ftoti.best.record.is_null() {
                    train_on_res.write(ftoti.best.first());
                    crate::witness::hit(crate::witness::PBS_OTHER_END_TRAIN);
                }
            }
        }
        ftoti.res
    }

    /// Find the train which has reserved a specific path: `tile` is a tile on the
    /// path and `track` a reserved track on it. Returns the vehicle holding the
    /// reservation or no vehicle if the path is stray.
    pub(crate) fn get_train_for_reservation(self, tile: TileIndex, track: Track) -> Handle {
        let s = self.shared();
        let w = World::new(s);
        self.assert_with(|| {
            self.has_reserved_tracks(
                tile,
                TrackBits(track_to_track_bits(track.0, |ok| self.check(ok))),
            )
        });
        let mut trackdir = Trackdir(track_to_trackdir(track.0));

        let rts = RailTypes((self.0.compatible)(w.tile_rail_type(tile.0)));

        /* Follow the path from tile to both ends, one of the end tiles should
         * have a train on it. We need FollowReservation to ignore one-way signals
         * here, as one of the two search directions will be the "wrong" way. */
        for i in 0..2 {
            // The original's loop increment, `trackdir = ReverseTrackdir(trackdir)`,
            // which also runs after the `continue` below.
            if i == 1 {
                trackdir = Trackdir(reverse_trackdir(trackdir.0));
                crate::witness::hit(crate::witness::PBS_SECOND_DIRECTION);
            }
            /* If the tile has a one-way block signal in the current trackdir, skip the
             * search in this direction as the reservation can't come from this side.*/
            if (self.0.blocking)(tile.0, reverse_trackdir(trackdir.0))
                && !(self.0.has_pbs)(tile.0, trackdir.0)
            {
                crate::witness::hit(crate::witness::PBS_SEARCH_ONEWAY_SKIP);
                continue;
            }

            let mut ftoti = FindTrainOnTrackInfo {
                res: self.follow_reservation(w.owner(tile), rts, tile, trackdir, true),
                best: Handle::NONE,
            };
            let res_tile = ftoti.res.tile;

            self.check_trains_on_track(&mut ftoti, res_tile);
            if !ftoti.best.record.is_null() {
                return ftoti.best;
            }

            /* Special case for stations: check the whole platform for a vehicle. */
            if w.is_rail_station_tile(res_tile) {
                let diff = self.map().tile_offs_by_diag_dir(
                    DiagDirection(trackdir_to_exitdir(reverse_trackdir(ftoti.res.trackdir.0))),
                    native_check,
                );
                let mut st_tile = res_tile + diff;
                while w.is_compatible_train_station_tile(st_tile, res_tile) {
                    self.check_trains_on_track(&mut ftoti, st_tile);
                    if !ftoti.best.record.is_null() {
                        crate::witness::hit(crate::witness::PBS_PLATFORM_TRAIN);
                        return ftoti.best;
                    }
                    st_tile += diff;
                }
            }

            /* Special case for bridges/tunnels: check the other end as well. */
            if is_tile_type(s, res_tile, TileType::MP_TUNNELBRIDGE) {
                self.check_trains_on_track(
                    &mut ftoti,
                    TileIndex((self.rail().other_tunnel_bridge_end)(res_tile.0)),
                );
                if !ftoti.best.record.is_null() {
                    crate::witness::hit(crate::witness::PBS_OTHER_END_TRAIN);
                    return ftoti.best;
                }
            }
        }

        crate::witness::hit(crate::witness::PBS_STRAY);
        Handle::NONE
    }

    /// `ft.new_td_bits &= DiagdirReachesTrackdirs(ft.exitdir)`, then
    /// `if (Rail90DegTurnDisallowed(..)) ft.new_td_bits &= ~TrackdirCrossesTrackdirs(trackdir)`:
    /// the reachable tracks of `IsSafeWaitingPosition` and `IsWaitingPositionFree`.
    fn mask_reachable_tracks(self, ft: &mut CFollowTrackT, trackdir: Trackdir, forbid_90deg: bool) {
        ft.new_td_bits &= TrackdirBits(diagdir_reaches_trackdirs(ft.exitdir.0));
        let w = World::new(self.shared());
        if (self.0.rail90)(
            w.tile_rail_type(ft.old_tile.0),
            w.tile_rail_type(ft.new_tile.0),
            forbid_90deg,
        ) {
            ft.new_td_bits &= !TrackdirBits(trackdir_crosses_trackdirs(trackdir.0));
        }
    }

    /// Determine whether a certain track on a tile is a safe position to end a path.
    /// `include_line_end` says whether end-of-line tiles are considered safe, and
    /// `forbid_90deg` forbids trains to make 90 degree turns.
    pub(crate) fn is_safe_waiting_position(
        self,
        v: Handle,
        tile: TileIndex,
        trackdir: Trackdir,
        include_line_end: bool,
        forbid_90deg: bool,
    ) -> bool {
        let s = self.shared();
        let w = World::new(s);
        if w.rail_depot(tile.0) {
            return true;
        }

        /* For non-pbs signals, stop on the signal tile. */
        if self.has_non_pbs_signal(tile, trackdir) {
            return true;
        }

        /* Check next tile. For performance reasons, we check for 90 degree turns ourself. */
        let mut ft = CFollowTrackT::vehicle(
            s,
            TransportType::TRANSPORT_RAIL,
            Some(v.vehicle()),
            RailTypes((s.all_compatible_railtypes)(v.get_railtypes())),
            true,
        );

        /* End of track? */
        if !ft.follow(s, tile, trackdir) {
            /* Last tile of a terminus station is a safe position. */
            if include_line_end {
                crate::witness::hit(crate::witness::PBS_SAFE_LINE_END);
                return true;
            }
        }

        /* Check for reachable tracks. */
        self.mask_reachable_tracks(&mut ft, trackdir, forbid_90deg);
        if ft.new_td_bits == TrackdirBits::TRACKDIR_BIT_NONE {
            return include_line_end;
        }

        if ft.new_td_bits != TrackdirBits::TRACKDIR_BIT_NONE
            && kill_first_bit_u32(u32::from(ft.new_td_bits.0)) == 0
        {
            let td = Trackdir(find_first_trackdir(ft.new_td_bits.0));
            /* PBS signal on next trackdir? Safe position. */
            if (self.0.has_pbs)(ft.new_tile.0, td.0) {
                crate::witness::hit(crate::witness::PBS_SAFE_PBS);
                return true;
            }
            /* One-way PBS signal against us? Safe if end-of-line is allowed. */
            if is_tile_type(s, ft.new_tile, TileType::MP_RAILWAY)
                && w.has_signal_on_trackdir(ft.new_tile.0, reverse_trackdir(td.0))
                && SignalType(w.signal_type(ft.new_tile.0, trackdir_to_track(td.0)))
                    == SignalType::SIGTYPE_PBS_ONEWAY
            {
                crate::witness::hit(crate::witness::PBS_SAFE_OPPOSING_PBS);
                return include_line_end;
            }
        }

        false
    }

    /// Check if a safe position is free; `forbid_90deg` forbids trains to make
    /// 90 degree turns.
    pub(crate) fn is_waiting_position_free(
        self,
        v: Handle,
        tile: TileIndex,
        trackdir: Trackdir,
        forbid_90deg: bool,
    ) -> bool {
        let s = self.shared();
        let track = Track(trackdir_to_track(trackdir.0));
        let reserved = self.get_reserved_trackbits(tile);

        /* Tile reserved? Can never be a free waiting position. */
        if track_overlaps_tracks(reserved.0, track.0, |ok| self.check(ok)) {
            crate::witness::hit(crate::witness::PBS_OVERLAP);
            return false;
        }

        /* Not reserved and depot or not a pbs signal -> free. */
        if World::new(s).rail_depot(tile.0) {
            return true;
        }
        if self.has_non_pbs_signal(tile, trackdir) {
            return true;
        }

        /* Check the next tile, it has to be free as well. Do not filter for compatible railtypes
         * to make sure we never accidentally join up reservations. */
        let mut ft = CFollowTrackT::vehicle(
            s,
            TransportType::TRANSPORT_RAIL,
            Some(v.vehicle()),
            RailTypes::EMPTY,
            true,
        );

        if !ft.follow(s, tile, trackdir) {
            return true;
        }

        /* Check for reachable tracks. */
        self.mask_reachable_tracks(&mut ft, trackdir, forbid_90deg);

        !self.has_reserved_tracks(
            ft.new_tile,
            TrackBits(trackdir_bits_to_track_bits(ft.new_td_bits.0)),
        )
    }
}

/// Temporary raw adapters for callers in files no pass has finished; each
/// delegates to the typed routine named after the original.
impl Pbs<'_> {
    /// [`Self::get_reserved_trackbits`] for `train_reservation.rs`, `rail_cmd.rs`,
    /// `rail_yapf.rs` and `road_cmd.rs`.
    pub(crate) fn get_reserved_trackbits_raw(self, tile: u32) -> u8 {
        self.get_reserved_trackbits(TileIndex(tile)).0
    }
    /// [`Self::has_reserved_tracks`] for `train_reservation.rs` and `train.rs`.
    pub(crate) fn has_reserved_tracks_raw(self, tile: u32, tracks: u8) -> bool {
        self.has_reserved_tracks(TileIndex(tile), TrackBits(tracks))
    }
    /// [`Self::set_rail_station_platform_reservation`] for `train_reservation.rs`
    /// and `train.rs`.
    pub(crate) fn set_rail_station_platform_reservation_raw(self, start: u32, dir: u8, b: bool) {
        self.set_rail_station_platform_reservation(TileIndex(start), DiagDirection(dir), b);
    }
    /// [`Self::try_reserve_rail_track`] for `train_reservation.rs`, `train.rs` and
    /// `rail_yapf.rs`.
    pub(crate) fn try_reserve_rail_track_raw(
        self,
        tile: u32,
        t: u8,
        trigger_stations: bool,
    ) -> bool {
        self.try_reserve_rail_track(TileIndex(tile), Track(t), trigger_stations)
    }
    /// [`Self::unreserve_rail_track`] for `train_reservation.rs`, `train.rs` and
    /// `rail_yapf.rs`.
    pub(crate) fn unreserve_rail_track_raw(self, tile: u32, t: u8) {
        self.unreserve_rail_track(TileIndex(tile), Track(t));
    }
    /// [`Self::follow_train_reservation`] for `train_reservation.rs` and
    /// `rail_yapf.rs`: with `check_other`, the `train_on_res` slot starts at
    /// `initial`; the record carries the slot's final value.
    pub(crate) fn follow_train_reservation_raw(
        self,
        v: Handle,
        check_other: bool,
        initial: Handle,
    ) -> crate::train_reservation::Pbs {
        let mut train_on_res = initial;
        let res = self.follow_train_reservation(v, check_other.then_some(&mut train_on_res));
        res.to_ffi(train_on_res)
    }
    /// [`Self::get_train_for_reservation`] for `rail_cmd.rs` and
    /// `tunnelbridge_cmd.rs`.
    pub(crate) fn get_train_for_reservation_raw(self, tile: u32, track: u8) -> Handle {
        self.get_train_for_reservation(TileIndex(tile), Track(track))
    }
    /// [`Self::is_safe_waiting_position`] for `train_reservation.rs`, `rail_cmd.rs`
    /// and `rail_yapf.rs`.
    pub(crate) fn is_safe_waiting_position_raw(
        self,
        v: Handle,
        tile: u32,
        trackdir: u8,
        include_line_end: bool,
        forbid_90deg: bool,
    ) -> bool {
        self.is_safe_waiting_position(
            v,
            TileIndex(tile),
            Trackdir(trackdir),
            include_line_end,
            forbid_90deg,
        )
    }
    /// [`Self::is_waiting_position_free`] for `train_reservation.rs` and
    /// `rail_yapf.rs`.
    pub(crate) fn is_waiting_position_free_raw(
        self,
        v: Handle,
        tile: u32,
        trackdir: u8,
        forbid_90deg: bool,
    ) -> bool {
        self.is_waiting_position_free(v, TileIndex(tile), Trackdir(trackdir), forbid_90deg)
    }
}

/// # Safety
/// The service tables and native world remain live for this synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pbs_reserved(s: *const PbsServices, tile: u32) -> u8 {
    Pbs(unsafe { &*s })
        .get_reserved_trackbits(TileIndex(tile))
        .0
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
    Pbs(unsafe { &*s }).set_rail_station_platform_reservation(
        TileIndex(tile),
        DiagDirection(dir),
        b,
    );
}
/// # Safety
/// Same synchronous service/world contract; track has the source preconditions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pbs_try_reserve_abi(
    s: *const PbsServices,
    tile: u32,
    track: u8,
    trigger: bool,
) -> u8 {
    u8::from(Pbs(unsafe { &*s }).try_reserve_rail_track(TileIndex(tile), Track(track), trigger))
}
/// # Safety
/// Same synchronous service/world contract; track has the source preconditions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pbs_unreserve(s: *const PbsServices, tile: u32, track: u8) {
    Pbs(unsafe { &*s }).unreserve_rail_track(TileIndex(tile), Track(track));
}
/// # Safety
/// Live serialized train and service tables outlive the synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pbs_follow_train(
    s: *const PbsServices,
    v: Handle,
    output: *mut TrainOutput,
) -> crate::train_reservation::Pbs {
    let pbs = Pbs(unsafe { &*s });
    let mut train_on_res = NativeTrainOnRes { pbs, output };
    let res = pbs.follow_train_reservation(v, (!output.is_null()).then_some(&mut train_on_res));
    res.to_ffi(Handle::NONE)
}
/// # Safety
/// Live world and tables; source requires a reserved track on this tile.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pbs_train_for(
    s: *const PbsServices,
    tile: u32,
    track: u8,
) -> Handle {
    Pbs(unsafe { &*s }).get_train_for_reservation(TileIndex(tile), Track(track))
}
/// # Safety
/// Live train and tables; original tile and trackdir preconditions apply.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pbs_safe_abi(
    s: *const PbsServices,
    v: Handle,
    tile: u32,
    td: u8,
    end: bool,
    no90: bool,
) -> u8 {
    u8::from(Pbs(unsafe { &*s }).is_safe_waiting_position(
        v,
        TileIndex(tile),
        Trackdir(td),
        end,
        no90,
    ))
}
/// # Safety
/// Live train and tables; original tile and trackdir preconditions apply.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pbs_free_abi(
    s: *const PbsServices,
    v: Handle,
    tile: u32,
    td: u8,
    no90: bool,
) -> u8 {
    u8::from(Pbs(unsafe { &*s }).is_waiting_position_free(v, TileIndex(tile), Trackdir(td), no90))
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
) -> crate::train_reservation::Pbs {
    Pbs(unsafe { &*s })
        .follow_reservation(
            crate::company_type::CompanyID(owner),
            RailTypes(types),
            TileIndex(tile),
            Trackdir(td),
            false,
        )
        .to_ffi(Handle::NONE)
}

/// Opaque native output slot, never dereferenced or borrowed by Rust.
#[repr(C)]
pub struct TrainOutput {
    _private: [u8; 0],
}
/// The `train_on_res` slot of `FollowTrainReservation`, read and written only
/// where the original reads and writes `*train_on_res`.
pub(crate) trait TrainOnRes {
    fn read(&self) -> Handle;
    fn write(&mut self, value: Handle);
}
impl TrainOnRes for Handle {
    fn read(&self) -> Handle {
        *self
    }
    fn write(&mut self, value: Handle) {
        *self = value;
    }
}
/// The native caller's `Vehicle **` slot, through the facade's two leaves.
struct NativeTrainOnRes<'a> {
    pbs: Pbs<'a>,
    output: *mut TrainOutput,
}
impl TrainOnRes for NativeTrainOnRes<'_> {
    fn read(&self) -> Handle {
        (self.pbs.0.read_train_on_res)(self.output)
    }
    fn write(&mut self, value: Handle) {
        (self.pbs.0.write_train_on_res)(self.output, value);
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDPbsServices" ["probe"] => PbsServices {
        shared,
        rail,
        train,
        depot_bits,
        crossing_bits,
        station_bits,
        wormhole_bits,
        set_depot,
        set_station,
        set_wormhole,
        try_track,
        unreserve_track,
        show_reservation,
        station,

        is_crossing,
        read_train_on_res,
        write_train_on_res,
        assertions,
        handle,
        compatible,
        rail90,
        blocking,
        has_pbs,
        index,
    };
}
