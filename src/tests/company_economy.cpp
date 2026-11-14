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
#include "../rust/command_post_ffi.h"
#include "../rust/network_frame_ffi.h"
#include "../rust/network_tcp_ffi.h"
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

/* Native owned-by error encodings; the first playable company, service intervals and the
 * owned-by error and owner selection are company::control_tests (#736 F). Retires with the
 * native CommandCost string encoding. */
TEST_CASE("Company control - native owned-by error encodings")
{
	REQUIRE(Company::GetNumItems() == 0);
	AutoRestoreBackup<CompanyID> current(_current_company, CompanyID(0));
	AutoRestoreBackup<CompanyID> local(_local_company, CompanyID(0));
	AutoRestoreBackup<GameMode> mode(_game_mode, GM_MENU);
	REQUIRE(Company::CanAllocateItem());
	std::unique_ptr<Company> company(new (CompanyID(3)) Company(STR_SV_UNNAMED, false));
	for (CompanyID observer : {CompanyID(0), COMPANY_SPECTATOR}) {
		_local_company = observer;
		for (Owner owner : {CompanyID(3), OWNER_NONE}) {
			auto error = CheckOwnership(owner);
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

/* The company-or-client defaults choice goes through the native settings services, and each
 * type's interval is read from the native VehicleDefaultSettings record they return; the
 * per-type field selection and the ignored percent flag are company::control_tests (#736 F).
 * With no human company, GetFirstPlayableCompanyID's Company::CanAllocateItem grants the
 * allocation credit that the unindexed construction of the company it returns consumes. Retires with the native settings-table services
 * and the native company pool. */
TEST_CASE("Company control - company or client service-interval defaults")
{
	REQUIRE(Company::GetNumItems() == 0);
	AutoRestoreBackup<VehicleDefaultSettings> client(_settings_client.company.vehicle, {});
	AutoRestoreBackup<GameMode> mode(_game_mode, GM_MENU);
	CompanyID vacant = GetFirstPlayableCompanyID();
	CHECK(vacant == CompanyID::Begin());
#ifdef WITH_ASSERT
	CHECK(_company_pool.checked == 1);
#endif
	/* Pool::GetNew(size) asserts and consumes the credit; placement at an index would not. */
	std::unique_ptr<Company> company(new Company(STR_SV_UNNAMED, false));
	CHECK(company->index == vacant);
#ifdef WITH_ASSERT
	CHECK(_company_pool.checked == 0);
#endif
	company->settings.vehicle = {.servint_ispercent = false, .servint_trains = 0, .servint_roadveh = 121, .servint_aircraft = 65535, .servint_ships = 423};
	_settings_client.company.vehicle = {.servint_ispercent = true, .servint_trains = 224, .servint_roadveh = 0, .servint_aircraft = 426, .servint_ships = 65535};
	for (auto [type, company_interval, client_interval] : {std::tuple{VEH_TRAIN, 0, 224}, {VEH_ROAD, 121, 0}, {VEH_AIRCRAFT, 65535, 426}, {VEH_SHIP, 423, 65535}}) {
		CHECK(CompanyServiceInterval(company.get(), type) == company_interval);
		CHECK(CompanyServiceInterval(nullptr, type) == client_interval);
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
static OpenTTDSharedServices fixture_shared;
static OpenTTDCommandPostServices fixture_post;
static OpenTTDCommandServices fixture_command;
static OpenTTDNetworkCommandServices fixture_network_command;
static OpenTTDNetworkFrameServices fixture_network_frame;
static OpenTTDNetworkTcpServices fixture_network_tcp;
static OpenTTDNetworkGameSocket fixture_network_socket;
static OpenTTDCompanyServices fixture_network_company, fixture_post_company;
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
	fixture_companies[id].reset();
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
static void FixtureNetworkNew(uint8_t id) noexcept { fixture_trace.push_back(fmt::format("network {}", id)); }
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
	fixture_services.network_server = FixtureServer;
	fixture_shared = GetRustSharedServices(); fixture_shared.local_company = FixtureLocal;
	fixture_services.shared = &fixture_shared;
	fixture_post = GetRustCommandPostServices();
	fixture_shared.command_post = []() noexcept -> const OpenTTDCommandPostServices & { return fixture_post; };
	fixture_command = GetRustCommandServices();
	fixture_command.finance = &fixture_finance;
	fixture_command.shared = &fixture_shared;
	fixture_command.company = []() noexcept -> const OpenTTDCompanyServices * { return &fixture_post_company; };
	fixture_command.shift_pressed = []() noexcept { return false; };
	fixture_post.command = []() noexcept -> const OpenTTDCommandServices & { return fixture_command; };
	fixture_post.desync_logging = []() noexcept { return false; };
	/* Observe the production send decision using a mock transport sink. */
	fixture_network_company = fixture_services;
	fixture_network_company.network_server = []() noexcept { return false; };
	fixture_network_command = GetRustNetworkCommandServices();
	fixture_network_command.company = &fixture_network_company;
	fixture_network_frame = GetRustNetworkFrameServices();
	fixture_network_frame.command = &fixture_network_command;
	fixture_network_frame.client_game = []() noexcept { return &fixture_network_socket; };
	fixture_network_tcp = GetRustNetworkTcpServices();
	fixture_network_tcp.packet_enqueue = [](NetworkTCPSocketHandler *, OpenTTDPacketState *) noexcept {};
	fixture_network_frame.tcp = &fixture_network_tcp;
	fixture_network_command.network = []() noexcept -> const OpenTTDNetworkFrameServices & { return fixture_network_frame; };
	fixture_post.network = []() noexcept -> const OpenTTDNetworkCommandServices & { return fixture_network_command; };
	fixture_post.pack_control = [](uint8_t action, uint8_t target, uint8_t reason, uint32_t client) noexcept {
		fixture_trace.push_back(fmt::format("post {}", target));
		return GetRustCommandPostServices().pack_control(action, target, reason, client);
	};
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
	fixture_services.assert_new_ai_slot = FixtureNoop; fixture_services.admin_company_new = FixtureNetworkNew;
	fixture_services.company_count = []() noexcept -> size_t { return std::count_if(fixture_companies.begin(), fixture_companies.end(), [](const auto &p) { return p != nullptr; }); };
	fixture_services.company_bankrupt_news = FixtureNoop;
	fixture_services.remove_engine_replacements = FixtureNoop;
	static const uint32_t no_tiles = 0; static OpenTTDMapStatics empty_map = _rust_map_statics; empty_map.size = &no_tiles; fixture_shared.map = &empty_map;
	fixture_services.delete_company = FixturePostDelete;
	fixture_services.company_removed = [](uint8_t, uint8_t) noexcept {};
	fixture_shared.screen_dirty = FixtureVoid;
	fixture_post_company = fixture_services;
	/* Company-control completion restores this fixture's initial current company. */
	static OpenTTDSharedServices post_shared; post_shared = fixture_shared; post_shared.local_company = []() noexcept -> uint8_t { return 18; }; fixture_post_company.shared = &post_shared;
}
/* Only the local company's cash error is encoded natively; the money short circuits and
 * subtraction are command::money_tests (#736 F). Retires with the native CommandCost string
 * encoding. */
TEST_CASE("Commands - local cash error encoding")
{
	const auto &command = GetRustCommandServices();
	AutoRestoreBackup<CompanyID> current(_current_company, CompanyID(0));
	AutoRestoreBackup<CompanyID> local(_local_company, CompanyID(0));
	for (CompanyID local_id : {CompanyID(0), CompanyID(1)}) {
		_local_company = local_id;
		CommandCost result(EXPENSES_CONSTRUCTION, Money(21));
		command.money_error(RustCommandCost(&result), 21);
		CHECK(result.GetErrorMessage() == STR_ERROR_NOT_ENOUGH_CASH_REQUIRES_CURRENCY);
		CHECK(result.GetEncodedMessage() == (local_id == 0 ? GetEncodedString(STR_ERROR_NOT_ENOUGH_CASH_REQUIRES_CURRENCY, Money(21)) : EncodedString{}));
	}
}

/* The production session loop reaches OnTick_Companies' port, whose competitor loop rereads
 * max_no_competitors before each CCA_NEW_AI post and returns to the session caller for each
 * Post. Expected counts follow
 * the original OnTick_Companies with competitors_interval 0 and no companies. Retires with the
 * native session services. */
TEST_CASE("Company economy - session loop posts each competitor through its caller")
{
	FixtureReset(false, false);
	for (bool change_limit : {false, true}) {
		fixture_trace.clear();
		uint32_t cursor = *openttd_rust_company_tick();
		static int posts;
		posts = 0;
		SessionFixture fixture;
		auto companies = fixture_services;
		companies.shared = &fixture.shared;
		fixture.finance.networking = FixtureNetworking;
		companies.network_server = FixtureServer;
		companies.competitor_fired = []() noexcept { return true; };
		companies.ai_can_start = []() noexcept { return true; };
		companies.company_count = []() noexcept -> size_t { return 0; };
		fixture.max_competitors = 2;
		fixture.services.company = &companies;
		fixture.services.post_ai = [](OpenTTDSessionFrame *) noexcept { CHECK(fixture_trace.empty()); ++posts; return true; };
		if (change_limit) fixture.services.post_ai = [](OpenTTDSessionFrame *) noexcept { CHECK(fixture_trace.empty()); ++posts; SessionFixture::active->max_competitors = 1; return true; };
		CHECK(fixture.Run(GM_NORMAL));
		CHECK(posts == (change_limit ? 1 : 2));
		CHECK(*openttd_rust_company_tick() == (cursor + 1) % MAX_COMPANIES);
	}
}

/* Offline and server bankrupt deletions post through the native command and network service
 * tables; the network client's mode is company::bankruptcy_tests (#736 F). Retires with the
 * native command-post and network glue. */
TEST_CASE("Company economy - offline and server bankrupt deletion posts")
{
	for (int mode : {0, 1}) {
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
		CHECK(fixture_trace == std::vector<std::string>{"post 1", "post 2"});
		for (uint32_t i : {1U, 2U}) CHECK((fixture_companies[i] == nullptr) == (mode == 0));
	}
	for (auto &p : fixture_companies) p.reset();
}
