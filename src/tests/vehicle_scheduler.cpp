/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file vehicle_scheduler.cpp Actual scheduler scalar/pool contracts and live traversal. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#include "../roadveh.h"
#include "../vehicle_func.h"
#include "../rust/vehicle_services.h"
#include "../rust/vehicle_maintenance_ffi.h"
#include "../rust/abi_ffi.h"
#include "../rust/fleet_ffi.h"
#include "../safeguards.h"
#ifdef WITH_RUST
TEST_CASE("Base vehicle scheduler - consumed boundary layouts")
{
	auto Layout = [](uint16_t type, const char *, std::initializer_list<size_t> fields) {
		uint8_t item = 0;
		for (size_t expected : fields) CHECK(openttd_rust_abi_layout(type, item++) == expected);
		CHECK(openttd_rust_abi_layout(type, item) == SIZE_MAX);
	};
	Layout(740, "OpenTTDVehiclePoolCursor", {sizeof(OpenTTDVehiclePoolCursor), alignof(OpenTTDVehiclePoolCursor), offsetof(OpenTTDVehiclePoolCursor, slots), offsetof(OpenTTDVehiclePoolCursor, slots_len), offsetof(OpenTTDVehiclePoolCursor, first_unused)});
	Layout(741, "OpenTTDVehiclePostTick", {sizeof(OpenTTDVehiclePostTick), alignof(OpenTTDVehiclePostTick), offsetof(OpenTTDVehiclePostTick, type), offsetof(OpenTTDVehiclePostTick, period), offsetof(OpenTTDVehiclePostTick, counter), offsetof(OpenTTDVehiclePostTick, motion), offsetof(OpenTTDVehiclePostTick, speed), offsetof(OpenTTDVehiclePostTick, front_type), offsetof(OpenTTDVehiclePostTick, subtype), offsetof(OpenTTDVehiclePostTick, tick), offsetof(OpenTTDVehiclePostTick, front_status), offsetof(OpenTTDVehiclePostTick, status)});
	Layout(743, "OpenTTDVehicleDayCallback", {sizeof(OpenTTDVehicleDayCallback), alignof(OpenTTDVehicleDayCallback), offsetof(OpenTTDVehicleDayCallback, day), offsetof(OpenTTDVehicleDayCallback, has_engine)});
	Layout(748, "OpenTTDVehicleStationLookup", {sizeof(OpenTTDVehicleStationLookup), alignof(OpenTTDVehicleStationLookup), offsetof(OpenTTDVehicleStationLookup, station), offsetof(OpenTTDVehicleStationLookup, id)});
	Layout(749, "OpenTTDVehicleSchedulerServices", {sizeof(OpenTTDVehicleSchedulerServices), alignof(OpenTTDVehicleSchedulerServices), offsetof(OpenTTDVehicleSchedulerServices, pool), offsetof(OpenTTDVehicleSchedulerServices, assertions), offsetof(OpenTTDVehicleSchedulerServices, tick), offsetof(OpenTTDVehicleSchedulerServices, calendar_day), offsetof(OpenTTDVehicleSchedulerServices, economy_day), offsetof(OpenTTDVehicleSchedulerServices, day_callback_read), offsetof(OpenTTDVehicleSchedulerServices, day_callback), offsetof(OpenTTDVehicleSchedulerServices, randomise_callback), offsetof(OpenTTDVehicleSchedulerServices, first_dirty), offsetof(OpenTTDVehicleSchedulerServices, unknown_day_callback), offsetof(OpenTTDVehicleSchedulerServices, cargo_owner), offsetof(OpenTTDVehicleSchedulerServices, running_sound), offsetof(OpenTTDVehicleSchedulerServices, periodic_sound), offsetof(OpenTTDVehicleSchedulerServices, next_station), offsetof(OpenTTDVehicleSchedulerServices, with_loading_timer), offsetof(OpenTTDVehicleSchedulerServices, reset_accumulators)});
	Layout(750, "OpenTTDVehicleSchedulerTables", {sizeof(OpenTTDVehicleSchedulerTables), alignof(OpenTTDVehicleSchedulerTables), offsetof(OpenTTDVehicleSchedulerTables, scheduler), offsetof(OpenTTDVehicleSchedulerTables, cargo), offsetof(OpenTTDVehicleSchedulerTables, station), offsetof(OpenTTDVehicleSchedulerTables, loading), offsetof(OpenTTDVehicleSchedulerTables, pending), offsetof(OpenTTDVehicleSchedulerTables, transaction), offsetof(OpenTTDVehicleSchedulerTables, group)});
}
/* Base Vehicle::Tick survives without a controller, so the Tick facade fills
 * the actual post-Tick member addresses of these road-typed shells. */
