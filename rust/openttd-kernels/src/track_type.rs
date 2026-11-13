/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Native `track_type.h` values used by movement policies.
#![allow(missing_docs)]
/// Native `TRACK_BIT_NONE`.
pub const TRACK_BIT_NONE: u8 = TrackBits::TRACK_BIT_NONE.0;
/// Native `INVALID_TRACKDIR`.
pub const INVALID_TRACKDIR: u8 = Trackdir::INVALID_TRACKDIR.0;
/// Native `TRACK_BIT_X`.
pub const TRACK_BIT_X: u8 = TrackBits::TRACK_BIT_X.0;
/// Native `TRACK_BIT_Y`.
pub const TRACK_BIT_Y: u8 = TrackBits::TRACK_BIT_Y.0;
/// Native `TRACK_BIT_UPPER`.
pub const TRACK_BIT_UPPER: u8 = TrackBits::TRACK_BIT_UPPER.0;
/// Native `TRACK_BIT_LOWER`.
pub const TRACK_BIT_LOWER: u8 = TrackBits::TRACK_BIT_LOWER.0;
/// Native `TRACK_BIT_LEFT`.
pub const TRACK_BIT_LEFT: u8 = TrackBits::TRACK_BIT_LEFT.0;
/// Native `TRACK_BIT_RIGHT`.
pub const TRACK_BIT_RIGHT: u8 = TrackBits::TRACK_BIT_RIGHT.0;
/// Native `TRACK_BIT_ALL`.
pub const TRACK_BIT_ALL: u8 = TrackBits::TRACK_BIT_ALL.0;
/// Native `TRACK_BIT_WORMHOLE`.
pub const TRACK_BIT_WORMHOLE: u8 = TrackBits::TRACK_BIT_WORMHOLE.0;
/// Native `TRACK_BIT_DEPOT`.
pub const TRACK_BIT_DEPOT: u8 = TrackBits::TRACK_BIT_DEPOT.0;
/// Native `TRACK_UPPER`.
pub const TRACK_UPPER: u8 = Track::TRACK_UPPER.0;
/// Native `TRACK_LOWER`.
pub const TRACK_LOWER: u8 = Track::TRACK_LOWER.0;
/// Native `TRACKDIR_BIT_X_NE`.
pub const TRACKDIR_BIT_X_NE: u16 = TrackdirBits::TRACKDIR_BIT_X_NE.0;
/// Native `TRACKDIR_BIT_Y_SE`.
pub const TRACKDIR_BIT_Y_SE: u16 = TrackdirBits::TRACKDIR_BIT_Y_SE.0;
/// Native `TRACKDIR_BIT_UPPER_E`.
pub const TRACKDIR_BIT_UPPER_E: u16 = TrackdirBits::TRACKDIR_BIT_UPPER_E.0;
/// Native `TRACKDIR_BIT_LOWER_E`.
pub const TRACKDIR_BIT_LOWER_E: u16 = TrackdirBits::TRACKDIR_BIT_LOWER_E.0;
/// Native `TRACKDIR_BIT_LEFT_S`.
pub const TRACKDIR_BIT_LEFT_S: u16 = TrackdirBits::TRACKDIR_BIT_LEFT_S.0;
/// Native `TRACKDIR_BIT_RIGHT_S`.
pub const TRACKDIR_BIT_RIGHT_S: u16 = TrackdirBits::TRACKDIR_BIT_RIGHT_S.0;
/// Native `TRACKDIR_BIT_X_SW`.
pub const TRACKDIR_BIT_X_SW: u16 = TrackdirBits::TRACKDIR_BIT_X_SW.0;
/// Native `TRACKDIR_BIT_Y_NW`.
pub const TRACKDIR_BIT_Y_NW: u16 = TrackdirBits::TRACKDIR_BIT_Y_NW.0;
/// Native `TRACKDIR_BIT_UPPER_W`.
pub const TRACKDIR_BIT_UPPER_W: u16 = TrackdirBits::TRACKDIR_BIT_UPPER_W.0;
/// Native `TRACKDIR_BIT_LOWER_W`.
pub const TRACKDIR_BIT_LOWER_W: u16 = TrackdirBits::TRACKDIR_BIT_LOWER_W.0;
/// Native `TRACKDIR_BIT_LEFT_N`.
pub const TRACKDIR_BIT_LEFT_N: u16 = TrackdirBits::TRACKDIR_BIT_LEFT_N.0;
/// Native `TRACKDIR_BIT_RIGHT_N`.
pub const TRACKDIR_BIT_RIGHT_N: u16 = TrackdirBits::TRACKDIR_BIT_RIGHT_N.0;

