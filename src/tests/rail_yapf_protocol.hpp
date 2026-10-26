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
#include "../rust/movement_ffi.h"
#include <array>
#include <memory>
#include <vector>
namespace RailYapfProbe {
inline uint32_t goal = 9;
inline bool hill = false, rollback = false;
inline bool follow_fail = false;
inline std::vector<uint32_t> follows, writes;
inline std::vector<char> follow_reads;
inline std::array<uint32_t, 8> stats;
inline uint32_t flushes, limits, red, restored;
inline OpenTTDRailTrain Train(void *) noexcept
{
	OpenTTDRailTrain train{};
	train.compatible = train.all_compatible = 1; train.dest_tile = goal; train.speed = 100;
	return train;
}
inline OpenTTDRailTile Tile(uint32_t tile, uint8_t td) noexcept
{
	OpenTTDRailTile info{};
	info.flags = 1; info.uphill = hill && tile == 2;
	info.signal_along = rollback && (td & 8) != 0;
	info.signal_type = 4; info.signal_green = 1;
	return info;
}
inline OpenTTDRailFollow Follow(void *, uint32_t tile, uint8_t td, uint64_t, uint8_t) noexcept
{
	follows.push_back((tile << 4) | td);
	follow_reads.push_back('f');
	return {tile + 1, 0, (1 << 0) | (1 << 2), static_cast<uint8_t>(!follow_fail), 0, 0, 0};
}
inline OpenTTDRailSpeed Speed(uint32_t) noexcept { follow_reads.push_back('s'); return {0, INT32_MAX}; }
inline uint8_t Station(uint32_t, uint32_t) noexcept { return 0; }
inline uint32_t Platform(uint32_t, uint8_t) noexcept { return 0; }
inline uint32_t Closest(void *, uint8_t) noexcept { return goal; }
inline uint8_t Write(uint32_t tile, uint8_t td, uint8_t op) noexcept
{
	writes.push_back((tile << 12) | (td << 4) | op);
	return !(rollback && tile == 1 && op == 1);
}
inline void Output(void *, uint8_t, const OpenTTDRailStep *) noexcept {}
inline void Debug(void *, uint8_t kind, const uint32_t *data, uint32_t count) noexcept
{
	if (kind == 0) std::copy_n(data, count, stats.begin());
	if (kind == 2) ++flushes;
	if (kind == 5 && data[0] == 0) ++limits;
	if (kind == 5 && data[0] == 1) ++red;
	if (kind == 5 && data[0] == 2) ++restored;
}
inline OpenTTDRailStep Run(uint32_t limit, uint32_t curve = 29, bool cache = false, bool reserve = false, uint8_t kind = 0)
{
	follows.clear(); writes.clear(); stats = {}; flushes = limits = red = restored = 0;
	follow_reads.clear();
	auto shared = OpenTTDFixtureSharedServices(nullptr, OpenTTDFixtureMapService<uint32_t, void *>, OpenTTDFixtureMapService<uint32_t, int32_t, int32_t, uint32_t *>);
	/* PBS is a Rust callee: provide its typed map/follower leaves rather than
	 * stubbing whole waiting/reservation policies. */
	std::unique_ptr<OpenTTDTrainState, decltype(&openttd_rust_train_state_destroy)> state{openttd_rust_train_state_new(), openttd_rust_train_state_destroy};
	OpenTTDRailMapServices rail{};
	rail.is_rail_depot = [](uint32_t) noexcept { return false; };
	rail.has_signal_trackdir = [](uint32_t, uint8_t) noexcept { return false; };
	rail.rail_reservation = [](uint32_t) noexcept { follow_reads.push_back('r'); return TRACK_BIT_NONE; };
	rail.set_signal_state = [](uint32_t tile, uint8_t td, uint8_t value) noexcept { Write(tile, td, value ? 6 : 5); };
	OpenTTDTrainServices train{};
	train.read_tile = [](OpenTTDTrainHandle) noexcept -> uint32_t { return 0; };
	openttd_rust_train_state_set_track(state.get(), TRACK_BIT_X);
	train.read_trackdir = [](OpenTTDTrainHandle) noexcept -> OpenTTDTrainTrackdirRead { return {0, 0, 1}; };
	train.first_track = [](OpenTTDTrainHandle, uint8_t) noexcept -> uint64_t { return 0; };
	train.track_direction = [](OpenTTDTrainHandle, uint8_t, uint8_t) noexcept -> uint64_t { return 0; };
	train.read_owner = [](OpenTTDTrainHandle) noexcept -> uint8_t { return 0; };
	OpenTTDPbsServices pbs{.shared = &shared, .rail = &rail, .train = &train};
	pbs.handle = [](const ::Train *v) noexcept { return OpenTTDTrainHandle{const_cast<::Train *>(v), reinterpret_cast<OpenTTDTrainState *>(const_cast<::Train *>(v))}; };
	pbs.show_reservation = []() noexcept { return false; };
	pbs.try_track = [](uint32_t tile, uint8_t track) noexcept { return Write(tile, track, 1) != 0; };
	pbs.unreserve_track = [](uint32_t tile, uint8_t track) noexcept { Write(tile, track, 2); };
	pbs.all_compatible = [](uint64_t) noexcept -> uint64_t { return 1; };
	pbs.follow = [](OpenTTDTrainHandle, uint8_t, uint64_t, OpenTTDTrainReservationFollow *ft) noexcept { ft->new_tile = ft->old_tile + 1; ft->dirs = 0x1001; ft->exitdir = 0; return true; };
	pbs.tile_rail_type = [](uint32_t) noexcept { return RAILTYPE_RAIL; };
	pbs.rail90 = [](uint8_t, uint8_t, bool) noexcept { return false; };
	shared.context = &pbs;
	shared.pbs_services = [](const OpenTTDSharedServices *s) noexcept { return static_cast<const OpenTTDPbsServices *>(s->context); };
	shared.train_trackdir = [](const ::Train *) noexcept { return TRACKDIR_X_NE; };
	shared.forbid_90_deg = []() noexcept { return false; };
	shared.tile_type = [](uint32_t) noexcept -> uint8_t { return 1; };
	shared.is_plain_rail = [](uint32_t) noexcept { return true; };
	shared.mark_dirty = [](uint32_t) noexcept {};
	static OpenTTDSharedServices world{};
	static OpenTTDRailMapServices movement_rail{};
	static OpenTTDRailServices ground{};
	static OpenTTDMovementServices movement{};
	world.tile_type = [](uint32_t) noexcept -> uint8_t { return 1; };
	movement_rail.rail_tile_type = [](uint32_t) noexcept -> RailTileType { return static_cast<RailTileType>(0); };
	ground.tracks = [](uint32_t) noexcept -> TrackBits { return static_cast<TrackBits>(5); };
	movement.rail = &ground;
	movement.shared = &world; movement.rail_map = &movement_rail;
	shared.movement_services = []() noexcept -> const OpenTTDMovementServices * { return &movement; };
	OpenTTDRailInput input{}; input.shared = &shared; input.vehicle = reinterpret_cast<const ::Train *>(state.get());
	input.settings.max_nodes = limit; input.settings.curve45 = curve;
	input.settings.slope = 50; input.settings.lookahead = !cache;
	input.map_x = 128; input.reserve = reserve;
	input.kind = kind;
	const OpenTTDRailLeaves leaves{.train = Train, .tile = Tile, .follow = Follow, .speed = Speed, .compatible_station = Station, .platform_length = Platform, .closest_station = Closest, .output = Output, .debug = Debug};
	std::unique_ptr<OpenTTDRailYapf, decltype(&openttd_rust_rail_destroy)> owner{openttd_rust_rail_new(&input, &leaves), openttd_rust_rail_destroy};
	return openttd_rust_rail_step(owner.get());
}
}
#endif
