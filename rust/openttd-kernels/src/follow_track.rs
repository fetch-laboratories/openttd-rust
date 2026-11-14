/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Track follower helper `CFollowTrackT` (`pathfinder/follow_track.hpp`) for
//! rail, road and water, held on its caller's stack across follows.
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
use crate::company_type::Owner;
use crate::direction_func::reverse_diag_dir;
use crate::direction_type::DiagDirection;
use crate::map_func::World;
use crate::pbs::Pbs;
use crate::rail_type::RailTypes;
use crate::road_type::{RoadBits, RoadTramType, RoadType, RoadTypes};
use crate::services::Services;
use crate::tile_type::{TileIndex, TileType};
use crate::track_func::{
    diagdir_reaches_trackdirs, reverse_trackdir, set_track_bit_iterator,
    track_bits_to_trackdir_bits, track_status_to_trackdir_bits, track_to_track_bits,
    track_to_trackdir_bits, trackdir_bits_to_track_bits, trackdir_crosses_trackdirs,
    trackdir_to_exitdir, trackdir_to_trackdir_bits, tracks_overlap,
};
use crate::track_type::{TrackBits, Trackdir, TrackdirBits};
use crate::transport_type::TransportType;
use crate::vehicle::VehicleRef;

const RAIL: TransportType = TransportType::TRANSPORT_RAIL;
const ROAD: TransportType = TransportType::TRANSPORT_ROAD;
const WATER: TransportType = TransportType::TRANSPORT_WATER;

/// `CFollowTrackT::ErrorCode`, nested in the original's template.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ErrorCode(pub u8);

impl ErrorCode {
    /// No error.
    pub const EC_NONE: Self = Self(0);
    /// Another owner's depot or track.
    pub const EC_OWNER: Self = Self(1);
    /// Incompatible rail or road type.
    pub const EC_RAIL_ROAD_TYPE: Self = Self(2);
    /// Only 90-degree turns remain, and they are forbidden.
    pub const EC_90DEG: Self = Self(3);
    /// No way onward.
    pub const EC_NO_WAY: Self = Self(4);
    /// Every onward track is reserved.
    pub const EC_RESERVED: Self = Self(5);
}

/// Vehicle facts invariant during a synchronous road search. No search callback
/// changes road kind or compatible types, owner, display speed or order speed.
pub(crate) struct RoadContext {
    pub tram: bool,
    pub compatible: RoadTypes,
}

/// `RoadVehicle::From(veh)` or `Train::From(veh)` where the original dereferences
/// `veh` after `Init` asserted it non-null. A null `veh` there is outside the
/// original's defined behaviour and takes the native unreachable path.
fn deref_veh(veh: Option<VehicleRef>) -> VehicleRef {
    match veh {
        Some(v) => v,
        None => crate::services_assert::not_reached(),
    }
}

/// Track follower helper template class (can serve pathfinders and vehicle
/// controllers), for 3 different transport types w/ or w/o 90-deg turns allowed.
#[derive(Clone, Copy)]
#[allow(clippy::struct_excessive_bools)] // `CFollowTrackT`'s flags and template parameter.
pub struct CFollowTrackT {
    /// moving vehicle
    pub veh: Option<VehicleRef>,
    /// owner of the vehicle
    pub veh_owner: Owner,
    /// the origin (vehicle moved from) before move
    pub old_tile: TileIndex,
    /// the trackdir (the vehicle was on) before move
    pub old_td: Trackdir,
    /// the new tile (the vehicle has entered)
    pub new_tile: TileIndex,
    /// the new set of available trackdirs
    pub new_td_bits: TrackdirBits,
    /// exit direction (leaving the old tile)
    pub exitdir: DiagDirection,
    /// last turn passed tunnel
    pub is_tunnel: bool,
    /// last turn passed bridge ramp
    pub is_bridge: bool,
    /// last turn passed station
    pub is_station: bool,
    /// number of skipped tunnel or station tiles
    pub tiles_skipped: i32,
    pub err: ErrorCode,
    pub railtypes: RailTypes,
    /// The template parameters `Ttr_type_` and `T90deg_turns_allowed_`, which
    /// callers choose at run time; `Tmask_reserved_tracks` is the caller's
    /// call of [`Self::mask_reserved_tracks`].
    pub mode: TransportType,
    pub allow90: bool,
}

