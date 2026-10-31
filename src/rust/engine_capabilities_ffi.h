/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file engine_capabilities_ffi.h Synchronous engine and articulated query services. */
#ifndef RUST_ENGINE_CAPABILITIES_FFI_H
#define RUST_ENGINE_CAPABILITIES_FFI_H
#include "newgrf_vehicle_ffi.h"
#include "engine_ffi.h"
struct OpenTTDEngineLoadingServices;
struct OpenTTDEngineCapabilitiesServices {
	const OpenTTDVehicleResolverServices *(*resolver)() noexcept OPENTTD_SERVICE_DEFAULT(resolver);
	const OpenTTDEngine *(*engine)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(engine);
	const OpenTTDEngine *(*engine_if_valid)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(engine_if_valid);
	VehicleType (*kind)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(kind);
	bool (*ground)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(ground);
	uint16_t (*id)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(id);
	uint8_t (*default_cargo)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(default_cargo);
	uint16_t (*callbacks)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(callbacks);
	OpenTTDEngineCarrying (*carrying)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(carrying);
	OpenTTDEngineCapacityConfig (*capacity_config)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(capacity_config);
	uint64_t (*refit_mask)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(refit_mask);
	uint16_t (*ship_capacity)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(ship_capacity);
	uint16_t (*passenger_capacity)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(passenger_capacity);
	uint8_t (*mail_capacity)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(mail_capacity);
	EngineClass (OPENTTD_RUST_CALL *train_class)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(train_class);
	uint8_t (OPENTTD_RUST_CALL *aircraft_subtype)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(aircraft_subtype);
	RailVehicleTypes (*rail_kind)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(rail_kind);
	const GRFFile *(*grf)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(grf);
	uint8_t (*version)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(version);
	const OpenTTDEngineLoadingServices *(*loading)() noexcept OPENTTD_SERVICE_DEFAULT(loading);
	uint16_t (*cargo_classes)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_classes);
	uint16_t (*multiplier)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(multiplier);
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
	uint8_t (*aircraft_cost)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(aircraft_cost);
	uint16_t (*aircraft_range)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(aircraft_range);
	uint8_t (*aircraft_running)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(aircraft_running);
	int32_t (*life_extension)() noexcept OPENTTD_SERVICE_DEFAULT(life_extension);
	int32_t (*life_years)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(life_years);
	int32_t (*price_shift)(const OpenTTDEngine *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(price_shift);
	uint8_t (*road_cost)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(road_cost);
	Price (*road_price)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(road_price);
	uint8_t (*road_running)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(road_running);
	uint8_t (*ship_cost)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(ship_cost);
	uint8_t (*ship_running)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(ship_running);
	uint8_t (*train_cost)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(train_cost);
	Price (*train_price)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(train_price);
	uint8_t (*train_running)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(train_running);
	bool (*valid_string)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(valid_string);
};
const OpenTTDEngineCapabilitiesServices &GetRustEngineCapabilitiesServices() noexcept;
extern "C" {
uint32_t openttd_rust_engine_grfid(const OpenTTDEngineCapabilitiesServices *, const OpenTTDEngine *);
int64_t openttd_rust_engine_running_cost(const OpenTTDEngineCapabilitiesServices *, const OpenTTDEngine *);
int64_t openttd_rust_engine_cost(const OpenTTDEngineCapabilitiesServices *, const OpenTTDEngine *);
int32_t openttd_rust_engine_life_days(const OpenTTDEngineCapabilitiesServices *, const OpenTTDEngine *);
uint16_t openttd_rust_engine_range(const OpenTTDEngineCapabilitiesServices *, const OpenTTDEngine *);
int64_t openttd_rust_engine_vehicle_running_cost(const OpenTTDEngineCapabilitiesServices *, const OpenTTDEngine *, OpenTTDVehicle *);
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
