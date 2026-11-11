/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Read helpers of `rail_map.h` for the audited map reader.
//! `None` routes to the original typed native accessor (including failed assertions).
use crate::bitmath_func::gb;
use crate::map_func::TileTypeAccess;
use crate::tile_type::TileIndex;
use crate::tile_type::TileType;

/// `GetRailGroundType` has no tile-type precondition in the original.
#[inline]
pub(crate) fn get_rail_ground_type(m: &TileTypeAccess, t: u32) -> Option<u8> {
    Some(m.m4(TileIndex(t))? & 15)
}

/// `RailTileType::Normal`, asserted by native compilation.
pub const RAIL_TILE_NORMAL: u8 = RailTileType::Normal.0;
/// `IsPbsSignal`, from `rail_map.h`.
#[inline]
pub const fn is_pbs_signal(kind: u8) -> bool {
    kind == crate::signal_type::SIGTYPE_PBS || kind == crate::signal_type::SIGTYPE_PBS_ONEWAY
}

/// `RailGroundType::HalfTileWater`.
pub const RAIL_GROUND_HALF_TILE_WATER: u8 = RailGroundType::HalfTileWater.0;
/// `RailTileType::Signals`.
pub const RAIL_TILE_SIGNALS: u8 = RailTileType::Signals.0;
/// `RailTileType::Depot`.
pub const RAIL_TILE_DEPOT: u8 = RailTileType::Depot.0;

#[allow(clippy::cast_possible_truncation)] // GB returns the original field width.
impl crate::map_func::World<'_> {
    /// `GetRailTileType`, retaining the native tile-type assertion.
    pub(crate) fn rail_tile_type(&self, tile: u32) -> u8 {
        self.asserted(|m| Some(m.require_type(TileIndex(tile), TileType::MP_RAILWAY)?.m5() >> 6))
    }

    #[inline]
    pub(crate) fn plain_rail(&self, t: u32) -> bool {
        self.kind(TileIndex(t)) == TileType::MP_RAILWAY
            && gb(u32::from(Self::byte(self.types.m5(TileIndex(t)))), 6, 2)
                <= u32::from(RAIL_TILE_SIGNALS)
    }
    #[inline]
    pub(crate) fn rail_depot(&self, t: u32) -> bool {
        self.kind(TileIndex(t)) == TileType::MP_RAILWAY
            && gb(u32::from(Self::byte(self.types.m5(TileIndex(t)))), 6, 2)
                == u32::from(RAIL_TILE_DEPOT)
    }
    #[inline]
    pub(crate) fn rail_type(&self, t: u32) -> u8 {
        gb(u32::from(Self::word(self.types.m8(TileIndex(t)))), 0, 6) as u8
    }
    #[inline]
    pub(crate) fn tracks(&self, t: u32) -> u8 {
        self.check(self.plain_rail(t));
        gb(u32::from(Self::byte(self.types.m5(TileIndex(t)))), 0, 6) as u8
    }
    /// `HasSignals`.
    pub(crate) fn has_signals(&self, t: u32) -> bool {
        self.rail_tile_type(t) == RAIL_TILE_SIGNALS
    }
    /// `IsRailDepot`, with `GetRailTileType`'s tile-type assertion.
    pub(crate) fn is_rail_depot(&self, t: u32) -> bool {
        self.rail_tile_type(t) == RAIL_TILE_DEPOT
    }
    /// `IsPlainRail`, with `GetRailTileType`'s tile-type assertion.
    pub(crate) fn is_plain_rail(&self, t: u32) -> bool {
        self.rail_tile_type(t) <= RAIL_TILE_SIGNALS
    }
    /// `GetRailGroundType`.
    pub(crate) fn rail_ground(&self, t: u32) -> u8 {
        self.asserted(|m| get_rail_ground_type(m, t))
    }
    /// `GetPresentSignals`.
    pub(crate) fn present_signals(&self, t: u32) -> u8 {
        (Self::byte(self.types.m3(TileIndex(t))) >> 4) & 0xF
    }
    /// `GetSignalStates`.
    pub(crate) fn signal_states(&self, t: u32) -> u8 {
        (Self::byte(self.types.m4(TileIndex(t))) >> 4) & 0xF
    }
    /// `GetSignalStateByTrackdir`: 1 is `SIGNAL_STATE_GREEN`.
    pub(crate) fn signal_state_by_trackdir(&self, t: u32, trackdir: u8) -> u8 {
        self.check(crate::track_func::is_valid_trackdir(trackdir));
        self.check(self.has_signal_on_track(t, crate::track_func::trackdir_to_track(trackdir)));
        u8::from(self.signal_states(t) & crate::signal_func::signal_along_trackdir(trackdir) != 0)
    }
    /// `IsPresignalEntry`.
    pub(crate) fn is_presignal_entry(&self, t: u32, track: u8) -> bool {
        self.signal_type(t, track) == crate::signal_type::SignalType::SIGTYPE_ENTRY.0
            || self.signal_type(t, track) == crate::signal_type::SignalType::SIGTYPE_COMBO.0
    }
    /// `IsPresignalExit`.
    pub(crate) fn is_presignal_exit(&self, t: u32, track: u8) -> bool {
        self.signal_type(t, track) == crate::signal_type::SignalType::SIGTYPE_EXIT.0
            || self.signal_type(t, track) == crate::signal_type::SignalType::SIGTYPE_COMBO.0
    }
    /// `IsOnewaySignal`.
    pub(crate) fn oneway_signal(&self, t: u32, track: u8) -> bool {
        self.signal_type(t, track) != crate::signal_type::SIGTYPE_PBS
    }
    /// `HasSignalOnTrack`.
    pub(crate) fn has_signal_on_track(&self, t: u32, track: u8) -> bool {
        self.check(crate::track_func::is_valid_track(track));
        self.has_signals(t)
            && self.present_signals(t) & crate::signal_func::signal_on_track(track) != 0
    }
    /// `HasSignalOnTrackdir`.
    pub(crate) fn has_signal_on_trackdir(&self, t: u32, trackdir: u8) -> bool {
        self.check(crate::track_func::is_valid_trackdir(trackdir));
        self.has_signals(t)
            && self.present_signals(t) & crate::signal_func::signal_along_trackdir(trackdir) != 0
    }
    /// `GetSignalType`.
    pub(crate) fn signal_type(&self, t: u32, track: u8) -> u8 {
        self.check(self.has_signals(t));
        let pos =
            if track == crate::track_type::TRACK_LOWER || track == crate::track_type::TRACK_RIGHT {
                4
            } else {
                0
            };
        gb(u32::from(Self::word(self.types.m2(TileIndex(t)))), pos, 3) as u8
    }
    #[inline]
    pub(crate) fn rail_depot_dir(&self, t: u32) -> u8 {
        gb(u32::from(Self::byte(self.types.m5(TileIndex(t)))), 0, 2) as u8
    }
}

/// Native `RailTileType` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RailTileType(pub u8);

/// Native `RailGroundType` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RailGroundType(pub u8);

#[allow(non_upper_case_globals)]
impl RailTileType {
    /// Normal rail tile without signals
    pub const Normal: Self = Self(0);
    /// Normal rail tile with signals
    pub const Signals: Self = Self(1);
    /// Depot (one entrance)
    pub const Depot: Self = Self(3);
}

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
