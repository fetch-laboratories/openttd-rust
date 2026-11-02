/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Action0 industry and tile property interpretation, including incremental failure effects.
use super::{
    Services,
    storage::{LayoutBuffer, LayoutTile, Services as Storage},
};
use crate::bitmath_func::{gb, sb};
use crate::cargo_type::INVALID_CARGO;
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
    spec: *mut super::SpecHandle,
    c: &Context,
    r: &mut Reader<'_>,
    p: &Storage,
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
                unsafe {
                    (p.layout_size_mismatch)(id);
                }
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
                        (p.layout_import_type)(id);
                        newgrf_loading::disable_invalid_id(loading);
                    }
                    return Ok(DISABLED);
                }
                if usize::from(index) >= unsafe { (p.original_layout_count)(kind) } {
                    unsafe {
                        (p.layout_import_index)(id);
                        newgrf_loading::disable_invalid_id(loading);
                    }
                    return Ok(DISABLED);
                }
                let original = unsafe { (p.original_layout)(kind, index) };
                layout.extend_from_slice(unsafe {
                    std::slice::from_raw_parts(original.data, original.size)
                });
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
                    unsafe {
                        (p.layout_tile_undefined)(local, id);
                    }
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
            new_layouts.push(LayoutBuffer::new(layout.clone()));
        } else {
            unsafe {
                (p.layout_invalid)(id);
            }
        }
    }
    unsafe {
        (p.install_layouts)(spec, new_layouts.as_ptr(), new_layouts.len());
    }
    Ok(SUCCESS)
}
unsafe fn tiles(
    ids: std::ops::Range<u32>,
    prop: i32,
    c: &Context,
    r: &mut Reader<'_>,
    s: &Services,
    p: &Storage,
    loading: &newgrf_loading::Services,
) -> Result<i32, ReadError> {
    let last = ids.end;
    if last > u32::from(NUM_INDUSTRYTILES_PER_GRF) {
        unsafe {
            (p.too_many_tiles)(last);
        }
        return Ok(INVALID_ID);
    }
    if unsafe { (s.tile_count)(c.file) } < last as usize {
        unsafe {
            (p.resize_tiles)(c.file, last as usize);
        }
    }
    let mut result = SUCCESS;
    for id in ids {
        let mut spec = unsafe { (s.file_tile)(c.file, id as usize) };
        if prop != 0x08 && spec.is_null() {
            result = result.max(ignore_tile(prop, r)?);
            continue;
        }
        match prop {
            0x08 => {
                let substitute = r.byte()?;
                if u16::from(substitute) >= NEW_INDUSTRYTILEOFFSET {
                    unsafe {
                        (p.tile_substitute_invalid)(substitute, id);
                    }
                    continue;
                }
                if spec.is_null() {
                    spec = unsafe { (p.create_tile)(c.file, id as u16, substitute) };
                    unsafe {
                        (s.shared().specs().industry_tile().substitute().set_enabled)(spec, true);
                        (p.clear_animation)(spec);
                        (p.tile_identity)(spec, c.file, id as u16, substitute);
                        super::allocate_tile(id as u16, c.grfid, u16::from(substitute));
                    }
                }
            }
            0x09 => {
                let override_id = r.byte()?;
                if u16::from(override_id) >= NEW_INDUSTRYTILEOFFSET {
                    unsafe {
                        (p.tile_override_invalid)(override_id, id);
                    }
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
                    (s.set_tile_accepted)(spec, slot, cargo);
                    (p.set_acceptance)(spec, slot, gb(u32::from(value), 8, 8).clamp(0, 16) as i8);
                    (p.invalidate_tile_accepted)(spec, slot);
                }
            }
            0x0d => unsafe {
                (p.set_tile_slopes)(spec, r.byte()?);
            },
            0x0e => unsafe {
                (p.set_tile_callbacks)(spec, r.byte()?);
            },
            0x0f => {
                (s.shared().specs().industry_tile().animation().frames)(spec, r.byte()?);
                (s.shared().specs().industry_tile().animation().status)(spec, r.byte()?);
            }
            0x10 => {
                (s.shared().specs().industry_tile().animation().speed)(spec, r.byte()?);
            }
            0x11 => {
                (s.shared().specs().industry_tile().animation().triggers)(spec, r.byte()?);
            }
            0x12 => unsafe {
                (p.set_tile_flags)(spec, r.byte()?);
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
                        unsafe {
                            (s.set_tile_accepted)(spec, i, cargo);
                        }
                        let acceptance = r.byte()? as i8;
                        unsafe {
                            (p.set_acceptance)(spec, i, acceptance);
                        }
                    } else {
                        unsafe {
                            (s.set_tile_accepted)(spec, i, INVALID_CARGO);
                            (p.set_acceptance)(spec, i, 0);
                        }
                    }
                    if i < INDUSTRY_ORIGINAL_NUM_INPUTS {
                        unsafe {
                            (p.invalidate_tile_accepted)(spec, i);
                        }
                    }
                }
            }
            0x14 => {
                let destination = (s.shared().specs().industry_tile().badge_list)(spec);
                unsafe { r.badges(newgrf::GSF_INDUSTRYTILES, destination) }?;
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
    s: &Services,
    p: &Storage,
    loading: &newgrf_loading::Services,
) -> Result<i32, ReadError> {
    let last = ids.end;
    if last > u32::from(NUM_INDUSTRYTYPES_PER_GRF) {
        unsafe {
            (p.too_many_industries)(last);
        }
        return Ok(INVALID_ID);
    }
    if unsafe { (s.industry_count)(c.file) } < last as usize {
        unsafe {
            (p.resize_industries)(c.file, last as usize);
        }
    }
    let mut result = SUCCESS;
    for id in ids {
        let mut spec = unsafe { (s.file_industry)(c.file, id as usize) };
        if prop != 0x08 && spec.is_null() {
            result = result.max(ignore_industry(prop, r)?);
            continue;
        }
        match prop {
            0x08 => {
                let substitute = r.byte()?;
                if substitute == 0xff {
                    unsafe {
                        (s.shared().specs().industry().substitute().set_enabled)(
                            (s.industry)(id as u16),
                            false,
                        );
                    }
                    continue;
                }
                if u16::from(substitute) >= NEW_INDUSTRYOFFSET {
                    unsafe {
                        (p.industry_substitute_invalid)(substitute, id);
                    }
                    continue;
                }
                if spec.is_null() {
                    spec = unsafe { (p.create_industry)(c.file, id as u16, substitute) };
                    unsafe {
                        (s.shared().specs().industry().substitute().set_enabled)(spec, true);
                        (p.industry_identity)(spec, c.file, id as u16, substitute);
                        (p.clear_placement_check)(spec);
                    }
                }
            }
            0x09 => {
                let override_id = r.byte()?;
                if u16::from(override_id) >= NEW_INDUSTRYOFFSET {
                    unsafe {
                        (p.industry_override_invalid)(override_id, id);
                    }
                    continue;
                }
                unsafe {
                    (s.shared().specs().industry().substitute().set_override)(
                        spec,
                        u16::from(override_id),
                    );
                    override_manager::openttd_rust_override_add(
                        override_manager::industry(),
                        id as u16,
                        c.grfid,
                        u32::from(override_id),
                    );
                }
            }
            0x0a => {
                let status = unsafe { layouts(id, spec, c, r, p, loading) }?;
                if status == DISABLED {
                    return Ok(status);
                }
            }
            0x0b => unsafe {
                (p.set_life)(spec, r.byte()?);
            },
            0x0c => unsafe {
                (p.map_closure_text)(spec, r.word()?);
            },
            0x0d => unsafe {
                (p.map_production_up_text)(spec, r.word()?);
            },
            0x0e => unsafe {
                (p.map_production_down_text)(spec, r.word()?);
            },
            0x0f => unsafe {
                (p.set_cost)(spec, r.byte()?);
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
                            (s.set_produced)(spec, i, cargo);
                            (p.invalidate_produced)(spec, i);
                        }
                    } else {
                        unsafe {
                            (s.set_accepted)(spec, i, cargo);
                            (p.invalidate_accepted)(spec, i);
                        }
                    }
                }
                if !produced {
                    r.byte()?;
                }
            }
            0x12 | 0x13 => unsafe {
                (p.set_rate)(spec, (prop - 0x12) as usize, r.byte()?);
            },
            0x14 => unsafe {
                (p.set_minimal_cargo)(spec, r.byte()?);
            },
            0x15 => {
                let count = r.byte()?;
                let mut sounds = Vec::with_capacity(usize::from(count));
                for _ in 0..count {
                    sounds.push(r.byte()?);
                }
                unsafe {
                    (p.set_sounds)(spec, sounds.as_ptr(), sounds.len());
                }
            }
            0x16 => {
                for i in 0..3 {
                    unsafe {
                        (p.set_conflicting)(spec, i, r.byte()?);
                    }
                }
            }
            0x17 => unsafe {
                (p.set_creation)(spec, c.climate, r.byte()?);
            },
            0x18 => unsafe {
                (p.set_ingame)(spec, c.climate, r.byte()?);
            },
            0x19 => unsafe {
                (p.set_map_colour)(spec, r.byte()?);
            },
            0x1a => unsafe {
                (p.set_behaviour)(spec, r.dword()?);
            },
            0x1b => unsafe {
                (p.map_new_industry_text)(spec, r.word()?);
            },
            0x1c..=0x1e => {
                let multiples = r.dword()?;
                unsafe {
                    (p.set_multiplier)(
                        spec,
                        (prop - 0x1c) as usize,
                        0,
                        gb(multiples, 0, 16) as u16,
                    );
                    (p.set_multiplier)(
                        spec,
                        (prop - 0x1c) as usize,
                        1,
                        gb(multiples, 16, 16) as u16,
                    );
                }
            }
            0x1f => unsafe {
                (p.map_name)(spec, r.word()?);
            },
            0x20 => unsafe {
                (p.set_prospecting)(spec, r.dword()?);
            },
            0x21 | 0x22 => {
                let mut mask = u32::from(unsafe { (p.callbacks)(spec) });
                sb(
                    &mut mask,
                    ((prop - 0x21) * 8) as u8,
                    8,
                    u32::from(r.byte()?),
                );
                unsafe {
                    (p.set_callbacks)(spec, mask as u16);
                }
            }
            0x23 => unsafe {
                (p.set_removal_cost)(spec, r.dword()?);
            },
            0x24 => {
                let string = r.word()?;
                if string == 0 {
                    unsafe {
                        (p.clear_station_name)(spec);
                    }
                } else {
                    unsafe {
                        (p.map_station_name)(spec, string);
                    }
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
                        unsafe {
                            (s.set_produced)(spec, i, cargo);
                        }
                    } else {
                        unsafe {
                            (s.set_accepted)(spec, i, cargo);
                        }
                    }
                    if i < labels {
                        if produced {
                            unsafe {
                                (p.invalidate_produced)(spec, i);
                            }
                        } else {
                            unsafe {
                                (p.invalidate_accepted)(spec, i);
                            }
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
                    unsafe {
                        (p.set_rate)(spec, i, rate);
                    }
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
                        unsafe {
                            (p.set_multiplier)(spec, i, j, multiplier);
                        }
                    }
                }
            }
            0x29 => {
                let destination = (s.shared().specs().industry().badge_list)(spec);
                unsafe { r.badges(newgrf::GSF_INDUSTRIES, destination) }?;
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
pub unsafe extern "C" fn openttd_rust_industry_properties(
    first: u32,
    last: u32,
    prop: i32,
    native: *mut ByteReader,
    c: *const Context,
    reads: *const ReaderServices,
    s: *const Services,
    p: *const Storage,
    loading: *const newgrf_loading::Services,
    result: *mut i32,
) -> bool {
    let (c, reads, s, p, loading) = unsafe { (&*c, &*reads, &*s, &*p, &*loading) };
    let mut reader = unsafe { Reader::new(native, reads) };
    finish(
        &unsafe { industries(first..last, prop, c, &mut reader, s, p, loading) },
        result,
    )
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_tile_properties(
    first: u32,
    last: u32,
    prop: i32,
    native: *mut ByteReader,
    c: *const Context,
    reads: *const ReaderServices,
    s: *const Services,
    p: *const Storage,
    loading: *const newgrf_loading::Services,
    result: *mut i32,
) -> bool {
    let (c, reads, s, p, loading) = unsafe { (&*c, &*reads, &*s, &*p, &*loading) };
    let mut reader = unsafe { Reader::new(native, reads) };
    finish(
        &unsafe { tiles(first..last, prop, c, &mut reader, s, p, loading) },
        result,
    )
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDIndustryPropertyContext" ["industry"] => Context { file, grfid, version, climate };
}
