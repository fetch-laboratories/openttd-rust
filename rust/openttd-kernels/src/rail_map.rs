/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! The readers of `rail_map.h`, as `World` methods over the shared map descriptor.
//!
//! Each reads its field in place at the original's observation point and keeps the
//! original's assertions: one on a value is `World::check`, one whose condition reads the
//! map is `World::assert_with`, so that the read runs only where the native build asserts.
//! The `_raw` methods at the end keep integer signatures for files no pass has finished.
//! The setters, reservation and depot bits of the header stay typed map services.
use crate::bitmath_func::gb;
use crate::direction_type::DiagDirection;
use crate::map_func::{TileTypeAccess, World};
use crate::rail_type::RailType;
use crate::signal_func::{signal_along_trackdir, signal_on_track};
use crate::signal_type::{SignalState, SignalType};
use crate::tile_type::{TileIndex, TileType};
use crate::track_func::{
    diag_dir_to_diag_track, is_valid_track, is_valid_trackdir, trackdir_to_track,
};
use crate::track_type::{Track, TrackBits, Trackdir};

/// Native `RailTileType` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RailTileType(pub u8);

#[allow(non_upper_case_globals)]
impl RailTileType {
    /// Normal rail tile without signals
    pub const Normal: Self = Self(0);
    /// Normal rail tile with signals
    pub const Signals: Self = Self(1);
    /// Depot (one entrance)
    pub const Depot: Self = Self(3);
}

/// Native `RailGroundType` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RailGroundType(pub u8);

#[allow(non_upper_case_globals)]
impl RailGroundType {
    /// Nothing (dirt)
    pub const Barren: Self = Self(0);
    /// Grassy
    pub const Grass: Self = Self(1);
    /// Grass with a fence at the NW edge
    pub const FenceNW: Self = Self(2);
    /// Grass with a fence at the SE edge
    pub const FenceSE: Self = Self(3);
    /// Grass with a fence at the NW and SE edges
    pub const FenceSENW: Self = Self(4);
    /// Grass with a fence at the NE edge
    pub const FenceNE: Self = Self(5);
    /// Grass with a fence at the SW edge
    pub const FenceSW: Self = Self(6);
    /// Grass with a fence at the NE and SW edges
    pub const FenceNESW: Self = Self(7);
    /// Grass with a fence at the eastern side
    pub const FenceVert1: Self = Self(8);
    /// Grass with a fence at the western side
    pub const FenceVert2: Self = Self(9);
    /// Grass with a fence at the southern side
    pub const FenceHoriz1: Self = Self(10);
    /// Grass with a fence at the northern side
    pub const FenceHoriz2: Self = Self(11);
    /// Icy or sandy
    pub const SnowOrDesert: Self = Self(12);
    /// Grass with a fence and shore or water on the free halftile
    pub const HalfTileWater: Self = Self(13);
    /// Snow only on higher part of slope (steep or one corner raised)
    pub const HalfTileSnow: Self = Self(14);
}

/// `RailTileType::Normal`, for the files no pass has finished.
pub const RAIL_TILE_NORMAL: u8 = RailTileType::Normal.0;
/// `RailGroundType::HalfTileWater`, for the files no pass has finished.
pub const RAIL_GROUND_HALF_TILE_WATER: u8 = RailGroundType::HalfTileWater.0;

/// `GetRailGroundType`: get the ground type for rail tiles. It has no tile-type
/// precondition in the original; `None` routes to the native accessor.
#[inline]
#[allow(
    clippy::cast_possible_truncation,
    reason = "GetRailGroundType's cast of the four-bit field"
)]
pub(crate) fn get_rail_ground_type(m: &TileTypeAccess, t: TileIndex) -> Option<RailGroundType> {
    Some(RailGroundType(gb(u32::from(m.m4(t)?), 0, 4) as u8))
}

/// Raw adapter: `GetRailGroundType` over a tile index.
#[inline]
pub(crate) fn get_rail_ground_type_raw(m: &TileTypeAccess, t: u32) -> Option<u8> {
    Some(get_rail_ground_type(m, TileIndex(t))?.0)
}

