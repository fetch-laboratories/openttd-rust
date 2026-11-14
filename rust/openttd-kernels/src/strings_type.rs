/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `strings_type.h` values. `services.cpp` asserts every value.

/// `INVALID_STRING_ID`; `CMD_ERROR` is `CommandCost(INVALID_STRING_ID)`.
pub const INVALID_STRING_ID: u32 = StringID::INVALID_STRING_ID.0;
/// `SPECSTR_TOWNNAME_START`, the first built-in town-name generator string.
pub const SPECSTR_TOWNNAME_START: u32 = 0x20C0;

/// `BUILTIN_TOWNNAME_GENERATOR_COUNT`.
pub const BUILTIN_TOWNNAME_GENERATOR_COUNT: u32 = 21;

/// Bits per string tab (`TAB_SIZE_BITS`).
pub const TAB_SIZE_BITS: u32 = 11;
/// First tab of GRF strings (`TEXT_TAB_NEWGRF_START`).
pub const TEXT_TAB_NEWGRF_START: u32 = StringTab::TEXT_TAB_NEWGRF_START.0 as u32;
/// Maximum GRF text entries (`TAB_SIZE_NEWGRF`).
pub const TAB_SIZE_NEWGRF: usize = (1 << TAB_SIZE_BITS) * 256;
/// `MakeStringID(TEXT_TAB_NEWGRF_START, index)`.
///
/// # Panics
/// The caller supplies a string-table index; an index above `u32::MAX` cannot
/// be represented by the native string ID.
#[must_use]
pub fn grf_string_id(index: usize) -> u32 {
    (TEXT_TAB_NEWGRF_START << TAB_SIZE_BITS) + u32::try_from(index).unwrap()
}

/// Native `StringID`, retaining every underlying `u32` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StringID(pub u32);

/// Native `StringIndexInTab`, retaining every underlying `u32` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StringIndexInTab(pub u32);

impl StringID {
    /// Invalid string ID.
    pub const INVALID_STRING_ID: Self = Self(0xFFFF);
    /// `STR_NULL`, the first generated string of `table/strings.h`.
    pub const STR_NULL: Self = Self(0);
}

/// Native `TextDirection`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextDirection(pub u8);

impl TextDirection {
    /// Text is written left-to-right by default
    pub const TD_LTR: Self = Self(0);
    /// Text is written right-to-left by default
    pub const TD_RTL: Self = Self(1);
}

/// Native `StringTab`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StringTab(pub u8);

impl StringTab {
    /// Native `TEXT_TAB_TOWN`.
    pub const TEXT_TAB_TOWN: Self = Self(4);
    /// Native `TEXT_TAB_INDUSTRY`.
    pub const TEXT_TAB_INDUSTRY: Self = Self(9);
    /// Native `TEXT_TAB_STATION`.
    pub const TEXT_TAB_STATION: Self = Self(12);
    /// Native `TEXT_TAB_SPECIAL`.
    pub const TEXT_TAB_SPECIAL: Self = Self(14);
    /// Native `TEXT_TAB_OLD_CUSTOM`.
    pub const TEXT_TAB_OLD_CUSTOM: Self = Self(15);
    /// Native `TEXT_TAB_VEHICLE`.
    pub const TEXT_TAB_VEHICLE: Self = Self(16);
    /// Native `TEXT_TAB_OLD_NEWGRF`.
    pub const TEXT_TAB_OLD_NEWGRF: Self = Self(26);
    /// End of language files.
    pub const TEXT_TAB_END: Self = Self(32);
    /// Start of `GameScript` supplied strings.
    pub const TEXT_TAB_GAMESCRIPT_START: Self = Self(32);
    /// Start of `NewGRF` supplied strings.
    pub const TEXT_TAB_NEWGRF_START: Self = Self(64);
}

/// Call-local borrowed projection of `StringParameterData` from `strings_type.h`.
/// Native parameter owners retain their strings throughout synchronous encoding.
#[derive(Clone, Copy)]
pub(crate) enum StringParameterData<'a> {
    /// `std::monostate`.
    Empty,
    /// Unsigned numeric parameter.
    Integer(u64),
    /// The parameter's native string bytes.
    String(&'a [u8]),
}

/// Native `EncodedString`: a string with its parameters encoded, which can be stored and
/// decoded later without storing the parameters separately.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct EncodedString {
    /// The encoded string.
    string: Vec<u8>,
}

impl EncodedString {
    /// An `EncodedString` can only be created by `GetEncodedStringWithArgs()`.
    pub(crate) const fn from_encoded(string: Vec<u8>) -> Self {
        Self { string }
    }
    /// The encoded bytes.
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.string
    }
}
