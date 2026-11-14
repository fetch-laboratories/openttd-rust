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
use crate::bitmath_func::has_bit;
use crate::cargo_type::{CargoType, CargoTypes};
use crate::debug::grf_msg;
use crate::newgrf_act2::{self, Services as Action2Services};
use crate::newgrf_bytereader::{ByteReader, ReadError, Reader};
use crate::newgrf_internal::ChangeInfoResult;
use crate::newgrf_layout::{Layout, LayoutSequence, Sequence};
use crate::newgrf_loading::Services as LoadingServices;
use crate::newgrf_roadstop::{RoadStopAvailabilityType, RoadStopClassID, RoadStopDrawModes};
use crate::newgrf_station::{StationClassID, StationSpecFlags};
use crate::station_cmd::{RailSpec, RoadSpec};
use std::ffi::c_void;
/// `NUM_STATIONS_PER_GRF`: the maximum number of stations a single GRF is
/// allowed to add.
const NUM_STATIONS_PER_GRF: u32 = u16::MAX as u32 - 1;
const GSF_STATIONS: u8 = crate::newgrf::GSF_STATIONS;
const GSF_ROADSTOPS: u8 = crate::newgrf::GSF_ROADSTOPS;
#[repr(C)]
pub struct Services {
    empty_string: u32,
    reader: *const crate::newgrf_bytereader::Services,
    shared: *const crate::services::Services,
    station_resize: unsafe extern "C" fn(*mut c_void, u32) -> (),
    station_count: unsafe extern "C" fn(*const c_void) -> usize,
    pub(crate) station_get: unsafe extern "C" fn(*mut c_void, u32) -> *mut RailSpec,
    station_new: unsafe extern "C" fn(*mut c_void, u32) -> *mut RailSpec,
    station_class: unsafe extern "C" fn(*mut RailSpec, u16) -> (),
    station_name: unsafe extern "C" fn(*mut RailSpec, u16) -> (),
    station_class_name: unsafe extern "C" fn(*mut RailSpec, u16) -> (),
    station_cargo_triggers: unsafe extern "C" fn(*mut RailSpec, u64) -> (),
    station_callback_mask: unsafe extern "C" fn(*mut RailSpec, u8) -> (),
    station_flags: unsafe extern "C" fn(*mut RailSpec, u8) -> (),
    station_set_group: unsafe extern "C" fn(*mut RailSpec, u8, *const c_void) -> (),
    road_resize: unsafe extern "C" fn(*mut c_void, u32) -> (),
    road_count: unsafe extern "C" fn(*const c_void) -> usize,
    road_get: unsafe extern "C" fn(*mut c_void, u32) -> *mut RoadSpec,
    road_new: unsafe extern "C" fn(*mut c_void, u32) -> *mut RoadSpec,
    road_class: unsafe extern "C" fn(*mut RoadSpec, u16) -> (),
    road_name: unsafe extern "C" fn(*mut RoadSpec, u16) -> (),
    road_class_name: unsafe extern "C" fn(*mut RoadSpec, u16) -> (),
    road_cargo_triggers: unsafe extern "C" fn(*mut RoadSpec, u64) -> (),
    road_callback_mask: unsafe extern "C" fn(*mut RoadSpec, u8) -> (),
    road_flags: unsafe extern "C" fn(*mut RoadSpec, u32) -> (),
    road_set_group: unsafe extern "C" fn(*mut RoadSpec, u8, *const c_void) -> (),
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
}
impl Services {
    fn shared(&self) -> &crate::services::Services {
        // SAFETY: `GetStationDefinitionServices` points at the process-lifetime shared table.
        unsafe { &*self.shared }
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDStationDefinitionServices" ["station_service"] => Services { empty_string, reader, shared, station_resize, station_count, station_get, station_new, station_class, station_name, station_class_name, station_cargo_triggers, station_callback_mask, station_flags, station_set_group, road_resize, road_count, road_get, road_new, road_class, road_name, road_class_name, road_cargo_triggers, road_callback_mask, road_flags, road_set_group, station_disallowed_platforms, station_disallowed_lengths, station_cargo_threshold, road_stop_type, road_draw_mode, road_build_cost, road_clear_cost, station_render_clear, station_render_reserve, station_render_size, station_render_append, station_render_pop, station_render_copy_append, sequence_reset, sequence_append, layout_sequence_move, layout_original, station_custom_assign, station_custom_tile, station_custom_copy, station_tileflags_size, station_tileflags_resize, station_pylons, station_wires, station_blocked, station_tileflags_assign, station_bridge_size, station_bridge_resize, road_bridge_size, station_bridge_height, station_bridge_pillars, road_bridge_height, road_bridge_pillars };
}

/// The `_cur_gps` loading state that `StationChangeInfo` reads, with the
/// native tables it writes through.
struct StationChangeCtx<'a> {
    s: &'a Services,
    action2: &'a Action2Services,
    loading: &'a LoadingServices,
    /// `_cur_gps.grffile`.
    file: *mut c_void,
    /// Scratch `std::vector<DrawTileSeqStruct> tmp_layout` of property 0x09.
    sequence: *mut Sequence,
}

