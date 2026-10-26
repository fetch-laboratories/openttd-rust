/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Complete vehicle Action0 property interpretation. Native specifications remain canonical.
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
    openttd_rust_engine_translate_refit, openttd_rust_vehicle_order, temp,
};
use crate::bitmath_func::{assign_bit, has_bit, sb, set_bit64};
use crate::engine_type::{
    AIR_CTOL, AIR_HELI, EC_DIESEL, EC_ELECTRIC, EC_MAGLEV, EC_MONORAIL, EC_STEAM, VE_DEFAULT,
    VE_DISABLE_EFFECT, VE_TYPE_COUNT, VE_TYPE_START,
};
use crate::newgrf_bytereader::{
    BadgeList, ByteReader, ReadError, Reader, Services as ReaderServices,
};
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
use crate::vehicle_func::CUSTOM_VEHICLE_SPRITENUM;
#[repr(C)]
pub struct Properties {
    loading: *const Services,
    set_intro: extern "C" fn(E, i32),
    set_decay: extern "C" fn(E, u8),
    set_life: extern "C" fn(E, i32),
    set_model_life: extern "C" fn(E, i32),
    set_climates: extern "C" fn(E, u8),
    set_load_amount: extern "C" fn(E, u8),
    set_cargo_label: extern "C" fn(E, u32),
    set_callbacks: extern "C" fn(E, u16),
    set_refit_cost: extern "C" fn(E, u8),
    set_retire: extern "C" fn(E, u8),
    set_misc: extern "C" fn(E, u8),
    set_extra: extern "C" fn(E, u32),
    set_cargo_age: extern "C" fn(E, u16),
    set_train_ai: extern "C" fn(E, u8),
    set_train_speed: extern "C" fn(E, u16),
    set_train_power: extern "C" fn(E, u16),
    set_train_kind: extern "C" fn(E, RailVehicleTypes),
    set_train_running: extern "C" fn(E, u8),
    set_train_price: extern "C" fn(E, u8),
    set_train_image: extern "C" fn(E, u8),
    set_train_capacity: extern "C" fn(E, u8),
    set_train_weight: extern "C" fn(E, u16),
    set_train_cost: extern "C" fn(E, u8),
    set_train_class: extern "C" fn(E, u8),
    set_train_wagon_power: extern "C" fn(E, u16),
    set_train_te: extern "C" fn(E, u8),
    set_train_drag: extern "C" fn(E, u8),
    set_train_shorten: extern "C" fn(E, u8),
    set_train_visual: extern "C" fn(E, u8),
    set_train_wagon_weight: extern "C" fn(E, u8),
    set_train_user: extern "C" fn(E, u8),
    set_train_curve: extern "C" fn(E, i16),
    set_road_running: extern "C" fn(E, u8),
    set_road_price: extern "C" fn(E, u8),
    set_road_image: extern "C" fn(E, u8),
    set_road_capacity: extern "C" fn(E, u8),
    set_road_cost: extern "C" fn(E, u8),
    set_road_sound: extern "C" fn(E, u16),
    set_road_power: extern "C" fn(E, u8),
    set_road_weight: extern "C" fn(E, u8),
    set_road_te: extern "C" fn(E, u8),
    set_road_drag: extern "C" fn(E, u8),
    set_road_visual: extern "C" fn(E, u8),
    set_road_shorten: extern "C" fn(E, u8),
    set_ship_image: extern "C" fn(E, u8),
    set_ship_cost: extern "C" fn(E, u8),
    set_ship_speed: extern "C" fn(E, u16),
    set_ship_capacity: extern "C" fn(E, u16),
    set_ship_running: extern "C" fn(E, u8),
    set_ship_sound: extern "C" fn(E, u16),
    set_ship_ocean: extern "C" fn(E, u8),
    set_ship_canal: extern "C" fn(E, u8),
    set_ship_visual: extern "C" fn(E, u8),
    set_ship_acceleration: extern "C" fn(E, u8),
    set_aircraft_image: extern "C" fn(E, u8),
    set_aircraft_subtype: extern "C" fn(E, u8),
    set_aircraft_cost: extern "C" fn(E, u8),
    set_aircraft_speed: extern "C" fn(E, u16),
    set_aircraft_acceleration: extern "C" fn(E, u8),
    set_aircraft_running: extern "C" fn(E, u8),
    set_aircraft_passengers: extern "C" fn(E, u16),
    set_aircraft_mail: extern "C" fn(E, u8),
    set_aircraft_sound: extern "C" fn(E, u16),
    set_aircraft_range: extern "C" fn(E, u16),
    id: extern "C" fn(E) -> u16,
    callbacks: extern "C" fn(E) -> u16,
    train_power: extern "C" fn(E) -> u16,
    train_kind: extern "C" fn(E) -> RailVehicleTypes,
    train_weight: extern "C" fn(E) -> u16,
    train_class: extern "C" fn(E) -> u8,
    aircraft_subtype: extern "C" fn(E) -> u8,
    badges: extern "C" fn(E) -> *mut BadgeList,
    sound: extern "C" fn(File, u16) -> u16,
    train_image_valid: extern "C" fn(u8) -> bool,
    road_image_valid: extern "C" fn(u8) -> bool,
    ship_image_valid: extern "C" fn(u8) -> bool,
    aircraft_image_valid: extern "C" fn(u8) -> bool,
    uses_2cc: extern "C" fn(bool),
    invalid_sprite: extern "C" fn(u8, u8),
    invalid_cargo: extern "C" fn(u8, u8),
    invalid_track: extern "C" fn(u8),
    invalid_weight: extern "C" fn(u8),
    ai_rank: extern "C" fn(),
    invalid_price: extern "C" fn(*const u8, usize, u32),
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDVehicleProperties" ["engine_loading"] => Properties { loading, set_intro, set_decay, set_life, set_model_life, set_climates, set_load_amount, set_cargo_label, set_callbacks, set_refit_cost, set_retire, set_misc, set_extra, set_cargo_age, set_train_ai, set_train_speed, set_train_power, set_train_kind, set_train_running, set_train_price, set_train_image, set_train_capacity, set_train_weight, set_train_cost, set_train_class, set_train_wagon_power, set_train_te, set_train_drag, set_train_shorten, set_train_visual, set_train_wagon_weight, set_train_user, set_train_curve, set_road_running, set_road_price, set_road_image, set_road_capacity, set_road_cost, set_road_sound, set_road_power, set_road_weight, set_road_te, set_road_drag, set_road_visual, set_road_shorten, set_ship_image, set_ship_cost, set_ship_speed, set_ship_capacity, set_ship_running, set_ship_sound, set_ship_ocean, set_ship_canal, set_ship_visual, set_ship_acceleration, set_aircraft_image, set_aircraft_subtype, set_aircraft_cost, set_aircraft_speed, set_aircraft_acceleration, set_aircraft_running, set_aircraft_passengers, set_aircraft_mail, set_aircraft_sound, set_aircraft_range, id, callbacks, train_power, train_kind, train_weight, train_class, aircraft_subtype, badges, sound, train_image_valid, road_image_valid, ship_image_valid, aircraft_image_valid, uses_2cc, invalid_sprite, invalid_cargo, invalid_track, invalid_weight, ai_rank, invalid_price };
}

