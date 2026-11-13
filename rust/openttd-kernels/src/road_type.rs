/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared values from `road_type.h`.
/// `INVALID_ROADTYPE`.
pub const INVALID_ROADTYPE: u8 = RoadType::INVALID_ROADTYPE.0;

/// `ROAD_X`.
pub const ROAD_X: u8 = RoadBits::ROAD_X.0;
/// `ROAD_Y`.
pub const ROAD_Y: u8 = RoadBits::ROAD_Y.0;

/// `ROAD_W`.
pub const ROAD_W: u8 = RoadBits::ROAD_W.0;
/// `ROAD_N`.
pub const ROAD_N: u8 = RoadBits::ROAD_N.0;
/// `ROAD_E`.
pub const ROAD_E: u8 = RoadBits::ROAD_E.0;
/// `ROAD_S`.
pub const ROAD_S: u8 = RoadBits::ROAD_S.0;
/// `ROAD_NW`.
pub const ROAD_NW: u8 = RoadBits::ROAD_NW.0;

/// `ROAD_SW`.
pub const ROAD_SW: u8 = RoadBits::ROAD_SW.0;
/// `ROAD_SE`.
pub const ROAD_SE: u8 = RoadBits::ROAD_SE.0;
/// `ROAD_NE`.
pub const ROAD_NE: u8 = RoadBits::ROAD_NE.0;

/// `ROADTYPE_END`.
pub const ROADTYPE_END: u8 = RoadType::ROADTYPE_END.0;

/// `ROADTYPE_ROAD`.
pub const ROADTYPE_ROAD: u8 = RoadType::ROADTYPE_ROAD.0;
/// `ROADTYPE_TRAM`.
pub const ROADTYPE_TRAM: u8 = RoadType::ROADTYPE_TRAM.0;

/// `ROADTYPE_BEGIN`.
pub const ROADTYPE_BEGIN: u8 = RoadType::ROADTYPE_BEGIN.0;
/// `RTT_ROAD`.
pub const RTT_ROAD: RoadTramType = RoadTramType::RTT_ROAD;
/// `RTT_TRAM`.
pub const RTT_TRAM: RoadTramType = RoadTramType::RTT_TRAM;

/// `ROAD_NONE`.
pub const ROAD_NONE: u8 = RoadBits::ROAD_NONE.0;

/// `ROAD_END`.
pub const ROAD_END: u8 = RoadBits::ROAD_END.0;

/// `DisallowedRoadDirections::DRD_NONE`.
pub const DRD_NONE: u8 = DisallowedRoadDirections::DRD_NONE.0;
/// `DisallowedRoadDirections::DRD_SOUTHBOUND`.
pub const DRD_SOUTHBOUND: u8 = DisallowedRoadDirections::DRD_SOUTHBOUND.0;
/// `DisallowedRoadDirections::DRD_NORTHBOUND`.
pub const DRD_NORTHBOUND: u8 = DisallowedRoadDirections::DRD_NORTHBOUND.0;
/// `DisallowedRoadDirections::DRD_BOTH`.
pub const DRD_BOTH: u8 = DisallowedRoadDirections::DRD_BOTH.0;

/// Native `RoadTramType : bool`; true denotes the tram mask.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoadTramType(pub bool);

/// Native `RoadType` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoadType(pub u8);

/// Native `RoadBits` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoadBits(pub u8);

impl RoadType {
    /// Used for iterations
    pub const ROADTYPE_BEGIN: Self = Self(0);
    /// Basic road type
    pub const ROADTYPE_ROAD: Self = Self(0);
    /// Trams
    pub const ROADTYPE_TRAM: Self = Self(1);
    /// Used for iterations
    pub const ROADTYPE_END: Self = Self(63);
    /// flag for invalid roadtype
    pub const INVALID_ROADTYPE: Self = Self(63);
}

impl RoadTramType {
    /// Road road type.
    pub const RTT_ROAD: Self = Self(false);
    /// Tram road type.
    pub const RTT_TRAM: Self = Self(true);
}

