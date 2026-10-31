/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file rail_yapf_protocol.hpp Bounded rail search/cache/reservation ABI fixture. */
#ifndef RAIL_YAPF_PROTOCOL_HPP
#define RAIL_YAPF_PROTOCOL_HPP
#include "../rust/rail_yapf_ffi.h"
#include "../rust/pbs_ffi.h"
#include "../track_type.h"
#include "../rail_type.h"
#include "follower_fixture.hpp"
#include "follower_probe.hpp"
#include <array>
#include <memory>
#include <vector>
namespace RailYapfProbe {
inline uint32_t goal = 9;
inline constexpr uint32_t start = 32 + 32 * 128;
inline bool hill = false, rollback = false;
inline bool follow_fail = false;
inline std::vector<uint32_t> follows, writes;
inline std::vector<char> follow_reads;
inline size_t search_read_count;
inline std::array<uint32_t, 8> stats;
inline uint32_t flushes, limits, red, restored;
inline OpenTTDRailTrain Train(void *) noexcept
{
	OpenTTDRailTrain train{};
	train.compatible = train.all_compatible = 1; train.tile = train.virtual_tile = train.rear_tile = train.rear_virtual_tile = start; train.dest_tile = start - goal; train.speed = 100;
	return train;
}
inline OpenTTDRailTile Tile(uint32_t tile, uint8_t td) noexcept
{
	OpenTTDRailTile info{};
	info.flags = 1; info.uphill = hill && tile == start - 2;
	info.signal_along = rollback && (td & 8) != 0;
	info.signal_type = 4; info.signal_green = 1;
	return info;
}
inline OpenTTDTestFollow Follow(uint32_t tile, uint8_t td)
{
	return OpenTTDTestFollowOnce(FollowerFixture::active->shared, TRANSPORT_RAIL, nullptr, 0, 1, false, tile, td);
}
inline uint32_t Platform(uint32_t, uint8_t) noexcept { return 0; }
inline uint32_t Closest(void *, uint8_t) noexcept { return start - goal; }
inline uint8_t Write(uint32_t tile, uint8_t td, uint8_t op) noexcept
{
	tile = start - tile;
	writes.push_back((tile << 12) | (td << 4) | op);
	return !(rollback && tile == 1 && op == 1);
}
inline void Output(void *, uint8_t, const OpenTTDRailStep *) noexcept {}
inline void Debug(void *, uint8_t kind, const uint32_t *data, uint32_t count) noexcept
{
	if (kind == 0) {
		std::copy_n(data, count, stats.begin());
		search_read_count = follow_reads.size();
	}
	if (kind == 2) ++flushes;
	if (kind == 5 && data[0] == 0) ++limits;
	if (kind == 5 && data[0] == 1) ++red;
	if (kind == 5 && data[0] == 2) ++restored;
}
inline OpenTTDRailStep Run(uint32_t limit, uint32_t curve = 29, bool cache = false, bool reserve = false, uint8_t kind = 0)
{
	follows.clear(); writes.clear(); stats = {}; flushes = limits = red = restored = 0;
	follow_reads.clear();
	search_read_count = 0;
	static FollowerFixture map{128, 128};
	FollowerFixture::active = &map;
	auto &shared = map.shared;
	for (uint32_t y = 1; y < 127; ++y) for (uint32_t x = 1; x < 127; ++x) map.Tile(x + y * 128, MP_RAILWAY, TRACK_BIT_ALL);
	if (follow_fail) map.tiles.base[start - 1].m5 = 0;
	// Owner lookup marks construction; reservation and speed reads use their
	// shared services, including origin and reservation-target probes.
	map.vehicle.owner = [](OpenTTDVehicle *) noexcept -> uint8_t { follow_reads.push_back('f'); return 0; };
	map.rail.rail_reservation = [](uint32_t) noexcept { follow_reads.push_back('r'); return TRACK_BIT_NONE; };
	map.physics.rail_speed = [](RailType) noexcept -> uint16_t { follow_reads.push_back('s'); return 0; };
	map.train.read_tile = [](OpenTTDTrainHandle) noexcept -> uint32_t { return start; };
	map.train.read_trackdir = [](OpenTTDTrainHandle) noexcept -> OpenTTDTrainTrackdirRead { return {0, 0, 1}; };
	map.train.first_track = [](OpenTTDTrainHandle, uint8_t) noexcept -> uint64_t { return 0; };
	map.train.track_direction = [](OpenTTDTrainHandle, uint8_t, uint8_t) noexcept -> uint64_t { return 0; };
	map.pbs.try_track = [](uint32_t tile, uint8_t track) noexcept { return Write(tile, track, 1) != 0; };
	map.pbs.unreserve_track = [](uint32_t tile, uint8_t track) noexcept { Write(tile, track, 2); };
	map.pbs.blocking = [](uint32_t, uint8_t) noexcept { return false; };
	map.pbs.has_pbs = [](uint32_t, uint8_t) noexcept { return false; };
	map.rail.set_signal_state = [](uint32_t tile, uint8_t td, uint8_t value) noexcept { Write(tile, td, value ? 6 : 5); };
	shared.train_trackdir = [](const ::Train *) noexcept { return TRACKDIR_X_NE; };
	OpenTTDRailInput input{}; input.shared = &shared; input.vehicle = reinterpret_cast<const ::Train *>(map.Vehicle()); input.tile = start;
	input.settings.max_nodes = limit; input.settings.curve45 = curve;
	input.settings.slope = 50; input.settings.lookahead = !cache;
	input.map_x = 128; input.reserve = reserve;
	input.kind = kind;
	const OpenTTDRailLeaves leaves{.train = Train, .tile = Tile, .closest_station = Closest, .output = Output, .debug = Debug};
	std::unique_ptr<OpenTTDRailYapf, decltype(&openttd_rust_rail_destroy)> owner{openttd_rust_rail_new(&input, &leaves), openttd_rust_rail_destroy};
	return openttd_rust_rail_step(owner.get());
}
}
#endif
