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
use crate::debug::grf_msg;
use crate::direction_type::Direction;
use crate::newgrf_bytereader::{ByteReader, ReadError, Reader};
use crate::newgrf_internal::ChangeInfoResult;
use crate::override_manager::{self, Manager};
use crate::{animated_tile::AirportSpec as TileSpec, station_cmd::AirportSpec};
use std::ffi::c_void;
/// `AirportTileTable`: Tile-offset / `AirportTileID` pair.
#[repr(C)]
pub struct AirportTileTable {
    /// Tile offset from the top-most airport tile.
    pub ti: crate::map_type::TileIndexDiffC,
    /// `AirportTile` to use for this tile.
    pub gfx: u8,
}
/// `AirportTileLayout`: Airport tile layout.
struct AirportTileLayout {
    /// List of all tiles in this layout.
    tiles: Vec<AirportTileTable>,
    /// Rotation of this layout.
    rotation: Direction,
}
#[repr(C)]
pub struct Services {
    reader: *const crate::newgrf_bytereader::Services,
    shared: *const crate::services::Services,
    pub(crate) airport_count: unsafe extern "C" fn(*const c_void) -> usize,
    airport_resize: unsafe extern "C" fn(*mut c_void, usize) -> (),
    pub(crate) airport_get: unsafe extern "C" fn(*mut c_void, usize) -> *mut AirportSpec,
    airport_new: unsafe extern "C" fn(*mut c_void, usize, *const AirportSpec) -> *mut AirportSpec,
    airport_set_substitute: unsafe extern "C" fn(*mut AirportSpec, u16) -> (),
    pub(crate) tile_count: unsafe extern "C" fn(*const c_void) -> usize,
    tile_resize: unsafe extern "C" fn(*mut c_void, usize) -> (),
    pub(crate) tile_get: unsafe extern "C" fn(*mut c_void, usize) -> *mut TileSpec,
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
        unsafe extern "C" fn(*mut AirportSpec, usize, u8, *const AirportTileTable, usize) -> (),
    dimensions: unsafe extern "C" fn(*mut AirportSpec, u8, u8) -> (),
}
impl Services {
    fn shared(&self) -> &crate::services::Services {
        // SAFETY: `GetAirportDefinitionServices` points at the process-lifetime shared table.
        unsafe { &*self.shared }
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "AirportTileTable" ["probe"] => AirportTileTable { ti, gfx };
    "OpenTTDAirportDefinitionServices" ["probe"] => Services { reader, shared, airport_count, airport_resize, airport_get, airport_new, airport_set_substitute, tile_count, tile_resize, tile_get, tile_new, tile_set_substitute, airport_base, tile_base, airport_default, airport_clear, tile_default, tile_clear, airport_publish, tile_publish, airport_file_clear, airport_file_shrink, tile_file_clear, tile_file_shrink, airport_min_year, airport_max_year, airport_ttdp_type, airport_catchment, airport_noise, airport_maintenance, airport_name, tile_clear_animation, tile_callbacks, layouts_resize, layout_assign, dimensions };
}

/// `AirportChangeInfo`: define properties for airports.
#[allow(clippy::too_many_lines)] // The complete AirportChangeInfo property switch.
unsafe fn airport_change_info(
    s: &Services,
    file: *mut c_void,
    first: u32,
    last: u32,
    prop: u32,
    buf: &mut Reader<'_>,
) -> Result<ChangeInfoResult, ReadError> {
    unsafe {
        let mut ret = ChangeInfoResult::CIR_SUCCESS;

        if last > NUM_AIRPORTS_PER_GRF {
            grf_msg!(
                1,
                "AirportChangeInfo: Too many airports, trying id ({}), max ({}). Ignoring.",
                last,
                NUM_AIRPORTS_PER_GRF
            );
            return Ok(ChangeInfoResult::CIR_INVALID_ID);
        }

        /* Allocate industry specs if they haven't been allocated already. */
        if (s.airport_count)(file) < last as usize {
            (s.airport_resize)(file, last as usize);
        }

        for id in first..last {
            let mut r#as = (s.airport_get)(file, id as usize);

            if r#as.is_null() && prop != 0x08 && prop != 0x09 {
                grf_msg!(
                    2,
                    "AirportChangeInfo: Attempt to modify undefined airport {}, ignoring",
                    id
                );
                return Ok(ChangeInfoResult::CIR_INVALID_ID);
            }

            match prop {
                // Modify original airport
                0x08 => {
                    let subs_id = buf.byte()?;
                    if subs_id == 0xFF {
                        /* Instead of defining a new airport, an airport id
                         * of 0xFF disables the old airport with the current id. */
                        (s.shared().specs().airport().substitute().set_enabled)(
                            (s.airport_base)(u16::from(id as u8)),
                            false,
                        );
                        continue;
                    } else if subs_id >= NEW_AIRPORT_OFFSET {
                        /* The substitute id must be one of the original airports. */
                        grf_msg!(
                            2,
                            "AirportChangeInfo: Attempt to use new airport {} as substitute airport for {}. Ignoring.",
                            subs_id,
                            id
                        );
                        continue;
                    }

                    /* Allocate space for this airport.
                     * Only need to do it once. If ever it is called again, it should not
                     * do anything */
                    if r#as.is_null() {
                        r#as = (s.airport_new)(
                            file,
                            id as usize,
                            (s.airport_base)(u16::from(subs_id)),
                        );

                        (s.shared().specs().airport().substitute().set_enabled)(r#as, true);
                        (s.shared().specs().airport().set_local_id)(r#as, id as u16);
                        (s.airport_set_substitute)(r#as, u16::from(subs_id));
                        (s.shared().specs().airport().set_file)(r#as, file);
                        /* override the default airport */
                        override_manager::add(
                            override_manager::airport(),
                            id as u16,
                            (s.shared().grf_file_id)(file),
                            u32::from(subs_id),
                        );
                    }
                }

                // Set airport layout
                0x0A => {
                    let num_layouts = buf.byte()?;
                    buf.dword()?; // Total size of definition, unneeded.
                    let mut size_x = 0u8;
                    let mut size_y = 0u8;

                    let mut layouts = Vec::with_capacity(usize::from(num_layouts));

                    for _ in 0..num_layouts {
                        let mut layout = AirportTileLayout {
                            // Rotation can only be DIR_NORTH, DIR_EAST, DIR_SOUTH or DIR_WEST.
                            rotation: Direction(buf.byte()? & 6),
                            tiles: Vec::new(),
                        };

                        loop {
                            let mut x = i16::from(buf.byte()?);
                            let mut y = i16::from(buf.byte()?);
                            if x == 0 && y == 0x80 {
                                /* Terminator, remove and finish up. */
                                break;
                            }

                            let mut gfx = buf.byte()?;

                            if gfx == 0xFE {
                                /* Use a new tile from this GRF */
                                let local_tile_id = buf.word()?;

                                /* Read the ID from the _airporttile_mngr. */
                                let tempid = override_manager::get(
                                    override_manager::airport_tile(),
                                    local_tile_id,
                                    (s.shared().grf_file_id)(file),
                                );

                                if tempid == INVALID_AIRPORTTILE {
                                    grf_msg!(
                                        2,
                                        "AirportChangeInfo: Attempt to use airport tile {} with airport id {}, not yet defined. Ignoring.",
                                        local_tile_id,
                                        id
                                    );
                                } else {
                                    /* Declared as been valid, can be used */
                                    gfx = tempid as u8;
                                }
                            } else if gfx == 0xFF {
                                x = i16::from(x as i8);
                                y = i16::from(y as i8);
                            }

                            /* Determine largest size. */
                            // std::max<uint8_t> narrows the extent before comparing it.
                            if layout.rotation == Direction::DIR_E
                                || layout.rotation == Direction::DIR_W
                            {
                                size_x = size_x.max(y.wrapping_add(1) as u8);
                                size_y = size_y.max(x.wrapping_add(1) as u8);
                            } else {
                                size_x = size_x.max(x.wrapping_add(1) as u8);
                                size_y = size_y.max(y.wrapping_add(1) as u8);
                            }
                            layout.tiles.push(AirportTileTable {
                                ti: crate::map_type::TileIndexDiffC { x, y },
                                gfx,
                            });
                        }
                        layouts.push(layout);
                    }
                    (s.layouts_resize)(r#as, layouts.len());
                    for (index, layout) in layouts.iter().enumerate() {
                        (s.layout_assign)(
                            r#as,
                            index,
                            layout.rotation.0,
                            layout.tiles.as_ptr(),
                            layout.tiles.len(),
                        );
                    }
                    (s.dimensions)(r#as, size_x, size_y);
                }

                0x0C => {
                    (s.airport_min_year)(r#as, i32::from(buf.word()?));
                    let max_year = buf.word()?;
                    (s.airport_max_year)(
                        r#as,
                        if max_year == 0xFFFF {
                            crate::timer_game_common::MAX_YEAR
                        } else {
                            i32::from(max_year)
                        },
                    );
                }

                0x0D => (s.airport_ttdp_type)(r#as, buf.byte()?),

                0x0E => (s.airport_catchment)(
                    r#as,
                    buf.byte()?.clamp(1, crate::station_type::MAX_CATCHMENT),
                ),

                0x0F => (s.airport_noise)(r#as, buf.byte()?),

                0x10 => (s.airport_name)(r#as, buf.word()?),

                // Maintenance cost factor
                0x11 => (s.airport_maintenance)(r#as, buf.word()?),

                // Badge list
                0x12 => buf.badges(
                    crate::newgrf::GSF_AIRPORTS,
                    (s.shared().specs().airport().badge_list)(r#as),
                )?,

                _ => ret = ChangeInfoResult::CIR_UNKNOWN,
            }
        }

        Ok(ret)
    }
}
/// `AirportTilesChangeInfo`: define properties for airport tiles.
unsafe fn airport_tiles_change_info(
    s: &Services,
    file: *mut c_void,
    first: u32,
    last: u32,
    prop: u32,
    buf: &mut Reader<'_>,
) -> Result<ChangeInfoResult, ReadError> {
    unsafe {
        let mut ret = ChangeInfoResult::CIR_SUCCESS;

        if last > NUM_AIRPORTTILES_PER_GRF {
            grf_msg!(
                1,
                "AirportTileChangeInfo: Too many airport tiles loaded ({}), max ({}). Ignoring.",
                last,
                NUM_AIRPORTTILES_PER_GRF
            );
            return Ok(ChangeInfoResult::CIR_INVALID_ID);
        }

        /* Allocate airport tile specs if they haven't been allocated already. */
        if (s.tile_count)(file) < last as usize {
            (s.tile_resize)(file, last as usize);
        }

        for id in first..last {
            let mut tsp = (s.tile_get)(file, id as usize);

            if prop != 0x08 && tsp.is_null() {
                grf_msg!(
                    2,
                    "AirportTileChangeInfo: Attempt to modify undefined airport tile {}. Ignoring.",
                    id
                );
                return Ok(ChangeInfoResult::CIR_INVALID_ID);
            }

            match prop {
                // Substitute airport tile type
                0x08 => {
                    let subs_id = buf.byte()?;
                    if subs_id >= NEW_AIRPORTTILE_OFFSET {
                        /* The substitute id must be one of the original airport tiles. */
                        grf_msg!(
                            2,
                            "AirportTileChangeInfo: Attempt to use new airport tile {} as substitute airport tile for {}. Ignoring.",
                            subs_id,
                            id
                        );
                        continue;
                    }

                    /* Allocate space for this airport tile. */
                    if tsp.is_null() {
                        tsp = (s.tile_new)(file, id as usize, (s.tile_base)(subs_id));

                        (s.shared().specs().airport_tile().substitute().set_enabled)(tsp, true);

                        (s.tile_clear_animation)(tsp);

                        (s.shared().specs().airport_tile().set_local_id)(tsp, id as u16);
                        (s.tile_set_substitute)(tsp, u16::from(subs_id));
                        (s.shared().specs().airport_tile().set_file)(tsp, file);
                        // pre-reserve the tile slot
                        override_manager::airport_tile_add_entity_id(
                            override_manager::airport_tile(),
                            id as u16,
                            (s.shared().grf_file_id)(file),
                            u16::from(subs_id),
                        );
                    }
                }

                // Airport tile override
                0x09 => {
                    let override_id = buf.byte()?;

                    /* The airport tile being overridden must be an original airport tile. */
                    if override_id >= NEW_AIRPORTTILE_OFFSET {
                        grf_msg!(
                            2,
                            "AirportTileChangeInfo: Attempt to override new airport tile {} with airport tile id {}. Ignoring.",
                            override_id,
                            id
                        );
                        continue;
                    }

                    override_manager::add(
                        override_manager::airport_tile(),
                        id as u16,
                        (s.shared().grf_file_id)(file),
                        u32::from(override_id),
                    );
                }

                // Callback mask
                0x0E => (s.tile_callbacks)(tsp, buf.byte()?),

                // Animation information
                0x0F => {
                    (s.shared().specs().airport_tile().animation().frames)(tsp, buf.byte()?);
                    (s.shared().specs().airport_tile().animation().status)(tsp, buf.byte()?);
                }

                // Animation speed
                0x10 => (s.shared().specs().airport_tile().animation().speed)(tsp, buf.byte()?),

                // Animation triggers
                0x11 => (s.shared().specs().airport_tile().animation().triggers)(tsp, buf.byte()?),

                // Badge list. AirportTilesChangeInfo uses the tram feature for badge-seen
                // bookkeeping.
                0x12 => buf.badges(
                    crate::newgrf::GSF_TRAMTYPES,
                    (s.shared().specs().airport_tile().badge_list)(tsp),
                )?,

                _ => ret = ChangeInfoResult::CIR_UNKNOWN,
            }
        }

        Ok(ret)
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
        ) -> u8 {
            let s = unsafe { &*s };
            let mut reader = unsafe { Reader::new(native, &*s.reader) };
            u8::from(
                match unsafe { $parser(s, file, first, last, prop, &mut reader) } {
                    Ok(value) => {
                        unsafe {
                            result.write(value.0);
                        }
                        true
                    }
                    Err(ReadError) => false,
                },
            )
        }
    };
}
parser_entry!(openttd_rust_airport_change_info_abi, airport_change_info);
parser_entry!(
    openttd_rust_airport_tile_change_info_abi,
    airport_tiles_change_info
);

/// `AirportSpec::ResetAirports`: this function initializes the airportspec array.
pub unsafe fn reset_airports(s: *const Services) {
    let s = unsafe { &*s };
    unsafe {
        for id in 0..u16::from(NEW_AIRPORT_OFFSET) {
            (s.airport_default)(id);
        }
        for id in u16::from(NEW_AIRPORT_OFFSET)..NUM_AIRPORTS {
            (s.airport_clear)(id);
        }
        override_manager::reset_override(override_manager::airport());
    }
}
/// `AirportTileSpec::ResetAirportTiles`: this function initializes the tile array of
/// `AirportTileSpec`.
pub unsafe fn reset_airport_tiles(s: *const Services) {
    let s = unsafe { &*s };
    unsafe {
        for id in 0..u16::from(NEW_AIRPORTTILE_OFFSET) {
            (s.tile_default)(id);
        }
        for id in u16::from(NEW_AIRPORTTILE_OFFSET)..NUM_AIRPORTTILES {
            (s.tile_clear)(id);
        }

        /* Reset any overrides that have been set. */
        override_manager::reset_override(override_manager::airport_tile());
    }
}
macro_rules! publisher {
    ($(#[$doc:meta])* $entry:ident, $spec:ty, $add_entity_id:path, $specs:ident, $publish:ident, $base:ident, $full:literal) => {
        $(#[$doc])*
        unsafe fn $entry(manager: *mut Manager, spec: *mut $spec, s: *const Services) {
            let s = unsafe { &*s };
            let substitute = s.shared().specs().$specs().substitute();
            let identity = (substitute.identity)(spec);
            let (invalid, offset) = unsafe { ((*manager).invalid, (*manager).offset) };
            // SetEntitySpec narrows AddEntityID before comparing with the wide invalid ID.
            let id = unsafe {
                $add_entity_id(manager, identity.local, identity.grfid, identity.substitute)
            } as u8;
            if u16::from(id) == invalid {
                grf_msg!(1, $full);
                return;
            }
            unsafe {
                (s.$publish)(id, spec);
            }

            /* Now add the overrides. */
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
    /// `AirportOverrideManager::SetEntitySpec`: install the airport spec in its slot and
    /// apply the overrides of the original airports.
    airport_set_entity_spec,
    AirportSpec,
    override_manager::airport_add_entity_id,
    airport,
    airport_publish,
    airport_base,
    "Airport.SetEntitySpec: Too many airports allocated. Ignoring."
);
publisher!(
    /// `AirportTileOverrideManager::SetEntitySpec`: install the airport tile spec in its slot
    /// and apply the overrides of the original airport tiles.
    airport_tile_set_entity_spec,
    TileSpec,
    override_manager::airport_tile_add_entity_id,
    airport_tile,
    tile_publish,
    tile_base,
    "AirportTile.SetEntitySpec: Too many airport tiles allocated. Ignoring."
);

/// `FinaliseAirportsArray`: add all new airports to the airport array. Airport properties can
/// be set at any time in the GRF file, so we can only add an airport spec to the airport array
/// after the file has finished loading.
pub unsafe fn finalise_airports_array(s: &Services) {
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
                    airport_set_entity_spec(override_manager::airport(), spec, s);
                }
            }
        }
        let count = unsafe { (s.tile_count)(file) };
        for id in 0..count {
            let spec = unsafe { (s.tile_get)(file, id) };
            if !spec.is_null() && (s.shared().specs().airport_tile().substitute().enabled)(spec) {
                unsafe {
                    airport_tile_set_entity_spec(override_manager::airport_tile(), spec, s);
                }
            }
        }
        /* Won't be used again */
        unsafe {
            (s.airport_file_clear)(file);
            (s.airport_file_shrink)(file);
            (s.tile_file_clear)(file);
            (s.tile_file_shrink)(file);
        }
        index += 1;
    }
}
/// `BindAirportSpecs`: tie all airportspecs to their class.
pub unsafe fn bind_airport_specs(s: *const Services) {
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
