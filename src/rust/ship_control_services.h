/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file ship_control_services.h The ship controller's services that C++ still owns. */
#include "vehicle_services.h"

static bool Ship_next_depot(uint32_t first, OpenTTDShipDepot *out) noexcept
{
	for (const Depot *d : Depot::Iterate(first)) {
		*out = {d->index.base(), d->xy.base()};
		return true;
	}
	return false;
}

static void Ship_update_viewport(OpenTTDVehicle *handle, bool force_update, bool update_delta) noexcept { Ship::From(VehicleOf(handle))->UpdateViewport(force_update, update_delta); }

const OpenTTDShipLeaves &GetRustShipServices() noexcept
{
	static const OpenTTDShipLeaves leaves{
		.next_depot = Ship_next_depot,
		.update_viewport = Ship_update_viewport,
		.vehicle = &GetRustVehicleServices(),
	};
	return leaves;
}
