/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file depot.cpp Handling of depots. */

#include "stdafx.h"
#include "depot_base.h"
#include "order_backup.h"
#include "order_func.h"
#include "window_func.h"
#include "core/pool_func.hpp"
#include "vehicle_gui.h"
#include "vehiclelist.h"

#include "safeguards.h"

/** All our depots tucked away in a pool. */
DepotPool _depot_pool("Depot");


/**
 * Clean up a depot
 */
Depot::~Depot()
{
	if (CleaningPool()) return;

	if (!IsDepotTile(this->Record().xy) || GetDepotIndex(this->Record().xy) != this->index) {
		/* It can happen there is no depot here anymore (TTO/TTD savegames) */
		return;
	}

	/* Clear the order backup. */
	OrderBackup::Reset(this->Record().xy, false);

	/* Clear the depot from all order-lists */
	RemoveOrderFromAllVehicles(OT_GOTO_DEPOT, this->index);

	/* Delete the depot-window */
	CloseWindowById(WC_VEHICLE_DEPOT, this->Record().xy);

	/* Delete the depot list */
	VehicleType vt = GetDepotVehicleType(this->Record().xy);
	CloseWindowById(GetWindowClassForVehicleType(vt), VehicleListIdentifier(VL_DEPOT_LIST, vt, GetTileOwner(this->Record().xy), this->index).ToWindowNumber());
}
