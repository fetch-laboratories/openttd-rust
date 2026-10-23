/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Read helpers of `clear_map.h` for the audited map reader.
//! `None` routes to the original typed native accessor (including failed assertions).
use crate::map_func::TileTypeAccess;
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
#[inline]
pub(crate) fn is_snow_tile(m: &TileTypeAccess, t: u32) -> Option<bool> {
    m.require_type(t, MP_CLEAR)?;
    Some(m.m3(t)? & 16 != 0)
}
#[inline]
pub(crate) fn get_clear_ground(m: &TileTypeAccess, t: u32) -> Option<u8> {
    m.require_type(t, MP_CLEAR)?;
    Some((m.m5(t)? >> 2) & 7)
}
#[inline]
pub(crate) fn get_clear_density(m: &TileTypeAccess, t: u32) -> Option<u8> {
    m.require_type(t, MP_CLEAR)?;
    Some(m.m5(t)? & 3)
}
#[inline]
pub(crate) fn get_clear_counter(m: &TileTypeAccess, t: u32) -> Option<u8> {
    m.require_type(t, MP_CLEAR)?;
    Some(m.m5(t)? >> 5)
}
#[inline]
fn require_field(m: &TileTypeAccess, t: u32) -> Option<()> {
    m.available(t)?;
    if m.assertions() && get_clear_ground(m, t)? != CLEAR_FIELDS {
        return None;
    }
    Some(())
}
#[inline]
pub(crate) fn get_field_type(m: &TileTypeAccess, t: u32) -> Option<u32> {
    require_field(m, t)?;
    Some(u32::from(m.m3(t)? & 15))
}
#[inline]
pub(crate) fn get_industry_index_of_field(m: &TileTypeAccess, t: u32) -> Option<u16> {
    require_field(m, t)?;
    m.m2(t)
}
#[inline]
pub(crate) fn get_fence(m: &TileTypeAccess, t: u32, side: u8) -> Option<u32> {
    require_field(m, t)?;
    Some(u32::from(match side {
        0 => m.m3(t)? >> 5,
        1 => (m.m4(t)? >> 2) & 7,
        2 => m.m4(t)? >> 5,
        3 => (m.m6(t)? >> 2) & 7,
        _ => unreachable!("DiagDirection"),
    }))
}
#[inline]
pub(crate) fn is_field_tile(m: &TileTypeAccess, t: u32) -> Option<bool> {
    Some(
        crate::tile_map::get_tile_type(m, t)? == MP_CLEAR
            && get_clear_ground(m, t)? == CLEAR_FIELDS,
    )
}
