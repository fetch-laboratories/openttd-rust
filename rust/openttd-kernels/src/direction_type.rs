/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `direction_type.h` values used by the shared helpers and ports.
//! `services.cpp` asserts every value.

/// `DIR_END`.
pub const DIR_END: u8 = Direction::DIR_END.0;
/// `DIR_N`.
pub const DIR_N: u8 = Direction::DIR_N.0;
/// `DIR_NE`.
pub const DIR_NE: u8 = Direction::DIR_NE.0;
/// `DIR_E`.
pub const DIR_E: u8 = Direction::DIR_E.0;
/// `DIR_SE`.
pub const DIR_SE: u8 = Direction::DIR_SE.0;
/// `DIR_S`.
pub const DIR_S: u8 = Direction::DIR_S.0;
/// `DIR_SW`.
pub const DIR_SW: u8 = Direction::DIR_SW.0;
/// `DIR_W`.
pub const DIR_W: u8 = Direction::DIR_W.0;
/// `DIR_NW`.
pub const DIR_NW: u8 = Direction::DIR_NW.0;
/// `DIAGDIR_NE`.
pub const DIAGDIR_NE: u8 = DiagDirection::DIAGDIR_NE.0;
/// `DIAGDIR_SE`.
pub const DIAGDIR_SE: u8 = DiagDirection::DIAGDIR_SE.0;
/// `DIAGDIR_SW`.
pub const DIAGDIR_SW: u8 = DiagDirection::DIAGDIR_SW.0;
/// `DIAGDIR_NW`.
pub const DIAGDIR_NW: u8 = DiagDirection::DIAGDIR_NW.0;
/// `DIAGDIR_BEGIN`.
pub const DIAGDIR_BEGIN: u8 = DiagDirection::DIAGDIR_BEGIN.0;
/// `DIAGDIR_END`.
pub const DIAGDIR_END: u8 = DiagDirection::DIAGDIR_END.0;
/// `INVALID_DIAGDIR`.
pub const INVALID_DIAGDIR: u8 = DiagDirection::INVALID_DIAGDIR.0;

/// `DIAGDIRDIFF_SAME`.
pub const DIAGDIRDIFF_SAME: u8 = DiagDirDiff::DIAGDIRDIFF_SAME.0;
/// `DIAGDIRDIFF_90RIGHT`.
pub const DIAGDIRDIFF_90RIGHT: u8 = DiagDirDiff::DIAGDIRDIFF_90RIGHT.0;
/// `DIAGDIRDIFF_REVERSE`.
pub const DIAGDIRDIFF_REVERSE: u8 = DiagDirDiff::DIAGDIRDIFF_REVERSE.0;
/// `DIAGDIRDIFF_90LEFT`.
pub const DIAGDIRDIFF_90LEFT: u8 = DiagDirDiff::DIAGDIRDIFF_90LEFT.0;
/// `DIAGDIRDIFF_END`.
pub const DIAGDIRDIFF_END: u8 = DiagDirDiff::DIAGDIRDIFF_END.0;
/// `AXIS_X`.
pub const AXIS_X: u8 = Axis::AXIS_X.0;
/// `AXIS_Y`.
pub const AXIS_Y: u8 = Axis::AXIS_Y.0;
/// `INVALID_AXIS`.
pub const INVALID_AXIS: u8 = Axis::INVALID_AXIS.0;

/// `AXIS_END`.
pub const AXIS_END: u8 = Axis::AXIS_END.0;

/// Native `DIRDIFF_REVERSE`.
pub const DIRDIFF_REVERSE: u8 = DirDiff::DIRDIFF_REVERSE.0;
/// Native `DIRDIFF_SAME`.
pub const DIRDIFF_SAME: u8 = DirDiff::DIRDIFF_SAME.0;

/// `DIRDIFF_90RIGHT`.
pub const DIRDIFF_90RIGHT: u8 = DirDiff::DIRDIFF_90RIGHT.0;

/// `DIRDIFF_90LEFT`.
pub const DIRDIFF_90LEFT: u8 = DirDiff::DIRDIFF_90LEFT.0;
/// Native `Axis`, including unnamed uint8 values.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Axis(pub u8);

impl Axis {
    /// Native `AXIS_X`.
    pub const X: Self = Self::AXIS_X;
    /// Native `AXIS_Y`.
    pub const Y: Self = Self::AXIS_Y;
    /// Native `AXIS_END`.
    pub const END: Self = Self::AXIS_END;
    /// Native `INVALID_AXIS`.
    pub const INVALID: Self = Self::INVALID_AXIS;
}

