/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file vehicle_cmd.cpp Commands for vehicles. */

#include "stdafx.h"
#include "roadveh.h"
#include "news_func.h"
#include "airport.h"
#include "command_func.h"
#include "company_func.h"
#include "train.h"
#include "aircraft.h"
#include "newgrf_text.h"
#include "vehicle_func.h"
#include "string_func.h"
#include "depot_map.h"
#include "vehiclelist.h"
#include "engine_func.h"
#include "articulated_vehicles.h"
#include "autoreplace_gui.h"
#include "group.h"
#include "order_backup.h"
#include "ship.h"
#include "newgrf.h"
#include "company_base.h"
#include "core/random_func.hpp"
#include "vehicle_cmd.h"
#include "aircraft_cmd.h"
#include "autoreplace_cmd.h"
#include "group_cmd.h"
#include "order_cmd.h"
#include "roadveh_cmd.h"
#include "train_cmd.h"
#include "ship_cmd.h"
#include <charconv>

#include "table/strings.h"


#include "rust/vehicle_transaction_ffi.h"
#include "rust/vehicle_refit_ffi.h"
#include "rust/engine_services.h"
#include "rust/services_cargo.h"
#include "rust/services_map.h"
#include "rust/settings_services.h"
#include "rust/command_cost_services.h"
#include "rust/vehicle_services.h"
#include "rust/company_services.h"
#include "rust/presentation_services.h"
#include "rust/orders_ffi.h"
#include "rust/vehicle_maintenance_ffi.h"
#include "rust/ship_control_ffi.h"
#include "rust/services_station.h"
#include "road_map.h"
#include "landscape.h"
#include "station_base.h"
#include "timer/timer_game_calendar.h"
#include "timer/timer_game_economy.h"
#include "table/sprites.h"

/* Synchronous native storage/presentation leaves only. No Rust borrow crosses
 * these calls; allocation or formatting failures terminate in noexcept leaves. */
