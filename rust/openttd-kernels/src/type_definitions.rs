/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Reservation and Action0 property policy. Each successful read is published
//! before the next native read; only the native caller rethrows reader failure.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap
)]
use crate::newgrf_internal::{CIR_INVALID_ID, CIR_SUCCESS, CIR_UNKNOWN};
use crate::{
    newgrf_bytereader::{BadgeList, ByteReader, ReadError, Reader, Services as ReaderServices},
    type_registry::{self as registry, TypeMap},
};
use crate::{rail_type::INVALID_RAILTYPE, road_type::INVALID_ROADTYPE};
use std::ffi::c_void;
type Handle = *const c_void;
#[repr(C)]
pub struct RailProperties {
    toolbar_caption: extern "C" fn(Handle, u16),
    menu_text: extern "C" fn(Handle, u16),
    build_caption: extern "C" fn(Handle, u16),
    replace_text: extern "C" fn(Handle, u16),
    name: extern "C" fn(Handle, u16),
    cost_multiplier: extern "C" fn(Handle, u16),
    max_speed: extern "C" fn(Handle, u16),
    map_colour: extern "C" fn(Handle, u8),
    introduction_date: extern "C" fn(Handle, i32),
    sorting_order: extern "C" fn(Handle, u8),
    maintenance_multiplier: extern "C" fn(Handle, u16),
    alternate_label: extern "C" fn(Handle, u32),
    new_engine: extern "C" fn(Handle, u16),
    flags: extern "C" fn(Handle, u8),
    powered: extern "C" fn(Handle, u8),
    required: extern "C" fn(Handle, u8),
    introduces: extern "C" fn(Handle, u8),
    compatible: extern "C" fn(Handle, u8),
    curve_speed: extern "C" fn(Handle, u8),
    fallback: extern "C" fn(Handle, u8),
    acceleration: extern "C" fn(Handle, u8),
    badges: extern "C" fn(Handle) -> *mut BadgeList,
    invalid_reserve: extern "C" fn(u32, usize),
    invalid_change: extern "C" fn(u32, usize),
    no_label: extern "C" fn(u32),
}

#[repr(C)]
pub struct RoadProperties {
    toolbar_caption: extern "C" fn(Handle, u16),
    menu_text: extern "C" fn(Handle, u16),
    build_caption: extern "C" fn(Handle, u16),
    replace_text: extern "C" fn(Handle, u16),
    name: extern "C" fn(Handle, u16),
    cost_multiplier: extern "C" fn(Handle, u16),
    max_speed: extern "C" fn(Handle, u16),
    map_colour: extern "C" fn(Handle, u8),
    introduction_date: extern "C" fn(Handle, i32),
    sorting_order: extern "C" fn(Handle, u8),
    maintenance_multiplier: extern "C" fn(Handle, u16),
    alternate_label: extern "C" fn(Handle, u32),
    new_engine: extern "C" fn(Handle, u16),
    flags: extern "C" fn(Handle, u8),
    powered: extern "C" fn(Handle, u8),
    required: extern "C" fn(Handle, u8),
    introduces: extern "C" fn(Handle, u8),
    badges: extern "C" fn(Handle) -> *mut BadgeList,
    invalid_reserve: extern "C" fn(u32, usize),
    invalid_change: extern "C" fn(u32, usize),
    no_label: extern "C" fn(u32),
    reserve_mismatch: extern "C" fn(u32),
    power_mismatch: extern "C" fn(u8, u8),
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRailProperties" ["engine"] => RailProperties { toolbar_caption, menu_text, build_caption, replace_text, name, cost_multiplier, max_speed, map_colour, introduction_date, sorting_order, maintenance_multiplier, alternate_label, new_engine, flags, powered, required, introduces, compatible, curve_speed, fallback, acceleration, badges, invalid_reserve, invalid_change, no_label };
    "OpenTTDRoadProperties" ["engine"] => RoadProperties { toolbar_caption, menu_text, build_caption, replace_text, name, cost_multiplier, max_speed, map_colour, introduction_date, sorting_order, maintenance_multiplier, alternate_label, new_engine, flags, powered, required, introduces, badges, invalid_reserve, invalid_change, no_label, reserve_mismatch, power_mismatch };
}

fn skip_labels(reader: &mut Reader<'_>) -> Result<(), ReadError> {
    for _ in 0..reader.byte()? {
        reader.dword()?;
    }
    Ok(())
}

