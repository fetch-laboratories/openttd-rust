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
		.callback = [](uint8_t, uint32_t) noexcept -> uint16_t { std::abort(); },
		.station_read = [](uint16_t, uint8_t, uint8_t) noexcept -> uint32_t { std::abort(); },
		.industry_read = [](void *, uint8_t, uint32_t) noexcept -> uint32_t { std::abort(); },
		.industry_write = [](void *, uint8_t, uint32_t, uint32_t) noexcept { std::abort(); },
		.resolver = nullptr,
		.accept = [](void *, uint32_t, uint32_t) noexcept { std::abort(); },
		.statistics = [](uint16_t, uint8_t, uint8_t, uint32_t, uint8_t) noexcept { std::abort(); },
		.subsidies = []() noexcept -> const OpenTTDSubsidyServices * { std::abort(); },
		.industry_effect = [](void *, uint8_t) noexcept { std::abort(); },
		.vehicle_read = [](void *, uint8_t) noexcept -> uint32_t { std::abort(); },
		.settle = [](void *, uint8_t, int64_t, int64_t) noexcept -> uint32_t { std::abort(); },
		.feeder = [](const void *, uint32_t) noexcept -> int64_t { std::abort(); },
		.setting = [](uint8_t) noexcept -> uint32_t { std::abort(); },
	};
}
#endif /* CARGO_PAYMENT_STUBS_HPP */
