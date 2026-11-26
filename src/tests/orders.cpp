/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file orders.cpp Typed order storage lifetime and native ABI agreement. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#include "../order_base.h"
#include "../base_consist.h"
#include "../timer/timer_game_economy.h"
#include "../rust/services_ffi.h"
#include "../rust/orders_ffi.h"
#include "../rust/station_catchment_ffi.h"
#include "../rust/vehicle_maintenance_ffi.h"
#include "../ground_vehicle.hpp"
#include "../timer/timer_game_tick.h"
#include "../vehicle_base.h"
#include "../rust/command_services.h"
#include "../rust/station_command_ffi.h"
#include "../rust/vehicle_ffi.h"
#include "../rust/vehicle_record_ffi.h"
#include "../company_func.h"
#include "../core/backup_type.hpp"
#include "../strings_func.h"
#include "../table/strings.h"
TEST_CASE("Order vector preserves values and allocation invalidation")
{
	RustOrderVector orders;
	std::vector<Order> original;
	for (uint16_t i = 0; i < 300; ++i) {
		Order order;
		order.MakeGoToStation(StationID(i));
		auto before = orders.data();
		auto reference_before = original.data();
		orders.emplace_back(Order(order));
		original.emplace_back(order);
		CHECK((before != orders.data()) == (reference_before != original.data()));
		CHECK(orders.size() == original.size());
	}
	/* Input aliasing survives a growth reallocation and overlapping insertion. */
	orders.emplace(orders.begin() + 2, std::move(orders[0]));
	original.emplace(original.begin() + 2, std::move(original[0]));
	orders.erase(orders.begin() + 1);
	original.erase(original.begin() + 1);
	std::rotate(orders.begin() + 2, orders.begin() + 3, orders.begin() + 10);
	std::rotate(original.begin() + 2, original.begin() + 3, original.begin() + 10);
	for (size_t i = 0; i < original.size(); ++i) CHECK(orders[i].Equals(original[i]));
	auto allocation = orders.data();
	orders.clear();
	orders.resize(2);
	CHECK(orders.data() == allocation);
	CHECK(orders[0].Equals(Order{}));
	CHECK(orders[1].Equals(Order{}));
}
namespace {
/* A live road vehicle in `_vehicle_pool` with its order list, built and
 * retired as `vehicle_scheduler.cpp` builds its vehicles. */
struct LiveOrdersVehicle {
	Vehicle *v;
	LiveOrdersVehicle()
	{
		REQUIRE(Vehicle::CanAllocateItem());
		REQUIRE(OrderList::CanAllocateItem());
		this->v = new Vehicle(VEH_ROAD);
		SetBit(this->v->Record().subtype, GVSF_FRONT);
		this->v->Record().owner = CompanyID(0).base();
		this->v->orders = new OrderList(this->v);
	}
	~LiveOrdersVehicle()
	{
		delete this->v->orders;
		this->v->orders = nullptr;
		delete this->v;
	}
	VehicleID Id() const { return this->v->index; }
};
}
TEST_CASE("Orders timetable - signed native economy dates cross into Rust")
{
	const auto saved_date = TimerGameEconomy::date;
	const auto saved_fraction = TimerGameEconomy::date_fract;
	TimerGameEconomy::date_fract = 0;
	LiveOrdersVehicle fixture;
	AutoRestoreBackup<CompanyID> actor(_current_company, CompanyID(0));
	struct Case { int32_t date; uint64_t days; bool valid; };
	const int32_t max_date = EconomyTime::MAX_DATE.base();
	for (const Case test : {Case{INT32_MIN, 0, false}, Case{-1, 1, true}, Case{0, 0, true}, Case{max_date, 0, true}, Case{max_date, 1, false}, Case{INT32_MAX, 0, false}, Case{INT32_MAX, 1, false}}) {
		CAPTURE(test.date, test.days);
		TimerGameEconomy::date = TimerGameEconomy::Date{test.date};
		const uint64_t tick = TimerGameTick::counter + test.days * Ticks::DAY_TICKS;
		CommandCost result;
		openttd_rust_timetable_start(0, fixture.Id().base(), 0, tick, &result, &GetRustOrdersLeaves());
		CHECK(result.Failed() == !test.valid);
		if (!test.valid) CHECK(result.GetErrorMessage() == INVALID_STRING_ID);
	}
	TimerGameEconomy::date = saved_date;
	TimerGameEconomy::date_fract = saved_fraction;
}
namespace {
/* The destination arm's station lookup and owner slots, on copies of the
 * production tables. */
const OpenTTDStationCatchmentServices *_order_test_catchment;
struct DestinationStub {
	OpenTTDOrdersLeaves leaves = GetRustOrdersLeaves();
	OpenTTDVehicleServices base = *GetRustOrdersLeaves().base;
	OpenTTDVehicleMaintenanceServices maintenance = *GetRustOrdersLeaves().base->maintenance;
	OpenTTDSharedServices shared = *GetRustOrdersLeaves().base->maintenance->shared;
	OpenTTDStationCatchmentServices catchment = *GetRustSharedServices().station_catchment();
	OpenTTDStationCommandServices commands = *GetRustOrdersLeaves().station_commands;
	OpenTTDStationServices station = *GetRustOrdersLeaves().station_commands->station;
	DestinationStub()
	{
		_order_test_catchment = &this->catchment;
		this->catchment.station_if_valid = []([[maybe_unused]] uint16_t id) noexcept { return reinterpret_cast<BaseStation *>(&_order_test_catchment); };
		this->shared.station_catchment = []() noexcept { return _order_test_catchment; };
		this->maintenance.shared = &this->shared;
		this->base.maintenance = &this->maintenance;
		this->station.station_owner = [](OpenTTDBaseStation *) noexcept -> uint8_t { return OWNER_WATER.base(); };
		this->commands.station = &this->station;
		this->leaves.base = &this->base;
		this->leaves.station_commands = &this->commands;
	}
};
}
TEST_CASE("Orders ownership - early validation and complete native error storage")
{
	LiveOrdersVehicle fixture;
	DestinationStub destination_stub;
	AutoRestoreBackup<CompanyID> actor(_current_company, CompanyID(0));
	AutoRestoreBackup<CompanyID> observer(_local_company, CompanyID(0));
	Order order; order.MakeGoToStation(StationID(0));
	ClrBit(fixture.v->Record().subtype, GVSF_FRONT);
	/* An unused pool index, then a vehicle that is not primary. */
	const VehicleID unused{fixture.Id().base() + 1};
	REQUIRE(!Vehicle::IsValidID(unused));
	for (VehicleID veh : {unused, fixture.Id()}) {
		CommandCost result;
		openttd_rust_command_insert(0, veh.base(), 0, &order, &result, &GetRustOrdersLeaves());
		CHECK(result.GetErrorMessage() == INVALID_STRING_ID);
		CHECK(result.Failed());
	}
	SetBit(fixture.v->Record().subtype, GVSF_FRONT);
	for (CompanyID local : {CompanyID(0), COMPANY_SPECTATOR}) {
		_local_company = local;
		for (bool destination : {false, true}) {
			fixture.v->Record().owner = destination ? 0 : OWNER_NONE.base();
			const OpenTTDOrdersLeaves &leaves = destination ? destination_stub.leaves : GetRustOrdersLeaves();
			for (bool timetable : {false, true}) {
				if (timetable && destination) continue;
				CommandCost result;
				if (timetable) {
					openttd_rust_timetable_start(0, fixture.Id().base(), 0, 0, &result, &leaves);
				} else {
					openttd_rust_command_insert(0, fixture.Id().base(), 0, &order, &result, &leaves);
				}
				CHECK(result.Failed());
				Owner owner = destination ? OWNER_WATER : OWNER_NONE;
				CHECK(result.GetErrorMessage() == STR_ERROR_OWNED_BY);
				CHECK(result.GetErrorOwner() == (local == 0 ? owner : INVALID_OWNER));
				CHECK(result.GetEncodedMessage() == (local == 0 ? GetEncodedString(STR_ERROR_OWNED_BY, STR_COMPANY_SOMEONE, std::monostate{}) : EncodedString{}));
			}
		}
	}
}

