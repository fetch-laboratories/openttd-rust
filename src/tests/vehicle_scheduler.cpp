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
#include "../openttd.h"
#include "../map_func.h"
#include "../tile_map.h"
#include "../landscape.h"
#include "../rust/vehicle_services.h"
#include "../rust/vehicle_maintenance_ffi.h"
#include "../newgrf_sound.h"
#include "abi_layout.hpp"
#include "../rust/fleet_ffi.h"
#include "../safeguards.h"

/* Exercise CallVehicleTicks itself, with the original PoolIterator as oracle.
 * Effect shells avoid unrelated transport post-tick policy. Both traversals run
 * identical virtual Ticks on real pools; no copied pool metadata drives them. */
static std::vector<uint32_t> _lifetime_visits;
static bool _lifetime_mutate;
static uint32_t _lifetime_growth;
struct SchedulerLifetimeVehicle final : Vehicle {
	SchedulerLifetimeVehicle() : Vehicle(VEH_EFFECT) { this->vehstatus.Set(VehState::Hidden); }
	bool Tick() override
	{
		const auto id = this->index.base();
		_lifetime_visits.push_back(id);
		if (!_lifetime_mutate) return true;
		if (id == 0) {
			delete this; // Also deletes the unvisited chain member at ID 1.
			CHECK_FALSE(Vehicle::IsValidID(VehicleID(1)));
			return false; // Original survival assertion requires ID 0 to remain empty.
		}
		if (id == 2) {
			REQUIRE(Vehicle::CanAllocateItem(3));
			new (VehicleID(0)) SchedulerLifetimeVehicle(); // Behind: waits one tick.
			delete Vehicle::Get(VehicleID(4));
			new (VehicleID(4)) SchedulerLifetimeVehicle(); // Ahead: visited this tick.
			new (VehicleID(3)) SchedulerLifetimeVehicle(); // Inside initialized storage.
			delete Vehicle::Get(VehicleID(5)); // Hole ahead must be skipped.
			const auto old_slots = reinterpret_cast<uintptr_t>(_vehicle_pool.data.data());
			_lifetime_growth = static_cast<uint32_t>(_vehicle_pool.data.capacity());
			REQUIRE(Vehicle::CanAllocateItem(2));
			new (VehicleID(_lifetime_growth)) SchedulerLifetimeVehicle();
			new (VehicleID(_lifetime_growth + 1)) SchedulerLifetimeVehicle();
			CHECK(reinterpret_cast<uintptr_t>(_vehicle_pool.data.data()) != old_slots);
		}
		if (id == _lifetime_growth) {
			delete this; // Newly discovered shell dies before the next slot is read.
			return false;
		}
		return true;
	}
};
TEST_CASE("Base vehicle scheduler - production deletion, reuse and reallocation order")
{
	struct Restore {
		GameMode mode = _game_mode;
		~Restore() { _vehicle_pool.CleanPool(); _game_mode = this->mode; }
	} restore;
	_game_mode = GM_MENU; // Exclude calendar/economy day work, as in the source.
	const auto Trace = [](bool production) {
		_vehicle_pool.CleanPool();
		_lifetime_visits.clear();
		_lifetime_growth = UINT32_MAX;
		REQUIRE(Vehicle::CanAllocateItem(5));
		auto *head = new (VehicleID(0)) SchedulerLifetimeVehicle();
		head->SetNext(new (VehicleID(1)) SchedulerLifetimeVehicle());
		new (VehicleID(2)) SchedulerLifetimeVehicle();
		new (VehicleID(4)) SchedulerLifetimeVehicle();
		new (VehicleID(5)) SchedulerLifetimeVehicle();
		for (bool mutate : {true, false}) {
			_lifetime_mutate = mutate;
			if (production) {
				CallVehicleTicks();
			} else {
				for (Vehicle *v : Vehicle::Iterate()) v->Tick();
			}
		}
		CHECK(_vehicle_pool.items == 5);
		CHECK(_lifetime_visits == std::vector<uint32_t>{0, 2, 3, 4, _lifetime_growth, _lifetime_growth + 1, 0, 2, 3, 4, _lifetime_growth + 1});
		_vehicle_pool.CleanPool();
		return _lifetime_visits;
	};
	const auto source = Trace(false);
	CHECK(Trace(true) == source);
}
#ifdef WITH_RUST
TEST_CASE("Base vehicle scheduler - consumed boundary layouts")
{
	auto Layout = [](const char *name, std::initializer_list<size_t> values) { CHECK(AbiLayoutMismatch(name, values) == -1); };
#include "abi-layout-scheduler.inc"
}
TEST_CASE("Aircraft phase boundary - canonical movement scalar address identity")
{
	REQUIRE(Vehicle::CanAllocateItem());
	auto *v = new Vehicle(VEH_ROAD);
	const auto &services = GetRustVehicleServices();
	{
		const auto movement = services.movement_observations(RustVehicle(v));
		const auto position = services.position_observations(RustVehicle(v));
		static_assert(std::is_same_v<decltype(movement.x), const decltype(v->x_pos) *>);
		static_assert(std::is_same_v<decltype(movement.y), const decltype(v->y_pos) *>);
		static_assert(std::is_same_v<decltype(position.z), const decltype(v->z_pos) *>);
		static_assert(std::is_same_v<decltype(movement.speed), const decltype(v->cur_speed) *>);
		static_assert(std::is_same_v<decltype(movement.direction), const Direction *>);
		static_assert(sizeof(Direction) == sizeof(uint8_t));
		CHECK(movement.x == &v->x_pos);
		CHECK(movement.y == &v->y_pos);
		CHECK(movement.speed == &v->cur_speed);
		CHECK(movement.direction == &v->direction);
		CHECK(movement.position.subtype == &v->subtype);
		CHECK(movement.position.z == &v->z_pos);
		CHECK(position.z == movement.position.z);
		CHECK(position.subtype == movement.position.subtype);
		services.write_coordinates(RustVehicle(v), -31, 49, 77);
		services.write_speed(RustVehicle(v), 123);
		services.write_direction(RustVehicle(v), DIR_SW);
		CHECK(*movement.x == -31);
		CHECK(*movement.y == 49);
		CHECK(*position.z == 77);
		CHECK(*movement.speed == 123);
		CHECK(*movement.direction == DIR_SW);
	} // End descriptors before destruction, not after ID reuse.
	delete v;
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
	auto Layout = [](const char *name, std::initializer_list<size_t> values) { CHECK(AbiLayoutMismatch(name, values) == -1); };
#include "abi-layout-maintenance.inc"
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
	base.play_vehicle_sound = [](OpenTTDVehicle *handle, VehicleSoundEvent event) noexcept {
		CHECK(event == VSE_BREAKDOWN);
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

TEST_CASE("Vehicle occupancy - canonical links, stale removal and copied cursors")
{
	Map::Allocate(64, 64);
	REQUIRE(Vehicle::CanAllocateItem(2));
	auto *a = new Vehicle(VEH_ROAD), *b = new Vehicle(VEH_ROAD);
	a->tile = b->tile = TileXY(1, 1);
	for (auto t : {TileXY(1, 1), TileXY(2, 1), TileXY(1, 2), TileXY(2, 2)}) SetTileHeight(t, 1);
	a->z_pos = b->z_pos = 8;
	a->UpdatePosition(); b->UpdatePosition();
	REQUIRE(GetTileMaxPixelZ(a->tile) == 8);
	CHECK(EnsureNoVehicleOnGround(a->tile).Failed());
	a->z_pos = b->z_pos = 9;
	CHECK(EnsureNoVehicleOnGround(a->tile).Succeeded());
	const auto &s = GetRustVehicleServices();
	CHECK(s.tile_links(RustVehicle(a)) == &a->tile_links);
	CHECK(s.tile_links(RustVehicle(b)) == &b->tile_links);
	const auto update = s.occupancy_update(RustVehicle(a));
	CHECK(update.links == &a->tile_links);
	CHECK(update.tile == a->tile.base());
	a->x_pos = 17; a->y_pos = 29;
	const auto near = s.occupancy_near(RustVehicle(a));
	CHECK(near.links == &a->tile_links);
	CHECK(near.xy.x == 17); CHECK(near.xy.y == 29);
	CHECK(b->tile_links.next == RustVehicle(a));
	CHECK(a->tile_links.prev == &b->tile_links.next);
	auto it = VehiclesOnTile(a->tile).begin(), copy = it;
	// Removal must not read tile, dimensions or either normal observation.
	auto removal = s;
	removal.tile = [](OpenTTDVehicle *) noexcept -> uint32_t { CHECK(false); return 0; };
	removal.map_log_x = []() noexcept -> uint32_t { CHECK(false); return 0; };
	removal.occupancy_update = [](OpenTTDVehicle *) noexcept -> OpenTTDOccupancyUpdateObservation { CHECK(false); return {}; };
	removal.occupancy_near = [](OpenTTDVehicle *) noexcept -> OpenTTDOccupancyNearObservation { CHECK(false); return {}; };
	openttd_rust_occupancy_update(&removal, RustVehicle(b), true);
	CHECK(b->tile_links.current == nullptr);
	CHECK(b->tile_links.next == RustVehicle(a)); // Source retains stale next.
	++it; ++copy;
	CHECK(*it == a); CHECK(it == copy);
	ResetVehicleHash();
	CHECK(a->tile_links.current == nullptr);
	CHECK(b->tile_links.next == RustVehicle(a));
	a->UpdatePosition(); b->UpdatePosition();
	CHECK(*VehiclesOnTile(a->tile).begin() == b);
	delete b; delete a;
}
#endif
