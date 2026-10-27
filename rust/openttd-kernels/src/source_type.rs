/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Inline cargo-source representation from `source_type.h`.
/// Canonical native source fields; padding carries no state.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct Source {
    /// Native `SourceID`, including the invalid sentinel.
    pub id: u16,
    /// `SourceType` representation.
    pub kind: u8,
}
impl Source {
    /// `SourceType::Industry`.
    pub const INDUSTRY: u8 = 0;
    /// `SourceType::Town`.
    pub const TOWN: u8 = 1;
    /// `SourceType::Headquarters`.
    pub const HEADQUARTERS: u8 = 2;
    /// A town source.
    #[must_use]
    pub const fn town(id: u16) -> Self {
        Self {
            id,
            kind: Self::TOWN,
        }
    }
    /// An industry source.
    #[must_use]
    pub const fn industry(id: u16) -> Self {
        Self {
            id,
            kind: Self::INDUSTRY,
        }
    }
    /// `Source::IsValid`.
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.id != u16::MAX
    }
}