static OpenTTDVehiclePostTick SurvivingPostTick(Vehicle *v)
{
	OpenTTDVehiclePostTick post{};
	REQUIRE(GetRustVehicleSchedulerServices().tick(RustVehicle(v), &post));
	return post;
}
TEST_CASE("Base vehicle scheduler - scalar member identity and lifetime")
{
	REQUIRE(Vehicle::CanAllocateItem(2));
	auto *v = new Vehicle(VEH_ROAD);
	auto *part = new Vehicle(VEH_ROAD);
	v->SetNext(part);
	const auto post = SurvivingPostTick(part);
	static_assert(std::is_same_v<decltype(post.period), const uint16_t *>);
	static_assert(std::is_same_v<decltype(post.counter), uint16_t *>);
	static_assert(std::is_same_v<decltype(post.motion), uint32_t *>);
	static_assert(std::is_same_v<decltype(post.speed), const uint16_t *>);
	static_assert(std::is_same_v<decltype(post.front_type), const VehicleType *>);
	static_assert(std::is_same_v<decltype(post.front_status), const uint8_t *>);
	static_assert(std::is_same_v<decltype(post.status), const uint8_t *>);
	static_assert(std::is_same_v<decltype(v->vehstatus.base_address()), const uint8_t *>);
	CHECK(post.type == VEH_ROAD);
	CHECK(part->First() == v);
	CHECK(post.period == &part->vcache.cached_cargo_age_period);
	CHECK(post.counter == &part->cargo_age_counter);
	CHECK(post.motion == &part->motion_counter);
	CHECK(post.speed == &v->cur_speed);
	CHECK(post.front_type == &v->type);
	CHECK(post.subtype == &part->subtype);
	CHECK(post.tick == &part->tick_counter);
	CHECK(post.front_status == v->vehstatus.base_address());
	CHECK(post.status == part->vehstatus.base_address());
	CHECK(post.front_status != post.status);
	v->vehstatus.Set(VehState::Stopped);
	part->vehstatus.Set(VehState::Hidden);
	CHECK(*post.front_status == RustVehicleStatus(v));
	CHECK(*post.status == RustVehicleStatus(part));
	v->vehstatus.Reset(VehState::Stopped);
	CHECK(*post.front_status == 0);
	*post.counter = UINT16_MAX;
	*post.motion = UINT32_MAX;
	CHECK(part->cargo_age_counter == UINT16_MAX);
	CHECK(part->motion_counter == UINT32_MAX);
	const auto old_id = v->index;
	const auto grow_id = VehicleID(static_cast<uint32_t>(_vehicle_pool.data.size()));
	REQUIRE(Vehicle::CanAllocateItem());
	new (grow_id) RoadVehicle();
	CHECK(SurvivingPostTick(part).counter == post.counter);
	CHECK(SurvivingPostTick(part).front_status == post.front_status);
	_vehicle_pool.CleanPool(); // All earlier descriptors are discarded here.
	REQUIRE(Vehicle::CanAllocateItem());
	auto *reused = new (old_id) Vehicle(VEH_ROAD);
	const auto fresh = SurvivingPostTick(reused);
	CHECK(fresh.counter == &reused->cargo_age_counter);
	CHECK(*fresh.counter == 1);
	CHECK(*fresh.motion == 0);
	_vehicle_pool.CleanPool();
}
/* Mutate the actual pool after each visit, identically for Rust and unchanged
 * PoolIterator. No copied IDs or test replica of native indexing metadata. */