static OpenTTDVehicleTransactionTarget VehicleTransactionTarget(void *shell) noexcept
{
	const Vehicle *v = static_cast<Vehicle *>(shell);
	return {.owner = RustVehicleOwner(v), .primary = RustVehiclePrimary(v)};
}
static void VehicleTransactionNameDirty(void *shell) noexcept
{
	InvalidateWindowClassesData(GetWindowClassForVehicleType(RustVehicleVehicleType(static_cast<Vehicle *>(shell))), 1);
	RustScreenDirty();
}
static uint16_t VehicleTransactionClampInterval(int32_t interval, bool percent) noexcept { return GetServiceIntervalClamped(interval, percent); }
static void VehicleTransactionSetInterval(void *shell, uint16_t interval, bool custom, bool percent) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(shell);
	v->SetServiceInterval(interval);
	v->SetServiceIntervalIsCustom(custom);
	RustVehicleSetServiceIntervalIsPercent(v, percent);
	GetRustVehicleServices().maintenance->details_dirty(RustVehicle(v));
}
const OpenTTDVehicleTransactionServices &GetRustVehicleTransactionServices() noexcept
{
	static const OpenTTDVehicleTransactionServices services{
		.shared = &GetRustSharedServices(),
		.commands = &GetRustCommandServices(),
		.cost = &GetRustCommandCostServices(),
		.groups = &FleetGroupServices(),
		.target = VehicleTransactionTarget,
		.name = RustVehicleName,
		.clear_name = RustVehicleClearName,
		.set_name = RustVehicleSetName,
		.name_dirty = VehicleTransactionNameDirty,
		.interval_percent = RustCompanyServiceIntervalPercent,
		.interval_clamp = VehicleTransactionClampInterval,
		.set_interval = VehicleTransactionSetInterval,
		.unique_name = STR_ERROR_NAME_MUST_BE_UNIQUE,
		.max_name = MAX_LENGTH_VEHICLE_NAME_CHARS,
	};
	return services;
}
static OpenTTDAircraftStartState VehicleTransactionAircraftStart(void *shell) noexcept
{
	const Aircraft *v = Aircraft::From(static_cast<Vehicle *>(shell));
	return {.state = v->state, .flags = v->flags};
}
static void VehicleTransactionWaitingNews(uint32_t id) noexcept { DeleteVehicleNews(VehicleID(id), AdviceType::VehicleWaiting); }
static void VehicleTransactionStartstopUI(void *shell) noexcept
{
	const Vehicle *v = static_cast<Vehicle *>(shell);
	GetRustVehicleServices().start_stop_dirty(RustVehicle(v));
	FleetGroupServices().depot_dirty(shell);
	openttd_orders_dirty_vehicle_windows(shell);
	RustVehicleInvalidateView(v->index);
}
const OpenTTDVehicleCommonServices &GetRustVehicleCommonServices() noexcept
{
	static const OpenTTDVehicleCommonServices services{
		.base = &GetRustVehicleTransactionServices(),
		.transactions = &FleetTransactionServices(),
		.vehicles = &GetRustVehicleServices(),
		.orders = &GetRustOrdersLeaves(),
		.strings = {
			.destroyed = STR_ERROR_VEHICLE_IS_DESTROYED,
			.stopped_in_depot = STR_ERROR_TRAIN_MUST_BE_STOPPED_INSIDE_DEPOT,
			.no_power = STR_ERROR_TRAIN_START_NO_POWER,
			.in_flight = STR_ERROR_AIRCRAFT_IS_IN_FLIGHT,
			.incompatible_rail = STR_ERROR_INCOMPATIBLE_RAIL_TYPES,
			.null_string = STR_NULL,
			.grf_text = GRFSTR_MISC_GRF_TEXT,
			.takeoff = STARTTAKEOFF,
			.terminal7 = TERM7,
			.direct_descent = 1 << VAF_HELI_DIRECT_DESCENT,
			.new_vehicle_expense = EXPENSES_NEW_VEHICLES,
			.invalid_client = INVALID_CLIENT_ID, .share_orders = CO_SHARE, .copy_orders = CO_COPY,
			.cargo_count = NUM_CARGO, .invalid_cargo = INVALID_CARGO,
		},
		.train_power = RustOpaqueVehicleRead<uint32_t, RustTrainCachedPower>,
		.aircraft_start = VehicleTransactionAircraftStart,
		.grf_version = RustOpaqueVehicleRead<uint8_t, RustVehicleGRFVersion>,
		.grf_string = RustGRFString,
		.waiting_news = VehicleTransactionWaitingNews,
		.flip_stopped = RustOpaqueVehicleInvoke<RustVehicleFlipStopped>,
		.startstop_ui = VehicleTransactionStartstopUI,
		.value = RustOpaqueVehicleRead<int64_t, RustVehicleValue>,
		.valid_cargo = OpenTTDIsValidCargo,
	};
	return services;
}
#include "rust/vehicle_refit_services.hpp"
static OpenTTDVehicleListId VehicleTransactionListId(const VehicleListIdentifier &vli) noexcept
{
	return {.kind = static_cast<uint8_t>(vli.type), .vehicle_type = vli.vtype, .company = vli.company.base(), .index = vli.index, .valid = vli.Valid()};
}
static void VehicleTransactionSend(OpenTTDCommandCost *out, void *v, uint16_t flags, uint8_t command) noexcept { *reinterpret_cast<CommandCost *>(out) = static_cast<Vehicle *>(v)->SendToDepot(DoCommandFlags(flags), DepotCommandFlags(command)); }
static const OpenTTDVehicleMassServices &GetVehicleMassServices() noexcept
{
	static const OpenTTDVehicleMassServices services{
		.common = &GetRustVehicleCommonServices(), .selection = &GetRustVehicleSelectionServices(),
		.is_depot = SharedIsDepotTile, .send = VehicleTransactionSend,
		.service = DepotCommandFlags(DepotCommandFlag::Service).base(), .mass_send = DepotCommandFlags(DepotCommandFlag::MassSend).base(), .dont_cancel = DepotCommandFlags(DepotCommandFlag::DontCancel).base(), .invalid_client = INVALID_CLIENT_ID,
	};
	return services;
}

#include "rust/vehicle_build_services.hpp"


#include "safeguards.h"

/* Tables used in vehicle_func.h to find the right error message for a certain vehicle type */
const StringID _veh_build_msg_table[] = {
	STR_ERROR_CAN_T_BUY_TRAIN,
	STR_ERROR_CAN_T_BUY_ROAD_VEHICLE,
	STR_ERROR_CAN_T_BUY_SHIP,
	STR_ERROR_CAN_T_BUY_AIRCRAFT,
};

const StringID _veh_sell_msg_table[] = {
	STR_ERROR_CAN_T_SELL_TRAIN,
	STR_ERROR_CAN_T_SELL_ROAD_VEHICLE,
	STR_ERROR_CAN_T_SELL_SHIP,
	STR_ERROR_CAN_T_SELL_AIRCRAFT,
};

