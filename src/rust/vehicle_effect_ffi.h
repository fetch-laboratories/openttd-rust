/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file vehicle_effect_ffi.h Vehicle effect emission observations and synchronous services. */
#ifndef RUST_VEHICLE_EFFECT_FFI_H
#define RUST_VEHICLE_EFFECT_FFI_H
#include "vehicle_ffi.h"
#include "services_ffi.h"
struct OpenTTDVisualGate { uint16_t speed; uint8_t status, smoke; bool primary; };
struct OpenTTDVisualShow { uint16_t speed; uint8_t kind, smoke; };
struct OpenTTDVisualBridgePosition { int32_t x, y; };
struct OpenTTDVisualEngine {
	const OpenTTDEngine *engine;
	uint8_t *cache;
	uint16_t id;
	uint8_t kind, visual;
	bool callback;
};
struct OpenTTDVisualDefaults { uint8_t rail_kind, engclass; };
struct OpenTTDVisualPart { uint8_t *cache; uint32_t tile; uint8_t kind, status, tick; };
struct OpenTTDVisualOffset { uint8_t direction, length; };
struct OpenTTDVisualTrain { OpenTTDTrainHandle handle; uint32_t tile; uint16_t speed; bool front; };
struct OpenTTDVisualPowerWeight { uint32_t power, weight; };
struct OpenTTDVisualServices {
	const OpenTTDVehicleServices *vehicle;
	const OpenTTDSharedServices *shared;
	OpenTTDVisualGate (*gate)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(gate);
	OpenTTDVisualShow (*show)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(show);
	OpenTTDVisualEngine (*engine)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(engine);
	OpenTTDVisualDefaults (*defaults)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(defaults);
	uint8_t (*grf_version)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(grf_version);
	void (*power_error)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(power_error);
	OpenTTDVisualPart (*part)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(part);
	OpenTTDVisualBridgePosition (*bridge_position)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(bridge_position);
	OpenTTDVisualOffset (*offset)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(offset);
	OpenTTDVisualTrain (*train)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(train);
	bool (*station)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station);
	bool (*depot)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot);
	bool (*tunnel)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tunnel);
	RailType (*tile_rail)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_rail);
	bool (*powered)(uint64_t, RailType) noexcept OPENTTD_SERVICE_DEFAULT(powered);
	OpenTTDVisualPowerWeight (*power_weight)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(power_weight);
};
const OpenTTDVisualServices &GetRustVisualServices();
extern "C" {
const OpenTTDVisualServices *openttd_rust_visual_bind(const OpenTTDVisualServices *);
void openttd_rust_visual_update(OpenTTDVehicle *, bool);
bool openttd_rust_visual_eligible(OpenTTDVehicle *);
void openttd_rust_visual_show(OpenTTDVehicle *, uint32_t);
}
#endif