/// Native `RoadTramTypes`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoadTramTypes(pub u8);

impl RoadTramTypes {
    /// Road road type bit.
    pub const RTTB_ROAD: Self = Self(1);
    /// Tram road type bit.
    pub const RTTB_TRAM: Self = Self(2);
}

impl RoadBits {
    /// No road-part is build
    pub const ROAD_NONE: Self = Self(0);
    /// North-west part
    pub const ROAD_NW: Self = Self(1);
    /// South-west part
    pub const ROAD_SW: Self = Self(2);
    /// South-east part
    pub const ROAD_SE: Self = Self(4);
    /// North-east part
    pub const ROAD_NE: Self = Self(8);
    /// Full road along the x-axis (south-west + north-east)
    pub const ROAD_X: Self = Self(10);
    /// Full road along the y-axis (north-west + south-east)
    pub const ROAD_Y: Self = Self(5);
    /// Road at the two northern edges
    pub const ROAD_N: Self = Self(9);
    /// Road at the two eastern edges
    pub const ROAD_E: Self = Self(12);
    /// Road at the two southern edges
    pub const ROAD_S: Self = Self(6);
    /// Road at the two western edges
    pub const ROAD_W: Self = Self(3);
    /// Full 4-way crossing
    pub const ROAD_ALL: Self = Self(15);
    /// Out-of-range roadbits, used for iterations
    pub const ROAD_END: Self = Self(16);
}

/// Native `DisallowedRoadDirections`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DisallowedRoadDirections(pub u8);

impl DisallowedRoadDirections {
    /// None of the directions are disallowed
    pub const DRD_NONE: Self = Self(0);
    /// All southbound traffic is disallowed
    pub const DRD_SOUTHBOUND: Self = Self(1);
    /// All northbound traffic is disallowed
    pub const DRD_NORTHBOUND: Self = Self(2);
    /// All directions are disallowed
    pub const DRD_BOTH: Self = Self(3);
    /// Sentinel
    pub const DRD_END: Self = Self(4);
}

/// Native `RoadTypes`, retaining every underlying `u64` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoadTypes(pub u64);

impl RoadTypes {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Set containing the named element (the element is a bit position).
    #[must_use]
    pub const fn from_element(element: RoadType) -> Self {
        Self(1_u64 << element.0)
    }
}

impl RoadTypes {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: RoadType) -> bool {
        self.0 & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    /// Native `BaseBitSet::Reset` of every element in `other`.
    #[must_use]
    pub const fn reset(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }
    /// Native `BaseBitSet::Any`: test if any of the given values are set.
    #[must_use]
    pub const fn any(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
    /// Native `BaseBitSet::All`: test if all of the given values are set.
    #[must_use]
    pub const fn all(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
    /// Native `BaseBitSet` iteration, visiting set elements in ascending bit order.
    pub fn iter(self) -> impl Iterator<Item = RoadType> {
        let mut bits = self.0;
        std::iter::from_fn(move || {
            if bits == 0 {
                return None;
            }
            let element = RoadType(crate::bitmath_func::find_first_bit(bits));
            bits &= bits - 1;
            Some(element)
        })
    }
}

/// Native `RoadTypeLabel`, retaining every underlying `u32` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoadTypeLabel(pub u32);

crate::base_bitset_type::impl_bitset_ops!(RoadTypes);
crate::base_bitset_type::impl_bitset_ops!(RoadBits);
crate::base_bitset_type::impl_bitset_ops!(RoadTramTypes);
crate::base_bitset_type::impl_bitset_ops!(DisallowedRoadDirections);

impl RoadTypeLabel {
    /// Standard road label.
    pub const ROADTYPE_LABEL_ROAD: Self = Self(u32::from_be_bytes(*b"ROAD"));
    /// Tramway label.
    pub const ROADTYPE_LABEL_TRAM: Self = Self(u32::from_be_bytes(*b"ELRL"));
}
