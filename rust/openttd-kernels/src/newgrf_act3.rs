/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared Action3 `TranslateCargo`, retaining the station-specific special cargos.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation
)]
use crate::cargo_registry::{openttd_rust_cargo_by_label, openttd_rust_cargo_translation_table};
use crate::cargo_type::{INVALID_CARGO, is_valid_cargo_type};
use crate::newgrf_commons::{SG_DEFAULT_NA, SG_PURCHASE};
use std::ffi::c_void;
#[repr(C)]
pub struct Services {
    out_of_range: unsafe extern "C" fn(u8, u32),
    not_available: unsafe extern "C" fn(u8),
    unsupported: unsafe extern "C" fn(u32),
    mapped: unsafe extern "C" fn(u32, u8),
}
pub(crate) fn translate_cargo(s: &Services, feature: u8, cargo: u8, file: *const c_void) -> u8 {
    if (feature == crate::newgrf::GSF_STATIONS || feature == crate::newgrf::GSF_ROADSTOPS)
        && cargo == 0xfe
    {
        return SG_DEFAULT_NA;
    }
    if cargo == 0xff {
        return SG_PURCHASE;
    }
    let table = openttd_rust_cargo_translation_table(file);
    if usize::from(cargo) >= table.size {
        unsafe {
            (s.out_of_range)(
                cargo,
                (crate::cargo_registry::file_cargo_count(file) as u32).wrapping_sub(1),
            );
        }
        return INVALID_CARGO;
    }
    let label = unsafe { table.data.add(usize::from(cargo)).read() };
    if label == 0 {
        unsafe {
            (s.not_available)(cargo);
        }
        return INVALID_CARGO;
    }
    let result = openttd_rust_cargo_by_label(label);
    if !is_valid_cargo_type(result) {
        unsafe {
            (s.unsupported)(label);
        }
        return INVALID_CARGO;
    }
    unsafe {
        (s.mapped)(label, result);
    }
    result
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_act3_translate_cargo(
    s: *const Services,
    feature: u8,
    cargo: u8,
    file: *const c_void,
) -> u8 {
    translate_cargo(unsafe { &*s }, feature, cargo, file)
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDAct3CargoServices" ["station_service"] => Services { out_of_range, not_available, unsupported, mapped };
}
