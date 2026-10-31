/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file cargo_payment_stubs.hpp Complete typed tables; unexpected fixture calls abort. */
#ifndef CARGO_PAYMENT_STUBS_HPP
#define CARGO_PAYMENT_STUBS_HPP
#include "../rust/cargo_payment_ffi.h"
#include <cstdlib>
inline OpenTTDCargoServices CargoTestServices()
{
	return {
		.spec = [](uint8_t, OpenTTDCargoSpec *) noexcept { std::abort(); },
		.registry = nullptr,
		.station_company = [](const Station *) noexcept -> uint8_t { std::abort(); },
		.station_town = [](const Station *) noexcept -> const OpenTTDTownHandle * { std::abort(); },
		.resolver = nullptr,
		.subsidies = []() noexcept -> const OpenTTDSubsidyServices * { std::abort(); },
		.industry_delivery = nullptr,
		.last_station = [](OpenTTDVehicle *) noexcept -> uint16_t { std::abort(); },
		.vehicle_owner = [](OpenTTDVehicle *) noexcept -> uint8_t { std::abort(); },
		.detach_payment = [](OpenTTDVehicle *) noexcept { std::abort(); },
		.finance = nullptr,
		.vehicles = nullptr,
		.local_company = []() noexcept -> bool { std::abort(); },
		.expense = [](OpenTTDVehicle *, bool) noexcept -> ExpensesType { std::abort(); },
		.profit = [](OpenTTDVehicle *) noexcept -> int64_t { std::abort(); },
		.x = [](OpenTTDVehicle *) noexcept -> int32_t { std::abort(); },
		.y = [](OpenTTDVehicle *) noexcept -> int32_t { std::abort(); },
		.z = [](OpenTTDVehicle *) noexcept -> int32_t { std::abort(); },
		.cost_animation = [](int32_t, int32_t, int32_t, int64_t) noexcept { std::abort(); },
		.feeder_animation = [](int32_t, int32_t, int32_t, int64_t, int64_t) noexcept { std::abort(); },
		.subsidy_multiplier = []() noexcept -> uint8_t { std::abort(); },
		.feeder_payment_share = []() noexcept -> uint8_t { std::abort(); },
	};
}
#endif /* CARGO_PAYMENT_STUBS_HPP */
