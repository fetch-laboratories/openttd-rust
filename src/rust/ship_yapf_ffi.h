/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file ship_yapf_ffi.h Rust ship pathfinding and the ship's path cache owner. */
#ifndef OPENTTD_RUST_SHIP_YAPF_FFI_H
#define OPENTTD_RUST_SHIP_YAPF_FFI_H
#include "services_ffi.h"
#include "water_regions_ffi.h"
#include <stddef.h>
#include <stdint.h>
extern "C" {
struct OpenTTDShipPath;
struct OpenTTDShipLeaves;
struct OpenTTDVehicle;
/** What a search reads from the ship, its engine and the settings; native tests fill it themselves. */
struct OpenTTDShipYapfInput {
	uint32_t tile, dest_tile;
	uint8_t order_type;
	uint16_t order_destination;
	uint8_t direction;
	uint16_t max_speed;
	uint8_t ocean_speed_frac, canal_speed_frac;
	uint32_t ship_curve45_penalty, ship_curve90_penalty;
	uint16_t unit_number;
};
/* The path owner belongs to its Ship facade; save errors occur outside Rust frames. */
OpenTTDShipPath *openttd_rust_ship_path_new();
OpenTTDShipPath *openttd_rust_ship_path_clone(const OpenTTDShipPath *);
void openttd_rust_ship_path_destroy(OpenTTDShipPath *);
size_t openttd_rust_ship_path_size(const OpenTTDShipPath *);
uint8_t openttd_rust_ship_path_get(const OpenTTDShipPath *, size_t);
void openttd_rust_ship_path_set(OpenTTDShipPath *, size_t, uint8_t);
void openttd_rust_ship_path_push(OpenTTDShipPath *, uint8_t);
void openttd_rust_ship_path_pop(OpenTTDShipPath *);
void openttd_rust_ship_path_clear(OpenTTDShipPath *);
/* YapfShipChooseTrack and YapfShipCheckReverse, with the ship's GetState() and GetVehicleTrackdir().
 * The search reads the map in place and calls only shared services and the controller's leaves,
 * which cannot reenter it, the ship's path owner or the controller. */
uint8_t openttd_rust_yapf_ship_choose_track(const OpenTTDSharedServices *, const OpenTTDShipLeaves *, const OpenTTDVehicle *, uint8_t, uint8_t, uint32_t, OpenTTDShipPath *, bool *);
uint8_t openttd_rust_yapf_ship_check_reverse(const OpenTTDSharedServices *, const OpenTTDShipLeaves *, const OpenTTDVehicle *, uint8_t, uint8_t, uint8_t *);
/* Native test probes: the production searches over a test's own input, and the ship node list's heap. */
struct OpenTTDShipYapfProbe {
	OpenTTDShipYapfInput input;
	uint8_t state, trackdir, path_found, best_origin_dir;
};
uint8_t openttd_test_ship_choose_ship_track(const OpenTTDSharedServices *, const OpenTTDShipLeaves *, OpenTTDShipYapfProbe *, uint32_t, uint16_t, uint16_t, OpenTTDShipPath *);
uint8_t openttd_test_ship_check_ship_reverse(const OpenTTDSharedServices *, const OpenTTDShipLeaves *, const OpenTTDShipYapfProbe *, uint8_t *);
size_t openttd_test_ship_find_water_region_path(const OpenTTDSharedServices *, const OpenTTDShipLeaves *, const OpenTTDShipYapfProbe *, uint32_t, int32_t, OpenTTDWaterPatch *, size_t);
void openttd_test_ship_heap_probe(const int32_t *, size_t, const uint32_t *, const int32_t *, size_t, uint32_t *);
}
#endif
