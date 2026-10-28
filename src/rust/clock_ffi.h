/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file clock_ffi.h Calendar/economy clock ownership boundary. */
#ifndef RUST_CLOCK_FFI_H
#define RUST_CLOCK_FFI_H
#include "clock_state.h"
#include "clock_timer.h"
#include "services_ffi.h"
#include "company_ffi.h"
#include "service_defaults.hpp"
#include "../openttd.h"
#include "../timer/timer_game_calendar.h"
#include "../timer/timer_game_economy.h"
struct OpenTTDNativeClockTimer;
struct OpenTTDConsoleException;
struct OpenTTDVehicle;
struct OpenTTDLiveGraph;
class LinkGraph;
using OpenTTDClockPeriod = TimerGameCalendar::TPeriod;
using OpenTTDEconomyClockPeriod = TimerGameEconomy::TPeriod;
struct OpenTTDClockServices {
	const OpenTTDSharedServices *shared;
	GameMode (*game_mode)() noexcept OPENTTD_SERVICE_DEFAULT(game_mode);
	uint16_t (*minutes_per_calendar_year)() noexcept OPENTTD_SERVICE_DEFAULT(minutes_per_calendar_year);
	bool (*using_wallclock_units)(bool) noexcept OPENTTD_SERVICE_DEFAULT(using_wallclock_units);
	uint32_t (*vehicle_id)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_id);
	void (*vehicle_shift_dates)(OpenTTDVehicle *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_shift_dates);
	LinkGraph *(*next_graph)(uint32_t *) noexcept OPENTTD_SERVICE_DEFAULT(next_graph);
	OpenTTDLiveGraph *(*graph_owner)(LinkGraph *) noexcept OPENTTD_SERVICE_DEFAULT(graph_owner);
};
struct OpenTTDIndustryBuilder;
struct OpenTTDIndustryServices;
struct OpenTTDTownLeaves;
struct OpenTTDCompanyServices;
struct OpenTTDEconomySettings;
struct OpenTTDEconomyMonth;
struct OpenTTDCompanyYear;
struct OpenTTDClockIndustry { OpenTTDIndustryBuilder *builder; const OpenTTDIndustryServices *services; };
struct OpenTTDClockTown { const OpenTTDTownLeaves *leaves; const OpenTTDSharedServices *shared; };
struct OpenTTDClockCompany {
	const OpenTTDCompanyServices *services;
	OpenTTDEconomySettings (*settings)() noexcept OPENTTD_SERVICE_DEFAULT(settings);
	OpenTTDEconomyMonth (*month)() noexcept OPENTTD_SERVICE_DEFAULT(month);
	OpenTTDCompanyYear (*year)() noexcept OPENTTD_SERVICE_DEFAULT(year);
};
const OpenTTDClockCompany &GetRustClockCompanyServices() noexcept;
OpenTTDEconomySettings RustEconomySettings() noexcept;
OpenTTDEconomyMonth RustEconomyClockMonth() noexcept;
OpenTTDCompanyYear RustCompanyClockYear() noexcept;
const OpenTTDClockServices &GetRustClockServices() noexcept;
extern "C" {
bool openttd_rust_clock_industry_daily(const void *, void *) noexcept;
bool openttd_rust_clock_industry_monthly(const void *, void *) noexcept;
bool openttd_rust_clock_engine_daily(const void *, void *) noexcept;
bool openttd_rust_clock_engine_monthly(const void *, void *) noexcept;
bool openttd_rust_clock_station_monthly(const void *, void *) noexcept;
bool openttd_rust_clock_subsidy_monthly(const void *, void *) noexcept;
bool openttd_rust_clock_town_monthly(const void *, void *) noexcept;
bool openttd_rust_clock_town_yearly(const void *, void *) noexcept;
bool openttd_rust_clock_disaster_daily(const void *, void *) noexcept;
bool openttd_rust_clock_economy_month(const void *, void *) noexcept;
bool openttd_rust_clock_economy_calendar_month(const void *, void *) noexcept;
bool openttd_rust_clock_company_yearly(const void *, void *) noexcept;
int32_t openttd_rust_clock_calendar_date();
int32_t openttd_rust_clock_calendar_year();
int32_t openttd_rust_clock_economy_date();
OpenTTDClockState *openttd_rust_clock_storage();
OpenTTDCalendarYMD openttd_rust_calendar_date_to_ymd(int32_t);
int32_t openttd_rust_calendar_ymd_to_date(int32_t, uint8_t, uint8_t);
OpenTTDCalendarYMD openttd_rust_economy_date_to_ymd(int32_t, bool);
int32_t openttd_rust_economy_ymd_to_date(int32_t, uint8_t, uint8_t, bool);
void openttd_rust_calendar_set_date(int32_t, uint16_t);
void openttd_rust_economy_set_date(int32_t, uint16_t, bool);
bool openttd_rust_calendar_elapsed(const OpenTTDClockServices *, OpenTTDConsoleException *);
bool openttd_rust_economy_elapsed(const OpenTTDClockServices *, OpenTTDConsoleException *);
bool openttd_rust_calendar_valid(OpenTTDClockPeriod);
bool openttd_rust_economy_valid(OpenTTDEconomyClockPeriod);
void openttd_rust_calendar_register(const OpenTTDNativeClockTimer *, const OpenTTDClockPeriod *);
void openttd_rust_economy_register(const OpenTTDNativeClockTimer *, const OpenTTDEconomyClockPeriod *);
void openttd_rust_calendar_unregister(const OpenTTDNativeClockTimer *, OpenTTDClockPeriod);
void openttd_rust_economy_unregister(const OpenTTDNativeClockTimer *, OpenTTDEconomyClockPeriod);
void openttd_rust_calendar_change(const OpenTTDNativeClockTimer *, OpenTTDClockPeriod *, OpenTTDClockPeriod, void (*)(bool) noexcept);
void openttd_rust_economy_change(const OpenTTDNativeClockTimer *, OpenTTDEconomyClockPeriod *, OpenTTDEconomyClockPeriod, void (*)(bool) noexcept);
void openttd_rust_calendar_bind(const OpenTTDNativeClockTimer *, OpenTTDClockPeriod, RustClockCallback, bool *);
void openttd_rust_economy_bind(const OpenTTDNativeClockTimer *, OpenTTDEconomyClockPeriod, RustClockCallback, bool *);
}
#endif
