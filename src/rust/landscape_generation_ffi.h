/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file landscape_generation_ffi.h Typed synchronous generation owner boundary. */
#ifndef RUST_LANDSCAPE_GENERATION_FFI_H
#define RUST_LANDSCAPE_GENERATION_FFI_H
#include "ground_ffi.h"
#include "landscape_loop_ffi.h"
#include "landscape_command_ffi.h"
#include "tgp_ffi.h"
#include "heightmap_ffi.h"
#include "../genworld.h"
struct OpenTTDGenerationFrame;
struct OpenTTDGenerationLoop { OpenTTDTreeLoopSettings settings; const OpenTTDLandscapeLoopServices *services; };
struct OpenTTDTerrainTemplate { uint32_t width, height; const uint8_t *data; };
struct OpenTTDLandscapeGenerationSettings {
	int32_t max_search_nodes;
	uint8_t climate, generator, rivers, min_river_length, route_random, terrain, sea, snow_coverage, desert_coverage, tree_placer, height_limit, rotation, heightmap_height;
	bool freeform;
};
/* Native map storage remains canonical. The immutable tables outlive each call;
 * leaves do not yield or retain pointers. No raw-map descriptor is authorized. */
struct OpenTTDLandscapeGenerationServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDClearServices *clear;
	const OpenTTDWaterGroundServices *water;
	const OpenTTDTreeServices *trees;
	const OpenTTDLandscapeCommandServices *landscape_commands;
	const OpenTTDWaterCommandServices *water_commands;
	bool (*set_progress)(OpenTTDGenerationFrame *, GenWorldProgress, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_progress);
	bool (*increase_progress)(OpenTTDGenerationFrame *, GenWorldProgress) noexcept OPENTTD_SERVICE_DEFAULT(increase_progress);
	const OpenTTDGenerationLoop *(*begin_loop)(OpenTTDGenerationFrame *) noexcept OPENTTD_SERVICE_DEFAULT(begin_loop);
	void (*end_loop)(OpenTTDGenerationFrame *) noexcept OPENTTD_SERVICE_DEFAULT(end_loop);
	OpenTTDTGPSettings (*tgp_settings)() noexcept OPENTTD_SERVICE_DEFAULT(tgp_settings);
	OpenTTDTerrainTemplate (*terrain_template)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(terrain_template);
	OpenTTDFlatTileHeight (*flat_height)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(flat_height);
	bool (*river)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(river);
	void (*set_snow_line)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_snow_line);
	void (*river_debug)(bool, uint32_t, uint32_t, uint32_t, int32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(river_debug);
	void (*sea_construct)(OpenTTDGenerationFrame *) noexcept OPENTTD_SERVICE_DEFAULT(sea_construct);
	void (*sea_destroy)(OpenTTDGenerationFrame *) noexcept OPENTTD_SERVICE_DEFAULT(sea_destroy);
	void (*sea_insert)(OpenTTDGenerationFrame *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(sea_insert);
	bool (*sea_contains)(OpenTTDGenerationFrame *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(sea_contains);
	size_t (*sea_size)(OpenTTDGenerationFrame *) noexcept OPENTTD_SERVICE_DEFAULT(sea_size);
	uint32_t (*sea_first)(OpenTTDGenerationFrame *) noexcept OPENTTD_SERVICE_DEFAULT(sea_first);
	uint32_t (*sea_next)(OpenTTDGenerationFrame *) noexcept OPENTTD_SERVICE_DEFAULT(sea_next);
	void (*set_height)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_height);
	bool (*inner_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(inner_tile);
	void (*make_void)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(make_void);
	bool (*surrounded_by_water)() noexcept OPENTTD_SERVICE_DEFAULT(surrounded_by_water);
	void (*set_freeform)(bool) noexcept OPENTTD_SERVICE_DEFAULT(set_freeform);
	void (*clear_bridge_middle)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(clear_bridge_middle);
};
const OpenTTDLandscapeGenerationServices &GetRustLandscapeGenerationServices() noexcept;
extern "C" {
void openttd_rust_initialize_landscape(const OpenTTDLandscapeGenerationServices *, bool);
void openttd_rust_convert_ground_water(const OpenTTDLandscapeGenerationServices *);
}
#endif /* RUST_LANDSCAPE_GENERATION_FFI_H */