static std::vector<uint32_t> VehicleCursorTrace(bool rust, bool indexing_growth = true)
{
	_vehicle_pool.CleanPool();
	REQUIRE(Vehicle::CanAllocateItem(3));
	new (VehicleID(0)) Vehicle(VEH_EFFECT);
	new (VehicleID(2)) Vehicle(VEH_EFFECT);
	new (VehicleID(4)) Vehicle(VEH_EFFECT);
	const auto &services = GetRustVehicleSchedulerServices();
	auto cursor = services.pool();
	static_assert(std::is_same_v<decltype(cursor.slots), Vehicle *const *>);
	static_assert(std::is_same_v<decltype(cursor.first_unused), const size_t *>);
	CHECK(cursor.slots == _vehicle_pool.data.data());
	CHECK(cursor.slots_len == _vehicle_pool.data.size());
	CHECK(cursor.first_unused == &_vehicle_pool.first_unused);
	std::vector<uint32_t> trace;
	auto Mutate = [&](Vehicle *v) {
		const auto id = v->index.base();
		trace.push_back(id);
		if (id == 0) {
			delete v; // Deleted-current and reuse behind the advancing cursor.
			REQUIRE(Vehicle::CanAllocateItem(4));
			new (VehicleID(0)) Vehicle(VEH_EFFECT);
			delete Vehicle::Get(VehicleID(2));
			new (VehicleID(2)) Vehicle(VEH_EFFECT); // Reuse ahead must still be visited.
			new (VehicleID(3)) Vehicle(VEH_EFFECT);
			new (VehicleID(6)) Vehicle(VEH_EFFECT); // Raise extent without indexing growth.
			CHECK(*cursor.first_unused == 7);
		} else if (id == 2) {
			delete Vehicle::Get(VehicleID(4)); // Hole ahead.
			if (indexing_growth) {
				const auto growth = static_cast<uint32_t>(_vehicle_pool.data.size());
				REQUIRE(Vehicle::CanAllocateItem(2));
				new (VehicleID(growth)) Vehicle(VEH_EFFECT); // Growth invalidates captured slots.
				new (VehicleID(growth + 1)) Vehicle(VEH_EFFECT);
			}
			CHECK(*cursor.first_unused == _vehicle_pool.first_unused); // Stable canonical address.
		}
	};
	if (rust) {
		size_t from = 0;
		while (auto *shell = openttd_rust_vehicle_cursor_next(&services, &cursor, &from)) Mutate(VehicleOf(shell));
		CHECK(cursor.slots == _vehicle_pool.data.data());
		CHECK(cursor.slots_len == _vehicle_pool.data.size());
	} else {
		for (Vehicle *v : Vehicle::Iterate()) Mutate(v);
	}
	_vehicle_pool.CleanPool();
	cursor = services.pool(); // Fresh acquisition after reset; empty pool ends.
	size_t from = 0;
	CHECK(openttd_rust_vehicle_cursor_next(&services, &cursor, &from) == nullptr);
	return trace;
}
TEST_CASE("Base vehicle scheduler - live cursor equals unchanged PoolIterator")
{
	const auto source = VehicleCursorTrace(false);
	CHECK(VehicleCursorTrace(true) == source);
	REQUIRE(source.size() == 6);
	CHECK(source[0] == 0);
	CHECK(source[1] == 2);
	CHECK(source[2] == 3);
	CHECK(source[3] == 6);
	const auto within_size = VehicleCursorTrace(false, false);
	CHECK(VehicleCursorTrace(true, false) == within_size);
	CHECK(within_size == std::vector<uint32_t>{0, 2, 3, 6});
}
/* Synthetic virtual Tick on actual pool shells checks the transient ordering
 * saves cannot show: growth before assertion, timing scope return before the
 * bundle fill, post-Tick type mutation, then deletion with no bundle write. */
