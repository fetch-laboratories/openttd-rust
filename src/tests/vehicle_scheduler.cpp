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
#include "../../tools/simulation/session-fixture.hpp"
#include "../rust/company_adapter.h"
#include "../rust/company_identity_ffi.h"
#include "../rust/session_ffi.h"
#include "../safeguards.h"

/* Exercise CallVehicleTicks itself, with the original PoolIterator as oracle.
 * Effect shells avoid unrelated transport post-tick policy. Both traversals run
 * identical virtual Ticks on real pools; no copied pool metadata drives them. */
static std::vector<uint32_t> _lifetime_visits;
static bool _lifetime_mutate;
static uint32_t _lifetime_growth;
struct SchedulerLifetimeVehicle final : Vehicle {
	SchedulerLifetimeVehicle() : Vehicle(VEH_EFFECT) { this->SetVehStatus(this->GetVehStatus().Set(VehState::Hidden)); }
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
			_lifetime_growth = static_cast<uint32_t>(_vehicle_pool.descriptor->slots.capacity);
			REQUIRE(Vehicle::CanAllocateItem(2));
			new (VehicleID(_lifetime_growth)) SchedulerLifetimeVehicle();
			new (VehicleID(_lifetime_growth + 1)) SchedulerLifetimeVehicle();
			/* Growth may extend an allocation in place. The visit trace below
			 * independently requires the newly initialized slots to be discovered. */
			CHECK(_vehicle_pool.descriptor->slots.capacity > _lifetime_growth);
		}
		if (id == _lifetime_growth) {
			delete this; // Newly discovered shell dies before the next slot is read.
			return false;
		}
		return true;
	}
};
TEST_CASE("Vehicle maintenance previous ID preserves the company sentinel")
{
	struct Restore { ~Restore() { _vehicle_pool.CleanPool(); } } restore;
	_vehicle_pool.CleanPool();
	REQUIRE(Vehicle::CanAllocateItem(2));
	auto *head = new (VehicleID(0)) SchedulerLifetimeVehicle();
	auto *part = new (VehicleID(1)) SchedulerLifetimeVehicle();
	head->SetNext(part);
	const auto &services = GetRustCompanyServices();
	CHECK(services.vehicle_previous(head->index.base()) == UINT32_MAX);
	CHECK(services.vehicle_previous(part->index.base()) == head->index.base());
}

TEST_CASE("Base vehicle scheduler - production deletion, reuse and growth order")
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
				SessionFixture fixture;
				CommandCost refit, temporary;
				fixture.tables = RustVehicleSchedulerTables(reinterpret_cast<OpenTTDCommandCost *>(&refit), reinterpret_cast<OpenTTDCommandCost *>(&temporary));
				CHECK(fixture.Run());
			} else {
				for (Vehicle *v : Vehicle::Iterate()) v->Tick();
			}
		}
		CHECK(_vehicle_pool.GetNumItems() == 5);
		CHECK(_lifetime_visits == std::vector<uint32_t>{0, 2, 3, 4, _lifetime_growth, _lifetime_growth + 1, 0, 2, 3, 4, _lifetime_growth + 1});
		_vehicle_pool.CleanPool();
		return _lifetime_visits;
	};
	const auto source = Trace(false);
	CHECK(Trace(true) == source);
}
TEST_CASE("Base vehicle scheduler - consumed boundary layouts")
{
	ABI_LAYOUT_CHECK;
#include "abi-layout-scheduler.inc"
}
/* Observe actual C++ owner-member teardown, which the raw Rust pool fixture cannot reach. */
struct PoolRecordLifetimeVehicle final : Vehicle {
	PoolRecordLifetimeVehicle() : Vehicle(VEH_EFFECT)
	{
		const auto &slot = _vehicle_pool.descriptor->slots[this->index.base()];
		CHECK(slot.shell == this);
		CHECK(slot.record == this->record);
	}
	~PoolRecordLifetimeVehicle() override
	{
		const auto &slot = _vehicle_pool.descriptor->slots[this->index.base()];
		CHECK(slot.shell == this);
		CHECK(slot.record == this->record);
	}
	static void operator delete(void *memory, size_t size)
	{
		auto *shell = static_cast<PoolRecordLifetimeVehicle *>(memory);
		{
			const auto &slot = _vehicle_pool.descriptor->slots[shell->index.base()];
			CHECK(slot.shell == memory);
			CHECK(slot.record == nullptr); // BaseVehicle retired its owner before sized delete.
		}
		VehiclePool::PoolItem<&_vehicle_pool>::operator delete(memory, size);
	}
};
TEST_CASE("Vehicle pool - native owner record publication and retirement")
{
	struct Restore { ~Restore() { _vehicle_pool.CleanPool(); } } restore;
	_vehicle_pool.CleanPool();
	REQUIRE(Vehicle::CanAllocateItem());
	auto *shell = new PoolRecordLifetimeVehicle();
	const auto id = shell->index;
	CHECK(_vehicle_pool.descriptor->items == 1);
	/* An unrelated shell/record cannot attach to this live ID or retire its owner. */
	OpenTTDVehicle unrelated{};
	_vehicle_pool.BindRecord(id.base(), nullptr, &unrelated);
	_vehicle_pool.RetireRecord(id.base(), nullptr, shell->record);
	CHECK(_vehicle_pool.descriptor->slots[id.base()].record == shell->record);
	delete shell;
	CHECK(_vehicle_pool.descriptor->slots[id.base()].shell == nullptr);
	CHECK(_vehicle_pool.descriptor->slots[id.base()].record == nullptr);
	CHECK(_vehicle_pool.descriptor->items == 0);
	REQUIRE(Vehicle::CanAllocateItem());
	new PoolRecordLifetimeVehicle();
	CHECK(Vehicle::IsValidID(id)); // The original lowest free ID is reused.
	_vehicle_pool.CleanPool(); // Native typed deletion observes the same owner order.
	CHECK(_vehicle_pool.descriptor->first_unused == 0);
	CHECK(_vehicle_pool.descriptor->slots.empty());
}

