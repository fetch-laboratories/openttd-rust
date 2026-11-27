/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `NewGRF` Action 0x00 handlers for railtypes (`newgrf/newgrf_act0_railtypes.cpp`) and
//! roadtypes (`newgrf/newgrf_act0_roadtypes.cpp`).
//!
//! Each body reads the loader's current file (`_cur_gps.grffile`) through its handle and writes
//! the type it resolves through the type registry's table (`type_registry.rs`). A property is
//! stored as soon as it is read, so a read error part way through a property keeps every
//! store before it; only the C++ caller rethrows the error.
use crate::debug::grf_msg;
use crate::engine_type::VehicleAccelerationModel;
use crate::newgrf::{GRFFile, GRFFileRef, GrfSpecFeature};
use crate::newgrf_bytereader::{ByteReader, ReadError, Reader, Services as ReaderServices};
use crate::newgrf_internal::ChangeInfoResult;
use crate::newgrf_text_type::GRFStringID;
use crate::rail::RailTypeFlags;
use crate::rail_type::{RailType, RailTypeLabel};
use crate::road::{RoadTypeFlags, get_road_tram_type};
use crate::road_type::{RoadTramType, RoadType, RoadTypeLabel};
use crate::timer_game_calendar::CalendarDate;
use crate::type_registry::{
    allocate_rail_type, allocate_road_type, get_rail_type_by_label, get_road_type_by_label,
    services,
};

/// `GRFStringID{buf.ReadWord()}`.
fn grf_string_id(buf: &mut Reader<'_>) -> Result<GRFStringID, ReadError> {
    Ok(GRFStringID(u32::from(buf.word()?)))
}

/// `RailTypeChangeInfo`: define properties for railtypes.
#[allow(
    unsafe_code,
    reason = "elements of the file's live type map, and the type's live badge list"
)]
#[allow(
    clippy::cast_possible_wrap,
    reason = "`TimerGameCalendar::Date(buf.ReadDWord())` converts the `uint32_t` to the `int32_t` date"
)]
pub(crate) fn rail_type_change_info(
    grffile: GRFFileRef<'_>,
    first: u32,
    last: u32,
    prop: i32,
    buf: &mut Reader<'_>,
) -> Result<ChangeInfoResult, ReadError> {
    let mut ret = ChangeInfoResult::CIR_SUCCESS;

    let s = services();
    let rail = s.rail();
    let add_string_for_mapping = s.shared().add_string_for_mapping;
    let type_map = grffile.railtype_map();

    if last as usize > type_map.size() {
        grf_msg!(
            1,
            "RailTypeChangeInfo: Rail type {} is invalid, max {}, ignoring",
            last,
            type_map.size()
        );
        return Ok(ChangeInfoResult::CIR_INVALID_ID);
    }

    for id in first..last {
        // SAFETY: `id < last <= std::size(type_map)`.
        let rt = unsafe { type_map.at(id as usize) };
        if rt == RailType::INVALID_RAILTYPE {
            return Ok(ChangeInfoResult::CIR_INVALID_ID);
        }

        let rti = (rail.spec)(rt);

        match prop {
            // Label of rail type
            0x08 => {
                /* Skipped here as this is loaded during reservation stage. */
                buf.dword()?;
            }

            // Toolbar caption of railtype (sets name as well for backwards compatibility for grf ver < 8)
            0x09 => {
                let str = grf_string_id(buf)?;
                add_string_for_mapping(str, (rail.strings_toolbar_caption)(rti));
                if grffile.grf_version() < 8 {
                    add_string_for_mapping(str, (rail.strings_name)(rti));
                }
            }

            // Menu text of railtype
            0x0A => add_string_for_mapping(grf_string_id(buf)?, (rail.strings_menu_text)(rti)),

            // Build window caption
            0x0B => add_string_for_mapping(grf_string_id(buf)?, (rail.strings_build_caption)(rti)),

            // Autoreplace text
            0x0C => add_string_for_mapping(grf_string_id(buf)?, (rail.strings_replace_text)(rti)),

            // New locomotive text
            0x0D => add_string_for_mapping(grf_string_id(buf)?, (rail.strings_new_loco)(rti)),

            // 0x0E: Compatible railtype list
            // 0x0F: Powered railtype list
            // 0x18: Railtype list required for date introduction
            // 0x19: Introduced railtype list
            0x0E | 0x0F | 0x18 | 0x19 => {
                /* Rail type compatibility bits are added to the existing bits
                 * to allow multiple GRFs to modify compatibility with the
                 * default rail types. */
                let n = buf.byte()?;
                for _ in 0..n {
                    let label = buf.dword()?;
                    let resolved_rt =
                        get_rail_type_by_label(RailTypeLabel(label.swap_bytes()), false);
                    if resolved_rt != RailType::INVALID_RAILTYPE {
                        match prop {
                            0x0F => {
                                (rail.powered_railtypes_set)(rti, resolved_rt);
                                // Powered implies compatible.
                                (rail.compatible_railtypes_set)(rti, resolved_rt);
                            }
                            0x0E => (rail.compatible_railtypes_set)(rti, resolved_rt),
                            0x18 => (rail.introduction_required_railtypes_set)(rti, resolved_rt),
                            0x19 => (rail.introduces_railtypes_set)(rti, resolved_rt),
                            _ => {}
                        }
                    }
                }
            }

            // Rail Type flags
            0x10 => (rail.set_flags)(rti, RailTypeFlags(buf.byte()?)),

            // Curve speed advantage
            0x11 => (rail.set_curve_speed)(rti, buf.byte()?),

            // Station graphic
            0x12 => (rail.set_fallback_railtype)(rti, buf.byte()?.min(2)),

            // Construction cost factor
            0x13 => (rail.set_cost_multiplier)(rti, buf.word()?),

            // Speed limit
            0x14 => (rail.set_max_speed)(rti, buf.word()?),

            // Acceleration model
            0x15 => {
                (rail.set_acceleration_type)(rti, VehicleAccelerationModel(buf.byte()?.min(2)));
            }

            // Map colour
            0x16 => (rail.set_map_colour)(rti, buf.byte()?),

            // Introduction date
            0x17 => (rail.set_introduction_date)(rti, CalendarDate(buf.dword()? as i32)),

            // Sort order
            0x1A => (rail.set_sorting_order)(rti, buf.byte()?),

            // Name of railtype (overridden by prop 09 for grf ver < 8)
            0x1B => add_string_for_mapping(grf_string_id(buf)?, (rail.strings_name)(rti)),

            // Maintenance cost factor
            0x1C => (rail.set_maintenance_multiplier)(rti, buf.word()?),

            // Alternate rail type label list
            0x1D => {
                /* Skipped here as this is loaded during reservation stage. */
                for _ in 0..buf.byte()? {
                    buf.dword()?;
                }
            }

            // Badge list
            0x1E => {
                // SAFETY: The type's badge list is live and nothing else reaches it during
                // the read.
                unsafe { buf.badges(GrfSpecFeature::GSF_RAILTYPES.0, (rail.badges)(rti))? };
            }

            _ => ret = ChangeInfoResult::CIR_UNKNOWN,
        }
    }

    Ok(ret)
}

