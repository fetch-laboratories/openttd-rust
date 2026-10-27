/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file industry-definitions-console.hpp Real GRF definitions, funding and reload evidence. */
#include "newgrf_config.h"
#include "newgrf_badge.h"
#include "newgrf/newgrf_internal.h"
#include "command_func.h"
#include "industry_cmd.h"
#include "saveload/saveload.h"
#include "table/strings.h"
#ifdef WITH_RUST
#include "rust/industry_definition_ffi.h"
#endif

static uint32_t DefinitionGRFID(const char *text)
{
	return uint32_t(uint8_t(text[0])) | uint32_t(uint8_t(text[1])) << 8 | uint32_t(uint8_t(text[2])) << 16 | uint32_t(uint8_t(text[3])) << 24;
}
static void DefinitionRequire(bool value, const char *name)
{
	fmt::print(stderr, "INDUSTRY-DEFINITIONS case {} {}\n", name, value);
	if (!value) std::abort();
}
static void DefinitionInspect(bool reused)
{
	const uint32_t main = DefinitionGRFID("I475");
	const auto *file = GetFileByGRFID(main);
	DefinitionRequire(file != nullptr && GetGRFConfig(main)->status == GCS_ACTIVATED, "main-active");
	DefinitionRequire(file->industryspec.empty() && file->indtspec.empty(), "staging-cleared");
	DefinitionRequire(_industry_mngr.GetID(17, main) == 0, "first-industry-override");
	uint16_t type = _industry_mngr.GetID(18, main);
	DefinitionRequire(type < NEW_INDUSTRYOFFSET && type != 0, "disabled-original-slot");
	const IndustrySpec *spec = GetIndustrySpec(0);
	DefinitionRequire(spec->enabled && spec->grf_prop.grfid == main && spec->grf_prop.local_id == 17 && spec->grf_prop.subst_id == 0, "industry-identity");
	DefinitionRequire(spec->layouts.size() == 2 && spec->layouts[0].size() == 1 && spec->layouts[0][0].gfx == NEW_INDUSTRYTILEOFFSET && spec->layouts[1].size() > 1, "custom-invalid-imported-layouts");
	DefinitionRequire(spec->cost_multiplier == 1 && spec->minimal_cargo == 0 && spec->map_colour.p == 77 && spec->behaviour.None() && spec->prospecting_chance == 0xFEDCBA98 && spec->removal_cost_multiplier == 12345 && spec->station_name == STR_NULL, "scalars");
	DefinitionRequire(spec->produced_cargo[0] == 1 && spec->produced_cargo[1] == INVALID_CARGO && spec->accepts_cargo[0] == INVALID_CARGO && spec->production_rate[0] == 80 && spec->production_rate[1] == 0 && spec->input_cargo_multiplier[0][0] == 256 && spec->input_cargo_multiplier[0][1] == 512 && spec->input_cargo_multiplier[1][0] == 768 && spec->input_cargo_multiplier[1][1] == 1024 && spec->input_cargo_multiplier[2][0] == 0, "cargo-multipliers");
	const IndustrySpec *water = GetIndustrySpec(type);
	DefinitionRequire(water->layouts.size() == 1 && water->layouts[0][0].ti.x == -1 && water->layouts[0][0].ti.y == 2 && water->layouts[0][0].gfx == GFX_WATERTILE_SPECIALCHECK && water->random_sounds == std::vector<uint8_t>({3, 4}), "pre-v8-water-offset");
	IndustryGfx gfx = _industile_mngr.GetID(0, main);
	DefinitionRequire(gfx == NEW_INDUSTRYTILEOFFSET && _industry_tile_specs[0].grf_prop.override_id == gfx && !_industry_tile_specs[0].enabled, "first-tile-override");
	const auto &tile = _industry_tile_specs[gfx];
	DefinitionRequire(tile.animation.frames == 3 && tile.animation.speed == 4 && tile.accepts_cargo[0] == 1 && tile.acceptance[0] == 8 && tile.accepts_cargo[1] == 0 && tile.acceptance[1] == -4 && tile.accepts_cargo[2] == INVALID_CARGO, "tile-properties");
	DefinitionRequire(_industile_mngr.GetID(79, main) == 254 && _industile_mngr.GetID(80, main) == 256 && _industile_mngr.GetID(81, main) == 257, "water-marker-slot-excluded");
	uint16_t conflict = _industry_mngr.GetID(17, DefinitionGRFID("C475"));
	DefinitionRequire(conflict < NUM_INDUSTRYTYPES && conflict != 0 && GetIndustrySpec(conflict)->grf_prop.override_id == (reused ? 0 : IT_INVALID), "industry-override-conflict");
	DefinitionRequire(tile.badges.size() == 1 && spec->badges == tile.badges, "badge-publication");
	for (const char *name : {"P475", "B475", "Q475", "X475"}) {
		const auto *config = GetGRFConfig(DefinitionGRFID(name));
		DefinitionRequire(config != nullptr && config->status == GCS_DISABLED && config->errors.size() == 1, "disabled-file");
		DefinitionRequire(config->errors.front().message == (name[0] == 'X' ? STR_NEWGRF_ERROR_INVALID_ID : STR_NEWGRF_ERROR_READ_BOUNDS), "native-error");
	}
	uint16_t partial_tile = _industile_mngr.GetID(0, DefinitionGRFID("P475"));
	DefinitionRequire(partial_tile < NUM_INDUSTRYTILES && _industry_tile_specs[partial_tile].animation.frames == 7, "incremental-animation-failure");
	const auto *badge_file = GetFileByGRFID(DefinitionGRFID("Q475"));
	uint16_t badge_tile = _industile_mngr.GetID(0, DefinitionGRFID("Q475"));
	DefinitionRequire(badge_file != nullptr && badge_file->badge_list.size() == 2 && _industry_tile_specs[badge_tile].badges == std::vector<BadgeID>{badge_file->badge_list[1]} && GetBadge(badge_file->badge_list[0])->features.Test(GSF_INDUSTRYTILES), "partial-badge-effects");
	uint16_t partial_industry = _industry_mngr.GetID(19, DefinitionGRFID("B475"));
	DefinitionRequire(partial_industry < NUM_INDUSTRYTYPES && GetIndustrySpec(partial_industry)->cost_multiplier == 93, "incremental-industry-failure");
	for (uint16_t id : {uint16_t(0), type, conflict, partial_industry}) {
		const auto *value = GetIndustrySpec(id);
		fmt::print(stderr, "INDUSTRY-DEFINITIONS industry {} {} {} {} {} {}\n", id, value->grf_prop.grfid, value->grf_prop.local_id, value->grf_prop.subst_id, value->grf_prop.override_id, value->layouts.size());
		for (const auto &layout : value->layouts) for (const auto &part : layout) fmt::print(stderr, "INDUSTRY-DEFINITIONS layout {} {} {}\n", part.ti.x, part.ti.y, part.gfx);
	}
	for (uint16_t id = 0; id < NUM_INDUSTRYTILES; ++id) {
		const auto &mapping = _industile_mngr.mappings[id];
		if (mapping.grfid != 0) fmt::print(stderr, "INDUSTRY-DEFINITIONS mapping {} {} {} {}\n", id, mapping.grfid, mapping.entity_id, mapping.substitute_id);
	}
	fmt::print(stderr, "INDUSTRY-DEFINITIONS rng {} {} {} {}\n", _random.state[0], _random.state[1], _interactive_random.state[0], _interactive_random.state[1]);
}
static void DefinitionClimateReset()
{
	Backup<LandscapeType> climate(_settings_game.game_creation.landscape);
	_industry_mngr.mappings[NUM_INDUSTRYTYPES - 1] = {0x475475, 127, 0};
	_industry_mngr.Add(126, 0x475475, 0);
	_settings_game.game_creation.landscape = LandscapeType::Toyland;
#ifdef WITH_RUST
	openttd_rust_industry_definition_reset(to_underlying(_settings_game.game_creation.landscape), &GetIndustryDefinitionServices());
#else
	ResetIndustries();
#endif
	DefinitionRequire(!GetIndustrySpec(0)->enabled && GetIndustrySpec(27)->enabled && _industry_mngr.GetID(127, 0x475475) == NUM_INDUSTRYTYPES - 1 && _industry_mngr.GetID(126, 0x475475) == IT_INVALID, "climate-and-reset-lifetimes");
	_settings_game.game_creation.landscape = LandscapeType::Arctic;
#ifdef WITH_RUST
	openttd_rust_industry_definition_reset(to_underlying(_settings_game.game_creation.landscape), &GetIndustryDefinitionServices());
#else
	ResetIndustries();
#endif
	DefinitionRequire(GetIndustrySpec(0)->enabled && !GetIndustrySpec(27)->enabled, "arctic-reset");
	_industry_mngr.ResetMapping();
	climate.Restore();
#ifdef WITH_RUST
	openttd_rust_industry_definition_reset(to_underlying(_settings_game.game_creation.landscape), &GetIndustryDefinitionServices());
#else
	ResetIndustries();
#endif
}
static bool IndustryDefinitionsScenario(std::span<std::string_view> args)
{
	if (args.size() < 2) return false;
	if (args[1] == "inspect") { _interactive_random.SetSeed(0x47500002); ReloadNewGRFData(); DefinitionInspect(true); fmt::print(stderr, "INDUSTRY-DEFINITIONS complete true\n"); return true; }
	if (args.size() < 3) return false;
	DefinitionClimateReset();
	SetRandomSeed(0x47500001);
	_grfconfig.clear();
	for (const char *name : {"main", "conflict", "partial", "broken", "badges", "invalid"}) {
		auto config = std::make_unique<GRFConfig>(std::string(args[2]) + "/" + name + ".grf");
		DefinitionRequire(FillGRFDetails(*config, false, NO_DIRECTORY), "real-file-scan");
		config->flags.Reset(GRFConfigFlag::InitOnly);
		_grfconfig.push_back(std::move(config));
	}
	ReloadNewGRFData();
	DefinitionInspect(false);
	ReloadNewGRFData();
	DefinitionInspect(true);
	_settings_game.construction.raw_industry_construction = 1;
	_settings_game.economy.multiple_industry_per_town = true;
	bool funded = false;
	for (const Industry *source : Industry::Iterate()) {
		if (source->type != 0) continue;
		for (const Station *station : source->stations_near) {
			AutoRestoreBackup company(_current_company, station->owner);
			for (TileIndex at{0}; at < Map::Size(); ++at) {
				if (!station->TileIsInCatchment(at) || !IsTileType(at, MP_CLEAR) || CmdBuildIndustry({}, at, 0, 0, true, 0x475).Failed()) continue;
				CommandCost result = Command<CMD_BUILD_INDUSTRY>::Do(DoCommandFlag::Execute, at, IndustryType{0}, 0, true, 0x475);
				if (result.Failed()) continue;
				const Industry *built = Industry::GetByTile(at);
				DefinitionRequire(built != nullptr && built->type == 0 && built->location.w == 1 && built->location.h == 1, "funded-custom-layout");
				fmt::print(stderr, "INDUSTRY-DEFINITIONS funded {} {} {} {}\n", built->index.base(), at.base(), station->index.base(), result.GetCost());
				funded = true; break;
			}
			if (funded) break;
		}
		if (funded) break;
	}
	DefinitionRequire(funded, "funding");
	if (args.size() > 3) DefinitionRequire(SaveOrLoad(std::string(args[3]), SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false) == SL_OK, "save-identities");
	fmt::print(stderr, "INDUSTRY-DEFINITIONS complete true\n");
	return true;
}