/// `TRACK_BIT_HORZ`.
pub const TRACK_BIT_HORZ: u8 = TrackBits::TRACK_BIT_HORZ.0;
/// `TRACK_BIT_VERT`.
pub const TRACK_BIT_VERT: u8 = TrackBits::TRACK_BIT_VERT.0;
/// `TRACKDIR_X_NE`.
pub const TRACKDIR_X_NE: u8 = Trackdir::TRACKDIR_X_NE.0;
/// `TRACKDIR_Y_SE`.
pub const TRACKDIR_Y_SE: u8 = Trackdir::TRACKDIR_Y_SE.0;
/// `TRACKDIR_X_SW`.
pub const TRACKDIR_X_SW: u8 = Trackdir::TRACKDIR_X_SW.0;
/// `TRACKDIR_Y_NW`.
pub const TRACKDIR_Y_NW: u8 = Trackdir::TRACKDIR_Y_NW.0;

/// `TRACKDIR_BIT_MASK`.
pub const TRACKDIR_BIT_MASK: u32 = TrackdirBits::TRACKDIR_BIT_MASK.0 as u32;
/// `TRACK_BIT_MASK`.
pub const TRACK_BIT_MASK: u8 = TrackBits::TRACK_BIT_MASK.0;

/// `TRACK_END`.
pub const TRACK_END: u8 = Track::TRACK_END.0;

/// `TRACK_X`.
pub const TRACK_X: u8 = Track::TRACK_X.0;
/// `TRACK_Y`.
pub const TRACK_Y: u8 = Track::TRACK_Y.0;
/// `TRACK_LEFT`.
pub const TRACK_LEFT: u8 = Track::TRACK_LEFT.0;
/// `TRACK_RIGHT`.
pub const TRACK_RIGHT: u8 = Track::TRACK_RIGHT.0;
/// `INVALID_TRACK`.
pub const INVALID_TRACK: u8 = Track::INVALID_TRACK.0;
/// `TRACKDIR_UPPER_E`.
pub const TRACKDIR_UPPER_E: u8 = Trackdir::TRACKDIR_UPPER_E.0;
/// `TRACKDIR_LOWER_E`.
pub const TRACKDIR_LOWER_E: u8 = Trackdir::TRACKDIR_LOWER_E.0;
/// `TRACKDIR_LEFT_S`.
pub const TRACKDIR_LEFT_S: u8 = Trackdir::TRACKDIR_LEFT_S.0;
/// `TRACKDIR_RIGHT_S`.
pub const TRACKDIR_RIGHT_S: u8 = Trackdir::TRACKDIR_RIGHT_S.0;
/// `TRACKDIR_UPPER_W`.
pub const TRACKDIR_UPPER_W: u8 = Trackdir::TRACKDIR_UPPER_W.0;
/// `TRACKDIR_LOWER_W`.
pub const TRACKDIR_LOWER_W: u8 = Trackdir::TRACKDIR_LOWER_W.0;
/// `TRACKDIR_LEFT_N`.
pub const TRACKDIR_LEFT_N: u8 = Trackdir::TRACKDIR_LEFT_N.0;
/// `TRACKDIR_RIGHT_N`.
pub const TRACKDIR_RIGHT_N: u8 = Trackdir::TRACKDIR_RIGHT_N.0;

/// Native `Trackdir` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Trackdir(pub u8);

/// Native `Track` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Track(pub u8);

/// Native `TrackBits` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackBits(pub u8);

impl Track {
    /// Used for iterations
    pub const TRACK_BEGIN: Self = Self(0);
    /// Track along the x-axis (north-east to south-west)
    pub const TRACK_X: Self = Self(0);
    /// Track along the y-axis (north-west to south-east)
    pub const TRACK_Y: Self = Self(1);
    /// Track in the upper corner of the tile (north)
    pub const TRACK_UPPER: Self = Self(2);
    /// Track in the lower corner of the tile (south)
    pub const TRACK_LOWER: Self = Self(3);
    /// Track in the left corner of the tile (west)
    pub const TRACK_LEFT: Self = Self(4);
    /// Track in the right corner of the tile (east)
    pub const TRACK_RIGHT: Self = Self(5);
    /// Used for iterations
    pub const TRACK_END: Self = Self(6);
    /// Flag for an invalid track
    pub const INVALID_TRACK: Self = Self(255);
}

impl TrackBits {
    /// No track
    pub const TRACK_BIT_NONE: Self = Self(0);
    /// X-axis track
    pub const TRACK_BIT_X: Self = Self(1);
    /// Y-axis track
    pub const TRACK_BIT_Y: Self = Self(2);
    /// Upper track
    pub const TRACK_BIT_UPPER: Self = Self(4);
    /// Lower track
    pub const TRACK_BIT_LOWER: Self = Self(8);
    /// Left track
    pub const TRACK_BIT_LEFT: Self = Self(16);
    /// Right track
    pub const TRACK_BIT_RIGHT: Self = Self(32);
    /// X-Y-axis cross
    pub const TRACK_BIT_CROSS: Self = Self(3);
    /// Upper and lower track
    pub const TRACK_BIT_HORZ: Self = Self(12);
    /// Left and right track
    pub const TRACK_BIT_VERT: Self = Self(48);
    /// "Arrow" to the north-east
    pub const TRACK_BIT_3WAY_NE: Self = Self(37);
    /// "Arrow" to the south-east
    pub const TRACK_BIT_3WAY_SE: Self = Self(42);
    /// "Arrow" to the south-west
    pub const TRACK_BIT_3WAY_SW: Self = Self(25);
    /// "Arrow" to the north-west
    pub const TRACK_BIT_3WAY_NW: Self = Self(22);
    /// All possible tracks
    pub const TRACK_BIT_ALL: Self = Self(63);
    /// Bitmask for the first 6 bits
    pub const TRACK_BIT_MASK: Self = Self(63);
    /// Bitflag for a wormhole (used for tunnels)
    pub const TRACK_BIT_WORMHOLE: Self = Self(64);
    /// Bitflag for a depot
    pub const TRACK_BIT_DEPOT: Self = Self(128);
}

