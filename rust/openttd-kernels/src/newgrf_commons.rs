/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared `newgrf_commons.h` vocabulary: tile layout flags, cargo sprite-group keys and the
//! fixed-property helpers.
use crate::cargo_type::CargoType;
use std::ffi::c_void;

/// `TileLayoutFlags`: flags to enable register usage in sprite layouts.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileLayoutFlags(pub u8);
impl TileLayoutFlags {
    /// `TLF_NOTHING`.
    pub const TLF_NOTHING: Self = Self(0x00);

    /// Only draw sprite if value of register `TileLayoutRegisters::dodraw` is non-zero.
    pub const TLF_DODRAW: Self = Self(0x01);
    /// Add signed offset to sprite from register `TileLayoutRegisters::sprite`.
    pub const TLF_SPRITE: Self = Self(0x02);
    /// Add signed offset to palette from register `TileLayoutRegisters::palette`.
    pub const TLF_PALETTE: Self = Self(0x04);
    /// Palette is from Action 1 (moved to `SPRITE_MODIFIER_CUSTOM_SPRITE` in palette during loading).
    pub const TLF_CUSTOM_PALETTE: Self = Self(0x08);

    /// Add signed offset to bounding box X and Y positions from register `TileLayoutRegisters::delta.parent[0..1]`.
    pub const TLF_BB_XY_OFFSET: Self = Self(0x10);
    /// Add signed offset to bounding box Z positions from register `TileLayoutRegisters::delta.parent[2]`.
    pub const TLF_BB_Z_OFFSET: Self = Self(0x20);

    /// Add signed offset to child sprite X positions from register `TileLayoutRegisters::delta.child[0]`.
    pub const TLF_CHILD_X_OFFSET: Self = Self(0x10);
    /// Add signed offset to child sprite Y positions from register `TileLayoutRegisters::delta.child[1]`.
    pub const TLF_CHILD_Y_OFFSET: Self = Self(0x20);

    /// Resolve sprite with a specific value in variable 10.
    pub const TLF_SPRITE_VAR10: Self = Self(0x40);
    /// Resolve palette with a specific value in variable 10.
    pub const TLF_PALETTE_VAR10: Self = Self(0x80);

    /// Known flags. Any unknown set flag will disable the GRF.
    pub const TLF_KNOWN_FLAGS: Self = Self(0xFF);

    /// Flags which are still required after loading the GRF.
    pub const TLF_DRAWING_FLAGS: Self = Self(Self::TLF_KNOWN_FLAGS.0 & !Self::TLF_CUSTOM_PALETTE.0);

    /// Flags which do not work for the (first) ground sprite.
    pub const TLF_NON_GROUND_FLAGS: Self = Self(
        Self::TLF_BB_XY_OFFSET.0
            | Self::TLF_BB_Z_OFFSET.0
            | Self::TLF_CHILD_X_OFFSET.0
            | Self::TLF_CHILD_Y_OFFSET.0,
    );

    /// Flags which refer to using multiple action-1-2-3 chains.
    pub const TLF_VAR10_FLAGS: Self = Self(Self::TLF_SPRITE_VAR10.0 | Self::TLF_PALETTE_VAR10.0);

    /// Flags which require resolving the action-1-2-3 chain for the sprite, even if it is no action-1 sprite.
    pub const TLF_SPRITE_REG_FLAGS: Self = Self(
        Self::TLF_DODRAW.0
            | Self::TLF_SPRITE.0
            | Self::TLF_BB_XY_OFFSET.0
            | Self::TLF_BB_Z_OFFSET.0
            | Self::TLF_CHILD_X_OFFSET.0
            | Self::TLF_CHILD_Y_OFFSET.0,
    );

    /// Flags which require resolving the action-1-2-3 chain for the palette, even if it is no action-1 palette.
    pub const TLF_PALETTE_REG_FLAGS: Self = Self::TLF_PALETTE;
}

/// Maximum value for var 10.
pub const TLR_MAX_VAR10: u32 = 7;

/// `IndustryTileOverrideManager::CheckValidNewID`.
pub(crate) fn industry_tile_check_valid_new_id(testid: u16) -> bool {
    testid != 0xFF
}

/// Metadata read together by substitute-spec publication.
#[repr(C)]
pub struct SubstituteSpecIdentity {
    /// GRF owning this definition.
    pub grfid: u32,
    /// GRF-local specification ID.
    pub local: u16,
    /// Original specification used as the substitute.
    pub substitute: u16,
}
/// `AirportTileOverrideManager::CheckValidNewID`.
pub(crate) fn airport_tile_check_valid_new_id(testid: u16) -> bool {
    testid != 0xFF
}

/// `FixedGRFFileProps::GetFirstSpriteGroupOf` and `VariableGRFFileProps::GetFirstSpriteGroupOf`:
/// get the first existing `SpriteGroup` from a list of options, or null if none exists. `Tkey`
/// is the props' key type and `get` its `GetSpriteGroup`, called in the order of `indices`.
pub(crate) fn get_first_sprite_group_of<Tkey: Copy, const N: usize>(
    indices: [Tkey; N],
    mut get: impl FnMut(Tkey) -> *const c_void,
) -> *const c_void {
    for key in indices {
        let result = get(key);
        if !result.is_null() {
            return result;
        }
    }
    std::ptr::null()
}

/// `StandardSpriteGroup`: standard sprite groups.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StandardSpriteGroup(pub i32);

#[allow(non_upper_case_globals)]
impl StandardSpriteGroup {
    /// Default type used when no more-specific group matches.
    pub const Default: Self = Self(0);
    /// Used before an entity exists.
    pub const Purchase: Self = Self(1);
    /// `End`.
    pub const End: Self = Self(2);
}