/// `StationChangeInfo`: define properties for stations.
unsafe fn station_change_info(
    c: &StationChangeCtx<'_>,
    first: u32,
    last: u32,
    prop: u32,
    reader: &mut Reader<'_>,
) -> Result<ChangeInfoResult, ReadError> {
    let (s, action2, loading, file, sequence) = (c.s, c.action2, c.loading, c.file, c.sequence);
    unsafe {
        if last > NUM_STATIONS_PER_GRF {
            grf_msg!(
                1,
                "StationChangeInfo: Station {} is invalid, max {}, ignoring",
                last,
                NUM_STATIONS_PER_GRF
            );
            return Ok(ChangeInfoResult::CIR_INVALID_ID);
        }
        // Allocate station specs if necessary
        if (s.station_count)(file) < last as usize {
            (s.station_resize)(file, last);
        }
        let mut result = ChangeInfoResult::CIR_SUCCESS;
        for id in first..last {
            let mut spec = (s.station_get)(file, id);
            // Check that the station we are modifying is defined.
            if spec.is_null() && prop != 0x08 {
                grf_msg!(
                    2,
                    "StationChangeInfo: Attempt to modify undefined station {}, ignoring",
                    id
                );
                return Ok(ChangeInfoResult::CIR_INVALID_ID);
            }
            match prop {
                // Class ID
                0x08 => {
                    // Property 0x08 is special; it is where the station is allocated
                    if spec.is_null() {
                        spec = (s.station_new)(file, id);
                    }
                    // Swap classid because we read it in BE meaning WAYP or DFLT
                    let class = StationClassID(crate::newgrf_class::station::allocate(
                        reader.dword()?.swap_bytes(),
                        crate::newgrf_station::STAT_CLASS_MAX,
                        s.empty_string,
                    ));
                    (s.station_class)(spec, class.0);
                }
                // Define sprite layout
                0x09 => {
                    let tiles = reader.extended()?;
                    // delete earlier loaded stuff
                    (s.station_render_clear)(spec);
                    (s.station_render_reserve)(spec, tiles);
                    for tile in 0..tiles {
                        let layout = (s.station_render_append)(spec);
                        let fields = (action2.layout_storage)(layout);
                        // Spritesets are unknown, so no limit.
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
                        // On error, bail out immediately. Temporary GRF data was already freed
                        if crate::newgrf_loading::is_disabled() {
                            return Ok(ChangeInfoResult::CIR_DISABLED);
                        }
                        (s.sequence_reset)(sequence);
                        loop {
                            let delta_x = reader.byte()?;
                            if delta_x == 0x80 {
                                break;
                            }
                            let item = (s.sequence_append)(sequence);
                            let fields = (action2.sequence_storage)(item);
                            // no relative bounding box support
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
                            // On error, bail out immediately. Temporary GRF data was already freed
                            if crate::newgrf_loading::is_disabled() {
                                return Ok(ChangeInfoResult::CIR_DISABLED);
                            }
                        }
                        (s.layout_sequence_move)(layout, sequence);
                        (s.sequence_reset)(sequence);
                    }
                    // Number of layouts must be even, alternating X and Y
                    if (s.station_render_size)(spec) & 1 != 0 {
                        grf_msg!(
                            1,
                            "StationChangeInfo: Station {} defines an odd number of sprite layouts, dropping the last item",
                            id
                        );
                        (s.station_render_pop)(spec);
                    }
                }
                // Copy sprite layout
                0x0a => {
                    let source_id = reader.extended()?;
                    let source = (s.station_get)(file, u32::from(source_id));
                    if source.is_null() {
                        grf_msg!(
                            1,
                            "StationChangeInfo: Station {} is not defined, cannot copy sprite layout to {}.",
                            source_id,
                            id
                        );
                        continue;
                    }
                    // delete earlier loaded stuff; this clears before observing
                    // the source, including self-copy.
                    (s.station_render_clear)(spec);
                    let count = (s.station_render_size)(source);
                    (s.station_render_reserve)(spec, count as u16);
                    for index in 0..count {
                        (s.station_render_copy_append)(spec, source, index);
                    }
                }
                // Callback mask
                0x0b => (s.station_callback_mask)(spec, reader.byte()?),
                // Disallowed number of platforms
                0x0c => (s.station_disallowed_platforms)(spec, reader.byte()?),
                // Disallowed platform lengths
                0x0d => (s.station_disallowed_lengths)(spec, reader.byte()?),
                // Define custom layout
                0x0e => {
                    while reader.has_data(1) {
                        let length = reader.byte()?;
                        let number = reader.byte()?;
                        if length == 0 || number == 0 {
                            break;
                        }
                        let count = usize::from(length) * usize::from(number);
                        let data = reader.bytes(count)?;
                        // Create entry in layouts and assign the layout to it.
                        let key = crate::newgrf_station::get_station_layout_key(number, length);
                        (s.station_custom_assign)(spec, key, data, count);
                        // Ensure the first bit, axis, is zero. The rest of the value is
                        // validated during rendering, as we don't know the range yet.
                        for index in 0..count {
                            let tile = data.add(index).read();
                            if tile & !1 != tile {
                                grf_msg!(
                                    1,
                                    "StationChangeInfo: Invalid tile {} in layout {}x{}",
                                    tile,
                                    length,
                                    number
                                );
                                (s.station_custom_tile)(spec, key, index, tile & !1);
                            }
                        }
                    }
                }
                // Copy custom layout
                0x0f => {
                    let source_id = reader.extended()?;
                    let source = (s.station_get)(file, u32::from(source_id));
                    if source.is_null() {
                        grf_msg!(
                            1,
                            "StationChangeInfo: Station {} is not defined, cannot copy tile layout to {}.",
                            source_id,
                            id
                        );
                        continue;
                    }
                    (s.station_custom_copy)(spec, source);
                }
                // Little/lots cargo threshold
                0x10 => (s.station_cargo_threshold)(spec, reader.word()?),
                // Pylon placement, overhead wire placement and blocked tiles
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
                        set(spec, index, has_bit(value, index));
                    }
                }
                // Cargo types for random triggers
                0x12 => {
                    let value = reader.dword()?;
                    let cargo = if ((*s.shared).grf_file_version)(file) >= 7 {
                        crate::engine_loading::translate_refit_mask(value, file)
                    } else {
                        CargoTypes(u64::from(value))
                    };
                    (s.station_cargo_triggers)(spec, cargo.0);
                }
                // General flags
                0x13 => (s.station_flags)(spec, StationSpecFlags(reader.byte()?).0),
                // Animation info
                0x16 => {
                    (s.shared().specs().station().animation().frames)(spec, reader.byte()?);
                    (s.shared().specs().station().animation().status)(spec, reader.byte()?);
                }
                // Animation speed
                0x17 => (s.shared().specs().station().animation().speed)(spec, reader.byte()?),
                // Animation triggers
                0x18 => (s.shared().specs().station().animation().triggers)(spec, reader.word()?),
                // 0x19 road routing (not implemented)

                // Advanced sprite layout
                0x1a => {
                    let tiles = reader.extended()?;
                    // delete earlier loaded stuff
                    (s.station_render_clear)(spec);
                    (s.station_render_reserve)(spec, tiles);
                    for _ in 0..tiles {
                        let layout = (s.station_render_append)(spec);
                        let count = reader.byte()?;
                        // On error, bail out immediately. Temporary GRF data was already freed
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
                            return Ok(ChangeInfoResult::CIR_DISABLED);
                        }
                    }
                    // Number of layouts must be even, alternating X and Y
                    if (s.station_render_size)(spec) & 1 != 0 {
                        grf_msg!(
                            1,
                            "StationChangeInfo: Station {} defines an odd number of sprite layouts, dropping the last item",
                            id
                        );
                        (s.station_render_pop)(spec);
                    }
                }
                // Minimum bridge height (not implemented)
                0x1b => {
                    for _ in 0..4 {
                        reader.word()?;
                    }
                }
                // Station Name
                0x1c => (s.station_name)(spec, reader.word()?),
                // Station Class name
                0x1d => (s.station_class_name)(spec, reader.word()?),
                // Extended tile flags (replaces prop 11, 14 and 15)
                0x1e => {
                    let tiles = usize::from(reader.extended()?);
                    let data = reader.bytes(tiles)?;
                    (s.station_tileflags_assign)(spec, data, tiles);
                }
                // Badge list
                0x1f => reader.badges(
                    GSF_STATIONS,
                    (s.shared().specs().station().badge_list)(spec),
                )?,
                // Minimum bridge height (extended) and disallowed bridge pillars
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
                _ => result = ChangeInfoResult::CIR_UNKNOWN,
            }
        }
        Ok(result)
    }
}
/// `IgnoreRoadStopProperty`: ignore properties for roadstops.
fn ignore_road_stop_property(
    prop: u32,
    reader: &mut Reader<'_>,
) -> Result<ChangeInfoResult, ReadError> {
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
        // Badge list
        0x16 => reader.skip_badges()?,
        _ => return Ok(ChangeInfoResult::CIR_UNKNOWN),
    }
    Ok(ChangeInfoResult::CIR_SUCCESS)
}
/// `RoadStopChangeInfo`: define properties for road stops.
unsafe fn road_stop_change_info(
    s: &Services,
    file: *mut c_void,
    first: u32,
    last: u32,
    prop: u32,
    reader: &mut Reader<'_>,
) -> Result<ChangeInfoResult, ReadError> {
    unsafe {
        if last > crate::newgrf_roadstop::NUM_ROADSTOPS_PER_GRF {
            grf_msg!(
                1,
                "RoadStopChangeInfo: RoadStop {} is invalid, max {}, ignoring",
                last,
                crate::newgrf_roadstop::NUM_ROADSTOPS_PER_GRF
            );
            return Ok(ChangeInfoResult::CIR_INVALID_ID);
        }
        if (s.road_count)(file) < last as usize {
            (s.road_resize)(file, last);
        }
        let mut result = ChangeInfoResult::CIR_SUCCESS;
        for id in first..last {
            let mut spec = (s.road_get)(file, id);
            if spec.is_null() && prop != 0x08 {
                grf_msg!(
                    1,
                    "RoadStopChangeInfo: Attempt to modify undefined road stop {}, ignoring",
                    id
                );
                result = result.max(ignore_road_stop_property(prop, reader)?);
                continue;
            }
            match prop {
                // Road Stop Class ID
                0x08 => {
                    if spec.is_null() {
                        spec = (s.road_new)(file, id);
                    }
                    // The class label is read with big-endian meaning: WAYP or DFLT.
                    let label = reader.dword()?.swap_bytes();
                    let class = RoadStopClassID(crate::newgrf_class::roadstop::allocate(
                        label,
                        crate::newgrf_roadstop::ROADSTOP_CLASS_MAX,
                        s.empty_string,
                    ));
                    (s.road_class)(spec, class.0);
                }
                // Road stop type
                0x09 => (s.road_stop_type)(spec, RoadStopAvailabilityType(reader.byte()?).0),
                // Road Stop Name
                0x0a => (s.road_name)(spec, reader.word()?),
                // Road Stop Class name
                0x0b => (s.road_class_name)(spec, reader.word()?),
                // The draw modes
                0x0c => (s.road_draw_mode)(spec, RoadStopDrawModes(reader.byte()?).0),
                // Cargo types for random triggers
                0x0d => {
                    let cargo = crate::engine_loading::translate_refit_mask(reader.dword()?, file);
                    (s.road_cargo_triggers)(spec, cargo.0);
                }
                // Animation info
                0x0e => {
                    (s.shared().specs().road_stop().animation().frames)(spec, reader.byte()?);
                    (s.shared().specs().road_stop().animation().status)(spec, reader.byte()?);
                }
                // Animation speed
                0x0f => (s.shared().specs().road_stop().animation().speed)(spec, reader.byte()?),
                // Animation triggers
                0x10 => (s.shared().specs().road_stop().animation().triggers)(spec, reader.word()?),
                // Callback mask
                0x11 => (s.road_callback_mask)(spec, reader.byte()?),
                // General flags. Future-proofing, size this as 4 bytes, but we only
                // need two byte's worth of flags at present. Preserve the native
                // narrowing point.
                0x12 => (s.road_flags)(spec, reader.dword()?),
                // Minimum bridge height and disallowed pillars for each of the
                // roadstop's tile layouts.
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
                // Cost multipliers
                0x15 => {
                    (s.road_build_cost)(spec, reader.byte()?);
                    (s.road_clear_cost)(spec, reader.byte()?);
                }
                // Badge list
                0x16 => reader.badges(
                    GSF_ROADSTOPS,
                    (s.shared().specs().road_stop().badge_list)(spec),
                )?,
                _ => result = ChangeInfoResult::CIR_UNKNOWN,
            }
        }
        Ok(result)
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_change_info_abi(
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
) -> u8 {
    let s = unsafe { &*s };
    let mut reader = unsafe { Reader::new(native, &*s.reader) };
    u8::from(
        match unsafe {
            station_change_info(
                &StationChangeCtx {
                    s,
                    action2: &*action2,
                    loading: &*loading,
                    file,
                    sequence,
                },
                first,
                last,
                prop,
                &mut reader,
            )
        } {
            Ok(value) => {
                unsafe {
                    output.write(value.0);
                }
                true
            }
            Err(_) => false,
        },
    )
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_roadstop_change_info_abi(
    s: *const Services,
    file: *mut c_void,
    first: u32,
    last: u32,
    prop: u32,
    native: *mut ByteReader,
    output: *mut u8,
) -> u8 {
    let s = unsafe { &*s };
    let mut reader = unsafe { Reader::new(native, &*s.reader) };
    u8::from(
        match unsafe { road_stop_change_info(s, file, first, last, prop, &mut reader) } {
            Ok(value) => {
                unsafe {
                    output.write(value.0);
                }
                true
            }
            Err(_) => false,
        },
    )
}

/// `CargoTypeMapSpriteGroupHandler<GSF_STATIONS>::MapSpecific`.
pub(crate) unsafe fn station_map_specific(
    s: *const Services,
    file: *mut c_void,
    id: u16,
    cid: u8,
    group: *const c_void,
) {
    let s = unsafe { &*s };
    // CargoTypeMapSpriteGroupHandler uses GSF_STATIONS for both native spec types.
    let cargo = CargoType(crate::newgrf_act3::translate_cargo(GSF_STATIONS, cid, file));
    if !crate::cargo_type::is_valid_cargo_type(cargo.0) {
        return;
    }
    let spec = unsafe { (s.station_get)(file, u32::from(id)) };
    if spec.is_null() {
        grf_msg!(1, "MapSpriteGroup: {} undefined, skipping", id);
    } else {
        unsafe {
            (s.station_set_group)(spec, cargo.0, group);
        }
    }
}
/// `CargoTypeMapSpriteGroupHandler<GSF_STATIONS>::MapDefault`.
pub(crate) unsafe fn station_map_default(
    s: *const Services,
    file: *mut c_void,
    id: u16,
    group: *const c_void,
) {
    let s = unsafe { &*s };
    let spec = unsafe { (s.station_get)(file, u32::from(id)) };
    if spec.is_null() {
        grf_msg!(1, "MapSpriteGroup: {} undefined, skipping", id);
    } else if (s.shared().specs().station().has_file)(spec) {
        grf_msg!(1, "MapSpriteGroup: {} mapped multiple times, skipping", id);
    } else {
        unsafe {
            (s.station_set_group)(spec, crate::newgrf_commons::SG_DEFAULT, group);
            (s.shared().specs().station().set_file)(spec, file);
            (s.shared().specs().station().set_local_id)(spec, id);
            crate::newgrf_class::station::assign(spec);
        }
    }
}

/// `CargoTypeMapSpriteGroupHandler<GSF_ROADSTOPS>::MapSpecific`.
pub(crate) unsafe fn roadstop_map_specific(
    s: *const Services,
    file: *mut c_void,
    id: u16,
    cid: u8,
    group: *const c_void,
) {
    let s = unsafe { &*s };
    // CargoTypeMapSpriteGroupHandler uses GSF_STATIONS for both native spec types.
    let cargo = CargoType(crate::newgrf_act3::translate_cargo(GSF_STATIONS, cid, file));
    if !crate::cargo_type::is_valid_cargo_type(cargo.0) {
        return;
    }
    let spec = unsafe { (s.road_get)(file, u32::from(id)) };
    if spec.is_null() {
        grf_msg!(1, "MapSpriteGroup: {} undefined, skipping", id);
    } else {
        unsafe {
            (s.road_set_group)(spec, cargo.0, group);
        }
    }
}
/// `CargoTypeMapSpriteGroupHandler<GSF_ROADSTOPS>::MapDefault`.
pub(crate) unsafe fn roadstop_map_default(
    s: *const Services,
    file: *mut c_void,
    id: u16,
    group: *const c_void,
) {
    let s = unsafe { &*s };
    let spec = unsafe { (s.road_get)(file, u32::from(id)) };
    if spec.is_null() {
        grf_msg!(1, "MapSpriteGroup: {} undefined, skipping", id);
    } else if (s.shared().specs().road_stop().has_file)(spec) {
        grf_msg!(1, "MapSpriteGroup: {} mapped multiple times, skipping", id);
    } else {
        unsafe {
            (s.road_set_group)(spec, crate::newgrf_commons::SG_DEFAULT, group);
            (s.shared().specs().road_stop().set_file)(spec, file);
            (s.shared().specs().road_stop().set_local_id)(spec, id);
            crate::newgrf_class::roadstop::assign(spec);
        }
    }
}
