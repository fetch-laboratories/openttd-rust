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
#include "call_ffi.h"
#ifdef __cplusplus
extern "C" {
#endif
typedef struct OpenTTDWaterRegions OpenTTDWaterRegions;
typedef struct OpenTTDWaterVisit OpenTTDWaterVisit;
typedef struct { int32_t x, y; uint8_t label; } OpenTTDWaterPatch;
typedef struct { uint16_t edges[4]; uint8_t labels[256], patches, aqueducts; } OpenTTDWaterSnapshot;
typedef struct OpenTTDWaterLeaves {
	const struct OpenTTDSharedServices *shared;
	uint32_t (OPENTTD_RUST_CALL *aqueduct)(uint32_t) OPENTTD_SERVICE_DEFAULT(aqueduct);
	void (OPENTTD_RUST_CALL *debug)(uint8_t, int32_t, int32_t) OPENTTD_SERVICE_DEFAULT(debug);
} OpenTTDWaterLeaves;
typedef struct OpenTTDWaterRegionContext { OpenTTDWaterRegions *owner; const OpenTTDWaterLeaves *leaves; } OpenTTDWaterRegionContext;
/* One map-lifetime owner, replaced at Map::Allocate. follow returns INVALID_TILE on failure;
 * aqueduct returns its other end or INVALID_TILE. The visitor cursor owns only traversal progress
 * and copied per-side labels; next returns before C++ visitor work and observes live cache changes
 * at the original points. Map replacement during a visitor violates the original region-reference
 * lifetime. Destroy each owner and cursor once, also on visitor exception paths.
 * Leaves are synchronous, nonthrowing and nonreentrant; initialized outputs are exclusive per call.
 * Each next call receives the live descriptor and retains the original region origin. */
OpenTTDWaterRegions *OPENTTD_RUST_CALL openttd_rust_water_new(uint32_t, uint32_t);
void OPENTTD_RUST_CALL openttd_rust_water_destroy(OpenTTDWaterRegions *);
uint8_t OPENTTD_RUST_CALL openttd_rust_water_label(OpenTTDWaterRegions *, const OpenTTDWaterLeaves *, uint32_t);
void OPENTTD_RUST_CALL openttd_rust_water_invalidate(OpenTTDWaterRegions *, const OpenTTDWaterLeaves *, uint32_t);
void OPENTTD_RUST_CALL openttd_rust_water_snapshot(OpenTTDWaterRegions *, const OpenTTDWaterLeaves *, uint32_t, OpenTTDWaterSnapshot *);
OpenTTDWaterVisit *OPENTTD_RUST_CALL openttd_rust_water_visit_new(OpenTTDWaterRegions *, const OpenTTDWaterLeaves *, OpenTTDWaterPatch);
void OPENTTD_RUST_CALL openttd_rust_water_visit_destroy(OpenTTDWaterVisit *);
uint8_t OPENTTD_RUST_CALL openttd_rust_water_visit_next(OpenTTDWaterRegions *, const OpenTTDWaterLeaves *, OpenTTDWaterVisit *, OpenTTDWaterPatch, OpenTTDWaterPatch *);
#ifdef __cplusplus
}
#endif
#ifdef __cplusplus
OpenTTDWaterRegionContext GetRustWaterRegionContext() noexcept;
#endif
#endif /* OPENTTD_RUST_WATER_REGIONS_FFI_H */
