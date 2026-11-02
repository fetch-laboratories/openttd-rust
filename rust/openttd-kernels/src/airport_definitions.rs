/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Airport loading owns interpretation, allocation and ordered publication. Native payload
//! shells remain canonical. The game-state mutex serializes entries; no manager borrow
//! survives a native call, move or reset. Checked read failures return before native rethrow;
//! local Rust layout vectors drop on failure. FFI panics abort.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_arguments
)]
use crate::airport::{
    INVALID_AIRPORTTILE, NEW_AIRPORT_OFFSET, NEW_AIRPORTTILE_OFFSET, NUM_AIRPORTS,
    NUM_AIRPORTS_PER_GRF, NUM_AIRPORTTILES, NUM_AIRPORTTILES_PER_GRF,
};
use crate::newgrf_bytereader::{ByteReader, ReadError, Reader};
use crate::newgrf_internal::{CIR_INVALID_ID, CIR_SUCCESS, CIR_UNKNOWN};
use crate::override_manager::{self, Manager};
use crate::{animated_tile::AirportSpec as TileSpec, station_cmd::AirportSpec};
use std::ffi::c_void;
#[repr(C)]
pub struct LayoutTile {
    pub ti: crate::map_type::TileIndexDiffC,
    pub gfx: u8,
}
struct Layout {
    tiles: Vec<LayoutTile>,
    rotation: u8,
}
#[repr(C)]
pub struct Services {
    reader: *const crate::newgrf_bytereader::Services,
    shared: *const crate::services::Services,
    airport_count: unsafe extern "C" fn(*const c_void) -> usize,
    airport_resize: unsafe extern "C" fn(*mut c_void, usize) -> (),
    airport_get: unsafe extern "C" fn(*mut c_void, usize) -> *mut AirportSpec,
    airport_new: unsafe extern "C" fn(*mut c_void, usize, *const AirportSpec) -> *mut AirportSpec,
    airport_set_substitute: unsafe extern "C" fn(*mut AirportSpec, u16) -> (),
    tile_count: unsafe extern "C" fn(*const c_void) -> usize,
    tile_resize: unsafe extern "C" fn(*mut c_void, usize) -> (),
    tile_get: unsafe extern "C" fn(*mut c_void, usize) -> *mut TileSpec,
    tile_new: unsafe extern "C" fn(*mut c_void, usize, *const TileSpec) -> *mut TileSpec,
    tile_set_substitute: unsafe extern "C" fn(*mut TileSpec, u16) -> (),
    airport_base: unsafe extern "C" fn(u16) -> *mut AirportSpec,
    tile_base: unsafe extern "C" fn(u8) -> *mut TileSpec,
    airport_default: unsafe extern "C" fn(u16) -> (),
    airport_clear: unsafe extern "C" fn(u16) -> (),
    tile_default: unsafe extern "C" fn(u16) -> (),
    tile_clear: unsafe extern "C" fn(u16) -> (),
    airport_publish: unsafe extern "C" fn(u8, *mut AirportSpec) -> (),
    tile_publish: unsafe extern "C" fn(u8, *mut TileSpec) -> (),
    airport_file_clear: unsafe extern "C" fn(*mut c_void) -> (),
    airport_file_shrink: unsafe extern "C" fn(*mut c_void) -> (),
    tile_file_clear: unsafe extern "C" fn(*mut c_void) -> (),
    tile_file_shrink: unsafe extern "C" fn(*mut c_void) -> (),
    airport_min_year: unsafe extern "C" fn(*mut AirportSpec, i32) -> (),
    airport_max_year: unsafe extern "C" fn(*mut AirportSpec, i32) -> (),
    airport_ttdp_type: unsafe extern "C" fn(*mut AirportSpec, u8) -> (),
    airport_catchment: unsafe extern "C" fn(*mut AirportSpec, u8) -> (),
    airport_noise: unsafe extern "C" fn(*mut AirportSpec, u8) -> (),
    airport_maintenance: unsafe extern "C" fn(*mut AirportSpec, u16) -> (),
    airport_name: unsafe extern "C" fn(*mut AirportSpec, u16) -> (),
    tile_clear_animation: unsafe extern "C" fn(*mut TileSpec) -> (),
    tile_callbacks: unsafe extern "C" fn(*mut TileSpec, u8) -> (),
    layouts_resize: unsafe extern "C" fn(*mut AirportSpec, usize) -> (),
    layout_assign:
        unsafe extern "C" fn(*mut AirportSpec, usize, u8, *const LayoutTile, usize) -> (),
    dimensions: unsafe extern "C" fn(*mut AirportSpec, u8, u8) -> (),
    airport_invalid: unsafe extern "C" fn(u32) -> (),
    airport_undefined: unsafe extern "C" fn(u32) -> (),
    airport_substitute_invalid: unsafe extern "C" fn(u8, u32) -> (),
    layout_tile_undefined: unsafe extern "C" fn(u16, u32) -> (),
    airport_full: unsafe extern "C" fn() -> (),
    tile_invalid: unsafe extern "C" fn(u32) -> (),
    tile_undefined: unsafe extern "C" fn(u32) -> (),
    tile_substitute_invalid: unsafe extern "C" fn(u8, u32) -> (),
    tile_override_invalid: unsafe extern "C" fn(u8, u32) -> (),
    tile_full: unsafe extern "C" fn() -> (),
}
impl Services {
    fn shared(&self) -> &crate::services::Services {
        // SAFETY: `GetAirportDefinitionServices` points at the process-lifetime shared table.
        unsafe { &*self.shared }
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "AirportTileTable" ["probe"] => LayoutTile { ti, gfx };
    "OpenTTDAirportDefinitionServices" ["probe"] => Services { reader, shared, airport_count, airport_resize, airport_get, airport_new, airport_set_substitute, tile_count, tile_resize, tile_get, tile_new, tile_set_substitute, airport_base, tile_base, airport_default, airport_clear, tile_default, tile_clear, airport_publish, tile_publish, airport_file_clear, airport_file_shrink, tile_file_clear, tile_file_shrink, airport_min_year, airport_max_year, airport_ttdp_type, airport_catchment, airport_noise, airport_maintenance, airport_name, tile_clear_animation, tile_callbacks, layouts_resize, layout_assign, dimensions, airport_invalid, airport_undefined, airport_substitute_invalid, layout_tile_undefined, airport_full, tile_invalid, tile_undefined, tile_substitute_invalid, tile_override_invalid, tile_full };
}

unsafe extern "C" fn airport_valid(_: *mut c_void, _: u16) -> bool {
    true
}
unsafe extern "C" fn tile_valid(_: *mut c_void, id: u16) -> bool {
    crate::newgrf_commons::airport_tile_check_valid_new_id(id)
}
unsafe fn allocate(
    manager: *mut Manager,
    local: u16,
    grfid: u32,
    substitute: u16,
    tile: bool,
) -> u16 {
    let existing = unsafe { override_manager::get(manager, local, grfid) };
    unsafe {
        override_manager::openttd_rust_override_allocate(
            manager,
            local,
            grfid,
            substitute,
            std::ptr::null_mut(),
            existing,
            if tile { tile_valid } else { airport_valid },
        )
    }
}
#[allow(clippy::too_many_lines)] // The complete AirportChangeInfo property switch.
unsafe fn airport_change(
    s: &Services,
    file: *mut c_void,
    first: u32,
    last: u32,
    prop: u32,
    b: &mut Reader<'_>,
) -> Result<u8, ReadError> {
    unsafe {
        if last > NUM_AIRPORTS_PER_GRF {
            (s.airport_invalid)(last);
            return Ok(CIR_INVALID_ID as u8);
        }
        if (s.airport_count)(file) < last as usize {
            (s.airport_resize)(file, last as usize);
        }
        let mut result = CIR_SUCCESS as u8;
        for id in first..last {
            let mut spec = (s.airport_get)(file, id as usize);
            if spec.is_null() && prop != 0x08 && prop != 0x09 {
                (s.airport_undefined)(id);
                return Ok(CIR_INVALID_ID as u8);
            }
            match prop {
                0x08 => {
                    let substitute = b.byte()?;
                    if substitute == 0xff {
                        // AirportChangeInfo disables the original slot, including custom IDs.
                        (s.shared().specs().airport().substitute().set_enabled)(
                            (s.airport_base)(u16::from(id as u8)),
                            false,
                        );
                        continue;
                    } else if substitute >= NEW_AIRPORT_OFFSET {
                        (s.airport_substitute_invalid)(substitute, id);
                        continue;
                    }
                    if spec.is_null() {
                        spec = (s.airport_new)(
                            file,
                            id as usize,
                            (s.airport_base)(u16::from(substitute)),
                        );
                        (s.shared().specs().airport().substitute().set_enabled)(spec, true);
                        (s.shared().specs().airport().set_local_id)(spec, id as u16);
                        (s.airport_set_substitute)(spec, u16::from(substitute));
                        (s.shared().specs().airport().set_file)(spec, file);
                        override_manager::openttd_rust_override_add(
                            override_manager::airport(),
                            id as u16,
                            (s.shared().grf_file_id)(file),
                            u32::from(substitute),
                        );
                    }
                }
                0x0a => {
                    let count = b.byte()?;
                    b.dword()?;
                    let (mut size_x, mut size_y) = (0u8, 0u8);
                    let mut layouts = Vec::with_capacity(usize::from(count));
                    for _ in 0..count {
                        let mut layout = Layout {
                            rotation: b.byte()? & 6,
                            tiles: Vec::new(),
                        };
                        loop {
                            let (mut x, mut y) = (i16::from(b.byte()?), i16::from(b.byte()?));
                            if x == 0 && y == 0x80 {
                                break;
                            }
                            let mut gfx = b.byte()?;
                            if gfx == 0xfe {
                                let local = b.word()?;
                                let mapped = override_manager::get(
                                    override_manager::airport_tile(),
                                    local,
                                    (s.shared().grf_file_id)(file),
                                );
                                if mapped == INVALID_AIRPORTTILE {
                                    (s.layout_tile_undefined)(local, id);
                                } else {
                                    gfx = mapped as u8;
                                }
                            } else if gfx == 0xff {
                                x = i16::from(x as i8);
                                y = i16::from(y as i8);
                            }
                            let (extent_x, extent_y) = if layout.rotation
                                == crate::direction_type::DIR_E
                                || layout.rotation == crate::direction_type::DIR_W
                            {
                                (y, x)
                            } else {
                                (x, y)
                            };
                            // std::max<uint8_t> narrows the extent before comparing it.
                            size_x = size_x.max(extent_x.wrapping_add(1) as u8);
                            size_y = size_y.max(extent_y.wrapping_add(1) as u8);
                            layout.tiles.push(LayoutTile {
                                ti: crate::map_type::TileIndexDiffC { x, y },
                                gfx,
                            });
                        }
                        layouts.push(layout);
                    }
                    (s.layouts_resize)(spec, layouts.len());
                    for (index, layout) in layouts.iter().enumerate() {
                        (s.layout_assign)(
                            spec,
                            index,
                            layout.rotation,
                            layout.tiles.as_ptr(),
                            layout.tiles.len(),
                        );
                    }
                    (s.dimensions)(spec, size_x, size_y);
                }
                0x0c => {
                    (s.airport_min_year)(spec, i32::from(b.word()?));
                    let year = b.word()?;
                    (s.airport_max_year)(
                        spec,
                        if year == 0xffff {
                            crate::timer_game_common::MAX_YEAR
                        } else {
                            i32::from(year)
                        },
                    );
                }
                0x0d => (s.airport_ttdp_type)(spec, b.byte()?),
                0x0e => (s.airport_catchment)(
                    spec,
                    b.byte()?.clamp(1, crate::station_type::MAX_CATCHMENT),
                ),
                0x0f => (s.airport_noise)(spec, b.byte()?),
                0x10 => (s.airport_name)(spec, b.word()?),
                0x11 => (s.airport_maintenance)(spec, b.word()?),
                0x12 => b.badges(
                    crate::newgrf::GSF_AIRPORTS,
                    (s.shared().specs().airport().badge_list)(spec),
                )?,
                _ => result = CIR_UNKNOWN as u8,
            }
        }
        Ok(result)
    }
}
unsafe fn tile_change(
    s: &Services,
    file: *mut c_void,
    first: u32,
    last: u32,
    prop: u32,
    b: &mut Reader<'_>,
) -> Result<u8, ReadError> {
    unsafe {
        if last > NUM_AIRPORTTILES_PER_GRF {
            (s.tile_invalid)(last);
            return Ok(CIR_INVALID_ID as u8);
        }
        if (s.tile_count)(file) < last as usize {
            (s.tile_resize)(file, last as usize);
        }
        let mut result = CIR_SUCCESS as u8;
        for id in first..last {
            let mut spec = (s.tile_get)(file, id as usize);
            if prop != 0x08 && spec.is_null() {
                (s.tile_undefined)(id);
                return Ok(CIR_INVALID_ID as u8);
            }
            match prop {
                0x08 => {
                    let substitute = b.byte()?;
                    if substitute >= NEW_AIRPORTTILE_OFFSET {
                        (s.tile_substitute_invalid)(substitute, id);
                        continue;
                    }
                    if spec.is_null() {
                        spec = (s.tile_new)(file, id as usize, (s.tile_base)(substitute));
                        (s.shared().specs().airport_tile().substitute().set_enabled)(spec, true);
                        (s.tile_clear_animation)(spec);
                        (s.shared().specs().airport_tile().set_local_id)(spec, id as u16);
                        (s.tile_set_substitute)(spec, u16::from(substitute));
                        (s.shared().specs().airport_tile().set_file)(spec, file);
                        allocate(
                            override_manager::airport_tile(),
                            id as u16,
                            (s.shared().grf_file_id)(file),
                            u16::from(substitute),
                            true,
                        );
                    }
                }
                0x09 => {
                    let override_id = b.byte()?;
                    if override_id >= NEW_AIRPORTTILE_OFFSET {
                        (s.tile_override_invalid)(override_id, id);
                        continue;
                    }
                    override_manager::openttd_rust_override_add(
                        override_manager::airport_tile(),
                        id as u16,
                        (s.shared().grf_file_id)(file),
                        u32::from(override_id),
                    );
                }
                0x0e => (s.tile_callbacks)(spec, b.byte()?),
                0x0f => {
                    (s.shared().specs().airport_tile().animation().frames)(spec, b.byte()?);
                    (s.shared().specs().airport_tile().animation().status)(spec, b.byte()?);
                }
                0x10 => (s.shared().specs().airport_tile().animation().speed)(spec, b.byte()?),
                0x11 => (s.shared().specs().airport_tile().animation().triggers)(spec, b.byte()?),
                // AirportTilesChangeInfo uses the tram feature for badge-seen bookkeeping.
                0x12 => b.badges(
                    crate::newgrf::GSF_TRAMTYPES,
                    (s.shared().specs().airport_tile().badge_list)(spec),
                )?,
                _ => result = CIR_UNKNOWN as u8,
            }
        }
        Ok(result)
    }
}
macro_rules! parser_entry {
    ($entry:ident, $parser:ident) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $entry(
            s: *const Services,
            file: *mut c_void,
            first: u32,
            last: u32,
            prop: u32,
            native: *mut ByteReader,
            result: *mut u8,
        ) -> bool {
            let s = unsafe { &*s };
            let mut reader = unsafe { Reader::new(native, &*s.reader) };
            match unsafe { $parser(s, file, first, last, prop, &mut reader) } {
                Ok(value) => {
                    unsafe {
                        result.write(value);
                    }
                    true
                }
                Err(ReadError) => false,
            }
        }
    };
}
parser_entry!(openttd_rust_airport_change_info, airport_change);
parser_entry!(openttd_rust_airport_tile_change_info, tile_change);

