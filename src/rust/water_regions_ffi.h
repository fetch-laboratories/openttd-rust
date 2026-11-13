/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file water_regions_ffi.h Rust water-region cache owner and visitor boundary. */
#ifndef OPENTTD_RUST_WATER_REGIONS_FFI_H
#define OPENTTD_RUST_WATER_REGIONS_FFI_H
#include <stdint.h>
#include "call_ffi.h"
#ifdef __cplusplus
extern "C" {
#endif
typedef struct OpenTTDWaterRegions OpenTTDWaterRegions;
typedef struct { int32_t x, y; uint8_t label; } OpenTTDWaterPatch;
/* One VisitWaterRegionPatchNeighbours in progress; a zeroed record starts a visit. */
typedef struct OpenTTDWaterVisit {
	OpenTTDWaterPatch origin;
	int32_t neighbour_x, neighbour_y;
	uint32_t aqueduct_tile;
	uint8_t labels[16];
	uint8_t started, side, count, read, aqueducts_read, has_cross_region_aqueducts;
} OpenTTDWaterVisit;
/* One map-lifetime owner, replaced at Map::Allocate and destroyed once. Each call
 * holds the owner only until it returns: a visitor between two next steps may enter
 * it again, and each step observes the caller's live descriptor and the live cache
 * at the original points. Replacing the owner during a visit violates the original
 * region-reference lifetime. */
OpenTTDWaterRegions *OPENTTD_RUST_CALL openttd_rust_water_new(const struct OpenTTDSharedServices *, uint32_t, uint32_t);
void OPENTTD_RUST_CALL openttd_rust_water_destroy(OpenTTDWaterRegions *);
uint8_t OPENTTD_RUST_CALL openttd_rust_water_label(OpenTTDWaterRegions *, const struct OpenTTDSharedServices *, uint32_t);
void OPENTTD_RUST_CALL openttd_rust_water_invalidate(OpenTTDWaterRegions *, const struct OpenTTDSharedServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_water_visit_next(OpenTTDWaterRegions *, const struct OpenTTDSharedServices *, OpenTTDWaterVisit *, OpenTTDWaterPatch, OpenTTDWaterPatch *);
void OPENTTD_RUST_CALL openttd_rust_water_print_debug_info(OpenTTDWaterRegions *, const struct OpenTTDSharedServices *, uint32_t);
#ifdef __cplusplus
}
#endif
#ifdef __cplusplus
OpenTTDWaterRegions *GetRustWaterRegions() noexcept;
#endif
#endif /* OPENTTD_RUST_WATER_REGIONS_FFI_H */
