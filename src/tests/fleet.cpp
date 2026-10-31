/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file fleet.cpp Fleet owner layouts, CommandCost stack lifetime and reentry checks. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#ifdef WITH_RUST
#include "../group.h"
#include "../autoreplace_base.h"
#include "../company_base.h"
#include "../command_type.h"
#include "../company_func.h"
#include "../core/backup_type.hpp"
#include "abi_layout.hpp"
#include "../rust/command_services.h"
#include "../rust/company_identity_ffi.h"
#include "../table/strings.h"
#include <map>
#endif
#include "../safeguards.h"
#ifdef WITH_RUST
TEST_CASE("Fleet - canonical state layouts and independent lifetimes")
{
	ABI_LAYOUT_CHECK;
#include "abi-layout-fleet.inc"

	/* Rust reads Livery as {in_use, colour1, colour2} bytes. */
	static_assert(sizeof(Livery) == 3 && offsetof(Livery, colour1) == 1 && offsetof(Livery, colour2) == 2);
	FleetOwner<FleetGroupFields, openttd_rust_fleet_group_create, openttd_rust_fleet_group_destroy> group;
	CHECK(group.state->parent == GroupID::Invalid());
	CHECK(group.state->owner == INVALID_OWNER);
	const std::string name = "Group \xE2\x98\x83";
	openttd_rust_fleet_set_name(group.state, reinterpret_cast<const uint8_t *>(name.data()), name.size());
	std::string copy(openttd_rust_fleet_name(group.state, nullptr, 0), '\0');
	openttd_rust_fleet_name(group.state, reinterpret_cast<uint8_t *>(copy.data()), copy.size());
	CHECK(copy == name);
	FleetChildren children{group.state};
	children.insert(GroupID(7));
	children.insert(GroupID(2));
	children.insert(GroupID(7));
	CHECK(std::vector<GroupID>(children.begin(), children.end()) == std::vector<GroupID>{GroupID(2), GroupID(7)});
	children.erase(GroupID(2));
	CHECK(std::vector<GroupID>(children.begin(), children.end()) == std::vector<GroupID>{GroupID(7)});
	GroupStatistics stats;
	CHECK(&stats.profit_last_year == &stats.state.state->profit_last_year);
	stats.num_engines[EngineID(3)]--;
	CHECK(openttd_rust_fleet_engine_count(stats.state.state, 3) == UINT16_MAX);
	stats.num_engines[EngineID(3)]++;
	CHECK(openttd_rust_fleet_engine_count(stats.state.state, 3) == 0);
	stats.profit_last_year = INT64_MIN;
	stats.num_engines[EngineID(4)] += 3;
	GroupStatistics copied = stats;
	CHECK(copied.state.state != stats.state.state);
	CHECK(&copied.profit_last_year == &copied.state.state->profit_last_year);
	CHECK(copied.profit_last_year == Money(INT64_MIN));
	copied.profit_last_year = 17;
	copied.num_engines[EngineID(4)]++;
	CHECK(stats.profit_last_year == Money(INT64_MIN));
	CHECK(openttd_rust_fleet_engine_count(stats.state.state, 4) == 3);
	stats = copied;
	CHECK(stats.profit_last_year == Money(17));
	CHECK(openttd_rust_fleet_engine_count(stats.state.state, 4) == 4);
	CompanyProperties company;
	CHECK(company.RenewalList() == nullptr);
	CompanyProperties detached = company;
	CHECK(&detached.RenewalList() != &company.RenewalList());
}