TEST_CASE("Vehicle records - distinct native shells and inline scalar aliases")
{
	REQUIRE(Vehicle::CanAllocateItem());
	auto *v = new Vehicle(VEH_ROAD);
	const auto &services = GetRustVehicleServices();
	auto *record = RustVehicle(v);
	CHECK(static_cast<void *>(record) != static_cast<void *>(v));
	CHECK(record == &v->Record());
	CHECK(VehicleOf(record) == v);
	CHECK(services.record_from_shell(v) == record);
	CHECK(RustVehicle(nullptr) == nullptr);
	CHECK(VehicleOf(static_cast<OpenTTDVehicle *>(nullptr)) == nullptr);
	record->x_pos = -31; record->y_pos = 49; record->z_pos = 77;
	record->cur_speed = 123; record->direction = DIR_SW;
	CHECK(v->Record().x_pos == -31);
	CHECK(v->Record().y_pos == 49);
	CHECK(v->Record().z_pos == 77);
	CHECK(v->Record().cur_speed == 123);
	CHECK(v->GetDirection() == DIR_SW);
	v->SetDirection(DIR_NE);
	CHECK(record->direction == DIR_NE);
	delete v;
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
	static_assert(std::is_same_v<decltype(cursor.slots), const RustPoolSlot *>);
	static_assert(std::is_same_v<decltype(cursor.first_unused), const size_t *>);
	CHECK(cursor.slots == _vehicle_pool.descriptor->slots.items);
	CHECK(cursor.slots_len == _vehicle_pool.descriptor->slots.size());
	CHECK(cursor.first_unused == &_vehicle_pool.descriptor->first_unused);
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
				const auto growth = static_cast<uint32_t>(_vehicle_pool.descriptor->slots.size());
				REQUIRE(Vehicle::CanAllocateItem(2));
				new (VehicleID(growth)) Vehicle(VEH_EFFECT); // Growth invalidates captured slots.
				new (VehicleID(growth + 1)) Vehicle(VEH_EFFECT);
			}
			CHECK(*cursor.first_unused == _vehicle_pool.descriptor->first_unused); // Stable canonical address.
		}
	};
	if (rust) {
		size_t from = 0;
		while (auto *shell = openttd_rust_vehicle_cursor_next(&services, &cursor, &from)) Mutate(VehicleOf(shell));
		CHECK(cursor.slots == _vehicle_pool.descriptor->slots.items);
		CHECK(cursor.slots_len == _vehicle_pool.descriptor->slots.size());
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
 * saves cannot show: growth before assertion, timing scope return before Rust's
 * post-Tick reads, post-Tick type mutation, then deletion. */
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
			new (VehicleID(static_cast<uint32_t>(_vehicle_pool.descriptor->slots.size()))) SchedulerProbeVehicle();
			this->SetType(VEH_DISASTER);
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
	services.pool = []() noexcept {
		++_scheduler_pool_reads;
		return GetRustVehicleSchedulerServices().pool();
	};
	services.tick = [](OpenTTDVehicle *shell) noexcept {
		const bool survives = GetRustVehicleSchedulerServices().tick(shell);
		CHECK(_scheduler_tick_scope_ended);
		if (!survives) return false; // Deleted shell: Rust reads no record.
		++_scheduler_post_visits;
		CHECK(shell->type == VEH_DISASTER); // Rust reads this fresh type, not the pre-Tick Effect tag.
		return true;
	};
	/* Rust borrows complete service values even when the pending map is empty.
	 * Every function-pointer slot must be non-null, including unused callbacks. */
	auto group = FleetGroupServices();
	const auto unused = [](auto...) noexcept { FAIL_CHECK("Unexpected pending fleet operation"); };
	OpenTTDFleetPendingServices pending{
		.set_current = [](uint8_t) noexcept {},
		.company = &GetRustCompanyIdentityServices(),
		.subtract = unused,
		.command = unused,
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
	SessionFixture fixture;
	fixture.tables = tables;
	CHECK(fixture.Run());
	CHECK(_scheduler_tick_scope_ended);
	CHECK(_scheduler_pool_reads == 2);
	CHECK(_scheduler_post_visits == 1); // Deleted higher shell must never be dereferenced.
	CHECK(_vehicle_pool.GetNumItems() == 1);
	_vehicle_pool.CleanPool();
}

TEST_CASE("Vehicle maintenance consumed native ABI")
{
	ABI_LAYOUT_CHECK;
#include "abi-layout-maintenance.inc"
}


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
	v->Record().breakdown_ctr = 1;
	v->breakdown_delay = 3;
	v->SetType(VEH_TRAIN);
	for (uint8_t tick : {1, 2, 3}) {
		v->Record().tick_counter = tick;
		CHECK(v->HandleBreakdown());
		CHECK(v->breakdown_delay == 3);
	}
	v->Record().tick_counter = 4;
	CHECK(v->HandleBreakdown());
	CHECK(v->breakdown_delay == 2);
	v->SetType(VEH_ROAD);
	v->breakdown_delay = 1;
	v->Record().tick_counter = 2;
	CHECK(v->HandleBreakdown());
	CHECK(v->Record().breakdown_ctr == 0);
	CHECK(v->breakdown_delay == 0);
	v->SetType(VEH_AIRCRAFT);
	v->Record().breakdown_ctr = 1;
	v->breakdown_delay = 1;
	CHECK_FALSE(v->HandleBreakdown());
	CHECK(v->breakdown_delay == 1);
	CHECK(v->Record().breakdown_ctr == 1);
	v->SetType(VEH_ROAD);
	v->Record().breakdown_ctr = 1;
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
		CHECK(current->Record().cur_speed == 0);
		CHECK(current->Record().breakdown_ctr == 1);
		CHECK(current->breakdowns_since_last_service == 255);
		current->SetVehStatus(current->GetVehStatus().Set(VehState::Hidden));
		return true;
	};
	maintenance.mark_dirty = [](OpenTTDVehicle *handle) noexcept {
		CHECK(VehicleOf(handle)->GetVehStatus().Test(VehState::Hidden));
		VehicleOf(handle)->SetType(VEH_AIRCRAFT);
	};
	v->Record().cur_speed = 10;
	v->Record().breakdown_ctr = 2;
	v->breakdown_delay = 5;
	v->breakdowns_since_last_service = 255;
	CHECK_FALSE(openttd_rust_vehicle_handle_breakdown(RustVehicle(v), &base));
	CHECK(v->breakdown_delay == 5);
	CHECK(v->GetType() == VEH_AIRCRAFT);
	v->SetType(VEH_EFFECT);
	v->Record().breakdown_ctr = 3;
	v->current_order.MakeLoading(false);
	CHECK_FALSE(v->HandleBreakdown());
	CHECK(v->Record().breakdown_ctr == 3);
	v->current_order.MakeDummy();
	CHECK_FALSE(v->HandleBreakdown());
	CHECK(v->Record().breakdown_ctr == 2);
	pool.CleanPool();
	CHECK(Vehicle::GetNumItems() == 0);
}

