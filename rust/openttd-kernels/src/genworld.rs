/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Complete world-generation lifecycle and canonical process-wide state.
//! Native map/pool/resources remain behind typed services. The native bool
//! alias shares static storage; accesses hold the game-state mutex and end
//! before any progress yield or synchronous callback. No panic unwinds.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::too_many_lines,
    clippy::cast_possible_truncation
)]
/// `GenWorldProgress : uint8_t`.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Progress(pub u8);
/// `GWP_LANDSCAPE`.
pub const GWP_LANDSCAPE: Progress = Progress(1);
/// `GWP_RIVER`.
pub const GWP_RIVER: Progress = Progress(2);
/// `GWM_HEIGHTMAP`.
pub const GWM_HEIGHTMAP: u8 = 3;
/// `LG_ORIGINAL`.
pub const LG_ORIGINAL: u8 = 0;
/// `LG_TERRAGENESIS`.
pub const LG_TERRAGENESIS: u8 = 1;
/// `CUSTOM_SEA_LEVEL_NUMBER_DIFFICULTY`.
pub const CUSTOM_SEA_LEVEL_NUMBER_DIFFICULTY: u8 = 4;

use crate::bitmath_func::gb;
use crate::clear_map::{CLEAR_DESERT, CLEAR_ROCKS, CLEAR_ROUGH};
use crate::company_type::CompanyID;
use crate::game_state::GameState;
use crate::landscape_generation;
use crate::openttd::{GameMode, SwitchMode};
use crate::raw_field::{read_field, write_field};
use crate::services::ClearGroundType;
use crate::tile_type::{DEF_SNOWLINE_HEIGHT, MAX_MAP_HEIGHT_LIMIT, MP_CLEAR, TileIndex};

/// Native `GenWorldMode`, retaining its full byte domain.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Mode(pub u8);
pub const GWM_EMPTY: Mode = Mode(1);
pub const MAP_HEIGHT_LIMIT_AUTO_MINIMUM: u32 = 30;
pub const MAP_HEIGHT_LIMIT_AUTO_CEILING_ROOM: u32 = 15;
pub const GENERATE_NEW_SEED: u32 = u32::MAX;
pub const GWP_MAP_INIT: Progress = Progress(0);
pub const GWP_ROUGH_ROCKY: Progress = Progress(3);
pub const GWP_OBJECT: Progress = Progress(7);
pub const GWP_GAME_INIT: Progress = Progress(9);
pub const GWP_RUNTILELOOP: Progress = Progress(10);
pub const GWP_RUNSCRIPT: Progress = Progress(11);
pub const GWP_GAME_START: Progress = Progress(12);
pub type Callback = Option<extern "C" fn()>;

struct Info {
    abort: bool,
    mode: Mode,
    local_company: CompanyID,
    size_x: u32,
    size_y: u32,
    done: Callback,
    abort_callback: Callback,
    generating: bool,
}
// Native bool lvalues address only `generating`. No owner reference is formed,
// and every field access ends before progress/yield or callback reentry.
static INFO: GameState<Info> = GameState::new(Info {
    abort: false,
    mode: Mode(0),
    local_company: CompanyID(0),
    size_x: 0,
    size_y: 0,
    done: None,
    abort_callback: None,
    generating: false,
});
#[repr(C)]
pub struct Frame {
    _private: [u8; 0],
}
#[repr(C)]
pub struct Settings {
    pub height_limit: u8,
    pub flat_height: u8,
    pub heightmap_height: u8,
    pub generator: u8,
}
#[repr(C)]
pub struct LandscapeCall {
    pub frame: *mut landscape_generation::Frame,
    pub decoder: *const landscape_generation::Decoder,
    pub settings: landscape_generation::Settings,
}
#[repr(C)]
pub struct TownCall {
    pub frame: *mut crate::town::founding::Frame,
    pub names: *mut crate::town::names::NameFrame,
    pub road: *const crate::road_cmd::CommandContext,
    pub layout: crate::town::founding::TownLayout,
}
#[repr(C)]
pub struct ObjectCall {
    pub frame: *mut crate::objects::GenerationFrame,
    pub out: *mut crate::services::CommandCost,
    pub tmp: *mut crate::services::CommandCost,
    pub aux: *mut crate::services::CommandCost,
}

