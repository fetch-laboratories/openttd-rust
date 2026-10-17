/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file towns-console.cpp Identical founding/house commands and otherwise unreachable name/cancellation inputs. */
#include "stdafx.h"
#include "console_internal.h"
#include "company_base.h"
#include "company_func.h"
#include "core/backup_type.hpp"
#include "core/random_func.hpp"
#include "town.h"
#include "town_cmd.h"
#include "townname_func.h"
#include "house.h"
#include "genworld.h"
#include "strings_func.h"
#include "error.h"
#include "table/strings.h"
#include "saveload/saveload.h"
#include "command_func.h"
#include "landscape_cmd.h"
#include "terraform_cmd.h"
#include "economy_func.h"
#include "town_map.h"
#include <map>
#ifdef WITH_RUST
#include "rust/town_founding_ffi.h"
#include "rust/generation_services.hpp"
#else
class AbortGenerateWorldSignal { };
#endif
extern Company *DoStartupNewCompany(bool, CompanyID);
namespace TownCancellationReference {
static uint phase, last_total;
static void SetProgress(GenWorldProgress, uint total) { last_total = total; if (phase == 0) throw AbortGenerateWorldSignal(); }
static void IncreaseProgress(GenWorldProgress) { throw AbortGenerateWorldSignal(); }
static Town *CreateRandomTown(uint, uint32_t, TownSize, bool, TownLayout) { std::abort(); }
#define SetGeneratingWorldProgress SetProgress
#define IncreaseGeneratingWorldProgress IncreaseProgress
#include "town-generation-original.hpp"
#undef IncreaseGeneratingWorldProgress
#undef SetGeneratingWorldProgress
}
static void Cost(const char *label, const CommandCost &cost, Money money = 0, TownID id = TownID::Invalid())
{
	fmt::print(stderr, "TOWN cost {} {} {} {} {} {}\n", label, cost.Succeeded(), cost.GetCost(), cost.GetErrorMessage(), money, id.base());
}
static void Cost(const char *label, const std::tuple<CommandCost, Money, TownID> &result) { const auto &[cost, money, id] = result; Cost(label, cost, money, id); }
static TileIndex Site()
{
	for (TileIndex tile{}; tile < Map::Size(); ++tile) {
		auto [cost, money, id] = CmdFoundTown({}, tile, TSZ_SMALL, false, TL_ORIGINAL, false, 0, "Site test");
		if (cost.Succeeded()) return tile;
	}
	std::abort();
}
/* Every original house through CmdPlaceHouse, replacement, protection,
 * autoslope and company clearance; aged houses then reach the rebuild path. */