static bool _scheduler_tick_scope_ended;
static uint32_t _scheduler_post_visits;
static uint32_t _scheduler_pool_reads;
struct SchedulerProbeVehicle final : Vehicle {
	SchedulerProbeVehicle() : Vehicle(VEH_EFFECT) {}
	bool Tick() override
	{
		struct EndScope { ~EndScope() { _scheduler_tick_scope_ended = true; } } scope;
		_scheduler_tick_scope_ended = false;
		if (this->index.base() == 0) {
			CHECK(Vehicle::CanAllocateItem());
			new (VehicleID(static_cast<uint32_t>(_vehicle_pool.data.size()))) SchedulerProbeVehicle();
			this->type = VEH_DISASTER;
			return true;
		}
		delete this;
		return false;
	}
};
TEST_CASE("Base vehicle scheduler - Tick growth, timing return and fresh type")
{
	_vehicle_pool.CleanPool();
	REQUIRE(Vehicle::CanAllocateItem());
	new (VehicleID(0)) SchedulerProbeVehicle();
	_scheduler_post_visits = _scheduler_pool_reads = 0;
	auto services = GetRustVehicleSchedulerServices();
#if !defined(NDEBUG) || defined(WITH_ASSERT)
	CHECK(services.assertions);
#else
	CHECK(!services.assertions);
#endif
	services.pool = []() noexcept {
		++_scheduler_pool_reads;
		return GetRustVehicleSchedulerServices().pool();
	};
	services.tick = [](OpenTTDVehicle *shell, OpenTTDVehiclePostTick *out) noexcept {
		out->type = VEH_INVALID;
		const bool survives = GetRustVehicleSchedulerServices().tick(shell, out);
		CHECK(_scheduler_tick_scope_ended);
		if (!survives) {
			CHECK(out->type == VEH_INVALID); // Deleted shell: no bundle fill.
			return false;
		}
		++_scheduler_post_visits;
		CHECK(out->type == VEH_DISASTER); // Mutable pre-Tick Effect tag must not be reused.
		CHECK(out->front_type == nullptr);
		CHECK(out->period == nullptr);
		CHECK(out->status == nullptr);
		return true;
	};
	services.next_station = [](uint32_t) noexcept { return OpenTTDVehicleStationLookup{}; };
	/* Rust borrows complete service values even when the pending map is empty.
	 * Every function-pointer slot must be non-null, including unused callbacks. */
	auto group = FleetGroupServices();
	group.current_company = []() noexcept -> uint8_t { return 0; };
	const auto unused = [](auto...) noexcept { FAIL_CHECK("Unexpected pending fleet operation"); };
	const auto unused_position = [](void *) noexcept -> int32_t { FAIL_CHECK("Unexpected pending fleet position"); return 0; };
	OpenTTDFleetPendingServices pending{
		.set_current = [](uint8_t) noexcept {},
		.restart = unused,
		.x = unused_position,
		.y = unused_position,
		.z = unused_position,
		.reserve = [](uint8_t) noexcept -> uint32_t { FAIL_CHECK("Unexpected pending fleet reserve"); return 0; },
		.subtract = unused,
		.command = unused,
		.animation = unused,
		.length_news = unused,
		.failed_news = unused,
		.cash = 0,
		.limit = 0,
	};
	auto transaction = FleetTransactionServices();
	OpenTTDVehicleSchedulerTables tables{};
	tables.scheduler = &services;
	tables.pending = &pending;
	tables.transaction = &transaction;
	tables.group = &group;
	openttd_rust_vehicle_ticks(&tables, false, 0, nullptr);
	CHECK(_scheduler_tick_scope_ended);
	CHECK(_scheduler_pool_reads == 2);
	CHECK(_scheduler_post_visits == 1); // Deleted higher shell must never be dereferenced.
	CHECK(_vehicle_pool.items == 1);
	_vehicle_pool.CleanPool();
}
#endif /* WITH_RUST */

