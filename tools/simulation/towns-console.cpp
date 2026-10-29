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
#include "tile_cmd.h"
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
#include "newgrf_spritegroup.h"
#include "newgrf_house.h"
#include "newgrf_cargo.h"
#include "newgrf_config.h"
#include "newgrf_badge.h"
#include "animated_tile_func.h"
#include "station_base.h"
#include "station_kdtree.h"
#include "waypoint_base.h"
#include "depot_base.h"
#include "object.h"
#include "object_base.h"
#include "clear_map.h"
#include "bridge_map.h"
#include "industry_cmd.h"
#include "subsidy_base.h"
#include "newgrf_storage.h"
#include "industry_map.h"
#include "water_map.h"
#include "road_map.h"
#include "road_cmd.h"
#include "road.h"
#include "object_map.h"
#include "tunnelbridge_cmd.h"
#include <map>
#include <set>
#ifdef WITH_RUST
#include "rust/town_founding_ffi.h"
#include "rust/generation_services.hpp"
#else
class AbortGenerateWorldSignal { };
#endif
extern Company *DoStartupNewCompany(bool, CompanyID);
extern bool _ignore_industry_restrictions;
void BuildOilRig(TileIndex);
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

/** Move every corner in [x0, x1] x [y0, y1] to height through the original terraform command, so neighbouring slopes stay valid. */
static void LevelCorners(uint x0, uint x1, uint y0, uint y1, uint height, const char *label)
{
	for (uint y = y0; y <= y1; ++y) for (uint x = x0; x <= x1; ++x) {
		TileIndex corner = TileXY(x, y);
		while (TileHeight(corner) != height) {
			CommandCost cost = std::get<0>(Command<CMD_TERRAFORM_LAND>::Do(DoCommandFlag::Execute, corner, SLOPE_N, TileHeight(corner) < height));
			if (cost.Failed()) { Cost(label, cost); std::abort(); }
		}
	}
}
/* Observe the real presentation request, then forward it unchanged. The null
 * driver cannot establish rendered errors or authority-window refreshes. */
static uint bribe_errors;
extern "C" void __real__Z16ShowErrorMessageO13EncodedStringS0_12WarningLeveliiS0_6PoolIDIh12CompanyIDTagLh15ELh255EE(EncodedString &&, EncodedString &&, WarningLevel, int, int, EncodedString &&, CompanyID);
extern "C" void __wrap__Z16ShowErrorMessageO13EncodedStringS0_12WarningLeveliiS0_6PoolIDIh12CompanyIDTagLh15ELh255EE(EncodedString &&summary, EncodedString &&detail, WarningLevel level, int x, int y, EncodedString &&extra, CompanyID company)
{
	if (summary == GetEncodedString(STR_ERROR_BRIBE_FAILED) && detail.empty() && level == WL_INFO) ++bribe_errors;
	__real__Z16ShowErrorMessageO13EncodedStringS0_12WarningLeveliiS0_6PoolIDIh12CompanyIDTagLh15ELh255EE(std::move(summary), std::move(detail), level, x, y, std::move(extra), company);
}
static TileIndex Site()
{
	for (TileIndex tile{}; tile < Map::Size(); ++tile) {
		auto [cost, money, id] = CmdFoundTown({}, tile, TSZ_SMALL, false, TL_ORIGINAL, false, 0, "Site test");
		if (cost.Succeeded()) return tile;
	}
	std::abort();
}
/* The ordinary authority fixture has ample cash. Exercise the real mask and
 * command gate at native prices, including queries for a non-current company. */
static void TownActionMoney(Town *town, Company *company, Company *other)
{
#ifdef WITH_RUST
	Backup<Money> cash(company->Finances().money), peer(other->Finances().money);
#else
	Backup<Money> cash(company->money), peer(other->money);
#endif
	Backup<bool> infinite(_settings_game.difficulty.infinite_money, false);
	std::array<Money, 8> prices;
	std::set<Money> balances{-1, 0};
	for (TownAction action{}; action < TownAction::End; ++action) {
		Money price = _price[PR_TOWN_ACTION] * GetTownActionCost(action) >> 8;
		prices[to_underlying(action)] = price;
		for (int delta : {-1, 0, 1}) balances.insert(price + delta);
		fmt::print(stderr, "TOWN money-price {} {}\n", to_underlying(action), price);
	}
	/* Interactive randomness starts from process-specific state, not the map seed. */
	Backup<Randomizer> interactive_seed(_interactive_random);
	_interactive_random.SetSeed(12345);
	const auto random = _random, interactive = _interactive_random;
	for (bool foreign : {false, true}) {
		for (Money balance : balances) {
			cash.Change(foreign ? Money{1000000000000LL} : balance);
			peer.Change(foreign ? balance : Money{1000000000000LL});
			fmt::print(stderr, "TOWN money-mask {} {} {} {}\n", foreign, balance, GetMaskOfTownActions(foreign ? other->index : company->index, town).base(), GetMaskOfTownActions(foreign ? company->index : other->index, town).base());
		}
	}
	peer.Restore();
	Backup<uint8_t> funding(town->fund_buildings_months, 0);
	Money price = prices[to_underlying(TownAction::FundBuildings)];
	for (int delta : {-1, 0}) {
		cash.Change(price + delta);
		funding.Change(0);
		auto save = [&](const char *phase) {
			std::string path = fmt::format("{}/money-{}-{}.sav", std::getenv("HOME"), delta, phase);
			if (SaveOrLoad(path, SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false) != SL_OK) std::abort();
		};
		fmt::print(stderr, "TOWN money-command {} {} {} {}\n", delta, price, town->index.base(), company->index.base());
		save("before");
		Cost("money-test", CmdDoTownAction({}, town->index, TownAction::FundBuildings));
		save("test");
		Cost("money-exec", CmdDoTownAction(DoCommandFlag::Execute, town->index, TownAction::FundBuildings));
		save("after");
		fmt::print(stderr, "TOWN money-funded {} {}\n", delta, town->fund_buildings_months);
	}
	fmt::print(stderr, "TOWN money-rng {} {} {} {} {}\n", _random.state[0], _random.state[1], _interactive_random.state[0], _interactive_random.state[1], _random.state[0] == random.state[0] && _random.state[1] == random.state[1] && _interactive_random.state[0] == interactive.state[0] && _interactive_random.state[1] == interactive.state[1]);
	cash.Restore(); funding.Restore(); infinite.Restore(); interactive_seed.Restore();
}
/* Block every search position with real roads, then open only the last one.
 * The normal authority fixture only witnesses a successful early statue. */