/// `IsPbsSignal`: is the signal a path based signal?
#[inline]
pub const fn is_pbs_signal(s: SignalType) -> bool {
    s.0 == SignalType::SIGTYPE_PBS.0 || s.0 == SignalType::SIGTYPE_PBS_ONEWAY.0
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "each reader casts its GB field to the original's field type"
)]
impl World<'_> {
    /// `GetRailTileType`: returns the `RailTileType` (normal with or without signals,
    /// waypoint or depot).
    #[inline]
    pub(crate) fn get_rail_tile_type(&self, t: TileIndex) -> RailTileType {
        self.asserted(|m| {
            Some(RailTileType(gb(
                u32::from(m.require_type(t, TileType::MP_RAILWAY)?.m5()),
                6,
                2,
            ) as u8))
        })
    }

    /// `IsPlainRail`: returns whether this is plain rails, with or without signals.
    #[inline]
    pub(crate) fn is_plain_rail(&self, t: TileIndex) -> bool {
        let rtt = self.get_rail_tile_type(t);
        rtt == RailTileType::Normal || rtt == RailTileType::Signals
    }

    /// `IsPlainRailTile`: checks whether the tile is a rail tile or rail tile with signals.
    #[inline]
    pub(crate) fn is_plain_rail_tile(&self, t: TileIndex) -> bool {
        self.is_tile_type(t, TileType::MP_RAILWAY) && self.is_plain_rail(t)
    }

    /// `HasSignals`: checks if a rail tile has signals.
    #[inline]
    pub(crate) fn has_signals(&self, t: TileIndex) -> bool {
        self.get_rail_tile_type(t) == RailTileType::Signals
    }

    /// `IsRailDepot`: is this rail tile a rail depot?
    #[inline]
    pub(crate) fn is_rail_depot(&self, t: TileIndex) -> bool {
        self.get_rail_tile_type(t) == RailTileType::Depot
    }

    /// `IsRailDepotTile`: is this tile rail tile and a rail depot?
    #[inline]
    pub(crate) fn is_rail_depot_tile(&self, t: TileIndex) -> bool {
        self.is_tile_type(t, TileType::MP_RAILWAY) && self.is_rail_depot(t)
    }

    /// `GetRailType`: gets the rail type of the given tile.
    #[inline]
    pub(crate) fn get_rail_type(&self, t: TileIndex) -> RailType {
        RailType(gb(u32::from(Self::word(self.types.m8(t))), 0, 6) as u8)
    }

    /// `GetTrackBits`: gets the track bits of the given tile.
    #[inline]
    pub(crate) fn get_track_bits(&self, tile: TileIndex) -> TrackBits {
        self.assert_with(|| self.is_plain_rail_tile(tile));
        TrackBits(gb(u32::from(Self::byte(self.types.m5(tile))), 0, 6) as u8)
    }

    /// `HasTrack`: returns whether the given track is present on the given tile.
    #[inline]
    pub(crate) fn has_track(&self, tile: TileIndex, track: Track) -> bool {
        crate::bitmath_func::has_bit(self.get_track_bits(tile).0, track.0)
    }

    /// `GetRailDepotDirection`: returns the direction the depot is facing to.
    #[inline]
    pub(crate) fn get_rail_depot_direction(&self, t: TileIndex) -> DiagDirection {
        DiagDirection(gb(u32::from(Self::byte(self.types.m5(t))), 0, 2) as u8)
    }

    /// `GetRailDepotTrack`: returns the track of a depot, ignoring direction.
    #[inline]
    pub(crate) fn get_rail_depot_track(&self, t: TileIndex) -> Track {
        diag_dir_to_diag_track(self.get_rail_depot_direction(t), |ok| self.check(ok))
    }

    /// `GetSignalType`: get the signal type for a track on a tile.
    #[inline]
    pub(crate) fn get_signal_type(&self, t: TileIndex, track: Track) -> SignalType {
        self.assert_with(|| self.get_rail_tile_type(t) == RailTileType::Signals);
        let pos = if track == Track::TRACK_LOWER || track == Track::TRACK_RIGHT {
            4
        } else {
            0
        };
        SignalType(gb(u32::from(Self::word(self.types.m2(t))), pos, 3) as u8)
    }

    /// `IsPresignalEntry`: checks whether the given signals is a presignal entry (starts
    /// a presignal block).
    #[inline]
    pub(crate) fn is_presignal_entry(&self, t: TileIndex, track: Track) -> bool {
        self.get_signal_type(t, track) == SignalType::SIGTYPE_ENTRY
            || self.get_signal_type(t, track) == SignalType::SIGTYPE_COMBO
    }

    /// `IsPresignalExit`: checks whether the given signals is a presignal exit (ends a
    /// presignal block).
    #[inline]
    pub(crate) fn is_presignal_exit(&self, t: TileIndex, track: Track) -> bool {
        self.get_signal_type(t, track) == SignalType::SIGTYPE_EXIT
            || self.get_signal_type(t, track) == SignalType::SIGTYPE_COMBO
    }

    /// `IsOnewaySignal`: one-way signals can't be passed the 'wrong' way.
    #[inline]
    pub(crate) fn is_oneway_signal(&self, t: TileIndex, track: Track) -> bool {
        self.get_signal_type(t, track) != SignalType::SIGTYPE_PBS
    }

    /// `GetSignalStates`: get the states of the signals of a tile.
    #[inline]
    pub(crate) fn get_signal_states(&self, tile: TileIndex) -> u32 {
        gb(u32::from(Self::byte(self.types.m4(tile))), 4, 4)
    }

    /// `GetPresentSignals`: get whether the given signals are present (along/against the
    /// track).
    #[inline]
    pub(crate) fn get_present_signals(&self, tile: TileIndex) -> u32 {
        gb(u32::from(Self::byte(self.types.m3(tile))), 4, 4)
    }

    /// `HasSignalOnTrack`: checks for the presence of signals (either way) on the given
    /// track on the given rail tile.
    #[inline]
    pub(crate) fn has_signal_on_track(&self, tile: TileIndex, track: Track) -> bool {
        self.check(is_valid_track(track));
        self.get_rail_tile_type(tile) == RailTileType::Signals
            && (self.get_present_signals(tile) & u32::from(signal_on_track(track))) != 0
    }

    /// `HasSignalOnTrackdir`: checks for the presence of signals along the given trackdir
    /// on the given rail tile.
    ///
    /// Along meaning if you are currently driving on the given trackdir, this is the
    /// signal that is facing us (for which we stop when it's red).
    #[inline]
    pub(crate) fn has_signal_on_trackdir(&self, tile: TileIndex, trackdir: Trackdir) -> bool {
        self.check(is_valid_trackdir(trackdir));
        self.get_rail_tile_type(tile) == RailTileType::Signals
            && self.get_present_signals(tile) & u32::from(signal_along_trackdir(trackdir)) != 0
    }

    /// `GetSignalStateByTrackdir`: gets the state of the signal along the given trackdir.
    ///
    /// Along meaning if you are currently driving on the given trackdir, this is the
    /// signal that is facing us (for which we stop when it's red).
    #[inline]
    pub(crate) fn get_signal_state_by_trackdir(
        &self,
        tile: TileIndex,
        trackdir: Trackdir,
    ) -> SignalState {
        self.check(is_valid_trackdir(trackdir));
        self.assert_with(|| {
            self.has_signal_on_track(tile, trackdir_to_track(trackdir, |ok| self.check(ok)))
        });
        if self.get_signal_states(tile) & u32::from(signal_along_trackdir(trackdir)) != 0 {
            SignalState::SIGNAL_STATE_GREEN
        } else {
            SignalState::SIGNAL_STATE_RED
        }
    }

    /// `GetRailGroundType`: get the ground type for rail tiles.
    #[inline]
    pub(crate) fn get_rail_ground_type(&self, t: TileIndex) -> RailGroundType {
        self.asserted(|m| get_rail_ground_type(m, t))
    }
}

