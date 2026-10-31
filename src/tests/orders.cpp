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
#ifdef WITH_RUST
#include "../timer/timer_game_economy.h"
#include "../rust/services_ffi.h"
#include "../rust/command_services.h"
#include "../rust/station_command_ffi.h"
#include "../rust/vehicle_ffi.h"
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
	orders.Move(2, 9);
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
struct TimetableDateFixture {
	OpenTTDVehicleOrderState vehicle{};
	OpenTTDOrderListState list{};
	RustOrderVector orders;
	uint8_t owner = 0;
	uint32_t owner_reads = 0;
	bool primary = true;
	TimetableDateFixture() { this->vehicle.orders = this; }
};
}
TEST_CASE("Orders timetable - signed native economy dates cross into Rust")
{
	const auto saved_date = TimerGameEconomy::date;
	TimetableDateFixture fixture;
	OpenTTDOrdersLeaves leaves{};
	leaves.vehicle = [](void *v) noexcept { return &static_cast<TimetableDateFixture *>(v)->vehicle; };
	leaves.list = [](void *v) noexcept { return &static_cast<TimetableDateFixture *>(v)->list; };
	leaves.vector = [](void *v) noexcept { return static_cast<TimetableDateFixture *>(v)->orders.GetRustOwner(); };
	leaves.primary_vehicle = [](void *) noexcept -> uint64_t { return 1; };
	auto base = GetRustVehicleServices();
	base.owner = [](OpenTTDVehicle *v) noexcept -> uint8_t { return reinterpret_cast<TimetableDateFixture *>(v)->owner; };
	OpenTTDStationCommandServices station{};
	station.command = &GetRustCommandServices(); station.cost = &GetRustCommandCostServices();
	leaves.base = &base; leaves.station_commands = &station;
	AutoRestoreBackup<CompanyID> actor(_current_company, CompanyID(0));
	leaves.tick_counter = []() noexcept -> uint64_t { return UINT64_MAX; };
	leaves.economy_fraction = []() noexcept -> uint64_t { return 0; };
	leaves.maximum_date = []() noexcept -> uint64_t { return INT32_MAX; };
	leaves.timetable_year_limit = []() noexcept -> uint64_t { return 366; };
	leaves.command_error = [](void *out, uint32_t error, uint32_t detail) noexcept { CHECK(error == 0); CHECK(detail == 0); *static_cast<CommandCost *>(out) = CMD_ERROR; };
	struct Case { int32_t date; uint64_t tick; bool valid; };
	for (const Case test : {Case{INT32_MIN, UINT64_MAX, false}, Case{-1, 73, true}, Case{0, UINT64_MAX, true}, Case{INT32_MAX, UINT64_MAX, true}, Case{INT32_MAX, 73, false}}) {
		CAPTURE(test.date, test.tick);
		TimerGameEconomy::date = TimerGameEconomy::Date{test.date};
		CommandCost result;
		CHECK(openttd_rust_timetable_start(&fixture, 0, 0, test.tick, &result, &leaves) == uint32_t(test.valid));
		CHECK(result.Failed() == !test.valid);
	}
	TimerGameEconomy::date = saved_date;
}
TEST_CASE("Orders ownership - early validation and complete native error storage")
{
	TimetableDateFixture fixture;
	OpenTTDOrdersLeaves leaves{};
	leaves.vehicle = [](void *v) noexcept { return &static_cast<TimetableDateFixture *>(v)->vehicle; };
	leaves.primary_vehicle = [](void *v) noexcept -> uint64_t { return static_cast<TimetableDateFixture *>(v)->primary; };
	leaves.command_error = [](void *out, uint32_t error, uint32_t detail) noexcept { CHECK(error == 0); CHECK(detail == 0); *static_cast<CommandCost *>(out) = CMD_ERROR; };
	auto base = GetRustVehicleServices();
	base.owner = [](OpenTTDVehicle *v) noexcept -> uint8_t { auto *f = reinterpret_cast<TimetableDateFixture *>(v); ++f->owner_reads; return f->owner; };
	OpenTTDStationCommandServices station{};
	station.command = &GetRustCommandServices(); station.cost = &GetRustCommandCostServices();
	leaves.base = &base; leaves.station_commands = &station;
	AutoRestoreBackup<CompanyID> actor(_current_company, CompanyID(0));
	AutoRestoreBackup<CompanyID> observer(_local_company, CompanyID(0));
	Order order; order.MakeGoToStation(StationID(0));
	fixture.primary = false;
	for (void *v : {static_cast<void *>(nullptr), static_cast<void *>(&fixture)}) {
		CommandCost result;
		CHECK(openttd_rust_command_insert(v, 0, 0, &order, &result, &leaves) == 0);
		CHECK(result.GetErrorMessage() == INVALID_STRING_ID);
		CHECK(result.Failed());
		CHECK(fixture.owner_reads == 0);
	}
	fixture.primary = true;
	for (CompanyID local : {CompanyID(0), COMPANY_SPECTATOR}) {
		_local_company = local;
		for (bool destination : {false, true}) {
			fixture.owner = destination ? 0 : OWNER_NONE.base();
			leaves.vehicle_type = destination ? +[](void *) noexcept -> uint64_t { return VEH_ROAD; } : OpenTTDOrdersLeaves{}.vehicle_type;
			leaves.valid_station = []([[maybe_unused]] uint16_t id) noexcept -> uint64_t { return 1; };
			leaves.station_owner = []([[maybe_unused]] uint16_t id) noexcept -> uint64_t { return OWNER_WATER.base(); };
			for (bool timetable : {false, true}) {
				if (timetable && destination) continue;
				CommandCost result;
				uint32_t reads = fixture.owner_reads;
				CHECK((timetable ? openttd_rust_timetable_start(&fixture, 0, 0, 0, &result, &leaves) : openttd_rust_command_insert(&fixture, 0, 0, &order, &result, &leaves)) == 0);
				CHECK(fixture.owner_reads == reads + 1);
				Owner owner = destination ? OWNER_WATER : OWNER_NONE;
				CHECK(result.GetErrorMessage() == STR_ERROR_OWNED_BY);
				CHECK(result.GetErrorOwner() == (local == 0 ? owner : INVALID_OWNER));
				CHECK(result.GetEncodedMessage() == (local == 0 ? GetEncodedString(STR_ERROR_OWNED_BY, STR_COMPANY_SOMEONE, std::monostate{}) : EncodedString{}));
			}
		}
	}
}

#endif
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