/// `RailTypeReserveInfo`: reserve railtypes by label, and skip every other property.
#[allow(unsafe_code, reason = "elements of the file's live type map")]
pub(crate) fn rail_type_reserve_info(
    grffile: GRFFileRef<'_>,
    first: u32,
    last: u32,
    prop: i32,
    buf: &mut Reader<'_>,
) -> Result<ChangeInfoResult, ReadError> {
    let mut ret = ChangeInfoResult::CIR_SUCCESS;

    let rail = services().rail();
    let type_map = grffile.railtype_map();

    if last as usize > type_map.size() {
        grf_msg!(
            1,
            "RailTypeReserveInfo: Rail type {} is invalid, max {}, ignoring",
            last,
            type_map.size()
        );
        return Ok(ChangeInfoResult::CIR_INVALID_ID);
    }

    for id in first..last {
        match prop {
            // Label of rail type
            0x08 => {
                let rtl = buf.dword()?;
                let rtl = RailTypeLabel(rtl.swap_bytes());

                let mut rt = get_rail_type_by_label(rtl, false);
                if rt == RailType::INVALID_RAILTYPE {
                    /* Set up new rail type */
                    rt = allocate_rail_type(rtl);
                }

                // SAFETY: `id < last <= std::size(type_map)`.
                unsafe { type_map.set(id as usize, rt) };
            }

            // 0x09: Toolbar caption of railtype
            // 0x0A: Menu text
            // 0x0B: Build window caption
            // 0x0C: Autoreplace text
            // 0x0D: New loco
            // 0x13: Construction cost
            // 0x14: Speed limit
            // 0x1B: Name of railtype
            // 0x1C: Maintenance cost factor
            0x09..=0x0D | 0x13 | 0x14 | 0x1B | 0x1C => {
                buf.word()?;
            }

            // Alternate rail type label list
            0x1D => {
                // SAFETY: `id < last <= std::size(type_map)`.
                let rt = unsafe { type_map.at(id as usize) };
                if rt == RailType::INVALID_RAILTYPE {
                    grf_msg!(
                        1,
                        "RailTypeReserveInfo: Ignoring property 1D for rail type {} because no label was set",
                        id
                    );
                    // Falls through to the label lists' skip.
                    for _ in 0..buf.byte()? {
                        buf.dword()?;
                    }
                } else {
                    let n = buf.byte()?;
                    let rti = (rail.spec)(rt);
                    for _ in 0..n {
                        (rail.alternate_labels_insert)(
                            rti,
                            RailTypeLabel(buf.dword()?.swap_bytes()),
                        );
                    }
                }
            }

            // 0x0E: Compatible railtype list
            // 0x0F: Powered railtype list
            // 0x18: Railtype list required for date introduction
            // 0x19: Introduced railtype list
            0x0E | 0x0F | 0x18 | 0x19 => {
                for _ in 0..buf.byte()? {
                    buf.dword()?;
                }
            }

            // 0x10: Rail Type flags
            // 0x11: Curve speed advantage
            // 0x12: Station graphic
            // 0x15: Acceleration model
            // 0x16: Map colour
            // 0x1A: Sort order
            0x10..=0x12 | 0x15 | 0x16 | 0x1A => {
                buf.byte()?;
            }

            // Introduction date
            0x17 => {
                buf.dword()?;
            }

            // Badge list
            0x1E => buf.skip_badges()?,

            _ => ret = ChangeInfoResult::CIR_UNKNOWN,
        }
    }

    Ok(ret)
}

