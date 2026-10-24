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

/* Generic callback programs are transient: reset before stack graphs die and
 * reinstall after native ReloadNewGRFData. Playback observation forwards normally. */
#include "newgrf_generic.h"
#include "tree_map.h"
#include "water_map.h"
#include "tile_cmd.h"
#ifdef WITH_RUST
#include "rust/landscape_loop_ffi.h"
#else
extern TileIndex _cur_tileloop_tile;
#endif
extern GrfMiscBits _misc_grf_features;
static void GenericNeed(bool ok) { if (!ok) std::abort(); }
/* This is a native observation terminal, not a parsed callback node: the common
 * evaluator must invoke its virtual method so duplicate-attempt counts are real. */
struct GenericObservedResult : SpriteGroup {
	uint16_t result;
	mutable uint attempts = 0;
	explicit GenericObservedResult(uint16_t result) : result(result) {}
	ResolverResult Resolve(ResolverObject &) const override { this->attempts++; return CallbackResult(this->result); }
};
static void GenericPrograms()
{
	GRFFile first, second; first.grfid = 0x42900001; second.grfid = 0x42900002; first.grf_version = 7; second.grf_version = 8;
	const CargoType cargo = GetCargoTypeByLabel(CT_PASSENGERS); GenericNeed(IsValidCargoType(cargo));
	first.cargo_map[cargo] = 11; second.cargo_map[cargo] = 22;
	GenericObservedResult yes(0x1234), no(CALLBACK_FAILED), error(0x4567);
	DeterministicSpriteGroup program, failure, procedure;
	for (auto *p : {&program, &failure, &procedure}) { p->size = DSG_SIZE_DWORD; p->var_scope = VSG_SCOPE_SELF; }
	procedure.default_result.calculated_result = true;
	procedure.adjusts = {{.operation = DSGA_OP_RST, .variable = 0x7D, .parameter = 0x0F, .and_mask = UINT32_MAX}};
	program.adjusts = {{.operation = DSGA_OP_RST, .variable = 0x7E, .and_mask = UINT32_MAX, .subroutine = &procedure}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x10E}};
	for (uint8_t variable : {0x40, 0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88}) {
		program.adjusts.push_back({.operation = DSGA_OP_RST, .variable = variable, .and_mask = UINT32_MAX});
		program.adjusts.push_back({.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = uint32_t(0x100 + (variable == 0x40 ? 0 : variable - 0x7F))});
	}
	program.default_result.group = &yes; program.error_group = &error;
	failure.adjusts = {{.operation = DSGA_OP_RST, .variable = 0x1A, .and_mask = 77}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x0F}};
	failure.default_result.group = &no;
	ResolverObject registers(nullptr);
	auto ai = [&](uint8_t source = IT_AI_UNKNOWN, uint8_t destination = IT_AI_TOWN) { return GetAiPurchaseCallbackResult(GSF_STATIONS, cargo, 13, source, destination, 55, AICE_STATION_GET_STATION_ID, 17, 19); };
	ResetGenericCallbacks(); GenericNeed(ai() == std::pair<const GRFFile *, uint16_t>{nullptr, CALLBACK_FAILED});
	AddGenericCallback(GSF_INVALID, &first, &program); GenericNeed(ai().second == CALLBACK_FAILED);
	AddGenericCallback(GSF_STATIONS, &first, &program); AddGenericCallback(GSF_STATIONS, &second, &failure); AddGenericCallback(GSF_STATIONS, &second, &failure);
	auto answer = ai(); GenericNeed(answer.first == &first && answer.second == 0x34 && no.attempts == 2 && yes.attempts == 1 && registers.GetRegister(0x10E) == 0);
	std::array<int32_t, 10> expected{11, cargo, CargoSpec::Get(cargo)->bitnum, 13, IT_AI_UNKNOWN, IT_AI_TOWN, 55, AICE_STATION_GET_STATION_ID, 17, 19};
	for (uint i = 0; i < expected.size(); ++i) GenericNeed(registers.GetRegister(0x100 + i) == expected[i]);
	fmt::print(stderr, "GENERIC fallback-duplicates-registers true\n");
	AddGenericCallback(GSF_STATIONS, &second, &program); answer = ai(); GenericNeed(answer.first == &second && answer.second == 0x1234 && registers.GetRegister(0x100) == 22 && no.attempts == 2);
	fmt::print(stderr, "GENERIC priority-v7-v8 true\n");
	IndustrySpec *spec = const_cast<IndustrySpec *>(GetIndustrySpec(0));
	{ AutoRestoreBackup<uint16_t> subst(spec->grf_prop.subst_id, 0x123); ai(0, IT_AI_UNKNOWN); GenericNeed(registers.GetRegister(0x104) == 0x23 && registers.GetRegister(0x105) == IT_AI_UNKNOWN); ai(0, 0); GenericNeed(registers.GetRegister(0x104) == 0x23 && registers.GetRegister(0x105) == 0x23); }
	{ AutoRestoreBackup<uint16_t> subst(spec->grf_prop.subst_id, IT_INVALID); ai(IT_AI_TOWN, 0); GenericNeed(registers.GetRegister(0x104) == IT_AI_TOWN && registers.GetRegister(0x105) == 0); }
	fmt::print(stderr, "GENERIC industry-sentinels-narrowing true\n");
	program.adjusts = {{.operation = DSGA_OP_RST, .variable = 0x89, .and_mask = UINT32_MAX}};
	GenericNeed(ai().second == 0x4567); program.var_scope = VSG_SCOPE_PARENT; GenericNeed(ai().second == 0x4567); program.var_scope = VSG_SCOPE_SELF;
	ResetGenericCallbacks(); AddGenericCallback(GSF_STATIONS, &first, &failure); AddGenericCallback(GSF_STATIONS, &second, &failure); GenericNeed(ai().first == nullptr && ai().second == CALLBACK_FAILED);
	fmt::print(stderr, "GENERIC unavailable-all-fail-reset true\n");
	ResetGenericCallbacks(); yes.result = 4;
	program.adjusts = {{.operation = DSGA_OP_RST, .variable = 0x10, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x100}, {.operation = DSGA_OP_RST, .variable = 0x0C, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x101}, {.operation = DSGA_OP_RST, .variable = 0x18, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x102}};
	const TileIndex tile = TileXY(4, 4); Tile t(tile);
	std::array<TileIndex, 3> corner_tiles{tile + 1, tile + TileDiffXY(0, 1), tile + TileDiffXY(1, 1)};
	std::array<uint8_t, 3> corner_heights; for (uint i = 0; i < 3; ++i) { corner_heights[i] = TileHeight(corner_tiles[i]); SetTileHeight(corner_tiles[i], 19); }
	auto fields = std::tuple(t.height(), t.m1(), t.m2(), t.m3(), t.m4(), t.m5(), t.m6(), t.m7(), t.m8());
	std::vector<uint8_t> types; for (Tile one : Map::Iterate()) { types.push_back(one.type()); one.type() = MP_VOID << 4; }
	AutoRestoreBackup<LandscapeType> climate(_settings_game.game_creation.landscape, LandscapeType::Tropic);
	AutoRestoreBackup<decltype(TimerGameTick::counter)> tick(TimerGameTick::counter, 1);
	AutoRestoreBackup<GrfMiscBits> features(_misc_grf_features, {});
	AutoRestoreBackup<bool> setting(_settings_client.sound.ambient, true);