fn rail_change(
    s: &RailProperties,
    reader: &mut Reader<'_>,
    map: TypeMap,
    version: u8,
    first: u32,
    last: u32,
    prop: i32,
) -> Result<i32, ReadError> {
    if last as usize > map.size {
        (s.invalid_change)(last, map.size);
        return Ok(CIR_INVALID_ID);
    }
    let mut result = CIR_SUCCESS;
    for id in first..last {
        let rt = unsafe { *map.data.add(id as usize) };
        if rt == INVALID_RAILTYPE {
            return Ok(CIR_INVALID_ID);
        }
        let info = registry::rail_spec(rt);
        match prop {
            0x08 => {
                reader.dword()?;
            }
            0x09 => {
                let value = reader.word()?;
                (s.toolbar_caption)(info, value);
                if version < 8 {
                    (s.name)(info, value);
                }
            }
            0x0a => (s.menu_text)(info, reader.word()?),
            0x0b => (s.build_caption)(info, reader.word()?),
            0x0c => (s.replace_text)(info, reader.word()?),
            0x0d => (s.new_engine)(info, reader.word()?),
            0x0e | 0x0f | 0x18 | 0x19 => {
                for _ in 0..reader.byte()? {
                    let label = reader.dword()?.swap_bytes();
                    let resolved = registry::openttd_rust_rail_by_label(label, false);
                    if resolved == INVALID_RAILTYPE {
                        continue;
                    }
                    match prop {
                        0x0f => {
                            (s.powered)(info, resolved);
                            (s.compatible)(info, resolved);
                        }
                        0x0e => (s.compatible)(info, resolved),
                        0x18 => (s.required)(info, resolved),
                        _ => (s.introduces)(info, resolved),
                    }
                }
            }
            0x10 => (s.flags)(info, reader.byte()?),
            0x11 => (s.curve_speed)(info, reader.byte()?),
            0x12 => (s.fallback)(info, reader.byte()?.min(2)),
            0x15 => (s.acceleration)(info, reader.byte()?.min(2)),
            0x13 => (s.cost_multiplier)(info, reader.word()?),
            0x14 => (s.max_speed)(info, reader.word()?),
            0x16 => (s.map_colour)(info, reader.byte()?),
            0x17 => (s.introduction_date)(info, reader.dword()? as i32),
            0x1a => (s.sorting_order)(info, reader.byte()?),
            0x1b => (s.name)(info, reader.word()?),
            0x1c => (s.maintenance_multiplier)(info, reader.word()?),
            0x1d => skip_labels(reader)?,
            0x1e => unsafe {
                reader.badges(crate::newgrf::GSF_RAILTYPES, (s.badges)(info))?;
            },
            _ => result = CIR_UNKNOWN,
        }
    }
    Ok(result)
}

