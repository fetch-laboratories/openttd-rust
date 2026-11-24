/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file vehicle_maintenance_ffi.h The services of Rust vehicle maintenance: its tables, `Vehicle::MarkDirty` and the members still stored in the C++ shell. */
#ifndef RUST_VEHICLE_MAINTENANCE_FFI_H
#define RUST_VEHICLE_MAINTENANCE_FFI_H
#include "service_defaults.hpp"
#include "vehicle_ffi.h"
#include "services_ffi.h"
struct Company;
struct OpenTTDClockServices;
struct OpenTTDCompanyIdentityServices;
struct OpenTTDFleetGroupServices;
struct OpenTTDFleetTransactionServices;
struct OpenTTDSharedServices;
struct OpenTTDVehicleMaintenanceServices {
	const OpenTTDClockServices *clock;
	const OpenTTDCompanyIdentityServices *company;
	int32_t (OPENTTD_RUST_CALL *economy_age)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(economy_age);
	const OpenTTDFleetGroupServices *group;
	void (OPENTTD_RUST_CALL *mark_dirty)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(mark_dirty);
	void (OPENTTD_RUST_CALL *set_age)(OpenTTDVehicle *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_age);
	void (OPENTTD_RUST_CALL *set_breakdown_chance)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_breakdown_chance);
	void (OPENTTD_RUST_CALL *set_breakdown_delay)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_breakdown_delay);
	void (OPENTTD_RUST_CALL *set_economy_age)(OpenTTDVehicle *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_economy_age);
	void (OPENTTD_RUST_CALL *set_reliability_spd_dec)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_reliability_spd_dec);
	void (OPENTTD_RUST_CALL *set_value)(OpenTTDVehicle *, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(set_value);
	const OpenTTDSharedServices *shared;
	const OpenTTDFleetTransactionServices *transaction;
};
extern const OpenTTDVehicleMaintenanceServices _rust_vehicle_maintenance_services;
extern "C" {
	void openttd_rust_vehicle_age(OpenTTDVehicle *, const OpenTTDVehicleServices *);
	void openttd_rust_vehicle_economy_age(OpenTTDVehicle *, const OpenTTDVehicleServices *);
	void openttd_rust_vehicle_decrease_value(OpenTTDVehicle *, const OpenTTDVehicleServices *);
	uint8_t openttd_rust_vehicle_needs_service_abi(OpenTTDVehicle *, const OpenTTDVehicleServices *);
	extern "C++" inline bool openttd_rust_vehicle_needs_service(OpenTTDVehicle *a0, const OpenTTDVehicleServices *a1) { return openttd_rust_vehicle_needs_service_abi(a0, a1) != 0; }
	uint8_t openttd_rust_vehicle_needs_automatic_service_abi(OpenTTDVehicle *, const OpenTTDVehicleServices *);
	extern "C++" inline bool openttd_rust_vehicle_needs_automatic_service(OpenTTDVehicle *a0, const OpenTTDVehicleServices *a1) { return openttd_rust_vehicle_needs_automatic_service_abi(a0, a1) != 0; }
	uint8_t openttd_rust_vehicle_handle_breakdown_abi(OpenTTDVehicle *, const OpenTTDVehicleServices *);
	extern "C++" inline bool openttd_rust_vehicle_handle_breakdown(OpenTTDVehicle *a0, const OpenTTDVehicleServices *a1) { return openttd_rust_vehicle_handle_breakdown_abi(a0, a1) != 0; }
	uint8_t openttd_rust_vehicle_needs_renew_abi(OpenTTDVehicle *, const OpenTTDVehicleServices *, const Company *, bool);
	extern "C++" inline bool openttd_rust_vehicle_needs_renew(OpenTTDVehicle *a0, const OpenTTDVehicleServices *a1, const Company *a2, bool a3) { return openttd_rust_vehicle_needs_renew_abi(a0, a1, a2, a3) != 0; }
}
#endif /* RUST_VEHICLE_MAINTENANCE_FFI_H */
