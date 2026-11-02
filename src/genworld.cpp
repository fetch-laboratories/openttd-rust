/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file genworld.cpp Functions to generate a map. */

#include "stdafx.h"
#include "landscape.h"
#include "company_func.h"
#include "town_cmd.h"
#include "signs_cmd.h"
#include "3rdparty/nlohmann/json.hpp"
#include "strings_func.h"
#include "genworld.h"
#include "gfxinit.h"
#include "window_func.h"
#include "network/network.h"
#include "heightmap.h"
#include "viewport_func.h"
#include "timer/timer_game_calendar.h"
#include "timer/timer_game_tick.h"
#include "engine_func.h"
#include "water.h"
#include "video/video_driver.hpp"
#include "tilehighlight_func.h"
#include "saveload/saveload.h"
#include "void_map.h"
#include "town.h"
#include "newgrf.h"
#include "newgrf_house.h"
#include "core/random_func.hpp"
#include "core/backup_type.hpp"
#include "progress.h"
#include "error.h"
#include "game/game.hpp"
#include "game/game_instance.hpp"
#include "newgrf_railtype.h"
#include "newgrf_roadtype.h"
#include "string_func.h"
#include "thread.h"
#include "tgp.h"

#include "table/strings.h"

#include "safeguards.h"


#include "rust/world_generation_ffi.h"
#include "rust/session_native.hpp"
#include "industry.h"
#include "rust/landscape_generation_frame.hpp"
#include "rust/industry_generation_adapter.hpp"
#include "rust/services_object.h"
#include "rust/object_resolver_ffi.h"
#include "rust/company_services.h"
#include "rust/company_adapter.h"
#include "rust/settings_services.h"
#include "rust/environment_services.h"
#include "rust/presentation_services.h"
#include "rust/newgrf_storage_ffi.h"
#include "rust/services_map.h"
#include "rust/service_table_check.hpp"
#include "rust/town_data_ffi.h"
#include "rust/services_assert.h"
#include "abi-visit-OpenTTDTownDataServices.hpp"
#include "abi-visit-OpenTTDWorldServices.hpp"

static_assert(GWM_EMPTY == 1 && GWP_MAP_INIT == 0 && GWP_ROUGH_ROCKY == 3 && GWP_OBJECT == 7 && GWP_GAME_INIT == 9 && GWP_RUNTILELOOP == 10 && GWP_RUNSCRIPT == 11 && GWP_GAME_START == 12);
static_assert(GM_MENU == 0 && GM_EDITOR == 2 && SM_EDITOR == 4 && SM_MENU == 6);
static_assert(MAP_HEIGHT_LIMIT_AUTO_MINIMUM == 30 && MAP_HEIGHT_LIMIT_AUTO_CEILING_ROOM == 15 && DEF_SNOWLINE_HEIGHT == 10 && MAX_MAP_HEIGHT_LIMIT == 255);
static_assert(GENERATE_NEW_SEED == UINT32_MAX && PSM_ENTER_GAMELOOP == 0 && PSM_LEAVE_GAMELOOP == 1);

void StartupCompanies();
void InitializeGame(uint size_x, uint size_y, bool reset_date, bool reset_settings);
std::exception_ptr TakeTreeGenerationException() noexcept;

struct RustWorldTownScratch { OpenTTDTownFoundingFrame frame; RustRoadCommandContext road; };
struct RustWorldObjectScratch { OpenTTDObjectGenerationFrame frame; CommandCost out, tmp, aux; };
struct OpenTTDWorldFrame {
	std::exception_ptr cancellation;
	std::optional<Backup<CompanyID>> company;
	std::optional<OpenTTDLandscapeGenerationCall> landscape;
	std::optional<RustWorldTownScratch> town;
	std::optional<OpenTTDIndustryGenerationContext> industry;
	std::optional<RustWorldObjectScratch> objects;
	std::optional<RustGenerationLoop> loop;
	std::optional<OpenTTDSessionFrame> session;
};
/* Game::StartNew/GameLoop, completion callbacks, sprite loading, InitializeGame,
	* mode switching and SaveOrLoad may throw. Their native frames outlive Rust. */
