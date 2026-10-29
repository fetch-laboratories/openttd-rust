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

/// `COLOUR_DARK_BLUE`.
pub const COLOUR_DARK_BLUE: u8 = 0;

/// `COLOUR_PALE_GREEN`.
pub const COLOUR_PALE_GREEN: u8 = 1;

/// `COLOUR_PINK`.
pub const COLOUR_PINK: u8 = 2;

/// `COLOUR_YELLOW`.
pub const COLOUR_YELLOW: u8 = 3;

/// `COLOUR_LIGHT_BLUE`.
pub const COLOUR_LIGHT_BLUE: u8 = 5;

/// `COLOUR_DARK_GREEN`.
pub const COLOUR_DARK_GREEN: u8 = 7;

/// `COLOUR_CREAM`.
pub const COLOUR_CREAM: u8 = 9;

/// `COLOUR_MAUVE`.
pub const COLOUR_MAUVE: u8 = 10;

/// `COLOUR_PURPLE`.
pub const COLOUR_PURPLE: u8 = 11;

/// `COLOUR_BROWN`.
pub const COLOUR_BROWN: u8 = 13;

/// `COLOUR_GREY`.
pub const COLOUR_GREY: u8 = 14;

/// `COLOUR_WHITE`.
pub const COLOUR_WHITE: u8 = 15;
