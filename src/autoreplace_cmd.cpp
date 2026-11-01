/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file autoreplace_cmd.cpp Deals with autoreplace execution but not the setup */

#include "stdafx.h"
#include "company_func.h"
#include "train.h"
#include "command_func.h"
#include "engine_func.h"
#include "vehicle_func.h"
#include "autoreplace_func.h"
#include "autoreplace_gui.h"
#include "articulated_vehicles.h"
#include "core/bitmath_func.hpp"
#include "core/random_func.hpp"
#include "vehiclelist.h"
#include "road.h"
#include "ai/ai.hpp"
#include "news_func.h"
#include "strings_func.h"
#include "autoreplace_cmd.h"
#include "group_cmd.h"
#include "order_cmd.h"
#include "train_cmd.h"
#include "vehicle_cmd.h"

#include "table/strings.h"

#include "rust/vehicle_services.h"
#include "safeguards.h"

extern void ChangeVehicleViewports(VehicleID from_index, VehicleID to_index);
extern void ChangeVehicleNews(VehicleID from_index, VehicleID to_index);
extern void ChangeVehicleViewWindow(VehicleID from_index, VehicleID to_index);

/**
 * Figure out if two engines got at least one type of cargo in common (refitting if needed)
 * @param engine_a one of the EngineIDs
 * @param engine_b the other EngineID
 * @return true if they can both carry the same type of cargo (or at least one of them got no capacity at all)
 */

static void CargoCapacityRead(void *shell, OpenTTDCargoCapacityVehicle *out) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(shell);
	*out = {RustVehicleCargoOwner(v), v->cargo_cap, v->cargo_type, static_cast<uint8_t>(v->type == VEH_TRAIN), static_cast<uint8_t>(v->IsArticulatedPart())};
}
static void *CargoCapacityOtherMultiheadedPart(void *shell) noexcept
{
	return Train::From(static_cast<Vehicle *>(shell))->other_multiheaded_part;
}
const OpenTTDCargoCapacityServices &GetRustCargoCapacityServices() noexcept
{
	static const OpenTTDCargoCapacityServices services{
		.read = CargoCapacityRead,
		.next_part = RustOpaqueVehicleRead<void *, RustVehicleNext>,
		.last_engine_part = RustOpaqueVehicleRead<void *, RustVehicleLastEnginePart>,
		.other_multiheaded_part = CargoCapacityOtherMultiheadedPart,
		.cargo = CargoStorageServices(),
	};
	return services;
}

/**
 * Check the capacity of all vehicles in a chain and spread cargo if needed.
 * @param v The vehicle to check.
 * @pre You can only do this if the consist is not loading or unloading. It
 *      must not carry reserved cargo, nor cargo to be unloaded or transferred.
 */

/**
 * Transfer cargo from a single (articulated )old vehicle to the new vehicle chain
 * @param old_veh Old vehicle that will be sold
 * @param new_head Head of the completely constructed new vehicle chain
 * @param part_of_chain The vehicle is part of a train
 * @pre You can only do this if both consists are not loading or unloading.
 *      They must not carry reserved cargo, nor cargo to be unloaded or
 *      transferred.
 */
static void TransferCargo(Vehicle *old_veh, Vehicle *new_head, bool part_of_chain)
{
	const auto &services = GetRustCargoCapacityServices();
	if (openttd_rust_cargo_capacity(&services, old_veh, new_head, 1, part_of_chain) != 0) Train::From(new_head)->ConsistChanged(CCF_LOADUNLOAD);
}

/**
 * Tests whether refit orders that applied to v will also apply to the new vehicle type
 * @param v The vehicle to be replaced
 * @param engine_type The type we want to replace with
 * @return true iff all refit orders stay valid
 */
#include "rust/fleet_transaction_services.hpp"
const OpenTTDFleetTransactionServices &FleetTransactionServices() { return _fleet_transaction_services; }

bool CheckAutoreplaceValidity(EngineID from, EngineID to, CompanyID company)
{
	assert(Engine::IsValidID(from) && Engine::IsValidID(to));
	return openttd_rust_fleet_valid(&_fleet_transaction_services, &FleetGroupServices(), from.base(), to.base(), company.base());
}

CommandCost CmdAutoreplaceVehicle(DoCommandFlags flags, VehicleID veh_id)
{
	NativeFleetCosts context;
	auto costs = context.View();
	openttd_rust_fleet_autoreplace(&_fleet_transaction_services, &FleetGroupServices(), &costs, flags.base(), veh_id.base());
	return std::move(context.result);
}

/**
 * Change engine renewal parameters
 * @param flags operation to perform
 * @param id_g engine group
 * @param old_engine_type old engine type
 * @param new_engine_type new engine type
 * @param when_old replace when engine gets old?
 * @return the cost of this operation or an error
 */
CommandCost CmdSetAutoReplace(DoCommandFlags flags, GroupID id_g, EngineID old_engine_type, EngineID new_engine_type, bool when_old)
{
	uint32_t error = openttd_rust_fleet_set_rule(&_fleet_transaction_services, &FleetGroupServices(), flags.base(), id_g.base(), old_engine_type.base(), new_engine_type.base(), when_old);
	return error == UINT32_MAX ? CommandCost() : CommandCost(error);
}


#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDFleetTransactionServices.hpp"
#include "abi-visit-OpenTTDCargoCapacityServices.hpp"
bool CheckAbi_autoreplace_cmd()
{
	return AbiServicesComplete(_fleet_transaction_services) &&
		AbiServicesComplete(GetRustCargoCapacityServices());
}
