/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Inline helpers of `town_map.h` over the typed house map services.
use crate::town::house::HouseServices;

/// `IncrementHouseAge`; the caller has checked `IsTileType(t, MP_HOUSE)`. One
/// observation reads `IsHouseCompleted(t)` and, for a completed house, `t.m5()`.
pub fn increment_house_age(h: &HouseServices, t: u32) {
    let house = (h.age)(t);
    if house.completed && house.age < 0xFF {
        (h.set_age)(t, house.age + 1);
    }
}