#[repr(C)]
pub struct WorldServices {
    pub landscape: *const landscape_generation::GenerationServices,
    pub company: *const crate::timer_game_common::ClockCompany,
    pub town: *const crate::town::founding::FoundingServices,
    pub industry: *const crate::industry::Services,
    pub industry_generation: *const crate::industry::GenerationServices,
    pub objects: *const crate::objects::ObjectServices,
    pub engines: *const crate::engine::Services,
    pub disasters: *const crate::disaster::Services,
    pub modal: extern "C" fn() -> bool,
    pub set_modal: extern "C" fn(bool),
    pub local_company: extern "C" fn() -> CompanyID,
    pub set_local_company: extern "C" fn(CompanyID),
    pub restore_companies: extern "C" fn(CompanyID),
    pub initialize: extern "C" fn(*mut Frame, u32, u32, bool) -> bool,
    pub prepare_progress: extern "C" fn(),
    pub settings: extern "C" fn() -> Settings,
    pub height_limit: extern "C" fn(u8),
    pub game_mode: extern "C" fn() -> crate::openttd::GameMode,
    pub seed: extern "C" fn() -> u32,
    pub set_seed: extern "C" fn(u32),
    pub interactive_random: extern "C" fn() -> u32,
    pub random_seed: extern "C" fn(u32),
    pub load_sprites: extern "C" fn(*mut Frame) -> bool,
    pub string_widths: extern "C" fn(),
    pub reset_windows: extern "C" fn(),
    pub setup_windows: extern "C" fn(),
    pub unshow_error: extern "C" fn(),
    pub close_windows: extern "C" fn(),
    pub hide_windows: extern "C" fn(),
    pub show_progress: extern "C" fn(),
    pub show_windows: extern "C" fn(),
    pub reset_object: extern "C" fn(),
    pub show_grf_error: extern "C" fn(),
    pub randomizers: extern "C" fn(),
    pub startup_companies: extern "C" fn(),
    pub place_cursor: extern "C" fn(),
    pub scroll: extern "C" fn(TileIndex),
    pub busy: extern "C" fn(bool),
    pub close_progress: extern "C" fn(),
    pub show_error: extern "C" fn(),
    pub dirty: extern "C" fn(),
    pub set_progress: extern "C" fn(*mut Frame, Progress, u32) -> bool,
    pub increase_progress: extern "C" fn(*mut Frame, Progress) -> bool,
    pub town_roads: extern "C" fn() -> bool,
    pub freeform: extern "C" fn() -> bool,
    pub count_land: extern "C" fn(),
    pub landscape_begin: extern "C" fn(*mut Frame) -> LandscapeCall,
    pub landscape_end: extern "C" fn(*mut Frame) -> bool,
    pub town_begin: extern "C" fn(*mut Frame) -> TownCall,
    pub town_end: extern "C" fn(*mut Frame) -> bool,
    pub industry_builder: extern "C" fn() -> *mut crate::industry::Builder,
    pub industry_begin: extern "C" fn(*mut Frame) -> *mut crate::industry::GenerationContext,
    pub industry_end: extern "C" fn(*mut Frame),
    pub object_begin: extern "C" fn(*mut Frame) -> ObjectCall,
    pub object_end: extern "C" fn(*mut Frame),
    pub tree_settings: extern "C" fn() -> crate::trees::GenerateSettings,
    pub tree_end: extern "C" fn(*mut Frame),
    pub loop_begin: extern "C" fn(*mut Frame) -> *const landscape_generation::Loop,
    pub loop_end: extern "C" fn(*mut Frame),
    pub advance_tick: extern "C" fn(),
    pub script_start: extern "C" fn(*mut Frame) -> bool,
    pub script_loop: extern "C" fn(*mut Frame) -> bool,
    pub script_exists: extern "C" fn() -> bool,
    pub script_sleeping: extern "C" fn() -> bool,
    pub trash_company: extern "C" fn(*mut Frame),
    pub company_valid: extern "C" fn(*mut Frame) -> bool,
    pub restore_company: extern "C" fn(*mut Frame),
    pub callback: extern "C" fn(*mut Frame, Callback) -> bool,
    pub exception_kind: extern "C" fn(*mut Frame) -> u8,
    pub storage_discard: extern "C" fn(*mut std::ffi::c_void),
    pub dedicated: extern "C" fn() -> bool,
    pub exit_game: extern "C" fn() -> bool,
    pub set_exit: extern "C" fn(),
    pub abort_mode: extern "C" fn(SwitchMode),
    pub switch_mode: extern "C" fn(*mut Frame) -> bool,
    pub debug_start: extern "C" fn(),
    pub debug_done: extern "C" fn(),
    pub debug_abort: extern "C" fn(),
    pub debug_map: extern "C" fn(u32),
    pub desync: extern "C" fn() -> bool,
    pub save: extern "C" fn(*mut Frame) -> bool,
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_generating_world() -> *mut bool {
    unsafe { std::ptr::addr_of_mut!((*INFO.get()).generating) }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_world_set_callback(callback: Callback) {
    write_field!(INFO.get(), done, callback);
}
#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn openttd_rust_world_set_abort_callback(callback: Callback) {
    write_field!(INFO.get(), abort_callback, callback);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_world_request_abort() {
    write_field!(INFO.get(), abort, true);
}
/// # Safety
/// The immutable native service table is live; serialized game-state entry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_world_aborted(s: &WorldServices) -> bool {
    read_field!(INFO.get(), abort) || (s.exit_game)()
}
/// # Safety
/// The native exception frame is live; callbacks catch before returning to Rust.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_world_handle_abort(
    s: &WorldServices,
    frame: *mut Frame,
) -> bool {
    (s.abort_mode)(if (s.game_mode)() == GameMode::EDITOR {
        SwitchMode::EDITOR
    } else {
        SwitchMode::MENU
    });
    if let Some(callback) = read_field!(INFO.get(), abort_callback) {
        return (s.callback)(frame, Some(callback));
    }
    true
}
/// # Safety
/// Live call-local native frame and immutable services. Native exceptions return
/// through the frame; no owner borrow spans initialization, resources or progress.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_world_setup(
    s: &WorldServices,
    frame: *mut Frame,
    mode: Mode,
    size_x: u32,
    size_y: u32,
    reset_settings: bool,
) -> bool {
    if (s.modal)() {
        return false;
    }
    write_field!(INFO.get(), mode, mode);
    write_field!(INFO.get(), size_x, size_x);
    write_field!(INFO.get(), size_y, size_y);
    (s.set_modal)(true);
    write_field!(INFO.get(), abort, false);
    write_field!(INFO.get(), abort_callback, None);
    write_field!(INFO.get(), local_company, (s.local_company)());
    (s.set_local_company)(CompanyID::SPECTATOR);
    if !(s.initialize)(
        frame,
        read_field!(INFO.get(), size_x),
        read_field!(INFO.get(), size_y),
        reset_settings,
    ) {
        return false;
    }
    (s.prepare_progress)();
    let g = (s.settings)();
    if g.height_limit == 0 {
        let mode = read_field!(INFO.get(), mode);
        let height = if mode == GWM_EMPTY && (s.game_mode)() != GameMode::MENU {
            u32::from(g.flat_height)
        } else if mode.0 == GWM_HEIGHTMAP {
            u32::from(g.heightmap_height)
        } else if g.generator == LG_TERRAGENESIS {
            let input = unsafe { ((*s.landscape).tgp_settings)() };
            unsafe { crate::tgp::openttd_rust_tgp_estimate(&raw const input) }
        } else {
            0
        };
        (s.height_limit)(
            height
                .wrapping_add(MAP_HEIGHT_LIMIT_AUTO_CEILING_ROOM)
                .clamp(MAP_HEIGHT_LIMIT_AUTO_MINIMUM, MAX_MAP_HEIGHT_LIMIT) as u8,
        );
    }
    if (s.seed)() == GENERATE_NEW_SEED {
        (s.set_seed)((s.interactive_random)());
    }
    if !(s.load_sprites)(frame) {
        return false;
    }
    crate::type_registry::openttd_rust_rail_saved_current();
    crate::type_registry::openttd_rust_road_saved_current();
    let town = unsafe { &*s.town };
    let growth = unsafe { &*town.growth };
    unsafe {
        crate::town::resolver::openttd_rust_house_counts_init(
            growth,
            &*town.shared,
            std::ptr::null_mut(),
            (growth.house_count)() as usize,
        );
    }
    (s.string_widths)();
    (s.reset_windows)();
    (s.setup_windows)();
    (s.place_cursor)();
    (s.unshow_error)();
    (s.close_windows)();
    (s.hide_windows)();
    (s.show_progress)();
    let a = unsafe { &*s.landscape };
    let shared = unsafe { &*a.shared };
    let map = shared.map();
    let size_y = (shared.map_max_y)() + 1;
    (s.scroll)(TileIndex(map.tile_xy(map.size_x() / 2, size_y / 2)));
    true
}
#[derive(Clone, Copy)]
enum Failure {
    Abort,
    Native,
}
fn native(success: bool) -> Result<(), Failure> {
    if success {
        Ok(())
    } else {
        Err(Failure::Native)
    }
}
struct World<'a> {
    s: &'a WorldServices,
    frame: *mut Frame,
}
impl World<'_> {
    fn set_progress(&self, stage: Progress, total: u32) -> Result<(), Failure> {
        native((self.s.set_progress)(self.frame, stage, total))
    }
    fn increase(&self, stage: Progress) -> Result<(), Failure> {
        native((self.s.increase_progress)(self.frame, stage))
    }
    fn cleanup(&self) {
        write_field!(INFO.get(), generating, false);
        (self.s.busy)(false);
        (self.s.set_modal)(false);
        write_field!(INFO.get(), done, None);
        write_field!(INFO.get(), abort_callback, None);
        (self.s.close_progress)();
        (self.s.show_error)();
        (self.s.dirty)();
    }
    fn abort(&self) -> Result<(), Failure> {
        native(unsafe { openttd_rust_world_handle_abort(self.s, self.frame) })?;
        Err(Failure::Abort)
    }
    fn storage_mode(&self, mode: u8, ignore: bool) {
        let landscape = unsafe { &*self.s.landscape };
        unsafe {
            crate::newgrf_storage::openttd_rust_storage_switch_mode(
                mode,
                ignore,
                landscape.assert_condition,
                self.s.storage_discard,
            );
        }
    }
    fn clear_tiles(&self) -> Result<(), Failure> {
        let a = unsafe { &*self.s.landscape };
        let shared = unsafe { &*a.shared };
        let clear = unsafe { &*a.clear };
        let map = shared.map();
        let size_y = (shared.map_max_y)() + 1;
        let mut count = map.scale_by_size(size_y, gb(shared.random(), 0, 10) + 0x400);
        let rocky = map.scale_by_size(size_y, gb(shared.random(), 0, 7) + 0x80);
        self.set_progress(GWP_ROUGH_ROCKY, rocky.wrapping_add(count))?;
        loop {
            self.increase(GWP_ROUGH_ROCKY)?;
            let tile = map.random_tile_seed(size_y, shared.random());
            if (shared.tile_type)(tile) == MP_CLEAR && (shared.clear_ground)(tile).0 != CLEAR_DESERT
            {
                (clear.set_ground)(tile, ClearGroundType(CLEAR_ROUGH), 3);
            }
            count = count.wrapping_sub(1);
            if count == 0 {
                break;
            }
        }
        count = rocky;
        loop {
            let r = shared.random();
            let mut tile = map.random_tile_seed(size_y, r);
            self.increase(GWP_ROUGH_ROCKY)?;
            if (shared.tile_type)(tile) == MP_CLEAR && (shared.clear_ground)(tile).0 != CLEAR_DESERT
            {
                let mut remaining = gb(r, 16, 4) + 5;
                'patch: loop {
                    (clear.set_ground)(tile, ClearGroundType(CLEAR_ROCKS), 3);
                    (shared.mark_dirty)(tile);
                    loop {
                        remaining = remaining.wrapping_sub(1);
                        if remaining == 0 {
                            break 'patch;
                        }
                        let next = map.add_diag(tile, gb(shared.random(), 0, 2) as u8);
                        if (shared.tile_type)(next) == MP_CLEAR
                            && (shared.clear_ground)(next).0 != CLEAR_DESERT
                        {
                            tile = next;
                            break;
                        }
                    }
                }
            }
            count = count.wrapping_sub(1);
            if count == 0 {
                break;
            }
        }
        Ok(())
    }
    fn landscape(&self) -> Result<bool, Failure> {
        let s = self.s;
        let call = (s.landscape_begin)(self.frame);
        let mode = read_field!(INFO.get(), mode).0;
        let generated = unsafe {
            landscape_generation::openttd_rust_generate_landscape(
                s.landscape,
                call.frame,
                call.decoder,
                call.settings,
                mode,
            )
        };
        native((s.landscape_end)(self.frame))?;
        Ok(generated)
    }
    fn populate(&self) -> Result<(), Failure> {
        let s = self.s;
        let call = (s.town_begin)(self.frame);
        let mut result = false;
        let completed = unsafe {
            crate::town::founding::openttd_rust_town_generate(
                s.town,
                call.road,
                call.frame,
                call.names,
                call.layout,
                false,
                0,
                &raw mut result,
            )
        };
        (s.town_end)(self.frame);
        native(completed)?;
        if !result {
            return self.abort();
        }
        let context = (s.industry_begin)(self.frame);
        let completed = unsafe {
            crate::industry::openttd_rust_industry_generate(
                (s.industry_builder)(),
                context,
                s.industry,
                s.industry_generation,
            )
        };
        (s.industry_end)(self.frame);
        native(completed)?;
        let call = (s.object_begin)(self.frame);
        let completed = unsafe {
            crate::objects::openttd_rust_object_generate(
                s.objects, call.out, call.tmp, call.aux, call.frame,
            )
        };
        (s.object_end)(self.frame);
        native(completed)?;
        let a = unsafe { &*s.landscape };
        let aborted = unsafe {
            crate::trees::openttd_rust_trees_generate((s.tree_settings)(), a.trees, a.shared)
        };
        if aborted {
            (s.tree_end)(self.frame);
            return Err(Failure::Native);
        }
        Ok(())
    }
    fn tile_loop(&self) {
        let s = self.s;
        let call = unsafe { &*(s.loop_begin)(self.frame) };
        let shared = unsafe { &*(*s.landscape).shared };
        unsafe {
            crate::tile_loop::openttd_rust_run_tile_loop(
                (shared.map_log_x)(),
                ((shared.map_max_y)() + 1).ilog2(),
                call.settings,
                call.services,
            );
        }
        (s.loop_end)(self.frame);
    }
    #[allow(clippy::if_not_else)]
    fn generate(&self) -> Result<(), Failure> {
        let s = self.s;
        write_field!(INFO.get(), generating, true);
        if (s.dedicated)() {
            (s.debug_start)();
        }
        (s.random_seed)((s.seed)());
        self.set_progress(GWP_MAP_INIT, 2)?;
        (s.place_cursor)();
        (s.randomizers)();
        self.storage_mode(crate::newgrf_storage::PSM_ENTER_GAMELOOP, false);
        self.increase(GWP_MAP_INIT)?;
        let company = unsafe { &*s.company };
        crate::company::openttd_rust_economy_startup(
            unsafe { &*company.services },
            &(company.settings)(),
        );
        if !(s.town_roads)() {
            return self.abort();
        }
        let generated = read_field!(INFO.get(), mode) != GWM_EMPTY && self.landscape()?;
        if !generated {
            self.set_progress(GWP_OBJECT, 1)?;
            let a = unsafe { &*s.landscape };
            let shared = unsafe { &*a.shared };
            if (s.freeform)() {
                let map = shared.map();
                for x in 0..map.size_x() {
                    (a.make_void)(map.tile_xy(x, 0));
                }
                for y in 0..=(shared.map_max_y)() {
                    (a.make_void)(map.tile_xy(0, y));
                }
            }
            if (s.game_mode)() != GameMode::MENU {
                let g = (s.settings)();
                unsafe {
                    landscape_generation::openttd_rust_flat_empty_world(
                        s.landscape,
                        g.flat_height,
                        (s.freeform)(),
                        g.height_limit,
                    );
                }
            }
            unsafe {
                landscape_generation::openttd_rust_convert_ground_water(s.landscape);
            }
            (s.count_land)();
            self.increase(GWP_OBJECT)?;
            (a.set_snow_line)(DEF_SNOWLINE_HEIGHT);
        } else {
            self.clear_tiles()?;
            (s.count_land)();
            if (s.game_mode)() != GameMode::EDITOR {
                self.populate()?;
            }
        }
        self.set_progress(GWP_GAME_INIT, 3)?;
        (s.startup_companies)();
        self.increase(GWP_GAME_INIT)?;
        unsafe {
            crate::engine::openttd_rust_engine_start_all(s.engines);
        }
        self.increase(GWP_GAME_INIT)?;
        unsafe {
            crate::disaster::openttd_rust_disaster_startup(s.disasters);
        }
        write_field!(INFO.get(), generating, false);
        native((s.script_start)(self.frame))?;
        if read_field!(INFO.get(), mode) != GWM_EMPTY {
            self.set_progress(GWP_RUNTILELOOP, 0x500)?;
            for _ in 0..0x500 {
                self.tile_loop();
                (s.advance_tick)();
                self.increase(GWP_RUNTILELOOP)?;
            }
            if (s.game_mode)() != GameMode::EDITOR && (s.script_exists)() {
                self.set_progress(GWP_RUNSCRIPT, 2500)?;
                write_field!(INFO.get(), generating, true);
                for _ in 0..2500 {
                    native((s.script_loop)(self.frame))?;
                    self.increase(GWP_RUNSCRIPT)?;
                    if (s.script_sleeping)() {
                        break;
                    }
                }
                write_field!(INFO.get(), generating, false);
            }
        }
        self.storage_mode(crate::newgrf_storage::PSM_LEAVE_GAMELOOP, false);
        (s.reset_object)();
        (s.trash_company)(self.frame);
        (s.restore_companies)(read_field!(INFO.get(), local_company));
        if (s.game_mode)() != GameMode::MENU {
            (s.show_windows)();
        }
        self.set_progress(GWP_GAME_START, 1)?;
        if let Some(callback) = read_field!(INFO.get(), done) {
            native((s.callback)(self.frame, Some(callback)))?;
        }
        self.increase(GWP_GAME_START)?;
        self.cleanup();
        (s.show_grf_error)();
        if (s.dedicated)() {
            (s.debug_done)();
        }
        (s.debug_map)((s.seed)());
        if (s.desync)() {
            native((s.save)(self.frame))?;
        }
        Ok(())
    }
}
/// # Safety
/// The native Backup<CompanyID> and exception/resource frames live through the
/// synchronous call. Named throwing services catch inside those frames; Rust
/// panics abort. No native container or owner borrow spans reentry or yielding.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_world_generate(s: &WorldServices, frame: *mut Frame) -> u8 {
    let world = World { s, frame };
    match world.generate() {
        Ok(()) => 0,
        Err(Failure::Native) if (s.exception_kind)(frame) != 1 => 2,
        Err(_) => {
            world.cleanup();
            world.storage_mode(crate::newgrf_storage::PSM_LEAVE_GAMELOOP, true);
            if (s.company_valid)(frame) {
                (s.restore_company)(frame);
            }
            if (s.dedicated)() {
                (s.debug_abort)();
                (s.set_exit)();
            } else if !(s.switch_mode)(frame) {
                return 2;
            }
            1
        }
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDWorldSettings" ["genworld"] => Settings { height_limit, flat_height, heightmap_height, generator, };
    "OpenTTDWorldLandscape" ["genworld"] => LandscapeCall { frame, decoder, settings, };
    "OpenTTDWorldTown" ["genworld"] => TownCall { frame, names, road, layout, };
    "OpenTTDWorldObject" ["genworld"] => ObjectCall { frame, out, tmp, aux, };
    "OpenTTDWorldServices" ["genworld"] => WorldServices {
        landscape,
        company,
        town,
        industry,
        industry_generation,
        objects,
        engines,
        disasters,
        modal,
        set_modal,
        local_company,
        set_local_company,
        restore_companies,
        initialize,
        prepare_progress,
        settings,
        height_limit,
        game_mode,
        seed,
        set_seed,
        interactive_random,
        random_seed,
        load_sprites,
        string_widths,
        reset_windows,
        setup_windows,
        unshow_error,
        close_windows,
        hide_windows,
        show_progress,
        show_windows,
        reset_object,
        show_grf_error,
        randomizers,
        startup_companies,
        place_cursor,
        scroll,
        busy,
        close_progress,
        show_error,
        dirty,
        set_progress,
        increase_progress,
        town_roads,
        freeform,
        count_land,
        landscape_begin,
        landscape_end,
        town_begin,
        town_end,
        industry_builder,
        industry_begin,
        industry_end,
        object_begin,
        object_end,
        tree_settings,
        tree_end,
        loop_begin,
        loop_end,
        advance_tick,
        script_start,
        script_loop,
        script_exists,
        script_sleeping,
        trash_company,
        company_valid,
        restore_company,
        callback,
        exception_kind,
        storage_discard,
        dedicated,
        exit_game,
        set_exit,
        abort_mode,
        switch_mode,
        debug_start,
        debug_done,
        debug_abort,
        debug_map,
        desync,
        save,
    };
}