/// `CargoGRFFileProps`, which here only holds its sprite-group keys.
pub enum CargoGRFFileProps {}
impl CargoGRFFileProps {
    /// Default type used when no more-specific cargo matches.
    pub const SG_DEFAULT: CargoType = CargoType::NUM_CARGO;
    /// Used in purchase lists before an item exists.
    pub const SG_PURCHASE: CargoType = CargoType(CargoType::NUM_CARGO.0 + 1);
    /// Used only by stations and roads when no more-specific cargo matches.
    pub const SG_DEFAULT_NA: CargoType = CargoType(CargoType::NUM_CARGO.0 + 2);
}

// Raw values of the typed constants for files no pass has finished; each goes with its last
// user's pass (#734's residue).

/// `CargoGRFFileProps::SG_DEFAULT`, for `engine/bindings.rs`, `newgrf_act3/loading.rs`,
/// `newgrf_vehicle.rs`, `newgrf_station.rs` and `newgrf_roadstop.rs`.
pub(crate) const SG_DEFAULT: u8 = CargoGRFFileProps::SG_DEFAULT.0;
/// `CargoGRFFileProps::SG_PURCHASE`, for `newgrf_act3.rs`, `newgrf_vehicle.rs`,
/// `newgrf_station.rs` and `newgrf_roadstop.rs`.
pub(crate) const SG_PURCHASE: u8 = CargoGRFFileProps::SG_PURCHASE.0;
/// `CargoGRFFileProps::SG_DEFAULT_NA`, for `newgrf_act3.rs`, `newgrf_station.rs` and
/// `newgrf_roadstop.rs`.
pub(crate) const SG_DEFAULT_NA: u8 = CargoGRFFileProps::SG_DEFAULT_NA.0;

/// `TileLayoutFlags::TLF_NOTHING`, for `newgrf_act2.rs`.
pub const TLF_NOTHING: u8 = TileLayoutFlags::TLF_NOTHING.0;
/// `TileLayoutFlags::TLF_DODRAW`, for `newgrf_act2.rs`.
pub const TLF_DODRAW: u8 = TileLayoutFlags::TLF_DODRAW.0;
/// `TileLayoutFlags::TLF_SPRITE`, for `newgrf_act2.rs`.
pub const TLF_SPRITE: u8 = TileLayoutFlags::TLF_SPRITE.0;
/// `TileLayoutFlags::TLF_PALETTE`, for `newgrf_act2.rs`.
pub const TLF_PALETTE: u8 = TileLayoutFlags::TLF_PALETTE.0;
/// `TileLayoutFlags::TLF_CUSTOM_PALETTE`, for `newgrf_act2.rs`.
pub const TLF_CUSTOM_PALETTE: u8 = TileLayoutFlags::TLF_CUSTOM_PALETTE.0;
/// `TileLayoutFlags::TLF_BB_XY_OFFSET`, for `newgrf_act2.rs`.
pub const TLF_BB_XY_OFFSET: u8 = TileLayoutFlags::TLF_BB_XY_OFFSET.0;
/// `TileLayoutFlags::TLF_BB_Z_OFFSET`, for `newgrf_act2.rs`.
pub const TLF_BB_Z_OFFSET: u8 = TileLayoutFlags::TLF_BB_Z_OFFSET.0;
/// `TileLayoutFlags::TLF_CHILD_X_OFFSET`, for `newgrf_act2.rs`.
pub const TLF_CHILD_X_OFFSET: u8 = TileLayoutFlags::TLF_CHILD_X_OFFSET.0;
/// `TileLayoutFlags::TLF_CHILD_Y_OFFSET`, for `newgrf_act2.rs`.
pub const TLF_CHILD_Y_OFFSET: u8 = TileLayoutFlags::TLF_CHILD_Y_OFFSET.0;
/// `TileLayoutFlags::TLF_SPRITE_VAR10`, for `newgrf_act2.rs`.
pub const TLF_SPRITE_VAR10: u8 = TileLayoutFlags::TLF_SPRITE_VAR10.0;
/// `TileLayoutFlags::TLF_PALETTE_VAR10`, for `newgrf_act2.rs`.
pub const TLF_PALETTE_VAR10: u8 = TileLayoutFlags::TLF_PALETTE_VAR10.0;
/// `TileLayoutFlags::TLF_KNOWN_FLAGS`, for `newgrf_act2.rs`.
pub const TLF_KNOWN_FLAGS: u8 = TileLayoutFlags::TLF_KNOWN_FLAGS.0;
/// `TileLayoutFlags::TLF_DRAWING_FLAGS`, for `newgrf_act2.rs`.
pub const TLF_DRAWING_FLAGS: u8 = TileLayoutFlags::TLF_DRAWING_FLAGS.0;
/// `TileLayoutFlags::TLF_NON_GROUND_FLAGS`, for `newgrf_act2.rs`.
pub const TLF_NON_GROUND_FLAGS: u8 = TileLayoutFlags::TLF_NON_GROUND_FLAGS.0;
/// `TileLayoutFlags::TLF_VAR10_FLAGS`, for `newgrf_act2.rs`.
pub const TLF_VAR10_FLAGS: u8 = TileLayoutFlags::TLF_VAR10_FLAGS.0;
/// `TileLayoutFlags::TLF_SPRITE_REG_FLAGS`, for `newgrf_act2.rs`.
pub const TLF_SPRITE_REG_FLAGS: u8 = TileLayoutFlags::TLF_SPRITE_REG_FLAGS.0;
/// `TileLayoutFlags::TLF_PALETTE_REG_FLAGS`, for `newgrf_act2.rs`.
pub const TLF_PALETTE_REG_FLAGS: u8 = TileLayoutFlags::TLF_PALETTE_REG_FLAGS.0;