template <typename F>
static bool WorldException(OpenTTDWorldFrame *frame, F &&operation) noexcept
{
	try { operation(); }
	catch (...) { frame->cancellation = std::current_exception(); return false; }
	return true;
}
static uint8_t WorldExceptionKind(OpenTTDWorldFrame *frame) noexcept
{
	if (!frame->cancellation) return 0;
	try { std::rethrow_exception(frame->cancellation); }
	catch (AbortGenerateWorldSignal &) { return 1; }
	catch (...) { return 2; }
}
const OpenTTDWorldServices &GetRustWorldServices() noexcept
{
	static const OpenTTDWorldServices services{
		.landscape = &GetRustLandscapeGenerationServices(),
		.company = &GetRustClockCompanyServices(),
		.town = &GetRustTownFoundingServices(),
		.industry = &GetIndustryRustServices(),
		.industry_generation = &GetIndustryGenerationServices(),
		.objects = &GetRustObjectServices(),
		.engines = &EngineServices(),
		.disasters = &DisasterServices(),
		.modal = RustHasModalProgress,
		.set_modal = [](bool value) noexcept { SetModalProgress(value); },
		.initialize = [](OpenTTDWorldSetupFrame *f, uint32_t x, uint32_t y, bool reset) noexcept { try { InitializeGame(x, y, true, reset); } catch (...) { f->cancellation = std::current_exception(); return false; } return true; },
		.prepare_progress = []() noexcept { PrepareGenerateWorldProgress(); },
		.settings = []() noexcept { return OpenTTDWorldSettings{RustMapHeightLimit(), _settings_game.game_creation.se_flat_world_height, _settings_game.game_creation.heightmap_height, _settings_game.game_creation.land_generator}; },
		.height_limit = RustSetMapHeightLimit,
		.session = &GetRustSessionServices(),
		.session_frame = [](OpenTTDWorldFrame *f) noexcept { return &f->session.emplace(); },
		.session_end = [](OpenTTDWorldFrame *f) noexcept { f->cancellation = f->session->exception; f->session.reset(); return !f->cancellation; },
		.seed = RustGenerationSeed,
		.set_seed = RustSetGenerationSeed,
		.interactive_random = []() noexcept { return InteractiveRandom(); },
		.random_seed = [](uint32_t value) noexcept { _random.SetSeed(value); },
		.load_sprites = [](OpenTTDWorldSetupFrame *f) noexcept { try { GfxLoadSprites(); } catch (...) { f->cancellation = std::current_exception(); return false; } return true; },
		.string_widths = []() noexcept { LoadStringWidthTable(); },
		.reset_windows = RustResetWindowSystem,
		.setup_windows = RustSetupColoursAndInitialWindow,
		.unshow_error = []() noexcept { UnshowCriticalError(); },
		.close_windows = []() noexcept { CloseAllNonVitalWindows(); },
		.hide_windows = []() noexcept { HideVitalWindows(); },
		.show_progress = []() noexcept { ShowGenerateWorldProgress(); },
		.show_windows = []() noexcept { ShowVitalWindows(); },
		.reset_object = []() noexcept { ResetObjectToPlace(); },
		.show_grf_error = []() noexcept { ShowNewGRFError(); },
		.randomizers = []() noexcept { ScriptObject::InitializeRandomizers(); },
		.startup_companies = []() noexcept { StartupCompanies(); },
		.place_cursor = []() noexcept { SetObjectToPlace(SPR_CURSOR_ZZZ, PAL_NONE, HT_NONE, WC_MAIN_WINDOW, 0); },
		.scroll = [](uint32_t tile) noexcept { ScrollMainWindowToTile(TileIndex(tile), true); },
		.busy = [](bool value) noexcept { SetMouseCursorBusy(value); },
		.close_progress = []() noexcept { CloseWindowByClass(WC_MODAL_PROGRESS); },
		.show_error = []() noexcept { ShowFirstError(); },
		.set_progress = [](OpenTTDWorldFrame *f, GenWorldProgress p, uint32_t total) noexcept { return RunGenerationProgress(f, [=] { RustSetGeneratingWorldProgress(p, total); }); },
		.increase_progress = [](OpenTTDWorldFrame *f, GenWorldProgress p) noexcept { return RunGenerationProgress(f, [=] { RustIncreaseGeneratingWorldProgress(p); }); },
		.freeform = OpenTTDFreeformEdges,
		.count_land = RustCountLandTiles,
		.landscape_begin = [](OpenTTDWorldFrame *f) noexcept { auto &c = f->landscape.emplace(); return OpenTTDWorldLandscape{&c.frame, &c.heightmap.view, c.settings}; },
		.landscape_end = [](OpenTTDWorldFrame *f) noexcept { f->cancellation = f->landscape->frame.cancellation; f->landscape.reset(); return !f->cancellation; },
		.town_begin = [](OpenTTDWorldFrame *f) noexcept { auto &t = f->town.emplace(); return OpenTTDWorldTown{&t.frame, &t.frame.names, &t.road.view, _settings_game.economy.town_layout}; },
		.town_end = [](OpenTTDWorldFrame *f) noexcept { f->cancellation = f->town->frame.cancellation; f->town.reset(); return !f->cancellation; },
		.industry_builder = RustIndustryBuilderState,
		.industry_begin = [](OpenTTDWorldFrame *f) noexcept { return &f->industry.emplace(); },
		.industry_end = [](OpenTTDWorldFrame *f) noexcept { f->cancellation = f->industry->cancellation; f->industry.reset(); },
		.object_begin = [](OpenTTDWorldFrame *f) noexcept { auto &o = f->objects.emplace(); return OpenTTDWorldObject{&o.frame, RustCommandCost(&o.out), RustCommandCost(&o.tmp), RustCommandCost(&o.aux)}; },
		.object_end = [](OpenTTDWorldFrame *f) noexcept { f->cancellation = f->objects->frame.cancellation; f->objects.reset(); },
		.tree_settings = TreeGenerateSettings,
		.tree_end = [](OpenTTDWorldFrame *f) noexcept { f->cancellation = TakeTreeGenerationException(); },
		.loop_begin = [](OpenTTDWorldFrame *f) noexcept -> const OpenTTDGenerationLoop * {
			auto &loop = f->loop.emplace();
			assert(*openttd_rust_tile_loop_cursor() != 0);
			return &loop.view;
		},
		.loop_end = [](OpenTTDWorldFrame *f) noexcept { f->loop.reset(); },
		.advance_tick = []() noexcept { TimerGameTick::counter++; },
		.script_start = [](OpenTTDWorldFrame *f) noexcept { return WorldException(f, [] { Game::StartNew(); }); },
		.script_loop = [](OpenTTDWorldFrame *f) noexcept { return WorldException(f, [] { Game::GameLoop(); }); },
		.script_exists = []() noexcept { return Game::GetInstance() != nullptr; },
		.script_sleeping = []() noexcept { return Game::GetInstance()->IsSleeping(); },
		.trash_company = [](OpenTTDWorldFrame *f) noexcept { f->company->Trash(); },
		.company_valid = [](OpenTTDWorldFrame *f) noexcept { return f->company->IsValid(); },
		.restore_company = [](OpenTTDWorldFrame *f) noexcept { f->company->Restore(); },
		.callback = [](OpenTTDWorldFrame *f, GWDoneProc *proc) noexcept { return WorldException(f, [=] { proc(); }); },
		.exception_kind = WorldExceptionKind,
		.dedicated = RustNetworkDedicated,
		.exit_game = RustExitGame,
		.set_exit = RustSetExitGame,
		.debug_start = []() noexcept { Debug(net, 3, "Generating map, please wait..."); },
		.debug_done = []() noexcept { Debug(net, 3, "Map generated, starting game"); },
		.debug_abort = []() noexcept { Debug(net, 0, "Generating map failed; closing server"); },
		.debug_map = RustDebugNewMap,
		.desync_level = RustDesyncLevel,
		.save = [](OpenTTDWorldFrame *f) noexcept { return WorldException(f, RustSaveDesyncGame); },
	};
	return services;
}

