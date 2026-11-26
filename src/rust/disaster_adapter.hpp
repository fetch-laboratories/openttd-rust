/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
/** @file disaster_adapter.hpp The disaster table, the members Rust runs and the daily timer. */
#include "disaster_ffi.h"
#include "clock_ffi.h"
#include "vehicle_services.h"
#include "disaster_counter.h"
#include "services_ffi.h"
#include "company_services.h"
#include "company_identity_ffi.h"
#include "environment_services.h"

static OpenTTDVehicle *OPENTTD_RUST_CALL DisasterAllocate() noexcept { return RustVehicle(new DisasterVehicle()); }
/* At namespace scope: the scheduler's table, which this one links, links it back, and two
 * function-local statics whose initializers name each other would recurse. */
static const OpenTTDDisasterServices _disaster_services{
	.aircraft = &GetRustAircraftLeaves(),
	.allocate = DisasterAllocate,
	.environment = &GetRustEnvironmentServices(),
	.identity = &GetRustCompanyIdentityServices(),
	.industry = &GetIndustryRustServices(),
	.road = &GetRustRoadLeaves(),
	.scheduler = &GetRustVehicleSchedulerServices(),
	.shared = &GetRustSharedServices(),
	.vehicles = &GetRustVehicleServices(),
};
const OpenTTDDisasterServices &DisasterServices() noexcept { return _disaster_services; }

void ReleaseDisasterVehicle(VehicleID vehicle)
{
	DisasterVehicle *v = DisasterVehicle::GetIfValid(vehicle);
	if (v == nullptr) return;
	openttd_rust_disaster_release_vehicle(&DisasterServices(), RustVehicle(v));
}
static const IntervalTimer<TimerGameEconomy> _economy_disaster_daily({TimerGameEconomy::DAY, TimerGameEconomy::Priority::DISASTER}, RustClockCallback{openttd_rust_clock_disaster_daily, &DisasterServices()});

#include "service_table_check.hpp"
#include "abi-visit-OpenTTDDisasterServices.hpp"
bool CheckAbi_disaster_vehicle() { return AbiServicesComplete(DisasterServices()); }
