/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Complete simulation-session lifecycle. Native resources and genuine exception frames stay native.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::too_many_lines,
    clippy::if_not_else,
    clippy::verbose_bit_mask
)]
use crate::company_type::{CompanyID, MAX_COMPANIES};
use crate::game_state::GameState;
use crate::openttd::{GameMode, PauseMode, SwitchMode};
use crate::raw_field::{read_field, write_field};
#[repr(C)]
pub struct Frame {
    _opaque: [u8; 0],
}
/// Uninterpreted native stack capacity, checked for size/alignment by C++.
#[repr(C, align(16))]
pub struct FrameStorage {
    pub storage: [std::mem::MaybeUninit<u8>; 4096],
}
/// Native world-generation frame capacity, including inline heightmap codec scratch.
#[repr(C, align(16))]
pub struct WorldFrameStorage {
    pub storage: [std::mem::MaybeUninit<u8>; 8192],
}
#[repr(C)]
pub struct Load {
    _opaque: [u8; 0],
}
#[repr(C)]
pub struct ScanCallback {
    _opaque: [u8; 0],
}
use crate::fileio_type::{AbstractFileType as FileKind, FT_HEIGHTMAP, FT_SAVEGAME, FT_SCENARIO};
use crate::gfx_type::COLOUR_END;
use crate::saveload::{SL_OK, SL_REINIT, SaveOrLoadResult as LoadResult};
#[repr(C)]
pub struct Dimensions {
    pub x: u32,
    pub y: u32,
}
#[repr(C)]
pub struct SecondaryColour {
    pub colour: u8,
    pub used: bool,
}
#[repr(C)]
pub struct Services {
    pub clock: *const crate::timer_game_common::ClockServices,
    pub company: *const crate::company::Services,
    pub shared: *const crate::services::Services,
    pub schedule: *const crate::linkgraph_schedule::Services,
    pub world: extern "C" fn() -> *const crate::genworld::WorldServices,
    pub networking: extern "C" fn() -> bool,
    pub network_server: extern "C" fn() -> bool,
    pub dedicated: extern "C" fn() -> bool,
    pub server_requested: extern "C" fn() -> bool,
    pub clear_server_requested: extern "C" fn(),
    pub network_available: extern "C" fn() -> bool,
    pub exit_game: extern "C" fn() -> bool,
    pub set_exit: extern "C" fn(),
    pub modal: extern "C" fn() -> bool,
    pub network_reboot: extern "C" fn(),
    pub network_disconnect: extern "C" fn(),
    pub network_server_start: extern "C" fn(*mut Frame) -> bool,
    pub network_background: extern "C" fn(),
    pub network_start_game: extern "C" fn(),
    pub network_join: extern "C" fn(),
    pub kill_ai: extern "C" fn(),
    pub load_config: extern "C" fn(),
    pub reset_session_stats: extern "C" fn(),
    pub reset_file_mode: extern "C" fn(),
    pub generate_id: extern "C" fn(),
    pub reset_windows: extern "C" fn(),
    pub setup_windows: extern "C" fn(),
    pub fix_title_zoom: extern "C" fn(),
    pub release_cursor: extern "C" fn(),
    pub check_glyphs: extern "C" fn(),
    pub music: extern "C" fn(),
    pub rail_gui: extern "C" fn(),
    pub road_gui: extern "C" fn(),
    pub check_engines: extern "C" fn(),
    pub check_industries: extern "C" fn(),
    pub close_saveload: extern "C" fn(),
    pub joining_social: extern "C" fn(),
    pub ask_survey: extern "C" fn(),
    pub process_async_save: extern "C" fn(),
    pub sprite_lru: extern "C" fn(),
    pub remote_messages: extern "C" fn(),
    pub palette: extern "C" fn(),
    pub sound: extern "C" fn(),
    pub social_callbacks: extern "C" fn(),
    pub news: extern "C" fn(),
    pub window_tick: extern "C" fn(),
    pub check_caches: extern "C" fn(),
    pub reset_engine_overrides: extern "C" fn(),
    pub dirty: extern "C" fn(),
    pub reload_config: extern "C" fn() -> bool,
    pub reset_grf: extern "C" fn(bool),
    pub autosave_frequency: extern "C" fn(bool),
    pub leave_survey: extern "C" fn(),
    pub file_kind: extern "C" fn() -> FileKind,
    pub map_dimensions: extern "C" fn() -> Dimensions,
    pub has_gui: extern "C" fn() -> bool,
    pub pause_on_newgame: extern "C" fn() -> bool,
    pub starting_colour: extern "C" fn() -> u8,
    pub secondary_colour: extern "C" fn() -> SecondaryColour,
    pub disable_elrails: extern "C" fn() -> bool,
    pub settings_elrail: extern "C" fn(bool),
    pub company_settings: extern "C" fn(),
    pub company_colour: extern "C" fn(bool, u8),
    pub post_pause: extern "C" fn(PauseMode, bool),
    pub reset_ai_config: extern "C" fn(CompanyID),
    pub reset_game_config: extern "C" fn(),
    pub publish_settings: extern "C" fn(),
    pub publish_old_settings: extern "C" fn(),
    pub newgame_year: extern "C" fn(i32),
    pub fallback_sound_warning: extern "C" fn() -> bool,
    pub show_sound_warning: extern "C" fn(),
    pub survey_ask: extern "C" fn() -> bool,
    pub show_load_error: extern "C" fn(bool),
    pub social_mode: extern "C" fn(GameMode),
    pub load_failed_debug: extern "C" fn(bool),
    pub pause_query: extern "C" fn(),
    pub pause_change: extern "C" fn(u8, PauseMode),
    pub pause_dirty: extern "C" fn(),
    pub full_animation: extern "C" fn() -> bool,
    pub reconnect_due: extern "C" fn() -> bool,
    pub console_script: extern "C" fn(*mut Frame) -> bool,
    pub game_script: extern "C" fn(*mut Frame) -> bool,
    pub ai_script: extern "C" fn(*mut Frame) -> bool,
    pub startup_company: extern "C" fn(*mut Frame) -> bool,
    pub network_loop: extern "C" fn(*mut Frame) -> bool,
    pub network_reconnect: extern "C" fn(*mut Frame) -> bool,
    pub state_frame_begin: extern "C" fn(*mut FrameStorage) -> *mut Frame,
    pub state_frame_end: extern "C" fn(*mut Frame, *mut Frame) -> bool,
    pub real_time: extern "C" fn(*mut Frame) -> bool,
    pub scan: extern "C" fn(*mut Frame, *mut ScanCallback) -> bool,
    pub world_setup_begin: extern "C" fn(*mut Frame) -> *mut crate::genworld::SetupFrame,
    pub world_setup_end: extern "C" fn(*mut Frame) -> bool,
    pub world_frame_begin: extern "C" fn(*mut WorldFrameStorage) -> *mut crate::genworld::Frame,
    pub world_frame_end: extern "C" fn(*mut Frame, *mut crate::genworld::Frame, u8) -> bool,
    pub heightmap_screenshot: extern "C" fn(*mut Frame) -> bool,
    pub desync_save: extern "C" fn(*mut Frame) -> bool,
    pub load: extern "C" fn(*mut Frame, *mut Load) -> LoadResult,
    pub intro_load: extern "C" fn(*mut Frame) -> LoadResult,
    pub save: extern "C" fn(*mut Frame) -> LoadResult,
    pub failed: extern "C" fn(*mut Frame) -> bool,
    pub console_exception:
        extern "C" fn(*mut Frame) -> *mut crate::timer_game_common::ConsoleException,
    pub state_begin: extern "C" fn(*mut Frame),
    pub company_begin: extern "C" fn(*mut Frame),
    pub company_restore: extern "C" fn(*mut Frame),
    pub scripts_begin: extern "C" fn(*mut Frame),
    pub scripts_end: extern "C" fn(*mut Frame),
    pub paused_measurements: extern "C" fn(),
    pub scripts_enabled: extern "C" fn() -> bool,
    pub assert_local_company: extern "C" fn(),
    pub desync_level: extern "C" fn() -> i32,
    pub map_log_y: extern "C" fn() -> u32,
    pub tick_timer: extern "C" fn(*mut Frame) -> bool,
    pub tile_loop_begin: extern "C" fn(*mut Frame) -> *const crate::landscape_generation::Loop,
    pub tile_loop_end: extern "C" fn(*mut Frame),
    pub vehicle_tables: extern "C" fn(*mut Frame, bool) -> crate::vehicle::scheduler::Tables,
    pub vehicle_cost: extern "C" fn(*mut Frame) -> *mut std::ffi::c_void,
    pub vehicle_end: extern "C" fn(*mut Frame),
    pub landscaping: extern "C" fn() -> crate::company::Landscaping,
    pub schedule_time: extern "C" fn() -> crate::linkgraph_schedule::Time,
    pub town: *const crate::town::Leaves,
    pub trees: *const crate::trees::TreeServices,
    pub station: *const crate::station_service::World,
    pub links: *const crate::station_service::Links,
    pub visits: *const crate::vehicle::visits::Services,
    pub tree_tick: extern "C" fn() -> crate::trees::TickSettings,
    pub industry_tick: extern "C" fn() -> crate::industry::TickRecord,
    pub industry_builder: extern "C" fn() -> *mut crate::industry::Builder,
    pub company_tick: extern "C" fn() -> crate::company::Tick,
    pub post_ai: extern "C" fn(*mut Frame) -> bool,
    pub road_begin: extern "C" fn(*mut Frame) -> *const crate::road_cmd::CommandContext,
    pub road_end: extern "C" fn(*mut Frame),
    pub landscape_begin: extern "C" fn(*mut Frame),
    pub landscape_end: extern "C" fn(*mut Frame),
    pub schedule_measurement: extern "C" fn(*mut Frame) -> *mut std::ffi::c_void,
    pub schedule_end: extern "C" fn(*mut Frame),
}

