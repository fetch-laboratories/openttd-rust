/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file fleet.cpp Fleet owner layouts, service tables, CommandCost and reentry checks. */
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
#include "../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDFleetGroupServices.hpp"
#include "abi-visit-OpenTTDFleetTransactionServices.hpp"
#include "abi-visit-OpenTTDCommandServices.hpp"
#include "abi-visit-OpenTTDCommandCostServices.hpp"
#include "../rust/command_services.h"
#include "../table/strings.h"
#include <map>
#endif
#include "../safeguards.h"
#ifdef WITH_RUST
TEST_CASE("Fleet - canonical state layouts and independent lifetimes")
{
	auto Layout = [](const char *name, std::initializer_list<size_t> values) { CHECK(AbiLayoutMismatch(name, values) == -1); };
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

TEST_CASE("Fleet - service tables match the Rust layout and fill every slot")
{
	using G = OpenTTDFleetGroupServices;
	using T = OpenTTDFleetTransactionServices;
	using C = OpenTTDFleetCosts;
	using P = OpenTTDFleetPendingServices;

	const G &group = FleetGroupServices();
	const T &transaction = FleetTransactionServices();
	CHECK(AbiServicesComplete(group));
	CHECK(AbiServicesComplete(transaction));
	CHECK(group.next_part == CargoCapacityNextPart);
	CHECK(group.recursion_error == STR_ERROR_GROUP_CAN_T_SET_PARENT_RECURSION);
	CHECK(transaction.unavailable == STR_ERROR_RAIL_VEHICLE_NOT_AVAILABLE);
	CHECK(transaction.too_long == STR_ERROR_TRAIN_TOO_LONG);
	CHECK(transaction.too_long_replacement == STR_ERROR_TRAIN_TOO_LONG_AFTER_REPLACEMENT);
	CHECK(transaction.nothing == STR_ERROR_AUTOREPLACE_NOTHING_TO_DO);
#if !defined(NDEBUG) || defined(WITH_ASSERT)
	CHECK(transaction.assertions);
#else
	CHECK_FALSE(transaction.assertions);
#endif
}

static bool SameCost(const CommandCost &a, const CommandCost &b)
{
	return a.Succeeded() == b.Succeeded() && a.GetCost() == b.GetCost() && a.GetErrorMessage() == b.GetErrorMessage() && a.GetExpensesType() == b.GetExpensesType() && a.GetExtraErrorMessage() == b.GetExtraErrorMessage() && a.GetErrorOwner() == b.GetErrorOwner();
}

TEST_CASE("Commands - shared native stack result layouts and encoded parameters")
{
	const auto &commands = GetRustCommandServices();
	CHECK(AbiServicesComplete(commands));
	const auto &w = GetRustCommandCostServices();
	CHECK(AbiServicesComplete(w));
	AutoRestoreBackup<CompanyID> current(_current_company, CompanyID(0));
	AutoRestoreBackup<CompanyID> local(_local_company, CompanyID(0));
	CommandCost result;
	w.error_param(RustCommandCost(&result), STR_ERROR_LOCAL_AUTHORITY_REFUSES_TO_ALLOW_THIS, UINT16_MAX);
	CommandCost expected = CommandCostWithParam(STR_ERROR_LOCAL_AUTHORITY_REFUSES_TO_ALLOW_THIS, UINT16_MAX);
	result.SetErrorOwner(CompanyID(4)); expected.SetErrorOwner(CompanyID(4));
	CHECK(SameCost(result, expected));
	CHECK_FALSE(expected.GetEncodedMessage().empty());
	CHECK(result.GetEncodedMessage() == expected.GetEncodedMessage());
	CommandCost moved;
	w.move(RustCommandCost(&moved), RustCommandCost(&result));
	CHECK(SameCost(moved, expected));
	CHECK(moved.GetEncodedMessage() == expected.GetEncodedMessage());
	w.error(RustCommandCost(&moved), INVALID_STRING_ID);
	CHECK_FALSE(w.success(RustCommandCost(&moved)));
	CHECK(w.error_message(RustCommandCost(&moved)) == INVALID_STRING_ID);
	w.expense_money(RustCommandCost(&moved), EXPENSES_CONSTRUCTION, INT64_MIN);
	CHECK(SameCost(moved, CommandCost(EXPENSES_CONSTRUCTION, Money(INT64_MIN))));
}

TEST_CASE("Fleet - CommandCost services keep native first-error and saturation")
{
	const auto &w = FleetTransactionServices();
	/* Replacement accumulator: build error first, then a later error and later costs. */
	CommandCost native(EXPENSES_NEW_VEHICLES, Money(0));
	CommandCost result(STR_ERROR_AUTOREPLACE_NOTHING_TO_DO);
	w.cost_vehicles(&result);
	CHECK(SameCost(result, native));
	CHECK(w.success(&result));
	CommandCost first(STR_ERROR_TRAIN_TOO_LONG);
	first.AddCost(Money(1234));
	CommandCost first_native = first;
	w.cost_add(&result, &first);
	native.AddCost(std::move(first_native));
	CHECK(SameCost(result, native));
	CHECK(w.error(&result) == STR_ERROR_TRAIN_TOO_LONG);
	CHECK(w.money(&result) == 1234);
	CommandCost later(STR_ERROR_TRAIN_TOO_LONG_AFTER_REPLACEMENT, STR_ERROR_AUTOREPLACE_MONEY_LIMIT);
	later.AddCost(Money(5));
	CommandCost later_native = later;
	w.cost_add(&result, &later);
	native.AddCost(std::move(later_native));
	CHECK(SameCost(result, native));
	CHECK(w.error(&result) == STR_ERROR_TRAIN_TOO_LONG);
	CHECK(w.money(&result) == 1239);
	/* Money saturation in both directions, including the wagon refund's negation. */
	w.cost_amount(&result, INT64_MAX);
	native.AddCost(Money(INT64_MAX));
	CHECK(SameCost(result, native));
	CHECK(w.money(&result) == INT64_MAX);
	w.cost_amount(&result, INT64_MIN);
	w.cost_amount(&result, INT64_MIN);
	native.AddCost(Money(INT64_MIN));
	native.AddCost(Money(INT64_MIN));
	CHECK(SameCost(result, native));
	CHECK(w.money(&result) == INT64_MIN);
	CHECK(-Money(INT64_MIN) == Money(INT64_MAX));
	/* Move keeps every native field, then error/zero overwrite the whole value. */
	CommandCost moved(STR_ERROR_TRAIN_TOO_LONG, STR_ERROR_AUTOREPLACE_MONEY_LIMIT);
	CommandCost target;
	w.cost_move(&target, &moved);
	CHECK(SameCost(target, CommandCost(STR_ERROR_TRAIN_TOO_LONG, STR_ERROR_AUTOREPLACE_MONEY_LIMIT)));
	w.cost_error(&target, STR_ERROR_AUTOREPLACE_NOTHING_TO_DO);
	CHECK(SameCost(target, CommandCost(STR_ERROR_AUTOREPLACE_NOTHING_TO_DO)));
	w.cost_zero(&target);
	CHECK(SameCost(target, CommandCost()));
	/* A successful item adds only its cost; the receiver keeps its expense type. */
	CommandCost success(EXPENSES_PROPERTY, Money(9));
	w.cost_add(&target, &success);
	CHECK(SameCost(target, CommandCost(INVALID_EXPENSES, Money(9))));
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
	pending.reserve = [](uint8_t) noexcept -> uint32_t { return 0; };
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
