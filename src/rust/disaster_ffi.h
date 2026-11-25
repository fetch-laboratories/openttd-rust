/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
/** @file disaster_ffi.h The disaster vehicle members Rust owns, its table and its entries. */
#ifndef OPENTTD_RUST_DISASTER_FFI_H
#define OPENTTD_RUST_DISASTER_FFI_H
#include "services_ffi.h"
#include "vehicle_ffi.h"
#include "aircraft_ffi.h"
#include "road_ffi.h"
#include "call_ffi.h"
struct Company;
struct OpenTTDCompanyIdentityServices;
struct OpenTTDIndustryServices;
struct OpenTTDDisasterState {
	uint32_t image_override;
	uint32_t big_ufo_destroyer_target;
	uint16_t state;
	uint8_t flags;
};
struct OpenTTDDisasterServices {
	const OpenTTDAircraftLeaves * aircraft;
	OpenTTDVehicle *(OPENTTD_RUST_CALL *allocate)() noexcept OPENTTD_SERVICE_DEFAULT(allocate);
	const OpenTTDEnvironmentServices * environment;
	const OpenTTDCompanyIdentityServices * identity;
	const OpenTTDIndustryServices * industry;
	const OpenTTDRoadLeaves * road;
	uint16_t (OPENTTD_RUST_CALL *road_count)(const Company *) noexcept OPENTTD_SERVICE_DEFAULT(road_count);
	const OpenTTDVehicleSchedulerServices * scheduler;
	const OpenTTDSharedServices * shared;
	const OpenTTDVehicleServices * vehicles;
};
/* The table is immutable and lives for the process. An entry's record and owner are live
 * and serialized for the call; no exception crosses this boundary. */
const OpenTTDDisasterServices &DisasterServices() noexcept;
extern "C" {
OpenTTDDisasterState *OPENTTD_RUST_CALL openttd_rust_disaster_state_create();
void OPENTTD_RUST_CALL openttd_rust_disaster_state_destroy(OpenTTDDisasterState *);
uint16_t *OPENTTD_RUST_CALL openttd_rust_disaster_delay();
uint8_t OPENTTD_RUST_CALL openttd_rust_disaster_tick_abi(const OpenTTDDisasterServices *, OpenTTDVehicle *);
extern "C++" inline bool OPENTTD_RUST_CALL openttd_rust_disaster_tick(const OpenTTDDisasterServices *a0, OpenTTDVehicle *a1) { return openttd_rust_disaster_tick_abi(a0, a1) != 0; }
void OPENTTD_RUST_CALL openttd_rust_disaster_release_vehicle(const OpenTTDDisasterServices *, OpenTTDVehicle *);
}
#endif
