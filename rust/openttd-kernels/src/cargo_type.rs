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
