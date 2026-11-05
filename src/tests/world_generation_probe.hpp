/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file world_generation_probe.hpp Native-test bridge to production Rust world-generation operations. */
#ifndef WORLD_GENERATION_PROBE_HPP
#define WORLD_GENERATION_PROBE_HPP
#include "../genworld.h"
#include "../rust/landscape_generation_ffi.h"
struct OpenTTDTownDataServices;
extern "C" {
void openttd_test_world_set_abort_callback(GWAbortProc *);
uint32_t openttd_test_town_data_target(const OpenTTDTownDataServices *, uint8_t, float, float);
GWAbortProc *openttd_test_world_abort_callback();
uint8_t openttd_test_snow_line_highest(const OpenTTDSharedServices *);
uint8_t openttd_test_map_surrounded_by_water_abi(const OpenTTDSharedServices *);
extern "C++" inline bool openttd_test_map_surrounded_by_water(const OpenTTDSharedServices *a0) { return openttd_test_map_surrounded_by_water_abi(a0) != 0; }
uint8_t openttd_test_generate_landscape_abi(const OpenTTDLandscapeGenerationServices *, OpenTTDGenerationFrame *, const OpenTTDHeightmapInput *, OpenTTDLandscapeGenerationSettings, uint8_t);
extern "C++" inline bool openttd_test_generate_landscape(const OpenTTDLandscapeGenerationServices *a0, OpenTTDGenerationFrame *a1, const OpenTTDHeightmapInput *a2, OpenTTDLandscapeGenerationSettings a3, uint8_t a4) { return openttd_test_generate_landscape_abi(a0, a1, a2, a3, a4) != 0; }
uint8_t openttd_test_load_heightmap_abi(const OpenTTDLandscapeGenerationServices *, const OpenTTDHeightmapInput *, uint8_t, bool, uint8_t, uint8_t);
extern "C++" inline bool openttd_test_load_heightmap(const OpenTTDLandscapeGenerationServices *a0, const OpenTTDHeightmapInput *a1, uint8_t a2, bool a3, uint8_t a4, uint8_t a5) { return openttd_test_load_heightmap_abi(a0, a1, a2, a3, a4, a5) != 0; }
void openttd_test_flat_empty_world(const OpenTTDLandscapeGenerationServices *, uint8_t, bool, uint8_t);
}
#endif /* WORLD_GENERATION_PROBE_HPP */
