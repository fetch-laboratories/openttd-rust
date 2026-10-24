/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared `newgrf_commons.h` cargo group identities and fixed-property helpers.
use std::ffi::c_void;
/// `CargoGRFFileProps::SG_DEFAULT`.
pub(crate) const SG_DEFAULT: u8 = crate::cargo_type::NUM_CARGO;
/// `CargoGRFFileProps::SG_PURCHASE`.
pub(crate) const SG_PURCHASE: u8 = crate::cargo_type::NUM_CARGO + 1;
/// `FixedGRFFileProps::GetFirstSpriteGroupOf`, retaining accessor call order.
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
