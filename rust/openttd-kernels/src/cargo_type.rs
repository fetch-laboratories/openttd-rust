/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared inline cargo identities from `cargo_type.h`.
#![allow(unsafe_code)]
/// Number of defined cargo slots.
pub const NUM_CARGO: u8 = 64;
/// `INVALID_CARGO` sentinel.
pub const INVALID_CARGO: u8 = 255;
/// `CARGO_AUTO_REFIT`: automatically choose the cargo when auto-refitting.
pub const CARGO_AUTO_REFIT: u8 = 0xFD;
/// Original `IsValidCargoType`; validity differs from cargo-spec validity.
#[must_use]
pub const fn is_valid_cargo_type(cargo: u8) -> bool {
    cargo != INVALID_CARGO
}

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
pub const CT_PASSENGERS: u32 = u32::from_be_bytes(*b"PASS");
pub const CT_MAIL: u32 = u32::from_be_bytes(*b"MAIL");
/// `CT_COAL`.
pub const CT_COAL: u32 = u32::from_be_bytes(*b"COAL");
/// `CT_OIL`.
pub const CT_OIL: u32 = u32::from_be_bytes(*b"OIL_");
/// `CT_LIVESTOCK`.
pub const CT_LIVESTOCK: u32 = u32::from_be_bytes(*b"LVST");
/// `CT_GOODS`.
pub const CT_GOODS: u32 = u32::from_be_bytes(*b"GOOD");
/// `CT_GRAIN`.
pub const CT_GRAIN: u32 = u32::from_be_bytes(*b"GRAI");
/// `CT_VALUABLES`.
pub const CT_VALUABLES: u32 = u32::from_be_bytes(*b"VALU");
/// `CT_WHEAT`.
pub const CT_WHEAT: u32 = u32::from_be_bytes(*b"WHEA");
/// `CT_GOLD`.
pub const CT_GOLD: u32 = u32::from_be_bytes(*b"GOLD");
/// `CT_FOOD`.
pub const CT_FOOD: u32 = u32::from_be_bytes(*b"FOOD");
/// `CT_FRUIT`.
pub const CT_FRUIT: u32 = u32::from_be_bytes(*b"FRUT");
/// `CT_MAIZE`.
pub const CT_MAIZE: u32 = u32::from_be_bytes(*b"MAIZ");
/// `CT_COPPER_ORE`.
pub const CT_COPPER_ORE: u32 = u32::from_be_bytes(*b"CORE");
/// `CT_DIAMONDS`.
pub const CT_DIAMONDS: u32 = u32::from_be_bytes(*b"DIAM");
/// `CT_SUGAR`.
pub const CT_SUGAR: u32 = u32::from_be_bytes(*b"SUGR");
/// `CT_CANDY`.
pub const CT_CANDY: u32 = u32::from_be_bytes(*b"SWET");
/// `CT_COLA`.
pub const CT_COLA: u32 = u32::from_be_bytes(*b"COLA");
/// Invalid cargo label.
pub const CT_INVALID: u32 = u32::MAX;

/// Mixed livestock/fruit definition.
pub const MCT_LIVESTOCK_FRUIT: u32 = 0;
/// Mixed grain/wheat/maize definition.
pub const MCT_GRAIN_WHEAT_MAIZE: u32 = 1;
/// Mixed valuables/gold/diamonds definition.
pub const MCT_VALUABLES_GOLD_DIAMONDS: u32 = 2;

/// Invalid cargo label.
pub const CT_INVALID: u32 = u32::MAX;

/// Native food label.
pub const CT_FOOD: u32 = u32::from_be_bytes(*b"FOOD");

/// Original cargo slot count.
pub const NUM_ORIGINAL_CARGO: u8 = 12;

/// All cargo bits, including undefined cargo slots.
pub const ALL_CARGOTYPES: u64 = u64::MAX;
