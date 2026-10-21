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
	auto Layout = [](const char *name, std::initializer_list<size_t> values) { CHECK(AbiLayoutMismatch(name, values) == -1); };
#include "abi-layout-company_economy.inc"

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
