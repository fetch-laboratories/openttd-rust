/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
/** @file company_economy.cpp Native company layouts, network gaps and startup continuations. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#include "../3rdparty/fmt/format.h"
#ifdef WITH_RUST
#include "../company_func.h"
#include "../company_base.h"
#include "../economy_type.h"
#include "abi_layout.hpp"
#include "../rust/company_adapter.h"
#endif
#include "../safeguards.h"
#ifdef WITH_RUST
TEST_CASE("Company economy - canonical owner layouts and property copy lifetime")
{
	CHECK(AbiLayoutMismatch(210, {sizeof(CompanyEconomyEntry), alignof(CompanyEconomyEntry), offsetof(CompanyEconomyEntry, income), offsetof(CompanyEconomyEntry, expenses), offsetof(CompanyEconomyEntry, delivered_cargo), offsetof(CompanyEconomyEntry, performance_history), offsetof(CompanyEconomyEntry, company_value)}) == -1);
	CHECK(AbiLayoutMismatch(211, {sizeof(CompanyFinances), alignof(CompanyFinances), offsetof(CompanyFinances, money), offsetof(CompanyFinances, money_fraction), offsetof(CompanyFinances, current_loan), offsetof(CompanyFinances, max_loan), offsetof(CompanyFinances, block_preview), offsetof(CompanyFinances, months_empty), offsetof(CompanyFinances, months_of_bankruptcy), offsetof(CompanyFinances, bankrupt_asked), offsetof(CompanyFinances, bankrupt_timeout), offsetof(CompanyFinances, bankrupt_value), offsetof(CompanyFinances, terraform_limit), offsetof(CompanyFinances, clear_limit), offsetof(CompanyFinances, tree_limit), offsetof(CompanyFinances, build_object_limit), offsetof(CompanyFinances, yearly_expenses), offsetof(CompanyFinances, cur_economy), offsetof(CompanyFinances, old_economy), offsetof(CompanyFinances, num_valid_stat_ent)}) == -1);
	CHECK(AbiLayoutMismatch(212, {sizeof(Economy), alignof(Economy), offsetof(Economy, max_loan), offsetof(Economy, fluct), offsetof(Economy, interest_rate), offsetof(Economy, infl_amount), offsetof(Economy, infl_amount_pr), offsetof(Economy, inflation_prices), offsetof(Economy, inflation_payment), offsetof(Economy, old_max_loan_unround), offsetof(Economy, old_max_loan_unround_fract)}) == -1);
	CHECK(AbiLayoutMismatch(213, {sizeof(OpenTTDCompanyFinanceServices), alignof(OpenTTDCompanyFinanceServices), offsetof(OpenTTDCompanyFinanceServices, current_company), offsetof(OpenTTDCompanyFinanceServices, set_current_company), offsetof(OpenTTDCompanyFinanceServices, networking), offsetof(OpenTTDCompanyFinanceServices, company_finances), offsetof(OpenTTDCompanyFinanceServices, invalidate_company_windows), offsetof(OpenTTDCompanyFinanceServices, give_money_message), offsetof(OpenTTDCompanyFinanceServices, money_animation)}) == -1);
	CHECK(AbiLayoutMismatch(214, {sizeof(OpenTTDCompanyServices), alignof(OpenTTDCompanyServices), offsetof(OpenTTDCompanyServices, finance), offsetof(OpenTTDCompanyServices, shared), offsetof(OpenTTDCompanyServices, company_next), offsetof(OpenTTDCompanyServices, company_is_ai), offsetof(OpenTTDCompanyServices, company_count), offsetof(OpenTTDCompanyServices, can_allocate_company), offsetof(OpenTTDCompanyServices, local_company), offsetof(OpenTTDCompanyServices, network_server), offsetof(OpenTTDCompanyServices, company_vehicle_counts), offsetof(OpenTTDCompanyServices, vehicle_limits), offsetof(OpenTTDCompanyServices, company_admin_update), offsetof(OpenTTDCompanyServices, company_in_trouble), offsetof(OpenTTDCompanyServices, post_company_delete), offsetof(OpenTTDCompanyServices, update_company_hq), offsetof(OpenTTDCompanyServices, performance_detail_dirty), offsetof(OpenTTDCompanyServices, company_graphs_dirty), offsetof(OpenTTDCompanyServices, company_infrastructure), offsetof(OpenTTDCompanyServices, rail_maintenance_cost), offsetof(OpenTTDCompanyServices, signal_maintenance_cost), offsetof(OpenTTDCompanyServices, road_maintenance_cost), offsetof(OpenTTDCompanyServices, canal_maintenance_cost), offsetof(OpenTTDCompanyServices, station_maintenance_cost), offsetof(OpenTTDCompanyServices, airport_maintenance_cost), offsetof(OpenTTDCompanyServices, recession_news), offsetof(OpenTTDCompanyServices, price_base), offsetof(OpenTTDCompanyServices, cargo_next), offsetof(OpenTTDCompanyServices, set_cargo_payment), offsetof(OpenTTDCompanyServices, price_windows_dirty), offsetof(OpenTTDCompanyServices, industry_daily_changes), offsetof(OpenTTDCompanyServices, clear_cargo_monitors), offsetof(OpenTTDCompanyServices, clear_all_cargo_monitors), offsetof(OpenTTDCompanyServices, generate_company_name), offsetof(OpenTTDCompanyServices, ask_merger), offsetof(OpenTTDCompanyServices, is_interactive_company), offsetof(OpenTTDCompanyServices, show_buy_company), offsetof(OpenTTDCompanyServices, script_random_next), offsetof(OpenTTDCompanyServices, reset_competitor_timeout), offsetof(OpenTTDCompanyServices, finances_dirty), offsetof(OpenTTDCompanyServices, show_company_finances), offsetof(OpenTTDCompanyServices, new_year_sound), offsetof(OpenTTDCompanyServices, generate_company_colour), offsetof(OpenTTDCompanyServices, allocate_company), offsetof(OpenTTDCompanyServices, set_company_colour), offsetof(OpenTTDCompanyServices, setup_new_company), offsetof(OpenTTDCompanyServices, new_company_events), offsetof(OpenTTDCompanyServices, company_league_dirty), offsetof(OpenTTDCompanyServices, company_ctrl_windows), offsetof(OpenTTDCompanyServices, close_network_status), offsetof(OpenTTDCompanyServices, network_spectate), offsetof(OpenTTDCompanyServices, network_company_new), offsetof(OpenTTDCompanyServices, network_own_company), offsetof(OpenTTDCompanyServices, assert_new_ai_slot), offsetof(OpenTTDCompanyServices, company_bankrupt_news), offsetof(OpenTTDCompanyServices, stop_ai), offsetof(OpenTTDCompanyServices, delete_company), offsetof(OpenTTDCompanyServices, company_removed), offsetof(OpenTTDCompanyServices, merger_news), offsetof(OpenTTDCompanyServices, acquisition_windows), offsetof(OpenTTDCompanyServices, clients_to_spectators), offsetof(OpenTTDCompanyServices, set_local_company), offsetof(OpenTTDCompanyServices, subsidy_next_awarded), offsetof(OpenTTDCompanyServices, delete_subsidy), offsetof(OpenTTDCompanyServices, set_subsidy_awarded), offsetof(OpenTTDCompanyServices, rebuild_subsidy_cache), offsetof(OpenTTDCompanyServices, town_next), offsetof(OpenTTDCompanyServices, town_rating), offsetof(OpenTTDCompanyServices, set_town_rating), offsetof(OpenTTDCompanyServices, set_town_exclusivity), offsetof(OpenTTDCompanyServices, vehicle_next_owned), offsetof(OpenTTDCompanyServices, aircraft_is_normal), offsetof(OpenTTDCompanyServices, vehicle_value), offsetof(OpenTTDCompanyServices, vehicle_is_primary), offsetof(OpenTTDCompanyServices, vehicle_profit), offsetof(OpenTTDCompanyServices, vehicle_previous), offsetof(OpenTTDCompanyServices, vehicle_group_flags), offsetof(OpenTTDCompanyServices, vehicle_service_interval_is_custom), offsetof(OpenTTDCompanyServices, delete_vehicle), offsetof(OpenTTDCompanyServices, count_group_engine), offsetof(OpenTTDCompanyServices, count_group_vehicle), offsetof(OpenTTDCompanyServices, reset_service_interval), offsetof(OpenTTDCompanyServices, set_vehicle_owner), offsetof(OpenTTDCompanyServices, assign_unit_number), offsetof(OpenTTDCompanyServices, remove_engine_replacements), offsetof(OpenTTDCompanyServices, group_next_owned), offsetof(OpenTTDCompanyServices, delete_group), offsetof(OpenTTDCompanyServices, transfer_group), offsetof(OpenTTDCompanyServices, copy_service_interval_defaults), offsetof(OpenTTDCompanyServices, update_autoreplace), offsetof(OpenTTDCompanyServices, map_size), offsetof(OpenTTDCompanyServices, change_tile_owner), offsetof(OpenTTDCompanyServices, signal_tile), offsetof(OpenTTDCompanyServices, has_signal_on_track), offsetof(OpenTTDCompanyServices, add_track_to_signal_buffer), offsetof(OpenTTDCompanyServices, update_signals_in_buffer), offsetof(OpenTTDCompanyServices, add_airport_infrastructure), offsetof(OpenTTDCompanyServices, station_next_owned), offsetof(OpenTTDCompanyServices, station_facility_count), offsetof(OpenTTDCompanyServices, station_times), offsetof(OpenTTDCompanyServices, set_station_owner), offsetof(OpenTTDCompanyServices, waypoint_next_owned), offsetof(OpenTTDCompanyServices, set_waypoint_owner), offsetof(OpenTTDCompanyServices, sign_next_owned), offsetof(OpenTTDCompanyServices, set_sign_owner), offsetof(OpenTTDCompanyServices, goal_next_owned), offsetof(OpenTTDCompanyServices, delete_goal), offsetof(OpenTTDCompanyServices, story_page_next_owned), offsetof(OpenTTDCompanyServices, delete_story_page), offsetof(OpenTTDCompanyServices, change_window_owner), offsetof(OpenTTDCompanyServices, mark_whole_screen_dirty)}) == -1);
	CHECK(AbiLayoutMismatch(400, {sizeof(OpenTTDCompanyRef), alignof(OpenTTDCompanyRef), offsetof(OpenTTDCompanyRef, finances), offsetof(OpenTTDCompanyRef, id)}) == -1);
	CHECK(AbiLayoutMismatch(401, {sizeof(OpenTTDCompanyVehicleCounts), alignof(OpenTTDCompanyVehicleCounts), offsetof(OpenTTDCompanyVehicleCounts, trains), offsetof(OpenTTDCompanyVehicleCounts, road), offsetof(OpenTTDCompanyVehicleCounts, ships), offsetof(OpenTTDCompanyVehicleCounts, aircraft)}) == -1);
	CHECK(AbiLayoutMismatch(402, {sizeof(OpenTTDCompanyInfrastructure), alignof(OpenTTDCompanyInfrastructure), offsetof(OpenTTDCompanyInfrastructure, rail), offsetof(OpenTTDCompanyInfrastructure, road), offsetof(OpenTTDCompanyInfrastructure, rail_total), offsetof(OpenTTDCompanyInfrastructure, road_total), offsetof(OpenTTDCompanyInfrastructure, tram_total), offsetof(OpenTTDCompanyInfrastructure, signal), offsetof(OpenTTDCompanyInfrastructure, water), offsetof(OpenTTDCompanyInfrastructure, station), offsetof(OpenTTDCompanyInfrastructure, road_is_road)}) == -1);
	CHECK(AbiLayoutMismatch(403, {sizeof(OpenTTDCompanyVehicleGroupFlags), alignof(OpenTTDCompanyVehicleGroupFlags), offsetof(OpenTTDCompanyVehicleGroupFlags, engine_countable), offsetof(OpenTTDCompanyVehicleGroupFlags, primary)}) == -1);
	CHECK(AbiLayoutMismatch(404, {sizeof(OpenTTDCompanyVehicleProfit), alignof(OpenTTDCompanyVehicleProfit), offsetof(OpenTTDCompanyVehicleProfit, profit_last_year), offsetof(OpenTTDCompanyVehicleProfit, economy_age)}) == -1);
	CHECK(AbiLayoutMismatch(405, {sizeof(OpenTTDCompanyStationTimes), alignof(OpenTTDCompanyStationTimes), offsetof(OpenTTDCompanyStationTimes, since_load), offsetof(OpenTTDCompanyStationTimes, since_unload)}) == -1);
	CHECK(AbiLayoutMismatch(406, {sizeof(OpenTTDCompanyTownRights), alignof(OpenTTDCompanyTownRights), offsetof(OpenTTDCompanyTownRights, rating), offsetof(OpenTTDCompanyTownRights, have_ratings), offsetof(OpenTTDCompanyTownRights, exclusive_counter), offsetof(OpenTTDCompanyTownRights, exclusivity)}) == -1);
	CHECK(AbiLayoutMismatch(407, {sizeof(OpenTTDCompanySignalTile), alignof(OpenTTDCompanySignalTile), offsetof(OpenTTDCompanySignalTile, railway), offsetof(OpenTTDCompanySignalTile, crossing), offsetof(OpenTTDCompanySignalTile, owner), offsetof(OpenTTDCompanySignalTile, has_signals), offsetof(OpenTTDCompanySignalTile, tracks)}) == -1);
	CHECK(AbiLayoutMismatch(408, {sizeof(OpenTTDCompanyPriceBase), alignof(OpenTTDCompanyPriceBase), offsetof(OpenTTDCompanyPriceBase, start_price), offsetof(OpenTTDCompanyPriceBase, category)}) == -1);
	CHECK(AbiLayoutMismatch(409, {sizeof(OpenTTDCompanyCost), alignof(OpenTTDCompanyCost), offsetof(OpenTTDCompanyCost, cost), offsetof(OpenTTDCompanyCost, param), offsetof(OpenTTDCompanyCost, error), offsetof(OpenTTDCompanyCost, expense)}) == -1);
	CHECK(AbiLayoutMismatch(410, {sizeof(OpenTTDCompanyStartup), alignof(OpenTTDCompanyStartup), offsetof(OpenTTDCompanyStartup, company), offsetof(OpenTTDCompanyStartup, requested), offsetof(OpenTTDCompanyStartup, is_ai), offsetof(OpenTTDCompanyStartup, stage)}) == -1);
	CHECK(AbiLayoutMismatch(411, {sizeof(OpenTTDCompanyControl), alignof(OpenTTDCompanyControl), offsetof(OpenTTDCompanyControl, result), offsetof(OpenTTDCompanyControl, startup), offsetof(OpenTTDCompanyControl, client), offsetof(OpenTTDCompanyControl, action), offsetof(OpenTTDCompanyControl, target), offsetof(OpenTTDCompanyControl, reason), offsetof(OpenTTDCompanyControl, execute), offsetof(OpenTTDCompanyControl, stage)}) == -1);
	CHECK(AbiLayoutMismatch(412, {sizeof(OpenTTDCompanyTick), alignof(OpenTTDCompanyTick), offsetof(OpenTTDCompanyTick, finances), offsetof(OpenTTDCompanyTick, num_companies), offsetof(OpenTTDCompanyTick, index), offsetof(OpenTTDCompanyTick, timeout), offsetof(OpenTTDCompanyTick, interval), offsetof(OpenTTDCompanyTick, editor), offsetof(OpenTTDCompanyTick, named), offsetof(OpenTTDCompanyTick, competitor_due), offsetof(OpenTTDCompanyTick, networking), offsetof(OpenTTDCompanyTick, max_companies), offsetof(OpenTTDCompanyTick, max_competitors), offsetof(OpenTTDCompanyTick, num_ais), offsetof(OpenTTDCompanyTick, stage)}) == -1);
	CHECK(AbiLayoutMismatch(413, {sizeof(OpenTTDCompanyCompetitors), alignof(OpenTTDCompanyCompetitors), offsetof(OpenTTDCompanyCompetitors, num_companies), offsetof(OpenTTDCompanyCompetitors, interval), offsetof(OpenTTDCompanyCompetitors, menu), offsetof(OpenTTDCompanyCompetitors, can_start), offsetof(OpenTTDCompanyCompetitors, networking), offsetof(OpenTTDCompanyCompetitors, max_companies), offsetof(OpenTTDCompanyCompetitors, max_competitors)}) == -1);
	CHECK(AbiLayoutMismatch(414, {sizeof(OpenTTDEconomyMonth), alignof(OpenTTDEconomyMonth), offsetof(OpenTTDEconomyMonth, month), offsetof(OpenTTDEconomyMonth, infinite_money), offsetof(OpenTTDEconomyMonth, maintenance), offsetof(OpenTTDEconomyMonth, fluctuating)}) == -1);
	CHECK(AbiLayoutMismatch(415, {sizeof(OpenTTDCompanyYear), alignof(OpenTTDCompanyYear), offsetof(OpenTTDCompanyYear, local_finances), offsetof(OpenTTDCompanyYear, local), offsetof(OpenTTDCompanyYear, show_finances), offsetof(OpenTTDCompanyYear, new_year_sound)}) == -1);
	CHECK(AbiLayoutMismatch(416, {sizeof(OpenTTDEconomySettings), alignof(OpenTTDEconomySettings), offsetof(OpenTTDEconomySettings, year), offsetof(OpenTTDEconomySettings, max_loan), offsetof(OpenTTDEconomySettings, initial_interest), offsetof(OpenTTDEconomySettings, vehicle_costs), offsetof(OpenTTDEconomySettings, construction_cost), offsetof(OpenTTDEconomySettings, inflation)}) == -1);
	CHECK(AbiLayoutMismatch(417, {sizeof(OpenTTDCompanyLandscaping), alignof(OpenTTDCompanyLandscaping), offsetof(OpenTTDCompanyLandscaping, terraform_per_64k), offsetof(OpenTTDCompanyLandscaping, clear_per_64k), offsetof(OpenTTDCompanyLandscaping, tree_per_64k), offsetof(OpenTTDCompanyLandscaping, object_per_64k), offsetof(OpenTTDCompanyLandscaping, terraform_burst), offsetof(OpenTTDCompanyLandscaping, clear_burst), offsetof(OpenTTDCompanyLandscaping, tree_burst), offsetof(OpenTTDCompanyLandscaping, object_burst)}) == -1);
	CompanyProperties original;
	original.Finances().money = INT64_MIN;
	original.Finances().old_economy[23].delivered_cargo[63] = UINT32_MAX;
	auto *address = &original.Finances().old_economy[23].delivered_cargo[63];
	CompanyProperties copy = original;
	CHECK(&copy.Finances() != &original.Finances());
	CHECK(copy.Finances().money == Money(INT64_MIN));
	CHECK(copy.Finances().old_economy[23].delivered_cargo[63] == UINT32_MAX);
	copy.Finances().money = 17;
	original = copy;
	CHECK(original.Finances().money == Money(17));
	CHECK(address == &original.Finances().old_economy[23].delivered_cargo[63]);
	copy.Finances().money = 19;
	CHECK(original.Finances().money == Money(17));
}

/* Semantic saves cannot witness a client waiting for a server-posted deletion or
 * a script memory-policy exception during AI startup. These fixtures copy the
 * game's service table and replace only the world services the checked paths
 * use; they are not an oracle for ordinary financial calculations. */
