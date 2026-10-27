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

/// `TLF_NOTHING`.
pub const TLF_NOTHING: u8 = 0;
/// `TLF_DODRAW`.
pub const TLF_DODRAW: u8 = 1;
/// `TLF_SPRITE`.
pub const TLF_SPRITE: u8 = 2;
/// `TLF_PALETTE`.
pub const TLF_PALETTE: u8 = 4;
/// `TLF_CUSTOM_PALETTE`.
pub const TLF_CUSTOM_PALETTE: u8 = 8;
/// `TLF_BB_XY_OFFSET`.
pub const TLF_BB_XY_OFFSET: u8 = 16;
/// `TLF_BB_Z_OFFSET`.
pub const TLF_BB_Z_OFFSET: u8 = 32;
/// `TLF_CHILD_X_OFFSET`.
pub const TLF_CHILD_X_OFFSET: u8 = 16;
/// `TLF_CHILD_Y_OFFSET`.
pub const TLF_CHILD_Y_OFFSET: u8 = 32;
/// `TLF_SPRITE_VAR10`.
pub const TLF_SPRITE_VAR10: u8 = 64;
/// `TLF_PALETTE_VAR10`.
pub const TLF_PALETTE_VAR10: u8 = 128;
/// `TLF_KNOWN_FLAGS`.
pub const TLF_KNOWN_FLAGS: u8 = 255;
/// `TLF_DRAWING_FLAGS`.
pub const TLF_DRAWING_FLAGS: u8 = TLF_KNOWN_FLAGS & !TLF_CUSTOM_PALETTE;
/// `TLF_NON_GROUND_FLAGS`.
pub const TLF_NON_GROUND_FLAGS: u8 =
    TLF_BB_XY_OFFSET | TLF_BB_Z_OFFSET | TLF_CHILD_X_OFFSET | TLF_CHILD_Y_OFFSET;
/// `TLF_VAR10_FLAGS`.
pub const TLF_VAR10_FLAGS: u8 = TLF_SPRITE_VAR10 | TLF_PALETTE_VAR10;
/// `TLF_SPRITE_REG_FLAGS`.
pub const TLF_SPRITE_REG_FLAGS: u8 = TLF_DODRAW | TLF_SPRITE | TLF_NON_GROUND_FLAGS;
/// `TLF_PALETTE_REG_FLAGS`.
pub const TLF_PALETTE_REG_FLAGS: u8 = TLF_PALETTE;
/// `TLR_MAX_VAR10`.
pub const TLR_MAX_VAR10: u8 = 7;
/// `IndustryTileOverrideManager::CheckValidNewID` excludes the water-check marker.
pub(crate) fn industry_tile_check_valid_new_id(id: u16) -> bool {
    id != crate::industry_map::GFX_WATERTILE_SPECIALCHECK
}

/// Metadata read together by substitute-spec publication.
#[repr(C)]
pub struct SubstituteSpecIdentity {
    pub grfid: u32,
    pub local: u16,
    pub substitute: u16,
}
/// `AirportTileOverrideManager::CheckValidNewID` excludes the layout marker.
pub(crate) fn airport_tile_check_valid_new_id(id: u16) -> bool {
    id != 0xff
}
