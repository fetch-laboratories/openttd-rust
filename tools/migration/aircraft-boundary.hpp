/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file aircraft-boundary.hpp Strict native fixture defaults for the direct aircraft ABI. */
#ifndef OPENTTD_AIRCRAFT_BOUNDARY_HPP
#define OPENTTD_AIRCRAFT_BOUNDARY_HPP
#include "rust/aircraft_ffi.h"
#include <cstdlib>
#include <cstdio>
/* Every function pointer is valid. A fixture explicitly supplies every service
 * it expects to use, so an unexpected crossing fails instead of silently passing. */
static OpenTTDAircraftLeaves AircraftTestLeaves(OpenTTDVehicleServices &vehicle, OpenTTDSharedServices &shared)
{
	OpenTTDAircraftLeaves leaves{};
	leaves.vehicle = &vehicle;
	leaves.shared = &shared;
	shared.context = nullptr;
	return leaves;
}
#endif /* OPENTTD_AIRCRAFT_BOUNDARY_HPP */
