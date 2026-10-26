/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! House definition loading, validation and publication. Native shells and defaults
//! remain canonical. Entries run under serialized GRF loading; no reference into
//! native storage spans a callback. Read failures return before native rethrow.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]
use crate::cargo_registry::{
    openttd_rust_cargo_by_label as by_label, openttd_rust_cargo_translate as translate,
};
use crate::cargo_type::{
    CT_CANDY, CT_FIZZY_DRINKS, CT_FOOD, CT_GOODS, CT_INVALID, INVALID_CARGO, is_valid_cargo_type,
};
use crate::gfx_type::{COLOUR_BLUE, COLOUR_GREEN, COLOUR_ORANGE, COLOUR_RED};
use crate::house::{
    BUILDING_HAS_1_TILE, BUILDING_HAS_2_TILES, BUILDING_HAS_4_TILES, BUILDING_IS_CHURCH,
    BUILDING_IS_STADIUM, HOUSE_NUM_ACCEPTS, HOUSE_ORIGINAL_NUM_ACCEPTS, HZ_CLIMATE_ALL,
    HZ_ZONE_ALL, INVALID_HOUSE_ID, NEW_HOUSE_OFFSET, NUM_HOUSES, NUM_HOUSES_PER_GRF,
};
use crate::newgrf_animation_type::{ANIMATION_LOOPING, ANIMATION_NON_LOOPING};
use crate::newgrf_bytereader::{BadgeList, ByteReader, ReadError, Reader};
use crate::timer_game_common::{MAX_YEAR, MIN_YEAR, ORIGINAL_BASE_YEAR};
use std::ffi::c_void;
type Spec = *const c_void;
type File = *mut c_void;
#[repr(C)]
pub struct Identity {
    grfid: u32,
    local: u16,
    substitute: u16,
}
#[repr(C)]
pub struct Validity {
    availability: u16,
    substitute: u16,
    flags: u8,
}
#[repr(C)]
pub struct Part {
    flags: u8,
    population: u8,
    enabled: bool,
}
#[repr(C)]
pub struct Early {
    year: i32,
    availability: u16,
    enabled: bool,
}
#[repr(C)]
pub struct Cargo {
    label: u32,
    cargo: u8,
}
#[repr(C)]
pub struct PendingOverride {
    grfid: u32,
    local: u16,
}
#[repr(C)]
pub struct Services {
    get: extern "C" fn(u16) -> Spec,
    count: extern "C" fn() -> usize,
    clear_specs: extern "C" fn(),
    reserve_specs: extern "C" fn(usize),
    append_default: extern "C" fn(u16),
    file_count: extern "C" fn() -> usize,
    file_at: extern "C" fn(usize) -> File,
    local_count: extern "C" fn(File) -> usize,
    has_filename: extern "C" fn(File) -> bool,
    resize_local: extern "C" fn(File, usize),
    local: extern "C" fn(File, usize) -> Spec,
    clear_local: extern "C" fn(File),
    copy_substitute: extern "C" fn(File, u16, Spec) -> Spec,
    set_identity: extern "C" fn(Spec, File, u16, u16),
    identity: extern "C" fn(Spec) -> Identity,
    validity: extern "C" fn(Spec) -> Validity,
    part: extern "C" fn(Spec) -> Part,
    flags: extern "C" fn(Spec) -> u8,
    early: extern "C" fn(Spec) -> Early,
    cargo: extern "C" fn(Spec, u8) -> Cargo,
    set_cargo: extern "C" fn(Spec, u8, u8),
    set_label: extern "C" fn(Spec, u8, u32),
    set_acceptance: extern "C" fn(Spec, u8, u8),
    set_colour: extern "C" fn(Spec, u8, u8),
    callbacks: extern "C" fn(Spec) -> u16,
    watched: extern "C" fn(Spec) -> u64,
    map_string: extern "C" fn(Spec, u16),
    set_animation: extern "C" fn(Spec, u8, u8),
    badges: extern "C" fn(Spec) -> *mut BadgeList,
    file_id: extern "C" fn(File) -> u32,
    add_override: extern "C" fn(u16, u32, u16),
    add_entity: extern "C" fn(u16, u32, u16) -> u16,
    pending_override: extern "C" fn(u16) -> PendingOverride,
    clear_override: extern "C" fn(u16),
    reset_override: extern "C" fn(),
    resize_specs: extern "C" fn(usize),
    move_spec: extern "C" fn(Spec, u16),
    set_override: extern "C" fn(Spec, u16),
    climate_mask: extern "C" fn() -> u16,
    landscape: extern "C" fn() -> u8,
    too_many: extern "C" fn(u32),
    invalid_substitute: extern "C" fn(u8, u32),
    invalid_override: extern "C" fn(u8, u32),
    allocation_failed: extern "C" fn(),
    disable_list: extern "C" fn(i32),
    invalid_missing: extern "C" fn(File, Spec),
    invalid_population: extern "C" fn(File, Spec),
    invalid_size: extern "C" fn(File, Spec),
    invalid_availability: extern "C" fn(File, Spec),
    set_enabled: extern "C" fn(Spec, bool),
    set_flags: extern "C" fn(Spec, u8),
    set_min_year: extern "C" fn(Spec, i32),
    set_max_year: extern "C" fn(Spec, i32),
    set_population: extern "C" fn(Spec, u8),
    set_mail: extern "C" fn(Spec, u8),
    set_rating: extern "C" fn(Spec, u16),
    set_removal: extern "C" fn(Spec, u8),
    set_availability: extern "C" fn(Spec, u16),
    set_callbacks: extern "C" fn(Spec, u16),
    set_processing: extern "C" fn(Spec, u8),
    set_probability: extern "C" fn(Spec, u8),
    set_extra: extern "C" fn(Spec, u8),
    set_animation_speed: extern "C" fn(Spec, u8),
    set_class_id: extern "C" fn(Spec, u16),
    set_minimum_life: extern "C" fn(Spec, u8),
    set_watched: extern "C" fn(Spec, u64),
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDHouseDefinitionIdentity" ["house_definition"] => Identity { grfid, local, substitute };
    "OpenTTDHouseDefinitionValidity" ["house_definition"] => Validity { availability, substitute, flags };
    "OpenTTDHouseDefinitionPart" ["house_definition"] => Part { flags, population, enabled };
    "OpenTTDHouseDefinitionEarly" ["house_definition"] => Early { year, availability, enabled };
    "OpenTTDHouseDefinitionCargo" ["house_definition"] => Cargo { label, cargo };
    "OpenTTDHouseDefinitionOverride" ["house_definition"] => PendingOverride { grfid, local };
    "OpenTTDHouseDefinitionServices" ["house_definition"] => Services { get, count, clear_specs, reserve_specs, append_default, file_count, file_at, local_count, has_filename, resize_local, local, clear_local, copy_substitute, set_identity, identity, validity, part, flags, early, cargo, set_cargo, set_label, set_acceptance, set_colour, callbacks, watched, map_string, set_animation, badges, file_id, add_override, add_entity, pending_override, clear_override, reset_override, resize_specs, move_spec, set_override, climate_mask, landscape, too_many, invalid_substitute, invalid_override, allocation_failed, disable_list, invalid_missing, invalid_population, invalid_size, invalid_availability, set_enabled, set_flags, set_min_year, set_max_year, set_population, set_mail, set_rating, set_removal, set_availability, set_callbacks, set_processing, set_probability, set_extra, set_animation_speed, set_class_id, set_minimum_life, set_watched };
}

