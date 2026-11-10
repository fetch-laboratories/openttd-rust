/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
#ifndef SIMULATION_SESSION_FIXTURE_HPP
#define SIMULATION_SESSION_FIXTURE_HPP
#include "../../src/rust/session_ffi.h"
#include "../../src/tests/fixture_map.hpp"
#include "../../src/rust/fleet_ffi.h"
#include "../../src/rust/command_services_ffi.h"
#include "../../src/rust/station_lifecycle_ffi.h"
#include "../../src/rust/roadstop_resolver_ffi.h"
#include "../../src/rust/animation_save.hpp"
#include "../../src/rust/newgrf_storage_ffi.h"
#include "../../src/newgrf_storage.h"
/* Narrow owner fixtures enter the production state loop. Unrelated phases use
 * empty typed worlds; each fixture replaces only its observed native services. */
struct SessionFixture {
	static inline SessionFixture *active = nullptr;
	SessionFixture *previous = active;
	OpenTTDFixtureMap map{6};
	OpenTTDConstructionLimits limits{};
	OpenTTDSettingAddresses settings{};
	uint8_t climate = 0, extra_tree_placement = 0, max_competitors = 0, max_companies = 15;
	uint16_t competitor_interval = 0;
	uint16_t recalc_interval = 4, industry_cargo_scale = 100;
	bool ambient = false;
	OpenTTDSharedServices shared{};
	OpenTTDClockServices clock{};
	OpenTTDLandscapeLoopServices tiles{};
	OpenTTDStationLifecycleServices station_tiles{};
	OpenTTDRoadStopResolverServices roadstops{};
	OpenTTDGenerationLoop loop{};
	OpenTTDVehicleServices vehicles{};
	OpenTTDVehicleSchedulerServices scheduler{};
	OpenTTDFleetPendingServices pending{};
	OpenTTDFleetTransactionServices transaction{};
	OpenTTDCommandServices commands{};
	OpenTTDFleetGroupServices group{};
	OpenTTDVehicleSchedulerTables tables{};
	OpenTTDTownLeaves towns{};
	OpenTTDTreeServices trees{};
	OpenTTDStationServices stations{};
	OpenTTDStationLinks links{};
	OpenTTDVehicleVisits visits{};
	OpenTTDIndustryServices industries{};
	OpenTTDWorldServices world{};
	OpenTTDCompanyServices companies{};
	OpenTTDCompanyFinanceServices finance{};
	OpenTTDScheduleServices schedule{};
	OpenTTDSessionServices services{};
	OpenTTDIndustryBuilder *builder = openttd_rust_industry_builder_new();
	bool animate = false;
	bool in_loop = false;
	bool skip_animation = false;
	OpenTTDAnimationServices animation{};
	std::optional<RustAnimationScope> animation_scope;
	std::optional<RustAnimationSave> animation_queue;
	static inline size_t empty_extent = 0;
	static void Nothing() noexcept {}
	static void FrameNothing(OpenTTDSessionFrame *) noexcept {}
	SessionFixture()
	{
		active = this;
		settings.construction_limits = &limits;
		settings.climate = &climate;
		settings.max_companies = &max_companies;
		settings.max_competitors = &max_competitors;
		settings.competitor_interval = &competitor_interval;
		settings.ambient = &ambient;
		settings.industry_cargo_scale = &industry_cargo_scale;
		settings.recalc_interval = &recalc_interval;
		settings.extra_tree_placement = &extra_tree_placement;
		shared.settings = &settings;
		shared.map = &map.statics;
		shared.tile_type = [](uint32_t) noexcept -> uint8_t { return 5; };
		tiles.shared = &shared;
		/* Station tiles enter the Rust tile loop; its first native call is GetByTile. */
		shared.station_type = [](uint32_t) noexcept { return StationType::End; };
		roadstops.by_tile = [](uint32_t) noexcept -> BaseStation * { return nullptr; };
		station_tiles.shared = &shared;
		station_tiles.roadstop = &roadstops;
		tiles.station = &station_tiles;
		group.shared = &shared; shared.current_company = []() noexcept -> uint8_t { return 0; };
		finance.set_current_company = [](uint8_t) noexcept {};
		commands.finance = &finance;
		transaction.commands = &commands;
		loop = {{.tick_counter = 1, .size_x = 64, .climate = 0, .extra = 0, .ambient = false, .ambient_callback = false}, &tiles};
		scheduler.pool = []() noexcept { return OpenTTDVehiclePoolCursor{nullptr, 0, &empty_extent}; };
		scheduler.vehicle = &vehicles;
		scheduler.shared = &shared;
		scheduler.with_loading_timer = [](const OpenTTDVehicleSchedulerTables *, void (*)(const OpenTTDVehicleSchedulerTables *)) noexcept {};
		scheduler.reset_accumulators = Nothing;
		tables = {};
		tables.scheduler = &scheduler;
		tables.pending = &pending;
		tables.transaction = &transaction;
		tables.group = &group;
		towns.environment.editor = []() noexcept { return true; };
		stations.environment.editor = []() noexcept { return true; };
		companies.shared = &shared;
		companies.finance = &finance;
		companies.competitor_fired = []() noexcept { return false; };
		finance.company_finances = [](uint8_t) noexcept -> CompanyFinances * { return nullptr; };
		companies.company_next = [](uint32_t) noexcept { return OpenTTDCompanyRef{nullptr, UINT32_MAX}; };
		industries.shared = &shared;
		industries.next_tick = [](uint32_t) noexcept { return OpenTTDIndustryEntry{}; };
		world.industry = &industries;
		clock.shared = &shared;
		clock.minutes_per_calendar_year = []() noexcept -> uint16_t { return 0; };
		clock.using_wallclock_units = [](bool) noexcept { return true; };
		services.clock = &clock;
		services.check_caches = Nothing;
		services.company_begin = FrameNothing;
		services.company_restore = FrameNothing;
		services.console_exception = [](OpenTTDSessionFrame *) noexcept -> OpenTTDConsoleException * { return nullptr; };
		services.failed = [] (OpenTTDSessionFrame *) noexcept { return false; };
		services.tick_timer = [] (OpenTTDSessionFrame *) noexcept { return true; };
		services.scripts_enabled = []() noexcept { return false; };
		services.company = &companies;
		services.shared = &shared;
		services.schedule = &schedule;
		services.world = []() noexcept -> const OpenTTDWorldServices * { return &active->world; };
		finance.networking = []() noexcept { return true; };
		companies.network_server = []() noexcept { return false; };
		world.modal = []() noexcept { return false; };
		services.state_frame_begin = [](OpenTTDSessionFrameStorage *p) noexcept { return reinterpret_cast<OpenTTDSessionFrame *>(p); };
		services.state_frame_end = [](OpenTTDSessionFrame *, OpenTTDSessionFrame *) noexcept { return true; };
		services.state_begin = FrameNothing;
		services.tile_loop_begin = [](OpenTTDSessionFrame *) noexcept -> const OpenTTDGenerationLoop * { active->in_loop = true; return &active->loop; };
		services.tile_loop_end = FrameNothing;
		services.vehicle_tables = [](OpenTTDSessionFrame *, bool) noexcept { return active->tables; };
		services.vehicle_cost = [](OpenTTDSessionFrame *) noexcept -> OpenTTDCommandCost * { return nullptr; };
		services.vehicle_end = FrameNothing;
		services.landscape_begin = [](OpenTTDSessionFrame *) noexcept {
			active->in_loop = true;
			if (active->skip_animation) {
				active->animation_queue.emplace(true);
				openttd_rust_animation_initialize();
				active->animation_scope.emplace(active->animation);
			}
		};
		services.landscape_end = [](OpenTTDSessionFrame *) noexcept {
			active->animation_scope.reset();
			active->animation_queue.reset();
			active->skip_animation = false;
		};
		services.road_begin = [](OpenTTDSessionFrame *) noexcept -> const OpenTTDRoadCommandContext * { return nullptr; };
		services.road_end = FrameNothing;
		services.town = &towns;
		services.trees = &trees;
		services.station = &stations;
		services.links = &links;
		services.visits = &visits;
		services.industry_builder = builder;
		services.schedule_measurement = [](OpenTTDSessionFrame *) noexcept -> void * { return nullptr; };
		services.schedule_end = FrameNothing;
		services.window_tick = Nothing;
		services.news = Nothing;
		services.assert_local_company = Nothing;
		world.landscape = &storage;
		storage.shared = &shared;
		storage.assert_condition = [](bool valid) noexcept { if (!valid) std::abort(); };
		shared.storage_discard = [](void *) noexcept {};
	}
	OpenTTDLandscapeGenerationServices storage{};
	~SessionFixture() { openttd_rust_industry_builder_destroy(builder); active = previous; }
	bool Run(GameMode mode = GM_EDITOR, PauseModes paused = {})
	{
		// Nested tile probes enter a fresh session while their outer fixture is
		// already in storage's game-loop mode. Restore that outer mode on return.
		const bool outer_loop = previous != nullptr && previous->in_loop;
		if (outer_loop) openttd_rust_storage_switch_mode(PSM_LEAVE_GAMELOOP, true, storage.assert_condition, shared.storage_discard);
		skip_animation = !animate && mode != GM_EDITOR;
		const auto clocks = *openttd_rust_clock_storage();
		if (mode != GM_EDITOR) { TimerGameEconomy::date_fract = 0; TimerGameCalendar::date_fract = 0; }
		const auto saved_cursor = *openttd_rust_tile_loop_cursor();
		if (saved_cursor == 0) *openttd_rust_tile_loop_cursor() = 1;
		const auto saved_company_tick = *openttd_rust_company_tick();
		const auto saved_mode = _game_mode;
		const auto saved_pause = _pause_mode;
		_game_mode = mode;
		_pause_mode = paused;
		const bool result = openttd_rust_session_state_loop(&services, nullptr);
		// An injected phase failure intentionally leaves the production storage
		// mode entered; the narrow fixture must not leak it into its next case.
		openttd_rust_storage_switch_mode(PSM_LEAVE_GAMELOOP, true, storage.assert_condition, shared.storage_discard);
		in_loop = false;
		if (outer_loop) openttd_rust_storage_switch_mode(PSM_ENTER_GAMELOOP, false, previous->storage.assert_condition, previous->shared.storage_discard);
		*openttd_rust_clock_storage() = clocks;
		_game_mode = saved_mode;
		_pause_mode = saved_pause;
		if (loop.services == &tiles || saved_cursor == 0) *openttd_rust_tile_loop_cursor() = saved_cursor;
		/* The live-mode company tick advances the process-wide OnTick_Companies cursor even with the empty company table. */
		if (services.company == &companies) *openttd_rust_company_tick() = saved_company_tick;
		return result;
	}
};
inline void FixtureStationTick(const OpenTTDStationServices &services, const OpenTTDStationLinks &links, const OpenTTDVehicleVisits &visits)
{
	SessionFixture fixture;
	auto station = services;
	fixture.services.station = &station;
	fixture.services.links = &links;
	fixture.services.visits = &visits;
	if (!fixture.Run(_game_mode)) std::abort();
}
#endif