impl CFollowTrackT {
    /// `Init(Owner o, RailTypes railtype_override)`, with `veh` already set by the
    /// constructor or the vehicle `Init`.
    pub(crate) fn new(
        s: &Services,
        mode: TransportType,
        v: Option<VehicleRef>,
        o: Owner,
        railtype_override: RailTypes,
        allow90: bool,
    ) -> Self {
        let m = crate::movement::table(s);
        m.native_assert_call(mode != ROAD || v.is_some());
        m.native_assert_call(mode != RAIL || railtype_override != RailTypes::INVALID_RAILTYPES);
        Self {
            veh: v,
            veh_owner: o,
            /* don't worry, all is inlined so compiler should remove unnecessary initializations */
            old_tile: TileIndex::INVALID,
            old_td: Trackdir::INVALID_TRACKDIR,
            new_tile: TileIndex::INVALID,
            new_td_bits: TrackdirBits::TRACKDIR_BIT_NONE,
            exitdir: DiagDirection::INVALID_DIAGDIR,
            is_station: false,
            is_bridge: false,
            is_tunnel: false,
            tiles_skipped: 0,
            err: ErrorCode::EC_NONE,
            railtypes: railtype_override,
            mode,
            allow90,
        }
    }
    /// `Init(const VehicleType *v, RailTypes railtype_override)`.
    pub(crate) fn vehicle(
        s: &Services,
        mode: TransportType,
        v: Option<VehicleRef>,
        railtype_override: RailTypes,
        allow90: bool,
    ) -> Self {
        let m = crate::movement::table(s);
        m.native_assert_call(
            mode != RAIL || v.is_some_and(|v| v.vehicle_type() == crate::vehicle_type::VEH_TRAIN),
        );
        let o = v.map_or(Owner::INVALID, |v| {
            crate::company_type::CompanyID(v.owner())
        });
        let railtypes = if mode == RAIL && railtype_override == RailTypes::INVALID_RAILTYPES {
            RailTypes(
                (m.vehicle().train_handle)(deref_veh(v).0.as_ptr()).get_compatible_railtypes(),
            )
        } else {
            railtype_override
        };
        Self::new(s, mode, v, o, railtypes, allow90)
    }
    /// `CFollowTrackRail(v, types)` for a train, else `CFollowTrackRail(owner, types)`.
    pub(crate) fn rail(s: &Services, v: crate::train::Handle, o: Owner, types: RailTypes) -> Self {
        let vehicle = std::ptr::NonNull::new(v.record).map(VehicleRef);
        if vehicle.is_some() {
            Self::vehicle(s, RAIL, vehicle, types, true)
        } else {
            Self::new(s, RAIL, None, o, types, true)
        }
    }
    /// Temporary raw adapter of [`Self::new`] for `native_test_support.rs` until
    /// its pass.
    #[cfg(feature = "native-test-support")]
    pub(crate) fn new_raw(
        s: &Services,
        mode: u8,
        v: Option<VehicleRef>,
        owner: u8,
        types: u64,
        allow90: bool,
    ) -> Self {
        Self::new(
            s,
            TransportType(mode),
            v,
            crate::company_type::CompanyID(owner),
            RailTypes(types),
            allow90,
        )
    }
    /// Temporary raw adapter of [`Self::vehicle`] for
    /// `native_test_support.rs` until its pass.
    #[cfg(feature = "native-test-support")]
    pub(crate) fn vehicle_raw(
        s: &Services,
        mode: u8,
        v: Option<VehicleRef>,
        types: u64,
        allow90: bool,
    ) -> Self {
        Self::vehicle(s, TransportType(mode), v, RailTypes(types), allow90)
    }
    /// Temporary raw adapter of [`Self::rail`] for `train_reservation.rs` until
    /// its pass.
    pub(crate) fn rail_raw(s: &Services, v: crate::train::Handle, owner: u8, types: u64) -> Self {
        Self::rail(
            s,
            v,
            crate::company_type::CompanyID(owner),
            RailTypes(types),
        )
    }

    /// `IsTram()`: the road context's road kind where the caller supplies one.
    fn is_tram(&self, w: &World<'_>, road: Option<&RoadContext>) -> bool {
        self.mode == ROAD
            && road.map_or_else(
                || (crate::movement::table(w.shared).road_kind)(deref_veh(self.veh)).0,
                |r| r.tram,
            )
    }