TEST_CASE("Base consist copies preserve values with independent aliases")
{
	BaseConsist original;
	original.name = "copy";
	original.round_trip_time = 4200;
	original.depot_unbunching_last_departure = 1700;
	original.cur_real_order_index = 23;
	original.vehicle_flags.Set(VehicleFlag::LoadingFinished);
	BaseConsist copied(original);
	CHECK(copied.round_trip_time == 4200);
	CHECK(copied.depot_unbunching_last_departure == 1700);
	CHECK(copied.cur_real_order_index == 23);
	CHECK(copied.vehicle_flags == original.vehicle_flags);
	CHECK(&copied.round_trip_time != &original.round_trip_time);
	copied.round_trip_time = 1;
	CHECK(original.round_trip_time == 4200);
	copied = original;
	CHECK(copied.round_trip_time == 4200);
}
TEST_CASE("Order list owners retire and reconstruct at a reused pool index")
{
	REQUIRE(OrderList::CanAllocateItem());
	OrderList *list = new OrderList();
	OrderListID index = list->index;
	Order order;
	order.MakeDummy();
	list->InsertOrderAt(std::move(order), 0);
	CHECK(list->GetNumOrders() == 1);
	CHECK(list->GetNumManualOrders() == 1);
	delete list;

	/* Save/load uses indexed construction before populating the new owner. */
	list = new (index) OrderList();
	CHECK(list->index == index);
	CHECK(list->GetNumOrders() == 0);
	CHECK(list->GetNumManualOrders() == 0);
	CHECK(list->GetNumVehicles() == 0);
	CHECK(list->GetTimetableDurationIncomplete() == 0);
	delete list;
}