const StringID _veh_sell_all_msg_table[] = {
	STR_ERROR_CAN_T_SELL_ALL_TRAIN,
	STR_ERROR_CAN_T_SELL_ALL_ROAD_VEHICLE,
	STR_ERROR_CAN_T_SELL_ALL_SHIP,
	STR_ERROR_CAN_T_SELL_ALL_AIRCRAFT,
};

const StringID _veh_autoreplace_msg_table[] = {
	STR_ERROR_CAN_T_AUTOREPLACE_TRAIN,
	STR_ERROR_CAN_T_AUTOREPLACE_ROAD_VEHICLE,
	STR_ERROR_CAN_T_AUTOREPLACE_SHIP,
	STR_ERROR_CAN_T_AUTOREPLACE_AIRCRAFT,
};

const StringID _veh_refit_msg_table[] = {
	STR_ERROR_CAN_T_REFIT_TRAIN,
	STR_ERROR_CAN_T_REFIT_ROAD_VEHICLE,
	STR_ERROR_CAN_T_REFIT_SHIP,
	STR_ERROR_CAN_T_REFIT_AIRCRAFT,
};

const StringID _send_to_depot_msg_table[] = {
	STR_ERROR_CAN_T_SEND_TRAIN_TO_DEPOT,
	STR_ERROR_CAN_T_SEND_ROAD_VEHICLE_TO_DEPOT,
	STR_ERROR_CAN_T_SEND_SHIP_TO_DEPOT,
	STR_ERROR_CAN_T_SEND_AIRCRAFT_TO_HANGAR,
};


/**
 * Build a vehicle.
 * @param flags for command
 * @param tile tile of depot where the vehicle is built
 * @param eid vehicle type being built.
 * @param use_free_vehicles use free vehicles when building the vehicle.
 * @param cargo refit cargo type.
 * @param client_id User
 * @return the cost of this operation + the new vehicle ID + the refitted capacity + the refitted mail capacity (aircraft) or an error
 */
std::tuple<CommandCost, VehicleID, uint, uint16_t, CargoArray> CmdBuildVehicle(DoCommandFlags flags, TileIndex tile, EngineID eid, bool use_free_vehicles, CargoType cargo, ClientID client_id)
{
	CommandCost cost, temporary, refit_temporary;
	auto result = openttd_rust_vehicle_build(&GetRustVehicleBuildServices(), RustCommandCost(&cost), RustCommandCost(&temporary), RustCommandCost(&refit_temporary), flags.base(), tile.base(), eid.base(), use_free_vehicles, cargo, client_id);
	CargoArray capacity;
	std::copy(std::begin(result.capacity.cargo), std::end(result.capacity.cargo), capacity.begin());
	return {std::move(cost), VehicleID(result.id), result.capacity.total, result.capacity.mail, capacity};
}

/**
 * Sell a vehicle.
 * @param flags for command.
 * @param v_id vehicle ID being sold.
 * @param sell_chain sell the vehicle and all vehicles following it in the chain.
 * @param backup_order make a backup of the vehicle's order (if an engine).
 * @param client_id User.
 * @return the cost of this operation or an error.
 */
CommandCost CmdSellVehicle(DoCommandFlags flags, VehicleID v_id, bool sell_chain, bool backup_order, ClientID client_id)
{
	CommandCost cost;
	openttd_rust_vehicle_sell(&GetRustVehicleCommonServices(), RustCommandCost(&cost), flags.base(), v_id.base(), sell_chain, backup_order, client_id);
	return cost;
}

/**
 * Helper to run the refit cost callback.
 * @param v The vehicle we are refitting, can be nullptr.
 * @param engine_type Which engine to refit
 * @param new_cargo_type Cargo type we are refitting to.
 * @param new_subtype New cargo subtype.
 * @param[out] auto_refit_allowed The refit is allowed as an auto-refit.
 * @return Price for refitting
 */

/**
 * Learn the price of refitting a certain engine
 * @param v The vehicle we are refitting, can be nullptr.
 * @param engine_type Which engine to refit
 * @param new_cargo_type Cargo type we are refitting to.
 * @param new_subtype New cargo subtype.
 * @param[out] auto_refit_allowed The refit is allowed as an auto-refit.
 * @return Price for refitting
 */


/**
 * Refits a vehicle to the specified cargo type.
 * @param flags type of operation
 * @param veh_id vehicle ID to refit
 * @param new_cargo_type New cargo type to refit to.
 * @param new_subtype New cargo subtype to refit to. 0xFF means to try keeping the same subtype according to GetBestFittingSubType().
 * @param auto_refit Automatic refitting.
 * @param only_this Refit only this vehicle. Used only for cloning vehicles.
 * @param num_vehicles Number of vehicles to refit (not counting articulated parts). Zero means all vehicles.
 *                     Only used if "refit only this vehicle" is false.
 * @return the cost of this operation or an error
 */