static std::array<std::unique_ptr<CompanyProperties>, 3> fixture_companies;
static bool fixture_networking, fixture_server, fixture_is_ai;
static uint8_t fixture_current;
static std::vector<std::string> fixture_trace;
static OpenTTDCompanyFinanceServices fixture_finance;
static OpenTTDCompanyServices fixture_services;
static uint8_t FixtureCurrent() noexcept { return fixture_current; }
static void FixtureSetCurrent(uint8_t id) noexcept { fixture_current = id; }
static bool FixtureNetworking() noexcept { return fixture_networking; }
static bool FixtureServer() noexcept { return fixture_server; }
static bool FixtureIsAI(uint8_t) noexcept { return fixture_is_ai; }
static uint8_t FixtureLocal() noexcept { return 0; }
static CompanyFinances *FixtureFinances(uint8_t id) noexcept { return id < fixture_companies.size() && fixture_companies[id] != nullptr ? &fixture_companies[id]->Finances() : nullptr; }
static OpenTTDCompanyRef FixtureNext(uint32_t from) noexcept
{
	for (uint32_t i = from; i < fixture_companies.size(); i++) if (fixture_companies[i] != nullptr) return {&fixture_companies[i]->Finances(), i};
	return {nullptr, UINT32_MAX};
}
static void FixtureNoop(uint8_t) noexcept {}
static void FixtureAdminUpdate(uint8_t id) noexcept { fixture_trace.push_back(fmt::format("admin {}", id)); }
static void FixturePostDelete(uint8_t id) noexcept
{
	fixture_trace.push_back(fmt::format("post {}", id));
	if (!fixture_networking) fixture_companies[id].reset(); // actual singleplayer Post deletes synchronously
}
static uint32_t FixtureRandomNext(uint32_t) noexcept { return 0; }
static void FixtureTimeout(uint32_t ticks) noexcept { fixture_trace.push_back(fmt::format("timeout {}", ticks)); }
static bool FixtureCanAllocate() noexcept { return true; }
static uint8_t FixtureColour() noexcept { return 3; }
static OpenTTDCompanyRef FixtureAllocate(uint8_t, bool) noexcept
{
	fixture_companies[2] = std::make_unique<CompanyProperties>();
	fixture_trace.push_back("allocate 2");
	return {&fixture_companies[2]->Finances(), 2};
}
static void FixtureColourSet(uint8_t, uint8_t) noexcept {}
static void FixtureSetup(uint8_t id, bool) noexcept { fixture_trace.push_back(fmt::format("setup {}", id)); }
static void FixtureEvents(uint8_t id) noexcept { fixture_trace.push_back(fmt::format("events {}", id)); }
static void FixtureVoid() noexcept {}
static void FixtureNetworkNew(uint8_t id, uint32_t, bool) noexcept { fixture_trace.push_back(fmt::format("network {}", id)); }
static void FixtureReset(bool networking, bool server)
{
	fixture_networking = networking; fixture_server = server; fixture_is_ai = false; fixture_current = 18; fixture_trace.clear();
	fixture_finance = GetRustCompanyFinanceServices();
	fixture_finance.current_company = FixtureCurrent; fixture_finance.set_current_company = FixtureSetCurrent;
	fixture_finance.networking = FixtureNetworking; fixture_finance.company_finances = FixtureFinances;
	fixture_finance.invalidate_company_windows = FixtureNoop;
	fixture_services = GetRustCompanyServices();
	fixture_services.finance = &fixture_finance;
	fixture_services.company_next = FixtureNext; fixture_services.company_is_ai = FixtureIsAI;
	fixture_services.local_company = FixtureLocal; fixture_services.network_server = FixtureServer;
	fixture_services.company_admin_update = FixtureAdminUpdate; fixture_services.post_company_delete = FixturePostDelete;
	fixture_services.script_random_next = FixtureRandomNext; fixture_services.reset_competitor_timeout = FixtureTimeout;
	fixture_services.can_allocate_company = FixtureCanAllocate; fixture_services.generate_company_colour = FixtureColour;
	fixture_services.allocate_company = FixtureAllocate; fixture_services.set_company_colour = FixtureColourSet;
	fixture_services.setup_new_company = FixtureSetup; fixture_services.new_company_events = FixtureEvents;
	fixture_services.company_league_dirty = FixtureVoid; fixture_services.company_ctrl_windows = FixtureVoid;
	fixture_services.assert_new_ai_slot = FixtureNoop; fixture_services.network_company_new = FixtureNetworkNew;
}
TEST_CASE("Company economy - network deferred deletion and synchronous pool iteration")
{
	for (int mode : {0, 1, 2}) {
		FixtureReset(mode != 0, mode == 1);
		auto *economy = new (openttd_rust_economy_state()) Economy{}; economy->max_loan = 300000; economy->fluct = 100;
		for (auto &p : fixture_companies) p = std::make_unique<CompanyProperties>();
		fixture_companies[0]->Finances().money = 1000000;
		for (uint32_t i : {1U, 2U}) {
			fixture_companies[i]->Finances().money = -500000;
			fixture_companies[i]->Finances().current_loan = 100000;
			fixture_companies[i]->Finances().months_of_bankruptcy = 9;
		}
		const OpenTTDEconomyMonth month{.month = 1, .infinite_money = false, .maintenance = false, .fluctuating = false};
		openttd_rust_economy_month(&fixture_services, &month);
		CHECK(fixture_current == 18);
		if (mode == 2) CHECK(fixture_trace == std::vector<std::string>{"admin 1", "admin 2"});
		else CHECK(fixture_trace == std::vector<std::string>{"post 1", "post 2"});
		for (uint32_t i : {1U, 2U}) CHECK((fixture_companies[i] == nullptr) == (mode == 0));
	}
	for (auto &p : fixture_companies) p.reset();
}
TEST_CASE("Company economy - competitor posts return to the caller before each Post")
{
	FixtureReset(false, false);
	const OpenTTDCompanyCompetitors competitors{.num_companies = 0, .interval = 1, .menu = false, .can_start = true, .networking = false, .max_companies = 15, .max_competitors = 1};
	CHECK(openttd_rust_company_competitor_timeout(&fixture_services, &competitors));
	OpenTTDCompanyTick tick{.finances = nullptr, .num_companies = 0, .index = 0, .timeout = 0, .interval = 0, .editor = false, .named = false,
		.competitor_due = true, .networking = false, .max_companies = 15, .max_competitors = 2, .num_ais = 0, .stage = 0};
	uint32_t cursor = *openttd_rust_company_tick();
	int posts = 0;
	while (openttd_rust_company_on_tick(&fixture_services, &tick)) {
		CHECK(fixture_trace.empty()); // the timer is reset only after the last Post
		posts++;
	}
	CHECK(posts == 2);
	CHECK(fixture_trace == std::vector<std::string>{"timeout 19425"});
	CHECK(*openttd_rust_company_tick() == (cursor + 1) % MAX_COMPANIES);
}
/* AIInstance::Initialize can raise Script_FatalError for the configured script
 * memory policy before ScriptInstance::Initialize's catch. Rust returns before
 * AI::StartNew; if it throws the frame is abandoned with the original partial
 * effects (allocated company, no new-company events, no network announcement). */
