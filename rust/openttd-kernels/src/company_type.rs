/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `company_type.h` owner values. `services.cpp` asserts every value.

/// `OWNER_TOWN`.
pub const OWNER_TOWN: u8 = 0x0F;
/// `OWNER_NONE`.
pub const OWNER_NONE: u8 = 0x10;
/// `OWNER_WATER`.
pub const OWNER_WATER: u8 = 0x11;
/// `OWNER_DEITY`.
pub const OWNER_DEITY: u8 = 0x12;
/// `INVALID_OWNER`.
pub const INVALID_OWNER: u8 = 0xFF;

/// `COMPANY_SPECTATOR`.
pub const COMPANY_SPECTATOR: u8 = 0xFF;
