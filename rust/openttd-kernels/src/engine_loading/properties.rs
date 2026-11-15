/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Complete vehicle Action0 property interpretation. Canonical Rust records are read and mutated directly.
//! Entries run under serialized loading; no TEMP, native storage or queue borrow spans a callback.
//! Checked reads return to the original native exception frame before it throws.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::too_many_lines
)]
use super::{
    CT_INVALID, E, File, INVALID_CARGO, RailVehicleTypes, Services, VEH_AIRCRAFT, VEH_ROAD,
    VEH_SHIP, VEH_TRAIN, is_valid_cargo_type, openttd_rust_cargo_translate,
    openttd_rust_engine_definition, openttd_rust_engine_temp_refittability,
    openttd_rust_vehicle_order, temp, translate_refit_mask,
};
use crate::bitmath_func::{assign_bit, has_bit, sb, set_bit64};
use crate::debug::grf_msg;
use crate::engine_type::{
    AIR_CTOL, AIR_HELI, EC_DIESEL, EC_ELECTRIC, EC_MAGLEV, EC_MONORAIL, EC_STEAM, VE_DEFAULT,
    VE_DISABLE_EFFECT, VE_TYPE_COUNT, VE_TYPE_START,
};
use crate::newgrf_bytereader::{ByteReader, ReadError, Reader, Services as ReaderServices};
use crate::newgrf_internal::{CIR_INVALID_ID, CIR_SUCCESS, CIR_UNKNOWN};
use crate::newgrf_properties::{
    PROP_AIRCRAFT_CARGO_AGE_PERIOD, PROP_AIRCRAFT_COST_FACTOR, PROP_AIRCRAFT_MAIL_CAPACITY,
    PROP_AIRCRAFT_PASSENGER_CAPACITY, PROP_AIRCRAFT_RANGE, PROP_AIRCRAFT_RUNNING_COST_FACTOR,
    PROP_AIRCRAFT_SPEED, PROP_ROADVEH_CARGO_AGE_PERIOD, PROP_ROADVEH_CARGO_CAPACITY,
    PROP_ROADVEH_COST_FACTOR, PROP_ROADVEH_POWER, PROP_ROADVEH_RUNNING_COST_FACTOR,
    PROP_ROADVEH_SHORTEN_FACTOR, PROP_ROADVEH_SPEED, PROP_ROADVEH_TRACTIVE_EFFORT,
    PROP_ROADVEH_WEIGHT, PROP_SHIP_CARGO_AGE_PERIOD, PROP_SHIP_CARGO_CAPACITY,
    PROP_SHIP_COST_FACTOR, PROP_SHIP_RUNNING_COST_FACTOR, PROP_SHIP_SPEED,
    PROP_TRAIN_CARGO_AGE_PERIOD, PROP_TRAIN_CARGO_CAPACITY, PROP_TRAIN_COST_FACTOR,
    PROP_TRAIN_CURVE_SPEED_MOD, PROP_TRAIN_POWER, PROP_TRAIN_RUNNING_COST_FACTOR,
    PROP_TRAIN_SHORTEN_FACTOR, PROP_TRAIN_SPEED, PROP_TRAIN_TRACTIVE_EFFORT, PROP_TRAIN_USER_DATA,
    PROP_TRAIN_WEIGHT, PROP_VEHICLE_LOAD_AMOUNT,
};
use crate::raw_field::{read_field, write_field};
use crate::vehicle::OpenTTDEngine;
use crate::vehicle_func::CUSTOM_VEHICLE_SPRITENUM;
#[repr(C)]
pub struct Properties {
    loading: *const Services,
    id: extern "C" fn(E) -> u16,
    sound: extern "C" fn(File, u16) -> u16,
    train_image_valid: extern "C" fn(u8) -> bool,
    road_image_valid: extern "C" fn(u8) -> bool,
    ship_image_valid: extern "C" fn(u8) -> bool,
    aircraft_image_valid: extern "C" fn(u8) -> bool,
    uses_2cc: extern "C" fn(bool),
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDVehicleProperties" ["engine_loading"] => Properties { loading, id, sound, train_image_valid, road_image_valid, ship_image_valid, aircraft_image_valid, uses_2cc };
}

