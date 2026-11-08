/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared inline cargo identities from `cargo_type.h`.
#![allow(unsafe_code)]
/// Number of defined cargo slots.
pub const NUM_CARGO: u8 = CargoType::NUM_CARGO.0;
/// `INVALID_CARGO` sentinel.
pub const INVALID_CARGO: u8 = CargoType::INVALID_CARGO.0;
/// `CARGO_AUTO_REFIT`: automatically choose the cargo when auto-refitting.
pub const CARGO_AUTO_REFIT: u8 = CargoType::CARGO_AUTO_REFIT.0;
/// `CARGO_NO_REFIT`: preserve the current cargo type.
pub const CARGO_NO_REFIT: u8 = CargoType::CARGO_NO_REFIT.0;
/// Original `IsValidCargoType`; validity differs from cargo-spec validity.
#[must_use]
pub const fn is_valid_cargo_type(cargo: u8) -> bool {
    cargo != INVALID_CARGO
}

/// Native `CargoArray`: one amount per cargo type.
pub type CargoArray = [u32; NUM_CARGO as usize];

/// Original `CargoArray::GetCount` over the native stack array's actual `data()`.
/// SAFETY: The caller supplies `NUM_CARGO` live scalar elements; no callback runs.
pub(crate) unsafe fn cargo_array_count(data: *const u32) -> u32 {
    (0..usize::from(NUM_CARGO))
        .filter(|i| unsafe { data.add(*i).read() != 0 })
        .count()
        .try_into()
        .unwrap()
}

/// Native cargo labels, in their four-character wire encoding.
pub const CT_PASSENGERS: u32 = CargoLabel::CT_PASSENGERS.0;
/// Native `CT_MAIL`.
pub const CT_MAIL: u32 = CargoLabel::CT_MAIL.0;
/// `CT_COAL`.
pub const CT_COAL: u32 = CargoLabel::CT_COAL.0;
/// `CT_OIL`.
pub const CT_OIL: u32 = CargoLabel::CT_OIL.0;
/// `CT_LIVESTOCK`.
pub const CT_LIVESTOCK: u32 = CargoLabel::CT_LIVESTOCK.0;
/// `CT_GOODS`.
pub const CT_GOODS: u32 = CargoLabel::CT_GOODS.0;
/// `CT_GRAIN`.
pub const CT_GRAIN: u32 = CargoLabel::CT_GRAIN.0;
/// `CT_VALUABLES`.
pub const CT_VALUABLES: u32 = CargoLabel::CT_VALUABLES.0;
/// `CT_WHEAT`.
pub const CT_WHEAT: u32 = CargoLabel::CT_WHEAT.0;
/// `CT_GOLD`.
pub const CT_GOLD: u32 = CargoLabel::CT_GOLD.0;
/// `CT_FOOD`.
pub const CT_FOOD: u32 = CargoLabel::CT_FOOD.0;
/// `CT_FRUIT`.
pub const CT_FRUIT: u32 = CargoLabel::CT_FRUIT.0;
/// `CT_MAIZE`.
pub const CT_MAIZE: u32 = CargoLabel::CT_MAIZE.0;
/// `CT_COPPER_ORE`.
pub const CT_COPPER_ORE: u32 = CargoLabel::CT_COPPER_ORE.0;
/// `CT_DIAMONDS`.
pub const CT_DIAMONDS: u32 = CargoLabel::CT_DIAMONDS.0;
/// `CT_SUGAR`.
pub const CT_SUGAR: u32 = CargoLabel::CT_SUGAR.0;
/// `CT_CANDY`.
pub const CT_CANDY: u32 = CargoLabel::CT_CANDY.0;
/// `CT_COLA`.
pub const CT_COLA: u32 = CargoLabel::CT_COLA.0;
/// Invalid cargo label.
pub const CT_INVALID: u32 = CargoLabel::CT_INVALID.0;

/// Mixed livestock/fruit definition.
pub const MCT_LIVESTOCK_FRUIT: u32 = 0;
/// Mixed grain/wheat/maize definition.
pub const MCT_GRAIN_WHEAT_MAIZE: u32 = 1;
/// Mixed valuables/gold/diamonds definition.
pub const MCT_VALUABLES_GOLD_DIAMONDS: u32 = 2;

/// Original cargo slot count.
pub const NUM_ORIGINAL_CARGO: u8 = CargoType::NUM_ORIGINAL_CARGO.0;

/// All cargo bits, including undefined cargo slots.
pub const ALL_CARGOTYPES: u64 = CargoTypes::ALL_CARGOTYPES.0;

/// `CT_FIZZY_DRINKS`.
pub const CT_FIZZY_DRINKS: u32 = CargoLabel::CT_FIZZY_DRINKS.0;

/// `CT_WOOD`.
pub const CT_WOOD: u32 = CargoLabel::CT_WOOD.0;

/// Native `CargoType`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CargoType(pub u8);

/// Native `CargoTypes`, retaining every underlying `u64` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CargoTypes(pub u64);

/// Native `CargoLabel`, retaining every underlying `u32` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CargoLabel(pub u32);

impl CargoType {
    /// Original number of cargo types.
    pub const NUM_ORIGINAL_CARGO: Self = Self(12);
    /// Maximum number of cargo types in a game.
    pub const NUM_CARGO: Self = Self(64);
    /// Automatically choose cargo type when auto-refitting; stored in saves.
    pub const CARGO_AUTO_REFIT: Self = Self(0xFD);
    /// Do not refit cargo; stored in saves.
    pub const CARGO_NO_REFIT: Self = Self(0xFE);
    /// Invalid cargo slot.
    pub const INVALID_CARGO: Self = Self(u8::MAX);
}

