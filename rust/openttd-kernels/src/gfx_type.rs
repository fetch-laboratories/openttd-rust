/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared colour constants from `gfx_type.h`.
pub const COLOUR_RED: u8 = 4;
pub const COLOUR_BLUE: u8 = 8;
pub const COLOUR_ORANGE: u8 = 12;
pub const COLOUR_GREEN: u8 = 6;

/// End of the native Colours enumeration, also the random-colour setting.
pub(crate) const COLOUR_END: u8 = 16;

/// Native invalid colour sentinel.
pub const INVALID_COLOUR: u8 = 255;