/// `RoadTypeChangeInfo`: define properties for roadtypes.
#[allow(
    unsafe_code,
    reason = "elements of the file's live type map, and the type's live badge list"
)]
#[allow(
    clippy::cast_possible_wrap,
    reason = "`TimerGameCalendar::Date(buf.ReadDWord())` converts the `uint32_t` to the `int32_t` date"
)]
pub(crate) fn road_type_change_info(
    grffile: GRFFileRef<'_>,
    first: u32,
    last: u32,
    prop: i32,
    buf: &mut Reader<'_>,
    rtt: RoadTramType,
) -> Result<ChangeInfoResult, ReadError> {
    let mut ret = ChangeInfoResult::CIR_SUCCESS;

    let s = services();
    let road = s.road();
    let add_string_for_mapping = s.shared().add_string_for_mapping;
    let type_map = if rtt == RoadTramType::RTT_TRAM {
        grffile.tramtype_map()
    } else {
        grffile.roadtype_map()
    };

    if last as usize > type_map.size() {
        grf_msg!(
            1,
            "RoadTypeChangeInfo: Road type {} is invalid, max {}, ignoring",
            last,
            type_map.size()
        );
        return Ok(ChangeInfoResult::CIR_INVALID_ID);
    }

    for id in first..last {
        // SAFETY: `id < last <= std::size(type_map)`.
        let rt = unsafe { type_map.at(id as usize) };
        if rt == RoadType::INVALID_ROADTYPE {
            return Ok(ChangeInfoResult::CIR_INVALID_ID);
        }

        let rti = (road.spec)(rt);

        match prop {
            // Label of road type
            0x08 => {
                /* Skipped here as this is loaded during reservation stage. */
                buf.dword()?;
            }

            // Toolbar caption of roadtype
            0x09 => {
                add_string_for_mapping(grf_string_id(buf)?, (road.strings_toolbar_caption)(rti));
            }

            // Menu text of roadtype
            0x0A => add_string_for_mapping(grf_string_id(buf)?, (road.strings_menu_text)(rti)),

            // Build window caption
            0x0B => add_string_for_mapping(grf_string_id(buf)?, (road.strings_build_caption)(rti)),

            // Autoreplace text
            0x0C => add_string_for_mapping(grf_string_id(buf)?, (road.strings_replace_text)(rti)),

            // New engine text
            0x0D => add_string_for_mapping(grf_string_id(buf)?, (road.strings_new_engine)(rti)),

            // 0x0F: Powered roadtype list
            // 0x18: Roadtype list required for date introduction
            // 0x19: Introduced roadtype list
            0x0F | 0x18 | 0x19 => {
                /* Road type compatibility bits are added to the existing bits
                 * to allow multiple GRFs to modify compatibility with the
                 * default road types. */
                let n = buf.byte()?;
                for _ in 0..n {
                    let label = buf.dword()?;
                    let resolved_rt =
                        get_road_type_by_label(RoadTypeLabel(label.swap_bytes()), false);
                    if resolved_rt != RoadType::INVALID_ROADTYPE {
                        match prop {
                            0x0F => {
                                if get_road_tram_type(resolved_rt) == rtt {
                                    (road.powered_roadtypes_set)(rti, resolved_rt);
                                } else {
                                    grf_msg!(
                                        1,
                                        "RoadTypeChangeInfo: Powered road type list: Road type {} road/tram type does not match road type {}, ignoring",
                                        resolved_rt.0,
                                        rt.0
                                    );
                                }
                            }
                            0x18 => (road.introduction_required_roadtypes_set)(rti, resolved_rt),
                            0x19 => (road.introduces_roadtypes_set)(rti, resolved_rt),
                            _ => {}
                        }
                    }
                }
            }

            // Road Type flags
            0x10 => (road.set_flags)(rti, RoadTypeFlags(buf.byte()?)),

            // Construction cost factor
            0x13 => (road.set_cost_multiplier)(rti, buf.word()?),

            // Speed limit
            0x14 => (road.set_max_speed)(rti, buf.word()?),

            // Map colour
            0x16 => (road.set_map_colour)(rti, buf.byte()?),

            // Introduction date
            0x17 => (road.set_introduction_date)(rti, CalendarDate(buf.dword()? as i32)),

            // Sort order
            0x1A => (road.set_sorting_order)(rti, buf.byte()?),

            // Name of roadtype
            0x1B => add_string_for_mapping(grf_string_id(buf)?, (road.strings_name)(rti)),

            // Maintenance cost factor
            0x1C => (road.set_maintenance_multiplier)(rti, buf.word()?),

            // Alternate road type label list
            0x1D => {
                /* Skipped here as this is loaded during reservation stage. */
                for _ in 0..buf.byte()? {
                    buf.dword()?;
                }
            }

            // Badge list
            0x1E => {
                // SAFETY: The type's badge list is live and nothing else reaches it during
                // the read.
                unsafe { buf.badges(GrfSpecFeature::GSF_ROADTYPES.0, (road.badges)(rti))? };
            }

            _ => ret = ChangeInfoResult::CIR_UNKNOWN,
        }
    }

    Ok(ret)
}