impl CargoTypes {
    /// All cargo slots, including undefined slots.
    pub const ALL_CARGOTYPES: Self = Self(u64::MAX);
}

impl CargoLabel {
    /// Native `CT_PASSENGERS` cargo label.
    pub const CT_PASSENGERS: Self = Self(u32::from_be_bytes(*b"PASS"));
    /// Native `CT_COAL` cargo label.
    pub const CT_COAL: Self = Self(u32::from_be_bytes(*b"COAL"));
    /// Native `CT_MAIL` cargo label.
    pub const CT_MAIL: Self = Self(u32::from_be_bytes(*b"MAIL"));
    /// Native `CT_OIL` cargo label.
    pub const CT_OIL: Self = Self(u32::from_be_bytes(*b"OIL_"));
    /// Native `CT_LIVESTOCK` cargo label.
    pub const CT_LIVESTOCK: Self = Self(u32::from_be_bytes(*b"LVST"));
    /// Native `CT_GOODS` cargo label.
    pub const CT_GOODS: Self = Self(u32::from_be_bytes(*b"GOOD"));
    /// Native `CT_GRAIN` cargo label.
    pub const CT_GRAIN: Self = Self(u32::from_be_bytes(*b"GRAI"));
    /// Native `CT_WOOD` cargo label.
    pub const CT_WOOD: Self = Self(u32::from_be_bytes(*b"WOOD"));
    /// Native `CT_IRON_ORE` cargo label.
    pub const CT_IRON_ORE: Self = Self(u32::from_be_bytes(*b"IORE"));
    /// Native `CT_STEEL` cargo label.
    pub const CT_STEEL: Self = Self(u32::from_be_bytes(*b"STEL"));
    /// Native `CT_VALUABLES` cargo label.
    pub const CT_VALUABLES: Self = Self(u32::from_be_bytes(*b"VALU"));
    /// Native `CT_WHEAT` cargo label.
    pub const CT_WHEAT: Self = Self(u32::from_be_bytes(*b"WHEA"));
    /// Native `CT_PAPER` cargo label.
    pub const CT_PAPER: Self = Self(u32::from_be_bytes(*b"PAPR"));
    /// Native `CT_GOLD` cargo label.
    pub const CT_GOLD: Self = Self(u32::from_be_bytes(*b"GOLD"));
    /// Native `CT_FOOD` cargo label.
    pub const CT_FOOD: Self = Self(u32::from_be_bytes(*b"FOOD"));
    /// Native `CT_RUBBER` cargo label.
    pub const CT_RUBBER: Self = Self(u32::from_be_bytes(*b"RUBR"));
    /// Native `CT_FRUIT` cargo label.
    pub const CT_FRUIT: Self = Self(u32::from_be_bytes(*b"FRUT"));
    /// Native `CT_MAIZE` cargo label.
    pub const CT_MAIZE: Self = Self(u32::from_be_bytes(*b"MAIZ"));
    /// Native `CT_COPPER_ORE` cargo label.
    pub const CT_COPPER_ORE: Self = Self(u32::from_be_bytes(*b"CORE"));
    /// Native `CT_WATER` cargo label.
    pub const CT_WATER: Self = Self(u32::from_be_bytes(*b"WATR"));
    /// Native `CT_DIAMONDS` cargo label.
    pub const CT_DIAMONDS: Self = Self(u32::from_be_bytes(*b"DIAM"));
    /// Native `CT_SUGAR` cargo label.
    pub const CT_SUGAR: Self = Self(u32::from_be_bytes(*b"SUGR"));
    /// Native `CT_TOYS` cargo label.
    pub const CT_TOYS: Self = Self(u32::from_be_bytes(*b"TOYS"));
    /// Native `CT_BATTERIES` cargo label.
    pub const CT_BATTERIES: Self = Self(u32::from_be_bytes(*b"BATT"));
    /// Native `CT_CANDY` cargo label.
    pub const CT_CANDY: Self = Self(u32::from_be_bytes(*b"SWET"));
    /// Native `CT_TOFFEE` cargo label.
    pub const CT_TOFFEE: Self = Self(u32::from_be_bytes(*b"TOFF"));
    /// Native `CT_COLA` cargo label.
    pub const CT_COLA: Self = Self(u32::from_be_bytes(*b"COLA"));
    /// Native `CT_COTTON_CANDY` cargo label.
    pub const CT_COTTON_CANDY: Self = Self(u32::from_be_bytes(*b"CTCD"));
    /// Native `CT_BUBBLES` cargo label.
    pub const CT_BUBBLES: Self = Self(u32::from_be_bytes(*b"BUBL"));
    /// Native `CT_PLASTIC` cargo label.
    pub const CT_PLASTIC: Self = Self(u32::from_be_bytes(*b"PLST"));
    /// Native `CT_FIZZY_DRINKS` cargo label.
    pub const CT_FIZZY_DRINKS: Self = Self(u32::from_be_bytes(*b"FZDR"));
    /// Invalid cargo label.
    pub const CT_INVALID: Self = Self(u32::MAX);
}

impl CargoLabel {
    /// Dummy label for engines with no cargo (zero passengers).
    pub const CT_NONE: Self = Self::CT_PASSENGERS;
}
