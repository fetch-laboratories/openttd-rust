/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file orders.cpp Typed order storage lifetime and native ABI agreement. */
#include "../stdafx.h"
#include "../rust/clock_ffi.h"
#include "../3rdparty/catch2/catch.hpp"
#include "../order_base.h"
#include "../base_consist.h"
#ifdef WITH_RUST
#include "../timer/timer_game_economy.h"
#include "../rust/services_ffi.h"
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
	leaves.vehicle_ownership = [](void *, void *) noexcept -> uint64_t { return 1; };
	leaves.economy_date = openttd_rust_clock_economy_date;
	leaves.tick_counter = []() noexcept -> uint64_t { return UINT64_MAX; };
	leaves.economy_fraction = []() noexcept -> uint64_t { return 0; };
	leaves.maximum_date = []() noexcept -> uint64_t { return INT32_MAX; };
	leaves.timetable_year_limit = []() noexcept -> uint64_t { return 366; };
	leaves.command_error = [](void *out, uint32_t error, uint32_t detail) noexcept { CHECK(error == 0); CHECK(detail == 0); *static_cast<bool *>(out) = true; };
	struct Case { int32_t date; uint64_t tick; bool valid; };
	for (const Case test : {Case{INT32_MIN, UINT64_MAX, false}, Case{-1, 73, true}, Case{0, UINT64_MAX, true}, Case{INT32_MAX, UINT64_MAX, true}, Case{INT32_MAX, 73, false}}) {
		CAPTURE(test.date, test.tick);
		TimerGameEconomy::date = TimerGameEconomy::Date{test.date};
		bool error = false;
		CHECK(openttd_rust_timetable_start(&fixture, 0, 0, test.tick, &error, &leaves) == uint32_t(test.valid));
		CHECK(error == !test.valid);
	}
	TimerGameEconomy::date = saved_date;
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