/// `RoadTypeReserveInfo`: reserve roadtypes by label, and skip every other property.
#[allow(unsafe_code, reason = "elements of the file's live type map")]
pub(crate) fn road_type_reserve_info(
    grffile: GRFFileRef<'_>,
    first: u32,
    last: u32,
    prop: i32,
    buf: &mut Reader<'_>,
    rtt: RoadTramType,
) -> Result<ChangeInfoResult, ReadError> {
    let mut ret = ChangeInfoResult::CIR_SUCCESS;

    let road = services().road();
    let type_map = if rtt == RoadTramType::RTT_TRAM {
        grffile.tramtype_map()
    } else {
        grffile.roadtype_map()
    };

    if last as usize > type_map.size() {
        grf_msg!(
            1,
            "RoadTypeReserveInfo: Road type {} is invalid, max {}, ignoring",
            last,
            type_map.size()
        );
        return Ok(ChangeInfoResult::CIR_INVALID_ID);
    }

    for id in first..last {
        match prop {
            // Label of road type
            0x08 => {
                let rtl = buf.dword()?;
                let rtl = RoadTypeLabel(rtl.swap_bytes());

                let mut rt = get_road_type_by_label(rtl, false);
                if rt == RoadType::INVALID_ROADTYPE {
                    /* Set up new road type */
                    rt = allocate_road_type(rtl, rtt);
                } else if get_road_tram_type(rt) != rtt {
                    grf_msg!(
                        1,
                        "RoadTypeReserveInfo: Road type {} is invalid type (road/tram), ignoring",
                        id
                    );
                    return Ok(ChangeInfoResult::CIR_INVALID_ID);
                }

                // SAFETY: `id < last <= std::size(type_map)`.
                unsafe { type_map.set(id as usize, rt) };
            }
            // 0x09: Toolbar caption of roadtype
            // 0x0A: Menu text
            // 0x0B: Build window caption
            // 0x0C: Autoreplace text
            // 0x0D: New loco
            // 0x13: Construction cost
            // 0x14: Speed limit
            // 0x1B: Name of roadtype
            // 0x1C: Maintenance cost factor
            0x09..=0x0D | 0x13 | 0x14 | 0x1B | 0x1C => {
                buf.word()?;
            }

            // Alternate road type label list
            0x1D => {
                // SAFETY: `id < last <= std::size(type_map)`.
                let rt = unsafe { type_map.at(id as usize) };
                if rt == RoadType::INVALID_ROADTYPE {
                    grf_msg!(
                        1,
                        "RoadTypeReserveInfo: Ignoring property 1D for road type {} because no label was set",
                        id
                    );
                    /* FALL THROUGH */
                    for _ in 0..buf.byte()? {
                        buf.dword()?;
                    }
                } else {
                    let n = buf.byte()?;
                    let rti = (road.spec)(rt);
                    for _ in 0..n {
                        (road.alternate_labels_insert)(
                            rti,
                            RoadTypeLabel(buf.dword()?.swap_bytes()),
                        );
                    }
                }
            }

            // 0x0F: Powered roadtype list
            // 0x18: Roadtype list required for date introduction
            // 0x19: Introduced roadtype list
            0x0F | 0x18 | 0x19 => {
                for _ in 0..buf.byte()? {
                    buf.dword()?;
                }
            }

            // 0x10: Road Type flags
            // 0x16: Map colour
            // 0x1A: Sort order
            0x10 | 0x16 | 0x1A => {
                buf.byte()?;
            }

            // Introduction date
            0x17 => {
                buf.dword()?;
            }

            // Badge list
            0x1E => buf.skip_badges()?,

            _ => ret = ChangeInfoResult::CIR_UNKNOWN,
        }
    }

    Ok(ret)
}

