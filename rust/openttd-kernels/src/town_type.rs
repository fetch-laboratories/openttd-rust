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
/// Minimum rating after removing a town house.
pub const RATING_HOUSE_MINIMUM: i32 = RATING_MINIMUM;
/// Rating increase for a successful bribe.
pub const RATING_BRIBE_UP_STEP: i32 = 200;
/// Maximum rating a bribe can reach.
pub const RATING_BRIBE_MAXIMUM: i32 = 800;
/// Rating after a failed bribe.
pub const RATING_BRIBE_DOWN_TO: i32 = -50;

/// Rating `VERYPOOR`.
pub const RATING_VERYPOOR: i32 = -200;
/// Rating `MEDIOCRE`.
pub const RATING_MEDIOCRE: i32 = 200;
/// When a town grows, all companies have rating increased a bit ...
pub const RATING_GROWTH_UP_STEP: i32 = 5;
/// ... up to `RATING_MEDIOCRE`.
pub const RATING_GROWTH_MAXIMUM: i32 = RATING_MEDIOCRE;
/// When a town grows, company gains reputation for all well serviced stations ...
pub const RATING_STATION_UP_STEP: i32 = 12;
/// ... but loses for badly serviced stations.
pub const RATING_STATION_DOWN_STEP: i32 = -15;
/// Rating needed to remove a town tunnel or bridge, "Lenient" difficulty settings.
pub const RATING_TUNNEL_BRIDGE_NEEDED_LENIENT: i32 = 144;
/// "Neutral".
pub const RATING_TUNNEL_BRIDGE_NEEDED_NEUTRAL: i32 = 208;
/// "Hostile".
pub const RATING_TUNNEL_BRIDGE_NEEDED_HOSTILE: i32 = 400;
/// "Permissive" (local authority disabled).
pub const RATING_TUNNEL_BRIDGE_NEEDED_PERMISSIVE: i32 = RATING_MINIMUM;
/// Rating needed to remove town road, "Lenient" difficulty settings.
pub const RATING_ROAD_NEEDED_LENIENT: i32 = 16;
/// "Neutral".
pub const RATING_ROAD_NEEDED_NEUTRAL: i32 = 64;
/// "Hostile".
pub const RATING_ROAD_NEEDED_HOSTILE: i32 = 112;
/// "Permissive" (local authority disabled).
pub const RATING_ROAD_NEEDED_PERMISSIVE: i32 = RATING_MINIMUM;

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

/// Native `TownExpandMode` bit positions, retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownExpandMode(pub u8);
#[allow(
    non_upper_case_globals,
    reason = "original town_type.h TownExpandMode enumerators"
)]
impl TownExpandMode {
    /// Allow town to place buildings.
    pub const Buildings: Self = Self(0);
    /// Allow town to place roads.
    pub const Roads: Self = Self(1);
}

/// Native `TownExpandModes`, retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownExpandModes(pub u8);
impl TownExpandModes {
    /// Native element-to-set conversion.
    #[must_use]
    pub const fn from_element(element: TownExpandMode) -> Self {
        Self(1 << element.0)
    }
    /// Native set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    /// Native `Test` of one element.
    #[must_use]
    pub const fn test(self, element: TownExpandMode) -> bool {
        self.0 & Self::from_element(element).0 != 0
    }
}
crate::base_bitset_type::impl_bitset_ops!(TownExpandModes);
/// Native `TownExpandModes{a, b}` from two elements.
impl std::ops::BitOr for TownExpandMode {
    type Output = TownExpandModes;
    fn bitor(self, other: Self) -> TownExpandModes {
        TownExpandModes::from_element(self).with(TownExpandModes::from_element(other))
    }
}

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

/// Town founding setting values; saved and loaded as an eight-bit value.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TownFounding(pub u8);
impl TownFounding {
    /// Used for iterations and limit testing.
    pub const TF_BEGIN: Self = Self(0);
    /// Forbidden.
    pub const TF_FORBIDDEN: Self = Self(0);
    /// Allowed.
    pub const TF_ALLOWED: Self = Self(1);
    /// Allowed, with custom town layout.
    pub const TF_CUSTOM_LAYOUT: Self = Self(2);
    /// Used for iterations and limit testing.
    pub const TF_END: Self = Self(3);
}

/// Town cargo generation modes (`town_type.h`), retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownCargoGenMode(pub u8);
impl TownCargoGenMode {
    /// First cargo generation mode.
    pub const TCGM_BEGIN: Self = Self(0);
    /// Original algorithm (quadratic cargo by population).
    pub const TCGM_ORIGINAL: Self = Self(0);
    /// Bit-counted algorithm (normal distribution from individual house population).
    pub const TCGM_BITCOUNT: Self = Self(1);
    /// End of cargo generation modes.
    pub const TCGM_END: Self = Self(2);
}
