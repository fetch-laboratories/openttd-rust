/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared graphics constants from `gfx_type.h`.

/// The number of a sprite, without mapping bits and colourtables.
pub type SpriteID = u32;

/// The number of the palette (`gfx_type.h`).
pub type PaletteID = u32;

/// `MILLISECONDS_PER_TICK`.
pub(crate) const MILLISECONDS_PER_TICK: u32 = 27;
pub const COLOUR_RED: u8 = Colours::COLOUR_RED.0;
pub const COLOUR_BLUE: u8 = Colours::COLOUR_BLUE.0;
pub const COLOUR_ORANGE: u8 = Colours::COLOUR_ORANGE.0;
pub const COLOUR_GREEN: u8 = Colours::COLOUR_GREEN.0;

/// End of the native Colours enumeration, also the random-colour setting.
pub(crate) const COLOUR_END: u8 = Colours::COLOUR_END.0;

/// Native invalid colour sentinel.
pub const INVALID_COLOUR: u8 = Colours::INVALID_COLOUR.0;

/// `COLOUR_DARK_BLUE`.
pub const COLOUR_DARK_BLUE: u8 = Colours::COLOUR_DARK_BLUE.0;

/// `COLOUR_PALE_GREEN`.
pub const COLOUR_PALE_GREEN: u8 = Colours::COLOUR_PALE_GREEN.0;

/// `COLOUR_YELLOW`.
pub const COLOUR_YELLOW: u8 = Colours::COLOUR_YELLOW.0;

/// `COLOUR_LIGHT_BLUE`.
pub const COLOUR_LIGHT_BLUE: u8 = Colours::COLOUR_LIGHT_BLUE.0;

/// `COLOUR_DARK_GREEN`.
pub const COLOUR_DARK_GREEN: u8 = Colours::COLOUR_DARK_GREEN.0;

/// `COLOUR_CREAM`.
pub const COLOUR_CREAM: u8 = Colours::COLOUR_CREAM.0;

/// `COLOUR_MAUVE`.
pub const COLOUR_MAUVE: u8 = Colours::COLOUR_MAUVE.0;

/// `COLOUR_PURPLE`.
pub const COLOUR_PURPLE: u8 = Colours::COLOUR_PURPLE.0;

/// `COLOUR_BROWN`.
pub const COLOUR_BROWN: u8 = Colours::COLOUR_BROWN.0;

/// `COLOUR_GREY`.
pub const COLOUR_GREY: u8 = Colours::COLOUR_GREY.0;

/// `COLOUR_WHITE`.
pub const COLOUR_WHITE: u8 = Colours::COLOUR_WHITE.0;

/// Native `gfx_type.h::Colours`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Colours(pub u8);

impl Colours {
    /// Beginning of the native colour range.
    pub const COLOUR_BEGIN: Self = Self(0);
    /// Native `COLOUR_DARK_BLUE`.
    pub const COLOUR_DARK_BLUE: Self = Self::COLOUR_BEGIN;
    /// Native `COLOUR_PALE_GREEN`.
    pub const COLOUR_PALE_GREEN: Self = Self(1);
    /// Native `COLOUR_PINK`.
    pub const COLOUR_PINK: Self = Self(2);
    /// Native `COLOUR_YELLOW`.
    pub const COLOUR_YELLOW: Self = Self(Self::COLOUR_PINK.0 + 1);
    /// Native `COLOUR_RED`.
    pub const COLOUR_RED: Self = Self(4);
    /// Native `COLOUR_LIGHT_BLUE`.
    pub const COLOUR_LIGHT_BLUE: Self = Self(5);
    /// Native `COLOUR_GREEN`.
    pub const COLOUR_GREEN: Self = Self(6);
    /// Native `COLOUR_DARK_GREEN`.
    pub const COLOUR_DARK_GREEN: Self = Self(7);
    /// Native `COLOUR_BLUE`.
    pub const COLOUR_BLUE: Self = Self(8);
    /// Native `COLOUR_CREAM`.
    pub const COLOUR_CREAM: Self = Self(9);
    /// Native `COLOUR_MAUVE`.
    pub const COLOUR_MAUVE: Self = Self(10);
    /// Native `COLOUR_PURPLE`.
    pub const COLOUR_PURPLE: Self = Self(11);
    /// Native `COLOUR_ORANGE`.
    pub const COLOUR_ORANGE: Self = Self(12);
    /// Native `COLOUR_BROWN`.
    pub const COLOUR_BROWN: Self = Self(13);
    /// Native `COLOUR_GREY`.
    pub const COLOUR_GREY: Self = Self(14);
    /// Native `COLOUR_WHITE`.
    pub const COLOUR_WHITE: Self = Self(15);
    /// End of the native colour range, also the random-colour setting.
    pub const COLOUR_END: Self = Self(16);
    /// Native invalid colour sentinel.
    pub const INVALID_COLOUR: Self = Self(0xFF);
}

/// `Colour` channels: `ColourRGBA` under Emscripten, otherwise `ColourBGRA` on
/// little-endian and `ColourARGB` on big-endian hosts.
#[cfg(target_os = "emscripten")]
#[repr(C, align(4))]
#[derive(Clone, Copy)]
pub(crate) struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}
/// `Colour` channels: `ColourRGBA` under Emscripten, otherwise `ColourBGRA` on
/// little-endian and `ColourARGB` on big-endian hosts.
#[cfg(all(not(target_os = "emscripten"), target_endian = "little"))]
#[repr(C, align(4))]
#[derive(Clone, Copy)]
pub(crate) struct Colour {
    pub b: u8,
    pub g: u8,
    pub r: u8,
    pub a: u8,
}
/// `Colour` channels: `ColourRGBA` under Emscripten, otherwise `ColourBGRA` on
/// little-endian and `ColourARGB` on big-endian hosts.
#[cfg(all(not(target_os = "emscripten"), target_endian = "big"))]
#[repr(C, align(4))]
#[derive(Clone, Copy)]
pub(crate) struct Colour {
    pub a: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
