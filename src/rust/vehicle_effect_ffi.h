/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file vehicle_effect_ffi.h Vehicle effect emission observations and synchronous services. */
#ifndef RUST_VEHICLE_EFFECT_FFI_H
#define RUST_VEHICLE_EFFECT_FFI_H
#include "vehicle_ffi.h"
#include "services_ffi.h"
enum RailVehicleTypes : uint8_t;
struct OpenTTDVisualServices {
	const OpenTTDVehicleServices *vehicle;
	const OpenTTDSharedServices *shared;
	uint8_t (*smoke)() noexcept OPENTTD_SERVICE_DEFAULT(smoke);
	void (*power_error)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(power_error);
	bool (*depot)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot);
	bool (*tunnel)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tunnel);
	bool (*powered)(uint64_t, RailType) noexcept OPENTTD_SERVICE_DEFAULT(powered);
};
const OpenTTDVisualServices &GetRustVisualServices();
extern "C" {
const OpenTTDVisualServices *openttd_rust_visual_bind(const OpenTTDVisualServices *);
void openttd_rust_visual_update(OpenTTDVehicle *, bool);
uint8_t openttd_rust_visual_eligible_abi(OpenTTDVehicle *);
extern "C++" inline bool openttd_rust_visual_eligible(OpenTTDVehicle *a0) { return openttd_rust_visual_eligible_abi(a0) != 0; }
void openttd_rust_visual_show(OpenTTDVehicle *, uint32_t);
}
#endif