/// The export's result: the `ChangeInfoResult`, or -1 for a read error the C++ caller
/// rethrows.
fn change_info_result(result: Result<ChangeInfoResult, ReadError>) -> i32 {
    result.map_or(-1, |result| i32::from(result.0))
}

/* Shims for `GrfChangeInfoHandler<GSF_RAILTYPES>`, `<GSF_ROADTYPES>` and `<GSF_TRAMTYPES>`,
 * which pass the loader's reader table, its current file and its reader. */

/// Shim for `RailTypeChangeInfo`.
///
/// # Safety
/// `reads` and `native` are the loader's live reader table and reader, and `file` is
/// `_cur_gps.grffile`, live while the loader reads the file's properties.
#[allow(
    unsafe_code,
    reason = "export; the loader's live reader and current file"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_change(
    reads: *const ReaderServices,
    file: *mut GRFFile,
    first: u32,
    last: u32,
    prop: i32,
    native: *mut ByteReader,
) -> i32 {
    // SAFETY: The loader passes its live reader with the reader table, and its current file,
    // live while it reads the file's properties.
    let mut buf = unsafe { Reader::new(native, &*reads) };
    let grffile = unsafe { GRFFileRef::of(services().shared(), file.cast_const()) };
    change_info_result(rail_type_change_info(grffile, first, last, prop, &mut buf))
}

/// Shim for `RailTypeReserveInfo`.
///
/// # Safety
/// `reads` and `native` are the loader's live reader table and reader, and `file` is
/// `_cur_gps.grffile`, live while the loader reads the file's properties.
#[allow(
    unsafe_code,
    reason = "export; the loader's live reader and current file"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_reserve(
    reads: *const ReaderServices,
    file: *mut GRFFile,
    first: u32,
    last: u32,
    prop: i32,
    native: *mut ByteReader,
) -> i32 {
    // SAFETY: As for `openttd_rust_rail_change`.
    let mut buf = unsafe { Reader::new(native, &*reads) };
    let grffile = unsafe { GRFFileRef::of(services().shared(), file.cast_const()) };
    change_info_result(rail_type_reserve_info(grffile, first, last, prop, &mut buf))
}

/// Shim for `RoadTypeChangeInfo`.
///
/// # Safety
/// `reads` and `native` are the loader's live reader table and reader, and `file` is
/// `_cur_gps.grffile`, live while the loader reads the file's properties.
#[allow(
    unsafe_code,
    reason = "export; the loader's live reader and current file"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_change(
    reads: *const ReaderServices,
    file: *mut GRFFile,
    first: u32,
    last: u32,
    prop: i32,
    native: *mut ByteReader,
    rtt: RoadTramType,
) -> i32 {
    // SAFETY: As for `openttd_rust_rail_change`.
    let mut buf = unsafe { Reader::new(native, &*reads) };
    let grffile = unsafe { GRFFileRef::of(services().shared(), file.cast_const()) };
    change_info_result(road_type_change_info(
        grffile, first, last, prop, &mut buf, rtt,
    ))
}

