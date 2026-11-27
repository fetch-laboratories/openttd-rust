/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
/** @file company_ffi.h Company finances, economy and lifecycle, owned by Rust (`company.rs`). */
#ifndef RUST_COMPANY_FFI_H
#define RUST_COMPANY_FFI_H
#include "service_defaults.hpp"
#include "byte_span_ffi.h"
#include <cstddef>
#include <cstdint>
#include "services_ffi.h"
#include "call_ffi.h"
#include "abi-record-company.h"
struct BaseStation;
struct Company;
struct CompanyFinances;
struct GRFFile;
struct Goal;
struct OpenTTDCommandCost;
struct OpenTTDCommandCostServices;
struct OpenTTDCompanyIdentityServices;
struct OpenTTDEngineServices;
struct OpenTTDObjectivesServices;
struct OpenTTDSettingServices;
struct OpenTTDStationLifecycleServices;
struct OpenTTDSubsidyServices;
struct OpenTTDTileOwnerContext;
struct OpenTTDVehicle;
struct Sign;
struct StoryPage;

/* Company IDs, owners, vehicle types, price bases and expenses cross as uint8_t;
 * tiles as uint32_t; Money as int64_t. Layouts are checked by the generated ABI probes. */

/* Caller-owned stack continuations. They exist only where AI::StartNew can raise
 * a script memory-policy Script_FatalError (AIInstance::Initialize registers the
 * API and loads compatibility scripts before ScriptInstance::Initialize's catch).
 * Rust returns before that call; if it throws, the frame is abandoned. */
/** The frame of DoStartupNewCompany(is_ai, company); index is UINT32_MAX and created nullptr when no company was created. */
struct OpenTTDCompanyStartup {
	bool is_ai;
	uint8_t company;
	uint8_t stage;
	uint32_t index;
	Company *created;
};
/** The frame of CmdCompanyCtrl: its arguments, its DoCommandFlags and CCA_NEW_AI's startup. */
struct OpenTTDCompanyControl {
	OpenTTDCmdCompanyCtrl args;
	uint16_t flags;
	uint8_t stage;
	OpenTTDCompanyStartup startup;
};

/** The finance services the money paths of other owners share. */
struct OpenTTDCompanyFinanceServices {
	uint8_t (OPENTTD_RUST_CALL *current_company)() noexcept OPENTTD_SERVICE_DEFAULT(current_company);
	void (OPENTTD_RUST_CALL *set_current_company)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_current_company);
	bool (OPENTTD_RUST_CALL *networking)() noexcept OPENTTD_SERVICE_DEFAULT(networking);
	CompanyFinances * (OPENTTD_RUST_CALL *company_finances)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(company_finances); ///< Null for an invalid ID.
	CompanyFinances * (OPENTTD_RUST_CALL *company_get_finances)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(company_get_finances); ///< Company::Get: asserts the ID is below the pool's first unused index.
	void (OPENTTD_RUST_CALL *invalidate_company_windows)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(invalidate_company_windows);
};

/**
 * The tables and the native callees of the company bodies. Every service is a typed
 * noexcept C++ call; environmental failures terminate inside it. Services that
 * reenter or delete (delete_*, allocate_company, ai_stop) run after Rust has ended
 * every raw field access they may invalidate. A slot that takes a company takes the
 * live Company the Rust caller resolved.
 */
