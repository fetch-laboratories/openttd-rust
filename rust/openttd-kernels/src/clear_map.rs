/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Read helpers of `clear_map.h` for the audited map reader.
//! `None` routes to the original typed native accessor (including failed assertions).
use crate::map_func::{TileFields, TileTypeAccess};
use crate::tile_type::MP_CLEAR;
/// `ClearGround::CLEAR_GRASS`.
pub(crate) const CLEAR_GRASS: u8 = 0;
/// `ClearGround::CLEAR_ROUGH`.
pub(crate) const CLEAR_ROUGH: u8 = 1;
/// `ClearGround::CLEAR_ROCKS`.
pub(crate) const CLEAR_ROCKS: u8 = 2;
/// `ClearGround::CLEAR_DESERT`.
pub(crate) const CLEAR_DESERT: u8 = 5;
/// `ClearGround::CLEAR_FIELDS`.
pub(crate) const CLEAR_FIELDS: u8 = 3;
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_snow_tile(m: &TileTypeAccess, t: u32) -> Option<bool> {
    Some(is_snow_tile_from_fields(&m.require_type(t, MP_CLEAR)?))
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_snow_tile_from_fields(fields: &TileFields<'_>) -> bool {
    fields.m3() & 16 != 0
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_clear_ground(m: &TileTypeAccess, t: u32) -> Option<u8> {
    Some(get_clear_ground_from_fields(&m.require_type(t, MP_CLEAR)?))
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_clear_ground_from_fields(fields: &TileFields<'_>) -> u8 {
    (fields.m5() >> 2) & 7
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_clear_density(m: &TileTypeAccess, t: u32) -> Option<u8> {
    Some(get_clear_density_from_fields(&m.require_type(t, MP_CLEAR)?))
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_clear_density_from_fields(fields: &TileFields<'_>) -> u8 {
    fields.m5() & 3
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_clear_counter(m: &TileTypeAccess, t: u32) -> Option<u8> {
    let fields = m.require_type(t, MP_CLEAR)?;
    Some(fields.m5() >> 5)
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn require_field(m: &TileTypeAccess, t: u32) -> Option<TileFields<'_>> {
    let fields = if m.assertions() {
        let fields = m.require_type(t, MP_CLEAR)?;
        if get_clear_ground_from_fields(&fields) != CLEAR_FIELDS {
            return None;
        }
        fields
    } else {
        m.fields(t)?
    };
    Some(fields)
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_field_type(fields: &TileFields<'_>) -> u32 {
    u32::from(fields.m3() & 15)
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_industry_index_of_field(fields: &TileFields<'_>) -> u16 {
    fields.m2()
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_fence(fields: &TileFields<'_>, side: u8) -> u32 {
    u32::from(match side {
        0 => fields.m3() >> 5,
        1 => (fields.m4() >> 2) & 7,
        2 => fields.m4() >> 5,
        3 => (fields.m6() >> 2) & 7,
        _ => unreachable!("DiagDirection"),
    })
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_field_tile(m: &TileTypeAccess, t: u32) -> Option<bool> {
    Some(
        crate::tile_map::get_tile_type(m, t)? == MP_CLEAR
            && get_clear_ground(m, t)? == CLEAR_FIELDS,
    )
}
