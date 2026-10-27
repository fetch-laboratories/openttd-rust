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
pub const MP_CLEAR: u8 = TileType::CLEAR.0;
/// `TileType::MP_RAILWAY`.
pub const MP_RAILWAY: u8 = TileType::RAILWAY.0;
/// `TileType::MP_ROAD`.
pub const MP_ROAD: u8 = TileType::ROAD.0;
/// `TileType::MP_HOUSE`.
pub const MP_HOUSE: u8 = TileType::HOUSE.0;
/// `TileType::MP_TREES`.
pub const MP_TREES: u8 = TileType::TREES.0;
/// `TileType::MP_STATION`.
pub const MP_STATION: u8 = TileType::STATION.0;
/// `TileType::MP_WATER`.
pub const MP_WATER: u8 = TileType::WATER.0;
/// `TileType::MP_VOID`.
pub const MP_VOID: u8 = TileType::VOID.0;
/// `TileType::MP_INDUSTRY`.
pub const MP_INDUSTRY: u8 = TileType::INDUSTRY.0;
/// `TileType::MP_OBJECT`.
pub const MP_OBJECT: u8 = TileType::OBJECT.0;
/// `TileType::MP_TUNNELBRIDGE`.
pub const MP_TUNNELBRIDGE: u8 = TileType::TUNNELBRIDGE.0;
/// `INVALID_TILE`.
pub const INVALID_TILE: u32 = TileIndex::INVALID.0;

/// `MAX_TILE_HEIGHT`.
pub const MAX_TILE_HEIGHT: u32 = 255;

/// `TROPICZONE_NORMAL`.
pub const TROPICZONE_NORMAL: u8 = 0;

/// `TROPICZONE_DESERT`.
pub const TROPICZONE_DESERT: u8 = 1;

/// `TROPICZONE_RAINFOREST`.
pub const TROPICZONE_RAINFOREST: u8 = 2;
/// `TILE_UNIT_MASK`.
pub const TILE_UNIT_MASK: u32 = TILE_SIZE - 1;

/// Native `TileIndex` with its full original integer domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TileIndex(pub u32);
impl TileIndex {
    /// Native invalid sentinel.
    pub const INVALID: Self = Self(u32::MAX);
}

/// Native `TileType`, including unnamed uint8 values.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileType(pub u8);
impl TileType {
    /// Native `MP_CLEAR`.
    pub const CLEAR: Self = Self(0);
    /// Native `MP_RAILWAY`.
    pub const RAILWAY: Self = Self(1);
    /// Native `MP_ROAD`.
    pub const ROAD: Self = Self(2);
    /// Native `MP_HOUSE`.
    pub const HOUSE: Self = Self(3);
    /// Native `MP_TREES`.
    pub const TREES: Self = Self(4);
    /// Native `MP_STATION`.
    pub const STATION: Self = Self(5);
    /// Native `MP_WATER`.
    pub const WATER: Self = Self(6);
    /// Native `MP_VOID`.
    pub const VOID: Self = Self(7);
    /// Native `MP_INDUSTRY`.
    pub const INDUSTRY: Self = Self(8);
    /// Native `MP_TUNNELBRIDGE`.
    pub const TUNNELBRIDGE: Self = Self(9);
    /// Native `MP_OBJECT`.
    pub const OBJECT: Self = Self(10);
}
