/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared livery schemes and native layout from `livery.h`.
/// List of different livery schemes from `livery.h`, retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LiveryScheme(pub u8);

impl LiveryScheme {
    /// Native `LS_BEGIN`.
    pub const LS_BEGIN: Self = Self(0);
    /// Native `LS_DEFAULT`.
    pub const LS_DEFAULT: Self = Self(0);

    // Rail vehicles.
    /// Native `LS_STEAM`.
    pub const LS_STEAM: Self = Self(1);
    /// Native `LS_DIESEL`.
    pub const LS_DIESEL: Self = Self(2);
    /// Native `LS_ELECTRIC`.
    pub const LS_ELECTRIC: Self = Self(3);
    /// Native `LS_MONORAIL`.
    pub const LS_MONORAIL: Self = Self(4);
    /// Native `LS_MAGLEV`.
    pub const LS_MAGLEV: Self = Self(5);
    /// Native `LS_DMU`.
    pub const LS_DMU: Self = Self(6);
    /// Native `LS_EMU`.
    pub const LS_EMU: Self = Self(7);
    /// Native `LS_PASSENGER_WAGON_STEAM`.
    pub const LS_PASSENGER_WAGON_STEAM: Self = Self(8);
    /// Native `LS_PASSENGER_WAGON_DIESEL`.
    pub const LS_PASSENGER_WAGON_DIESEL: Self = Self(9);
    /// Native `LS_PASSENGER_WAGON_ELECTRIC`.
    pub const LS_PASSENGER_WAGON_ELECTRIC: Self = Self(10);
    /// Native `LS_PASSENGER_WAGON_MONORAIL`.
    pub const LS_PASSENGER_WAGON_MONORAIL: Self = Self(11);
    /// Native `LS_PASSENGER_WAGON_MAGLEV`.
    pub const LS_PASSENGER_WAGON_MAGLEV: Self = Self(12);
    /// Native `LS_FREIGHT_WAGON`.
    pub const LS_FREIGHT_WAGON: Self = Self(13);

    // Road vehicles.
    /// Native `LS_BUS`.
    pub const LS_BUS: Self = Self(14);
    /// Native `LS_TRUCK`.
    pub const LS_TRUCK: Self = Self(15);

    // Ships.
    /// Native `LS_PASSENGER_SHIP`.
    pub const LS_PASSENGER_SHIP: Self = Self(16);
    /// Native `LS_FREIGHT_SHIP`.
    pub const LS_FREIGHT_SHIP: Self = Self(17);

    // Aircraft.
    /// Native `LS_HELICOPTER`.
    pub const LS_HELICOPTER: Self = Self(18);
    /// Native `LS_SMALL_PLANE`.
    pub const LS_SMALL_PLANE: Self = Self(19);
    /// Native `LS_LARGE_PLANE`.
    pub const LS_LARGE_PLANE: Self = Self(20);

    // Trams (appear on Road Vehicles tab).
    /// Native `LS_PASSENGER_TRAM`.
    pub const LS_PASSENGER_TRAM: Self = Self(21);
    /// Native `LS_FREIGHT_TRAM`.
    pub const LS_FREIGHT_TRAM: Self = Self(22);
    /// Native `LS_END`.
    pub const LS_END: Self = Self(23);
}

