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
use crate::debug::debug;
use crate::game_state::GameState;
use crate::network_action::{Actions, Resume};
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
use crate::saveload::SaveOrLoadResult as LoadResult;
#[repr(C)]
pub struct Services {
    pub clock: *const crate::timer_game_common::ClockServices,
    pub company: *const crate::company::Services,
    pub network: *const crate::network_frame::Services,
    pub shared: *const crate::services::Services,
    pub schedule: *const crate::linkgraph_schedule::Services,
    pub world: extern "C" fn() -> *const crate::genworld::WorldServices,
    pub server_requested: extern "C" fn() -> bool,
    pub clear_server_requested: extern "C" fn(),
    pub network_available: extern "C" fn() -> bool,
    pub network_background: extern "C" fn(),
    pub kill_ai: extern "C" fn(),
    pub load_config: extern "C" fn(),
    pub reset_session_stats: extern "C" fn(),
    pub reset_file_mode: extern "C" fn(),
    pub generate_id: extern "C" fn(),
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
    pub reset_grf: extern "C" fn(bool),
    pub autosave_frequency: extern "C" fn(bool),
    pub leave_survey: extern "C" fn(),
    pub file_kind: extern "C" fn() -> FileKind,
    pub has_gui: extern "C" fn() -> bool,
    pub settings_elrail: extern "C" fn(bool),
    pub company_settings: extern "C" fn(),
    pub reset_ai_config: extern "C" fn(CompanyID),
    pub reset_game_config: extern "C" fn(),
    pub publish_settings: extern "C" fn(),
    pub publish_old_settings: extern "C" fn(),
    pub newgame_year: extern "C" fn(i32),
    pub fallback_sound_warning: extern "C" fn() -> bool,
    pub show_sound_warning: extern "C" fn(),
    pub show_load_error: extern "C" fn(bool),
    pub social_mode: extern "C" fn(GameMode),
    pub pause_query: extern "C" fn(),
    pub pause_change: extern "C" fn(u8, PauseMode),
    pub pause_dirty: extern "C" fn(),
    pub full_animation: extern "C" fn() -> bool,
    pub console_script: extern "C" fn(*mut Frame) -> bool,
    pub game_script: extern "C" fn(*mut Frame) -> bool,
    pub ai_script: extern "C" fn(*mut Frame) -> bool,
    pub startup_company: extern "C" fn(*mut Frame) -> bool,
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
    pub network_actions: extern "C" fn(*mut Frame) -> *mut crate::network_action::Storage,
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
    pub tick_timer: extern "C" fn(*mut Frame) -> bool,
    pub tile_loop_begin: extern "C" fn(*mut Frame) -> *const crate::landscape_generation::Loop,
    pub tile_loop_end: extern "C" fn(*mut Frame),
    pub vehicle_tables: extern "C" fn(*mut Frame, bool) -> crate::vehicle::scheduler::Tables,
    pub vehicle_cost: extern "C" fn(*mut Frame) -> *mut std::ffi::c_void,
    pub vehicle_end: extern "C" fn(*mut Frame),
    pub town: *const crate::town::Leaves,
    pub trees: *const crate::trees::TreeServices,
    pub station: *const crate::station_service::World,
    pub links: *const crate::station_service::Links,
    pub visits: *const crate::vehicle::visits::Services,
    pub industry_builder: *mut crate::industry::Builder,
    pub post_ai: extern "C" fn(*mut Frame) -> bool,
    pub road_begin: extern "C" fn(*mut Frame) -> *const crate::road_cmd::CommandContext,
    pub road_end: extern "C" fn(*mut Frame),
    pub landscape_begin: extern "C" fn(*mut Frame),
    pub landscape_end: extern "C" fn(*mut Frame),
    pub schedule_measurement: extern "C" fn(*mut Frame) -> *mut std::ffi::c_void,
    pub schedule_end: extern "C" fn(*mut Frame),
}
/// `ParticipateSurvey::PS_ASK`.
const PS_ASK: u8 = 0;
impl Services {
    /// One client or game setting from the shared catalog, loaded at its observation point.
    fn setting<T: Copy>(
        &self,
        field: impl FnOnce(&crate::settings_type::SettingAddresses) -> *const T,
    ) -> T {
        // SAFETY: The shared table and catalog are immutable for the process lifetime; one
        // field-sized load of a process-wide setting.
        unsafe { field((*self.shared).settings()).read() }
    }
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
pub extern "C" fn openttd_rust_session_scan_abi(callback: *mut ScanCallback) -> u8 {
    if unsafe { *SCAN.get() } {
        return u8::from(false);
    }
    unsafe {
        *SCAN.get() = true;
        *SCAN_CALLBACK.get() = callback;
    }
    u8::from(true)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_session_pause_abi(
    s: &Services,
    mode: PauseMode,
    paused: bool,
    execute: bool,
) -> u8 {
    match mode {
        PauseMode::SAVE_LOAD
        | PauseMode::ERROR
        | PauseMode::NORMAL
        | PauseMode::GAME_SCRIPT
        | PauseMode::LINK_GRAPH => {}
        PauseMode::JOIN | PauseMode::ACTIVE_CLIENTS
            if (unsafe { &*s.company }.finance.networking)() => {}
        _ => return u8::from(false),
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
    u8::from(true)
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
    world: std::cell::OnceCell<*const crate::genworld::WorldServices>,
}
impl<'a> Session<'a> {
    fn new(s: &'a Services, frame: *mut Frame) -> Self {
        Self {
            s,
            frame,
            world: std::cell::OnceCell::new(),
        }
    }
    // Cache only the immutable table identity; its operations observe live native state.
    fn world(&self) -> &crate::genworld::WorldServices {
        unsafe { &**self.world.get_or_init(|| (self.s.world)()) }
    }
    fn network(&self) -> &crate::network_frame::Services {
        // SAFETY: The immutable native service table outlives every session call.
        unsafe { &*self.s.network }
    }
    fn actions(&self) -> Actions {
        let work = unsafe { Actions::from_storage((self.s.network_actions)(self.frame)) };
        work.bind(self.network(), self.s, self.frame);
        work
    }

    fn checked(&self) -> Outcome {
        complete(!(self.s.failed)(self.frame))
    }
    fn generate(&self, kind: crate::genworld::Mode, x: u32, y: u32, reset: bool) -> Outcome {
        let s = self.s;
        let setup = (s.world_setup_begin)(self.frame);
        let ready = unsafe { crate::genworld::setup(self.world(), setup, kind, x, y, reset) };
        complete((s.world_setup_end)(self.frame))?;
        if ready {
            complete(crate::genworld::generate(s, self.frame))?;
        }
        Ok(())
    }
    fn sized_world(&self, kind: crate::genworld::Mode, reset: bool) -> Outcome {
        let settings = unsafe { &*self.s.shared }.settings();
        let x = 1_u32 << unsafe { settings.map_x.read() };
        let y = 1_u32 << unsafe { settings.map_y.read() };
        self.generate(kind, x, y, reset)
    }
    fn intro(&self, newgrfs: bool) -> Outcome {
        let s = self.s;
        set_mode(GameMode::MENU);
        if newgrfs {
            (s.reset_grf)(false);
        }
        (self.world().reset_windows)();
        (self.world().setup_windows)();
        let result = (s.intro_load)(self.frame);
        self.checked()?;
        if result != LoadResult::SL_OK {
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
        crate::network_lifecycle::on_game_start(self.network().lifecycle, self.network(), s);
        complete((s.console_script)(self.frame))
    }
    fn new_game_done(&self) -> Outcome {
        let s = self.s;
        (s.settings_elrail)(unsafe { (&*s.shared).settings().disable_elrails.read() });
        if !(s.has_gui)() {
            self.start_game(true)?;
            if s.setting(|c| c.pause_on_newgame) {
                crate::command_post::pause(unsafe { &*s.shared }, PauseMode::NORMAL, true);
            }
            return Ok(());
        }
        complete((s.startup_company)(self.frame))?;
        (s.company_settings)();
        // COLOUR_END selects the original random colour.
        let colour = s.setting(|c| c.starting_colour);
        if colour != COLOUR_END {
            crate::command_post::colour(unsafe { &*s.shared }, true, colour);
        }
        let settings = unsafe { &*s.shared }.settings();
        let secondary = unsafe { settings.secondary_colour.read() };
        if secondary != COLOUR_END
            && unsafe { settings.used_liveries.read() } & (1_u64 << crate::livery::LS_DEFAULT) != 0
        {
            crate::command_post::colour(unsafe { &*s.shared }, false, secondary);
        }
        self.start_game(false)?;
        (s.rail_gui)();
        (s.road_gui)();
        if s.setting(|c| c.pause_on_newgame) {
            crate::command_post::pause(unsafe { &*s.shared }, PauseMode::NORMAL, true);
        }
        (s.check_engines)();
        (s.check_industries)();
        (unsafe { &*s.shared }.screen_dirty)();
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
        if result == LoadResult::SL_OK {
            return Ok(true);
        }
        if (self.world().dedicated)() && old == GameMode::MENU {
            debug!(net, 0, "Loading requested map failed; closing server.");
            (self.world().set_exit)();
            return Ok(false);
        }
        if result != LoadResult::SL_REINIT {
            set_mode(old);
            return Ok(false);
        }
        if (self.world().dedicated)() {
            debug!(
                net,
                0, "Loading game failed, so a new (random) game will be started"
            );
            self.new_game(false, true)?;
            return Ok(false);
        }
        if (unsafe { &*s.company }.network_server)() {
            let work = self.actions();
            crate::network_lifecycle::disconnect(
                self.network().lifecycle,
                self.network(),
                true,
                work,
            );
            if work.pending() {
                work.push(Resume::LoadRestore(old));
                return Err(());
            }
        }
        self.load_restore(old)?;
        Ok(false)
    }
    fn load_restore(&self, old: GameMode) -> Outcome {
        if old == GameMode::EDITOR {
            self.new_editor()?;
        } else {
            self.intro(true)?;
        }
        Ok(())
    }
    fn switch(&self, new_mode: SwitchMode) -> Outcome {
        let s = self.s;
        let work = self.actions();
        if new_mode != SwitchMode::SAVE_GAME && (unsafe { &*s.company }.finance.networking)() {
            if (unsafe { &*s.company }.network_server)()
                && matches!(
                    new_mode,
                    SwitchMode::LOAD_GAME | SwitchMode::NEWGAME | SwitchMode::RESTARTGAME
                )
            {
                crate::network_lifecycle::reboot(self.network().lifecycle, self.network(), s, work);
            } else {
                crate::network_lifecycle::disconnect(
                    self.network().lifecycle,
                    self.network(),
                    true,
                    work,
                );
            }
            if work.pending() {
                work.push(Resume::SwitchNetwork(new_mode));
                return Err(());
            }
        }
        self.switch_network(new_mode, work)
    }
    fn switch_network(&self, new_mode: SwitchMode, work: Actions) -> Outcome {
        let s = self.s;
        if new_mode != SwitchMode::SAVE_GAME && (s.server_requested)() {
            if new_mode != SwitchMode::MENU {
                if s.setting(|c| c.reload_cfg) {
                    (s.load_config)();
                    openttd_rust_session_settings(s);
                    (s.reset_grf)(false);
                }
                if crate::network_lifecycle::server_start(
                    self.network().lifecycle,
                    self.network(),
                    s,
                    self.frame,
                    work,
                )
                .is_err()
                {
                    work.push(Resume::SwitchStartup(new_mode));
                    return Err(());
                }
            } else {
                (s.clear_server_requested)();
            }
        }
        self.switch_startup(new_mode)
    }
    fn switch_startup(&self, new_mode: SwitchMode) -> Outcome {
        let s = self.s;
        if new_mode != SwitchMode::SAVE_GAME {
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
                (self.world().reset_windows)();
                let Ok(loaded) = self.safe_load(std::ptr::null_mut(), GameMode::NORMAL) else {
                    self.actions().push(Resume::SwitchLoad(GameMode::NORMAL));
                    return Err(());
                };
                if loaded {
                    if (s.file_kind)() == FT_SCENARIO {
                        self.start_scenario();
                    }
                    self.start_game((self.world().dedicated)())?;
                    crate::command_post::pause(unsafe { &*s.shared }, PauseMode::SAVE_LOAD, false);
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
                (unsafe { &*s.shared }.screen_dirty)();
                (s.social_mode)(GameMode::EDITOR);
            }
            SwitchMode::LOAD_SCENARIO => {
                let Ok(loaded) = self.safe_load(std::ptr::null_mut(), GameMode::EDITOR) else {
                    self.actions().push(Resume::SwitchLoad(GameMode::EDITOR));
                    return Err(());
                };
                if loaded {
                    crate::company::set_local_company(unsafe { &*s.company }, CompanyID::NONE);
                    (s.generate_id)();
                    (s.newgame_year)(crate::timer_game_common::calendar_year().0);
                    crate::command_post::pause(unsafe { &*s.shared }, PauseMode::SAVE_LOAD, false);
                } else {
                    (s.show_load_error)(true);
                }
                (s.social_mode)(GameMode::EDITOR);
            }
            SwitchMode::JOIN_GAME => {
                self.intro(true)?;
                let work = self.actions();
                crate::network_lifecycle::join_game(self.network().lifecycle, self.network(), work);
                if work.pending() {
                    work.push(Resume::JoiningSocial);
                    return Err(());
                }
                (s.joining_social)();
            }
            SwitchMode::MENU => {
                self.intro(true)?;
                if (s.fallback_sound_warning)() {
                    (s.show_sound_warning)();
                }
                if s.setting(|c| c.participate_survey) == PS_ASK && !unsafe { *ASKED_SURVEY.get() }
                {
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
                if result == LoadResult::SL_OK {
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
                (unsafe { &*s.shared }.screen_dirty)();
            }
            _ => (unsafe { &*s.shared }.not_reached)(),
        }
        Ok(())
    }
    fn start_scenario(&self) {
        (self.s.reset_engine_overrides)();
        let services = unsafe { &*self.world().industry };
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
            unsafe { &*self.s.shared }.settings().construction_limits,
        );
    }
    fn storage(&self, mode: u8) {
        let world = self.world();
        unsafe {
            crate::newgrf_storage::openttd_rust_storage_switch_mode(
                mode,
                false,
                (*world.landscape).assert_condition,
                (*(*world.landscape).shared).storage_discard,
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
                shared.map_log_y(),
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
            crate::trees::tick(s.trees, s.shared);
            crate::station_service::on_tick_station(&*s.station, &*s.links, &*s.visits);
            let world = self.world();
            crate::industry::on_tick_industry(s.industry_builder, world.industry);
        }
        (s.landscape_end)(self.frame);
        let mut company = crate::company::Tick::default();
        while unsafe { crate::company::on_tick(&*s.company, &mut company) } {
            complete((s.post_ai)(self.frame))?;
        }
        crate::linkgraph_schedule::tick(
            unsafe { &*s.schedule },
            unsafe { &*s.shared },
            (unsafe { &*s.company }.finance.networking)()
                && !(unsafe { &*s.company }.network_server)(),
            (s.schedule_measurement)(self.frame),
        );
        (s.schedule_end)(self.frame);
        Ok(())
    }
    fn state_loop(&self) -> Outcome {
        let s = self.s;
        if !(unsafe { &*s.company }.finance.networking)()
            || (unsafe { &*s.company }.network_server)()
        {
            unsafe {
                crate::linkgraph_schedule::pause(
                    &*s.schedule,
                    &*s.shared,
                    pause() & PauseMode::LINK_GRAPH.bit() != 0,
                    pause() == 0,
                );
            }
        }
        if pause() != 0 || (self.world().modal)() {
            (s.paused_measurements)();
            if !(self.world().modal)() {
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
            if crate::debug::level(|l| l.desync) > 2
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
            if (self.world().exit_game)() {
                return Ok(());
            }
        }
        (s.process_async_save)();
        if mode() == GameMode::NORMAL {
            complete((s.real_time)(self.frame))?;
        }
        if pending() != SwitchMode::NONE && !(self.world().modal)() {
            if self.switch(pending()).is_err() {
                self.actions().push(Resume::LoopSwitch);
                return Err(());
            }
            return self.loop_switch();
        }
        self.loop_network()
    }
    fn loop_switch(&self) -> Outcome {
        set_pending(SwitchMode::NONE);
        if (self.world().exit_game)() {
            return Ok(());
        }
        self.loop_network()
    }
    fn loop_network(&self) -> Outcome {
        let s = self.s;
        let work = self.actions();
        (s.sprite_lru)();
        if (s.network_available)() {
            (s.network_background)();
        }
        (s.remote_messages)();
        if (unsafe { &*s.company }.finance.networking)() && !(self.world().modal)() {
            // SAFETY: The native table outlives the session.
            if crate::network_frame::game_loop(s, unsafe { &*s.network }, self.frame, work).is_err()
            {
                work.push(Resume::LoopEnd);
                return Err(());
            }
        } else {
            if crate::network_lifecycle::reconnect_due(self.network()) {
                crate::network_lifecycle::reconnect(
                    self.network().lifecycle,
                    self.network(),
                    s,
                    self.frame,
                    work,
                );
                if work.pending() {
                    work.push(Resume::LoopState);
                    return Err(());
                }
            }
            return self.loop_state();
        }
        loop_end(s);
        Ok(())
    }
    fn loop_state(&self) -> Outcome {
        complete(unsafe { openttd_rust_session_state_loop_abi(self.s, self.frame) != 0 })?;
        loop_end(self.s);
        Ok(())
    }
}

fn loop_end(s: &Services) {
    if pause() == 0 && (s.full_animation)() {
        (s.palette)();
    }
    (s.sound)();
    (s.music)();
    (s.social_callbacks)();
}

/// # Safety
/// Services and the stack native frame remain live through synchronous reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_session_intro_abi(
    s: &Services,
    frame: *mut Frame,
    newgrfs: bool,
) -> u8 {
    u8::from((Session::new(s, frame)).intro(newgrfs).is_ok())
}
/// # Safety
/// `input` is null for the current file or a live native load argument object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_session_safe_load_abi(
    s: &Services,
    frame: *mut Frame,
    input: *mut Load,
    new_mode: GameMode,
) -> u8 {
    u8::from(
        (Session::new(s, frame))
            .safe_load(input, new_mode)
            .unwrap_or(false),
    )
}
/// # Safety
/// The native exception/resource frame is live, and no exception crosses Rust.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_session_switch_abi(
    s: &Services,
    frame: *mut Frame,
    mode: SwitchMode,
) -> u8 {
    u8::from((Session::new(s, frame)).switch(mode).is_ok())
}
/// # Safety
/// Native Backup<CompanyID> restores on frame destruction if a phase fails.
#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn openttd_rust_session_state_loop_abi(
    s: &Services,
    parent: *mut Frame,
) -> u8 {
    let mut storage = std::mem::MaybeUninit::<FrameStorage>::uninit();
    let frame = (s.state_frame_begin)(storage.as_mut_ptr());
    let result = (Session::new(s, frame)).state_loop().is_ok();
    u8::from((s.state_frame_end)(parent, frame) && result)
}
/// # Safety
/// Native frame is live through loading, network and callback reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_session_game_loop_abi(s: &Services, frame: *mut Frame) -> u8 {
    u8::from((Session::new(s, frame)).game_loop().is_ok())
}

pub(crate) fn resume_switch_network(
    s: &Services,
    frame: *mut Frame,
    mode: SwitchMode,
    work: Actions,
) -> Outcome {
    (Session::new(s, frame)).switch_network(mode, work)
}
pub(crate) fn resume_switch_startup(s: &Services, frame: *mut Frame, mode: SwitchMode) -> Outcome {
    (Session::new(s, frame)).switch_startup(mode)
}
pub(crate) fn resume_load_restore(s: &Services, frame: *mut Frame, mode: GameMode) -> Outcome {
    (Session::new(s, frame)).load_restore(mode)
}
pub(crate) fn resume_loop_switch(s: &Services, frame: *mut Frame, _work: Actions) -> Outcome {
    (Session::new(s, frame)).loop_switch()
}
pub(crate) fn resume_loop_state(s: &Services, frame: *mut Frame) -> Outcome {
    (Session::new(s, frame)).loop_state()
}
pub(crate) fn resume_loop_end(s: &Services) {
    loop_end(s);
}
pub(crate) fn resume_switch_load(s: &Services, mode: GameMode) {
    (s.show_load_error)(true);
    (s.social_mode)(mode);
}

pub(crate) fn safe_load(
    s: &Services,
    frame: *mut Frame,
    input: *mut Load,
    new_mode: GameMode,
) -> Result<bool, ()> {
    (Session::new(s, frame)).safe_load(input, new_mode)
}

pub(crate) fn new_game_done(s: &Services, frame: *mut Frame) -> bool {
    (Session::new(s, frame)).new_game_done().is_ok()
}
pub(crate) unsafe fn switch(s: &Services, frame: *mut Frame, mode: SwitchMode) -> bool {
    (Session::new(s, frame)).switch(mode).is_ok()
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDSessionFrameStorage" ["session"] => FrameStorage { storage, };
    "OpenTTDWorldFrameStorage" ["session"] => WorldFrameStorage { storage, };
    "OpenTTDSessionState" ["session"] => State { mode, switch_mode, pause, };
    "OpenTTDSessionServices" ["session"] => Services{
        clock,
        company,
        network,
        shared,
        schedule,
        world,
        server_requested,
        clear_server_requested,
        network_available,
        network_background,
        kill_ai,
        load_config,
        reset_session_stats,
        reset_file_mode,
        generate_id,
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
        reset_grf,
        autosave_frequency,
        leave_survey,
        file_kind,
        has_gui,
        settings_elrail,
        company_settings,
        reset_ai_config,
        reset_game_config,
        publish_settings,
        publish_old_settings,
        newgame_year,
        fallback_sound_warning,
        show_sound_warning,
        show_load_error,
        social_mode,
        pause_query,
        pause_change,
        pause_dirty,
        full_animation,
        console_script,
        game_script,
        ai_script,
        startup_company,
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
        network_actions,
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
        tick_timer,
        tile_loop_begin,
        tile_loop_end,
        vehicle_tables,
        vehicle_cost,
        vehicle_end,
        town,
        trees,
        station,
        links,
        visits,
        industry_builder,
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
pub unsafe extern "C" fn openttd_rust_session_generate_abi(
    s: &Services,
    frame: *mut Frame,
    kind: crate::genworld::Mode,
    x: u32,
    y: u32,
    reset: bool,
    callback: *const crate::genworld::Callback,
) -> u8 {
    if !callback.is_null() {
        crate::genworld::set_native_callback(unsafe { callback.read() });
    }
    u8::from((Session::new(s, frame)).generate(kind, x, y, reset).is_ok())
}
