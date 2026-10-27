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
pub(crate) struct LayoutSequence {
    _opaque: [u8; 0],
}
