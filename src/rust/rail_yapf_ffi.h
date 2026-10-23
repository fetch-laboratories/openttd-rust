/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file rail_yapf_ffi.h Complete rail search/cache/reservation owner. */
#ifndef OPENTTD_RUST_RAIL_YAPF_FFI_H
#define OPENTTD_RUST_RAIL_YAPF_FFI_H
#include "service_defaults.hpp"
#include <stdint.h>
#include "services_ffi.h"
extern "C" {
struct OpenTTDRailYapf;
struct OpenTTDRailSettings {
	uint32_t max_nodes, firstred, firstred_exit, lastred, lastred_exit;
	uint32_t station, slope, curve45, curve90, depot_reverse, crossing, lookahead;
	int32_t p0, p1, p2;
	uint32_t pbs_cross, pbs_station, pbs_back, doubleslip;
	uint32_t longer, longer_tile, shorter, shorter_tile;
	uint8_t firstred_eol;
};
struct OpenTTDRailTile {
	uint32_t flags, other_end;
	uint16_t station;
	uint8_t railtype, tracks, station_track, tunnel_dir;
	uint8_t uphill, signal_along, signal_against, signal_green, signal_type, oneway;
};
struct OpenTTDRailFollow {
	uint32_t tile;
	int32_t skipped;
	uint16_t dirs;
	uint8_t followed, error, station, exitdir;
};
struct OpenTTDRailSpeed { int32_t min_speed, max_speed; };
struct OpenTTDRailTrain {
	uint64_t compatible, all_compatible;
	uint32_t tile, rear_tile, virtual_tile, rear_virtual_tile, dest_tile, length, speed;
	uint16_t order_destination;
	uint8_t td, rear_td, wormhole, rear_wormhole, order, nearest_depot, complex_waypoint;
};
struct OpenTTDRailInput {
	void *context;
	const Train *vehicle;
	const OpenTTDSharedServices *shared;
	OpenTTDRailSettings settings;
	uint32_t map_x, tile;
	int32_t max_cost, desync;
	uint8_t kind, td, override_railtype, forbid90, reserve;
};
struct OpenTTDRailStep {
	uint32_t tile, destination, target_tile, best_length;
	uint8_t action, td, found, reverse, value, target_td, target_okay;
};
struct OpenTTDRailLeaves {
	OpenTTDRailTrain (*train)(void *) noexcept OPENTTD_SERVICE_DEFAULT(train);
	OpenTTDRailTile (*tile)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(tile);
	OpenTTDRailFollow (*follow)(void *, uint32_t, uint8_t, uint64_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(follow);
	OpenTTDRailSpeed (*speed)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(speed);
	uint8_t (*compatible_station)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(compatible_station);
	uint32_t (*platform_length)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(platform_length);
	uint32_t (*closest_station)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(closest_station);
	void (*output)(void *, uint8_t, const OpenTTDRailStep *) noexcept OPENTTD_SERVICE_DEFAULT(output);
	void (*debug)(void *, uint8_t, const uint32_t *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(debug);
};
/* One serialized game-thread owner holds the change counter and six cache banks.
 * Searches keep private arenas/queues and call the PBS Rust owner directly.
 * Native synchronous leaves are noexcept; no native object/container layout is
 * viewed by Rust. PBS station triggers run in their original synchronous order.
 * step returns action=1 before the existing explicit platform/waypoint triggers,
 * releasing all owner borrows; a waypoint retains its original second trigger.
 * The next step resumes that traversal. Destroy once, including on unwinding an
 * ordinary station callback. Map replacement violates the source lifetime.
 * output writes caller targets at their original observation points. No owner
 * state is serialized; invalidation advances the counter and banks flush lazily.
 * Panics/OOM abort; environmental exceptions terminate in noexcept leaves. */
OpenTTDRailYapf *openttd_rust_rail_new(const OpenTTDRailInput *, const OpenTTDRailLeaves *);
OpenTTDRailStep openttd_rust_rail_step(OpenTTDRailYapf *);
void openttd_rust_rail_destroy(OpenTTDRailYapf *);
void openttd_rust_rail_invalidate();
}
#endif
