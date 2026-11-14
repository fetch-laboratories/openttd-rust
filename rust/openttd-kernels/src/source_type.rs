/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Inline cargo-source representation from `source_type.h`.

use crate::strings_type::StringID;
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
    pub const INDUSTRY: u8 = SourceType::Industry.0;
    /// `SourceType::Town`.
    pub const TOWN: u8 = SourceType::Town.0;
    /// `SourceType::Headquarters`.
    pub const HEADQUARTERS: u8 = SourceType::Headquarters.0;
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
    /// `Source::GetNewsReference`: get the `NewsReference` for a subsidy Source.
    #[must_use]
    pub fn get_news_reference(self) -> crate::news_type::NewsReference {
        match SourceType(self.kind) {
            SourceType::Industry => {
                crate::news_type::NewsReference::Industry(crate::industry_type::IndustryID(self.id))
            }
            SourceType::Town => {
                crate::news_type::NewsReference::Town(crate::town_type::TownId(self.id))
            }
            _ => crate::services_assert::not_reached(),
        }
    }
    /// `Source::GetFormat`: get the format string for a subsidy Source.
    #[must_use]
    pub fn get_format(self, strings: &FormatStrings) -> StringID {
        match SourceType(self.kind) {
            SourceType::Industry => StringID(strings.industry_name),
            SourceType::Town => StringID(strings.town_name),
            _ => crate::services_assert::not_reached(),
        }
    }
}

/// The generated string IDs `Source::GetFormat` returns.
#[repr(C)]
pub struct FormatStrings {
    /// `STR_INDUSTRY_NAME`.
    pub industry_name: u32,
    /// `STR_TOWN_NAME`.
    pub town_name: u32,
}

/// Native `SourceType`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceType(pub u8);

#[allow(non_upper_case_globals)]
impl SourceType {
    /// Source/destination is an industry
    pub const Industry: Self = Self(0);
    /// Source/destination is a town
    pub const Town: Self = Self(1);
    /// Source/destination are company headquarters
    pub const Headquarters: Self = Self(2);
}

/// Native `SourceID`, retaining every underlying `u16` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceID(pub u16);
