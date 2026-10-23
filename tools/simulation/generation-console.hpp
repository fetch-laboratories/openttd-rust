/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file generation-console.hpp Early semantic snapshots and unsaveable cancellation. */
#include "heightmap.h"
#include "landscape.h"
#include "openttd.h"
#include "fios.h"
#include "core/random_func.hpp"
extern "C" void __real__Z26SetGeneratingWorldProgress16GenWorldProgressj(GenWorldProgress, uint);
extern "C" void __real__Z31IncreaseGeneratingWorldProgress16GenWorldProgress(GenWorldProgress);
extern "C" void __real__Z29GenerateWorldSetAbortCallbackPFvvE(GWAbortProc *);
static int generation_cancel = -1, generation_events = 0, generation_cleanup = 0;
static GWAbortProc *generation_abort = nullptr;
static void GenerationCleanup() { generation_cleanup++; generation_abort(); }
extern "C" void __wrap__Z29GenerateWorldSetAbortCallbackPFvvE(GWAbortProc *callback)
{
	generation_abort = callback;
	__real__Z29GenerateWorldSetAbortCallbackPFvvE(generation_cancel >= 0 && callback != nullptr ? GenerationCleanup : callback);
}
static void GenerationProgress(GenWorldProgress stage)
{
	if (generation_cancel >= 0 && stage == GWP_LANDSCAPE && generation_events++ == generation_cancel) HandleGeneratingWorldAbortion();
}
extern "C" void __wrap__Z26SetGeneratingWorldProgress16GenWorldProgressj(GenWorldProgress stage, uint total)
{
	GenerationProgress(stage); __real__Z26SetGeneratingWorldProgress16GenWorldProgressj(stage, total);
}
extern "C" void __wrap__Z31IncreaseGeneratingWorldProgress16GenWorldProgress(GenWorldProgress stage)
{
	GenerationProgress(stage); __real__Z31IncreaseGeneratingWorldProgress16GenWorldProgress(stage);
}
static void GenerationCancellation()
{
	AutoRestoreBackup<uint8_t> generator(_settings_game.game_creation.land_generator, LG_TERRAGENESIS);
	AutoRestoreBackup<SwitchMode> mode(_switch_mode, _switch_mode);
	for (generation_cancel = 0; generation_cancel < 3; generation_cancel++) {
		generation_events = generation_cleanup = 0; bool caught = false;
		try { GenerateLandscape(GWM_NEWGAME); } catch (AbortGenerateWorldSignal &) { caught = true; }
		fmt::print(stderr, "GENERATION cancel {} {} {} {}\n", generation_cancel, caught, generation_events, generation_cleanup);
		GenerateWorldSetAbortCallback(nullptr);
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
		AutoRestoreBackup<std::string> filename(_file_to_saveload.name, "generation-missing.bmp");
		AutoRestoreBackup<DetailedFileType> type(_file_to_saveload.ftype.detailed, DFT_HEIGHTMAP_BMP);
		fmt::print(stderr, "GENERATION missing {} {}\n", loaded, GenerateLandscape(GWM_HEIGHTMAP));
	}
	FlatEmptyWorld(7); GenerationSnapshot("flat");
	for (TileIndex index : Map::Iterate()) {
		Tile t{index}; auto &v = map[index.base()];
		t.type() = v[0]; t.height() = v[1]; t.m2() = v[2]; t.m1() = v[3]; t.m3() = v[4]; t.m4() = v[5]; t.m5() = v[6]; t.m6() = v[7]; t.m7() = v[8]; t.m8() = v[9];
	}
	GenerationCancellation(); RestoreRandomSeeds(seeds);
	std::fflush(stderr); return true;
}
