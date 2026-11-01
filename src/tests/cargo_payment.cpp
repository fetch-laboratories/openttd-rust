/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file cargo_payment.cpp Payment lifetime and native serialization ABI. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#include "../core/overflowsafe_type.hpp"
#include "cargo_payment_stubs.hpp"
#include "../company_base.h"
#include "../rust/vehicle_ffi.h"
#include "../economy_type.h"
#include "../newgrf_sound.h"
#include "../sound_type.h"
#include "../safeguards.h"

static std::vector<std::tuple<uint8_t, int64_t, int64_t>> settlement;
static CompanyFinances finances;
static bool local, custom_sound, valid_company;
static uint8_t current;
TEST_CASE("Cargo payment - settlement order, Money boundaries and cleanup")
{
	OpenTTDCargoServices s = CargoTestServices();
	OpenTTDCompanyFinanceServices finance{};
	finance.current_company = []() noexcept { settlement.emplace_back(1, 0, 0); return current; };
	finance.set_current_company = [](uint8_t id) noexcept { settlement.emplace_back(id == 3 ? 2 : 10, id, 0); current = id; };
	finance.company_finances = [](uint8_t id) noexcept -> CompanyFinances * { CHECK(id == 3); return valid_company ? &finances : nullptr; };
	finance.invalidate_company_windows = [](uint8_t id) noexcept { settlement.emplace_back(3, id, 0); };
	OpenTTDVehicleServices vehicles{};
	vehicles.write_profit = [](OpenTTDVehicle *, int64_t value) noexcept { settlement.emplace_back(4, value, 0); };
	vehicles.play_vehicle_sound = [](OpenTTDVehicle *, VehicleSoundEvent event) noexcept { CHECK(event == VSE_LOAD_UNLOAD); settlement.emplace_back(6, 0, 0); return custom_sound; };
	vehicles.play_sound = [](OpenTTDVehicle *, uint16_t sound) noexcept { CHECK(sound == SND_14_CASHTILL); settlement.emplace_back(7, 0, 0); };
	s.finance = &finance;
	s.vehicles = &vehicles;
	s.detach_payment = [](OpenTTDVehicle *) noexcept { settlement.emplace_back(0, 0, 0); };
	s.vehicle_owner = [](OpenTTDVehicle *) noexcept -> uint8_t { return 3; };
	s.local_company = []() noexcept { settlement.emplace_back(5, 0, 0); return local; };
	s.expense = [](OpenTTDVehicle *, bool income) noexcept { CHECK(income); return EXPENSES_TRAIN_REVENUE; };
	s.profit = [](OpenTTDVehicle *) noexcept -> int64_t { return 1000; };
	s.x = [](OpenTTDVehicle *) noexcept -> int32_t { return 11; };
	s.y = [](OpenTTDVehicle *) noexcept -> int32_t { return 22; };
	s.z = [](OpenTTDVehicle *) noexcept -> int32_t { return 33; };
	s.cost_animation = [](int32_t x, int32_t y, int32_t z, int64_t income) noexcept { CHECK(std::tuple{x, y, z} == std::tuple{11, 22, 33}); settlement.emplace_back(9, 0, income); };
	s.feeder_animation = [](int32_t x, int32_t y, int32_t z, int64_t transfer, int64_t income) noexcept { CHECK(std::tuple{x, y, z} == std::tuple{11, 22, 33}); settlement.emplace_back(8, transfer, income); };
	for (bool cleaning : {false, true}) for (bool transfer : {false, true}) for (bool is_local : {false, true}) for (bool custom : {false, true}) for (bool valid : {false, true}) {
		settlement.clear();
		finances = {};
		current = 17;
		local = is_local;
		custom_sound = custom;
		valid_company = valid;
		int shell;
		auto *front = reinterpret_cast<OpenTTDVehicle *>(&shell);
		auto *p = openttd_rust_cargo_payment_new(front, 42);
		p->fields = {front, INT64_MIN, INT64_MIN, transfer ? INT64_MAX : 0};
		CHECK(openttd_rust_cargo_payment_front(p) == front);
		openttd_rust_cargo_payment_destroy(p, &s, cleaning);
		if (cleaning) { CHECK(settlement.empty()); continue; }
		std::vector<uint8_t> order{0, 1, 2};
		if (valid_company) order.push_back(3);
		order.insert(order.end(), {4, 5});
		if (local) { order.push_back(6); if (!custom_sound) order.push_back(7); }
		order.push_back(transfer ? 8 : 9);
		order.push_back(10);
		std::vector<uint8_t> actual;
		for (auto [op, first, second] : settlement) actual.push_back(op);
		CHECK(actual == order);
		CHECK(finances.money.base() == (valid_company ? -INT64_MAX : 0));
		CHECK(finances.yearly_expenses[0][EXPENSES_TRAIN_REVENUE].base() == (valid_company ? INT64_MAX : 0));
		CHECK(std::get<1>(settlement[valid_company ? 4 : 3]) == (transfer ? 744 : 1000));
		CHECK(std::get<2>(settlement[settlement.size() - 2]) == INT64_MAX);
		CHECK(current == 17);
	}
	settlement.clear();
	auto *p = openttd_rust_cargo_payment_new(nullptr, 0xFFFF);
	p->fields = {nullptr, INT64_MAX, 0, 0};
	openttd_rust_cargo_payment_destroy(p, &s, 0);
	CHECK(settlement == std::vector<std::tuple<uint8_t, int64_t, int64_t>>{{0, 0, 0}});
}