#ifdef WITH_RUST
TEST_CASE("Vehicle maintenance consumed native ABI")
{
	auto Layout = [](uint16_t type, const char *, std::initializer_list<size_t> fields) {
		uint8_t item = 0;
		for (size_t expected : fields) CHECK(openttd_rust_abi_layout(type, item++) == expected);
		CHECK(openttd_rust_abi_layout(type, item) == SIZE_MAX);
	};
	Layout(751, "OpenTTDVehicleMaintenanceRenewal", {sizeof(OpenTTDVehicleMaintenanceRenewal), alignof(OpenTTDVehicleMaintenanceRenewal), offsetof(OpenTTDVehicleMaintenanceRenewal, age), offsetof(OpenTTDVehicleMaintenanceRenewal, max_age), offsetof(OpenTTDVehicleMaintenanceRenewal, months), offsetof(OpenTTDVehicleMaintenanceRenewal, enabled), offsetof(OpenTTDVehicleMaintenanceRenewal, kind), offsetof(OpenTTDVehicleMaintenanceRenewal, subtype)});
	Layout(752, "OpenTTDVehicleMaintenanceReset", {sizeof(OpenTTDVehicleMaintenanceReset), alignof(OpenTTDVehicleMaintenanceReset), offsetof(OpenTTDVehicleMaintenanceReset, economy_date), offsetof(OpenTTDVehicleMaintenanceReset, calendar_date), offsetof(OpenTTDVehicleMaintenanceReset, reliability), offsetof(OpenTTDVehicleMaintenanceReset, chance), offsetof(OpenTTDVehicleMaintenanceReset, breakdowns)});
	Layout(753, "OpenTTDVehicleMaintenanceResetWrite", {sizeof(OpenTTDVehicleMaintenanceResetWrite), alignof(OpenTTDVehicleMaintenanceResetWrite), offsetof(OpenTTDVehicleMaintenanceResetWrite, economy_date), offsetof(OpenTTDVehicleMaintenanceResetWrite, calendar_date), offsetof(OpenTTDVehicleMaintenanceResetWrite, reliability), offsetof(OpenTTDVehicleMaintenanceResetWrite, chance), offsetof(OpenTTDVehicleMaintenanceResetWrite, since)});
	Layout(754, "OpenTTDVehicleMaintenanceInterval", {sizeof(OpenTTDVehicleMaintenanceInterval), alignof(OpenTTDVehicleMaintenanceInterval), offsetof(OpenTTDVehicleMaintenanceInterval, company), offsetof(OpenTTDVehicleMaintenanceInterval, last_service), offsetof(OpenTTDVehicleMaintenanceInterval, now), offsetof(OpenTTDVehicleMaintenanceInterval, interval), offsetof(OpenTTDVehicleMaintenanceInterval, reliability), offsetof(OpenTTDVehicleMaintenanceInterval, percent), offsetof(OpenTTDVehicleMaintenanceInterval, wallclock), offsetof(OpenTTDVehicleMaintenanceInterval, no_service), offsetof(OpenTTDVehicleMaintenanceInterval, breakdowns)});
	Layout(755, "OpenTTDVehicleMaintenanceReplacement", {sizeof(OpenTTDVehicleMaintenanceReplacement), alignof(OpenTTDVehicleMaintenanceReplacement), offsetof(OpenTTDVehicleMaintenanceReplacement, engine), offsetof(OpenTTDVehicleMaintenanceReplacement, when_old)});
	Layout(756, "OpenTTDVehicleMaintenanceBudget", {sizeof(OpenTTDVehicleMaintenanceBudget), alignof(OpenTTDVehicleMaintenanceBudget), offsetof(OpenTTDVehicleMaintenanceBudget, needed), offsetof(OpenTTDVehicleMaintenanceBudget, available)});
	Layout(757, "OpenTTDVehicleMaintenanceReplacementCost", {sizeof(OpenTTDVehicleMaintenanceReplacementCost), alignof(OpenTTDVehicleMaintenanceReplacementCost), offsetof(OpenTTDVehicleMaintenanceReplacementCost, cost), offsetof(OpenTTDVehicleMaintenanceReplacementCost, available)});
	Layout(758, "OpenTTDVehicleMaintenanceAutomatic", {sizeof(OpenTTDVehicleMaintenanceAutomatic), alignof(OpenTTDVehicleMaintenanceAutomatic), offsetof(OpenTTDVehicleMaintenanceAutomatic, has_depot_order), offsetof(OpenTTDVehicleMaintenanceAutomatic, order_type)});
	Layout(759, "OpenTTDVehicleMaintenanceCheckBreakdown", {sizeof(OpenTTDVehicleMaintenanceCheckBreakdown), alignof(OpenTTDVehicleMaintenanceCheckBreakdown), offsetof(OpenTTDVehicleMaintenanceCheckBreakdown, reliability), offsetof(OpenTTDVehicleMaintenanceCheckBreakdown, decay), offsetof(OpenTTDVehicleMaintenanceCheckBreakdown, no_service), offsetof(OpenTTDVehicleMaintenanceCheckBreakdown, breakdowns)});
	Layout(760, "OpenTTDVehicleMaintenanceBreakdownGuard", {sizeof(OpenTTDVehicleMaintenanceBreakdownGuard), alignof(OpenTTDVehicleMaintenanceBreakdownGuard), offsetof(OpenTTDVehicleMaintenanceBreakdownGuard, speed), offsetof(OpenTTDVehicleMaintenanceBreakdownGuard, counter), offsetof(OpenTTDVehicleMaintenanceBreakdownGuard, status), offsetof(OpenTTDVehicleMaintenanceBreakdownGuard, breakdowns), offsetof(OpenTTDVehicleMaintenanceBreakdownGuard, menu)});
	Layout(761, "OpenTTDVehicleMaintenanceBreakdownTrigger", {sizeof(OpenTTDVehicleMaintenanceBreakdownTrigger), alignof(OpenTTDVehicleMaintenanceBreakdownTrigger), offsetof(OpenTTDVehicleMaintenanceBreakdownTrigger, reliability), offsetof(OpenTTDVehicleMaintenanceBreakdownTrigger, chance), offsetof(OpenTTDVehicleMaintenanceBreakdownTrigger, kind), offsetof(OpenTTDVehicleMaintenanceBreakdownTrigger, breakdowns)});
	Layout(762, "OpenTTDVehicleMaintenanceBreakdownProbe", {sizeof(OpenTTDVehicleMaintenanceBreakdownProbe), alignof(OpenTTDVehicleMaintenanceBreakdownProbe), offsetof(OpenTTDVehicleMaintenanceBreakdownProbe, counter), offsetof(OpenTTDVehicleMaintenanceBreakdownProbe, status)});
	Layout(763, "OpenTTDVehicleMaintenanceBreakdownStart", {sizeof(OpenTTDVehicleMaintenanceBreakdownStart), alignof(OpenTTDVehicleMaintenanceBreakdownStart), offsetof(OpenTTDVehicleMaintenanceBreakdownStart, since), offsetof(OpenTTDVehicleMaintenanceBreakdownStart, kind)});
	Layout(764, "OpenTTDVehicleMaintenanceBreakdownActive", {sizeof(OpenTTDVehicleMaintenanceBreakdownActive), alignof(OpenTTDVehicleMaintenanceBreakdownActive), offsetof(OpenTTDVehicleMaintenanceBreakdownActive, kind), offsetof(OpenTTDVehicleMaintenanceBreakdownActive, tick), offsetof(OpenTTDVehicleMaintenanceBreakdownActive, delay)});
	Layout(765, "OpenTTDVehicleMaintenanceBreakdownCountdown", {sizeof(OpenTTDVehicleMaintenanceBreakdownCountdown), alignof(OpenTTDVehicleMaintenanceBreakdownCountdown), offsetof(OpenTTDVehicleMaintenanceBreakdownCountdown, order_type), offsetof(OpenTTDVehicleMaintenanceBreakdownCountdown, counter)});
	Layout(766, "OpenTTDVehicleMaintenanceBreakdownSoundState", {sizeof(OpenTTDVehicleMaintenanceBreakdownSoundState), alignof(OpenTTDVehicleMaintenanceBreakdownSoundState), offsetof(OpenTTDVehicleMaintenanceBreakdownSoundState, kind), offsetof(OpenTTDVehicleMaintenanceBreakdownSoundState, landscape)});
	Layout(767, "OpenTTDVehicleMaintenanceBreakdownSmoke", {sizeof(OpenTTDVehicleMaintenanceBreakdownSmoke), alignof(OpenTTDVehicleMaintenanceBreakdownSmoke), offsetof(OpenTTDVehicleMaintenanceBreakdownSmoke, shell), offsetof(OpenTTDVehicleMaintenanceBreakdownSmoke, delay)});
	Layout(768, "OpenTTDVehicleMaintenanceAge", {sizeof(OpenTTDVehicleMaintenanceAge), alignof(OpenTTDVehicleMaintenanceAge), offsetof(OpenTTDVehicleMaintenanceAge, age), offsetof(OpenTTDVehicleMaintenanceAge, max_age), offsetof(OpenTTDVehicleMaintenanceAge, decay)});
	Layout(769, "OpenTTDVehicleMaintenanceWarning", {sizeof(OpenTTDVehicleMaintenanceWarning), alignof(OpenTTDVehicleMaintenanceWarning), offsetof(OpenTTDVehicleMaintenanceWarning, enabled), offsetof(OpenTTDVehicleMaintenanceWarning, has_previous), offsetof(OpenTTDVehicleMaintenanceWarning, local), offsetof(OpenTTDVehicleMaintenanceWarning, status)});
	Layout(770, "OpenTTDVehicleMaintenanceWarningCompany", {sizeof(OpenTTDVehicleMaintenanceWarningCompany), alignof(OpenTTDVehicleMaintenanceWarningCompany), offsetof(OpenTTDVehicleMaintenanceWarningCompany, company), offsetof(OpenTTDVehicleMaintenanceWarningCompany, renewal)});
	Layout(771, "OpenTTDVehicleMaintenanceServices", {sizeof(OpenTTDVehicleMaintenanceServices), alignof(OpenTTDVehicleMaintenanceServices), offsetof(OpenTTDVehicleMaintenanceServices, shared), offsetof(OpenTTDVehicleMaintenanceServices, transaction), offsetof(OpenTTDVehicleMaintenanceServices, group), offsetof(OpenTTDVehicleMaintenanceServices, renewal), offsetof(OpenTTDVehicleMaintenanceServices, reset), offsetof(OpenTTDVehicleMaintenanceServices, reset_write), offsetof(OpenTTDVehicleMaintenanceServices, has_engine), offsetof(OpenTTDVehicleMaintenanceServices, interval), offsetof(OpenTTDVehicleMaintenanceServices, engine_reliability), offsetof(OpenTTDVehicleMaintenanceServices, budget), offsetof(OpenTTDVehicleMaintenanceServices, replacement), offsetof(OpenTTDVehicleMaintenanceServices, replacement_available), offsetof(OpenTTDVehicleMaintenanceServices, replacement_cost), offsetof(OpenTTDVehicleMaintenanceServices, automatic), offsetof(OpenTTDVehicleMaintenanceServices, depot_service), offsetof(OpenTTDVehicleMaintenanceServices, check_breakdown), offsetof(OpenTTDVehicleMaintenanceServices, breakdown_guard), offsetof(OpenTTDVehicleMaintenanceServices, breakdown_trigger), offsetof(OpenTTDVehicleMaintenanceServices, chance_write), offsetof(OpenTTDVehicleMaintenanceServices, trigger), offsetof(OpenTTDVehicleMaintenanceServices, breakdown_counter), offsetof(OpenTTDVehicleMaintenanceServices, breakdown_probe), offsetof(OpenTTDVehicleMaintenanceServices, breakdown_start), offsetof(OpenTTDVehicleMaintenanceServices, breakdown_start_write), offsetof(OpenTTDVehicleMaintenanceServices, breakdown_active), offsetof(OpenTTDVehicleMaintenanceServices, breakdown_countdown), offsetof(OpenTTDVehicleMaintenanceServices, breakdown_sound_state), offsetof(OpenTTDVehicleMaintenanceServices, breakdown_delay_write), offsetof(OpenTTDVehicleMaintenanceServices, breakdown_sound), offsetof(OpenTTDVehicleMaintenanceServices, smoke_disabled), offsetof(OpenTTDVehicleMaintenanceServices, smoke), offsetof(OpenTTDVehicleMaintenanceServices, smoke_animation), offsetof(OpenTTDVehicleMaintenanceServices, aircraft_broken), offsetof(OpenTTDVehicleMaintenanceServices, mark_dirty), offsetof(OpenTTDVehicleMaintenanceServices, view_dirty), offsetof(OpenTTDVehicleMaintenanceServices, details_dirty), offsetof(OpenTTDVehicleMaintenanceServices, age), offsetof(OpenTTDVehicleMaintenanceServices, age_write), offsetof(OpenTTDVehicleMaintenanceServices, decay_write), offsetof(OpenTTDVehicleMaintenanceServices, warning), offsetof(OpenTTDVehicleMaintenanceServices, warning_company), offsetof(OpenTTDVehicleMaintenanceServices, engine_available_any), offsetof(OpenTTDVehicleMaintenanceServices, replacement_active), offsetof(OpenTTDVehicleMaintenanceServices, old_news), offsetof(OpenTTDVehicleMaintenanceServices, economy_age), offsetof(OpenTTDVehicleMaintenanceServices, economy_age_write), offsetof(OpenTTDVehicleMaintenanceServices, reached_min_age), offsetof(OpenTTDVehicleMaintenanceServices, value), offsetof(OpenTTDVehicleMaintenanceServices, value_write), offsetof(OpenTTDVehicleMaintenanceServices, max_calendar_date), offsetof(OpenTTDVehicleMaintenanceServices, max_economy_date), offsetof(OpenTTDVehicleMaintenanceServices, profit_min_age), offsetof(OpenTTDVehicleMaintenanceServices, anniversaries), offsetof(OpenTTDVehicleMaintenanceServices, before_zero), offsetof(OpenTTDVehicleMaintenanceServices, news_getting_old), offsetof(OpenTTDVehicleMaintenanceServices, news_very_old), offsetof(OpenTTDVehicleMaintenanceServices, news_very_old_and)});
	Layout(772, "OpenTTDCalendarDate", {sizeof(OpenTTDCalendarDate), alignof(OpenTTDCalendarDate), offsetof(OpenTTDCalendarDate, value)});
	Layout(773, "OpenTTDEconomyDate", {sizeof(OpenTTDEconomyDate), alignof(OpenTTDEconomyDate), offsetof(OpenTTDEconomyDate, value)});
	Layout(774, "OrderType", {sizeof(OrderType), alignof(OrderType)});
	Layout(775, "LandscapeType", {sizeof(LandscapeType), alignof(LandscapeType)});

}

