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