pub unsafe fn reset(s: *const Services) {
    let s = unsafe { &*s };
    unsafe {
        for id in 0..u16::from(NEW_AIRPORT_OFFSET) {
            (s.airport_default)(id);
        }
        for id in u16::from(NEW_AIRPORT_OFFSET)..NUM_AIRPORTS {
            (s.airport_clear)(id);
        }
        override_manager::openttd_rust_override_reset(override_manager::airport());
    }
}
pub unsafe fn reset_tiles(s: *const Services) {
    let s = unsafe { &*s };
    unsafe {
        for id in 0..u16::from(NEW_AIRPORTTILE_OFFSET) {
            (s.tile_default)(id);
        }
        for id in u16::from(NEW_AIRPORTTILE_OFFSET)..NUM_AIRPORTTILES {
            (s.tile_clear)(id);
        }
        override_manager::openttd_rust_override_reset(override_manager::airport_tile());
    }
}
macro_rules! publisher {
    ($entry:ident, $spec:ty, $tile:literal, $specs:ident, $publish:ident, $base:ident, $full:ident) => {
        unsafe fn $entry(manager: *mut Manager, spec: *mut $spec, s: *const Services) {
            let s = unsafe { &*s };
            let substitute = s.shared().specs().$specs().substitute();
            let identity = (substitute.identity)(spec);
            let (invalid, offset) = unsafe { ((*manager).invalid, (*manager).offset) };
            // SetEntitySpec narrows AddEntityID before comparing with the wide invalid ID.
            let id = unsafe {
                allocate(
                    manager,
                    identity.local,
                    identity.grfid,
                    identity.substitute,
                    $tile,
                )
            } as u8;
            if u16::from(id) == invalid {
                unsafe {
                    (s.$full)();
                }
                return;
            }
            unsafe {
                (s.$publish)(id, spec);
            }
            for i in 0..offset {
                // Resolve the original slot before inspecting its override entry.
                let overridden = unsafe { (s.$base)(i as _) };
                let matched = unsafe {
                    (&(*manager).entities)[usize::from(i)] == identity.local
                        && (&(*manager).grfids)[usize::from(i)] == identity.grfid
                };
                if !matched {
                    continue;
                }
                unsafe {
                    (substitute.set_override)(overridden, u16::from(id));
                    (substitute.set_enabled)(overridden, false);
                    (&mut (*manager).entities)[usize::from(i)] = invalid;
                    (&mut (*manager).grfids)[usize::from(i)] = 0;
                }
            }
        }
    };
}
publisher!(
    publish,
    AirportSpec,
    false,
    airport,
    airport_publish,
    airport_base,
    airport_full
);
publisher!(
    publish_tile,
    TileSpec,
    true,
    airport_tile,
    tile_publish,
    tile_base,
    tile_full
);