/// Raw adapters with integer signatures for callers in files no pass has finished. Each
/// wraps its arguments, calls the typed reader above and unwraps its result.
impl World<'_> {
    /// Raw adapter: `GetRailTileType`.
    pub(crate) fn get_rail_tile_type_raw(&self, t: u32) -> u8 {
        self.get_rail_tile_type(TileIndex(t)).0
    }
    /// Raw adapter: `IsPlainRailTile`.
    pub(crate) fn is_plain_rail_tile_raw(&self, t: u32) -> bool {
        self.is_plain_rail_tile(TileIndex(t))
    }
    /// Raw adapter: `HasSignals`.
    pub(crate) fn has_signals_raw(&self, t: u32) -> bool {
        self.has_signals(TileIndex(t))
    }
    /// Raw adapter: `IsRailDepot`.
    pub(crate) fn is_rail_depot_raw(&self, t: u32) -> bool {
        self.is_rail_depot(TileIndex(t))
    }
    /// Raw adapter: `IsRailDepotTile`.
    pub(crate) fn is_rail_depot_tile_raw(&self, t: u32) -> bool {
        self.is_rail_depot_tile(TileIndex(t))
    }
    /// Raw adapter: `GetRailType`.
    pub(crate) fn get_rail_type_raw(&self, t: u32) -> u8 {
        self.get_rail_type(TileIndex(t)).0
    }
    /// Raw adapter: `GetTrackBits`.
    pub(crate) fn get_track_bits_raw(&self, t: u32) -> u8 {
        self.get_track_bits(TileIndex(t)).0
    }
    /// Raw adapter: `GetRailDepotDirection`.
    pub(crate) fn get_rail_depot_direction_raw(&self, t: u32) -> u8 {
        self.get_rail_depot_direction(TileIndex(t)).0
    }
    /// Raw adapter: `GetSignalType`.
    pub(crate) fn get_signal_type_raw(&self, t: u32, track: u8) -> u8 {
        self.get_signal_type(TileIndex(t), Track(track)).0
    }
    /// Raw adapter: `GetSignalStates`.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "GetSignalStates' four-bit field"
    )]
    /// Raw adapter: `GetPresentSignals`.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "GetPresentSignals' four-bit field"
    )]
    /// Raw adapter: `HasSignalOnTrack`.
    pub(crate) fn has_signal_on_track_raw(&self, t: u32, track: u8) -> bool {
        self.has_signal_on_track(TileIndex(t), Track(track))
    }
    /// Raw adapter: `HasSignalOnTrackdir`.
    pub(crate) fn has_signal_on_trackdir_raw(&self, t: u32, trackdir: u8) -> bool {
        self.has_signal_on_trackdir(TileIndex(t), Trackdir(trackdir))
    }
    /// Raw adapter: `GetRailGroundType`.
    pub(crate) fn get_rail_ground_type_raw(&self, t: u32) -> u8 {
        self.get_rail_ground_type(TileIndex(t)).0
    }
}

