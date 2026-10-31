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
static const OpenTTDCargoStorageServices services{
	.can_allocate = CannotAllocate,
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
