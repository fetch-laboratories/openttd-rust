/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
/** @file company_ffi.h Canonical company finances and economy control. */
#ifndef RUST_COMPANY_FFI_H
#define RUST_COMPANY_FFI_H
#include <cstddef>
#include <cstdint>
#include "services_ffi.h"
#if defined(_MSC_VER)
#define OPENTTD_COMPANY_CALL __cdecl
#elif defined(__i386__)
#define OPENTTD_COMPANY_CALL __attribute__((cdecl))
#else
#define OPENTTD_COMPANY_CALL
#endif
struct CompanyFinances;
extern "C" {
/* Company IDs, owners, vehicle types, tracks and expenses cross as uint8_t;
 * pool indices and tiles as uint32_t (UINT32_MAX ends an iteration); Money as
 * int64_t. Layouts are checked by ABI 210-214 and 400-417. */
struct OpenTTDCompanyRef { CompanyFinances *finances; uint32_t id; };
struct OpenTTDCompanyVehicleCounts { uint16_t trains, road, ships, aircraft; };
struct OpenTTDCompanyInfrastructure {
	uint32_t rail[64], road[63];
	uint32_t rail_total, road_total, tram_total, signal, water, station;
	uint64_t road_is_road; ///< Bit rt set when RoadTypeIsRoad(rt).
};
struct OpenTTDCompanyVehicleGroupFlags { bool engine_countable, primary; };
struct OpenTTDCompanyVehicleProfit { int64_t profit_last_year; int32_t economy_age; };
struct OpenTTDCompanyStationTimes { uint8_t since_load, since_unload; };
struct OpenTTDCompanyTownRights { int16_t rating; uint16_t have_ratings; uint8_t exclusive_counter, exclusivity; };
struct OpenTTDCompanySignalTile { bool railway, crossing; uint8_t owner; bool has_signals; uint8_t tracks; };
struct OpenTTDCompanyPriceBase { int64_t start_price; uint8_t category; };
/** error: 0 ok, 1 CMD_ERROR, 2 max loan (param), 3 repaid, 4 currency required (param), 5 insufficient funds, 6 too many vehicles. */
struct OpenTTDCompanyCost { int64_t cost, param; uint8_t error, expense; };
/* Caller-owned stack continuations. They exist only where AI::StartNew can raise
 * a script memory-policy Script_FatalError (AIInstance::Initialize registers the
 * API and loads compatibility scripts before ScriptInstance::Initialize's catch).
 * Rust returns before that call; if it throws, the frame is abandoned. */
struct OpenTTDCompanyStartup { uint32_t company; uint8_t requested; bool is_ai; uint8_t stage; };
struct OpenTTDCompanyControl {
	OpenTTDCompanyCost result;
	OpenTTDCompanyStartup startup;
	uint32_t client;
	uint8_t action, target, reason;
	bool execute;
	uint8_t stage;
};
struct OpenTTDCompanyTick {
	CompanyFinances *finances; ///< Company _cur_company_tick_index, or null.
	size_t num_companies;
	int32_t index, timeout;
	uint16_t interval;
	bool editor, named, competitor_due, networking;
	uint8_t max_companies, max_competitors, num_ais, stage;
};
struct OpenTTDCompanyCompetitors {
	size_t num_companies;
	uint16_t interval;
	bool menu, can_start, networking;
	uint8_t max_companies, max_competitors;
};
struct OpenTTDEconomyMonth { uint8_t month; bool infinite_money, maintenance, fluctuating; };
struct OpenTTDCompanyYear { CompanyFinances *local_finances; uint8_t local; bool show_finances, new_year_sound; };
struct OpenTTDEconomySettings { int32_t year; uint32_t max_loan; uint8_t initial_interest, vehicle_costs, construction_cost; bool inflation; };
struct OpenTTDCompanyLandscaping {
	uint32_t terraform_per_64k, clear_per_64k, tree_per_64k, object_per_64k;
	uint16_t terraform_burst, clear_burst, tree_burst, object_burst;
};
#define OPENTTD_COMPANY_SERVICE(ret, name, ...) ret (OPENTTD_COMPANY_CALL *name)(__VA_ARGS__) noexcept
/** Finance services used by money paths and financial commands. */
struct OpenTTDCompanyFinanceServices {
	OPENTTD_COMPANY_SERVICE(uint8_t, current_company);
	OPENTTD_COMPANY_SERVICE(void, set_current_company, uint8_t);
	OPENTTD_COMPANY_SERVICE(bool, networking);
	OPENTTD_COMPANY_SERVICE(CompanyFinances *, company_finances, uint8_t); ///< Null for an invalid ID.
	OPENTTD_COMPANY_SERVICE(void, invalidate_company_windows, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, give_money_message, uint8_t, int64_t);
	OPENTTD_COMPANY_SERVICE(void, money_animation, uint32_t, int64_t);
};
/**
 * Lifecycle, economy and ownership-transfer services. Every service is a typed
 * noexcept C++ call; environmental failures terminate inside it. Services that
 * reenter or delete (post_company_delete, change_tile_owner, delete_*,
 * allocate_company, reset_service_interval, stop_ai) run after Rust has ended
 * every raw field access it may invalidate. *_next_owned scans a pool from an
 * index for the next item of that owner, in pool order.
 */
struct OpenTTDCompanyServices {
	const OpenTTDCompanyFinanceServices *finance;
	const OpenTTDSharedServices *shared;
	OPENTTD_COMPANY_SERVICE(OpenTTDCompanyRef, company_next, uint32_t);
	OPENTTD_COMPANY_SERVICE(bool, company_is_ai, uint8_t);
	OPENTTD_COMPANY_SERVICE(size_t, company_count);
	OPENTTD_COMPANY_SERVICE(bool, can_allocate_company);
	OPENTTD_COMPANY_SERVICE(uint8_t, local_company);
	OPENTTD_COMPANY_SERVICE(bool, network_server);
	OPENTTD_COMPANY_SERVICE(OpenTTDCompanyVehicleCounts, company_vehicle_counts, uint8_t);
	OPENTTD_COMPANY_SERVICE(OpenTTDCompanyVehicleCounts, vehicle_limits);
	OPENTTD_COMPANY_SERVICE(void, company_admin_update, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, company_in_trouble, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, post_company_delete, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, update_company_hq, uint8_t, int32_t);
	OPENTTD_COMPANY_SERVICE(void, performance_detail_dirty);
	OPENTTD_COMPANY_SERVICE(void, company_graphs_dirty);
	OPENTTD_COMPANY_SERVICE(void, company_infrastructure, uint8_t, OpenTTDCompanyInfrastructure *);
	OPENTTD_COMPANY_SERVICE(int64_t, rail_maintenance_cost, uint8_t, uint32_t, uint32_t);
	OPENTTD_COMPANY_SERVICE(int64_t, signal_maintenance_cost, uint32_t);
	OPENTTD_COMPANY_SERVICE(int64_t, road_maintenance_cost, uint8_t, uint32_t, uint32_t);
	OPENTTD_COMPANY_SERVICE(int64_t, canal_maintenance_cost, uint32_t);
	OPENTTD_COMPANY_SERVICE(int64_t, station_maintenance_cost, uint32_t);
	OPENTTD_COMPANY_SERVICE(int64_t, airport_maintenance_cost, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, recession_news, bool);
	OPENTTD_COMPANY_SERVICE(OpenTTDCompanyPriceBase, price_base, uint32_t);
	OPENTTD_COMPANY_SERVICE(uint32_t, cargo_next, uint32_t, int64_t *);
	OPENTTD_COMPANY_SERVICE(void, set_cargo_payment, uint32_t, int64_t);
	OPENTTD_COMPANY_SERVICE(void, price_windows_dirty);
	OPENTTD_COMPANY_SERVICE(void, industry_daily_changes, bool);
	OPENTTD_COMPANY_SERVICE(void, clear_cargo_monitors, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, clear_all_cargo_monitors);
	OPENTTD_COMPANY_SERVICE(void, generate_company_name, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, ask_merger, uint8_t, uint8_t, int64_t);
	OPENTTD_COMPANY_SERVICE(bool, is_interactive_company, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, show_buy_company, uint8_t);
	OPENTTD_COMPANY_SERVICE(uint32_t, script_random_next, uint32_t);
	OPENTTD_COMPANY_SERVICE(void, reset_competitor_timeout, uint32_t);
	OPENTTD_COMPANY_SERVICE(void, finances_dirty, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, show_company_finances, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, new_year_sound, bool);
	OPENTTD_COMPANY_SERVICE(uint8_t, generate_company_colour);
	OPENTTD_COMPANY_SERVICE(OpenTTDCompanyRef, allocate_company, uint8_t, bool);
	OPENTTD_COMPANY_SERVICE(void, set_company_colour, uint8_t, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, setup_new_company, uint8_t, bool);
	OPENTTD_COMPANY_SERVICE(void, new_company_events, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, company_league_dirty);
	OPENTTD_COMPANY_SERVICE(void, company_ctrl_windows);
	OPENTTD_COMPANY_SERVICE(void, close_network_status);
	OPENTTD_COMPANY_SERVICE(void, network_spectate, uint32_t);
	OPENTTD_COMPANY_SERVICE(void, network_company_new, uint8_t, uint32_t, bool);
	OPENTTD_COMPANY_SERVICE(void, network_own_company, uint8_t, uint32_t);
	OPENTTD_COMPANY_SERVICE(void, assert_new_ai_slot, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, company_bankrupt_news, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, stop_ai, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, delete_company, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, company_removed, uint8_t, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, merger_news, uint8_t, bool);
	OPENTTD_COMPANY_SERVICE(void, acquisition_windows, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, clients_to_spectators, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, set_local_company, uint8_t);
	OPENTTD_COMPANY_SERVICE(uint32_t, subsidy_next_awarded, uint8_t, uint32_t);
	OPENTTD_COMPANY_SERVICE(void, delete_subsidy, uint32_t);
	OPENTTD_COMPANY_SERVICE(void, set_subsidy_awarded, uint32_t, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, rebuild_subsidy_cache);
	OPENTTD_COMPANY_SERVICE(uint32_t, town_next, uint32_t, uint8_t, OpenTTDCompanyTownRights *);
	OPENTTD_COMPANY_SERVICE(int16_t, town_rating, uint32_t, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, set_town_rating, uint32_t, uint8_t, int16_t, bool);
	OPENTTD_COMPANY_SERVICE(void, set_town_exclusivity, uint32_t, uint8_t, uint8_t);
	OPENTTD_COMPANY_SERVICE(uint32_t, vehicle_next_owned, uint8_t, uint32_t, uint8_t *);
	OPENTTD_COMPANY_SERVICE(bool, aircraft_is_normal, uint32_t);
	OPENTTD_COMPANY_SERVICE(int64_t, vehicle_value, uint32_t);
	OPENTTD_COMPANY_SERVICE(bool, vehicle_is_primary, uint32_t);
	OPENTTD_COMPANY_SERVICE(OpenTTDCompanyVehicleProfit, vehicle_profit, uint32_t);
	OPENTTD_COMPANY_SERVICE(uint32_t, vehicle_previous, uint32_t);
	OPENTTD_COMPANY_SERVICE(OpenTTDCompanyVehicleGroupFlags, vehicle_group_flags, uint32_t);
	OPENTTD_COMPANY_SERVICE(bool, vehicle_service_interval_is_custom, uint32_t);
	OPENTTD_COMPANY_SERVICE(void, delete_vehicle, uint32_t);
	OPENTTD_COMPANY_SERVICE(void, count_group_engine, uint32_t, int32_t);
	OPENTTD_COMPANY_SERVICE(void, count_group_vehicle, uint32_t, int32_t);
	OPENTTD_COMPANY_SERVICE(void, reset_service_interval, uint32_t, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, set_vehicle_owner, uint32_t, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, assign_unit_number, uint32_t, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, remove_engine_replacements, uint8_t);
	OPENTTD_COMPANY_SERVICE(uint32_t, group_next_owned, uint8_t, uint32_t);
	OPENTTD_COMPANY_SERVICE(void, delete_group, uint32_t);
	OPENTTD_COMPANY_SERVICE(void, transfer_group, uint32_t, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, copy_service_interval_defaults, uint8_t, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, update_autoreplace, uint8_t);
	OPENTTD_COMPANY_SERVICE(uint32_t, map_size);
	OPENTTD_COMPANY_SERVICE(void, change_tile_owner, uint32_t, uint8_t, uint8_t);
	OPENTTD_COMPANY_SERVICE(OpenTTDCompanySignalTile, signal_tile, uint32_t);
	OPENTTD_COMPANY_SERVICE(bool, has_signal_on_track, uint32_t, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, add_track_to_signal_buffer, uint32_t, uint8_t, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, update_signals_in_buffer);
	OPENTTD_COMPANY_SERVICE(void, add_airport_infrastructure, uint8_t, uint8_t);
	OPENTTD_COMPANY_SERVICE(uint32_t, station_next_owned, uint8_t, uint32_t);
	OPENTTD_COMPANY_SERVICE(uint32_t, station_facility_count, uint32_t);
	OPENTTD_COMPANY_SERVICE(OpenTTDCompanyStationTimes, station_times, uint32_t);
	OPENTTD_COMPANY_SERVICE(void, set_station_owner, uint32_t, uint8_t);
	OPENTTD_COMPANY_SERVICE(uint32_t, waypoint_next_owned, uint8_t, uint32_t);
	OPENTTD_COMPANY_SERVICE(void, set_waypoint_owner, uint32_t, uint8_t);
	OPENTTD_COMPANY_SERVICE(uint32_t, sign_next_owned, uint8_t, uint32_t);
	OPENTTD_COMPANY_SERVICE(void, set_sign_owner, uint32_t, uint8_t);
	OPENTTD_COMPANY_SERVICE(uint32_t, goal_next_owned, uint8_t, uint32_t);
	OPENTTD_COMPANY_SERVICE(void, delete_goal, uint32_t);
	OPENTTD_COMPANY_SERVICE(uint32_t, story_page_next_owned, uint8_t, uint32_t);
	OPENTTD_COMPANY_SERVICE(void, delete_story_page, uint32_t);
	OPENTTD_COMPANY_SERVICE(void, change_window_owner, uint8_t, uint8_t);
	OPENTTD_COMPANY_SERVICE(void, mark_whole_screen_dirty);
};
#undef OPENTTD_COMPANY_SERVICE
/* All entry points run serially on the game thread with borrowed static tables.
 * Rust owns the finance allocations; C++ starts the field objects in that raw
 * storage and keeps stable field addresses for existing readers and save
 * adapters. Rust accesses only raw fields, never across a service that may
 * invalidate them. Save/load errors run entirely outside Rust. Rust panics abort. */
CompanyFinances *OPENTTD_COMPANY_CALL openttd_rust_company_state_create();
void OPENTTD_COMPANY_CALL openttd_rust_company_state_destroy(CompanyFinances *);
void OPENTTD_COMPANY_CALL openttd_rust_company_initialize(CompanyFinances *, uint32_t, uint32_t, uint32_t, uint32_t);
void *OPENTTD_COMPANY_CALL openttd_rust_economy_state();
int64_t *OPENTTD_COMPANY_CALL openttd_rust_economy_prices();
int64_t *OPENTTD_COMPANY_CALL openttd_rust_company_scores();
uint32_t *OPENTTD_COMPANY_CALL openttd_rust_company_tick();
void OPENTTD_COMPANY_CALL openttd_rust_company_subtract(const OpenTTDCompanyFinanceServices *, CompanyFinances *, uint8_t, int64_t, uint8_t);
void OPENTTD_COMPANY_CALL openttd_rust_company_subtract_fraction(const OpenTTDCompanyFinanceServices *, CompanyFinances *, uint8_t, int64_t, uint8_t);
void OPENTTD_COMPANY_CALL openttd_rust_company_update_landscaping_limits(const OpenTTDCompanyServices *, const OpenTTDCompanyLandscaping *);
int64_t OPENTTD_COMPANY_CALL openttd_rust_company_value(const OpenTTDCompanyServices *, const CompanyFinances *, uint8_t, bool);
int64_t OPENTTD_COMPANY_CALL openttd_rust_company_hostile_takeover_value(const OpenTTDCompanyServices *, const CompanyFinances *, uint8_t);
int32_t OPENTTD_COMPANY_CALL openttd_rust_company_update_rating(const OpenTTDCompanyServices *, CompanyFinances *, uint8_t, bool);
void OPENTTD_COMPANY_CALL openttd_rust_economy_month(const OpenTTDCompanyServices *, const OpenTTDEconomyMonth *);
bool OPENTTD_COMPANY_CALL openttd_rust_economy_add_inflation(bool, int32_t);
void OPENTTD_COMPANY_CALL openttd_rust_economy_recompute_prices(const OpenTTDCompanyServices *, const OpenTTDEconomySettings *);
void OPENTTD_COMPANY_CALL openttd_rust_economy_calendar_month(const OpenTTDCompanyServices *, const OpenTTDEconomySettings *);
void OPENTTD_COMPANY_CALL openttd_rust_economy_reset_price_multipliers();
void OPENTTD_COMPANY_CALL openttd_rust_economy_set_price_multiplier(uint32_t, int32_t);
void OPENTTD_COMPANY_CALL openttd_rust_economy_startup(const OpenTTDCompanyServices *, const OpenTTDEconomySettings *);
void OPENTTD_COMPANY_CALL openttd_rust_economy_initialize(const OpenTTDCompanyServices *);
int64_t OPENTTD_COMPANY_CALL openttd_rust_economy_price(uint32_t, uint32_t, int32_t);
int64_t OPENTTD_COMPANY_CALL openttd_rust_company_available_money(const CompanyFinances *, bool);
bool OPENTTD_COMPANY_CALL openttd_rust_company_has_money(const CompanyFinances *, bool, int64_t);
int64_t OPENTTD_COMPANY_CALL openttd_rust_company_max_loan(const CompanyFinances *);
bool OPENTTD_COMPANY_CALL openttd_rust_company_startup(const OpenTTDCompanyServices *, OpenTTDCompanyStartup *);
bool OPENTTD_COMPANY_CALL openttd_rust_company_competitor_timeout(const OpenTTDCompanyServices *, const OpenTTDCompanyCompetitors *);
bool OPENTTD_COMPANY_CALL openttd_rust_company_on_tick(const OpenTTDCompanyServices *, OpenTTDCompanyTick *);
void OPENTTD_COMPANY_CALL openttd_rust_company_yearly(const OpenTTDCompanyServices *, const OpenTTDCompanyYear *);
bool OPENTTD_COMPANY_CALL openttd_rust_company_takeover_allowed(const OpenTTDCompanyServices *, uint8_t, uint8_t);
bool OPENTTD_COMPANY_CALL openttd_rust_company_control(const OpenTTDCompanyServices *, OpenTTDCompanyControl *);
void OPENTTD_COMPANY_CALL openttd_rust_company_change_ownership(const OpenTTDCompanyServices *, uint8_t, uint8_t);
OpenTTDCompanyCost OPENTTD_COMPANY_CALL openttd_rust_company_buy(const OpenTTDCompanyServices *, uint8_t, bool, bool);
OpenTTDCompanyCost OPENTTD_COMPANY_CALL openttd_rust_company_give_money(const OpenTTDCompanyFinanceServices *, bool, int64_t, uint8_t, bool);
OpenTTDCompanyCost OPENTTD_COMPANY_CALL openttd_rust_company_increase_loan(const OpenTTDCompanyFinanceServices *, uint8_t, int64_t, bool, bool);
OpenTTDCompanyCost OPENTTD_COMPANY_CALL openttd_rust_company_decrease_loan(const OpenTTDCompanyFinanceServices *, uint8_t, int64_t, bool, bool);
OpenTTDCompanyCost OPENTTD_COMPANY_CALL openttd_rust_company_set_max_loan(const OpenTTDCompanyFinanceServices *, uint8_t, int64_t, bool);
OpenTTDCompanyCost OPENTTD_COMPANY_CALL openttd_rust_company_change_bank_balance(const OpenTTDCompanyFinanceServices *, uint32_t, int64_t, uint8_t, uint8_t, bool);
}
#endif /* RUST_COMPANY_FFI_H */
