/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared `sprite.h` local sequence payload; no native vector layout is exposed.
#[repr(C)]
#[derive(Default)]
pub(crate) struct Sequence {
    pub origin: [i8; 3],
    pub extent: [u8; 3],
    pub offset: [i8; 3],
    pub image: crate::gfx_type::PalSprite,
}
crate::abi::layouts! {
    fn abi_layout;
    "DrawTileSeqStruct" ["station_service"] => Sequence { origin, extent, offset, image };
}
