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
#include "../rust/vehicle_services.h"
#include "../rust/vehicle_maintenance_ffi.h"
#include "abi_layout.hpp"
#include "../rust/fleet_ffi.h"
#include "../../tools/simulation/session-fixture.hpp"
#include "../rust/company_adapter.h"
#include "../rust/company_identity_ffi.h"
#include "../rust/session_ffi.h"
#include "../safeguards.h"

/* A hidden effect shell for the native glue checks below. */
struct SchedulerShellVehicle final : Vehicle {
	SchedulerShellVehicle() : Vehicle(VEH_EFFECT) { this->SetVehStatus(this->GetVehStatus().Set(VehState::Hidden)); }
	bool Tick() override { return true; }
};
TEST_CASE("Vehicle maintenance previous ID preserves the company sentinel")
{
	struct Restore { ~Restore() { _vehicle_pool.CleanPool(); } } restore;
	_vehicle_pool.CleanPool();
	REQUIRE(Vehicle::CanAllocateItem(2));
	auto *head = new (VehicleID(0)) SchedulerShellVehicle();
	auto *part = new (VehicleID(1)) SchedulerShellVehicle();
	head->SetNext(part);
	const auto &services = GetRustCompanyServices();
	CHECK(services.vehicle_previous(head->index.base()) == UINT32_MAX);
	CHECK(services.vehicle_previous(part->index.base()) == head->index.base());
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
/* The native tick leaf's timing scope ends before Rust's post-Tick reads; the rest of this
 * case (growth, pool reads, the fresh type and deletion) is vehicle::scheduler::pool_tests
 * (#736 F). Retires with the native tick leaf. */
static bool _scheduler_tick_scope_ended;
struct SchedulerProbeVehicle final : Vehicle {
	SchedulerProbeVehicle() : Vehicle(VEH_EFFECT) {}
	bool Tick() override
	{
		struct EndScope { ~EndScope() { _scheduler_tick_scope_ended = true; } } scope;
		_scheduler_tick_scope_ended = false;
		return true;
	}
};
TEST_CASE("Base vehicle scheduler - Tick timing scope returns before post-Tick reads")
{
	_vehicle_pool.CleanPool();
	REQUIRE(Vehicle::CanAllocateItem());
	new (VehicleID(0)) SchedulerProbeVehicle();
	auto services = GetRustVehicleSchedulerServices();
	services.tick = [](OpenTTDVehicle *shell) noexcept {
		const bool survives = GetRustVehicleSchedulerServices().tick(shell);
		CHECK(_scheduler_tick_scope_ended);
		return survives;
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
	_vehicle_pool.CleanPool();
}

TEST_CASE("Vehicle maintenance consumed native ABI")
{
	ABI_LAYOUT_CHECK;
#include "abi-layout-maintenance.inc"
}


/* The native VehiclesOnTile iterator copies its cursor; the index rules (ground check, links,
 * stale removal, reset) are vehicle::occupancy::index_tests (#736 F). Retires with the native
 * iterator and the native tile_links member. */
TEST_CASE("Vehicle occupancy - native tile links and copied cursors")
{
	Map::Allocate(64, 64);
	REQUIRE(Vehicle::CanAllocateItem(2));
	auto *a = new Vehicle(VEH_ROAD), *b = new Vehicle(VEH_ROAD);
	a->SetTile(TileXY(1, 1));
	b->SetTile(TileXY(1, 1));
	a->UpdatePosition(); b->UpdatePosition();
	const auto &s = GetRustVehicleServices();
	CHECK(s.tile_links(RustVehicle(a)) == &a->tile_links);
	CHECK(s.tile_links(RustVehicle(b)) == &b->tile_links);
	auto it = VehiclesOnTile(a->GetTile()).begin(), copy = it;
	CHECK(*it == b);
	openttd_rust_occupancy_update(&s, RustVehicle(b), true);
	++it; ++copy;
	CHECK(*it == a); CHECK(it == copy);
	ResetVehicleHash();
	delete b; delete a;
}
