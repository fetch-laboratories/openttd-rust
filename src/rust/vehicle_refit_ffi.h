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
const OpenTTDVehicleRefitServices &GetRustVehicleRefitServices() noexcept;
extern "C" {
OpenTTDVehicleRefitCapacity openttd_rust_vehicle_refit(const OpenTTDVehicleRefitServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, uint8_t, bool, bool, uint8_t);
void openttd_rust_vehicle_refit_cost(const OpenTTDVehicleRefitServices *, OpenTTDCommandCost *, OpenTTDVehicle *, uint16_t, uint8_t, uint8_t, bool *);
}
#endif
