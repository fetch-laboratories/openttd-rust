/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared town-rating constants from `town_type.h`.
/// Initial rating.
pub const RATING_INITIAL: i32 = 500;
/// Rating increase for improving a town bridge.
pub const RATING_TUNNEL_BRIDGE_UP_STEP: i32 = 50;
/// Rating decrease for removing a town tunnel or bridge.
pub const RATING_TUNNEL_BRIDGE_DOWN_STEP: i32 = -250;
/// Minimum rating after removing a town tunnel or bridge.
pub const RATING_TUNNEL_BRIDGE_MINIMUM: i32 = 0;
/// Maximum town rating.
pub const RATING_MAXIMUM: i32 = 1000;
/// Minimum town rating.
pub const RATING_MINIMUM: i32 = -1000;
/// Rating increase for a successful bribe.
pub const RATING_BRIBE_UP_STEP: i32 = 200;
/// Maximum rating a bribe can reach.
pub const RATING_BRIBE_MAXIMUM: i32 = 800;
/// Rating after a failed bribe.
pub const RATING_BRIBE_DOWN_TO: i32 = -50;

/// Native `TownID`; all uint16 values remain representable.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TownId(pub u16);

/// Opaque native `Town`; valid only while its native pool shell lives.
#[repr(C)]
pub struct TownHandle {
    _private: [u8; 0],
}

impl TownId {
    /// Native `TownID::Invalid()`.
    pub const INVALID: Self = Self(u16::MAX);
}

/// `TSZ_SMALL`.
pub const TSZ_SMALL: u8 = TownSize::TSZ_SMALL.0;
/// `TownExpandModes{TownExpandMode::Buildings}`.
pub const TOWN_EXPAND_BUILDINGS: u8 = 1 << 0;
/// `TownExpandModes{TownExpandMode::Roads}`.
pub const TOWN_EXPAND_ROADS: u8 = 1 << 1;

impl TownId {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(64000);
}

/// Original header spelling for the existing town identity.
pub use TownId as TownID;

/// The maximum length of a town name in characters including '\0'.
pub const MAX_LENGTH_TOWN_NAME_CHARS: u32 = 32;

/// Supported initial town sizes from `town_type.h`, including unnamed byte values.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TownSize(pub u8);
impl TownSize {
    /// Small town.
    pub const TSZ_SMALL: Self = Self(0);
    /// Medium town.
    pub const TSZ_MEDIUM: Self = Self(1);
    /// Large town.
    pub const TSZ_LARGE: Self = Self(2);
    /// Random size, bigger than small, smaller than large.
    pub const TSZ_RANDOM: Self = Self(3);
    /// Number of available town sizes.
    pub const TSZ_END: Self = Self(4);
}

/// Town layouts from `town_type.h`; saved and loaded as an eight-bit value.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TownLayout(pub u8);
impl TownLayout {
    /// First town layout.
    pub const TL_BEGIN: Self = Self(0);
    /// Original algorithm (minimum one tile between roads).
    pub const TL_ORIGINAL: Self = Self(0);
    /// Extended original algorithm (minimum two tiles between roads).
    pub const TL_BETTER_ROADS: Self = Self(1);
    /// Geometric 2x2 grid algorithm.
    pub const TL_2X2_GRID: Self = Self(2);
    /// Geometric 3x3 grid algorithm.
    pub const TL_3X3_GRID: Self = Self(3);
    /// Random town layout.
    pub const TL_RANDOM: Self = Self(4);
    /// Number of town layouts.
    pub const NUM_TLS: Self = Self(5);
}