    /// Tests if a tile is a road tile with a single tramtrack (tram can reverse)
    fn get_single_tram_bit(
        &self,
        w: &World<'_>,
        tile: TileIndex,
        road: Option<&RoadContext>,
    ) -> DiagDirection {
        w.check(self.is_tram(w, road)); // this function shouldn't be called in other cases

        if w.is_normal_road_tile(tile.0) {
            let rb = RoadBits(w.road_bits(tile.0, RoadTramType::RTT_TRAM.0));
            match rb {
                RoadBits::ROAD_NW => return DiagDirection::DIAGDIR_NW,
                RoadBits::ROAD_SW => return DiagDirection::DIAGDIR_SW,
                RoadBits::ROAD_SE => return DiagDirection::DIAGDIR_SE,
                RoadBits::ROAD_NE => return DiagDirection::DIAGDIR_NE,
                _ => {}
            }
        }
        DiagDirection::INVALID_DIAGDIR
    }

    /// Main follower routine. Fills all members and returns true on success.
    /// Otherwise returns false if track can't be followed.
    pub(crate) fn follow(&mut self, s: &Services, old_tile: TileIndex, old_td: Trackdir) -> bool {
        self.follow_with_road(s, old_tile, old_td, None)
    }
    /// Temporary raw adapter of [`Self::follow`] for `train_reservation.rs` and
    /// `native_test_support.rs` until their passes.
    pub(crate) fn follow_raw(&mut self, s: &Services, tile: u32, td: u8) -> bool {
        self.follow(s, TileIndex(tile), Trackdir(td))
    }
    /// `Follow`, with the road search's invariant vehicle facts where the caller
    /// supplies them.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn follow_with_road(
        &mut self,
        s: &Services,
        old_tile: TileIndex,
        old_td: Trackdir,
        road: Option<&RoadContext>,
    ) -> bool {
        let w = World::new(s);
        self.old_tile = old_tile;
        self.old_td = old_td;
        self.err = ErrorCode::EC_NONE;

        // The original's assertion lambda runs only in assertion builds.
        if w.types.assertions() {
            w.check(
                // Skip the check for single tram bits
                (self.is_tram(&w, road)
                    && self.get_single_tram_bit(&w, self.old_tile, road)
                        != DiagDirection::INVALID_DIAGDIR)
                    || {
                        let sub_mode = if self.mode == ROAD && self.veh.is_some() {
                            u32::from(self.is_tram(&w, road))
                        } else {
                            0
                        };
                        let old_tile_valid_dirs = TrackdirBits(track_status_to_trackdir_bits(
                            crate::movement::table(s).get_tile_track_status_in_world(
                                &w,
                                self.old_tile,
                                self.mode,
                                sub_mode,
                                DiagDirection::INVALID_DIAGDIR,
                            ),
                        ));
                        (old_tile_valid_dirs
                            & TrackdirBits(trackdir_to_trackdir_bits(self.old_td.0)))
                            != TrackdirBits::TRACKDIR_BIT_NONE
                    },
            );
        }

        self.exitdir = DiagDirection(trackdir_to_exitdir(self.old_td.0));
        if self.forced_reverse(&w, road) {
            return true;
        }
        if !self.can_exit_old_tile(&w, road) {
            return false;
        }
        self.follow_tile_exit(&w);
        if !self.query_new_tile_track_status(&w, road) {
            return self.try_reverse(&w, road);
        }
        self.new_td_bits &= TrackdirBits(diagdir_reaches_trackdirs(self.exitdir.0));
        if self.new_td_bits == TrackdirBits::TRACKDIR_BIT_NONE || !self.can_enter_new_tile(&w, road)
        {
            /* In case we can't enter the next tile, but are
             * a normal road vehicle, then we can actually
             * try to reverse as this is the end of the road.
             * Trams can only turn on the appropriate bits in
             * which case reaching this would mean a dead end
             * near a building and in that case there would
             * a "false" QueryNewTileTrackStatus result and
             * as such reversing is already tried. The fact
             * that function failed can have to do with a
             * missing road bit, or inability to connect the
             * different bits due to slopes. */
            if self.mode == ROAD && !self.is_tram(&w, road) && self.try_reverse(&w, road) {
                return true;
            }

            /* CanEnterNewTile already set a reason.
             * Do NOT overwrite it (important for example for EC_RAIL_ROAD_TYPE).
             * Only set a reason if CanEnterNewTile was not called */
            if self.new_td_bits == TrackdirBits::TRACKDIR_BIT_NONE {
                self.err = ErrorCode::EC_NO_WAY;
            }

            return false;
        }
        if (self.mode != RAIL && !self.allow90)
            || (self.mode == RAIL
                && (Pbs::acquire(s).0.rail90)(
                    w.tile_rail_type(self.old_tile.0),
                    w.tile_rail_type(self.new_tile.0),
                    !self.allow90,
                ))
        {
            self.new_td_bits &= !TrackdirBits(trackdir_crosses_trackdirs(self.old_td.0));
            if self.new_td_bits == TrackdirBits::TRACKDIR_BIT_NONE {
                self.err = ErrorCode::EC_90DEG;
                return false;
            }
        }
        true
    }

    /// `MaskReservedTracks` for a follower whose `DoTrackMasking()` is true; the
    /// caller calls it only for such a follower, after `Follow` succeeds. All
    /// accesses are call-local scalars and typed services; no world borrow spans
    /// a service.
    pub(crate) fn mask_reserved_tracks(&mut self, pbs: Pbs<'_>) -> bool {
        if self.is_station {
            /* Check skipped station tiles as well. */
            let diff = pbs
                .map()
                .tile_offs_by_diag_dir(self.exitdir, crate::map_func::native_check);
            let mut tile = self.new_tile - diff.wrapping_mul(self.tiles_skipped);
            while tile != self.new_tile {
                if World::new(pbs.shared()).has_station_reservation(tile) {
                    self.new_td_bits = TrackdirBits::TRACKDIR_BIT_NONE;
                    self.err = ErrorCode::EC_RESERVED;
                    return false;
                }
                tile += diff;
            }
        }

        let reserved = pbs.get_reserved_trackbits(self.new_tile);
        /* Mask already reserved trackdirs. */
        self.new_td_bits &= !TrackdirBits(track_bits_to_trackdir_bits(reserved.0));
        /* Mask out all trackdirs that conflict with the reservation. */
        for t in set_track_bit_iterator(TrackBits(trackdir_bits_to_track_bits(self.new_td_bits.0)))
        {
            if tracks_overlap(
                reserved.0
                    | track_to_track_bits(t.0, |ok| {
                        crate::movement::native_assert(pbs.shared(), ok);
                    }),
            ) {
                self.new_td_bits &= !TrackdirBits(track_to_trackdir_bits(t.0));
            }
        }
        if self.new_td_bits == TrackdirBits::TRACKDIR_BIT_NONE {
            self.err = ErrorCode::EC_RESERVED;
            return false;
        }
        true
    }

    /// Follow the exitdir from `old_tile` and fill `new_tile` and `tiles_skipped`
    fn follow_tile_exit(&mut self, w: &World<'_>) {
        self.is_station = false;
        self.is_bridge = false;
        self.is_tunnel = false;
        self.tiles_skipped = 0;

        /* extra handling for tunnels and bridges in our direction */
        if w.kind(self.old_tile) == TileType::MP_TUNNELBRIDGE {
            let enterdir = DiagDirection(w.tunnel_bridge_dir(self.old_tile.0));
            if enterdir == self.exitdir {
                /* we are entering the tunnel / bridge */
                if w.tunnel(self.old_tile.0) {
                    self.is_tunnel = true;
                    self.new_tile = TileIndex((w.shared.other_tunnel_end)(self.old_tile.0));
                } else {
                    // IsBridge(old_tile)
                    self.is_bridge = true;
                    self.new_tile = TileIndex((crate::movement::table(w.shared)
                        .tunnel_bridge()
                        .other_bridge_end)(
                        self.old_tile.0
                    ));
                }
                self.tiles_skipped = crate::tunnelbridge::get_tunnel_bridge_length(
                    w.dimensions,
                    self.new_tile.0,
                    self.old_tile.0,
                ) as i32;
                return;
            }
            w.check(reverse_diag_dir(enterdir, |ok| w.check(ok)) == self.exitdir);
        }

        /* normal or station tile, do one step */
        self.new_tile = w.dimensions.tile_add_by_diag_dir(
            self.old_tile,
            self.exitdir,
            crate::map_func::native_check,
        );

        /* special handling for stations */
        if (self.mode == RAIL && w.has_station_tile_rail(self.new_tile))
            || (self.mode == ROAD && w.is_station_road_stop_tile(self.new_tile))
        {
            self.is_station = true;
        }
    }

    /// stores track status (available trackdirs) for the new tile into `new_td_bits`
    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn query_new_tile_track_status(&mut self, w: &World<'_>, road: Option<&RoadContext>) -> bool {
        if self.mode == RAIL && w.plain_rail(self.new_tile.0) {
            self.new_td_bits = TrackdirBits(u16::from(w.tracks(self.new_tile.0)) * 0x101);
        } else if self.mode == ROAD {
            self.new_td_bits = crate::pathfinder_func::get_trackdir_bits_for_road_in_world(
                crate::movement::table(w.shared),
                w,
                self.new_tile,
                if self.is_tram(w, road) {
                    RoadTramType::RTT_TRAM
                } else {
                    RoadTramType::RTT_ROAD
                },
            );
        } else {
            self.new_td_bits = TrackdirBits(track_status_to_trackdir_bits(
                crate::movement::table(w.shared).get_tile_track_status_in_world(
                    w,
                    self.new_tile,
                    self.mode,
                    0,
                    DiagDirection::INVALID_DIAGDIR,
                ),
            ));
        }
        self.new_td_bits != TrackdirBits::TRACKDIR_BIT_NONE
    }

    /// return true if we can leave `old_tile` in exitdir
    fn can_exit_old_tile(&mut self, w: &World<'_>, road: Option<&RoadContext>) -> bool {
        /* road stop can be left at one direction only unless it's a drive-through stop */
        if self.mode == ROAD && w.is_bay_road_stop_tile(self.old_tile) {
            let exitdir = w.get_bay_road_stop_dir(self.old_tile);
            if exitdir != self.exitdir {
                self.err = ErrorCode::EC_NO_WAY;
                return false;
            }
        }

        /* single tram bits can only be left in one direction */
        if self.is_tram(w, road) {
            let single_tram = self.get_single_tram_bit(w, self.old_tile, road);
            if single_tram != DiagDirection::INVALID_DIAGDIR && single_tram != self.exitdir {
                self.err = ErrorCode::EC_NO_WAY;
                return false;
            }
        }

        /* road depots can be also left in one direction only */
        if self.mode == ROAD && w.depot(self.old_tile.0, self.mode.0) {
            let exitdir = DiagDirection(w.road_depot_dir(self.old_tile.0));
            if exitdir != self.exitdir {
                self.err = ErrorCode::EC_NO_WAY;
                return false;
            }
        }
        true
    }

    /// return true if we can enter `new_tile` from exitdir
    fn can_enter_new_tile(&mut self, w: &World<'_>, road: Option<&RoadContext>) -> bool {
        if self.mode == ROAD && w.is_bay_road_stop_tile(self.new_tile) {
            /* road stop can be entered from one direction only unless it's a drive-through stop */
            let exitdir = w.get_bay_road_stop_dir(self.new_tile);
            if reverse_diag_dir(exitdir, |ok| w.check(ok)) != self.exitdir {
                self.err = ErrorCode::EC_NO_WAY;
                return false;
            }
        }

        /* single tram bits can only be entered from one direction */
        if self.is_tram(w, road) {
            let single_tram = self.get_single_tram_bit(w, self.new_tile, road);
            if single_tram != DiagDirection::INVALID_DIAGDIR
                && single_tram != reverse_diag_dir(self.exitdir, |ok| w.check(ok))
            {
                self.err = ErrorCode::EC_NO_WAY;
                return false;
            }
        }

        /* road and rail depots can also be entered from one direction only */
        if self.mode == ROAD && w.depot(self.new_tile.0, self.mode.0) {
            let exitdir = DiagDirection(w.road_depot_dir(self.new_tile.0));
            if reverse_diag_dir(exitdir, |ok| w.check(ok)) != self.exitdir {
                self.err = ErrorCode::EC_NO_WAY;
                return false;
            }
            /* don't try to enter other company's depots */
            if w.owner(self.new_tile) != self.veh_owner {
                self.err = ErrorCode::EC_OWNER;
                return false;
            }
        }
        if self.mode == RAIL && w.depot(self.new_tile.0, self.mode.0) {
            let exitdir = DiagDirection(w.rail_depot_dir(self.new_tile.0));
            if reverse_diag_dir(exitdir, |ok| w.check(ok)) != self.exitdir {
                self.err = ErrorCode::EC_NO_WAY;
                return false;
            }
        }

        /* rail transport is possible only on tiles with the same owner as vehicle */
        if self.mode == RAIL && w.owner(self.new_tile) != self.veh_owner {
            /* different owner */
            self.err = ErrorCode::EC_NO_WAY;
            return false;
        }

        /* rail transport is possible only on compatible rail types */
        if self.mode == RAIL && self.railtypes.any() {
            let rail_type = crate::rail_type::RailType(w.tile_rail_type(self.new_tile.0));
            if !self.railtypes.test(rail_type) {
                /* incompatible rail type */
                self.err = ErrorCode::EC_RAIL_ROAD_TYPE;
                return false;
            }
        }

        /* road transport is possible only on compatible road types */
        if self.mode == ROAD {
            let roadtype = RoadType(w.road_type(self.new_tile.0, self.is_tram(w, road)));
            let compatible_roadtypes = road.map_or_else(
                || {
                    RoadTypes((crate::movement::table(w.shared).road_compatible)(
                        deref_veh(self.veh),
                    ))
                },
                |r| r.compatible,
            );
            if !compatible_roadtypes.test(roadtype) {
                /* incompatible road type */
                self.err = ErrorCode::EC_RAIL_ROAD_TYPE;
                return false;
            }
        }

        /* tunnel holes and bridge ramps can be entered only from proper direction */
        if w.kind(self.new_tile) == TileType::MP_TUNNELBRIDGE {
            if w.tunnel(self.new_tile.0) {
                if !self.is_tunnel {
                    let tunnel_enterdir = DiagDirection(w.tunnel_bridge_dir(self.new_tile.0));
                    if tunnel_enterdir != self.exitdir {
                        self.err = ErrorCode::EC_NO_WAY;
                        return false;
                    }
                }
            } else {
                // IsBridge(new_tile)
                if !self.is_bridge {
                    let ramp_enderdir = DiagDirection(w.tunnel_bridge_dir(self.new_tile.0));
                    if ramp_enderdir != self.exitdir {
                        self.err = ErrorCode::EC_NO_WAY;
                        return false;
                    }
                }
            }
        }

        /* special handling for rail stations - get to the end of platform */
        if self.mode == RAIL && self.is_station {
            /* entered railway station
             * get platform length */
            let length = crate::station::get_platform_length_by_tile(
                w,
                self.new_tile,
                DiagDirection(trackdir_to_exitdir(self.old_td.0)),
            );
            /* how big step we must do to get to the last platform tile? */
            self.tiles_skipped = length.wrapping_sub(1) as i32;
            /* move to the platform end */
            let mut diff = w
                .dimensions
                .tile_offs_by_diag_dir(self.exitdir, crate::map_func::native_check);
            diff = diff.wrapping_mul(self.tiles_skipped);
            self.new_tile =
                w.dimensions
                    .tile_add(self.new_tile, diff, crate::map_func::native_check);
            return true;
        }

        true
    }

    /// `ForcedReverse` and `TryReverse`'s shared stores for a reversal on `old_tile`,
    /// in the original's order.
    fn reverse_state(&mut self, exitdir: DiagDirection) {
        self.new_tile = self.old_tile;
        self.new_td_bits = TrackdirBits(trackdir_to_trackdir_bits(reverse_trackdir(self.old_td.0)));
        self.exitdir = exitdir;
        self.tiles_skipped = 0;
        self.is_tunnel = false;
        self.is_bridge = false;
        self.is_station = false;
    }

    /// return true if we must reverse (in depots and single tram bits)
    fn forced_reverse(&mut self, w: &World<'_>, road: Option<&RoadContext>) -> bool {
        /* rail and road depots cause reversing */
        if self.mode != WATER && w.depot(self.old_tile.0, self.mode.0) {
            let exitdir = DiagDirection(if self.mode == RAIL {
                w.rail_depot_dir(self.old_tile.0)
            } else {
                w.road_depot_dir(self.old_tile.0)
            });
            if exitdir != self.exitdir {
                /* reverse */
                self.reverse_state(exitdir);
                return true;
            }
        }

        /* Single tram bits and standard road stops cause reversing. */
        if self.mode == ROAD
            && ((self.is_tram(w, road)
                && self.get_single_tram_bit(w, self.old_tile, road)
                    == reverse_diag_dir(self.exitdir, |ok| w.check(ok)))
                || (w.is_bay_road_stop_tile(self.old_tile)
                    && w.get_bay_road_stop_dir(self.old_tile)
                        == reverse_diag_dir(self.exitdir, |ok| w.check(ok))))
        {
            /* reverse */
            self.reverse_state(reverse_diag_dir(self.exitdir, |ok| w.check(ok)));
            return true;
        }

        false
    }

    /// return true if we successfully reversed at end of road/track
    fn try_reverse(&mut self, w: &World<'_>, road: Option<&RoadContext>) -> bool {
        if self.mode == ROAD && !self.is_tram(w, road) {
            /* if we reached the end of road, we can reverse the RV and continue moving */
            self.exitdir = reverse_diag_dir(self.exitdir, |ok| w.check(ok));
            /* new tile will be the same as old one */
            self.new_tile = self.old_tile;
            /* set new trackdir bits to all reachable trackdirs */
            self.query_new_tile_track_status(w, road);
            self.new_td_bits &= TrackdirBits(diagdir_reaches_trackdirs(self.exitdir.0));
            if self.new_td_bits != TrackdirBits::TRACKDIR_BIT_NONE {
                /* we have some trackdirs reachable after reversal */
                return true;
            }
        }
        self.err = ErrorCode::EC_NO_WAY;
        false
    }
}

