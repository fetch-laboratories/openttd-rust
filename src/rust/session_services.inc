/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file session_services.inc Native leaves for Rust session orchestration. */
#include "service_table_check.hpp"
#include "services_station.h"
#include "services_industry.h"
#include "town_services.h"
#include "abi-visit-OpenTTDSessionServices.hpp"
static void RustSessionRealtime()
{
	static auto last_time = std::chrono::steady_clock::now();
	auto now = std::chrono::steady_clock::now();
	auto delta = std::chrono::duration_cast<std::chrono::milliseconds>(now - last_time);
	if (delta.count() != 0) {
		TimerManager<TimerGameRealtime>::Elapsed(delta);
		last_time = now;
	}
}
static void RustSessionDesyncSave()
{
	std::string name = fmt::format("dmp_cmds_{:08x}_{:08x}.sav", _settings_game.game_creation.generation_seed, TimerGameEconomy::date);
	SaveOrLoad(name, SLO_SAVE, DFT_GAME_FILE, AUTOSAVE_DIR, false);
}
const OpenTTDSessionServices &GetRustSessionServices() noexcept
{
	static const OpenTTDSessionServices services{
		.clock = &GetRustClockServices(),
		.company = &GetRustCompanyServices(),
		.shared = &GetRustSharedServices(),
		.schedule = &LinkGraphSchedule::services,
		.world = []() noexcept { return &GetRustWorldServices(); },
		.networking = RustNetworking,
		.network_server = RustNetworkServer,
		.dedicated = RustNetworkDedicated,
		.server_requested = []() noexcept { return _is_network_server; },
		.clear_server_requested = []() noexcept { _is_network_server = false; },
		.network_available = RustNetworkAvailable,
		.exit_game = RustExitGame,
		.set_exit = RustSetExitGame,
		.modal = RustHasModalProgress,
		.network_reboot = []() noexcept { NetworkReboot(); },
		.network_disconnect = []() noexcept { NetworkDisconnect(); },
		.network_server_start = [](OpenTTDSessionFrame *f) noexcept { return SessionException(f, [] { NetworkServerStart(); }); },
		.network_background = []() noexcept { NetworkBackgroundLoop(); },
		.network_start_game = []() noexcept { NetworkOnGameStart(); },
		.network_join = []() noexcept { NetworkClientJoinGame(); },
		.kill_ai = []() noexcept { AI::KillAll(); },
		.load_config = []() noexcept { LoadFromConfig(); },
		.reset_session_stats = []() noexcept { _game_session_stats.start_time = std::chrono::steady_clock::now(); _game_session_stats.savegame_size = std::nullopt; },
		.reset_file_mode = []() noexcept { _file_to_saveload.SetMode(FIOS_TYPE_INVALID, SLO_INVALID); },
		.generate_id = []() noexcept { GenerateSavegameId(); },
		.reset_windows = []() noexcept { ResetWindowSystem(); },
		.setup_windows = []() noexcept { SetupColoursAndInitialWindow(); },
		.fix_title_zoom = []() noexcept { FixTitleGameZoom(); },
		.release_cursor = []() noexcept { _cursor.fix_at = false; },
		.check_glyphs = []() noexcept { CheckForMissingGlyphs(); },
		.music = []() noexcept { MusicLoop(); },
		.rail_gui = []() noexcept { InitializeRailGUI(); },
		.road_gui = []() noexcept { InitializeRoadGUI(); },
		.check_engines = []() noexcept { CheckEngines(); },
		.check_industries = []() noexcept { CheckIndustries(); },
		.close_saveload = []() noexcept { CloseWindowById(WC_SAVELOAD, 0); },
		.joining_social = []() noexcept { SocialIntegration::EventJoiningMultiplayer(); },
		.ask_survey = []() noexcept { ShowNetworkAskSurvey(); },
		.process_async_save = []() noexcept { ProcessAsyncSaveFinish(); },
		.sprite_lru = []() noexcept { IncreaseSpriteLRU(); },
		.remote_messages = []() noexcept { DebugSendRemoteMessages(); },
		.palette = []() noexcept { DoPaletteAnimations(); },
		.sound = []() noexcept { SoundDriver::GetInstance()->MainLoop(); },
		.social_callbacks = []() noexcept { SocialIntegration::RunCallbacks(); },
		.news = []() noexcept { NewsLoop(); },
		.window_tick = []() noexcept { CallWindowGameTickEvent(); },
		.check_caches = []() noexcept { CheckCaches(); },
		.reset_engine_overrides = []() noexcept { EngineOverrideManager::ResetToCurrentNewGRFConfig(); },
		.dirty = RustScreenDirty,
		.set_local_company = RustSetLocalCompany,
		.first_playable_company = []() noexcept { return GetFirstPlayableCompanyID().base(); },
		.reload_config = []() noexcept { return _settings_client.network.reload_cfg; },
		.reset_grf = [](bool value) noexcept { ResetGRFConfig(value); },
		.autosave_frequency = [](bool reset) noexcept { ChangeAutosaveFrequency(reset); },
		.leave_survey = []() noexcept { _survey.Transmit(NetworkSurveyHandler::Reason::LEAVE); },
		.file_kind = []() noexcept { return _file_to_saveload.ftype.abstract; },
		.map_dimensions = []() noexcept { return OpenTTDSessionDimensions{uint32_t(1U << _settings_game.game_creation.map_x), uint32_t(1U << _settings_game.game_creation.map_y)}; },
		.has_gui = []() noexcept { return VideoDriver::GetInstance()->HasGUI(); },
		.pause_on_newgame = []() noexcept { return _settings_client.gui.pause_on_newgame; },
		.starting_colour = []() noexcept { return _settings_client.gui.starting_colour; },
		.secondary_colour = []() noexcept { return OpenTTDSessionSecondaryColour{_settings_client.gui.starting_colour_secondary, HasBit(_loaded_newgrf_features.used_liveries, LS_DEFAULT)}; },
		.disable_elrails = RustDisableElrails,
		.settings_elrail = [](bool value) noexcept { SettingsDisableElrail(value); },
		.company_settings = []() noexcept { Company::Get(CompanyID::Begin())->settings = _settings_client.company; },
		.company_colour = [](bool primary, Colours colour) noexcept { Command<CMD_SET_COMPANY_COLOUR>::Post(LS_DEFAULT, primary, colour); },
		.post_pause = [](PauseMode mode, bool pause) noexcept { Command<CMD_PAUSE>::Post(mode, pause); },
		.reset_ai_config = [](uint8_t c) noexcept { _settings_game.script_config.ai[CompanyID(c)].reset(); },
		.reset_game_config = []() noexcept { _settings_game.script_config.game.reset(); },
		.publish_settings = []() noexcept { _settings_game = _settings_newgame; },
		.publish_old_settings = []() noexcept { _old_vds = _settings_client.company.vehicle; },
		.newgame_year = [](int32_t year) noexcept { _settings_newgame.game_creation.starting_year = TimerGameCalendar::Year(year); },
		.fallback_sound_warning = []() noexcept { return BaseSounds::ini_set.empty() && BaseSounds::GetUsedSet()->fallback && SoundDriver::GetInstance()->HasOutput(); },
		.show_sound_warning = []() noexcept { ShowErrorMessage(GetEncodedString(STR_WARNING_FALLBACK_SOUNDSET), {}, WL_CRITICAL); BaseSounds::ini_set = BaseSounds::GetUsedSet()->name; },
		.survey_ask = []() noexcept { return _settings_client.network.participate_survey == PS_ASK; },
		.show_load_error = [](bool critical) noexcept { ShowErrorMessage(GetSaveLoadErrorType(), GetSaveLoadErrorMessage(), critical ? WL_CRITICAL : WL_ERROR); },
		.social_mode = [](GameMode mode) noexcept { UpdateSocialIntegration(mode); },
		.load_failed_debug = [](bool initial) noexcept { if (initial) { Debug(net, 0, "Loading requested map failed; closing server."); } else { Debug(net, 0, "Loading game failed, so a new (random) game will be started"); } },
		.pause_query = RustAskUnsafeUnpause,
		.pause_change = [](uint8_t prev, PauseMode mode) noexcept { NetworkHandlePauseChange(PauseModes{prev}, mode); },
		.pause_dirty = []() noexcept { SetWindowDirty(WC_STATUS_BAR, 0); SetWindowDirty(WC_MAIN_TOOLBAR, 0); },
		.full_animation = []() noexcept { return HasBit(_display_opt, DO_FULL_ANIMATION); },
		.reconnect_due = []() noexcept { return _network_reconnect > 0 && --_network_reconnect == 0; },
		.console_script = [](OpenTTDSessionFrame *f) noexcept { return SessionException(f, [=] { IConsoleCmdExec("exec scripts/game_start.scr 0"); }); },
		.game_script = [](OpenTTDSessionFrame *f) noexcept { return SessionException(f, [=] { Game::GameLoop(); }); },
		.ai_script = [](OpenTTDSessionFrame *f) noexcept { return SessionException(f, [=] { AI::GameLoop(); }); },
		.startup_company = [](OpenTTDSessionFrame *f) noexcept { return SessionException(f, [=] { DoStartupNewCompany(false); }); },
		.network_loop = [](OpenTTDSessionFrame *f) noexcept { return SessionException(f, [=] { NetworkGameLoop(); }); },
		.network_reconnect = [](OpenTTDSessionFrame *f) noexcept { return SessionException(f, [=] { NetworkClientConnectGame(_settings_client.network.last_joined, COMPANY_SPECTATOR); }); },
		.state_loop = [](OpenTTDSessionFrame *f) noexcept { return SessionException(f, [=] { StateGameLoop(); }); },
		.real_time = [](OpenTTDSessionFrame *f) noexcept { return SessionException(f, [=] { RustSessionRealtime(); }); },
		.scan = [](OpenTTDSessionFrame *f, NewGRFScanCallback *callback) noexcept { return SessionException(f, [=] { ScanNewGRFFiles(callback); }); },
		.world_setup_begin = [](OpenTTDSessionFrame *f) noexcept { return &f->setup.emplace(); },
		.world_setup_end = [](OpenTTDSessionFrame *f) noexcept { f->exception = f->setup->cancellation; f->setup.reset(); return !f->exception; },
		.world_run = RustRunWorldGeneration,
		.heightmap_screenshot = [](OpenTTDSessionFrame *f) noexcept { return SessionException(f, [=] { MakeHeightmapScreenshot(_file_to_saveload.name); }); },
		.desync_save = [](OpenTTDSessionFrame *f) noexcept { return SessionException(f, [=] { RustSessionDesyncSave(); }); },
		.load = [](OpenTTDSessionFrame *f, OpenTTDSessionLoad *load) noexcept { SaveOrLoadResult result = SL_ERROR; SessionException(f, [&] { result = load == nullptr ? SaveOrLoad(_file_to_saveload.name, _file_to_saveload.file_op, _file_to_saveload.ftype.detailed, NO_DIRECTORY) : load->filter == nullptr ? SaveOrLoad(load->filename, load->operation, load->type, load->subdir) : LoadWithFilter(std::move(load->filter)); }); return result; },
		.intro_load = [](OpenTTDSessionFrame *f) noexcept { SaveOrLoadResult result = SL_ERROR; SessionException(f, [&] { result = SaveOrLoad("opntitle.dat", SLO_LOAD, DFT_GAME_FILE, BASESET_DIR); }); return result; },
		.save = [](OpenTTDSessionFrame *f) noexcept { SaveOrLoadResult result = SL_ERROR; SessionException(f, [&] { result = SaveOrLoad(_file_to_saveload.name, SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY); }); return result; },
		.failed = [](OpenTTDSessionFrame *f) noexcept { return f->exception != nullptr; },
		.console_exception = [](OpenTTDSessionFrame *f) noexcept { return reinterpret_cast<OpenTTDConsoleException *>(&f->exception); },
		.state_begin = [](OpenTTDSessionFrame *f) noexcept { f->framerate.emplace(PFE_GAMELOOP); PerformanceAccumulator::Reset(PFE_GL_LANDSCAPE); },
		.company_begin = [](OpenTTDSessionFrame *f) noexcept { f->company.emplace(_current_company, OWNER_NONE); },
		.company_restore = [](OpenTTDSessionFrame *f) noexcept { f->company->Restore(); },
		.scripts_begin = [](OpenTTDSessionFrame *f) noexcept { f->scripts.emplace(PFE_ALLSCRIPTS); },
		.scripts_end = [](OpenTTDSessionFrame *f) noexcept { f->scripts.reset(); },
		.paused_measurements = []() noexcept { for (auto p : {PFE_GAMELOOP, PFE_GL_ECONOMY, PFE_GL_TRAINS, PFE_GL_ROADVEHS, PFE_GL_SHIPS, PFE_GL_AIRCRAFT, PFE_GL_LANDSCAPE}) PerformanceMeasurer::Paused(p); },
		.scripts_enabled = []() noexcept {
#ifdef DEBUG_DUMP_COMMANDS
			return false;
#else
			return true;
#endif
		},
		.assert_local_company = []() noexcept { assert(IsLocalCompany()); },
		.desync_level = []() noexcept { return _debug_desync_level; },
		.tick_timer = [](OpenTTDSessionFrame *f) noexcept { return SessionException(f, [] { TimerManager<TimerGameTick>::Elapsed(1); }); },
		.tile_loop_begin = [](OpenTTDSessionFrame *f) noexcept -> const OpenTTDGenerationLoop * { assert(*openttd_rust_tile_loop_cursor() != 0); return &f->loop.emplace().view; },
		.tile_loop_end = [](OpenTTDSessionFrame *f) noexcept { f->loop.reset(); },
		.vehicle_tables = [](OpenTTDSessionFrame *f, bool tick) noexcept { if (!tick) return RustVehicleSchedulerTables(); auto &costs = f->vehicle.emplace(); return RustVehicleSchedulerTables(RustCommandCost(&costs.refit), RustCommandCost(&costs.temporary)); },
		.vehicle_cost = [](OpenTTDSessionFrame *f) noexcept { return RustCommandCost(&f->vehicle->replacement); },
		.vehicle_end = [](OpenTTDSessionFrame *f) noexcept { f->vehicle.reset(); },
		.landscaping = RustLandscapingRates,
		.schedule_time = RustScheduleTime,
		.town = &GetRustTownServices(),
		.trees = &GetRustTreeServices(),
		.station = &GetRustStationServices(),
		.links = &GetRustStationLinks(),
		.visits = &GetRustVehicleVisits(),
		.tree_tick = RustTreeTickSettings,
		.industry_tick = RustIndustryTickRecord,
		.industry_builder = RustIndustryBuilderState,
		.company_tick = RustCompanyTick,
		.post_ai = [](OpenTTDSessionFrame *f) noexcept { return SessionException(f, [] { Command<CMD_COMPANY_CTRL>::Post(CCA_NEW_AI, CompanyID::Invalid(), CRR_NONE, INVALID_CLIENT_ID); }); },
		.road_begin = [](OpenTTDSessionFrame *f) noexcept -> const OpenTTDRoadCommandContext * { return &f->road.emplace().view; },
		.road_end = [](OpenTTDSessionFrame *f) noexcept { f->road.reset(); },
		.landscape_begin = [](OpenTTDSessionFrame *f) noexcept { f->landscape.emplace(PFE_GL_LANDSCAPE); },
		.landscape_end = [](OpenTTDSessionFrame *f) noexcept { f->landscape.reset(); },
		.schedule_measurement = [](OpenTTDSessionFrame *f) noexcept -> void * { return &f->schedule; },
		.schedule_end = [](OpenTTDSessionFrame *f) noexcept { f->schedule.reset(); },
	};
	return services;
}
bool CheckAbi_session() { return AbiServicesComplete(GetRustSessionServices()); }
