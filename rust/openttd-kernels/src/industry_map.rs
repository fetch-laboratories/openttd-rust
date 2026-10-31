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
/// `Industry::GetByTile` (`industry.h`).
pub(crate) fn get_industry_by_tile(
    m: &crate::map_func::TileTypeAccess,
    tile: u32,
    get: extern "C" fn(crate::industry_type::IndustryID) -> *mut crate::industry::tile::ShellHandle,
    check: extern "C" fn(bool),
) -> *mut crate::industry::tile::ShellHandle {
    check(m.read(tile) == crate::tile_type::MP_INDUSTRY);
    get(get_industry_index(m, tile).unwrap())
}
/// `GetIndustryType`, with the source assertion before and after its lookup.
pub(crate) fn get_industry_type(
    m: &crate::map_func::TileTypeAccess,
    tile: u32,
    get: extern "C" fn(crate::industry_type::IndustryID) -> *mut crate::industry::tile::ShellHandle,
    owner: extern "C" fn(*mut crate::industry::tile::ShellHandle) -> *mut crate::industry::Industry,
    check: extern "C" fn(bool),
) -> u8 {
    check(m.read(tile) == crate::tile_type::MP_INDUSTRY);
    let shell = get_industry_by_tile(m, tile, get, check);
    check(!shell.is_null());
    unsafe { (*crate::industry::openttd_rust_industry_metadata(owner(shell))).kind }
}
