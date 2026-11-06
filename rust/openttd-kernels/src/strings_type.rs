/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `strings_type.h` values. `services.cpp` asserts every value.

/// `INVALID_STRING_ID`; `CMD_ERROR` is `CommandCost(INVALID_STRING_ID)`.
pub const INVALID_STRING_ID: u32 = 0xFFFF;
/// `SPECSTR_TOWNNAME_START`, the first built-in town-name generator string.
pub const SPECSTR_TOWNNAME_START: u32 = 0x20C0;

/// `BUILTIN_TOWNNAME_GENERATOR_COUNT`.
pub const BUILTIN_TOWNNAME_GENERATOR_COUNT: u32 = 21;

/// Bits per string tab (`TAB_SIZE_BITS`).
pub const TAB_SIZE_BITS: u32 = 11;
/// First tab of GRF strings (`TEXT_TAB_NEWGRF_START`).
pub const TEXT_TAB_NEWGRF_START: u32 = 64;
/// Maximum GRF text entries (`TAB_SIZE_NEWGRF`).
pub const TAB_SIZE_NEWGRF: usize = (1 << TAB_SIZE_BITS) * 256;
/// `MakeStringID(TEXT_TAB_NEWGRF_START, index)`.
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
}