struct OpenTTDCompanyServices {
	const OpenTTDCommandCostServices *cost;
	const OpenTTDCompanyFinanceServices *finance;
	const OpenTTDCompanyIdentityServices *identity;
	const OpenTTDSharedServices *shared;
	const OpenTTDEngineServices *(OPENTTD_RUST_CALL *engines)() noexcept OPENTTD_SERVICE_DEFAULT(engines);
	const OpenTTDObjectivesServices &(*objectives)() noexcept OPENTTD_SERVICE_DEFAULT(objectives);
	const OpenTTDSettingServices &(*settings)() noexcept OPENTTD_SERVICE_DEFAULT(settings);
	const OpenTTDStationLifecycleServices *(OPENTTD_RUST_CALL *station_lifecycle)() noexcept OPENTTD_SERVICE_DEFAULT(station_lifecycle);
	const OpenTTDSubsidyServices *(OPENTTD_RUST_CALL *subsidies)() noexcept OPENTTD_SERVICE_DEFAULT(subsidies);
	bool (OPENTTD_RUST_CALL *ai_can_start_new)() noexcept OPENTTD_SERVICE_DEFAULT(ai_can_start_new);
	void (OPENTTD_RUST_CALL *ai_stop)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(ai_stop);
	Company *(OPENTTD_RUST_CALL *allocate_company)(uint8_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(allocate_company);
	bool (OPENTTD_RUST_CALL *allow_list_add)(Company *, OpenTTDByteSpan) noexcept OPENTTD_SERVICE_DEFAULT(allow_list_add);
	bool (OPENTTD_RUST_CALL *allow_list_remove)(Company *, OpenTTDByteSpan) noexcept OPENTTD_SERVICE_DEFAULT(allow_list_remove);
	bool (OPENTTD_RUST_CALL *can_allocate_company)() noexcept OPENTTD_SERVICE_DEFAULT(can_allocate_company);
	int64_t (OPENTTD_RUST_CALL *canal_maintenance_cost)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(canal_maintenance_cost);
	int64_t (OPENTTD_RUST_CALL *cargo_initial_payment)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_initial_payment);
	void (OPENTTD_RUST_CALL *cargo_set_current_payment)(const void *, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_set_current_payment);
	void (OPENTTD_RUST_CALL *change_window_owner)(uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(change_window_owner);
	void (OPENTTD_RUST_CALL *close_company_windows)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(close_company_windows);
	void (OPENTTD_RUST_CALL *close_construction_windows)() noexcept OPENTTD_SERVICE_DEFAULT(close_construction_windows);
	void (OPENTTD_RUST_CALL *company_admin_remove)(uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(company_admin_remove);
	size_t (OPENTTD_RUST_CALL *company_count)() noexcept OPENTTD_SERVICE_DEFAULT(company_count);
	void (OPENTTD_RUST_CALL *delete_company)(Company *) noexcept OPENTTD_SERVICE_DEFAULT(delete_company);
	void (OPENTTD_RUST_CALL *delete_goal)(Goal *) noexcept OPENTTD_SERVICE_DEFAULT(delete_goal);
	void (OPENTTD_RUST_CALL *delete_story_page)(StoryPage *) noexcept OPENTTD_SERVICE_DEFAULT(delete_story_page);
	uint16_t (OPENTTD_RUST_CALL *draw_string_company_colour)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(draw_string_company_colour);
	void *(OPENTTD_RUST_CALL *group_all)(Company *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(group_all);
	void *(OPENTTD_RUST_CALL *group_default)(Company *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(group_default);
	uint32_t (OPENTTD_RUST_CALL *location_of_hq)(Company *) noexcept OPENTTD_SERVICE_DEFAULT(location_of_hq);
	void (OPENTTD_RUST_CALL *network_admin_company_new)(Company *) noexcept OPENTTD_SERVICE_DEFAULT(network_admin_company_new);
	void (OPENTTD_RUST_CALL *network_clients_to_spectators)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(network_clients_to_spectators);
	bool (OPENTTD_RUST_CALL *network_server)() noexcept OPENTTD_SERVICE_DEFAULT(network_server);
	bool (OPENTTD_RUST_CALL *new_competitor_timeout_has_fired)() noexcept OPENTTD_SERVICE_DEFAULT(new_competitor_timeout_has_fired);
	void (OPENTTD_RUST_CALL *new_competitor_timeout_reset)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(new_competitor_timeout_reset);
	OpenTTDCompanyPoolCursor (OPENTTD_RUST_CALL *next_goal)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next_goal);
	OpenTTDCompanyPoolCursor (OPENTTD_RUST_CALL *next_sign)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next_sign);
	OpenTTDCompanyPoolCursor (OPENTTD_RUST_CALL *next_story_page)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next_story_page);
	OpenTTDCompanyPoolCursor (OPENTTD_RUST_CALL *next_waypoint)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next_waypoint);
	bool (OPENTTD_RUST_CALL *parse_company_manager_face)(Company *) noexcept OPENTTD_SERVICE_DEFAULT(parse_company_manager_face);
	uint8_t (OPENTTD_RUST_CALL *price_base_category)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(price_base_category);
	int64_t (OPENTTD_RUST_CALL *price_base_start_price)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(price_base_start_price);
	int64_t (OPENTTD_RUST_CALL *rail_maintenance_cost)(uint8_t, uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_maintenance_cost);
	void (OPENTTD_RUST_CALL *release_free_group)(Company *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(release_free_group);
	int64_t (OPENTTD_RUST_CALL *road_maintenance_cost)(uint8_t, uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(road_maintenance_cost);
	uint32_t (OPENTTD_RUST_CALL *script_random_next)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(script_random_next);
	void (OPENTTD_RUST_CALL *send_company_manager_face)(Company *) noexcept OPENTTD_SERVICE_DEFAULT(send_company_manager_face);
	void (OPENTTD_RUST_CALL *set_avail_railtypes)(Company *, uint64_t) noexcept OPENTTD_SERVICE_DEFAULT(set_avail_railtypes);
	void (OPENTTD_RUST_CALL *set_avail_roadtypes)(Company *, uint64_t) noexcept OPENTTD_SERVICE_DEFAULT(set_avail_roadtypes);
	void (OPENTTD_RUST_CALL *set_default_company_settings)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_default_company_settings);
	void (OPENTTD_RUST_CALL *set_inaugurated_year)(Company *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_inaugurated_year);
	void (OPENTTD_RUST_CALL *set_inaugurated_year_calendar)(Company *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_inaugurated_year_calendar);
	void (OPENTTD_RUST_CALL *set_unitnumber)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_unitnumber);
	void (OPENTTD_RUST_CALL *show_buy_company_dialog)(uint8_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(show_buy_company_dialog);
	void (OPENTTD_RUST_CALL *show_company_finances)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(show_company_finances);
	uint8_t *(OPENTTD_RUST_CALL *sign_owner)(Sign *) noexcept OPENTTD_SERVICE_DEFAULT(sign_owner);
	int64_t (OPENTTD_RUST_CALL *signal_maintenance_cost)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(signal_maintenance_cost);
	void (OPENTTD_RUST_CALL *snd_play_fx)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(snd_play_fx);
	int64_t (OPENTTD_RUST_CALL *station_maintenance_cost)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_maintenance_cost);
	void (OPENTTD_RUST_CALL *sync_company_settings)() noexcept OPENTTD_SERVICE_DEFAULT(sync_company_settings);
	uint16_t (OPENTTD_RUST_CALL *use_free_group)(Company *) noexcept OPENTTD_SERVICE_DEFAULT(use_free_group);
	uint16_t (OPENTTD_RUST_CALL *use_free_unit)(Company *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(use_free_unit);
};

/* All entry points run serially on the game thread with borrowed static tables.
 * Rust owns the finance allocations; C++ starts the field objects in that raw
 * storage and keeps stable field addresses for existing readers and save
 * adapters. Rust accesses only raw fields, never across a service that may
 * invalidate them. Save/load errors run entirely outside Rust. Rust panics abort. */
extern "C" {
CompanyFinances *OPENTTD_RUST_CALL openttd_rust_company_state_create();
void OPENTTD_RUST_CALL openttd_rust_company_state_destroy(CompanyFinances *);
void OPENTTD_RUST_CALL openttd_rust_company_initialize(CompanyFinances *, uint32_t, uint32_t, uint32_t, uint32_t);
void *OPENTTD_RUST_CALL openttd_rust_economy_state();
int64_t *OPENTTD_RUST_CALL openttd_rust_economy_prices();
int64_t *OPENTTD_RUST_CALL openttd_rust_company_scores();
uint32_t *OPENTTD_RUST_CALL openttd_rust_company_tick();
int64_t OPENTTD_RUST_CALL openttd_rust_company_max_loan(const CompanyFinances *);
void openttd_rust_company_set_local(const OpenTTDCompanyServices *, uint8_t);
int64_t OPENTTD_RUST_CALL openttd_rust_company_available_money(const OpenTTDCompanyServices *, uint8_t);
void OPENTTD_RUST_CALL openttd_rust_company_subtract_fraction(const OpenTTDCompanyFinanceServices *, uint8_t, int64_t, uint8_t);
void openttd_rust_company_check_ownership(const OpenTTDCompanyServices *, OpenTTDCommandCost *, uint8_t, uint32_t);
void openttd_rust_company_check_tile_ownership(const OpenTTDCompanyServices *, OpenTTDCommandCost *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_company_startup_abi(const OpenTTDCompanyServices *, OpenTTDCompanyStartup *);
extern "C++" inline bool OPENTTD_RUST_CALL openttd_rust_company_startup(const OpenTTDCompanyServices *a0, OpenTTDCompanyStartup *a1) { return openttd_rust_company_startup_abi(a0, a1) != 0; }
uint8_t OPENTTD_RUST_CALL openttd_rust_company_competitor_timeout_abi(const OpenTTDCompanyServices *);
extern "C++" inline bool OPENTTD_RUST_CALL openttd_rust_company_competitor_timeout(const OpenTTDCompanyServices *a0) { return openttd_rust_company_competitor_timeout_abi(a0) != 0; }
uint8_t OPENTTD_RUST_CALL openttd_rust_company_control_abi(const OpenTTDCompanyServices *, const OpenTTDTileOwnerContext *, OpenTTDCommandCost *, OpenTTDCompanyControl *);
extern "C++" inline bool OPENTTD_RUST_CALL openttd_rust_company_control(const OpenTTDCompanyServices *a0, const OpenTTDTileOwnerContext *a1, OpenTTDCommandCost *a2, OpenTTDCompanyControl *a3) { return openttd_rust_company_control_abi(a0, a1, a2, a3) != 0; }
void openttd_rust_company_allow_list(const OpenTTDCompanyServices *, OpenTTDCommandCost *, uint16_t, uint8_t, OpenTTDByteSpan);
int32_t openttd_rust_company_service_interval(const OpenTTDCompanyServices *, const Company *, uint8_t);
void OPENTTD_RUST_CALL openttd_rust_company_give_money(const OpenTTDCompanyServices *, OpenTTDCommandCost *, uint16_t, int64_t, uint8_t);
uint8_t openttd_rust_company_first_playable(const OpenTTDCompanyServices *);
int64_t OPENTTD_RUST_CALL openttd_rust_company_value(const OpenTTDCompanyServices *, const Company *, bool);
int64_t OPENTTD_RUST_CALL openttd_rust_company_hostile_takeover_value(const OpenTTDCompanyServices *, const Company *);
int32_t OPENTTD_RUST_CALL openttd_rust_company_update_rating(const OpenTTDCompanyServices *, Company *, bool);
void OPENTTD_RUST_CALL openttd_rust_company_change_ownership(const OpenTTDCompanyServices *, const OpenTTDTileOwnerContext *, uint8_t, uint8_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_economy_add_inflation_abi(bool);
extern "C++" inline bool OPENTTD_RUST_CALL openttd_rust_economy_add_inflation(bool a0) { return openttd_rust_economy_add_inflation_abi(a0) != 0; }
void OPENTTD_RUST_CALL openttd_rust_economy_recompute_prices(const OpenTTDCompanyServices *);
void OPENTTD_RUST_CALL openttd_rust_economy_reset_price_multipliers();
void OPENTTD_RUST_CALL openttd_rust_economy_set_price_multiplier(uint8_t, int32_t);
void OPENTTD_RUST_CALL openttd_rust_economy_initialize();
int64_t OPENTTD_RUST_CALL openttd_rust_economy_get_price(const OpenTTDSharedServices *, uint8_t, uint32_t, const GRFFile *, int32_t);
uint32_t OPENTTD_RUST_CALL openttd_rust_economy_scale_by_inverse_cargo_scale(const OpenTTDSharedServices *, uint32_t, bool);
void OPENTTD_RUST_CALL openttd_rust_company_buy(const OpenTTDCompanyServices *, const OpenTTDTileOwnerContext *, OpenTTDCommandCost *, uint16_t, uint8_t, bool);
void OPENTTD_RUST_CALL openttd_rust_company_increase_loan(const OpenTTDCompanyServices *, OpenTTDCommandCost *, uint16_t, uint8_t, int64_t);
void OPENTTD_RUST_CALL openttd_rust_company_decrease_loan(const OpenTTDCompanyServices *, OpenTTDCommandCost *, uint16_t, uint8_t, int64_t);
void OPENTTD_RUST_CALL openttd_rust_company_set_max_loan(const OpenTTDCompanyServices *, OpenTTDCommandCost *, uint16_t, uint8_t, int64_t);
void OPENTTD_RUST_CALL openttd_rust_company_change_bank_balance(const OpenTTDCompanyServices *, OpenTTDCommandCost *, uint16_t, uint32_t, int64_t, uint8_t, uint8_t);
}
#endif /* RUST_COMPANY_FFI_H */
