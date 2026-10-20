/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file landscape_loop_ffi.h Synchronous tile scheduler and canonical cursor. */
#ifndef RUST_LANDSCAPE_LOOP_FFI_H
#define RUST_LANDSCAPE_LOOP_FFI_H
#include "trees_ffi.h"
#include "industry_ffi.h"
#include "ground_ffi.h"

/* Only existing tables consumed by the industry tile loop; borrowed for a batch. */
struct OpenTTDIndustryLoopServices {
	const OpenTTDIndustryServices *production;
	const OpenTTDIndustryConstructionServices *construction;
	const OpenTTDIndustryLifecycleServices *lifecycle;
	const OpenTTDIndustryTileServices *tiles;
	const OpenTTDIndustryGenerationServices *generation;
};
/* Configuration is invariant under the same serialized callback proof as the
 * existing tree settings. The native result context is independent on reentry. */
struct OpenTTDGroundLoopServices {
	const OpenTTDClearServices *clear;
	const OpenTTDRailServices *rail;
	const OpenTTDRoadGroundServices *road;
	const OpenTTDTunnelServices *tunnel;
	const OpenTTDWaterGroundServices *water;
	const OpenTTDGroundCostServices *costs;
	OpenTTDGroundCall *call;
	LandscapeType climate;
	bool editor;
	bool rebuild;
	bool ambient_callback;
};
/* Native handlers retain their complete original bodies. No generic dispatch slot.
 * World callbacks may mutate/reenter; no cursor borrow spans a callback. Tables are
 * immutable and live for the synchronous invocation. All wrappers are noexcept. */
struct OpenTTDLandscapeLoopServices {
	const OpenTTDSharedServices *shared;
	OpenTTDTileTypeAllocation tile_types;
	const OpenTTDTreeServices *trees;
	OpenTTDIndustryLoopServices industry;
	OpenTTDGroundLoopServices ground;
	void (*house_loop)(uint32_t) noexcept;
	void (*station_loop)(uint32_t) noexcept;
	void (*object_loop)(uint32_t) noexcept;
};
const OpenTTDTreeServices &GetRustTreeServices() noexcept;
OpenTTDTreeLoopSettings GetRustTreeLoopSettings() noexcept;
OpenTTDIndustryLoopServices GetRustIndustryLoopServices() noexcept;
extern "C" {
/* Process-global raw u32; unchanged DATE/legacy descriptors access only at serial
 * save/load/reset boundaries. No C++ or Rust reference survives a scheduler call. */
uint32_t *openttd_rust_tile_loop_cursor();
void openttd_rust_run_tile_loop(unsigned int log_x, unsigned int log_y, OpenTTDTreeLoopSettings, const OpenTTDLandscapeLoopServices *);
}
#endif /* RUST_LANDSCAPE_LOOP_FFI_H */