static bool HousesScenario(std::span<std::string_view> args)
{
	using Place = Command<CMD_PLACE_HOUSE>;
	using Clear = Command<CMD_LANDSCAPE_CLEAR>;
	Company *company = Company::GetNumItems() == 0 ? DoStartupNewCompany(false, CompanyID::Invalid()) : *Company::Iterate().begin();
	Backup<CompanyID> owner(_current_company, OWNER_DEITY);
	if (args.size() > 2 && args[2] == "recession") _economy.fluct = -1;
	Backup<PlaceHouses> place(_settings_game.economy.place_houses, PH_FORBIDDEN);
	Cost("house-forbidden", Place::Do(DoCommandFlag::Execute, TileIndex{}, 0, false, false));
	place.Change(PH_ALLOWED);
	Cost("house-invalid", Place::Do(DoCommandFlag::Execute, TileIndex{}, static_cast<HouseID>(HouseSpec::Specs().size()), false, false));
	TileIndex centre{};
	while (centre < Map::Size() && !IsTileType(centre, MP_HOUSE)) ++centre;
	std::vector<TileIndex> window;
	for (int dy = -12; dy <= 12; ++dy) for (int dx = -12; dx <= 12; ++dx) if (TileIndex tile = TileAddWrap(centre, dx, dy); tile != INVALID_TILE) window.push_back(tile);
	std::vector<std::pair<TileIndex, HouseID>> sites;
	for (const HouseSpec &spec : HouseSpec::Specs()) {
		HouseID house = spec.Index();
		if (house >= NEW_HOUSE_OFFSET) break;
		if (!spec.enabled) { Cost("house-disabled", Place::Do(DoCommandFlag::Execute, centre, house, false, false)); continue; }
		if (!spec.building_flags.Any(BUILDING_HAS_1_TILE)) continue;
		place.Change(house % 2 == 0 ? PH_ALLOWED_CONSTRUCTED : PH_ALLOWED);
		std::map<StringID, uint> errors;
		for (TileIndex tile : window) {
			CommandCost test = Place::Do({}, tile, house, house % 3 == 0, false);
			if (test.Failed()) { errors[test.GetErrorMessage()]++; continue; }
			Cost("house-place", Place::Do(DoCommandFlag::Execute, tile, house, house % 3 == 0, false));
			sites.emplace_back(tile, house);
			break;
		}
		for (const auto &[message, count] : errors) fmt::print(stderr, "TOWN house-errors {} {} {}\n", house, message, count);
	}
	fmt::print(stderr, "TOWN house-sites {}\n", sites.size());
	auto single = std::ranges::find_if(sites, [](const auto &site) { return HouseSpec::Get(site.second)->building_flags.Test(BuildingFlag::Size1x1); });
	if (single != sites.end()) {
		HouseID other = single == sites.begin() ? sites.back().second : sites.front().second;
		Cost("house-occupied", Place::Do(DoCommandFlag::Execute, single->first, other, false, false));
		Cost("house-replace", Place::Do(DoCommandFlag::Execute, single->first, single->second, false, true));
	}
	for (const auto &[tile, house] : sites) Cost("house-protected", Clear::Do({}, tile));
	/* Autoslope applies to companies only; funding sets the growth flag the rebuild path needs. */
	owner.Change(company->index);
#ifdef WITH_RUST
	Backup<Money> money(company->Finances().money, 1000000000000LL);
#else
	Backup<Money> money(company->money, 1000000000000LL);
#endif
	Cost("house-fund", Command<CMD_DO_TOWN_ACTION>::Do(DoCommandFlag::Execute, ClosestTownFromTile(centre, UINT_MAX)->index, TownAction::FundBuildings));
	bool terraformed = false;
	for (const auto &[tile, house] : sites) {
		CommandCost lower = std::get<0>(Command<CMD_TERRAFORM_LAND>::Do({}, tile, SLOPE_N, false));
		Cost("house-terraform", lower);
		if (lower.Succeeded() && !terraformed) {
			terraformed = true;
			Cost("house-terraform-execute", std::get<0>(Command<CMD_TERRAFORM_LAND>::Do(DoCommandFlag::Execute, tile, SLOPE_N, false)));
		}
	}
	uint cleared = 0;
	for (TileIndex tile : window) {
		if (cleared == 24 || !IsTileType(tile, MP_HOUSE)) continue;
		Cost("house-clear-test", Clear::Do({}, tile));
		Cost("house-clear-auto", Clear::Do({DoCommandFlag::Execute, DoCommandFlag::Auto}, tile));
		CommandCost clear = Clear::Do(DoCommandFlag::Execute, tile);
		Cost("house-clear", clear);
		cleared += clear.Succeeded();
	}
	for (TileIndex tile{}; tile < Map::Size(); ++tile) {
		if (IsTileType(tile, MP_HOUSE)) for (int age = 0; age < 255; ++age) IncrementHouseAge(tile);
	}
	money.Restore(); place.Restore(); owner.Restore();
	fmt::print(stderr, "TOWN house-done {}\n", cleared);
	return true;
}
static bool TownsScenario(std::span<std::string_view> args)
{
	if (args.size() < 2) return false;
	if (args[1] == "inspect") {
		bool found = false;
		for (const Town *town : Town::Iterate()) found |= town->GetCachedName() == "Migration renamed";
		fmt::print(stderr, "TOWN reloaded {}\n", found); return true;
	}
	if (args[1] == "houses") return HousesScenario(args);
	Company *company = Company::GetNumItems() == 0 ? DoStartupNewCompany(false, CompanyID::Invalid()) : *Company::Iterate().begin();
	Backup<CompanyID> owner(_current_company, OWNER_DEITY);
	Backup<TownFounding> setting(_settings_game.economy.found_town, TF_CUSTOM_LAYOUT);
	Cost("invalid-size", CmdFoundTown({}, TileIndex{}, static_cast<TownSize>(255), false, TL_ORIGINAL, false, 0, "Invalid"));
	Cost("invalid-layout", CmdFoundTown({}, TileIndex{}, TSZ_SMALL, false, static_cast<TownLayout>(255), false, 0, "Invalid"));
	Cost("edge", CmdFoundTown({}, TileIndex{}, TSZ_SMALL, false, TL_ORIGINAL, false, 0, "Edge"));
	TileIndex site = Site();
	Town *initial = *Town::Iterate().begin(); TownNameParams initial_params(initial); std::string generated_name = GetTownName(initial);
	bool generated_duplicate = !VerifyTownName(initial->townnameparts, &initial_params);
	Cost("custom-generated", CmdFoundTown({}, site, TSZ_SMALL, false, TL_ORIGINAL, false, 0, generated_name));
	Cost("custom-initial", CmdRenameTown(DoCommandFlag::Execute, initial->index, generated_name));
	fmt::print(stderr, "TOWN automatic {} {}\n", generated_duplicate, !VerifyTownName(initial->townnameparts, &initial_params));
	owner.Change(company->index);
#ifdef WITH_RUST
	Backup<Money> money(company->Finances().money, 0);
#else
	Backup<Money> money(company->money, 0);
#endif
	auto [poor, extra, absent] = CmdFoundTown(DoCommandFlag::Execute, site, TSZ_SMALL, false, TL_ORIGINAL, false, 0, "Poor town");
	Cost("poor", poor, extra, absent); fmt::print(stderr, "TOWN deficit {} {} {}\n", poor.Succeeded(), extra > 0, absent == TownID::Invalid());
	money.Change(1000000000000LL);
	Cost("test", CmdFoundTown({}, site, TSZ_SMALL, false, TL_ORIGINAL, false, 0, "Migration town"));
	auto [cost, unused, id] = CmdFoundTown(DoCommandFlag::Execute, site, TSZ_SMALL, false, TL_ORIGINAL, false, 0, "Migration town");
	Cost("found", cost, unused, id); if (cost.Failed()) std::abort();
	Town *town = Town::Get(id);
	Cost("duplicate", CmdFoundTown({}, Site(), TSZ_SMALL, false, TL_ORIGINAL, false, 0, "Migration town"));
	Cost("rename-duplicate", CmdRenameTown(DoCommandFlag::Execute, id, "Migration town"));
	const std::string &reference = town->GetCachedName();
	Cost("bytes", CmdRenameTown(DoCommandFlag::Execute, id, "Migration bytes")); town->name = std::string("A\0B", 3);
	bool bytes = town->name == std::string("A\0B", 3) && reference == std::string("A\0B", 3) && &reference == &town->GetCachedName();
	Cost("reset", CmdRenameTown(DoCommandFlag::Execute, id, {}));
	bool reset = reference.empty() && &reference != &town->GetCachedName();
	Cost("rename", CmdRenameTown(DoCommandFlag::Execute, id, "Migration renamed"));
	fmt::print(stderr, "TOWN lifetime {} {} {}\n", bytes, reset, reference == "Migration renamed" && &reference == &town->GetCachedName());
	Cost("long", CmdRenameTown(DoCommandFlag::Execute, id, std::string(32, 'x')));
	owner.Change(OWNER_DEITY);
	Cost("deity", CmdFoundTown(DoCommandFlag::Execute, Site(), TSZ_LARGE, true, TL_2X2_GRID, false, 0, "Deity city"));
	owner.Change(company->index);
	Backup<GameMode> mode(_game_mode, GM_EDITOR);
	const uint expected_id = Town::GetNumItems();
	std::vector<bool> enabled; for (auto &house : HouseSpec::Specs()) { enabled.push_back(house.enabled); house.enabled = false; }
	Cost("zero-growth", CmdFoundTown(DoCommandFlag::Execute, TileIndex{}, TSZ_SMALL, false, TL_ORIGINAL, true, 0, "Zero town"));
	for (size_t i = 0; i < enabled.size(); ++i) HouseSpec::Get(i)->enabled = enabled[i];
	auto [editor, editor_money, editor_id] = CmdFoundTown(DoCommandFlag::Execute, TileIndex{}, TSZ_SMALL, false, TL_3X3_GRID, true, 0, "Editor town");
	Cost("editor", editor, editor_money, editor_id); fmt::print(stderr, "TOWN reuse {}\n", editor.Succeeded() && editor_id.base() == expected_id);
	fmt::print(stderr, "TOWN generate {}\n", GenerateTowns(TL_RANDOM, 2));
	TownNames names; TownNameParams parameters(_settings_game.game_creation.town_name);
	Randomizer test = _random, fill = test; uint32_t output = 0x87654321;
	for (uint i = 0; i < 1000; ++i) names.insert(GetTownName(&parameters, fill.Next()));
	bool exhausted = !GenerateTownName(test, &output, &names) && output == 0x87654321 && std::equal(std::begin(test.state), std::end(test.state), std::begin(fill.state));
	fmt::print(stderr, "TOWN name-retries {}\n", exhausted);
	{
		Backup<uint8_t> density(_settings_game.difficulty.number_towns, 0); Backup<uint16_t> custom(_settings_newgame.game_creation.custom_town_number, 65535);
		for (uint8_t d = 0; d < 5; ++d) { density.Change(d); fmt::print(stderr, "TOWN default {} {}\n", d, GetDefaultTownsForMapSize()); } density.Restore(); custom.Restore();
	}
	for (uint phase : {0, 1}) {
		SavedRandomSeeds seeds; SaveRandomSeeds(&seeds); TownCancellationReference::phase = phase; bool original = false, candidate = false;
		try { TownCancellationReference::GenerateTowns(TL_ORIGINAL, phase == 0 ? UINT_MAX : 0); } catch (AbortGenerateWorldSignal &) { original = true; }
		Randomizer after = _random; uint total = TownCancellationReference::last_total; RestoreRandomSeeds(seeds);
#ifdef WITH_RUST
		auto services = GetRustTownFoundingServices(); OpenTTDTownFoundingFrame frame; bool result;
		services.set_progress = [](OpenTTDTownFoundingFrame *f, uint total) noexcept { return RunGenerationProgress(f, [total] { TownCancellationReference::SetProgress(GWP_TOWN, total); }); };
		services.increase_progress = [](OpenTTDTownFoundingFrame *f) noexcept { return RunGenerationProgress(f, [] { TownCancellationReference::IncreaseProgress(GWP_TOWN); }); };
		try { if (!openttd_rust_town_generate(&services, &frame, &frame.names, TL_ORIGINAL, true, phase == 0 ? UINT_MAX : 0, &result)) std::rethrow_exception(frame.cancellation); } catch (AbortGenerateWorldSignal &) { candidate = true; }
#else
		try { TownCancellationReference::GenerateTowns(TL_ORIGINAL, phase == 0 ? UINT_MAX : 0); } catch (AbortGenerateWorldSignal &) { candidate = true; }
#endif
		fmt::print(stderr, "TOWN cancellation {} {} {} {}\n", phase, original && candidate, total == TownCancellationReference::last_total && total == (phase == 0 ? 64000 : 1), std::equal(std::begin(after.state), std::end(after.state), std::begin(_random.state))); RestoreRandomSeeds(seeds);
	}
	mode.Restore(); money.Restore(); owner.Restore(); setting.Restore(); SetGeneratingWorldProgress(GWP_GAME_START, 1);
	if (args.size() == 3) SaveOrLoad(std::string(args[2]), SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false);
	return true;
}
extern "C" void __real__Z22IConsoleStdLibRegisterv();
extern "C" void __wrap__Z22IConsoleStdLibRegisterv() { __real__Z22IConsoleStdLibRegisterv(); IConsole::CmdRegister("towns_scenario", TownsScenario); }
