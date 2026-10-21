/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file vehicle_refit_ffi.h Deferred common refit transaction services. */
#ifndef RUST_VEHICLE_REFIT_FFI_H
#define RUST_VEHICLE_REFIT_FFI_H
#include "vehicle_transaction_ffi.h"
#include "vehicle_ffi.h"
#include "train_ffi.h"
#include "road_ffi.h"
#include "aircraft_ffi.h"
#include "ship_control_ffi.h"
#include "cargo_storage_ffi.h"
struct OpenTTDVehicleRefitCapacity { uint32_t total; uint16_t mail; uint32_t cargo[64]; };
struct OpenTTDRefitCargoKind { uint8_t cargo, subtype; };
struct OpenTTDRefitCommit { uint8_t cargo; uint16_t capacity; };
struct OpenTTDRefitEngine { uint64_t mask; bool automatic; };
struct OpenTTDRefitCargo { uint16_t classes; uint8_t local; };
struct OpenTTDRefitPrices {
	uint32_t train, wagon, road, ship, aircraft;
	uint8_t train_run, road_run, ship_run, aircraft_run, auto_refit, refit;
};
struct OpenTTDVehicleRefitServices {
	const OpenTTDVehicleCommonServices *common;
	const OpenTTDVehicleSelectionServices *selection;
	const OpenTTDCargoStorageServices *cargo;
	const OpenTTDVehicleSchedulerServices *scheduler;
	const OpenTTDTrainServices *train;
	const OpenTTDRoadLeaves *road;
	const OpenTTDAircraftLeaves *aircraft;
	const OpenTTDShipLeaves *ship;
	const OpenTTDGroundServices *road_ground;
	OpenTTDRefitPrices prices;
	const OpenTTDEngine *(*engine)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(engine);
	const OpenTTDEngine *(*vehicle_engine)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_engine);
	bool (*has_grf)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(has_grf);
	uint8_t (*refit_cost)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(refit_cost);
	OpenTTDRefitCargo (*refit_cargo)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(refit_cargo);
	uint16_t (*cost_callback)(OpenTTDVehicle *, uint16_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(cost_callback);
	VehicleType (*engine_kind)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_kind);
	RailVehicleTypes (*rail_kind)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(rail_kind);
	int32_t (*price_shift)(const OpenTTDEngine *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(price_shift);
	bool (*can_carry)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(can_carry);
	OpenTTDRefitEngine (*refit_engine)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(refit_engine);
	uint32_t (*capacity)(const OpenTTDEngine *, OpenTTDVehicle *, uint16_t *) noexcept OPENTTD_SERVICE_DEFAULT(capacity);
	OpenTTDRefitCargoKind (*cargo_kind)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_kind);
	void (*set_cargo_kind)(OpenTTDVehicle *, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_cargo_kind);
	ExpensesType (*expense)(OpenTTDVehicle *, bool) noexcept OPENTTD_SERVICE_DEFAULT(expense);
	void (*invalidate_chain)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(invalidate_chain);
	OpenTTDRefitCommit (*commit)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(commit);
	void (*set_refit_cap)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_refit_cap);
	uint16_t (*refit_cap)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(refit_cap);
	uint16_t (*cargo_cap)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_cap);
	void (*set_capacity)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_capacity);
	void (*write_refit)(OpenTTDVehicle *, uint8_t, uint16_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_refit);
	uint8_t (*mail_cargo)() noexcept OPENTTD_SERVICE_DEFAULT(mail_cargo);
	OpenTTDTrainHandle (*train_handle)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(train_handle);
	OpenTTDRoadState *(*road_state)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(road_state);
	OpenTTDAircraftVehicle (*aircraft_handle)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(aircraft_handle);
	uint8_t (*road_acceleration)() noexcept OPENTTD_SERVICE_DEFAULT(road_acceleration);
	void (*details_ui)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(details_ui);
	void (*lists_ui)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(lists_ui);
};
struct OpenTTDArticulatedPart { uint8_t image; uint16_t capacity; };
struct OpenTTDArticulatedProbability { bool present, value; };
struct OpenTTDVehicleArticulatedServices {
	const OpenTTDVehicleRefitServices *refit;
	uint8_t (*default_cargo)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(default_cargo);
	void (*capacity)(uint16_t, uint32_t *) noexcept OPENTTD_SERVICE_DEFAULT(capacity);
	bool (*pool_available)(size_t) noexcept OPENTTD_SERVICE_DEFAULT(pool_available);
	bool (*has_callback)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(has_callback);
	uint16_t (*next_part)(uint32_t, uint16_t, void *, bool *) noexcept OPENTTD_SERVICE_DEFAULT(next_part);
	void *(*prepare_previous)(void *) noexcept OPENTTD_SERVICE_DEFAULT(prepare_previous);
	void *(*new_train)(void *) noexcept OPENTTD_SERVICE_DEFAULT(new_train);
	void *(*new_road)(void *) noexcept OPENTTD_SERVICE_DEFAULT(new_road);
	void (*road_length)(void *) noexcept OPENTTD_SERVICE_DEFAULT(road_length);
	void (*copy_roadtype)(void *, void *) noexcept OPENTTD_SERVICE_DEFAULT(copy_roadtype);
	OpenTTDArticulatedPart (*train_info)(const OpenTTDEngine *, bool) noexcept OPENTTD_SERVICE_DEFAULT(train_info);
	OpenTTDArticulatedPart (*road_info)(const OpenTTDEngine *, bool) noexcept OPENTTD_SERVICE_DEFAULT(road_info);
	void (*set_part)(void *, uint8_t, uint8_t, uint16_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_part);
	void (*mark_train)(void *) noexcept OPENTTD_SERVICE_DEFAULT(mark_train);
	void (*mark_road)(void *) noexcept OPENTTD_SERVICE_DEFAULT(mark_road);
	void (*copy_common)(void *, void *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(copy_common);
	void (*random_bits)(void *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(random_bits);
	void (*flip_image)(void *) noexcept OPENTTD_SERVICE_DEFAULT(flip_image);
	OpenTTDArticulatedProbability (*reverse)(void *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(reverse);
	void (*position)(void *) noexcept OPENTTD_SERVICE_DEFAULT(position);
	uint64_t (*available_cargo)(uint16_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(available_cargo);
	void (*warning)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(warning);
	bool (*has_articulated)(void *) noexcept OPENTTD_SERVICE_DEFAULT(has_articulated);
	void *(*next_articulated)(void *) noexcept OPENTTD_SERVICE_DEFAULT(next_articulated);
	uint32_t max_parts;
	uint64_t all_cargo;
};
const OpenTTDVehicleArticulatedServices &GetRustVehicleArticulatedServices() noexcept;
struct OpenTTDVehicleBuildResult { uint32_t id; OpenTTDVehicleRefitCapacity capacity; };
struct OpenTTDAircraftInfo { uint8_t image; uint16_t passengers, mail; uint8_t subtype; };
struct OpenTTDVehicleConstructServices {
	const OpenTTDVehicleArticulatedServices *articulated;
	uint64_t (*powered_roadtypes)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(powered_roadtypes);
	bool (*any_roadtype)(uint32_t, uint64_t) noexcept OPENTTD_SERVICE_DEFAULT(any_roadtype);
	void *(*new_road)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(new_road);
	void (*road_front)(void *, const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(road_front);
	const OpenTTDStation *(*station)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station);
	bool (*can_use_station)(const OpenTTDEngine *, const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(can_use_station);
	uint32_t (*hangar_tile)(const OpenTTDStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(hangar_tile);
	uint8_t (*pos_on_build)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(pos_on_build);
	void *(*new_aircraft)() noexcept OPENTTD_SERVICE_DEFAULT(new_aircraft);
	void (*aircraft_place)(void *, void *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(aircraft_place);
	OpenTTDAircraftInfo (*aircraft_info)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(aircraft_info);
	void (*aircraft_properties)(void *, void *, const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(aircraft_properties);
	void (*rotor)(void *, void *, void *) noexcept OPENTTD_SERVICE_DEFAULT(rotor);
	void (*aircraft_capacity)(const OpenTTDEngine *, void *, void *) noexcept OPENTTD_SERVICE_DEFAULT(aircraft_capacity);
	void (*road_capacity)(void *, void *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(road_capacity);
	void (*set_cargo_type)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_cargo_type);
	void (*engine_common)(void *, const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_common);
	void (*service_dates)(void *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(service_dates);
	void (*build_stamp)(void *) noexcept OPENTTD_SERVICE_DEFAULT(build_stamp);
	bool (*exclusive_preview)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(exclusive_preview);
	void (*service_flags)(void *, bool, bool) noexcept OPENTTD_SERVICE_DEFAULT(service_flags);
	uint32_t wrong_depot;
};
struct OpenTTDVehicleBuildServices {
	const OpenTTDVehicleRefitServices *refit;
	const OpenTTDVehicleMassServices *mass;
	const OpenTTDRandomServices *random;
	VehicleType (*depot_kind)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_kind);
	int64_t (*engine_cost)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_cost);
	uint8_t (*default_cargo)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(default_cargo);
	uint32_t (*articulate_count)(uint16_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(articulate_count);
	bool (*pool_available)(size_t) noexcept OPENTTD_SERVICE_DEFAULT(pool_available);
	uint16_t (*free_unit)(VehicleType) noexcept OPENTTD_SERVICE_DEFAULT(free_unit);
	const OpenTTDVehicleConstructServices *construct;
	void *(*rail)(OpenTTDCommandCost *, uint16_t, uint32_t, const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(rail);
	uint32_t (*ship_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(ship_tile);
	void *(*new_ship)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(new_ship);
	void (*set_purchase)(void *, uint16_t, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(set_purchase);
	void (*articulated_capacity)(uint16_t, uint32_t *) noexcept OPENTTD_SERVICE_DEFAULT(articulated_capacity);
	uint32_t (*display_capacity)(const OpenTTDEngine *, uint16_t *) noexcept OPENTTD_SERVICE_DEFAULT(display_capacity);
	bool (*is_engine)(void *) noexcept OPENTTD_SERVICE_DEFAULT(is_engine);
	void (*normalize)(void *) noexcept OPENTTD_SERVICE_DEFAULT(normalize);
	void (*presentation)(void *, VehicleType) noexcept OPENTTD_SERVICE_DEFAULT(presentation);
	void (*use_unit)(void *) noexcept OPENTTD_SERVICE_DEFAULT(use_unit);
	uint32_t too_many, unavailable;
};
struct OpenTTDVehicleCloneServices {
	const OpenTTDVehicleBuildServices *build;
	void *(*next_vehicle)(void *) noexcept OPENTTD_SERVICE_DEFAULT(next_vehicle);
	bool (*has_articulated)(void *) noexcept OPENTTD_SERVICE_DEFAULT(has_articulated);
	void *(*next_articulated)(void *) noexcept OPENTTD_SERVICE_DEFAULT(next_articulated);
	void (*copy_service)(void *, void *) noexcept OPENTTD_SERVICE_DEFAULT(copy_service);
	long (*parse_suffix)(OpenTTDByteSpan) noexcept OPENTTD_SERVICE_DEFAULT(parse_suffix);
};
const OpenTTDVehicleCloneServices &GetRustVehicleCloneServices() noexcept;
const OpenTTDVehicleBuildServices &GetRustVehicleBuildServices() noexcept;
const OpenTTDVehicleRefitServices &GetRustVehicleRefitServices() noexcept;
extern "C" {
void openttd_rust_vehicle_add_articulated(const OpenTTDVehicleArticulatedServices *, void *);
void openttd_rust_vehicle_articulated_consistency(const OpenTTDVehicleArticulatedServices *, void *);
uint32_t openttd_rust_vehicle_clone(const OpenTTDVehicleCloneServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint32_t, bool);
OpenTTDVehicleBuildResult openttd_rust_vehicle_build(const OpenTTDVehicleBuildServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint16_t, bool, uint8_t, uint32_t);
OpenTTDVehicleRefitCapacity openttd_rust_vehicle_refit(const OpenTTDVehicleRefitServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, uint8_t, bool, bool, uint8_t);
}
#endif
