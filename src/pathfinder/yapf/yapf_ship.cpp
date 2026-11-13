/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file yapf_ship.cpp Implementation of YAPF for ships. */

#include "../../stdafx.h"
#include "../../ship.h"

#include "yapf.h"

#include "../../rust/ship_yapf_ffi.h"
#include "../../rust/ship_control_ffi.h"
#include "../../rust/vehicle_services.h"

#include "../../safeguards.h"

/** Ship controller helper - path finder invoker. */
Track YapfShipChooseTrack(const Ship *v, TileIndex tile, bool &path_found, ShipPathCache &path_cache)
{
	return static_cast<Track>(openttd_rust_yapf_ship_choose_track(&GetRustSharedServices(), &GetRustShipServices(), RustVehicle(v), v->GetState(), v->GetVehicleTrackdir(), tile.base(), path_cache.GetOwner(), &path_found));
}

bool YapfShipCheckReverse(const Ship *v, Trackdir *trackdir)
{
	uint8_t td = INVALID_TRACKDIR;
	const bool reverse = openttd_rust_yapf_ship_check_reverse(&GetRustSharedServices(), &GetRustShipServices(), RustVehicle(v), v->GetState(), v->GetVehicleTrackdir(), trackdir != nullptr ? &td : nullptr) != 0;
	if (trackdir != nullptr) *trackdir = static_cast<Trackdir>(td);
	return reverse;
}