/// Canonical native aliases occupy only these three one-byte subobjects.
#[repr(C)]
pub struct State {
    pub mode: GameMode,
    pub switch_mode: SwitchMode,
    pub pause: u8,
}
static STATE: GameState<State> = GameState::new(State {
    mode: GameMode::MENU,
    switch_mode: SwitchMode::NONE,
    pause: 0,
});
static SCAN: GameState<bool> = GameState::new(false);
static SCAN_CALLBACK: GameState<*mut ScanCallback> = GameState::new(std::ptr::null_mut());
static ASKED_SURVEY: GameState<bool> = GameState::new(false);
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_session_storage() -> *mut State {
    STATE.get()
}
pub(crate) fn mode() -> GameMode {
    read_field!(STATE.get(), mode)
}
fn set_mode(mode: GameMode) {
    write_field!(STATE.get(), mode, mode);
}
pub(crate) fn pending() -> SwitchMode {
    read_field!(STATE.get(), switch_mode)
}
pub(crate) fn set_pending(mode: SwitchMode) {
    write_field!(STATE.get(), switch_mode, mode);
}
pub(crate) fn pause() -> u8 {
    read_field!(STATE.get(), pause)
}
pub(crate) fn set_pause(value: u8) {
    write_field!(STATE.get(), pause, value);
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_session_scan(callback: *mut ScanCallback) -> bool {
    if unsafe { *SCAN.get() } {
        return false;
    }
    unsafe {
        *SCAN.get() = true;
        *SCAN_CALLBACK.get() = callback;
    }
    true
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_session_pause(
    s: &Services,
    mode: PauseMode,
    paused: bool,
    execute: bool,
) -> bool {
    match mode {
        PauseMode::SAVE_LOAD
        | PauseMode::ERROR
        | PauseMode::NORMAL
        | PauseMode::GAME_SCRIPT
        | PauseMode::LINK_GRAPH => {}
        PauseMode::JOIN | PauseMode::ACTIVE_CLIENTS if (s.networking)() => {}
        _ => return false,
    }
    if execute {
        if mode == PauseMode::NORMAL && pause() & PauseMode::ERROR.bit() != 0 {
            (s.pause_query)();
        } else {
            let previous = pause();
            if paused {
                set_pause(previous | mode.bit());
            } else {
                let remaining = previous & !mode.bit();
                set_pause(if remaining == PauseMode::COMMAND_DURING_PAUSE.bit() {
                    0
                } else {
                    remaining
                });
            }
            (s.pause_change)(previous, mode);
        }
        (s.pause_dirty)();
    }
    true
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_session_settings(s: &Services) {
    for company in 0..MAX_COMPANIES {
        (s.reset_ai_config)(CompanyID(company));
    }
    (s.reset_game_config)();
    (s.publish_settings)();
    (s.publish_old_settings)();
}

type Outcome<T = ()> = Result<T, ()>;
fn complete(success: bool) -> Outcome {
    if success { Ok(()) } else { Err(()) }
}
struct Session<'a> {
    s: &'a Services,
    frame: *mut Frame,
}
impl Session<'_> {
    fn checked(&self) -> Outcome {
        complete(!(self.s.failed)(self.frame))
    }
    fn generate(&self, kind: crate::genworld::Mode, x: u32, y: u32, reset: bool) -> Outcome {
        let s = self.s;
        let setup = (s.world_setup_begin)(self.frame);
        let ready = unsafe { crate::genworld::setup(&*(s.world)(), setup, kind, x, y, reset) };
        complete((s.world_setup_end)(self.frame))?;
        if ready {
            complete(crate::genworld::generate(s, self.frame))?;
        }
        Ok(())
    }
    fn sized_world(&self, kind: crate::genworld::Mode, reset: bool) -> Outcome {
        let size = (self.s.map_dimensions)();
        self.generate(kind, size.x, size.y, reset)
    }
    fn intro(&self, newgrfs: bool) -> Outcome {
        let s = self.s;
        set_mode(GameMode::MENU);
        if newgrfs {
            (s.reset_grf)(false);
        }
        (s.reset_windows)();
        (s.setup_windows)();
        let result = (s.intro_load)(self.frame);
        self.checked()?;
        if result != SL_OK {
            self.generate(crate::genworld::GWM_EMPTY, 64, 64, true)?;
            crate::company::set_local_company(unsafe { &*s.company }, CompanyID::SPECTATOR);
        } else {
            crate::company::set_local_company(unsafe { &*s.company }, CompanyID::BEGIN);
        }
        (s.fix_title_zoom)();
        set_pause(0);
        (s.release_cursor)();
        (s.check_glyphs)();
        (s.music)();
        Ok(())
    }
    fn start_game(&self, dedicated: bool) -> Outcome {
        let s = self.s;
        crate::company::set_local_company(
            unsafe { &*s.company },
            if dedicated {
                CompanyID::SPECTATOR
            } else {
                crate::company::first_playable(unsafe { &*s.company })
            },
        );
        (s.network_start_game)();
        complete((s.console_script)(self.frame))
    }
    fn new_game_done(&self) -> Outcome {
        let s = self.s;
        (s.settings_elrail)((s.disable_elrails)());
        if !(s.has_gui)() {
            self.start_game(true)?;
            if (s.pause_on_newgame)() {
                (s.post_pause)(PauseMode::NORMAL, true);
            }
            return Ok(());
        }
        complete((s.startup_company)(self.frame))?;
        (s.company_settings)();
        // COLOUR_END selects the original random colour.
        let colour = (s.starting_colour)();
        if colour != COLOUR_END {
            (s.company_colour)(true, colour);
        }
        let secondary = (s.secondary_colour)();
        if secondary.colour != COLOUR_END && secondary.used {
            (s.company_colour)(false, secondary.colour);
        }
        self.start_game(false)?;
        (s.rail_gui)();
        (s.road_gui)();
        if (s.pause_on_newgame)() {
            (s.post_pause)(PauseMode::NORMAL, true);
        }
        (s.check_engines)();
        (s.check_industries)();
        (s.dirty)();
        Ok(())
    }
    fn new_game(&self, heightmap: bool, reset: bool) -> Outcome {
        set_mode(GameMode::NORMAL);
        if !heightmap {
            (self.s.reset_file_mode)();
        }
        (self.s.reset_grf)(true);
        crate::genworld::session_callback(false);
        self.sized_world(
            if heightmap {
                crate::genworld::Mode(crate::genworld::GWM_HEIGHTMAP)
            } else {
                crate::genworld::GWM_NEWGAME
            },
            reset,
        )
    }
    fn new_editor(&self) -> Outcome {
        set_mode(GameMode::EDITOR);
        (self.s.reset_file_mode)();
        (self.s.reset_grf)(true);
        crate::genworld::session_callback(true);
        self.sized_world(crate::genworld::GWM_EMPTY, true)
    }
    fn safe_load(&self, input: *mut Load, new_mode: GameMode) -> Outcome<bool> {
        let s = self.s;
        let old = mode();
        set_mode(new_mode);
        let result = (s.load)(self.frame, input);
        self.checked()?;
        if result == SL_OK {
            return Ok(true);
        }
        if (s.dedicated)() && old == GameMode::MENU {
            (s.load_failed_debug)(true);
            (s.set_exit)();
            return Ok(false);
        }
        if result != SL_REINIT {
            set_mode(old);
            return Ok(false);
        }
        if (s.dedicated)() {
            (s.load_failed_debug)(false);
            self.new_game(false, true)?;
            return Ok(false);
        }
        if (s.network_server)() {
            (s.network_disconnect)();
        }
        if old == GameMode::EDITOR {
            self.new_editor()?;
        } else {
            self.intro(true)?;
        }
        Ok(false)
    }
    fn switch(&self, new_mode: SwitchMode) -> Outcome {
        let s = self.s;
        if new_mode != SwitchMode::SAVE_GAME {
            if (s.networking)() {
                if (s.network_server)()
                    && matches!(
                        new_mode,
                        SwitchMode::LOAD_GAME | SwitchMode::NEWGAME | SwitchMode::RESTARTGAME
                    )
                {
                    (s.network_reboot)();
                } else {
                    (s.network_disconnect)();
                }
            }
            if (s.server_requested)() {
                if new_mode != SwitchMode::MENU {
                    if (s.reload_config)() {
                        (s.load_config)();
                        openttd_rust_session_settings(s);
                        (s.reset_grf)(false);
                    }
                    complete((s.network_server_start)(self.frame))?;
                } else {
                    (s.clear_server_requested)();
                }
            }
            (s.kill_ai)();
            (s.autosave_frequency)(true);
        }
        if mode() == GameMode::NORMAL && new_mode != SwitchMode::SAVE_GAME {
            (s.leave_survey)();
        }
        if new_mode != SwitchMode::SAVE_GAME {
            (s.reset_session_stats)();
        }
        match new_mode {
            SwitchMode::EDITOR => {
                self.new_editor()?;
                (s.generate_id)();
                (s.social_mode)(GameMode::EDITOR);
            }
            SwitchMode::RELOADGAME => {
                let file = (s.file_kind)();
                if file == FT_SAVEGAME || file == FT_SCENARIO {
                    set_pending(if mode() == GameMode::EDITOR {
                        SwitchMode::LOAD_SCENARIO
                    } else {
                        SwitchMode::LOAD_GAME
                    });
                    return self.switch(pending());
                }
                if file == FT_HEIGHTMAP {
                    set_pending(if mode() == GameMode::EDITOR {
                        SwitchMode::LOAD_HEIGHTMAP
                    } else {
                        SwitchMode::RESTART_HEIGHTMAP
                    });
                    return self.switch(pending());
                }
                self.new_game(false, false)?;
                (s.generate_id)();
                (s.social_mode)(GameMode::NORMAL);
            }
            SwitchMode::RESTARTGAME | SwitchMode::NEWGAME => {
                self.new_game(false, new_mode == SwitchMode::NEWGAME)?;
                (s.generate_id)();
                (s.social_mode)(GameMode::NORMAL);
            }
            SwitchMode::LOAD_GAME => {
                (s.reset_grf)(true);
                (s.reset_windows)();
                if self.safe_load(std::ptr::null_mut(), GameMode::NORMAL)? {
                    if (s.file_kind)() == FT_SCENARIO {
                        self.start_scenario();
                    }
                    self.start_game((s.dedicated)())?;
                    (s.post_pause)(PauseMode::SAVE_LOAD, false);
                } else {
                    (s.show_load_error)(true);
                }
                (s.social_mode)(GameMode::NORMAL);
            }
            SwitchMode::RESTART_HEIGHTMAP | SwitchMode::START_HEIGHTMAP => {
                self.new_game(true, new_mode == SwitchMode::START_HEIGHTMAP)?;
                (s.generate_id)();
                (s.social_mode)(GameMode::NORMAL);
            }
            SwitchMode::LOAD_HEIGHTMAP => {
                crate::company::set_local_company(unsafe { &*s.company }, CompanyID::NONE);
                set_mode(GameMode::EDITOR);
                self.sized_world(crate::genworld::Mode(crate::genworld::GWM_HEIGHTMAP), true)?;
                (s.generate_id)();
                (s.dirty)();
                (s.social_mode)(GameMode::EDITOR);
            }
            SwitchMode::LOAD_SCENARIO => {
                if self.safe_load(std::ptr::null_mut(), GameMode::EDITOR)? {
                    crate::company::set_local_company(unsafe { &*s.company }, CompanyID::NONE);
                    (s.generate_id)();
                    (s.newgame_year)(crate::timer_game_common::calendar_year().0);
                    (s.post_pause)(PauseMode::SAVE_LOAD, false);
                } else {
                    (s.show_load_error)(true);
                }
                (s.social_mode)(GameMode::EDITOR);
            }
            SwitchMode::JOIN_GAME => {
                self.intro(true)?;
                (s.network_join)();
                (s.joining_social)();
            }
            SwitchMode::MENU => {
                self.intro(true)?;
                if (s.fallback_sound_warning)() {
                    (s.show_sound_warning)();
                }
                if (s.survey_ask)() && !unsafe { *ASKED_SURVEY.get() } {
                    unsafe {
                        *ASKED_SURVEY.get() = true;
                    }
                    (s.ask_survey)();
                }
                (s.social_mode)(GameMode::MENU);
            }
            SwitchMode::SAVE_GAME => {
                let result = (s.save)(self.frame);
                self.checked()?;
                if result == SL_OK {
                    (s.close_saveload)();
                } else {
                    (s.show_load_error)(false);
                }
            }
            SwitchMode::SAVE_HEIGHTMAP => {
                complete((s.heightmap_screenshot)(self.frame))?;
                (s.close_saveload)();
            }
            SwitchMode::GENRANDLAND => {
                crate::company::set_local_company(unsafe { &*s.company }, CompanyID::NONE);
                self.sized_world(crate::genworld::GWM_RANDOM, true)?;
                (s.dirty)();
            }
            _ => (unsafe { &*s.shared }.not_reached)(),
        }
        Ok(())
    }
    fn start_scenario(&self) {
        (self.s.reset_engine_overrides)();
        let services = unsafe { &*(*(self.s.world)()).industry };
        let mut from = 0;
        loop {
            let entry = unsafe { (services.next_tick)(from) };
            if entry.owner.is_null() {
                break;
            }
            from = u32::from(entry.id) + 1;
            unsafe {
                (*crate::industry::openttd_rust_industry_fields(entry.owner)).last_prod_year =
                    crate::timer_game_common::economy_year().0;
            }
        }
    }
    fn landscaping(&self) {
        crate::company::update_landscaping_limits(
            unsafe { &*self.s.company },
            &(self.s.landscaping)(),
        );
    }
    fn storage(&self, mode: u8) {
        let world = unsafe { &*(self.s.world)() };
        unsafe {
            crate::newgrf_storage::openttd_rust_storage_switch_mode(
                mode,
                false,
                (*world.landscape).assert_condition,
                world.storage_discard,
            );
        }
    }
    fn tile_loop(&self) {
        let s = self.s;
        let call = unsafe { &*(s.tile_loop_begin)(self.frame) };
        let shared = unsafe { &*s.shared };
        unsafe {
            crate::tile_loop::run(
                u32::from(shared.map().log_x),
                (s.map_log_y)(),
                call.settings,
                call.services,
            );
        }
        (s.tile_loop_end)(self.frame);
    }
    fn vehicles(&self, calendar: bool) {
        let s = self.s;
        let tables = (s.vehicle_tables)(self.frame, !calendar);
        unsafe {
            if calendar {
                crate::vehicle::scheduler::calendar_day(
                    &raw const tables,
                    mode() == GameMode::NORMAL,
                    usize::from(crate::timer_game_common::calendar_fraction()),
                );
            } else {
                crate::vehicle::scheduler::ticks(
                    &raw const tables,
                    mode() == GameMode::NORMAL,
                    usize::from(crate::timer_game_common::economy_fraction()),
                    (s.vehicle_cost)(self.frame).cast(),
                );
            }
        }
        if !calendar {
            (s.vehicle_end)(self.frame);
        }
    }
    fn landscape_tick(&self) -> Outcome {
        let s = self.s;
        (s.landscape_begin)(self.frame);
        let road = (s.road_begin)(self.frame);
        unsafe {
            crate::town::tick_all(s.town, s.shared, road);
        }
        (s.road_end)(self.frame);
        unsafe {
            crate::trees::tick((s.tree_tick)(), s.trees, s.shared);
            crate::station_service::tick(s.station, s.links, s.visits);
            let world = &*(s.world)();
            crate::industry::tick((s.industry_builder)(), &(s.industry_tick)(), world.industry);
        }
        (s.landscape_end)(self.frame);
        let mut company = (s.company_tick)();
        while unsafe { crate::company::on_tick(&*s.company, &mut company) } {
            complete((s.post_ai)(self.frame))?;
        }
        crate::linkgraph_schedule::tick(
            unsafe { &*s.schedule },
            (s.schedule_time)(),
            (s.networking)() && !(s.network_server)(),
            (s.schedule_measurement)(self.frame),
        );
        (s.schedule_end)(self.frame);
        Ok(())
    }
    fn state_loop(&self) -> Outcome {
        let s = self.s;
        if !(s.networking)() || (s.network_server)() {
            unsafe {
                crate::linkgraph_schedule::pause(
                    &*s.schedule,
                    (s.schedule_time)(),
                    pause() & PauseMode::LINK_GRAPH.bit() != 0,
                    pause() == 0,
                );
            }
        }
        if pause() != 0 || (s.modal)() {
            (s.paused_measurements)();
            if !(s.modal)() {
                self.landscaping();
            }
            if (s.scripts_enabled)() && mode() == GameMode::NORMAL {
                complete((s.game_script)(self.frame))?;
            }
            return Ok(());
        }
        (s.state_begin)(self.frame);
        if mode() == GameMode::EDITOR {
            self.storage(crate::newgrf_storage::PSM_ENTER_GAMELOOP);
            self.tile_loop();
            self.vehicles(false);
            self.landscape_tick()?;
            self.storage(crate::newgrf_storage::PSM_LEAVE_GAMELOOP);
            self.landscaping();
            (s.window_tick)();
            (s.news)();
        } else {
            if (s.desync_level)() > 2
                && crate::timer_game_common::economy_fraction() == 0
                && crate::timer_game_common::economy_date().0 & 0x1f == 0
            {
                complete((s.desync_save)(self.frame))?;
            }
            (s.check_caches)();
            (s.company_begin)(self.frame);
            self.storage(crate::newgrf_storage::PSM_ENTER_GAMELOOP);
            (s.landscape_begin)(self.frame);
            crate::animated_tile::run();
            (s.landscape_end)(self.frame);
            let exception = (s.console_exception)(self.frame);
            let advanced = unsafe { crate::timer_game_calendar::elapsed(&*s.clock, exception) };
            self.checked()?;
            if advanced {
                self.vehicles(true);
            }
            unsafe {
                crate::timer_game_economy::elapsed(&*s.clock, exception);
            }
            self.checked()?;
            complete((s.tick_timer)(self.frame))?;
            self.tile_loop();
            self.vehicles(false);
            self.landscape_tick()?;
            self.storage(crate::newgrf_storage::PSM_LEAVE_GAMELOOP);
            if (s.scripts_enabled)() {
                (s.scripts_begin)(self.frame);
                complete((s.ai_script)(self.frame))?;
                complete((s.game_script)(self.frame))?;
                (s.scripts_end)(self.frame);
            }
            self.landscaping();
            (s.window_tick)();
            (s.news)();
            (s.company_restore)(self.frame);
        }
        (s.assert_local_company)();
        Ok(())
    }
    fn game_loop(&self) -> Outcome {
        let s = self.s;
        if mode() == GameMode::BOOTSTRAP {
            if (s.network_available)() {
                (s.network_background)();
            }
            return Ok(());
        }
        if unsafe { *SCAN.get() } {
            complete((s.scan)(self.frame, unsafe { *SCAN_CALLBACK.get() }))?;
            unsafe {
                *SCAN.get() = false;
                *SCAN_CALLBACK.get() = std::ptr::null_mut();
            }
            if (s.exit_game)() {
                return Ok(());
            }
        }
        (s.process_async_save)();
        if mode() == GameMode::NORMAL {
            complete((s.real_time)(self.frame))?;
        }
        if pending() != SwitchMode::NONE && !(s.modal)() {
            self.switch(pending())?;
            set_pending(SwitchMode::NONE);
            if (s.exit_game)() {
                return Ok(());
            }
        }
        (s.sprite_lru)();
        if (s.network_available)() {
            (s.network_background)();
        }
        (s.remote_messages)();
        if (s.networking)() && !(s.modal)() {
            complete((s.network_loop)(self.frame))?;
        } else {
            if (s.reconnect_due)() {
                complete((s.network_reconnect)(self.frame))?;
            }
            complete(unsafe { openttd_rust_session_state_loop(s, self.frame) })?;
        }
        if pause() == 0 && (s.full_animation)() {
            (s.palette)();
        }
        (s.sound)();
        (s.music)();
        (s.social_callbacks)();
        Ok(())
    }
}

/// # Safety
/// Services and the stack native frame remain live through synchronous reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_session_intro(
    s: &Services,
    frame: *mut Frame,
    newgrfs: bool,
) -> bool {
    (Session { s, frame }).intro(newgrfs).is_ok()
}
/// # Safety
/// `input` is null for the current file or a live native load argument object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_session_safe_load(
    s: &Services,
    frame: *mut Frame,
    input: *mut Load,
    new_mode: GameMode,
) -> bool {
    (Session { s, frame })
        .safe_load(input, new_mode)
        .unwrap_or(false)
}
/// # Safety
/// The native exception/resource frame is live, and no exception crosses Rust.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_session_switch(
    s: &Services,
    frame: *mut Frame,
    mode: SwitchMode,
) -> bool {
    (Session { s, frame }).switch(mode).is_ok()
}
/// # Safety
/// Native Backup<CompanyID> restores on frame destruction if a phase fails.
#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn openttd_rust_session_state_loop(s: &Services, parent: *mut Frame) -> bool {
    let mut storage = std::mem::MaybeUninit::<FrameStorage>::uninit();
    let frame = (s.state_frame_begin)(storage.as_mut_ptr());
    let result = (Session { s, frame }).state_loop().is_ok();
    (s.state_frame_end)(parent, frame) && result
}
/// # Safety
/// Native frame is live through loading, network and callback reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_session_game_loop(s: &Services, frame: *mut Frame) -> bool {
    (Session { s, frame }).game_loop().is_ok()
}