#[cfg(test)]
mod tests {
    use crate::map_func::{BaseTileLayout, ExtendedTileLayout, TileTypeAllocation, World};
    use crate::signal_type::{SignalState, SignalType};
    use crate::tile_type::{TileIndex, TileType};
    use crate::track_type::{Track, TrackBits, Trackdir};
    use std::cell::RefCell;

    thread_local! {
        static CHECKS: RefCell<Vec<bool>> = const { RefCell::new(Vec::new()) };
    }
    extern "C" fn record_check(condition: bool) {
        CHECKS.with(|c| c.borrow_mut().push(condition));
    }
    /// The number of failed native assertions since the last call.
    fn failed_checks() -> usize {
        CHECKS.with(|c| {
            std::mem::take(&mut *c.borrow_mut())
                .iter()
                .filter(|ok| !**ok)
                .count()
        })
    }

    /// A four-tile rail map whose native assertion records: tile 0 a depot (`RailTileType::Depot`
    /// in m5 bits 6..7), tile 1 plain rail with `TRACK_BIT_X`, tiles 2 and 3 `TRACK_BIT_X` with
    /// signals, a PBS signal on `TRACK_X` (m2 bits 0..2) and the signal along `TRACKDIR_X_NE`
    /// present (m3 bit 7), green on tile 2 and red on tile 3 (m4 bit 7).
    fn map() -> World<'static> {
        let rail = TileType::MP_RAILWAY.0 << 4;
        let tiles = [
            (0xC0, 0, 0, 0),
            (0x01, 0, 0, 0),
            (0x41, 4, 0x80, 0x80),
            (0x41, 4, 0x80, 0),
        ];
        let base: Box<[BaseTileLayout]> = tiles
            .iter()
            .map(|&(m5, m2, m3, m4)| BaseTileLayout {
                kind: rail,
                height: 0,
                m2,
                m1: 0,
                m3,
                m4,
                m5,
            })
            .collect();
        let extended: Box<[ExtendedTileLayout]> = (0..4)
            .map(|_| ExtendedTileLayout {
                m6: 0,
                m7: 0,
                m8: 0,
            })
            .collect();
        crate::map_func::fixture::set(
            TileTypeAllocation {
                base: Box::into_raw(base).cast::<BaseTileLayout>(),
                extended: Box::into_raw(extended).cast::<ExtendedTileLayout>(),
                count: 4,
                assertions: true,
            },
            1,
        );
        World::new(Box::leak(Box::new(crate::services::Services {
            map: crate::map_func::fixture::statics(),
            movement: Box::leak(Box::new(crate::movement::MovementServices {
                native_assert: record_check,
                ..crate::test_tables::MOVEMENT
            })),
            ..crate::test_tables::SHARED
        })))
    }

    /// `GetTrackBits` asserts `IsPlainRailTile` and `GetSignalType` asserts a signal tile:
    /// each records one failed assertion on a tile that fails it, none on a valid tile, and
    /// reads the field in place either way.
    #[test]
    fn readers_assert_the_tile_they_read() {
        let w = map();
        failed_checks();
        assert_eq!(w.get_track_bits(TileIndex(1)), TrackBits::TRACK_BIT_X);
        assert_eq!(failed_checks(), 0);
        assert_eq!(w.get_track_bits(TileIndex(0)), TrackBits::TRACK_BIT_NONE);
        assert_eq!(failed_checks(), 1);
        assert_eq!(
            w.get_signal_type(TileIndex(2), Track::TRACK_X),
            SignalType::SIGTYPE_PBS
        );
        assert_eq!(failed_checks(), 0);
        assert_eq!(
            w.get_signal_type(TileIndex(1), Track::TRACK_X),
            SignalType::SIGTYPE_BLOCK
        );
        assert_eq!(failed_checks(), 1);
    }

    /// `GetSignalStateByTrackdir` is `SIGNAL_STATE_GREEN` for a set state bit along the
    /// trackdir and `SIGNAL_STATE_RED` for a clear one, after asserting a signal on its track.
    #[test]
    fn signal_state_by_trackdir_follows_the_state_bit() {
        let w = map();
        failed_checks();
        assert_eq!(
            w.get_signal_state_by_trackdir(TileIndex(2), Trackdir::TRACKDIR_X_NE),
            SignalState::SIGNAL_STATE_GREEN
        );
        assert_eq!(
            w.get_signal_state_by_trackdir(TileIndex(3), Trackdir::TRACKDIR_X_NE),
            SignalState::SIGNAL_STATE_RED
        );
        assert_eq!(failed_checks(), 0);
        // Tile 1 has no signal on `TRACK_X`.
        assert_eq!(
            w.get_signal_state_by_trackdir(TileIndex(1), Trackdir::TRACKDIR_X_NE),
            SignalState::SIGNAL_STATE_RED
        );
        assert_eq!(failed_checks(), 1);
    }
}
