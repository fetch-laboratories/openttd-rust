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
#include "../../tools/simulation/session-fixture.hpp"
#include "../rust/clock_ffi.h"
#include "../company_func.h"
#include "../company_base.h"
#include "../economy_type.h"
#include "abi_layout.hpp"
#include "../rust/company_adapter.h"
#include "../rust/company_identity_ffi.h"
#include "../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDCompanyIdentityServices.hpp"
#include "abi-visit-OpenTTDFaceServices.hpp"
#include "../company_manager_face.h"
#include <thread>
#include "../rust/tile_owner_services.h"
#include "../rust/command_services.h"
#include "../core/backup_type.hpp"
#include "../strings_func.h"
#include "../table/strings.h"
#include "../road.h"
#include "../settings_type.h"
#include "../tile_map.h"
#include "../openttd.h"
#include "../rust/movement_ffi.h"
#include "../rust/objectives_ffi.h"
#include "../window_func.h"
#include "../network/network_type.h"
#include "../safeguards.h"
TEST_CASE("Company economy - canonical owner layouts and property copy lifetime")
{
	ABI_LAYOUT_CHECK;
#include "abi-layout-company_economy.inc"
	CHECK(AbiServicesComplete(GetRustCompanyIdentityServices()));
	CHECK(AbiServicesComplete(GetRustFaceServices()));

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

/* These entries read real pool/settings storage and preserve native error objects. */
TEST_CASE("Company control - human preference, allocation credit, defaults and encoded owners")
{
	REQUIRE(Company::GetNumItems() == 0);
	AutoRestoreBackup<CompanyID> current(_current_company, CompanyID(0));
	AutoRestoreBackup<CompanyID> local(_local_company, CompanyID(0));
	AutoRestoreBackup<VehicleDefaultSettings> client(_settings_client.company.vehicle, {});
	AutoRestoreBackup<GameMode> mode(_game_mode, GM_MENU);
	std::array<std::unique_ptr<Company>, MAX_COMPANIES> companies;
	REQUIRE(Company::CanAllocateItem());
	companies[3].reset(new (CompanyID(3)) Company(STR_SV_UNNAMED, false));
	CHECK(GetFirstPlayableCompanyID() == CompanyID(3)); // human wins before hole zero
	companies[3]->is_ai = true;
	auto vacant = GetFirstPlayableCompanyID();
	CHECK(vacant == CompanyID(0));
	companies[0].reset(new (vacant) Company(STR_SV_UNNAMED, true)); // consumes the selector's allocation credit
	for (uint8_t i = 1; i < MAX_COMPANIES; ++i) if (i != 3) {
		REQUIRE(Company::CanAllocateItem());
		companies[i].reset(new (CompanyID(i)) Company(STR_SV_UNNAMED, true));
	}
	CHECK(GetFirstPlayableCompanyID() == CompanyID::Begin()); // full AI pool
	companies[9]->is_ai = false; companies[12]->is_ai = false;
	CHECK(GetFirstPlayableCompanyID() == CompanyID(9));
	companies[1].reset();
	CHECK(GetFirstPlayableCompanyID() == CompanyID(9));
	companies[9]->is_ai = true; companies[12]->is_ai = true;
	CHECK(GetFirstPlayableCompanyID() == CompanyID(1));

	auto &supplied = companies[3]->settings.vehicle;
	for (bool percent : {false, true}) {
		supplied = {.servint_ispercent = percent, .servint_trains = 0, .servint_roadveh = 121, .servint_aircraft = 65535, .servint_ships = 423};
		_settings_client.company.vehicle = {.servint_ispercent = !percent, .servint_trains = 224, .servint_roadveh = 0, .servint_aircraft = 426, .servint_ships = 65535};
		for (auto [type, company_interval, client_interval] : {std::tuple{VEH_TRAIN, 0, 224}, {VEH_ROAD, 121, 0}, {VEH_AIRCRAFT, 65535, 426}, {VEH_SHIP, 423, 65535}}) {
			CHECK(CompanyServiceInterval(companies[3].get(), type) == company_interval);
			CHECK(CompanyServiceInterval(nullptr, type) == client_interval);
		}
	}
	for (CompanyID observer : {CompanyID(0), COMPANY_SPECTATOR}) {
		_local_company = observer;
		CHECK(CheckOwnership(CompanyID(0)).Succeeded());
		for (Owner owner : {CompanyID(3), OWNER_NONE}) {
			auto error = CheckOwnership(owner);
			CHECK(error.GetErrorMessage() == STR_ERROR_OWNED_BY);
			CHECK(error.GetErrorOwner() == (observer == 0 ? owner : INVALID_OWNER));
			EncodedString expected;
			if (observer == 0) expected = owner == 3 ? GetEncodedString(STR_ERROR_OWNED_BY, STR_COMPANY_NAME, CompanyID(3)) : GetEncodedString(STR_ERROR_OWNED_BY, STR_COMPANY_SOMEONE, std::monostate{});
			CHECK(error.GetEncodedMessage() == expected);
		}
	}
	Map::Allocate(64, 64);
	TileIndex tile = TileXY(20, 20);
	SetTileType(tile, MP_CLEAR); SetTileOwner(tile, CompanyID(3));
	_local_company = CompanyID(0);
	auto tile_error = CheckTileOwnership(tile), owner_error = CheckOwnership(CompanyID(3), tile);
	CHECK(tile_error.GetEncodedMessage() == owner_error.GetEncodedMessage());
	CHECK(tile_error.GetErrorOwner() == CompanyID(3));
}

/* The native fatal operation is replaced only to observe both failed preconditions. */
TEST_CASE("Company ownership - both original assertions precede error selection")
{
	static std::vector<bool> failed;
	static OpenTTDMovementServices movement;
	movement = GetRustMovementServices();
	movement.native_assert = [](bool condition) noexcept { failed.push_back(condition); };
	auto shared = GetRustSharedServices();
	shared.movement = &movement;
	auto services = GetRustCompanyServices(); services.shared = &shared;
	AutoRestoreBackup<CompanyID> current(_current_company, CompanyID(0)), local(_local_company, COMPANY_SPECTATOR);
	for (auto [owner, tile] : {std::pair{OWNER_END, TileIndex(1)}, {OWNER_TOWN, TileIndex(0)}}) {
		failed.clear(); CommandCost cost;
		openttd_rust_company_check_ownership(&services, RustCommandCost(&cost), owner.base(), tile.base());
		CHECK(failed == std::vector<bool>{false});
		CHECK(cost.GetErrorMessage() == STR_ERROR_OWNED_BY);
		CHECK(cost.GetEncodedMessage().empty());
	}
}

TEST_CASE("Company identity - process-wide colour alias and face bit truncation")
{
	AutoRestoreBackup<Colours> colour(_company_colours[CompanyID(0)], _company_colours[CompanyID(0)]);
	CHECK(reinterpret_cast<uint8_t *>(&_company_colours[CompanyID(0)]) == openttd_rust_company_colours());
	std::thread worker([] { _company_colours[CompanyID(0)] = COLOUR_RED; });
	worker.join();
	CHECK(openttd_rust_company_colours()[0] == COLOUR_RED);

	FaceVar var{.type = FaceVarType::Sprite, .position = 0, .offset = 3, .length = 5, .valid_values = 6, .data = SpriteID(0)};
	CompanyManagerFace face{0, UINT32_MAX, {}};
	CHECK(var.GetBits(face) == 31);
	CHECK(var.ScaleBits(31) == 5);
	var.ChangeBits(face, 0);
	CHECK(var.GetBits(face) == 1);
	face.bits = UINT32_MAX;
	var.ChangeBits(face, 127);
	CHECK(var.GetBits(face) == 4);
	var.SetBits(face, 2);
	CHECK(var.GetBits(face) == 2);
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
static OpenTTDCompanyIdentityServices fixture_identity;
static OpenTTDSharedServices fixture_identity_shared;
static OpenTTDFaceServices fixture_faces;
static CompanyProperties *FixtureProperties(const Company *c) noexcept { return reinterpret_cast<CompanyProperties *>(const_cast<Company *>(c)); }
static uint8_t FixtureCurrent() noexcept { return fixture_current; }
static void FixtureSetCurrent(uint8_t id) noexcept { fixture_current = id; }
static bool FixtureNetworking() noexcept { return fixture_networking; }
static bool FixtureServer() noexcept { return fixture_server; }
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
static OpenTTDCompanyRef FixtureAllocate(uint8_t, bool) noexcept
{
	fixture_companies[2] = std::make_unique<CompanyProperties>();
	fixture_trace.push_back("allocate 2");
	return {&fixture_companies[2]->Finances(), 2};
}
static bool FixturePrepare(uint8_t id, bool) noexcept { fixture_trace.push_back(fmt::format("setup {}", id)); return true; }
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
	fixture_services.company_next = FixtureNext;
	fixture_services.local_company = FixtureLocal; fixture_services.network_server = FixtureServer;
	fixture_services.post_company_delete = FixturePostDelete;
	fixture_services.script_random_next = FixtureRandomNext; fixture_services.reset_competitor_timeout = FixtureTimeout;
	fixture_services.can_allocate_company = FixtureCanAllocate;
	fixture_services.allocate_company = FixtureAllocate;
	fixture_services.prepare_new_company = FixturePrepare;
	fixture_services.company_defaults = FixtureNoop; fixture_services.startup_windows = FixtureVoid;
	fixture_services.new_company_events = FixtureEvents;
	fixture_identity = GetRustCompanyIdentityServices();
	fixture_identity_shared = GetRustSharedServices();
	fixture_identity_shared.random = [](void *) noexcept -> uint32_t { return 0; };
	fixture_identity.shared = &fixture_identity_shared;
	fixture_identity.company = [](uint8_t id) noexcept { return reinterpret_cast<Company *>(fixture_companies[id].get()); };
	fixture_identity.next_company = [](uint32_t) noexcept -> Company * { return nullptr; };
	fixture_identity.company_id = [](const Company *) noexcept -> uint8_t { return 2; };
	fixture_identity.set_colour = [](Company *c, uint8_t value) noexcept { FixtureProperties(c)->colour = static_cast<Colours>(value); };
	fixture_identity.colour = [](const Company *c) noexcept -> uint8_t { return FixtureProperties(c)->colour; };
	fixture_identity.livery = [](Company *c, LiveryScheme scheme) noexcept { return &FixtureProperties(c)->livery[scheme]; };
	fixture_identity.face = [](Company *c) noexcept { return &FixtureProperties(c)->face; };
	fixture_identity.set_president_id = [](Company *c, uint32_t value) noexcept { FixtureProperties(c)->president_name_1 = value; };
	fixture_identity.set_president_parts = [](Company *c, uint32_t value) noexcept { FixtureProperties(c)->president_name_2 = value; };
	fixture_identity.render = [](OpenTTDCompanyNameStorage *, uint32_t, uint32_t) noexcept { return OpenTTDByteSpan{reinterpret_cast<const uint8_t *>("President"), 9}; };
	fixture_identity.admin_update = FixtureAdminUpdate;
	fixture_identity.is_ai = [](const Company *) noexcept { return fixture_is_ai; };
	fixture_faces = GetRustFaceServices();
	fixture_faces.count = []() noexcept -> uint32_t { return 1; };
	fixture_faces.vars = [](uint32_t) noexcept { return OpenTTDFaceVars{nullptr, 0}; };
	fixture_faces.spec = [](uint32_t) noexcept { static const FaceSpec spec{"fixture", FaceVars{}}; return &spec; };
	fixture_faces.random = [](Randomizer *) noexcept -> uint32_t { return 0; };
	fixture_identity.faces = &fixture_faces;
	fixture_services.identity = &fixture_identity;
	fixture_services.company_league_dirty = FixtureVoid; fixture_services.company_ctrl_windows = FixtureVoid;
	fixture_services.assert_new_ai_slot = FixtureNoop; fixture_services.network_company_new = FixtureNetworkNew;
}
/* The default maps do not load unclassified NewGRF road masks or extreme prices. */
TEST_CASE("Commands - road price saturation, rounding and independent type masks")
{
	extern RoadTypes &_roadtypes_road, &_roadtypes_tram;
	extern RoadTypeInfo _roadtypes[ROADTYPE_END];
	AutoRestoreBackup<Money> build(_price[PR_BUILD_ROAD], Money(7));
	AutoRestoreBackup<Money> clear(_price[PR_CLEAR_ROAD], Money(-27));
	AutoRestoreBackup<RoadTypes> roads(_roadtypes_road, {ROADTYPE_ROAD});
	AutoRestoreBackup<RoadTypes> trams(_roadtypes_tram, {ROADTYPE_TRAM});
	AutoRestoreBackup<uint16_t> road_multiplier(_roadtypes[ROADTYPE_ROAD].cost_multiplier, 3);
	AutoRestoreBackup<uint16_t> tram_multiplier(_roadtypes[ROADTYPE_TRAM].cost_multiplier, UINT16_MAX);
	CHECK(RoadBuildCost(ROADTYPE_ROAD) == Money(2));
	CHECK(RoadClearCost(ROADTYPE_ROAD) == Money(-27));
	CHECK(RoadConvertCost(ROADTYPE_ROAD, ROADTYPE_ROAD) == Money(0));
	_roadtypes_road.Reset(); // neither road nor tram: source clear policy still earns removal value
	CHECK(RoadClearCost(ROADTYPE_ROAD) == Money(-28));
	_price[PR_BUILD_ROAD] = -7;
	CHECK(RoadBuildCost(ROADTYPE_ROAD) == Money(-3));
	CHECK(RoadClearCost(ROADTYPE_ROAD) == Money(-25));
	_price[PR_BUILD_ROAD] = INT64_MAX;
	CHECK(RoadBuildCost(ROADTYPE_TRAM) == Money(1152921504606846975LL));
	CHECK(RoadClearCost(ROADTYPE_TRAM) == Money(-864691128455135258LL));
	_price[PR_BUILD_ROAD] = INT64_MIN;
	CHECK(RoadBuildCost(ROADTYPE_TRAM) == Money(-1152921504606846976LL));
	CHECK(RoadClearCost(ROADTYPE_TRAM) == Money(864691128455135205LL));
}

/* Save comparison cannot observe skipped metadata reads or local encoded errors.
 * Unreachable callbacks retain their generated abort defaults. */
TEST_CASE("Commands - pre-post money short circuits and local error encoding")
{
	FixtureReset(false, false);
	auto command = GetRustCommandServices();
	const auto &costs = GetRustCommandCostServices();
	OpenTTDCompanyFinanceServices unread{};
	command.finance = &unread;
	command.infinite_money = OpenTTDCommandServices{}.infinite_money;
	auto skipped = costs;
	skipped.money = OpenTTDCommandCostServices{}.money;
	for (bool test : {false, true}) {
		CommandCost failed(STR_ERROR_INSUFFICIENT_FUNDS);
		openttd_rust_command_before(&command, false, test);
		openttd_rust_command_after(&command, &skipped, RustCommandCost(&failed), 0, true, test);
		CHECK(failed.Failed());
		for (DoCommandFlags flags : {DoCommandFlags{}, DoCommandFlags{DoCommandFlag::Bankrupt}, DoCommandFlags{DoCommandFlag::QueryCost}}) {
			bool top = flags.Any();
			if (!test && flags.Test(DoCommandFlag::QueryCost)) continue;
			CommandCost result(EXPENSES_CONSTRUCTION, Money(50));
			openttd_rust_command_before(&command, false, test);
			openttd_rust_command_after(&command, &skipped, RustCommandCost(&result), flags.base(), top, test);
			CHECK(result.Succeeded());
		}
	}
	for (int64_t amount : {-5, 0}) {
		CommandCost result(EXPENSES_CONSTRUCTION, Money(amount));
		CHECK(openttd_rust_command_check_money(&command, &costs, RustCommandCost(&result)));
	}
	command.infinite_money = []() noexcept { return true; };
	CommandCost infinite(EXPENSES_CONSTRUCTION, Money(INT64_MAX));
	CHECK(openttd_rust_command_check_money(&command, &costs, RustCommandCost(&infinite)));
	command.infinite_money = []() noexcept { return false; };
	command.finance = &fixture_finance;
	fixture_companies[0] = std::make_unique<CompanyProperties>();
	fixture_companies[0]->Finances().money = 20;
	fixture_current = 0;
	AutoRestoreBackup<CompanyID> current(_current_company, CompanyID(0));
	AutoRestoreBackup<CompanyID> local(_local_company, CompanyID(0));
	for (CompanyID local_id : {CompanyID(0), CompanyID(1)}) {
		_local_company = local_id;
		CommandCost result(EXPENSES_CONSTRUCTION, Money(21));
		CHECK_FALSE(openttd_rust_command_check_money(&command, &costs, RustCommandCost(&result)));
		CHECK(result.GetCost() == Money(21));
		CHECK(result.GetErrorMessage() == STR_ERROR_NOT_ENOUGH_CASH_REQUIRES_CURRENCY);
		CHECK(result.GetEncodedMessage() == (local_id == 0 ? GetEncodedString(STR_ERROR_NOT_ENOUGH_CASH_REQUIRES_CURRENCY, Money(21)) : EncodedString{}));
	}
	CommandCost zero;
	skipped = costs; skipped.expense = OpenTTDCommandCostServices{}.expense;
	openttd_rust_command_subtract_money(&command, &skipped, RustCommandCost(&zero));
	for (auto [amount, expense] : {std::pair{5, EXPENSES_CONSTRUCTION}, std::pair{-3, EXPENSES_ROADVEH_REVENUE}, std::pair{2, EXPENSES_ROADVEH_RUN}}) {
		CommandCost result(expense, Money(amount));
		openttd_rust_command_after(&command, &costs, RustCommandCost(&result), 0, true, false);
	}
	const auto &finances = fixture_companies[0]->Finances();
	CHECK(finances.money == Money(16));
	CHECK(finances.yearly_expenses[0][EXPENSES_CONSTRUCTION] == Money(5));
	CHECK(finances.cur_economy.income == Money(3));
	CHECK(finances.cur_economy.expenses == Money(-2));
	fixture_current = 18;
	openttd_rust_command_subtract_money(&command, &skipped, RustCommandCost(&infinite));
	CHECK(finances.money == Money(16));
	fixture_companies[0].reset();
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
		const OpenTTDClockCompany clock{.services = &fixture_services, .month = []() noexcept { return OpenTTDEconomyMonth{.month = 1, .infinite_money = false, .maintenance = false, .fluctuating = false}; }};
		openttd_rust_clock_economy_month(&clock, nullptr);
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
	uint32_t cursor = *openttd_rust_company_tick();
	static int posts;
	posts = 0;
	SessionFixture fixture;
	fixture.services.company = &fixture_services;
	fixture.services.company_tick = []() noexcept { return OpenTTDCompanyTick{.finances = nullptr, .num_companies = 0, .index = 0, .timeout = 0, .interval = 0, .editor = false, .named = false, .competitor_due = true, .networking = false, .max_companies = 15, .max_competitors = 2, .num_ais = 0, .stage = 0}; };
	fixture.services.post_ai = [](OpenTTDSessionFrame *) noexcept { CHECK(fixture_trace.empty()); ++posts; return true; };
	CHECK(fixture.Run());
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
	AutoRestoreBackup<Colours> colour(_company_colours[CompanyID(2)], _company_colours[CompanyID(2)]);
	RustTileOwnerContext ownership;
	FixtureReset(false, false);
	OpenTTDCompanyControl abandoned{.result = {}, .startup = {}, .client = 0, .action = static_cast<uint8_t>(CCA_NEW_AI), .target = 255, .reason = 0, .execute = true, .stage = 0};
	REQUIRE(openttd_rust_company_control(&fixture_services, &ownership.view, &abandoned));
	CHECK(abandoned.startup.company == 2);
	CHECK(fixture_trace == std::vector<std::string>{"allocate 2", "setup 2"});
	CHECK(fixture_companies[2]->Finances().money == fixture_companies[2]->Finances().current_loan);

	FixtureReset(false, false);
	OpenTTDCompanyControl resumed{.result = {}, .startup = {}, .client = 0, .action = static_cast<uint8_t>(CCA_NEW_AI), .target = 255, .reason = 0, .execute = true, .stage = 0};
	int starts = 0;
	while (openttd_rust_company_control(&fixture_services, &ownership.view, &resumed)) starts++;
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
TEST_CASE("Company control - network local switch precedes native settings continuation")
{
	static uint8_t local;
	static bool own;
	static OpenTTDObjectivesServices windows;
	AutoRestoreBackup<Colours> colour(_company_colours[CompanyID(2)], _company_colours[CompanyID(2)]);
	RustTileOwnerContext ownership;
	for (bool matching_client : {false, true}) {
		FixtureReset(true, false); local = COMPANY_SPECTATOR.base(); fixture_current = local; own = matching_client;
		fixture_identity_shared.company_valid = [](uint8_t id) noexcept { return FixtureFinances(id) != nullptr; };
		fixture_services.shared = &fixture_identity_shared;
		fixture_services.local_company = []() noexcept { return local; };
		fixture_services.close_network_status = FixtureVoid;
		fixture_services.network_own_company = [](uint8_t id, uint32_t) noexcept -> Company * { fixture_trace.push_back("request"); return own ? fixture_identity.company(id) : nullptr; };
		fixture_services.network_own_company_settings = [](Company *) noexcept { CHECK(local == 2); CHECK(fixture_current == 2); fixture_trace.push_back("settings"); };
		windows = GetRustObjectivesServices();
		windows.window_data = [](uint16_t cls, uint32_t number, int32_t data) noexcept { CHECK(cls == WC_SEND_NETWORK_MSG); CHECK(number == DESTTYPE_TEAM); CHECK(data == COMPANY_SPECTATOR.base()); CHECK(local == COMPANY_SPECTATOR.base()); CHECK(fixture_current == COMPANY_SPECTATOR.base()); fixture_trace.push_back("chat"); };
		windows.window_classes = [](uint16_t, int32_t) noexcept { fixture_trace.push_back("classes"); };
		fixture_services.objectives = []() noexcept -> const OpenTTDObjectivesServices & { return windows; };
		fixture_services.assign_company_context = [](uint8_t company) noexcept { local = company; fixture_current = company; fixture_trace.push_back("assign"); };
		fixture_services.close_construction_windows = []() noexcept { fixture_trace.push_back("construction"); };
		fixture_identity_shared.screen_dirty = []() noexcept { fixture_trace.push_back("screen"); };
		fixture_identity.reset_colourmap = []() noexcept { fixture_trace.push_back("colourmap"); };
		OpenTTDCompanyControl call{.result = {}, .startup = {}, .client = 7, .action = static_cast<uint8_t>(CCA_NEW), .target = 255, .reason = 0, .execute = true, .stage = 0};
		CHECK_FALSE(openttd_rust_company_control(&fixture_services, &ownership.view, &call));
		std::vector<std::string> expected{"allocate 2", "setup 2", "events 2", "network 2", "request"};
		if (matching_client) expected.insert(expected.end(), {"chat", "assign", "classes", "construction", "screen", "classes", "classes", "classes", "colourmap", "settings"});
		CHECK(fixture_trace == expected);
		CHECK(call.result.error == 0);
		CHECK(local == (matching_client ? 2 : COMPANY_SPECTATOR.base()));
		fixture_companies[2].reset();
	}
}

TEST_CASE("Company economy - special owners have no bankruptcy takeover offer")
{
	RustTileOwnerContext ownership;
	FixtureReset(true, false);
	fixture_companies[1] = std::make_unique<CompanyProperties>();
	for (uint16_t mask : {uint16_t{0}, uint16_t{UINT16_MAX}}) {
		fixture_companies[1]->Finances().bankrupt_asked = CompanyMask{};
		if (mask != 0) fixture_companies[1]->Finances().bankrupt_asked.Set();
		for (uint32_t current = 16; current <= UINT8_MAX; current++) {
			fixture_current = static_cast<uint8_t>(current);
			if (current < 64) CHECK_FALSE(fixture_companies[1]->Finances().bankrupt_asked.Test(CompanyID(static_cast<uint8_t>(current))));
			for (bool hostile : {false, true}) {
				CHECK(openttd_rust_company_buy(&fixture_services, &ownership.view, 1, hostile, false).error == 1);
			}
		}
	}
	fixture_companies[1].reset();
}
