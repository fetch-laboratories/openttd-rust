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
use crate::game_state::GameState;
use crate::newgrf::GSF_END;
use std::{collections::BTreeMap, ffi::c_void, ptr};

#[repr(C)]
pub struct Execution {
    stage: u8,
    spriteid: u32,
    file: *mut c_void,
    grffile: *mut c_void,
    grfconfig: *mut c_void,
    nfo_line: u32,
    skip_sprites: i32,
    spritegroups: [*const c_void; 256],
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
pub unsafe extern "C" fn openttd_rust_grf_has_sets(feature: u8) -> bool {
    unsafe { !(*STATE.get()).spritesets[usize::from(feature)].is_empty() }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_has_set(feature: u8, set: u32) -> bool {
    unsafe { (*STATE.get()).spritesets[usize::from(feature)].contains_key(&set) }
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
    "OpenTTDGrfLoadingServices" ["grf_loading"] => Services { global, engine_range, parameter, file_id, config_id, config_status, config_flags, set_status, set_flag, set_config_id, set_file_info, config_version, param_count, set_parameter, find_config, find_file, file_for_config, error, error_name, networking, cargo_label, rail_label, road_label, road_is_road, road_is_tram, read_byte, read_word, read_dword, read_block, position, seek, skip, container_version, sprite_offsets, skip_sprite_data, open_cached, open_temporary, scan_name, scan_info, set_pitch, set_width, set_rail_cost, set_misc, safe_change_info, reserve_change_info, feature_change_info, skip_act1, new_sprite_set, new_sprite_group, feature_map_sprite_group, feature_new_name, skip_act5, graphics_new, skip_act_a, sprite_replace, grf_load_error, grf_comment, feature_town_name, skip_act11, grf_sound, skip_act12, load_font_glyph, translate_grf_strings, static_grf_info, never_expire_airports, max_bridge_length, never_expire_vehicles, station_noise, gradual_loading, signal_side, disable_elrails, build_on_slopes, freight_trains, wagon_speed_limits, allow_town_roads, generating_world, improved_load, dynamic_engines, inflation, starting_year, plane_speed, height_limit, generation_seed, log_x, log_y, read_bounds, unexpected_sprite, multiple_action8, grm_failed, static_desync, force_disabled, colourmap_sprite, slopes_sprite, shore_sprite , generic, clock, set_clock, economy_ymd_to_date, config_at, exists, init_file, override_grf, too_many, clear_errors, cleanup_strings, cleanup_townnames, reset_badges, setup_engines, reset_bridges, reset_railtypes, reset_roadtypes, init_engine_loading, reset_prices, reset_currencies, reset_houses, reset_industries, reset_object_classes, reset_objects, reset_station_classes, reset_airport_classes, reset_airports, reset_airporttiles, reset_roadstop_classes, reset_water_features, reset_faces, clear_snowline, reset_files, setup_cargo, reset_loaded_features, clear_engine_overrides, initialize_sounds, clean_spritegroups, reset_callbacks, finish_callbacks, finalise_strings, finalise_badges, finalise_cargo, calculate_refits, finalise_engines, finalise_canals, finalise_houses, finalise_industries, finalise_objects, sort_cargo, sort_industries, industry_legend, linkstats_legend, finalise_airports, bind_airports, townnames, vehicle_list_orders, activate_shore, activate_tram_depot, init_railtypes, init_roadtypes, aging_stops, finalise_prices, clear_engine_loading, cargo_translation_map, bind_engine_types, baseset_dir, newgrf_dir , assert_override_size, assert_file_match, unexpected_large, unexpected_real, missing_file, sprite_grm_failed, grm_failure, unknown_operation, id_mismatch, invalid_version , null_error, unknown_action, handling_action, preloaded_sprite, unexpected_data, unexpected_import, read_bounds_error, reading_file, invalid_format, unsupported_compression, loaded_sprites };
    "OpenTTDGrfEngineRange" ["grf_loading"] => EngineRange { offset, count };
    "OpenTTDGrfClock" ["grf_loading"] => Clock { date, year, date_fract, economy_date, economy_year, economy_date_fract, ticks, display };
    "OpenTTDGrfExecution" ["grf_loading"] => Execution {
        stage, spriteid, file, grffile, grfconfig, nfo_line, skip_sprites, spritegroups
    };
}

#[repr(C)]
pub struct Services {
    global: *const crate::newgrf::Services,
    engine_range: unsafe extern "C" fn(u8) -> EngineRange,
    parameter: unsafe extern "C" fn(*const c_void, u32) -> u32,
    file_id: unsafe extern "C" fn(*const c_void) -> u32,
    config_id: unsafe extern "C" fn(*const c_void) -> u32,
    config_status: unsafe extern "C" fn(*const c_void) -> u8,
    config_flags: unsafe extern "C" fn(*const c_void) -> u8,
    set_status: unsafe extern "C" fn(*mut c_void, u8),
    set_flag: unsafe extern "C" fn(*mut c_void, u8, bool),
    set_config_id: unsafe extern "C" fn(*mut c_void, u32),
    set_file_info: unsafe extern "C" fn(*mut c_void, u32, u8),
    config_version: unsafe extern "C" fn(*const c_void) -> u32,
    param_count: unsafe extern "C" fn(*const c_void) -> usize,
    set_parameter: unsafe extern "C" fn(*mut c_void, u8, u32),
    find_config: unsafe extern "C" fn(u32, u32) -> *mut c_void,
    find_file: unsafe extern "C" fn(u32) -> *mut c_void,
    file_for_config: unsafe extern "C" fn(*const c_void) -> *mut c_void,
    error: unsafe extern "C" fn(*mut c_void, u32, u32, bool) -> *mut c_void,
    error_name: unsafe extern "C" fn(*mut c_void, *const c_void),
    networking: unsafe extern "C" fn() -> bool,
    cargo_label: unsafe extern "C" fn(u32) -> u8,
    rail_label: unsafe extern "C" fn(u32) -> u8,
    road_label: unsafe extern "C" fn(u32) -> u8,
    road_is_road: unsafe extern "C" fn(u8) -> bool,
    road_is_tram: unsafe extern "C" fn(u8) -> bool,
    read_byte: unsafe extern "C" fn(*mut c_void) -> u8,
    read_word: unsafe extern "C" fn(*mut c_void) -> u16,
    read_dword: unsafe extern "C" fn(*mut c_void) -> u32,
    read_block: unsafe extern "C" fn(*mut c_void, *mut u8, usize),
    position: unsafe extern "C" fn(*const c_void) -> usize,
    seek: unsafe extern "C" fn(*mut c_void, usize, bool),
    skip: unsafe extern "C" fn(*mut c_void, usize),
    container_version: unsafe extern "C" fn(*const c_void) -> u8,
    sprite_offsets: unsafe extern "C" fn(*mut c_void),
    skip_sprite_data: unsafe extern "C" fn(*mut c_void, u8, u32),
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
    new_sprite_group: unsafe extern "C" fn(*const u8, usize) -> bool,
    feature_map_sprite_group: unsafe extern "C" fn(*const u8, usize) -> bool,
    feature_new_name: unsafe extern "C" fn(*const u8, usize) -> bool,
    skip_act5: unsafe extern "C" fn(*const u8, usize) -> bool,
    graphics_new: unsafe extern "C" fn(*const u8, usize) -> bool,
    skip_act_a: unsafe extern "C" fn(*const u8, usize) -> bool,
    sprite_replace: unsafe extern "C" fn(*const u8, usize) -> bool,
    grf_load_error: unsafe extern "C" fn(*const u8, usize) -> bool,
    grf_comment: unsafe extern "C" fn(*const u8, usize) -> bool,
    feature_town_name: unsafe extern "C" fn(*const u8, usize) -> bool,
    skip_act11: unsafe extern "C" fn(*const u8, usize) -> bool,
    grf_sound: unsafe extern "C" fn(*const u8, usize) -> bool,
    skip_act12: unsafe extern "C" fn(*const u8, usize) -> bool,
    load_font_glyph: unsafe extern "C" fn(*const u8, usize) -> bool,
    translate_grf_strings: unsafe extern "C" fn(*const u8, usize) -> bool,
    static_grf_info: unsafe extern "C" fn(*const u8, usize) -> bool,
    never_expire_airports: unsafe extern "C" fn() -> bool,
    max_bridge_length: unsafe extern "C" fn() -> u16,
    never_expire_vehicles: unsafe extern "C" fn() -> bool,
    station_noise: unsafe extern "C" fn() -> bool,
    gradual_loading: unsafe extern "C" fn() -> bool,
    signal_side: unsafe extern "C" fn() -> u8,
    disable_elrails: unsafe extern "C" fn() -> bool,
    build_on_slopes: unsafe extern "C" fn() -> bool,
    freight_trains: unsafe extern "C" fn() -> u8,
    wagon_speed_limits: unsafe extern "C" fn() -> bool,
    allow_town_roads: unsafe extern "C" fn() -> bool,
    generating_world: unsafe extern "C" fn() -> bool,
    improved_load: unsafe extern "C" fn() -> bool,
    dynamic_engines: unsafe extern "C" fn() -> bool,
    inflation: unsafe extern "C" fn() -> bool,
    starting_year: unsafe extern "C" fn() -> i32,
    plane_speed: unsafe extern "C" fn() -> u8,
    height_limit: unsafe extern "C" fn() -> u8,
    generation_seed: unsafe extern "C" fn() -> u32,
    log_x: unsafe extern "C" fn() -> u32,
    log_y: unsafe extern "C" fn() -> u32,
    read_bounds: u32,
    unexpected_sprite: u32,
    multiple_action8: u32,
    grm_failed: u32,
    static_desync: u32,
    force_disabled: u32,
    colourmap_sprite: u32,
    slopes_sprite: u32,
    shore_sprite: u32,
    generic: *const crate::newgrf_generic::Services,
    clock: unsafe extern "C" fn() -> Clock,
    set_clock: unsafe extern "C" fn(*const Clock),
    economy_ymd_to_date: unsafe extern "C" fn(i32, u8, u8) -> i32,
    config_at: unsafe extern "C" fn(usize) -> *mut c_void,
    exists: unsafe extern "C" fn(*const c_void, u8) -> bool,
    init_file: unsafe extern "C" fn(*const c_void),
    override_grf: unsafe extern "C" fn(u32, u32),
    too_many: unsafe extern "C" fn(*mut c_void),
    clear_errors: unsafe extern "C" fn(*mut c_void),
    cleanup_strings: unsafe extern "C" fn(),
    cleanup_townnames: unsafe extern "C" fn(),
    reset_badges: unsafe extern "C" fn(),
    setup_engines: unsafe extern "C" fn(),
    reset_bridges: unsafe extern "C" fn(),
    reset_railtypes: unsafe extern "C" fn(),
    reset_roadtypes: unsafe extern "C" fn(),
    init_engine_loading: unsafe extern "C" fn(),
    reset_prices: unsafe extern "C" fn(),
    reset_currencies: unsafe extern "C" fn(),
    reset_houses: unsafe extern "C" fn(),
    reset_industries: unsafe extern "C" fn(),
    reset_object_classes: unsafe extern "C" fn(),
    reset_objects: unsafe extern "C" fn(),
    reset_station_classes: unsafe extern "C" fn(),
    reset_airport_classes: unsafe extern "C" fn(),
    reset_airports: unsafe extern "C" fn(),
    reset_airporttiles: unsafe extern "C" fn(),
    reset_roadstop_classes: unsafe extern "C" fn(),
    reset_water_features: unsafe extern "C" fn(),
    reset_faces: unsafe extern "C" fn(),
    clear_snowline: unsafe extern "C" fn(),
    reset_files: unsafe extern "C" fn(),
    setup_cargo: unsafe extern "C" fn(),
    reset_loaded_features: unsafe extern "C" fn(),
    clear_engine_overrides: unsafe extern "C" fn(),
    initialize_sounds: unsafe extern "C" fn(),
    clean_spritegroups: unsafe extern "C" fn(),
    reset_callbacks: unsafe extern "C" fn(),
    finish_callbacks: unsafe extern "C" fn(),
    finalise_strings: unsafe extern "C" fn(),
    finalise_badges: unsafe extern "C" fn(),
    finalise_cargo: unsafe extern "C" fn(),
    calculate_refits: unsafe extern "C" fn(),
    finalise_engines: unsafe extern "C" fn(),
    finalise_canals: unsafe extern "C" fn(),
    finalise_houses: unsafe extern "C" fn(),
    finalise_industries: unsafe extern "C" fn(),
    finalise_objects: unsafe extern "C" fn(),
    sort_cargo: unsafe extern "C" fn(),
    sort_industries: unsafe extern "C" fn(),
    industry_legend: unsafe extern "C" fn(),
    linkstats_legend: unsafe extern "C" fn(),
    finalise_airports: unsafe extern "C" fn(),
    bind_airports: unsafe extern "C" fn(),
    townnames: unsafe extern "C" fn(),
    vehicle_list_orders: unsafe extern "C" fn(),
    activate_shore: unsafe extern "C" fn(),
    activate_tram_depot: unsafe extern "C" fn(),
    init_railtypes: unsafe extern "C" fn(),
    init_roadtypes: unsafe extern "C" fn(),
    aging_stops: unsafe extern "C" fn(),
    finalise_prices: unsafe extern "C" fn(),
    clear_engine_loading: unsafe extern "C" fn(),
    cargo_translation_map: unsafe extern "C" fn(),
    bind_engine_types: unsafe extern "C" fn(),
    baseset_dir: u8,
    newgrf_dir: u8,
    assert_override_size: unsafe extern "C" fn(usize, u32),
    assert_file_match: unsafe extern "C" fn(*const c_void, *const c_void),
    unexpected_large: unsafe extern "C" fn(),
    unexpected_real: unsafe extern "C" fn(),
    missing_file: unsafe extern "C" fn(*const c_void),
    sprite_grm_failed: unsafe extern "C" fn(u16),
    grm_failure: unsafe extern "C" fn(u16, bool, bool),
    unknown_operation: unsafe extern "C" fn(u8),
    id_mismatch: unsafe extern "C" fn(u32, u32),
    invalid_version: unsafe extern "C" fn(*const c_void, u32, u8, *const u8, usize),
    null_error: u32,
    unknown_action: unsafe extern "C" fn(u8),
    handling_action: unsafe extern "C" fn(u8, u8),
    preloaded_sprite: unsafe extern "C" fn(),
    unexpected_data: unsafe extern "C" fn(),
    unexpected_import: unsafe extern "C" fn(),
    read_bounds_error: unsafe extern "C" fn(),
    reading_file: unsafe extern "C" fn(*const c_void),
    invalid_format: unsafe extern "C" fn(),
    unsupported_compression: unsafe extern "C" fn(),
    loaded_sprites: unsafe extern "C" fn(u32),
}

use crate::newgrf::{
    GLS_ACTIVATION, GLS_FILESCAN, GLS_INIT, GLS_LABELSCAN, GLS_RESERVE, GLS_SAFETYSCAN,
};
use crate::newgrf_bytereader::{ByteReader, ReadBounds};
use crate::newgrf_config::{
    GCS_ACTIVATED, GCS_DISABLED, GCS_INITIALISED, GCS_NOT_FOUND, GCS_UNKNOWN, INIT_ONLY, INVALID,
    RESERVED, STATIC, SYSTEM, UNSAFE,
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
unsafe fn file() -> *mut c_void {
    unsafe { (*execution()).grffile }
}
unsafe fn config() -> *mut c_void {
    unsafe { (*execution()).grfconfig }
}
unsafe fn stage() -> u8 {
    unsafe { (*execution()).stage }
}
unsafe fn line() -> u32 {
    unsafe { (*execution()).nfo_line }
}
unsafe fn is_static(s: &Services, config: *const c_void) -> bool {
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
pub unsafe extern "C" fn openttd_rust_grf_reserved_sprite(
    grfid: u32,
    first: u32,
    count: u16,
) -> bool {
    unsafe {
        (*GRM_SPRITES.get())
            .iter()
            .any(|(&(id, _), &(sprite, num))| {
                id == grfid
                    && sprite <= first
                    && sprite.wrapping_add(u32::from(num)) >= first.wrapping_add(u32::from(count))
            })
    }
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
            (s.find_file)((s.config_id)(c))
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
        if crate::newgrf::openttd_rust_global_variable(
            &*s.global,
            param.wrapping_sub(0x80),
            &raw mut value,
            file(),
        ) {
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
            p if p < 0x80 => (s.parameter)(file(), u32::from(p)),
            _ => u32::MAX,
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
                break;
            }
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
            return Ok(());
        }
        let result = if cond >= 11 {
            match cond {
                11 | 12 => {
                    let valid =
                        crate::cargo_type::is_valid_cargo_type((s.cargo_label)(value.swap_bytes()));
                    if cond == 11 { !valid } else { valid }
                }
                13 | 14 => {
                    let valid =
                        (s.rail_label)(value.swap_bytes()) != crate::rail_type::INVALID_RAILTYPE;
                    if cond == 13 { !valid } else { valid }
                }
                15..=18 => {
                    let rt = (s.road_label)(value.swap_bytes());
                    let valid = rt != crate::road_type::INVALID_ROADTYPE
                        && if cond < 17 {
                            (s.road_is_road)(rt)
                        } else {
                            (s.road_is_tram)(rt)
                        };
                    if cond % 2 == 1 { !valid } else { valid }
                }
                _ => return Ok(()),
            }
        } else if pn == 0x88 {
            let mut c = (s.find_config)(value, mask);
            if !c.is_null() && is_static(s, c) && !is_static(s, config()) && (s.networking)() {
                static_desync(s, c);
                c = ptr::null_mut();
            }
            if cond != 10 && c.is_null() {
                return Ok(());
            }
            match cond {
                6 => (s.config_status)(c) == GCS_ACTIVATED,
                7 => (s.config_status)(c) != GCS_ACTIVATED,
                8 => (s.config_status)(c) == GCS_INITIALISED,
                9 => matches!((s.config_status)(c), GCS_ACTIVATED | GCS_INITIALISED),
                10 => c.is_null() || matches!((s.config_status)(c), GCS_DISABLED | GCS_NOT_FOUND),
                _ => return Ok(()),
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
                _ => return Ok(()),
            }
        };
        if !result {
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
            (s.seek)((*execution()).file, label.pos, false);
            (*execution()).nfo_line = label.nfo_line;
            return Ok(());
        }
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
unsafe fn grf_info(s: &Services, b: &mut ByteReader, scan: bool) -> Result<(), ReadBounds> {
    unsafe {
        let version = b.byte()?;
        let id = b.dword()?;
        let (name, size) = b.string();
        if scan {
            (s.set_config_id)(config(), id);
            if !(2..=8).contains(&version) {
                (s.set_flag)(config(), INVALID, true);
                (s.invalid_version)(config(), id, version, name, size);
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
            let old_id = (s.file_id)(file());
            if old_id != id {
                (s.id_mismatch)(old_id, id);
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
                let x = (s.log_x)().wrapping_sub(6) as u8;
                let y = (s.log_y)().wrapping_sub(6) as u8;
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
            0x14 => u32::from((s.height_limit)()),
            0x15 => s.slopes_sprite,
            0x16 => s.shore_sprite,
            0x17 => (s.generation_seed)(),
            _ => 0,
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
                .add((s.parameter)(file(), u32::from(target)) as usize)
                .read();
        }
        let mut start = if op == 2 || op == 3 {
            (s.parameter)(file(), u32::from(target))
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
                let id = (s.file_id)(file());
                for i in 0..u32::from(count) {
                    grm.add(start.wrapping_add(i) as usize).write(id);
                }
            }
            return start;
        }
        (s.grm_failure)(
            count,
            grm != (*GRM_CARGOES.get()).as_mut_ptr(),
            op != 4 && op != 5,
        );
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
                                (s.sprite_grm_failed)(count);
                                disable(s, s.grm_failed);
                                return Ok(());
                            }
                            (*GRM_SPRITES.get())
                                .insert(((s.file_id)(file()), line()), (sprite, count));
                            (*execution()).spriteid = sprite.wrapping_add(u32::from(count));
                        }
                        src1 = 0;
                    } else if stage() == GLS_ACTIVATION {
                        match feature {
                            0..=3 => {
                                if !(s.dynamic_engines)() {
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
                                        (s.parameter)(file(), u32::from(target))
                                    } else {
                                        0
                                    };
                                }
                            }
                            crate::newgrf::GSF_GLOBALVAR => {
                                src1 = match operation {
                                    0 => {
                                        (*GRM_SPRITES.get())
                                            .entry(((s.file_id)(file()), line()))
                                            .or_default()
                                            .0
                                    }
                                    1 => (*execution()).spriteid,
                                    _ => return Ok(()),
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
                            _ => return Ok(()),
                        }
                    } else {
                        src1 = 0;
                    }
                }
            } else {
                let f = (s.find_file)(data);
                let c = (s.find_config)(data, u32::MAX);
                if !c.is_null() && is_static(s, c) && !is_static(s, config()) && (s.networking)() {
                    static_desync(s, c);
                    src1 = 0;
                } else if f.is_null() || c.is_null() || (s.config_status)(c) == GCS_DISABLED {
                    src1 = 0;
                } else if src1 == 0xFE {
                    src1 = (s.config_version)(c);
                } else {
                    src1 = (s.parameter)(f, src1);
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
                (s.unknown_operation)(op);
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
                if (s.disable_elrails)() {
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
            t if t < 128 => (s.set_parameter)(file(), t, result),
            _ => (),
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
            | (u32::from((s.never_expire_airports)()) << 0x0C)
            | (u32::from((s.max_bridge_length)() > 16) << 0x0F)
            | (u32::from((s.never_expire_vehicles)()) << 0x16);
        (*PATCH_FLAGS.get())[1] = 0x07DC_0300
            | (u32::from((s.station_noise)()) << 0x07)
            | (u32::from((s.gradual_loading)()) << 0x0C)
            | (u32::from((s.signal_side)() == 1) << 0x1B)
            | (u32::from(!((s.disable_elrails)())) << 0x1C);
        (*PATCH_FLAGS.get())[2] = 0x5EDC_C40A
            | (u32::from((s.build_on_slopes)()) << 0x0D)
            | (u32::from((s.build_on_slopes)()) << 0x15)
            | (u32::from((s.freight_trains)() > 1) << 0x18)
            | (u32::from((s.wagon_speed_limits)()) << 0x1D);
        (*PATCH_FLAGS.get())[3] = 0xC0D7_F8EA
            | (u32::from(!((s.allow_town_roads)() || (s.generating_world)())) << 0x02)
            | (u32::from((s.improved_load)()) << 0x08)
            | (u32::from((s.dynamic_engines)()) << 0x18);
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
            (14, GLS_SAFETYSCAN) => return inhibit(s, b, true),
            (14, GLS_INIT | GLS_RESERVE | GLS_ACTIVATION) => return inhibit(s, b, false),
            (16, GLS_LABELSCAN) => return define_label(s, b),
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
            (2, GLS_ACTIVATION) => (s.new_sprite_group)(data, len),
            (3 | 15 | 17, GLS_SAFETYSCAN) => {
                openttd_rust_grf_unsafe(s);
                true
            }
            (3, GLS_ACTIVATION) => (s.feature_map_sprite_group)(data, len),
            (4, GLS_ACTIVATION) => (s.feature_new_name)(data, len),
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
            (15, GLS_INIT) => (s.feature_town_name)(data, len),
            (17, GLS_FILESCAN | GLS_LABELSCAN | GLS_RESERVE) => (s.skip_act11)(data, len),
            (17, GLS_INIT | GLS_ACTIVATION) => (s.grf_sound)(data, len),
            (18, GLS_FILESCAN | GLS_SAFETYSCAN | GLS_LABELSCAN | GLS_INIT | GLS_RESERVE) => {
                (s.skip_act12)(data, len)
            }
            (18, GLS_ACTIVATION) => (s.load_font_glyph)(data, len),
            (19, GLS_ACTIVATION) => (s.translate_grf_strings)(data, len),
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
            (s.preloaded_sprite)();
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
                0xFF => (s.unexpected_data)(),
                0xFE => (s.unexpected_import)(),
                0x00..=0x14 => {
                    (s.handling_action)(a, stage);
                    return dispatch(s, a, stage, &mut reader);
                }
                _ => (s.unknown_action)(a),
            }
            Ok(())
        });
        if result.is_err() {
            (s.read_bounds_error)();
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
        (s.reading_file)(config());
        let version = (s.container_version)(f);
        if version == 0 {
            (s.invalid_format)();
            return;
        }
        if stage == GLS_INIT || stage == GLS_ACTIVATION {
            (s.sprite_offsets)(f);
        } else if version >= 2 {
            (s.read_dword)(f);
        }
        if version >= 2 && (s.read_byte)(f) != 0 {
            (s.unsupported_compression)();
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
            (s.invalid_format)();
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
                        (s.unexpected_large)();
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
                    (s.unexpected_real)();
                    disable(s, s.unexpected_sprite);
                    break;
                }
                if version >= 2 && typ == 0xFD {
                    (s.skip)(f, num as usize);
                } else {
                    (s.skip)(f, 7);
                    (s.skip_sprite_data)(f, typ, num.wrapping_sub(8));
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
    date: i32,
    year: i32,
    date_fract: u16,
    economy_date: i32,
    economy_year: i32,
    economy_date_fract: u16,
    ticks: u64,
    display: u8,
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_reset(s: &Services) {
    unsafe {
        (s.cleanup_strings)();
        (s.cleanup_townnames)();
        (s.reset_badges)();
        (s.setup_engines)();
        (s.reset_bridges)();
        (s.reset_railtypes)();
        (s.reset_roadtypes)();
        (s.init_engine_loading)();
        openttd_rust_grf_reset_grm();
        crate::newgrf_generic::openttd_rust_generic_reset(s.generic);
        (s.reset_prices)();
        (s.reset_currencies)();
        (s.reset_houses)();
        (s.reset_industries)();
        (s.reset_object_classes)();
        (s.reset_objects)();
        (s.reset_station_classes)();
        (s.reset_airport_classes)();
        (s.reset_airports)();
        (s.reset_airporttiles)();
        (s.reset_roadstop_classes)();
        (s.reset_water_features)();
        (s.reset_faces)();
        (s.clear_snowline)();
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
        (s.setup_cargo)();
        (s.set_misc)(0);
        (s.reset_loaded_features)();
        (s.clear_engine_overrides)();
        (s.initialize_sounds)();
        (s.clean_spritegroups)();
        (s.reset_callbacks)();
    }
}
unsafe fn after_load(s: &Services) {
    unsafe {
        (s.finish_callbacks)();
        (s.finalise_strings)();
        openttd_rust_grf_clear_overrides();
        (s.finalise_badges)();
        (s.finalise_cargo)();
        (s.calculate_refits)();
        (s.finalise_engines)();
        (s.finalise_canals)();
        (s.finalise_houses)();
        (s.finalise_industries)();
        (s.finalise_objects)();
        (s.sort_cargo)();
        (s.sort_industries)();
        (s.industry_legend)();
        (s.linkstats_legend)();
        (s.finalise_airports)();
        (s.bind_airports)();
        (s.townnames)();
        (s.vehicle_list_orders)();
        (s.activate_shore)();
        (s.activate_tram_depot)();
        (s.init_railtypes)();
        (s.init_roadtypes)();
        (s.bind_engine_types)();
        (s.aging_stops)();
        (s.finalise_prices)();
        (s.clear_engine_loading)();
        openttd_rust_grf_clear_grm_sprites();
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_load(s: &Services, load_index: u32, num_baseset: u32) {
    unsafe {
        let saved = (s.clock)();
        if (s.networking)() {
            let year = (s.starting_year)();
            let clock = Clock {
                year,
                date: ((*s.global).ymd_to_date)(year, 0, 1),
                date_fract: 0,
                economy_year: year,
                economy_date: (s.economy_ymd_to_date)(year, 0, 1),
                economy_date_fract: 0,
                ticks: 0,
                display: 0,
            };
            (s.set_clock)(&raw const clock);
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
                    (s.override_grf)(source.swap_bytes(), target.swap_bytes());
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
                    (s.missing_file)(c);
                    (s.set_status)(c, GCS_NOT_FOUND);
                    continue;
                }
                if stage == GLS_LABELSCAN {
                    (s.init_file)(c);
                }
                if flags & ((1 << STATIC) | (1 << SYSTEM)) == 0 {
                    if num_non_static == crate::network_config::NETWORK_MAX_GRF_COUNT {
                        (s.set_status)(c, GCS_DISABLED);
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
                    (s.cargo_translation_map)();
                    (s.loaded_sprites)((*execution()).spriteid);
                } else if stage == GLS_INIT && (s.config_flags)(c) & (1 << INIT_ONLY) != 0 {
                    clear_labels(file());
                }
            }
        }
        (*execution()).grfconfig = ptr::null_mut();
        (*execution()).grffile = ptr::null_mut();
        openttd_rust_grf_clear_file();
        after_load(s);
        (s.set_clock)(&raw const saved);
    }
}
