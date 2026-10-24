/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file engine_capabilities_ffi.h Synchronous engine and articulated query services. */
#ifndef RUST_ENGINE_CAPABILITIES_FFI_H
#define RUST_ENGINE_CAPABILITIES_FFI_H
#include "newgrf_vehicle_ffi.h"
struct OpenTTDEngineCapabilitiesServices {
	const OpenTTDVehicleResolverServices *(*resolver)() noexcept OPENTTD_SERVICE_DEFAULT(resolver);
	const OpenTTDEngine *(*engine)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(engine);
	const OpenTTDEngine *(*engine_if_valid)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(engine_if_valid);
	VehicleType (*kind)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(kind);
	bool (*ground)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(ground);
	uint16_t (*id)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(id);
	uint8_t (*default_cargo)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(default_cargo);
	uint16_t (*callbacks)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(callbacks);
	uint8_t (*misc)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(misc);
	uint64_t (*refit_mask)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(refit_mask);
	uint8_t (*rail_capacity)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(rail_capacity);
	uint8_t (*road_capacity)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(road_capacity);
	uint16_t (*ship_capacity)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(ship_capacity);
	uint16_t (*passenger_capacity)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(passenger_capacity);
	uint8_t (*mail_capacity)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(mail_capacity);
	RailVehicleTypes (*rail_kind)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(rail_kind);
	const GRFFile *(*grf)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(grf);
	uint8_t (*version)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(version);
	uint16_t (*new_engine)(const GRFFile *, VehicleType, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(new_engine);
	uint16_t (*cargo_classes)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_classes);
	uint16_t (*multiplier)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(multiplier);
	uint8_t (*cargo_by_label)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_by_label);
	bool (*pool_available)(size_t) noexcept OPENTTD_SERVICE_DEFAULT(pool_available);
	OpenTTDVehicle *(*new_temporary)() noexcept OPENTTD_SERVICE_DEFAULT(new_temporary);
	void (*destroy_temporary)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(destroy_temporary);
	void (*set_engine)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_engine);
	void (*set_owner)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_owner);
	uint8_t (*current_company)() noexcept OPENTTD_SERVICE_DEFAULT(current_company);
	uint16_t (*vehicle_engine)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_engine);
	const OpenTTDEngine *(*vehicle_engine_object)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_engine_object);
	uint8_t (*vehicle_cargo)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_cargo);
	uint8_t (*vehicle_subtype)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_subtype);
	bool (*has_articulated)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(has_articulated);
	OpenTTDVehicle *(*next_articulated)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(next_articulated);
	void (*check)(bool) noexcept OPENTTD_SERVICE_DEFAULT(check);
	bool assertions;
};
const OpenTTDEngineCapabilitiesServices &GetRustEngineCapabilitiesServices() noexcept;
extern "C" {
bool openttd_rust_engine_can_carry(const OpenTTDEngineCapabilitiesServices *, const OpenTTDEngine *);
uint32_t openttd_rust_engine_capacity(const OpenTTDEngineCapabilitiesServices *, const OpenTTDEngine *, const Vehicle *, uint16_t *);
bool openttd_rust_engine_refittable(const OpenTTDEngineCapabilitiesServices *, uint16_t);
bool openttd_rust_articulated_is(const OpenTTDEngineCapabilitiesServices *, uint16_t);
uint32_t openttd_rust_articulated_count(const OpenTTDEngineCapabilitiesServices *, uint16_t, bool);
void openttd_rust_articulated_capacity(const OpenTTDEngineCapabilitiesServices *, uint16_t, uint32_t *);
uint64_t openttd_rust_articulated_cargoes(const OpenTTDEngineCapabilitiesServices *, uint16_t);
bool openttd_rust_articulated_refittable(const OpenTTDEngineCapabilitiesServices *, uint16_t);
void openttd_rust_articulated_refit_masks(const OpenTTDEngineCapabilitiesServices *, uint16_t, bool, uint64_t *, uint64_t *);
uint64_t openttd_rust_articulated_refit_union(const OpenTTDEngineCapabilitiesServices *, uint16_t, bool);
uint64_t openttd_rust_articulated_live_cargoes(const OpenTTDEngineCapabilitiesServices *, const Vehicle *, uint8_t *);
}
#endif