fn common(_s: &Properties, e: E, prop: i32, r: &mut Reader<'_>) -> Result<i32, ReadError> {
    match prop {
        0x00 => write_field!(
            e.cast_mut(),
            info.base_intro,
            crate::timer_game_common::DAYS_TILL_ORIGINAL_BASE_YEAR + i32::from(r.word()?)
        ),
        0x02 => write_field!(e.cast_mut(), info.decay_speed, r.byte()?),
        0x03 => write_field!(e.cast_mut(), info.lifelength, i32::from(r.byte()?)),
        0x04 => write_field!(e.cast_mut(), info.base_life, i32::from(r.byte()?)),
        0x06 => write_field!(e.cast_mut(), info.climates, r.byte()?),
        p if p == i32::from(PROP_VEHICLE_LOAD_AMOUNT) => {
            write_field!(e.cast_mut(), info.load_amount, r.byte()?);
        }
        _ => return Ok(CIR_UNKNOWN),
    }
    Ok(CIR_SUCCESS)
}
fn base_price(pointer: u32) -> Option<u8> {
    const START: u32 = 0x4b34;
    const SIZE: u32 = 6;
    if pointer == 0 {
        return Some(crate::economy_type::INVALID_PRICE);
    }
    if pointer < START
        || !(pointer - START).is_multiple_of(SIZE)
        || (pointer - START) / SIZE >= u32::from(crate::economy_type::PR_END)
    {
        None
    } else {
        Some(((pointer - START) / SIZE) as u8)
    }
}
fn price(_s: &Properties, e: E, kind: u8, value: u32) {
    if let Some(index) = base_price(value) {
        if kind == VEH_TRAIN {
            write_field!(OpenTTDEngine::rail(e), running_cost_class, index);
        } else {
            write_field!(OpenTTDEngine::road(e), running_cost_class, index);
        }
    } else {
        let name = if kind == VEH_TRAIN {
            "RailVehicleChangeInfo"
        } else {
            "RoadVehicleChangeInfo"
        };
        grf_msg!(
            1,
            "{}: Unsupported running cost base 0x{:04X}, ignoring",
            name,
            value
        );
    }
}
/// Change-info handler names by vehicle type, as the messages name them.
const PROPERTY_LOCATIONS: [&str; 4] = [
    "RailVehicleChangeInfo",
    "RoadVehicleChangeInfo",
    "ShipVehicleChangeInfo",
    "AircraftVehicleChangeInfo",
];
fn image(s: &Properties, kind: u8, value: u8) -> u8 {
    let mut index = value;
    if kind != VEH_TRAIN && index == 255 {
        index = CUSTOM_VEHICLE_SPRITENUM;
    }
    if index < CUSTOM_VEHICLE_SPRITENUM {
        index >>= 1;
    }
    let valid = match kind {
        VEH_TRAIN => (s.train_image_valid)(index),
        VEH_ROAD => (s.road_image_valid)(index),
        VEH_SHIP => (s.ship_image_valid)(index),
        _ => (s.aircraft_image_valid)(index),
    };
    if valid {
        index
    } else {
        grf_msg!(
            1,
            "{}: Invalid Sprite {} specified, ignoring",
            PROPERTY_LOCATIONS[usize::from(kind)],
            value
        );
        0
    }
}
fn visual(s: &Properties, value: u8) -> u8 {
    let mut effect = u32::from(value);
    if value == VE_DEFAULT {
        crate::movement::native_assert(
            unsafe { &*(*s.loading).shared },
            has_bit(effect, VE_DISABLE_EFFECT),
        );
        sb(&mut effect, VE_TYPE_START, VE_TYPE_COUNT, 0);
    }
    effect as u8
}
fn callbacks(_s: &Properties, e: E, r: &mut Reader<'_>, high: bool) -> Result<(), ReadError> {
    let mut mask = u32::from(read_field!(e, info.callback_mask));
    sb(&mut mask, if high { 8 } else { 0 }, 8, u32::from(r.byte()?));
    write_field!(e.cast_mut(), info.callback_mask, mask as u16);
    Ok(())
}
fn misc(s: &Properties, e: E, value: u8) {
    write_field!(e.cast_mut(), info.misc_flags, value);
    (s.uses_2cc)(value & crate::engine_type::USES_2CC != 0);
}
unsafe fn default_cargo(
    _s: &Properties,
    e: E,
    id: u16,
    file: File,
    kind: u8,
    r: &mut Reader<'_>,
) -> Result<(), ReadError> {
    unsafe {
        (*temp(id)).fields.defaultcargo_grf = file;
    }
    let value = r.byte()?;
    let cargo = if value == 255 {
        INVALID_CARGO
    } else {
        let cargo = openttd_rust_cargo_translate(value, file, false);
        if cargo == INVALID_CARGO {
            grf_msg!(
                2,
                "{}: Invalid cargo type {}, using first refittable",
                PROPERTY_LOCATIONS[usize::from(kind)],
                value
            );
        }
        cargo
    };
    write_field!(e.cast_mut(), info.cargo_type, cargo);
    write_field!(
        e.cast_mut(),
        info.cargo_label,
        crate::newgrf::CargoLabel {
            value: CT_INVALID,
            mixed: false
        }
    );
    Ok(())
}
unsafe fn refit(_s: &Properties, e: E, id: u16, file: File, value: u32) {
    unsafe {
        openttd_rust_engine_temp_refittability(&raw mut (*temp(id)).fields, value != 0);
    }
    write_field!(
        e.cast_mut(),
        info.refit_mask,
        translate_refit_mask(value, file).0
    );
    unsafe {
        (*temp(id)).fields.defaultcargo_grf = file;
    }
}
unsafe fn allowed(id: u16, file: File, value: u16) {
    unsafe {
        (*temp(id)).fields.cargo_allowed = value;
        openttd_rust_engine_temp_refittability(&raw mut (*temp(id)).fields, value != 0);
        (*temp(id)).fields.defaultcargo_grf = file;
    }
}
unsafe fn disallowed(id: u16, value: u16) {
    unsafe {
        (*temp(id)).fields.cargo_disallowed = value;
        openttd_rust_engine_temp_refittability(&raw mut (*temp(id)).fields, false);
    }
}
unsafe fn cargo_list(
    id: u16,
    file: File,
    include: bool,
    r: &mut Reader<'_>,
) -> Result<(), ReadError> {
    let count = r.byte()?;
    unsafe {
        openttd_rust_engine_temp_refittability(&raw mut (*temp(id)).fields, include && count != 0);
        if include {
            (*temp(id)).fields.defaultcargo_grf = file;
        }
        if include {
            (*temp(id)).fields.ctt_include_mask = 0;
        } else {
            (*temp(id)).fields.ctt_exclude_mask = 0;
        }
    }
    for _ in 0..count {
        let cargo = openttd_rust_cargo_translate(r.byte()?, file, false);
        if is_valid_cargo_type(cargo) {
            unsafe {
                if include {
                    set_bit64(&mut (*temp(id)).fields.ctt_include_mask, cargo);
                } else {
                    set_bit64(&mut (*temp(id)).fields.ctt_exclude_mask, cargo);
                }
            }
        }
    }
    Ok(())
}
unsafe fn train(
    s: &Properties,
    e: E,
    id: u16,
    file: File,
    prop: i32,
    r: &mut Reader<'_>,
) -> Result<i32, ReadError> {
    match prop {
        0x05 => {
            let track = r.byte()?;
            unsafe {
                (*temp(id)).labels.clear();
            }
            let list = crate::type_registry::rail_labels(file);
            if usize::from(track) < list.len() {
                let label = unsafe { list.at(usize::from(track)) };
                unsafe {
                    (*temp(id)).labels.push(label);
                }
            } else {
                use crate::rail_type::{
                    RAILTYPE_LABEL_ELECTRIC, RAILTYPE_LABEL_MAGLEV, RAILTYPE_LABEL_MONO,
                    RAILTYPE_LABEL_RAIL,
                };
                let label = match track {
                    0 => Some(
                        if read_field!(OpenTTDEngine::rail(e), engclass) >= EC_ELECTRIC {
                            RAILTYPE_LABEL_ELECTRIC
                        } else {
                            RAILTYPE_LABEL_RAIL
                        },
                    ),
                    1 => Some(RAILTYPE_LABEL_MONO),
                    2 => Some(RAILTYPE_LABEL_MAGLEV),
                    _ => {
                        grf_msg!(
                            1,
                            "RailVehicleChangeInfo: Invalid track type {} specified, ignoring",
                            track
                        );
                        None
                    }
                };
                if let Some(label) = label {
                    unsafe {
                        (*temp(id)).labels.push(label);
                    }
                }
            }
        }
        0x08 => write_field!(OpenTTDEngine::rail(e), ai_passenger_only, r.byte()?),
        p if p == i32::from(PROP_TRAIN_SPEED) => {
            let speed = r.word()?;
            write_field!(
                OpenTTDEngine::rail(e),
                max_speed,
                if speed == 0xffff { 0 } else { speed }
            );
        }
        p if p == i32::from(PROP_TRAIN_POWER) => {
            let power = r.word()?;
            write_field!(OpenTTDEngine::rail(e), power, power);
            if power != 0 {
                if RailVehicleTypes(read_field!(OpenTTDEngine::rail(e), railveh_type))
                    == RailVehicleTypes::WAGON
                {
                    write_field!(
                        OpenTTDEngine::rail(e),
                        railveh_type,
                        (RailVehicleTypes::SINGLEHEAD).0
                    );
                }
            } else {
                write_field!(
                    OpenTTDEngine::rail(e),
                    railveh_type,
                    (RailVehicleTypes::WAGON).0
                );
            }
        }
        p if p == i32::from(PROP_TRAIN_RUNNING_COST_FACTOR) => {
            write_field!(OpenTTDEngine::rail(e), running_cost, r.byte()?);
        }
        0x0e => price(s, e, VEH_TRAIN, r.dword()?),
        0x12 => write_field!(
            OpenTTDEngine::rail(e),
            image_index,
            image(s, VEH_TRAIN, r.byte()?)
        ),
        0x13 => {
            let dual = r.byte()?;
            let kind = if dual != 0 {
                RailVehicleTypes::MULTIHEAD
            } else if read_field!(OpenTTDEngine::rail(e), power) == 0 {
                RailVehicleTypes::WAGON
            } else {
                RailVehicleTypes::SINGLEHEAD
            };
            write_field!(OpenTTDEngine::rail(e), railveh_type, (kind).0);
        }
        p if p == i32::from(PROP_TRAIN_CARGO_CAPACITY) => {
            write_field!(OpenTTDEngine::rail(e), capacity, r.byte()?);
        }
        0x15 => unsafe {
            default_cargo(s, e, id, file, VEH_TRAIN, r)?;
        },
        p if p == i32::from(PROP_TRAIN_WEIGHT) => {
            let low = r.byte()?;
            let mut weight = u32::from(read_field!(OpenTTDEngine::rail(e), weight));
            sb(&mut weight, 0, 8, u32::from(low));
            write_field!(OpenTTDEngine::rail(e), weight, weight as u16);
        }
        p if p == i32::from(PROP_TRAIN_COST_FACTOR) => {
            write_field!(OpenTTDEngine::rail(e), cost_factor, r.byte()?);
        }
        0x18 => {
            grf_msg!(
                2,
                "RailVehicleChangeInfo: Property 0x18 'AI rank' not used by NoAI, ignored."
            );
            r.byte()?;
        }
        0x19 => {
            let traction = r.byte()?;
            let class = match traction {
                0..=7 => EC_STEAM,
                8..=0x27 => EC_DIESEL,
                0x28..=0x31 => EC_ELECTRIC,
                0x32..=0x37 => EC_MONORAIL,
                0x38..=0x41 => EC_MAGLEV,
                _ => return Ok(CIR_SUCCESS),
            };
            if crate::type_registry::rail_labels(file).len() == 0 {
                use crate::rail_type::{RAILTYPE_LABEL_ELECTRIC, RAILTYPE_LABEL_RAIL};
                if let Some(label) = unsafe { &mut (*temp(id)).labels }.first_mut() {
                    if *label == RAILTYPE_LABEL_RAIL && class >= EC_ELECTRIC {
                        *label = RAILTYPE_LABEL_ELECTRIC;
                    }
                    if *label == RAILTYPE_LABEL_ELECTRIC && class < EC_ELECTRIC {
                        *label = RAILTYPE_LABEL_RAIL;
                    }
                }
            }
            write_field!(OpenTTDEngine::rail(e), engclass, class);
        }
        0x1a => unsafe {
            openttd_rust_vehicle_order(id, r.extended()?);
        },
        0x1b => write_field!(OpenTTDEngine::rail(e), pow_wag_power, r.word()?),
        0x1c => write_field!(e.cast_mut(), info.refit_cost, r.byte()?),
        0x1d => unsafe {
            refit(s, e, id, file, r.dword()?);
        },
        0x1e => callbacks(s, e, r, false)?,
        p if p == i32::from(PROP_TRAIN_TRACTIVE_EFFORT) => {
            write_field!(OpenTTDEngine::rail(e), tractive_effort, r.byte()?);
        }
        0x20 => write_field!(OpenTTDEngine::rail(e), air_drag, r.byte()?),
        p if p == i32::from(PROP_TRAIN_SHORTEN_FACTOR) => {
            write_field!(OpenTTDEngine::rail(e), shorten_factor, r.byte()?);
        }
        0x22 => write_field!(OpenTTDEngine::rail(e), visual_effect, visual(s, r.byte()?)),
        0x23 => write_field!(OpenTTDEngine::rail(e), pow_wag_weight, r.byte()?),
        0x24 => {
            let high = r.byte()?;
            if high > 4 {
                grf_msg!(
                    2,
                    "RailVehicleChangeInfo: Nonsensical weight of {} tons, ignoring",
                    u32::from(high) << 8
                );
            } else {
                let mut weight = u32::from(read_field!(OpenTTDEngine::rail(e), weight));
                sb(&mut weight, 8, 8, u32::from(high));
                write_field!(OpenTTDEngine::rail(e), weight, weight as u16);
            }
        }
        p if p == i32::from(PROP_TRAIN_USER_DATA) => {
            write_field!(OpenTTDEngine::rail(e), user_def_data, r.byte()?);
        }
        0x26 => write_field!(e.cast_mut(), info.retire_early, (r.byte()?) as i8),
        0x27 => misc(s, e, r.byte()?),
        0x28 => unsafe {
            allowed(id, file, r.word()?);
        },
        0x29 => unsafe {
            disallowed(id, r.word()?);
        },
        0x2a => write_field!(e.cast_mut(), info.base_intro, r.dword()? as i32),
        p if p == i32::from(PROP_TRAIN_CARGO_AGE_PERIOD) => {
            write_field!(e.cast_mut(), info.cargo_age_period, r.word()?);
        }
        0x2c | 0x2d => unsafe {
            cargo_list(id, file, prop == 0x2c, r)?;
        },
        p if p == i32::from(PROP_TRAIN_CURVE_SPEED_MOD) => {
            write_field!(OpenTTDEngine::rail(e), curve_speed_mod, r.word()? as i16);
        }
        0x2f => write_field!(
            e.cast_mut(),
            info.variant_id,
            crate::engine_type::EngineID(r.word()?)
        ),
        0x30 => write_field!(e.cast_mut(), info.extra_flags, (r.dword()?) as u8),
        0x31 => callbacks(s, e, r, true)?,
        0x32 => unsafe {
            (*temp(id)).fields.cargo_allowed_required = r.word()?;
        },
        0x33 => unsafe {
            let badges = r.badge_ids(VEH_TRAIN)?;
            (*e.cast_mut()).bindings.badges = crate::industrytype::NativeVec::from_vec(badges);
        },
        0x34 => {
            let count = r.byte()?;
            unsafe {
                (*temp(id)).labels.clear();
            }
            for _ in 0..count {
                let track = r.byte()?;
                let list = crate::type_registry::rail_labels(file);
                if usize::from(track) < list.len() {
                    let label = unsafe { list.at(usize::from(track)) };
                    unsafe {
                        (*temp(id)).labels.push(label);
                    }
                } else {
                    grf_msg!(
                        1,
                        "RailVehicleChangeInfo: Invalid track type {} specified, ignoring",
                        track
                    );
                }
            }
        }
        _ => return common(s, e, prop, r),
    }
    Ok(CIR_SUCCESS)
}
unsafe fn road(
    s: &Properties,
    e: E,
    id: u16,
    file: File,
    prop: i32,
    r: &mut Reader<'_>,
) -> Result<i32, ReadError> {
    match prop {
        0x05 => unsafe {
            (*temp(id)).fields.roadtramtype = r.byte()?.wrapping_add(1);
        },
        0x08 => write_field!(OpenTTDEngine::road(e), max_speed, u16::from(r.byte()?)),
        p if p == i32::from(PROP_ROADVEH_RUNNING_COST_FACTOR) => {
            write_field!(OpenTTDEngine::road(e), running_cost, r.byte()?);
        }
        0x0a => price(s, e, VEH_ROAD, r.dword()?),
        0x0e => write_field!(
            OpenTTDEngine::road(e),
            image_index,
            image(s, VEH_ROAD, r.byte()?)
        ),
        p if p == i32::from(PROP_ROADVEH_CARGO_CAPACITY) => {
            write_field!(OpenTTDEngine::road(e), capacity, r.byte()?);
        }
        0x10 => unsafe {
            default_cargo(s, e, id, file, VEH_ROAD, r)?;
        },
        p if p == i32::from(PROP_ROADVEH_COST_FACTOR) => {
            write_field!(OpenTTDEngine::road(e), cost_factor, r.byte()?);
        }
        0x12 => write_field!(
            OpenTTDEngine::road(e),
            sfx,
            (s.sound)(file, u16::from(r.byte()?))
        ),
        p if p == i32::from(PROP_ROADVEH_POWER) => {
            write_field!(OpenTTDEngine::road(e), power, r.byte()?);
        }
        p if p == i32::from(PROP_ROADVEH_WEIGHT) => {
            write_field!(OpenTTDEngine::road(e), weight, r.byte()?);
        }
        p if p == i32::from(PROP_ROADVEH_SPEED) => unsafe {
            (*temp(id)).fields.rv_max_speed = r.byte()?;
        },
        0x16 => unsafe {
            refit(s, e, id, file, r.dword()?);
        },
        0x17 => callbacks(s, e, r, false)?,
        p if p == i32::from(PROP_ROADVEH_TRACTIVE_EFFORT) => {
            write_field!(OpenTTDEngine::road(e), tractive_effort, r.byte()?);
        }
        0x19 => write_field!(OpenTTDEngine::road(e), air_drag, r.byte()?),
        0x1a => write_field!(e.cast_mut(), info.refit_cost, r.byte()?),
        0x1b => write_field!(e.cast_mut(), info.retire_early, (r.byte()?) as i8),
        0x1c => misc(s, e, r.byte()?),
        0x1d => unsafe {
            allowed(id, file, r.word()?);
        },
        0x1e => unsafe {
            disallowed(id, r.word()?);
        },
        0x1f => write_field!(e.cast_mut(), info.base_intro, r.dword()? as i32),
        0x20 => unsafe {
            openttd_rust_vehicle_order(id, r.extended()?);
        },
        0x21 => write_field!(OpenTTDEngine::road(e), visual_effect, visual(s, r.byte()?)),
        p if p == i32::from(PROP_ROADVEH_CARGO_AGE_PERIOD) => {
            write_field!(e.cast_mut(), info.cargo_age_period, r.word()?);
        }
        p if p == i32::from(PROP_ROADVEH_SHORTEN_FACTOR) => {
            write_field!(OpenTTDEngine::road(e), shorten_factor, r.byte()?);
        }
        0x24 | 0x25 => unsafe {
            cargo_list(id, file, prop == 0x24, r)?;
        },
        0x26 => write_field!(
            e.cast_mut(),
            info.variant_id,
            crate::engine_type::EngineID(r.word()?)
        ),
        0x27 => write_field!(e.cast_mut(), info.extra_flags, (r.dword()?) as u8),
        0x28 => callbacks(s, e, r, true)?,
        0x29 => unsafe {
            (*temp(id)).fields.cargo_allowed_required = r.word()?;
        },
        0x2a => unsafe {
            let badges = r.badge_ids(VEH_ROAD)?;
            (*e.cast_mut()).bindings.badges = crate::industrytype::NativeVec::from_vec(badges);
        },
        _ => return common(s, e, prop, r),
    }
    Ok(CIR_SUCCESS)
}
unsafe fn ship(
    s: &Properties,
    e: E,
    id: u16,
    file: File,
    prop: i32,
    r: &mut Reader<'_>,
) -> Result<i32, ReadError> {
    match prop {
        0x08 => write_field!(
            OpenTTDEngine::ship(e),
            image_index,
            image(s, VEH_SHIP, r.byte()?)
        ),
        0x09 => write_field!(OpenTTDEngine::ship(e), old_refittable, r.byte()? != 0),
        p if p == i32::from(PROP_SHIP_COST_FACTOR) => {
            write_field!(OpenTTDEngine::ship(e), cost_factor, r.byte()?);
        }
        p if p == i32::from(PROP_SHIP_SPEED) => {
            write_field!(OpenTTDEngine::ship(e), max_speed, u16::from(r.byte()?));
        }
        0x0c => unsafe {
            default_cargo(s, e, id, file, VEH_SHIP, r)?;
        },
        p if p == i32::from(PROP_SHIP_CARGO_CAPACITY) => {
            write_field!(OpenTTDEngine::ship(e), capacity, r.word()?);
        }
        p if p == i32::from(PROP_SHIP_RUNNING_COST_FACTOR) => {
            write_field!(OpenTTDEngine::ship(e), running_cost, r.byte()?);
        }
        0x10 => write_field!(
            OpenTTDEngine::ship(e),
            sfx,
            (s.sound)(file, u16::from(r.byte()?))
        ),
        0x11 => unsafe {
            refit(s, e, id, file, r.dword()?);
        },
        0x12 => callbacks(s, e, r, false)?,
        0x13 => write_field!(e.cast_mut(), info.refit_cost, r.byte()?),
        0x14 => write_field!(OpenTTDEngine::ship(e), ocean_speed_frac, r.byte()?),
        0x15 => write_field!(OpenTTDEngine::ship(e), canal_speed_frac, r.byte()?),
        0x16 => write_field!(e.cast_mut(), info.retire_early, (r.byte()?) as i8),
        0x17 => misc(s, e, r.byte()?),
        0x18 => unsafe {
            allowed(id, file, r.word()?);
        },
        0x19 => unsafe {
            disallowed(id, r.word()?);
        },
        0x1a => write_field!(e.cast_mut(), info.base_intro, r.dword()? as i32),
        0x1b => unsafe {
            openttd_rust_vehicle_order(id, r.extended()?);
        },
        0x1c => write_field!(OpenTTDEngine::ship(e), visual_effect, visual(s, r.byte()?)),
        p if p == i32::from(PROP_SHIP_CARGO_AGE_PERIOD) => {
            write_field!(e.cast_mut(), info.cargo_age_period, r.word()?);
        }
        0x1e | 0x1f => unsafe {
            cargo_list(id, file, prop == 0x1e, r)?;
        },
        0x20 => write_field!(
            e.cast_mut(),
            info.variant_id,
            crate::engine_type::EngineID(r.word()?)
        ),
        0x21 => write_field!(e.cast_mut(), info.extra_flags, (r.dword()?) as u8),
        0x22 => callbacks(s, e, r, true)?,
        0x23 => write_field!(OpenTTDEngine::ship(e), max_speed, r.word()?),
        0x24 => write_field!(OpenTTDEngine::ship(e), acceleration, r.byte()?.max(1)),
        0x25 => unsafe {
            (*temp(id)).fields.cargo_allowed_required = r.word()?;
        },
        0x26 => unsafe {
            let badges = r.badge_ids(VEH_SHIP)?;
            (*e.cast_mut()).bindings.badges = crate::industrytype::NativeVec::from_vec(badges);
        },
        _ => return common(s, e, prop, r),
    }
    Ok(CIR_SUCCESS)
}
unsafe fn aircraft(
    s: &Properties,
    e: E,
    id: u16,
    file: File,
    prop: i32,
    r: &mut Reader<'_>,
) -> Result<i32, ReadError> {
    match prop {
        0x08 => write_field!(
            OpenTTDEngine::aircraft(e),
            image_index,
            image(s, VEH_AIRCRAFT, r.byte()?)
        ),
        0x09 => {
            let value = r.byte()?;
            let subtype = if value == 0 {
                AIR_HELI
            } else {
                let mut subtype = u32::from(read_field!(OpenTTDEngine::aircraft(e), subtype));
                sb(&mut subtype, 0, 1, u32::from(AIR_CTOL));
                subtype as u8
            };
            write_field!(OpenTTDEngine::aircraft(e), subtype, subtype);
        }
        0x0a => {
            let large = r.byte()? != 0;
            let mut subtype = u32::from(read_field!(OpenTTDEngine::aircraft(e), subtype));
            assign_bit(&mut subtype, 1, large);
            write_field!(OpenTTDEngine::aircraft(e), subtype, subtype as u8);
        }
        p if p == i32::from(PROP_AIRCRAFT_COST_FACTOR) => {
            write_field!(OpenTTDEngine::aircraft(e), cost_factor, r.byte()?);
        }
        p if p == i32::from(PROP_AIRCRAFT_SPEED) => {
            write_field!(
                OpenTTDEngine::aircraft(e),
                max_speed,
                u16::from(r.byte()?) * 128 / 10
            );
        }
        0x0d => write_field!(OpenTTDEngine::aircraft(e), acceleration, r.byte()?),
        p if p == i32::from(PROP_AIRCRAFT_RUNNING_COST_FACTOR) => {
            write_field!(OpenTTDEngine::aircraft(e), running_cost, r.byte()?);
        }
        p if p == i32::from(PROP_AIRCRAFT_PASSENGER_CAPACITY) => {
            write_field!(OpenTTDEngine::aircraft(e), passenger_capacity, r.word()?);
        }
        p if p == i32::from(PROP_AIRCRAFT_MAIL_CAPACITY) => {
            write_field!(OpenTTDEngine::aircraft(e), mail_capacity, r.byte()?);
        }
        0x12 => write_field!(
            OpenTTDEngine::aircraft(e),
            sfx,
            (s.sound)(file, u16::from(r.byte()?))
        ),
        0x13 => unsafe {
            refit(s, e, id, file, r.dword()?);
        },
        0x14 => callbacks(s, e, r, false)?,
        0x15 => write_field!(e.cast_mut(), info.refit_cost, r.byte()?),
        0x16 => write_field!(e.cast_mut(), info.retire_early, (r.byte()?) as i8),
        0x17 => misc(s, e, r.byte()?),
        0x18 => unsafe {
            allowed(id, file, r.word()?);
        },
        0x19 => unsafe {
            disallowed(id, r.word()?);
        },
        0x1a => write_field!(e.cast_mut(), info.base_intro, r.dword()? as i32),
        0x1b => unsafe {
            openttd_rust_vehicle_order(id, r.extended()?);
        },
        p if p == i32::from(PROP_AIRCRAFT_CARGO_AGE_PERIOD) => {
            write_field!(e.cast_mut(), info.cargo_age_period, r.word()?);
        }
        0x1d | 0x1e => unsafe {
            cargo_list(id, file, prop == 0x1d, r)?;
        },
        p if p == i32::from(PROP_AIRCRAFT_RANGE) => {
            write_field!(OpenTTDEngine::aircraft(e), max_range, r.word()?);
        }
        0x20 => write_field!(
            e.cast_mut(),
            info.variant_id,
            crate::engine_type::EngineID(r.word()?)
        ),
        0x21 => write_field!(e.cast_mut(), info.extra_flags, (r.dword()?) as u8),
        0x22 => callbacks(s, e, r, true)?,
        0x23 => unsafe {
            (*temp(id)).fields.cargo_allowed_required = r.word()?;
        },
        0x24 => unsafe {
            let badges = r.badge_ids(VEH_AIRCRAFT)?;
            (*e.cast_mut()).bindings.badges = crate::industrytype::NativeVec::from_vec(badges);
        },
        _ => return common(s, e, prop, r),
    }
    Ok(CIR_SUCCESS)
}
/// # Safety
/// The services and native reader remain valid for one serialized property range.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_properties(
    s: &Properties,
    reads: &ReaderServices,
    file: File,
    kind: u8,
    first: u32,
    last: u32,
    prop: i32,
    native: *mut ByteReader,
) -> i32 {
    let mut r = unsafe { Reader::new(native, reads) };
    let mut result = CIR_SUCCESS;
    for local in first..last {
        let e =
            unsafe { openttd_rust_engine_definition(&*s.loading, file, kind, local as u16, false) };
        if e.is_null() {
            return CIR_INVALID_ID;
        }
        let id = (s.id)(e);
        let changed = unsafe {
            match kind {
                VEH_TRAIN => train(s, e, id, file, prop, &mut r),
                VEH_ROAD => road(s, e, id, file, prop, &mut r),
                VEH_SHIP => ship(s, e, id, file, prop, &mut r),
                _ => aircraft(s, e, id, file, prop, &mut r),
            }
        };
        match changed {
            Ok(value) => result = value,
            Err(ReadError) => return -1,
        }
    }
    result
}

