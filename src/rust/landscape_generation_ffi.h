/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file landscape_generation_ffi.h Typed synchronous generation owner boundary. */
#ifndef RUST_LANDSCAPE_GENERATION_FFI_H
#define RUST_LANDSCAPE_GENERATION_FFI_H
#include "ground_ffi.h"
/* Native map storage remains canonical. The immutable tables outlive each call;
 * leaves do not yield or retain pointers. No raw-map descriptor is authorized. */
struct OpenTTDLandscapeGenerationServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDClearServices *clear;
	const OpenTTDWaterGroundServices *water;
	void (*set_height)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_height);
	bool (*inner_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(inner_tile);
	void (*make_void)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(make_void);
	void (*clear_bridge_middle)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(clear_bridge_middle);
};
const OpenTTDLandscapeGenerationServices &GetRustLandscapeGenerationServices() noexcept;
extern "C" {
void openttd_rust_initialize_landscape(const OpenTTDLandscapeGenerationServices *, bool);
void openttd_rust_fix_slopes(const OpenTTDLandscapeGenerationServices *, uint8_t);
}
#endif /* RUST_LANDSCAPE_GENERATION_FFI_H */
