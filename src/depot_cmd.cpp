/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file depot_cmd.cpp %Command Handling for depots. */

#include "stdafx.h"
#include "command_func.h"
#include "depot_base.h"
#include "company_func.h"
#include "string_func.h"
#include "town.h"
#include "vehicle_gui.h"
#include "vehiclelist.h"
#include "window_func.h"
#include "depot_cmd.h"
#include "rust/command_cost_services.h"
#include "rust/command_services_ffi.h"
#include "rust/depot_ffi.h"
#include "rust/rail_command_ffi.h"
#include "rust/services_ffi.h"
#include "rust/station_command_ffi.h"
#include "rust/vehicle_refit_ffi.h"

#include "table/strings.h"

#include "safeguards.h"

static_assert(MAX_LENGTH_DEPOT_NAME_CHARS == 32);

static void RustDepotRenamed(Depot *d) noexcept
{
	/* Update the orders and depot */
	GetRustStationCommandServices().orders_dirty();
	SetWindowDirty(WC_VEHICLE_DEPOT, d->Record().xy);

	/* Update the depot list */
	VehicleType vt = GetRustVehicleBuildServices().depot_kind(d->Record().xy.base());
	SetWindowDirty(GetWindowClassForVehicleType(vt), VehicleListIdentifier(VL_DEPOT_LIST, vt, CompanyID(GetRustSharedServices().tile_owner(d->Record().xy.base())), d->index).ToWindowNumber());
}

static const OpenTTDDepotServices &GetRustDepotServices() noexcept
{
	static const OpenTTDDepotServices services{
		.shared = &GetRustSharedServices(),
		.command = &GetRustCommandServices(),
		.costs = &GetRustCommandCostServices(),
		.renamed = RustDepotRenamed,
		.name_unique = STR_ERROR_NAME_MUST_BE_UNIQUE,
	};
	return services;
}


/**
 * Rename a depot.
 * @param flags type of operation
 * @param depot_id id of depot
 * @param text the new name or an empty string when resetting to the default
 * @return the cost of this operation or an error
 */
CommandCost CmdRenameDepot(DoCommandFlags flags, DepotID depot_id, const std::string &text)
{
	CommandCost cost;
	openttd_rust_depot_rename(&GetRustDepotServices(), RustCommandCost(&cost), flags.base(), depot_id.base(), {reinterpret_cast<const uint8_t *>(text.data()), text.size()});
	return cost;
}
