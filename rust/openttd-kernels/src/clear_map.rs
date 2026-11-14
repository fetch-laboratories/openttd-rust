/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Read helpers of `clear_map.h` for the audited map reader.
//! `None` routes to the original typed native accessor (including failed assertions).
use crate::direction_type::DiagDirection;
use crate::industry_type::IndustryID;
use crate::map_func::{TileFields, TileTypeAccess};
use crate::tile_type::TileIndex;
use crate::tile_type::TileType;
/// `ClearGround::CLEAR_GRASS`.
pub(crate) const CLEAR_GRASS: u8 = ClearGround::CLEAR_GRASS.0;
/// `ClearGround::CLEAR_ROUGH`.
pub(crate) const CLEAR_ROUGH: u8 = ClearGround::CLEAR_ROUGH.0;
/// `ClearGround::CLEAR_ROCKS`.
pub(crate) const CLEAR_ROCKS: u8 = ClearGround::CLEAR_ROCKS.0;
/// `ClearGround::CLEAR_DESERT`.
pub(crate) const CLEAR_DESERT: u8 = ClearGround::CLEAR_DESERT.0;
/// `ClearGround::CLEAR_FIELDS`.
pub(crate) const CLEAR_FIELDS: u8 = ClearGround::CLEAR_FIELDS.0;
/// `IsSnowTile`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_snow_tile(m: &TileTypeAccess, t: TileIndex) -> Option<bool> {
    Some(is_snow_tile_from_fields(
        &m.require_type(t, TileType::MP_CLEAR)?,
    ))
}
/// `IsSnowTile` over fields already checked as `MP_CLEAR`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_snow_tile_from_fields(fields: &TileFields<'_>) -> bool {
    fields.m3() & 16 != 0
}
/// `GetClearGround`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_clear_ground(m: &TileTypeAccess, t: TileIndex) -> Option<ClearGround> {
    Some(get_clear_ground_from_fields(
        &m.require_type(t, TileType::MP_CLEAR)?,
    ))
}
/// `IsClearGround`: whether the clear tile has the given ground type.
#[inline]
pub(crate) fn is_clear_ground(m: &TileTypeAccess, t: TileIndex, ct: ClearGround) -> Option<bool> {
    Some(get_clear_ground(m, t)? == ct)
}
/// `GetClearGround` over fields already checked as `MP_CLEAR`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_clear_ground_from_fields(fields: &TileFields<'_>) -> ClearGround {
    ClearGround((fields.m5() >> 2) & 7)
}
/// `GetClearDensity`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_clear_density(m: &TileTypeAccess, t: TileIndex) -> Option<u8> {
    Some(get_clear_density_from_fields(
        &m.require_type(t, TileType::MP_CLEAR)?,
    ))
}
/// `GetClearDensity` over fields already checked as `MP_CLEAR`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_clear_density_from_fields(fields: &TileFields<'_>) -> u8 {
    fields.m5() & 3
}
/// `GetClearCounter`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_clear_counter(m: &TileTypeAccess, t: TileIndex) -> Option<u8> {
    let fields = m.require_type(t, TileType::MP_CLEAR)?;
    Some(fields.m5() >> 5)
}
/// `AddClearDensity`: increment the density of a non-field clear tile.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "`t.m5() += d` stores the int sum back into the original uint8_t m5"
)]
pub(crate) fn add_clear_density(m: &TileTypeAccess, t: TileIndex, d: i32) -> Option<()> {
    let fields = m.require_type(t, TileType::MP_CLEAR)?; // XXX incomplete
    m.set_m5(t, i32::from(fields.m5()).wrapping_add(d) as u8)
}
/// `AddClearCounter`: increments the counter used to advance to the next clear density/field
/// type.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "`t.m5() += c << 5` stores the int sum back into the original uint8_t m5"
)]
pub(crate) fn add_clear_counter(m: &TileTypeAccess, t: TileIndex, c: i32) -> Option<()> {
    let fields = m.require_type(t, TileType::MP_CLEAR)?; // XXX incomplete
    m.set_m5(t, i32::from(fields.m5()).wrapping_add(c << 5) as u8)
}
/// `SetClearGroundDensity`: sets ground type and density in one go, also sets the counter to 0.
#[allow(
    clippy::cast_possible_truncation,
    reason = "`t.m5() = 0 << 5 | type << 2 | density` stores into the original uint8_t m5"
)]
pub(crate) fn set_clear_ground_density(
    m: &TileTypeAccess,
    t: TileIndex,
    r#type: ClearGround,
    density: u32,
) -> Option<()> {
    m.require_type(t, TileType::MP_CLEAR)?; // XXX incomplete
    m.set_m5(t, (u32::from(r#type.0) << 2 | density) as u8)
}
/// The fields of a tile, checked as `IsClearGround(t, CLEAR_FIELDS)` when asserting.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn require_field(m: &TileTypeAccess, t: TileIndex) -> Option<TileFields<'_>> {
    let fields = if m.assertions() {
        let fields = m.require_type(t, TileType::MP_CLEAR)?;
        m.check(get_clear_ground_from_fields(&fields) == ClearGround::CLEAR_FIELDS)?;
        fields
    } else {
        m.fields(t)?
    };
    Some(fields)
}
/// `GetFieldType`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_field_type(fields: &TileFields<'_>) -> u32 {
    u32::from(fields.m3() & 15)
}
/// `GetIndustryIndexOfField`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_industry_index_of_field(fields: &TileFields<'_>) -> IndustryID {
    IndustryID(fields.m2())
}
/// `GetFence`; the caller supplies its non-returning failure operation.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_fence(
    fields: &TileFields<'_>,
    side: DiagDirection,
    not_reached: impl FnOnce(),
) -> u32 {
    u32::from(match side {
        DiagDirection::DIAGDIR_SE => (fields.m4() >> 2) & 7,
        DiagDirection::DIAGDIR_SW => fields.m4() >> 5,
        DiagDirection::DIAGDIR_NE => fields.m3() >> 5,
        DiagDirection::DIAGDIR_NW => (fields.m6() >> 2) & 7,
        _ => {
            not_reached();
            std::process::abort()
        }
    })
}

/// Raw adapter: `IsSnowTile` over a base tile, for files no pass has finished yet.
#[inline]
pub(crate) fn is_snow_tile_raw(m: &TileTypeAccess, t: u32) -> Option<bool> {
    is_snow_tile(m, TileIndex(t))
}
/// Raw adapter: `GetClearGround` over base values.
#[inline]
pub(crate) fn get_clear_ground_raw(m: &TileTypeAccess, t: u32) -> Option<u8> {
    get_clear_ground(m, TileIndex(t)).map(|g| g.0)
}

/// Native `ClearGround` representation; every byte pattern is retained.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClearGround(pub u8);

impl ClearGround {
    /// 0-3
    pub const CLEAR_GRASS: Self = Self(0);
    /// 3
    pub const CLEAR_ROUGH: Self = Self(1);
    /// 3
    pub const CLEAR_ROCKS: Self = Self(2);
    /// 3
    pub const CLEAR_FIELDS: Self = Self(3);
    /// 0-3 (Not stored in map.)
    pub const CLEAR_SNOW: Self = Self(4);
    /// 1,3
    pub const CLEAR_DESERT: Self = Self(5);
}
