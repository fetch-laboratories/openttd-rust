/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file settings_table_ffi.h Simulation setting callbacks and setting commands owned by Rust. */
#ifndef RUST_SETTINGS_TABLE_FFI_H
#define RUST_SETTINGS_TABLE_FFI_H
#include "byte_span_ffi.h"
#include "services_ffi.h"
#include "service_defaults.hpp"

struct OpenTTDClockServices;
struct OpenTTDCompanyServices;
struct OpenTTDVehicleTransactionServices;
struct OpenTTDScheduleServices;
struct OpenTTDEngineLoadingServices;
struct VehicleDefaultSettings;
struct SettingDesc;
struct OpenTTDCommandCost;
struct OpenTTDSettingRange { int32_t min; uint32_t max; };
struct OpenTTDSettingServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDClockServices *clock;
	const OpenTTDCompanyServices *company;
	const OpenTTDVehicleTransactionServices *transactions;
	const OpenTTDScheduleServices *schedule;
	const OpenTTDEngineLoadingServices *engines;
	OpenTTDEnvironmentServices environment;
	VehicleDefaultSettings *(*client_vehicle_defaults)() noexcept OPENTTD_SERVICE_DEFAULT(client_vehicle_defaults);
	VehicleDefaultSettings *(*company_vehicle_defaults)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(company_vehicle_defaults);
	void (*set_service_interval)(uint32_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_service_interval);
	void (*set_service_interval_percent)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_service_interval_percent);
	uint8_t (*road_acceleration_model)() noexcept OPENTTD_SERVICE_DEFAULT(road_acceleration_model);
	uint16_t (*newgame_minutes_per_year)() noexcept OPENTTD_SERVICE_DEFAULT(newgame_minutes_per_year);
	void (*set_minutes_per_year)(bool, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_minutes_per_year);
	void (*set_newgame_wallclock)() noexcept OPENTTD_SERVICE_DEFAULT(set_newgame_wallclock);
	const SettingDesc *(*setting)(OpenTTDByteSpan) noexcept OPENTTD_SERVICE_DEFAULT(setting);
	const SettingDesc *(*company_setting)(OpenTTDByteSpan) noexcept OPENTTD_SERVICE_DEFAULT(company_setting);
	bool (*setting_current)(const SettingDesc *) noexcept OPENTTD_SERVICE_DEFAULT(setting_current);
	bool (*setting_int)(const SettingDesc *) noexcept OPENTTD_SERVICE_DEFAULT(setting_int);
	bool (*setting_editable)(const SettingDesc *) noexcept OPENTTD_SERVICE_DEFAULT(setting_editable);
	void (*change_game_setting)(const SettingDesc *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(change_game_setting);
	void (*change_company_setting)(const SettingDesc *, uint8_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(change_company_setting);
	void (*window_classes_dirty)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(window_classes_dirty);
	void (*window_classes_data)(uint16_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(window_classes_data);
	void (*screen_dirty)() noexcept OPENTTD_SERVICE_DEFAULT(screen_dirty);
	void (*error_message)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(error_message);
	uint32_t edges_not_empty;
	uint32_t edges_not_water;
	uint32_t dynamic_engines_existing_vehicles;
	uint32_t too_high_mountain;
};
const OpenTTDSettingServices &GetRustSettingServices() noexcept;
extern "C" {
void openttd_rust_settings_update_consists(const OpenTTDSettingServices *);
void openttd_rust_settings_update_all_service_interval(const OpenTTDSettingServices *, int32_t);
bool openttd_rust_settings_can_update_service_interval(const OpenTTDSettingServices *, int32_t);
void openttd_rust_settings_update_service_interval(const OpenTTDSettingServices *, uint8_t, int32_t);
int32_t openttd_rust_settings_default_service_interval(const OpenTTDSettingServices *, int32_t, uint8_t);
OpenTTDSettingRange openttd_rust_settings_service_interval_range(const OpenTTDSettingServices *);
void openttd_rust_settings_train_acceleration_model_changed(const OpenTTDSettingServices *);
void openttd_rust_settings_train_slope_steepness_changed(const OpenTTDSettingServices *);
void openttd_rust_settings_road_acceleration_model_changed(const OpenTTDSettingServices *);
void openttd_rust_settings_road_slope_steepness_changed(const OpenTTDSettingServices *);
void openttd_rust_settings_difficulty_noise_change(const OpenTTDSettingServices *);
bool openttd_rust_settings_check_road_side(const OpenTTDSettingServices *);
bool openttd_rust_settings_check_freeform_edges(const OpenTTDSettingServices *, int32_t);
void openttd_rust_settings_update_freeform_edges(const OpenTTDSettingServices *, int32_t);
bool openttd_rust_settings_check_dynamic_engines(const OpenTTDSettingServices *);
bool openttd_rust_settings_check_max_height_level(const OpenTTDSettingServices *, int32_t);
void openttd_rust_settings_station_catchment_changed(const OpenTTDSettingServices *);
void openttd_rust_settings_change_timekeeping_units(const OpenTTDSettingServices *);
void openttd_rust_settings_change_minutes_per_year(const OpenTTDSettingServices *, int32_t);
OpenTTDSettingRange openttd_rust_settings_minutes_per_year_range(const OpenTTDSettingServices *);
bool openttd_rust_settings_can_change_timetable_mode(const OpenTTDSettingServices *);
void openttd_rust_cmd_change_setting(const OpenTTDSettingServices *, OpenTTDCommandCost *, uint16_t, OpenTTDByteSpan, int32_t);
void openttd_rust_cmd_change_company_setting(const OpenTTDSettingServices *, OpenTTDCommandCost *, uint16_t, OpenTTDByteSpan, int32_t);
}
#endif /* RUST_SETTINGS_TABLE_FFI_H */