#[cfg(test)]
pub(super) unsafe fn partial_read_tests() {
    extern "C" fn unused1<A, R>(_: A) -> R {
        panic!("unused property service")
    }
    extern "C" fn unused2<A, B, R>(_: A, _: B) -> R {
        panic!("unused property service")
    }
    extern "C" fn unused5<A, B, C, D, E, R>(_: A, _: B, _: C, _: D, _: E) -> R {
        unreachable!()
    }
    extern "C" fn unused3<A, B, C, R>(_: A, _: B, _: C) -> R {
        panic!("unused property service")
    }
    unsafe extern "C" fn byte(reader: *mut ByteReader, out: *mut u8) -> bool {
        let cursor = unsafe { &mut *reader.cast::<crate::newgrf_bytereader::Cursor>() };
        match unsafe { cursor.byte() } {
            Ok(value) => {
                unsafe {
                    *out = value;
                }
                true
            }
            Err(_) => false,
        }
    }
    unsafe extern "C" fn word(reader: *mut ByteReader, out: *mut u16) -> bool {
        let cursor = unsafe { &mut *reader.cast::<crate::newgrf_bytereader::Cursor>() };
        match unsafe { cursor.word() } {
            Ok(value) => {
                unsafe {
                    *out = value;
                }
                true
            }
            Err(_) => false,
        }
    }
    let services = Properties {
        loading: std::ptr::null(),
        id: unused1,
        sound: unused2,
        train_image_valid: unused1,
        road_image_valid: unused1,
        ship_image_valid: unused1,
        aircraft_image_valid: unused1,
        uses_2cc: unused1,
    };
    let reads = ReaderServices {
        has_data: unused2,
        peek_dword: unused2,
        read_bytes: unused3,
        read_byte: byte,
        read_word: word,
        read_dword: unused2,
        skip: unused2,
        assign_badges: unused3,
        read_badge_ids: unused5,
        skip_badges: unused1,
    };
    let file = std::ptr::dangling::<std::ffi::c_void>();
    for include in [false, true] {
        for bytes in [&[][..], &[2][..]] {
            unsafe {
                (*temp(1)).fields.ctt_include_mask = 17;
                (*temp(1)).fields.ctt_exclude_mask = 23;
                (*temp(1)).fields.defaultcargo_grf = std::ptr::null();
                let mut cursor = crate::newgrf_bytereader::Cursor::new(bytes.as_ptr(), bytes.len());
                let mut reader = Reader::new((&raw mut cursor).cast(), &reads);
                assert!(cargo_list(1, file, include, &mut reader).is_err());
                assert_eq!(
                    (*temp(1)).fields.ctt_include_mask,
                    if include && !bytes.is_empty() { 0 } else { 17 }
                );
                assert_eq!(
                    (*temp(1)).fields.ctt_exclude_mask,
                    if !include && !bytes.is_empty() { 0 } else { 23 }
                );
                assert_eq!(
                    (*temp(1)).fields.defaultcargo_grf,
                    if include && !bytes.is_empty() {
                        file
                    } else {
                        std::ptr::null()
                    }
                );
                assert_eq!(cursor.remaining(), 0);
            }
        }
    }
    unsafe {
        let bytes = [255, 1];
        let mut cursor = crate::newgrf_bytereader::Cursor::new(bytes.as_ptr(), bytes.len());
        let mut reader = Reader::new((&raw mut cursor).cast(), &reads);
        assert!(reader.extended().is_err());
        assert_eq!(cursor.remaining(), 1);
        let mut cursor = crate::newgrf_bytereader::Cursor::new(bytes.as_ptr(), 0);
        let mut reader = Reader::new((&raw mut cursor).cast(), &reads);
        (*temp(1)).fields.defaultcargo_grf = std::ptr::null();
        assert!(
            default_cargo(&services, std::ptr::null(), 1, file, VEH_TRAIN, &mut reader).is_err()
        );
        assert_eq!((*temp(1)).fields.defaultcargo_grf, file);
        for bytes in [&[][..], &[2][..]] {
            (*temp(1)).labels = vec![crate::rail_type::RAILTYPE_LABEL_RAIL];
            let mut cursor = crate::newgrf_bytereader::Cursor::new(bytes.as_ptr(), bytes.len());
            let mut reader = Reader::new((&raw mut cursor).cast(), &reads);
            assert!(train(&services, std::ptr::null(), 1, file, 0x34, &mut reader).is_err());
            assert_eq!((*temp(1)).labels.len(), usize::from(bytes.is_empty()));
            assert_eq!(cursor.remaining(), 0);
        }
    }
}
