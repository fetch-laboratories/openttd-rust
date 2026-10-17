/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file vehicle_maintenance_ffi.h Operation-sized observations for Rust vehicle maintenance. */
#ifndef RUST_VEHICLE_MAINTENANCE_FFI_H
#define RUST_VEHICLE_MAINTENANCE_FFI_H
#include "vehicle_ffi.h"
#include "services_ffi.h"
enum OrderType : uint8_t;
struct Company;
struct OpenTTDSharedServices;
struct OpenTTDFleetTransactionServices;
struct OpenTTDFleetGroupServices;
struct OpenTTDVehicleMaintenanceRenewal {
	OpenTTDCalendarDate age;
	OpenTTDCalendarDate max_age;
	int16_t months;
	bool enabled;
	VehicleType kind;
	uint8_t subtype;
};
struct OpenTTDVehicleMaintenanceReset {
	OpenTTDEconomyDate economy_date;
	OpenTTDCalendarDate calendar_date;
	uint16_t reliability;
	uint8_t chance;
	uint8_t breakdowns;
};
struct OpenTTDVehicleMaintenanceResetWrite {
	OpenTTDEconomyDate economy_date;
	OpenTTDCalendarDate calendar_date;
	uint16_t reliability;
	uint8_t chance;
	uint8_t since;
};
struct OpenTTDVehicleMaintenanceInterval {
	const Company *company;
	OpenTTDEconomyDate last_service;
	OpenTTDEconomyDate now;
	uint16_t interval;
	uint16_t reliability;
	bool percent;
	bool wallclock;
	bool no_service;
	uint8_t breakdowns;
};
struct OpenTTDVehicleMaintenanceReplacement {
	uint16_t engine;
	bool when_old;
};
struct OpenTTDVehicleMaintenanceBudget {
	int64_t needed;
	int64_t available;
};
struct OpenTTDVehicleMaintenanceReplacementCost {
	int64_t cost;
	int64_t available;
};
struct OpenTTDVehicleMaintenanceAutomatic {
	bool has_depot_order;
	OrderType order_type;
};
struct OpenTTDVehicleMaintenanceCheckBreakdown {
	uint16_t reliability;
	uint16_t decay;
	bool no_service;
	uint8_t breakdowns;
};
struct OpenTTDVehicleMaintenanceBreakdownGuard {
	uint16_t speed;
	uint8_t counter;
	uint8_t status;
	uint8_t breakdowns;
	bool menu;
};
struct OpenTTDVehicleMaintenanceBreakdownTrigger {
	uint16_t reliability;
	uint8_t chance;
	VehicleType kind;
	uint8_t breakdowns;
};
struct OpenTTDVehicleMaintenanceBreakdownProbe {
	uint8_t counter;
	uint8_t status;
};
struct OpenTTDVehicleMaintenanceBreakdownStart {
	uint8_t since;
	VehicleType kind;
};
struct OpenTTDVehicleMaintenanceBreakdownActive {
	VehicleType kind;
	uint8_t tick;
	uint8_t delay;
};
struct OpenTTDVehicleMaintenanceBreakdownCountdown {
	OrderType order_type;
	uint8_t counter;
};
struct OpenTTDVehicleMaintenanceBreakdownSoundState {
	VehicleType kind;
	LandscapeType landscape;
};
struct OpenTTDVehicleMaintenanceBreakdownSmoke {
	OpenTTDVehicle *shell;
	uint8_t delay;
};
struct OpenTTDVehicleMaintenanceAge {
	OpenTTDCalendarDate age;
	OpenTTDCalendarDate max_age;
	uint16_t decay;
};
struct OpenTTDVehicleMaintenanceWarning {
	bool enabled;
	bool has_previous;
	bool local;
	uint8_t status;
};
struct OpenTTDVehicleMaintenanceWarningCompany {
	const Company *company;
	bool renewal;
};
struct OpenTTDVehicleMaintenanceServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDFleetTransactionServices *transaction;
	const OpenTTDFleetGroupServices *group;
	OpenTTDVehicleMaintenanceRenewal (OPENTTD_VEHICLE_CALL *renewal)(OpenTTDVehicle *, const Company *) noexcept;
	OpenTTDVehicleMaintenanceReset (OPENTTD_VEHICLE_CALL *reset)(OpenTTDVehicle *) noexcept;
	void (OPENTTD_VEHICLE_CALL *reset_write)(OpenTTDVehicle *, OpenTTDVehicleMaintenanceResetWrite) noexcept;
	bool (OPENTTD_VEHICLE_CALL *has_engine)(OpenTTDVehicle *) noexcept;
	OpenTTDVehicleMaintenanceInterval (OPENTTD_VEHICLE_CALL *interval)(OpenTTDVehicle *) noexcept;
	uint16_t (OPENTTD_VEHICLE_CALL *engine_reliability)(OpenTTDVehicle *) noexcept;
	OpenTTDVehicleMaintenanceBudget (OPENTTD_VEHICLE_CALL *budget)(const Company *) noexcept;
	OpenTTDVehicleMaintenanceReplacement (OPENTTD_VEHICLE_CALL *replacement)(OpenTTDVehicle *, const Company *) noexcept;
	bool (OPENTTD_VEHICLE_CALL *replacement_available)(OpenTTDVehicle *, uint16_t) noexcept;
	OpenTTDVehicleMaintenanceReplacementCost (OPENTTD_VEHICLE_CALL *replacement_cost)(const Company *, uint16_t) noexcept;
	OpenTTDVehicleMaintenanceAutomatic (OPENTTD_VEHICLE_CALL *automatic)(OpenTTDVehicle *) noexcept;
	bool (OPENTTD_VEHICLE_CALL *depot_service)(OpenTTDVehicle *) noexcept;
	OpenTTDVehicleMaintenanceCheckBreakdown (OPENTTD_VEHICLE_CALL *check_breakdown)(OpenTTDVehicle *) noexcept;
	OpenTTDVehicleMaintenanceBreakdownGuard (OPENTTD_VEHICLE_CALL *breakdown_guard)(OpenTTDVehicle *) noexcept;
	OpenTTDVehicleMaintenanceBreakdownTrigger (OPENTTD_VEHICLE_CALL *breakdown_trigger)(OpenTTDVehicle *) noexcept;
	void (OPENTTD_VEHICLE_CALL *chance_write)(OpenTTDVehicle *, uint8_t) noexcept;
	void (OPENTTD_VEHICLE_CALL *trigger)(OpenTTDVehicle *, uint8_t, uint8_t) noexcept;
	uint8_t (OPENTTD_VEHICLE_CALL *breakdown_counter)(OpenTTDVehicle *) noexcept;
	OpenTTDVehicleMaintenanceBreakdownProbe (OPENTTD_VEHICLE_CALL *breakdown_probe)(OpenTTDVehicle *) noexcept;
	OpenTTDVehicleMaintenanceBreakdownStart (OPENTTD_VEHICLE_CALL *breakdown_start)(OpenTTDVehicle *) noexcept;
	void (OPENTTD_VEHICLE_CALL *breakdown_start_write)(OpenTTDVehicle *, uint8_t, uint8_t) noexcept;
	OpenTTDVehicleMaintenanceBreakdownActive (OPENTTD_VEHICLE_CALL *breakdown_active)(OpenTTDVehicle *) noexcept;
	OpenTTDVehicleMaintenanceBreakdownCountdown (OPENTTD_VEHICLE_CALL *breakdown_countdown)(OpenTTDVehicle *) noexcept;
	OpenTTDVehicleMaintenanceBreakdownSoundState (OPENTTD_VEHICLE_CALL *breakdown_sound_state)(OpenTTDVehicle *) noexcept;
	void (OPENTTD_VEHICLE_CALL *breakdown_delay_write)(OpenTTDVehicle *, uint8_t) noexcept;
	bool (OPENTTD_VEHICLE_CALL *breakdown_sound)(OpenTTDVehicle *) noexcept;
	bool (OPENTTD_VEHICLE_CALL *smoke_disabled)(OpenTTDVehicle *) noexcept;
	OpenTTDVehicleMaintenanceBreakdownSmoke (OPENTTD_VEHICLE_CALL *smoke)(OpenTTDVehicle *) noexcept;
	void (OPENTTD_VEHICLE_CALL *smoke_animation)(OpenTTDVehicle *, uint16_t) noexcept;
	void (OPENTTD_VEHICLE_CALL *aircraft_broken)(OpenTTDVehicle *) noexcept;
	void (OPENTTD_VEHICLE_CALL *mark_dirty)(OpenTTDVehicle *) noexcept;
	void (OPENTTD_VEHICLE_CALL *view_dirty)(OpenTTDVehicle *) noexcept;
	void (OPENTTD_VEHICLE_CALL *details_dirty)(OpenTTDVehicle *) noexcept;
	OpenTTDVehicleMaintenanceAge (OPENTTD_VEHICLE_CALL *age)(OpenTTDVehicle *) noexcept;
	void (OPENTTD_VEHICLE_CALL *age_write)(OpenTTDVehicle *, OpenTTDCalendarDate) noexcept;
	void (OPENTTD_VEHICLE_CALL *decay_write)(OpenTTDVehicle *, uint16_t) noexcept;
	OpenTTDVehicleMaintenanceWarning (OPENTTD_VEHICLE_CALL *warning)(OpenTTDVehicle *) noexcept;
	OpenTTDVehicleMaintenanceWarningCompany (OPENTTD_VEHICLE_CALL *warning_company)(OpenTTDVehicle *) noexcept;
	bool (OPENTTD_VEHICLE_CALL *engine_available_any)(OpenTTDVehicle *) noexcept;
	bool (OPENTTD_VEHICLE_CALL *replacement_active)(OpenTTDVehicle *, const Company *) noexcept;
	void (OPENTTD_VEHICLE_CALL *old_news)(OpenTTDVehicle *, uint32_t) noexcept;
	OpenTTDEconomyDate (OPENTTD_VEHICLE_CALL *economy_age)(OpenTTDVehicle *) noexcept;
	void (OPENTTD_VEHICLE_CALL *economy_age_write)(OpenTTDVehicle *, OpenTTDEconomyDate) noexcept;
	void (OPENTTD_VEHICLE_CALL *reached_min_age)(OpenTTDVehicle *) noexcept;
	int64_t (OPENTTD_VEHICLE_CALL *value)(OpenTTDVehicle *) noexcept;
	void (OPENTTD_VEHICLE_CALL *value_write)(OpenTTDVehicle *, int64_t) noexcept;
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
	bool openttd_rust_vehicle_needs_service(OpenTTDVehicle *, const OpenTTDVehicleServices *);
	bool openttd_rust_vehicle_needs_automatic_service(OpenTTDVehicle *, const OpenTTDVehicleServices *);
	bool openttd_rust_vehicle_handle_breakdown(OpenTTDVehicle *, const OpenTTDVehicleServices *);
	bool openttd_rust_vehicle_needs_renew(OpenTTDVehicle *, const OpenTTDVehicleServices *, const Company *, bool);
}
#endif /* RUST_VEHICLE_MAINTENANCE_FFI_H */