static const OpenTTDTownDataServices &GetRustTownDataServices() noexcept;
bool CheckAbi_genworld() { return AbiServicesComplete(GetRustWorldServices()) && AbiServicesComplete(GetRustTownDataServices()); }

static_assert(sizeof(OpenTTDWorldFrame) <= sizeof(OpenTTDWorldFrameStorage));
static_assert(alignof(OpenTTDWorldFrame) <= alignof(OpenTTDWorldFrameStorage));
OpenTTDWorldFrame *RustWorldFrameBegin(OpenTTDWorldFrameStorage *storage) noexcept
{
	auto *frame = new (storage) OpenTTDWorldFrame;
	frame->company.emplace(_current_company, OWNER_NONE);
	return frame;
}
bool RustWorldFrameEnd(OpenTTDSessionFrame *parent, OpenTTDWorldFrame *frame, uint8_t result) noexcept
{
	if (result == 2) parent->exception = frame->cancellation;
	std::destroy_at(frame);
	return result != 2;
}
void GenerateWorld(GenWorldMode mode, uint size_x, uint size_y, bool reset_settings, GWDoneProc *const *callback)
{
	OpenTTDSessionFrame frame;
	openttd_rust_session_generate(&GetRustSessionServices(), &frame, mode, size_x, size_y, reset_settings, callback);
	frame.Rethrow();
}
void AbortGeneratingWorld() { openttd_rust_world_request_abort(); }
bool IsGeneratingWorldAborted() { return openttd_rust_world_aborted(&GetRustWorldServices()); }
void HandleGeneratingWorldAbortion()
{
	OpenTTDWorldFrame frame;
	if (!openttd_rust_world_handle_abort(&GetRustWorldServices(), &frame)) std::rethrow_exception(frame.cancellation);
	throw AbortGenerateWorldSignal();
}

