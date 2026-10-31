/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file cargo-split-reference.cpp Named unreachable pool-limit Split failure. */
#include <cstdint>
#include <cstdio>
#include <cassert>
#include "rust/cargo_storage_ffi.h"
using uint = unsigned;
using Money = int64_t;
static unsigned allocation_calls;
static uint8_t CannotAllocate() noexcept { ++allocation_calls; return 0; }
static void *UnusedCreate(const OpenTTDCargoPacketFields *) noexcept { std::abort(); }
static OpenTTDCargoPacket *UnusedPacket(void *) noexcept { std::abort(); }
static void UnusedDestroy(void *) noexcept { std::abort(); }
static uint32_t UnusedRandom(uint32_t) noexcept { std::abort(); }
static uint32_t UnusedCoordinate(uint32_t, uint8_t) noexcept { std::abort(); }
static uint16_t UnusedFlow(const void *, uint8_t, uint16_t, uint16_t, uint16_t, const uint16_t *, size_t, uint8_t *) noexcept { std::abort(); }
static int64_t UnusedPay(void *, uint8_t, uint8_t, const void *, uint32_t, uint32_t) noexcept { std::abort(); }
static void UnusedOrigin(void *, uint16_t, uint32_t, uint8_t) noexcept { std::abort(); }
static OpenTTDCargoFlowMap *UnusedFlowMap(const void *) noexcept { std::abort(); }
static uint32_t UnusedDraw() noexcept { std::abort(); }
static void *UnusedNext(void *) noexcept { std::abort(); }
static const OpenTTDCargoStorageServices services{
	.can_allocate = CannotAllocate,
	.create = UnusedCreate,
	.packet = UnusedPacket,
	.destroy = UnusedDestroy,
	.random = UnusedRandom,
	.coordinate = UnusedCoordinate,
	.flow = UnusedFlow,
	.pay = UnusedPay,
	.origin = UnusedOrigin,
	.flow_map = UnusedFlowMap,
	.random_draw = UnusedDraw,
	.packet_next = UnusedNext,
};

#ifndef WITH_RUST
struct CargoPacket {
	uint16_t count;
	Money feeder_share;
	CargoPacket(uint16_t count, Money feeder_share) : count(count), feeder_share(feeder_share) {}
	CargoPacket(uint16_t count, Money feeder_share, CargoPacket &) : CargoPacket(count, feeder_share) {}
	static bool CanAllocateItem() { return CannotAllocate() != 0; }
	Money GetFeederShare(uint amount) const { return this->feeder_share * amount / this->count; }
	CargoPacket *Split(uint new_size);
};
@UNCHANGED_SPLIT@
#endif

int main()
{
	for (uint16_t count : {uint16_t(1), uint16_t(17), uint16_t(UINT16_MAX)}) for (Money feeder : {INT64_MIN, int64_t(-13), int64_t(0), INT64_MAX}) {
		for (uint amount : {0U, 1U, 11U, UINT32_MAX}) {
			allocation_calls = 0;
#ifdef WITH_RUST
			OpenTTDCargoPacketFields fields;
			fields.count = count; fields.feeder_share = feeder;
			auto *packet = openttd_rust_cargo_packet_new(&fields);
			auto *part = openttd_rust_cargo_packet_split(packet, &services, amount);
			OpenTTDCargoPacketFields after;
			openttd_rust_cargo_packet_export(packet, &after);
			assert(after.count == count && after.feeder_share == feeder);
			openttd_rust_cargo_packet_destroy(packet);
#else
			CargoPacket packet(count, feeder);
			auto *part = packet.Split(amount);
			assert(packet.count == count && packet.feeder_share == feeder);
#endif
			assert(part == nullptr && allocation_calls == 1);
			std::printf("%u %lld %u %u\n", count, static_cast<long long>(feeder), amount, allocation_calls);
		}
	}
}