/// Native byte enums use transparent carriers: every bit pattern remains valid
/// across FFI, including source sentinels and values outside named variants.
/// Native `DiagDirection` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DiagDirection(pub u8);

/// Native `Direction`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Direction(pub u8);

#[allow(non_upper_case_globals)]
impl Direction {
    /// Used to iterate
    pub const DIR_BEGIN: Self = Self(0);
    /// North
    pub const DIR_N: Self = Self(0);
    /// Northeast
    pub const DIR_NE: Self = Self(1);
    /// East
    pub const DIR_E: Self = Self(2);
    /// Southeast
    pub const DIR_SE: Self = Self(3);
    /// South
    pub const DIR_S: Self = Self(4);
    /// Southwest
    pub const DIR_SW: Self = Self(5);
    /// West
    pub const DIR_W: Self = Self(6);
    /// Northwest
    pub const DIR_NW: Self = Self(7);
    /// Used to iterate
    pub const DIR_END: Self = Self(8);
    /// Flag for an invalid direction
    pub const INVALID_DIR: Self = Self(255);
}

/// Native `DirDiff`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DirDiff(pub u8);

#[allow(non_upper_case_globals)]
impl DirDiff {
    /// Both directions faces to the same direction
    pub const DIRDIFF_SAME: Self = Self(0);
    /// Angle of 45 degrees right
    pub const DIRDIFF_45RIGHT: Self = Self(1);
    /// Angle of 90 degrees right
    pub const DIRDIFF_90RIGHT: Self = Self(2);
    /// One direction is the opposite of the other one
    pub const DIRDIFF_REVERSE: Self = Self(4);
    /// Angle of 90 degrees left
    pub const DIRDIFF_90LEFT: Self = Self(6);
    /// Angle of 45 degrees left
    pub const DIRDIFF_45LEFT: Self = Self(7);
}

#[allow(non_upper_case_globals)]
impl DiagDirection {
    /// Used for iterations
    pub const DIAGDIR_BEGIN: Self = Self(0);
    /// Northeast, upper right on your monitor
    pub const DIAGDIR_NE: Self = Self(0);
    /// Southeast
    pub const DIAGDIR_SE: Self = Self(1);
    /// Southwest
    pub const DIAGDIR_SW: Self = Self(2);
    /// Northwest
    pub const DIAGDIR_NW: Self = Self(3);
    /// Used for iterations
    pub const DIAGDIR_END: Self = Self(4);
    /// Flag for an invalid DiagDirection
    pub const INVALID_DIAGDIR: Self = Self(255);
}

/// Native `DiagDirDiff`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DiagDirDiff(pub u8);

#[allow(non_upper_case_globals)]
impl DiagDirDiff {
    /// Used for iterations
    pub const DIAGDIRDIFF_BEGIN: Self = Self(0);
    /// Same directions
    pub const DIAGDIRDIFF_SAME: Self = Self(0);
    /// 90 degrees right
    pub const DIAGDIRDIFF_90RIGHT: Self = Self(1);
    /// Reverse directions
    pub const DIAGDIRDIFF_REVERSE: Self = Self(2);
    /// 90 degrees left
    pub const DIAGDIRDIFF_90LEFT: Self = Self(3);
    /// Used for iterations
    pub const DIAGDIRDIFF_END: Self = Self(4);
}

#[allow(non_upper_case_globals)]
impl Axis {
    /// The X axis
    pub const AXIS_X: Self = Self(0);
    /// The y axis
    pub const AXIS_Y: Self = Self(1);
    /// Used for iterations
    pub const AXIS_END: Self = Self(2);
    /// Flag for an invalid Axis
    pub const INVALID_AXIS: Self = Self(255);
}

/// Native `Directions`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Directions(pub u8);

impl Directions {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Set containing the named element (the element is a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: Direction) -> Self {
        Self((1_u64 << element.0) as u8)
    }
}

/// Native `DiagDirections`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DiagDirections(pub u8);

impl DiagDirections {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Set containing the named element (the element is a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: DiagDirection) -> Self {
        Self((1_u64 << element.0) as u8)
    }
}

impl Directions {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: Direction) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl DiagDirections {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: DiagDirection) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl Directions {
    /// All eight directions.
    pub const DIRECTIONS_ALL: Self = Self(u8::MAX);
}

crate::base_bitset_type::impl_bitset_ops!(Directions);
crate::base_bitset_type::impl_bitset_ops!(DiagDirections);