/// Shim for `RoadTypeReserveInfo`.
///
/// # Safety
/// `reads` and `native` are the loader's live reader table and reader, and `file` is
/// `_cur_gps.grffile`, live while the loader reads the file's properties.
#[allow(
    unsafe_code,
    reason = "export; the loader's live reader and current file"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_reserve(
    reads: *const ReaderServices,
    file: *mut GRFFile,
    first: u32,
    last: u32,
    prop: i32,
    native: *mut ByteReader,
    rtt: RoadTramType,
) -> i32 {
    // SAFETY: As for `openttd_rust_rail_change`.
    let mut buf = unsafe { Reader::new(native, &*reads) };
    let grffile = unsafe { GRFFileRef::of(services().shared(), file.cast_const()) };
    change_info_result(road_type_reserve_info(
        grffile, first, last, prop, &mut buf, rtt,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::newgrf_bytereader::Cursor;
    use crate::rail::RailTypeInfo;
    use crate::road::RoadTypeInfo;
    use crate::services::Services as Shared;
    use crate::strings_type::StringID;
    use crate::type_registry::{
        RailServices, RailTypeMap, RoadServices, RoadTypeMap, Services, TEST_RAIL, TEST_ROAD,
        exchange_services,
    };
    use std::cell::{Cell, RefCell};
    use std::ffi::c_void;
    use std::ptr;

    /// A member of an element, recorded as the store or string mapping that reached it.
    type Store = (&'static str, u64);

    thread_local! {
        static STORES: RefCell<Vec<Store>> = const { RefCell::new(Vec::new()) };
        static VERSION: Cell<u8> = const { Cell::new(8) };
        static RAIL_MAP: RefCell<[RailType; 64]> = const { RefCell::new([RailType(1); 64]) };
        static ROAD_MAP: RefCell<[RoadType; 63]> = const { RefCell::new([RoadType(1); 63]) };
    }

    /// The `StringID` members, by the code their fake address carries.
    const STRINGS: [&str; 7] = [
        "",
        "strings.toolbar_caption",
        "strings.menu_text",
        "strings.build_caption",
        "strings.replace_text",
        "strings.name",
        "strings.new_loco/new_engine",
    ];

    fn record(member: &'static str, value: u64) {
        STORES.with(|s| s.borrow_mut().push((member, value)));
    }
    /// The fake address of the `StringID` member `STRINGS[code]`.
    fn string(code: usize) -> *mut StringID {
        ptr::without_provenance_mut(code)
    }

    extern "C" fn add_string_for_mapping(source: GRFStringID, target: *mut StringID) {
        record(STRINGS[target.addr()], u64::from(source.0));
    }
    extern "C" fn grf_file_version(_: *const c_void) -> u8 {
        VERSION.with(Cell::get)
    }
    extern "C" fn grf_file_railtype_map(_: *mut c_void) -> RailTypeMap {
        RAIL_MAP.with(|m| RailTypeMap {
            data: m.borrow_mut().as_mut_ptr(),
            size: 64,
        })
    }
    extern "C" fn grf_file_roadtype_map(_: *mut c_void) -> RoadTypeMap {
        ROAD_MAP.with(|m| RoadTypeMap {
            data: m.borrow_mut().as_mut_ptr(),
            size: 63,
        })
    }
    /// The label of type 5 is `ABCD`; every other type is unlabelled.
    extern "C" fn rail_type_label(rti: *const RailTypeInfo) -> u32 {
        if rti.addr() == 5 {
            u32::from_be_bytes(*b"ABCD")
        } else {
            0
        }
    }

    extern "C" fn rail_spec(rt: RailType) -> *mut RailTypeInfo {
        ptr::without_provenance_mut(usize::from(rt.0))
    }
    extern "C" fn rail_toolbar(_: *mut RailTypeInfo) -> *mut StringID {
        string(1)
    }
    extern "C" fn rail_menu(_: *mut RailTypeInfo) -> *mut StringID {
        string(2)
    }
    extern "C" fn rail_build(_: *mut RailTypeInfo) -> *mut StringID {
        string(3)
    }
    extern "C" fn rail_replace(_: *mut RailTypeInfo) -> *mut StringID {
        string(4)
    }
    extern "C" fn rail_name(_: *mut RailTypeInfo) -> *mut StringID {
        string(5)
    }
    extern "C" fn rail_new_loco(_: *mut RailTypeInfo) -> *mut StringID {
        string(6)
    }
    extern "C" fn rail_compatible(_: *mut RailTypeInfo, rt: RailType) {
        record("compatible_railtypes.Set", u64::from(rt.0));
    }
    extern "C" fn rail_fallback(_: *mut RailTypeInfo, value: u8) {
        record("fallback_railtype", u64::from(value));
    }
    extern "C" fn rail_acceleration(_: *mut RailTypeInfo, value: VehicleAccelerationModel) {
        record("acceleration_type", u64::from(value.0));
    }

    extern "C" fn road_spec(rt: RoadType) -> *mut RoadTypeInfo {
        ptr::without_provenance_mut(usize::from(rt.0))
    }
    extern "C" fn road_toolbar(_: *mut RoadTypeInfo) -> *mut StringID {
        string(1)
    }
    extern "C" fn road_menu(_: *mut RoadTypeInfo) -> *mut StringID {
        string(2)
    }
    extern "C" fn road_build(_: *mut RoadTypeInfo) -> *mut StringID {
        string(3)
    }
    extern "C" fn road_replace(_: *mut RoadTypeInfo) -> *mut StringID {
        string(4)
    }
    extern "C" fn road_name(_: *mut RoadTypeInfo) -> *mut StringID {
        string(5)
    }
    extern "C" fn road_new_engine(_: *mut RoadTypeInfo) -> *mut StringID {
        string(6)
    }

    #[allow(unsafe_code, reason = "the test reader's cursor")]
    unsafe extern "C" fn read_byte(reader: *mut ByteReader, out: *mut u8) -> bool {
        // SAFETY: The test's reader is a live cursor and `out` its caller's local.
        unsafe {
            (*reader.cast::<Cursor>())
                .byte()
                .map(|v| out.write(v))
                .is_ok()
        }
    }
    #[allow(unsafe_code, reason = "the test reader's cursor")]
    unsafe extern "C" fn read_word(reader: *mut ByteReader, out: *mut u16) -> bool {
        // SAFETY: As for `read_byte`.
        unsafe {
            (*reader.cast::<Cursor>())
                .word()
                .map(|v| out.write(v))
                .is_ok()
        }
    }
    #[allow(unsafe_code, reason = "the test reader's cursor")]
    unsafe extern "C" fn read_dword(reader: *mut ByteReader, out: *mut u32) -> bool {
        // SAFETY: As for `read_byte`.
        unsafe {
            (*reader.cast::<Cursor>())
                .dword()
                .map(|v| out.write(v))
                .is_ok()
        }
    }
    #[allow(unsafe_code, reason = "a stub no row reaches")]
    unsafe extern "C" fn unreachable_reader<A, B>(_: A, _: B) -> bool {
        unreachable!("unexpected reader service")
    }
    #[allow(unsafe_code, reason = "a stub no row reaches")]
    unsafe extern "C" fn unreachable_reader3<A, B, C>(_: A, _: B, _: C) -> bool {
        unreachable!("unexpected reader service")
    }

    #[allow(unsafe_code, reason = "a stub no row reaches")]
    unsafe extern "C" fn assign_badges(
        _: *mut crate::newgrf_bytereader::BadgeList,
        _: *const u32,
        _: usize,
    ) {
        unreachable!("unexpected badge assignment")
    }
    #[allow(unsafe_code, reason = "a stub no row reaches")]
    unsafe extern "C" fn read_badge_ids(
        _: *mut ByteReader,
        _: u8,
        _: u16,
        _: *mut u32,
        _: *mut usize,
    ) -> bool {
        unreachable!("unexpected badge read")
    }
    #[allow(unsafe_code, reason = "a stub no row reaches")]
    unsafe extern "C" fn skip_badges(_: *mut ByteReader) -> bool {
        unreachable!("unexpected badge skip")
    }

    /// Install the recording registry, then run `body` over `bytes` for one property of type
    /// 1 at file ID 0, returning the result, the stores and whether every byte was read.
    #[allow(unsafe_code, reason = "the test's reader and stand-in file")]
    fn run(
        version: u8,
        bytes: &[u8],
        body: impl FnOnce(GRFFileRef<'_>, &mut Reader<'_>) -> Result<ChangeInfoResult, ReadError>,
    ) -> (ChangeInfoResult, Vec<Store>, bool) {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let shared: &'static Shared = Box::leak(Box::new(Shared {
            rail_type_label,
            grf_file_version,
            add_string_for_mapping,
            grf_file_railtype_map,
            grf_file_roadtype_map,
            ..crate::test_tables::SHARED
        }));
        let rail: &'static RailServices = Box::leak(Box::new(RailServices {
            spec: rail_spec,
            strings_toolbar_caption: rail_toolbar,
            strings_menu_text: rail_menu,
            strings_build_caption: rail_build,
            strings_replace_text: rail_replace,
            strings_name: rail_name,
            strings_new_loco: rail_new_loco,
            compatible_railtypes_set: rail_compatible,
            set_fallback_railtype: rail_fallback,
            set_acceleration_type: rail_acceleration,
            ..TEST_RAIL
        }));
        let road: &'static RoadServices = Box::leak(Box::new(RoadServices {
            spec: road_spec,
            strings_toolbar_caption: road_toolbar,
            strings_menu_text: road_menu,
            strings_build_caption: road_build,
            strings_replace_text: road_replace,
            strings_name: road_name,
            strings_new_engine: road_new_engine,
            ..TEST_ROAD
        }));
        let s: *const Services = Box::leak(Box::new(crate::type_registry::test_services(
            rail, road, shared,
        )));
        let reads = ReaderServices {
            has_data: unreachable_reader,
            peek_dword: unreachable_reader,
            read_bytes: unreachable_reader3,
            read_byte,
            read_word,
            read_dword,
            skip: unreachable_reader,
            assign_badges,
            read_badge_ids,
            skip_badges,
        };
        VERSION.with(|v| v.set(version));
        STORES.with(|s| s.borrow_mut().clear());
        let saved = exchange_services(s);
        let mut cursor = Cursor::new(bytes.as_ptr(), bytes.len());
        // SAFETY: The cursor and table outlive the reader, and the file is stand-in storage
        // only the stubs above receive.
        let mut reader = unsafe { Reader::new((&raw mut cursor).cast(), &reads) };
        let file = unsafe { GRFFileRef::of(shared, ptr::dangling::<GRFFile>()) };
        let result = body(file, &mut reader).expect("every row reads its whole property");
        exchange_services(saved);
        (result, STORES.with(RefCell::take), cursor.remaining() == 0)
    }

    /// The rail properties the scenario's files do not set, or set to one value: each string
    /// property maps its word into its own `StringID`, property 0x09 also into `name` below
    /// version 8, 0x0E sets only the compatible bit of the type its label names, and 0x12 and
    /// 0x15 clamp to 2.
    #[test]
    fn rail_type_change_info_stores_each_unvaried_property_in_its_member() {
        let word = [0x34, 0x12];
        let rows: [(u8, i32, &[u8], &[Store]); 10] = [
            (
                7,
                0x09,
                &word,
                &[
                    ("strings.toolbar_caption", 0x1234),
                    ("strings.name", 0x1234),
                ],
            ),
            (8, 0x09, &word, &[("strings.toolbar_caption", 0x1234)]),
            (8, 0x0A, &word, &[("strings.menu_text", 0x1234)]),
            (8, 0x0B, &word, &[("strings.build_caption", 0x1234)]),
            (8, 0x0C, &word, &[("strings.replace_text", 0x1234)]),
            (8, 0x0D, &word, &[("strings.new_loco/new_engine", 0x1234)]),
            (8, 0x1B, &word, &[("strings.name", 0x1234)]),
            (8, 0x0E, b"\x01ABCD", &[("compatible_railtypes.Set", 5)]),
            (8, 0x12, &[3], &[("fallback_railtype", 2)]),
            (8, 0x15, &[3], &[("acceleration_type", 2)]),
        ];
        for (version, prop, bytes, expected) in rows {
            let (result, stores, consumed) = run(version, bytes, |file, reader| {
                rail_type_change_info(file, 0, 1, prop, reader)
            });
            assert_eq!(result, ChangeInfoResult::CIR_SUCCESS, "property {prop:#x}");
            assert_eq!(stores, expected, "property {prop:#x} at version {version}");
            assert!(consumed, "property {prop:#x}");
        }
    }

    /// The road string properties, which the scenario's files do not set: each maps its word
    /// into its own `StringID`, and 0x09 into `toolbar_caption` only, at any version.
    #[test]
    fn road_type_change_info_stores_each_string_property_in_its_member() {
        let word = [0x34, 0x12];
        let rows: [(u8, i32, &[Store]); 7] = [
            (7, 0x09, &[("strings.toolbar_caption", 0x1234)]),
            (8, 0x09, &[("strings.toolbar_caption", 0x1234)]),
            (8, 0x0A, &[("strings.menu_text", 0x1234)]),
            (8, 0x0B, &[("strings.build_caption", 0x1234)]),
            (8, 0x0C, &[("strings.replace_text", 0x1234)]),
            (8, 0x0D, &[("strings.new_loco/new_engine", 0x1234)]),
            (8, 0x1B, &[("strings.name", 0x1234)]),
        ];
        for (version, prop, expected) in rows {
            let (result, stores, consumed) = run(version, &word, |file, reader| {
                road_type_change_info(file, 0, 1, prop, reader, RoadTramType::RTT_ROAD)
            });
            assert_eq!(result, ChangeInfoResult::CIR_SUCCESS, "property {prop:#x}");
            assert_eq!(stores, expected, "property {prop:#x} at version {version}");
            assert!(consumed, "property {prop:#x}");
        }
    }
}
