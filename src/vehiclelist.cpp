/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file vehiclelist.cpp Lists of vehicles. */

#include "stdafx.h"
#include "train.h"
#include "vehicle_func.h"
#include "vehiclelist.h"
#include "vehiclelist_func.h"
#include "group.h"

#include "rust/vehicle_transaction_ffi.h"
#include "rust/vehicle_services.h"
static void SelectionWithOrders(void *ctx, bool (*vehicle)(void *, const Vehicle *), bool (*order)(void *, const Order *), void (*visit)(void *, const Vehicle *)) noexcept
{
	FindVehiclesWithOrder([&](const Vehicle *v) { return vehicle(ctx, v); }, [&](const Order *o) { return order(ctx, o); }, [&](const Vehicle *v) { visit(ctx, v); });
}
static void SelectionListClear(void *list) noexcept { static_cast<VehicleList *>(list)->clear(); }
static void SelectionListPush(void *list, void *v) noexcept { static_cast<VehicleList *>(list)->push_back(static_cast<Vehicle *>(v)); }
const OpenTTDVehicleSelectionServices &GetRustVehicleSelectionServices() noexcept
{
	static const OpenTTDVehicleSelectionServices services{
		.groups = &FleetGroupServices(),
		.transactions = &FleetTransactionServices(),
		.vehicles = &GetRustVehicleServices(),
		.in_depot = RustOpaqueVehicleRead<bool, RustVehicleInDepot>,
		.free_wagon = RustOpaqueVehicleRead<bool, RustTrainFreeWagon>,
		.first_engine_part = RustOpaqueVehicleRead<void *, RustTrainFirstEnginePart>,
		.multiheaded = RustOpaqueVehicleRead<bool, RustTrainMultiheaded>,
		.other_multihead = RustOpaqueVehicleRead<void *, RustTrainOtherMultihead>,
		.with_orders = SelectionWithOrders,
		.list_clear = SelectionListClear,
		.list_push = SelectionListPush,
	};
	return services;
}

#include "safeguards.h"

/**
 * Pack a VehicleListIdentifier in 32 bits so it can be used as unique WindowNumber.
 * @return The window number.
 */
WindowNumber VehicleListIdentifier::ToWindowNumber() const
{
	uint8_t c = this->company == OWNER_NONE ? 0xF : this->company.base();
	assert(c             < (1 <<  4));
	assert(this->vtype   < (1 <<  2));
	assert(this->index   < (1 << 20));
	assert(this->type    < VLT_END);
	static_assert(VLT_END <= (1 <<  3));

	return c << 28 | this->type << 23 | this->vtype << 26 | this->index;
}

/**
 * Generate a list of vehicles inside a depot.
 * @param type    Type of vehicle
 * @param tile    The tile the depot is located on
 * @param engines Pointer to list to add vehicles to
 * @param wagons  Pointer to list to add wagons to (can be nullptr)
 * @param individual_wagons If true add every wagon to \a wagons which is not attached to an engine. If false only add the first wagon of every row.
 */
void BuildDepotVehicleList(VehicleType type, TileIndex tile, VehicleList *engines, VehicleList *wagons, bool individual_wagons)
{
	openttd_rust_vehicle_depot_list(&GetRustVehicleSelectionServices(), type, tile.base(), engines, wagons, individual_wagons);
}

/**
 * Generate a list of vehicles based on window type.
 * @param list Pointer to list to add vehicles to
 * @param vli  The identifier of this vehicle list.
 * @return false if invalid list is requested
 */
bool GenerateVehicleSortList(VehicleList *list, const VehicleListIdentifier &vli)
{
	return openttd_rust_vehicle_generate_list(&GetRustVehicleSelectionServices(), list, vli.type, vli.vtype, vli.company.base(), vli.index);
}

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDVehicleSelectionServices.hpp"
bool CheckAbi_vehiclelist() { return AbiServicesComplete(GetRustVehicleSelectionServices()); }
