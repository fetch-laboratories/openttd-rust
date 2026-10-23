/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file landscape_generation_ffi.h Typed synchronous generation owner boundary. */
#ifndef RUST_LANDSCAPE_GENERATION_FFI_H
#define RUST_LANDSCAPE_GENERATION_FFI_H
#include "ground_ffi.h"
struct OpenTTDHeightmapImage { uint32_t width, height; const uint8_t *data; };
struct OpenTTDHeightmapDecoder {
	void *context;
	bool (*read)(void *, OpenTTDHeightmapImage *) noexcept OPENTTD_SERVICE_DEFAULT(read);
};
/* Native map storage remains canonical. The immutable tables outlive each call;
 * leaves do not yield or retain pointers. No raw-map descriptor is authorized. */
struct OpenTTDLandscapeGenerationServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDClearServices *clear;
	const OpenTTDWaterGroundServices *water;
	void (*set_height)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_height);
	bool (*inner_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(inner_tile);
	void (*make_void)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(make_void);
	void (*screen_dirty)() noexcept OPENTTD_SERVICE_DEFAULT(screen_dirty);
	bool (*surrounded_by_water)() noexcept OPENTTD_SERVICE_DEFAULT(surrounded_by_water);
	void (*set_freeform)(bool) noexcept OPENTTD_SERVICE_DEFAULT(set_freeform);
	void (*clear_bridge_middle)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(clear_bridge_middle);
};
const OpenTTDLandscapeGenerationServices &GetRustLandscapeGenerationServices() noexcept;
extern "C" {
void openttd_rust_initialize_landscape(const OpenTTDLandscapeGenerationServices *, bool);
void openttd_rust_convert_ground_water(const OpenTTDLandscapeGenerationServices *);
void openttd_rust_flat_empty_world(const OpenTTDLandscapeGenerationServices *, uint8_t, bool, uint8_t);
bool openttd_rust_load_heightmap(const OpenTTDLandscapeGenerationServices *, const OpenTTDHeightmapDecoder *, uint8_t, bool, uint8_t, uint8_t);
void openttd_rust_fix_slopes(const OpenTTDLandscapeGenerationServices *, uint8_t);
}
#endif /* RUST_LANDSCAPE_GENERATION_FFI_H */
