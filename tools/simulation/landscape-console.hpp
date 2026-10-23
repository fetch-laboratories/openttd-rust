/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file landscape-console.hpp Identical landscape transaction inputs in both games. */
#include "terraform_cmd.h"
#include "clear_map.h"
#include "vehicle_base.h"
#include "vehicle_func.h"
#include "tunnelbridge_cmd.h"
static void LandscapeResult(const char *name, const CommandCost &cost, Money cash = 0, TileIndex error = INVALID_TILE)
{
	fmt::print(stderr, "LANDSCAPE {} {} {} {} {} {}\n", name, cost.Succeeded(), cost.GetCost(), cost.GetErrorMessage(), cash, error.base());
}
template<typename... Extra> static void LandscapeResult(const char *name, const std::tuple<CommandCost, Extra...> &result)
{
	if constexpr (sizeof...(Extra) == 2) LandscapeResult(name, std::get<0>(result), std::get<1>(result), std::get<2>(result));
	else LandscapeResult(name, std::get<0>(result), std::get<1>(result));
}
static bool LandscapeScenario(std::span<std::string_view> args)
{
	if (args.size() < 2) return false;
	Company *c = *Company::Iterate().begin(); AutoRestoreBackup<CompanyID> company(_current_company, c->index);
#ifdef WITH_RUST
	auto &finance = c->Finances();
#else
	auto &finance = *c;
#endif
	if (args[1] == "inspect") { fmt::print(stderr, "LANDSCAPE reload {} {} {}\n", finance.money, finance.clear_limit, finance.terraform_limit); return true; }
	finance.money = 1000000000; finance.clear_limit = finance.terraform_limit = UINT32_MAX;
	AutoRestoreBackup<bool> infinite(_settings_game.difficulty.infinite_money, false);
	TileIndex tile = INVALID_TILE; std::vector<uint8_t> heights;
	for (uint y = 2; y + 14 < Map::MaxY() && tile == INVALID_TILE; y++) for (uint x = 2; x + 14 < Map::MaxX(); x++) {
		TileArea area(TileXY(x, y), 14, 14); bool clear = true;
		for (TileIndex t : area) clear &= IsTileType(t, MP_CLEAR);
		if (!clear) continue;
		tile = TileXY(x + 3, y + 3); for (TileIndex t : area) { heights.push_back(TileHeight(t)); SetTileHeight(t, 3); } break;
	}
	if (tile == INVALID_TILE) std::abort();
	auto terraform = [&](const char *name, TileIndex t, Slope slope, bool up, DoCommandFlags flags) { LandscapeResult(name, Command<CMD_TERRAFORM_LAND>::Do(flags, t, slope, up)); };
	for (DoCommandFlags flags : {DoCommandFlags{}, DoCommandFlags{DoCommandFlag::QueryCost}, DoCommandFlags{DoCommandFlag::Execute}}) {
		terraform("raise", tile, SLOPE_N, true, flags); terraform("lower", tile, SLOPE_N, false, flags);
	}
	for (uint i = 0; i < 3; i++) terraform("recursive", tile, SLOPE_N, true, DoCommandFlag::Execute);
	for (bool diagonal : {false, true}) for (LevelMode lm : {LM_LEVEL, LM_LOWER, LM_RAISE}) {
		LandscapeResult("level-test", Command<CMD_LEVEL_LAND>::Do({}, tile + TileDiffXY(2, 2), tile, diagonal, lm));
		LandscapeResult("level-exec", Command<CMD_LEVEL_LAND>::Do(DoCommandFlag::Execute, tile + TileDiffXY(2, 2), tile, diagonal, lm));
	}
	{ AutoRestoreBackup<uint32_t> limit(finance.terraform_limit, 0); terraform("terraform-limit", tile, SLOPE_N, true, {}); }
	{ AutoRestoreBackup<uint32_t> limit(finance.terraform_limit, 1U << 16); LandscapeResult("level-limit", Command<CMD_LEVEL_LAND>::Do({}, tile + 2, tile, false, LM_RAISE)); }
	for (bool diagonal : {false, true}) {
		LandscapeResult("clear-test", Command<CMD_CLEAR_AREA>::Do({}, tile + TileDiffXY(2, 2), tile, diagonal));
		LandscapeResult("clear-exec", Command<CMD_CLEAR_AREA>::Do(DoCommandFlag::Execute, tile + TileDiffXY(2, 2), tile, diagonal));
	}
	{ AutoRestoreBackup<uint32_t> limit(finance.clear_limit, 0); LandscapeResult("clear-limit", Command<CMD_CLEAR_AREA>::Do(DoCommandFlag::Execute, tile, tile, false)); }
	{ AutoRestoreBackup<Money> money(finance.money, 0); MakeClear(tile, CLEAR_GRASS, 3); ResetScratch(); RecursiveCommandCounter nested;
		LandscapeResult("clear-cash", CmdClearArea(DoCommandFlag::Execute, tile + 1, tile, false));
		LandscapeResult("level-cash", CmdLevelLand(DoCommandFlag::Execute, tile + 1, tile, false, LM_RAISE)); }
	ObjectSpec *spec = const_cast<ObjectSpec *>(ObjectSpec::Get(OBJECT_TRANSMITTER)); Backup<ObjectSpec> original(*spec);
	TileIndex object = tile + TileDiffXY(5, 5); spec->flags = {ObjectFlag::Autoremove}; spec->size = 0x22;
	Object::CanAllocateItem(); BuildObject(OBJECT_TRANSMITTER, object, c->index);
	terraform("object-two-pass", object, SLOPE_ELEVATED, true, DoCommandFlag::Execute);
	spec->size = OBJECT_SIZE_1X1; spec->flags = {}; Object::CanAllocateItem(); BuildObject(OBJECT_TRANSMITTER, object, c->index);
	ResetScratch(); { RecursiveCommandCounter nested; LandscapeResult("partial-clear", CmdClearArea({DoCommandFlag::Execute, DoCommandFlag::Auto}, object, object - 1, false)); }
	LandscapeResult("object-remove", Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, object)); original.Restore();
	{ AutoRestoreBackup<decltype(_game_mode)> editor(_game_mode, GM_EDITOR); bool before = _generating_world;
		terraform("editor", tile, SLOPE_N, true, DoCommandFlag::Execute); fmt::print(stderr, "LANDSCAPE editor-restored {}\n", _generating_world == before); }
	{ AutoRestoreBackup<bool> edges(_settings_game.construction.freeform_edges, false); terraform("edge", TileXY(1, 1), SLOPE_N, true, {}); }
	{ AutoRestoreBackup<bool> edges(_settings_game.construction.freeform_edges, true); uint height = TileHeight(TileIndex{});
		terraform("void", TileIndex{}, SLOPE_N, true, DoCommandFlag::Execute); SetTileHeight(TileIndex{}, height); }
	{ AutoRestoreBackup<uint8_t> limit(_settings_game.construction.map_height_limit, TileHeight(tile)); terraform("too-high", tile, SLOPE_N, true, {}); }
	TileIndex bridge = tile + TileDiffXY(0, 6);
	LandscapeResult("bridge-build", Command<CMD_BUILD_BRIDGE>::Do(DoCommandFlag::Execute, bridge, bridge + 3, TRANSPORT_ROAD, 0, RoadType{0}));
	for (uint i = 0; i < 3; i++) terraform("bridge-damage", bridge + 1, SLOPE_ELEVATED, true, {});
	LandscapeResult("bridge-remove", Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, bridge));
	TileIndex tunnel = tile + TileDiffXY(0, 8);
	for (TileIndex t : {tunnel, tunnel + TileDiffXY(0, 1), tunnel + 5, tunnel + TileDiffXY(5, 1)}) SetTileHeight(t, 2);
	LandscapeResult("tunnel-build", Command<CMD_BUILD_TUNNEL>::Do(DoCommandFlag::Execute, tunnel, TRANSPORT_ROAD, 0));
	terraform("tunnel-damage", tunnel + 2, SLOPE_N, false, {});
	LandscapeResult("tunnel-remove", Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, tunnel));
	for (const Vehicle *v : Vehicle::Iterate()) if ((v->type == VEH_ROAD || v->type == VEH_TRAIN) && v->owner == c->index && EnsureNoVehicleOnGround(v->tile).Failed()) {
		ResetScratch(); uint32_t before = finance.clear_limit; LandscapeResult("occupied-clear", CmdLandscapeClear(DoCommandFlag::Execute, v->tile));
		fmt::print(stderr, "LANDSCAPE occupied-limit {}\n", before - finance.clear_limit); break;
	}

	uint index = 0; for (TileIndex t : TileArea(tile - TileDiffXY(3, 3), 14, 14)) SetTileHeight(t, heights[index++]);
	fmt::print(stderr, "LANDSCAPE final {} {} {}\n", finance.money, finance.clear_limit, finance.terraform_limit);
	if (args.size() == 3) SaveOrLoad(std::string(args[2]), SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false);
	std::fflush(stderr); return true;
}