fn ignore_property(prop: i32, reader: &mut Reader<'_>) -> Result<u8, ReadError> {
    match prop {
        0x09 | 0x0b..=0x0f | 0x11 | 0x14..=0x16 | 0x18..=0x1d | 0x1f => {
            reader.byte()?;
        }
        0x0a | 0x10 | 0x12 | 0x13 | 0x21 | 0x22 => {
            reader.word()?;
        }
        0x1e => {
            reader.dword()?;
        }
        0x17 => {
            for _ in 0..4 {
                reader.byte()?;
            }
        }
        0x20 => {
            let count = reader.byte()?;
            for _ in 0..count {
                reader.byte()?;
            }
        }
        0x23 => {
            let count = reader.byte()?;
            reader.skip(usize::from(count) * 2)?;
        }
        _ => return Ok(3),
    }
    Ok(0)
}
fn property(
    s: &Services,
    file: File,
    first: u32,
    last: u32,
    prop: i32,
    reader: &mut Reader<'_>,
) -> Result<u8, ReadError> {
    if last > NUM_HOUSES_PER_GRF {
        (s.too_many)(last);
        return Ok(4);
    }
    if (s.local_count)(file) < last as usize {
        (s.resize_local)(file, last as usize);
    }
    let mut result = 0;
    for id in first..last {
        let spec = (s.local)(file, id as usize);
        if prop != 0x08 && spec.is_null() {
            result = result.max(ignore_property(prop, reader)?);
            continue;
        }
        match prop {
            0x08 => {
                let substitute = reader.byte()?;
                if substitute == 0xff {
                    if id < u32::from(NEW_HOUSE_OFFSET) {
                        (s.set_enabled)((s.get)(id as u16), false);
                    }
                    continue;
                } else if u16::from(substitute) >= NEW_HOUSE_OFFSET {
                    (s.invalid_substitute)(substitute, id);
                    continue;
                }
                if spec.is_null() {
                    let source = (s.get)(u16::from(substitute));
                    let spec = (s.copy_substitute)(file, id as u16, source);
                    (s.set_enabled)(spec, true);
                    (s.set_identity)(spec, file, id as u16, u16::from(substitute));
                    for (index, colour) in [COLOUR_RED, COLOUR_BLUE, COLOUR_ORANGE, COLOUR_GREEN]
                        .into_iter()
                        .enumerate()
                    {
                        (s.set_colour)(spec, index as u8, colour);
                    }
                    let flags = (s.flags)(spec);
                    (s.set_flags)(spec, flags & !(BUILDING_IS_CHURCH | BUILDING_IS_STADIUM));
                    let cargo = (s.cargo)(spec, 2);
                    let cargo = if is_valid_cargo_type(cargo.cargo) {
                        cargo.cargo
                    } else {
                        by_label(cargo.label)
                    };
                    if !is_valid_cargo_type(cargo) {
                        (s.set_acceptance)(spec, 2, 0);
                    }
                }
            }
            0x09 => (s.set_flags)(spec, reader.byte()?),
            0x0a => {
                let years = reader.word()?;
                let year = |v: u8| {
                    if v > 150 {
                        MAX_YEAR
                    } else {
                        ORIGINAL_BASE_YEAR + i32::from(v)
                    }
                };
                (s.set_min_year)(spec, year(years as u8));
                (s.set_max_year)(spec, year((years >> 8) as u8));
            }
            0x0b => (s.set_population)(spec, reader.byte()?),
            0x0c => (s.set_mail)(spec, reader.byte()?),
            0x0d | 0x0e => (s.set_acceptance)(spec, (prop - 0x0d) as u8, reader.byte()?),
            0x0f => {
                let mut goods = i16::from(reader.byte()? as i8);
                let toyland = (s.landscape)() == crate::landscape_type::TOYLAND;
                let label = if goods >= 0 {
                    if toyland { CT_CANDY } else { CT_GOODS }
                } else if toyland {
                    CT_FIZZY_DRINKS
                } else {
                    CT_FOOD
                };
                let cargo = by_label(label);
                if !is_valid_cargo_type(cargo) {
                    goods = 0;
                }
                (s.set_cargo)(spec, 2, cargo);
                (s.set_label)(spec, 2, CT_INVALID);
                (s.set_acceptance)(spec, 2, goods.unsigned_abs() as u8);
            }
            0x10 => (s.set_rating)(spec, reader.word()?),
            0x11 => (s.set_removal)(spec, reader.byte()?),
            0x12 => (s.map_string)(spec, reader.word()?),
            0x13 => (s.set_availability)(spec, reader.word()?),
            0x14 => {
                let mask = (s.callbacks)(spec);
                (s.set_callbacks)(spec, (mask & 0xff00) | u16::from(reader.byte()?));
            }
            0x15 => {
                let target = reader.byte()?;
                if u16::from(target) >= NEW_HOUSE_OFFSET {
                    (s.invalid_override)(target, id);
                    continue;
                }
                (s.add_override)(id as u16, (s.file_id)(file), u16::from(target));
            }
            0x16 => (s.set_processing)(spec, reader.byte()?.min(63)),
            0x17 => {
                for index in 0..4 {
                    (s.set_colour)(spec, index, reader.byte()? & 0x0f);
                }
            }
            0x18 => (s.set_probability)(spec, reader.byte()?),
            0x19 => (s.set_extra)(spec, reader.byte()?),
            0x1a => {
                let info = reader.byte()?;
                (s.set_animation)(
                    spec,
                    info & 0x7f,
                    if info & 0x80 != 0 {
                        ANIMATION_LOOPING
                    } else {
                        ANIMATION_NON_LOOPING
                    },
                );
            }
            0x1b => (s.set_animation_speed)(spec, reader.byte()?.clamp(2, 16)),
            0x1c => {
                let class = crate::town::resolver::openttd_rust_house_class_allocate(
                    reader.byte()?,
                    (s.file_id)(file),
                );
                (s.set_class_id)(spec, class);
            }
            0x1d => {
                let mask = (s.callbacks)(spec);
                (s.set_callbacks)(spec, (mask & 0x00ff) | (u16::from(reader.byte()?) << 8));
            }
            0x1e => {
                let cargos = reader.dword()?;
                if cargos == u32::MAX {
                    continue;
                }
                for index in 0..HOUSE_ORIGINAL_NUM_ACCEPTS {
                    let cargo = translate((cargos >> (8 * index)) as u8, file, false);
                    if is_valid_cargo_type(cargo) {
                        (s.set_cargo)(spec, index, cargo);
                    } else {
                        (s.set_acceptance)(spec, index, 0);
                    }
                    (s.set_label)(spec, index, CT_INVALID);
                }
            }
            0x1f => (s.set_minimum_life)(spec, reader.byte()?),
            0x20 => {
                let count = reader.byte()?;
                for _ in 0..count {
                    let cargo = translate(reader.byte()?, file, false);
                    if is_valid_cargo_type(cargo) {
                        (s.set_watched)(spec, (s.watched)(spec) | (1_u64 << cargo));
                    }
                }
            }
            0x21 => (s.set_min_year)(spec, i32::from(reader.word()?)),
            0x22 => {
                let year = reader.word()?;
                (s.set_max_year)(
                    spec,
                    if year == u16::MAX {
                        MAX_YEAR
                    } else {
                        i32::from(year)
                    },
                );
            }
            0x23 => {
                let count = reader.byte()?;
                if count > HOUSE_NUM_ACCEPTS {
                    (s.disable_list)(prop);
                    return Ok(1);
                }
                for index in 0..HOUSE_NUM_ACCEPTS {
                    if index < count {
                        (s.set_cargo)(spec, index, translate(reader.byte()?, file, false));
                        (s.set_acceptance)(spec, index, reader.byte()?);
                    } else {
                        (s.set_cargo)(spec, index, INVALID_CARGO);
                        (s.set_acceptance)(spec, index, 0);
                    }
                    if index < HOUSE_ORIGINAL_NUM_ACCEPTS {
                        (s.set_label)(spec, index, CT_INVALID);
                    }
                }
            }
            0x24 => unsafe {
                reader.badges(crate::newgrf::GSF_HOUSES, (s.badges)(spec))?;
            },
            _ => result = 3,
        }
    }
    Ok(result)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_definition_property(
    s: *const Services,
    reads: *const crate::newgrf_bytereader::Services,
    file: File,
    first: u32,
    last: u32,
    prop: i32,
    native: *mut ByteReader,
) -> u8 {
    let mut reader = unsafe { Reader::new(native, &*reads) };
    property(unsafe { &*s }, file, first, last, prop, &mut reader).unwrap_or(u8::MAX)
}

fn valid(s: &Services, spec: Spec, next: [Spec; 3], file: File) -> bool {
    let info = (s.validity)(spec);
    let two = info.flags & BUILDING_HAS_2_TILES != 0;
    let four = info.flags & BUILDING_HAS_4_TILES != 0;
    let part = |spec: Spec| {
        if spec.is_null() {
            None
        } else {
            Some((s.part)(spec))
        }
    };
    let next1 = if two { part(next[0]) } else { None };
    let next2 = if four { part(next[1]) } else { None };
    let next3 = if four { part(next[2]) } else { None };
    let missing = |part: &Option<Part>| {
        part.as_ref()
            .is_none_or(|info| !info.enabled || info.flags & BUILDING_HAS_1_TILE != 0)
    };
    let report = !file.is_null() && (s.has_filename)(file);
    if (two && missing(&next1)) || (four && (missing(&next2) || missing(&next3))) {
        (s.set_enabled)(spec, false);
        if report {
            (s.invalid_missing)(file, spec);
        }
        return false;
    }
    if (two && next1.as_ref().is_some_and(|info| info.population != 0))
        || (four
            && (next2.as_ref().is_some_and(|info| info.population != 0)
                || next3.as_ref().is_some_and(|info| info.population != 0)))
    {
        (s.set_enabled)(spec, false);
        if report {
            (s.invalid_population)(file, spec);
        }
        return false;
    }
    if report
        && (info.flags & BUILDING_HAS_1_TILE)
            != ((s.flags)((s.get)(info.substitute)) & BUILDING_HAS_1_TILE)
    {
        (s.set_enabled)(spec, false);
        (s.invalid_size)(file, spec);
        return false;
    }
    if info.flags & BUILDING_HAS_1_TILE == 0
        && info.availability & HZ_ZONE_ALL != 0
        && info.availability & HZ_CLIMATE_ALL != 0
    {
        (s.set_enabled)(spec, false);
        if report {
            (s.invalid_availability)(file, spec);
        }
        return false;
    }
    true
}
fn early_house(s: &Services, mask: u16) {
    let mut min_year = MAX_YEAR;
    for id in 0..(s.count)() {
        let info = (s.early)((s.get)(id as u16));
        if !info.enabled || info.availability & mask != mask {
            continue;
        }
        min_year = min_year.min(info.year);
    }
    if min_year == 0 {
        return;
    }
    for id in 0..(s.count)() {
        let spec = (s.get)(id as u16);
        let info = (s.early)(spec);
        if !info.enabled || info.availability & mask != mask {
            continue;
        }
        if info.year == min_year {
            (s.set_min_year)(spec, MIN_YEAR);
        }
    }
}
pub fn reset(s: &Services) {
    (s.clear_specs)();
    (s.reserve_specs)(usize::from(NEW_HOUSE_OFFSET));
    crate::town::resolver::openttd_rust_house_classes_reset();
    for id in 0..NEW_HOUSE_OFFSET {
        (s.append_default)(id);
    }
    (s.reset_override)();
}
pub fn publish(s: &Services, spec: Spec) {
    let identity = (s.identity)(spec);
    let id = (s.add_entity)(identity.local, identity.grfid, identity.substitute);
    if id == INVALID_HOUSE_ID {
        (s.allocation_failed)();
        return;
    }
    if usize::from(id) >= (s.count)() {
        (s.resize_specs)(usize::from(id) + 1);
    }
    (s.move_spec)(spec, id);
    for original in 0..NEW_HOUSE_OFFSET {
        let overridden = (s.get)(original);
        let pending = (s.pending_override)(original);
        if pending.local != identity.local || pending.grfid != identity.grfid {
            continue;
        }
        (s.set_override)(overridden, id);
        (s.clear_override)(original);
    }
}
pub fn finalise(s: &Services) {
    for index in 0..(s.file_count)() {
        let file = (s.file_at)(index);
        let count = (s.local_count)(file);
        if count == 0 {
            continue;
        }
        for id in 0..count {
            let spec = (s.local)(file, id);
            if spec.is_null() {
                continue;
            }
            let next = [1, 2, 3].map(|offset| {
                if id + offset < count {
                    (s.local)(file, id + offset)
                } else {
                    std::ptr::null()
                }
            });
            if !valid(s, spec, next, file) {
                continue;
            }
            publish(s, spec);
        }
        (s.clear_local)(file);
    }
    let mut id = 0;
    while id < (s.count)() {
        let spec = (s.get)(id as u16);
        let next = [1, 2, 3].map(|offset| {
            if id + offset < usize::from(NUM_HOUSES) {
                (s.get)((id + offset) as u16)
            } else {
                std::ptr::null()
            }
        });
        if !valid(s, spec, next, std::ptr::null_mut()) {
            (s.set_flags)(spec, 0);
        }
        for index in 0..HOUSE_ORIGINAL_NUM_ACCEPTS {
            let info = (s.cargo)(spec, index);
            let cargo = if is_valid_cargo_type(info.cargo) {
                info.cargo
            } else {
                let cargo = by_label(info.label);
                (s.set_cargo)(spec, index, cargo);
                cargo
            };
            if !is_valid_cargo_type(cargo) {
                (s.set_acceptance)(spec, index, 0);
            }
        }
        id += 1;
    }
    let climates = (s.climate_mask)();
    for climate in 0..16 {
        if climates & (1 << climate) != 0 {
            for zone in 0..16 {
                if HZ_ZONE_ALL & (1 << zone) != 0 {
                    early_house(s, (1 << climate) | (1 << zone));
                }
            }
        }
    }
}
