/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `tile_type.h` values. `services.cpp` and `landscape.cpp` assert every value.

/// `TILE_SIZE`: tile edge length in world units.
pub const TILE_SIZE: u32 = 16;
/// `TILE_HEIGHT`, shared by raw-to-pixel map consumers.
pub const TILE_HEIGHT: u32 = 8;
/// `TileType::MP_CLEAR`.
pub const MP_CLEAR: u8 = 0;
/// `TileType::MP_RAILWAY`.
pub const MP_RAILWAY: u8 = 1;
/// `TileType::MP_ROAD`.
pub const MP_ROAD: u8 = 2;
/// `TileType::MP_HOUSE`.
pub const MP_HOUSE: u8 = 3;
/// `TileType::MP_TREES`.
pub const MP_TREES: u8 = 4;
/// `TileType::MP_STATION`.
pub const MP_STATION: u8 = 5;
/// `TileType::MP_WATER`.
pub const MP_WATER: u8 = 6;
/// `TileType::MP_VOID`.
pub const MP_VOID: u8 = 7;
/// `TileType::MP_INDUSTRY`.
pub const MP_INDUSTRY: u8 = 8;
/// `TileType::MP_OBJECT`.
pub const MP_OBJECT: u8 = 10;
/// `TileType::MP_TUNNELBRIDGE`.
pub const MP_TUNNELBRIDGE: u8 = 9;
/// `INVALID_TILE`.
pub const INVALID_TILE: u32 = u32::MAX;