#ifdef WITH_RUST
	auto &cursor = *openttd_rust_tile_loop_cursor();
#else
	auto &cursor = _cur_tileloop_tile;
#endif
	const auto saved_cursor = cursor;
	for (const TileType kind : {MP_CLEAR, MP_TREES, MP_WATER}) for (uint version : {7U, 8U}) for (uint mode = 0; mode < 4; ++mode) {
		ResetGenericCallbacks(); first.grf_version = version;
		AddGenericCallback(GSF_SOUNDFX, &first, &program); AddGenericCallback(GSF_SOUNDFX, &second, &failure);
		if (kind == MP_CLEAR) MakeClear(tile, CLEAR_GRASS, 3);
		if (kind == MP_TREES) MakeTree(tile, TREE_TEMPERATE, 1, TreeGrowthStage::Grown, TREE_GROUND_GRASS, 3);
		if (kind == MP_WATER) { MakeCanal(tile, OWNER_NONE, 0); SetNonFloodingWaterTile(tile, true); }
		SetTileHeight(tile, 19); SetTropicZone(tile, TROPICZONE_DESERT);
		_misc_grf_features = mode == 0 ? GrfMiscBits{} : GrfMiscBits{GrfMiscBit::AmbientSoundCallback};
		_settings_client.sound.ambient = mode != 1;
		Randomizer before; uint32_t draw = 0;
		for (uint seed = 1;; ++seed) { before.SetSeed(0x42900000U + seed); auto next = before; draw = next.Next(); if (Chance16I(1, 200, draw) == (mode != 2) && GB(draw, 16, 8) != 0) break; }
		auto expected_rng = before; if (mode != 0) expected_rng.Next(); _random = before;
		uint32_t packed = version < 8 ? kind << 28 | 15U << 24 | GB(draw, 16, 8) << 16 | GetTerrainType(tile) : kind << 24 | GetTileZ(tile) << 16 | GB(draw, 16, 8) << 8 | (HasTileWaterClass(tile) ? to_underlying(GetWaterClass(tile)) : 0) << 3 | GetTerrainType(tile);
		generic_sound_calls = 0; generic_sound_file = nullptr;
#ifdef WITH_RUST
		cursor = tile.base();
#else
		cursor = tile;
#endif
		 RunTileLoop();
		GenericNeed(_random.state[0] == expected_rng.state[0] && _random.state[1] == expected_rng.state[1]);
		GenericNeed(generic_sound_calls == uint(mode == 3));
		if (mode == 3) GenericNeed(generic_sound_file == &first && generic_sound_id == 4 && generic_sound_tile == tile && registers.GetRegister(0x100) == static_cast<int32_t>(packed) && registers.GetRegister(0x101) == CBID_SOUNDS_AMBIENT_EFFECT && registers.GetRegister(0x102) == 0);
		fmt::print(stderr, "GENERIC ambient {} {} {} {} {} {} {} {}\n", to_underlying(kind), version, mode, generic_sound_calls, mode == 3 ? packed : 0, generic_sound_file == &first, _random.state[0], _random.state[1]);
	}
	program.adjusts = {{.operation = DSGA_OP_RST, .variable = 0x80, .and_mask = UINT32_MAX}}; program.error_group = &no;
	for (uint seed = 1;; ++seed) { _random.SetSeed(seed); auto next = _random; if (Chance16I(1, 200, next.Next())) break; }
	auto expected_rng = _random; expected_rng.Next(); generic_sound_calls = 0;
#ifdef WITH_RUST
	cursor = tile.base();
#else
	cursor = tile;
#endif
	RunTileLoop(); GenericNeed(generic_sound_calls == 0 && _random.state[0] == expected_rng.state[0] && _random.state[1] == expected_rng.state[1]);
	fmt::print(stderr, "GENERIC ambient-unavailable-all-fail true\n");
	cursor = saved_cursor; uint i = 0; for (Tile one : Map::Iterate()) one.type() = types[i++];
	for (uint i = 0; i < 3; ++i) SetTileHeight(corner_tiles[i], corner_heights[i]);
	std::tie(t.height(), t.m1(), t.m2(), t.m3(), t.m4(), t.m5(), t.m6(), t.m7(), t.m8()) = fields;
	ResetGenericCallbacks(); // Before every graph/file destruction, including native reload.
	fmt::print(stderr, "GENERIC programs-complete true\n");
}
static bool GenericScenario(std::span<std::string_view> args)
{
	if (args.size() != 2) return false;
	GenericPrograms(); ReloadNewGRFData(); GenericPrograms();
	fmt::print(stderr, "GENERIC reinstalled-after-native-reload true\n"); std::fflush(stderr); return true;
}
