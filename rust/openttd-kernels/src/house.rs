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

/// Individual `BuildingFlag` values and footprint masks from `house.h`.
pub const BUILDING_NOT_SLOPED: u8 = 1 << 1;
pub const BUILDING_SIZE_2X1: u8 = 1 << 2;
pub const BUILDING_SIZE_1X2: u8 = 1 << 3;
pub const BUILDING_SIZE_2X2: u8 = 1 << 4;
pub const BUILDING_IS_ANIMATED: u8 = 1 << 5;
pub const BUILDING_2_TILES_X: u8 = BUILDING_SIZE_2X1 | BUILDING_SIZE_2X2;
pub const BUILDING_2_TILES_Y: u8 = BUILDING_SIZE_1X2 | BUILDING_SIZE_2X2;

/// `GetClimateMaskForLandscape`, with the native unreachable landscape domain.
pub(crate) fn climate_mask(landscape: u8) -> u16 {
    match landscape {
        0 => 1 << 12,
        1 => (1 << 11) | (1 << 13),
        2 => 1 << 14,
        3 => 1 << 15,
        _ => unreachable!("landscape"),
    }
}
