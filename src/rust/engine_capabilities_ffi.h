/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file engine_capabilities_ffi.h Synchronous engine and articulated query services. */
#ifndef RUST_ENGINE_CAPABILITIES_FFI_H
#define RUST_ENGINE_CAPABILITIES_FFI_H
#include "newgrf_vehicle_ffi.h"
#include "engine_ffi.h"
struct OpenTTDEngineLoadingServices;
struct OpenTTDSharedServices;
struct OpenTTDVehicleServices;
struct OpenTTDEngineCapabilitiesServices {
	const OpenTTDVehicleServices *vehicles;
	const OpenTTDEngine *(*engine)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(engine);
	const OpenTTDEngine *(*engine_if_valid)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(engine_if_valid);
	uint16_t (*id)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(id);
	const OpenTTDEngineLoadingServices *(*loading)() noexcept OPENTTD_SERVICE_DEFAULT(loading);
	uint16_t (*cargo_classes)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_classes);
	uint16_t (*multiplier)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(multiplier);
	OpenTTDVehicle *(*new_temporary)() noexcept OPENTTD_SERVICE_DEFAULT(new_temporary);
	const OpenTTDEngine *(*vehicle_engine_object)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_engine_object);
	uint8_t (*vehicle_cargo)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_cargo);
	uint8_t (*vehicle_subtype)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_subtype);
	const OpenTTDSharedServices *shared;
	bool assertions;
	int32_t (*life_extension)() noexcept OPENTTD_SERVICE_DEFAULT(life_extension);
	StringID invalid_string_id;
};
const OpenTTDEngineCapabilitiesServices &GetRustEngineCapabilitiesServices() noexcept;
extern "C" {
uint32_t openttd_rust_engine_grfid(const OpenTTDEngineCapabilitiesServices *, const OpenTTDEngine *);
int64_t openttd_rust_engine_running_cost(const OpenTTDEngineCapabilitiesServices *, const OpenTTDEngine *);
int64_t openttd_rust_engine_cost(const OpenTTDEngineCapabilitiesServices *, const OpenTTDEngine *);
int32_t openttd_rust_engine_life_days(const OpenTTDEngineCapabilitiesServices *, const OpenTTDEngine *);
uint16_t openttd_rust_engine_range(const OpenTTDEngineCapabilitiesServices *, const OpenTTDEngine *);
int64_t openttd_rust_engine_vehicle_running_cost(const OpenTTDEngineCapabilitiesServices *, const OpenTTDEngine *, OpenTTDVehicle *);
uint8_t openttd_rust_engine_can_carry_abi(const OpenTTDEngineCapabilitiesServices *, const OpenTTDEngine *);
extern "C++" inline bool openttd_rust_engine_can_carry(const OpenTTDEngineCapabilitiesServices *a0, const OpenTTDEngine *a1) { return openttd_rust_engine_can_carry_abi(a0, a1) != 0; }
uint32_t openttd_rust_engine_capacity(const OpenTTDEngineCapabilitiesServices *, const OpenTTDEngine *, const OpenTTDVehicle *, uint16_t *);
uint8_t openttd_rust_engine_refittable_abi(const OpenTTDEngineCapabilitiesServices *, uint16_t);
extern "C++" inline bool openttd_rust_engine_refittable(const OpenTTDEngineCapabilitiesServices *a0, uint16_t a1) { return openttd_rust_engine_refittable_abi(a0, a1) != 0; }
uint8_t openttd_rust_articulated_is_abi(const OpenTTDEngineCapabilitiesServices *, uint16_t);
extern "C++" inline bool openttd_rust_articulated_is(const OpenTTDEngineCapabilitiesServices *a0, uint16_t a1) { return openttd_rust_articulated_is_abi(a0, a1) != 0; }
uint32_t openttd_rust_articulated_count(const OpenTTDEngineCapabilitiesServices *, uint16_t, bool);
void openttd_rust_articulated_capacity(const OpenTTDEngineCapabilitiesServices *, uint16_t, uint32_t *);
uint64_t openttd_rust_articulated_cargoes(const OpenTTDEngineCapabilitiesServices *, uint16_t);
uint8_t openttd_rust_articulated_refittable_abi(const OpenTTDEngineCapabilitiesServices *, uint16_t);
extern "C++" inline bool openttd_rust_articulated_refittable(const OpenTTDEngineCapabilitiesServices *a0, uint16_t a1) { return openttd_rust_articulated_refittable_abi(a0, a1) != 0; }
void openttd_rust_articulated_refit_masks(const OpenTTDEngineCapabilitiesServices *, uint16_t, bool, uint64_t *, uint64_t *);
uint64_t openttd_rust_articulated_refit_union(const OpenTTDEngineCapabilitiesServices *, uint16_t, bool);
uint64_t openttd_rust_articulated_live_cargoes(const OpenTTDEngineCapabilitiesServices *, const OpenTTDVehicle *, uint8_t *);
}
#endif
