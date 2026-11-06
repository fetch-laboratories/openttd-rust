/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file vehicle_maintenance_ffi.h Operation-sized observations for Rust vehicle maintenance. */
#ifndef RUST_VEHICLE_MAINTENANCE_FFI_H
#define RUST_VEHICLE_MAINTENANCE_FFI_H
#include "service_defaults.hpp"
#include "vehicle_ffi.h"
#include "services_ffi.h"
#include "unit_numbers_ffi.h"
enum OrderType : uint8_t;
struct Company;
struct OpenTTDSharedServices;
struct OpenTTDFleetTransactionServices;
struct OpenTTDFleetGroupServices;
struct OpenTTDCompanyIdentityServices;
struct OpenTTDVehicleMaintenanceReplacement {
	uint16_t engine;
	bool when_old;
};

struct OpenTTDVehicleMaintenanceServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDFleetTransactionServices *transaction;
	const OpenTTDFleetGroupServices *group;
	const OpenTTDCompanyIdentityServices *company;
	OpenTTDVehicleMaintenanceReplacement (OPENTTD_RUST_CALL *replacement)(OpenTTDVehicle *, const Company *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(replacement);
	bool (OPENTTD_RUST_CALL *replacement_available)(uint8_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(replacement_available);
	bool (OPENTTD_RUST_CALL *smoke_disabled)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(smoke_disabled);
	void (OPENTTD_RUST_CALL *mark_dirty)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(mark_dirty);
	void (OPENTTD_RUST_CALL *view_dirty)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(view_dirty);
	void (OPENTTD_RUST_CALL *details_dirty)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(details_dirty);
	bool (OPENTTD_RUST_CALL *engine_available_any)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(engine_available_any);
	bool (OPENTTD_RUST_CALL *replacement_active)(OpenTTDVehicle *, const Company *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(replacement_active);
	void (OPENTTD_RUST_CALL *old_news)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(old_news);
	void (OPENTTD_RUST_CALL *reached_min_age)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(reached_min_age);
	OpenTTDCalendarDate max_calendar_date;
	OpenTTDEconomyDate max_economy_date;
	OpenTTDEconomyDate profit_min_age;
	OpenTTDCalendarDate anniversaries[5];
	OpenTTDCalendarDate before_zero;
	uint32_t news_getting_old;
	uint32_t news_very_old;
	uint32_t news_very_old_and;
};
extern const OpenTTDVehicleMaintenanceServices _rust_vehicle_maintenance_services;
extern "C" {
	void openttd_rust_vehicle_service_depot(OpenTTDVehicle *, const OpenTTDVehicleServices *);
	void openttd_rust_vehicle_age(OpenTTDVehicle *, const OpenTTDVehicleServices *);
	void openttd_rust_vehicle_economy_age(OpenTTDVehicle *, const OpenTTDVehicleServices *);
	void openttd_rust_vehicle_decrease_value(OpenTTDVehicle *, const OpenTTDVehicleServices *);
	void openttd_rust_vehicle_check_breakdown(OpenTTDVehicle *, const OpenTTDVehicleServices *);
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