static std::vector<OpenTTDCommandCost *> command_cost_stack;
static bool command_cost_lifo;
TEST_CASE("Command - opaque stack lifetime and moved encoded errors")
{
	const auto &native = GetRustCommandCostServices();
	auto traced = native;
	traced.construct = [](OpenTTDCommandCostStorage *storage) noexcept {
		auto *p = GetRustCommandCostServices().construct(storage); command_cost_stack.push_back(p); return p;
	};
	traced.destroy = [](OpenTTDCommandCost *p) noexcept {
		command_cost_lifo &= !command_cost_stack.empty() && command_cost_stack.back() == p;
		command_cost_stack.pop_back(); GetRustCommandCostServices().destroy(p);
	};
	command_cost_lifo = true;
	CHECK(openttd_rust_command_stack_probe(&traced));
	CHECK(command_cost_lifo);
	CHECK(command_cost_stack.empty());
	AutoRestoreBackup<CompanyID> company(_current_company, CompanyID(0)), local(_local_company, CompanyID(0));
	OpenTTDCommandCostStorage first, second;
	auto *a = native.construct(&first), *b = native.construct(&second);
	native.error_param(a, STR_ERROR_LOCAL_AUTHORITY_REFUSES_TO_ALLOW_THIS, UINT16_MAX);
	const auto expected = reinterpret_cast<CommandCost *>(a)->GetEncodedMessage();
	REQUIRE_FALSE(expected.empty());
	native.move(b, a);
	native.destroy(a);
	CHECK(reinterpret_cast<CommandCost *>(b)->GetEncodedMessage() == expected);
	native.destroy(b);
}

/* Drain reentry: the command callback inserts and overwrites map entries. */
static std::vector<std::pair<uint32_t, uint8_t>> _fleet_drain_trace;
static uint8_t _fleet_drain_company = 0;

TEST_CASE("Fleet - pending drain matches ordered map iteration under reentry")
{
	OpenTTDFleetGroupServices group{};
	group.current_company = []() noexcept -> uint8_t { return _fleet_drain_company; };
	group.vehicle = [](uint32_t id) noexcept -> void * { return reinterpret_cast<void *>(static_cast<uintptr_t>(id) + 1); };
	group.vehicle_owner = [](void *shell) noexcept -> uint8_t { return static_cast<uint8_t>(reinterpret_cast<uintptr_t>(shell) % 3); };
	OpenTTDFleetTransactionServices transaction{};
	transaction.local = []() noexcept { return false; };
	OpenTTDFleetPendingServices pending{};
	pending.set_current = [](uint8_t company) noexcept { _fleet_drain_company = company; };
	pending.restart = [](void *) noexcept {};
	pending.x = [](void *) noexcept -> int32_t { return 0; };
	pending.y = [](void *) noexcept -> int32_t { return 0; };
	pending.z = [](void *) noexcept -> int32_t { return 0; };
	OpenTTDCompanyIdentityServices company{};
	company.company = [](uint8_t id) noexcept { return reinterpret_cast<Company *>(static_cast<uintptr_t>(id) + 1); };
	company.renew_reserve = [](const Company *c) noexcept -> uint32_t { CHECK(static_cast<uint8_t>(reinterpret_cast<uintptr_t>(c) - 1) == _fleet_drain_company); return 0; };
	pending.company = &company;
	pending.subtract = [](int64_t) noexcept {};
	pending.command = [](void *, uint32_t id) noexcept {
		_fleet_drain_trace.emplace_back(id, _fleet_drain_company);
		/* Later keys are visited this tick; earlier and current keys are not revisited. */
		if (id == 5) { openttd_rust_fleet_pending_add(9, true); openttd_rust_fleet_pending_add(2, true); openttd_rust_fleet_pending_add(5, false); }
		if (id == 9) openttd_rust_fleet_pending_add(0xFFFFF, false);
	};
	/* Reference: the original std::map loop with the same callback insertions. */
	std::map<uint32_t, bool> reference{{5, true}, {3, false}, {7, true}};
	std::vector<std::pair<uint32_t, uint8_t>> expected;
	for (auto &it : reference) {
		expected.emplace_back(it.first, static_cast<uint8_t>((it.first + 1) % 3));
		if (it.first == 5) { reference[9] = true; reference[2] = true; reference[5] = false; }
		if (it.first == 9) reference[0xFFFFF] = false;
	}
	openttd_rust_fleet_pending_clear();
	openttd_rust_fleet_pending_add(5, true);
	openttd_rust_fleet_pending_add(3, false);
	openttd_rust_fleet_pending_add(7, true);
	_fleet_drain_company = 11;
	_fleet_drain_trace.clear();
	CommandCost cost;
	openttd_rust_fleet_pending_drain(&pending, &transaction, &group, &cost);
	CHECK(_fleet_drain_trace == expected);
	CHECK(_fleet_drain_company == 11);
	openttd_rust_fleet_pending_clear();
}
#endif
