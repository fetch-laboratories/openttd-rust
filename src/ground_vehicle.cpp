/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file ground_vehicle.cpp Implementation of GroundVehicle. */

#include "stdafx.h"
#include "train.h"
#include "roadveh.h"
#include "depot_map.h"

#include "safeguards.h"


/**
 * Recalculates the cached total power of a vehicle. Should be called when the consist is changed.
 */
template <class T, VehicleType Type>
void GroundVehicle<T, Type>::PowerChanged()
{
	return openttd_rust_ground_power_changed(&GetRustGroundPhysicsServices(), this);
}

/**
 * Recalculates the cached weight of a vehicle and its parts. Should be called each time the cargo on
 * the consist changes.
 */
template <class T, VehicleType Type>
void GroundVehicle<T, Type>::CargoChanged()
{
	return openttd_rust_ground_cargo_changed(&GetRustGroundPhysicsServices(), this);
}

/**
 * Calculates the acceleration of the vehicle under its current conditions.
 * @return Current acceleration of the vehicle.
 */
template <class T, VehicleType Type>
int GroundVehicle<T, Type>::GetAcceleration() const
{
	return openttd_rust_ground_acceleration(&GetRustGroundPhysicsServices(), this);
}

/**
 * Check whether the whole vehicle chain is in the depot.
 * @return true if and only if the whole chain is in the depot.
 */
template <class T, VehicleType Type>
bool GroundVehicle<T, Type>::IsChainInDepot() const
{
	return openttd_rust_ground_chain_in_depot(&GetRustGroundPhysicsServices(), this);
}

/* Instantiation for Train */
template struct GroundVehicle<Train, VEH_TRAIN>;
/* Instantiation for RoadVehicle */
template struct GroundVehicle<RoadVehicle, VEH_ROAD>;
