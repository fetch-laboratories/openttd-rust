/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared `newgrf_commons.h` cargo group identities and fixed-property helpers.
use std::ffi::c_void;
/// `CargoGRFFileProps::SG_DEFAULT`.
pub(crate) const SG_DEFAULT: u8 = crate::cargo_type::NUM_CARGO;
/// `CargoGRFFileProps::SG_PURCHASE`.
pub(crate) const SG_PURCHASE: u8 = crate::cargo_type::NUM_CARGO + 1;
/// `CargoGRFFileProps::SG_DEFAULT_NA`.
pub(crate) const SG_DEFAULT_NA: u8 = crate::cargo_type::NUM_CARGO + 2;
/// `FixedGRFFileProps` / `VariableGRFFileProps::GetFirstSpriteGroupOf`, retaining accessor call order.
pub(crate) fn first_sprite_group_of<const N: usize>(
    indices: [u8; N],
    mut get: impl FnMut(u8) -> *const c_void,
) -> *const c_void {
    for key in indices {
        let result = get(key);
        if !result.is_null() {
            return result;
        }
    }
    std::ptr::null()
}

/// `IndustryTileOverrideManager::CheckValidNewID` excludes the water-check marker.
pub(crate) fn industry_tile_check_valid_new_id(id: u16) -> bool {
    id != crate::industry_map::GFX_WATERTILE_SPECIALCHECK
}
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_industry_tile_valid_id(id: u16) -> bool {
    industry_tile_check_valid_new_id(id)
}
