/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `livery.h`: the livery schemes, the livery flags and the `Livery` record that
//! `Company` and `Group` hold.
use crate::gfx_type::Colours;

/// List of different livery schemes, retaining every underlying byte.
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

// Raw adapters of `LiveryScheme` for the unfinished `session.rs` and `engine_loading.rs`.
/// Raw compatibility view of `LiveryScheme::LS_DEFAULT`.
pub const LS_DEFAULT: u8 = LiveryScheme::LS_DEFAULT.0;
/// Raw compatibility view of `LiveryScheme::LS_STEAM`.
pub const LS_STEAM: u8 = LiveryScheme::LS_STEAM.0;
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
/// Raw compatibility view of `LiveryScheme::LS_FREIGHT_WAGON`.
pub const LS_FREIGHT_WAGON: u8 = LiveryScheme::LS_FREIGHT_WAGON.0;

/// `Livery::Flag`: the colours a livery sets.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LiveryFlag(pub u8);

#[allow(non_upper_case_globals)]
impl LiveryFlag {
    /// Primary colour is set.
    pub const Primary: Self = Self(0);
    /// Secondary colour is set.
    pub const Secondary: Self = Self(1);
}

/// `Livery::Flags`, the `EnumBitSet` of `LiveryFlag`, retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LiveryFlags(pub u8);

impl LiveryFlags {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Set containing the named element (the element is a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: LiveryFlag) -> Self {
        Self((1_u64 << element.0) as u8)
    }
    /// Native `BaseBitSet::Test`.
    #[must_use]
    pub const fn test(self, element: LiveryFlag) -> bool {
        crate::base_bitset_type::test(self.0 as u64, element.0)
    }
    /// Native `BaseBitSet::Set` of one element.
    pub fn set(&mut self, element: LiveryFlag) {
        self.0 |= Self::from_element(element).0;
    }
    /// Native `BaseBitSet::Set(value, set)`: set or reset one element.
    pub fn set_to(&mut self, element: LiveryFlag, set: bool) {
        if set {
            self.set(element);
        } else {
            self.reset(element);
        }
    }
    /// Native `BaseBitSet::Reset` of one element.
    pub fn reset(&mut self, element: LiveryFlag) {
        self.0 &= !Self::from_element(element).0;
    }
    /// Native `BaseBitSet::Reset` of a set.
    pub fn reset_set(&mut self, other: Self) {
        self.0 &= !other.0;
    }
    /// Native `BaseBitSet::Reset()`: clear every bit.
    pub fn reset_all(&mut self) {
        self.0 = 0;
    }
    /// Native `BaseBitSet::Any` of a set.
    #[must_use]
    pub const fn any(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

crate::base_bitset_type::impl_bitset_ops!(LiveryFlags);

/// Raw mask of `LiveryFlag::Primary`, for the unfinished `fleet.rs`.
pub const PRIMARY: u8 = LiveryFlags::from_element(LiveryFlag::Primary).0;
/// Raw mask of `LiveryFlag::Secondary`, for the unfinished `fleet.rs`.
pub const SECONDARY: u8 = LiveryFlags::from_element(LiveryFlag::Secondary).0;

/// Information about a particular livery.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Livery {
    /// Livery flags.
    pub in_use: LiveryFlags,
    /// First colour, for all vehicles.
    pub colour1: Colours,
    /// Second colour, for vehicles with 2CC support.
    pub colour2: Colours,
}

impl Default for Livery {
    /// The header's member initialisers: no flags and `COLOUR_BEGIN` for both colours.
    fn default() -> Self {
        Self {
            in_use: LiveryFlags::EMPTY,
            colour1: Colours::COLOUR_BEGIN,
            colour2: Colours::COLOUR_BEGIN,
        }
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "Livery" ["company_economy"] => Livery {
        in_use,
        colour1,
        colour2,
    };
}
