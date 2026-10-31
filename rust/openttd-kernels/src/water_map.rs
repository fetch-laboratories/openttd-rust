/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Read helpers of `water_map.h` for the audited map reader.
//! `None` routes to the original typed native accessor (including failed assertions).
use crate::map_func::{TileFields, TileTypeAccess};

use crate::tile_type::{MP_INDUSTRY, MP_OBJECT, MP_STATION, MP_TREES, MP_WATER};
/// `WaterTileType::Coast`.
const COAST: u8 = 1;
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_water_tile_type(m: &TileTypeAccess, t: u32) -> Option<u8> {
    let fields = m.require_type(t, MP_WATER)?;
    Some(fields.m5() >> 4)
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn has_tile_water_class(m: &TileTypeAccess, t: u32) -> Option<bool> {
    Some(has_tile_water_class_from_fields(&m.require_range(t)?))
}
#[allow(clippy::inline_always)]
#[inline(always)]
fn has_tile_water_class_from_fields(fields: &TileFields<'_>) -> bool {
    matches!(
        crate::tile_map::get_tile_type_from_fields(fields),
        MP_WATER | MP_STATION | MP_INDUSTRY | MP_OBJECT | MP_TREES
    )
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_water_class(m: &TileTypeAccess, t: u32) -> Option<u8> {
    let fields = if m.assertions() {
        let fields = m.require_range(t)?;
        if !has_tile_water_class_from_fields(&fields) {
            return None;
        }
        fields
    } else {
        m.fields(t)?
    };
    Some((fields.m1() >> 5) & 3)
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_coast(m: &TileTypeAccess, t: u32) -> Option<bool> {
    Some(get_water_tile_type(m, t)? == COAST)
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_non_flooding_water_tile(m: &TileTypeAccess, t: u32) -> Option<bool> {
    let fields = m.require_type(t, MP_WATER)?;
    Some(fields.m3() & 1 != 0)
}

/// `IsWater`.
pub(crate) fn is_water(m: &TileTypeAccess, tile: u32) -> Option<bool> {
    Some(get_water_tile_type(m, tile)? == 0)
}