fn common(s: &Properties, e: E, prop: i32, r: &mut Reader<'_>) -> Result<i32, ReadError> {
    match prop {
        0x00 => (s.set_intro)(
            e,
            crate::timer_game_common::DAYS_TILL_ORIGINAL_BASE_YEAR + i32::from(r.word()?),
        ),
        0x02 => (s.set_decay)(e, r.byte()?),
        0x03 => (s.set_life)(e, i32::from(r.byte()?)),
        0x04 => (s.set_model_life)(e, i32::from(r.byte()?)),
        0x06 => (s.set_climates)(e, r.byte()?),
        p if p == i32::from(PROP_VEHICLE_LOAD_AMOUNT) => (s.set_load_amount)(e, r.byte()?),
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
fn price(s: &Properties, e: E, kind: u8, value: u32) {
    if let Some(index) = base_price(value) {
        if kind == VEH_TRAIN {
            (s.set_train_price)(e, index);
        } else {
            (s.set_road_price)(e, index);
        }
    } else {
        let name = if kind == VEH_TRAIN {
            "RailVehicleChangeInfo"
        } else {
            "RoadVehicleChangeInfo"
        };
        (s.invalid_price)(name.as_ptr(), name.len(), value);
    }
}
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
        (s.invalid_sprite)(kind, value);
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
fn callbacks(s: &Properties, e: E, r: &mut Reader<'_>, high: bool) -> Result<(), ReadError> {
    let mut mask = u32::from((s.callbacks)(e));
    sb(&mut mask, if high { 8 } else { 0 }, 8, u32::from(r.byte()?));
    (s.set_callbacks)(e, mask as u16);
    Ok(())
}
fn misc(s: &Properties, e: E, value: u8) {
    (s.set_misc)(e, value);
    (s.uses_2cc)(value & crate::engine_type::USES_2CC != 0);
}
unsafe fn default_cargo(
    s: &Properties,
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
            (s.invalid_cargo)(kind, value);
        }
        cargo
    };
    (unsafe { &*s.loading }.set_cargo)(e, cargo);
    (s.set_cargo_label)(e, CT_INVALID);
    Ok(())
}
unsafe fn refit(s: &Properties, e: E, id: u16, file: File, value: u32) {
    unsafe {
        openttd_rust_engine_temp_refittability(&raw mut (*temp(id)).fields, value != 0);
    }
    (unsafe { &*s.loading }.set_refit)(e, openttd_rust_engine_translate_refit(value, file));
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
                    0 => Some(if (s.train_class)(e) >= EC_ELECTRIC {
                        RAILTYPE_LABEL_ELECTRIC
                    } else {
                        RAILTYPE_LABEL_RAIL
                    }),
                    1 => Some(RAILTYPE_LABEL_MONO),
                    2 => Some(RAILTYPE_LABEL_MAGLEV),
                    _ => {
                        (s.invalid_track)(track);
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
        0x08 => (s.set_train_ai)(e, r.byte()?),
        p if p == i32::from(PROP_TRAIN_SPEED) => {
            let speed = r.word()?;
            (s.set_train_speed)(e, if speed == 0xffff { 0 } else { speed });
        }
        p if p == i32::from(PROP_TRAIN_POWER) => {
            let power = r.word()?;
            (s.set_train_power)(e, power);
            if power != 0 {
                if (s.train_kind)(e) == RailVehicleTypes::WAGON {
                    (s.set_train_kind)(e, RailVehicleTypes::SINGLEHEAD);
                }
            } else {
                (s.set_train_kind)(e, RailVehicleTypes::WAGON);
            }
        }
        p if p == i32::from(PROP_TRAIN_RUNNING_COST_FACTOR) => (s.set_train_running)(e, r.byte()?),
        0x0e => price(s, e, VEH_TRAIN, r.dword()?),
        0x12 => (s.set_train_image)(e, image(s, VEH_TRAIN, r.byte()?)),
        0x13 => {
            let dual = r.byte()?;
            let kind = if dual != 0 {
                RailVehicleTypes::MULTIHEAD
            } else if (s.train_power)(e) == 0 {
                RailVehicleTypes::WAGON
            } else {
                RailVehicleTypes::SINGLEHEAD
            };
            (s.set_train_kind)(e, kind);
        }
        p if p == i32::from(PROP_TRAIN_CARGO_CAPACITY) => (s.set_train_capacity)(e, r.byte()?),
        0x15 => unsafe {
            default_cargo(s, e, id, file, VEH_TRAIN, r)?;
        },
        p if p == i32::from(PROP_TRAIN_WEIGHT) => {
            let low = r.byte()?;
            let mut weight = u32::from((s.train_weight)(e));
            sb(&mut weight, 0, 8, u32::from(low));
            (s.set_train_weight)(e, weight as u16);
        }
        p if p == i32::from(PROP_TRAIN_COST_FACTOR) => (s.set_train_cost)(e, r.byte()?),
        0x18 => {
            (s.ai_rank)();
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
            (s.set_train_class)(e, class);
        }
        0x1a => unsafe {
            openttd_rust_vehicle_order(id, r.extended()?);
        },
        0x1b => (s.set_train_wagon_power)(e, r.word()?),
        0x1c => (s.set_refit_cost)(e, r.byte()?),
        0x1d => unsafe {
            refit(s, e, id, file, r.dword()?);
        },
        0x1e => callbacks(s, e, r, false)?,
        p if p == i32::from(PROP_TRAIN_TRACTIVE_EFFORT) => (s.set_train_te)(e, r.byte()?),
        0x20 => (s.set_train_drag)(e, r.byte()?),
        p if p == i32::from(PROP_TRAIN_SHORTEN_FACTOR) => (s.set_train_shorten)(e, r.byte()?),
        0x22 => (s.set_train_visual)(e, visual(s, r.byte()?)),
        0x23 => (s.set_train_wagon_weight)(e, r.byte()?),
        0x24 => {
            let high = r.byte()?;
            if high > 4 {
                (s.invalid_weight)(high);
            } else {
                let mut weight = u32::from((s.train_weight)(e));
                sb(&mut weight, 8, 8, u32::from(high));
                (s.set_train_weight)(e, weight as u16);
            }
        }
        p if p == i32::from(PROP_TRAIN_USER_DATA) => (s.set_train_user)(e, r.byte()?),
        0x26 => (s.set_retire)(e, r.byte()?),
        0x27 => misc(s, e, r.byte()?),
        0x28 => unsafe {
            allowed(id, file, r.word()?);
        },
        0x29 => unsafe {
            disallowed(id, r.word()?);
        },
        0x2a => (s.set_intro)(e, r.dword()? as i32),
        p if p == i32::from(PROP_TRAIN_CARGO_AGE_PERIOD) => (s.set_cargo_age)(e, r.word()?),
        0x2c | 0x2d => unsafe {
            cargo_list(id, file, prop == 0x2c, r)?;
        },
        p if p == i32::from(PROP_TRAIN_CURVE_SPEED_MOD) => (s.set_train_curve)(e, r.word()? as i16),
        0x2f => (unsafe { &*s.loading }.set_variant)(e, r.word()?),
        0x30 => (s.set_extra)(e, r.dword()?),
        0x31 => callbacks(s, e, r, true)?,
        0x32 => unsafe {
            (*temp(id)).fields.cargo_allowed_required = r.word()?;
        },
        0x33 => unsafe {
            r.badges(VEH_TRAIN, (s.badges)(e))?;
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
                    (s.invalid_track)(track);
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
        0x08 => (unsafe { &*s.loading }.set_road_speed)(e, u16::from(r.byte()?)),
        p if p == i32::from(PROP_ROADVEH_RUNNING_COST_FACTOR) => (s.set_road_running)(e, r.byte()?),
        0x0a => price(s, e, VEH_ROAD, r.dword()?),
        0x0e => (s.set_road_image)(e, image(s, VEH_ROAD, r.byte()?)),
        p if p == i32::from(PROP_ROADVEH_CARGO_CAPACITY) => (s.set_road_capacity)(e, r.byte()?),
        0x10 => unsafe {
            default_cargo(s, e, id, file, VEH_ROAD, r)?;
        },
        p if p == i32::from(PROP_ROADVEH_COST_FACTOR) => (s.set_road_cost)(e, r.byte()?),
        0x12 => (s.set_road_sound)(e, (s.sound)(file, u16::from(r.byte()?))),
        p if p == i32::from(PROP_ROADVEH_POWER) => (s.set_road_power)(e, r.byte()?),
        p if p == i32::from(PROP_ROADVEH_WEIGHT) => (s.set_road_weight)(e, r.byte()?),
        p if p == i32::from(PROP_ROADVEH_SPEED) => unsafe {
            (*temp(id)).fields.rv_max_speed = r.byte()?;
        },
        0x16 => unsafe {
            refit(s, e, id, file, r.dword()?);
        },
        0x17 => callbacks(s, e, r, false)?,
        p if p == i32::from(PROP_ROADVEH_TRACTIVE_EFFORT) => (s.set_road_te)(e, r.byte()?),
        0x19 => (s.set_road_drag)(e, r.byte()?),
        0x1a => (s.set_refit_cost)(e, r.byte()?),
        0x1b => (s.set_retire)(e, r.byte()?),
        0x1c => misc(s, e, r.byte()?),
        0x1d => unsafe {
            allowed(id, file, r.word()?);
        },
        0x1e => unsafe {
            disallowed(id, r.word()?);
        },
        0x1f => (s.set_intro)(e, r.dword()? as i32),
        0x20 => unsafe {
            openttd_rust_vehicle_order(id, r.extended()?);
        },
        0x21 => (s.set_road_visual)(e, visual(s, r.byte()?)),
        p if p == i32::from(PROP_ROADVEH_CARGO_AGE_PERIOD) => (s.set_cargo_age)(e, r.word()?),
        p if p == i32::from(PROP_ROADVEH_SHORTEN_FACTOR) => (s.set_road_shorten)(e, r.byte()?),
        0x24 | 0x25 => unsafe {
            cargo_list(id, file, prop == 0x24, r)?;
        },
        0x26 => (unsafe { &*s.loading }.set_variant)(e, r.word()?),
        0x27 => (s.set_extra)(e, r.dword()?),
        0x28 => callbacks(s, e, r, true)?,
        0x29 => unsafe {
            (*temp(id)).fields.cargo_allowed_required = r.word()?;
        },
        0x2a => unsafe {
            r.badges(VEH_ROAD, (s.badges)(e))?;
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
        0x08 => (s.set_ship_image)(e, image(s, VEH_SHIP, r.byte()?)),
        0x09 => (unsafe { &*s.loading }.set_old_refittable)(e, r.byte()? != 0),
        p if p == i32::from(PROP_SHIP_COST_FACTOR) => (s.set_ship_cost)(e, r.byte()?),
        p if p == i32::from(PROP_SHIP_SPEED) => (s.set_ship_speed)(e, u16::from(r.byte()?)),
        0x0c => unsafe {
            default_cargo(s, e, id, file, VEH_SHIP, r)?;
        },
        p if p == i32::from(PROP_SHIP_CARGO_CAPACITY) => (s.set_ship_capacity)(e, r.word()?),
        p if p == i32::from(PROP_SHIP_RUNNING_COST_FACTOR) => (s.set_ship_running)(e, r.byte()?),
        0x10 => (s.set_ship_sound)(e, (s.sound)(file, u16::from(r.byte()?))),
        0x11 => unsafe {
            refit(s, e, id, file, r.dword()?);
        },
        0x12 => callbacks(s, e, r, false)?,
        0x13 => (s.set_refit_cost)(e, r.byte()?),
        0x14 => (s.set_ship_ocean)(e, r.byte()?),
        0x15 => (s.set_ship_canal)(e, r.byte()?),
        0x16 => (s.set_retire)(e, r.byte()?),
        0x17 => misc(s, e, r.byte()?),
        0x18 => unsafe {
            allowed(id, file, r.word()?);
        },
        0x19 => unsafe {
            disallowed(id, r.word()?);
        },
        0x1a => (s.set_intro)(e, r.dword()? as i32),
        0x1b => unsafe {
            openttd_rust_vehicle_order(id, r.extended()?);
        },
        0x1c => (s.set_ship_visual)(e, visual(s, r.byte()?)),
        p if p == i32::from(PROP_SHIP_CARGO_AGE_PERIOD) => (s.set_cargo_age)(e, r.word()?),
        0x1e | 0x1f => unsafe {
            cargo_list(id, file, prop == 0x1e, r)?;
        },
        0x20 => (unsafe { &*s.loading }.set_variant)(e, r.word()?),
        0x21 => (s.set_extra)(e, r.dword()?),
        0x22 => callbacks(s, e, r, true)?,
        0x23 => (s.set_ship_speed)(e, r.word()?),
        0x24 => (s.set_ship_acceleration)(e, r.byte()?.max(1)),
        0x25 => unsafe {
            (*temp(id)).fields.cargo_allowed_required = r.word()?;
        },
        0x26 => unsafe {
            r.badges(VEH_SHIP, (s.badges)(e))?;
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
        0x08 => (s.set_aircraft_image)(e, image(s, VEH_AIRCRAFT, r.byte()?)),
        0x09 => {
            let value = r.byte()?;
            let subtype = if value == 0 {
                AIR_HELI
            } else {
                let mut subtype = u32::from((s.aircraft_subtype)(e));
                sb(&mut subtype, 0, 1, u32::from(AIR_CTOL));
                subtype as u8
            };
            (s.set_aircraft_subtype)(e, subtype);
        }
        0x0a => {
            let large = r.byte()? != 0;
            let mut subtype = u32::from((s.aircraft_subtype)(e));
            assign_bit(&mut subtype, 1, large);
            (s.set_aircraft_subtype)(e, subtype as u8);
        }
        p if p == i32::from(PROP_AIRCRAFT_COST_FACTOR) => (s.set_aircraft_cost)(e, r.byte()?),
        p if p == i32::from(PROP_AIRCRAFT_SPEED) => {
            (s.set_aircraft_speed)(e, u16::from(r.byte()?) * 128 / 10);
        }
        0x0d => (s.set_aircraft_acceleration)(e, r.byte()?),
        p if p == i32::from(PROP_AIRCRAFT_RUNNING_COST_FACTOR) => {
            (s.set_aircraft_running)(e, r.byte()?);
        }
        p if p == i32::from(PROP_AIRCRAFT_PASSENGER_CAPACITY) => {
            (s.set_aircraft_passengers)(e, r.word()?);
        }
        p if p == i32::from(PROP_AIRCRAFT_MAIL_CAPACITY) => (s.set_aircraft_mail)(e, r.byte()?),
        0x12 => (s.set_aircraft_sound)(e, (s.sound)(file, u16::from(r.byte()?))),
        0x13 => unsafe {
            refit(s, e, id, file, r.dword()?);
        },
        0x14 => callbacks(s, e, r, false)?,
        0x15 => (s.set_refit_cost)(e, r.byte()?),
        0x16 => (s.set_retire)(e, r.byte()?),
        0x17 => misc(s, e, r.byte()?),
        0x18 => unsafe {
            allowed(id, file, r.word()?);
        },
        0x19 => unsafe {
            disallowed(id, r.word()?);
        },
        0x1a => (s.set_intro)(e, r.dword()? as i32),
        0x1b => unsafe {
            openttd_rust_vehicle_order(id, r.extended()?);
        },
        p if p == i32::from(PROP_AIRCRAFT_CARGO_AGE_PERIOD) => (s.set_cargo_age)(e, r.word()?),
        0x1d | 0x1e => unsafe {
            cargo_list(id, file, prop == 0x1d, r)?;
        },
        p if p == i32::from(PROP_AIRCRAFT_RANGE) => (s.set_aircraft_range)(e, r.word()?),
        0x20 => (unsafe { &*s.loading }.set_variant)(e, r.word()?),
        0x21 => (s.set_extra)(e, r.dword()?),
        0x22 => callbacks(s, e, r, true)?,
        0x23 => unsafe {
            (*temp(id)).fields.cargo_allowed_required = r.word()?;
        },
        0x24 => unsafe {
            r.badges(VEH_AIRCRAFT, (s.badges)(e))?;
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
    extern "C" fn unused0<R>() -> R {
        panic!("unused property service")
    }
    extern "C" fn unused1<A, R>(_: A) -> R {
        panic!("unused property service")
    }
    extern "C" fn unused2<A, B, R>(_: A, _: B) -> R {
        panic!("unused property service")
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
        set_intro: unused2,
        set_decay: unused2,
        set_life: unused2,
        set_model_life: unused2,
        set_climates: unused2,
        set_load_amount: unused2,
        set_cargo_label: unused2,
        set_callbacks: unused2,
        set_refit_cost: unused2,
        set_retire: unused2,
        set_misc: unused2,
        set_extra: unused2,
        set_cargo_age: unused2,
        set_train_ai: unused2,
        set_train_speed: unused2,
        set_train_power: unused2,
        set_train_kind: unused2,
        set_train_running: unused2,
        set_train_price: unused2,
        set_train_image: unused2,
        set_train_capacity: unused2,
        set_train_weight: unused2,
        set_train_cost: unused2,
        set_train_class: unused2,
        set_train_wagon_power: unused2,
        set_train_te: unused2,
        set_train_drag: unused2,
        set_train_shorten: unused2,
        set_train_visual: unused2,
        set_train_wagon_weight: unused2,
        set_train_user: unused2,
        set_train_curve: unused2,
        set_road_running: unused2,
        set_road_price: unused2,
        set_road_image: unused2,
        set_road_capacity: unused2,
        set_road_cost: unused2,
        set_road_sound: unused2,
        set_road_power: unused2,
        set_road_weight: unused2,
        set_road_te: unused2,
        set_road_drag: unused2,
        set_road_visual: unused2,
        set_road_shorten: unused2,
        set_ship_image: unused2,
        set_ship_cost: unused2,
        set_ship_speed: unused2,
        set_ship_capacity: unused2,
        set_ship_running: unused2,
        set_ship_sound: unused2,
        set_ship_ocean: unused2,
        set_ship_canal: unused2,
        set_ship_visual: unused2,
        set_ship_acceleration: unused2,
        set_aircraft_image: unused2,
        set_aircraft_subtype: unused2,
        set_aircraft_cost: unused2,
        set_aircraft_speed: unused2,
        set_aircraft_acceleration: unused2,
        set_aircraft_running: unused2,
        set_aircraft_passengers: unused2,
        set_aircraft_mail: unused2,
        set_aircraft_sound: unused2,
        set_aircraft_range: unused2,
        id: unused1,
        callbacks: unused1,
        train_power: unused1,
        train_kind: unused1,
        train_weight: unused1,
        train_class: unused1,
        aircraft_subtype: unused1,
        badges: unused1,
        sound: unused2,
        train_image_valid: unused1,
        road_image_valid: unused1,
        ship_image_valid: unused1,
        aircraft_image_valid: unused1,
        uses_2cc: unused1,
        invalid_sprite: unused2,
        invalid_cargo: unused2,
        invalid_track: unused1,
        invalid_weight: unused1,
        ai_rank: unused0,
        invalid_price: unused3,
    };
    let reads = ReaderServices {
        read_byte: byte,
        read_word: word,
        read_dword: unused2,
        skip: unused2,
        read_badges: unused3,
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