TEST_CASE("Company economy - AI startup continuation and abandoned frames")
{
	FixtureReset(false, false);
	OpenTTDCompanyControl abandoned{.result = {}, .startup = {}, .client = 0, .action = static_cast<uint8_t>(CCA_NEW_AI), .target = 255, .reason = 0, .execute = true, .stage = 0};
	REQUIRE(openttd_rust_company_control(&fixture_services, &abandoned));
	CHECK(abandoned.startup.company == 2);
	CHECK(fixture_trace == std::vector<std::string>{"allocate 2", "setup 2"});
	CHECK(fixture_companies[2]->Finances().money == fixture_companies[2]->Finances().current_loan);

	FixtureReset(false, false);
	OpenTTDCompanyControl resumed{.result = {}, .startup = {}, .client = 0, .action = static_cast<uint8_t>(CCA_NEW_AI), .target = 255, .reason = 0, .execute = true, .stage = 0};
	int starts = 0;
	while (openttd_rust_company_control(&fixture_services, &resumed)) starts++;
	CHECK(starts == 1);
	CHECK(fixture_trace == std::vector<std::string>{"allocate 2", "setup 2", "events 2", "network 2"});
	CHECK(resumed.result.error == 0);

	FixtureReset(true, false); // network clients do not start AIs
	OpenTTDCompanyStartup client{.company = UINT32_MAX, .requested = 255, .is_ai = true, .stage = 0};
	CHECK_FALSE(openttd_rust_company_startup(&fixture_services, &client));
	CHECK(fixture_trace == std::vector<std::string>{"allocate 2", "setup 2", "events 2"});
	fixture_companies[2].reset();
}
/* AI command APIs cannot select deity/non-company owners. The original wider
 * CompanyMask test is defined through bit 63; compare those special owners and
 * also check the inactive/new/spectator sentinels without an oversized shift. */
TEST_CASE("Company economy - special owners have no bankruptcy takeover offer")
{
	FixtureReset(true, false);
	fixture_companies[1] = std::make_unique<CompanyProperties>();
	for (uint16_t mask : {uint16_t{0}, uint16_t{UINT16_MAX}}) {
		fixture_companies[1]->Finances().bankrupt_asked = CompanyMask{};
		if (mask != 0) fixture_companies[1]->Finances().bankrupt_asked.Set();
		for (uint32_t current = 16; current <= UINT8_MAX; current++) {
			fixture_current = static_cast<uint8_t>(current);
			if (current < 64) CHECK_FALSE(fixture_companies[1]->Finances().bankrupt_asked.Test(CompanyID(static_cast<uint8_t>(current))));
			for (bool hostile : {false, true}) {
				CHECK(openttd_rust_company_buy(&fixture_services, 1, hostile, false).error == 1);
			}
		}
	}
	fixture_companies[1].reset();
}
#endif /* WITH_RUST */
