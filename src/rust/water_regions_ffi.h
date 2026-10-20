/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file water_regions_ffi.h Rust water-region cache owner and visitor boundary. */
#ifndef OPENTTD_RUST_WATER_REGIONS_FFI_H
#define OPENTTD_RUST_WATER_REGIONS_FFI_H
#include "service_defaults.hpp"
#include <stdint.h>
#if defined(_MSC_VER)
#define OPENTTD_WATER_CALL __cdecl
#elif defined(__i386__)
#define OPENTTD_WATER_CALL __attribute__((cdecl))
#else
#define OPENTTD_WATER_CALL
#endif
#ifdef __cplusplus
extern "C" {
#endif
typedef struct OpenTTDWaterRegions OpenTTDWaterRegions;
typedef struct OpenTTDWaterVisit OpenTTDWaterVisit;
typedef struct { int32_t x, y; uint8_t label; } OpenTTDWaterPatch;
typedef struct { uint16_t edges[4]; uint8_t labels[256], patches, aqueducts; } OpenTTDWaterSnapshot;
typedef struct OpenTTDWaterLeaves {
	uint16_t (OPENTTD_WATER_CALL *tracks)(uint32_t) OPENTTD_SERVICE_DEFAULT(tracks);
	uint32_t (OPENTTD_WATER_CALL *follow)(uint32_t, uint8_t, uint8_t *) OPENTTD_SERVICE_DEFAULT(follow);
	uint32_t (OPENTTD_WATER_CALL *aqueduct)(uint32_t) OPENTTD_SERVICE_DEFAULT(aqueduct);
	void (OPENTTD_WATER_CALL *debug)(uint8_t, int32_t, int32_t) OPENTTD_SERVICE_DEFAULT(debug);
} OpenTTDWaterLeaves;
typedef struct { OpenTTDWaterRegions *owner; const OpenTTDWaterLeaves *leaves; } OpenTTDWaterRegionContext;
/* One map-lifetime owner, replaced at Map::Allocate; no map/STL pointer enters
 * Rust. Leaves are synchronous noexcept/nonreentrant shared map/follower queries
 * and debug output. follow returns INVALID_TILE on failure; aqueduct returns its
 * other end or INVALID_TILE. Initialized outputs are exclusive for each call.
 * Visitor cursor owns only traversal progress and copied per-side labels. next
 * returns before arbitrary C++ visitor work; no cache borrow survives it. The
 * next call receives the live caller descriptor and observes live cache changes
 * at the original observation points, retaining the original region origin.
 * Map replacement during a visitor violates the original region-reference
 * lifetime. Destroy each owner/cursor once, including visitor exception paths.
 * Panics/OOM abort; no exception or panic crosses the ABI. */
OpenTTDWaterRegions *OPENTTD_WATER_CALL openttd_rust_water_new(uint32_t, uint32_t);
void OPENTTD_WATER_CALL openttd_rust_water_destroy(OpenTTDWaterRegions *);
uint8_t OPENTTD_WATER_CALL openttd_rust_water_label(OpenTTDWaterRegions *, const OpenTTDWaterLeaves *, uint32_t);
void OPENTTD_WATER_CALL openttd_rust_water_invalidate(OpenTTDWaterRegions *, const OpenTTDWaterLeaves *, uint32_t);
void OPENTTD_WATER_CALL openttd_rust_water_snapshot(OpenTTDWaterRegions *, const OpenTTDWaterLeaves *, uint32_t, OpenTTDWaterSnapshot *);
OpenTTDWaterVisit *OPENTTD_WATER_CALL openttd_rust_water_visit_new(OpenTTDWaterRegions *, const OpenTTDWaterLeaves *, OpenTTDWaterPatch);
void OPENTTD_WATER_CALL openttd_rust_water_visit_destroy(OpenTTDWaterVisit *);
uint8_t OPENTTD_WATER_CALL openttd_rust_water_visit_next(OpenTTDWaterRegions *, const OpenTTDWaterLeaves *, OpenTTDWaterVisit *, OpenTTDWaterPatch, OpenTTDWaterPatch *);
#ifdef __cplusplus
}
#endif
OpenTTDWaterRegionContext GetRustWaterRegionContext() noexcept;
#endif /* OPENTTD_RUST_WATER_REGIONS_FFI_H */