static void TownStatueSearch(Town *town, Company *company)
{
	std::vector<TileIndex> tiles;
	for (TileIndex tile : SpiralTileSequence(town->xy, 9)) tiles.push_back(tile);
	if (tiles.size() != 81) std::abort();
	Backup<CompanyID> owner(_current_company, OWNER_DEITY);
	for (TileIndex tile : tiles) {
		if (Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, tile).Failed()) std::abort();
	}
	/* Canonical fixture terrain: every road tile and all its corners are flat. */
	std::map<TileIndex, uint8_t> heights;
	for (TileIndex tile : tiles) {
		for (int dy : {0, 1}) for (int dx : {0, 1}) {
			TileIndex corner = TileAddXY(tile, dx, dy);
			heights.try_emplace(corner, TileHeight(corner));
			SetTileHeight(corner, 4);
		}
	}
	owner.Change(company->index);
	for (TileIndex tile : tiles) {
		if (Command<CMD_BUILD_ROAD>::Do(DoCommandFlag::Execute, tile, ROAD_X, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid()).Failed()) std::abort();
	}
	Backup<Randomizer> interactive_seed(_interactive_random);
	_interactive_random.SetSeed(12345);
	const TileIndex last = tiles.back();
	for (bool open : {false, true}) {
		if (open && Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, last).Failed()) std::abort();
		fmt::print(stderr, "TOWN statue-input {} {} {} {} {} {}\n", open, town->index.base(), company->index.base(), last.base(), tiles.size(), _price[PR_TOWN_ACTION] * GetTownActionCost(TownAction::BuildStatue) >> 8);
		auto save = [&](const char *phase) {
			std::string path = fmt::format("{}/statue-{}-{}.sav", std::getenv("HOME"), open, phase);
			if (SaveOrLoad(path, SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false) != SL_OK) std::abort();
		};
		save("before");
		const auto random = _random, interactive = _interactive_random;
		Randomizer expected = random;
		for (bool execute : {false, true}) {
			CommandCost cost = CmdDoTownAction(execute ? DoCommandFlags{DoCommandFlag::Execute} : DoCommandFlags{}, town->index, TownAction::BuildStatue);
			Cost(execute ? "statue-exec" : "statue-test", cost);
			fmt::print(stderr, "TOWN statue-error {} {} {}\n", open, execute, cost.GetErrorMessage() == STR_ERROR_STATUE_NO_SUITABLE_PLACE);
			if (open && execute) expected.Next(); // BuildObject's single tile Random().
			fmt::print(stderr, "TOWN statue-rng {} {} {} {} {} {} {}\n", open, execute, _random.state[0], _random.state[1], _interactive_random.state[0], _interactive_random.state[1], _random.state[0] == expected.state[0] && _random.state[1] == expected.state[1] && _interactive_random.state[0] == interactive.state[0] && _interactive_random.state[1] == interactive.state[1]);
			save(execute ? "after" : "test");
		}
		bool placed = IsObjectTypeTile(last, OBJECT_STATUE) && GetTileOwner(last) == company->index && Object::GetByTile(last)->town == town;
		uint statues = 0;
		for (TileIndex tile : tiles) statues += IsObjectTypeTile(tile, OBJECT_STATUE);
		fmt::print(stderr, "TOWN statue-state {} {} {} {}\n", open, placed, statues, town->statues.Test(company->index));
	}
	owner.Change(OWNER_DEITY);
	for (TileIndex tile : tiles) {
		if (Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, tile).Failed()) std::abort();
	}
	for (const auto &[corner, height] : heights) SetTileHeight(corner, height);
	bool restored = std::ranges::all_of(heights, [](const auto &entry) { return TileHeight(entry.first) == entry.second; });
	fmt::print(stderr, "TOWN statue-cleanup {} {}\n", !town->statues.Test(company->index) && !IsObjectTypeTile(last, OBJECT_STATUE), restored);
	owner.Restore(); interactive_seed.Restore();
}
/* Real CmdPlaceHouse and animation bodies in both games, with controlled spec
 * overrides. The save corpus has no external NewGRF providing callback 0x1C.
 * Restore specs before returning so ordinary simulation and saves stay valid. */
static void HouseOverridePlacement(TileIndex tile, HouseID house)
{
	using Place = Command<CMD_PLACE_HOUSE>;
	const auto flags = HouseSpec::Get(house)->building_flags;
	std::vector<TileIndex> parts{tile};
	if (flags.Any(BUILDING_2_TILES_Y)) parts.push_back(TileAddXY(tile, 0, 1));
	if (flags.Any(BUILDING_2_TILES_X)) parts.push_back(TileAddXY(tile, 1, 0));
	if (flags.Any(BUILDING_HAS_4_TILES)) parts.push_back(TileAddXY(tile, 1, 1));
	const HouseID override = static_cast<HouseID>(HouseSpec::Specs().size());
	assert(override >= NEW_HOUSE_OFFSET);
	HouseSpec::Specs().resize(override + parts.size());
	CallbackResultSpriteGroup frame(7);
	for (uint mask : {0U, 5U, 15U}) {
		CommandCost clear = Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, tile);
		Cost("house-override-clear", clear);
		if (clear.Failed()) std::abort();
		for (size_t i = 0; i < parts.size(); ++i) {
			auto *source = HouseSpec::Get(house + i);
			auto *target = HouseSpec::Get(override + i);
			*target = *source;
			target->callback_mask.Reset();
			if ((mask & (1U << i)) != 0) target->callback_mask.Set(HouseCallbackMask::AnimationTriggerConstructionStageChanged);
			target->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, &frame);
			source->grf_prop.override_id = override + i;
		}
		Randomizer trace = _random;
		CommandCost placed = Place::Do(DoCommandFlag::Execute, tile, house, false, false);
		Cost("house-override-place", placed);
		if (placed.Failed()) std::abort();
		/* Emit every actual draw by replaying the pre-command generator up to
		 * its post-command state. Both missing and extra draws change the log. */
		fmt::print(stderr, "TOWN house-override {} {} {}", house, parts.size(), mask);
		uint draws = 0;
		while (trace.state[0] != _random.state[0] || trace.state[1] != _random.state[1]) {
			if (++draws > 16) std::abort();
			fmt::print(stderr, " rng={}", trace.Next());
		}
		for (TileIndex part : parts) {
			fmt::print(stderr, " part={}/{}/{}", GetCleanHouseType(part), GetHouseType(part), GetAnimationFrame(part));
			DeleteAnimatedTile(part, true);
		}
		fmt::print(stderr, "\n");
		for (size_t i = 0; i < parts.size(); ++i) {
			HouseSpec::Get(house + i)->grf_prop.override_id = INVALID_HOUSE_ID;
		}
	}
	HouseSpec::Specs().resize(override);
}
#include "house-resolver-console.inc"
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
	HouseResolverScenario(sites);
	BuildingFlags covered;
	for (const auto &[tile, house] : sites) {
		const auto size = HouseSpec::Get(house)->building_flags & BUILDING_HAS_1_TILE;
		if (house < 96 && !IsHouseProtected(tile) && !covered.Any(size)) { HouseOverridePlacement(tile, house); covered |= size; }
	}
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
	if (args.size() == 4) SaveOrLoad(std::string(args[3]), SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false);
	return true;
}
/* Commands use the unchanged native bodies in the reference. The explicit pool
 * associations below cover editor/lifetime inputs absent from ordinary games. */
