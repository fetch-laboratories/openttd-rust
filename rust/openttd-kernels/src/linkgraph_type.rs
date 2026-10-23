/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `linkgraph_type.h` values. `services.cpp` asserts every value.

/// `LinkGraphID::Invalid().base()`.
pub(crate) const INVALID_LINK_GRAPH: u16 = u16::MAX;

/// `EdgeUpdateModes`, the `EnumBitSet<EdgeUpdateMode, uint8_t>` base value.
#[repr(transparent)]
#[derive(Clone, Copy)]
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
    /// Set union, `{a, b}`.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}
