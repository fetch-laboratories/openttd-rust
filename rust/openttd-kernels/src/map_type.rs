/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `map_type.h` landscape leveling modes; native compilation asserts the values.

/// `LM_LEVEL`.
pub const LM_LEVEL: u8 = 0;
/// `LM_LOWER`.
pub const LM_LOWER: u8 = 1;
/// `LM_RAISE`.
pub const LM_RAISE: u8 = 2;

/// `TileIndexDiffC` coordinate pair.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TileIndexDiffC {
    pub x: i16,
    pub y: i16,
}
crate::abi::layouts! { fn offset_abi; "TileIndexDiffC" ["probe"] => TileIndexDiffC { x, y, }; }