fn rail_reserve(
    s: &RailProperties,
    reader: &mut Reader<'_>,
    map: TypeMap,
    first: u32,
    last: u32,
    prop: i32,
) -> Result<i32, ReadError> {
    if last as usize > map.size {
        (s.invalid_reserve)(last, map.size);
        return Ok(CIR_INVALID_ID);
    }
    let mut result = CIR_SUCCESS;
    for id in first..last {
        match prop {
            0x08 => {
                let label = reader.dword()?.swap_bytes();
                let mut rt = registry::openttd_rust_rail_by_label(label, false);
                if rt == INVALID_RAILTYPE {
                    rt = registry::openttd_rust_rail_allocate(label);
                }
                unsafe {
                    *map.data.add(id as usize) = rt;
                }
            }
            0x09..=0x0d | 0x13 | 0x14 | 0x1b | 0x1c => {
                reader.word()?;
            }
            0x1d => {
                let rt = unsafe { *map.data.add(id as usize) };
                if rt != INVALID_RAILTYPE {
                    for _ in 0..reader.byte()? {
                        let label = reader.dword()?.swap_bytes();
                        (s.alternate_label)(registry::rail_spec(rt), label);
                    }
                } else {
                    (s.no_label)(id);
                    skip_labels(reader)?;
                }
            }
            0x0e | 0x0f | 0x18 | 0x19 => skip_labels(reader)?,
            0x10..=0x12 | 0x15 | 0x16 | 0x1a => {
                reader.byte()?;
            }
            0x17 => {
                reader.dword()?;
            }
            0x1e => reader.skip_badges()?,
            _ => result = CIR_UNKNOWN,
        }
    }
    Ok(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_change(
    s: *const RailProperties,
    reads: *const ReaderServices,
    map: TypeMap,
    version: u8,
    first: u32,
    last: u32,
    prop: i32,
    native: *mut ByteReader,
) -> i32 {
    let mut reader = unsafe { Reader::new(native, &*reads) };
    rail_change(unsafe { &*s }, &mut reader, map, version, first, last, prop).unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_reserve(
    s: *const RailProperties,
    reads: *const ReaderServices,
    map: TypeMap,
    _version: u8,
    first: u32,
    last: u32,
    prop: i32,
    native: *mut ByteReader,
) -> i32 {
    let mut reader = unsafe { Reader::new(native, &*reads) };
    rail_reserve(unsafe { &*s }, &mut reader, map, first, last, prop).unwrap_or(-1)
}

fn road_change(
    s: &RoadProperties,
    reader: &mut Reader<'_>,
    map: TypeMap,
    _version: u8,
    first: u32,
    last: u32,
    prop: i32,
    rtt: u8,
) -> Result<i32, ReadError> {
    if last as usize > map.size {
        (s.invalid_change)(last, map.size);
        return Ok(CIR_INVALID_ID);
    }
    let mut result = CIR_SUCCESS;
    for id in first..last {
        let rt = unsafe { *map.data.add(id as usize) };
        if rt == INVALID_ROADTYPE {
            return Ok(CIR_INVALID_ID);
        }
        let info = registry::road_spec(rt);
        match prop {
            0x08 => {
                reader.dword()?;
            }
            0x09 => {
                let value = reader.word()?;
                (s.toolbar_caption)(info, value);
            }
            0x0a => (s.menu_text)(info, reader.word()?),
            0x0b => (s.build_caption)(info, reader.word()?),
            0x0c => (s.replace_text)(info, reader.word()?),
            0x0d => (s.new_engine)(info, reader.word()?),
            0x0f | 0x18 | 0x19 => {
                for _ in 0..reader.byte()? {
                    let label = reader.dword()?.swap_bytes();
                    let resolved = registry::openttd_rust_road_by_label(label, false);
                    if resolved == INVALID_ROADTYPE {
                        continue;
                    }
                    match prop {
                        0x0f => {
                            if registry::road_tram_type(resolved) == rtt {
                                (s.powered)(info, resolved);
                            } else {
                                (s.power_mismatch)(resolved, rt);
                            }
                        }
                        0x18 => (s.required)(info, resolved),
                        _ => (s.introduces)(info, resolved),
                    }
                }
            }
            0x10 => (s.flags)(info, reader.byte()?),
            0x13 => (s.cost_multiplier)(info, reader.word()?),
            0x14 => (s.max_speed)(info, reader.word()?),
            0x16 => (s.map_colour)(info, reader.byte()?),
            0x17 => (s.introduction_date)(info, reader.dword()? as i32),
            0x1a => (s.sorting_order)(info, reader.byte()?),
            0x1b => (s.name)(info, reader.word()?),
            0x1c => (s.maintenance_multiplier)(info, reader.word()?),
            0x1d => skip_labels(reader)?,
            0x1e => unsafe {
                reader.badges(crate::newgrf::GSF_ROADTYPES, (s.badges)(info))?;
            },
            _ => result = CIR_UNKNOWN,
        }
    }
    Ok(result)
}

fn road_reserve(
    s: &RoadProperties,
    reader: &mut Reader<'_>,
    map: TypeMap,
    first: u32,
    last: u32,
    prop: i32,
    rtt: u8,
) -> Result<i32, ReadError> {
    if last as usize > map.size {
        (s.invalid_reserve)(last, map.size);
        return Ok(CIR_INVALID_ID);
    }
    let mut result = CIR_SUCCESS;
    for id in first..last {
        match prop {
            0x08 => {
                let label = reader.dword()?.swap_bytes();
                let mut rt = registry::openttd_rust_road_by_label(label, false);
                if rt == INVALID_ROADTYPE {
                    rt = registry::openttd_rust_road_allocate(label, rtt);
                } else if registry::road_tram_type(rt) != rtt {
                    (s.reserve_mismatch)(id);
                    return Ok(CIR_INVALID_ID);
                }
                unsafe {
                    *map.data.add(id as usize) = rt;
                }
            }
            0x09..=0x0d | 0x13 | 0x14 | 0x1b | 0x1c => {
                reader.word()?;
            }
            0x1d => {
                let rt = unsafe { *map.data.add(id as usize) };
                if rt != INVALID_ROADTYPE {
                    for _ in 0..reader.byte()? {
                        let label = reader.dword()?.swap_bytes();
                        (s.alternate_label)(registry::road_spec(rt), label);
                    }
                } else {
                    (s.no_label)(id);
                    skip_labels(reader)?;
                }
            }
            0x0f | 0x18 | 0x19 => skip_labels(reader)?,
            0x10 | 0x16 | 0x1a => {
                reader.byte()?;
            }
            0x17 => {
                reader.dword()?;
            }
            0x1e => reader.skip_badges()?,
            _ => result = CIR_UNKNOWN,
        }
    }
    Ok(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_change(
    s: *const RoadProperties,
    reads: *const ReaderServices,
    map: TypeMap,
    _version: u8,
    first: u32,
    last: u32,
    prop: i32,
    native: *mut ByteReader,
    rtt: u8,
) -> i32 {
    let mut reader = unsafe { Reader::new(native, &*reads) };
    road_change(unsafe { &*s }, &mut reader, map, 0, first, last, prop, rtt).unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_reserve(
    s: *const RoadProperties,
    reads: *const ReaderServices,
    map: TypeMap,
    _version: u8,
    first: u32,
    last: u32,
    prop: i32,
    native: *mut ByteReader,
    rtt: u8,
) -> i32 {
    let mut reader = unsafe { Reader::new(native, &*reads) };
    road_reserve(unsafe { &*s }, &mut reader, map, first, last, prop, rtt).unwrap_or(-1)
}
