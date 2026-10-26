/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Canonical native layout and call-local sequence payload. The native facade owns
//! the sequence vector on its stack and Rust controls its construction and move.
//! Field addresses belong to one appended item and end before emplace/reset/move;
//! no Rust reference or field access crosses a checked helper writing that payload.
#[repr(C)]
pub(crate) struct Layout {
    _opaque: [u8; 0],
}
#[repr(C)]
pub(crate) struct PalSprite {
    _opaque: [u8; 0],
}
#[repr(C)]
pub(crate) struct Sequence {
    _opaque: [u8; 0],
}
#[repr(C)]
pub(crate) struct SequenceFields {
    pub origin_x: *mut i8,
    pub origin_y: *mut i8,
    pub origin_z: *mut i8,
    pub extent_x: *mut u8,
    pub extent_y: *mut u8,
    pub extent_z: *mut u8,
    pub image: *mut PalSprite,
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDStationSequenceFields" ["station_service"] => SequenceFields { origin_x, origin_y, origin_z, extent_x, extent_y, extent_z, image };
}
