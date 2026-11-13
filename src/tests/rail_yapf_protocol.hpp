/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file rail_yapf_protocol.hpp Bounded rail search/cache/reservation fixture over the production search. */
#ifndef RAIL_YAPF_PROTOCOL_HPP
#define RAIL_YAPF_PROTOCOL_HPP
#include "../rust/rail_yapf_ffi.h"
#include "../rust/pbs_ffi.h"
#include "abi-record-ground.h"
#include "abi-record-settings.h"
#include "../track_type.h"
#include "../track_func.h"
#include "../rail_type.h"
#include "../signal_func.h"
#include "follower_fixture.hpp"
#include "follower_probe.hpp"
#include <array>
#include <vector>
extern "C" {
/** One production rail search's result, FindPath statistics and witness count differences. */
struct OpenTTDTestRailYapf {
	int32_t num_steps, open, closed, cache_hits, cost_calcs, cost, distance;
	uint32_t flushes, limits, red, restored;
	uint8_t found, td;
};
OpenTTDTestRailYapf openttd_test_rail_yapf(const OpenTTDSharedServices *, OpenTTDVehicle *, bool safe_tile, uint32_t tile, uint8_t td, bool reserve_track);
}
namespace RailYapfProbe {
inline uint32_t goal = 9;
inline constexpr uint32_t start = 32 + 32 * 128;
inline bool hill = false, rollback = false;
inline bool follow_fail = false;
inline std::vector<uint32_t> follows, writes;
inline std::vector<char> follow_reads;
inline OpenTTDTestRailYapf stats;
/* The settings catalog the search reads: the fixture's four values, every other penalty zero. */
inline uint32_t max_search_nodes, curve45, slope, look_ahead_max_signals, zero;
inline int32_t zero_signed;
inline bool zero_bool;
inline OpenTTDSettingAddresses Settings()
{
	OpenTTDSettingAddresses settings{};
	settings.max_search_nodes = &max_search_nodes;
	settings.rail_crossing_penalty = &zero;
	settings.rail_curve45_penalty = &curve45;
	settings.rail_curve90_penalty = &zero;
	settings.rail_depot_reverse_penalty = &zero;
	settings.rail_doubleslip_penalty = &zero;
	settings.rail_firstred_exit_penalty = &zero;
	settings.rail_firstred_penalty = &zero;
	settings.rail_firstred_twoway_eol = &zero_bool;
	settings.rail_lastred_exit_penalty = &zero;
	settings.rail_lastred_penalty = &zero;
	settings.rail_longer_platform_penalty = &zero;
	settings.rail_longer_platform_per_tile_penalty = &zero;
	settings.rail_look_ahead_max_signals = &look_ahead_max_signals;
	settings.rail_look_ahead_signal_p0 = &zero_signed;
	settings.rail_look_ahead_signal_p1 = &zero_signed;
	settings.rail_look_ahead_signal_p2 = &zero_signed;
	settings.rail_pbs_cross_penalty = &zero;
	settings.rail_pbs_signal_back_penalty = &zero;
	settings.rail_pbs_station_penalty = &zero;
	settings.rail_shorter_platform_penalty = &zero;
	settings.rail_shorter_platform_per_tile_penalty = &zero;
	settings.rail_slope_penalty = &slope;
	settings.rail_station_penalty = &zero;
	return settings;
}
inline OpenTTDTestFollow Follow(uint32_t tile, uint8_t td)
{
	return OpenTTDTestFollowOnce(FollowerFixture::active->shared, TRANSPORT_RAIL, nullptr, 0, 1, false, tile, td);
}
inline uint8_t Write(uint32_t tile, uint8_t td, uint8_t op) noexcept
{
	tile = start - tile;
	writes.push_back((tile << 12) | (td << 4) | op);
	return !(rollback && tile == 1 && op == 1);
}
/** HasPbsSignalOnTrackdir over the fixture's tile fields. */
inline bool HasPbsSignal(uint32_t tile, uint8_t td) noexcept
{
	const auto &t = FollowerFixture::active->tiles.base[tile];
	if (t.type >> 4 != MP_RAILWAY || t.m5 >> 6 != RAIL_TILE_SIGNALS) return false;
	if (((t.m3 >> 4) & SignalAlongTrackdir(static_cast<Trackdir>(td))) == 0) return false;
	const Track track = TrackdirToTrack(static_cast<Trackdir>(td));
	return IsPbsSignal(static_cast<SignalType>(GB(t.m2, track == TRACK_LOWER || track == TRACK_RIGHT ? 4 : 0, 3)));
}
inline OpenTTDTestRailYapf Run(uint32_t limit, uint32_t curve = 29, bool cache = false, bool reserve = false, uint8_t kind = 0)
{
	follows.clear(); writes.clear(); stats = {};
	follow_reads.clear();
	static FollowerFixture map{128, 128};
	static OpenTTDGroundState ground{};
	static OpenTTDSettingAddresses settings = Settings();
	FollowerFixture::active = &map;
	auto &shared = map.shared;
	shared.settings = &settings;
	max_search_nodes = limit; curve45 = curve; slope = 50; look_ahead_max_signals = !cache;
	for (uint32_t y = 1; y < 127; ++y) for (uint32_t x = 1; x < 127; ++x) map.Tile(x + y * 128, MP_RAILWAY, TRACK_BIT_ALL);
	if (follow_fail) map.tiles.base[start - 1].m5 = 0;
	/* Raising the north and east corners of start - 2 gives it SLOPE_NE, uphill for TRACKDIR_X_NE. */
	if (hill) map.tiles.base[start - 2].height = map.tiles.base[start - 2 + 128].height = 1;
	/* Green path signals face the reversed trackdir TRACKDIR_X_SW on the path's tiles. */
	if (rollback) {
		for (uint32_t tile : {start - 1, start - 2, start - 3}) {
			auto &t = map.tiles.base[tile];
			t.m5 = (RAIL_TILE_SIGNALS << 6) | TRACK_BIT_ALL;
			t.m3 = SignalAlongTrackdir(TRACKDIR_X_SW) << 4;
			t.m4 = SignalAlongTrackdir(TRACKDIR_X_SW) << 4;
			t.m2 = SIGTYPE_PBS;
		}
	}
	// Reservation and speed callbacks mark their original observation points.
	map.record.owner = 0;
	map.record.tile = start;
	map.record.ground = &ground;
	map.record.x_pos = 32 * 16; map.record.y_pos = 32 * 16;
	map.record.dest_tile = start - goal; map.record.cached_max_speed = 100;
	map.order_state.current.max_speed = 100;
	map.state->track = TRACK_BIT_X;
	map.record.vehstatus = 1;
	map.record.direction = 0;
	map.rail.rail_reservation = [](uint32_t) noexcept { follow_reads.push_back('r'); return TRACK_BIT_NONE; };
	map.physics.rail_speed = [](RailType) noexcept -> uint16_t { follow_reads.push_back('s'); return 0; };
	map.train.first_track = [](OpenTTDTrainHandle, uint8_t) noexcept -> uint64_t { return 0; };
	map.train.track_direction = [](OpenTTDTrainHandle, uint8_t, uint8_t) noexcept -> uint64_t { return 0; };
	map.pbs.try_track = [](uint32_t tile, uint8_t track) noexcept { return Write(tile, track, 1) != 0; };
	map.pbs.unreserve_track = [](uint32_t tile, uint8_t track) noexcept { Write(tile, track, 2); };
	map.pbs.blocking = [](uint32_t, uint8_t) noexcept { return false; };
	map.pbs.has_pbs = HasPbsSignal;
	map.rail.set_signal_state = [](uint32_t tile, uint8_t td, uint8_t value) noexcept { Write(tile, td, value ? 6 : 5); };
	shared.train_trackdir = [](const ::Train *) noexcept { return TRACKDIR_X_NE; };
	stats = openttd_test_rail_yapf(&shared, map.Vehicle(), kind == 3, start, TRACKDIR_X_NE, reserve);
	return stats;
}
}
#endif