impl Trackdir {
    /// Used for iterations
    pub const TRACKDIR_BEGIN: Self = Self(0);
    /// X-axis and direction to north-east
    pub const TRACKDIR_X_NE: Self = Self(0);
    /// Y-axis and direction to south-east
    pub const TRACKDIR_Y_SE: Self = Self(1);
    /// Upper track and direction to east
    pub const TRACKDIR_UPPER_E: Self = Self(2);
    /// Lower track and direction to east
    pub const TRACKDIR_LOWER_E: Self = Self(3);
    /// Left track and direction to south
    pub const TRACKDIR_LEFT_S: Self = Self(4);
    /// Right track and direction to south
    pub const TRACKDIR_RIGHT_S: Self = Self(5);
    /// (Road vehicle) reverse direction north-east
    pub const TRACKDIR_RVREV_NE: Self = Self(6);
    /// (Road vehicle) reverse direction south-east
    pub const TRACKDIR_RVREV_SE: Self = Self(7);
    /// X-axis and direction to south-west
    pub const TRACKDIR_X_SW: Self = Self(8);
    /// Y-axis and direction to north-west
    pub const TRACKDIR_Y_NW: Self = Self(9);
    /// Upper track and direction to west
    pub const TRACKDIR_UPPER_W: Self = Self(10);
    /// Lower track and direction to west
    pub const TRACKDIR_LOWER_W: Self = Self(11);
    /// Left track and direction to north
    pub const TRACKDIR_LEFT_N: Self = Self(12);
    /// Right track and direction to north
    pub const TRACKDIR_RIGHT_N: Self = Self(13);
    /// (Road vehicle) reverse direction south-west
    pub const TRACKDIR_RVREV_SW: Self = Self(14);
    /// (Road vehicle) reverse direction north-west
    pub const TRACKDIR_RVREV_NW: Self = Self(15);
    /// Used for iterations
    pub const TRACKDIR_END: Self = Self(16);
    /// Flag for an invalid trackdir
    pub const INVALID_TRACKDIR: Self = Self(255);
}

/// Native `TrackdirBits`, retaining every underlying `u16` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackdirBits(pub u16);

impl TrackdirBits {
    /// No track build
    pub const TRACKDIR_BIT_NONE: Self = Self(0);
    /// Track x-axis, direction north-east
    pub const TRACKDIR_BIT_X_NE: Self = Self(1);
    /// Track y-axis, direction south-east
    pub const TRACKDIR_BIT_Y_SE: Self = Self(2);
    /// Track upper, direction east
    pub const TRACKDIR_BIT_UPPER_E: Self = Self(4);
    /// Track lower, direction east
    pub const TRACKDIR_BIT_LOWER_E: Self = Self(8);
    /// Track left, direction south
    pub const TRACKDIR_BIT_LEFT_S: Self = Self(16);
    /// Track right, direction south
    pub const TRACKDIR_BIT_RIGHT_S: Self = Self(32);
    /// Track x-axis, direction south-west
    pub const TRACKDIR_BIT_X_SW: Self = Self(256);
    /// Track y-axis, direction north-west
    pub const TRACKDIR_BIT_Y_NW: Self = Self(512);
    /// Track upper, direction west
    pub const TRACKDIR_BIT_UPPER_W: Self = Self(1024);
    /// Track lower, direction west
    pub const TRACKDIR_BIT_LOWER_W: Self = Self(2048);
    /// Track left, direction north
    pub const TRACKDIR_BIT_LEFT_N: Self = Self(4096);
    /// Track right, direction north
    pub const TRACKDIR_BIT_RIGHT_N: Self = Self(8192);
    /// Bitmask for bit-operations
    pub const TRACKDIR_BIT_MASK: Self = Self(16191);
    /// Flag for an invalid trackdirbit value
    pub const INVALID_TRACKDIR_BIT: Self = Self(65535);
}

/// `TrackStatus`: the status of a tile for a transport type, with the available
/// trackdirs in the low 16 bits and the red signals in the high 16 bits.
pub type TrackStatus = u32;

crate::base_bitset_type::impl_bitset_ops!(TrackBits);
crate::base_bitset_type::impl_bitset_ops!(TrackdirBits);
