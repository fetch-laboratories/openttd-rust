/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Common global-variable policy for Action2 and Action7/9/D. Leaves keep the source
//! file and config preconditions, including non-null GRF and config where variables need them.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::decimal_bitwise_operands
)]
use crate::bitmath_func::{gb, sb};
use crate::rail_type::{RAILTYPE_ELECTRIC, RAILTYPE_MAGLEV, RAILTYPE_MONO, RAILTYPE_RAIL};
use crate::services::Services as Shared;
use crate::timer_game_common::{
    DAYS_TILL_ORIGINAL_BASE_YEAR, ORIGINAL_BASE_YEAR, ORIGINAL_MAX_YEAR, is_leap_year,
};
use std::ffi::c_void;
/// `GrfMiscBit::TrainWidth32Pixels`.
pub const TRAIN_WIDTH_32_PIXELS: u8 = 3;
#[repr(C)]
pub struct Services {
    pub(crate) climate: unsafe extern "C" fn() -> u8,
    disable_elrails: unsafe extern "C" fn() -> bool,
    pub(crate) file_version: unsafe extern "C" fn(*const c_void) -> u8,
    height_limit: unsafe extern "C" fn() -> u8,
    pub(crate) misc_bits: unsafe extern "C" fn() -> u8,
    newgrf_version: unsafe extern "C" fn() -> u32,
    palette: unsafe extern "C" fn(*const c_void) -> u8,
    pitch: unsafe extern "C" fn(*const c_void) -> i32,
    rail_cost: unsafe extern "C" fn(u8) -> u16,
    road_side: unsafe extern "C" fn() -> u8,
    shared: *const Shared,
    width: unsafe extern "C" fn(*const c_void) -> u32,
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_global_variable(
    s: &Services,
    variable: u8,
    value: *mut u32,
    file: *const c_void,
) -> bool {
    unsafe {
        let shared = &*s.shared;
        let result = match variable {
            0x00 => (shared.calendar_date)()
                .wrapping_sub(DAYS_TILL_ORIGINAL_BASE_YEAR)
                .max(0) as u32,
            0x01 => {
                ((shared.calendar_year)().clamp(ORIGINAL_BASE_YEAR, ORIGINAL_MAX_YEAR)
                    - ORIGINAL_BASE_YEAR) as u32
            }
            0x02 => {
                let ymd = crate::timer_game_common::calendar_date_to_ymd((shared.calendar_date)());
                let first = crate::timer_game_common::calendar_ymd_to_date(ymd.year, 0, 1);
                u32::from(ymd.month)
                    | (u32::from(ymd.day) - 1) << 8
                    | u32::from(is_leap_year(ymd.year)) << 15
                    | ((shared.calendar_date)().wrapping_sub(first) as u32) << 16
            }
            0x03 => u32::from((s.climate)()),
            0x06 => u32::from((s.road_side)()) << 4,
            0x09 => u32::from(crate::timer_game_common::calendar_fraction()) * 885,
            0x0A => gb((shared.tick_counter)() as u32, 0, 16),
            0x0B => (2 << 24) | (6 << 20) | (1 << 16) | 1382,
            0x0D => u32::from((s.palette)(file)),
            0x0E => (s.pitch)(file) as u32,
            0x0F => {
                let mut costs = 0;
                sb(&mut costs, 0, 8, u32::from((s.rail_cost)(RAILTYPE_RAIL)));
                let middle = if (s.disable_elrails)() {
                    RAILTYPE_MONO
                } else {
                    RAILTYPE_ELECTRIC
                };
                sb(&mut costs, 8, 8, u32::from((s.rail_cost)(middle)));
                sb(&mut costs, 16, 8, u32::from((s.rail_cost)(RAILTYPE_MAGLEV)));
                costs
            }
            0x11 => 0,
            0x12 => u32::from(crate::session::mode().0),
            0x1A => u32::MAX,
            0x1B => 0x3F,
            0x1D => 1,
            0x1E => {
                let mut bits = (s.misc_bits)();
                if (s.width)(file) == crate::newgrf_engine::VEHICLEINFO_FULL_VEHICLE_WIDTH {
                    bits |= 1 << TRAIN_WIDTH_32_PIXELS;
                }
                u32::from(bits)
            }
            0x20 => {
                let snow = (shared.snow_line)();
                if (s.climate)() == crate::landscape_type::ARCTIC && snow <= (s.height_limit)() {
                    (u32::from(snow)
                        * if (s.file_version)(file) >= 8 {
                            1
                        } else {
                            crate::tile_type::TILE_HEIGHT
                        })
                    .clamp(0, 0xFE)
                } else {
                    0xFF
                }
            }
            0x21 => (s.newgrf_version)(),
            0x22 => crate::settings_type::SP_CUSTOM,
            0x23 => (shared.calendar_date)() as u32,
            0x24 => (shared.calendar_year)() as u32,
            _ => return false,
        };
        value.write(result);
        true
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDGlobalVariableServices" ["storage"] => Services { climate, disable_elrails, file_version, height_limit, misc_bits, newgrf_version, palette, pitch, rail_cost, road_side, shared, width };
}

// Feature identities from newgrf.h, pinned by native static assertions.
pub const GSF_SOUNDFX: u8 = 12;
pub const GSF_CARGOES: u8 = 11;
pub const GSF_END: u8 = 22;

pub const GSF_INDUSTRYTILES: u8 = 9;
pub const GSF_INDUSTRIES: u8 = 10;
/// `GSF_RAILTYPES`.
pub const GSF_RAILTYPES: u8 = 0x10;
/// `GSF_ROADTYPES`.
pub const GSF_ROADTYPES: u8 = 0x12;
/// Unreserved engine mapping scope.
pub const INVALID_GRFID: u32 = 0xFFFF_FFFF;

/// Native cargo-label or mixed-cargo alternative from `GetActiveCargoLabel`.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct CargoLabel {
    pub(crate) value: u32,
    pub(crate) mixed: bool,
}

/// First existing label, in the order supplied by the definition.
pub(crate) fn get_active_cargo_label_from_list(
    labels: &[u32],
    mut cargo_by_label: impl FnMut(u32) -> u8,
) -> u32 {
    for &label in labels {
        if cargo_by_label(label) != crate::cargo_type::INVALID_CARGO {
            return label;
        }
    }
    crate::cargo_type::CT_INVALID
}

/// Resolve a mixed label; a literal label passes through without a lookup.
pub(crate) fn get_active_cargo_label(
    label: CargoLabel,
    cargo_by_label: impl FnMut(u32) -> u8,
) -> u32 {
    use crate::cargo_type::{
        CT_DIAMONDS, CT_FRUIT, CT_GOLD, CT_GRAIN, CT_LIVESTOCK, CT_MAIZE, CT_VALUABLES, CT_WHEAT,
        MCT_GRAIN_WHEAT_MAIZE, MCT_LIVESTOCK_FRUIT, MCT_VALUABLES_GOLD_DIAMONDS,
    };
    if !label.mixed {
        return label.value;
    }
    let labels: &[u32] = match label.value {
        MCT_LIVESTOCK_FRUIT => &[CT_LIVESTOCK, CT_FRUIT],
        MCT_GRAIN_WHEAT_MAIZE => &[CT_GRAIN, CT_WHEAT, CT_MAIZE],
        MCT_VALUABLES_GOLD_DIAMONDS => &[CT_VALUABLES, CT_GOLD, CT_DIAMONDS],
        _ => unreachable!(),
    };
    get_active_cargo_label_from_list(labels, cargo_by_label)
}
// Loading-stage identities from newgrf.h.
pub const GLS_FILESCAN: u8 = 0;
pub const GLS_SAFETYSCAN: u8 = 1;
pub const GLS_LABELSCAN: u8 = 2;
pub const GLS_INIT: u8 = 3;
pub const GLS_RESERVE: u8 = 4;
pub const GLS_ACTIVATION: u8 = 5;

pub const GSF_GLOBALVAR: u8 = 8;
pub const SECOND_ROCKY_TILE_SET: u8 = 6;

/// `GSF_TRAINS`.
pub const GSF_TRAINS: u8 = 0;

/// `GSF_ROADVEHICLES`.
pub const GSF_ROADVEHICLES: u8 = 1;

/// `GSF_SHIPS`.
pub const GSF_SHIPS: u8 = 2;

/// `GSF_AIRCRAFT`.
pub const GSF_AIRCRAFT: u8 = 3;

/// `GSF_STATIONS`.
pub const GSF_STATIONS: u8 = 4;

/// `GSF_CANALS`.
pub const GSF_CANALS: u8 = 5;

/// `GSF_HOUSES`.
pub const GSF_HOUSES: u8 = 7;

/// `GSF_AIRPORTS`.
pub const GSF_AIRPORTS: u8 = 13;

/// `GSF_OBJECTS`.
pub const GSF_OBJECTS: u8 = 15;

/// `GSF_AIRPORTTILES`.
pub const GSF_AIRPORTTILES: u8 = 17;

/// `GSF_TRAMTYPES`.
pub const GSF_TRAMTYPES: u8 = 19;

/// `GSF_ROADSTOPS`.
pub const GSF_ROADSTOPS: u8 = 20;

/// `GSF_BADGES`.
pub const GSF_BADGES: u8 = 21;