static bool CommandsScenario(std::span<std::string_view> args)
{
	if (args[1] == "authority-inspect") {
		const Town *town = *Town::Iterate().begin();
#ifdef WITH_RUST
		bool text = town->HasText();
#else
		bool text = !town->text.empty();
#endif
		fmt::print(stderr, "TOWN authority-reload {}\n", text); return true;
	}
	Company *company = Company::GetNumItems() == 0 ? DoStartupNewCompany(false, CompanyID::Invalid()) : *Company::Iterate().begin();
	Company *other = DoStartupNewCompany(false, CompanyID::Invalid());
	Town *town = *Town::Iterate().begin(); TownID id = town->index;
	Backup<CompanyID> owner(_current_company, company->index);
	Backup<GameMode> mode(_game_mode, GM_EDITOR);
	Backup<bool> roads(_settings_game.economy.fund_roads, true), buildings(_settings_game.economy.fund_buildings, true), rights(_settings_game.economy.exclusive_rights, true), bribes(_settings_game.economy.bribe, true);
#ifdef WITH_RUST
	Backup<Money> money(company->Finances().money, 1000000000000LL), other_money(other->Finances().money, 1000000000000LL);
#else
	Backup<Money> money(company->money, 1000000000000LL), other_money(other->money, 1000000000000LL);
#endif
	Cost("goal-wrong-owner", CmdTownCargoGoal({}, id, TAE_PASSENGERS, 100));
	Cost("text-wrong-owner", CmdTownSetText({}, id, {}));
	Cost("rate-wrong-owner", CmdTownGrowthRate({}, id, 1));
	Cost("rating-wrong-owner", CmdTownRating({}, id, company->index, 1));
	owner.Change(OWNER_DEITY);
	Cost("goal-invalid", CmdTownCargoGoal({}, id, static_cast<TownAcceptanceEffect>(255), 1));
	Cost("text-invalid", CmdTownSetText({}, TownID::Invalid(), {}));
	Cost("rating-invalid", CmdTownRating({}, id, CompanyID::Invalid(), 1));
	for (uint8_t effect = TAE_BEGIN; effect < TAE_END; ++effect) {
		Cost("goal-test", CmdTownCargoGoal({}, id, static_cast<TownAcceptanceEffect>(effect), 100 + effect));
		Cost("goal-exec", CmdTownCargoGoal(DoCommandFlag::Execute, id, static_cast<TownAcceptanceEffect>(effect), 100 + effect));
	}
	EncodedString text = GetEncodedString(STR_TOWN_NAME, id);
	Cost("text-test", CmdTownSetText({}, id, text));
	Cost("text-exec", CmdTownSetText(DoCommandFlag::Execute, id, text));
	Cost("text-clear", CmdTownSetText(DoCommandFlag::Execute, id, {}));
	Cost("text-restore", CmdTownSetText(DoCommandFlag::Execute, id, text));
	for (uint16_t rate : {uint16_t{1}, uint16_t{65535}, uint16_t{0}}) {
		Cost("rate-test", CmdTownGrowthRate({}, id, rate)); Cost("rate-exec", CmdTownGrowthRate(DoCommandFlag::Execute, id, rate));
	}
	for (int16_t rating : {int16_t{-32768}, int16_t{32767}, int16_t{500}}) {
		Cost("rating-test", CmdTownRating({}, id, company->index, rating)); Cost("rating-exec", CmdTownRating(DoCommandFlag::Execute, id, company->index, rating));
	}
	Cost("expand-invalid", CmdExpandTown({}, id, 1, {}));
	Cost("expand-test", CmdExpandTown({}, id, 1, TownExpandMode::Buildings));
	Cost("expand-exec", CmdExpandTown(DoCommandFlag::Execute, id, 1, TownExpandMode::Buildings));
	Cost("expand-random", CmdExpandTown(DoCommandFlag::Execute, id, 0, TownExpandMode::Roads));
	owner.Change(company->index);
	if (!Station::CanAllocateItem()) std::abort();
	Station *station = new Station(town->xy); _station_kdtree.Insert(station->index); station->town = town; station->owner = company->index;
	TownActionMoney(town, company, other);
	TownStatueSearch(town, company);
	fmt::print(stderr, "TOWN action-mask {} {}\n", GetMaskOfTownActions(company->index, town).base(), GetMaskOfTownActions(COMPANY_SPECTATOR, town).base());
	for (TownAction action{}; action < TownAction::Bribe; ++action) {
		Cost("action-test", CmdDoTownAction({}, id, action)); Cost("action-exec", CmdDoTownAction(DoCommandFlag::Execute, id, action));
	}
	owner.Change(OWNER_DEITY);
	auto [neighbour_cost, neighbour_money, neighbour_id] = CmdFoundTown(DoCommandFlag::Execute, Site(), TSZ_SMALL, false, TL_ORIGINAL, false, 0, "Bribe neighbour");
	Cost("bribe-neighbour", neighbour_cost, neighbour_money, neighbour_id); if (neighbour_cost.Failed()) std::abort();
	Town *neighbour = Town::Get(neighbour_id);
	owner.Change(company->index);
	std::array<Station *, 4> stations{station};
	for (size_t i = 1; i < stations.size(); ++i) {
		if (!Station::CanAllocateItem()) std::abort();
		stations[i] = new Station(i < 2 ? town->xy : neighbour->xy);
		_station_kdtree.Insert(stations[i]->index);
		stations[i]->town = i < 2 ? town : neighbour;
		stations[i]->owner = i % 2 == 0 ? company->index : other->index;
	}
	Backup<CompanyID> local(_local_company, company->index);
	/* Observe ShowErrorMessage without creating a GUI window under -vnull. */
	Backup<uint8_t> duration(_settings_client.gui.errmsg_duration, 0);
	for (int16_t initial : {500, -51, -50, -49}) {
		const bool failed = initial != 500;
		uint seed = 1;
		for (; ; ++seed) { Randomizer trial; trial.SetSeed(seed); if (Chance16I(1, 14, trial.Next()) == failed) break; }
		_random.SetSeed(seed); _interactive_random.SetSeed(12345);
		Randomizer expected = _random; expected.Next();
		const Randomizer interactive = _interactive_random;
		town->unwanted[company->index] = 0; town->ratings[company->index] = initial;
		town->exclusivity = other->index; town->exclusive_counter = 12;
		for (size_t i = 0; i < stations.size(); ++i) {
			for (size_t cargo = 0; cargo < stations[i]->goods.size(); ++cargo) stations[i]->goods[cargo].rating = 70 + 20 * i + cargo;
		}
		fmt::print(stderr, "TOWN bribe-input {} {} {} {} {} {} {} {} {} {} {}\n", initial, failed, seed, id.base(), company->index.base(), other->index.base(), neighbour_id.base(), stations[0]->index.base(), stations[1]->index.base(), stations[2]->index.base(), stations[3]->index.base());
		auto save = [&](const char *phase) {
			std::string path = fmt::format("{}/bribe-{}-{}.sav", std::getenv("HOME"), initial, phase);
			if (SaveOrLoad(path, SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false) != SL_OK) std::abort();
		};
		save("before");
		bribe_errors = 0;
		Cost("bribe-test", CmdDoTownAction({}, id, TownAction::Bribe));
		save("test");
		fmt::print(stderr, "TOWN bribe-test-errors {} {}\n", initial, bribe_errors);
		Cost("bribe-exec", CmdDoTownAction(DoCommandFlag::Execute, id, TownAction::Bribe));
		fmt::print(stderr, "TOWN bribe-outcome {} {} {} {} {} {} {}\n", initial, bribe_errors, _random.state[0], _random.state[1], _interactive_random.state[0], _interactive_random.state[1], _random.state[0] == expected.state[0] && _random.state[1] == expected.state[1] && _interactive_random.state[0] == interactive.state[0] && _interactive_random.state[1] == interactive.state[1]);
		save("after");
		fmt::print(stderr, "TOWN bribe {} {} {} {} {} {} {}\n", failed, town->ratings[company->index], town->unwanted[company->index], station->goods[0].rating, town->exclusive_counter, town->exclusivity.base(), GetMaskOfTownActions(company->index, town).base());
	}
	duration.Restore(); local.Restore();
	/* Querying a different company must retain the original _current_company comparison. */
	town->ratings[other->index] = RATING_BRIBE_MAXIMUM;
	fmt::print(stderr, "TOWN foreign-mask {}\n", GetMaskOfTownActions(other->index, town).base());
	for (Station *st : stations) delete st;
	roads.Change(false); buildings.Change(false); rights.Change(false); bribes.Change(false);
	fmt::print(stderr, "TOWN disabled-mask {}\n", GetMaskOfTownActions(company->index, town).base());
	Cost("action-invalid", CmdDoTownAction({}, id, static_cast<TownAction>(255)));
	/* Real founded town and real command-built map contents; explicit shells cover later veto phases. */
	owner.Change(OWNER_DEITY);
	auto [founded, unused, removed] = CmdFoundTown(DoCommandFlag::Execute, Site(), TSZ_SMALL, false, TL_ORIGINAL, false, 0, "Deleted town");
	Cost("delete-found", founded, unused, removed); if (founded.Failed()) std::abort();
	Town *target = Town::Get(removed);
	/* Build an oil rig through the original industry command, then require the
	 * original station clearer's refusal before testing later veto phases. */
	Backup<bool> restrictions(_ignore_industry_restrictions, true);
	for (uint y = 6; y < 23; ++y) for (uint x = 6; x < 23; ++x) {
		TileIndex sea = TileXY(x, y); CommandCost cleared = Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, sea);
		if (cleared.Failed()) { Cost("fixture-sea", cleared); std::abort(); }
	}
	/* Level corners 6..23 through CMD_TERRAFORM_LAND, which also adjusts and clears the neighbouring tiles. */
	LevelCorners(6, 23, 6, 23, 0, "fixture-sea-level");
	for (uint y = 6; y < 23; ++y) for (uint x = 6; x < 23; ++x) {
		TileIndex sea = TileXY(x, y);
		if (!IsTileType(sea, MP_CLEAR) || GetTileSlope(sea) != SLOPE_FLAT || TileHeight(sea) != 0) std::abort();
		MakeSea(sea);
	}
	IndustryID oilrig = IndustryID::Invalid();
	std::set<IndustryID> original_industries; for (const Industry *i : Industry::Iterate()) original_industries.insert(i->index);
	for (TileIndex at{}; at < Map::Size(); ++at) {
		if (!IsTileType(at, MP_WATER) || CmdBuildIndustry({}, at, IndustryType{5}, 0, true, 123).Failed()) continue;
		CommandCost built = CmdBuildIndustry(DoCommandFlag::Execute, at, IndustryType{5}, 0, true, 123); Cost("delete-oilrig-build", built);
		if (built.Failed()) std::abort();
		Industry *industry = nullptr; for (Industry *i : Industry::Iterate()) if (!original_industries.contains(i->index)) industry = i;
		if (industry == nullptr) std::abort();
		industry->town = target; oilrig = industry->index;
		for (TileIndex part : industry->location) if (IsTileType(part, MP_INDUSTRY) && GetIndustryGfx(part) == GFX_OILRIG_1 && IsTileType(part + TileDiffXY(0, 1), MP_INDUSTRY)) { BuildOilRig(part); break; }
		for (Station *st : Station::Iterate()) if (st->industry == industry) st->town = target;
		break;
	}
	restrictions.Restore();
	if (oilrig == IndustryID::Invalid()) { Cost("fixture-oilrig", CmdBuildIndustry({}, TileXY(8, 8), IndustryType{5}, 0, true, 123)); std::abort(); }
	CommandCost oilrig_refusal = CmdDeleteTown(DoCommandFlag::Execute, removed); Cost("delete-oilrig-refusal", oilrig_refusal);
	fmt::print(stderr, "TOWN oilrig-refusal {} {}\n", oilrig_refusal.Failed(), Industry::IsValidID(oilrig));
	for (TileIndex part : Industry::Get(oilrig)->location) if (IsTileType(part, MP_INDUSTRY)) { Cost("delete-oilrig-remove", Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, part)); break; }
	if (!Station::CanAllocateItem()) std::abort();
	Station *veto = new Station(target->xy); _station_kdtree.Insert(veto->index); veto->town = target; veto->owner = company->index;
	Cost("delete-station-veto", CmdDeleteTown(DoCommandFlag::Execute, removed));
	delete veto;
	if (!Waypoint::CanAllocateItem()) std::abort();
	Waypoint *waypoint = new Waypoint(target->xy); waypoint->town = target; waypoint->owner = company->index;
	Cost("delete-waypoint-veto", CmdDeleteTown({}, removed)); delete waypoint;
	if (!Depot::CanAllocateItem()) std::abort();
	Depot *depot = new Depot(target->xy); depot->town = target;
	Cost("delete-depot-veto", CmdDeleteTown({}, removed)); delete depot;
	bool industry_built = false;
	for (TileIndex at{}; at < Map::Size(); ++at) {
		if (!IsTileType(at, MP_CLEAR) || CmdBuildIndustry({}, at, IndustryType{0}, 0, true, 321).Failed()) continue;
		CommandCost built = CmdBuildIndustry(DoCommandFlag::Execute, at, IndustryType{0}, 0, true, 321); Cost("delete-industry-build", built);
		if (built.Failed()) std::abort();
		Industry::GetByTile(at)->town = target; industry_built = true; break;
	}
	if (!industry_built) std::abort();
	for (uint y = 40; y <= 44; ++y) for (uint x = 40; x <= 50; ++x) {
		TileIndex at = TileXY(x, y); CommandCost clear = Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, at);
		if (clear.Failed()) { Cost("fixture-bridge-clear", clear); std::abort(); }
	}
	LevelCorners(40, 51, 40, 45, 1, "fixture-bridge-level");
	for (uint y = 40; y <= 44; ++y) for (uint x = 40; x <= 50; ++x) MakeClear(TileXY(x, y), CLEAR_GRASS, 3);
	bool bridge_built = false;
	owner.Change(OWNER_TOWN);
	for (TileIndex at = TileXY(5, 5); at < Map::Size() - Map::SizeX() * 5; ++at) {
		bool free = TileX(at) > 3 && TileX(at) < Map::MaxX() - 5;
		for (int x = -1; x <= 4 && free; ++x) free &= IsTileType(at + x, MP_CLEAR) && GetTileSlope(at + x) == SLOPE_FLAT && !IsBridgeAbove(at + x);
		if (!free || CmdBuildBridge({}, at + 3, at, TRANSPORT_ROAD, BridgeType{0}, ROADTYPE_ROAD).Failed()) continue;
		CommandCost built = CmdBuildBridge(DoCommandFlag::Execute, at + 3, at, TRANSPORT_ROAD, BridgeType{0}, ROADTYPE_ROAD); Cost("delete-bridge-build", built);
		if (built.Failed()) std::abort();
		MakeRoadNormal(at - 1, ROAD_X, ROADTYPE_ROAD, INVALID_ROADTYPE, removed, OWNER_TOWN, OWNER_NONE); bridge_built = true; break;
	}
	owner.Change(OWNER_DEITY);
	if (!bridge_built) { Cost("fixture-bridge", CmdBuildBridge({}, TileXY(45, 42), TileXY(42, 42), TRANSPORT_ROAD, BridgeType{0}, ROADTYPE_ROAD)); std::abort(); }
	if (!Object::CanAllocateItem()) std::abort();
	Object *detached = nullptr;
	for (TileIndex at{}; at < Map::Size(); ++at) if (IsTileType(at, MP_CLEAR) && GetTileSlope(at) == SLOPE_FLAT && !IsBridgeAbove(at)) { BuildObject(OBJECT_TRANSMITTER, at, OWNER_NONE, target); detached = Object::GetByTile(at); break; }
	if (detached == nullptr) std::abort();
	if (!PersistentStorage::CanAllocateItem() || !CargoPacket::CanAllocateItem() || !Subsidy::CanAllocateItem()) std::abort();
	auto *storage = new PersistentStorage(0x12345678, GSF_FAKE_TOWNS, target->xy); auto storage_id = storage->index; target->psa_list.push_back(storage);
	auto *packet = new CargoPacket(StationID::Invalid(), 7, {removed, SourceType::Town});
	auto *subsidy = new Subsidy(CargoType{0}, {removed, SourceType::Town}, {id, SourceType::Town}, 12); auto subsidy_id = subsidy->index;
	Cost("delete-test", CmdDeleteTown({}, removed));
	CommandCost deleted = CmdDeleteTown(DoCommandFlag::Execute, removed); Cost("delete-exec", deleted);
	fmt::print(stderr, "TOWN deleted-dependencies {} {} {}\n", !PersistentStorage::IsValidID(storage_id), !Subsidy::IsValidID(subsidy_id), !packet->GetSource().IsValid()); delete packet;
	fmt::print(stderr, "TOWN deletion {} {} {}\n", deleted.Succeeded(), !Town::IsValidID(removed), detached->town != nullptr && detached->town != target);
	/* Reuse the released ID through the actual founding command. */
	auto [reused, extra, reused_id] = CmdFoundTown(DoCommandFlag::Execute, Site(), TSZ_SMALL, false, TL_ORIGINAL, false, 0, "Reused town");
	Cost("delete-reuse", reused, extra, reused_id); fmt::print(stderr, "TOWN deletion-reuse {}\n", reused.Succeeded() && reused_id == removed);
	mode.Restore(); money.Restore(); other_money.Restore(); owner.Restore(); roads.Restore(); buildings.Restore(); rights.Restore(); bribes.Restore();
	SetGeneratingWorldProgress(GWP_GAME_START, 1);
	if (args.size() == 3) SaveOrLoad(std::string(args[2]), SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false);
	else {
		Backup<GameMode> cleanup_mode(_game_mode, GM_EDITOR); Backup<CompanyID> cleanup_owner(_current_company, OWNER_DEITY);
		while (Town::GetNumItems() != 0) {
			TownID last = (*Town::Iterate().begin())->index;
			CommandCost cost = CmdDeleteTown(DoCommandFlag::Execute, last); Cost("delete-final", cost); if (cost.Failed()) std::abort();
		}
		fmt::print(stderr, "TOWN final-town {} {}\n", Town::GetNumItems(), Object::GetNumItems());
		/* Keep the simulation valid after the final-town input. */
		auto recovery = CmdFoundTown(DoCommandFlag::Execute, Site(), TSZ_SMALL, false, TL_ORIGINAL, false, 0, "Recovery town");
		Cost("delete-final-recover", recovery); if (std::get<0>(recovery).Failed()) std::abort();
		cleanup_owner.Restore(); cleanup_mode.Restore();
	}
	return true;
}
#include "saveload/saveload.cpp"
#include "storage-console.inc"
/* File-loaded house definitions exercise production parsing, publication and growth. */
static constexpr uint32_t HOUSE_DEFINITION_GRF = 0x37373448; // H477 in file byte order.
static void HouseDefinitionNeed(bool value, const char *name)
{
	fmt::print(stderr, "TOWN definition {} {}\n", name, value);
	if (!value) std::abort();
}
static const HouseSpec *DefinitionSpec(uint16_t local, uint32_t grfid = HOUSE_DEFINITION_GRF)
{
	HouseID id = _house_mngr.GetID(local, grfid);
	HouseDefinitionNeed(id != INVALID_HOUSE_ID, "mapped");
	return HouseSpec::Get(id);
}
static void InspectHouseDefinitions()
{
	const auto *north = DefinitionSpec(0), *part = DefinitionSpec(1), *properties = DefinitionSpec(2);
	HouseDefinitionNeed(north->enabled && part->enabled && north->Index() + 1 == part->Index() && north->building_flags == BuildingFlags{BuildingFlag::Size1x2}, "multitile");
	HouseDefinitionNeed(north->population == 41 && part->population == 0 && part->building_availability.None(), "population-parts");
	HouseDefinitionNeed(north->grf_prop.subst_id == 7 && north->min_year == CalendarTime::MIN_YEAR && north->max_year == CalendarTime::MAX_YEAR, "substitute-early");
	HouseDefinitionNeed(HouseSpec::Get(7)->grf_prop.override_id == north->Index() && HouseSpec::Get(0)->grf_prop.override_id == INVALID_HOUSE_ID, "override-first");
	for (uint16_t local : {3, 5, 7, 9}) HouseDefinitionNeed(_house_mngr.GetID(local, HOUSE_DEFINITION_GRF) == INVALID_HOUSE_ID, "invalid-local");
	HouseDefinitionNeed(!DefinitionSpec(12)->enabled && DefinitionSpec(12)->building_flags.None() && DefinitionSpec(13)->enabled, "global-gap-flags");
	HouseDefinitionNeed(DefinitionSpec(10)->cargo_acceptance[2] == 0 && DefinitionSpec(10)->accepts_cargo[2] == INVALID_CARGO && DefinitionSpec(10)->min_year == CalendarTime::MAX_YEAR && DefinitionSpec(10)->max_year == TimerGameCalendar::Year{2070}, "signed-cargo-years");
	HouseDefinitionNeed(properties->population == 73 && properties->mail_generation == 29 && properties->callback_mask.base() == 0x1234 && properties->processing_time == 63 && properties->animation.speed == 2, "properties");
	HouseDefinitionNeed(properties->min_year == 2345 && properties->max_year == CalendarTime::MAX_YEAR && properties->watched_cargoes == (CargoTypes{1} << GetCargoTypeByLabel(CT_PASSENGERS) | CargoTypes{1} << GetCargoTypeByLabel(CT_MAIL)), "year-watch");
	HouseDefinitionNeed(properties->cargo_acceptance[0] == 7 && properties->accepts_cargo[0] == GetCargoTypeByLabel(CT_PASSENGERS) && properties->cargo_acceptance[1] == 9 && properties->accepts_cargo[2] == INVALID_CARGO && properties->cargo_acceptance[2] == 0, "variable-cargo");
	for (auto [grfid, name] : {std::pair{0x37373442U, "partial-colours"}, std::pair{0x37373443U, "partial-cargo"}}) {
		const auto *config = GetGRFConfig(grfid);
		HouseDefinitionNeed(config != nullptr && config->status == GCS_DISABLED && config->errors.size() == 1 && config->errors.front().message == STR_NEWGRF_ERROR_READ_BOUNDS, "read-error");
		const auto *spec = DefinitionSpec(0, grfid);
		if (grfid == 0x37373442U) HouseDefinitionNeed(spec->population == 77 && spec->random_colour[0] == COLOUR_PALE_GREEN && spec->random_colour[1] == COLOUR_PINK && spec->random_colour[2] == COLOUR_ORANGE && spec->random_colour[3] == COLOUR_GREEN, name);
		else HouseDefinitionNeed(spec->accepts_cargo[0] == GetCargoTypeByLabel(CT_MAIL) && spec->cargo_acceptance[0] == 8 && spec->accepts_cargo_label[0] == CT_PASSENGERS, name);
	}
	const auto *badge_spec = DefinitionSpec(0, 0x37373444U);
	const auto *partial = GetBadgeByLabel("house/partial"), *keep = GetBadgeByLabel("house/keep");
	HouseDefinitionNeed(GetGRFConfig(0x37373444U)->status == GCS_DISABLED && badge_spec->badges.size() == 1 && badge_spec->badges.front() == keep->index && partial->features.Test(GSF_HOUSES), "partial-badge-seen");
	const auto *oversized = GetGRFConfig(0x3737344cU);
	HouseDefinitionNeed(oversized->status == GCS_DISABLED && oversized->errors.front().message == STR_NEWGRF_ERROR_LIST_PROPERTY_TOO_LONG && oversized->errors.front().param_value[1] == 0x23 && DefinitionSpec(0, 0x3737344cU)->population == 53, "list-limit");
	const auto *invalid_id = GetGRFConfig(0x37373449U);
	HouseDefinitionNeed(invalid_id->status == GCS_DISABLED && invalid_id->errors.size() == 1 && invalid_id->errors.front().message == STR_NEWGRF_ERROR_INVALID_ID && DefinitionSpec(0, 0x37373449U)->population == 54, "invalid-id");
	for (HouseID id = 0; id < HouseSpec::Specs().size(); ++id) {
		const auto *spec = HouseSpec::Get(id);
		fmt::print(stderr, "TOWN definition-state {} {} {} {} {} {} {} {} {}\n", id, spec->enabled, spec->building_flags.base(), spec->building_availability.base(), spec->population, spec->min_year.base(), spec->max_year.base(), spec->grf_prop.override_id, spec->watched_cargoes);
		for (uint i = 0; i < HOUSE_NUM_ACCEPTS; ++i) fmt::print(stderr, "TOWN definition-cargo {} {} {} {}\n", id, i, spec->accepts_cargo[i], spec->cargo_acceptance[i]);
	}
}
static size_t OccupiedDefinitionTiles()
{
	size_t count = 0;
	for (TileIndex tile{}; tile < Map::Size(); ++tile) {
		if (!IsTileType(tile, MP_HOUSE)) continue;
		HouseID id = GetHouseType(tile);
		if (id != _house_mngr.GetID(0, HOUSE_DEFINITION_GRF) && id != _house_mngr.GetID(1, HOUSE_DEFINITION_GRF)) continue;
		++count;
		if (count == 1) {
			auto check_mapping = [&] {
				HouseResolverObject resolver(id, tile, Town::GetByTile(tile));
				bool available = true;
				HouseDefinitionNeed(resolver.house_scope.GetVariable(0x67, 0, available) == HOUSE_DEFINITION_GRF && available, "resolver-grfid");
				const uint32_t count = resolver.house_scope.GetVariable(0x61, HouseSpec::Get(id)->grf_prop.local_id, available);
				HouseDefinitionNeed((count & 0xFF) == std::min<uint>(255, Town::GetByTile(tile)->cache.building_counts.id_count[id]) && (count & 0xFF) > 0 && available, "resolver-local-id");
			};
			check_mapping();
			{
				HouseOverrideManager temporary(NEW_HOUSE_OFFSET, NUM_HOUSES, INVALID_HOUSE_ID);
				temporary.mappings[id] = {.grfid = 0xBAD, .entity_id = 0, .substitute_id = 0};
				check_mapping();
				HouseOverrideManager copied = temporary;
				check_mapping();
				copied = _house_mngr;
				check_mapping();
			}
			check_mapping();
		}
		CargoArray accepted{}, produced{}; CargoTypes always = 0;
		AddAcceptedCargo(tile, accepted, &always); AddProducedCargo(tile, produced);
		for (CargoType cargo = 0; cargo < NUM_CARGO; ++cargo) {
			if (accepted[cargo] != 0 || produced[cargo] != 0) fmt::print(stderr, "TOWN definition-tile-cargo {} {} {} {} {}\n", tile.base(), cargo, accepted[cargo], produced[cargo], always);
		}
		fmt::print(stderr, "TOWN definition-tile {} {} {} {}\n", tile.base(), id, GetTownIndex(tile).base(), GetHouseBuildingStage(tile));
	}
	return count;
}
/* R477 houses 0 and 1 differ only in their AllowConstruction answer (0 and 1). */
static constexpr uint32_t HOUSE_ALLOW_GRF = 0x37373452; // R477 in file byte order.
static std::pair<uint, uint> AllowConstructionTiles()
{
	const HouseID rejected = _house_mngr.GetID(0, HOUSE_ALLOW_GRF), allowed = _house_mngr.GetID(1, HOUSE_ALLOW_GRF);
	std::pair<uint, uint> counts{};
	for (TileIndex tile{}; tile < Map::Size(); ++tile) {
		if (!IsTileType(tile, MP_HOUSE)) continue;
		counts.first += GetHouseType(tile) == rejected;
		counts.second += GetHouseType(tile) == allowed;
	}
	return counts;
}
static bool RoadTypeGrowthScenario()
{
	/* TownCanGrowRoad permits a slower town-buildable type while GetTownRoadType prefers ROAD. */
	extern RoadTypeInfo _roadtypes[ROADTYPE_END];
	const RoadType alternate = static_cast<RoadType>(2);
	_roadtypes[alternate] = _roadtypes[ROADTYPE_ROAD];
	_roadtypes[alternate].label = 0x54475232;
	_roadtypes[alternate].max_speed = 1;
	_roadtypes[alternate].flags.Set(RoadTypeFlag::TownBuild);
	_roadtypes[alternate].powered_roadtypes.Set(alternate);
	_roadtypes[ROADTYPE_ROAD].powered_roadtypes.Set(alternate);
#ifdef WITH_RUST
	extern RoadTypes &_roadtypes_road;
#else
	extern RoadTypes _roadtypes_road;
#endif
	_roadtypes_road.Set(alternate);
	Backup<CompanyID> owner(_current_company, OWNER_DEITY);
	HouseDefinitionNeed(Town::GetNumItems() > 0, "road-growth-town");
	HouseDefinitionNeed(GetTownRoadType() == ROADTYPE_ROAD, "road-growth-selected");
	HouseDefinitionNeed(Command<CMD_CONVERT_ROAD>::Do(DoCommandFlag::Execute, TileXY(Map::SizeX() - 2, Map::SizeY() - 2), TileXY(1, 1), alternate, false).Succeeded(), "road-growth-conversion");
	std::vector<RoadBits> before(Map::Size(), ROAD_NONE);
	uint converted = 0;
	for (TileIndex tile : Map::Iterate()) {
		if (!IsTileType(tile, MP_ROAD) || !IsNormalRoad(tile)) continue;
		before[tile.base()] = GetRoadBits(tile, RTT_ROAD);
		converted += GetRoadTypeRoad(tile) == alternate;
	}
	HouseDefinitionNeed(converted > 0, "road-growth-input");
	fmt::print(stderr, "TOWN road-growth input {} {} {} {}\n", GetTownRoadType(), alternate, _roadtypes[alternate].flags.base(), converted);
	HouseDefinitionNeed(CmdExpandTown(DoCommandFlag::Execute, (*Town::Iterate().begin())->index, 128, {TownExpandMode::Roads}).Succeeded(), "road-growth-expansion");
	uint added = 0, changed = 0;
	for (TileIndex tile : Map::Iterate()) {
		if (!IsTileType(tile, MP_ROAD) || !IsNormalRoad(tile)) continue;
		RoadBits bits = GetRoadBits(tile, RTT_ROAD);
		added += before[tile.base()] == ROAD_NONE && bits != ROAD_NONE;
		changed += before[tile.base()] != ROAD_NONE && before[tile.base()] != bits;
	}
	HouseDefinitionNeed(added + changed > 0, "road-growth-physical-change");
	fmt::print(stderr, "TOWN road-growth output {} {} {} {} {} {}\n", added, changed, _random.state[0], _random.state[1], _interactive_random.state[0], _interactive_random.state[1]);
	owner.Restore();
	return true;
}
static bool HouseDefinitionsScenario(std::span<std::string_view> args)
{
	if (args[1] == "definitions-inspect") {
		InspectHouseDefinitions();
		HouseDefinitionNeed(OccupiedDefinitionTiles() > 0, "occupied-reload");
		fmt::print(stderr, "TOWN definition-reloaded true\n");
		return true;
	}
	if (args.size() < 4) return false;
	SetRandomSeed(0x47700001);
	_grfconfig.clear();
	for (const char *name : {"houses", "conflict", "colours", "cargo", "badges", "oversized", "invalid-id", "reject"}) {
		auto config = std::make_unique<GRFConfig>(std::string(args[2]) + "/" + name + ".grf");
		HouseDefinitionNeed(FillGRFDetails(*config, false, NO_DIRECTORY), "file-scan");
		config->flags.Reset(GRFConfigFlag::InitOnly);
		_grfconfig.push_back(std::move(config));
	}
	_house_mngr.mappings[124] = {.grfid = HOUSE_DEFINITION_GRF, .entity_id = 12, .substitute_id = 7};
	_house_mngr.mappings[126] = {.grfid = HOUSE_DEFINITION_GRF, .entity_id = 13, .substitute_id = 8};
	ReloadNewGRFData();
	InspectHouseDefinitions();
	for (uint16_t local : {0, 1}) {
		const auto *spec = DefinitionSpec(local, HOUSE_ALLOW_GRF);
		HouseDefinitionNeed(spec->enabled && spec->callback_mask.Test(HouseCallbackMask::AllowConstruction) && spec->min_year == CalendarTime::MIN_YEAR, "allow-construction-spec");
	}
	Backup<GameMode> mode(_game_mode, GM_EDITOR);
	Backup<CompanyID> owner(_current_company, OWNER_DEITY);
	Backup<bool> roads(_settings_game.economy.allow_town_roads, true);
	auto [cost, unused, town_id] = CmdFoundTown(DoCommandFlag::Execute, Site(), TSZ_LARGE, true, TL_2X2_GRID, false, 0, "Definition town");
	HouseDefinitionNeed(cost.Succeeded(), "founding");
	HouseDefinitionNeed(OccupiedDefinitionTiles() > 0, "natural-placement");
	const auto founded = AllowConstructionTiles();
	fmt::print(stderr, "TOWN definition-allow-construction founding {} {}\n", founded.first, founded.second);
	HouseDefinitionNeed(founded.first == 0 && founded.second > 0, "allow-construction-founding");
	Town *town = Town::Get(town_id);
	uint population = town->cache.population;
	HouseDefinitionNeed(CmdExpandTown(DoCommandFlag::Execute, town_id, 128, {TownExpandMode::Buildings, TownExpandMode::Roads}).Succeeded(), "expand-command");
	HouseDefinitionNeed(town->cache.population > population, "growth");
	const auto expanded = AllowConstructionTiles();
	fmt::print(stderr, "TOWN definition-allow-construction expansion {} {}\n", expanded.first, expanded.second);
	HouseDefinitionNeed(expanded.first == 0 && expanded.second > founded.second, "allow-construction-expansion");
	fmt::print(stderr, "TOWN definition-population {} {}\n", population, town->cache.population);
	HouseID before = _house_mngr.GetID(0, HOUSE_DEFINITION_GRF);
	ResetNewGRFData();
	HouseDefinitionNeed(_house_mngr.GetID(0, HOUSE_DEFINITION_GRF) == before && HouseSpec::Specs().size() == NEW_HOUSE_OFFSET, "reset-keeps-hids");
	ReloadNewGRFData();
	InspectHouseDefinitions();
	HouseDefinitionNeed(_house_mngr.GetID(0, HOUSE_DEFINITION_GRF) == before && OccupiedDefinitionTiles() > 0, "reload-keeps-hids");
	mode.Restore(); owner.Restore(); roads.Restore();
	fmt::print(stderr, "TOWN definition-rng {} {} {} {}\n", _random.state[0], _random.state[1], _interactive_random.state[0], _interactive_random.state[1]);
	HouseDefinitionNeed(SaveOrLoad(std::string(args[3]), SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false) == SL_OK, "save");
	std::vector<std::pair<TileIndex, HouseID>> substitutes;
	for (TileIndex tile{}; tile < Map::Size(); ++tile) {
		if (!IsTileType(tile, MP_HOUSE)) continue;
		HouseID id = GetCleanHouseType(tile);
		if (_house_mngr.GetGRFID(id) == HOUSE_DEFINITION_GRF) substitutes.emplace_back(tile, _house_mngr.GetSubstituteID(id));
	}
	std::erase_if(_grfconfig, [](const auto &config) { return config->ident.grfid == HOUSE_DEFINITION_GRF; });
	ReloadNewGRFData();
	HouseDefinitionNeed(!substitutes.empty() && _house_mngr.GetID(0, HOUSE_DEFINITION_GRF) == before, "missing-keeps-hids");
	/* GetHouseType applies any remaining GRF override to the stored substitute. */
	for (auto [tile, substitute] : substitutes) HouseDefinitionNeed(IsTileType(tile, MP_HOUSE) && GetCleanHouseType(tile) == substitute, "missing-substitute");
	fmt::print(stderr, "TOWN definition-missing-rng {} {} {} {}\n", _random.state[0], _random.state[1], _interactive_random.state[0], _interactive_random.state[1]);
	fmt::print(stderr, "TOWN definition-complete true\n");
	return true;
}
static bool TownsScenario(std::span<std::string_view> args)
{
	if (args.size() < 2) return false;
	if (args[1] == "road-types") return RoadTypeGrowthScenario();
	if (args[1].starts_with("definitions")) return HouseDefinitionsScenario(args);
	if (args[1].starts_with("storage")) return StorageScenario(args);
	if (args[1] == "authority" || args[1] == "authority-inspect") return CommandsScenario(args);
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
		try { if (!openttd_rust_town_generate(&services, nullptr, &frame, &frame.names, TL_ORIGINAL, true, phase == 0 ? UINT_MAX : 0, &result)) std::rethrow_exception(frame.cancellation); } catch (AbortGenerateWorldSignal &) { candidate = true; }
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
