/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `linkgraph_type.h` values. `services.cpp` asserts every value.

/// `INVALID_NODE`.
pub const INVALID_NODE: u16 = u16::MAX;
/// `LinkGraphID::Invalid()`.
pub const INVALID_LINK_GRAPH: u16 = LinkGraphID::INVALID.0;

/// `LinkGraphID`, retaining the native u16 coordinate and reuse semantics.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LinkGraphID(pub u16);
impl crate::pool::PoolId for LinkGraphID {
    fn index(self) -> usize {
        usize::from(self.0)
    }
}

/// `LinkGraphJobID`, with the source u16 index/end/null representation.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LinkGraphJobID(pub u16);
impl crate::pool::PoolId for LinkGraphJobID {
    fn index(self) -> usize {
        usize::from(self.0)
    }
}

/// `EdgeUpdateModes`, the `EnumBitSet<EdgeUpdateMode, uint8_t>` base value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeUpdateModes(pub u8);
impl EdgeUpdateModes {
    /// `EdgeUpdateMode::Increase`.
    pub const INCREASE: Self = Self(1 << 0);
    /// `EdgeUpdateMode::Refresh`.
    pub const REFRESH: Self = Self(1 << 1);
    /// `EdgeUpdateMode::Restricted`.
    pub const RESTRICTED: Self = Self(1 << 2);
    /// `EdgeUpdateMode::Unrestricted`.
    pub const UNRESTRICTED: Self = Self(1 << 3);
    /// `EnumBitSet::Test`, for one named update mode.
    #[must_use]
    pub const fn test(self, mode: Self) -> bool {
        self.0 & mode.0 != 0
    }
    /// Set union, `{a, b}`.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl LinkGraphID {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(65535);
    /// Native `PoolID::Invalid()`.
    pub const INVALID: Self = Self(65535);
}

impl LinkGraphJobID {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(65535);
    /// Native `PoolID::Invalid()`.
    pub const INVALID: Self = Self(65535);
}

/// Native `EdgeUpdateMode`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeUpdateMode(pub u8);

#[allow(non_upper_case_globals)]
impl EdgeUpdateMode {
    /// Increase capacity.
    pub const Increase: Self = Self(0);
    /// Refresh capacity.
    pub const Refresh: Self = Self(1);
    /// Use restricted link.
    pub const Restricted: Self = Self(2);
    /// Use unrestricted link.
    pub const Unrestricted: Self = Self(3);
}

/// Native `NodeID`, retaining every underlying `u16` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeID(pub u16);
