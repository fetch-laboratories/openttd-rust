/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Action0 industry and tile property interpretation (`newgrf_act0_industries.cpp`),
//! including incremental failure effects.
use super::Services;
use crate::bitmath_func::{gb, sb};
use crate::cargo_registry::get_cargo_translation;
use crate::cargo_type::{CT_INVALID, CargoType};
use crate::debug::grf_msg;
use crate::industry_type::{
    INDUSTRY_NUM_INPUTS, INDUSTRY_NUM_OUTPUTS, INDUSTRY_ORIGINAL_NUM_INPUTS,
    INDUSTRY_ORIGINAL_NUM_OUTPUTS, IndustryGfx, IndustryType,
};
use crate::industrytype::{
    self, CheckProc, IndustrySpec, IndustryTileSpec, LayoutTile, NativeVec, get_industry_spec,
    get_industry_tile_spec,
};
use crate::newgrf::GrfSpecFeature;
use crate::newgrf_bytereader::{ByteReader, ReadError, Reader, Services as ReaderServices};
use crate::newgrf_internal::ChangeInfoResult;
use crate::strings_type::StringID;
use crate::{newgrf_loading, override_manager};
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
    value: CT_INVALID,
    mixed: false,
};
/// `GRFFilePropsBase::SetGRFFile` with the context's non-null file.
fn set_grf_file(props: &mut industrytype::SubstituteProps, c: &Context) {
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
/// Ignore an industry tile property.
fn ignore_industry_tile_property(
    prop: i32,
    buf: &mut Reader<'_>,
) -> Result<ChangeInfoResult, ReadError> {
    match prop {
        0x09 | 0x0D | 0x0E | 0x10..=0x12 => {
            buf.byte()?;
        }
        0x0A..=0x0C | 0x0F => {
            buf.word()?;
        }
        0x13 => {
            let count = buf.byte()?;
            buf.skip(usize::from(count) * 2)?;
        }
        _ => return Ok(ChangeInfoResult::CIR_UNKNOWN),
    }
    Ok(ChangeInfoResult::CIR_SUCCESS)
}
/// Ignore an industry property.
fn ignore_industry_property(
    prop: i32,
    buf: &mut Reader<'_>,
) -> Result<ChangeInfoResult, ReadError> {
    match prop {
        0x09 | 0x0B | 0x0F | 0x12..=0x14 | 0x17..=0x19 | 0x21 | 0x22 => {
            buf.byte()?;
        }
        // 0x10: INDUSTRY_ORIGINAL_NUM_OUTPUTS bytes
        0x0C..=0x0E | 0x10 | 0x1B | 0x1F | 0x24 => {
            buf.word()?;
        }
        // 0x11: INDUSTRY_ORIGINAL_NUM_INPUTS bytes + 1
        0x11 | 0x1A | 0x1C..=0x1E | 0x20 | 0x23 => {
            buf.dword()?;
        }
        0x0A => {
            let num_table = buf.byte()?;
            for _ in 0..num_table {
                let mut k = 0_u32;
                loop {
                    let x = buf.byte()?;
                    if x == 0xFE && k == 0 {
                        buf.byte()?;
                        buf.byte()?;
                        break;
                    }
                    let y = buf.byte()?;
                    if x == 0 && y == 0x80 {
                        break;
                    }
                    let gfx = buf.byte()?;
                    if gfx == 0xFE {
                        buf.word()?;
                    }
                    k = k.wrapping_add(1);
                }
            }
        }
        0x16 => {
            for _ in 0..INDUSTRY_ORIGINAL_NUM_INPUTS {
                buf.byte()?;
            }
        }
        0x15 | 0x25..=0x27 => {
            let count = buf.byte()?;
            buf.skip(usize::from(count))?;
        }
        0x28 => {
            let num_inputs = buf.byte()?;
            let num_outputs = buf.byte()?;
            buf.skip(usize::from(num_inputs) * usize::from(num_outputs) * 2)?;
        }
        // Badge list
        0x29 => buf.skip_badges()?,
        _ => return Ok(ChangeInfoResult::CIR_UNKNOWN),
    }
    Ok(ChangeInfoResult::CIR_SUCCESS)
}
/// Validate the industry layout; e.g. to prevent duplicate tiles.
fn validate_industry_layout(layout: &[LayoutTile]) -> bool {
    let size = layout.len();
    if size == 0 {
        return false;
    }
    for i in 0..size - 1 {
        for j in i + 1..size {
            if layout[i].ti.x == layout[j].ti.x && layout[i].ti.y == layout[j].ti.y {
                return false;
            }
        }
    }
    layout
        .iter()
        .any(|tilelayout| IndustryGfx(tilelayout.gfx) != IndustryGfx::GFX_WATERTILE_SPECIALCHECK)
}
/// Industry property 0x0A: set industry layout(s).
unsafe fn read_industry_layouts(
    id: u32,
    indsp: *mut IndustrySpec,
    c: &Context,
    buf: &mut Reader<'_>,
    s: &Services,
    loading: &newgrf_loading::Services,
) -> Result<ChangeInfoResult, ReadError> {
    let new_num_layouts = buf.byte()?;
    let mut definition_size = buf.dword()?;
    let mut bytes_read = 0_u32;
    let mut new_layouts = Vec::new();
    let mut layout = Vec::new();
    for _ in 0..new_num_layouts {
        layout.clear();
        let mut k = 0_u32;
        loop {
            if bytes_read >= definition_size {
                grf_msg!(
                    3,
                    "IndustriesChangeInfo: Incorrect size for industry tile layout definition for industry {}.",
                    id
                );
                // Avoid warning twice
                definition_size = u32::MAX;
            }
            let mut it = LayoutTile {
                ti: crate::map_type::TileIndexDiffC {
                    // Offsets from northernmost tile
                    x: i16::from(buf.byte()?),
                    y: 0,
                },
                gfx: 0,
            };
            bytes_read = bytes_read.wrapping_add(1);
            if it.ti.x == 0xFE && k == 0 {
                // This means we have to borrow the layout from an old industry
                let kind = IndustryType(buf.byte()?);
                let laynbr = buf.byte()?;
                bytes_read = bytes_read.wrapping_add(2);
                // `lengthof(_origin_industry_specs)` is `NEW_INDUSTRYOFFSET`.
                if kind >= IndustryType::NEW_INDUSTRYOFFSET {
                    unsafe {
                        grf_msg!(
                            1,
                            "IndustriesChangeInfo: Invalid original industry number for layout import, industry {}",
                            id
                        );
                        newgrf_loading::disable_invalid_id(loading);
                    }
                    return Ok(ChangeInfoResult::CIR_DISABLED);
                }
                let originals = unsafe { (*s.origin.add(usize::from(kind.0))).layouts.as_slice() };
                if usize::from(laynbr) >= originals.len() {
                    unsafe {
                        grf_msg!(
                            1,
                            "IndustriesChangeInfo: Invalid original industry layout index for layout import, industry {}",
                            id
                        );
                        newgrf_loading::disable_invalid_id(loading);
                    }
                    return Ok(ChangeInfoResult::CIR_DISABLED);
                }
                // `k == 0`, so the layout is still empty and this replaces it.
                layout.extend_from_slice(originals[usize::from(laynbr)].as_slice());
                break;
            }
            // Or table definition finalisation
            it.ti.y = i16::from(buf.byte()?);
            bytes_read = bytes_read.wrapping_add(1);
            if it.ti.x == 0 && it.ti.y == 0x80 {
                // Terminator, remove and finish up
                break;
            }
            it.gfx = u16::from(buf.byte()?);
            bytes_read = bytes_read.wrapping_add(1);
            if it.gfx == 0xFE {
                // Use a new tile from this GRF
                let local_tile_id = buf.word()?;
                bytes_read = bytes_read.wrapping_add(2);
                // Read the ID from the _industile_mngr.
                let tempid = IndustryGfx(unsafe {
                    override_manager::get(override_manager::tile(), local_tile_id, c.grfid)
                });
                if tempid == IndustryGfx::INVALID_INDUSTRYTILE {
                    grf_msg!(
                        2,
                        "IndustriesChangeInfo: Attempt to use industry tile {} with industry id {}, not yet defined. Ignoring.",
                        local_tile_id,
                        id
                    );
                } else {
                    // Declared as been valid, can be used
                    it.gfx = tempid.0;
                }
            } else if IndustryGfx(it.gfx) == IndustryGfx::GFX_WATERTILE_SPECIALCHECK {
                it.ti.x = i16::from(gb(it.ti.x as u32, 0, 8) as i8);
                it.ti.y = i16::from(gb(it.ti.y as u32, 0, 8) as i8);
                // When there were only 256x256 maps, TileIndex was a uint16_t and
                // it.ti was just a TileIndexDiff that was added to it.
                // As such negative "x" values were shifted into the "y" position.
                //   x = -1, y = 1 -> x = 255, y = 0
                // Since GRF version 8 the position is interpreted as pair of independent int8.
                // For GRF version < 8 we need to emulate the old shifting behaviour.
                if c.version < 8 && it.ti.x < 0 {
                    it.ti.y = it.ti.y.wrapping_add(1);
                }
            }
            layout.push(it);
            k = k.wrapping_add(1);
        }
        if validate_industry_layout(&layout) {
            new_layouts.push(NativeVec::from_vec(layout.clone()));
        } else {
            // The industry layout was not valid, so skip this one.
            grf_msg!(
                1,
                "IndustriesChangeInfo: Invalid industry layout for industry id {}. Ignoring",
                id
            );
        }
    }
    // Install final layout construction in the industry spec
    unsafe { (*indsp).layouts = NativeVec::from_vec(new_layouts) };
    Ok(ChangeInfoResult::CIR_SUCCESS)
}
/// Define properties for industry tiles, local IDs `first..last`.
unsafe fn industrytiles_change_info(
    ids: std::ops::Range<u32>,
    prop: i32,
    c: &Context,
    buf: &mut Reader<'_>,
    loading: &newgrf_loading::Services,
) -> Result<ChangeInfoResult, ReadError> {
    let last = ids.end;
    if last > u32::from(IndustryGfx::NUM_INDUSTRYTILES_PER_GRF.0) {
        grf_msg!(
            1,
            "IndustryTilesChangeInfo: Too many industry tiles loaded ({}), max ({}). Ignoring.",
            last,
            IndustryGfx::NUM_INDUSTRYTILES_PER_GRF.0
        );
        return Ok(ChangeInfoResult::CIR_INVALID_ID);
    }
    // Allocate industry tile specs if they haven't been allocated already.
    let specs = unsafe { industrytype::file_specs(c.file) };
    if specs.tiles.len() < last as usize {
        specs.tiles.resize_with(last as usize, || None);
    }
    let mut ret = ChangeInfoResult::CIR_SUCCESS;
    for id in ids {
        let mut tsp = file_tile(c.file, id);
        if prop != 0x08 && tsp.is_null() {
            // `if (cir > ret) ret = cir;`
            ret = ret.max(ignore_industry_tile_property(prop, buf)?);
            continue;
        }
        match prop {
            // Substitute industry tile type
            0x08 => {
                let subs_id = buf.byte()?;
                if IndustryGfx(u16::from(subs_id)) >= IndustryGfx::NEW_INDUSTRYTILEOFFSET {
                    // The substitute id must be one of the original industry tile.
                    grf_msg!(
                        2,
                        "IndustryTilesChangeInfo: Attempt to use new industry tile {} as substitute industry tile for {}. Ignoring.",
                        subs_id,
                        id
                    );
                    continue;
                }
                // Allocate space for this industry.
                if tsp.is_null() {
                    // A copy of the current, not the original, substitute tile.
                    let mut tile = unsafe {
                        (*get_industry_tile_spec(IndustryGfx(u16::from(subs_id)))).clone()
                    };
                    tile.enabled = true;
                    // A copied tile should not have the animation infos copied too.
                    // The anim_state should be left untouched, though
                    // It is up to the author to animate them
                    tile.anim_production = IndustryGfx::INDUSTRYTILE_NOANIM.0 as u8;
                    tile.anim_next = IndustryGfx::INDUSTRYTILE_NOANIM.0 as u8;
                    tile.grf_prop.local_id = id as u16;
                    tile.grf_prop.subst_id = u16::from(subs_id);
                    set_grf_file(&mut tile.grf_prop, c);
                    unsafe { industrytype::file_specs(c.file) }.tiles[id as usize] =
                        Some(Box::new(tile));
                    tsp = file_tile(c.file, id);
                    // pre-reserve the tile slot
                    unsafe {
                        super::allocate_tile(id as u16, c.grfid, u16::from(subs_id));
                    }
                }
            }
            // Industry tile override
            0x09 => {
                let ovrid = buf.byte()?;
                // The industry being overridden must be an original industry.
                if IndustryGfx(u16::from(ovrid)) >= IndustryGfx::NEW_INDUSTRYTILEOFFSET {
                    grf_msg!(
                        2,
                        "IndustryTilesChangeInfo: Attempt to override new industry tile {} with industry tile id {}. Ignoring.",
                        ovrid,
                        id
                    );
                    continue;
                }
                unsafe {
                    override_manager::openttd_rust_override_add(
                        override_manager::tile(),
                        id as u16,
                        c.grfid,
                        u32::from(ovrid),
                    );
                }
            }
            // Tile acceptance
            0x0A..=0x0C => {
                let acctp = u32::from(buf.word()?);
                let slot = (prop - 0x0A) as usize;
                let cargo = get_cargo_translation(gb(acctp, 0, 8) as u8, c.file, false);
                unsafe {
                    (*tsp).accepts_cargo[slot] = cargo.0;
                    (*tsp).acceptance[slot] = gb(acctp, 8, 8).clamp(0, 16) as i8;
                    (*tsp).accepts_cargo_label[slot] = INVALID_LABEL;
                }
            }
            // Land shape flags
            0x0D => unsafe {
                (*tsp).slopes_refused = buf.byte()?;
            },
            // Callback mask
            0x0E => unsafe {
                (*tsp).callback_mask = buf.byte()?;
            },
            // Animation information
            0x0F => unsafe {
                (*tsp).animation.frames = buf.byte()?;
                (*tsp).animation.status = buf.byte()?;
            },
            // Animation speed
            0x10 => unsafe {
                (*tsp).animation.speed = buf.byte()?;
            },
            // Triggers for callback 25
            0x11 => unsafe {
                (*tsp).animation.triggers = buf.byte()?;
            },
            // Special flags
            0x12 => unsafe {
                (*tsp).special_flags = buf.byte()?;
            },
            // variable length cargo acceptance
            0x13 => {
                let num_cargoes = usize::from(buf.byte()?);
                if num_cargoes > INDUSTRY_NUM_INPUTS {
                    unsafe {
                        newgrf_loading::disable_list_property(loading, prop as u32);
                    }
                    return Ok(ChangeInfoResult::CIR_DISABLED);
                }
                for i in 0..INDUSTRY_NUM_INPUTS {
                    if i < num_cargoes {
                        let cargo = get_cargo_translation(buf.byte()?, c.file, false);
                        unsafe { (*tsp).accepts_cargo[i] = cargo.0 };
                        // Tile acceptance can be negative to counteract the
                        // IndustryTileSpecialFlag::AcceptsAllCargo flag
                        let acceptance = buf.byte()? as i8;
                        unsafe { (*tsp).acceptance[i] = acceptance };
                    } else {
                        unsafe {
                            (*tsp).accepts_cargo[i] = CargoType::INVALID_CARGO.0;
                            (*tsp).acceptance[i] = 0;
                        }
                    }
                    if i < INDUSTRY_ORIGINAL_NUM_INPUTS {
                        unsafe { (*tsp).accepts_cargo_label[i] = INVALID_LABEL };
                    }
                }
            }
            // Badge list
            0x14 => {
                let ids = buf.badge_ids(GrfSpecFeature::GSF_INDUSTRYTILES.0)?;
                unsafe { (*tsp).badges = NativeVec::from_vec(ids) };
            }
            _ => ret = ChangeInfoResult::CIR_UNKNOWN,
        }
    }
    Ok(ret)
}
/// Define properties for industries, local IDs `first..last`.
unsafe fn industries_change_info(
    ids: std::ops::Range<u32>,
    prop: i32,
    c: &Context,
    buf: &mut Reader<'_>,
    loading: &newgrf_loading::Services,
) -> Result<ChangeInfoResult, ReadError> {
    let last = ids.end;
    if last > u32::from(IndustryType::NUM_INDUSTRYTYPES_PER_GRF.0) {
        grf_msg!(
            1,
            "IndustriesChangeInfo: Too many industries loaded ({}), max ({}). Ignoring.",
            last,
            IndustryType::NUM_INDUSTRYTYPES_PER_GRF.0
        );
        return Ok(ChangeInfoResult::CIR_INVALID_ID);
    }
    let s = unsafe { &*loading.industry };
    // Allocate industry specs if they haven't been allocated already.
    let specs = unsafe { industrytype::file_specs(c.file) };
    if specs.industries.len() < last as usize {
        specs.industries.resize_with(last as usize, || None);
    }
    let mut ret = ChangeInfoResult::CIR_SUCCESS;
    for id in ids {
        let mut indsp = file_industry(c.file, id);
        if prop != 0x08 && indsp.is_null() {
            // `if (cir > ret) ret = cir;`
            ret = ret.max(ignore_industry_property(prop, buf)?);
            continue;
        }
        match prop {
            // Substitute industry type
            0x08 => {
                let subs_id = buf.byte()?;
                if subs_id == 0xFF {
                    // Instead of defining a new industry, a substitute industry id
                    // of 0xFF disables the old industry with the current id.
                    unsafe { (*get_industry_spec(IndustryType(id as u8))).enabled = false };
                    continue;
                }
                if IndustryType(subs_id) >= IndustryType::NEW_INDUSTRYOFFSET {
                    // The substitute id must be one of the original industry.
                    grf_msg!(
                        2,
                        "_industry_specs: Attempt to use new industry {} as substitute industry for {}. Ignoring.",
                        subs_id,
                        id
                    );
                    continue;
                }
                // Allocate space for this industry.
                // Only need to do it once. If ever it is called again, it should not
                // do anything
                if indsp.is_null() {
                    let mut industry = unsafe { (*s.origin.add(usize::from(subs_id))).clone() };
                    industry.enabled = true;
                    industry.grf_prop.local_id = id as u16;
                    industry.grf_prop.subst_id = u16::from(subs_id);
                    set_grf_file(&mut industry.grf_prop, c);
                    // If the grf industry needs to check its surrounding upon creation, it should
                    // rely on callbacks, not on the original placement functions
                    industry.check_proc = CheckProc::CHECK_NOTHING.0;
                    unsafe { industrytype::file_specs(c.file) }.industries[id as usize] =
                        Some(Box::new(industry));
                    indsp = file_industry(c.file, id);
                }
            }
            // Industry type override
            0x09 => {
                let ovrid = buf.byte()?;
                // The industry being overridden must be an original industry.
                if IndustryType(ovrid) >= IndustryType::NEW_INDUSTRYOFFSET {
                    grf_msg!(
                        2,
                        "IndustriesChangeInfo: Attempt to override new industry {} with industry id {}. Ignoring.",
                        ovrid,
                        id
                    );
                    continue;
                }
                unsafe {
                    (*indsp).grf_prop.override_id = u16::from(ovrid);
                    override_manager::openttd_rust_override_add(
                        override_manager::industry(),
                        id as u16,
                        c.grfid,
                        u32::from(ovrid),
                    );
                }
            }
            // Set industry layout(s)
            0x0A => {
                let status = unsafe { read_industry_layouts(id, indsp, c, buf, s, loading) }?;
                if status == ChangeInfoResult::CIR_DISABLED {
                    return Ok(status);
                }
            }
            // Industry production flags
            0x0B => unsafe {
                (*indsp).life_type = buf.byte()?;
            },
            // Industry closure message
            0x0C => unsafe {
                (s.map_string)(buf.word()?, &raw mut (*indsp).closure_text);
            },
            // Production increase message
            0x0D => unsafe {
                (s.map_string)(buf.word()?, &raw mut (*indsp).production_up_text);
            },
            // Production decrease message
            0x0E => unsafe {
                (s.map_string)(buf.word()?, &raw mut (*indsp).production_down_text);
            },
            // Fund cost multiplier
            0x0F => unsafe {
                (*indsp).cost_multiplier = buf.byte()?;
            },
            // 0x10: production cargo types; 0x11: acceptance cargo types
            0x10 | 0x11 => {
                let produced = prop == 0x10;
                let count = if produced {
                    INDUSTRY_ORIGINAL_NUM_OUTPUTS
                } else {
                    INDUSTRY_ORIGINAL_NUM_INPUTS
                };
                for j in 0..count {
                    let cargo = get_cargo_translation(buf.byte()?, c.file, false);
                    if produced {
                        unsafe {
                            (*indsp).produced_cargo[j] = cargo.0;
                            (*indsp).produced_cargo_label[j] = INVALID_LABEL;
                        }
                    } else {
                        unsafe {
                            (*indsp).accepts_cargo[j] = cargo.0;
                            (*indsp).accepts_cargo_label[j] = INVALID_LABEL;
                        }
                    }
                }
                if !produced {
                    // Unnused, eat it up
                    buf.byte()?;
                }
            }
            // Production multipliers
            0x12 | 0x13 => unsafe {
                (*indsp).production_rate[(prop - 0x12) as usize] = buf.byte()?;
            },
            // Minimal amount of cargo distributed
            0x14 => unsafe {
                (*indsp).minimal_cargo = buf.byte()?;
            },
            // Random sound effects
            0x15 => {
                let num_sounds = buf.byte()?;
                let mut sounds = Vec::with_capacity(usize::from(num_sounds));
                for _ in 0..num_sounds {
                    sounds.push(buf.byte()?);
                }
                unsafe { (*indsp).random_sounds = NativeVec::from_vec(sounds) };
            }
            // Conflicting industry types
            0x16 => {
                for j in 0..3 {
                    unsafe { (*indsp).conflicting[j] = buf.byte()? };
                }
            }
            // Probability in random game
            0x17 => unsafe {
                (*indsp).appear_creation[usize::from(c.climate)] = buf.byte()?;
            },
            // Probability during gameplay
            0x18 => unsafe {
                (*indsp).appear_ingame[usize::from(c.climate)] = buf.byte()?;
            },
            // Map colour
            0x19 => unsafe {
                (*indsp).map_colour = buf.byte()?;
            },
            // Special industry flags to define special behavior
            0x1A => unsafe {
                (*indsp).behaviour = buf.dword()?;
            },
            // New industry text ID
            0x1B => unsafe {
                (s.map_string)(buf.word()?, &raw mut (*indsp).new_industry_text);
            },
            // Input cargo multipliers for the three input cargo types
            0x1C..=0x1E => {
                let multiples = buf.dword()?;
                let input = (prop - 0x1C) as usize;
                unsafe {
                    (*indsp).input_cargo_multiplier[input][0] = gb(multiples, 0, 16) as u16;
                    (*indsp).input_cargo_multiplier[input][1] = gb(multiples, 16, 16) as u16;
                }
            }
            // Industry name
            0x1F => unsafe {
                (s.map_string)(buf.word()?, &raw mut (*indsp).name);
            },
            // Prospecting success chance
            0x20 => unsafe {
                (*indsp).prospecting_chance = buf.dword()?;
            },
            // 0x21: callback mask; 0x22: callback additional mask
            0x21 | 0x22 => {
                let mut mask = u32::from(unsafe { (*indsp).callback_mask });
                sb(
                    &mut mask,
                    ((prop - 0x21) * 8) as u8,
                    8,
                    u32::from(buf.byte()?),
                );
                unsafe { (*indsp).callback_mask = mask as u16 };
            }
            // removal cost multiplier
            0x23 => unsafe {
                (*indsp).removal_cost_multiplier = buf.dword()?;
            },
            // name for nearby station
            0x24 => {
                let str = buf.word()?;
                if str == 0 {
                    unsafe { (*indsp).station_name = StringID::STR_NULL.0 };
                } else {
                    unsafe { (s.map_string)(str, &raw mut (*indsp).station_name) };
                }
            }
            // 0x25: variable length produced cargoes; 0x26: variable length accepted cargoes
            0x25 | 0x26 => {
                let produced = prop == 0x25;
                let num_cargoes = usize::from(buf.byte()?);
                let size = if produced {
                    INDUSTRY_NUM_OUTPUTS
                } else {
                    INDUSTRY_NUM_INPUTS
                };
                if num_cargoes > size {
                    unsafe {
                        newgrf_loading::disable_list_property(loading, prop as u32);
                    }
                    return Ok(ChangeInfoResult::CIR_DISABLED);
                }
                let labels = if produced {
                    INDUSTRY_ORIGINAL_NUM_OUTPUTS
                } else {
                    INDUSTRY_ORIGINAL_NUM_INPUTS
                };
                for i in 0..size {
                    let cargo = if i < num_cargoes {
                        get_cargo_translation(buf.byte()?, c.file, false)
                    } else {
                        CargoType::INVALID_CARGO
                    };
                    if produced {
                        unsafe { (*indsp).produced_cargo[i] = cargo.0 };
                    } else {
                        unsafe { (*indsp).accepts_cargo[i] = cargo.0 };
                    }
                    if i < labels {
                        if produced {
                            unsafe { (*indsp).produced_cargo_label[i] = INVALID_LABEL };
                        } else {
                            unsafe { (*indsp).accepts_cargo_label[i] = INVALID_LABEL };
                        }
                    }
                }
            }
            // variable length production rates
            0x27 => {
                let num_cargoes = usize::from(buf.byte()?);
                if num_cargoes > INDUSTRY_NUM_OUTPUTS {
                    unsafe {
                        newgrf_loading::disable_list_property(loading, prop as u32);
                    }
                    return Ok(ChangeInfoResult::CIR_DISABLED);
                }
                for i in 0..INDUSTRY_NUM_OUTPUTS {
                    let rate = if i < num_cargoes { buf.byte()? } else { 0 };
                    unsafe { (*indsp).production_rate[i] = rate };
                }
            }
            // variable size input/output production multiplier table
            0x28 => {
                let num_inputs = usize::from(buf.byte()?);
                let num_outputs = usize::from(buf.byte()?);
                if num_inputs > INDUSTRY_NUM_INPUTS || num_outputs > INDUSTRY_NUM_OUTPUTS {
                    unsafe {
                        newgrf_loading::disable_list_property(loading, prop as u32);
                    }
                    return Ok(ChangeInfoResult::CIR_DISABLED);
                }
                for i in 0..INDUSTRY_NUM_INPUTS {
                    for j in 0..INDUSTRY_NUM_OUTPUTS {
                        let mult = if i < num_inputs && j < num_outputs {
                            buf.word()?
                        } else {
                            0
                        };
                        unsafe { (*indsp).input_cargo_multiplier[i][j] = mult };
                    }
                }
            }
            // Badge list
            0x29 => {
                let ids = buf.badge_ids(GrfSpecFeature::GSF_INDUSTRIES.0)?;
                unsafe { (*indsp).badges = NativeVec::from_vec(ids) };
            }
            _ => ret = ChangeInfoResult::CIR_UNKNOWN,
        }
    }
    Ok(ret)
}
/// Store a completed property's `ChangeInfoResult`; a short read reports failure.
fn finish(status: &Result<ChangeInfoResult, ReadError>, result: *mut i32) -> bool {
    match status {
        Ok(value) => {
            unsafe {
                ptr::write(result, i32::from(value.0));
            }
            true
        }
        Err(ReadError) => false,
    }
}
/// `GrfChangeInfoHandler<GSF_INDUSTRIES>::Activation`.
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
        &unsafe { industries_change_info(first..last, prop, c, &mut reader, loading) },
        result,
    ))
}
/// `GrfChangeInfoHandler<GSF_INDUSTRYTILES>::Activation`.
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
        &unsafe { industrytiles_change_info(first..last, prop, c, &mut reader, loading) },
        result,
    ))
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDIndustryPropertyContext" ["industry"] => Context { file, grfid, version, climate };
}
