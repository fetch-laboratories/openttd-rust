/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared `gfx_type.h` palette and sprite payload.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(crate) struct PalSprite {
    pub sprite: u32,
    pub pal: u32,
}
crate::abi::layouts! {
    fn abi_layout;
    "PalSpriteID" ["station_service"] => PalSprite { sprite, pal };
}
