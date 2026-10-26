/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Opaque canonical native `NewGRFSpriteLayout` payload.
#[repr(C)]
pub(crate) struct Layout {
    _opaque: [u8; 0],
}
