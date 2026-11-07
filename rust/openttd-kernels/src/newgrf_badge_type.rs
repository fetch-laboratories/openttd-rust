/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared definitions from `newgrf_badge_type.h`.

/// Native `BadgeFlag`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BadgeFlag(pub u8);

#[allow(non_upper_case_globals)]
impl BadgeFlag {
    /// Copy badge to related things.
    pub const Copy: Self = Self(0);
    /// Stop adding names to the name list after this badge.
    pub const NameListStop: Self = Self(1);
    /// Don't add this name to the name list if not first.
    pub const NameListFirstOnly: Self = Self(2);
    /// Apply company colour palette to this badge.
    pub const UseCompanyColour: Self = Self(3);
    /// Don't show name in name list at all.
    pub const NameListSkip: Self = Self(4);
    /// Internal flag set if the badge has text.
    pub const HasText: Self = Self(5);
}

/// Native `BadgeFlags`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BadgeFlags(pub u8);

impl BadgeFlags {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Set containing the named element (the element is a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: BadgeFlag) -> Self {
        Self((1_u64 << element.0) as u8)
    }
}

/// Native `BadgeID`, retaining every underlying `u32` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BadgeID(pub u32);

/// Native `BadgeClassID`, retaining every underlying `u32` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BadgeClassID(pub u32);

impl BadgeFlags {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: BadgeFlag) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

crate::base_bitset_type::impl_bitset_ops!(BadgeFlags);