pub(crate) fn new_game_done(s: &Services, frame: *mut Frame) -> bool {
    (Session { s, frame }).new_game_done().is_ok()
}
pub(crate) unsafe fn switch(s: &Services, frame: *mut Frame, mode: SwitchMode) -> bool {
    (Session { s, frame }).switch(mode).is_ok()
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDSessionDimensions" ["session"] => Dimensions { x, y, };
    "OpenTTDSessionSecondaryColour" ["session"] => SecondaryColour { colour, used, };
    "OpenTTDSessionFrameStorage" ["session"] => FrameStorage { storage, };
    "OpenTTDWorldFrameStorage" ["session"] => WorldFrameStorage { storage, };
    "OpenTTDSessionState" ["session"] => State { mode, switch_mode, pause, };
    "OpenTTDSessionServices" ["session"] => Services{
        clock,
        company,
        shared,
        schedule,
        world,
        networking,
        network_server,
        dedicated,
        server_requested,
        clear_server_requested,
        network_available,
        exit_game,
        set_exit,
        modal,
        network_reboot,
        network_disconnect,
        network_server_start,
        network_background,
        network_start_game,
        network_join,
        kill_ai,
        load_config,
        reset_session_stats,
        reset_file_mode,
        generate_id,
        reset_windows,
        setup_windows,
        fix_title_zoom,
        release_cursor,
        check_glyphs,
        music,
        rail_gui,
        road_gui,
        check_engines,
        check_industries,
        close_saveload,
        joining_social,
        ask_survey,
        process_async_save,
        sprite_lru,
        remote_messages,
        palette,
        sound,
        social_callbacks,
        news,
        window_tick,
        check_caches,
        reset_engine_overrides,
        dirty,
        reload_config,
        reset_grf,
        autosave_frequency,
        leave_survey,
        file_kind,
        map_dimensions,
        has_gui,
        pause_on_newgame,
        starting_colour,
        secondary_colour,
        disable_elrails,
        settings_elrail,
        company_settings,
        company_colour,
        post_pause,
        reset_ai_config,
        reset_game_config,
        publish_settings,
        publish_old_settings,
        newgame_year,
        fallback_sound_warning,
        show_sound_warning,
        survey_ask,
        show_load_error,
        social_mode,
        load_failed_debug,
        pause_query,
        pause_change,
        pause_dirty,
        full_animation,
        reconnect_due,
        console_script,
        game_script,
        ai_script,
        startup_company,
        network_loop,
        network_reconnect,
        state_frame_begin,
        state_frame_end,
        real_time,
        scan,
        world_setup_begin,
        world_setup_end,
        world_frame_begin,
        world_frame_end,
        heightmap_screenshot,
        desync_save,
        load,
        intro_load,
        save,
        failed,
        console_exception,
        state_begin,
        company_begin,
        company_restore,
        scripts_begin,
        scripts_end,
        paused_measurements,
        scripts_enabled,
        assert_local_company,
        desync_level,
        map_log_y,
        tick_timer,
        tile_loop_begin,
        tile_loop_end,
        vehicle_tables,
        vehicle_cost,
        vehicle_end,
        landscaping,
        schedule_time,
        town,
        trees,
        station,
        links,
        visits,
        tree_tick,
        industry_tick,
        industry_builder,
        company_tick,
        post_ai,
        road_begin,
        road_end,
        landscape_begin,
        landscape_end,
        schedule_measurement,
        schedule_end,
    };
}

/// # Safety
/// The native setup and profiling frames stay live through synchronous generation.
/// A nonnull callback pointer is readable at entry; its nullable value is copied
/// before setup and never borrowed across generation or callback reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_session_generate(
    s: &Services,
    frame: *mut Frame,
    kind: crate::genworld::Mode,
    x: u32,
    y: u32,
    reset: bool,
    callback: *const crate::genworld::Callback,
) -> bool {
    if !callback.is_null() {
        crate::genworld::set_native_callback(unsafe { callback.read() });
    }
    (Session { s, frame }).generate(kind, x, y, reset).is_ok()
}
