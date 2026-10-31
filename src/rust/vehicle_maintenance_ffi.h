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
	OpenTTDVehicleMaintenanceRenewal (OPENTTD_RUST_CALL *renewal)(OpenTTDVehicle *, const Company *) noexcept OPENTTD_SERVICE_DEFAULT(renewal);
	OpenTTDVehicleMaintenanceReset (OPENTTD_RUST_CALL *reset)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(reset);
	bool (OPENTTD_RUST_CALL *has_engine)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(has_engine);
	OpenTTDVehicleMaintenanceInterval (OPENTTD_RUST_CALL *interval)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(interval);
	uint16_t (OPENTTD_RUST_CALL *engine_reliability)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(engine_reliability);
	OpenTTDVehicleMaintenanceBudget (OPENTTD_RUST_CALL *budget)(const Company *) noexcept OPENTTD_SERVICE_DEFAULT(budget);
	OpenTTDVehicleMaintenanceReplacement (OPENTTD_RUST_CALL *replacement)(OpenTTDVehicle *, const Company *) noexcept OPENTTD_SERVICE_DEFAULT(replacement);
	bool (OPENTTD_RUST_CALL *replacement_available)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(replacement_available);
	int64_t (*available_money)(const Company *) noexcept OPENTTD_SERVICE_DEFAULT(available_money);
	OpenTTDVehicleMaintenanceAutomatic (OPENTTD_RUST_CALL *automatic)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(automatic);
	bool (OPENTTD_RUST_CALL *depot_service)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(depot_service);
	OpenTTDVehicleMaintenanceCheckBreakdown (OPENTTD_RUST_CALL *check_breakdown)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(check_breakdown);
	OpenTTDVehicleMaintenanceBreakdownGuard (OPENTTD_RUST_CALL *breakdown_guard)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(breakdown_guard);
	OpenTTDVehicleMaintenanceBreakdownTrigger (OPENTTD_RUST_CALL *breakdown_trigger)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(breakdown_trigger);
	void (OPENTTD_RUST_CALL *chance_write)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(chance_write);
	void (OPENTTD_RUST_CALL *trigger)(OpenTTDVehicle *, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(trigger);
	uint8_t (OPENTTD_RUST_CALL *breakdown_counter)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(breakdown_counter);
	OpenTTDVehicleMaintenanceBreakdownProbe (OPENTTD_RUST_CALL *breakdown_probe)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(breakdown_probe);
	OpenTTDVehicleMaintenanceBreakdownStart (OPENTTD_RUST_CALL *breakdown_start)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(breakdown_start);
	void (OPENTTD_RUST_CALL *breakdown_start_write)(OpenTTDVehicle *, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(breakdown_start_write);
	OpenTTDVehicleMaintenanceBreakdownActive (OPENTTD_RUST_CALL *breakdown_active)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(breakdown_active);
	OpenTTDVehicleMaintenanceBreakdownCountdown (OPENTTD_RUST_CALL *breakdown_countdown)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(breakdown_countdown);
	OpenTTDVehicleMaintenanceBreakdownSoundState (OPENTTD_RUST_CALL *breakdown_sound_state)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(breakdown_sound_state);
	void (OPENTTD_RUST_CALL *breakdown_delay_write)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(breakdown_delay_write);
	bool (OPENTTD_RUST_CALL *smoke_disabled)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(smoke_disabled);
	uint8_t (OPENTTD_RUST_CALL *smoke_delay)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(smoke_delay);
	void (OPENTTD_RUST_CALL *aircraft_broken)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(aircraft_broken);
	void (OPENTTD_RUST_CALL *mark_dirty)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(mark_dirty);
	void (OPENTTD_RUST_CALL *view_dirty)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(view_dirty);
	void (OPENTTD_RUST_CALL *details_dirty)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(details_dirty);
	OpenTTDVehicleMaintenanceAge (OPENTTD_RUST_CALL *age)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(age);
	void (OPENTTD_RUST_CALL *age_write)(OpenTTDVehicle *, OpenTTDCalendarDate) noexcept OPENTTD_SERVICE_DEFAULT(age_write);
	void (OPENTTD_RUST_CALL *decay_write)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(decay_write);
	OpenTTDVehicleMaintenanceWarning (OPENTTD_RUST_CALL *warning)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(warning);
	OpenTTDVehicleMaintenanceWarningCompany (OPENTTD_RUST_CALL *warning_company)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(warning_company);
	bool (OPENTTD_RUST_CALL *engine_available_any)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(engine_available_any);
	bool (OPENTTD_RUST_CALL *replacement_active)(OpenTTDVehicle *, const Company *) noexcept OPENTTD_SERVICE_DEFAULT(replacement_active);
	void (OPENTTD_RUST_CALL *old_news)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(old_news);
	OpenTTDEconomyDate (OPENTTD_RUST_CALL *economy_age)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(economy_age);
	void (OPENTTD_RUST_CALL *economy_age_write)(OpenTTDVehicle *, OpenTTDEconomyDate) noexcept OPENTTD_SERVICE_DEFAULT(economy_age_write);
	void (OPENTTD_RUST_CALL *reached_min_age)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(reached_min_age);
	int64_t (OPENTTD_RUST_CALL *value)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(value);
	void (OPENTTD_RUST_CALL *value_write)(OpenTTDVehicle *, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(value_write);
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
