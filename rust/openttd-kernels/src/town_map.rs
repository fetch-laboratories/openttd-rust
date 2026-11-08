/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Inline helpers of `town_map.h` over the typed house map services.
use crate::tile_type::TileIndex;
use crate::town::house::HouseServices;

/// Increments the age of the house (`IncrementHouseAge`).
/// `t` is the tile of this house; the caller has checked `IsTileType(t, MP_HOUSE)`.
/// One observation reads `IsHouseCompleted(t)` and, for a completed house, `t.m5()`.
pub fn increment_house_age(h: &HouseServices, t: TileIndex) {
    let house = (h.age)(t.0);
    if house.completed && house.age < 0xFF {
        (h.set_age)(t.0, house.age + 1);
    }
}
/// Raw adapter for `increment_house_age`, retained for `town/house.rs` until its pass.
pub fn increment_house_age_raw(h: &HouseServices, t: u32) {
    increment_house_age(h, TileIndex(t));
}
