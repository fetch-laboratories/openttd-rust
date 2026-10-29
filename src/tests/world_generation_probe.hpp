/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file world_generation_probe.hpp Native-test bridge to production Rust world-generation operations. */
#ifndef WORLD_GENERATION_PROBE_HPP
#define WORLD_GENERATION_PROBE_HPP
#include "../genworld.h"
#include "../rust/landscape_generation_ffi.h"
extern "C" {
void openttd_test_world_set_abort_callback(GWAbortProc *);
GWAbortProc *openttd_test_world_abort_callback();
bool openttd_test_generate_landscape(const OpenTTDLandscapeGenerationServices *, OpenTTDGenerationFrame *, const OpenTTDHeightmapDecoder *, OpenTTDLandscapeGenerationSettings, uint8_t);
void openttd_test_flat_empty_world(const OpenTTDLandscapeGenerationServices *, uint8_t, bool, uint8_t);
}
#endif /* WORLD_GENERATION_PROBE_HPP */