#endif

#ifdef WITH_RUST
TEST_CASE("Vehicle maintenance native scalar stores and cadence")
{
	auto &pool = _vehicle_pool;
	REQUIRE(Vehicle::GetNumItems() == 0);
	REQUIRE(Vehicle::CanAllocateItem());
	Vehicle *v = new Vehicle(VEH_EFFECT);
	v->age = CalendarTime::MAX_DATE - 1;
	AgeVehicle(v);
	CHECK(v->age == CalendarTime::MAX_DATE);
	AgeVehicle(v);
	CHECK(v->age == CalendarTime::MAX_DATE);
	v->economy_age = EconomyTime::MAX_DATE - 1;
	EconomyAgeVehicle(v);
	CHECK(v->economy_age == EconomyTime::MAX_DATE);
	EconomyAgeVehicle(v);
	CHECK(v->economy_age == EconomyTime::MAX_DATE);
	v->value = Money{65535};
	DecreaseVehicleValue(v);
	CHECK(v->value == 65280);
	v->value = Money{-257};
	DecreaseVehicleValue(v);
	CHECK(v->value == -255);
	v->breakdown_ctr = 1;
	v->breakdown_delay = 3;
	v->type = VEH_TRAIN;
	for (uint8_t tick : {1, 2, 3}) {
		v->tick_counter = tick;
		CHECK(v->HandleBreakdown());
		CHECK(v->breakdown_delay == 3);
	}
	v->tick_counter = 4;
	CHECK(v->HandleBreakdown());
	CHECK(v->breakdown_delay == 2);
	v->type = VEH_ROAD;
	v->breakdown_delay = 1;
	v->tick_counter = 2;
	CHECK(v->HandleBreakdown());
	CHECK(v->breakdown_ctr == 0);
	CHECK(v->breakdown_delay == 0);
	v->type = VEH_AIRCRAFT;
	v->breakdown_ctr = 1;
	v->breakdown_delay = 1;
	CHECK_FALSE(v->HandleBreakdown());
	CHECK(v->breakdown_delay == 1);
	CHECK(v->breakdown_ctr == 1);
	v->type = VEH_ROAD;
	v->breakdown_ctr = 1;
	v->breakdown_delay = 0;
	CHECK(v->HandleBreakdown());
	CHECK(v->breakdown_delay == 255); // Native uint8_t pre-decrement wraps.
	/* A sound/dirty callback may change current state. Rust must reacquire the
	 * hidden flag before smoke, and current type after the dirty callbacks. */
	auto maintenance = _rust_vehicle_maintenance_services;
	auto base = GetRustVehicleServices();
	base.maintenance = &maintenance;
	maintenance.breakdown_sound = [](OpenTTDVehicle *handle) noexcept {
		Vehicle *current = VehicleOf(handle);
		CHECK(current->cur_speed == 0);
		CHECK(current->breakdown_ctr == 1);
		CHECK(current->breakdowns_since_last_service == 255);
		current->vehstatus.Set(VehState::Hidden);
		return true;
	};
	maintenance.mark_dirty = [](OpenTTDVehicle *handle) noexcept {
		CHECK(VehicleOf(handle)->vehstatus.Test(VehState::Hidden));
		VehicleOf(handle)->type = VEH_AIRCRAFT;
	};
	v->cur_speed = 10;
	v->breakdown_ctr = 2;
	v->breakdown_delay = 5;
	v->breakdowns_since_last_service = 255;
	CHECK_FALSE(openttd_rust_vehicle_handle_breakdown(RustVehicle(v), &base));
	CHECK(v->breakdown_delay == 5);
	CHECK(v->type == VEH_AIRCRAFT);
	v->type = VEH_EFFECT;
	v->breakdown_ctr = 3;
	v->current_order.MakeLoading(false);
	CHECK_FALSE(v->HandleBreakdown());
	CHECK(v->breakdown_ctr == 3);
	v->current_order.MakeDummy();
	CHECK_FALSE(v->HandleBreakdown());
	CHECK(v->breakdown_ctr == 2);
	pool.CleanPool();
	CHECK(Vehicle::GetNumItems() == 0);
}
#endif
