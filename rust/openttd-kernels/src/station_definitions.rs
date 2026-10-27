/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Station and road-stop Action0 definitions and cargo group publication.
//! Native GRF/spec shells and payload containers stay canonical. Each entry borrows
//! a live reader under the game-state mutex. No spec borrow crosses layout parsing:
//! disabling a GRF may destroy it. Read failures return before the decoder rethrows.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
use crate::newgrf_act2::{self, Services as Action2Services};
use crate::newgrf_bytereader::{BadgeList, ByteReader, ReadError, Reader};
use crate::newgrf_internal::{CIR_DISABLED, CIR_INVALID_ID, CIR_SUCCESS, CIR_UNKNOWN};
use crate::newgrf_layout::{Layout, LayoutSequence, Sequence};
use crate::newgrf_loading::Services as LoadingServices;
use crate::station_cmd::{RailSpec, RoadSpec};
use std::ffi::c_void;
const NUM_STATIONS_PER_GRF: u32 = u16::MAX as u32 - 1;
const GSF_STATIONS: u8 = crate::newgrf::GSF_STATIONS;
const GSF_ROADSTOPS: u8 = crate::newgrf::GSF_ROADSTOPS;
#[repr(C)]
pub struct Services {
    empty_string: u32,
    cargo: *const crate::newgrf_act3::Services,
    reader: *const crate::newgrf_bytereader::Services,
    file_version: unsafe extern "C" fn(*const c_void) -> u8,
    station_resize: unsafe extern "C" fn(*mut c_void, u32) -> (),
    station_count: unsafe extern "C" fn(*const c_void) -> usize,
    station_get: unsafe extern "C" fn(*mut c_void, u32) -> *mut RailSpec,
    station_new: unsafe extern "C" fn(*mut c_void, u32) -> *mut RailSpec,
    station_class: unsafe extern "C" fn(*mut RailSpec, u16) -> (),
    station_name: unsafe extern "C" fn(*mut RailSpec, u16) -> (),
    station_class_name: unsafe extern "C" fn(*mut RailSpec, u16) -> (),
    station_badges: unsafe extern "C" fn(*mut RailSpec) -> *mut BadgeList,
    station_cargo_triggers: unsafe extern "C" fn(*mut RailSpec, u64) -> (),
    station_frames: unsafe extern "C" fn(*mut RailSpec, u8) -> (),
    station_status: unsafe extern "C" fn(*mut RailSpec, u8) -> (),
    station_speed: unsafe extern "C" fn(*mut RailSpec, u8) -> (),
    station_triggers: unsafe extern "C" fn(*mut RailSpec, u16) -> (),
    station_callback_mask: unsafe extern "C" fn(*mut RailSpec, u8) -> (),
    station_flags: unsafe extern "C" fn(*mut RailSpec, u8) -> (),
    station_has_file: unsafe extern "C" fn(*const RailSpec) -> bool,
    station_set_group: unsafe extern "C" fn(*mut RailSpec, u8, *const c_void) -> (),
    station_set_file: unsafe extern "C" fn(*mut RailSpec, *const c_void) -> (),
    station_set_local: unsafe extern "C" fn(*mut RailSpec, u16) -> (),
    road_resize: unsafe extern "C" fn(*mut c_void, u32) -> (),
    road_count: unsafe extern "C" fn(*const c_void) -> usize,
    road_get: unsafe extern "C" fn(*mut c_void, u32) -> *mut RoadSpec,
    road_new: unsafe extern "C" fn(*mut c_void, u32) -> *mut RoadSpec,
    road_class: unsafe extern "C" fn(*mut RoadSpec, u16) -> (),
    road_name: unsafe extern "C" fn(*mut RoadSpec, u16) -> (),
    road_class_name: unsafe extern "C" fn(*mut RoadSpec, u16) -> (),
    road_badges: unsafe extern "C" fn(*mut RoadSpec) -> *mut BadgeList,
    road_cargo_triggers: unsafe extern "C" fn(*mut RoadSpec, u64) -> (),
    road_frames: unsafe extern "C" fn(*mut RoadSpec, u8) -> (),
    road_status: unsafe extern "C" fn(*mut RoadSpec, u8) -> (),
    road_speed: unsafe extern "C" fn(*mut RoadSpec, u8) -> (),
    road_triggers: unsafe extern "C" fn(*mut RoadSpec, u16) -> (),
    road_callback_mask: unsafe extern "C" fn(*mut RoadSpec, u8) -> (),
    road_flags: unsafe extern "C" fn(*mut RoadSpec, u32) -> (),
    road_has_file: unsafe extern "C" fn(*const RoadSpec) -> bool,
    road_set_group: unsafe extern "C" fn(*mut RoadSpec, u8, *const c_void) -> (),
    road_set_file: unsafe extern "C" fn(*mut RoadSpec, *const c_void) -> (),
    road_set_local: unsafe extern "C" fn(*mut RoadSpec, u16) -> (),
    station_disallowed_platforms: unsafe extern "C" fn(*mut RailSpec, u8) -> (),
    station_disallowed_lengths: unsafe extern "C" fn(*mut RailSpec, u8) -> (),
    station_cargo_threshold: unsafe extern "C" fn(*mut RailSpec, u16) -> (),
    road_stop_type: unsafe extern "C" fn(*mut RoadSpec, u8) -> (),
    road_draw_mode: unsafe extern "C" fn(*mut RoadSpec, u8) -> (),
    road_build_cost: unsafe extern "C" fn(*mut RoadSpec, u8) -> (),
    road_clear_cost: unsafe extern "C" fn(*mut RoadSpec, u8) -> (),
    station_render_clear: unsafe extern "C" fn(*mut RailSpec) -> (),
    station_render_reserve: unsafe extern "C" fn(*mut RailSpec, u16) -> (),
    station_render_size: unsafe extern "C" fn(*const RailSpec) -> usize,
    station_render_append: unsafe extern "C" fn(*mut RailSpec) -> *mut Layout,
    station_render_pop: unsafe extern "C" fn(*mut RailSpec) -> (),
    station_render_copy_append: unsafe extern "C" fn(*mut RailSpec, *const RailSpec, usize) -> (),
    sequence_reset: unsafe extern "C" fn(*mut Sequence),
    sequence_append: unsafe extern "C" fn(*mut Sequence) -> *mut LayoutSequence,
    layout_sequence_move: unsafe extern "C" fn(*mut Layout, *mut Sequence),
    layout_original: unsafe extern "C" fn(*mut Layout, u32) -> (),
    station_custom_assign: unsafe extern "C" fn(*mut RailSpec, u16, *const u8, usize) -> (),
    station_custom_tile: unsafe extern "C" fn(*mut RailSpec, u16, usize, u8) -> (),
    station_custom_copy: unsafe extern "C" fn(*mut RailSpec, *const RailSpec) -> (),
    station_tileflags_size: unsafe extern "C" fn(*const RailSpec) -> usize,
    station_tileflags_resize: unsafe extern "C" fn(*mut RailSpec, usize) -> (),
    station_pylons: unsafe extern "C" fn(*mut RailSpec, u8, bool) -> (),
    station_wires: unsafe extern "C" fn(*mut RailSpec, u8, bool) -> (),
    station_blocked: unsafe extern "C" fn(*mut RailSpec, u8, bool) -> (),
    station_tileflags_assign: unsafe extern "C" fn(*mut RailSpec, *const u8, usize) -> (),
    station_bridge_size: unsafe extern "C" fn(*const RailSpec) -> usize,
    station_bridge_resize: unsafe extern "C" fn(*mut RailSpec, usize) -> (),
    road_bridge_size: unsafe extern "C" fn(*const RoadSpec) -> usize,
    station_bridge_height: unsafe extern "C" fn(*mut RailSpec, usize, u8) -> (),
    station_bridge_pillars: unsafe extern "C" fn(*mut RailSpec, usize, u8) -> (),
    road_bridge_height: unsafe extern "C" fn(*mut RoadSpec, usize, u8) -> (),
    road_bridge_pillars: unsafe extern "C" fn(*mut RoadSpec, usize, u8) -> (),
    station_invalid: unsafe extern "C" fn(u32) -> (),
    road_invalid: unsafe extern "C" fn(u32) -> (),
    station_undefined: unsafe extern "C" fn(u32) -> (),
    road_undefined: unsafe extern "C" fn(u32) -> (),
    odd_layout: unsafe extern "C" fn(u32) -> (),
    missing_sprite: unsafe extern "C" fn(u16, u32) -> (),
    missing_custom: unsafe extern "C" fn(u16, u32) -> (),
    invalid_tile: unsafe extern "C" fn(u8, u8, u8) -> (),
    undefined_publication: unsafe extern "C" fn(u16) -> (),
    duplicate_publication: unsafe extern "C" fn(u16) -> (),
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDStationDefinitionServices" ["station_service"] => Services { empty_string, cargo, reader, file_version, station_resize, station_count, station_get, station_new, station_class, station_name, station_class_name, station_badges, station_cargo_triggers, station_frames, station_status, station_speed, station_triggers, station_callback_mask, station_flags, station_has_file, station_set_group, station_set_file, station_set_local, road_resize, road_count, road_get, road_new, road_class, road_name, road_class_name, road_badges, road_cargo_triggers, road_frames, road_status, road_speed, road_triggers, road_callback_mask, road_flags, road_has_file, road_set_group, road_set_file, road_set_local, station_disallowed_platforms, station_disallowed_lengths, station_cargo_threshold, road_stop_type, road_draw_mode, road_build_cost, road_clear_cost, station_render_clear, station_render_reserve, station_render_size, station_render_append, station_render_pop, station_render_copy_append, sequence_reset, sequence_append, layout_sequence_move, layout_original, station_custom_assign, station_custom_tile, station_custom_copy, station_tileflags_size, station_tileflags_resize, station_pylons, station_wires, station_blocked, station_tileflags_assign, station_bridge_size, station_bridge_resize, road_bridge_size, station_bridge_height, station_bridge_pillars, road_bridge_height, road_bridge_pillars, station_invalid, road_invalid, station_undefined, road_undefined, odd_layout, missing_sprite, missing_custom, invalid_tile, undefined_publication, duplicate_publication };
}

