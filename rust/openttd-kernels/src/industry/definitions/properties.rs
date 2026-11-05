/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Action0 industry and tile property interpretation, including incremental failure effects.
use super::Services;
use crate::industrytype::{self, IndustrySpec, IndustryTileSpec, LayoutTile, NativeVec};
use crate::bitmath_func::{gb, sb};
use crate::cargo_type::INVALID_CARGO;
use crate::debug::grf_msg;
use crate::industry_map::GFX_WATERTILE_SPECIALCHECK;
use crate::industry_type::{
    INDUSTRY_NUM_INPUTS, INDUSTRY_NUM_OUTPUTS, INDUSTRY_ORIGINAL_NUM_INPUTS,
    INDUSTRY_ORIGINAL_NUM_OUTPUTS, INVALID_INDUSTRYTILE, NEW_INDUSTRYOFFSET,
    NEW_INDUSTRYTILEOFFSET, NUM_INDUSTRYTILES_PER_GRF, NUM_INDUSTRYTYPES_PER_GRF,
};
use crate::newgrf_bytereader::{ByteReader, ReadError, Reader, Services as ReaderServices};
use crate::newgrf_internal::{
    CIR_DISABLED as DISABLED, CIR_INVALID_ID as INVALID_ID, CIR_SUCCESS as SUCCESS,
    CIR_UNKNOWN as UNKNOWN,
};
use crate::{newgrf, newgrf_loading, override_manager};
use std::{ffi::c_void, ptr};
#[repr(C)]
pub struct Context {
    pub file: *mut c_void,
    pub grfid: u32,
    pub version: u8,
    pub climate: u8,
}
/// `CT_INVALID` as a default cargo label.
const INVALID_LABEL: crate::newgrf::CargoLabel = crate::newgrf::CargoLabel {
    value: u32::MAX,
    mixed: false,
};
/// `INDUSTRYTILE_NOANIM`.
const INDUSTRYTILE_NOANIM: u8 = 0xff;
/// `CheckProc::CHECK_NOTHING`.
const CHECK_NOTHING: u8 = 0;
/// `STR_NULL`.
const STR_NULL: u32 = 0;
/// `GRFFilePropsBase::SetGRFFile` with the context's non-null file.
fn set_file(props: &mut industrytype::SubstituteProps, c: &Context) {
    props.grffile = c.file;
    props.grfid = c.grfid;
}
/// `GRFFile::industryspec[id]`, or null.
fn file_industry(file: *const c_void, id: u32) -> *mut IndustrySpec {
    unsafe { industrytype::file_specs(file) }.industries[id as usize]
        .as_deref_mut()
        .map_or(ptr::null_mut(), ptr::from_mut)
}
/// `GRFFile::indtspec[id]`, or null.
fn file_tile(file: *const c_void, id: u32) -> *mut IndustryTileSpec {
    unsafe { industrytype::file_specs(file) }.tiles[id as usize]
        .as_deref_mut()
        .map_or(ptr::null_mut(), ptr::from_mut)
}
fn ignore_tile(prop: i32, r: &mut Reader<'_>) -> Result<i32, ReadError> {
    match prop {
        0x09 | 0x0d | 0x0e | 0x10..=0x12 => {
            r.byte()?;
        }
        0x0a..=0x0c | 0x0f => {
            r.word()?;
        }
        0x13 => {
            let count = r.byte()?;
            r.skip(usize::from(count) * 2)?;
        }
        _ => return Ok(UNKNOWN),
    }
    Ok(SUCCESS)
}
fn ignore_industry(prop: i32, r: &mut Reader<'_>) -> Result<i32, ReadError> {
    match prop {
        0x09 | 0x0b | 0x0f | 0x12..=0x14 | 0x17..=0x19 | 0x21 | 0x22 => {
            r.byte()?;
        }
        0x0c..=0x0e | 0x10 | 0x1b | 0x1f | 0x24 => {
            r.word()?;
        }
        0x11 | 0x1a | 0x1c..=0x1e | 0x20 | 0x23 => {
            r.dword()?;
        }
        0x0a => {
            let count = r.byte()?;
            for _ in 0..count {
                let mut k = 0_u32;
                loop {
                    let x = r.byte()?;
                    if x == 0xfe && k == 0 {
                        r.byte()?;
                        r.byte()?;
                        break;
                    }
                    let y = r.byte()?;
                    if x == 0 && y == 0x80 {
                        break;
                    }
                    if r.byte()? == 0xfe {
                        r.word()?;
                    }
                    k = k.wrapping_add(1);
                }
            }
        }
        0x16 => {
            for _ in 0..INDUSTRY_ORIGINAL_NUM_INPUTS {
                r.byte()?;
            }
        }
        0x15 | 0x25..=0x27 => {
            let count = r.byte()?;
            r.skip(usize::from(count))?;
        }
        0x28 => {
            let inputs = r.byte()?;
            let outputs = r.byte()?;
            r.skip(usize::from(inputs) * usize::from(outputs) * 2)?;
        }
        0x29 => r.skip_badges()?,
        _ => return Ok(UNKNOWN),
    }
    Ok(SUCCESS)
}
fn valid_layout(layout: &[LayoutTile]) -> bool {
    if layout.is_empty() {
        return false;
    }
    for i in 0..layout.len() - 1 {
        for j in i + 1..layout.len() {
            if layout[i].ti.x == layout[j].ti.x && layout[i].ti.y == layout[j].ti.y {
                return false;
            }
        }
    }
    layout
        .iter()
        .any(|tile| tile.gfx != GFX_WATERTILE_SPECIALCHECK)
}
unsafe fn layouts(
    id: u32,
    spec: *mut IndustrySpec,
    c: &Context,
    r: &mut Reader<'_>,
    s: &Services,
    loading: &newgrf_loading::Services,
) -> Result<i32, ReadError> {
    let count = r.byte()?;
    let mut definition_size = r.dword()?;
    let mut bytes_read = 0_u32;
    let mut new_layouts = Vec::new();
    let mut layout = Vec::new();
    for _ in 0..count {
        layout.clear();
        let mut k = 0_u32;
        loop {
            if bytes_read >= definition_size {
                grf_msg!(
                    3,
                    "IndustriesChangeInfo: Incorrect size for industry tile layout definition for industry {}.",
                    id
                );
                definition_size = u32::MAX;
            }
            let mut tile = LayoutTile {
                ti: crate::map_type::TileIndexDiffC {
                    x: i16::from(r.byte()?),
                    y: 0,
                },
                gfx: 0,
            };
            bytes_read = bytes_read.wrapping_add(1);
            if tile.ti.x == 0xfe && k == 0 {
                let kind = r.byte()?;
                let index = r.byte()?;
                bytes_read = bytes_read.wrapping_add(2);
                if u16::from(kind) >= NEW_INDUSTRYOFFSET {
                    unsafe {
                        grf_msg!(
                            1,
                            "IndustriesChangeInfo: Invalid original industry number for layout import, industry {}",
                            id
                        );
                        newgrf_loading::disable_invalid_id(loading);
                    }
                    return Ok(DISABLED);
                }
                let originals = unsafe { (*s.origin.add(usize::from(kind))).layouts.as_slice() };
                if usize::from(index) >= originals.len() {
                    unsafe {
                        grf_msg!(
                            1,
                            "IndustriesChangeInfo: Invalid original industry layout index for layout import, industry {}",
                            id
                        );
                        newgrf_loading::disable_invalid_id(loading);
                    }
                    return Ok(DISABLED);
                }
                layout.extend_from_slice(originals[usize::from(index)].as_slice());
                break;
            }
            tile.ti.y = i16::from(r.byte()?);
            bytes_read = bytes_read.wrapping_add(1);
            if tile.ti.x == 0 && tile.ti.y == 0x80 {
                break;
            }
            tile.gfx = u16::from(r.byte()?);
            bytes_read = bytes_read.wrapping_add(1);
            if tile.gfx == 0xfe {
                let local = r.word()?;
                bytes_read = bytes_read.wrapping_add(2);
                let mapped =
                    unsafe { override_manager::get(override_manager::tile(), local, c.grfid) };
                if mapped == INVALID_INDUSTRYTILE {
                    grf_msg!(
                        2,
                        "IndustriesChangeInfo: Attempt to use industry tile {} with industry id {}, not yet defined. Ignoring.",
                        local,
                        id
                    );
                } else {
                    tile.gfx = mapped;
                }
            } else if tile.gfx == GFX_WATERTILE_SPECIALCHECK {
                tile.ti.x = i16::from(gb(tile.ti.x as u32, 0, 8) as i8);
                tile.ti.y = i16::from(gb(tile.ti.y as u32, 0, 8) as i8);
                // Pre-v8 TileIndexDiff shifts a negative x into the y coordinate.
                if c.version < 8 && tile.ti.x < 0 {
                    tile.ti.y = tile.ti.y.wrapping_add(1);
                }
            }
            layout.push(tile);
            k = k.wrapping_add(1);
        }
        if valid_layout(&layout) {
            new_layouts.push(NativeVec::from_vec(layout.clone()));
        } else {
            grf_msg!(
                1,
                "IndustriesChangeInfo: Invalid industry layout for industry id {}. Ignoring",
                id
            );
        }
    }
    unsafe { (*spec).layouts = NativeVec::from_vec(new_layouts) };
    Ok(SUCCESS)
}
unsafe fn tiles(
    ids: std::ops::Range<u32>,
    prop: i32,
    c: &Context,
    r: &mut Reader<'_>,
    loading: &newgrf_loading::Services,
) -> Result<i32, ReadError> {
    let last = ids.end;
    if last > u32::from(NUM_INDUSTRYTILES_PER_GRF) {
        grf_msg!(
            1,
            "IndustryTilesChangeInfo: Too many industry tiles loaded ({}), max ({}). Ignoring.",
            last,
            NUM_INDUSTRYTILES_PER_GRF
        );
        return Ok(INVALID_ID);
    }
    let specs = unsafe { industrytype::file_specs(c.file) };
    if specs.tiles.len() < last as usize {
        specs.tiles.resize_with(last as usize, || None);
    }
    let mut result = SUCCESS;
    for id in ids {
        let mut spec = file_tile(c.file, id);
        if prop != 0x08 && spec.is_null() {
            result = result.max(ignore_tile(prop, r)?);
            continue;
        }
        match prop {
            0x08 => {
                let substitute = r.byte()?;
                if u16::from(substitute) >= NEW_INDUSTRYTILEOFFSET {
                    grf_msg!(
                        2,
                        "IndustryTilesChangeInfo: Attempt to use new industry tile {} as substitute industry tile for {}. Ignoring.",
                        substitute,
                        id
                    );
                    continue;
                }
                if spec.is_null() {
                    // A copy of the current, not the original, substitute tile.
                    let mut tile = unsafe { (*industrytype::tile(u16::from(substitute))).clone() };
                    tile.enabled = true;
                    tile.anim_production = INDUSTRYTILE_NOANIM;
                    tile.anim_next = INDUSTRYTILE_NOANIM;
                    tile.grf_prop.local_id = id as u16;
                    tile.grf_prop.subst_id = u16::from(substitute);
                    set_file(&mut tile.grf_prop, c);
                    unsafe { industrytype::file_specs(c.file) }.tiles[id as usize] =
                        Some(Box::new(tile));
                    spec = file_tile(c.file, id);
                    unsafe {
                        super::allocate_tile(id as u16, c.grfid, u16::from(substitute));
                    }
                }
            }
            0x09 => {
                let override_id = r.byte()?;
                if u16::from(override_id) >= NEW_INDUSTRYTILEOFFSET {
                    grf_msg!(
                        2,
                        "IndustryTilesChangeInfo: Attempt to override new industry tile {} with industry tile id {}. Ignoring.",
                        override_id,
                        id
                    );
                    continue;
                }
                unsafe {
                    override_manager::openttd_rust_override_add(
                        override_manager::tile(),
                        id as u16,
                        c.grfid,
                        u32::from(override_id),
                    );
                }
            }
            0x0a..=0x0c => {
                let value = r.word()?;
                let slot = (prop - 0x0a) as usize;
                let cargo = crate::cargo_registry::openttd_rust_cargo_translate(
                    gb(u32::from(value), 0, 8) as u8,
                    c.file,
                    false,
                );
                unsafe {
                    (*spec).accepts_cargo[slot] = cargo;
                    (*spec).acceptance[slot] = gb(u32::from(value), 8, 8).clamp(0, 16) as i8;
                    (*spec).accepts_cargo_label[slot] = INVALID_LABEL;
                }
            }
            0x0d => unsafe {
                (*spec).slopes_refused = r.byte()?;
            },
            0x0e => unsafe {
                (*spec).callback_mask = r.byte()?;
            },
            0x0f => unsafe {
                (*spec).animation.frames = r.byte()?;
                (*spec).animation.status = r.byte()?;
            },
            0x10 => unsafe {
                (*spec).animation.speed = r.byte()?;
            },
            0x11 => unsafe {
                (*spec).animation.triggers = r.byte()?;
            },
            0x12 => unsafe {
                (*spec).special_flags = r.byte()?;
            },
            0x13 => {
                let count = usize::from(r.byte()?);
                if count > INDUSTRY_NUM_INPUTS {
                    unsafe {
                        newgrf_loading::disable_list_property(loading, prop as u32);
                    }
                    return Ok(DISABLED);
                }
                for i in 0..INDUSTRY_NUM_INPUTS {
                    if i < count {
                        let cargo = crate::cargo_registry::openttd_rust_cargo_translate(
                            r.byte()?,
                            c.file,
                            false,
                        );
                        unsafe { (*spec).accepts_cargo[i] = cargo };
                        let acceptance = r.byte()? as i8;
                        unsafe { (*spec).acceptance[i] = acceptance };
                    } else {
                        unsafe {
                            (*spec).accepts_cargo[i] = INVALID_CARGO;
                            (*spec).acceptance[i] = 0;
                        }
                    }
                    if i < INDUSTRY_ORIGINAL_NUM_INPUTS {
                        unsafe { (*spec).accepts_cargo_label[i] = INVALID_LABEL };
                    }
                }
            }
            0x14 => {
                unsafe { r.badge_ids(newgrf::GSF_INDUSTRYTILES, &raw mut (*spec).badges) }?;
            }
            _ => result = UNKNOWN,
        }
    }
    Ok(result)
}
unsafe fn industries(
    ids: std::ops::Range<u32>,
    prop: i32,
    c: &Context,
    r: &mut Reader<'_>,
    loading: &newgrf_loading::Services,
) -> Result<i32, ReadError> {
    let last = ids.end;
    if last > u32::from(NUM_INDUSTRYTYPES_PER_GRF) {
        grf_msg!(
            1,
            "IndustriesChangeInfo: Too many industries loaded ({}), max ({}). Ignoring.",
            last,
            NUM_INDUSTRYTYPES_PER_GRF
        );
        return Ok(INVALID_ID);
    }
    let s = unsafe { &*loading.industry };
    let specs = unsafe { industrytype::file_specs(c.file) };
    if specs.industries.len() < last as usize {
        specs.industries.resize_with(last as usize, || None);
    }
    let mut result = SUCCESS;
    for id in ids {
        let mut spec = file_industry(c.file, id);
        if prop != 0x08 && spec.is_null() {
            result = result.max(ignore_industry(prop, r)?);
            continue;
        }
        match prop {
            0x08 => {
                let substitute = r.byte()?;
                if substitute == 0xff {
                    // The original disables the global slot with the local ID.
                    unsafe { (*industrytype::industry(id as u16)).enabled = false };
                    continue;
                }
                if u16::from(substitute) >= NEW_INDUSTRYOFFSET {
                    grf_msg!(
                        2,
                        "_industry_specs: Attempt to use new industry {} as substitute industry for {}. Ignoring.",
                        substitute,
                        id
                    );
                    continue;
                }
                if spec.is_null() {
                    let mut industry =
                        unsafe { (*s.origin.add(usize::from(substitute))).clone() };
                    industry.enabled = true;
                    industry.grf_prop.local_id = id as u16;
                    industry.grf_prop.subst_id = u16::from(substitute);
                    set_file(&mut industry.grf_prop, c);
                    industry.check_proc = CHECK_NOTHING;
                    unsafe { industrytype::file_specs(c.file) }.industries[id as usize] =
                        Some(Box::new(industry));
                    spec = file_industry(c.file, id);
                }
            }
            0x09 => {
                let override_id = r.byte()?;
                if u16::from(override_id) >= NEW_INDUSTRYOFFSET {
                    grf_msg!(
                        2,
                        "IndustriesChangeInfo: Attempt to override new industry {} with industry id {}. Ignoring.",
                        override_id,
                        id
                    );
                    continue;
                }
                unsafe {
                    (*spec).grf_prop.override_id = u16::from(override_id);
                    override_manager::openttd_rust_override_add(
                        override_manager::industry(),
                        id as u16,
                        c.grfid,
                        u32::from(override_id),
                    );
                }
            }
            0x0a => {
                let status = unsafe { layouts(id, spec, c, r, s, loading) }?;
                if status == DISABLED {
                    return Ok(status);
                }
            }
            0x0b => unsafe {
                (*spec).life_type = r.byte()?;
            },
            0x0c => unsafe {
                (s.map_string)(r.word()?, &raw mut (*spec).closure_text);
            },
            0x0d => unsafe {
                (s.map_string)(r.word()?, &raw mut (*spec).production_up_text);
            },
            0x0e => unsafe {
                (s.map_string)(r.word()?, &raw mut (*spec).production_down_text);
            },
            0x0f => unsafe {
                (*spec).cost_multiplier = r.byte()?;
            },
            0x10 | 0x11 => {
                let produced = prop == 0x10;
                let count = if produced {
                    INDUSTRY_ORIGINAL_NUM_OUTPUTS
                } else {
                    INDUSTRY_ORIGINAL_NUM_INPUTS
                };
                for i in 0..count {
                    let cargo = crate::cargo_registry::openttd_rust_cargo_translate(
                        r.byte()?,
                        c.file,
                        false,
                    );
                    if produced {
                        unsafe {
                            (*spec).produced_cargo[i] = cargo;
                            (*spec).produced_cargo_label[i] = INVALID_LABEL;
                        }
                    } else {
                        unsafe {
                            (*spec).accepts_cargo[i] = cargo;
                            (*spec).accepts_cargo_label[i] = INVALID_LABEL;
                        }
                    }
                }
                if !produced {
                    r.byte()?;
                }
            }
            0x12 | 0x13 => unsafe {
                (*spec).production_rate[(prop - 0x12) as usize] = r.byte()?;
            },
            0x14 => unsafe {
                (*spec).minimal_cargo = r.byte()?;
            },
            0x15 => {
                let count = r.byte()?;
                let mut sounds = Vec::with_capacity(usize::from(count));
                for _ in 0..count {
                    sounds.push(r.byte()?);
                }
                unsafe { (*spec).random_sounds = NativeVec::from_vec(sounds) };
            }
            0x16 => {
                for i in 0..3 {
                    unsafe { (*spec).conflicting[i] = r.byte()? };
                }
            }
            0x17 => unsafe {
                (*spec).appear_creation[usize::from(c.climate)] = r.byte()?;
            },
            0x18 => unsafe {
                (*spec).appear_ingame[usize::from(c.climate)] = r.byte()?;
            },
            0x19 => unsafe {
                (*spec).map_colour = r.byte()?;
            },
            0x1a => unsafe {
                (*spec).behaviour = r.dword()?;
            },
            0x1b => unsafe {
                (s.map_string)(r.word()?, &raw mut (*spec).new_industry_text);
            },
            0x1c..=0x1e => {
                let multiples = r.dword()?;
                let input = (prop - 0x1c) as usize;
                unsafe {
                    (*spec).input_cargo_multiplier[input][0] = gb(multiples, 0, 16) as u16;
                    (*spec).input_cargo_multiplier[input][1] = gb(multiples, 16, 16) as u16;
                }
            }
            0x1f => unsafe {
                (s.map_string)(r.word()?, &raw mut (*spec).name);
            },
            0x20 => unsafe {
                (*spec).prospecting_chance = r.dword()?;
            },
            0x21 | 0x22 => {
                let mut mask = u32::from(unsafe { (*spec).callback_mask });
                sb(
                    &mut mask,
                    ((prop - 0x21) * 8) as u8,
                    8,
                    u32::from(r.byte()?),
                );
                unsafe { (*spec).callback_mask = mask as u16 };
            }
            0x23 => unsafe {
                (*spec).removal_cost_multiplier = r.dword()?;
            },
            0x24 => {
                let string = r.word()?;
                if string == 0 {
                    unsafe { (*spec).station_name = STR_NULL };
                } else {
                    unsafe { (s.map_string)(string, &raw mut (*spec).station_name) };
                }
            }
            0x25 | 0x26 => {
                let produced = prop == 0x25;
                let count = usize::from(r.byte()?);
                let maximum = if produced {
                    INDUSTRY_NUM_OUTPUTS
                } else {
                    INDUSTRY_NUM_INPUTS
                };
                if count > maximum {
                    unsafe {
                        newgrf_loading::disable_list_property(loading, prop as u32);
                    }
                    return Ok(DISABLED);
                }
                let labels = if produced {
                    INDUSTRY_ORIGINAL_NUM_OUTPUTS
                } else {
                    INDUSTRY_ORIGINAL_NUM_INPUTS
                };
                for i in 0..maximum {
                    let cargo = if i < count {
                        crate::cargo_registry::openttd_rust_cargo_translate(
                            r.byte()?,
                            c.file,
                            false,
                        )
                    } else {
                        INVALID_CARGO
                    };
                    if produced {
                        unsafe { (*spec).produced_cargo[i] = cargo };
                    } else {
                        unsafe { (*spec).accepts_cargo[i] = cargo };
                    }
                    if i < labels {
                        if produced {
                            unsafe { (*spec).produced_cargo_label[i] = INVALID_LABEL };
                        } else {
                            unsafe { (*spec).accepts_cargo_label[i] = INVALID_LABEL };
                        }
                    }
                }
            }
            0x27 => {
                let count = usize::from(r.byte()?);
                if count > INDUSTRY_NUM_OUTPUTS {
                    unsafe {
                        newgrf_loading::disable_list_property(loading, prop as u32);
                    }
                    return Ok(DISABLED);
                }
                for i in 0..INDUSTRY_NUM_OUTPUTS {
                    let rate = if i < count { r.byte()? } else { 0 };
                    unsafe { (*spec).production_rate[i] = rate };
                }
            }
            0x28 => {
                let inputs = usize::from(r.byte()?);
                let outputs = usize::from(r.byte()?);
                if inputs > INDUSTRY_NUM_INPUTS || outputs > INDUSTRY_NUM_OUTPUTS {
                    unsafe {
                        newgrf_loading::disable_list_property(loading, prop as u32);
                    }
                    return Ok(DISABLED);
                }
                for i in 0..INDUSTRY_NUM_INPUTS {
                    for j in 0..INDUSTRY_NUM_OUTPUTS {
                        let multiplier = if i < inputs && j < outputs {
                            r.word()?
                        } else {
                            0
                        };
                        unsafe { (*spec).input_cargo_multiplier[i][j] = multiplier };
                    }
                }
            }
            0x29 => {
                unsafe { r.badge_ids(newgrf::GSF_INDUSTRIES, &raw mut (*spec).badges) }?;
            }
            _ => result = UNKNOWN,
        }
    }
    Ok(result)
}
fn finish(status: &Result<i32, ReadError>, result: *mut i32) -> bool {
    match status {
        Ok(value) => {
            unsafe {
                ptr::write(result, *value);
            }
            true
        }
        Err(ReadError) => false,
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_properties_abi(
    first: u32,
    last: u32,
    prop: i32,
    native: *mut ByteReader,
    c: *const Context,
    reads: *const ReaderServices,
    loading: *const newgrf_loading::Services,
    result: *mut i32,
) -> u8 {
    let (c, reads, loading) = unsafe { (&*c, &*reads, &*loading) };
    let mut reader = unsafe { Reader::new(native, reads) };
    u8::from(finish(
        &unsafe { industries(first..last, prop, c, &mut reader, loading) },
        result,
    ))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_tile_properties_abi(
    first: u32,
    last: u32,
    prop: i32,
    native: *mut ByteReader,
    c: *const Context,
    reads: *const ReaderServices,
    loading: *const newgrf_loading::Services,
    result: *mut i32,
) -> u8 {
    let (c, reads, loading) = unsafe { (&*c, &*reads, &*loading) };
    let mut reader = unsafe { Reader::new(native, reads) };
    u8::from(finish(
        &unsafe { tiles(first..last, prop, c, &mut reader, loading) },
        result,
    ))
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDIndustryPropertyContext" ["industry"] => Context { file, grfid, version, climate };
}
