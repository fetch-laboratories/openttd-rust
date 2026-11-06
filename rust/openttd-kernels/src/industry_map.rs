/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared industry-map graphics identities.
#![allow(unsafe_code)]

pub const GFX_WATERTILE_SPECIALCHECK: u16 = 255;

/// `GetIndustryIndex`, including its tile-type assertion policy.
pub(crate) fn get_industry_index(
    m: &crate::map_func::TileTypeAccess,
    tile: u32,
) -> Option<crate::industry_type::IndustryID> {
    m.require_type(tile, crate::tile_type::MP_INDUSTRY)?;
    Some(crate::industry_type::IndustryID(m.m2(tile)?))
}
/// `Industry::TileBelongsToIndustry`, with its short-circuited map type check.
pub(crate) fn tile_belongs_to_industry(
    m: &crate::map_func::TileTypeAccess,
    tile: u32,
    id: crate::industry_type::IndustryID,
) -> bool {
    m.read(tile) == crate::tile_type::MP_INDUSTRY && get_industry_index(m, tile) == Some(id)
}
/// `Industry::GetByTile` (`industry.h`).
pub(crate) fn get_industry_by_tile(
    m: &crate::map_func::TileTypeAccess,
    tile: u32,
    shared: &crate::services::Services,
    check: extern "C" fn(bool),
) -> crate::industry::Entry {
    check(m.read(tile) == crate::tile_type::MP_INDUSTRY);
    crate::industry::reference(shared, get_industry_index(m, tile).unwrap())
}
/// `GetIndustryType`, with the source assertion before and after its lookup.
pub(crate) fn get_industry_type(
    m: &crate::map_func::TileTypeAccess,
    tile: u32,
    shared: &crate::services::Services,
    check: extern "C" fn(bool),
) -> u8 {
    check(m.read(tile) == crate::tile_type::MP_INDUSTRY);
    let entry = get_industry_by_tile(m, tile, shared, check);
    check(!entry.industry.is_null());
    unsafe { (*entry.owner).record.r#type }
}
