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

/// `GetRailGroundType` has no tile-type precondition in the original.
#[inline]
pub(crate) fn get_rail_ground_type(m: &TileTypeAccess, t: u32) -> Option<u8> {
    Some(m.m4(t)? & 15)
}

/// `RailTileType::Normal`, asserted by native compilation.
pub const RAIL_TILE_NORMAL: u8 = 0;
/// `IsPbsSignal`, from `rail_map.h`.
#[inline]
pub const fn is_pbs_signal(kind: u8) -> bool {
    kind == crate::signal_type::SIGTYPE_PBS || kind == crate::signal_type::SIGTYPE_PBS_ONEWAY
}

/// `RailGroundType::HalfTileWater`.
pub const RAIL_GROUND_HALF_TILE_WATER: u8 = 13;
/// `RailTileType::Signals`.
pub const RAIL_TILE_SIGNALS: u8 = 1;
/// `RailTileType::Depot`.
pub const RAIL_TILE_DEPOT: u8 = 3;

#[allow(clippy::cast_possible_truncation)] // GB returns the original field width.
impl crate::map_func::World<'_> {
    /// `GetRailTileType`, retaining the native tile-type assertion.
    pub(crate) fn rail_tile_type(&self, tile: u32) -> u8 {
        self.asserted(|m| Some(m.require_type(tile, crate::tile_type::MP_RAILWAY)?.m5() >> 6))
    }

    #[inline]
    pub(crate) fn plain_rail(&self, t: u32) -> bool {
        self.kind(t) == crate::tile_type::MP_RAILWAY
            && gb(u32::from(Self::byte(self.types.m5(t))), 6, 2) <= u32::from(RAIL_TILE_SIGNALS)
    }
    #[inline]
    pub(crate) fn rail_depot(&self, t: u32) -> bool {
        self.kind(t) == crate::tile_type::MP_RAILWAY
            && gb(u32::from(Self::byte(self.types.m5(t))), 6, 2) == u32::from(RAIL_TILE_DEPOT)
    }
    #[inline]
    pub(crate) fn rail_type(&self, t: u32) -> u8 {
        gb(u32::from(Self::word(self.types.m8(t))), 0, 6) as u8
    }
    #[inline]
    pub(crate) fn tracks(&self, t: u32) -> u8 {
        self.check(self.plain_rail(t));
        gb(u32::from(Self::byte(self.types.m5(t))), 0, 6) as u8
    }
    /// `HasSignals`.
    pub(crate) fn has_signals(&self, t: u32) -> bool {
        self.rail_tile_type(t) == RAIL_TILE_SIGNALS
    }
    /// `IsPlainRail`, with `GetRailTileType`'s tile-type assertion.
    pub(crate) fn is_plain_rail(&self, t: u32) -> bool {
        self.rail_tile_type(t) <= RAIL_TILE_SIGNALS
    }
    /// `GetRailGroundType`.
    pub(crate) fn rail_ground(&self, t: u32) -> u8 {
        Self::byte(self.types.m4(t)) & 15
    }
    /// `GetPresentSignals`.
    pub(crate) fn present_signals(&self, t: u32) -> u8 {
        (Self::byte(self.types.m3(t)) >> 4) & 0xF
    }
    /// `GetSignalStates`.
    pub(crate) fn signal_states(&self, t: u32) -> u8 {
        (Self::byte(self.types.m4(t)) >> 4) & 0xF
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
        gb(u32::from(Self::word(self.types.m2(t))), pos, 3) as u8
    }
    #[inline]
    pub(crate) fn rail_depot_dir(&self, t: u32) -> u8 {
        gb(u32::from(Self::byte(self.types.m5(t))), 0, 2) as u8
    }
}
