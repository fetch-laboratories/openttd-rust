/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file generation-console.hpp Early semantic snapshots and unsaveable cancellation. */
#include "heightmap.h"
#include "landscape.h"
#include "openttd.h"
#include "fios.h"
#include "core/random_func.hpp"
extern "C" void __real__Z26SetGeneratingWorldProgress16GenWorldProgressj(GenWorldProgress, uint);
extern "C" void __real__Z31IncreaseGeneratingWorldProgress16GenWorldProgress(GenWorldProgress);
#ifdef WITH_RUST
#include "rust/world_generation_ffi.h"
#include "rust/landscape_generation_frame.hpp"
#define GENERATION_REAL_ABORT __real_openttd_rust_world_set_abort_callback
#define GENERATION_WRAP_ABORT __wrap_openttd_rust_world_set_abort_callback
static void GenerationSetAbort(GWAbortProc *proc) { openttd_rust_world_set_abort_callback(proc); }
#else
#define GENERATION_REAL_ABORT __real__Z29GenerateWorldSetAbortCallbackPFvvE
#define GENERATION_WRAP_ABORT __wrap__Z29GenerateWorldSetAbortCallbackPFvvE
static void GenerationSetAbort(GWAbortProc *proc) { GenerateWorldSetAbortCallback(proc); }
#endif
extern "C" void GENERATION_REAL_ABORT(GWAbortProc *);
static bool GenerationLandscape(uint8_t mode)
{
#ifdef WITH_RUST
	OpenTTDLandscapeGenerationCall call;
	bool generated = openttd_rust_generate_landscape(&GetRustLandscapeGenerationServices(), &call.frame, &call.heightmap, call.settings, mode);
	if (call.frame.cancellation) std::rethrow_exception(call.frame.cancellation);
	return generated;
#else
	return GenerateLandscape(mode);
#endif
}
static int generation_cancel = -1, generation_events = 0, generation_cleanup = 0;
static GWAbortProc *generation_abort = nullptr;
static bool generation_heightmap = false;
static uint generation_landscape_steps = 0, generation_river_steps = 0;
static void GenerationSnapshot(std::string_view name);
static void GenerationCleanup() { generation_cleanup++; generation_abort(); }
extern "C" void GENERATION_WRAP_ABORT(GWAbortProc *callback)
{
	generation_abort = callback;
	GENERATION_REAL_ABORT(generation_cancel >= 0 && callback != nullptr ? GenerationCleanup : callback);
}
#include "progress.h"
#include "network/network.h"
#include "newgrf_storage.h"
#include "game/game.hpp"
#include "game/game_config.hpp"
static bool generation_lifecycle = false, generation_rocky = false;
static int generation_target = -1, generation_done = 0, generation_outer_cleanup = 0, generation_outer_abort = 0;
static uint generation_warmup = 0, generation_script = 0;
static Randomizer generation_rough_previous;
static PersistentStorageArray<int32_t, 16> *generation_storage = nullptr;
extern "C" void __real__Z16SetModalProgressb(bool);
extern "C" void __wrap__Z16SetModalProgressb(bool value)
{
	if (generation_lifecycle && !value) generation_outer_cleanup++;
	__real__Z16SetModalProgressb(value);
}
static void GenerationOuterAbort() { generation_outer_abort++; }
static void GenerationDone() { generation_done++; }
static void GenerationOuterProgress(GenWorldProgress stage, bool set)
{
	if (!generation_lifecycle) return;
	if (stage == GWP_RUNTILELOOP && !set) generation_warmup++;
	if (stage == GWP_RUNSCRIPT && !set) generation_script++;
	if (stage == GWP_ROUGH_ROCKY) {
		if (set) { generation_rough_previous = _random; generation_rocky = false; }
		else {
			Randomizer next = generation_rough_previous;
			next.Next();
			if ((_random.state[0] != generation_rough_previous.state[0] || _random.state[1] != generation_rough_previous.state[1]) && (_random.state[0] != next.state[0] || _random.state[1] != next.state[1])) generation_rocky = true;
			generation_rough_previous = _random;
		}
	}
	bool cancel = (generation_target == 0 && stage == GWP_MAP_INIT && set) ||
		(generation_target == 1 && stage == GWP_ROUGH_ROCKY && !set && !generation_rocky) ||
		(generation_target == 2 && stage == GWP_ROUGH_ROCKY && !set && generation_rocky) ||
		(generation_target == 3 && stage == GWP_RUNTILELOOP && !set) ||
		(generation_target == 4 && stage == GWP_GAME_START && !set) ||
		(generation_target == 5 && stage == GWP_MAP_INIT && !set);
	if (!cancel) return;
	generation_storage->StoreValue(0, 33);
	generation_target = -1;
	GenerationSetAbort(GenerationOuterAbort);
	HandleGeneratingWorldAbortion();
}
static void GenerationLifecycle()
{
	extern bool _in_modal_progress;
	bool saved_exit = _exit_game;
	AutoRestoreBackup<bool> modal(_in_modal_progress, false), dedicated(_network_dedicated, true);
	_exit_game = false;
	AutoRestoreBackup<SwitchMode> switching(_switch_mode, _switch_mode);
	generation_lifecycle = true;
	for (int kind = 0; kind < 16; kind++) {
		_game_mode = kind < 6 || (kind >= 12 && kind <= 14) ? GM_NORMAL : GM_EDITOR;
		_network_dedicated = kind != 14;
		_exit_game = false;
		_settings_game.game_creation.generation_seed = 12345;
		_settings_game.game_creation.se_flat_world_height = 41;
		_settings_game.game_creation.heightmap_height = 63;
		_settings_game.game_creation.land_generator = kind == 10 ? LG_ORIGINAL : LG_TERRAGENESIS;
		_settings_game.construction.map_height_limit = 0;
		_settings_game.construction.freeform_edges = kind % 2 == 0;
		_settings_game.difficulty.number_towns = 1;
		_settings_game.difficulty.industry_density = ID_FUND_ONLY;
		GenWorldMode mode = kind < 6 || kind == 10 || (kind >= 12 && kind <= 14) ? GWM_NEWGAME : kind == 8 ? GWM_HEIGHTMAP : kind == 9 ? GWM_RANDOM : GWM_EMPTY;
		_file_to_saveload.name = "generation-missing.bmp";
		_file_to_saveload.ftype.detailed = DFT_HEIGHTMAP_BMP;
		generation_target = kind < 6 ? kind : kind == 14 ? 0 : -1;
		generation_done = generation_outer_cleanup = generation_outer_abort = 0;
		generation_warmup = generation_script = 0;
		PersistentStorageArray<int32_t, 16> storage;
		generation_storage = &storage;
		_current_company = _local_company = COMPANY_SPECTATOR;
		if (kind == 12 || kind == 13) GameConfig::GetConfig(GameConfig::SSS_FORCE_GAME)->Change(kind == 12 ? "GenerationSleep" : "GenerationCap");
		GenerateWorldSetCallback(GenerationDone);
		GenerateWorld(mode, 64, 64, false);
		fmt::print(stderr, "GENERATION lifecycle {} {} {} {} {} {} {} {} {} {} {} {} {} {}\n", kind, mode, _settings_game.construction.map_height_limit, generation_done, generation_outer_cleanup, generation_outer_abort, _current_company.base(), _local_company.base(), _generating_world, storage.GetValue(0), _exit_game.load(), generation_warmup, generation_script, _game_mode);
		GenerationSnapshot("lifecycle");
		if (kind == 12 || kind == 13) { Game::Uninitialize(true); GameConfig::GetConfig(GameConfig::SSS_FORCE_GAME)->Change(std::nullopt); }
	}
	generation_lifecycle = false; generation_target = -1; generation_storage = nullptr;
	_exit_game = saved_exit;
	_game_mode = GM_NORMAL;
	/* The enclosing new-game callback still owes its final progress increment. */
	SetModalProgress(true);
	SetGeneratingWorldProgress(GWP_GAME_START, 1);
	_current_company = _local_company = COMPANY_SPECTATOR;
}