/// Helper for pathfinders - get max speed on `old_tile`, and its minimum
/// speed, always 0, into `pmin_speed` where the caller requests it.
pub(crate) fn get_speed_limit(
    s: &Services,
    mode: TransportType,
    veh: Option<VehicleRef>,
    old_tile: TileIndex,
    pmin_speed: Option<&mut i32>,
) -> i32 {
    let min_speed = 0;
    let max_speed = get_speed_limit_with_road(s, mode, veh, old_tile, None);

    /* if min speed was requested, return it */
    if let Some(pmin_speed) = pmin_speed {
        *pmin_speed = min_speed;
    }
    max_speed
}
/// `GetSpeedLimit`, with the road search's invariant road kind where the caller
/// supplies it.
pub(crate) fn get_speed_limit_with_road(
    s: &Services,
    mode: TransportType,
    veh: Option<VehicleRef>,
    old_tile: TileIndex,
    rtt: Option<RoadTramType>,
) -> i32 {
    let w = World::new(s);
    let m = crate::movement::table(s);
    let mut max_speed = i32::MAX; // no limit

    /* Check for on-bridge speed limit */
    if mode != WATER && w.kind(old_tile) == TileType::MP_TUNNELBRIDGE && w.bridge(old_tile.0) {
        let mut spd = i32::from((m.tunnel_bridge().speed)(w.bridge_type(old_tile.0)));
        if mode == ROAD {
            spd *= 2;
        }
        max_speed = max_speed.min(spd);
    }
    /* Check for speed limit imposed by railtype */
    if mode == RAIL {
        let rail_speed =
            (m.train().physics.rail_speed)(crate::services::RailType(w.rail_type(old_tile.0)));
        if rail_speed > 0 {
            max_speed = max_speed.min(i32::from(rail_speed));
        }
    }
    if mode == ROAD {
        /* max_speed is already in roadvehicle units, no need to further modify (divide by 2) */
        let rtt = rtt.unwrap_or_else(|| (m.road_kind)(deref_veh(veh)));
        let road_speed = (m.road_type_speed)(RoadType(w.road_type(old_tile.0, rtt.0)));
        if road_speed > 0 {
            max_speed = max_speed.min(i32::from(road_speed));
        }
    }

    /* if min speed was requested, return it: no caller requests it */
    max_speed
}
/// Temporary raw adapter of [`get_speed_limit`] for `native_test_support.rs`
/// until its pass.
#[cfg(feature = "native-test-support")]
pub(crate) fn get_speed_limit_raw(
    s: &Services,
    mode: u8,
    veh: Option<VehicleRef>,
    tile: u32,
) -> i32 {
    get_speed_limit(s, TransportType(mode), veh, TileIndex(tile), None)
}

/// `CFollowTrackWater(v)` with a null vehicle, followed once from `tile`: the
/// raw adapter for `ship_yapf.rs` until its pass.
pub(crate) fn water(s: &Services, tile: u32, td: u8) -> (CFollowTrackT, bool) {
    let mut ft = CFollowTrackT::vehicle(s, WATER, None, RailTypes::INVALID_RAILTYPES, true);
    let ok = ft.follow(s, TileIndex(tile), Trackdir(td));
    (ft, ok)
}
