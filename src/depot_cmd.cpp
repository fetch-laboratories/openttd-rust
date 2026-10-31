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
#ifdef WITH_RUST
#include "rust/command_cost_services.h"
#include "rust/command_services_ffi.h"
#include "rust/depot_ffi.h"
#include "rust/rail_command_ffi.h"
#include "rust/services_ffi.h"
#endif

#include "table/strings.h"

#include "safeguards.h"

#ifdef WITH_RUST
static_assert(MAX_LENGTH_DEPOT_NAME_CHARS == 32);

static void RustDepotRenamed(Depot *d) noexcept
{
	/* Update the orders and depot */
	SetWindowClassesDirty(WC_VEHICLE_ORDERS);
	SetWindowDirty(WC_VEHICLE_DEPOT, d->xy);

	/* Update the depot list */
	VehicleType vt = GetDepotVehicleType(d->xy);
	SetWindowDirty(GetWindowClassForVehicleType(vt), VehicleListIdentifier(VL_DEPOT_LIST, vt, CompanyID(GetRustSharedServices().tile_owner(d->xy.base())), d->index).ToWindowNumber());
}

static const OpenTTDDepotServices &GetRustDepotServices() noexcept
{
	static const OpenTTDDepotServices services{
		.shared = &GetRustSharedServices(),
		.command = &GetRustCommandServices(),
		.costs = &GetRustCommandCostServices(),
		.rail = &GetRustRailCommandServices(),
		.depot = [](uint16_t id) noexcept { return Depot::GetIfValid(DepotID(id)); },
		.pool_size = []() noexcept { return static_cast<uint32_t>(Depot::GetPoolSize()); },
		.xy = [](const Depot *d) noexcept { return d->xy.base(); },
		.name = [](const Depot *d) noexcept { return OpenTTDByteSpan{reinterpret_cast<const uint8_t *>(d->name.data()), d->name.size()}; },
		.set_name = [](Depot *d, OpenTTDByteSpan text) noexcept { d->name.assign(reinterpret_cast<const char *>(text.data), text.size); },
		.renamed = RustDepotRenamed,
		.name_unique = STR_ERROR_NAME_MUST_BE_UNIQUE,
	};
	return services;
}
#endif

/**
 * Check whether the given name is globally unique amongst depots.
 * @param name The name to check.
 * @return True if there is no depot with the given name.
 */
#ifndef WITH_RUST
static bool IsUniqueDepotName(const std::string &name)
{
	for (const Depot *d : Depot::Iterate()) {
		if (!d->name.empty() && d->name == name) return false;
	}

	return true;
}
#endif

/**
 * Rename a depot.
 * @param flags type of operation
 * @param depot_id id of depot
 * @param text the new name or an empty string when resetting to the default
 * @return the cost of this operation or an error
 */
CommandCost CmdRenameDepot(DoCommandFlags flags, DepotID depot_id, const std::string &text)
{
#ifdef WITH_RUST
	CommandCost cost;
	openttd_rust_depot_rename(&GetRustDepotServices(), RustCommandCost(&cost), flags.base(), depot_id.base(), {reinterpret_cast<const uint8_t *>(text.data()), text.size()});
	return cost;
#else
	Depot *d = Depot::GetIfValid(depot_id);
	if (d == nullptr) return CMD_ERROR;

	CommandCost ret = CheckTileOwnership(d->xy);
	if (ret.Failed()) return ret;

	bool reset = text.empty();

	if (!reset) {
		if (Utf8StringLength(text) >= MAX_LENGTH_DEPOT_NAME_CHARS) return CMD_ERROR;
		if (!IsUniqueDepotName(text)) return CommandCost(STR_ERROR_NAME_MUST_BE_UNIQUE);
	}

	if (flags.Test(DoCommandFlag::Execute)) {
		if (reset) {
			d->name.clear();
			MakeDefaultName(d);
		} else {
			d->name = text;
		}

		/* Update the orders and depot */
		SetWindowClassesDirty(WC_VEHICLE_ORDERS);
		SetWindowDirty(WC_VEHICLE_DEPOT, d->xy);

		/* Update the depot list */
		VehicleType vt = GetDepotVehicleType(d->xy);
		SetWindowDirty(GetWindowClassForVehicleType(vt), VehicleListIdentifier(VL_DEPOT_LIST, vt, GetTileOwner(d->xy), d->index).ToWindowNumber());
	}
	return CommandCost();
#endif
}
