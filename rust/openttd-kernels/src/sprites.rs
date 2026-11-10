/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `table/sprites.h` identifiers used by simulation.
use crate::gfx_type::{PaletteID, SpriteID};
/// `SPR_MAPGEN_BEGIN`.
pub const SPR_MAPGEN_BEGIN: SpriteID = 4845;

/// `SPR_IMG_QUERY`.
pub const SPR_IMG_QUERY: SpriteID = 723;
/// `PAL_NONE`.
pub const PAL_NONE: PaletteID = 0;
/// Number of bits for the sprite number (`SPRITE_WIDTH`).
pub const SPRITE_WIDTH: u8 = 24;

/* These bits are applied to sprite ID */
/// Toggles transparency in the sprite.
pub const TRANSPARENT_BIT: u8 = 31;
/// Toggles recolouring in the sprite.
pub const RECOLOUR_BIT: u8 = 30;
/// Custom sprite modifier bit.
pub const CUSTOM_BIT: u8 = 29;
/// Opaque sprite modifier bit.
pub const OPAQUE_BIT: u8 = 28;

// These masks change the colours of the palette for a sprite. Apart from this
// bit, a sprite number is needed to define the palette used for recolouring.
// This palette is stored in the bits marked by PALETTE_SPRITE_MASK.
// Do not modify the native enum; alter SpriteSetup instead.
/// Set when a sprite originates from an Action 1.
pub const SPRITE_MODIFIER_CUSTOM_SPRITE: u8 = CUSTOM_BIT;
/// Set when a sprite must not ever be displayed transparently.
pub const SPRITE_MODIFIER_OPAQUE: u8 = OPAQUE_BIT;
/// Set when a sprite is to be displayed transparently.
pub const PALETTE_MODIFIER_TRANSPARENT: u8 = TRANSPARENT_BIT;
/// Set when a recolouring process is in action.
pub const PALETTE_MODIFIER_COLOUR: u8 = RECOLOUR_BIT;

/* Easter egg/disaster sprites */
/// `SPR_BLIMP`.
pub const SPR_BLIMP: SpriteID = 3905; // Zeppelin
/// `SPR_BLIMP_CRASHING`.
pub const SPR_BLIMP_CRASHING: SpriteID = 3906;
/// `SPR_BLIMP_CRASHED`.
pub const SPR_BLIMP_CRASHED: SpriteID = 3907;
/// `SPR_UFO_SMALL_SCOUT`.
pub const SPR_UFO_SMALL_SCOUT: SpriteID = 3908; // XCOM - UFO Defense
/// `SPR_UFO_SMALL_SCOUT_DARKER`.
pub const SPR_UFO_SMALL_SCOUT_DARKER: SpriteID = 3909;
/// `SPR_SUB_SMALL_NE`.
pub const SPR_SUB_SMALL_NE: SpriteID = 3910; // Silent Service
/// `SPR_SUB_SMALL_SE`.
pub const SPR_SUB_SMALL_SE: SpriteID = 3911;
/// `SPR_SUB_SMALL_SW`.
pub const SPR_SUB_SMALL_SW: SpriteID = 3912;
/// `SPR_SUB_SMALL_NW`.
pub const SPR_SUB_SMALL_NW: SpriteID = 3913;
/// `SPR_SUB_LARGE_NE`.
pub const SPR_SUB_LARGE_NE: SpriteID = 3914;
/// `SPR_SUB_LARGE_SE`.
pub const SPR_SUB_LARGE_SE: SpriteID = 3915;
/// `SPR_SUB_LARGE_SW`.
pub const SPR_SUB_LARGE_SW: SpriteID = 3916;
/// `SPR_SUB_LARGE_NW`.
pub const SPR_SUB_LARGE_NW: SpriteID = 3917;
/// `SPR_F_15`.
pub const SPR_F_15: SpriteID = 3918; // F-15 Strike Eagle
/// `SPR_F_15_FIRING`.
pub const SPR_F_15_FIRING: SpriteID = 3919;
/// `SPR_UFO_HARVESTER`.
pub const SPR_UFO_HARVESTER: SpriteID = 3920; // XCOM - UFO Defense
/// `SPR_XCOM_SKYRANGER`.
pub const SPR_XCOM_SKYRANGER: SpriteID = 3921;
/// `SPR_AH_64A`.
pub const SPR_AH_64A: SpriteID = 3922; // Gunship
/// `SPR_AH_64A_FIRING`.
pub const SPR_AH_64A_FIRING: SpriteID = 3923;
/// `SPR_ROTOR_MOVING_1`.
pub const SPR_ROTOR_MOVING_1: SpriteID = 3902;
/// `SPR_ROTOR_MOVING_3`.
pub const SPR_ROTOR_MOVING_3: SpriteID = 3904;
