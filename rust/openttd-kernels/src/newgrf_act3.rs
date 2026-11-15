/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared Action3 `TranslateCargo`, retaining the station-specific special cargos.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation
)]
pub(crate) mod loading;
use crate::cargo_registry::{get_cargo_translation_table, openttd_rust_cargo_by_label};
use crate::cargo_type::{INVALID_CARGO, is_valid_cargo_type};
use crate::debug::{Char, grf_msg};
use crate::newgrf_commons::{SG_DEFAULT_NA, SG_PURCHASE};
use std::ffi::c_void;
pub(crate) fn translate_cargo(feature: u8, cargo: u8, file: *const c_void) -> u8 {
    if (feature == crate::newgrf::GSF_STATIONS || feature == crate::newgrf::GSF_ROADSTOPS)
        && cargo == 0xfe
    {
        return SG_DEFAULT_NA;
    }
    if cargo == 0xff {
        return SG_PURCHASE;
    }
    let table = get_cargo_translation_table(file);
    if usize::from(cargo) >= table.size {
        grf_msg!(
            1,
            "TranslateCargo: Cargo type {} out of range (max {}), skipping.",
            cargo,
            (crate::cargo_registry::file_cargo_count(file) as u32).wrapping_sub(1)
        );
        return INVALID_CARGO;
    }
    let label = unsafe { table.data.add(usize::from(cargo)).read() };
    if label == crate::cargo_type::CT_INVALID {
        grf_msg!(
            5,
            "TranslateCargo: Cargo type {} not available in this climate, skipping.",
            cargo
        );
        return INVALID_CARGO;
    }
    let result = openttd_rust_cargo_by_label(label);
    if !is_valid_cargo_type(result) {
        grf_msg!(
            5,
            "TranslateCargo: Cargo '{}{}{}{}' unsupported, skipping.",
            Char(label.to_be_bytes()[0]),
            Char(label.to_be_bytes()[1]),
            Char(label.to_be_bytes()[2]),
            Char(label.to_be_bytes()[3])
        );
        return INVALID_CARGO;
    }
    grf_msg!(
        6,
        "TranslateCargo: Cargo '{}{}{}{}' mapped to cargo type {}.",
        Char(label.to_be_bytes()[0]),
        Char(label.to_be_bytes()[1]),
        Char(label.to_be_bytes()[2]),
        Char(label.to_be_bytes()[3]),
        result
    );
    result
}
