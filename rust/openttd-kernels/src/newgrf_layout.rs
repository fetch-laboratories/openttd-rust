/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Opaque native sprite-layout records, and the call-local sequence the native facade builds.
//! The native facade owns the sequence vector on its stack and Rust controls its construction
//! and move. Field addresses belong to one appended item and end before emplace/reset/move;
//! no Rust reference or field access crosses a checked helper writing that payload.

/// `NewGRFSpriteLayout` (`newgrf_commons.h`), opaque.
#[repr(C)]
pub(crate) struct NewGRFSpriteLayout {
    _opaque: [u8; 0],
}
/// `PalSpriteID` (`gfx_type.h`), opaque.
#[repr(C)]
pub(crate) struct PalSpriteID {
    _opaque: [u8; 0],
}
/// The native facade's call-local `std::vector<DrawTileSeqStruct>`, which has no name in the
/// original; opaque.
#[repr(C)]
pub(crate) struct Sequence {
    _opaque: [u8; 0],
}
/// `DrawTileSeqStruct` (`sprite.h`), opaque.
#[repr(C)]
pub(crate) struct DrawTileSeqStruct {
    _opaque: [u8; 0],
}
