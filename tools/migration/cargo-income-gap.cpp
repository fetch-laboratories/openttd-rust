/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file cargo-income-gap.cpp Unchanged-reference arithmetic unreachable in road saves. */
#include "stdafx.h"
#include "core/overflowsafe_type.hpp"
#include "core/bitmath_func.hpp"
#include "tests/cargo_payment_stubs.hpp"
#include "rust/cargo_registry_ffi.h"
using Money = OverflowSafeInt64;
using CargoType = uint8_t;
using TileIndex = uint32_t;
enum class CargoCallbackMask { ProfitCalc };
constexpr uint16_t CBID_CARGO_PROFIT_CALC = 0, CALLBACK_FAILED = 0xFFFF;
static uint16_t callback_value;
static uint32_t callback_argument;
struct CargoSpec {
	Money current_payment;
	uint8_t transit_periods[2];
	bool valid, use_callback;
	struct Mask { bool value; bool Test(CargoCallbackMask) const { return value; } } callback_mask;
	bool IsValid() const { return this->valid; }
	static const CargoSpec *Get(CargoType);
};
static CargoSpec spec;
const CargoSpec *CargoSpec::Get(CargoType) { return &spec; }
static uint16_t GetCargoCallback(uint16_t, uint32_t, uint32_t arg, const CargoSpec *) { callback_argument = arg; return callback_value; }
struct CargoPacket {
	Money feeder;
	uint32_t distance;
	uint16_t periods;
	Money GetFeederShare(uint) const { return this->feeder; }
	uint32_t GetDistance(TileIndex) const { return this->distance; }
	uint16_t GetPeriodsInTransit() const { return this->periods; }
};
struct CargoPayment {
	Money visual_transfer;
	Money PayTransfer(CargoType, const CargoPacket *, uint, TileIndex);
};
static struct { struct { uint8_t feeder_payment_share; } economy; } _settings_game;
#include "cargo-reference.inc"
#include "cargo-destination-gap.inc"
int main()
{
	OpenTTDCargoServices s = CargoTestServices();
	s.spec = [](uint8_t, OpenTTDCargoSpec *out) noexcept { *out = {spec.current_payment.base(), uint8_t(spec.valid), uint8_t(spec.callback_mask.value), spec.transit_periods[0], spec.transit_periods[1]}; };
	static const OpenTTDCargoRegistryServices registry{
		.spec = [](uint8_t) noexcept -> const CargoSpec * { return &spec; },
		.invocation = [](const CargoSpec *, OpenTTDCargoInvocation *) noexcept {},
		.callback_frame = [](const OpenTTDCargoInvocation *invocation, int32_t *, size_t) noexcept -> uint16_t { callback_argument = invocation->param2; return callback_value; },
	};
	s.registry = &registry;
	s.feeder = [](const void *p, uint32_t) noexcept { return static_cast<const CargoPacket *>(p)->feeder.base(); };
	s.setting = [](uint8_t) noexcept -> uint32_t { return _settings_game.economy.feeder_payment_share; };
	uint64_t cases = 0;
	for (int64_t payment : {INT64_MIN, int64_t(INT32_MIN), int64_t(-1), int64_t(0), int64_t(6000), int64_t(INT32_MAX), INT64_MAX}) {
		for (uint8_t p1 : {0, 20, 255}) for (uint8_t p2 : {0, 20, 224, 225, 255}) {
			for (uint32_t pieces : {0u, 1u, 255u, 256u, 65535u, 0x80000000u, UINT32_MAX}) {
				for (uint32_t distance : {0u, 1u, 65535u, 65536u, 0x80000000u, UINT32_MAX}) {
					for (uint16_t periods : {0, 20, 244, 245, 255, 256, 65535}) {
						for (uint16_t callback : {0u, 0x3FFFu, 0x4000u, 0x7FFFu, 0x8000u, 0xFFFFu}) {
							callback_value = callback;
							spec = {Money(payment), {p1, p2}, true, true, {true}};
							Money expected = GetTransportedGoodsIncome(pieces, distance, periods, 0);
							auto argument = callback_argument;
							auto actual = openttd_rust_cargo_income(&s, pieces, distance, periods, 0);
							if (actual != expected.base() || argument != callback_argument) return 1;
							cases++;
						}
					}
				}
			}
		}
	}
	for (int64_t initial : {INT64_MIN, int64_t(-1), int64_t(0), INT64_MAX}) {
		for (int64_t feeder : {INT64_MIN, int64_t(-1), int64_t(0), INT64_MAX}) {
			for (uint8_t share : {0, 75, 100, 255}) {
				_settings_game.economy.feeder_payment_share = share;
				CargoPacket packet{Money(feeder), 250, 45};
				CargoPayment reference{Money(initial)};
				auto *candidate = openttd_rust_cargo_payment_new(nullptr, 0xFFFF);
				candidate->fields = {nullptr, INT64_MIN, INT64_MAX, initial};
				auto expected = reference.PayTransfer(0, &packet, 65535, 0);
				auto actual = openttd_rust_cargo_payment_transfer(candidate, &s, 0, &packet, 65535, 250, 45);
				const auto &state = candidate->fields;
				if (actual != expected.base() || state.visual_transfer != reference.visual_transfer.base() || state.route_profit != INT64_MIN || state.visual_profit != INT64_MAX) return 2;
				openttd_rust_cargo_payment_destroy(candidate, nullptr, 1);
			}
		}
	}
	spec.valid = false;
	if (openttd_rust_cargo_income(&s, UINT32_MAX, UINT32_MAX, 65535, 0) != 0) return 3;
	if (CheckCargoDestinationGap(s)) return 4;
	std::printf("cargo income gap: %llu income cases, 64 transfer saturation cases passed\n", static_cast<unsigned long long>(cases));
}