unsafe fn station_change(
    s: &Services,
    action2: &Action2Services,
    loading: &LoadingServices,
    file: *mut c_void,
    first: u32,
    last: u32,
    prop: u32,
    sequence: *mut Sequence,
    reader: &mut Reader<'_>,
) -> Result<i32, ReadError> {
    unsafe {
        if last > NUM_STATIONS_PER_GRF {
            (s.station_invalid)(last);
            return Ok(CIR_INVALID_ID);
        }
        if (s.station_count)(file) < last as usize {
            (s.station_resize)(file, last);
        }
        let mut result = CIR_SUCCESS;
        for id in first..last {
            let mut spec = (s.station_get)(file, id);
            if spec.is_null() && prop != 0x08 {
                (s.station_undefined)(id);
                return Ok(CIR_INVALID_ID);
            }
            match prop {
                0x08 => {
                    if spec.is_null() {
                        spec = (s.station_new)(file, id);
                    }
                    let label = reader.dword()?.swap_bytes();
                    let class = crate::newgrf_class::station::allocate(
                        label,
                        crate::newgrf_station::STAT_CLASS_MAX,
                        s.empty_string,
                    );
                    (s.station_class)(spec, class);
                }
                0x09 => {
                    let tiles = reader.extended()?;
                    (s.station_render_clear)(spec);
                    (s.station_render_reserve)(spec, tiles);
                    for tile in 0..tiles {
                        let layout = (s.station_render_append)(spec);
                        let fields = (action2.layout_storage)(layout);
                        fields.consistent_max_offset.write(u32::from(u16::MAX));
                        if reader.has_data(4) && reader.peek_dword()? == 0 {
                            reader.skip(4)?;
                            (s.layout_original)(layout, u32::from(tile) % 8);
                            continue;
                        }
                        newgrf_act2::layout_sprite(
                            action2,
                            loading,
                            reader,
                            false,
                            false,
                            false,
                            GSF_STATIONS,
                            fields.ground,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                        )?;
                        if crate::newgrf_loading::is_disabled() {
                            return Ok(CIR_DISABLED);
                        }
                        (s.sequence_reset)(sequence);
                        loop {
                            let delta_x = reader.byte()?;
                            if delta_x == 0x80 {
                                break;
                            }
                            let item = (s.sequence_append)(sequence);
                            let fields = (action2.sequence_storage)(item);
                            fields.x.write(delta_x as i8);
                            fields.y.write(reader.byte()? as i8);
                            fields.z.write(reader.byte()? as i8);
                            fields.extent_x.write(reader.byte()?);
                            fields.extent_y.write(reader.byte()?);
                            fields.extent_z.write(reader.byte()?);
                            newgrf_act2::layout_sprite(
                                action2,
                                loading,
                                reader,
                                false,
                                true,
                                false,
                                GSF_STATIONS,
                                fields.image,
                                std::ptr::null_mut(),
                                std::ptr::null_mut(),
                            )?;
                            if crate::newgrf_loading::is_disabled() {
                                return Ok(CIR_DISABLED);
                            }
                        }
                        (s.layout_sequence_move)(layout, sequence);
                        (s.sequence_reset)(sequence);
                    }
                    if (s.station_render_size)(spec) & 1 != 0 {
                        (s.odd_layout)(id);
                        (s.station_render_pop)(spec);
                    }
                }
                0x0a => {
                    let source_id = reader.extended()?;
                    let source = (s.station_get)(file, u32::from(source_id));
                    if source.is_null() {
                        (s.missing_sprite)(source_id, id);
                        continue;
                    }
                    // StationChangeInfo clears before observing the source, including self-copy.
                    (s.station_render_clear)(spec);
                    let count = (s.station_render_size)(source);
                    (s.station_render_reserve)(spec, count as u16);
                    for index in 0..count {
                        (s.station_render_copy_append)(spec, source, index);
                    }
                }
                0x0b => (s.station_callback_mask)(spec, reader.byte()?),
                0x0c => (s.station_disallowed_platforms)(spec, reader.byte()?),
                0x0d => (s.station_disallowed_lengths)(spec, reader.byte()?),
                0x0e => {
                    while reader.has_data(1) {
                        let length = reader.byte()?;
                        let number = reader.byte()?;
                        if length == 0 || number == 0 {
                            break;
                        }
                        let count = usize::from(length) * usize::from(number);
                        let data = reader.bytes(count)?;
                        let key = crate::newgrf_station::get_station_layout_key(number, length);
                        (s.station_custom_assign)(spec, key, data, count);
                        for index in 0..count {
                            let tile = data.add(index).read();
                            if tile & !1 != tile {
                                (s.invalid_tile)(tile, length, number);
                                (s.station_custom_tile)(spec, key, index, tile & !1);
                            }
                        }
                    }
                }
                0x0f => {
                    let source_id = reader.extended()?;
                    let source = (s.station_get)(file, u32::from(source_id));
                    if source.is_null() {
                        (s.missing_custom)(source_id, id);
                        continue;
                    }
                    (s.station_custom_copy)(spec, source);
                }
                0x10 => (s.station_cargo_threshold)(spec, reader.word()?),
                0x11 | 0x14 | 0x15 => {
                    let value = reader.byte()?;
                    if (s.station_tileflags_size)(spec) < 8 {
                        (s.station_tileflags_resize)(spec, 8);
                    }
                    let set = match prop {
                        0x11 => s.station_pylons,
                        0x14 => s.station_wires,
                        _ => s.station_blocked,
                    };
                    for index in 0..8 {
                        set(spec, index, value & (1 << index) != 0);
                    }
                }
                0x12 => {
                    let value = reader.dword()?;
                    let cargo = if (s.file_version)(file) >= 7 {
                        crate::engine_loading::openttd_rust_engine_translate_refit(value, file)
                    } else {
                        u64::from(value)
                    };
                    (s.station_cargo_triggers)(spec, cargo);
                }
                0x13 => (s.station_flags)(spec, reader.byte()?),
                0x16 => {
                    (s.station_frames)(spec, reader.byte()?);
                    (s.station_status)(spec, reader.byte()?);
                }
                0x17 => (s.station_speed)(spec, reader.byte()?),
                0x18 => (s.station_triggers)(spec, reader.word()?),
                0x1a => {
                    let tiles = reader.extended()?;
                    (s.station_render_clear)(spec);
                    (s.station_render_reserve)(spec, tiles);
                    for _ in 0..tiles {
                        let layout = (s.station_render_append)(spec);
                        let count = reader.byte()?;
                        if newgrf_act2::read_layout(
                            action2,
                            loading,
                            reader,
                            u32::from(count),
                            false,
                            GSF_STATIONS,
                            true,
                            false,
                            layout,
                        )? {
                            return Ok(CIR_DISABLED);
                        }
                    }
                    if (s.station_render_size)(spec) & 1 != 0 {
                        (s.odd_layout)(id);
                        (s.station_render_pop)(spec);
                    }
                }
                0x1b => {
                    for _ in 0..4 {
                        reader.word()?;
                    }
                }
                0x1c => (s.station_name)(spec, reader.word()?),
                0x1d => (s.station_class_name)(spec, reader.word()?),
                0x1e => {
                    let tiles = usize::from(reader.extended()?);
                    let data = reader.bytes(tiles)?;
                    (s.station_tileflags_assign)(spec, data, tiles);
                }
                0x1f => reader.badges(GSF_STATIONS, (s.station_badges)(spec))?,
                0x20 | 0x21 => {
                    let tiles = usize::from(reader.extended()?);
                    if (s.station_bridge_size)(spec) < tiles {
                        (s.station_bridge_resize)(spec, tiles);
                    }
                    let write = if prop == 0x20 {
                        s.station_bridge_height
                    } else {
                        s.station_bridge_pillars
                    };
                    for index in 0..tiles {
                        write(spec, index, reader.byte()?);
                    }
                }
                _ => result = CIR_UNKNOWN,
            }
        }
        Ok(result)
    }
}
fn ignore_road_property(prop: u32, reader: &mut Reader<'_>) -> Result<i32, ReadError> {
    match prop {
        0x09 | 0x0c | 0x0f | 0x11 => {
            reader.byte()?;
        }
        0x0a | 0x0b | 0x0e | 0x10 | 0x15 => {
            reader.word()?;
        }
        0x08 | 0x0d | 0x12 => {
            reader.dword()?;
        }
        0x13 | 0x14 => {
            let count = usize::from(reader.extended()?);
            reader.skip(count)?;
        }
        0x16 => reader.skip_badges()?,
        _ => return Ok(CIR_UNKNOWN),
    }
    Ok(CIR_SUCCESS)
}
unsafe fn road_change(
    s: &Services,
    file: *mut c_void,
    first: u32,
    last: u32,
    prop: u32,
    reader: &mut Reader<'_>,
) -> Result<i32, ReadError> {
    unsafe {
        if last > crate::newgrf_roadstop::NUM_ROADSTOPS_PER_GRF {
            (s.road_invalid)(last);
            return Ok(CIR_INVALID_ID);
        }
        if (s.road_count)(file) < last as usize {
            (s.road_resize)(file, last);
        }
        let mut result = CIR_SUCCESS;
        for id in first..last {
            let mut spec = (s.road_get)(file, id);
            if spec.is_null() && prop != 0x08 {
                (s.road_undefined)(id);
                result = result.max(ignore_road_property(prop, reader)?);
                continue;
            }
            match prop {
                0x08 => {
                    if spec.is_null() {
                        spec = (s.road_new)(file, id);
                    }
                    let label = reader.dword()?.swap_bytes();
                    let class = crate::newgrf_class::roadstop::allocate(
                        label,
                        crate::newgrf_roadstop::ROADSTOP_CLASS_MAX,
                        s.empty_string,
                    );
                    (s.road_class)(spec, class);
                }
                0x09 => (s.road_stop_type)(spec, reader.byte()?),
                0x0a => (s.road_name)(spec, reader.word()?),
                0x0b => (s.road_class_name)(spec, reader.word()?),
                0x0c => (s.road_draw_mode)(spec, reader.byte()?),
                0x0d => {
                    let cargo = crate::engine_loading::openttd_rust_engine_translate_refit(
                        reader.dword()?,
                        file,
                    );
                    (s.road_cargo_triggers)(spec, cargo);
                }
                0x0e => {
                    (s.road_frames)(spec, reader.byte()?);
                    (s.road_status)(spec, reader.byte()?);
                }
                0x0f => (s.road_speed)(spec, reader.byte()?),
                0x10 => (s.road_triggers)(spec, reader.word()?),
                0x11 => (s.road_callback_mask)(spec, reader.byte()?),
                0x12 => (s.road_flags)(spec, reader.dword()?),
                0x13 | 0x14 => {
                    let tiles = usize::from(reader.extended()?);
                    let write = if prop == 0x13 {
                        s.road_bridge_height
                    } else {
                        s.road_bridge_pillars
                    };
                    for index in 0..tiles {
                        if index < (s.road_bridge_size)(spec) {
                            write(spec, index, reader.byte()?);
                        } else {
                            reader.byte()?;
                        }
                    }
                }
                0x15 => {
                    (s.road_build_cost)(spec, reader.byte()?);
                    (s.road_clear_cost)(spec, reader.byte()?);
                }
                0x16 => reader.badges(GSF_ROADSTOPS, (s.road_badges)(spec))?,
                _ => result = CIR_UNKNOWN,
            }
        }
        Ok(result)
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_change_info(
    s: *const Services,
    action2: *const Action2Services,
    loading: *const LoadingServices,
    file: *mut c_void,
    first: u32,
    last: u32,
    prop: u32,
    native: *mut ByteReader,
    sequence: *mut Sequence,
    output: *mut u8,
) -> bool {
    let s = unsafe { &*s };
    let mut reader = unsafe { Reader::new(native, &*s.reader) };
    match unsafe {
        station_change(
            s,
            &*action2,
            &*loading,
            file,
            first,
            last,
            prop,
            sequence,
            &mut reader,
        )
    } {
        Ok(value) => {
            unsafe {
                output.write(value as u8);
            }
            true
        }
        Err(_) => false,
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_roadstop_change_info(
    s: *const Services,
    file: *mut c_void,
    first: u32,
    last: u32,
    prop: u32,
    native: *mut ByteReader,
    output: *mut u8,
) -> bool {
    let s = unsafe { &*s };
    let mut reader = unsafe { Reader::new(native, &*s.reader) };
    match unsafe { road_change(s, file, first, last, prop, &mut reader) } {
        Ok(value) => {
            unsafe {
                output.write(value as u8);
            }
            true
        }
        Err(_) => false,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_map_specific(
    s: *const Services,
    file: *mut c_void,
    id: u16,
    cid: u8,
    group: *const c_void,
) {
    let s = unsafe { &*s };
    // CargoTypeMapSpriteGroupHandler uses GSF_STATIONS for both native spec types.
    let cargo = crate::newgrf_act3::translate_cargo(unsafe { &*s.cargo }, GSF_STATIONS, cid, file);
    if !crate::cargo_type::is_valid_cargo_type(cargo) {
        return;
    }
    let spec = unsafe { (s.station_get)(file, u32::from(id)) };
    if spec.is_null() {
        unsafe {
            (s.undefined_publication)(id);
        }
    } else {
        unsafe {
            (s.station_set_group)(spec, cargo, group);
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_map_default(
    s: *const Services,
    file: *mut c_void,
    id: u16,
    group: *const c_void,
) {
    let s = unsafe { &*s };
    let spec = unsafe { (s.station_get)(file, u32::from(id)) };
    if spec.is_null() {
        unsafe {
            (s.undefined_publication)(id);
        }
    } else if unsafe { (s.station_has_file)(spec) } {
        unsafe {
            (s.duplicate_publication)(id);
        }
    } else {
        unsafe {
            (s.station_set_group)(spec, crate::newgrf_commons::SG_DEFAULT, group);
            (s.station_set_file)(spec, file);
            (s.station_set_local)(spec, id);
            crate::newgrf_class::station::assign(spec);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_roadstop_map_specific(
    s: *const Services,
    file: *mut c_void,
    id: u16,
    cid: u8,
    group: *const c_void,
) {
    let s = unsafe { &*s };
    // CargoTypeMapSpriteGroupHandler uses GSF_STATIONS for both native spec types.
    let cargo = crate::newgrf_act3::translate_cargo(unsafe { &*s.cargo }, GSF_STATIONS, cid, file);
    if !crate::cargo_type::is_valid_cargo_type(cargo) {
        return;
    }
    let spec = unsafe { (s.road_get)(file, u32::from(id)) };
    if spec.is_null() {
        unsafe {
            (s.undefined_publication)(id);
        }
    } else {
        unsafe {
            (s.road_set_group)(spec, cargo, group);
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_roadstop_map_default(
    s: *const Services,
    file: *mut c_void,
    id: u16,
    group: *const c_void,
) {
    let s = unsafe { &*s };
    let spec = unsafe { (s.road_get)(file, u32::from(id)) };
    if spec.is_null() {
        unsafe {
            (s.undefined_publication)(id);
        }
    } else if unsafe { (s.road_has_file)(spec) } {
        unsafe {
            (s.duplicate_publication)(id);
        }
    } else {
        unsafe {
            (s.road_set_group)(spec, crate::newgrf_commons::SG_DEFAULT, group);
            (s.road_set_file)(spec, file);
            (s.road_set_local)(spec, id);
            crate::newgrf_class::roadstop::assign(spec);
        }
    }
}
