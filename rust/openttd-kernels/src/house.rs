/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared values from `house.h`.
pub const TOWN_HOUSE_COMPLETED: u8 = 3;
pub const HOUSE_NO_CLASS: u16 = 0;
pub const NEW_HOUSE_OFFSET: u16 = 110;
pub const INVALID_HOUSE_ID: u16 = u16::MAX;
/// `HouseZone::TownEdge`.
pub const HOUSE_ZONE_TOWN_EDGE: u8 = 0;

/// House capacity and size/availability bits from `house.h`.
pub const NUM_HOUSES: u16 = 4096;
pub const NUM_HOUSES_PER_GRF: u32 = NUM_HOUSES as u32;
pub const HOUSE_NUM_ACCEPTS: u8 = 16;
pub const HOUSE_ORIGINAL_NUM_ACCEPTS: u8 = 3;
pub const BUILDING_HAS_1_TILE: u8 = 0x1d;
pub const BUILDING_HAS_2_TILES: u8 = 0x1c;
pub const BUILDING_HAS_4_TILES: u8 = 0x10;
pub const BUILDING_IS_CHURCH: u8 = 0x40;
pub const BUILDING_IS_STADIUM: u8 = 0x80;
pub const HZ_ZONE_ALL: u16 = 0x1f;
pub const HZ_CLIMATE_ALL: u16 = 0xf800;
