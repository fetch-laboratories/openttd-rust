/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file ship_yapf_ffi.h Complete ship searches and canonical path owner. */
#ifndef OPENTTD_RUST_SHIP_YAPF_FFI_H
#define OPENTTD_RUST_SHIP_YAPF_FFI_H
#include "service_defaults.hpp"
#include "water_regions_ffi.h"
#include "services_ffi.h"
#include <stddef.h>
extern "C" {
struct OpenTTDShipPath;
struct OpenTTDShipRegionPath;
struct OpenTTDShipYapfInput {
	uint32_t map_x, map_y, tile, dest_tile;
	int32_t curve90, curve45;
	uint32_t max_speed;
	uint16_t dest_dirs, reverse_dirs;
	uint8_t trackdir, ocean_frac, canal_frac, station;
	uint32_t unit_number;
};
struct OpenTTDShipFollow { uint32_t tile; int32_t skipped; uint16_t dirs; uint8_t followed; };
struct OpenTTDShipTile { uint32_t ships; uint8_t docking, sea, lock_middle, destination; };
struct OpenTTDShipYapfResult { uint8_t direction, found, origin; uint32_t stats[13]; };
struct OpenTTDShipYapfDestination { uint32_t tile; bool station; };
struct OpenTTDShipYapfLeaves {
	OpenTTDShipYapfDestination (*destination)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(destination);
	OpenTTDShipFollow (*follow)(const void *, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(follow);
	OpenTTDShipTile (*tile)(const void *, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(tile);
	OpenTTDWaterPatch (*patch)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(patch);
	void *(*visit_new)(OpenTTDWaterPatch) noexcept OPENTTD_SERVICE_DEFAULT(visit_new);
	uint8_t (*visit_next)(void *, OpenTTDWaterPatch, OpenTTDWaterPatch *) noexcept OPENTTD_SERVICE_DEFAULT(visit_next);
	void (*visit_destroy)(void *) noexcept OPENTTD_SERVICE_DEFAULT(visit_destroy);
	void (*debug)(uint32_t, uint8_t, uint8_t, uint32_t, uint32_t, uint32_t, int32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(debug);
};
/* Search owns its arena, hashes, heap and copied world observations. The ship
	* context is opaque and live for the synchronous call. Leaves never reenter this
	* search or path owner; region calls may enter the distinct water-cache owner.
	* No world/STL borrow survives a leaf. All leaves are noexcept; environment
	* failures terminate. Panics/OOM abort. Path owner belongs to its Ship facade;
	* no reference to its Vec escapes, and save errors occur outside Rust frames. */
OpenTTDShipPath *openttd_rust_ship_path_new();
OpenTTDShipPath *openttd_rust_ship_path_clone(const OpenTTDShipPath *);
void openttd_rust_ship_path_destroy(OpenTTDShipPath *);
size_t openttd_rust_ship_path_size(const OpenTTDShipPath *);
uint8_t openttd_rust_ship_path_get(const OpenTTDShipPath *, size_t);
void openttd_rust_ship_path_set(OpenTTDShipPath *, size_t, uint8_t);
void openttd_rust_ship_path_push(OpenTTDShipPath *, uint8_t);
void openttd_rust_ship_path_pop(OpenTTDShipPath *);
void openttd_rust_ship_path_clear(OpenTTDShipPath *);
OpenTTDShipYapfResult openttd_rust_ship_choose(OpenTTDShipPath *, const OpenTTDShipYapfInput *, const OpenTTDShipYapfLeaves *, const OpenTTDSharedServices *, const void *, uint32_t, uint16_t, uint16_t, const uint32_t *, size_t);
OpenTTDShipYapfResult openttd_rust_ship_reverse(const OpenTTDShipYapfInput *, const OpenTTDShipYapfLeaves *, const OpenTTDSharedServices *, const void *, uint8_t, const uint32_t *, size_t);
OpenTTDShipRegionPath *openttd_rust_ship_regions(const OpenTTDShipYapfInput *, const OpenTTDShipYapfLeaves *, uint32_t, int32_t, const uint32_t *, size_t);
size_t openttd_rust_ship_regions_size(const OpenTTDShipRegionPath *);
OpenTTDWaterPatch openttd_rust_ship_regions_get(const OpenTTDShipRegionPath *, size_t);
void openttd_rust_ship_regions_destroy(OpenTTDShipRegionPath *);
/* Evidence-only heap operations: costs[nodes], commands[count], values[count]
 * readable initialized spans, out[count] exclusive writable; extents <=isize::MAX.
 * Inputs remain live/read-only and do not overlap output. Node indices and heap
 * membership obey Include/Remove preconditions; no pointer survives the call. */
void openttd_rust_ship_heap_probe(const int32_t *, size_t, const uint32_t *, const int32_t *, size_t, uint32_t *);
}
#endif