TEST_CASE("Vehicle occupancy - canonical links, stale removal and copied cursors")
{
	Map::Allocate(64, 64);
	REQUIRE(Vehicle::CanAllocateItem(2));
	auto *a = new Vehicle(VEH_ROAD), *b = new Vehicle(VEH_ROAD);
	a->SetTile(TileXY(1, 1));
	b->SetTile(TileXY(1, 1));
	for (auto t : {TileXY(1, 1), TileXY(2, 1), TileXY(1, 2), TileXY(2, 2)}) SetTileHeight(t, 1);
	a->Record().z_pos = b->Record().z_pos = 8;
	a->UpdatePosition(); b->UpdatePosition();
	REQUIRE(GetTileMaxPixelZ(a->GetTile()) == 8);
	CHECK(EnsureNoVehicleOnGround(a->GetTile()).Failed());
	a->Record().z_pos = b->Record().z_pos = 9;
	CHECK(EnsureNoVehicleOnGround(a->GetTile()).Succeeded());
	const auto &s = GetRustVehicleServices();
	CHECK(s.tile_links(RustVehicle(a)) == &a->tile_links);
	CHECK(s.tile_links(RustVehicle(b)) == &b->tile_links);
	a->Record().x_pos = 17; a->Record().y_pos = 29;
	CHECK(b->tile_links.next == RustVehicle(a));
	CHECK(a->tile_links.prev == &b->tile_links.next);
	auto it = VehiclesOnTile(a->GetTile()).begin(), copy = it;
	// Removal does not resolve map dimensions.
	auto removal = s;
	removal.map_log_x = []() noexcept -> uint32_t { CHECK(false); return 0; };
	openttd_rust_occupancy_update(&removal, RustVehicle(b), true);
	CHECK(b->tile_links.current == nullptr);
	CHECK(b->tile_links.next == RustVehicle(a)); // Source retains stale next.
	++it; ++copy;
	CHECK(*it == a); CHECK(it == copy);
	ResetVehicleHash();
	CHECK(a->tile_links.current == nullptr);
	CHECK(b->tile_links.next == RustVehicle(a));
	a->UpdatePosition(); b->UpdatePosition();
	CHECK(*VehiclesOnTile(a->GetTile()).begin() == b);
	delete b; delete a;
}