std::tuple<CommandCost, uint, uint16_t, CargoArray> CmdRefitVehicle(DoCommandFlags flags, VehicleID veh_id, CargoType new_cargo_type, uint8_t new_subtype, bool auto_refit, bool only_this, uint8_t num_vehicles)
{
	CommandCost cost, temporary;
	auto result = openttd_rust_vehicle_refit(&GetRustVehicleRefitServices(), reinterpret_cast<OpenTTDCommandCost *>(&cost), reinterpret_cast<OpenTTDCommandCost *>(&temporary), flags.base(), veh_id.base(), new_cargo_type, new_subtype, auto_refit, only_this, num_vehicles);
	CargoArray capacities{};
	std::copy(std::begin(result.cargo), std::end(result.cargo), capacities.begin());
	return {std::move(cost), result.total, result.mail, capacities};
}

/**
 * Start/Stop a vehicle
 * @param flags type of operation
 * @param veh_id vehicle to start/stop, don't forget to change CcStartStopVehicle if you modify this!
 * @param evaluate_startstop_cb Shall the start/stop newgrf callback be evaluated (only valid with DoCommandFlag::AutoReplace for network safety)
 * @return the cost of this operation or an error
 */
CommandCost CmdStartStopVehicle(DoCommandFlags flags, VehicleID veh_id, bool evaluate_startstop_cb)
{
	CommandCost cost;
	openttd_rust_vehicle_startstop(&GetRustVehicleCommonServices(), RustCommandCost(&cost), flags.base(), veh_id.base(), evaluate_startstop_cb);
	return cost;
}

/**
 * Starts or stops a lot of vehicles
 * @param flags type of operation
 * @param tile Tile of the depot where the vehicles are started/stopped (only used for depots)
 * @param do_start set = start vehicles, unset = stop vehicles
 * @param vehicle_list_window if set, then it's a vehicle list window, not a depot and Tile is ignored in this case
 * @param vli VehicleListIdentifier
 * @return the cost of this operation or an error
 */
CommandCost CmdMassStartStopVehicle(DoCommandFlags flags, TileIndex tile, bool do_start, bool vehicle_list_window, const VehicleListIdentifier &vli)
{
	CommandCost cost, item;
	auto list = VehicleTransactionListId(vli);
	openttd_rust_vehicle_mass_start(&GetVehicleMassServices(), RustCommandCost(&cost), RustCommandCost(&item), flags.base(), tile.base(), do_start, vehicle_list_window, &list);
	return cost;
}

/**
 * Sells all vehicles in a depot
 * @param flags type of operation
 * @param tile Tile of the depot where the depot is
 * @param vehicle_type Vehicle type
 * @return the cost of this operation or an error
 */
CommandCost CmdDepotSellAllVehicles(DoCommandFlags flags, TileIndex tile, VehicleType vehicle_type)
{
	CommandCost cost, item, last;
	openttd_rust_vehicle_depot_sell(&GetVehicleMassServices(), RustCommandCost(&cost), RustCommandCost(&item), RustCommandCost(&last), flags.base(), tile.base(), vehicle_type);
	return cost;
}

/**
 * Autoreplace all vehicles in the depot
 * @param flags type of operation
 * @param tile Tile of the depot where the vehicles are
 * @param vehicle_type Type of vehicle
 * @return the cost of this operation or an error
 */
CommandCost CmdDepotMassAutoReplace(DoCommandFlags flags, TileIndex tile, VehicleType vehicle_type)
{
	CommandCost cost, item, replace, build, copy, temporary;
	SavedRandomSeeds seeds;
	OpenTTDFleetCosts costs{&item, &replace, &build, &copy, &temporary, &seeds};
	openttd_rust_vehicle_depot_replace(&GetVehicleMassServices(), RustCommandCost(&cost), &costs, flags.base(), tile.base(), vehicle_type);
	return cost;
}

/**
 * Test if a name is unique among vehicle names.
 * @param name Name to test.
 * @return True ifffffff the name is unique.
 */
bool IsUniqueVehicleName(const std::string &name)
{
	return openttd_rust_vehicle_unique_name(&GetRustVehicleTransactionServices(), {reinterpret_cast<const uint8_t *>(name.data()), name.size()});
}

