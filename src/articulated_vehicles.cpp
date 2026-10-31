/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file articulated_vehicles.cpp Implementation of articulated vehicles. */

#include "stdafx.h"
#include "core/bitmath_func.hpp"
#include "core/random_func.hpp"
#include "train.h"
#include "roadveh.h"
#include "vehicle_func.h"
#include "engine_func.h"
#include "company_func.h"
#include "newgrf.h"

#include "table/strings.h"

#include "rust/vehicle_refit_ffi.h"
#include "rust/engine_services.h"
#include "rust/engine_capabilities_ffi.h"
#include "rust/services_cargo.h"
#include "safeguards.h"


/**
 * Determines the next articulated part to attach
 * @param index Position in chain
 * @param front_type Front engine type
 * @param front Front engine
 * @param mirrored Returns whether the part shall be flipped.
 * @return engine to add or EngineID::Invalid()
 */


/**
 * Does a NewGRF report that this should be an articulated vehicle?
 * @param engine_type The engine to check.
 * @return True iff the articulated engine callback flag is set.
 */
bool IsArticulatedEngine(EngineID engine_type)
{
	return openttd_rust_articulated_is(&GetRustEngineCapabilitiesServices(), engine_type.base());
}

/**
 * Count the number of articulated parts of an engine.
 * @param engine_type The engine to get the number of parts of.
 * @param purchase_window Whether we are in the scope of the purchase window or not, i.e. whether we cannot allocate vehicles.
 * @return The number of parts.
 */
uint CountArticulatedParts(EngineID engine_type, bool purchase_window)
{
	return openttd_rust_articulated_count(&GetRustEngineCapabilitiesServices(), engine_type.base(), purchase_window);
}


/**
 * Returns the default (non-refitted) cargo and capacity of a specific EngineID.
 * @param engine the EngineID of interest
 * @return cargo and capacity
 */

/**
 * Returns all cargoes a vehicle can carry.
 * @param engine the EngineID of interest
 * @param include_initial_cargo_type if true the default cargo type of the vehicle is included; if false only the refit_mask
 * @return bit set of CargoTypes
 */

/**
 * Get the capacity of the parts of a given engine.
 * @param engine The engine to get the capacities from.
 * @return The cargo capacities.
 */
CargoArray GetCapacityOfArticulatedParts(EngineID engine)
{
	CargoArray capacity{};
	openttd_rust_articulated_capacity(&GetRustEngineCapabilitiesServices(), engine.base(), capacity.data());
	return capacity;
}

/**
 * Get the cargo mask of the parts of a given engine.
 * @param engine The engine to get the capacities from.
 * @return The cargo mask.
 */
CargoTypes GetCargoTypesOfArticulatedParts(EngineID engine)
{
	return openttd_rust_articulated_cargoes(&GetRustEngineCapabilitiesServices(), engine.base());
}

/**
 * Checks whether any of the articulated parts is refittable
 * @param engine the first part
 * @return true if refittable
 */
bool IsArticulatedVehicleRefittable(EngineID engine)
{
	return openttd_rust_articulated_refittable(&GetRustEngineCapabilitiesServices(), engine.base());
}

/**
 * Merges the refit_masks of all articulated parts.
 * @param engine the first part
 * @param include_initial_cargo_type if true the default cargo type of the vehicle is included; if false only the refit_mask
 * @param union_mask returns bit mask of CargoTypes which are a refit option for at least one articulated part
 * @param intersection_mask returns bit mask of CargoTypes which are a refit option for every articulated part (with default capacity > 0)
 */
void GetArticulatedRefitMasks(EngineID engine, bool include_initial_cargo_type, CargoTypes *union_mask, CargoTypes *intersection_mask)
{
	openttd_rust_articulated_refit_masks(&GetRustEngineCapabilitiesServices(), engine.base(), include_initial_cargo_type, union_mask, intersection_mask);
}

/**
 * Ors the refit_masks of all articulated parts.
 * @param engine the first part
 * @param include_initial_cargo_type if true the default cargo type of the vehicle is included; if false only the refit_mask
 * @return bit mask of CargoTypes which are a refit option for at least one articulated part
 */
CargoTypes GetUnionOfArticulatedRefitMasks(EngineID engine, bool include_initial_cargo_type)
{
	return openttd_rust_articulated_refit_union(&GetRustEngineCapabilitiesServices(), engine.base(), include_initial_cargo_type);
}

/**
 * Get cargo mask of all cargoes carried by an articulated vehicle.
 * Note: Vehicles not carrying anything are ignored
 * @param v the first vehicle in the chain
 * @param cargo_type returns the common CargoType if needed. (INVALID_CARGO if no part is carrying something or they are carrying different things)
 * @return cargo mask, may be 0 if the no vehicle parts have cargo capacity
 */
CargoTypes GetCargoTypesOfArticulatedVehicle(const Vehicle *v, CargoType *cargo_type)
{
	return openttd_rust_articulated_live_cargoes(&GetRustEngineCapabilitiesServices(), v, cargo_type);
}

#include "rust/vehicle_articulated_services.hpp"

/**
 * Checks whether the specs of freshly build articulated vehicles are consistent with the information specified in the purchase list.
 * Only essential information is checked to leave room for magic tricks/workarounds to grfcoders.
 * It checks:
 *   For autoreplace/-renew:
 *    - Default cargo type (without capacity)
 *    - intersection and union of refit masks.
 */
void CheckConsistencyOfArticulatedVehicle(const Vehicle *v)
{
	openttd_rust_vehicle_articulated_consistency(&GetRustVehicleArticulatedServices(), const_cast<Vehicle *>(v));
}

/**
 * Add the remaining articulated parts to the given vehicle.
 * @param first The head of the articulated bit.
 */
void AddArticulatedParts(Vehicle *first)
{
	openttd_rust_vehicle_add_articulated(&GetRustVehicleArticulatedServices(), first);
}
