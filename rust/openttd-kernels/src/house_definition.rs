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
use crate::bitmath_func::{gb, has_bit, set_bit64};
use crate::cargo_registry::{get_cargo_translation, get_cargo_type_by_label};
use crate::cargo_type::{
    CT_CANDY, CT_FIZZY_DRINKS, CT_FOOD, CT_GOODS, CT_INVALID, CargoLabel, INVALID_CARGO,
    is_valid_cargo_type,
};
use crate::debug::{Bytes, debug, grf_msg};
use crate::gfx_type::{COLOUR_BLUE, COLOUR_GREEN, COLOUR_ORANGE, COLOUR_RED};
use crate::house::{
    BuildingFlag, BuildingFlags, HOUSE_NUM_ACCEPTS, HOUSE_ORIGINAL_NUM_ACCEPTS, HouseExtraFlags,
    HouseZones, INVALID_HOUSE_ID, NEW_HOUSE_OFFSET, NUM_HOUSES, NUM_HOUSES_PER_GRF,
};
use crate::house_type::HouseID;
use crate::landscape_type::LandscapeType;
use crate::newgrf_animation_type::AnimationStatus;
use crate::newgrf_bytereader::{BadgeList, ByteReader, ReadError, Reader};
use crate::newgrf_callbacks::HouseCallbackMasks;
use crate::newgrf_internal::ChangeInfoResult;
use crate::timer_game_common::{MAX_YEAR, MIN_YEAR, ORIGINAL_BASE_YEAR};
use crate::{newgrf_loading, override_manager};
use std::ffi::c_void;
type Spec = *const c_void;
type File = *mut c_void;
#[repr(C)]
pub struct Identity {
    grfid: u32,
    pub(crate) local: u16,
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
pub struct Services {
    resolver: *const crate::town::resolver::ResolverServices,
    get: extern "C" fn(u16) -> Spec,
    count: extern "C" fn() -> usize,
    clear_specs: extern "C" fn(),
    reserve_specs: extern "C" fn(usize),
    append_default: extern "C" fn(u16),
    file_count: extern "C" fn() -> usize,
    file_at: extern "C" fn(usize) -> File,
    pub(crate) local_count: extern "C" fn(File) -> usize,
    resize_local: extern "C" fn(File, usize),
    pub(crate) local: extern "C" fn(File, usize) -> Spec,
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
    resize_specs: extern "C" fn(usize),
    move_spec: extern "C" fn(Spec, u16),
    set_override: extern "C" fn(Spec, u16),
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
    "OpenTTDHouseDefinitionServices" ["house_definition"] => Services { resolver, get, count, clear_specs, reserve_specs, append_default, file_count, file_at, local_count, resize_local, local, clear_local, copy_substitute, set_identity, identity, validity, part, flags, early, cargo, set_cargo, set_label, set_acceptance, set_colour, callbacks, watched, map_string, set_animation, badges, resize_specs, move_spec, set_override, set_enabled, set_flags, set_min_year, set_max_year, set_population, set_mail, set_rating, set_removal, set_availability, set_callbacks, set_processing, set_probability, set_extra, set_animation_speed, set_class_id, set_minimum_life, set_watched };
}

/// Ignore a house property, retaining its original byte consumption.
fn ignore_property(prop: i32, reader: &mut Reader<'_>) -> Result<ChangeInfoResult, ReadError> {
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
        _ => return Ok(ChangeInfoResult::CIR_UNKNOWN),
    }
    Ok(ChangeInfoResult::CIR_SUCCESS)
}
/// Define properties for the local house ID interval.
fn property(
    s: &Services,
    loading: &newgrf_loading::Services,
    file: File,
    first: u32,
    last: u32,
    prop: i32,
    reader: &mut Reader<'_>,
) -> Result<ChangeInfoResult, ReadError> {
    if last > u32::from(NUM_HOUSES_PER_GRF) {
        grf_msg!(
            1,
            "TownHouseChangeInfo: Too many houses loaded ({}), max ({}). Ignoring.",
            last,
            NUM_HOUSES_PER_GRF
        );
        unsafe {
            newgrf_loading::disable_invalid_id(loading);
        }
        return Ok(ChangeInfoResult::CIR_DISABLED);
    }
    // Allocate house specs if they have not been allocated already.
    if (s.local_count)(file) < last as usize {
        (s.resize_local)(file, last as usize);
    }
    let mut result = ChangeInfoResult::CIR_SUCCESS;
    for id in first..last {
        let spec = (s.local)(file, id as usize);
        if prop != 0x08 && spec.is_null() {
            // If property 08 is not yet set, ignore this property.
            result = result.max(ignore_property(prop, reader)?);
            continue;
        }
        match prop {
            // Substitute building type and definition of a new house.
            0x08 => {
                let substitute = reader.byte()?;
                if substitute == 0xff {
                    // A substitute ID of 0xFF disables the old house with the current ID
                    // instead of defining a new house.
                    if id < u32::from(NEW_HOUSE_OFFSET) {
                        (s.set_enabled)((s.get)(id as HouseID), false);
                    }
                    continue;
                } else if u16::from(substitute) >= NEW_HOUSE_OFFSET {
                    // The substitute ID must be one of the original houses.
                    grf_msg!(
                        2,
                        "TownHouseChangeInfo: Attempt to use new house {} as substitute house for {}. Ignoring.",
                        substitute,
                        id
                    );
                    continue;
                }
                // Allocate space for this house.
                if spec.is_null() {
                    // Only the first property 08 setting copies properties; later changes
                    // leave the properties in place.
                    let source = (s.get)(u16::from(substitute));
                    let spec = (s.copy_substitute)(file, id as HouseID, source);
                    (s.set_enabled)(spec, true);
                    (s.set_identity)(spec, file, id as HouseID, HouseID::from(substitute));
                    // Default randomization colours, used if not overridden.
                    for (index, colour) in [COLOUR_RED, COLOUR_BLUE, COLOUR_ORANGE, COLOUR_GREEN]
                        .into_iter()
                        .enumerate()
                    {
                        (s.set_colour)(spec, index as u8, colour);
                    }
                    // House flags 40 and 80 are exceptions: never set them automatically.
                    let flags = BuildingFlags((s.flags)(spec));
                    (s.set_flags)(
                        spec,
                        flags
                            .without(
                                BuildingFlags::from_element(BuildingFlag::IsChurch)
                                    .with(BuildingFlags::from_element(BuildingFlag::IsStadium)),
                            )
                            .0,
                    );
                    // Ensure the third cargo type is valid in this climate. Copying a
                    // food-accepting house can otherwise cause problems in temperate games.
                    let cargo = (s.cargo)(spec, 2);
                    let cargo = if is_valid_cargo_type(cargo.cargo) {
                        cargo.cargo
                    } else {
                        get_cargo_type_by_label(CargoLabel(cargo.label)).0
                    };
                    if !is_valid_cargo_type(cargo) {
                        (s.set_acceptance)(spec, 2, 0);
                    }
                }
            }
            // Building flags.
            0x09 => (s.set_flags)(spec, BuildingFlags(reader.byte()?).0),
            // Availability years.
            0x0a => {
                let years = reader.word()?;
                let year = |v: u32| {
                    if v > 150 {
                        MAX_YEAR
                    } else {
                        ORIGINAL_BASE_YEAR + v as i32
                    }
                };
                (s.set_min_year)(spec, year(gb(u32::from(years), 0, 8)));
                (s.set_max_year)(spec, year(gb(u32::from(years), 8, 8)));
            }
            // Population.
            0x0b => (s.set_population)(spec, reader.byte()?),
            // Mail generation multiplier.
            0x0c => (s.set_mail)(spec, reader.byte()?),
            // Passenger acceptance and mail acceptance.
            0x0d | 0x0e => (s.set_acceptance)(spec, (prop - 0x0d) as u8, reader.byte()?),
            // Goods/candy, food/fizzy drinks acceptance.
            0x0f => {
                let mut goods = i16::from(reader.byte()? as i8);
                // Negative goods acceptance means food, or fizzy drinks in toyland.
                // Nonnegative values mean goods, or candy in toyland.
                let toyland =
                    LandscapeType(unsafe { (*s.resolver).shared().settings().climate.read() })
                        == LandscapeType::Toyland;
                let label = if goods >= 0 {
                    if toyland { CT_CANDY } else { CT_GOODS }
                } else if toyland {
                    CT_FIZZY_DRINKS
                } else {
                    CT_FOOD
                };
                let cargo = get_cargo_type_by_label(CargoLabel(label)).0;
                // Make sure the cargo type is valid in this climate.
                if !is_valid_cargo_type(cargo) {
                    goods = 0;
                }
                (s.set_cargo)(spec, 2, cargo);
                (s.set_label)(spec, 2, CT_INVALID);
                // Acceptance needs the positive value.
                (s.set_acceptance)(spec, 2, goods.unsigned_abs() as u8);
            }
            // Local authority rating decrease on removal.
            0x10 => (s.set_rating)(spec, reader.word()?),
            // Removal cost multiplier.
            0x11 => (s.set_removal)(spec, reader.byte()?),
            // Building name ID.
            0x12 => (s.map_string)(spec, reader.word()?),
            // Building availability mask.
            0x13 => (s.set_availability)(spec, HouseZones(reader.word()?).0),
            // House callback mask.
            0x14 => {
                let mut mask = u32::from(HouseCallbackMasks((s.callbacks)(spec)).0);
                crate::bitmath_func::sb(&mut mask, 0, 8, u32::from(reader.byte()?));
                (s.set_callbacks)(spec, HouseCallbackMasks(mask as u16).0);
            }
            // House override byte.
            0x15 => {
                let target = reader.byte()?;
                // The house being overridden must be an original house.
                if u16::from(target) >= NEW_HOUSE_OFFSET {
                    grf_msg!(
                        2,
                        "TownHouseChangeInfo: Attempt to override new house {} with house id {}. Ignoring.",
                        target,
                        id
                    );
                    continue;
                }
                unsafe {
                    override_manager::openttd_rust_override_add(
                        override_manager::house(),
                        id as HouseID,
                        ((*s.resolver).shared().grf_file_id)(file),
                        u32::from(target),
                    );
                }
            }
            // Periodic refresh multiplier.
            0x16 => (s.set_processing)(spec, reader.byte()?.min(63)),
            // Four random colours to use.
            0x17 => {
                for index in 0..4 {
                    (s.set_colour)(spec, index, gb(u32::from(reader.byte()?), 0, 4) as u8);
                }
            }
            // Relative probability of appearing.
            0x18 => (s.set_probability)(spec, reader.byte()?),
            // Extra flags.
            0x19 => (s.set_extra)(spec, HouseExtraFlags(reader.byte()?).0),
            // Animation frames.
            0x1a => {
                let info = reader.byte()?;
                (s.set_animation)(
                    spec,
                    gb(u32::from(info), 0, 7) as u8,
                    if has_bit(info, 7) {
                        AnimationStatus::Looping.0
                    } else {
                        AnimationStatus::NonLooping.0
                    },
                );
            }
            // Animation speed.
            0x1b => (s.set_animation_speed)(spec, reader.byte()?.clamp(2, 16)),
            // Class of the building type.
            0x1c => {
                let class = crate::town::resolver::allocate_house_class_id(
                    reader.byte()?,
                    (unsafe { &*s.resolver }.shared().grf_file_id)(file),
                );
                (s.set_class_id)(spec, class);
            }
            // Callback mask part 2.
            0x1d => {
                let mut mask = u32::from(HouseCallbackMasks((s.callbacks)(spec)).0);
                crate::bitmath_func::sb(&mut mask, 8, 8, u32::from(reader.byte()?));
                (s.set_callbacks)(spec, HouseCallbackMasks(mask as u16).0);
            }
            // Accepted cargo types.
            0x1e => {
                let cargos = reader.dword()?;
                // Check whether the cargo types should stay unchanged.
                if cargos == u32::MAX {
                    continue;
                }
                for index in 0..HOUSE_ORIGINAL_NUM_ACCEPTS {
                    // Get the cargo number from the list.
                    let cargo =
                        get_cargo_translation(gb(cargos, 8 * index, 8) as u8, file, false).0;
                    if is_valid_cargo_type(cargo) {
                        (s.set_cargo)(spec, index, cargo);
                    } else {
                        // Disable acceptance of an invalid cargo type.
                        (s.set_acceptance)(spec, index, 0);
                    }
                    (s.set_label)(spec, index, CT_INVALID);
                }
            }
            // Minimum life span.
            0x1f => (s.set_minimum_life)(spec, reader.byte()?),
            // Cargo acceptance watch list.
            0x20 => {
                let count = reader.byte()?;
                for _ in 0..count {
                    let cargo = get_cargo_translation(reader.byte()?, file, false).0;
                    if is_valid_cargo_type(cargo) {
                        (s.set_watched)(spec, set_bit64(&mut (s.watched)(spec), cargo));
                    }
                }
            }
            // Long introduction year.
            0x21 => (s.set_min_year)(spec, i32::from(reader.word()?)),
            // Long maximum year.
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
            // Variable length cargo types accepted.
            0x23 => {
                let count = reader.byte()?;
                if count > HOUSE_NUM_ACCEPTS {
                    unsafe {
                        newgrf_loading::disable_list_property(loading, prop as u32);
                    }
                    return Ok(ChangeInfoResult::CIR_DISABLED);
                }
                // Always write the full accepts_cargo array, checking each index against
                // the provided data to initialize all values and avoid array overruns.
                for index in 0..HOUSE_NUM_ACCEPTS {
                    if index < count {
                        (s.set_cargo)(
                            spec,
                            index,
                            get_cargo_translation(reader.byte()?, file, false).0,
                        );
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
            // Badge list.
            0x24 => unsafe {
                reader.badges(crate::newgrf::GSF_HOUSES, (s.badges)(spec))?;
            },
            _ => result = ChangeInfoResult::CIR_UNKNOWN,
        }
    }
    Ok(result)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_definition_property(
    s: *const Services,
    reads: *const crate::newgrf_bytereader::Services,
    loading: *const newgrf_loading::Services,
    file: File,
    first: u32,
    last: u32,
    prop: i32,
    native: *mut ByteReader,
) -> u8 {
    let mut reader = unsafe { Reader::new(native, &*reads) };
    property(
        unsafe { &*s },
        unsafe { &*loading },
        file,
        first,
        last,
        prop,
        &mut reader,
    )
    .map_or(u8::MAX, |result| result.0)
}

/// Check a house spec and disable it when invalid; following specs validate multitile houses.
fn valid(s: &Services, spec: Spec, next: [Spec; 3], file: File) -> bool {
    let info = (s.validity)(spec);
    let flags = BuildingFlags(info.flags);
    let two = flags.any(BuildingFlags::BUILDING_HAS_2_TILES);
    let four = flags.any(BuildingFlags::BUILDING_HAS_4_TILES);
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
        part.as_ref().is_none_or(|info| {
            !info.enabled || BuildingFlags(info.flags).any(BuildingFlags::BUILDING_HAS_1_TILE)
        })
    };
    // The original passes an empty name for files without one, and reports only nonempty names.
    let filename = if file.is_null() {
        &[][..]
    } else {
        unsafe { ((*s.resolver).shared().grf_file_name)(file.cast()).bytes() }
    };
    let report = !filename.is_empty();
    if (two && missing(&next1)) || (four && (missing(&next2) || missing(&next3))) {
        (s.set_enabled)(spec, false);
        if report {
            debug!(
                grf,
                1,
                "FinaliseHouseArray: {} defines house {} as multitile, but no suitable tiles follow. Disabling house.",
                Bytes(filename),
                (s.identity)(spec).local
            );
        }
        return false;
    }
    // Some places count population only on north tiles, others on all tiles, causing
    // desyncs. NewGRF requires zero population on non-north tiles, so disable offenders.
    // Allowing nonzero population would require summing all tiles everywhere.
    if (two && next1.as_ref().is_some_and(|info| info.population != 0))
        || (four
            && (next2.as_ref().is_some_and(|info| info.population != 0)
                || next3.as_ref().is_some_and(|info| info.population != 0)))
    {
        (s.set_enabled)(spec, false);
        if report {
            debug!(
                grf,
                1,
                "FinaliseHouseArray: {} defines multitile house {} with non-zero population on additional tiles. Disabling house.",
                Bytes(filename),
                (s.identity)(spec).local
            );
        }
        return false;
    }
    // The substitute type is also used for overrides; a different size causes crashes.
    // Check only NewGRF houses, because original houses have no substitute ID set.
    if report
        && (flags & BuildingFlags::BUILDING_HAS_1_TILE)
            != (BuildingFlags((s.flags)((s.get)(info.substitute)))
                & BuildingFlags::BUILDING_HAS_1_TILE)
    {
        (s.set_enabled)(spec, false);
        debug!(
            grf,
            1,
            "FinaliseHouseArray: {} defines house {} with different house size then it's substitute type. Disabling house.",
            Bytes(filename),
            (s.identity)(spec).local
        );
        return false;
    }
    // Additional parts of multitile houses must not be available.
    if !flags.any(BuildingFlags::BUILDING_HAS_1_TILE)
        && HouseZones(info.availability).any(HouseZones::HZ_ZONE_ALL)
        && HouseZones(info.availability).any(HouseZones::HZ_CLIMATE_ALL)
    {
        (s.set_enabled)(spec, false);
        if report {
            debug!(
                grf,
                1,
                "FinaliseHouseArray: {} defines house {} without a size but marked it as available. Disabling house.",
                Bytes(filename),
                (s.identity)(spec).local
            );
        }
        return false;
    }
    true
}
/// Ensure a house is available in year 0 for exactly one climate and one town zone.
fn early_house(s: &Services, mask: HouseZones) {
    let mut min_year = MAX_YEAR;
    for id in 0..(s.count)() {
        let info = (s.early)((s.get)(id as HouseID));
        if !info.enabled || !HouseZones(info.availability).all(mask) {
            continue;
        }
        min_year = min_year.min(info.year);
    }
    if min_year == 0 {
        return;
    }
    for id in 0..(s.count)() {
        let spec = (s.get)(id as HouseID);
        let info = (s.early)(spec);
        if !info.enabled || !HouseZones(info.availability).all(mask) {
            continue;
        }
        if info.year == min_year {
            (s.set_min_year)(spec, MIN_YEAR);
        }
    }
}
/// Reset and initialize house specs.
pub fn reset(s: &Services) {
    (s.clear_specs)();
    (s.reserve_specs)(usize::from(NEW_HOUSE_OFFSET));
    unsafe {
        crate::town::resolver::openttd_rust_house_resolver_exchange(s.resolver);
    }
    crate::town::resolver::reset_house_classes();
    // Copy default houses.
    for id in 0..NEW_HOUSE_OFFSET {
        (s.append_default)(id);
    }
    // Reset any overrides that have been set.
    unsafe {
        override_manager::reset_override(override_manager::house());
    }
}
pub fn publish(s: &Services, spec: Spec) {
    let manager = override_manager::house();
    let identity = (s.identity)(spec);
    let id = unsafe {
        override_manager::allocate_house(
            manager,
            identity.local,
            identity.grfid,
            identity.substitute,
        )
    };
    if id == INVALID_HOUSE_ID {
        grf_msg!(
            1,
            "House.SetEntitySpec: Too many houses allocated. Ignoring."
        );
        return;
    }
    if usize::from(id) >= (s.count)() {
        (s.resize_specs)(usize::from(id) + 1);
    }
    (s.move_spec)(spec, id);
    for original in 0..NEW_HOUSE_OFFSET {
        let overridden = (s.get)(original);
        let pending = unsafe { override_manager::pending_override(manager, original) };
        if pending != (identity.local, identity.grfid) {
            continue;
        }
        (s.set_override)(overridden, id);
        unsafe {
            override_manager::clear_override(manager, original);
        }
    }
}
/// Add new houses only after all properties in their GRF file have been loaded,
/// and check dates to emulate `TTDPatch`'s early-year behavior.
pub fn finalise(s: &Services) {
    // TTDPatch pretends a pre-1930 date is 1930 when no houses start before then.
    // Rather than hard-code 1930, reset houses with the lowest introduction date
    // to year 0; leave later dates alone when earlier houses have been defined.
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
        // These local specs will not be used again.
        (s.clear_local)(file);
    }
    let mut id = 0;
    while id < (s.count)() {
        let spec = (s.get)(id as HouseID);
        let next = [1, 2, 3].map(|offset| {
            if id + offset < usize::from(NUM_HOUSES) {
                (s.get)((id + offset) as HouseID)
            } else {
                std::ptr::null()
            }
        });
        // Recheck all houses to ensure multitile parts have consecutive IDs and none
        // are missing.
        if !valid(s, spec, next, std::ptr::null_mut()) {
            // GetHouseNorthPart checks the three preceding houses for multitile flags.
            // Disabled invalid houses must not influence valid following tiles: clear
            // their flags so the following house is handled correctly as a 1x1 house.
            (s.set_flags)(spec, BuildingFlags(0).0);
        }
        // Apply the default cargo translation map for unset cargo slots.
        for index in 0..HOUSE_ORIGINAL_NUM_ACCEPTS {
            let info = (s.cargo)(spec, index);
            let cargo = if is_valid_cargo_type(info.cargo) {
                info.cargo
            } else {
                let cargo = get_cargo_type_by_label(CargoLabel(info.label)).0;
                (s.set_cargo)(spec, index, cargo);
                cargo
            };
            // Disable acceptance if the cargo type is invalid.
            if !is_valid_cargo_type(cargo) {
                (s.set_acceptance)(spec, index, 0);
            }
        }
        id += 1;
    }
    let climate_mask = crate::house::get_climate_mask_for_landscape(LandscapeType(unsafe {
        (*s.resolver).shared().settings().climate.read()
    }));
    for climate in climate_mask.iter() {
        for zone in HouseZones::HZ_ZONE_ALL.iter() {
            early_house(
                s,
                HouseZones::from_element(climate).with(HouseZones::from_element(zone)),
            );
        }
    }
}
