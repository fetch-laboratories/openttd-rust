/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Canonical `NewGRF` loading execution state. The game-state mutex serializes native aliases
//! and Rust entries. Raw field accesses end before native parser reentry; the carrier never
//! moves, and native aliases own no storage. Parsed objects remain native.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    clippy::decimal_bitwise_operands,
    clippy::cast_possible_wrap,
    clippy::if_not_else,
    clippy::manual_checked_ops,
    clippy::match_same_arms,
    clippy::comparison_chain,
    clippy::struct_field_names
)]
use crate::debug::{Bytes, debug, grf_msg};
use crate::game_state::GameState;
use crate::newgrf::GSF_END;
use crate::string_validation::make_valid;
use std::{collections::BTreeMap, ffi::c_void, ptr};

#[repr(C)]
pub struct Execution {
    stage: u8,
    pub(crate) spriteid: u32,
    file: *mut c_void,
    pub(crate) grffile: *mut c_void,
    grfconfig: *mut c_void,
    pub(crate) nfo_line: u32,
    pub(crate) skip_sprites: i32,
    pub(crate) spritegroups: [*const c_void; 256],
}
static EXECUTION: GameState<Execution> = GameState::new(Execution {
    stage: 0,
    spriteid: 0,
    file: ptr::null_mut(),
    grffile: ptr::null_mut(),
    grfconfig: ptr::null_mut(),
    nfo_line: 0,
    skip_sprites: 0,
    spritegroups: [ptr::null(); 256],
});
#[derive(Clone, Copy)]
struct SpriteSet {
    sprite: u32,
    num_sprites: u32,
}
struct State {
    spritesets: [BTreeMap<u32, SpriteSet>; GSF_END as usize],
}
static STATE: GameState<State> = GameState::new(State {
    spritesets: [const { BTreeMap::new() }; GSF_END as usize],
});
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_grf_execution() -> *mut Execution {
    EXECUTION.get()
}
unsafe fn openttd_rust_grf_clear_file() {
    unsafe {
        (*EXECUTION.get()).nfo_line = 0;
        (*EXECUTION.get()).skip_sprites = 0;
        (*EXECUTION.get()).spritegroups.fill(ptr::null());
        for sets in &mut (*STATE.get()).spritesets {
            sets.clear();
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_add_sets(
    feature: u8,
    first_sprite: u32,
    first_set: u32,
    numsets: u32,
    numents: u32,
) {
    let sets = unsafe { &mut (*STATE.get()).spritesets[usize::from(feature)] };
    for i in 0..numsets {
        sets.insert(
            first_set.wrapping_add(i),
            SpriteSet {
                sprite: first_sprite.wrapping_add(i.wrapping_mul(numents)),
                num_sprites: numents,
            },
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_has_sets_abi(feature: u8) -> u8 {
    u8::from(unsafe { !(*STATE.get()).spritesets[usize::from(feature)].is_empty() })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_has_set_abi(feature: u8, set: u32) -> u8 {
    u8::from(unsafe { (*STATE.get()).spritesets[usize::from(feature)].contains_key(&set) })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_set_sprite(feature: u8, set: u32) -> u32 {
    unsafe { (&(*STATE.get()).spritesets[usize::from(feature)])[&set].sprite }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_set_size(feature: u8, set: u32) -> u32 {
    unsafe { (&(*STATE.get()).spritesets[usize::from(feature)])[&set].num_sprites }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDGrfLoadingServices" ["grf_loading"] => Services { clocks, finance, global, cargo, engine, houses, industry, airports, types, action2, text, bindings, stations, engine_range, config_md5, config_id, config_status, config_flags, set_status, set_flag, set_config_id, set_file_info, config_version, config_filename, config_palette, param_count, set_parameter, find_config, file_for_config, error, error_name, error_property, error_after_translated_file, load_after, read_byte, read_word, read_dword, read_block, position, seek, skip, container_version, sprite_offsets, skip_sprite_data, open_cached, open_temporary, scan_name, scan_info, set_pitch, set_width, set_rail_cost, set_misc, safe_change_info, reserve_change_info, feature_change_info, skip_act1, new_sprite_set, skip_act5, graphics_new, skip_act_a, sprite_replace, grf_load_error, grf_comment, skip_act11, grf_sound, skip_act12, load_font_glyph, static_grf_info, gradual_loading, signal_side, freight_trains, wagon_speed_limits, improved_load, inflation, starting_year, plane_speed, read_bounds, unexpected_sprite, multiple_action8, grm_failed, static_desync, list_property_too_long, invalid_id, force_disabled, colourmap_sprite, slopes_sprite, shore_sprite, generic, clock, set_clock, config_at, exists, init_file, too_many, clear_errors, reset_badges, reset_bridges, reset_prices, reset_currencies, object_classes, reset_objects, station_classes, airport_classes, roadstop_classes, reset_water_features, reset_faces, reset_files, reset_loaded_features, initialize_sounds, clean_spritegroups, finalise_strings, finalise_badges, finalise_canals, finalise_objects, sort_industries, industry_legend, linkstats_legend, activate_shore, activate_tram_depot, aging_stops, finalise_prices, baseset_dir, newgrf_dir, assert_override_size, assert_file_match, null_error, undefined };
    "OpenTTDGrfEngineRange" ["grf_loading"] => EngineRange { offset, count };
    "OpenTTDGrfClock" ["grf_loading"] => Clock { ticks, display };
    "OpenTTDGrfExecution" ["grf_loading"] => Execution {
        stage, spriteid, file, grffile, grfconfig, nfo_line, skip_sprites, spritegroups
    };
}

#[repr(C)]
pub struct Services {
    clocks: &'static crate::timer_game_common::ClockServices,
    finance: &'static crate::company::FinanceServices,
    pub(crate) global: *const crate::newgrf::Services,
    cargo: *const crate::cargo_registry::Services,
    pub(crate) engine: *const crate::engine_loading::Services,
    pub(crate) houses: *const crate::house_definition::Services,
    pub(crate) industry: *const crate::industry::definitions::Services,
    pub(crate) airports: *const crate::airport_definitions::Services,
    pub(crate) types: *const crate::type_registry::Services,
    action2: *const crate::newgrf_act2::Services,
    pub(crate) text: *const crate::newgrf_text::Services,
    pub(crate) bindings: *const crate::newgrf_text::actions::BindingServices,
    pub(crate) stations: *const crate::station_definitions::Services,
    engine_range: unsafe extern "C" fn(u8) -> EngineRange,
    pub(crate) config_md5: unsafe extern "C" fn(*const c_void) -> crate::services::ByteSpan,
    pub(crate) config_id: unsafe extern "C" fn(*const c_void) -> u32,
    pub(crate) config_status: unsafe extern "C" fn(*const c_void) -> u8,
    pub(crate) config_flags: unsafe extern "C" fn(*const c_void) -> u8,
    pub(crate) set_status: unsafe extern "C" fn(*mut c_void, u8),
    set_flag: unsafe extern "C" fn(*mut c_void, u8, bool),
    set_config_id: unsafe extern "C" fn(*mut c_void, u32),
    set_file_info: unsafe extern "C" fn(*mut c_void, u32, u8),
    config_version: unsafe extern "C" fn(*const c_void) -> u32,
    /// `GRFConfig::filename`, live while the config is.
    config_filename: unsafe extern "C" fn(*const c_void) -> crate::services::ByteSpan,
    /// `GRFConfig::palette`.
    config_palette: unsafe extern "C" fn(*const c_void) -> u8,
    param_count: unsafe extern "C" fn(*const c_void) -> usize,
    set_parameter: unsafe extern "C" fn(*mut c_void, u8, u32),
    pub(crate) find_config: unsafe extern "C" fn(u32, u32) -> *mut c_void,
    file_for_config: unsafe extern "C" fn(*const c_void) -> *mut c_void,
    pub(crate) error: unsafe extern "C" fn(*mut c_void, u32, u32, bool) -> *mut c_void,
    error_name: unsafe extern "C" fn(*mut c_void, *const c_void),
    error_property: unsafe extern "C" fn(*mut c_void, u32),
    pub(crate) error_after_translated_file: unsafe extern "C" fn(*mut c_void),
    pub(crate) load_after: u32,
    read_byte: unsafe extern "C" fn(*mut c_void) -> u8,
    read_word: unsafe extern "C" fn(*mut c_void) -> u16,
    read_dword: unsafe extern "C" fn(*mut c_void) -> u32,
    read_block: unsafe extern "C" fn(*mut c_void, *mut u8, usize),
    position: unsafe extern "C" fn(*const c_void) -> usize,
    seek: unsafe extern "C" fn(*mut c_void, usize, bool),
    skip: unsafe extern "C" fn(*mut c_void, usize),
    container_version: unsafe extern "C" fn(*const c_void) -> u8,
    sprite_offsets: unsafe extern "C" fn(*mut c_void),
    skip_sprite_data: unsafe extern "C" fn(*mut c_void, u8, u16) -> bool,
    open_cached: unsafe extern "C" fn(*const c_void, u8) -> *mut c_void,
    open_temporary: unsafe extern "C" fn(*const c_void, u8, *mut c_void) -> *mut c_void,
    scan_name: unsafe extern "C" fn(*mut c_void, u32, *const u8, usize),
    scan_info: unsafe extern "C" fn(*mut c_void, u32, *const u8, usize),
    set_pitch: unsafe extern "C" fn(*mut c_void, u32),
    set_width: unsafe extern "C" fn(*mut c_void, u32),
    set_rail_cost: unsafe extern "C" fn(u8, u16),
    set_misc: unsafe extern "C" fn(u8),
    safe_change_info: unsafe extern "C" fn(*const u8, usize) -> bool,
    reserve_change_info: unsafe extern "C" fn(*const u8, usize) -> bool,
    feature_change_info: unsafe extern "C" fn(*const u8, usize) -> bool,
    skip_act1: unsafe extern "C" fn(*const u8, usize) -> bool,
    new_sprite_set: unsafe extern "C" fn(*const u8, usize) -> bool,
    skip_act5: unsafe extern "C" fn(*const u8, usize) -> bool,
    graphics_new: unsafe extern "C" fn(*const u8, usize) -> bool,
    skip_act_a: unsafe extern "C" fn(*const u8, usize) -> bool,
    sprite_replace: unsafe extern "C" fn(*const u8, usize) -> bool,
    grf_load_error: unsafe extern "C" fn(*const u8, usize) -> bool,
    grf_comment: unsafe extern "C" fn(*const u8, usize) -> bool,
    skip_act11: unsafe extern "C" fn(*const u8, usize) -> bool,
    grf_sound: unsafe extern "C" fn(*const u8, usize) -> bool,
    skip_act12: unsafe extern "C" fn(*const u8, usize) -> bool,
    load_font_glyph: unsafe extern "C" fn(*const u8, usize) -> bool,
    static_grf_info: unsafe extern "C" fn(*const u8, usize) -> bool,
    gradual_loading: unsafe extern "C" fn() -> bool,
    signal_side: unsafe extern "C" fn() -> u8,
    freight_trains: unsafe extern "C" fn() -> u8,
    wagon_speed_limits: unsafe extern "C" fn() -> bool,
    improved_load: unsafe extern "C" fn() -> bool,
    inflation: unsafe extern "C" fn() -> bool,
    starting_year: unsafe extern "C" fn() -> i32,
    plane_speed: unsafe extern "C" fn() -> u8,
    read_bounds: u32,
    unexpected_sprite: u32,
    multiple_action8: u32,
    grm_failed: u32,
    static_desync: u32,
    list_property_too_long: u32,
    pub(crate) invalid_id: u32,
    force_disabled: u32,
    colourmap_sprite: u32,
    slopes_sprite: u32,
    shore_sprite: u32,
    generic: *const crate::newgrf_generic::Services,
    clock: unsafe extern "C" fn() -> Clock,
    set_clock: unsafe extern "C" fn(*const Clock),
    pub(crate) config_at: unsafe extern "C" fn(usize) -> *mut c_void,
    exists: unsafe extern "C" fn(*const c_void, u8) -> bool,
    init_file: unsafe extern "C" fn(*const c_void),
    too_many: unsafe extern "C" fn(*mut c_void),
    clear_errors: unsafe extern "C" fn(*mut c_void),
    reset_badges: unsafe extern "C" fn(),
    reset_bridges: unsafe extern "C" fn(),
    reset_prices: unsafe extern "C" fn(),
    reset_currencies: unsafe extern "C" fn(),
    object_classes: *const crate::newgrf_class::Services<crate::objects::Spec>,
    reset_objects: unsafe extern "C" fn(),
    station_classes: *const crate::newgrf_class::Services<crate::station_cmd::RailSpec>,
    airport_classes: *const crate::newgrf_class::Services<crate::station_cmd::AirportSpec>,
    roadstop_classes: *const crate::newgrf_class::Services<crate::station_cmd::RoadSpec>,
    reset_water_features: unsafe extern "C" fn(),
    reset_faces: unsafe extern "C" fn(),
    reset_files: unsafe extern "C" fn(),
    reset_loaded_features: unsafe extern "C" fn(),
    initialize_sounds: unsafe extern "C" fn(),
    clean_spritegroups: unsafe extern "C" fn(),
    finalise_strings: unsafe extern "C" fn(),
    finalise_badges: unsafe extern "C" fn(),
    finalise_canals: unsafe extern "C" fn(),
    finalise_objects: unsafe extern "C" fn(),
    sort_industries: unsafe extern "C" fn(),
    industry_legend: unsafe extern "C" fn(),
    linkstats_legend: unsafe extern "C" fn(),
    activate_shore: unsafe extern "C" fn(),
    activate_tram_depot: unsafe extern "C" fn(),
    aging_stops: unsafe extern "C" fn(),
    finalise_prices: unsafe extern "C" fn(),
    baseset_dir: u8,
    newgrf_dir: u8,
    assert_override_size: unsafe extern "C" fn(usize, u32),
    assert_file_match: unsafe extern "C" fn(*const c_void, *const c_void),
    null_error: u32,
    pub(crate) undefined: u32,
}
impl Services {
    pub(crate) fn shared(&self) -> &crate::services::Services {
        // SAFETY: the engine-loading table and its shared table are process-lifetime.
        unsafe { &*(*self.engine).shared }
    }
}

use crate::newgrf::{
    GLS_ACTIVATION, GLS_FILESCAN, GLS_INIT, GLS_LABELSCAN, GLS_RESERVE, GLS_SAFETYSCAN,
};
use crate::newgrf_bytereader::{Cursor as ByteReader, ReadBounds};
use crate::newgrf_config::{
    GCS_ACTIVATED, GCS_DISABLED, GCS_INITIALISED, GCS_NOT_FOUND, GCS_UNKNOWN, GRFP_USE_MASK,
    INIT_ONLY, INVALID, RESERVED, STATIC, SYSTEM, UNSAFE,
};
#[derive(Clone, Copy)]
struct Label {
    label: u8,
    nfo_line: u32,
    pos: usize,
}
static PATCH_FLAGS: GameState<[u32; 8]> = GameState::new([0; 8]);
static GRM_ENGINES: GameState<[u32; 256]> = GameState::new([0; 256]);
static GRM_CARGOES: GameState<[u32; crate::cargo_type::NUM_CARGO as usize * 2]> =
    GameState::new([0; crate::cargo_type::NUM_CARGO as usize * 2]);
type Location = (u32, u32);
static GRM_SPRITES: GameState<BTreeMap<Location, (u32, u16)>> = GameState::new(BTreeMap::new());
static OVERRIDES: GameState<BTreeMap<Location, Vec<u8>>> = GameState::new(BTreeMap::new());
static LABELS: GameState<BTreeMap<usize, Vec<Label>>> = GameState::new(BTreeMap::new());
fn execution() -> *mut Execution {
    EXECUTION.get()
}
pub(crate) unsafe fn file() -> *mut c_void {
    unsafe { (*execution()).grffile }
}
/// `GRFFile::grf_version` through the engine loader's accessor.
pub(crate) unsafe fn file_version(s: &Services, file: *const c_void) -> u8 {
    unsafe { ((*(*s.engine).shared).grf_file_version)(file) }
}
pub(crate) unsafe fn config() -> *mut c_void {
    unsafe { (*execution()).grfconfig }
}
unsafe fn stage() -> u8 {
    unsafe { (*execution()).stage }
}
unsafe fn line() -> u32 {
    unsafe { (*execution()).nfo_line }
}
pub(crate) unsafe fn is_static(s: &Services, config: *const c_void) -> bool {
    unsafe { (s.config_flags)(config) & (1 << STATIC) != 0 }
}
unsafe fn clear_labels(file: *const c_void) {
    unsafe {
        (*LABELS.get()).remove(&(file as usize));
    }
}

unsafe fn openttd_rust_grf_reset_grm() {
    unsafe {
        (*GRM_ENGINES.get()).fill(0);
        (*GRM_CARGOES.get()).fill(0);
    }
}
unsafe fn openttd_rust_grf_clear_overrides() {
    unsafe {
        (*OVERRIDES.get()).clear();
    }
}
unsafe fn openttd_rust_grf_clear_grm_sprites() {
    unsafe {
        (*GRM_SPRITES.get()).clear();
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_reserved_sprite_abi(
    grfid: u32,
    first: u32,
    count: u16,
) -> u8 {
    u8::from(unsafe {
        (*GRM_SPRITES.get())
            .iter()
            .any(|(&(id, _), &(sprite, num))| {
                id == grfid
                    && sprite <= first
                    && sprite.wrapping_add(u32::from(num)) >= first.wrapping_add(u32::from(count))
            })
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_disable(
    s: &Services,
    message: u32,
    mut c: *mut c_void,
) -> *mut c_void {
    unsafe {
        let f = if c.is_null() {
            c = config();
            file()
        } else {
            ((*s.engine).find_file)((s.config_id)(c))
        };
        (s.set_status)(c, GCS_DISABLED);
        if !f.is_null() {
            clear_labels(f);
        }
        let current = c == config();
        if current {
            (*execution()).skip_sprites = -1;
        }
        if message == s.null_error {
            return ptr::null_mut();
        }
        (s.error)(c, line(), message, current)
    }
}
/// Disable the current file and retain the offending Action0 property.
pub unsafe fn disable_list_property(s: &Services, property: u32) {
    unsafe {
        let error = openttd_rust_grf_disable(s, s.list_property_too_long, ptr::null_mut());
        (s.error_property)(error, property);
    }
}
/// Disable the current file after an invalid feature ID.
pub unsafe fn disable_invalid_id(s: &Services) {
    unsafe { disable(s, s.invalid_id) }
}
unsafe fn disable(s: &Services, message: u32) {
    unsafe {
        openttd_rust_grf_disable(s, message, ptr::null_mut());
    }
}
unsafe fn static_desync(s: &Services, c: *mut c_void) {
    unsafe {
        let e = openttd_rust_grf_disable(s, s.static_desync, c);
        (s.error_name)(e, config());
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_unsafe(s: &Services) {
    unsafe {
        (s.set_flag)(config(), UNSAFE, true);
        (*execution()).skip_sprites = -1;
    }
}
unsafe fn openttd_rust_grf_param(s: &Services, param: u8, cond: *mut u32) -> u32 {
    unsafe {
        let mut value = 0;
        if crate::newgrf::openttd_rust_global_variable_abi(
            &*s.global,
            param.wrapping_sub(0x80),
            &raw mut value,
            file(),
        ) != 0
        {
            return value;
        }
        match param {
            0x84 => {
                u32::from(stage() > GLS_INIT)
                    | u32::from(stage() == GLS_RESERVE) << 8
                    | u32::from(stage() == GLS_ACTIVATION) << 9
            }
            0x85 => {
                if cond.is_null() {
                    return 0;
                }
                let index = cond.read() / 32;
                let value = (*PATCH_FLAGS.get())
                    .get(index as usize)
                    .copied()
                    .unwrap_or(0);
                cond.write(cond.read() % 32);
                value
            }
            0x88 => 0,
            p if p < 0x80 => (s.shared().grf_parameter)(file(), u32::from(p)),
            _ => {
                grf_msg!(1, "Unsupported in-game variable 0x{:02X}", param);
                u32::MAX
            }
        }
    }
}
unsafe fn param(s: &Services, number: u8) -> u32 {
    unsafe { openttd_rust_grf_param(s, number, ptr::null_mut()) }
}
unsafe fn cfg_apply(s: &Services, b: &mut ByteReader) -> Result<(), ReadBounds> {
    unsafe {
        let f = (*execution()).file;
        let pos = (s.position)(f);
        let num = if (s.container_version)(f) >= 2 {
            (s.read_dword)(f)
        } else {
            u32::from((s.read_word)(f))
        };
        let typ = (s.read_byte)(f);
        if typ != 0xFF {
            grf_msg!(2, "CfgApply: Ignoring (next sprite is real, unsupported)");
            (s.seek)(f, pos, false);
            return Ok(());
        }
        let location = ((s.config_id)(config()), line().wrapping_add(1));
        let data = {
            let buf = (*OVERRIDES.get()).entry(location).or_default();
            if buf.is_empty() {
                buf.resize(num as usize, 0);
                buf.as_mut_ptr()
            } else {
                ptr::null_mut()
            }
        };
        if !data.is_null() {
            (s.read_block)(f, data, num as usize);
        }
        (s.seek)(f, pos, false);
        loop {
            let pn = u32::from(b.byte()?);
            if pn == 255 {
                break;
            }
            let size = b.byte()?;
            let add = size & 128 != 0;
            let size = u32::from(size & 127);
            let offset = u32::from(b.extended()?);
            if pn < 128
                && pn.wrapping_add(size.wrapping_sub(1) / 4) as usize >= (s.param_count)(file())
            {
                grf_msg!(
                    2,
                    "CfgApply: Ignoring (param {} not set)",
                    pn.wrapping_add(size.wrapping_sub(1) / 4)
                );
                break;
            }
            grf_msg!(
                8,
                "CfgApply: Applying {} bytes from parameter 0x{:02X} at offset 0x{:04X}",
                size,
                pn,
                offset
            );
            let mut carry = false;
            for i in 0..size {
                if offset.wrapping_add(i) >= num {
                    break;
                }
                let value = param(s, pn.wrapping_add(i / 4) as u8);
                if i % 4 == 0 {
                    carry = false;
                }
                let byte = (value >> ((i % 4) * 8)) as u8;
                let target = &mut (*OVERRIDES.get()).get_mut(&location).unwrap()
                    [offset.wrapping_add(i) as usize];
                if add {
                    let value = u32::from(*target) + u32::from(byte) + u32::from(carry);
                    *target = value as u8;
                    carry = value >= 256;
                } else {
                    *target = byte;
                }
            }
        }
        Ok(())
    }
}
unsafe fn skip_if(s: &Services, b: &mut ByteReader) -> Result<(), ReadBounds> {
    unsafe {
        let pn = b.byte()?;
        let mut size = b.byte()?;
        let cond = b.byte()?;
        if cond < 2 {
            size = 1;
        }
        let (mut value, mask) = match size {
            8 => (b.dword()?, b.dword()?),
            4 => (b.dword()?, u32::MAX),
            2 => (u32::from(b.word()?), 65535),
            1 => (u32::from(b.byte()?), 255),
            _ => (0, 0),
        };
        if pn < 128 && (s.param_count)(file()) <= usize::from(pn) {
            grf_msg!(7, "SkipIf: Param {} undefined, skipping test", pn);
            return Ok(());
        }
        grf_msg!(
            7,
            "SkipIf: Test condtype {}, param 0x{:02X}, condval 0x{:08X}",
            cond,
            pn,
            value
        );
        let result = if cond >= 11 {
            match cond {
                11 | 12 => {
                    let valid = crate::cargo_type::is_valid_cargo_type(
                        crate::cargo_registry::openttd_rust_cargo_by_label(value.swap_bytes()),
                    );
                    if cond == 11 { !valid } else { valid }
                }
                13 | 14 => {
                    let valid =
                        crate::type_registry::openttd_rust_rail_by_label(value.swap_bytes(), true)
                            != crate::rail_type::INVALID_RAILTYPE;
                    if cond == 13 { !valid } else { valid }
                }
                15..=18 => {
                    let rt =
                        crate::type_registry::openttd_rust_road_by_label(value.swap_bytes(), true);
                    let valid = rt != crate::road_type::INVALID_ROADTYPE
                        && if cond < 17 {
                            crate::road::road_type_is_road(rt)
                        } else {
                            crate::road::road_type_is_tram(rt)
                        };
                    if cond % 2 == 1 { !valid } else { valid }
                }
                _ => {
                    grf_msg!(
                        1,
                        "SkipIf: Unsupported condition type {:02X}. Ignoring",
                        cond
                    );
                    return Ok(());
                }
            }
        } else if pn == 0x88 {
            let mut c = (s.find_config)(value, mask);
            if !c.is_null()
                && is_static(s, c)
                && !is_static(s, config())
                && (s.finance.networking)()
            {
                static_desync(s, c);
                c = ptr::null_mut();
            }
            if cond != 10 && c.is_null() {
                grf_msg!(
                    7,
                    "SkipIf: GRFID 0x{:08X} unknown, skipping test",
                    value.swap_bytes()
                );
                return Ok(());
            }
            match cond {
                6 => (s.config_status)(c) == GCS_ACTIVATED,
                7 => (s.config_status)(c) != GCS_ACTIVATED,
                8 => (s.config_status)(c) == GCS_INITIALISED,
                9 => matches!((s.config_status)(c), GCS_ACTIVATED | GCS_INITIALISED),
                10 => c.is_null() || matches!((s.config_status)(c), GCS_DISABLED | GCS_NOT_FOUND),
                _ => {
                    grf_msg!(
                        1,
                        "SkipIf: Unsupported GRF condition type {:02X}. Ignoring",
                        cond
                    );
                    return Ok(());
                }
            }
        } else {
            let p = openttd_rust_grf_param(s, pn, &raw mut value);
            match cond {
                0 => p & 1u32.wrapping_shl(value) != 0,
                1 => p & 1u32.wrapping_shl(value) == 0,
                2 => p & mask == value,
                3 => p & mask != value,
                4 => (p & mask) < value,
                5 => (p & mask) > value,
                _ => {
                    grf_msg!(
                        1,
                        "SkipIf: Unsupported condition type {:02X}. Ignoring",
                        cond
                    );
                    return Ok(());
                }
            }
        };
        if !result {
            grf_msg!(2, "SkipIf: Not skipping sprites, test was false");
            return Ok(());
        }
        let count = b.byte()?;
        let mut choice = None;
        if let Some(labels) = (*LABELS.get()).get(&(file() as usize)) {
            for label in labels {
                if label.label != count {
                    continue;
                }
                if choice.is_none() {
                    choice = Some(*label);
                }
                if label.nfo_line > line() {
                    choice = Some(*label);
                    break;
                }
            }
        }
        if let Some(label) = choice {
            grf_msg!(
                2,
                "SkipIf: Jumping to label 0x{:X} at line {}, test was true",
                label.label,
                label.nfo_line
            );
            (s.seek)((*execution()).file, label.pos, false);
            (*execution()).nfo_line = label.nfo_line;
            return Ok(());
        }
        grf_msg!(2, "SkipIf: Skipping {} sprites, test was true", count);
        (*execution()).skip_sprites = i32::from(count);
        if count == 0 {
            (*execution()).skip_sprites = -1;
            if (s.config_status)(config())
                != if stage() < GLS_RESERVE {
                    GCS_INITIALISED
                } else {
                    GCS_ACTIVATED
                }
            {
                disable(s, s.null_error);
            }
        }
        Ok(())
    }
}
/// `GRFConfig::filename` of a live config.
unsafe fn config_filename(s: &Services, c: *const c_void) -> &[u8] {
    unsafe { (s.config_filename)(c).bytes() }
}
unsafe fn grf_info(s: &Services, b: &mut ByteReader, scan: bool) -> Result<(), ReadBounds> {
    unsafe {
        let version = b.byte()?;
        let id = b.dword()?;
        let (name, size) = b.string();
        if scan {
            (s.set_config_id)(config(), id);
            if !(2..=8).contains(&version) {
                (s.set_flag)(config(), INVALID, true);
                debug!(
                    grf,
                    0,
                    "{}: NewGRF \"{}\" (GRFID {:08X}) uses GRF version {}, which is incompatible with this version of OpenTTD.",
                    Bytes(config_filename(s, config())),
                    Bytes(&make_valid(
                        std::slice::from_raw_parts(name, size),
                        crate::string_type::StringValidationSettings::from_element(
                            crate::string_type::StringValidationSetting::ReplaceWithQuestionMark
                        )
                    )),
                    id.swap_bytes(),
                    version
                );
            }
            if id & 255 == 255 {
                (s.set_flag)(config(), SYSTEM, true);
            }
            (s.scan_name)(config(), id, name, size);
            if b.remaining() > 0 {
                let (info, size) = b.string();
                (s.scan_info)(config(), id, info, size);
            }
            (*execution()).skip_sprites = -1;
        } else {
            if stage() < GLS_RESERVE && (s.config_status)(config()) != GCS_UNKNOWN {
                disable(s, s.multiple_action8);
                return Ok(());
            }
            let old_id = (s.shared().grf_file_id)(file());
            if old_id != id {
                debug!(
                    grf,
                    0,
                    "GRFInfo: GRFID {:08X} in FILESCAN stage does not match GRFID {:08X} in INIT/RESERVE/ACTIVATION stage",
                    old_id.swap_bytes(),
                    id.swap_bytes()
                );
            }
            (s.set_file_info)(file(), id, version);
            (s.set_status)(
                config(),
                if stage() < GLS_RESERVE {
                    GCS_INITIALISED
                } else {
                    GCS_ACTIVATED
                },
            );
            debug!(
                grf,
                1,
                "GRFInfo: Loaded GRFv{} set {:08X} - {} (palette: {}, version: {})",
                version,
                id.swap_bytes(),
                Bytes(&make_valid(
                    std::slice::from_raw_parts(name, size),
                    crate::string_type::StringValidationSettings::from_element(
                        crate::string_type::StringValidationSetting::ReplaceWithQuestionMark
                    )
                )),
                if (s.config_palette)(config()) & GRFP_USE_MASK != 0 {
                    "Windows"
                } else {
                    "DOS"
                },
                (s.config_version)(config())
            );
        }
        Ok(())
    }
}
unsafe fn inhibit(s: &Services, b: &mut ByteReader, safe: bool) -> Result<(), ReadBounds> {
    unsafe {
        let num = b.byte()?;
        for _ in 0..num {
            let id = b.dword()?;
            if safe {
                if id != (s.config_id)(config()) {
                    openttd_rust_grf_unsafe(s);
                    return Ok(());
                }
            } else {
                let c = (s.find_config)(id, u32::MAX);
                if !c.is_null() && c != config() {
                    grf_msg!(
                        2,
                        "GRFInhibit: Deactivating file '{}'",
                        Bytes(config_filename(s, c))
                    );
                    let e = openttd_rust_grf_disable(s, s.force_disabled, c);
                    (s.error_name)(e, config());
                }
            }
        }
        Ok(())
    }
}
unsafe fn define_label(s: &Services, b: &mut ByteReader) -> Result<(), ReadBounds> {
    unsafe {
        let label = b.byte()?;
        let pos = (s.position)((*execution()).file);
        (*LABELS.get())
            .entry(file() as usize)
            .or_default()
            .push(Label {
                label,
                nfo_line: line(),
                pos,
            });
        grf_msg!(2, "DefineGotoLabel: GOTO target with label 0x{:02X}", label);
        Ok(())
    }
}
unsafe fn patch_variable(s: &Services, pn: u8) -> u32 {
    unsafe {
        match pn {
            0x0B => (s.starting_year)()
                .max(crate::timer_game_common::ORIGINAL_BASE_YEAR)
                .wrapping_sub(crate::timer_game_common::ORIGINAL_BASE_YEAR)
                as u32,
            0x0E => u32::from((s.freight_trains)()),
            0x0F => 0,
            0x10 => match (s.plane_speed)() {
                1 => 4,
                2 | 3 => 2,
                _ => 1,
            },
            0x11 => s.colourmap_sprite,
            0x13 => {
                let x = s.shared().map().log_x.wrapping_sub(6) as u8;
                let y = s.shared().map_log_y().wrapping_sub(6) as u8;
                let bits = if x == y {
                    1
                } else if y > x {
                    2
                } else {
                    0
                };
                (bits << 24)
                    | (u32::from(x.min(y)) << 20)
                    | (u32::from(x.max(y)) << 16)
                    | (u32::from(x) << 12)
                    | (u32::from(y) << 8)
                    | (u32::from(x) + u32::from(y))
            }
            0x14 => u32::from(s.shared().settings().height_limit.read()),
            0x15 => s.slopes_sprite,
            0x16 => s.shore_sprite,
            0x17 => s.shared().settings().generation_seed.read(),
            _ => {
                grf_msg!(2, "ParamSet: Unknown Patch variable 0x{:02X}.", pn);
                0
            }
        }
    }
}
unsafe fn perform_grm(
    s: &Services,
    grm: *mut u32,
    len: u32,
    count: u16,
    op: u8,
    target: u8,
) -> u32 {
    unsafe {
        if op == 6 {
            return grm
                .add((s.shared().grf_parameter)(file(), u32::from(target)) as usize)
                .read();
        }
        let mut start = if op == 2 || op == 3 {
            (s.shared().grf_parameter)(file(), u32::from(target))
        } else {
            0
        };
        let mut size = 0;
        let first = start;
        for i in first..len {
            if grm.add(i as usize).read() == 0 {
                size += 1;
            } else {
                if op == 2 || op == 3 {
                    break;
                }
                start = i.wrapping_add(1);
                size = 0;
            }
            if size == u32::from(count) {
                break;
            }
        }
        if size == u32::from(count) {
            if op == 0 || op == 3 {
                grf_msg!(
                    2,
                    "ParamSet: GRM: Reserving {} {} at {}",
                    count,
                    if grm != (*GRM_CARGOES.get()).as_mut_ptr() {
                        "vehicles"
                    } else {
                        "cargoes"
                    },
                    start
                );
                let id = (s.shared().grf_file_id)(file());
                for i in 0..u32::from(count) {
                    grm.add(start.wrapping_add(i) as usize).write(id);
                }
            }
            return start;
        }
        let kind = if grm != (*GRM_CARGOES.get()).as_mut_ptr() {
            "vehicles"
        } else {
            "cargoes"
        };
        if op != 4 && op != 5 {
            grf_msg!(
                0,
                "ParamSet: GRM: Unable to allocate {} {}, deactivating",
                count,
                kind
            );
        } else {
            grf_msg!(1, "ParamSet: GRM: Unable to allocate {} {}", count, kind);
        }
        if op != 4 && op != 5 {
            disable(s, s.grm_failed);
        }
        u32::MAX
    }
}
unsafe fn param_set(s: &Services, b: &mut ByteReader) -> Result<(), ReadBounds> {
    unsafe {
        let target = b.byte()?;
        let mut op = b.byte()?;
        let mut src1 = u32::from(b.byte()?);
        let mut src2 = u32::from(b.byte()?);
        let data = if b.remaining() >= 4 { b.dword()? } else { 0 };
        if op & 128 != 0 {
            if target < 128 && usize::from(target) < (s.param_count)(file()) {
                grf_msg!(7, "ParamSet: Param {} already defined, skipping", target);
                return Ok(());
            }
            op &= 127;
        }
        if src2 == 0xFE {
            if data & 255 == 255 {
                if data == 65535 {
                    src1 = patch_variable(s, src1 as u8);
                } else {
                    let operation = src1 as u8;
                    let feature = (data >> 8) as u8;
                    let count = (data >> 16) as u16;
                    if stage() == GLS_RESERVE {
                        if feature == crate::newgrf::GSF_GLOBALVAR && operation == 0 {
                            let sprite = (*execution()).spriteid;
                            if sprite.wrapping_add(u32::from(count)) >= 16384 {
                                grf_msg!(
                                    0,
                                    "ParamSet: GRM: Unable to allocate {} sprites; try changing NewGRF order",
                                    count
                                );
                                disable(s, s.grm_failed);
                                return Ok(());
                            }
                            grf_msg!(
                                4,
                                "ParamSet: GRM: Allocated {} sprites at {}",
                                count,
                                sprite
                            );
                            (*GRM_SPRITES.get()).insert(
                                ((s.shared().grf_file_id)(file()), line()),
                                (sprite, count),
                            );
                            (*execution()).spriteid = sprite.wrapping_add(u32::from(count));
                        }
                        src1 = 0;
                    } else if stage() == GLS_ACTIVATION {
                        match feature {
                            0..=3 => {
                                if !((*s.engine).dynamic)() {
                                    let range = (s.engine_range)(feature);
                                    let start = range.offset;
                                    src1 = perform_grm(
                                        s,
                                        (*GRM_ENGINES.get()).as_mut_ptr().add(usize::from(start)),
                                        u32::from(range.count),
                                        count,
                                        operation,
                                        target,
                                    );
                                    if (*execution()).skip_sprites == -1 {
                                        return Ok(());
                                    }
                                } else {
                                    src1 = if operation == 2 || operation == 3 {
                                        (s.shared().grf_parameter)(file(), u32::from(target))
                                    } else {
                                        0
                                    };
                                }
                            }
                            crate::newgrf::GSF_GLOBALVAR => {
                                src1 = match operation {
                                    0 => {
                                        let sprite = (*GRM_SPRITES.get())
                                            .entry(((s.shared().grf_file_id)(file()), line()))
                                            .or_default()
                                            .0;
                                        grf_msg!(
                                            4,
                                            "ParamSet: GRM: Using pre-allocated sprites at {}",
                                            sprite
                                        );
                                        sprite
                                    }
                                    1 => (*execution()).spriteid,
                                    _ => {
                                        grf_msg!(
                                            1,
                                            "ParamSet: GRM: Unsupported operation {} for general sprites",
                                            operation
                                        );
                                        return Ok(());
                                    }
                                };
                            }
                            crate::newgrf::GSF_CARGOES => {
                                src1 = perform_grm(
                                    s,
                                    (*GRM_CARGOES.get()).as_mut_ptr(),
                                    u32::from(crate::cargo_type::NUM_CARGO) * 2,
                                    count,
                                    operation,
                                    target,
                                );
                                if (*execution()).skip_sprites == -1 {
                                    return Ok(());
                                }
                            }
                            _ => {
                                grf_msg!(1, "ParamSet: GRM: Unsupported feature 0x{:X}", feature);
                                return Ok(());
                            }
                        }
                    } else {
                        src1 = 0;
                    }
                }
            } else {
                let f = ((*s.engine).find_file)(data);
                let c = (s.find_config)(data, u32::MAX);
                if !c.is_null()
                    && is_static(s, c)
                    && !is_static(s, config())
                    && (s.finance.networking)()
                {
                    static_desync(s, c);
                    src1 = 0;
                } else if f.is_null() || c.is_null() || (s.config_status)(c) == GCS_DISABLED {
                    src1 = 0;
                } else if src1 == 0xFE {
                    src1 = (s.config_version)(c);
                } else {
                    src1 = (s.shared().grf_parameter)(f, src1);
                }
            }
        } else {
            src1 = if src1 == 255 {
                data
            } else {
                param(s, src1 as u8)
            };
            src2 = if src2 == 255 {
                data
            } else {
                param(s, src2 as u8)
            };
        }
        let result = match op {
            0 => src1,
            1 => src1.wrapping_add(src2),
            2 => src1.wrapping_sub(src2),
            3 | 4 => src1.wrapping_mul(src2),
            5 => {
                if (src2 as i32) < 0 {
                    src1.wrapping_shr(src2.wrapping_neg())
                } else {
                    src1.wrapping_shl(src2 & 31)
                }
            }
            6 => {
                if (src2 as i32) < 0 {
                    (src1 as i32).wrapping_shr(src2.wrapping_neg()) as u32
                } else {
                    (src1 as i32).wrapping_shl(src2 & 31) as u32
                }
            }
            7 => src1 & src2,
            8 => src1 | src2,
            9 => {
                if src2 == 0 {
                    src1
                } else {
                    src1 / src2
                }
            }
            10 => {
                if src2 == 0 {
                    src1
                } else {
                    (src1 as i32).wrapping_div(src2 as i32) as u32
                }
            }
            11 => {
                if src2 == 0 {
                    src1
                } else {
                    src1 % src2
                }
            }
            12 => {
                if src2 == 0 {
                    src1
                } else {
                    (src1 as i32).wrapping_rem(src2 as i32) as u32
                }
            }
            _ => {
                grf_msg!(0, "ParamSet: Unknown operation {}, skipping", op);
                return Ok(());
            }
        };
        match target {
            0x8E => (s.set_pitch)(file(), result),
            0x8F => {
                use crate::rail_type::{
                    RAILTYPE_ELECTRIC, RAILTYPE_MAGLEV, RAILTYPE_MONO, RAILTYPE_RAIL,
                };
                (s.set_rail_cost)(RAILTYPE_RAIL, u16::from(result as u8));
                if s.shared().settings().disable_elrails.read() {
                    (s.set_rail_cost)(RAILTYPE_ELECTRIC, u16::from(result as u8));
                    (s.set_rail_cost)(RAILTYPE_MONO, u16::from((result >> 8) as u8));
                } else {
                    (s.set_rail_cost)(RAILTYPE_ELECTRIC, u16::from((result >> 8) as u8));
                    (s.set_rail_cost)(RAILTYPE_MONO, u16::from((result >> 16) as u8));
                }
                (s.set_rail_cost)(RAILTYPE_MAGLEV, u16::from((result >> 16) as u8));
            }
            0x9E => {
                let local = 1 << crate::newgrf::TRAIN_WIDTH_32_PIXELS;
                (s.set_width)(
                    file(),
                    if result & local != 0 {
                        crate::newgrf_engine::VEHICLEINFO_FULL_VEHICLE_WIDTH
                    } else {
                        crate::newgrf_engine::TRAININFO_DEFAULT_VEHICLE_WIDTH
                    },
                );
                let bits = result & !local;
                let bits = if is_static(s, config()) {
                    let safe = 1 << crate::newgrf::SECOND_ROCKY_TILE_SET;
                    (u32::from(((*s.global).misc_bits)()) & !safe) | (bits & safe)
                } else {
                    bits
                };
                (s.set_misc)(bits as u8);
            }
            0x93..=0x97 | 0x99 | 0x9F => grf_msg!(
                7,
                "ParamSet: Skipping unimplemented target 0x{:02X}",
                target
            ),
            t if t < 128 => (s.set_parameter)(file(), t, result),
            _ => grf_msg!(7, "ParamSet: Skipping unknown target 0x{:02X}", target),
        }
        Ok(())
    }
}

#[repr(C)]
pub struct EngineRange {
    offset: u8,
    count: u8,
}

unsafe fn openttd_rust_grf_patch_flags(s: &Services) {
    unsafe {
        (*PATCH_FLAGS.get())[0] = 0x680C_6000
            | (u32::from(s.shared().settings().never_expire_airports.read()) << 0x0C)
            | (u32::from(s.shared().settings().max_bridge_length.read() > 16) << 0x0F)
            | (u32::from(s.shared().settings().never_expire_vehicles.read()) << 0x16);
        (*PATCH_FLAGS.get())[1] = 0x07DC_0300
            | (u32::from(s.shared().settings().station_noise.read()) << 0x07)
            | (u32::from((s.gradual_loading)()) << 0x0C)
            | (u32::from((s.signal_side)() == 1) << 0x1B)
            | (u32::from(!(s.shared().settings().disable_elrails.read())) << 0x1C);
        (*PATCH_FLAGS.get())[2] = 0x5EDC_C40A
            | (u32::from((s.shared().build_on_slopes)()) << 0x0D)
            | (u32::from((s.shared().build_on_slopes)()) << 0x15)
            | (u32::from((s.freight_trains)() > 1) << 0x18)
            | (u32::from((s.wagon_speed_limits)()) << 0x1D);
        (*PATCH_FLAGS.get())[3] = 0xC0D7_F8EA
            | (u32::from(
                !(s.shared().settings().allow_town_roads.read() || crate::genworld::generating()),
            ) << 0x02)
            | (u32::from((s.improved_load)()) << 0x08)
            | (u32::from(((*s.engine).dynamic)()) << 0x18);
        (*PATCH_FLAGS.get())[4] = 0x0000_0005 | (u32::from((s.inflation)()) << 0x01);
    }
}
unsafe fn dispatch(
    s: &Services,
    action: u8,
    stage: u8,
    b: &mut ByteReader,
) -> Result<(), ReadBounds> {
    unsafe {
        match (action, stage) {
            (6, GLS_INIT | GLS_RESERVE | GLS_ACTIVATION) => return cfg_apply(s, b),
            (7, GLS_RESERVE | GLS_ACTIVATION) | (9, GLS_INIT | GLS_RESERVE | GLS_ACTIVATION) => {
                return skip_if(s, b);
            }
            (8, GLS_FILESCAN) => return grf_info(s, b, true),
            (8, GLS_INIT | GLS_RESERVE | GLS_ACTIVATION) => return grf_info(s, b, false),
            (13, GLS_SAFETYSCAN) => {
                let target = b.byte()?;
                if target >= 128 && target != 0x9E {
                    openttd_rust_grf_unsafe(s);
                }
                return Ok(());
            }
            (13, GLS_INIT | GLS_RESERVE | GLS_ACTIVATION) => return param_set(s, b),
            (2, GLS_ACTIVATION) => {
                return if crate::newgrf_act2::parse_action(&*s.action2, s, b) {
                    Ok(())
                } else {
                    Err(ReadBounds)
                };
            }
            (14, GLS_SAFETYSCAN) => return inhibit(s, b, true),
            (14, GLS_INIT | GLS_RESERVE | GLS_ACTIVATION) => return inhibit(s, b, false),
            (15, GLS_INIT) => return crate::newgrf_townname::feature_town_name(s, b),
            (16, GLS_LABELSCAN) => return define_label(s, b),
            (3, GLS_ACTIVATION) => {
                return crate::newgrf_act3::loading::feature_map_sprite_group(s, b);
            }
            (4, GLS_ACTIVATION) => return crate::newgrf_text::actions::feature_new_name(s, b),
            (19, GLS_ACTIVATION) => {
                return crate::newgrf_text::actions::translate_grf_strings(s, b);
            }
            _ => (),
        }
        let (data, len) = b.tail();
        let success = match (action, stage) {
            (0, GLS_SAFETYSCAN) => (s.safe_change_info)(data, len),
            (0, GLS_RESERVE) => (s.reserve_change_info)(data, len),
            (0, GLS_ACTIVATION) => (s.feature_change_info)(data, len),
            (1, GLS_FILESCAN | GLS_SAFETYSCAN | GLS_LABELSCAN | GLS_INIT | GLS_RESERVE) => {
                (s.skip_act1)(data, len)
            }
            (1, GLS_ACTIVATION) => (s.new_sprite_set)(data, len),
            (3 | 15 | 17, GLS_SAFETYSCAN) => {
                openttd_rust_grf_unsafe(s);
                true
            }
            (5, GLS_FILESCAN | GLS_SAFETYSCAN | GLS_LABELSCAN | GLS_INIT | GLS_RESERVE) => {
                (s.skip_act5)(data, len)
            }
            (5, GLS_ACTIVATION) => (s.graphics_new)(data, len),
            (10, GLS_FILESCAN | GLS_SAFETYSCAN | GLS_LABELSCAN | GLS_INIT | GLS_RESERVE) => {
                (s.skip_act_a)(data, len)
            }
            (10, GLS_ACTIVATION) => (s.sprite_replace)(data, len),
            (11, GLS_INIT | GLS_RESERVE | GLS_ACTIVATION) => (s.grf_load_error)(data, len),
            (12, GLS_INIT | GLS_ACTIVATION) => (s.grf_comment)(data, len),
            (17, GLS_FILESCAN | GLS_LABELSCAN | GLS_RESERVE) => (s.skip_act11)(data, len),
            (17, GLS_INIT | GLS_ACTIVATION) => (s.grf_sound)(data, len),
            (18, GLS_FILESCAN | GLS_SAFETYSCAN | GLS_LABELSCAN | GLS_INIT | GLS_RESERVE) => {
                (s.skip_act12)(data, len)
            }
            (18, GLS_ACTIVATION) => (s.load_font_glyph)(data, len),
            (20, GLS_FILESCAN) => (s.static_grf_info)(data, len),
            _ => true,
        };
        if success { Ok(()) } else { Err(ReadBounds) }
    }
}
unsafe fn decode_special_sprite(s: &Services, allocator: &mut Vec<u8>, num: u32, stage: u8) {
    unsafe {
        let location = ((s.config_id)(config()), line());
        let preload = (*OVERRIDES.get())
            .get(&location)
            .map(|v| (v.as_ptr(), v.len()));
        let data = if let Some((data, len)) = preload {
            (s.assert_override_size)(len, num);
            grf_msg!(7, "DecodeSpecialSprite: Using preloaded pseudo sprite data");
            (s.seek)((*execution()).file, num as usize, true);
            data
        } else {
            if allocator.len() < num as usize {
                allocator.resize(num as usize, 0);
            }
            (s.read_block)((*execution()).file, allocator.as_mut_ptr(), num as usize);
            allocator.as_ptr()
        };
        let mut reader = ByteReader::new(data, num as usize);
        let result = reader.byte().and_then(|a| {
            match a {
                0xFF => grf_msg!(2, "DecodeSpecialSprite: Unexpected data block, skipping"),
                0xFE => grf_msg!(2, "DecodeSpecialSprite: Unexpected import block, skipping"),
                0x00..=0x14 => {
                    grf_msg!(
                        7,
                        "DecodeSpecialSprite: Handling action 0x{:02X} in stage {}",
                        a,
                        stage
                    );
                    return dispatch(s, a, stage, &mut reader);
                }
                _ => grf_msg!(
                    7,
                    "DecodeSpecialSprite: Skipping unknown action 0x{:02X}",
                    a
                ),
            }
            Ok(())
        });
        if result.is_err() {
            grf_msg!(
                1,
                "DecodeSpecialSprite: Tried to read past end of pseudo-sprite data"
            );
            disable(s, s.read_bounds);
        }
    }
}
unsafe fn load_from_file(s: &Services, c: *mut c_void, stage: u8, f: *mut c_void) {
    unsafe {
        let old_file = (*execution()).file;
        let old_config = config();
        (*execution()).file = f;
        (*execution()).grfconfig = c;
        load_file_body(s, stage, f);
        (*execution()).file = old_file;
        (*execution()).grfconfig = old_config;
    }
}
unsafe fn load_file_body(s: &Services, stage: u8, f: *mut c_void) {
    unsafe {
        debug!(
            grf,
            2,
            "LoadNewGRFFile: Reading NewGRF-file '{}'",
            Bytes(config_filename(s, config()))
        );
        let version = (s.container_version)(f);
        if version == 0 {
            debug!(grf, 7, "LoadNewGRFFile: Custom .grf has invalid format");
            return;
        }
        if stage == GLS_INIT || stage == GLS_ACTIVATION {
            (s.sprite_offsets)(f);
        } else if version >= 2 {
            (s.read_dword)(f);
        }
        if version >= 2 && (s.read_byte)(f) != 0 {
            debug!(grf, 7, "LoadNewGRFFile: Unsupported compression format");
            return;
        }
        let size = if version >= 2 {
            (s.read_dword)(f)
        } else {
            u32::from((s.read_word)(f))
        };
        if size == 4 && (s.read_byte)(f) == 0xFF {
            (s.read_dword)(f);
        } else {
            debug!(grf, 7, "LoadNewGRFFile: Custom .grf has invalid format");
            return;
        }
        openttd_rust_grf_clear_file();
        let mut allocator = Vec::new();
        loop {
            let num = if version >= 2 {
                (s.read_dword)(f)
            } else {
                u32::from((s.read_word)(f))
            };
            if num == 0 {
                break;
            }
            let typ = (s.read_byte)(f);
            (*execution()).nfo_line = line().wrapping_add(1);
            if typ == 0xFF {
                if (*execution()).skip_sprites == 0 {
                    if num > 1024 * 1024 {
                        grf_msg!(0, "LoadNewGRFFile: Unexpectedly large sprite, disabling");
                        disable(s, s.unexpected_sprite);
                        break;
                    }
                    decode_special_sprite(s, &mut allocator, num, stage);
                    if (*execution()).skip_sprites == -1 {
                        break;
                    }
                    continue;
                }
                (s.skip)(f, num as usize);
            } else {
                if (*execution()).skip_sprites == 0 {
                    grf_msg!(0, "LoadNewGRFFile: Unexpected sprite, disabling");
                    disable(s, s.unexpected_sprite);
                    break;
                }
                if version >= 2 && typ == 0xFD {
                    (s.skip)(f, num as usize);
                } else {
                    (s.skip)(f, 7);
                    (s.skip_sprite_data)(f, typ, num.wrapping_sub(8) as u16);
                }
            }
            if (*execution()).skip_sprites > 0 {
                (*execution()).skip_sprites -= 1;
            }
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_load_file(
    s: &Services,
    c: *mut c_void,
    stage: u8,
    subdir: u8,
    temporary: *mut c_void,
) {
    unsafe {
        if stage != GLS_FILESCAN && stage != GLS_SAFETYSCAN && stage != GLS_LABELSCAN {
            (*execution()).grffile = (s.file_for_config)(c);
            if stage == GLS_RESERVE && (s.config_status)(c) != GCS_INITIALISED {
                return;
            }
            if stage == GLS_ACTIVATION && (s.config_flags)(c) & (1 << RESERVED) == 0 {
                return;
            }
        }
        let f = if temporary.is_null() {
            (s.open_cached)(c, subdir)
        } else {
            (s.open_temporary)(c, subdir, temporary)
        };
        load_from_file(s, c, stage, f);
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Clock {
    ticks: u64,
    display: u8,
}
#[derive(Clone, Copy)]
struct SavedClock {
    date: crate::services::CalendarDate,
    year: crate::timer_game_calendar::CalendarYear,
    date_fract: u16,
    economy_date: crate::services::EconomyDate,
    economy_year: crate::timer_game_economy::EconomyYear,
    economy_date_fract: u16,
    native: Clock,
}
impl SavedClock {
    unsafe fn read(s: &Services) -> Self {
        use crate::timer_game_common as clock;
        Self {
            date: clock::calendar_date(),
            year: clock::calendar_year(),
            date_fract: clock::calendar_fraction(),
            economy_date: clock::economy_date(),
            economy_year: clock::economy_year(),
            economy_date_fract: clock::economy_fraction(),
            native: unsafe { (s.clock)() },
        }
    }
    unsafe fn restore(self, s: &Services) {
        use crate::timer_game_common as clock;
        clock::write_calendar_date(self.date);
        clock::write_calendar_year(self.year);
        clock::write_calendar_fraction(self.date_fract);
        clock::write_economy_date(self.economy_date);
        clock::write_economy_year(self.economy_year);
        clock::write_economy_fraction(self.economy_date_fract);
        unsafe {
            (s.set_clock)(&raw const self.native);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_reset(s: &Services) {
    unsafe {
        crate::newgrf_text::openttd_rust_grf_text_cleanup();
        crate::newgrf_townname::clean_up();
        (s.reset_badges)();
        crate::engine_loading::openttd_rust_engine_setup(&*s.engine);
        (s.reset_bridges)();
        crate::type_registry::openttd_rust_rail_reset(s.types);
        crate::type_registry::openttd_rust_road_reset(s.types);
        crate::engine_loading::openttd_rust_engine_temp_initialize(&*s.engine);
        openttd_rust_grf_reset_grm();
        crate::newgrf_generic::openttd_rust_generic_reset(s.generic);
        (s.reset_prices)();
        (s.reset_currencies)();
        crate::house_definition::reset(&*s.houses);
        crate::industry::definitions::reset(
            crate::landscape_type::LandscapeType(s.shared().settings().climate.read()),
            &*s.industry,
        );
        crate::newgrf_class::object::reset(s.object_classes);
        (s.reset_objects)();
        crate::newgrf_class::station::reset(s.station_classes);
        crate::newgrf_class::airport::reset(s.airport_classes);
        crate::airport_definitions::reset(s.airports);
        crate::airport_definitions::reset_tiles(s.airports);
        crate::newgrf_class::roadstop::reset(s.roadstop_classes);
        (s.reset_water_features)();
        (s.reset_faces)();
        crate::snow_line::clear(&*(*s.global).shared);
        (*execution()).grfconfig = ptr::null_mut();
        (*execution()).grffile = ptr::null_mut();
        (*LABELS.get()).clear();
        (s.reset_files)();
        let mut i = 0;
        loop {
            let c = (s.config_at)(i);
            if c.is_null() {
                break;
            }
            (s.clear_errors)(c);
            i += 1;
        }
        crate::cargo_registry::openttd_rust_cargo_setup(
            s.cargo,
            s.shared().settings().climate.read(),
        );
        (s.set_misc)(0);
        (s.reset_loaded_features)();
        crate::engine_loading::openttd_rust_engine_override_clear();
        (s.initialize_sounds)();
        (s.clean_spritegroups)();
        crate::newgrf_act2::openttd_rust_action2_reset(false);
    }
}
unsafe fn after_load(s: &Services) {
    unsafe {
        crate::newgrf_act2::openttd_rust_action2_reset(true);
        (s.finalise_strings)();
        openttd_rust_grf_clear_overrides();
        (s.finalise_badges)();
        crate::cargo_registry::openttd_rust_cargo_finalise();
        crate::engine_loading::openttd_rust_engine_calculate_refits(&*s.engine);
        crate::engine_loading::openttd_rust_engine_finalise(&*s.engine);
        (s.finalise_canals)();
        crate::house_definition::finalise(&*s.houses);
        crate::industry::definitions::finalise(&*s.industry);
        (s.finalise_objects)();
        crate::cargo_registry::openttd_rust_cargo_sort(s.cargo);
        (s.sort_industries)();
        (s.industry_legend)();
        (s.linkstats_legend)();
        crate::airport_definitions::finalise(&*s.airports);
        crate::airport_definitions::bind(s.airports);
        crate::newgrf_townname::init_generator_names();
        crate::engine_loading::openttd_rust_vehicle_order_commit(&*s.engine);
        (s.activate_shore)();
        (s.activate_tram_depot)();
        crate::type_registry::rail_init();
        crate::type_registry::road_init();
        crate::engine_loading::openttd_rust_engine_bind_types(&*s.engine);
        (s.aging_stops)();
        (s.finalise_prices)();
        crate::engine_loading::openttd_rust_engine_temp_clear();
        openttd_rust_grf_clear_grm_sprites();
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_load(s: &Services, load_index: u32, num_baseset: u32) {
    unsafe {
        let saved = SavedClock::read(s);
        if (s.finance.networking)() {
            let year = (s.starting_year)();
            let clock = SavedClock {
                year: crate::timer_game_calendar::CalendarYear(year),
                date: crate::services::CalendarDate(
                    crate::timer_game_common::calendar_ymd_to_date(year, 0, 1),
                ),
                date_fract: 0,
                economy_year: crate::timer_game_economy::EconomyYear(year),
                economy_date: crate::services::EconomyDate(crate::timer_game_economy::ymd_to_date(
                    year,
                    0,
                    1,
                    (s.clocks.using_wallclock_units)(false),
                )),
                economy_date_fract: 0,
                native: Clock {
                    ticks: 0,
                    display: 0,
                },
            };
            clock.restore(s);
        }
        openttd_rust_grf_patch_flags(s);
        openttd_rust_grf_reset(s);
        let mut i = 0;
        loop {
            let c = (s.config_at)(i);
            if c.is_null() {
                break;
            }
            if (s.config_status)(c) != GCS_NOT_FOUND {
                (s.set_status)(c, GCS_UNKNOWN);
            }
            i += 1;
        }
        (*execution()).spriteid = load_index;
        for stage in GLS_LABELSCAN..=GLS_ACTIVATION {
            let mut i = 0;
            loop {
                let c = (s.config_at)(i);
                if c.is_null() {
                    break;
                }
                if (s.config_status)(c) == GCS_ACTIVATED {
                    (s.set_status)(c, GCS_INITIALISED);
                }
                i += 1;
            }
            if stage == GLS_RESERVE {
                for (source, target) in [
                    (0x4444_2202u32, 0x4444_0111u32),
                    (0x6D62_0402, 0x6D62_0401),
                    (0x4D65_6F20, 0x4D65_6F17),
                ] {
                    crate::engine_loading::openttd_rust_engine_set_override(
                        &*s.engine,
                        source.swap_bytes(),
                        target.swap_bytes(),
                    );
                }
            }
            let mut num_grfs = 0;
            let mut num_non_static = 0;
            (*execution()).stage = stage;
            let mut i = 0;
            loop {
                let c = (s.config_at)(i);
                if c.is_null() {
                    break;
                }
                i += 1;
                if matches!((s.config_status)(c), GCS_DISABLED | GCS_NOT_FOUND) {
                    continue;
                }
                let flags = (s.config_flags)(c);
                if stage > GLS_INIT && flags & (1 << INIT_ONLY) != 0 {
                    continue;
                }
                let subdir = if num_grfs < num_baseset {
                    s.baseset_dir
                } else {
                    s.newgrf_dir
                };
                if !(s.exists)(c, subdir) {
                    debug!(
                        grf,
                        0,
                        "NewGRF file is missing '{}'; disabling",
                        Bytes(config_filename(s, c))
                    );
                    (s.set_status)(c, GCS_NOT_FOUND);
                    continue;
                }
                if stage == GLS_LABELSCAN {
                    (s.init_file)(c);
                }
                if flags & ((1 << STATIC) | (1 << SYSTEM)) == 0 {
                    if num_non_static == crate::network_config::NETWORK_MAX_GRF_COUNT {
                        (s.set_status)(c, GCS_DISABLED);
                        debug!(
                            grf,
                            0,
                            "'{}' is not loaded as the maximum number of non-static GRFs has been reached",
                            Bytes(config_filename(s, c))
                        );
                        (s.too_many)(c);
                        continue;
                    }
                    num_non_static += 1;
                }
                num_grfs += 1;
                openttd_rust_grf_load_file(s, c, stage, subdir, ptr::null_mut());
                if stage == GLS_RESERVE {
                    (s.set_flag)(c, RESERVED, true);
                } else if stage == GLS_ACTIVATION {
                    (s.set_flag)(c, RESERVED, false);
                    (s.assert_file_match)(c, file());
                    clear_labels(file());
                    crate::cargo_registry::openttd_rust_cargo_build_inverse(file());
                    debug!(
                        sprite,
                        2,
                        "LoadNewGRF: Currently {} sprites are loaded",
                        (*execution()).spriteid
                    );
                } else if stage == GLS_INIT && (s.config_flags)(c) & (1 << INIT_ONLY) != 0 {
                    clear_labels(file());
                }
            }
        }
        (*execution()).grfconfig = ptr::null_mut();
        (*execution()).grffile = ptr::null_mut();
        openttd_rust_grf_clear_file();
        after_load(s);
        saved.restore(s);
    }
}

pub(crate) fn is_disabled() -> bool {
    unsafe { (*EXECUTION.get()).skip_sprites < 0 }
}

/// Test-only table for `crate::test_tables`; its private fields keep it in this module.
#[cfg(test)]
pub(crate) const TEST_TABLE: Services = Services {
    clocks: &crate::test_tables::CLOCK,
    finance: &crate::test_tables::FINANCE,
    global: std::ptr::null(),
    cargo: std::ptr::null(),
    engine: std::ptr::null(),
    houses: std::ptr::null(),
    industry: std::ptr::null(),
    airports: std::ptr::null(),
    types: std::ptr::null(),
    action2: std::ptr::null(),
    text: std::ptr::null(),
    bindings: std::ptr::null(),
    stations: std::ptr::null(),
    engine_range: crate::test_tables::unreachable1,
    config_md5: crate::test_tables::unreachable1,
    config_id: crate::test_tables::unreachable1,
    config_status: crate::test_tables::unreachable1,
    config_flags: crate::test_tables::unreachable1,
    set_status: crate::test_tables::unreachable2,
    set_flag: crate::test_tables::unreachable3,
    set_config_id: crate::test_tables::unreachable2,
    set_file_info: crate::test_tables::unreachable3,
    config_version: crate::test_tables::unreachable1,
    config_filename: crate::test_tables::unreachable1,
    config_palette: crate::test_tables::unreachable1,
    param_count: crate::test_tables::unreachable1,
    set_parameter: crate::test_tables::unreachable3,
    find_config: crate::test_tables::unreachable2,
    file_for_config: crate::test_tables::unreachable1,
    error: crate::test_tables::unreachable4,
    error_name: crate::test_tables::unreachable2,
    error_property: crate::test_tables::unreachable2,
    error_after_translated_file: crate::test_tables::unreachable1,
    load_after: 0,
    read_byte: crate::test_tables::unreachable1,
    read_word: crate::test_tables::unreachable1,
    read_dword: crate::test_tables::unreachable1,
    read_block: crate::test_tables::unreachable3,
    position: crate::test_tables::unreachable1,
    seek: crate::test_tables::unreachable3,
    skip: crate::test_tables::unreachable2,
    container_version: crate::test_tables::unreachable1,
    sprite_offsets: crate::test_tables::unreachable1,
    skip_sprite_data: crate::test_tables::unreachable3,
    open_cached: crate::test_tables::unreachable2,
    open_temporary: crate::test_tables::unreachable3,
    scan_name: crate::test_tables::unreachable4,
    scan_info: crate::test_tables::unreachable4,
    set_pitch: crate::test_tables::unreachable2,
    set_width: crate::test_tables::unreachable2,
    set_rail_cost: crate::test_tables::unreachable2,
    set_misc: crate::test_tables::unreachable1,
    safe_change_info: crate::test_tables::unreachable2,
    reserve_change_info: crate::test_tables::unreachable2,
    feature_change_info: crate::test_tables::unreachable2,
    skip_act1: crate::test_tables::unreachable2,
    new_sprite_set: crate::test_tables::unreachable2,
    skip_act5: crate::test_tables::unreachable2,
    graphics_new: crate::test_tables::unreachable2,
    skip_act_a: crate::test_tables::unreachable2,
    sprite_replace: crate::test_tables::unreachable2,
    grf_load_error: crate::test_tables::unreachable2,
    grf_comment: crate::test_tables::unreachable2,
    skip_act11: crate::test_tables::unreachable2,
    grf_sound: crate::test_tables::unreachable2,
    skip_act12: crate::test_tables::unreachable2,
    load_font_glyph: crate::test_tables::unreachable2,
    static_grf_info: crate::test_tables::unreachable2,
    gradual_loading: crate::test_tables::unreachable0,
    signal_side: crate::test_tables::unreachable0,
    freight_trains: crate::test_tables::unreachable0,
    wagon_speed_limits: crate::test_tables::unreachable0,
    improved_load: crate::test_tables::unreachable0,
    inflation: crate::test_tables::unreachable0,
    starting_year: crate::test_tables::unreachable0,
    plane_speed: crate::test_tables::unreachable0,
    read_bounds: 0,
    unexpected_sprite: 0,
    multiple_action8: 0,
    grm_failed: 0,
    static_desync: 0,
    list_property_too_long: 0,
    invalid_id: 0,
    force_disabled: 0,
    colourmap_sprite: 0,
    slopes_sprite: 0,
    shore_sprite: 0,
    generic: std::ptr::null(),
    clock: crate::test_tables::unreachable0,
    set_clock: crate::test_tables::unreachable1,
    config_at: crate::test_tables::unreachable1,
    exists: crate::test_tables::unreachable2,
    init_file: crate::test_tables::unreachable1,
    too_many: crate::test_tables::unreachable1,
    clear_errors: crate::test_tables::unreachable1,
    reset_badges: crate::test_tables::unreachable0,
    reset_bridges: crate::test_tables::unreachable0,
    reset_prices: crate::test_tables::unreachable0,
    reset_currencies: crate::test_tables::unreachable0,
    object_classes: std::ptr::null(),
    reset_objects: crate::test_tables::unreachable0,
    station_classes: std::ptr::null(),
    airport_classes: std::ptr::null(),
    roadstop_classes: std::ptr::null(),
    reset_water_features: crate::test_tables::unreachable0,
    reset_faces: crate::test_tables::unreachable0,
    reset_files: crate::test_tables::unreachable0,
    reset_loaded_features: crate::test_tables::unreachable0,
    initialize_sounds: crate::test_tables::unreachable0,
    clean_spritegroups: crate::test_tables::unreachable0,
    finalise_strings: crate::test_tables::unreachable0,
    finalise_badges: crate::test_tables::unreachable0,
    finalise_canals: crate::test_tables::unreachable0,
    finalise_objects: crate::test_tables::unreachable0,
    sort_industries: crate::test_tables::unreachable0,
    industry_legend: crate::test_tables::unreachable0,
    linkstats_legend: crate::test_tables::unreachable0,
    activate_shore: crate::test_tables::unreachable0,
    activate_tram_depot: crate::test_tables::unreachable0,
    aging_stops: crate::test_tables::unreachable0,
    finalise_prices: crate::test_tables::unreachable0,
    baseset_dir: 0,
    newgrf_dir: 0,
    assert_override_size: crate::test_tables::unreachable2,
    assert_file_match: crate::test_tables::unreachable2,
    null_error: 0,
    undefined: 0,
};