/**
 * Clone the custom name of a vehicle, adding or incrementing a number.
 * @param src Source vehicle, with a custom name.
 * @param dst Destination vehicle.
 */

/**
 * Clone a vehicle. If it is a train, it will clone all the cars too
 * @param flags type of operation
 * @param tile tile of the depot where the cloned vehicle is build
 * @param veh_id the original vehicle's index
 * @param share_orders shared orders, else copied orders
 * @return the cost of this operation + the new vehicle ID or an error
 */
std::tuple<CommandCost, VehicleID> CmdCloneVehicle(DoCommandFlags flags, TileIndex tile, VehicleID veh_id, bool share_orders)
{
	CommandCost total, part, temporary, refit_temporary;
	uint32_t id = openttd_rust_vehicle_clone(&GetRustVehicleCloneServices(), RustCommandCost(&total), RustCommandCost(&part), RustCommandCost(&temporary), RustCommandCost(&refit_temporary), flags.base(), tile.base(), veh_id.base(), share_orders);
	return {std::move(total), VehicleID(id)};
}

/**
 * Send all vehicles of type to depots
 * @param flags   the flags used for DoCommand()
 * @param service should the vehicles only get service in the depots
 * @param vli     identifier of the vehicle list
 * @return 0 for success and CMD_ERROR if no vehicle is able to go to depot
 */

/**
 * Send a vehicle to the depot.
 * @param flags for command type
 * @param veh_id vehicle ID to send to the depot
 * @param depot_cmd DEPOT_ flags (see vehicle_type.h)
 * @param vli VehicleListIdentifier.
 * @return the cost of this operation or an error
 */
CommandCost CmdSendVehicleToDepot(DoCommandFlags flags, VehicleID veh_id, DepotCommandFlags depot_cmd, const VehicleListIdentifier &vli)
{
	CommandCost cost, item;
	auto list = VehicleTransactionListId(vli);
	openttd_rust_vehicle_send(&GetVehicleMassServices(), RustCommandCost(&cost), RustCommandCost(&item), flags.base(), veh_id.base(), depot_cmd.base(), &list);
	return cost;
}

/**
 * Give a custom name to your vehicle
 * @param flags type of operation
 * @param veh_id vehicle ID to name
 * @param text the new name or an empty string when resetting to the default
 * @return the cost of this operation or an error
 */
CommandCost CmdRenameVehicle(DoCommandFlags flags, VehicleID veh_id, const std::string &text)
{
	CommandCost cost;
	openttd_rust_vehicle_rename(&GetRustVehicleTransactionServices(), RustCommandCost(&cost), flags.base(), veh_id.base(), {reinterpret_cast<const uint8_t *>(text.data()), text.size()});
	return cost;
}


/**
 * Change the service interval of a vehicle
 * @param flags type of operation
 * @param veh_id vehicle ID that is being service-interval-changed
 * @param serv_int new service interval
 * @param is_custom service interval is custom flag
 * @param is_percent service interval is percentage flag
 * @return the cost of this operation or an error
 */
CommandCost CmdChangeServiceInt(DoCommandFlags flags, VehicleID veh_id, uint16_t serv_int, bool is_custom, bool is_percent)
{
	CommandCost cost;
	openttd_rust_vehicle_service_interval(&GetRustVehicleTransactionServices(), RustCommandCost(&cost), flags.base(), veh_id.base(), serv_int, is_custom, is_percent);
	return cost;
}

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDVehicleTransactionServices.hpp"
#include "abi-visit-OpenTTDVehicleCommonServices.hpp"
#include "abi-visit-OpenTTDVehicleMassServices.hpp"
#include "abi-visit-OpenTTDVehicleRefitServices.hpp"
#include "abi-visit-OpenTTDVehicleBuildServices.hpp"
#include "abi-visit-OpenTTDVehicleCloneServices.hpp"
#include "abi-visit-OpenTTDVehicleArticulatedServices.hpp"
#include "abi-visit-OpenTTDVehicleConstructServices.hpp"
bool CheckAbi_vehicle_cmd() { return AbiServicesComplete(GetRustVehicleTransactionServices()) && AbiServicesComplete(GetRustVehicleCommonServices()) && AbiServicesComplete(GetRustVehicleRefitServices()) && AbiServicesComplete(GetVehicleMassServices()) && AbiServicesComplete(GetRustVehicleBuildServices()) && AbiServicesComplete(GetRustVehicleCloneServices()) && AbiServicesComplete(GetRustVehicleArticulatedServices()) && AbiServicesComplete(GetVehicleConstructServices()); }
