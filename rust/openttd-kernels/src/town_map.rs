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
/// `t` is the tile of this house.
/// One observation reads `IsHouseCompleted(t)` and, for a completed house, `t.m5()`.
pub fn increment_house_age(s: &crate::services::Services, h: &HouseServices, t: TileIndex) {
    let world = crate::map_func::World::new(s);
    world.assert_with(|| {
        crate::tile_map::is_tile_type(s, t.0, crate::tile_type::TileType::MP_HOUSE.0)
    });
    let house = (h.age)(t.0);
    if house.completed && house.age < 0xFF {
        (h.set_age)(t.0, house.age + 1);
    }
}