pub unsafe fn finalise(s: &Services) {
    let mut index = 0;
    loop {
        let file = (s.shared().grf_file_at_or_null)(index);
        if file.is_null() {
            break;
        }
        let count = unsafe { (s.airport_count)(file) };
        for id in 0..count {
            let spec = unsafe { (s.airport_get)(file, id) };
            if !spec.is_null() && (s.shared().specs().airport().substitute().enabled)(spec) {
                unsafe {
                    publish(override_manager::airport(), spec, s);
                }
            }
        }
        let count = unsafe { (s.tile_count)(file) };
        for id in 0..count {
            let spec = unsafe { (s.tile_get)(file, id) };
            if !spec.is_null() && (s.shared().specs().airport_tile().substitute().enabled)(spec) {
                unsafe {
                    publish_tile(override_manager::airport_tile(), spec, s);
                }
            }
        }
        unsafe {
            (s.airport_file_clear)(file);
            (s.airport_file_shrink)(file);
            (s.tile_file_clear)(file);
            (s.tile_file_shrink)(file);
        }
        index += 1;
    }
}
pub unsafe fn bind(s: *const Services) {
    let s = unsafe { &*s };
    for id in 0..NUM_AIRPORTS {
        let spec = unsafe { (s.airport_base)(id) };
        if (s.shared().specs().airport().substitute().enabled)(spec) {
            unsafe {
                crate::newgrf_class::airport::assign(spec);
            }
        }
    }
}
