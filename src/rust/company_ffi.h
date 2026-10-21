/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
/** @file company_ffi.h Canonical company finances and economy control. */
#ifndef RUST_COMPANY_FFI_H
#define RUST_COMPANY_FFI_H
#include "service_defaults.hpp"
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
struct OpenTTDTileOwnerContext;
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
/** Finance services used by money paths and financial commands. */
struct OpenTTDCompanyFinanceServices {
	uint8_t (OPENTTD_COMPANY_CALL *current_company)() noexcept OPENTTD_SERVICE_DEFAULT(current_company);
	void (OPENTTD_COMPANY_CALL *set_current_company)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_current_company);
	bool (OPENTTD_COMPANY_CALL *networking)() noexcept OPENTTD_SERVICE_DEFAULT(networking);
	CompanyFinances * (OPENTTD_COMPANY_CALL *company_finances)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(company_finances); ///< Null for an invalid ID.
	void (OPENTTD_COMPANY_CALL *invalidate_company_windows)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(invalidate_company_windows);
	void (OPENTTD_COMPANY_CALL *give_money_message)(uint8_t, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(give_money_message);
	void (OPENTTD_COMPANY_CALL *money_animation)(uint32_t, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(money_animation);
};
/**
 * Lifecycle, economy and ownership-transfer services. Every service is a typed
 * noexcept C++ call; environmental failures terminate inside it. Services that
 * reenter or delete (post_company_delete, delete_*,
 * allocate_company, reset_service_interval, stop_ai) run after Rust has ended
 * every raw field access it may invalidate. *_next_owned scans a pool from an
 * index for the next item of that owner, in pool order.
 */
struct OpenTTDObjectServices;
struct OpenTTDCompanyServices {
	const OpenTTDCompanyFinanceServices *finance;
	const OpenTTDSharedServices *shared;
	OpenTTDCompanyRef (OPENTTD_COMPANY_CALL *company_next)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(company_next);
	bool (OPENTTD_COMPANY_CALL *company_is_ai)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(company_is_ai);
	size_t (OPENTTD_COMPANY_CALL *company_count)() noexcept OPENTTD_SERVICE_DEFAULT(company_count);
	bool (OPENTTD_COMPANY_CALL *can_allocate_company)() noexcept OPENTTD_SERVICE_DEFAULT(can_allocate_company);
	uint8_t (OPENTTD_COMPANY_CALL *local_company)() noexcept OPENTTD_SERVICE_DEFAULT(local_company);
	bool (OPENTTD_COMPANY_CALL *network_server)() noexcept OPENTTD_SERVICE_DEFAULT(network_server);
	OpenTTDCompanyVehicleCounts (OPENTTD_COMPANY_CALL *company_vehicle_counts)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(company_vehicle_counts);
	OpenTTDCompanyVehicleCounts (OPENTTD_COMPANY_CALL *vehicle_limits)() noexcept OPENTTD_SERVICE_DEFAULT(vehicle_limits);
	void (OPENTTD_COMPANY_CALL *company_admin_update)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(company_admin_update);
	void (OPENTTD_COMPANY_CALL *company_in_trouble)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(company_in_trouble);
	void (OPENTTD_COMPANY_CALL *post_company_delete)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(post_company_delete);
	uint32_t (OPENTTD_COMPANY_CALL *hq_location)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(hq_location);
	const OpenTTDObjectServices *(OPENTTD_COMPANY_CALL *object_services)() noexcept OPENTTD_SERVICE_DEFAULT(object_services);
	void (OPENTTD_COMPANY_CALL *performance_detail_dirty)() noexcept OPENTTD_SERVICE_DEFAULT(performance_detail_dirty);
	void (OPENTTD_COMPANY_CALL *company_graphs_dirty)() noexcept OPENTTD_SERVICE_DEFAULT(company_graphs_dirty);
	void (OPENTTD_COMPANY_CALL *company_infrastructure)(uint8_t, OpenTTDCompanyInfrastructure *) noexcept OPENTTD_SERVICE_DEFAULT(company_infrastructure);
	int64_t (OPENTTD_COMPANY_CALL *rail_maintenance_cost)(uint8_t, uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_maintenance_cost);
	int64_t (OPENTTD_COMPANY_CALL *signal_maintenance_cost)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(signal_maintenance_cost);
	int64_t (OPENTTD_COMPANY_CALL *road_maintenance_cost)(uint8_t, uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(road_maintenance_cost);
	int64_t (OPENTTD_COMPANY_CALL *canal_maintenance_cost)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(canal_maintenance_cost);
	int64_t (OPENTTD_COMPANY_CALL *station_maintenance_cost)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_maintenance_cost);
	int64_t (OPENTTD_COMPANY_CALL *airport_maintenance_cost)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(airport_maintenance_cost);
	void (OPENTTD_COMPANY_CALL *recession_news)(bool) noexcept OPENTTD_SERVICE_DEFAULT(recession_news);
	OpenTTDCompanyPriceBase (OPENTTD_COMPANY_CALL *price_base)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(price_base);
	uint32_t (OPENTTD_COMPANY_CALL *cargo_next)(uint32_t, int64_t *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_next);
	void (OPENTTD_COMPANY_CALL *set_cargo_payment)(uint32_t, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(set_cargo_payment);
	void (OPENTTD_COMPANY_CALL *price_windows_dirty)() noexcept OPENTTD_SERVICE_DEFAULT(price_windows_dirty);
	void (OPENTTD_COMPANY_CALL *industry_daily_changes)(bool) noexcept OPENTTD_SERVICE_DEFAULT(industry_daily_changes);
	void (OPENTTD_COMPANY_CALL *clear_cargo_monitors)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(clear_cargo_monitors);
	void (OPENTTD_COMPANY_CALL *clear_all_cargo_monitors)() noexcept OPENTTD_SERVICE_DEFAULT(clear_all_cargo_monitors);
	void (OPENTTD_COMPANY_CALL *generate_company_name)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(generate_company_name);
	void (OPENTTD_COMPANY_CALL *ask_merger)(uint8_t, uint8_t, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(ask_merger);
	bool (OPENTTD_COMPANY_CALL *is_interactive_company)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(is_interactive_company);
	void (OPENTTD_COMPANY_CALL *show_buy_company)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(show_buy_company);
	uint32_t (OPENTTD_COMPANY_CALL *script_random_next)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(script_random_next);
	void (OPENTTD_COMPANY_CALL *reset_competitor_timeout)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(reset_competitor_timeout);
	void (OPENTTD_COMPANY_CALL *finances_dirty)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(finances_dirty);
	void (OPENTTD_COMPANY_CALL *show_company_finances)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(show_company_finances);
	void (OPENTTD_COMPANY_CALL *new_year_sound)(bool) noexcept OPENTTD_SERVICE_DEFAULT(new_year_sound);
	uint8_t (OPENTTD_COMPANY_CALL *generate_company_colour)() noexcept OPENTTD_SERVICE_DEFAULT(generate_company_colour);
	OpenTTDCompanyRef (OPENTTD_COMPANY_CALL *allocate_company)(uint8_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(allocate_company);
	void (OPENTTD_COMPANY_CALL *set_company_colour)(uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_company_colour);
	void (OPENTTD_COMPANY_CALL *setup_new_company)(uint8_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(setup_new_company);
	void (OPENTTD_COMPANY_CALL *new_company_events)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(new_company_events);
	void (OPENTTD_COMPANY_CALL *company_league_dirty)() noexcept OPENTTD_SERVICE_DEFAULT(company_league_dirty);
	void (OPENTTD_COMPANY_CALL *company_ctrl_windows)() noexcept OPENTTD_SERVICE_DEFAULT(company_ctrl_windows);
	void (OPENTTD_COMPANY_CALL *close_network_status)() noexcept OPENTTD_SERVICE_DEFAULT(close_network_status);
	void (OPENTTD_COMPANY_CALL *network_spectate)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(network_spectate);
	void (OPENTTD_COMPANY_CALL *network_company_new)(uint8_t, uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(network_company_new);
	void (OPENTTD_COMPANY_CALL *network_own_company)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(network_own_company);
	void (OPENTTD_COMPANY_CALL *assert_new_ai_slot)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(assert_new_ai_slot);
	void (OPENTTD_COMPANY_CALL *company_bankrupt_news)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(company_bankrupt_news);
	void (OPENTTD_COMPANY_CALL *stop_ai)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(stop_ai);
	void (OPENTTD_COMPANY_CALL *delete_company)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(delete_company);
	void (OPENTTD_COMPANY_CALL *company_removed)(uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(company_removed);
	void (OPENTTD_COMPANY_CALL *merger_news)(uint8_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(merger_news);
	void (OPENTTD_COMPANY_CALL *acquisition_windows)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(acquisition_windows);
	void (OPENTTD_COMPANY_CALL *clients_to_spectators)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(clients_to_spectators);
	void (OPENTTD_COMPANY_CALL *set_local_company)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_local_company);
	uint32_t (OPENTTD_COMPANY_CALL *subsidy_next_awarded)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(subsidy_next_awarded);
	void (OPENTTD_COMPANY_CALL *delete_subsidy)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(delete_subsidy);
	void (OPENTTD_COMPANY_CALL *set_subsidy_awarded)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_subsidy_awarded);
	void (OPENTTD_COMPANY_CALL *rebuild_subsidy_cache)() noexcept OPENTTD_SERVICE_DEFAULT(rebuild_subsidy_cache);
	uint32_t (OPENTTD_COMPANY_CALL *town_next)(uint32_t, uint8_t, OpenTTDCompanyTownRights *) noexcept OPENTTD_SERVICE_DEFAULT(town_next);
	int16_t (OPENTTD_COMPANY_CALL *town_rating)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(town_rating);
	void (OPENTTD_COMPANY_CALL *set_town_rating)(uint32_t, uint8_t, int16_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_town_rating);
	void (OPENTTD_COMPANY_CALL *set_town_exclusivity)(uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_town_exclusivity);
	uint32_t (OPENTTD_COMPANY_CALL *vehicle_next_owned)(uint8_t, uint32_t, uint8_t *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_next_owned);
	bool (OPENTTD_COMPANY_CALL *aircraft_is_normal)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(aircraft_is_normal);
	int64_t (OPENTTD_COMPANY_CALL *vehicle_value)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_value);
	bool (OPENTTD_COMPANY_CALL *vehicle_is_primary)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_is_primary);
	OpenTTDCompanyVehicleProfit (OPENTTD_COMPANY_CALL *vehicle_profit)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_profit);
	uint32_t (OPENTTD_COMPANY_CALL *vehicle_previous)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_previous);
	OpenTTDCompanyVehicleGroupFlags (OPENTTD_COMPANY_CALL *vehicle_group_flags)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_group_flags);
	bool (OPENTTD_COMPANY_CALL *vehicle_service_interval_is_custom)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_service_interval_is_custom);
	void (OPENTTD_COMPANY_CALL *delete_vehicle)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(delete_vehicle);
	void (OPENTTD_COMPANY_CALL *count_group_engine)(uint32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(count_group_engine);
	void (OPENTTD_COMPANY_CALL *count_group_vehicle)(uint32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(count_group_vehicle);
	void (OPENTTD_COMPANY_CALL *reset_service_interval)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(reset_service_interval);
	void (OPENTTD_COMPANY_CALL *set_vehicle_owner)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_vehicle_owner);
	void (OPENTTD_COMPANY_CALL *assign_unit_number)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(assign_unit_number);
	void (OPENTTD_COMPANY_CALL *remove_engine_replacements)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(remove_engine_replacements);
	uint32_t (OPENTTD_COMPANY_CALL *group_next_owned)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(group_next_owned);
	void (OPENTTD_COMPANY_CALL *delete_group)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(delete_group);
	void (OPENTTD_COMPANY_CALL *transfer_group)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(transfer_group);
	void (OPENTTD_COMPANY_CALL *copy_service_interval_defaults)(uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(copy_service_interval_defaults);
	void (OPENTTD_COMPANY_CALL *update_autoreplace)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(update_autoreplace);
	uint32_t (OPENTTD_COMPANY_CALL *map_size)() noexcept OPENTTD_SERVICE_DEFAULT(map_size);
	OpenTTDCompanySignalTile (OPENTTD_COMPANY_CALL *signal_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(signal_tile);
	bool (OPENTTD_COMPANY_CALL *has_signal_on_track)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(has_signal_on_track);
	void (OPENTTD_COMPANY_CALL *add_track_to_signal_buffer)(uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(add_track_to_signal_buffer);
	void (OPENTTD_COMPANY_CALL *update_signals_in_buffer)() noexcept OPENTTD_SERVICE_DEFAULT(update_signals_in_buffer);
	void (OPENTTD_COMPANY_CALL *add_airport_infrastructure)(uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(add_airport_infrastructure);
	uint32_t (OPENTTD_COMPANY_CALL *station_next_owned)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_next_owned);
	uint32_t (OPENTTD_COMPANY_CALL *station_facility_count)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_facility_count);
	OpenTTDCompanyStationTimes (OPENTTD_COMPANY_CALL *station_times)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_times);
	void (OPENTTD_COMPANY_CALL *set_station_owner)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_station_owner);
	uint32_t (OPENTTD_COMPANY_CALL *waypoint_next_owned)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(waypoint_next_owned);
	void (OPENTTD_COMPANY_CALL *set_waypoint_owner)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_waypoint_owner);
	uint32_t (OPENTTD_COMPANY_CALL *sign_next_owned)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(sign_next_owned);
	void (OPENTTD_COMPANY_CALL *set_sign_owner)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_sign_owner);
	uint32_t (OPENTTD_COMPANY_CALL *goal_next_owned)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(goal_next_owned);
	void (OPENTTD_COMPANY_CALL *delete_goal)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(delete_goal);
	uint32_t (OPENTTD_COMPANY_CALL *story_page_next_owned)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(story_page_next_owned);
	void (OPENTTD_COMPANY_CALL *delete_story_page)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(delete_story_page);
	void (OPENTTD_COMPANY_CALL *change_window_owner)(uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(change_window_owner);
	void (OPENTTD_COMPANY_CALL *mark_whole_screen_dirty)() noexcept OPENTTD_SERVICE_DEFAULT(mark_whole_screen_dirty);
};
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
int64_t OPENTTD_COMPANY_CALL openttd_rust_company_max_loan(const CompanyFinances *);
bool OPENTTD_COMPANY_CALL openttd_rust_company_startup(const OpenTTDCompanyServices *, OpenTTDCompanyStartup *);
bool OPENTTD_COMPANY_CALL openttd_rust_company_competitor_timeout(const OpenTTDCompanyServices *, const OpenTTDCompanyCompetitors *);
bool OPENTTD_COMPANY_CALL openttd_rust_company_on_tick(const OpenTTDCompanyServices *, OpenTTDCompanyTick *);
void OPENTTD_COMPANY_CALL openttd_rust_company_yearly(const OpenTTDCompanyServices *, const OpenTTDCompanyYear *);
bool OPENTTD_COMPANY_CALL openttd_rust_company_takeover_allowed(const OpenTTDCompanyServices *, uint8_t, uint8_t);
bool OPENTTD_COMPANY_CALL openttd_rust_company_control(const OpenTTDCompanyServices *, const OpenTTDTileOwnerContext *, OpenTTDCompanyControl *);
void OPENTTD_COMPANY_CALL openttd_rust_company_change_ownership(const OpenTTDCompanyServices *, const OpenTTDTileOwnerContext *, uint8_t, uint8_t);
OpenTTDCompanyCost OPENTTD_COMPANY_CALL openttd_rust_company_buy(const OpenTTDCompanyServices *, const OpenTTDTileOwnerContext *, uint8_t, bool, bool);
OpenTTDCompanyCost OPENTTD_COMPANY_CALL openttd_rust_company_give_money(const OpenTTDCompanyFinanceServices *, bool, int64_t, uint8_t, bool);
OpenTTDCompanyCost OPENTTD_COMPANY_CALL openttd_rust_company_increase_loan(const OpenTTDCompanyFinanceServices *, uint8_t, int64_t, bool, bool);
OpenTTDCompanyCost OPENTTD_COMPANY_CALL openttd_rust_company_decrease_loan(const OpenTTDCompanyFinanceServices *, uint8_t, int64_t, bool, bool);
OpenTTDCompanyCost OPENTTD_COMPANY_CALL openttd_rust_company_set_max_loan(const OpenTTDCompanyFinanceServices *, uint8_t, int64_t, bool);
OpenTTDCompanyCost OPENTTD_COMPANY_CALL openttd_rust_company_change_bank_balance(const OpenTTDCompanyFinanceServices *, uint32_t, int64_t, uint8_t, uint8_t, bool);
}
#endif /* RUST_COMPANY_FFI_H */