/// Raw compatibility view of `LiveryScheme::LS_BEGIN`.
pub const LS_BEGIN: u8 = LiveryScheme::LS_BEGIN.0;
/// Raw compatibility view of `LiveryScheme::LS_DEFAULT`.
pub const LS_DEFAULT: u8 = LiveryScheme::LS_DEFAULT.0;
/// Raw compatibility view of `LiveryScheme::LS_STEAM`.
pub const LS_STEAM: u8 = LiveryScheme::LS_STEAM.0;
/// Raw compatibility view of `LiveryScheme::LS_DIESEL`.
pub const LS_DIESEL: u8 = LiveryScheme::LS_DIESEL.0;
/// Raw compatibility view of `LiveryScheme::LS_ELECTRIC`.
pub const LS_ELECTRIC: u8 = LiveryScheme::LS_ELECTRIC.0;
/// Raw compatibility view of `LiveryScheme::LS_MONORAIL`.
pub const LS_MONORAIL: u8 = LiveryScheme::LS_MONORAIL.0;
/// Raw compatibility view of `LiveryScheme::LS_MAGLEV`.
pub const LS_MAGLEV: u8 = LiveryScheme::LS_MAGLEV.0;
/// Raw compatibility view of `LiveryScheme::LS_DMU`.
pub const LS_DMU: u8 = LiveryScheme::LS_DMU.0;
/// Raw compatibility view of `LiveryScheme::LS_EMU`.
pub const LS_EMU: u8 = LiveryScheme::LS_EMU.0;
/// Raw compatibility view of `LiveryScheme::LS_PASSENGER_WAGON_STEAM`.
pub const LS_PASSENGER_WAGON_STEAM: u8 = LiveryScheme::LS_PASSENGER_WAGON_STEAM.0;
/// Raw compatibility view of `LiveryScheme::LS_PASSENGER_WAGON_DIESEL`.
pub const LS_PASSENGER_WAGON_DIESEL: u8 = LiveryScheme::LS_PASSENGER_WAGON_DIESEL.0;
/// Raw compatibility view of `LiveryScheme::LS_PASSENGER_WAGON_ELECTRIC`.
pub const LS_PASSENGER_WAGON_ELECTRIC: u8 = LiveryScheme::LS_PASSENGER_WAGON_ELECTRIC.0;
/// Raw compatibility view of `LiveryScheme::LS_PASSENGER_WAGON_MONORAIL`.
pub const LS_PASSENGER_WAGON_MONORAIL: u8 = LiveryScheme::LS_PASSENGER_WAGON_MONORAIL.0;
/// Raw compatibility view of `LiveryScheme::LS_PASSENGER_WAGON_MAGLEV`.
pub const LS_PASSENGER_WAGON_MAGLEV: u8 = LiveryScheme::LS_PASSENGER_WAGON_MAGLEV.0;
/// Raw compatibility view of `LiveryScheme::LS_FREIGHT_WAGON`.
pub const LS_FREIGHT_WAGON: u8 = LiveryScheme::LS_FREIGHT_WAGON.0;
/// Raw compatibility view of `LiveryScheme::LS_BUS`.
pub const LS_BUS: u8 = LiveryScheme::LS_BUS.0;
/// Raw compatibility view of `LiveryScheme::LS_TRUCK`.
pub const LS_TRUCK: u8 = LiveryScheme::LS_TRUCK.0;
/// Raw compatibility view of `LiveryScheme::LS_PASSENGER_SHIP`.
pub const LS_PASSENGER_SHIP: u8 = LiveryScheme::LS_PASSENGER_SHIP.0;
/// Raw compatibility view of `LiveryScheme::LS_FREIGHT_SHIP`.
pub const LS_FREIGHT_SHIP: u8 = LiveryScheme::LS_FREIGHT_SHIP.0;
/// Raw compatibility view of `LiveryScheme::LS_HELICOPTER`.
pub const LS_HELICOPTER: u8 = LiveryScheme::LS_HELICOPTER.0;
/// Raw compatibility view of `LiveryScheme::LS_SMALL_PLANE`.
pub const LS_SMALL_PLANE: u8 = LiveryScheme::LS_SMALL_PLANE.0;
/// Raw compatibility view of `LiveryScheme::LS_LARGE_PLANE`.
pub const LS_LARGE_PLANE: u8 = LiveryScheme::LS_LARGE_PLANE.0;
/// Raw compatibility view of `LiveryScheme::LS_PASSENGER_TRAM`.
pub const LS_PASSENGER_TRAM: u8 = LiveryScheme::LS_PASSENGER_TRAM.0;
/// Raw compatibility view of `LiveryScheme::LS_FREIGHT_TRAM`.
pub const LS_FREIGHT_TRAM: u8 = LiveryScheme::LS_FREIGHT_TRAM.0;
/// Raw compatibility view of `LiveryScheme::LS_END`.
pub const LS_END: u8 = LiveryScheme::LS_END.0;
/// Mask for `Livery::Flag::Primary`.
pub const PRIMARY: u8 = 1 << 0;
/// Mask for `Livery::Flag::Secondary`.
pub const SECONDARY: u8 = 1 << 1;
/// Shared native livery layout; used only through raw field accesses.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Livery {
    /// Livery flags, including unnamed bits.
    pub in_use: u8,
    /// First colour, for all vehicles.
    pub colour1: u8,
    /// Second colour, for vehicles with 2CC support.
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
