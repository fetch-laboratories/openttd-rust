/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared livery scheme indices from `livery.h`.
pub const LS_STEAM: u8 = 1;
pub const LS_MAGLEV: u8 = 5;
pub const LS_DMU: u8 = 6;
pub const LS_EMU: u8 = 7;
pub const LS_PASSENGER_WAGON_STEAM: u8 = 8;
pub const LS_PASSENGER_WAGON_DIESEL: u8 = 9;
pub const LS_FREIGHT_WAGON: u8 = 13;
pub const LS_DEFAULT: u8 = 0;
pub const LS_DIESEL: u8 = 2;
pub const LS_ELECTRIC: u8 = 3;
pub const LS_MONORAIL: u8 = 4;
pub const LS_PASSENGER_WAGON_ELECTRIC: u8 = 10;
pub const LS_PASSENGER_WAGON_MONORAIL: u8 = 11;
pub const LS_PASSENGER_WAGON_MAGLEV: u8 = 12;
pub const LS_BUS: u8 = 14;
pub const LS_TRUCK: u8 = 15;
pub const LS_PASSENGER_SHIP: u8 = 16;
pub const LS_FREIGHT_SHIP: u8 = 17;
pub const LS_HELICOPTER: u8 = 18;
pub const LS_SMALL_PLANE: u8 = 19;
pub const LS_LARGE_PLANE: u8 = 20;
pub const LS_PASSENGER_TRAM: u8 = 21;
pub const LS_FREIGHT_TRAM: u8 = 22;
pub const LS_END: u8 = 23;
/// Mask for `Livery::Flag::Primary`.
pub const PRIMARY: u8 = 1 << 0;
/// Mask for `Livery::Flag::Secondary`.
pub const SECONDARY: u8 = 1 << 1;
/// Shared native livery layout; used only through raw field accesses.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Livery {
    pub in_use: u8,
    pub colour1: u8,
    pub colour2: u8,
}

crate::abi::layouts! {
    fn abi_layout;
    "Livery" ["company_economy"] => Livery {
        in_use,
        colour1,
        colour2,
    };
}
