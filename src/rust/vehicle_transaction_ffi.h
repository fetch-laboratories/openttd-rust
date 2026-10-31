/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file vehicle_transaction_ffi.h Common vehicle transaction owner boundary. */
#ifndef RUST_VEHICLE_TRANSACTION_FFI_H
#define RUST_VEHICLE_TRANSACTION_FFI_H
#include "byte_span_ffi.h"
#include "../newgrf_text_type.h"
#include "command_services_ffi.h"
#include "fleet_ffi.h"
#include "services_ffi.h"

struct Vehicle;
struct Order;
struct OpenTTDVehicleServices;
struct OpenTTDVehicleTransactionTarget { uint8_t owner; bool primary; };
struct OpenTTDVehicleTransactionServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDCommandServices *commands;
	const OpenTTDCommandCostServices *cost;
	const OpenTTDFleetGroupServices *groups;
	OpenTTDVehicleTransactionTarget (*target)(void *) noexcept OPENTTD_SERVICE_DEFAULT(target);
	OpenTTDByteSpan (*name)(void *) noexcept OPENTTD_SERVICE_DEFAULT(name);
	void (*clear_name)(void *) noexcept OPENTTD_SERVICE_DEFAULT(clear_name);
	void (*set_name)(void *, OpenTTDByteSpan) noexcept OPENTTD_SERVICE_DEFAULT(set_name);
	void (*name_dirty)(void *) noexcept OPENTTD_SERVICE_DEFAULT(name_dirty);
	bool (*interval_percent)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(interval_percent);
	uint16_t (*interval_clamp)(int32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(interval_clamp);
	void (*set_interval)(void *, uint16_t, bool, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_interval);
	uint32_t unique_name;
	size_t max_name;
};
struct OpenTTDVehicleSelectionServices {
	const OpenTTDFleetGroupServices *groups;
	const OpenTTDFleetTransactionServices *transactions;
	const OpenTTDVehicleServices *vehicles;
	bool (*in_depot)(void *) noexcept OPENTTD_SERVICE_DEFAULT(in_depot);
	bool (*free_wagon)(void *) noexcept OPENTTD_SERVICE_DEFAULT(free_wagon);
	void *(*first_engine_part)(void *) noexcept OPENTTD_SERVICE_DEFAULT(first_engine_part);
	bool (*multiheaded)(void *) noexcept OPENTTD_SERVICE_DEFAULT(multiheaded);
	void *(*other_multihead)(void *) noexcept OPENTTD_SERVICE_DEFAULT(other_multihead);
	void (*with_orders)(void *, bool (*)(void *, const Vehicle *), bool (*)(void *, const Order *), void (*)(void *, const Vehicle *)) noexcept OPENTTD_SERVICE_DEFAULT(with_orders);
	void (*list_clear)(void *) noexcept OPENTTD_SERVICE_DEFAULT(list_clear);
	void (*list_push)(void *, void *) noexcept OPENTTD_SERVICE_DEFAULT(list_push);
};
struct OpenTTDOrdersLeaves;
struct OpenTTDAircraftStartState { uint8_t state, flags; };
struct OpenTTDVehicleTransactionStrings {
	uint32_t destroyed, stopped_in_depot, no_power, in_flight, incompatible_rail, null_string;
	GRFStringID grf_text;
	uint8_t takeoff, terminal7, direct_descent, new_vehicle_expense;
	uint32_t invalid_client;
	uint8_t share_orders, copy_orders, cargo_count, invalid_cargo;
};
struct OpenTTDVehicleCommonServices {
	const OpenTTDVehicleTransactionServices *base;
	const OpenTTDFleetTransactionServices *transactions;
	const OpenTTDVehicleServices *vehicles;
	const OpenTTDOrdersLeaves *orders;
	OpenTTDVehicleTransactionStrings strings;
	uint32_t (*train_power)(void *) noexcept OPENTTD_SERVICE_DEFAULT(train_power);
	OpenTTDAircraftStartState (*aircraft_start)(void *) noexcept OPENTTD_SERVICE_DEFAULT(aircraft_start);
	uint8_t (*grf_version)(void *) noexcept OPENTTD_SERVICE_DEFAULT(grf_version);
	uint32_t (*grf_string)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(grf_string);
	void (*waiting_news)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(waiting_news);
	void (*flip_stopped)(void *) noexcept OPENTTD_SERVICE_DEFAULT(flip_stopped);
	void (*startstop_ui)(void *) noexcept OPENTTD_SERVICE_DEFAULT(startstop_ui);
	int64_t (*value)(void *) noexcept OPENTTD_SERVICE_DEFAULT(value);
	bool (*valid_cargo)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(valid_cargo);
};
struct OpenTTDVehicleListId { uint8_t kind, vehicle_type, company; uint32_t index; bool valid; };
struct OpenTTDVehicleMassServices {
	const OpenTTDVehicleCommonServices *common;
	const OpenTTDVehicleSelectionServices *selection;
	bool (*is_depot)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_depot);
	void (*send)(OpenTTDCommandCost *, void *, uint16_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(send);
	uint8_t service, mass_send, dont_cancel;
	uint32_t invalid_client;
};
const OpenTTDVehicleCommonServices &GetRustVehicleCommonServices() noexcept;
const OpenTTDVehicleSelectionServices &GetRustVehicleSelectionServices() noexcept;
const OpenTTDVehicleTransactionServices &GetRustVehicleTransactionServices() noexcept;
extern "C" {
void openttd_rust_vehicle_mass_start(const OpenTTDVehicleMassServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, bool, bool, const OpenTTDVehicleListId *);
void openttd_rust_vehicle_depot_sell(const OpenTTDVehicleMassServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t);
void openttd_rust_vehicle_depot_replace(const OpenTTDVehicleMassServices *, OpenTTDCommandCost *, const OpenTTDFleetCosts *, uint16_t, uint32_t, uint8_t);
void openttd_rust_vehicle_send(const OpenTTDVehicleMassServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, const OpenTTDVehicleListId *);
void openttd_rust_vehicle_startstop(const OpenTTDVehicleCommonServices *, OpenTTDCommandCost *, uint16_t, uint32_t, bool);
void openttd_rust_vehicle_sell(const OpenTTDVehicleCommonServices *, OpenTTDCommandCost *, uint16_t, uint32_t, bool, bool, uint32_t);
void openttd_rust_vehicle_selection_set(const OpenTTDVehicleSelectionServices *, void *, uint8_t, void *, void (*)(void *, uint32_t));
void openttd_rust_vehicle_depot_list(const OpenTTDVehicleSelectionServices *, uint8_t, uint32_t, void *, void *, bool);
bool openttd_rust_vehicle_generate_list(const OpenTTDVehicleSelectionServices *, void *, uint8_t, uint8_t, uint8_t, uint32_t);
bool openttd_rust_vehicle_unique_name(const OpenTTDVehicleTransactionServices *, OpenTTDByteSpan);
void openttd_rust_vehicle_rename(const OpenTTDVehicleTransactionServices *, OpenTTDCommandCost *, uint16_t, uint32_t, OpenTTDByteSpan);
void openttd_rust_vehicle_service_interval(const OpenTTDVehicleTransactionServices *, OpenTTDCommandCost *, uint16_t, uint32_t, uint16_t, bool, bool);
}
#endif /* RUST_VEHICLE_TRANSACTION_FFI_H */