static void GenerationProgress(GenWorldProgress stage)
{
	if (generation_cancel >= 0 && stage == GWP_LANDSCAPE && generation_events++ == generation_cancel) HandleGeneratingWorldAbortion();
}
extern "C" void __wrap__Z26SetGeneratingWorldProgress16GenWorldProgressj(GenWorldProgress stage, uint total)
{
	if (generation_heightmap) {
		fmt::print(stderr, "GENERATION progress set {} {}\n", stage, total);
		if (stage == GWP_RIVER) GenerationSnapshot("heightmap-climate");
	}
	GenerationOuterProgress(stage, true);
	GenerationProgress(stage); __real__Z26SetGeneratingWorldProgress16GenWorldProgressj(stage, total);
}
extern "C" void __wrap__Z31IncreaseGeneratingWorldProgress16GenWorldProgress(GenWorldProgress stage)
{
	if (generation_heightmap) {
		if (stage == GWP_LANDSCAPE) {
			++generation_landscape_steps;
			GenerationSnapshot("heightmap-progress");
		}
		if (stage == GWP_RIVER) ++generation_river_steps;
	}
	GenerationOuterProgress(stage, false);
	GenerationProgress(stage); __real__Z31IncreaseGeneratingWorldProgress16GenWorldProgress(stage);
}
static void GenerationCancellation()
{
	AutoRestoreBackup<uint8_t> generator(_settings_game.game_creation.land_generator, LG_TERRAGENESIS);
	AutoRestoreBackup<SwitchMode> mode(_switch_mode, _switch_mode);
	for (generation_cancel = 0; generation_cancel < 3; generation_cancel++) {
		generation_events = generation_cleanup = 0; bool caught = false;
		try { GenerationLandscape(GWM_NEWGAME); } catch (AbortGenerateWorldSignal &) { caught = true; }
		fmt::print(stderr, "GENERATION cancel {} {} {} {}\n", generation_cancel, caught, generation_events, generation_cleanup);
		GenerationSetAbort(nullptr);
	}
	generation_cancel = -1;
}
static std::array<uint32_t, 10> GenerationTile(Tile t)
{
	return {t.type(), t.height(), t.m2(), t.m1(), t.m3(), t.m4(), t.m5(), t.m6(), t.m7(), t.m8()};
}
static void GenerationSnapshot(std::string_view name)
{
	uint64_t map = 14695981039346656037ULL;
	for (TileIndex tile : Map::Iterate()) for (uint32_t value : GenerationTile(Tile{tile})) map = (map ^ value) * 1099511628211ULL;
	fmt::print(stderr, "GENERATION map {} {} {} {} {} {}\n", name, map, _random.state[0], _random.state[1], _settings_game.construction.freeform_edges, _settings_game.game_creation.snow_line_height);
}
static bool GenerationScenario(std::span<std::string_view> args)
{
	if (args.size() != 2) return false;
	GenerationSnapshot("early");
	if (args[1] == "snapshot") return true;
	if (args[1] == "lifecycle") { GenerationLifecycle(); return true; }
	/* Nested probes must not replace the enclosing new-game progress stage. */
	extern bool _in_modal_progress; AutoRestoreBackup<bool> modal(_in_modal_progress, false);
	SavedRandomSeeds seeds; SaveRandomSeeds(&seeds);
	AutoRestoreBackup<uint8_t> rotation(_settings_game.game_creation.heightmap_rotation, _settings_game.game_creation.heightmap_rotation), height(_settings_game.game_creation.heightmap_height, 31);
	AutoRestoreBackup<bool> freeform(_settings_game.construction.freeform_edges, _settings_game.construction.freeform_edges);
	AutoRestoreBackup<uint8_t> limit(_settings_game.construction.map_height_limit, 32);
	std::vector<std::array<uint32_t, 10>> map; for (TileIndex tile : Map::Iterate()) map.push_back(GenerationTile(Tile{tile}));
	for (const char *aspect : {"wide", "tall"}) for (uint8_t rotate : {HM_COUNTER_CLOCKWISE, HM_CLOCKWISE}) for (bool edges : {false, true}) {
		_settings_game.game_creation.heightmap_rotation = rotate; _settings_game.construction.freeform_edges = edges;
		std::string filename = std::string(std::getenv("HOME")) + "/" + aspect + ".bmp";
		bool loaded = LoadHeightmap(DFT_HEIGHTMAP_BMP, filename);
		fmt::print(stderr, "GENERATION heightmap {} {} {} {}\n", aspect, rotate, edges, loaded);
		GenerationSnapshot("heightmap");
	}
	bool loaded = LoadHeightmap(DFT_HEIGHTMAP_BMP, "generation-missing.bmp");
	{
		/* Successful LoadHeightmap alone misses GenerateLandscape's continuation:
			* slope repair, watering, climate policy and the complete river pass. */
		AutoRestoreBackup<std::string> filename(_file_to_saveload.name, std::string(std::getenv("HOME")) + "/wide.bmp");
		AutoRestoreBackup<DetailedFileType> type(_file_to_saveload.ftype.detailed, DFT_HEIGHTMAP_BMP);
		AutoRestoreBackup<LandscapeType> climate(_settings_game.game_creation.landscape, LandscapeType::Temperate);
		AutoRestoreBackup<uint8_t> rivers(_settings_game.game_creation.amount_of_rivers, 2), length(_settings_game.game_creation.min_river_length, 4);
		AutoRestoreBackup<uint8_t> snow(_settings_game.game_creation.snow_line_height, _settings_game.game_creation.snow_line_height);
		for (LandscapeType landscape : {LandscapeType::Temperate, LandscapeType::Arctic, LandscapeType::Tropic, LandscapeType::Toyland}) {
			_settings_game.game_creation.landscape = landscape;
			InitializeLandscape();
			generation_landscape_steps = generation_river_steps = 0;
			generation_heightmap = true;
			bool generated = GenerationLandscape(GWM_HEIGHTMAP);
			generation_heightmap = false;
			fmt::print(stderr, "GENERATION success {} {} {} {}\n", landscape, generated, generation_landscape_steps, generation_river_steps);
			GenerationSnapshot("heightmap-generated");
		}
	}
	{
		AutoRestoreBackup<std::string> filename(_file_to_saveload.name, "generation-missing.bmp");
		AutoRestoreBackup<DetailedFileType> type(_file_to_saveload.ftype.detailed, DFT_HEIGHTMAP_BMP);
		fmt::print(stderr, "GENERATION missing {} {}\n", loaded, GenerationLandscape(GWM_HEIGHTMAP));
	}
#ifdef WITH_RUST
	openttd_rust_flat_empty_world(&GetRustLandscapeGenerationServices(), 7, _settings_game.construction.freeform_edges, _settings_game.construction.map_height_limit);
#else
	FlatEmptyWorld(7);
#endif
	GenerationSnapshot("flat");
	for (TileIndex index : Map::Iterate()) {
		Tile t{index}; auto &v = map[index.base()];
		t.type() = v[0]; t.height() = v[1]; t.m2() = v[2]; t.m1() = v[3]; t.m3() = v[4]; t.m4() = v[5]; t.m5() = v[6]; t.m6() = v[7]; t.m7() = v[8]; t.m8() = v[9];
	}
	GenerationCancellation(); RestoreRandomSeeds(seeds);
	std::fflush(stderr); return true;
}