/** Parsed document and current feature name for native command and presentation calls. */
struct OpenTTDTownDataFrame {
	nlohmann::json town_data;
	std::string name;
};
static_assert(TSZ_SMALL == 0 && to_underlying(TownExpandMode::Buildings) == 0 && to_underlying(TownExpandMode::Roads) == 1);
static bool TownDataLoad(OpenTTDTownDataFrame *frame) noexcept
{
	size_t filesize;
	auto f = FioFOpenFile(_file_to_saveload.name, "rb", HEIGHTMAP_DIR, &filesize);
	if (!f.has_value()) return false;
	std::string text(filesize, '\0');
	size_t len = fread(text.data(), filesize, 1, *f);
	f.reset();
	if (len != 1) return false;
	try {
		frame->town_data = nlohmann::json::parse(text);
	} catch (nlohmann::json::exception &) {
		return false;
	}
	return frame->town_data.is_array();
}
/* The original's type checks, in order, then its get_to conversions. */
static uint8_t TownDataFeature(OpenTTDTownDataFrame *frame, size_t index, OpenTTDTownDataFeature *out) noexcept
{
	if (index >= frame->town_data.size()) return OPENTTD_TOWN_DATA_END;
	auto &feature = frame->town_data[index];
	if (!feature.is_object()) return OPENTTD_TOWN_DATA_MALFORMED;
	if (!feature.contains("name") || !feature.at("name").is_string()) return OPENTTD_TOWN_DATA_MALFORMED;
	feature.at("name").get_to(frame->name);
	if (!feature.contains("population") || !feature.at("population").is_number() ||
			!feature.contains("city") || !feature.at("city").is_boolean() ||
			!feature.contains("x") || !feature.at("x").is_number() ||
			!feature.contains("y") || !feature.at("y").is_number()) {
		return OPENTTD_TOWN_DATA_NAMED_MALFORMED;
	}
	feature.at("population").get_to(out->population);
	feature.at("city").get_to(out->city);
	feature.at("x").get_to(out->x);
	feature.at("y").get_to(out->y);
	return OPENTTD_TOWN_DATA_OK;
}
static const OpenTTDTownDataServices &GetRustTownDataServices() noexcept
{
	static const OpenTTDTownDataServices services{
		.shared = &GetRustSharedServices(),
		.founding = &GetRustTownFoundingServices(),
		.strings = {
			.load_failed = STR_TOWN_DATA_ERROR_LOAD_FAILED, .json_format = STR_TOWN_DATA_ERROR_JSON_FORMATTED_INCORRECTLY,
			.town_format = STR_TOWN_DATA_ERROR_TOWN_FORMATTED_INCORRECTLY, .bad_coordinate = STR_TOWN_DATA_ERROR_BAD_COORDINATE,
			.failed_towns = STR_TOWN_DATA_ERROR_FAILED_TO_FOUND_TOWN,
		},
		.load = TownDataLoad,
		.feature = TownDataFeature,
		.named_error = [](uint32_t summary, uint32_t detail, const OpenTTDTownDataFrame *frame) noexcept { ShowErrorMessage(GetEncodedString(summary), GetEncodedString(detail, frame->name), WL_ERROR); },
		.failed_warning = [](uint32_t detail, uint32_t count) noexcept { ShowErrorMessage(GetEncodedString(detail, count), {}, WL_WARNING); },
		.found = [](const OpenTTDTownDataFrame *frame, uint32_t tile, TownSize size, bool city, TownLayout layout, bool random, uint32_t parts) noexcept {
			return std::get<TownID>(Command<CMD_FOUND_TOWN>::Do(DoCommandFlag::Execute, TileIndex(tile), size, city, layout, random, parts, frame->name)).base();
		},
		.place_sign = [](uint32_t tile, const OpenTTDTownDataFrame *frame) noexcept { Command<CMD_PLACE_SIGN>::Post(TileIndex(tile), frame->name); },
		.expand = [](uint16_t town, uint32_t amount, uint8_t modes) noexcept { Command<CMD_EXPAND_TOWN>::Post(TownID(town), amount, TownExpandModes(modes)); },
	};
	return services;
}

/**
 * Load town data from _file_to_saveload, place towns at the appropriate locations, and expand them to their target populations.
 */
void LoadTownData()
{
	OpenTTDTownDataFrame frame;
	openttd_rust_load_town_data(&GetRustTownDataServices(), &frame, _settings_game.game_creation.heightmap_rotation);
}
