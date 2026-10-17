/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file windows-abi.cpp Bounded first-32-bit layout and real C ABI call checks. */
#include "stdafx.h"
#include "rust/vehicle_services.h"
#include "rust/abi_ffi.h"
#include "rust/vehicle_ffi.h"
#include "rust/vehicle_maintenance_ffi.h"
#include "rust/command_cost_ffi.h"
#include "rust/command_services_ffi.h"
#include "rust/water_regions_ffi.h"
#include "rust/cargo_payment_ffi.h"
#include "rust/cargo_storage_ffi.h"
#include "rust/cargo_flow_ffi.h"
#include "rust/ship_yapf_ffi.h"
#include "rust/rail_yapf_ffi.h"
#include "rust/train_ffi.h"
#include "rust/train_reservation_ffi.h"
#include "rust/linkgraph_ffi.h"
#include "rust/trees_ffi.h"
#include "rust/disaster_ffi.h"
#include "rust/aircraft_ffi.h"
#include "aircraft-boundary.hpp"
#include "rust/townname_ffi.h"
#include "rust/town_founding_ffi.h"
#include "rust/effect_ffi.h"
#include "rust/road_ffi.h"
#include "rust/ship_control_ffi.h"
#include "rust/orders_ffi.h"
#include "rust/road_yapf_ffi.h"
#include "tests/abi_layout.hpp"
#include "tests/effect_protocol.hpp"
#include "tests/water_regions_protocol.hpp"
#include "tests/ship_yapf_protocol.hpp"
#include "tests/town_protocol.hpp"
#include "rust/ffi.h"
#include "rust/utf8_ffi.h"
#include "rust/builder_ffi.h"
#include "rust/alternating_ffi.h"
#include "rust/consumer_ffi.h"
#include "rust/encoded_ffi.h"
#include "rust/spiral_ffi.h"
#include "rust/byte_strings_ffi.h"
#include "rust/history_ffi.h"
#include "rust/math_ffi.h"
#include "rust/station_cargo_ffi.h"
#include "rust/station_service_ffi.h"
#include "rust/script_list_ffi.h"
#include "rust/packet_ffi.h"
#include "rust/string_validation_ffi.h"
#include "rust/crypto_primitives_ffi.h"
#include "rust/blake2b_ffi.h"
#include "rust/x25519_ffi.h"
#include "3rdparty/monocypher/monocypher.h"
#include <algorithm>
#include <array>
#include <cctype>
#include <clocale>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <initializer_list>
#include <limits>
#include <memory>
#include <string>
#include <stdexcept>
#include <type_traits>
#include "misc/history_type.hpp"

#define CHECK(condition) do { if (!(condition)) { std::fprintf(stderr, "%s:%d: %s\n", __FILE__, __LINE__, #condition); std::abort(); } } while (0)

/* The one table of Rust ABI layouts for FFI types; native game structure lists stay in their tests. */
static void Layout(uint16_t type, const char *name, std::initializer_list<size_t> values)
{
	CHECK(AbiLayoutMismatch(type, values) == -1);
	std::printf("layout %s", name);
	for (size_t value : values) std::printf(" %zu", value);
	std::printf("\n");
}

static void Layouts()
{
	Layout(740, "OpenTTDVehiclePoolCursor", {sizeof(OpenTTDVehiclePoolCursor), alignof(OpenTTDVehiclePoolCursor), offsetof(OpenTTDVehiclePoolCursor, slots), offsetof(OpenTTDVehiclePoolCursor, slots_len), offsetof(OpenTTDVehiclePoolCursor, first_unused)});
	Layout(741, "OpenTTDVehiclePostTick", {sizeof(OpenTTDVehiclePostTick), alignof(OpenTTDVehiclePostTick), offsetof(OpenTTDVehiclePostTick, type), offsetof(OpenTTDVehiclePostTick, period), offsetof(OpenTTDVehiclePostTick, counter), offsetof(OpenTTDVehiclePostTick, motion), offsetof(OpenTTDVehiclePostTick, speed), offsetof(OpenTTDVehiclePostTick, front_type), offsetof(OpenTTDVehiclePostTick, subtype), offsetof(OpenTTDVehiclePostTick, tick), offsetof(OpenTTDVehiclePostTick, front_status), offsetof(OpenTTDVehiclePostTick, status)});
	Layout(743, "OpenTTDVehicleDayCallback", {sizeof(OpenTTDVehicleDayCallback), alignof(OpenTTDVehicleDayCallback), offsetof(OpenTTDVehicleDayCallback, day), offsetof(OpenTTDVehicleDayCallback, has_engine)});
	Layout(748, "OpenTTDVehicleStationLookup", {sizeof(OpenTTDVehicleStationLookup), alignof(OpenTTDVehicleStationLookup), offsetof(OpenTTDVehicleStationLookup, station), offsetof(OpenTTDVehicleStationLookup, id)});
	Layout(749, "OpenTTDVehicleSchedulerServices", {sizeof(OpenTTDVehicleSchedulerServices), alignof(OpenTTDVehicleSchedulerServices), offsetof(OpenTTDVehicleSchedulerServices, pool), offsetof(OpenTTDVehicleSchedulerServices, assertions), offsetof(OpenTTDVehicleSchedulerServices, tick), offsetof(OpenTTDVehicleSchedulerServices, calendar_day), offsetof(OpenTTDVehicleSchedulerServices, economy_day), offsetof(OpenTTDVehicleSchedulerServices, day_callback_read), offsetof(OpenTTDVehicleSchedulerServices, day_callback), offsetof(OpenTTDVehicleSchedulerServices, randomise_callback), offsetof(OpenTTDVehicleSchedulerServices, first_dirty), offsetof(OpenTTDVehicleSchedulerServices, unknown_day_callback), offsetof(OpenTTDVehicleSchedulerServices, cargo_owner), offsetof(OpenTTDVehicleSchedulerServices, running_sound), offsetof(OpenTTDVehicleSchedulerServices, periodic_sound), offsetof(OpenTTDVehicleSchedulerServices, next_station), offsetof(OpenTTDVehicleSchedulerServices, with_loading_timer), offsetof(OpenTTDVehicleSchedulerServices, reset_accumulators)});
	Layout(750, "OpenTTDVehicleSchedulerTables", {sizeof(OpenTTDVehicleSchedulerTables), alignof(OpenTTDVehicleSchedulerTables), offsetof(OpenTTDVehicleSchedulerTables, scheduler), offsetof(OpenTTDVehicleSchedulerTables, cargo), offsetof(OpenTTDVehicleSchedulerTables, station), offsetof(OpenTTDVehicleSchedulerTables, loading), offsetof(OpenTTDVehicleSchedulerTables, pending), offsetof(OpenTTDVehicleSchedulerTables, transaction), offsetof(OpenTTDVehicleSchedulerTables, group)});
	Layout(751, "OpenTTDVehicleMaintenanceRenewal", {sizeof(OpenTTDVehicleMaintenanceRenewal), alignof(OpenTTDVehicleMaintenanceRenewal), offsetof(OpenTTDVehicleMaintenanceRenewal, age), offsetof(OpenTTDVehicleMaintenanceRenewal, max_age), offsetof(OpenTTDVehicleMaintenanceRenewal, months), offsetof(OpenTTDVehicleMaintenanceRenewal, enabled), offsetof(OpenTTDVehicleMaintenanceRenewal, kind), offsetof(OpenTTDVehicleMaintenanceRenewal, subtype)});
	Layout(752, "OpenTTDVehicleMaintenanceReset", {sizeof(OpenTTDVehicleMaintenanceReset), alignof(OpenTTDVehicleMaintenanceReset), offsetof(OpenTTDVehicleMaintenanceReset, economy_date), offsetof(OpenTTDVehicleMaintenanceReset, calendar_date), offsetof(OpenTTDVehicleMaintenanceReset, reliability), offsetof(OpenTTDVehicleMaintenanceReset, chance), offsetof(OpenTTDVehicleMaintenanceReset, breakdowns)});
	Layout(753, "OpenTTDVehicleMaintenanceResetWrite", {sizeof(OpenTTDVehicleMaintenanceResetWrite), alignof(OpenTTDVehicleMaintenanceResetWrite), offsetof(OpenTTDVehicleMaintenanceResetWrite, economy_date), offsetof(OpenTTDVehicleMaintenanceResetWrite, calendar_date), offsetof(OpenTTDVehicleMaintenanceResetWrite, reliability), offsetof(OpenTTDVehicleMaintenanceResetWrite, chance), offsetof(OpenTTDVehicleMaintenanceResetWrite, since)});
	Layout(754, "OpenTTDVehicleMaintenanceInterval", {sizeof(OpenTTDVehicleMaintenanceInterval), alignof(OpenTTDVehicleMaintenanceInterval), offsetof(OpenTTDVehicleMaintenanceInterval, company), offsetof(OpenTTDVehicleMaintenanceInterval, last_service), offsetof(OpenTTDVehicleMaintenanceInterval, now), offsetof(OpenTTDVehicleMaintenanceInterval, interval), offsetof(OpenTTDVehicleMaintenanceInterval, reliability), offsetof(OpenTTDVehicleMaintenanceInterval, percent), offsetof(OpenTTDVehicleMaintenanceInterval, wallclock), offsetof(OpenTTDVehicleMaintenanceInterval, no_service), offsetof(OpenTTDVehicleMaintenanceInterval, breakdowns)});
	Layout(755, "OpenTTDVehicleMaintenanceReplacement", {sizeof(OpenTTDVehicleMaintenanceReplacement), alignof(OpenTTDVehicleMaintenanceReplacement), offsetof(OpenTTDVehicleMaintenanceReplacement, engine), offsetof(OpenTTDVehicleMaintenanceReplacement, when_old)});
	Layout(756, "OpenTTDVehicleMaintenanceBudget", {sizeof(OpenTTDVehicleMaintenanceBudget), alignof(OpenTTDVehicleMaintenanceBudget), offsetof(OpenTTDVehicleMaintenanceBudget, needed), offsetof(OpenTTDVehicleMaintenanceBudget, available)});
	Layout(757, "OpenTTDVehicleMaintenanceReplacementCost", {sizeof(OpenTTDVehicleMaintenanceReplacementCost), alignof(OpenTTDVehicleMaintenanceReplacementCost), offsetof(OpenTTDVehicleMaintenanceReplacementCost, cost), offsetof(OpenTTDVehicleMaintenanceReplacementCost, available)});
	Layout(758, "OpenTTDVehicleMaintenanceAutomatic", {sizeof(OpenTTDVehicleMaintenanceAutomatic), alignof(OpenTTDVehicleMaintenanceAutomatic), offsetof(OpenTTDVehicleMaintenanceAutomatic, has_depot_order), offsetof(OpenTTDVehicleMaintenanceAutomatic, order_type)});
	Layout(759, "OpenTTDVehicleMaintenanceCheckBreakdown", {sizeof(OpenTTDVehicleMaintenanceCheckBreakdown), alignof(OpenTTDVehicleMaintenanceCheckBreakdown), offsetof(OpenTTDVehicleMaintenanceCheckBreakdown, reliability), offsetof(OpenTTDVehicleMaintenanceCheckBreakdown, decay), offsetof(OpenTTDVehicleMaintenanceCheckBreakdown, no_service), offsetof(OpenTTDVehicleMaintenanceCheckBreakdown, breakdowns)});
	Layout(760, "OpenTTDVehicleMaintenanceBreakdownGuard", {sizeof(OpenTTDVehicleMaintenanceBreakdownGuard), alignof(OpenTTDVehicleMaintenanceBreakdownGuard), offsetof(OpenTTDVehicleMaintenanceBreakdownGuard, speed), offsetof(OpenTTDVehicleMaintenanceBreakdownGuard, counter), offsetof(OpenTTDVehicleMaintenanceBreakdownGuard, status), offsetof(OpenTTDVehicleMaintenanceBreakdownGuard, breakdowns), offsetof(OpenTTDVehicleMaintenanceBreakdownGuard, menu)});
	Layout(761, "OpenTTDVehicleMaintenanceBreakdownTrigger", {sizeof(OpenTTDVehicleMaintenanceBreakdownTrigger), alignof(OpenTTDVehicleMaintenanceBreakdownTrigger), offsetof(OpenTTDVehicleMaintenanceBreakdownTrigger, reliability), offsetof(OpenTTDVehicleMaintenanceBreakdownTrigger, chance), offsetof(OpenTTDVehicleMaintenanceBreakdownTrigger, kind), offsetof(OpenTTDVehicleMaintenanceBreakdownTrigger, breakdowns)});
	Layout(762, "OpenTTDVehicleMaintenanceBreakdownProbe", {sizeof(OpenTTDVehicleMaintenanceBreakdownProbe), alignof(OpenTTDVehicleMaintenanceBreakdownProbe), offsetof(OpenTTDVehicleMaintenanceBreakdownProbe, counter), offsetof(OpenTTDVehicleMaintenanceBreakdownProbe, status)});
	Layout(763, "OpenTTDVehicleMaintenanceBreakdownStart", {sizeof(OpenTTDVehicleMaintenanceBreakdownStart), alignof(OpenTTDVehicleMaintenanceBreakdownStart), offsetof(OpenTTDVehicleMaintenanceBreakdownStart, since), offsetof(OpenTTDVehicleMaintenanceBreakdownStart, kind)});
	Layout(764, "OpenTTDVehicleMaintenanceBreakdownActive", {sizeof(OpenTTDVehicleMaintenanceBreakdownActive), alignof(OpenTTDVehicleMaintenanceBreakdownActive), offsetof(OpenTTDVehicleMaintenanceBreakdownActive, kind), offsetof(OpenTTDVehicleMaintenanceBreakdownActive, tick), offsetof(OpenTTDVehicleMaintenanceBreakdownActive, delay)});
	Layout(765, "OpenTTDVehicleMaintenanceBreakdownCountdown", {sizeof(OpenTTDVehicleMaintenanceBreakdownCountdown), alignof(OpenTTDVehicleMaintenanceBreakdownCountdown), offsetof(OpenTTDVehicleMaintenanceBreakdownCountdown, order_type), offsetof(OpenTTDVehicleMaintenanceBreakdownCountdown, counter)});
	Layout(766, "OpenTTDVehicleMaintenanceBreakdownSoundState", {sizeof(OpenTTDVehicleMaintenanceBreakdownSoundState), alignof(OpenTTDVehicleMaintenanceBreakdownSoundState), offsetof(OpenTTDVehicleMaintenanceBreakdownSoundState, kind), offsetof(OpenTTDVehicleMaintenanceBreakdownSoundState, landscape)});
	Layout(767, "OpenTTDVehicleMaintenanceBreakdownSmoke", {sizeof(OpenTTDVehicleMaintenanceBreakdownSmoke), alignof(OpenTTDVehicleMaintenanceBreakdownSmoke), offsetof(OpenTTDVehicleMaintenanceBreakdownSmoke, shell), offsetof(OpenTTDVehicleMaintenanceBreakdownSmoke, delay)});
	Layout(768, "OpenTTDVehicleMaintenanceAge", {sizeof(OpenTTDVehicleMaintenanceAge), alignof(OpenTTDVehicleMaintenanceAge), offsetof(OpenTTDVehicleMaintenanceAge, age), offsetof(OpenTTDVehicleMaintenanceAge, max_age), offsetof(OpenTTDVehicleMaintenanceAge, decay)});
	Layout(769, "OpenTTDVehicleMaintenanceWarning", {sizeof(OpenTTDVehicleMaintenanceWarning), alignof(OpenTTDVehicleMaintenanceWarning), offsetof(OpenTTDVehicleMaintenanceWarning, enabled), offsetof(OpenTTDVehicleMaintenanceWarning, has_previous), offsetof(OpenTTDVehicleMaintenanceWarning, local), offsetof(OpenTTDVehicleMaintenanceWarning, status)});
	Layout(770, "OpenTTDVehicleMaintenanceWarningCompany", {sizeof(OpenTTDVehicleMaintenanceWarningCompany), alignof(OpenTTDVehicleMaintenanceWarningCompany), offsetof(OpenTTDVehicleMaintenanceWarningCompany, company), offsetof(OpenTTDVehicleMaintenanceWarningCompany, renewal)});
	Layout(771, "OpenTTDVehicleMaintenanceServices", {sizeof(OpenTTDVehicleMaintenanceServices), alignof(OpenTTDVehicleMaintenanceServices), offsetof(OpenTTDVehicleMaintenanceServices, shared), offsetof(OpenTTDVehicleMaintenanceServices, transaction), offsetof(OpenTTDVehicleMaintenanceServices, group), offsetof(OpenTTDVehicleMaintenanceServices, renewal), offsetof(OpenTTDVehicleMaintenanceServices, reset), offsetof(OpenTTDVehicleMaintenanceServices, reset_write), offsetof(OpenTTDVehicleMaintenanceServices, has_engine), offsetof(OpenTTDVehicleMaintenanceServices, interval), offsetof(OpenTTDVehicleMaintenanceServices, engine_reliability), offsetof(OpenTTDVehicleMaintenanceServices, budget), offsetof(OpenTTDVehicleMaintenanceServices, replacement), offsetof(OpenTTDVehicleMaintenanceServices, replacement_available), offsetof(OpenTTDVehicleMaintenanceServices, replacement_cost), offsetof(OpenTTDVehicleMaintenanceServices, automatic), offsetof(OpenTTDVehicleMaintenanceServices, depot_service), offsetof(OpenTTDVehicleMaintenanceServices, check_breakdown), offsetof(OpenTTDVehicleMaintenanceServices, breakdown_guard), offsetof(OpenTTDVehicleMaintenanceServices, breakdown_trigger), offsetof(OpenTTDVehicleMaintenanceServices, chance_write), offsetof(OpenTTDVehicleMaintenanceServices, trigger), offsetof(OpenTTDVehicleMaintenanceServices, breakdown_counter), offsetof(OpenTTDVehicleMaintenanceServices, breakdown_probe), offsetof(OpenTTDVehicleMaintenanceServices, breakdown_start), offsetof(OpenTTDVehicleMaintenanceServices, breakdown_start_write), offsetof(OpenTTDVehicleMaintenanceServices, breakdown_active), offsetof(OpenTTDVehicleMaintenanceServices, breakdown_countdown), offsetof(OpenTTDVehicleMaintenanceServices, breakdown_sound_state), offsetof(OpenTTDVehicleMaintenanceServices, breakdown_delay_write), offsetof(OpenTTDVehicleMaintenanceServices, breakdown_sound), offsetof(OpenTTDVehicleMaintenanceServices, smoke_disabled), offsetof(OpenTTDVehicleMaintenanceServices, smoke), offsetof(OpenTTDVehicleMaintenanceServices, smoke_animation), offsetof(OpenTTDVehicleMaintenanceServices, aircraft_broken), offsetof(OpenTTDVehicleMaintenanceServices, mark_dirty), offsetof(OpenTTDVehicleMaintenanceServices, view_dirty), offsetof(OpenTTDVehicleMaintenanceServices, details_dirty), offsetof(OpenTTDVehicleMaintenanceServices, age), offsetof(OpenTTDVehicleMaintenanceServices, age_write), offsetof(OpenTTDVehicleMaintenanceServices, decay_write), offsetof(OpenTTDVehicleMaintenanceServices, warning), offsetof(OpenTTDVehicleMaintenanceServices, warning_company), offsetof(OpenTTDVehicleMaintenanceServices, engine_available_any), offsetof(OpenTTDVehicleMaintenanceServices, replacement_active), offsetof(OpenTTDVehicleMaintenanceServices, old_news), offsetof(OpenTTDVehicleMaintenanceServices, economy_age), offsetof(OpenTTDVehicleMaintenanceServices, economy_age_write), offsetof(OpenTTDVehicleMaintenanceServices, reached_min_age), offsetof(OpenTTDVehicleMaintenanceServices, value), offsetof(OpenTTDVehicleMaintenanceServices, value_write), offsetof(OpenTTDVehicleMaintenanceServices, max_calendar_date), offsetof(OpenTTDVehicleMaintenanceServices, max_economy_date), offsetof(OpenTTDVehicleMaintenanceServices, profit_min_age), offsetof(OpenTTDVehicleMaintenanceServices, anniversaries), offsetof(OpenTTDVehicleMaintenanceServices, before_zero), offsetof(OpenTTDVehicleMaintenanceServices, news_getting_old), offsetof(OpenTTDVehicleMaintenanceServices, news_very_old), offsetof(OpenTTDVehicleMaintenanceServices, news_very_old_and)});
	Layout(772, "OpenTTDCalendarDate", {sizeof(OpenTTDCalendarDate), alignof(OpenTTDCalendarDate), offsetof(OpenTTDCalendarDate, value)});
	Layout(773, "OpenTTDEconomyDate", {sizeof(OpenTTDEconomyDate), alignof(OpenTTDEconomyDate), offsetof(OpenTTDEconomyDate, value)});
	Layout(774, "OrderType", {sizeof(OrderType), alignof(OrderType)});
	Layout(775, "LandscapeType", {sizeof(LandscapeType), alignof(LandscapeType)});

	Layout(630, "OpenTTDTownRef", {sizeof(OpenTTDTownRef), alignof(OpenTTDTownRef), offsetof(OpenTTDTownRef, shell), offsetof(OpenTTDTownRef, state), offsetof(OpenTTDTownRef, id)});
	Layout(631, "OpenTTDTownNameServices", {sizeof(OpenTTDTownNameServices), alignof(OpenTTDTownNameServices), offsetof(OpenTTDTownNameServices, next), offsetof(OpenTTDTownNameServices, render), offsetof(OpenTTDTownNameServices, render_other), offsetof(OpenTTDTownNameServices, utf8_length), offsetof(OpenTTDTownNameServices, random_next)});
	Layout(632, "OpenTTDTownNameMetadata", {sizeof(OpenTTDTownNameMetadata), alignof(OpenTTDTownNameMetadata), offsetof(OpenTTDTownNameMetadata, grfid), offsetof(OpenTTDTownNameMetadata, type)});
	Layout(633, "OpenTTDTownFoundingServices", {sizeof(OpenTTDTownFoundingServices), alignof(OpenTTDTownFoundingServices), offsetof(OpenTTDTownFoundingServices, shared), offsetof(OpenTTDTownFoundingServices, growth), offsetof(OpenTTDTownFoundingServices, names), offsetof(OpenTTDTownFoundingServices, cost), offsetof(OpenTTDTownFoundingServices, allocate), offsetof(OpenTTDTownFoundingServices, resolve), offsetof(OpenTTDTownFoundingServices, can_allocate), offsetof(OpenTTDTownFoundingServices, close_to_town), offsetof(OpenTTDTownFoundingServices, flat), offsetof(OpenTTDTownFoundingServices, clear_ground), offsetof(OpenTTDTownFoundingServices, tree_ground), offsetof(OpenTTDTownFoundingServices, water_distance), offsetof(OpenTTDTownFoundingServices, scale_size), offsetof(OpenTTDTownFoundingServices, scale_land), offsetof(OpenTTDTownFoundingServices, found_setting), offsetof(OpenTTDTownFoundingServices, layout_setting), offsetof(OpenTTDTownFoundingServices, initial_city_size), offsetof(OpenTTDTownFoundingServices, town_density), offsetof(OpenTTDTownFoundingServices, custom_number), offsetof(OpenTTDTownFoundingServices, larger_towns), offsetof(OpenTTDTownFoundingServices, parameters), offsetof(OpenTTDTownFoundingServices, metadata), offsetof(OpenTTDTownFoundingServices, initialize_counts), offsetof(OpenTTDTownFoundingServices, insert_town), offsetof(OpenTTDTownFoundingServices, cargo_effect_exists), offsetof(OpenTTDTownFoundingServices, prepare_rename), offsetof(OpenTTDTownFoundingServices, name_changed), offsetof(OpenTTDTownFoundingServices, update_virt), offsetof(OpenTTDTownFoundingServices, directory_rebuild), offsetof(OpenTTDTownFoundingServices, directory_resort), offsetof(OpenTTDTownFoundingServices, update_noise), offsetof(OpenTTDTownFoundingServices, rebuild_tree), offsetof(OpenTTDTownFoundingServices, delete_town), offsetof(OpenTTDTownFoundingServices, assert_random_context), offsetof(OpenTTDTownFoundingServices, assert_allocation), offsetof(OpenTTDTownFoundingServices, initialize_cost), offsetof(OpenTTDTownFoundingServices, sufficient_money), offsetof(OpenTTDTownFoundingServices, money_deficit), offsetof(OpenTTDTownFoundingServices, set_result), offsetof(OpenTTDTownFoundingServices, exchange_generating), offsetof(OpenTTDTownFoundingServices, nearest_town_roads), offsetof(OpenTTDTownFoundingServices, founded_news), offsetof(OpenTTDTownFoundingServices, founded_events), offsetof(OpenTTDTownFoundingServices, clear_station_names), offsetof(OpenTTDTownFoundingServices, clear_industry_names), offsetof(OpenTTDTownFoundingServices, update_station_coords), offsetof(OpenTTDTownFoundingServices, cached_name), offsetof(OpenTTDTownFoundingServices, set_progress), offsetof(OpenTTDTownFoundingServices, increase_progress), offsetof(OpenTTDTownFoundingServices, no_town_error), offsetof(OpenTTDTownFoundingServices, town_count), offsetof(OpenTTDTownFoundingServices, random), offsetof(OpenTTDTownFoundingServices, edge_error), offsetof(OpenTTDTownFoundingServices, close_error), offsetof(OpenTTDTownFoundingServices, unsuitable_error), offsetof(OpenTTDTownFoundingServices, unique_error), offsetof(OpenTTDTownFoundingServices, too_many_error), offsetof(OpenTTDTownFoundingServices, no_space_error)});
	Layout(445, "OpenTTDTownYearlyServices", {sizeof(OpenTTDTownYearlyServices), alignof(OpenTTDTownYearlyServices), offsetof(OpenTTDTownYearlyServices, completed), offsetof(OpenTTDTownYearlyServices, age), offsetof(OpenTTDTownYearlyServices, set_age)});
	Layout(444, "OpenTTDTownSuppliedEntry", {sizeof(OpenTTDTownSuppliedEntry), alignof(OpenTTDTownSuppliedEntry), offsetof(OpenTTDTownSuppliedEntry, entry), offsetof(OpenTTDTownSuppliedEntry, created)});
	Layout(443, "OpenTTDTownAuthorityServices", {sizeof(OpenTTDTownAuthorityServices), alignof(OpenTTDTownAuthorityServices), offsetof(OpenTTDTownAuthorityServices, council_tolerance), offsetof(OpenTTDTownAuthorityServices, bulldozer), offsetof(OpenTTDTownAuthorityServices, authority_distance), offsetof(OpenTTDTownAuthorityServices, town_index), offsetof(OpenTTDTownAuthorityServices, refusal)});
	Layout(621, "OpenTTDCommandServices", {sizeof(OpenTTDCommandServices), alignof(OpenTTDCommandServices), offsetof(OpenTTDCommandServices, landscape_clear)});
	Layout(620, "OpenTTDCommandCostServices", {sizeof(OpenTTDCommandCostServices), alignof(OpenTTDCommandCostServices), offsetof(OpenTTDCommandCostServices, zero), offsetof(OpenTTDCommandCostServices, error), offsetof(OpenTTDCommandCostServices, expense_money), offsetof(OpenTTDCommandCostServices, error_param), offsetof(OpenTTDCommandCostServices, add), offsetof(OpenTTDCommandCostServices, move), offsetof(OpenTTDCommandCostServices, amount), offsetof(OpenTTDCommandCostServices, success), offsetof(OpenTTDCommandCostServices, error_message), offsetof(OpenTTDCommandCostServices, money)});
	Layout(115, "OpenTTDStationLoading", {sizeof(OpenTTDStationLoading), alignof(OpenTTDStationLoading), offsetof(OpenTTDStationLoading, control), offsetof(OpenTTDStationLoading, part), offsetof(OpenTTDStationLoading, set_refit_capacity_and_stats), offsetof(OpenTTDStationLoading, wait_observation), offsetof(OpenTTDStationLoading, load_engine), offsetof(OpenTTDStationLoading, grf_version), offsetof(OpenTTDStationLoading, engine_load_callback), offsetof(OpenTTDStationLoading, engine_no_multiplier), offsetof(OpenTTDStationLoading, delivery_remaining), offsetof(OpenTTDStationLoading, cargo_stored), offsetof(OpenTTDStationLoading, cargo_reservation), offsetof(OpenTTDStationLoading, cargo_capacity), offsetof(OpenTTDStationLoading, vehicle_type), offsetof(OpenTTDStationLoading, cargo_type), offsetof(OpenTTDStationLoading, cargo_cap), offsetof(OpenTTDStationLoading, stored_count), offsetof(OpenTTDStationLoading, unload_count), offsetof(OpenTTDStationLoading, deliver_count), offsetof(OpenTTDStationLoading, transfer_count), offsetof(OpenTTDStationLoading, load_count), offsetof(OpenTTDStationLoading, unload_type), offsetof(OpenTTDStationLoading, has_orders), offsetof(OpenTTDStationLoading, load_unload_ticks), offsetof(OpenTTDStationLoading, status), offsetof(OpenTTDStationLoading, has_articulated_part), offsetof(OpenTTDStationLoading, is_articulated_part), offsetof(OpenTTDStationLoading, is_rear_dualheaded), offsetof(OpenTTDStationLoading, is_normal_aircraft), offsetof(OpenTTDStationLoading, is_multiheaded), offsetof(OpenTTDStationLoading, cargo_multiplier), offsetof(OpenTTDStationLoading, cached_total_length), offsetof(OpenTTDStationLoading, full_load_order), offsetof(OpenTTDStationLoading, vehicle_owner), offsetof(OpenTTDStationLoading, passenger_cargo), offsetof(OpenTTDStationLoading, station_length_penalty), offsetof(OpenTTDStationLoading, current_company), offsetof(OpenTTDStationLoading, vehicle), offsetof(OpenTTDStationLoading, total_count), offsetof(OpenTTDStationLoading, order_time), offsetof(OpenTTDStationLoading, refit_mask), offsetof(OpenTTDStationLoading, set_cargo_unloading), offsetof(OpenTTDStationLoading, set_loading_finished), offsetof(OpenTTDStationLoading, set_stop), offsetof(OpenTTDStationLoading, set_load_ticks), offsetof(OpenTTDStationLoading, set_speed), offsetof(OpenTTDStationLoading, subtract_profit), offsetof(OpenTTDStationLoading, unknown_load_result), offsetof(OpenTTDStationLoading, refresh_capacity), offsetof(OpenTTDStationLoading, vehicle_dirty), offsetof(OpenTTDStationLoading, update_indicator), offsetof(OpenTTDStationLoading, set_current_company), offsetof(OpenTTDStationLoading, next_vehicle), offsetof(OpenTTDStationLoading, next_articulated), offsetof(OpenTTDStationLoading, other_multiheaded), offsetof(OpenTTDStationLoading, first_engine_part), offsetof(OpenTTDStationLoading, first_vehicle), offsetof(OpenTTDStationLoading, previous_vehicle), offsetof(OpenTTDStationLoading, with_next_stations), offsetof(OpenTTDStationLoading, stage), offsetof(OpenTTDStationLoading, return_cargo), offsetof(OpenTTDStationLoading, unload_cargo), offsetof(OpenTTDStationLoading, load_cargo), offsetof(OpenTTDStationLoading, reserve_cargo), offsetof(OpenTTDStationLoading, deliver_to_transfer), offsetof(OpenTTDStationLoading, deliver_to_keep), offsetof(OpenTTDStationLoading, has_cargo_for), offsetof(OpenTTDStationLoading, platform_length), offsetof(OpenTTDStationLoading, vehicle_tile), offsetof(OpenTTDStationLoading, station_index), offsetof(OpenTTDStationLoading, station_cargo_total), offsetof(OpenTTDStationLoading, load_property), offsetof(OpenTTDStationLoading, load_callback), offsetof(OpenTTDStationLoading, get_payment), offsetof(OpenTTDStationLoading, create_payment), offsetof(OpenTTDStationLoading, delete_payment), offsetof(OpenTTDStationLoading, flush_delivery), offsetof(OpenTTDStationLoading, vehicle_new_cargo), offsetof(OpenTTDStationLoading, cargo_taken), offsetof(OpenTTDStationLoading, train_loads), offsetof(OpenTTDStationLoading, road_loads), offsetof(OpenTTDStationLoading, vehicle_empty), offsetof(OpenTTDStationLoading, station_dirty), offsetof(OpenTTDStationLoading, test_refit), offsetof(OpenTTDStationLoading, execute_refit)});
	Layout(114, "OpenTTDStationLinks", {sizeof(OpenTTDStationLinks), alignof(OpenTTDStationLinks), offsetof(OpenTTDStationLinks, graph), offsetof(OpenTTDStationLinks, automatic_distribution), offsetof(OpenTTDStationLinks, node), offsetof(OpenTTDStationLinks, edge_count), offsetof(OpenTTDStationLinks, date), offsetof(OpenTTDStationLinks, station_id), offsetof(OpenTTDStationLinks, edge_last_update), offsetof(OpenTTDStationLinks, last_compression), offsetof(OpenTTDStationLinks, edge), offsetof(OpenTTDStationLinks, edge_unrestricted), offsetof(OpenTTDStationLinks, edge_restricted), offsetof(OpenTTDStationLinks, delete_flows), offsetof(OpenTTDStationLinks, restrict_edge), offsetof(OpenTTDStationLinks, restrict_flows), offsetof(OpenTTDStationLinks, release_edge), offsetof(OpenTTDStationLinks, remove_edge), offsetof(OpenTTDStationLinks, compress), offsetof(OpenTTDStationLinks, order_list), offsetof(OpenTTDStationLinks, order_pool_size), offsetof(OpenTTDStationLinks, order_count), offsetof(OpenTTDStationLinks, order_destination), offsetof(OpenTTDStationLinks, order_vehicle), offsetof(OpenTTDStationLinks, next_vehicle), offsetof(OpenTTDStationLinks, next_shared), offsetof(OpenTTDStationLinks, stopped_in_depot), offsetof(OpenTTDStationLinks, last_service), offsetof(OpenTTDStationLinks, cargo_type), offsetof(OpenTTDStationLinks, refresh), offsetof(OpenTTDStationLinks, reroute)});
	Layout(112, "OpenTTDStationServices", {sizeof(OpenTTDStationServices), alignof(OpenTTDStationServices), offsetof(OpenTTDStationServices, station), offsetof(OpenTTDStationServices, station_owner), offsetof(OpenTTDStationServices, station_exclusivity), offsetof(OpenTTDStationServices, station_next), offsetof(OpenTTDStationServices, rating_counts), offsetof(OpenTTDStationServices, get), offsetof(OpenTTDStationServices, owner), offsetof(OpenTTDStationServices, pool_size), offsetof(OpenTTDStationServices, tick_counter), offsetof(OpenTTDStationServices, editor), offsetof(OpenTTDStationServices, facilities), offsetof(OpenTTDStationServices, in_use), offsetof(OpenTTDStationServices, is_station), offsetof(OpenTTDStationServices, local_owner), offsetof(OpenTTDStationServices, rectangle_empty), offsetof(OpenTTDStationServices, town_statue), offsetof(OpenTTDStationServices, select_goods), offsetof(OpenTTDStationServices, rating_cheat), offsetof(OpenTTDStationServices, available), offsetof(OpenTTDStationServices, cargo_valid), offsetof(OpenTTDStationServices, rating_callback_enabled), offsetof(OpenTTDStationServices, passenger_cargo), offsetof(OpenTTDStationServices, packet_available), offsetof(OpenTTDStationServices, rating_redraw), offsetof(OpenTTDStationServices, set_demand), offsetof(OpenTTDStationServices, acceptance_news), offsetof(OpenTTDStationServices, acceptance_redraw), offsetof(OpenTTDStationServices, delete_station), offsetof(OpenTTDStationServices, acceptance_animation), offsetof(OpenTTDStationServices, watched_animation), offsetof(OpenTTDStationServices, append_packet), offsetof(OpenTTDStationServices, update_supply), offsetof(OpenTTDStationServices, station_list_invalidation), offsetof(OpenTTDStationServices, new_cargo_animation), offsetof(OpenTTDStationServices, waiting_redraw), offsetof(OpenTTDStationServices, rating_callback), offsetof(OpenTTDStationServices, with_tiles), offsetof(OpenTTDStationServices, tile_next), offsetof(OpenTTDStationServices, accept_tile), offsetof(OpenTTDStationServices, acceptance_amounts), offsetof(OpenTTDStationServices, with_truncation), offsetof(OpenTTDStationServices, truncate_next), offsetof(OpenTTDStationServices, shared)});
	Layout(508, "OpenTTDLoadingWait", {sizeof(OpenTTDLoadingWait), alignof(OpenTTDLoadingWait), offsetof(OpenTTDLoadingWait, lateness), offsetof(OpenTTDLoadingWait, wait)});
	Layout(509, "OpenTTDOrderDestination", {sizeof(OpenTTDOrderDestination), alignof(OpenTTDOrderDestination), offsetof(OpenTTDOrderDestination, destination), offsetof(OpenTTDOrderDestination, is_station)});
	Layout(510, "OpenTTDTruncatedSource", {sizeof(OpenTTDTruncatedSource), alignof(OpenTTDTruncatedSource), offsetof(OpenTTDTruncatedSource, amount), offsetof(OpenTTDTruncatedSource, source), offsetof(OpenTTDTruncatedSource, present)});
	Layout(511, "OpenTTDCargoSource", {sizeof(OpenTTDCargoSource), alignof(OpenTTDCargoSource), offsetof(OpenTTDCargoSource, id), offsetof(OpenTTDCargoSource, type)});
	Layout(507, "OpenTTDLoadEngine", {sizeof(OpenTTDLoadEngine), alignof(OpenTTDLoadEngine), offsetof(OpenTTDLoadEngine, engine), offsetof(OpenTTDLoadEngine, amount)});
	Layout(506, "OpenTTDDeliveryRemaining", {sizeof(OpenTTDDeliveryRemaining), alignof(OpenTTDDeliveryRemaining), offsetof(OpenTTDDeliveryRemaining, remaining), offsetof(OpenTTDDeliveryRemaining, deliver), offsetof(OpenTTDDeliveryRemaining, capacity)});
	Layout(505, "OpenTTDCargoStored", {sizeof(OpenTTDCargoStored), alignof(OpenTTDCargoStored), offsetof(OpenTTDCargoStored, stored), offsetof(OpenTTDCargoStored, capacity)});
	Layout(504, "OpenTTDCargoReservation", {sizeof(OpenTTDCargoReservation), alignof(OpenTTDCargoReservation), offsetof(OpenTTDCargoReservation, mask), offsetof(OpenTTDCargoReservation, reserved), offsetof(OpenTTDCargoReservation, capacity), offsetof(OpenTTDCargoReservation, cargo)});
	Layout(515, "OpenTTDLoadingStation", {sizeof(OpenTTDLoadingStation), alignof(OpenTTDLoadingStation), offsetof(OpenTTDLoadingStation, native), offsetof(OpenTTDLoadingStation, base), offsetof(OpenTTDLoadingStation, state)});
	Layout(503, "OpenTTDCargoCapacity", {sizeof(OpenTTDCargoCapacity), alignof(OpenTTDCargoCapacity), offsetof(OpenTTDCargoCapacity, owner), offsetof(OpenTTDCargoCapacity, capacity), offsetof(OpenTTDCargoCapacity, cargo)});
	Layout(512, "OpenTTDLoadingControl", {sizeof(OpenTTDLoadingControl), alignof(OpenTTDLoadingControl), offsetof(OpenTTDLoadingControl, ticks), offsetof(OpenTTDLoadingControl, vehicle_type), offsetof(OpenTTDLoadingControl, unload_type), offsetof(OpenTTDLoadingControl, load_type), offsetof(OpenTTDLoadingControl, refit_cargo), offsetof(OpenTTDLoadingControl, refit), offsetof(OpenTTDLoadingControl, full_load), offsetof(OpenTTDLoadingControl, stop), offsetof(OpenTTDLoadingControl, gradual), offsetof(OpenTTDLoadingControl, improved)});
	Layout(513, "OpenTTDLoadingPart", {sizeof(OpenTTDLoadingPart), alignof(OpenTTDLoadingPart), offsetof(OpenTTDLoadingPart, capacity), offsetof(OpenTTDLoadingPart, cargo), offsetof(OpenTTDLoadingPart, vehicle_type), offsetof(OpenTTDLoadingPart, unloading)});
	Layout(514, "OpenTTDLoadingStats", {sizeof(OpenTTDLoadingStats), alignof(OpenTTDLoadingStats), offsetof(OpenTTDLoadingStats, stored), offsetof(OpenTTDLoadingStats, load_count), offsetof(OpenTTDLoadingStats, speed), offsetof(OpenTTDLoadingStats, age), offsetof(OpenTTDLoadingStats, capacity), offsetof(OpenTTDLoadingStats, cargo)});
	Layout(501, "OpenTTDStationExclusivity", {sizeof(OpenTTDStationExclusivity), alignof(OpenTTDStationExclusivity), offsetof(OpenTTDStationExclusivity, counter), offsetof(OpenTTDStationExclusivity, exclusive)});
	Layout(500, "OpenTTDRatingCounts", {sizeof(OpenTTDRatingCounts), alignof(OpenTTDRatingCounts), offsetof(OpenTTDRatingCounts, available), offsetof(OpenTTDRatingCounts, destinations)});
	Layout(320, "OpenTTDShipPosition", {sizeof(OpenTTDShipPosition), alignof(OpenTTDShipPosition), offsetof(OpenTTDShipPosition, x), offsetof(OpenTTDShipPosition, y), offsetof(OpenTTDShipPosition, old_tile), offsetof(OpenTTDShipPosition, new_tile)});
	Layout(321, "OpenTTDShipLeaves", {sizeof(OpenTTDShipLeaves), alignof(OpenTTDShipLeaves), offsetof(OpenTTDShipLeaves, depot_dir), offsetof(OpenTTDShipLeaves, depot_axis), offsetof(OpenTTDShipLeaves, is_depot), offsetof(OpenTTDShipLeaves, depot_index), offsetof(OpenTTDShipLeaves, wait_unbunch), offsetof(OpenTTDShipLeaves, chain_depot), offsetof(OpenTTDShipLeaves, servint), offsetof(OpenTTDShipLeaves, needs_service), offsetof(OpenTTDShipLeaves, max_distance), offsetof(OpenTTDShipLeaves, tile_valid), offsetof(OpenTTDShipLeaves, tile_type), offsetof(OpenTTDShipLeaves, water_class), offsetof(OpenTTDShipLeaves, lock_middle), offsetof(OpenTTDShipLeaves, lock_dir), offsetof(OpenTTDShipLeaves, tile_min_z), offsetof(OpenTTDShipLeaves, tile_max_z), offsetof(OpenTTDShipLeaves, track_status), offsetof(OpenTTDShipLeaves, offset), offsetof(OpenTTDShipLeaves, diag_between), offsetof(OpenTTDShipLeaves, dist_square), offsetof(OpenTTDShipLeaves, dist_manhattan), offsetof(OpenTTDShipLeaves, docking), offsetof(OpenTTDShipLeaves, dock), offsetof(OpenTTDShipLeaves, dock_water), offsetof(OpenTTDShipLeaves, station), offsetof(OpenTTDShipLeaves, industry_station), offsetof(OpenTTDShipLeaves, oilrig), offsetof(OpenTTDShipLeaves, station_use), offsetof(OpenTTDShipLeaves, station_xy), offsetof(OpenTTDShipLeaves, station_contains), offsetof(OpenTTDShipLeaves, station_dock), offsetof(OpenTTDShipLeaves, station_visits), offsetof(OpenTTDShipLeaves, visit_set), offsetof(OpenTTDShipLeaves, arrival), offsetof(OpenTTDShipLeaves, service), offsetof(OpenTTDShipLeaves, leave_unbunch), offsetof(OpenTTDShipLeaves, path_result), offsetof(OpenTTDShipLeaves, order_free), offsetof(OpenTTDShipLeaves, order_dummy), offsetof(OpenTTDShipLeaves, order_depot), offsetof(OpenTTDShipLeaves, order_leave), offsetof(OpenTTDShipLeaves, order_increment), offsetof(OpenTTDShipLeaves, timetable), offsetof(OpenTTDShipLeaves, position), offsetof(OpenTTDShipLeaves, start_dirty), offsetof(OpenTTDShipLeaves, depot_dirty), offsetof(OpenTTDShipLeaves, depot_invalidate), offsetof(OpenTTDShipLeaves, ships_dirty), offsetof(OpenTTDShipLeaves, details_dirty), offsetof(OpenTTDShipLeaves, age), offsetof(OpenTTDShipLeaves, economy_age), offsetof(OpenTTDShipLeaves, decrease_value), offsetof(OpenTTDShipLeaves, check_breakdown), offsetof(OpenTTDShipLeaves, check_orders), offsetof(OpenTTDShipLeaves, running_cost), offsetof(OpenTTDShipLeaves, cost_divisor), offsetof(OpenTTDShipLeaves, pay_running), offsetof(OpenTTDShipLeaves, speed_default), offsetof(OpenTTDShipLeaves, age_default), offsetof(OpenTTDShipLeaves, speed_frac), offsetof(OpenTTDShipLeaves, speed_property), offsetof(OpenTTDShipLeaves, age_property), offsetof(OpenTTDShipLeaves, update_visual), offsetof(OpenTTDShipLeaves, cache_invalidate), offsetof(OpenTTDShipLeaves, capacity), offsetof(OpenTTDShipLeaves, sprite_direction), offsetof(OpenTTDShipLeaves, tile_x), offsetof(OpenTTDShipLeaves, tile_y), offsetof(OpenTTDShipLeaves, build_flag), offsetof(OpenTTDShipLeaves, build_random), offsetof(OpenTTDShipLeaves, new_position), offsetof(OpenTTDShipLeaves, exit_dir), offsetof(OpenTTDShipLeaves, track_direction), offsetof(OpenTTDShipLeaves, tracks_reach), offsetof(OpenTTDShipLeaves, busy_tile), offsetof(OpenTTDShipLeaves, path_size), offsetof(OpenTTDShipLeaves, path_back), offsetof(OpenTTDShipLeaves, path_pop), offsetof(OpenTTDShipLeaves, path_clear), offsetof(OpenTTDShipLeaves, enter_tile), offsetof(OpenTTDShipLeaves, enter_depot), offsetof(OpenTTDShipLeaves, process_orders), offsetof(OpenTTDShipLeaves, loading), offsetof(OpenTTDShipLeaves, begin_loading), offsetof(OpenTTDShipLeaves, breakdown), offsetof(OpenTTDShipLeaves, viewport), offsetof(OpenTTDShipLeaves, base_viewport), offsetof(OpenTTDShipLeaves, visual), offsetof(OpenTTDShipLeaves, cache), offsetof(OpenTTDShipLeaves, play_sound), offsetof(OpenTTDShipLeaves, yapf_reverse), offsetof(OpenTTDShipLeaves, yapf_choose), offsetof(OpenTTDShipLeaves, update_delta), offsetof(OpenTTDShipLeaves, build_owner), offsetof(OpenTTDShipLeaves, build_z), offsetof(OpenTTDShipLeaves, build_properties), offsetof(OpenTTDShipLeaves, build_dates), offsetof(OpenTTDShipLeaves, build_acceleration), offsetof(OpenTTDShipLeaves, build_prototype), offsetof(OpenTTDShipLeaves, build_interval_percent), offsetof(OpenTTDShipLeaves, build_capacity), offsetof(OpenTTDShipLeaves, set_tile), offsetof(OpenTTDShipLeaves, set_x), offsetof(OpenTTDShipLeaves, set_y), offsetof(OpenTTDShipLeaves, set_z), offsetof(OpenTTDShipLeaves, set_direction), offsetof(OpenTTDShipLeaves, set_speed), offsetof(OpenTTDShipLeaves, set_tick), offsetof(OpenTTDShipLeaves, set_running), offsetof(OpenTTDShipLeaves, set_day), offsetof(OpenTTDShipLeaves, set_order_time), offsetof(OpenTTDShipLeaves, set_progress), offsetof(OpenTTDShipLeaves, set_last_station), offsetof(OpenTTDShipLeaves, set_hidden), offsetof(OpenTTDShipLeaves, set_max_speed), offsetof(OpenTTDShipLeaves, set_cargo_age), offsetof(OpenTTDShipLeaves, set_dest), offsetof(OpenTTDShipLeaves, tile), offsetof(OpenTTDShipLeaves, dest), offsetof(OpenTTDShipLeaves, x), offsetof(OpenTTDShipLeaves, y), offsetof(OpenTTDShipLeaves, z), offsetof(OpenTTDShipLeaves, direction), offsetof(OpenTTDShipLeaves, speed), offsetof(OpenTTDShipLeaves, tick), offsetof(OpenTTDShipLeaves, running), offsetof(OpenTTDShipLeaves, day), offsetof(OpenTTDShipLeaves, order_time), offsetof(OpenTTDShipLeaves, progress), offsetof(OpenTTDShipLeaves, status), offsetof(OpenTTDShipLeaves, owner), offsetof(OpenTTDShipLeaves, last_station), offsetof(OpenTTDShipLeaves, order_destination), offsetof(OpenTTDShipLeaves, order_type), offsetof(OpenTTDShipLeaves, order_max_speed), offsetof(OpenTTDShipLeaves, acceleration), offsetof(OpenTTDShipLeaves, max_speed), offsetof(OpenTTDShipLeaves, state_owner), offsetof(OpenTTDShipLeaves, patch), offsetof(OpenTTDShipLeaves, neighbours), offsetof(OpenTTDShipLeaves, next_depot)});
	Layout(322, "OpenTTDShipReverseResult", {sizeof(OpenTTDShipReverseResult), alignof(OpenTTDShipReverseResult), offsetof(OpenTTDShipReverseResult, reverse), offsetof(OpenTTDShipReverseResult, trackdir)});
	Layout(323, "OpenTTDShipDepot", {sizeof(OpenTTDShipDepot), alignof(OpenTTDShipDepot), offsetof(OpenTTDShipDepot, id), offsetof(OpenTTDShipDepot, tile), offsetof(OpenTTDShipDepot, owner), offsetof(OpenTTDShipDepot, ship)});
	Layout(324, "OpenTTDShipTrackResult", {sizeof(OpenTTDShipTrackResult), alignof(OpenTTDShipTrackResult), offsetof(OpenTTDShipTrackResult, track), offsetof(OpenTTDShipTrackResult, found)});
	Layout(325, "OpenTTDShipDepotResult", {sizeof(OpenTTDShipDepotResult), alignof(OpenTTDShipDepotResult), offsetof(OpenTTDShipDepotResult, tile), offsetof(OpenTTDShipDepotResult, id), offsetof(OpenTTDShipDepotResult, valid)});
	Layout(0, "OpenTTDRustIntegerResult", {sizeof(OpenTTDRustIntegerResult), alignof(OpenTTDRustIntegerResult), offsetof(OpenTTDRustIntegerResult, value_bits), offsetof(OpenTTDRustIntegerResult, length), offsetof(OpenTTDRustIntegerResult, error_offset), offsetof(OpenTTDRustIntegerResult, error_length), offsetof(OpenTTDRustIntegerResult, error_kind)});
	Layout(1, "OpenTTDRustUtf8Encoded", {sizeof(OpenTTDRustUtf8Encoded), alignof(OpenTTDRustUtf8Encoded), offsetof(OpenTTDRustUtf8Encoded, bytes), offsetof(OpenTTDRustUtf8Encoded, length)});
	Layout(2, "OpenTTDRustUtf8Decoded", {sizeof(OpenTTDRustUtf8Decoded), alignof(OpenTTDRustUtf8Decoded), offsetof(OpenTTDRustUtf8Decoded, length), offsetof(OpenTTDRustUtf8Decoded, codepoint)});
	Layout(3, "OpenTTDRustLittleEndian", {sizeof(OpenTTDRustLittleEndian), alignof(OpenTTDRustLittleEndian), offsetof(OpenTTDRustLittleEndian, bytes)});
	Layout(4, "OpenTTDRustFormattedInteger", {sizeof(OpenTTDRustFormattedInteger), alignof(OpenTTDRustFormattedInteger), offsetof(OpenTTDRustFormattedInteger, bytes), offsetof(OpenTTDRustFormattedInteger, length)});
	Layout(5, "OpenTTDRustAlternatingState", {sizeof(OpenTTDRustAlternatingState), alignof(OpenTTDRustAlternatingState), offsetof(OpenTTDRustAlternatingState, position), offsetof(OpenTTDRustAlternatingState, next_after), offsetof(OpenTTDRustAlternatingState, current_after)});
	Layout(6, "OpenTTDRustAlternatingStep", {sizeof(OpenTTDRustAlternatingStep), alignof(OpenTTDRustAlternatingStep), offsetof(OpenTTDRustAlternatingStep, state), offsetof(OpenTTDRustAlternatingStep, movement)});
	Layout(7, "OpenTTDRustConsumerBound", {sizeof(OpenTTDRustConsumerBound), alignof(OpenTTDRustConsumerBound), offsetof(OpenTTDRustConsumerBound, length), offsetof(OpenTTDRustConsumerBound, position), offsetof(OpenTTDRustConsumerBound, shortfall)});
	Layout(8, "OpenTTDRustConsumerByte", {sizeof(OpenTTDRustConsumerByte), alignof(OpenTTDRustConsumerByte), offsetof(OpenTTDRustConsumerByte, value_bits), offsetof(OpenTTDRustConsumerByte, length)});
	Layout(9, "OpenTTDRustConsumerMatch", {sizeof(OpenTTDRustConsumerMatch), alignof(OpenTTDRustConsumerMatch), offsetof(OpenTTDRustConsumerMatch, length), offsetof(OpenTTDRustConsumerMatch, matched)});
	Layout(10, "OpenTTDRustConsumerSeparator", {sizeof(OpenTTDRustConsumerSeparator), alignof(OpenTTDRustConsumerSeparator), offsetof(OpenTTDRustConsumerSeparator, result_length), offsetof(OpenTTDRustConsumerSeparator, consumed_length)});
	Layout(11, "OpenTTDRustEncodedParameter", {sizeof(OpenTTDRustEncodedParameter), alignof(OpenTTDRustEncodedParameter), offsetof(OpenTTDRustEncodedParameter, value), offsetof(OpenTTDRustEncodedParameter, bytes), offsetof(OpenTTDRustEncodedParameter, length), offsetof(OpenTTDRustEncodedParameter, kind)});
	Layout(12, "OpenTTDRustEncodedView", {sizeof(OpenTTDRustEncodedView), alignof(OpenTTDRustEncodedView), offsetof(OpenTTDRustEncodedView, bytes), offsetof(OpenTTDRustEncodedView, length), offsetof(OpenTTDRustEncodedView, diagnostic_count), offsetof(OpenTTDRustEncodedView, assertion_offset), offsetof(OpenTTDRustEncodedView, assertion_length), offsetof(OpenTTDRustEncodedView, assertion_codepoint), offsetof(OpenTTDRustEncodedView, apply), offsetof(OpenTTDRustEncodedView, assertion_kind)});
	Layout(13, "OpenTTDRustEncodedDiagnostic", {sizeof(OpenTTDRustEncodedDiagnostic), alignof(OpenTTDRustEncodedDiagnostic), offsetof(OpenTTDRustEncodedDiagnostic, offset), offsetof(OpenTTDRustEncodedDiagnostic, length), offsetof(OpenTTDRustEncodedDiagnostic, tail_length), offsetof(OpenTTDRustEncodedDiagnostic, kind)});
	Layout(14, "OpenTTDRustSpiralState", {sizeof(OpenTTDRustSpiralState), alignof(OpenTTDRustSpiralState), offsetof(OpenTTDRustSpiralState, max_radius), offsetof(OpenTTDRustSpiralState, extent), offsetof(OpenTTDRustSpiralState, cur_radius), offsetof(OpenTTDRustSpiralState, position), offsetof(OpenTTDRustSpiralState, x), offsetof(OpenTTDRustSpiralState, y), offsetof(OpenTTDRustSpiralState, direction)});
	Layout(15, "OpenTTDRustByteTrim", {sizeof(OpenTTDRustByteTrim), alignof(OpenTTDRustByteTrim), offsetof(OpenTTDRustByteTrim, offset), offsetof(OpenTTDRustByteTrim, length)});
	Layout(16, "OpenTTDRustHistoryDescriptor", {sizeof(OpenTTDRustHistoryDescriptor), alignof(OpenTTDRustHistoryDescriptor), offsetof(OpenTTDRustHistoryDescriptor, child), offsetof(OpenTTDRustHistoryDescriptor, periods), offsetof(OpenTTDRustHistoryDescriptor, records), offsetof(OpenTTDRustHistoryDescriptor, first), offsetof(OpenTTDRustHistoryDescriptor, last), offsetof(OpenTTDRustHistoryDescriptor, division), offsetof(OpenTTDRustHistoryDescriptor, total_division), offsetof(OpenTTDRustHistoryDescriptor, child_periods), offsetof(OpenTTDRustHistoryDescriptor, child_division)});
	Layout(17, "OpenTTDRustHistoryStep", {sizeof(OpenTTDRustHistoryStep), alignof(OpenTTDRustHistoryStep), offsetof(OpenTTDRustHistoryStep, kind), offsetof(OpenTTDRustHistoryStep, count), offsetof(OpenTTDRustHistoryStep, first), offsetof(OpenTTDRustHistoryStep, last), offsetof(OpenTTDRustHistoryStep, target), offsetof(OpenTTDRustHistoryStep, token), offsetof(OpenTTDRustHistoryStep, value)});
	Layout(18, "OpenTTDCargoCollector", {sizeof(OpenTTDCargoCollector), alignof(OpenTTDCargoCollector), offsetof(OpenTTDCargoCollector, amount), offsetof(OpenTTDCargoCollector, previous), offsetof(OpenTTDCargoCollector, last_key), offsetof(OpenTTDCargoCollector, other), offsetof(OpenTTDCargoCollector, origin), offsetof(OpenTTDCargoCollector, selector), offsetof(OpenTTDCargoCollector, finalized)});
	Layout(19, "OpenTTDCryptoLeaves", {sizeof(OpenTTDCryptoLeaves), alignof(OpenTTDCryptoLeaves), offsetof(OpenTTDCryptoLeaves, wipe), offsetof(OpenTTDCryptoLeaves, verify16)});
	Layout(20, "OpenTTDPolyLayout", {sizeof(OpenTTDPolyLayout), alignof(OpenTTDPolyLayout), offsetof(OpenTTDPolyLayout, size), offsetof(OpenTTDPolyLayout, alignment), offsetof(OpenTTDPolyLayout, c), offsetof(OpenTTDPolyLayout, c_idx), offsetof(OpenTTDPolyLayout, r), offsetof(OpenTTDPolyLayout, pad), offsetof(OpenTTDPolyLayout, h)});
	Layout(21, "OpenTTDAeadLayout", {sizeof(OpenTTDAeadLayout), alignof(OpenTTDAeadLayout), offsetof(OpenTTDAeadLayout, size), offsetof(OpenTTDAeadLayout, alignment), offsetof(OpenTTDAeadLayout, counter), offsetof(OpenTTDAeadLayout, key), offsetof(OpenTTDAeadLayout, nonce)});
	Layout(22, "OpenTTDBlake2bLayout", {sizeof(OpenTTDBlake2bLayout), alignof(OpenTTDBlake2bLayout), offsetof(OpenTTDBlake2bLayout, size), offsetof(OpenTTDBlake2bLayout, alignment), offsetof(OpenTTDBlake2bLayout, hash), offsetof(OpenTTDBlake2bLayout, input_offset), offsetof(OpenTTDBlake2bLayout, input), offsetof(OpenTTDBlake2bLayout, input_idx), offsetof(OpenTTDBlake2bLayout, hash_size)});
	Layout(23, "OpenTTDPacketState", {sizeof(OpenTTDPacketState), alignof(OpenTTDPacketState), offsetof(OpenTTDPacketState, limit), offsetof(OpenTTDPacketState, position)});
	Layout(24, "OpenTTDPacketFrame", {sizeof(OpenTTDPacketFrame), alignof(OpenTTDPacketFrame), offsetof(OpenTTDPacketFrame, message), offsetof(OpenTTDPacketFrame, payload)});
	Layout(25, "OpenTTDX25519Leaves", {sizeof(OpenTTDX25519Leaves), alignof(OpenTTDX25519Leaves), offsetof(OpenTTDX25519Leaves, wipe), offsetof(OpenTTDX25519Leaves, verify32)});
	Layout(26, "OpenTTDValidationStep", {sizeof(OpenTTDValidationStep), alignof(OpenTTDValidationStep), offsetof(OpenTTDValidationStep, consumed), offsetof(OpenTTDValidationStep, output), offsetof(OpenTTDValidationStep, stopped)});
	Layout(27, "OpenTTDInplaceWrite", {sizeof(OpenTTDInplaceWrite), alignof(OpenTTDInplaceWrite), offsetof(OpenTTDInplaceWrite, position), offsetof(OpenTTDInplaceWrite, accepted)});
	Layout(34, "OpenTTDLinkGraphNode", {sizeof(OpenTTDLinkGraphNode), alignof(OpenTTDLinkGraphNode), offsetof(OpenTTDLinkGraphNode, supply), offsetof(OpenTTDLinkGraphNode, demand), offsetof(OpenTTDLinkGraphNode, station), offsetof(OpenTTDLinkGraphNode, x), offsetof(OpenTTDLinkGraphNode, y), offsetof(OpenTTDLinkGraphNode, edge_begin), offsetof(OpenTTDLinkGraphNode, edge_count)});
	Layout(35, "OpenTTDLinkGraphEdge", {sizeof(OpenTTDLinkGraphEdge), alignof(OpenTTDLinkGraphEdge), offsetof(OpenTTDLinkGraphEdge, capacity), offsetof(OpenTTDLinkGraphEdge, travel_time), offsetof(OpenTTDLinkGraphEdge, dest)});
	Layout(36, "OpenTTDLinkGraphSettings", {sizeof(OpenTTDLinkGraphSettings), alignof(OpenTTDLinkGraphSettings), offsetof(OpenTTDLinkGraphSettings, accuracy), offsetof(OpenTTDLinkGraphSettings, demand_distance), offsetof(OpenTTDLinkGraphSettings, demand_size), offsetof(OpenTTDLinkGraphSettings, saturation), offsetof(OpenTTDLinkGraphSettings, distribution), offsetof(OpenTTDLinkGraphSettings, express), offsetof(OpenTTDLinkGraphSettings, map_max_x), offsetof(OpenTTDLinkGraphSettings, map_max_y), offsetof(OpenTTDLinkGraphSettings, runtime)});
	Layout(38, "OpenTTDTreeServices", {sizeof(OpenTTDTreeServices), alignof(OpenTTDTreeServices), offsetof(OpenTTDTreeServices, plant_observation), offsetof(OpenTTDTreeServices, tree_observation), offsetof(OpenTTDTreeServices, tree_count), offsetof(OpenTTDTreeServices, clear_observation), offsetof(OpenTTDTreeServices, snow_line), offsetof(OpenTTDTreeServices, sin), offsetof(OpenTTDTreeServices, cos), offsetof(OpenTTDTreeServices, make_tree), offsetof(OpenTTDTreeServices, set_ground_density), offsetof(OpenTTDTreeServices, add_count), offsetof(OpenTTDTreeServices, add_growth), offsetof(OpenTTDTreeServices, set_growth), offsetof(OpenTTDTreeServices, make_clear), offsetof(OpenTTDTreeServices, make_shore), offsetof(OpenTTDTreeServices, make_snow), offsetof(OpenTTDTreeServices, clear_neighbour_flooding), offsetof(OpenTTDTreeServices, tile_loop_water), offsetof(OpenTTDTreeServices, ambient), offsetof(OpenTTDTreeServices, play_sound), offsetof(OpenTTDTreeServices, progress), offsetof(OpenTTDTreeServices, progress_total), offsetof(OpenTTDTreeServices, clear_square), offsetof(OpenTTDTreeServices, town_rating), offsetof(OpenTTDTreeServices, command_begin), offsetof(OpenTTDTreeServices, command_tile), offsetof(OpenTTDTreeServices, command_next), offsetof(OpenTTDTreeServices, command_debit), offsetof(OpenTTDTreeServices, landscape_clear)});
	Layout(28, "OpenTTDTreePlantObservation", {sizeof(OpenTTDTreePlantObservation), alignof(OpenTTDTreePlantObservation), offsetof(OpenTTDTreePlantObservation, tile_type), offsetof(OpenTTDTreePlantObservation, bridge), offsetof(OpenTTDTreePlantObservation, ground), offsetof(OpenTTDTreePlantObservation, density), offsetof(OpenTTDTreePlantObservation, snow), offsetof(OpenTTDTreePlantObservation, coast), offsetof(OpenTTDTreePlantObservation, zone)});
	Layout(31, "OpenTTDTreeObservation", {sizeof(OpenTTDTreeObservation), alignof(OpenTTDTreeObservation), offsetof(OpenTTDTreeObservation, ground), offsetof(OpenTTDTreeObservation, density), offsetof(OpenTTDTreeObservation, species), offsetof(OpenTTDTreeObservation, count), offsetof(OpenTTDTreeObservation, growth), offsetof(OpenTTDTreeObservation, zone)});
	Layout(386, "OpenTTDTreeClearObservation", {sizeof(OpenTTDTreeClearObservation), alignof(OpenTTDTreeClearObservation), offsetof(OpenTTDTreeClearObservation, count), offsetof(OpenTTDTreeClearObservation, species)});
	Layout(32, "OpenTTDTreeLoopSettings", {sizeof(OpenTTDTreeLoopSettings), alignof(OpenTTDTreeLoopSettings), offsetof(OpenTTDTreeLoopSettings, tick_counter), offsetof(OpenTTDTreeLoopSettings, size_x), offsetof(OpenTTDTreeLoopSettings, climate), offsetof(OpenTTDTreeLoopSettings, extra), offsetof(OpenTTDTreeLoopSettings, ambient)});
	Layout(33, "OpenTTDTreeTickSettings", {sizeof(OpenTTDTreeTickSettings), alignof(OpenTTDTreeTickSettings), offsetof(OpenTTDTreeTickSettings, tick_counter), offsetof(OpenTTDTreeTickSettings, size), offsetof(OpenTTDTreeTickSettings, climate), offsetof(OpenTTDTreeTickSettings, extra)});
	Layout(43, "OpenTTDTreeGenerateSettings", {sizeof(OpenTTDTreeGenerateSettings), alignof(OpenTTDTreeGenerateSettings), offsetof(OpenTTDTreeGenerateSettings, size), offsetof(OpenTTDTreeGenerateSettings, size_x), offsetof(OpenTTDTreeGenerateSettings, size_y), offsetof(OpenTTDTreeGenerateSettings, climate), offsetof(OpenTTDTreeGenerateSettings, placer), offsetof(OpenTTDTreeGenerateSettings, height_limit), offsetof(OpenTTDTreeGenerateSettings, editor), offsetof(OpenTTDTreeGenerateSettings, freeform_edges)});
	Layout(57, "OpenTTDTreeCommandSettings", {sizeof(OpenTTDTreeCommandSettings), alignof(OpenTTDTreeCommandSettings), offsetof(OpenTTDTreeCommandSettings, build_price), offsetof(OpenTTDTreeCommandSettings, size), offsetof(OpenTTDTreeCommandSettings, climate), offsetof(OpenTTDTreeCommandSettings, editor), offsetof(OpenTTDTreeCommandSettings, execute), offsetof(OpenTTDTreeCommandSettings, company_valid)});
	Layout(58, "OpenTTDTreeCommandResult", {sizeof(OpenTTDTreeCommandResult), alignof(OpenTTDTreeCommandResult), offsetof(OpenTTDTreeCommandResult, cost), offsetof(OpenTTDTreeCommandResult, status), offsetof(OpenTTDTreeCommandResult, message)});
	Layout(59, "OpenTTDTreeLandscapeClear", {sizeof(OpenTTDTreeLandscapeClear), alignof(OpenTTDTreeLandscapeClear), offsetof(OpenTTDTreeLandscapeClear, cost), offsetof(OpenTTDTreeLandscapeClear, failed)});
	Layout(820, "OpenTTDSignalServices", {sizeof(OpenTTDSignalServices), alignof(OpenTTDSignalServices), offsetof(OpenTTDSignalServices, assertions), offsetof(OpenTTDSignalServices, assert_condition), offsetof(OpenTTDSignalServices, assert_wormhole_start), offsetof(OpenTTDSignalServices, train_on_tile), offsetof(OpenTTDSignalServices, train_on_tracks), offsetof(OpenTTDSignalServices, overflow_tbu), offsetof(OpenTTDSignalServices, overflow_tbd), offsetof(OpenTTDSignalServices, overflow_global), offsetof(OpenTTDSignalServices, assert_company)});
	Layout(821, "OpenTTDRailMapServices", {sizeof(OpenTTDRailMapServices), alignof(OpenTTDRailMapServices), offsetof(OpenTTDRailMapServices, traversal), offsetof(OpenTTDRailMapServices, signal_presence), offsetof(OpenTTDRailMapServices, signal_update), offsetof(OpenTTDRailMapServices, station_admission), offsetof(OpenTTDRailMapServices, crossing_admission), offsetof(OpenTTDRailMapServices, wormhole_admission), offsetof(OpenTTDRailMapServices, is_rail_depot), offsetof(OpenTTDRailMapServices, rail_depot_direction), offsetof(OpenTTDRailMapServices, has_signal_trackdir), offsetof(OpenTTDRailMapServices, signal_state), offsetof(OpenTTDRailMapServices, set_signal_state), offsetof(OpenTTDRailMapServices, other_tunnel_bridge_end), offsetof(OpenTTDRailMapServices, rail_track_status)});
	Layout(822, "OpenTTDSignalSnapshot", {sizeof(OpenTTDSignalSnapshot), alignof(OpenTTDSignalSnapshot), offsetof(OpenTTDSignalSnapshot, tbu_count), offsetof(OpenTTDSignalSnapshot, tbd_count), offsetof(OpenTTDSignalSnapshot, global_count), offsetof(OpenTTDSignalSnapshot, overflow), offsetof(OpenTTDSignalSnapshot, last_owner), offsetof(OpenTTDSignalSnapshot, tbu), offsetof(OpenTTDSignalSnapshot, tbd), offsetof(OpenTTDSignalSnapshot, global)});
	Layout(823, "OpenTTDSignalItem", {sizeof(OpenTTDSignalItem), alignof(OpenTTDSignalItem), offsetof(OpenTTDSignalItem, tile), offsetof(OpenTTDSignalItem, dir)});
	Layout(824, "OpenTTDRailTraversal", {sizeof(OpenTTDRailTraversal), alignof(OpenTTDRailTraversal), offsetof(OpenTTDRailTraversal, depot), offsetof(OpenTTDRailTraversal, tracks)});
	Layout(825, "OpenTTDRailSignalPresence", {sizeof(OpenTTDRailSignalPresence), alignof(OpenTTDRailSignalPresence), offsetof(OpenTTDRailSignalPresence, signals), offsetof(OpenTTDRailSignalPresence, track), offsetof(OpenTTDRailSignalPresence, kind), offsetof(OpenTTDRailSignalPresence, forward), offsetof(OpenTTDRailSignalPresence, reverse)});
	Layout(826, "OpenTTDRailSignalUpdate", {sizeof(OpenTTDRailSignalUpdate), alignof(OpenTTDRailSignalUpdate), offsetof(OpenTTDRailSignalUpdate, kind), offsetof(OpenTTDRailSignalUpdate, reservations)});
	Layout(827, "OpenTTDRailStationAdmission", {sizeof(OpenTTDRailStationAdmission), alignof(OpenTTDRailStationAdmission), offsetof(OpenTTDRailStationAdmission, rail), offsetof(OpenTTDRailStationAdmission, owner), offsetof(OpenTTDRailStationAdmission, axis), offsetof(OpenTTDRailStationAdmission, blocked)});
	Layout(828, "OpenTTDRailCrossingAdmission", {sizeof(OpenTTDRailCrossingAdmission), alignof(OpenTTDRailCrossingAdmission), offsetof(OpenTTDRailCrossingAdmission, crossing), offsetof(OpenTTDRailCrossingAdmission, owner), offsetof(OpenTTDRailCrossingAdmission, axis)});
	Layout(829, "OpenTTDRailWormholeAdmission", {sizeof(OpenTTDRailWormholeAdmission), alignof(OpenTTDRailWormholeAdmission), offsetof(OpenTTDRailWormholeAdmission, owner), offsetof(OpenTTDRailWormholeAdmission, transport), offsetof(OpenTTDRailWormholeAdmission, direction)});
	Layout(831, "OpenTTDSignalSetTrace", {sizeof(OpenTTDSignalSetTrace), alignof(OpenTTDSignalSetTrace), offsetof(OpenTTDSignalSetTrace, duplicate_count), offsetof(OpenTTDSignalSetTrace, duplicate_pop), offsetof(OpenTTDSignalSetTrace, full_failed), offsetof(OpenTTDSignalSetTrace, overflow_full), offsetof(OpenTTDSignalSetTrace, first_after_remove), offsetof(OpenTTDSignalSetTrace, refill_succeeded), offsetof(OpenTTDSignalSetTrace, refill_pop), offsetof(OpenTTDSignalSetTrace, drained_count), offsetof(OpenTTDSignalSetTrace, overflow_drained), offsetof(OpenTTDSignalSetTrace, overflow_reset)});
	Layout(830, "OpenTTDSignalContext", {sizeof(OpenTTDSignalContext), alignof(OpenTTDSignalContext), offsetof(OpenTTDSignalContext, shared), offsetof(OpenTTDSignalContext, rail), offsetof(OpenTTDSignalContext, native), offsetof(OpenTTDSignalContext, log_x), sizeof(decltype(OpenTTDSignalContext::log_x))});
	Layout(42, "OpenTTDSharedServices", {sizeof(OpenTTDSharedServices), alignof(OpenTTDSharedServices), offsetof(OpenTTDSharedServices, context), offsetof(OpenTTDSharedServices, random), offsetof(OpenTTDSharedServices, industry), offsetof(OpenTTDSharedServices, tile_type), offsetof(OpenTTDSharedServices, bridge_above), offsetof(OpenTTDSharedServices, tropic_zone), offsetof(OpenTTDSharedServices, tile_z), offsetof(OpenTTDSharedServices, tile_slope), offsetof(OpenTTDSharedServices, mark_dirty), offsetof(OpenTTDSharedServices, set_tropic_zone), offsetof(OpenTTDSharedServices, tile_owner), offsetof(OpenTTDSharedServices, station_index), offsetof(OpenTTDSharedServices, depot_index), offsetof(OpenTTDSharedServices, is_level_crossing), offsetof(OpenTTDSharedServices, tunnel_bridge_direction), offsetof(OpenTTDSharedServices, bridge_speed), offsetof(OpenTTDSharedServices, is_water_tile), offsetof(OpenTTDSharedServices, company_valid), offsetof(OpenTTDSharedServices, next_company), offsetof(OpenTTDSharedServices, current_company), offsetof(OpenTTDSharedServices, closest_town), offsetof(OpenTTDSharedServices, town_owner), offsetof(OpenTTDSharedServices, tile_valid), offsetof(OpenTTDSharedServices, distance_from_edge), offsetof(OpenTTDSharedServices, tile_max_z), offsetof(OpenTTDSharedServices, tile_height), offsetof(OpenTTDSharedServices, bridge_axis), offsetof(OpenTTDSharedServices, has_water_ground), offsetof(OpenTTDSharedServices, is_sea), offsetof(OpenTTDSharedServices, is_plain_rail), offsetof(OpenTTDSharedServices, is_bridge), offsetof(OpenTTDSharedServices, tunnel_transport), offsetof(OpenTTDSharedServices, other_tunnel_end), offsetof(OpenTTDSharedServices, is_road_depot), offsetof(OpenTTDSharedServices, road_depot_direction), offsetof(OpenTTDSharedServices, is_normal_road), offsetof(OpenTTDSharedServices, disallowed_road_directions), offsetof(OpenTTDSharedServices, is_bay_stop), offsetof(OpenTTDSharedServices, bay_stop_direction), offsetof(OpenTTDSharedServices, is_drive_through_stop), offsetof(OpenTTDSharedServices, drive_through_axis), offsetof(OpenTTDSharedServices, is_any_road_stop), offsetof(OpenTTDSharedServices, road_type), offsetof(OpenTTDSharedServices, has_road_type), offsetof(OpenTTDSharedServices, road_owner), offsetof(OpenTTDSharedServices, town_index), offsetof(OpenTTDSharedServices, road_bits), offsetof(OpenTTDSharedServices, snow_line), offsetof(OpenTTDSharedServices, calendar_year), offsetof(OpenTTDSharedServices, map_log_x), offsetof(OpenTTDSharedServices, map_max_x), offsetof(OpenTTDSharedServices, map_max_y), offsetof(OpenTTDSharedServices, plane_speed), offsetof(OpenTTDSharedServices, no_jetcrash), offsetof(OpenTTDSharedServices, plane_crashes), offsetof(OpenTTDSharedServices, service_at_helipad), offsetof(OpenTTDSharedServices, disaster_sound), offsetof(OpenTTDSharedServices, economy_date), offsetof(OpenTTDSharedServices, calendar_date), offsetof(OpenTTDSharedServices, slope)});
	Layout(37, "OpenTTDLinkGraphShare", {sizeof(OpenTTDLinkGraphShare), alignof(OpenTTDLinkGraphShare), offsetof(OpenTTDLinkGraphShare, node), offsetof(OpenTTDLinkGraphShare, origin), offsetof(OpenTTDLinkGraphShare, via), offsetof(OpenTTDLinkGraphShare, cumulative), offsetof(OpenTTDLinkGraphShare, unrestricted), offsetof(OpenTTDLinkGraphShare, has_share)});
	Layout(39, "OpenTTDEffectView", {sizeof(OpenTTDEffectView), alignof(OpenTTDEffectView), offsetof(OpenTTDEffectView, x), offsetof(OpenTTDEffectView, y), offsetof(OpenTTDEffectView, z), offsetof(OpenTTDEffectView, sprite), offsetof(OpenTTDEffectView, progress), offsetof(OpenTTDEffectView, spritenum), offsetof(OpenTTDEffectView, subtype), offsetof(OpenTTDEffectView, ambient)});
	Layout(41, "OpenTTDEffectLeaves", {sizeof(OpenTTDEffectLeaves), alignof(OpenTTDEffectLeaves), offsetof(OpenTTDEffectLeaves, observe), offsetof(OpenTTDEffectLeaves, write), offsetof(OpenTTDEffectLeaves, viewport), offsetof(OpenTTDEffectLeaves, sound), offsetof(OpenTTDEffectLeaves, animated)});
	// ABI reservations: train 380-385; tree additions 386-389; fleet 390-399; company 400-429; town 430-459; shared vehicle/road 460-499; station 500-559; industry 560-619; shared CommandCost 620-629 (town-owned). Trees retain existing low IDs.
	Layout(461, "OpenTTDVehicleNewPosition", {sizeof(OpenTTDVehicleNewPosition), alignof(OpenTTDVehicleNewPosition), offsetof(OpenTTDVehicleNewPosition, x), offsetof(OpenTTDVehicleNewPosition, y), offsetof(OpenTTDVehicleNewPosition, tile)});
	Layout(462, "OpenTTDVehicleBreakdown", {sizeof(OpenTTDVehicleBreakdown), alignof(OpenTTDVehicleBreakdown), offsetof(OpenTTDVehicleBreakdown, broken), offsetof(OpenTTDVehicleBreakdown, status)});
	Layout(460, "OpenTTDVehicleServices", {sizeof(OpenTTDVehicleServices), alignof(OpenTTDVehicleServices), offsetof(OpenTTDVehicleServices, write_counters), offsetof(OpenTTDVehicleServices, handle_breakdown), offsetof(OpenTTDVehicleServices, waiting_for_unbunching), offsetof(OpenTTDVehicleServices, leave_unbunching_depot), offsetof(OpenTTDVehicleServices, reset_depot_unbunching), offsetof(OpenTTDVehicleServices, service_in_depot), offsetof(OpenTTDVehicleServices, needs_automatic_servicing), offsetof(OpenTTDVehicleServices, enter_depot), offsetof(OpenTTDVehicleServices, pathfinding_result), offsetof(OpenTTDVehicleServices, new_position), offsetof(OpenTTDVehicleServices, move_then_status), offsetof(OpenTTDVehicleServices, base_viewport), offsetof(OpenTTDVehicleServices, property), offsetof(OpenTTDVehicleServices, length_callback), offsetof(OpenTTDVehicleServices, unknown_length_result), offsetof(OpenTTDVehicleServices, length_changed), offsetof(OpenTTDVehicleServices, play_start_sound), offsetof(OpenTTDVehicleServices, play_sound), offsetof(OpenTTDVehicleServices, large_explosion), offsetof(OpenTTDVehicleServices, order_free), offsetof(OpenTTDVehicleServices, order_dummy), offsetof(OpenTTDVehicleServices, start_stop_dirty), offsetof(OpenTTDVehicleServices, destroy), offsetof(OpenTTDVehicleServices, age), offsetof(OpenTTDVehicleServices, economy_age), offsetof(OpenTTDVehicleServices, decrease_value), offsetof(OpenTTDVehicleServices, check_breakdown), offsetof(OpenTTDVehicleServices, check_orders), offsetof(OpenTTDVehicleServices, write_day), offsetof(OpenTTDVehicleServices, write_direction), offsetof(OpenTTDVehicleServices, write_speed), offsetof(OpenTTDVehicleServices, last_station), offsetof(OpenTTDVehicleServices, write_last_station), offsetof(OpenTTDVehicleServices, write_dest), offsetof(OpenTTDVehicleServices, write_progress), offsetof(OpenTTDVehicleServices, write_max_speed), offsetof(OpenTTDVehicleServices, previous_tile), offsetof(OpenTTDVehicleServices, detach_last), offsetof(OpenTTDVehicleServices, next), offsetof(OpenTTDVehicleServices, engine_id), offsetof(OpenTTDVehicleServices, engine_for_length), offsetof(OpenTTDVehicleServices, write_tick), offsetof(OpenTTDVehicleServices, direction), offsetof(OpenTTDVehicleServices, status), offsetof(OpenTTDVehicleServices, tile), offsetof(OpenTTDVehicleServices, progress), offsetof(OpenTTDVehicleServices, dest_tile), offsetof(OpenTTDVehicleServices, order_type), offsetof(OpenTTDVehicleServices, order_destination), offsetof(OpenTTDVehicleServices, order_nonstop), offsetof(OpenTTDVehicleServices, subtype), offsetof(OpenTTDVehicleServices, x), offsetof(OpenTTDVehicleServices, y), offsetof(OpenTTDVehicleServices, z), offsetof(OpenTTDVehicleServices, tick), offsetof(OpenTTDVehicleServices, owner), offsetof(OpenTTDVehicleServices, speed), offsetof(OpenTTDVehicleServices, subspeed), offsetof(OpenTTDVehicleServices, running), offsetof(OpenTTDVehicleServices, order_time), offsetof(OpenTTDVehicleServices, vehicle_type), offsetof(OpenTTDVehicleServices, write_x), offsetof(OpenTTDVehicleServices, write_y), offsetof(OpenTTDVehicleServices, write_z), offsetof(OpenTTDVehicleServices, write_tile), offsetof(OpenTTDVehicleServices, write_status), offsetof(OpenTTDVehicleServices, write_subspeed), offsetof(OpenTTDVehicleServices, write_breakdown_counter), offsetof(OpenTTDVehicleServices, write_running), offsetof(OpenTTDVehicleServices, write_order_time), offsetof(OpenTTDVehicleServices, write_profit), offsetof(OpenTTDVehicleServices, write_economy_service), offsetof(OpenTTDVehicleServices, write_calendar_service), offsetof(OpenTTDVehicleServices, write_breakdowns), offsetof(OpenTTDVehicleServices, write_reliability), offsetof(OpenTTDVehicleServices, write_cargo_age), offsetof(OpenTTDVehicleServices, coordinates), offsetof(OpenTTDVehicleServices, xy), offsetof(OpenTTDVehicleServices, speed_inputs), offsetof(OpenTTDVehicleServices, direction_progress), offsetof(OpenTTDVehicleServices, tick_status), offsetof(OpenTTDVehicleServices, day_cost), offsetof(OpenTTDVehicleServices, make_dummy), offsetof(OpenTTDVehicleServices, loading), offsetof(OpenTTDVehicleServices, breakdown), offsetof(OpenTTDVehicleServices, check_orders_only), offsetof(OpenTTDVehicleServices, write_coordinates), offsetof(OpenTTDVehicleServices, altitude_inputs), offsetof(OpenTTDVehicleServices, maintenance)});
	Layout(464, "OpenTTDGroundMoved", {sizeof(OpenTTDGroundMoved), alignof(OpenTTDGroundMoved), offsetof(OpenTTDGroundMoved, old_z), offsetof(OpenTTDGroundMoved, z), offsetof(OpenTTDGroundMoved, cur_speed), offsetof(OpenTTDGroundMoved, max_track_speed)});
	Layout(465, "OpenTTDGroundSpeed", {sizeof(OpenTTDGroundSpeed), alignof(OpenTTDGroundSpeed), offsetof(OpenTTDGroundSpeed, distance), offsetof(OpenTTDGroundSpeed, cur_speed)});
	Layout(466, "OpenTTDGroundLastSpeed", {sizeof(OpenTTDGroundLastSpeed), alignof(OpenTTDGroundLastSpeed), offsetof(OpenTTDGroundLastSpeed, status)});
	Layout(467, "OpenTTDGroundCrash", {sizeof(OpenTTDGroundCrash), alignof(OpenTTDGroundCrash), offsetof(OpenTTDGroundCrash, victims), offsetof(OpenTTDGroundCrash, front)});
	Layout(463, "OpenTTDGroundServices", {sizeof(OpenTTDGroundServices), alignof(OpenTTDGroundServices), offsetof(OpenTTDGroundServices, cache_begin), offsetof(OpenTTDGroundServices, move_incline), offsetof(OpenTTDGroundServices, update_viewport_then_next), offsetof(OpenTTDGroundServices, turn), offsetof(OpenTTDGroundServices, do_update_speed), offsetof(OpenTTDGroundServices, set_last_speed), offsetof(OpenTTDGroundServices, acceleration), offsetof(OpenTTDGroundServices, crash), offsetof(OpenTTDGroundServices, first_cargo_changed), offsetof(OpenTTDGroundServices, go_to_depot_service), offsetof(OpenTTDGroundServices, write_first_engine), offsetof(OpenTTDGroundServices, write_length), offsetof(OpenTTDGroundServices, cached_length), offsetof(OpenTTDGroundServices, has_articulated)});
	Layout(80, "OpenTTDRoadPathElement", {sizeof(OpenTTDRoadPathElement), alignof(OpenTTDRoadPathElement), offsetof(OpenTTDRoadPathElement, trackdir), offsetof(OpenTTDRoadPathElement, tile)});
	Layout(81, "OpenTTDRoadEntry", {sizeof(OpenTTDRoadEntry), alignof(OpenTTDRoadEntry), offsetof(OpenTTDRoadEntry, acceleration_model), offsetof(OpenTTDRoadEntry, road_side), offsetof(OpenTTDRoadEntry, queue), offsetof(OpenTTDRoadEntry, map_log_x)});
	Layout(215, "OpenTTDRoadTickRead", {sizeof(OpenTTDRoadTickRead), alignof(OpenTTDRoadTickRead), offsetof(OpenTTDRoadTickRead, tick), offsetof(OpenTTDRoadTickRead, front)});
	Layout(473, "OpenTTDGroundViewportNext", {sizeof(OpenTTDGroundViewportNext), alignof(OpenTTDGroundViewportNext), offsetof(OpenTTDGroundViewportNext, next), offsetof(OpenTTDGroundViewportNext, final_progress)});
	Layout(471, "OpenTTDRoadLink", {sizeof(OpenTTDRoadLink), alignof(OpenTTDRoadLink), offsetof(OpenTTDRoadLink, next), offsetof(OpenTTDRoadLink, next_state)});
	Layout(472, "OpenTTDRoadLinkTile", {sizeof(OpenTTDRoadLinkTile), alignof(OpenTTDRoadLinkTile), offsetof(OpenTTDRoadLinkTile, link), offsetof(OpenTTDRoadLinkTile, tile)});
	Layout(470, "OpenTTDRoadFrontTickRead", {sizeof(OpenTTDRoadFrontTickRead), alignof(OpenTTDRoadFrontTickRead), offsetof(OpenTTDRoadFrontTickRead, movement), offsetof(OpenTTDRoadFrontTickRead, order_time), offsetof(OpenTTDRoadFrontTickRead, running), offsetof(OpenTTDRoadFrontTickRead, status)});
	Layout(216, "OpenTTDRoadMaxSpeedRead", {sizeof(OpenTTDRoadMaxSpeedRead), alignof(OpenTTDRoadMaxSpeedRead), offsetof(OpenTTDRoadMaxSpeedRead, next), offsetof(OpenTTDRoadMaxSpeedRead, next_state), offsetof(OpenTTDRoadMaxSpeedRead, tile), offsetof(OpenTTDRoadMaxSpeedRead, max_track_speed), offsetof(OpenTTDRoadMaxSpeedRead, order_max_speed), offsetof(OpenTTDRoadMaxSpeedRead, direction), offsetof(OpenTTDRoadMaxSpeedRead, status)});
	Layout(217, "OpenTTDRoadSpeedRead", {sizeof(OpenTTDRoadSpeedRead), alignof(OpenTTDRoadSpeedRead), offsetof(OpenTTDRoadSpeedRead, max), offsetof(OpenTTDRoadSpeedRead, x), offsetof(OpenTTDRoadSpeedRead, y), offsetof(OpenTTDRoadSpeedRead, z), offsetof(OpenTTDRoadSpeedRead, acceleration), offsetof(OpenTTDRoadSpeedRead, length), offsetof(OpenTTDRoadSpeedRead, tram)});
	Layout(218, "OpenTTDRoadSpeedPart", {sizeof(OpenTTDRoadSpeedPart), alignof(OpenTTDRoadSpeedPart), offsetof(OpenTTDRoadSpeedPart, next), offsetof(OpenTTDRoadSpeedPart, next_state), offsetof(OpenTTDRoadSpeedPart, tile), offsetof(OpenTTDRoadSpeedPart, direction), offsetof(OpenTTDRoadSpeedPart, status)});
	Layout(219, "OpenTTDRoadStep", {sizeof(OpenTTDRoadStep), alignof(OpenTTDRoadStep), offsetof(OpenTTDRoadStep, tile), offsetof(OpenTTDRoadStep, x), offsetof(OpenTTDRoadStep, y), offsetof(OpenTTDRoadStep, z), offsetof(OpenTTDRoadStep, cur_speed), offsetof(OpenTTDRoadStep, direction), offsetof(OpenTTDRoadStep, length), offsetof(OpenTTDRoadStep, front), offsetof(OpenTTDRoadStep, tram)});
	Layout(220, "OpenTTDRoadCrashPart", {sizeof(OpenTTDRoadCrashPart), alignof(OpenTTDRoadCrashPart), offsetof(OpenTTDRoadCrashPart, next), offsetof(OpenTTDRoadCrashPart, next_state), offsetof(OpenTTDRoadCrashPart, tile), offsetof(OpenTTDRoadCrashPart, z), offsetof(OpenTTDRoadCrashPart, crossing)});
	Layout(221, "OpenTTDRoadEntered", {sizeof(OpenTTDRoadEntered), alignof(OpenTTDRoadEntered), offsetof(OpenTTDRoadEntered, tile), offsetof(OpenTTDRoadEntered, cur_speed), offsetof(OpenTTDRoadEntered, flags), offsetof(OpenTTDRoadEntered, direction), offsetof(OpenTTDRoadEntered, order_type)});
	Layout(222, "OpenTTDRoadCandidate", {sizeof(OpenTTDRoadCandidate), alignof(OpenTTDRoadCandidate), offsetof(OpenTTDRoadCandidate, vehicle), offsetof(OpenTTDRoadCandidate, state), offsetof(OpenTTDRoadCandidate, first), offsetof(OpenTTDRoadCandidate, first_state), offsetof(OpenTTDRoadCandidate, index), offsetof(OpenTTDRoadCandidate, x), offsetof(OpenTTDRoadCandidate, y), offsetof(OpenTTDRoadCandidate, z), offsetof(OpenTTDRoadCandidate, first_speed), offsetof(OpenTTDRoadCandidate, direction)});
	Layout(223, "OpenTTDRoadTileVehicle", {sizeof(OpenTTDRoadTileVehicle), alignof(OpenTTDRoadTileVehicle), offsetof(OpenTTDRoadTileVehicle, vehicle), offsetof(OpenTTDRoadTileVehicle, first)});
	Layout(224, "OpenTTDRoadPartRead", {sizeof(OpenTTDRoadPartRead), alignof(OpenTTDRoadPartRead), offsetof(OpenTTDRoadPartRead, next), offsetof(OpenTTDRoadPartRead, next_state), offsetof(OpenTTDRoadPartRead, tile), offsetof(OpenTTDRoadPartRead, z), offsetof(OpenTTDRoadPartRead, tram)});
	Layout(225, "OpenTTDRoadPathRead", {sizeof(OpenTTDRoadPathRead), alignof(OpenTTDRoadPathRead), offsetof(OpenTTDRoadPathRead, dest), offsetof(OpenTTDRoadPathRead, owner), offsetof(OpenTTDRoadPathRead, articulated), offsetof(OpenTTDRoadPathRead, bus)});
	Layout(226, "OpenTTDRoadStopRead", {sizeof(OpenTTDRoadStopRead), alignof(OpenTTDRoadStopRead), offsetof(OpenTTDRoadStopRead, owner), offsetof(OpenTTDRoadStopRead, bus)});
	Layout(227, "OpenTTDRoadOvertakeRead", {sizeof(OpenTTDRoadOvertakeRead), alignof(OpenTTDRoadOvertakeRead), offsetof(OpenTTDRoadOvertakeRead, tile), offsetof(OpenTTDRoadOvertakeRead, cur_speed), offsetof(OpenTTDRoadOvertakeRead, direction), offsetof(OpenTTDRoadOvertakeRead, status)});
	Layout(228, "OpenTTDRoadRoadTypes", {sizeof(OpenTTDRoadRoadTypes), alignof(OpenTTDRoadRoadTypes), offsetof(OpenTTDRoadRoadTypes, before), offsetof(OpenTTDRoadRoadTypes, after)});
	Layout(468, "OpenTTDVehiclePreviousTile", {sizeof(OpenTTDVehiclePreviousTile), alignof(OpenTTDVehiclePreviousTile), offsetof(OpenTTDVehiclePreviousTile, tile), offsetof(OpenTTDVehiclePreviousTile, exists)});
	Layout(230, "OpenTTDRoadTrackChoice", {sizeof(OpenTTDRoadTrackChoice), alignof(OpenTTDRoadTrackChoice), offsetof(OpenTTDRoadTrackChoice, trackdir), offsetof(OpenTTDRoadTrackChoice, found)});
	Layout(231, "OpenTTDRoadDepotResult", {sizeof(OpenTTDRoadDepotResult), alignof(OpenTTDRoadDepotResult), offsetof(OpenTTDRoadDepotResult, tile), offsetof(OpenTTDRoadDepotResult, length)});
	Layout(233, "OpenTTDRoadArrivalRead", {sizeof(OpenTTDRoadArrivalRead), alignof(OpenTTDRoadArrivalRead), offsetof(OpenTTDRoadArrivalRead, had_vehicle_of_type), offsetof(OpenTTDRoadArrivalRead, bus)});
	Layout(234, "OpenTTDRoadSoundRead", {sizeof(OpenTTDRoadSoundRead), alignof(OpenTTDRoadSoundRead), offsetof(OpenTTDRoadSoundRead, sound), offsetof(OpenTTDRoadSoundRead, tick)});
	Layout(469, "OpenTTDEngineForLength", {sizeof(OpenTTDEngineForLength), alignof(OpenTTDEngineForLength), offsetof(OpenTTDEngineForLength, engine), offsetof(OpenTTDEngineForLength, grf_version)});
	Layout(236, "OpenTTDRoadCostRead", {sizeof(OpenTTDRoadCostRead), alignof(OpenTTDRoadCostRead), offsetof(OpenTTDRoadCostRead, engine), offsetof(OpenTTDRoadCostRead, cost_class), offsetof(OpenTTDRoadCostRead, cost_factor)});
	Layout(237, "OpenTTDRoadServiceRead", {sizeof(OpenTTDRoadServiceRead), alignof(OpenTTDRoadServiceRead), offsetof(OpenTTDRoadServiceRead, tile), offsetof(OpenTTDRoadServiceRead, servint), offsetof(OpenTTDRoadServiceRead, cur_speed)});
	Layout(238, "OpenTTDRoadTurnRead", {sizeof(OpenTTDRoadTurnRead), alignof(OpenTTDRoadTurnRead), offsetof(OpenTTDRoadTurnRead, tile), offsetof(OpenTTDRoadTurnRead, breakdown_ctr), offsetof(OpenTTDRoadTurnRead, direction), offsetof(OpenTTDRoadTurnRead, order_type), offsetof(OpenTTDRoadTurnRead, status)});
	Layout(239, "OpenTTDRoadTrackdirRead", {sizeof(OpenTTDRoadTrackdirRead), alignof(OpenTTDRoadTrackdirRead), offsetof(OpenTTDRoadTrackdirRead, tile), offsetof(OpenTTDRoadTrackdirRead, direction), offsetof(OpenTTDRoadTrackdirRead, status)});
	Layout(82, "OpenTTDRoadLeaves", {sizeof(OpenTTDRoadLeaves), alignof(OpenTTDRoadLeaves), offsetof(OpenTTDRoadLeaves, vehicle), offsetof(OpenTTDRoadLeaves, ground), offsetof(OpenTTDRoadLeaves, shared), offsetof(OpenTTDRoadLeaves, tick_front), offsetof(OpenTTDRoadLeaves, max_depot_penalty), offsetof(OpenTTDRoadLeaves, process_orders_then_loading), offsetof(OpenTTDRoadLeaves, visual_then_speed), offsetof(OpenTTDRoadLeaves, max_speed_read), offsetof(OpenTTDRoadLeaves, speed_part), offsetof(OpenTTDRoadLeaves, step), offsetof(OpenTTDRoadLeaves, crash_part), offsetof(OpenTTDRoadLeaves, part), offsetof(OpenTTDRoadLeaves, link), offsetof(OpenTTDRoadLeaves, link_tile), offsetof(OpenTTDRoadLeaves, enter_tile), offsetof(OpenTTDRoadLeaves, visit_close), offsetof(OpenTTDRoadLeaves, visit_trains), offsetof(OpenTTDRoadLeaves, visit_tile), offsetof(OpenTTDRoadLeaves, has_road), offsetof(OpenTTDRoadLeaves, track_status), offsetof(OpenTTDRoadLeaves, any_road_bits), offsetof(OpenTTDRoadLeaves, tram_bits), offsetof(OpenTTDRoadLeaves, is_road_depot_tile), offsetof(OpenTTDRoadLeaves, depot_direction), offsetof(OpenTTDRoadLeaves, is_normal_road), offsetof(OpenTTDRoadLeaves, has_road_works), offsetof(OpenTTDRoadLeaves, disallowed_directions), offsetof(OpenTTDRoadLeaves, is_bay_stop), offsetof(OpenTTDRoadLeaves, bay_direction), offsetof(OpenTTDRoadLeaves, is_drive_through), offsetof(OpenTTDRoadLeaves, is_station_road_stop), offsetof(OpenTTDRoadLeaves, stop_type), offsetof(OpenTTDRoadLeaves, has_free_bay), offsetof(OpenTTDRoadLeaves, continuation), offsetof(OpenTTDRoadLeaves, move_tile), offsetof(OpenTTDRoadLeaves, path_read), offsetof(OpenTTDRoadLeaves, choose_track), offsetof(OpenTTDRoadLeaves, find_depot), offsetof(OpenTTDRoadLeaves, can_build_tram), offsetof(OpenTTDRoadLeaves, disconnect), offsetof(OpenTTDRoadLeaves, stop_read), offsetof(OpenTTDRoadLeaves, should_stop), offsetof(OpenTTDRoadLeaves, stop_leave), offsetof(OpenTTDRoadLeaves, stop_entrance), offsetof(OpenTTDRoadLeaves, stop_entrance_busy), offsetof(OpenTTDRoadLeaves, arrival_read), offsetof(OpenTTDRoadLeaves, arrival_news), offsetof(OpenTTDRoadLeaves, begin_loading_at), offsetof(OpenTTDRoadLeaves, overtake_read), offsetof(OpenTTDRoadLeaves, sound_read), offsetof(OpenTTDRoadLeaves, depot_exit), offsetof(OpenTTDRoadLeaves, crash_news), offsetof(OpenTTDRoadLeaves, shorten_factor), offsetof(OpenTTDRoadLeaves, default_max_speed), offsetof(OpenTTDRoadLeaves, update_cargo_age), offsetof(OpenTTDRoadLeaves, cost_read), offsetof(OpenTTDRoadLeaves, price), offsetof(OpenTTDRoadLeaves, pay_running), offsetof(OpenTTDRoadLeaves, service_read), offsetof(OpenTTDRoadLeaves, turn_read), offsetof(OpenTTDRoadLeaves, trackdir_read)});
	Layout(44, "OpenTTDDisasterState", {sizeof(OpenTTDDisasterState), alignof(OpenTTDDisasterState), offsetof(OpenTTDDisasterState, image_override), offsetof(OpenTTDDisasterState, target), offsetof(OpenTTDDisasterState, state), offsetof(OpenTTDDisasterState, flags)});
	Layout(45, "OpenTTDDisasterAction", {sizeof(OpenTTDDisasterAction), alignof(OpenTTDDisasterAction), offsetof(OpenTTDDisasterAction, kind), offsetof(OpenTTDDisasterAction, id), offsetof(OpenTTDDisasterAction, other), offsetof(OpenTTDDisasterAction, a), offsetof(OpenTTDDisasterAction, b), offsetof(OpenTTDDisasterAction, c), offsetof(OpenTTDDisasterAction, d)});
	Layout(46, "OpenTTDWaterPatch", {sizeof(OpenTTDWaterPatch), alignof(OpenTTDWaterPatch), offsetof(OpenTTDWaterPatch, x), offsetof(OpenTTDWaterPatch, y), offsetof(OpenTTDWaterPatch, label)});
	Layout(47, "OpenTTDWaterSnapshot", {sizeof(OpenTTDWaterSnapshot), alignof(OpenTTDWaterSnapshot), offsetof(OpenTTDWaterSnapshot, edges), offsetof(OpenTTDWaterSnapshot, labels), offsetof(OpenTTDWaterSnapshot, patches), offsetof(OpenTTDWaterSnapshot, aqueducts)});
	Layout(48, "OpenTTDWaterLeaves", {sizeof(OpenTTDWaterLeaves), alignof(OpenTTDWaterLeaves), offsetof(OpenTTDWaterLeaves, tracks), offsetof(OpenTTDWaterLeaves, follow), offsetof(OpenTTDWaterLeaves, aqueduct), offsetof(OpenTTDWaterLeaves, debug)});
	Layout(180, "OpenTTDAircraftState", {sizeof(OpenTTDAircraftState), alignof(OpenTTDAircraftState), offsetof(OpenTTDAircraftState, cached_max_range_sqr), offsetof(OpenTTDAircraftState, cached_max_range), offsetof(OpenTTDAircraftState, cache_padding), offsetof(OpenTTDAircraftState, crashed_counter), offsetof(OpenTTDAircraftState, targetairport), offsetof(OpenTTDAircraftState, pos), offsetof(OpenTTDAircraftState, previous_pos), offsetof(OpenTTDAircraftState, state), offsetof(OpenTTDAircraftState, last_direction), offsetof(OpenTTDAircraftState, number_consecutive_turns), offsetof(OpenTTDAircraftState, turn_counter), offsetof(OpenTTDAircraftState, flags)});
	Layout(181, "OpenTTDAircraftVehicle", {sizeof(OpenTTDAircraftVehicle), alignof(OpenTTDAircraftVehicle), offsetof(OpenTTDAircraftVehicle, handle), offsetof(OpenTTDAircraftVehicle, state), offsetof(OpenTTDAircraftVehicle, id)});
	Layout(182, "OpenTTDAircraftNode", {sizeof(OpenTTDAircraftNode), alignof(OpenTTDAircraftNode), offsetof(OpenTTDAircraftNode, next), offsetof(OpenTTDAircraftNode, blocks), offsetof(OpenTTDAircraftNode, position), offsetof(OpenTTDAircraftNode, next_position), offsetof(OpenTTDAircraftNode, heading)});
	Layout(183, "OpenTTDAircraftMoving", {sizeof(OpenTTDAircraftMoving), alignof(OpenTTDAircraftMoving), offsetof(OpenTTDAircraftMoving, x), offsetof(OpenTTDAircraftMoving, y), offsetof(OpenTTDAircraftMoving, flags), offsetof(OpenTTDAircraftMoving, direction)});
	Layout(185, "OpenTTDAircraftLeaves", {sizeof(OpenTTDAircraftLeaves), alignof(OpenTTDAircraftLeaves), offsetof(OpenTTDAircraftLeaves, hangar_candidate), offsetof(OpenTTDAircraftLeaves, geometry), offsetof(OpenTTDAircraftLeaves, layout_blocks), offsetof(OpenTTDAircraftLeaves, layout_heading), offsetof(OpenTTDAircraftLeaves, layout_next_position), offsetof(OpenTTDAircraftLeaves, layout_next), offsetof(OpenTTDAircraftLeaves, layout_route), offsetof(OpenTTDAircraftLeaves, vehicle), offsetof(OpenTTDAircraftLeaves, shared), offsetof(OpenTTDAircraftLeaves, next), offsetof(OpenTTDAircraftLeaves, station), offsetof(OpenTTDAircraftLeaves, airport_tile), offsetof(OpenTTDAircraftLeaves, station_tile), offsetof(OpenTTDAircraftLeaves, rotation), offsetof(OpenTTDAircraftLeaves, airport_type), offsetof(OpenTTDAircraftLeaves, station_owner), offsetof(OpenTTDAircraftLeaves, has_hangar), offsetof(OpenTTDAircraftLeaves, airport_fta), offsetof(OpenTTDAircraftLeaves, airport_blocks), offsetof(OpenTTDAircraftLeaves, had_vehicle), offsetof(OpenTTDAircraftLeaves, dummy_airport), offsetof(OpenTTDAircraftLeaves, airport_elements), offsetof(OpenTTDAircraftLeaves, helipads), offsetof(OpenTTDAircraftLeaves, airport_flags), offsetof(OpenTTDAircraftLeaves, airport_delta_z), offsetof(OpenTTDAircraftLeaves, node), offsetof(OpenTTDAircraftLeaves, fta), offsetof(OpenTTDAircraftLeaves, moving), offsetof(OpenTTDAircraftLeaves, engine_speed), offsetof(OpenTTDAircraftLeaves, engine_subtype), offsetof(OpenTTDAircraftLeaves, engine_sound), offsetof(OpenTTDAircraftLeaves, engine_reliability), offsetof(OpenTTDAircraftLeaves, airport_entry), offsetof(OpenTTDAircraftLeaves, direction_towards), offsetof(OpenTTDAircraftLeaves, hangar_height), offsetof(OpenTTDAircraftLeaves, terminal_count), offsetof(OpenTTDAircraftLeaves, hangar_exit), offsetof(OpenTTDAircraftLeaves, can_use_station), offsetof(OpenTTDAircraftLeaves, service_interval), offsetof(OpenTTDAircraftLeaves, chain_in_depot), offsetof(OpenTTDAircraftLeaves, nearest_depot_order), offsetof(OpenTTDAircraftLeaves, part_of_orders), offsetof(OpenTTDAircraftLeaves, next_station), offsetof(OpenTTDAircraftLeaves, next_aircraft), offsetof(OpenTTDAircraftLeaves, update_position), offsetof(OpenTTDAircraftLeaves, rotor_image), offsetof(OpenTTDAircraftLeaves, copy_sprite), offsetof(OpenTTDAircraftLeaves, position_viewport), offsetof(OpenTTDAircraftLeaves, create_effect), offsetof(OpenTTDAircraftLeaves, truncate_cargo), offsetof(OpenTTDAircraftLeaves, crash_news), offsetof(OpenTTDAircraftLeaves, station_rating), offsetof(OpenTTDAircraftLeaves, landing_rating), offsetof(OpenTTDAircraftLeaves, dirty_depot), offsetof(OpenTTDAircraftLeaves, first_arrival), offsetof(OpenTTDAircraftLeaves, begin_loading), offsetof(OpenTTDAircraftLeaves, dirty_details), offsetof(OpenTTDAircraftLeaves, update_delta), offsetof(OpenTTDAircraftLeaves, touchdown_animation), offsetof(OpenTTDAircraftLeaves, destination_too_far), offsetof(OpenTTDAircraftLeaves, delete_range_news), offsetof(OpenTTDAircraftLeaves, service_order), offsetof(OpenTTDAircraftLeaves, running_cost), offsetof(OpenTTDAircraftLeaves, subtract_cost), offsetof(OpenTTDAircraftLeaves, dirty_lists), offsetof(OpenTTDAircraftLeaves, next_stopping_station), offsetof(OpenTTDAircraftLeaves, remove_depot_orders), offsetof(OpenTTDAircraftLeaves, assert_flying), offsetof(OpenTTDAircraftLeaves, invalid_movement), offsetof(OpenTTDAircraftLeaves, invalid_position), offsetof(OpenTTDAircraftLeaves, invalid_scheme), offsetof(OpenTTDAircraftLeaves, unreachable), offsetof(OpenTTDAircraftLeaves, cargo_age_property), offsetof(OpenTTDAircraftLeaves, range_property), offsetof(OpenTTDAircraftLeaves, touchdown_sound), offsetof(OpenTTDAircraftLeaves, rotor_image_if_changed), offsetof(OpenTTDAircraftLeaves, update_rotor_image), offsetof(OpenTTDAircraftLeaves, process_orders), offsetof(OpenTTDAircraftLeaves, vehicle_crash), offsetof(OpenTTDAircraftLeaves, send_to_depot), offsetof(OpenTTDAircraftLeaves, fta_next_position), offsetof(OpenTTDAircraftLeaves, block_node), offsetof(OpenTTDAircraftLeaves, route_node), offsetof(OpenTTDAircraftLeaves, block_choice)});
	Layout(670, "OpenTTDVehicleCoordinates", {sizeof(OpenTTDVehicleCoordinates), alignof(OpenTTDVehicleCoordinates), offsetof(OpenTTDVehicleCoordinates, x), offsetof(OpenTTDVehicleCoordinates, y), offsetof(OpenTTDVehicleCoordinates, z)});
	Layout(671, "OpenTTDVehicleXY", {sizeof(OpenTTDVehicleXY), alignof(OpenTTDVehicleXY), offsetof(OpenTTDVehicleXY, x), offsetof(OpenTTDVehicleXY, y)});
	Layout(672, "OpenTTDVehicleSpeedInputs", {sizeof(OpenTTDVehicleSpeedInputs), alignof(OpenTTDVehicleSpeedInputs), offsetof(OpenTTDVehicleSpeedInputs, speed), offsetof(OpenTTDVehicleSpeedInputs, max_speed), offsetof(OpenTTDVehicleSpeedInputs, acceleration), offsetof(OpenTTDVehicleSpeedInputs, status), offsetof(OpenTTDVehicleSpeedInputs, subspeed)});
	Layout(673, "OpenTTDVehicleDirectionProgress", {sizeof(OpenTTDVehicleDirectionProgress), alignof(OpenTTDVehicleDirectionProgress), offsetof(OpenTTDVehicleDirectionProgress, direction), offsetof(OpenTTDVehicleDirectionProgress, progress)});
	Layout(674, "OpenTTDVehicleTickStatus", {sizeof(OpenTTDVehicleTickStatus), alignof(OpenTTDVehicleTickStatus), offsetof(OpenTTDVehicleTickStatus, tick), offsetof(OpenTTDVehicleTickStatus, status)});
	Layout(675, "OpenTTDVehicleDayCost", {sizeof(OpenTTDVehicleDayCost), alignof(OpenTTDVehicleDayCost), offsetof(OpenTTDVehicleDayCost, profit), offsetof(OpenTTDVehicleDayCost, running)});
	Layout(676, "OpenTTDAircraftGeometry", {sizeof(OpenTTDAircraftGeometry), alignof(OpenTTDAircraftGeometry), offsetof(OpenTTDAircraftGeometry, tile), offsetof(OpenTTDAircraftGeometry, rotation), offsetof(OpenTTDAircraftGeometry, width), offsetof(OpenTTDAircraftGeometry, height), offsetof(OpenTTDAircraftGeometry, airport)});
	Layout(677, "OpenTTDAircraftLayoutRoute", {sizeof(OpenTTDAircraftLayoutRoute), alignof(OpenTTDAircraftLayoutRoute), offsetof(OpenTTDAircraftLayoutRoute, node), offsetof(OpenTTDAircraftLayoutRoute, next), offsetof(OpenTTDAircraftLayoutRoute, next_position), offsetof(OpenTTDAircraftLayoutRoute, heading)});
	Layout(678, "OpenTTDVehicleAltitudeInputs", {sizeof(OpenTTDVehicleAltitudeInputs), alignof(OpenTTDVehicleAltitudeInputs), offsetof(OpenTTDVehicleAltitudeInputs, max_speed), offsetof(OpenTTDVehicleAltitudeInputs, direction)});
	Layout(679, "OpenTTDAircraftHangarCandidate", {sizeof(OpenTTDAircraftHangarCandidate), alignof(OpenTTDAircraftHangarCandidate), offsetof(OpenTTDAircraftHangarCandidate, owner), offsetof(OpenTTDAircraftHangarCandidate, airport), offsetof(OpenTTDAircraftHangarCandidate, hangar)});
	Layout(186, "OpenTTDAircraftBlockNode", {sizeof(OpenTTDAircraftBlockNode), alignof(OpenTTDAircraftBlockNode), offsetof(OpenTTDAircraftBlockNode, blocks), offsetof(OpenTTDAircraftBlockNode, position), offsetof(OpenTTDAircraftBlockNode, next_position)});
	Layout(187, "OpenTTDAircraftRouteNode", {sizeof(OpenTTDAircraftRouteNode), alignof(OpenTTDAircraftRouteNode), offsetof(OpenTTDAircraftRouteNode, next), offsetof(OpenTTDAircraftRouteNode, next_position), offsetof(OpenTTDAircraftRouteNode, heading)});
	Layout(188, "OpenTTDAircraftBlockChoice", {sizeof(OpenTTDAircraftBlockChoice), alignof(OpenTTDAircraftBlockChoice), offsetof(OpenTTDAircraftBlockChoice, next), offsetof(OpenTTDAircraftBlockChoice, blocks), offsetof(OpenTTDAircraftBlockChoice, heading)});

	Layout(340, "OpenTTDTrainHandle", {sizeof(OpenTTDTrainHandle), alignof(OpenTTDTrainHandle), offsetof(OpenTTDTrainHandle, shell), offsetof(OpenTTDTrainHandle, owner)});
	Layout(341, "OpenTTDTrainConsistChangedRead", {sizeof(OpenTTDTrainConsistChangedRead), alignof(OpenTTDTrainConsistChangedRead), offsetof(OpenTTDTrainConsistChangedRead, engine), offsetof(OpenTTDTrainConsistChangedRead, front)});
	Layout(342, "OpenTTDTrainConsistChanged1Read", {sizeof(OpenTTDTrainConsistChanged1Read), alignof(OpenTTDTrainConsistChanged1Read), offsetof(OpenTTDTrainConsistChanged1Read, engine), offsetof(OpenTTDTrainConsistChanged1Read, engine_part)});
	Layout(343, "OpenTTDTrainConsistChanged2Read", {sizeof(OpenTTDTrainConsistChanged2Read), alignof(OpenTTDTrainConsistChanged2Read), offsetof(OpenTTDTrainConsistChanged2Read, cargo_cap)});
	Layout(344, "OpenTTDTrainCurveLimitRead", {sizeof(OpenTTDTrainCurveLimitRead), alignof(OpenTTDTrainCurveLimitRead), offsetof(OpenTTDTrainCurveLimitRead, next), offsetof(OpenTTDTrainCurveLimitRead, direction)});
	Layout(345, "OpenTTDTrainStopLocationRead", {sizeof(OpenTTDTrainStopLocationRead), alignof(OpenTTDTrainStopLocationRead), offsetof(OpenTTDTrainStopLocationRead, length), offsetof(OpenTTDTrainStopLocationRead, total_length), offsetof(OpenTTDTrainStopLocationRead, order_destination), offsetof(OpenTTDTrainStopLocationRead, order)});
	Layout(346, "OpenTTDTrainCurrentMaxSpeedRead", {sizeof(OpenTTDTrainCurrentMaxSpeedRead), alignof(OpenTTDTrainCurrentMaxSpeedRead), offsetof(OpenTTDTrainCurrentMaxSpeedRead, tile), offsetof(OpenTTDTrainCurrentMaxSpeedRead, max_track_speed), offsetof(OpenTTDTrainCurrentMaxSpeedRead, speed)});
	Layout(347, "OpenTTDTrainCurrentMaxSpeed6Read", {sizeof(OpenTTDTrainCurrentMaxSpeed6Read), alignof(OpenTTDTrainCurrentMaxSpeed6Read), offsetof(OpenTTDTrainCurrentMaxSpeed6Read, next), offsetof(OpenTTDTrainCurrentMaxSpeed6Read, tile), offsetof(OpenTTDTrainCurrentMaxSpeed6Read, status)});
	Layout(348, "OpenTTDTrainUpdateAccelerationRead", {sizeof(OpenTTDTrainUpdateAccelerationRead), alignof(OpenTTDTrainUpdateAccelerationRead), offsetof(OpenTTDTrainUpdateAccelerationRead, power), offsetof(OpenTTDTrainUpdateAccelerationRead, weight)});
	Layout(349, "OpenTTDTrainUpdateSpeedRead", {sizeof(OpenTTDTrainUpdateSpeedRead), alignof(OpenTTDTrainUpdateSpeedRead), offsetof(OpenTTDTrainUpdateSpeedRead, status), offsetof(OpenTTDTrainUpdateSpeedRead, acceleration)});
	Layout(350, "OpenTTDTrainTrackdirRead", {sizeof(OpenTTDTrainTrackdirRead), alignof(OpenTTDTrainTrackdirRead), offsetof(OpenTTDTrainTrackdirRead, tile), offsetof(OpenTTDTrainTrackdirRead, direction), offsetof(OpenTTDTrainTrackdirRead, status)});
	Layout(351, "OpenTTDTrainCanLeaveRead", {sizeof(OpenTTDTrainCanLeaveRead), alignof(OpenTTDTrainCanLeaveRead), offsetof(OpenTTDTrainCanLeaveRead, tile), offsetof(OpenTTDTrainCanLeaveRead, direction)});
	Layout(353, "OpenTTDTrainCrossingApproachRead", {sizeof(OpenTTDTrainCrossingApproachRead), alignof(OpenTTDTrainCrossingApproachRead), offsetof(OpenTTDTrainCrossingApproachRead, status), offsetof(OpenTTDTrainCrossingApproachRead, front)});
	Layout(354, "OpenTTDTrainNextOffsetRead", {sizeof(OpenTTDTrainNextOffsetRead), alignof(OpenTTDTrainNextOffsetRead), offsetof(OpenTTDTrainNextOffsetRead, next), offsetof(OpenTTDTrainNextOffsetRead, length)});
	Layout(355, "OpenTTDTrainAfterSwapRead", {sizeof(OpenTTDTrainAfterSwapRead), alignof(OpenTTDTrainAfterSwapRead), offsetof(OpenTTDTrainAfterSwapRead, tile), offsetof(OpenTTDTrainAfterSwapRead, x), offsetof(OpenTTDTrainAfterSwapRead, y)});
	Layout(356, "OpenTTDTrainReverseSwapRead", {sizeof(OpenTTDTrainReverseSwapRead), alignof(OpenTTDTrainReverseSwapRead), offsetof(OpenTTDTrainReverseSwapRead, tile), offsetof(OpenTTDTrainReverseSwapRead, x), offsetof(OpenTTDTrainReverseSwapRead, y), offsetof(OpenTTDTrainReverseSwapRead, z), offsetof(OpenTTDTrainReverseSwapRead, direction), offsetof(OpenTTDTrainReverseSwapRead, status)});
	Layout(357, "OpenTTDTrainApproachingEndRead", {sizeof(OpenTTDTrainApproachingEndRead), alignof(OpenTTDTrainApproachingEndRead), offsetof(OpenTTDTrainApproachingEndRead, x), offsetof(OpenTTDTrainApproachingEndRead, y), offsetof(OpenTTDTrainApproachingEndRead, length), offsetof(OpenTTDTrainApproachingEndRead, speed), offsetof(OpenTTDTrainApproachingEndRead, direction)});
	Layout(358, "OpenTTDTrainLineEndsRead", {sizeof(OpenTTDTrainLineEndsRead), alignof(OpenTTDTrainLineEndsRead), offsetof(OpenTTDTrainLineEndsRead, tile), offsetof(OpenTTDTrainLineEndsRead, speed), offsetof(OpenTTDTrainLineEndsRead, breakdown)});
	Layout(359, "OpenTTDTrainSpeedZRead", {sizeof(OpenTTDTrainSpeedZRead), alignof(OpenTTDTrainSpeedZRead), offsetof(OpenTTDTrainSpeedZRead, z), offsetof(OpenTTDTrainSpeedZRead, max_track_speed), offsetof(OpenTTDTrainSpeedZRead, speed)});
	Layout(360, "OpenTTDTrainMoveVehicleRead", {sizeof(OpenTTDTrainMoveVehicleRead), alignof(OpenTTDTrainMoveVehicleRead), offsetof(OpenTTDTrainMoveVehicleRead, tile), offsetof(OpenTTDTrainMoveVehicleRead, x), offsetof(OpenTTDTrainMoveVehicleRead, y), offsetof(OpenTTDTrainMoveVehicleRead, direction), offsetof(OpenTTDTrainMoveVehicleRead, front), offsetof(OpenTTDTrainMoveVehicleRead, articulated)});
	Layout(361, "OpenTTDTrainMoveVehicle20Read", {sizeof(OpenTTDTrainMoveVehicle20Read), alignof(OpenTTDTrainMoveVehicle20Read), offsetof(OpenTTDTrainMoveVehicle20Read, speed), offsetof(OpenTTDTrainMoveVehicle20Read, status), offsetof(OpenTTDTrainMoveVehicle20Read, front)});
	Layout(362, "OpenTTDTrainCollisionOneRead", {sizeof(OpenTTDTrainCollisionOneRead), alignof(OpenTTDTrainCollisionOneRead), offsetof(OpenTTDTrainCollisionOneRead, first), offsetof(OpenTTDTrainCollisionOneRead, x), offsetof(OpenTTDTrainCollisionOneRead, y), offsetof(OpenTTDTrainCollisionOneRead, z), offsetof(OpenTTDTrainCollisionOneRead, length), offsetof(OpenTTDTrainCollisionOneRead, owner)});
	Layout(363, "OpenTTDTrainCollisionOne22Read", {sizeof(OpenTTDTrainCollisionOne22Read), alignof(OpenTTDTrainCollisionOne22Read), offsetof(OpenTTDTrainCollisionOne22Read, x), offsetof(OpenTTDTrainCollisionOne22Read, y), offsetof(OpenTTDTrainCollisionOne22Read, z), offsetof(OpenTTDTrainCollisionOne22Read, length), offsetof(OpenTTDTrainCollisionOne22Read, owner)});
	Layout(364, "OpenTTDTrainDeleteLastRead", {sizeof(OpenTTDTrainDeleteLastRead), alignof(OpenTTDTrainDeleteLastRead), offsetof(OpenTTDTrainDeleteLastRead, tile), offsetof(OpenTTDTrainDeleteLastRead, owner)});
	Layout(365, "OpenTTDTrainStayDepotRead", {sizeof(OpenTTDTrainStayDepotRead), alignof(OpenTTDTrainStayDepotRead), offsetof(OpenTTDTrainStayDepotRead, tile), offsetof(OpenTTDTrainStayDepotRead, power)});
	Layout(366, "OpenTTDTrainLocoRead", {sizeof(OpenTTDTrainLocoRead), alignof(OpenTTDTrainLocoRead), offsetof(OpenTTDTrainLocoRead, speed), offsetof(OpenTTDTrainLocoRead, status), offsetof(OpenTTDTrainLocoRead, order)});
	Layout(367, "OpenTTDTrainLoco26Read", {sizeof(OpenTTDTrainLoco26Read), alignof(OpenTTDTrainLoco26Read), offsetof(OpenTTDTrainLoco26Read, tile), offsetof(OpenTTDTrainLoco26Read, order_destination), offsetof(OpenTTDTrainLoco26Read, order), offsetof(OpenTTDTrainLoco26Read, nonstop)});
	Layout(368, "OpenTTDTrainTickRead", {sizeof(OpenTTDTrainTickRead), alignof(OpenTTDTrainTickRead), offsetof(OpenTTDTrainTickRead, speed), offsetof(OpenTTDTrainTickRead, status), offsetof(OpenTTDTrainTickRead, running), offsetof(OpenTTDTrainTickRead, front), offsetof(OpenTTDTrainTickRead, free_wagon)});
	Layout(369, "OpenTTDTrainNeedsServiceRead", {sizeof(OpenTTDTrainNeedsServiceRead), alignof(OpenTTDTrainNeedsServiceRead), offsetof(OpenTTDTrainNeedsServiceRead, order_destination), offsetof(OpenTTDTrainNeedsServiceRead, order)});
	Layout(370, "OpenTTDTrainNextForceRead", {sizeof(OpenTTDTrainNextForceRead), alignof(OpenTTDTrainNextForceRead), offsetof(OpenTTDTrainNextForceRead, tile), offsetof(OpenTTDTrainNextForceRead, status)});
	Layout(371, "OpenTTDTrainReverseCommandRead", {sizeof(OpenTTDTrainReverseCommandRead), alignof(OpenTTDTrainReverseCommandRead), offsetof(OpenTTDTrainReverseCommandRead, status), offsetof(OpenTTDTrainReverseCommandRead, breakdown), offsetof(OpenTTDTrainReverseCommandRead, front)});
	Layout(372, "OpenTTDTrainServices", {sizeof(OpenTTDTrainServices), alignof(OpenTTDTrainServices), offsetof(OpenTTDTrainServices, read_first), offsetof(OpenTTDTrainServices, read_next), offsetof(OpenTTDTrainServices, read_previous), offsetof(OpenTTDTrainServices, read_next_unit), offsetof(OpenTTDTrainServices, read_last), offsetof(OpenTTDTrainServices, read_tile), offsetof(OpenTTDTrainServices, read_dest), offsetof(OpenTTDTrainServices, read_order_time), offsetof(OpenTTDTrainServices, read_length), offsetof(OpenTTDTrainServices, read_total_length), offsetof(OpenTTDTrainServices, read_max_track_speed), offsetof(OpenTTDTrainServices, read_speed), offsetof(OpenTTDTrainServices, read_gv_flags), offsetof(OpenTTDTrainServices, read_refit_cap), offsetof(OpenTTDTrainServices, read_last_station), offsetof(OpenTTDTrainServices, read_direction), offsetof(OpenTTDTrainServices, read_status), offsetof(OpenTTDTrainServices, read_tick), offsetof(OpenTTDTrainServices, read_running), offsetof(OpenTTDTrainServices, read_day), offsetof(OpenTTDTrainServices, read_progress), offsetof(OpenTTDTrainServices, read_order), offsetof(OpenTTDTrainServices, read_front), offsetof(OpenTTDTrainServices, read_articulated), offsetof(OpenTTDTrainServices, read_multiheaded), offsetof(OpenTTDTrainServices, read_owner), offsetof(OpenTTDTrainServices, read_vis_effect), offsetof(OpenTTDTrainServices, read_consist_changed), offsetof(OpenTTDTrainServices, read_consist_changed_1), offsetof(OpenTTDTrainServices, read_consist_changed_2), offsetof(OpenTTDTrainServices, read_curve_limit), offsetof(OpenTTDTrainServices, read_stop_location), offsetof(OpenTTDTrainServices, read_current_max_speed), offsetof(OpenTTDTrainServices, read_current_max_speed_6), offsetof(OpenTTDTrainServices, read_update_acceleration), offsetof(OpenTTDTrainServices, read_update_speed), offsetof(OpenTTDTrainServices, read_trackdir), offsetof(OpenTTDTrainServices, read_can_leave), offsetof(OpenTTDTrainServices, read_crossing_approach), offsetof(OpenTTDTrainServices, read_next_offset), offsetof(OpenTTDTrainServices, read_after_swap), offsetof(OpenTTDTrainServices, read_reverse_swap), offsetof(OpenTTDTrainServices, read_approaching_end), offsetof(OpenTTDTrainServices, read_line_ends), offsetof(OpenTTDTrainServices, read_speed_z), offsetof(OpenTTDTrainServices, read_move_vehicle), offsetof(OpenTTDTrainServices, read_move_vehicle_20), offsetof(OpenTTDTrainServices, read_collision_one), offsetof(OpenTTDTrainServices, read_collision_one_22), offsetof(OpenTTDTrainServices, read_delete_last), offsetof(OpenTTDTrainServices, read_stay_depot), offsetof(OpenTTDTrainServices, read_loco), offsetof(OpenTTDTrainServices, read_loco_26), offsetof(OpenTTDTrainServices, read_tick_state), offsetof(OpenTTDTrainServices, read_needs_service), offsetof(OpenTTDTrainServices, read_next_force), offsetof(OpenTTDTrainServices, read_reverse_command), offsetof(OpenTTDTrainServices, write_tile), offsetof(OpenTTDTrainServices, write_dest), offsetof(OpenTTDTrainServices, write_x), offsetof(OpenTTDTrainServices, write_y), offsetof(OpenTTDTrainServices, write_z), offsetof(OpenTTDTrainServices, write_direction), offsetof(OpenTTDTrainServices, write_speed), offsetof(OpenTTDTrainServices, write_tick), offsetof(OpenTTDTrainServices, write_running), offsetof(OpenTTDTrainServices, write_day), offsetof(OpenTTDTrainServices, write_order_time), offsetof(OpenTTDTrainServices, write_progress), offsetof(OpenTTDTrainServices, write_subspeed), offsetof(OpenTTDTrainServices, write_gv_flags), offsetof(OpenTTDTrainServices, write_acceleration), offsetof(OpenTTDTrainServices, write_length), offsetof(OpenTTDTrainServices, write_total_length), offsetof(OpenTTDTrainServices, write_first_engine), offsetof(OpenTTDTrainServices, write_max_speed), offsetof(OpenTTDTrainServices, write_cargo_cap), offsetof(OpenTTDTrainServices, write_refit_cap), offsetof(OpenTTDTrainServices, write_cargo_age), offsetof(OpenTTDTrainServices, write_last_station), offsetof(OpenTTDTrainServices, write_colourmap), offsetof(OpenTTDTrainServices, write_status), offsetof(OpenTTDTrainServices, acceleration), offsetof(OpenTTDTrainServices, acc_model), offsetof(OpenTTDTrainServices, acc_type), offsetof(OpenTTDTrainServices, advance_distance), offsetof(OpenTTDTrainServices, age), offsetof(OpenTTDTrainServices, all_powered), offsetof(OpenTTDTrainServices, ambient_sound), offsetof(OpenTTDTrainServices, arrival_news), offsetof(OpenTTDTrainServices, arrival_triggers), offsetof(OpenTTDTrainServices, axis_diag), offsetof(OpenTTDTrainServices, backoff), offsetof(OpenTTDTrainServices, base_viewport), offsetof(OpenTTDTrainServices, begin_loading), offsetof(OpenTTDTrainServices, bridge_speed), offsetof(OpenTTDTrainServices, cache_override), offsetof(OpenTTDTrainServices, callback_length), offsetof(OpenTTDTrainServices, capacity), offsetof(OpenTTDTrainServices, capacity_error), offsetof(OpenTTDTrainServices, cargo_age_default), offsetof(OpenTTDTrainServices, cargo_changed), offsetof(OpenTTDTrainServices, chain_depot), offsetof(OpenTTDTrainServices, check_breakdown), offsetof(OpenTTDTrainServices, check_next), offsetof(OpenTTDTrainServices, check_orders), offsetof(OpenTTDTrainServices, check_reverse), offsetof(OpenTTDTrainServices, choose_track), offsetof(OpenTTDTrainServices, clear_reservation), offsetof(OpenTTDTrainServices, compatible_rail_owner), offsetof(OpenTTDTrainServices, consist_windows), offsetof(OpenTTDTrainServices, cost_class), offsetof(OpenTTDTrainServices, cost_default), offsetof(OpenTTDTrainServices, cost_divisor), offsetof(OpenTTDTrainServices, count_chain), offsetof(OpenTTDTrainServices, crash_event), offsetof(OpenTTDTrainServices, crash_ground), offsetof(OpenTTDTrainServices, crash_news), offsetof(OpenTTDTrainServices, crash_rating), offsetof(OpenTTDTrainServices, crash_sound), offsetof(OpenTTDTrainServices, crossing_barred), offsetof(OpenTTDTrainServices, crossing_rail_axis), offsetof(OpenTTDTrainServices, crossing_reserved), offsetof(OpenTTDTrainServices, crossing_road_axis), offsetof(OpenTTDTrainServices, crossing_sound), offsetof(OpenTTDTrainServices, curve_advantage), offsetof(OpenTTDTrainServices, curve_mod), offsetof(OpenTTDTrainServices, day_ticks), offsetof(OpenTTDTrainServices, decrease_value), offsetof(OpenTTDTrainServices, delete_vehicle), offsetof(OpenTTDTrainServices, depot_dir), offsetof(OpenTTDTrainServices, depot_dirty), offsetof(OpenTTDTrainServices, depot_index), offsetof(OpenTTDTrainServices, depot_track), offsetof(OpenTTDTrainServices, depot_window), offsetof(OpenTTDTrainServices, diag_axis), offsetof(OpenTTDTrainServices, diag_between), offsetof(OpenTTDTrainServices, diag_reaches_tracks), offsetof(OpenTTDTrainServices, diag_trackdir), offsetof(OpenTTDTrainServices, dirty_tile), offsetof(OpenTTDTrainServices, dir_diag), offsetof(OpenTTDTrainServices, disaster_sound), offsetof(OpenTTDTrainServices, disconnect), offsetof(OpenTTDTrainServices, economy_age), offsetof(OpenTTDTrainServices, engine_power), offsetof(OpenTTDTrainServices, enter_depot), offsetof(OpenTTDTrainServices, enter_tile), offsetof(OpenTTDTrainServices, find_depot), offsetof(OpenTTDTrainServices, first_track), offsetof(OpenTTDTrainServices, free_reservation), offsetof(OpenTTDTrainServices, grf_version), offsetof(OpenTTDTrainServices, handle_breakdown), offsetof(OpenTTDTrainServices, has_depot_res), offsetof(OpenTTDTrainServices, has_reserved), offsetof(OpenTTDTrainServices, has_signal), offsetof(OpenTTDTrainServices, has_signals), offsetof(OpenTTDTrainServices, has_signal_td), offsetof(OpenTTDTrainServices, hide_fill), offsetof(OpenTTDTrainServices, inclination), offsetof(OpenTTDTrainServices, invalidate_grf), offsetof(OpenTTDTrainServices, invalid_price), offsetof(OpenTTDTrainServices, is_bridge), offsetof(OpenTTDTrainServices, is_crossing), offsetof(OpenTTDTrainServices, is_depot), offsetof(OpenTTDTrainServices, is_plain_rail), offsetof(OpenTTDTrainServices, is_railway), offsetof(OpenTTDTrainServices, is_station), offsetof(OpenTTDTrainServices, is_station_any), offsetof(OpenTTDTrainServices, is_tunnelbridge), offsetof(OpenTTDTrainServices, large_explosion), offsetof(OpenTTDTrainServices, last_speed), offsetof(OpenTTDTrainServices, leave_sound), offsetof(OpenTTDTrainServices, leave_station), offsetof(OpenTTDTrainServices, leave_unbunch), offsetof(OpenTTDTrainServices, length_callback), offsetof(OpenTTDTrainServices, length_changed), offsetof(OpenTTDTrainServices, length_default), offsetof(OpenTTDTrainServices, length_error), offsetof(OpenTTDTrainServices, loading), offsetof(OpenTTDTrainServices, local_company), offsetof(OpenTTDTrainServices, lost_warn), offsetof(OpenTTDTrainServices, map_size), offsetof(OpenTTDTrainServices, max_depot_penalty), offsetof(OpenTTDTrainServices, needs_service), offsetof(OpenTTDTrainServices, no_90), offsetof(OpenTTDTrainServices, oneway_blocking), offsetof(OpenTTDTrainServices, order_depot_service), offsetof(OpenTTDTrainServices, order_dummy), offsetof(OpenTTDTrainServices, order_free), offsetof(OpenTTDTrainServices, order_max_speed), offsetof(OpenTTDTrainServices, order_stop), offsetof(OpenTTDTrainServices, other_end), offsetof(OpenTTDTrainServices, pay_running), offsetof(OpenTTDTrainServices, pbs_signal_type), offsetof(OpenTTDTrainServices, platform_ahead), offsetof(OpenTTDTrainServices, platform_length), offsetof(OpenTTDTrainServices, position), offsetof(OpenTTDTrainServices, pow_wag_power)});
	Layout(384, "OpenTTDTrainServices", {sizeof(OpenTTDTrainServices), alignof(OpenTTDTrainServices), offsetof(OpenTTDTrainServices, price), offsetof(OpenTTDTrainServices, process_orders), offsetof(OpenTTDTrainServices, property), offsetof(OpenTTDTrainServices, railveh_wagon), offsetof(OpenTTDTrainServices, rail_tilt), offsetof(OpenTTDTrainServices, rail_type), offsetof(OpenTTDTrainServices, rail_types), offsetof(OpenTTDTrainServices, reserve_paths), offsetof(OpenTTDTrainServices, reserve_under), offsetof(OpenTTDTrainServices, reset_unbunch), offsetof(OpenTTDTrainServices, reverse_at_signals), offsetof(OpenTTDTrainServices, reverse_single_blocked), offsetof(OpenTTDTrainServices, reverse_windows), offsetof(OpenTTDTrainServices, running_windows), offsetof(OpenTTDTrainServices, service), offsetof(OpenTTDTrainServices, servint), offsetof(OpenTTDTrainServices, set_depot_res), offsetof(OpenTTDTrainServices, set_next), offsetof(OpenTTDTrainServices, set_platform_res), offsetof(OpenTTDTrainServices, set_signal_state), offsetof(OpenTTDTrainServices, set_tunnel_res), offsetof(OpenTTDTrainServices, show_effect), offsetof(OpenTTDTrainServices, show_reservation), offsetof(OpenTTDTrainServices, signal_context), offsetof(OpenTTDTrainServices, signal_has_pbs), offsetof(OpenTTDTrainServices, signal_pbs), offsetof(OpenTTDTrainServices, signal_type), offsetof(OpenTTDTrainServices, sigseg_full), offsetof(OpenTTDTrainServices, sigseg_pbs), offsetof(OpenTTDTrainServices, small_explosion), offsetof(OpenTTDTrainServices, speed_default), offsetof(OpenTTDTrainServices, start_stop_dirty), offsetof(OpenTTDTrainServices, station), offsetof(OpenTTDTrainServices, station_axis), offsetof(OpenTTDTrainServices, station_compatible), offsetof(OpenTTDTrainServices, station_dest), offsetof(OpenTTDTrainServices, stopped_in_depot), offsetof(OpenTTDTrainServices, stop_location), offsetof(OpenTTDTrainServices, stuck_news), offsetof(OpenTTDTrainServices, suppress_implicit), offsetof(OpenTTDTrainServices, ticks_leave_depot), offsetof(OpenTTDTrainServices, tile_add_diag), offsetof(OpenTTDTrainServices, tile_offset_axis), offsetof(OpenTTDTrainServices, tile_offset_diag), offsetof(OpenTTDTrainServices, tile_owner), offsetof(OpenTTDTrainServices, tile_rail_type), offsetof(OpenTTDTrainServices, tile_virt), offsetof(OpenTTDTrainServices, trackdir_exit), offsetof(OpenTTDTrainServices, trackdir_reaches), offsetof(OpenTTDTrainServices, track_bits), offsetof(OpenTTDTrainServices, track_crosses), offsetof(OpenTTDTrainServices, track_direction), offsetof(OpenTTDTrainServices, track_status), offsetof(OpenTTDTrainServices, train_list), offsetof(OpenTTDTrainServices, train_visit), offsetof(OpenTTDTrainServices, truncate_cargo), offsetof(OpenTTDTrainServices, try_path), offsetof(OpenTTDTrainServices, try_reserve), offsetof(OpenTTDTrainServices, tunnel_dir), offsetof(OpenTTDTrainServices, unreserve), offsetof(OpenTTDTrainServices, update_delta), offsetof(OpenTTDTrainServices, update_speed), offsetof(OpenTTDTrainServices, user_default), offsetof(OpenTTDTrainServices, veh_exit_dir), offsetof(OpenTTDTrainServices, viewport), offsetof(OpenTTDTrainServices, view_window), offsetof(OpenTTDTrainServices, visit_type), offsetof(OpenTTDTrainServices, vis_effect), offsetof(OpenTTDTrainServices, wagon_override), offsetof(OpenTTDTrainServices, wagon_speed_limits), offsetof(OpenTTDTrainServices, wait_oneway), offsetof(OpenTTDTrainServices, wait_pbs), offsetof(OpenTTDTrainServices, wait_twoway), offsetof(OpenTTDTrainServices, wait_unbunch), offsetof(OpenTTDTrainServices, write_crossing_bar), offsetof(OpenTTDTrainServices, write_crossing_res), offsetof(OpenTTDTrainServices, write_visit_type), offsetof(OpenTTDTrainServices, visit_tile), offsetof(OpenTTDTrainServices, visit_near)});
	Layout(383, "OpenTTDTrainDepot", {sizeof(OpenTTDTrainDepot), alignof(OpenTTDTrainDepot), offsetof(OpenTTDTrainDepot,tile), offsetof(OpenTTDTrainDepot,length)});
	Layout(373, "OpenTTDTrainReservationFreeRead", {sizeof(OpenTTDTrainReservationFreeRead), alignof(OpenTTDTrainReservationFreeRead), offsetof(OpenTTDTrainReservationFreeRead, tile)});
	Layout(374, "OpenTTDTrainReservationFree1Read", {sizeof(OpenTTDTrainReservationFree1Read), alignof(OpenTTDTrainReservationFree1Read), offsetof(OpenTTDTrainReservationFree1Read, tile), offsetof(OpenTTDTrainReservationFree1Read, next)});
	Layout(375, "OpenTTDTrainReservationNewRead", {sizeof(OpenTTDTrainReservationNewRead), alignof(OpenTTDTrainReservationNewRead), offsetof(OpenTTDTrainReservationNewRead, dest), offsetof(OpenTTDTrainReservationNewRead, last_station), offsetof(OpenTTDTrainReservationNewRead, order_index), offsetof(OpenTTDTrainReservationNewRead, suppress)});
	Layout(376, "OpenTTDTrainReservationChooseRead", {sizeof(OpenTTDTrainReservationChooseRead), alignof(OpenTTDTrainReservationChooseRead), offsetof(OpenTTDTrainReservationChooseRead, tile), offsetof(OpenTTDTrainReservationChooseRead, dest), offsetof(OpenTTDTrainReservationChooseRead, destination), offsetof(OpenTTDTrainReservationChooseRead, order)});
	Layout(377, "OpenTTDTrainReservationChoose4Read", {sizeof(OpenTTDTrainReservationChoose4Read), alignof(OpenTTDTrainReservationChoose4Read), offsetof(OpenTTDTrainReservationChoose4Read, order), offsetof(OpenTTDTrainReservationChoose4Read, nearest)});
	Layout(378, "OpenTTDTrainReservationCheckNextRead", {sizeof(OpenTTDTrainReservationCheckNextRead), alignof(OpenTTDTrainReservationCheckNextRead), offsetof(OpenTTDTrainReservationCheckNextRead, tile), offsetof(OpenTTDTrainReservationCheckNextRead, dest), offsetof(OpenTTDTrainReservationCheckNextRead, destination), offsetof(OpenTTDTrainReservationCheckNextRead, order), offsetof(OpenTTDTrainReservationCheckNextRead, num_orders)});
	Layout(379, "OpenTTDTrainReservationFollow", {sizeof(OpenTTDTrainReservationFollow), alignof(OpenTTDTrainReservationFollow), offsetof(OpenTTDTrainReservationFollow, old_tile), offsetof(OpenTTDTrainReservationFollow, new_tile), offsetof(OpenTTDTrainReservationFollow, skipped), offsetof(OpenTTDTrainReservationFollow, dirs), offsetof(OpenTTDTrainReservationFollow, old_td), offsetof(OpenTTDTrainReservationFollow, exitdir), offsetof(OpenTTDTrainReservationFollow, tunnel), offsetof(OpenTTDTrainReservationFollow, bridge), offsetof(OpenTTDTrainReservationFollow, station), offsetof(OpenTTDTrainReservationFollow, error)});
	Layout(380, "OpenTTDTrainReservationPbs", {sizeof(OpenTTDTrainReservationPbs), alignof(OpenTTDTrainReservationPbs), offsetof(OpenTTDTrainReservationPbs, tile), offsetof(OpenTTDTrainReservationPbs, other), offsetof(OpenTTDTrainReservationPbs, td), offsetof(OpenTTDTrainReservationPbs, okay)});
	Layout(381, "OpenTTDTrainReservationSearch", {sizeof(OpenTTDTrainReservationSearch),alignof(OpenTTDTrainReservationSearch),offsetof(OpenTTDTrainReservationSearch,track),offsetof(OpenTTDTrainReservationSearch,tile),offsetof(OpenTTDTrainReservationSearch,final_dest),offsetof(OpenTTDTrainReservationSearch,td),offsetof(OpenTTDTrainReservationSearch,found),offsetof(OpenTTDTrainReservationSearch,okay)});
	Layout(382, "OpenTTDTrainReservationLeaves", {sizeof(OpenTTDTrainReservationLeaves), alignof(OpenTTDTrainReservationLeaves), offsetof(OpenTTDTrainReservationLeaves, read_tile), offsetof(OpenTTDTrainReservationLeaves, read_next), offsetof(OpenTTDTrainReservationLeaves, read_last_station), offsetof(OpenTTDTrainReservationLeaves, read_direction), offsetof(OpenTTDTrainReservationLeaves, read_order), offsetof(OpenTTDTrainReservationLeaves, read_num_orders), offsetof(OpenTTDTrainReservationLeaves, read_order_index), offsetof(OpenTTDTrainReservationLeaves, read_free), offsetof(OpenTTDTrainReservationLeaves, read_free_1), offsetof(OpenTTDTrainReservationLeaves, read_new), offsetof(OpenTTDTrainReservationLeaves, read_choose), offsetof(OpenTTDTrainReservationLeaves, read_choose_4), offsetof(OpenTTDTrainReservationLeaves, read_check_next), offsetof(OpenTTDTrainReservationLeaves, all_compat), offsetof(OpenTTDTrainReservationLeaves, backoff), offsetof(OpenTTDTrainReservationLeaves, bits_track), offsetof(OpenTTDTrainReservationLeaves, blocking), offsetof(OpenTTDTrainReservationLeaves, check_reverse), offsetof(OpenTTDTrainReservationLeaves, compat_station), offsetof(OpenTTDTrainReservationLeaves, conditional), offsetof(OpenTTDTrainReservationLeaves, copy_order), offsetof(OpenTTDTrainReservationLeaves, cross_dirs), offsetof(OpenTTDTrainReservationLeaves, cross_tracks), offsetof(OpenTTDTrainReservationLeaves, depot_dir), offsetof(OpenTTDTrainReservationLeaves, depot_reserved), offsetof(OpenTTDTrainReservationLeaves, diag_reach_dirs), offsetof(OpenTTDTrainReservationLeaves, diag_track), offsetof(OpenTTDTrainReservationLeaves, enter_td), offsetof(OpenTTDTrainReservationLeaves, exit_dir), offsetof(OpenTTDTrainReservationLeaves, free), offsetof(OpenTTDTrainReservationLeaves, green), offsetof(OpenTTDTrainReservationLeaves, has_pbs), offsetof(OpenTTDTrainReservationLeaves, has_reserved), offsetof(OpenTTDTrainReservationLeaves, has_signal), offsetof(OpenTTDTrainReservationLeaves, increment_order), offsetof(OpenTTDTrainReservationLeaves, is_bridge), offsetof(OpenTTDTrainReservationLeaves, is_depot), offsetof(OpenTTDTrainReservationLeaves, is_pbs), offsetof(OpenTTDTrainReservationLeaves, is_plain), offsetof(OpenTTDTrainReservationLeaves, is_railway), offsetof(OpenTTDTrainReservationLeaves, is_station), offsetof(OpenTTDTrainReservationLeaves, is_tunnel), offsetof(OpenTTDTrainReservationLeaves, is_waypoint), offsetof(OpenTTDTrainReservationLeaves, line_reverse), offsetof(OpenTTDTrainReservationLeaves, mark_bridge), offsetof(OpenTTDTrainReservationLeaves, mark_tile), offsetof(OpenTTDTrainReservationLeaves, needs_service), offsetof(OpenTTDTrainReservationLeaves, oneway), offsetof(OpenTTDTrainReservationLeaves, order_service), offsetof(OpenTTDTrainReservationLeaves, order_stop), offsetof(OpenTTDTrainReservationLeaves, order_type), offsetof(OpenTTDTrainReservationLeaves, other_end), offsetof(OpenTTDTrainReservationLeaves, overlap), offsetof(OpenTTDTrainReservationLeaves, path_result), offsetof(OpenTTDTrainReservationLeaves, rail90), offsetof(OpenTTDTrainReservationLeaves, reach_dirs), offsetof(OpenTTDTrainReservationLeaves, reach_tracks), offsetof(OpenTTDTrainReservationLeaves, reserved), offsetof(OpenTTDTrainReservationLeaves, reserve_paths), offsetof(OpenTTDTrainReservationLeaves, restore_order), offsetof(OpenTTDTrainReservationLeaves, safe), offsetof(OpenTTDTrainReservationLeaves, save_order), offsetof(OpenTTDTrainReservationLeaves, service), offsetof(OpenTTDTrainReservationLeaves, set_depot), offsetof(OpenTTDTrainReservationLeaves, set_depot_dest), offsetof(OpenTTDTrainReservationLeaves, set_platform), offsetof(OpenTTDTrainReservationLeaves, set_signal), offsetof(OpenTTDTrainReservationLeaves, set_tunnel), offsetof(OpenTTDTrainReservationLeaves, show_res), offsetof(OpenTTDTrainReservationLeaves, signal_context), offsetof(OpenTTDTrainReservationLeaves, vehicle), offsetof(OpenTTDTrainReservationLeaves, start_stop), offsetof(OpenTTDTrainReservationLeaves, station), offsetof(OpenTTDTrainReservationLeaves, station_train), offsetof(OpenTTDTrainReservationLeaves, station_xy), offsetof(OpenTTDTrainReservationLeaves, stuck), offsetof(OpenTTDTrainReservationLeaves, tile_add), offsetof(OpenTTDTrainReservationLeaves, tile_offset), offsetof(OpenTTDTrainReservationLeaves, trackdir), offsetof(OpenTTDTrainReservationLeaves, track_status), offsetof(OpenTTDTrainReservationLeaves, try_track), offsetof(OpenTTDTrainReservationLeaves, tunnel_dir), offsetof(OpenTTDTrainReservationLeaves, tunnel_free), offsetof(OpenTTDTrainReservationLeaves, unreserve), offsetof(OpenTTDTrainReservationLeaves, write_dest), offsetof(OpenTTDTrainReservationLeaves, write_last), offsetof(OpenTTDTrainReservationLeaves, write_suppress), offsetof(OpenTTDTrainReservationLeaves, follow), offsetof(OpenTTDTrainReservationLeaves, origin), offsetof(OpenTTDTrainReservationLeaves, pathfind), offsetof(OpenTTDTrainReservationLeaves, safe_track), offsetof(OpenTTDTrainReservationLeaves, process_orders), offsetof(OpenTTDTrainReservationLeaves, update_order_dest)});
	Layout(385, "OpenTTDTrainReservationChoice", {sizeof(OpenTTDTrainReservationChoice),alignof(OpenTTDTrainReservationChoice),offsetof(OpenTTDTrainReservationChoice,track),offsetof(OpenTTDTrainReservationChoice,reserved)});
	Layout(49, "OpenTTDCargoSpec", {sizeof(OpenTTDCargoSpec), alignof(OpenTTDCargoSpec), offsetof(OpenTTDCargoSpec, payment), offsetof(OpenTTDCargoSpec, valid), offsetof(OpenTTDCargoSpec, callback), offsetof(OpenTTDCargoSpec, periods1), offsetof(OpenTTDCargoSpec, periods2)});
	Layout(50, "OpenTTDCargoPaymentFields", {sizeof(OpenTTDCargoPaymentFields), alignof(OpenTTDCargoPaymentFields), offsetof(OpenTTDCargoPaymentFields, front), offsetof(OpenTTDCargoPaymentFields, route_profit), offsetof(OpenTTDCargoPaymentFields, visual_profit), offsetof(OpenTTDCargoPaymentFields, visual_transfer)});
	Layout(60, "OpenTTDCargoPayment", {sizeof(OpenTTDCargoPayment), alignof(OpenTTDCargoPayment), offsetof(OpenTTDCargoPayment, fields), offsetof(OpenTTDCargoPayment, station)});
	Layout(51, "OpenTTDCargoServices", {sizeof(OpenTTDCargoServices), alignof(OpenTTDCargoServices), offsetof(OpenTTDCargoServices, spec), offsetof(OpenTTDCargoServices, callback), offsetof(OpenTTDCargoServices, nearby_industry), offsetof(OpenTTDCargoServices, station_read), offsetof(OpenTTDCargoServices, industry_read), offsetof(OpenTTDCargoServices, industry_write), offsetof(OpenTTDCargoServices, refuses), offsetof(OpenTTDCargoServices, accept), offsetof(OpenTTDCargoServices, statistics), offsetof(OpenTTDCargoServices, monitor), offsetof(OpenTTDCargoServices, subsidised), offsetof(OpenTTDCargoServices, industry_effect), offsetof(OpenTTDCargoServices, vehicle_read), offsetof(OpenTTDCargoServices, settle), offsetof(OpenTTDCargoServices, feeder), offsetof(OpenTTDCargoServices, setting)});
	Layout(52, "OpenTTDShipYapfInput", {sizeof(OpenTTDShipYapfInput), alignof(OpenTTDShipYapfInput), offsetof(OpenTTDShipYapfInput, map_x), offsetof(OpenTTDShipYapfInput, map_y), offsetof(OpenTTDShipYapfInput, tile), offsetof(OpenTTDShipYapfInput, dest_tile), offsetof(OpenTTDShipYapfInput, curve90), offsetof(OpenTTDShipYapfInput, curve45), offsetof(OpenTTDShipYapfInput, max_speed), offsetof(OpenTTDShipYapfInput, dest_dirs), offsetof(OpenTTDShipYapfInput, reverse_dirs), offsetof(OpenTTDShipYapfInput, trackdir), offsetof(OpenTTDShipYapfInput, ocean_frac), offsetof(OpenTTDShipYapfInput, canal_frac), offsetof(OpenTTDShipYapfInput, station), offsetof(OpenTTDShipYapfInput, unit_number)});
	Layout(53, "OpenTTDShipYapfLeaves", {sizeof(OpenTTDShipYapfLeaves), alignof(OpenTTDShipYapfLeaves), offsetof(OpenTTDShipYapfLeaves, destination), offsetof(OpenTTDShipYapfLeaves, follow), offsetof(OpenTTDShipYapfLeaves, tile), offsetof(OpenTTDShipYapfLeaves, patch), offsetof(OpenTTDShipYapfLeaves, visit_new), offsetof(OpenTTDShipYapfLeaves, visit_next), offsetof(OpenTTDShipYapfLeaves, visit_destroy), offsetof(OpenTTDShipYapfLeaves, debug)});
	Layout(54, "OpenTTDShipFollow", {sizeof(OpenTTDShipFollow), alignof(OpenTTDShipFollow), offsetof(OpenTTDShipFollow, tile), offsetof(OpenTTDShipFollow, skipped), offsetof(OpenTTDShipFollow, dirs), offsetof(OpenTTDShipFollow, followed)});
	Layout(55, "OpenTTDShipTile", {sizeof(OpenTTDShipTile), alignof(OpenTTDShipTile), offsetof(OpenTTDShipTile, ships), offsetof(OpenTTDShipTile, docking), offsetof(OpenTTDShipTile, sea), offsetof(OpenTTDShipTile, lock_middle), offsetof(OpenTTDShipTile, destination)});
	Layout(56, "OpenTTDShipYapfResult", {sizeof(OpenTTDShipYapfResult), alignof(OpenTTDShipYapfResult), offsetof(OpenTTDShipYapfResult, direction), offsetof(OpenTTDShipYapfResult, found), offsetof(OpenTTDShipYapfResult, origin), offsetof(OpenTTDShipYapfResult, stats)});

	Layout(432, "OpenTTDTownSimulationFields", {sizeof(OpenTTDTownSimulationFields), alignof(OpenTTDTownSimulationFields), offsetof(OpenTTDTownSimulationFields, valid_history), offsetof(OpenTTDTownSimulationFields, xy), offsetof(OpenTTDTownSimulationFields, num_houses), offsetof(OpenTTDTownSimulationFields, population), offsetof(OpenTTDTownSimulationFields, radius), offsetof(OpenTTDTownSimulationFields, goal), offsetof(OpenTTDTownSimulationFields, ratings), offsetof(OpenTTDTownSimulationFields, received), offsetof(OpenTTDTownSimulationFields, noise_reached), offsetof(OpenTTDTownSimulationFields, time_until_rebuild), offsetof(OpenTTDTownSimulationFields, statues), offsetof(OpenTTDTownSimulationFields, have_ratings), offsetof(OpenTTDTownSimulationFields, unwanted), offsetof(OpenTTDTownSimulationFields, exclusivity), offsetof(OpenTTDTownSimulationFields, exclusive_counter), offsetof(OpenTTDTownSimulationFields, larger_town), offsetof(OpenTTDTownSimulationFields, layout), offsetof(OpenTTDTownSimulationFields, townnamegrfid), offsetof(OpenTTDTownSimulationFields, townnametype), offsetof(OpenTTDTownSimulationFields, townnameparts)});
	Layout(433, "OpenTTDTownReceived", {sizeof(OpenTTDTownReceived), alignof(OpenTTDTownReceived), offsetof(OpenTTDTownReceived, old_max), offsetof(OpenTTDTownReceived, new_max), offsetof(OpenTTDTownReceived, old_act), offsetof(OpenTTDTownReceived, new_act)});
	Layout(434, "OpenTTDTownSuppliedHistory", {sizeof(OpenTTDTownSuppliedHistory), alignof(OpenTTDTownSuppliedHistory), offsetof(OpenTTDTownSuppliedHistory, production), offsetof(OpenTTDTownSuppliedHistory, transported)});
	Layout(435, "OpenTTDTownSupplied", {sizeof(OpenTTDTownSupplied), alignof(OpenTTDTownSupplied), offsetof(OpenTTDTownSupplied, cargo), offsetof(OpenTTDTownSupplied, history)});
	Layout(439, "OpenTTDTownRoadChoice", {sizeof(OpenTTDTownRoadChoice), alignof(OpenTTDTownRoadChoice), offsetof(OpenTTDTownRoadChoice, introduction_date), offsetof(OpenTTDTownRoadChoice, max_speed), offsetof(OpenTTDTownRoadChoice, town_build)});
	Layout(440, "OpenTTDTownHouseAvailability", {sizeof(OpenTTDTownHouseAvailability), alignof(OpenTTDTownHouseAvailability), offsetof(OpenTTDTownHouseAvailability, zones), offsetof(OpenTTDTownHouseAvailability, override_id), offsetof(OpenTTDTownHouseAvailability, class_id), offsetof(OpenTTDTownHouseAvailability, probability), offsetof(OpenTTDTownHouseAvailability, enabled)});
	Layout(441, "OpenTTDTownHousePlacement", {sizeof(OpenTTDTownHousePlacement), alignof(OpenTTDTownHousePlacement), offsetof(OpenTTDTownHousePlacement, min_year), offsetof(OpenTTDTownHousePlacement, max_year), offsetof(OpenTTDTownHousePlacement, flags), offsetof(OpenTTDTownHousePlacement, extra_flags)});
	Layout(442, "OpenTTDTownHouseConstruction", {sizeof(OpenTTDTownHouseConstruction), alignof(OpenTTDTownHouseConstruction), offsetof(OpenTTDTownHouseConstruction, extra_flags), offsetof(OpenTTDTownHouseConstruction, population)});
	Layout(438, "OpenTTDTownStationActivity", {sizeof(OpenTTDTownStationActivity), alignof(OpenTTDTownStationActivity), offsetof(OpenTTDTownStationActivity, time_since_load), offsetof(OpenTTDTownStationActivity, time_since_unload)});
	Layout(437, "OpenTTDTownGrowthFields", {sizeof(OpenTTDTownGrowthFields), alignof(OpenTTDTownGrowthFields), offsetof(OpenTTDTownGrowthFields, counter), offsetof(OpenTTDTownGrowthFields, rate), offsetof(OpenTTDTownGrowthFields, funding), offsetof(OpenTTDTownGrowthFields, road), offsetof(OpenTTDTownGrowthFields, flags)});
	Layout(436, "OpenTTDTownSlots", {sizeof(OpenTTDTownSlots), alignof(OpenTTDTownSlots), offsetof(OpenTTDTownSlots, data), offsetof(OpenTTDTownSlots, size)});
	Layout(430, "OpenTTDTownCommandResult", {sizeof(OpenTTDTownCommandResult), alignof(OpenTTDTownCommandResult), offsetof(OpenTTDTownCommandResult, cost), offsetof(OpenTTDTownCommandResult, success)});
	Layout(431, "OpenTTDTownLeaves", {sizeof(OpenTTDTownLeaves), alignof(OpenTTDTownLeaves), offsetof(OpenTTDTownLeaves, map_log_x), offsetof(OpenTTDTownLeaves, build_on_slopes), offsetof(OpenTTDTownLeaves, growth_rate), offsetof(OpenTTDTownLeaves, snow_line), offsetof(OpenTTDTownLeaves, editor), offsetof(OpenTTDTownLeaves, generating_world), offsetof(OpenTTDTownLeaves, allow_crossings), offsetof(OpenTTDTownLeaves, climate), offsetof(OpenTTDTownLeaves, highest_snow_line), offsetof(OpenTTDTownLeaves, calendar_year), offsetof(OpenTTDTownLeaves, allow_roads), offsetof(OpenTTDTownLeaves, road_flags), offsetof(OpenTTDTownLeaves, house_availability), offsetof(OpenTTDTownLeaves, house_placement), offsetof(OpenTTDTownLeaves, house_construction), offsetof(OpenTTDTownLeaves, house_flags), offsetof(OpenTTDTownLeaves, house_extra_flags), offsetof(OpenTTDTownLeaves, house_allows_callback), offsetof(OpenTTDTownLeaves, road_info), offsetof(OpenTTDTownLeaves, state), offsetof(OpenTTDTownLeaves, stations), offsetof(OpenTTDTownLeaves, station_activity_owner), offsetof(OpenTTDTownLeaves, station_owner), offsetof(OpenTTDTownLeaves, dirty_authority), offsetof(OpenTTDTownLeaves, dirty_town), offsetof(OpenTTDTownLeaves, foundation_slope), offsetof(OpenTTDTownLeaves, terraform_price), offsetof(OpenTTDTownLeaves, clean_road_bits), offsetof(OpenTTDTownLeaves, set_road_town), offsetof(OpenTTDTownLeaves, exchange_company), offsetof(OpenTTDTownLeaves, house_class), offsetof(OpenTTDTownLeaves, make_house), offsetof(OpenTTDTownLeaves, add_animation), offsetof(OpenTTDTownLeaves, population_changed), offsetof(OpenTTDTownLeaves, nearby_stations), offsetof(OpenTTDTownLeaves, house_count), offsetof(OpenTTDTownLeaves, update_max_pass), offsetof(OpenTTDTownLeaves, next_town), offsetof(OpenTTDTownLeaves, road_mask), offsetof(OpenTTDTownLeaves, build_road), offsetof(OpenTTDTownLeaves, clear_land), offsetof(OpenTTDTownLeaves, terraform_land), offsetof(OpenTTDTownLeaves, build_bridge), offsetof(OpenTTDTownLeaves, build_tunnel), offsetof(OpenTTDTownLeaves, clear_house_land), offsetof(OpenTTDTownLeaves, allow_construction), offsetof(OpenTTDTownLeaves, construction_animation)});
	Layout(90, "OpenTTDRailSettings", {sizeof(OpenTTDRailSettings), alignof(OpenTTDRailSettings), offsetof(OpenTTDRailSettings, max_nodes), offsetof(OpenTTDRailSettings, firstred), offsetof(OpenTTDRailSettings, firstred_exit), offsetof(OpenTTDRailSettings, lastred), offsetof(OpenTTDRailSettings, lastred_exit), offsetof(OpenTTDRailSettings, station), offsetof(OpenTTDRailSettings, slope), offsetof(OpenTTDRailSettings, curve45), offsetof(OpenTTDRailSettings, curve90), offsetof(OpenTTDRailSettings, depot_reverse), offsetof(OpenTTDRailSettings, crossing), offsetof(OpenTTDRailSettings, lookahead), offsetof(OpenTTDRailSettings, p0), offsetof(OpenTTDRailSettings, p1), offsetof(OpenTTDRailSettings, p2), offsetof(OpenTTDRailSettings, pbs_cross), offsetof(OpenTTDRailSettings, pbs_station), offsetof(OpenTTDRailSettings, pbs_back), offsetof(OpenTTDRailSettings, doubleslip), offsetof(OpenTTDRailSettings, longer), offsetof(OpenTTDRailSettings, longer_tile), offsetof(OpenTTDRailSettings, shorter), offsetof(OpenTTDRailSettings, shorter_tile), offsetof(OpenTTDRailSettings, firstred_eol)});
	Layout(91, "OpenTTDRailTile", {sizeof(OpenTTDRailTile), alignof(OpenTTDRailTile), offsetof(OpenTTDRailTile, flags), offsetof(OpenTTDRailTile, other_end), offsetof(OpenTTDRailTile, station), offsetof(OpenTTDRailTile, railtype), offsetof(OpenTTDRailTile, tracks), offsetof(OpenTTDRailTile, reserved), offsetof(OpenTTDRailTile, station_track), offsetof(OpenTTDRailTile, tunnel_dir), offsetof(OpenTTDRailTile, uphill), offsetof(OpenTTDRailTile, flat_ramp), offsetof(OpenTTDRailTile, signal_along), offsetof(OpenTTDRailTile, signal_against), offsetof(OpenTTDRailTile, signal_green), offsetof(OpenTTDRailTile, signal_type), offsetof(OpenTTDRailTile, oneway)});
	Layout(92, "OpenTTDRailFollow", {sizeof(OpenTTDRailFollow), alignof(OpenTTDRailFollow), offsetof(OpenTTDRailFollow, tile), offsetof(OpenTTDRailFollow, skipped), offsetof(OpenTTDRailFollow, min_speed), offsetof(OpenTTDRailFollow, max_speed), offsetof(OpenTTDRailFollow, dirs), offsetof(OpenTTDRailFollow, followed), offsetof(OpenTTDRailFollow, error), offsetof(OpenTTDRailFollow, station)});
	Layout(93, "OpenTTDRailLeaves", {sizeof(OpenTTDRailLeaves), alignof(OpenTTDRailLeaves), offsetof(OpenTTDRailLeaves, train), offsetof(OpenTTDRailLeaves, tile), offsetof(OpenTTDRailLeaves, follow), offsetof(OpenTTDRailLeaves, safe), offsetof(OpenTTDRailLeaves, free), offsetof(OpenTTDRailLeaves, compatible_station), offsetof(OpenTTDRailLeaves, platform_length), offsetof(OpenTTDRailLeaves, closest_station), offsetof(OpenTTDRailLeaves, destination_dirs), offsetof(OpenTTDRailLeaves, origin), offsetof(OpenTTDRailLeaves, write), offsetof(OpenTTDRailLeaves, output), offsetof(OpenTTDRailLeaves, debug)});
	Layout(94, "OpenTTDRailInput", {sizeof(OpenTTDRailInput), alignof(OpenTTDRailInput), offsetof(OpenTTDRailInput, context), offsetof(OpenTTDRailInput, settings), offsetof(OpenTTDRailInput, map_x), offsetof(OpenTTDRailInput, tile), offsetof(OpenTTDRailInput, max_cost), offsetof(OpenTTDRailInput, desync), offsetof(OpenTTDRailInput, kind), offsetof(OpenTTDRailInput, td), offsetof(OpenTTDRailInput, override_railtype), offsetof(OpenTTDRailInput, forbid90), offsetof(OpenTTDRailInput, reserve)});
	Layout(95, "OpenTTDRailStep", {sizeof(OpenTTDRailStep), alignof(OpenTTDRailStep), offsetof(OpenTTDRailStep, tile), offsetof(OpenTTDRailStep, destination), offsetof(OpenTTDRailStep, target_tile), offsetof(OpenTTDRailStep, best_length), offsetof(OpenTTDRailStep, action), offsetof(OpenTTDRailStep, td), offsetof(OpenTTDRailStep, found), offsetof(OpenTTDRailStep, reverse), offsetof(OpenTTDRailStep, value), offsetof(OpenTTDRailStep, target_td), offsetof(OpenTTDRailStep, target_okay)});
	Layout(97, "OpenTTDRailTrain", {sizeof(OpenTTDRailTrain), alignof(OpenTTDRailTrain), offsetof(OpenTTDRailTrain, compatible), offsetof(OpenTTDRailTrain, all_compatible), offsetof(OpenTTDRailTrain, tile), offsetof(OpenTTDRailTrain, rear_tile), offsetof(OpenTTDRailTrain, virtual_tile), offsetof(OpenTTDRailTrain, rear_virtual_tile), offsetof(OpenTTDRailTrain, dest_tile), offsetof(OpenTTDRailTrain, length), offsetof(OpenTTDRailTrain, speed), offsetof(OpenTTDRailTrain, order_destination), offsetof(OpenTTDRailTrain, td), offsetof(OpenTTDRailTrain, rear_td), offsetof(OpenTTDRailTrain, wormhole), offsetof(OpenTTDRailTrain, rear_wormhole), offsetof(OpenTTDRailTrain, order), offsetof(OpenTTDRailTrain, nearest_depot), offsetof(OpenTTDRailTrain, complex_waypoint)});

	Layout(280, "OpenTTDCargoPacketFields", {sizeof(OpenTTDCargoPacketFields), alignof(OpenTTDCargoPacketFields), offsetof(OpenTTDCargoPacketFields, feeder_share), offsetof(OpenTTDCargoPacketFields, source_xy), offsetof(OpenTTDCargoPacketFields, count), offsetof(OpenTTDCargoPacketFields, periods_in_transit), offsetof(OpenTTDCargoPacketFields, first_station), offsetof(OpenTTDCargoPacketFields, next_hop), offsetof(OpenTTDCargoPacketFields, source_id), offsetof(OpenTTDCargoPacketFields, travelled_x), offsetof(OpenTTDCargoPacketFields, travelled_y), offsetof(OpenTTDCargoPacketFields, source_type), offsetof(OpenTTDCargoPacketFields, in_vehicle)});
	Layout(281, "OpenTTDCargoListFields", {sizeof(OpenTTDCargoListFields), alignof(OpenTTDCargoListFields), offsetof(OpenTTDCargoListFields, cargo_periods_in_transit), offsetof(OpenTTDCargoListFields, feeder_share), offsetof(OpenTTDCargoListFields, count), offsetof(OpenTTDCargoListFields, reserved_count), offsetof(OpenTTDCargoListFields, action_counts)});
	Layout(282, "OpenTTDCargoStorageServices", {sizeof(OpenTTDCargoStorageServices), alignof(OpenTTDCargoStorageServices), offsetof(OpenTTDCargoStorageServices, can_allocate), offsetof(OpenTTDCargoStorageServices, create), offsetof(OpenTTDCargoStorageServices, packet), offsetof(OpenTTDCargoStorageServices, destroy), offsetof(OpenTTDCargoStorageServices, random), offsetof(OpenTTDCargoStorageServices, coordinate), offsetof(OpenTTDCargoStorageServices, flow), offsetof(OpenTTDCargoStorageServices, pay), offsetof(OpenTTDCargoStorageServices, origin), offsetof(OpenTTDCargoStorageServices, flow_owner), offsetof(OpenTTDCargoStorageServices, random_draw), offsetof(OpenTTDCargoStorageServices, packet_next)});
	Layout(300, "OpenTTDCargoShare", {sizeof(OpenTTDCargoShare), alignof(OpenTTDCargoShare), offsetof(OpenTTDCargoShare, cumulative), offsetof(OpenTTDCargoShare, station), offsetof(OpenTTDCargoShare, found)});
	Layout(301, "OpenTTDCargoOrigin", {sizeof(OpenTTDCargoOrigin), alignof(OpenTTDCargoOrigin), offsetof(OpenTTDCargoOrigin, flow), offsetof(OpenTTDCargoOrigin, origin), offsetof(OpenTTDCargoOrigin, found)});
	Layout(302, "OpenTTDCargoFlowServices", {sizeof(OpenTTDCargoFlowServices), alignof(OpenTTDCargoFlowServices), offsetof(OpenTTDCargoFlowServices, context), offsetof(OpenTTDCargoFlowServices, read), offsetof(OpenTTDCargoFlowServices, job_flows), offsetof(OpenTTDCargoFlowServices, live_flows), offsetof(OpenTTDCargoFlowServices, reroute), offsetof(OpenTTDCargoFlowServices, finish)});
	Layout(310, "OpenTTDCargoCapacityVehicle", {sizeof(OpenTTDCargoCapacityVehicle), alignof(OpenTTDCargoCapacityVehicle), offsetof(OpenTTDCargoCapacityVehicle, list), offsetof(OpenTTDCargoCapacityVehicle, capacity), offsetof(OpenTTDCargoCapacityVehicle, cargo), offsetof(OpenTTDCargoCapacityVehicle, train), offsetof(OpenTTDCargoCapacityVehicle, articulated)});
	Layout(311, "OpenTTDCargoCapacityServices", {sizeof(OpenTTDCargoCapacityServices), alignof(OpenTTDCargoCapacityServices), offsetof(OpenTTDCargoCapacityServices, read), offsetof(OpenTTDCargoCapacityServices, next_part), offsetof(OpenTTDCargoCapacityServices, last_engine_part), offsetof(OpenTTDCargoCapacityServices, other_multiheaded_part), offsetof(OpenTTDCargoCapacityServices, cargo)});
	Layout(260, "OpenTTDOrderFields", {sizeof(OpenTTDOrderFields), alignof(OpenTTDOrderFields), offsetof(OpenTTDOrderFields, type), offsetof(OpenTTDOrderFields, flags), offsetof(OpenTTDOrderFields, destination), offsetof(OpenTTDOrderFields, refit_cargo), offsetof(OpenTTDOrderFields, wait_time), offsetof(OpenTTDOrderFields, travel_time), offsetof(OpenTTDOrderFields, max_speed)});
	Layout(261, "OpenTTDConsistState", {sizeof(OpenTTDConsistState), alignof(OpenTTDConsistState), offsetof(OpenTTDConsistState, current_order_time), offsetof(OpenTTDConsistState, lateness_counter), offsetof(OpenTTDConsistState, timetable_start), offsetof(OpenTTDConsistState, last_departure), offsetof(OpenTTDConsistState, next_departure), offsetof(OpenTTDConsistState, round_trip_time), offsetof(OpenTTDConsistState, real_index), offsetof(OpenTTDConsistState, implicit_index), offsetof(OpenTTDConsistState, vehicle_flags)});
	Layout(262, "OpenTTDVehicleOrderState", {sizeof(OpenTTDVehicleOrderState), alignof(OpenTTDVehicleOrderState), offsetof(OpenTTDVehicleOrderState, current), offsetof(OpenTTDVehicleOrderState, orders), offsetof(OpenTTDVehicleOrderState, next_shared), offsetof(OpenTTDVehicleOrderState, previous_shared)});
	Layout(263, "OpenTTDOrderListState", {sizeof(OpenTTDOrderListState), alignof(OpenTTDOrderListState), offsetof(OpenTTDOrderListState, manual), offsetof(OpenTTDOrderListState, vehicles), offsetof(OpenTTDOrderListState, first_shared), offsetof(OpenTTDOrderListState, timetable_duration), offsetof(OpenTTDOrderListState, total_duration)});
	Layout(264, "OpenTTDOrderBackupState", {sizeof(OpenTTDOrderBackupState), alignof(OpenTTDOrderBackupState), offsetof(OpenTTDOrderBackupState, user), offsetof(OpenTTDOrderBackupState, tile), offsetof(OpenTTDOrderBackupState, group), offsetof(OpenTTDOrderBackupState, clone)});
	Layout(265, "OpenTTDOrdersLeaves", {sizeof(OpenTTDOrdersLeaves), alignof(OpenTTDOrdersLeaves), offsetof(OpenTTDOrdersLeaves, vehicle), offsetof(OpenTTDOrdersLeaves, consist), offsetof(OpenTTDOrdersLeaves, list), offsetof(OpenTTDOrdersLeaves, vector), offsetof(OpenTTDOrdersLeaves, backup), offsetof(OpenTTDOrdersLeaves, backup_vector), offsetof(OpenTTDOrdersLeaves, backup_consist), offsetof(OpenTTDOrdersLeaves, first_vehicle), offsetof(OpenTTDOrdersLeaves, last_station), offsetof(OpenTTDOrdersLeaves, ownerless_station), offsetof(OpenTTDOrdersLeaves, vehicle_type), offsetof(OpenTTDOrdersLeaves, vehicle_status), offsetof(OpenTTDOrdersLeaves, tick_counter), offsetof(OpenTTDOrdersLeaves, primary_vehicle), offsetof(OpenTTDOrdersLeaves, vehicle_ownership), offsetof(OpenTTDOrdersLeaves, ticks_per_second), offsetof(OpenTTDOrdersLeaves, unit_number), offsetof(OpenTTDOrdersLeaves, economy_date), offsetof(OpenTTDOrdersLeaves, economy_fraction), offsetof(OpenTTDOrdersLeaves, maximum_date), offsetof(OpenTTDOrdersLeaves, timetable_year_limit), offsetof(OpenTTDOrdersLeaves, stopped_or_crashed), offsetof(OpenTTDOrdersLeaves, allocate_list), offsetof(OpenTTDOrdersLeaves, suppress_implicit), offsetof(OpenTTDOrdersLeaves, shared_window), offsetof(OpenTTDOrdersLeaves, vehicle_id), offsetof(OpenTTDOrdersLeaves, percent_filled), offsetof(OpenTTDOrdersLeaves, reliability), offsetof(OpenTTDOrdersLeaves, engine_reliability), offsetof(OpenTTDOrdersLeaves, display_speed), offsetof(OpenTTDOrdersLeaves, age_years), offsetof(OpenTTDOrdersLeaves, needs_service), offsetof(OpenTTDOrdersLeaves, remaining_years), offsetof(OpenTTDOrdersLeaves, airport_tile), offsetof(OpenTTDOrdersLeaves, base_station_tile), offsetof(OpenTTDOrdersLeaves, station_tile), offsetof(OpenTTDOrdersLeaves, depot_tile), offsetof(OpenTTDOrdersLeaves, distance), offsetof(OpenTTDOrdersLeaves, station_location), offsetof(OpenTTDOrdersLeaves, destination_tile), offsetof(OpenTTDOrdersLeaves, aircraft_flying), offsetof(OpenTTDOrdersLeaves, target_airport), offsetof(OpenTTDOrdersLeaves, waypoint_tile), offsetof(OpenTTDOrdersLeaves, at_station), offsetof(OpenTTDOrdersLeaves, tile_station), offsetof(OpenTTDOrdersLeaves, ship_station_tile), offsetof(OpenTTDOrdersLeaves, valid_station), offsetof(OpenTTDOrdersLeaves, station_owner), offsetof(OpenTTDOrdersLeaves, can_use_station), offsetof(OpenTTDOrdersLeaves, owner_check), offsetof(OpenTTDOrdersLeaves, station_error), offsetof(OpenTTDOrdersLeaves, has_hangar), offsetof(OpenTTDOrdersLeaves, valid_depot), offsetof(OpenTTDOrdersLeaves, depot_owner), offsetof(OpenTTDOrdersLeaves, rail_depot), offsetof(OpenTTDOrdersLeaves, road_depot), offsetof(OpenTTDOrdersLeaves, ship_depot), offsetof(OpenTTDOrdersLeaves, valid_waypoint), offsetof(OpenTTDOrdersLeaves, waypoint_facilities), offsetof(OpenTTDOrdersLeaves, waypoint_owner), offsetof(OpenTTDOrdersLeaves, list_capacity), offsetof(OpenTTDOrdersLeaves, next_backup), offsetof(OpenTTDOrdersLeaves, next_vehicle), offsetof(OpenTTDOrdersLeaves, aircraft_range), offsetof(OpenTTDOrdersLeaves, aircraft_range_square), offsetof(OpenTTDOrdersLeaves, bus), offsetof(OpenTTDOrdersLeaves, backup_capacity), offsetof(OpenTTDOrdersLeaves, create_backup), offsetof(OpenTTDOrdersLeaves, networking), offsetof(OpenTTDOrdersLeaves, network_server), offsetof(OpenTTDOrdersLeaves, network_client), offsetof(OpenTTDOrdersLeaves, server_client), offsetof(OpenTTDOrdersLeaves, default_group), offsetof(OpenTTDOrdersLeaves, vehicle_tile), offsetof(OpenTTDOrdersLeaves, vehicle_group), offsetof(OpenTTDOrdersLeaves, unique_backup_name), offsetof(OpenTTDOrdersLeaves, backup_hangar), offsetof(OpenTTDOrdersLeaves, review_setting), offsetof(OpenTTDOrdersLeaves, local_owner), offsetof(OpenTTDOrdersLeaves, day_counter), offsetof(OpenTTDOrdersLeaves, fast_aircraft), offsetof(OpenTTDOrdersLeaves, short_strip), offsetof(OpenTTDOrdersLeaves, no_jet_crash), offsetof(OpenTTDOrdersLeaves, append_station), offsetof(OpenTTDOrdersLeaves, invalidate_station_list), offsetof(OpenTTDOrdersLeaves, command_error), offsetof(OpenTTDOrdersLeaves, timetable_dirty), offsetof(OpenTTDOrdersLeaves, invalidate_order), offsetof(OpenTTDOrdersLeaves, vehicle_dirty), offsetof(OpenTTDOrdersLeaves, delete_order_news), offsetof(OpenTTDOrdersLeaves, suppress_implicit_write), offsetof(OpenTTDOrdersLeaves, invalidate_vehicle_list), offsetof(OpenTTDOrdersLeaves, close_shared_window), offsetof(OpenTTDOrdersLeaves, invalidate_shared_window), offsetof(OpenTTDOrdersLeaves, last_station_write), offsetof(OpenTTDOrdersLeaves, dirty_vehicle_windows), offsetof(OpenTTDOrdersLeaves, capture_backup_metadata), offsetof(OpenTTDOrdersLeaves, clear_backup_name), offsetof(OpenTTDOrdersLeaves, restore_backup_metadata), offsetof(OpenTTDOrdersLeaves, order_news), offsetof(OpenTTDOrdersLeaves, debug_list), offsetof(OpenTTDOrdersLeaves, assert_departure_range), offsetof(OpenTTDOrdersLeaves, delete_list), offsetof(OpenTTDOrdersLeaves, leave_station), offsetof(OpenTTDOrdersLeaves, reverse_train), offsetof(OpenTTDOrdersLeaves, next_airport), offsetof(OpenTTDOrdersLeaves, set_destination), offsetof(OpenTTDOrdersLeaves, closest_depot), offsetof(OpenTTDOrdersLeaves, share_command), offsetof(OpenTTDOrdersLeaves, group_command), offsetof(OpenTTDOrdersLeaves, delete_backup), offsetof(OpenTTDOrdersLeaves, clear_backup_gui), offsetof(OpenTTDOrdersLeaves, clear_backup_post), offsetof(OpenTTDOrdersLeaves, missing_aircraft_orders), offsetof(OpenTTDOrdersLeaves, change_timetable_command)});
	Layout(266, "OpenTTDOrdersClosest", {sizeof(OpenTTDOrdersClosest), alignof(OpenTTDOrdersClosest), offsetof(OpenTTDOrdersClosest, tile), offsetof(OpenTTDOrdersClosest, destination), offsetof(OpenTTDOrdersClosest, reverse), offsetof(OpenTTDOrdersClosest, found)});
	CHECK(openttd_rust_abi_layout(255, 0) == SIZE_MAX);
	static_assert(std::is_same_v<decltype(OpenTTDSignalContext::log_x), unsigned int>);
	CHECK(static_cast<size_t>(PTRDIFF_MAX) == (SIZE_MAX >> 1));
	std::printf("pointer_bytes %zu sentinel %zu borrow_limit %zu\n", sizeof(void *), SIZE_MAX, static_cast<size_t>(PTRDIFF_MAX));
	Layout(29, "OpenTTDListControlInput", {sizeof(OpenTTDListControlInput), alignof(OpenTTDListControlInput), offsetof(OpenTTDListControlInput, a), offsetof(OpenTTDListControlInput, b), offsetof(OpenTTDListControlInput, flag), offsetof(OpenTTDListControlInput, kind)});
	Layout(30, "OpenTTDListControlAction", {sizeof(OpenTTDListControlAction), alignof(OpenTTDListControlAction), offsetof(OpenTTDListControlAction, a), offsetof(OpenTTDListControlAction, kind)});

	Layout(100, "OpenTTDRoadYapfInput", {sizeof(OpenTTDRoadYapfInput), alignof(OpenTTDRoadYapfInput), offsetof(OpenTTDRoadYapfInput, map_x), offsetof(OpenTTDRoadYapfInput, map_y), offsetof(OpenTTDRoadYapfInput, tile), offsetof(OpenTTDRoadYapfInput, dest_tile), offsetof(OpenTTDRoadYapfInput, max_nodes), offsetof(OpenTTDRoadYapfInput, slope), offsetof(OpenTTDRoadYapfInput, crossing), offsetof(OpenTTDRoadYapfInput, stop), offsetof(OpenTTDRoadYapfInput, occupied), offsetof(OpenTTDRoadYapfInput, bay), offsetof(OpenTTDRoadYapfInput, curve), offsetof(OpenTTDRoadYapfInput, display_speed), offsetof(OpenTTDRoadYapfInput, order_destination), offsetof(OpenTTDRoadYapfInput, order_speed), offsetof(OpenTTDRoadYapfInput, order_type), offsetof(OpenTTDRoadYapfInput, bus), offsetof(OpenTTDRoadYapfInput, articulated), offsetof(OpenTTDRoadYapfInput, trackdir)});
	Layout(101, "OpenTTDRoadYapfTile", {sizeof(OpenTTDRoadYapfTile), alignof(OpenTTDRoadYapfTile), offsetof(OpenTTDRoadYapfTile, occupied), offsetof(OpenTTDRoadYapfTile, length), offsetof(OpenTTDRoadYapfTile, station), offsetof(OpenTTDRoadYapfTile, type), offsetof(OpenTTDRoadYapfTile, station_type), offsetof(OpenTTDRoadYapfTile, depot), offsetof(OpenTTDRoadYapfTile, depot_dir), offsetof(OpenTTDRoadYapfTile, crossing), offsetof(OpenTTDRoadYapfTile, waypoint), offsetof(OpenTTDRoadYapfTile, drive_through), offsetof(OpenTTDRoadYapfTile, continuation), offsetof(OpenTTDRoadYapfTile, busy_bays)});
	Layout(102, "OpenTTDRoadYapfFollow", {sizeof(OpenTTDRoadYapfFollow), alignof(OpenTTDRoadYapfFollow), offsetof(OpenTTDRoadYapfFollow, tile), offsetof(OpenTTDRoadYapfFollow, skipped), offsetof(OpenTTDRoadYapfFollow, max_speed), offsetof(OpenTTDRoadYapfFollow, min_speed), offsetof(OpenTTDRoadYapfFollow, dirs), offsetof(OpenTTDRoadYapfFollow, followed)});
	Layout(103, "OpenTTDRoadYapfArea", {sizeof(OpenTTDRoadYapfArea), alignof(OpenTTDRoadYapfArea), offsetof(OpenTTDRoadYapfArea, tile), offsetof(OpenTTDRoadYapfArea, width), offsetof(OpenTTDRoadYapfArea, height), offsetof(OpenTTDRoadYapfArea, valid), offsetof(OpenTTDRoadYapfArea, stop), offsetof(OpenTTDRoadYapfArea, drive_through), offsetof(OpenTTDRoadYapfArea, next)});
	Layout(104, "OpenTTDRoadYapfLeaves", {sizeof(OpenTTDRoadYapfLeaves), alignof(OpenTTDRoadYapfLeaves), offsetof(OpenTTDRoadYapfLeaves, tile), offsetof(OpenTTDRoadYapfLeaves, follow), offsetof(OpenTTDRoadYapfLeaves, tracks), offsetof(OpenTTDRoadYapfLeaves, height), offsetof(OpenTTDRoadYapfLeaves, closest), offsetof(OpenTTDRoadYapfLeaves, area)});
	Layout(105, "OpenTTDRoadYapfResult", {sizeof(OpenTTDRoadYapfResult), alignof(OpenTTDRoadYapfResult), offsetof(OpenTTDRoadYapfResult, tile), offsetof(OpenTTDRoadYapfResult, cost), offsetof(OpenTTDRoadYapfResult, direction), offsetof(OpenTTDRoadYapfResult, found), offsetof(OpenTTDRoadYapfResult, rounds), offsetof(OpenTTDRoadYapfResult, open), offsetof(OpenTTDRoadYapfResult, closed), offsetof(OpenTTDRoadYapfResult, calcs), offsetof(OpenTTDRoadYapfResult, distance)});

}

static void Calls()
{
	CheckTownProtocol([](bool condition) { CHECK(condition); });
	{
		static OpenTTDTownState *owner = openttd_rust_town_new();
		OpenTTDSharedServices shared{}; OpenTTDTownLeaves leaves{}; OpenTTDTownNameServices names{}; OpenTTDCommandCostServices costs{}; OpenTTDTownFoundingServices founding{};
		founding.shared = &shared; founding.growth = &leaves; founding.names = &names; founding.cost = &costs; founding.close_error = 0x87654321;
		costs.error = [](OpenTTDCommandCost *cost, uint32_t error) noexcept { *reinterpret_cast<uint32_t *>(cost) = error; };
		leaves.editor = []() noexcept { return false; };
		names.next = [](uint from) noexcept -> OpenTTDTownRef { CHECK(from == 0 || from == UINT16_MAX); return from == 0 ? OpenTTDTownRef{reinterpret_cast<Town *>(uintptr_t{1}), owner, UINT16_MAX - 1} : OpenTTDTownRef{nullptr, nullptr, UINT16_MAX}; };
		names.utf8_length = [](const uint8_t *data, size_t size) noexcept -> size_t { CHECK(size == 1 && *data == 'x'); return size; };
		founding.parameters = [](OpenTTDTownFoundingFrame *) noexcept -> const TownNameParams * { return nullptr; };
		shared.current_company = []() noexcept -> uint8_t { return UINT8_MAX; };
		founding.found_setting = []() noexcept { return TF_CUSTOM_LAYOUT; };
		founding.can_allocate = []() noexcept { return true; };
		shared.distance_from_edge = [](uint32_t tile) noexcept -> uint32_t { CHECK(tile == UINT32_MAX); return 13; };
		founding.close_to_town = [](uint32_t tile, uint distance) noexcept { CHECK(tile == UINT32_MAX && distance == 20); return true; };
		founding.resolve = [](uint16_t id) noexcept -> OpenTTDTownRef { CHECK(id == UINT16_MAX); return {nullptr, nullptr, id}; };
		uint32_t error = 0; auto *cost = reinterpret_cast<OpenTTDCommandCost *>(&error); OpenTTDTownFoundingFrame frame; const uint8_t text = 'x';
		openttd_rust_town_found(&founding, &frame, &frame.names, cost, UINT32_MAX, TSZ_SMALL, false, TL_ORIGINAL, false, 0, {&text, 1}, false);
		CHECK(error == 0x87654321); openttd_rust_town_rename(&founding, cost, UINT16_MAX, {nullptr, 0}, false);
		CHECK(error == UINT16_MAX); openttd_rust_town_destroy(owner);
	}
	std::unique_ptr<OpenTTDRoadState, decltype(&openttd_rust_road_destroy)> road(openttd_rust_road_new(), openttd_rust_road_destroy);
	for (uint8_t field = 0; field < 7; ++field) {
		CHECK(openttd_rust_road_get(road.get(), field) == 0);
		openttd_rust_road_set(road.get(), field, 65535);
		CHECK(openttd_rust_road_get(road.get(), field) == (field == 2 || field == 5 ? 65535 : 255));
	}
	CHECK(openttd_rust_road_path_size(road.get()) == 0);
	const OpenTTDRoadPathElement road_path[] = {{255, UINT32_MAX}, {9, 123456}};
	openttd_rust_road_path_replace(road.get(), road_path, 2);
	CHECK(openttd_rust_road_path_get(road.get(), 1).tile == 123456);
	openttd_rust_road_path_push(road.get(), {6, 42});
	CHECK(openttd_rust_road_path_get(road.get(), 2).trackdir == 6);
	openttd_rust_road_path_pop(road.get());
	CHECK(openttd_rust_road_path_size(road.get()) == 2);
	openttd_rust_road_path_clear(road.get());
	openttd_rust_road_path_replace(road.get(), nullptr, 0);
	CHECK(openttd_rust_road_path_size(road.get()) == 0);
	road.reset();
	const std::array<uint8_t, 8> bytes{0x10, 0x32, 0x54, 0x76, 0x98, 0xBA, 0xDC, 0xFE};
	constexpr uint64_t bits = UINT64_C(0xFEDCBA9876543210);
	auto le = openttd_rust_encode_uint_le(bits);
	CHECK(std::memcmp(le.bytes, bytes.data(), bytes.size()) == 0);
	auto number = openttd_rust_consumer_little_endian(bytes.data(), bytes.size(), 8);
	CHECK(number.value_bits == bits && number.length == 8);
	const std::string maximum = "ffffffffffffffff";
	auto integer = openttd_rust_parse_integer(reinterpret_cast<const uint8_t *>(maximum.data()), maximum.size(), 16, 64, 0, 0);
	CHECK(integer.value_bits == UINT64_MAX && integer.length == 16 && integer.error_kind == 0);
	auto formatted = openttd_rust_format_integer(UINT64_MAX, 0, 16);
	CHECK(formatted.length == maximum.size() && std::memcmp(formatted.bytes, maximum.data(), maximum.size()) == 0);
	auto empty_integer = openttd_rust_parse_integer(nullptr, 0, 10, 64, 0, 0);
	CHECK(empty_integer.length == 0 && empty_integer.error_kind == 1);
	CHECK(openttd_rust_skip_integer(nullptr, 0, 10) == 0);
	auto utf8 = openttd_rust_encode_utf8(0x1F600);
	CHECK(utf8.length == 4 && utf8.bytes[0] == 0xF0 && utf8.bytes[3] == 0x80);
	auto decoded = openttd_rust_decode_utf8(utf8.bytes, utf8.length);
	CHECK(decoded.length == 4 && decoded.codepoint == 0x1F600);
	CHECK(openttd_rust_decode_utf8(nullptr, 0).length == 0);
	CHECK(openttd_rust_utf8_next(utf8.bytes, utf8.length, 0) == 4);
	CHECK(openttd_rust_utf8_previous(utf8.bytes, utf8.length, 4) == 0);
	CHECK(openttd_rust_utf8_at_byte(utf8.bytes, utf8.length, 2) == 0);
	auto state = openttd_rust_alternating_initialize(SIZE_MAX - 1, 1);
	CHECK(state.position == SIZE_MAX - 1 && state.current_after == 1);
	auto step = openttd_rust_alternating_advance(state, SIZE_MAX);
	CHECK(step.state.position == SIZE_MAX && step.movement == 0 && step.state.current_after == 1);
	CHECK(openttd_rust_alternating_compare(state, step.state) == -1);
	auto bound = openttd_rust_consumer_bound(31, 7, SIZE_MAX);
	CHECK(bound.length == 24 && bound.position == 31 && bound.shortfall == 0);
	auto match = openttd_rust_consumer_prefix(nullptr, 0, nullptr, 0);
	CHECK(match.length == 0 && match.matched == 1);
	CHECK(openttd_rust_consumer_find(bytes.data(), bytes.size(), utf8.bytes, 1, 0) == SIZE_MAX);
	auto trimmed = openttd_rust_bytes_trim(nullptr, 0, nullptr, 0);
	CHECK(trimmed.offset == 0 && trimmed.length == 0);
	CHECK(openttd_rust_bytes_case(nullptr, 0, nullptr, 0, 0, std::is_signed_v<char>, 0) == 0);
	auto spiral = openttd_rust_spiral_square(10, 10, 3, 32, 32);
	CHECK(openttd_rust_spiral_equal(spiral, spiral) == 1 && openttd_rust_spiral_end(spiral) == 0);
	spiral = openttd_rust_spiral_advance(spiral, 32, 32);
	CHECK(openttd_rust_spiral_end(spiral) == 0);
	CHECK(openttd_rust_get_partial_pixel_z(3, 12, 0) == 0);
	CHECK(openttd_rust_get_partial_pixel_z(-2, 0, 0x03) == UINT32_MAX);
	CHECK(openttd_rust_get_partial_pixel_z(0, 0, 0x10) == UINT64_MAX);
	std::printf("real_calls high_bits=%llx passed\n", static_cast<unsigned long long>(number.value_bits));
}

using EncodedOwner = std::unique_ptr<OpenTTDRustEncodedResult, decltype(&openttd_rust_encoded_destroy)>;

static void Encoded()
{
	const std::array<uint8_t, 3> bytes{'A', 0, 'B'};
	const std::array<OpenTTDRustEncodedParameter, 3> parameters{{{UINT64_MAX, nullptr, 0, 1}, {0, bytes.data(), bytes.size(), 2}, {0, nullptr, 0, 0}}};
	std::printf("descriptor_array_alignment %zu address_mod %zu\n", alignof(OpenTTDRustEncodedParameter), reinterpret_cast<uintptr_t>(parameters.data()) % alignof(OpenTTDRustEncodedParameter));
	EncodedOwner owner(openttd_rust_encoded_serialize(42, parameters.data(), parameters.size(), 0), openttd_rust_encoded_destroy);
	auto view = openttd_rust_encoded_view(owner.get());
	std::string expected = "\xEE\x80\x81" "2a\x1E\xEE\x80\x82" "ffffffffffffffff\x1E\xEE\x80\x83";
	expected.append("A\0B", 3);
	expected.push_back('\x1E');
	CHECK(view.length == expected.size() && std::memcmp(view.bytes, expected.data(), view.length) == 0);
	std::string copied(reinterpret_cast<const char *>(view.bytes), view.length);
	owner.reset();
	CHECK(copied == expected);
	/* Byte storage is deliberately not aligned as a Rust descriptor. The boundary
	 * copies initialized field representations without any typed C++-storage borrow. */
	alignas(8) std::array<uint8_t, sizeof(parameters) + 8> storage{};
	std::memcpy(storage.data() + 1, parameters.data(), sizeof(parameters));
	auto raw = reinterpret_cast<const OpenTTDRustEncodedParameter *>(storage.data() + 1);
	CHECK(reinterpret_cast<uintptr_t>(raw) % alignof(OpenTTDRustEncodedParameter) != 0);
	owner.reset(openttd_rust_encoded_serialize(42, raw, parameters.size(), 0));
	view = openttd_rust_encoded_view(owner.get());
	CHECK(view.length == expected.size() && std::memcmp(view.bytes, expected.data(), view.length) == 0);
	owner.reset(openttd_rust_encoded_serialize(42, nullptr, 0, 0));
	view = openttd_rust_encoded_view(owner.get());
	CHECK(view.length == 5);
	owner.reset(openttd_rust_encoded_replace(reinterpret_cast<const uint8_t *>(expected.data()), expected.size(), 0, parameters[0], 0, 0));
	view = openttd_rust_encoded_view(owner.get());
	/* Original replacement does not parse a final empty record after its last separator. */
	CHECK(view.length == expected.size() - 1 && std::memcmp(view.bytes, expected.data(), view.length) == 0);
	const std::string invalid = "\xEE\x80\x80" "0\x1E\xEE\x80\x82" "10000000000000000";
	owner.reset(openttd_rust_encoded_negatives(reinterpret_cast<const uint8_t *>(invalid.data()), invalid.size()));
	view = openttd_rust_encoded_view(owner.get());
	CHECK(view.diagnostic_count == 1);
	auto diagnostic = openttd_rust_encoded_diagnostic(owner.get(), 0);
	CHECK(diagnostic.kind == 2 && diagnostic.offset == 8 && diagnostic.length == 17 && diagnostic.tail_length == 0);
	openttd_rust_encoded_destroy(nullptr);
	std::printf("encoded owners, by-value replacement/view/diagnostic and raw unaligned array passed\n");
}

static void Locale()
{
	CHECK(std::setlocale(LC_CTYPE, "C") != nullptr);
#ifdef _WIN32
	int previous = _configthreadlocale(_ENABLE_PER_THREAD_LOCALE);
	CHECK(previous != -1);
	CHECK(std::setlocale(LC_CTYPE, ".1252") != nullptr);
#endif
	uint8_t byte = 0xC9;
	uint8_t expected = static_cast<uint8_t>(std::tolower(static_cast<unsigned char>(byte)));
#ifdef _WIN32
	CHECK(expected == 0xE9); // A real non-C mapping is required to distinguish locale state.
#endif
	openttd_rust_bytes_lower(&byte, 1, 0);
	CHECK(byte == expected);
	std::printf("shared_locale %s lower_C9 %u\n", std::setlocale(LC_CTYPE, nullptr), static_cast<unsigned>(byte));
	CHECK(std::setlocale(LC_CTYPE, "C") != nullptr);
#ifdef _WIN32
	CHECK(_configthreadlocale(previous) != -1);
#endif
}

using HistoryOwner = std::unique_ptr<OpenTTDRustHistoryEngine, decltype(&openttd_rust_history_destroy)>;

static void HistoryDescribe(OpenTTDRustHistoryEngine *owner, const HistoryRange &range)
{
	auto step = openttd_rust_history_next(owner);
	CHECK(step.kind == 1 && step.token == reinterpret_cast<uintptr_t>(&range));
	openttd_rust_history_describe(owner, {reinterpret_cast<uintptr_t>(range.hr), range.periods, range.records, range.first, range.last, range.division, range.total_division, range.hr != nullptr ? range.hr->periods : uint8_t{0}, range.hr != nullptr ? range.hr->division : uint8_t{0}});
}

static void History()
{
	const HistoryRange leaf{2};
	const HistoryRange parent{leaf, 1, 2};
	constexpr uint64_t bits = UINT64_C(0xFEDCBA9876543210);
	HistoryOwner owner(openttd_rust_history_create(0, reinterpret_cast<uintptr_t>(&leaf), bits, 0, 0), openttd_rust_history_destroy);
	HistoryDescribe(owner.get(), leaf);
	auto step = openttd_rust_history_next(owner.get());
	CHECK(step.kind == 0 && step.value == (bits | 2));
	owner.reset(openttd_rust_history_create(1, reinterpret_cast<uintptr_t>(&leaf), bits | 2, 0, 0));
	HistoryDescribe(owner.get(), leaf);
	step = openttd_rust_history_next(owner.get());
	CHECK(step.kind == 0 && step.value == 1);
	owner.reset(openttd_rust_history_create(2, reinterpret_cast<uintptr_t>(&leaf), bits | 2, 0, 1));
	HistoryDescribe(owner.get(), leaf);
	step = openttd_rust_history_next(owner.get());
	CHECK(step.kind == 2 && step.first == 1 && step.last == 3);
	step = openttd_rust_history_next(owner.get());
	CHECK(step.kind == 3 && step.first == 0 && step.target == 1);
	step = openttd_rust_history_next(owner.get());
	CHECK(step.kind == 4 && step.target == 0);
	CHECK(openttd_rust_history_next(owner.get()).kind == 0);
	owner.reset(openttd_rust_history_create(3, reinterpret_cast<uintptr_t>(&parent), bits | 2, 0, 0));
	HistoryDescribe(owner.get(), parent);
	step = openttd_rust_history_next(owner.get());
	CHECK(step.kind == 6 && step.count == 1);
	openttd_rust_history_phase(owner.get(), 0);
	step = openttd_rust_history_next(owner.get());
	CHECK(step.kind == 7 && step.target == 0);
	HistoryDescribe(owner.get(), leaf);
	step = openttd_rust_history_next(owner.get());
	CHECK(step.kind == 9 && step.first == 1);
	CHECK(openttd_rust_history_complete(owner.get()) == 1);
	step = openttd_rust_history_next(owner.get());
	CHECK(step.kind == 8 && step.count == 1);
	CHECK(openttd_rust_history_complete(owner.get()) == 1);
	step = openttd_rust_history_next(owner.get());
	CHECK(step.kind == 0 && step.value == 1);
	std::printf("history descriptor identities, high-bit mask and staged by-value calls passed\n");
}

static void Math()
{
	/* Full uint32 input/output, including the rounded result beyond uint16. */
	CHECK(openttd_rust_int_sqrt(0) == 0);
	CHECK(openttd_rust_int_sqrt(6) == 2);
	CHECK(openttd_rust_int_sqrt(7) == 3);
	CHECK(openttd_rust_int_sqrt(UINT32_C(0x80000000)) == 46341);
	CHECK(openttd_rust_int_sqrt(UINT32_C(4294836225)) == 65535);
	CHECK(openttd_rust_int_sqrt(UINT32_MAX) == 65536);
	constexpr uint64_t bits = UINT64_C(0xFEDCBA9876543210);
	constexpr uint64_t high = UINT64_C(0x8000000000000000);
	/* The narrow signed reversed intervals intentionally retain C++ promotions. */
	CHECK(openttd_rust_soft_clamp(bits, high, UINT64_MAX, 64, 0) == bits);
	CHECK(openttd_rust_soft_clamp(0, high, UINT64_MAX, 64, 0) == high);
	CHECK(openttd_rust_soft_clamp(UINT64_MAX, 0, bits, 64, 0) == bits);
	CHECK(openttd_rust_soft_clamp(0, UINT64_MAX, 0, 64, 0) == high);
	CHECK(openttd_rust_soft_clamp(0, UINT64_MAX, UINT64_MAX - 2, 8, 1) == 126);
	CHECK(openttd_rust_soft_clamp(0, UINT64_MAX, UINT64_MAX - 2, 16, 1) == 32766);
	CHECK(openttd_rust_soft_clamp(0, 1500000000, static_cast<uint64_t>(-1500000000), 32, 1) == 0);
	CHECK(openttd_rust_soft_clamp(high, high, INT64_MAX, 64, 1) == high);
	std::printf("math int_sqrt and soft_clamp scalar high-bit, width and boundary calls passed\n");
}

static void CryptoPrimitives()
{
	/* Actual public facades invoke all 15 Rust functions with native descriptors.
	 * Counter high bits, initialized-only Poly1305 storage, callback convention,
	 * in-place failure/retry and rekey are observed on the current native target. */
	std::array<uint8_t, 32> key{}, h{};
	std::array<uint8_t, 24> nonce{};
	std::array<uint8_t, 65> cipher{}, plain{}, decoded{};
	for (size_t i = 0; i < key.size(); ++i) key[i] = static_cast<uint8_t>(i * 13 + 9);
	for (size_t i = 0; i < nonce.size(); ++i) nonce[i] = static_cast<uint8_t>(i * 7 + 101);
	nonce[0] = 0x12; nonce[1] = 0x34; nonce[2] = 0x56; nonce[3] = 0xF8;
	crypto_chacha20_h(h.data(), key.data(), nonce.data());
	CHECK(crypto_chacha20_djb(cipher.data(), nullptr, 65, key.data(), nonce.data(), UINT64_MAX) == 1);
	CHECK(crypto_chacha20_ietf(cipher.data(), nullptr, 65, key.data(), nonce.data(), UINT32_MAX) == 1);
	CHECK(crypto_chacha20_x(cipher.data(), nullptr, 65, key.data(), nonce.data(), UINT64_MAX) == 1);
	plain.fill(0x42);
	CHECK(crypto_chacha20_djb(cipher.data(), plain.data(), plain.size(), key.data(), nonce.data(), UINT64_C(0xFEDCBA9876543210)) == UINT64_C(0xFEDCBA9876543212));
	crypto_chacha20_djb(decoded.data(), cipher.data(), cipher.size(), key.data(), nonce.data(), UINT64_C(0xFEDCBA9876543210));
	CHECK(decoded == plain);
	crypto_poly1305_ctx poly;
	std::fill_n(reinterpret_cast<uint8_t *>(&poly), sizeof(poly), 0xA5);
	crypto_poly1305_init(&poly, key.data());
	CHECK(std::all_of(std::begin(poly.c), std::end(poly.c), [](uint8_t c) { return c == 0xA5; }));
	crypto_poly1305_update(&poly, nullptr, 0);
	crypto_poly1305_update(&poly, plain.data(), 15);
	crypto_poly1305_update(&poly, plain.data() + 15, plain.size() - 15);
	std::array<uint8_t, 16> mac{}, split{};
	crypto_poly1305_final(&poly, split.data());
	CHECK(std::all_of(reinterpret_cast<const uint8_t *>(&poly), reinterpret_cast<const uint8_t *>(&poly) + sizeof(poly), [](uint8_t c) { return c == 0; }));
	crypto_poly1305(mac.data(), plain.data(), plain.size(), key.data());
	CHECK(mac == split);
	for (unsigned variant = 0; variant != 3; ++variant) {
		crypto_aead_ctx send, receive;
		if (variant == 0) { crypto_aead_init_x(&send, key.data(), nonce.data()); crypto_aead_init_x(&receive, key.data(), nonce.data()); }
		if (variant == 1) { crypto_aead_init_djb(&send, key.data(), nonce.data()); crypto_aead_init_djb(&receive, key.data(), nonce.data()); }
		if (variant == 2) { crypto_aead_init_ietf(&send, key.data(), nonce.data()); crypto_aead_init_ietf(&receive, key.data(), nonce.data()); }
		auto counter = send.counter;
		CHECK(counter == (variant == 2 ? UINT64_C(0xF856341200000000) : uint64_t{0}));
		for (unsigned chunk = 0; chunk != 2; ++chunk) {
			crypto_aead_write(&send, cipher.data(), mac.data(), nonce.data(), 17, plain.data(), plain.size());
			CHECK(send.counter == counter);
			auto bad = mac; bad[0] ^= 1;
			decoded.fill(0xA5);
			auto before = receive;
			CHECK(crypto_aead_read(&receive, decoded.data(), bad.data(), nonce.data(), 17, cipher.data(), cipher.size()) == -1);
			CHECK(std::all_of(decoded.begin(), decoded.end(), [](uint8_t c) { return c == 0xA5; }));
			CHECK(receive.counter == before.counter && std::equal(std::begin(receive.key), std::end(receive.key), std::begin(before.key)));
			CHECK(crypto_aead_read(&receive, cipher.data(), mac.data(), nonce.data(), 17, cipher.data(), cipher.size()) == 0);
			CHECK(cipher == plain && receive.counter == counter && std::equal(std::begin(send.key), std::end(send.key), std::begin(receive.key)));
		}
	}
	crypto_aead_lock(cipher.data(), mac.data(), key.data(), nonce.data(), nonce.data(), 17, plain.data(), plain.size());
	CHECK(crypto_aead_unlock(decoded.data(), mac.data(), key.data(), nonce.data(), nonce.data(), 17, cipher.data(), cipher.size()) == 0 && decoded == plain);
	std::printf("crypto 15 Rust algorithms, cdecl leaves, caller layouts, counter high bits, partial init, padding and failed retry passed; poly_size=%zu poly_align=%zu aead_size=%zu aead_align=%zu\n", sizeof(poly), alignof(crypto_poly1305_ctx), sizeof(crypto_aead_ctx), alignof(crypto_aead_ctx));
}

static void Blake2b()
{
	std::array<uint8_t, 129> message;
	std::array<uint8_t, 128> key;
	for (size_t i = 0; i < message.size(); ++i) message[i] = static_cast<uint8_t>(i * 37 + 11);
	for (size_t i = 0; i < key.size(); ++i) key[i] = static_cast<uint8_t>(i * 13 + 7);
	std::array<uint8_t, 64> expected, actual;
	crypto_blake2b_keyed(expected.data(), 32, key.data(), 65, message.data(), message.size());
	crypto_blake2b_ctx ctx;
	std::fill_n(reinterpret_cast<uint8_t *>(&ctx), sizeof(ctx), 0xA5);
	crypto_blake2b_keyed_init(&ctx, 32, key.data(), 65);
	CHECK(ctx.input_idx == 128 && ctx.hash_size == 32 && ctx.input_offset[0] == 0 && ctx.input_offset[1] == 0);
	crypto_blake2b_update(&ctx, nullptr, 0);
	crypto_blake2b_update(&ctx, message.data(), 127);
	auto copy = ctx;
	crypto_blake2b_update(&ctx, message.data() + 127, 2);
	crypto_blake2b_final(&ctx, actual.data());
	CHECK(std::equal(actual.begin(), actual.begin() + 32, expected.begin()));
	CHECK(std::all_of(reinterpret_cast<const uint8_t *>(&ctx), reinterpret_cast<const uint8_t *>(&ctx) + sizeof(ctx), [](uint8_t x) { return x == 0; }));
	crypto_blake2b_update(&copy, message.data() + 127, 1);
	crypto_blake2b_final(&copy, actual.data());
	crypto_blake2b_keyed(expected.data(), 32, key.data(), 65, message.data(), 128);
	CHECK(std::equal(actual.begin(), actual.begin() + 32, expected.begin()));
	crypto_blake2b(expected.data(), 64, message.data(), message.size());
	crypto_blake2b_init(&ctx, 64);
	crypto_blake2b_update(&ctx, message.data(), 128);
	CHECK(ctx.input_idx == 128 && ctx.input_offset[0] == 0);
	crypto_blake2b_update(&ctx, message.data() + 128, 1);
	crypto_blake2b_final(&ctx, actual.data()); CHECK(actual == expected);
	crypto_blake2b_init(&ctx, 64); crypto_blake2b_update(&ctx, message.data(), 128);
	ctx.input_offset[0] = UINT64_MAX - 127; ctx.input_offset[1] = UINT64_C(0x1122334455667788);
	crypto_blake2b_update(&ctx, message.data() + 128, 1);
	CHECK(ctx.input_offset[0] == 0 && ctx.input_offset[1] == UINT64_C(0x1122334455667789) && ctx.input_idx == 1);
	crypto_blake2b_final(&ctx, actual.data());
	crypto_blake2b_init(&ctx, 0); crypto_blake2b_final(&ctx, nullptr);
	CHECK(std::all_of(reinterpret_cast<const uint8_t *>(&ctx), reinterpret_cast<const uint8_t *>(&ctx) + sizeof(ctx), [](uint8_t x) { return x == 0; }));
	std::array<uint8_t, 72> sentinel; sentinel.fill(0xA5);
	crypto_blake2b_keyed(sentinel.data(), 65, key.data(), 128, message.data(), message.size());
	CHECK(std::all_of(sentinel.begin() + 64, sentinel.end(), [](uint8_t x) { return x == 0xA5; }));
	std::printf("blake2b six facades, context size %zu align %zu offsets %zu %zu %zu %zu %zu; copy, carry, pending block, wipe and source sizes passed\n", sizeof(ctx), alignof(crypto_blake2b_ctx), offsetof(crypto_blake2b_ctx, hash), offsetof(crypto_blake2b_ctx, input_offset), offsetof(crypto_blake2b_ctx, input), offsetof(crypto_blake2b_ctx, input_idx), offsetof(crypto_blake2b_ctx, hash_size));
}

static size_t x25519_wipes, x25519_last_wipe;
static unsigned x25519_verifies;
static bool x25519_wipes_zero;
static void OPENTTD_CRYPTO_CALL X25519Wipe(void *data, size_t size) noexcept
{
	crypto_wipe(data, size);
	x25519_wipes++; x25519_last_wipe = size;
	const auto *bytes = static_cast<const uint8_t *>(data);
	x25519_wipes_zero &= std::all_of(bytes, bytes + size, [](uint8_t x) { return x == 0; });
}
static int32_t OPENTTD_CRYPTO_CALL X25519Verify32(const uint8_t *a, const uint8_t *b) noexcept
{
	x25519_verifies++;
	return crypto_verify32(a, b);
}
static void X25519()
{
	const OpenTTDX25519Leaves leaves{X25519Wipe, X25519Verify32};
	std::array<uint8_t, 32> secret, point{}, expected, actual;
	for (size_t i = 0; i < secret.size(); ++i) secret[i] = static_cast<uint8_t>(i * 37 + 13);
	point[0] = 9;
	crypto_x25519_public_key(expected.data(), secret.data());
	x25519_wipes = x25519_verifies = 0; x25519_wipes_zero = true;
	openttd_rust_x25519_public_key(&leaves, actual.data(), secret.data());
	CHECK(actual == expected && x25519_verifies == 4 && x25519_wipes == 29 && x25519_last_wipe == 32 && x25519_wipes_zero);
	x25519_wipes = x25519_verifies = 0;
	openttd_rust_x25519(&leaves, actual.data(), secret.data(), point.data());
	CHECK(actual == expected && x25519_verifies == 4 && x25519_wipes == 29 && x25519_last_wipe == 32 && x25519_wipes_zero);
	std::array<uint8_t, 64> storage;
	storage.fill(13); storage[33] = 0xA5;
	openttd_rust_x25519_trim(storage.data() + 1, storage.data());
	CHECK(storage[1] == 8 && storage[32] == 0x4D && storage[33] == 0xA5);
	CHECK(std::all_of(storage.begin() + 2, storage.begin() + 32, [](uint8_t x) { return x == 13; }));
	secret.fill(0); secret[31] = 0x80;
	std::array<uint8_t, 32> short_result, full_result;
	x25519_wipes = x25519_verifies = 0;
	openttd_rust_x25519_ladder(&leaves, short_result.data(), secret.data(), point.data(), 255);
	CHECK(x25519_verifies == 4 && x25519_wipes == 28 && x25519_last_wipe == 40 && x25519_wipes_zero);
	openttd_rust_x25519_ladder(&leaves, full_result.data(), secret.data(), point.data(), 256);
	CHECK(short_result != full_result && x25519_verifies == 8 && x25519_wipes == 56 && x25519_last_wipe == 40 && x25519_wipes_zero);
	std::printf("x25519 four exports, cdecl wipe/verify32, physical wipe counts/final sizes, forward trim and 255/256 high-bit distinction passed\n");
}

static void StationCargo()
{
	auto list = std::unique_ptr<OpenTTDScriptList, decltype(&openttd_rust_list_destroy)>(openttd_rust_list_new(), openttd_rust_list_destroy);
	OpenTTDCargoCollector state;
	CHECK(openttd_rust_cargo_plan(0, 1) == 1 && openttd_rust_cargo_plan(1, 3) == 3);
	CHECK(openttd_rust_cargo_plan(9, 0) == UINT8_MAX);
	openttd_rust_cargo_init(&state, 0, UINT16_MAX);
	openttd_rust_cargo_packet(&state, list.get(), UINT16_MAX, 2, UINT32_MAX);
	openttd_rust_cargo_packet(&state, list.get(), UINT16_MAX, 2, 2);
	openttd_rust_cargo_packet(&state, list.get(), 3, 2, 7);
	int64_t value = 0;
	CHECK(openttd_rust_list_get(list.get(), UINT16_MAX, &value) == 1 && value == 1);
	openttd_rust_cargo_finish(&state, list.get());
	CHECK(openttd_rust_list_get(list.get(), 3, &value) == 1 && value == 7);
	auto token = openttd_rust_list_token(list.get());
	openttd_rust_cargo_finish(&state, list.get());
	CHECK(openttd_rust_list_token(list.get()) == token);
	openttd_rust_cargo_init(&state, 2, 0);
	openttd_rust_cargo_origin(&state, 10);
	openttd_rust_cargo_share(&state, list.get(), 4, UINT32_MAX);
	openttd_rust_cargo_origin(&state, 11);
	openttd_rust_cargo_share(&state, list.get(), 4, 1);
	openttd_rust_cargo_finish(&state, list.get());
	CHECK(openttd_rust_list_get(list.get(), 4, &value) == 0);
	std::printf("station_cargo scalar layout, uint32 wrapping, origin reset and finalization passed\n");
}

static void PacketState()
{
	static_assert(sizeof(intptr_t) == sizeof(size_t));
	OpenTTDPacketState state;
	openttd_rust_packet_init(&state, SIZE_MAX);
	CHECK(state.limit == SIZE_MAX && state.position == 0);
	CHECK(openttd_rust_packet_can_write(&state, SIZE_MAX, 1) == 1);
	CHECK(openttd_rust_packet_boolean(128) == 1 && openttd_rust_packet_boolean(0) == 0);
	CHECK(openttd_rust_packet_buffer_size(SIZE_MAX) == 1 && openttd_rust_packet_prefix(65537) == 1);
	std::array<uint8_t, 8> bytes{8, 0, 1, 2, 3, 4, 5, 6};
	CHECK(openttd_rust_packet_parse_size(&state, bytes.data(), bytes.size()) == 8);
	openttd_rust_packet_read_start(&state);
	CHECK(openttd_rust_packet_has_size(&state) == 1 && openttd_rust_packet_can_read(&state, 8, 6) == 1);
	CHECK(openttd_rust_packet_recv(&state, bytes.data(), bytes.size(), 2) == 0x0201 && state.position == 4);
	uint16_t count = 1;
	uint8_t byte = 0;
	CHECK(openttd_rust_packet_buffer_next(&state, bytes.data(), bytes.size(), &count, &byte) == 1 && byte == 3 && state.position == 5);
	CHECK(openttd_rust_packet_buffer_next(&state, bytes.data(), bytes.size(), &count, &byte) == 0 && count == UINT16_MAX);
	CHECK(openttd_rust_packet_remaining(&state, 8) == 3 && openttd_rust_packet_transfer_amount(&state, 8, 2) == 2);
	openttd_rust_packet_transfer_commit(&state, -1);
	CHECK(state.position == 5);
	openttd_rust_packet_transfer_commit(&state, 65537);
	CHECK(state.position == 6);
	openttd_rust_packet_skip_mac(&state, 65537);
	CHECK(state.position == 7);
	OpenTTDPacketFrame frame;
	CHECK(openttd_rust_packet_frame(2, 8, 2, &frame) == 1 && frame.message == 4 && frame.payload == 4);
	CHECK(openttd_rust_packet_frame(2, 4, 2, &frame) == 0);
	CHECK(openttd_rust_packet_send_amount(&state, SIZE_MAX - 2, 4) == 2);
	openttd_rust_packet_write_header(bytes.data(), bytes.size());
	CHECK(bytes[0] == 8 && bytes[1] == 0);
	openttd_rust_packet_send_reset(&state);
	CHECK(state.position == 0);
	std::printf("packet scalar framing, binary reads and native cursor transitions passed\n");
}

static void StringValidation()
{
	static_assert(sizeof(char32_t) == sizeof(uint32_t));
	std::array<uint8_t, 4> input{13,10,0,0xFF};
	auto step = openttd_rust_validation_step(input.data(), input.size(), 15);
	CHECK(step.consumed == 1 && step.output.length == 0 && step.stopped == 0);
	step = openttd_rust_validation_step(input.data() + 1, input.size() - 1, 15);
	CHECK(step.consumed == 1 && step.output.length == 1 && step.output.bytes[0] == 10);
	step = openttd_rust_validation_step(input.data() + 2, 2, 15);
	CHECK(step.consumed == 1 && step.stopped == 1 && step.output.length == 0);
	step = openttd_rust_validation_step(input.data() + 3, 1, 255);
	CHECK(step.consumed == 1 && step.stopped == 0 && step.output.length == 0);
	std::array<uint8_t, 4> surrogate{0xED,0xA0,0x80,0};
	CHECK(openttd_rust_validation_valid(surrogate.data(), surrogate.size()) == 1);
	CHECK(openttd_rust_validation_valid(surrogate.data(), surrogate.size() - 1) == 0);
	std::array<uint8_t, 8> bytes{'a','b','c','d','e','f','g','h'};
	auto write = openttd_rust_inplace_write(bytes.data(), 0, 6, bytes.data() + 2, 4);
	CHECK(write.accepted == 1 && write.position == 4 && bytes[0] == 'c' && bytes[3] == 'f' && bytes[4] == 'e');
	write = openttd_rust_inplace_write(bytes.data(), 4, 4, input.data(), 1);
	CHECK(write.accepted == 0 && write.position == 4 && bytes[4] == 'e');
	write = openttd_rust_inplace_write(bytes.data(), 4, 1, input.data(), 1);
	CHECK(write.accepted == 1 && write.position == 5 && bytes[4] == 13);
	std::printf("string_validation historical policy, NUL, surrogate and live in-place copy passed\n");
}

static void ScriptListControl()
{
	auto *list = openttd_rust_list_new();
	auto *control = openttd_rust_list_control_new(5);
	OpenTTDListControlInput input{};
	OpenTTDListControlAction action{};
	auto step = [&] { openttd_rust_list_control_step(control, list, &input, &action); input = {}; };
	step(); CHECK(action.kind == LC_TYPE && action.a == 2);
	input.kind = 0x05000002; step(); CHECK(action.kind == LC_GET_INT);
	input.a = INT64_MIN; step(); CHECK(action.kind == LC_TYPE && action.a == 3);
	input.kind = 0x01000008; step(); CHECK(action.kind == LC_GET_BOOL);
	input.flag = UINT64_MAX; step(); CHECK(action.kind == LC_RETURN && action.a == 0);
	int64_t value;
	CHECK(openttd_rust_list_get(list, INT64_MIN, &value) == 1 && value == 1);
	openttd_rust_list_control_destroy(control);
	control = openttd_rust_list_control_new(4);
	step(); CHECK(action.kind == LC_TYPE);
	input.kind = 0x05000002; step(); CHECK(action.kind == LC_GET_INT);
	input.a = INT64_MIN; step(); CHECK(action.kind == LC_PUSH_INT && action.a == 1);
	step(); CHECK(action.kind == LC_RETURN && action.a == 1);
	openttd_rust_list_control_destroy(control);
	control = openttd_rust_list_control_new(1);
	step(); CHECK(action.kind == LC_TOP);
	input.a = 1; step(); CHECK(action.kind == LC_DISABLE);
	step(); CHECK(action.kind == LC_ITEM);
	input.flag = 1; step(); CHECK(action.kind == LC_ITEM);
	input.flag = 2; step(); CHECK(action.kind == LC_INDEX);
	input.a = INT64_MAX; step(); CHECK(action.kind == LC_ITEM);
	step(); CHECK(action.kind == LC_RETURN && action.a == 0);
	CHECK(openttd_rust_list_get(list, INT64_MAX, &value) == 1 && value == 0);
	openttd_rust_list_control_destroy(control);
	control = openttd_rust_list_control_new(3);
	step(); CHECK(action.kind == LC_TYPE && action.a == -1);
	input.kind = 0x01000001; step(); CHECK(action.kind == LC_RETURN && action.a == 0);
	openttd_rust_list_control_destroy(control);
	openttd_rust_list_destroy(list);
	std::printf("script_list control high-bit bool/key, typed filtering and load rejection passed\n");
}

static uint8_t OPENTTD_LINKGRAPH_CALL LinkGraphAbort(const void *context) noexcept
{
	auto *state = static_cast<const std::array<uint32_t, 2> *>(context);
	return (*state)[0];
}

static void LinkGraphJob()
{
	std::array<OpenTTDLinkGraphNode, 2> nodes{{{10, 0, 12, 1, 1, 0, 1}, {0, 1, 24, 4, 1, 1, 0}}};
	OpenTTDLinkGraphEdge edge{20, 0, 1};
	OpenTTDLinkGraphSettings settings{2, 100, 100, 80, 1, 1, 127, 127, 30};
	std::array<uint32_t, 2> context{0, 0};
	auto *owner = openttd_rust_linkgraph_run(nodes.data(), nodes.size(), &edge, 1, &settings, &context, LinkGraphAbort);
	size_t count = 0;
	const auto *shares = openttd_rust_linkgraph_shares(owner, &count);
	CHECK(count == 2 && shares[0].node == 0 && shares[1].node == 1);
	CHECK(shares[0].origin == 12 && shares[0].via == 24 && shares[0].cumulative == 10 && shares[0].unrestricted == 10);
	CHECK(shares[1].origin == 12 && shares[1].via == 24 && shares[1].cumulative == 10 && shares[1].unrestricted == 10);
	CHECK(openttd_rust_linkgraph_edges(owner)[0] == 10);
	openttd_rust_linkgraph_destroy(owner);
	context[0] = 1;
	owner = openttd_rust_linkgraph_run(nodes.data(), nodes.size(), &edge, 1, &settings, &context, LinkGraphAbort);
	openttd_rust_linkgraph_shares(owner, &count);
	CHECK(count == 0 && openttd_rust_linkgraph_edges(owner)[0] == 0);
	openttd_rust_linkgraph_destroy(owner);
	std::printf("linkgraph owned snapshot/result and cdecl abort callback passed\n");
}

// ABI/reentry probe, not simulation evidence. The full game harness compares map/DATE.
struct TreeProbe {
	OpenTTDTreePlantObservation plant{.tile_type = 7, .bridge = false, .ground = 0, .density = 0, .snow = false, .coast = false, .zone = 0};
	OpenTTDTreeObservation tree{.ground = 3, .density = 3, .species = 20, .count = 2, .growth = 3, .zone = 0};
	uint32_t writes = 0, waters = 0, clears = 0;
	int64_t nested_cost = 0;
	bool abort = false;
};
static TreeProbe tree_probe;
template <typename Result, typename... Args>
static Result TreeUnused(Args...) noexcept
{
	if constexpr (!std::is_void_v<Result>) return {};
}
static uint32_t TreeRandom(void *) noexcept { return UINT32_MAX; }
static const OpenTTDTreeServices tree_probe_services{
	.plant_observation = [](uint32_t) noexcept { return tree_probe.plant; },
	.tree_observation = [](uint32_t) noexcept { return tree_probe.tree; },
	.tree_count = [](uint32_t) noexcept { return uint32_t{tree_probe.tree.count}; },
	.clear_observation = [](uint32_t) noexcept { return OpenTTDTreeClearObservation{.count = tree_probe.tree.count, .species = tree_probe.tree.species}; },
	.snow_line = TreeUnused<uint8_t>,
	.sin = [](float value) noexcept { return value; },
	.cos = [](float value) noexcept { return value; },
	.make_tree = [](uint32_t, uint8_t type, uint32_t count, uint8_t growth, uint8_t ground, uint32_t density) noexcept {
		++tree_probe.writes;
		CHECK(type == 12 && count == 2 && growth == 6 && ground == 0 && density == 2);
	},
	.set_ground_density = TreeUnused<void, uint32_t, uint8_t, uint32_t>,
	.add_count = TreeUnused<void, uint32_t, int32_t>,
	.add_growth = TreeUnused<void, uint32_t, int32_t>,
	.set_growth = TreeUnused<void, uint32_t, uint8_t>,
	.make_clear = TreeUnused<void, uint32_t, uint8_t, uint32_t>,
	.make_shore = TreeUnused<void, uint32_t>,
	.make_snow = TreeUnused<void, uint32_t, uint32_t>,
	.clear_neighbour_flooding = TreeUnused<void, uint32_t>,
	.tile_loop_water = [](uint32_t tile) noexcept {
		/* Flooding reenters tree code through a nested clear of a neighbouring tile. */
		++tree_probe.waters;
		tree_probe.nested_cost = openttd_rust_trees_clear_tile(nullptr, tile + 1, 5, true, false, &tree_probe_services);
	},
	.ambient = TreeUnused<bool, uint32_t>,
	.play_sound = TreeUnused<void, uint32_t, uint16_t>,
	.progress = []() noexcept { return tree_probe.abort; },
	.progress_total = TreeUnused<bool, uint32_t>,
	.clear_square = [](uint32_t) noexcept { ++tree_probe.clears; },
	.town_rating = TreeUnused<void, OpenTTDTreeCommand *, uint32_t, bool>,
	.command_begin = [](OpenTTDTreeCommand *, uint32_t, uint32_t, bool) noexcept { return INT32_MAX; },
	.command_tile = [](OpenTTDTreeCommand *) noexcept { return uint32_t{1}; },
	.command_next = [](OpenTTDTreeCommand *) noexcept { return UINT32_MAX; },
	.command_debit = TreeUnused<void, OpenTTDTreeCommand *>,
	.landscape_clear = TreeUnused<OpenTTDTreeLandscapeClear, OpenTTDTreeCommand *, uint32_t>,
};
static void Trees()
{
	const OpenTTDSharedServices shared = OpenTTDFixtureSharedServices(nullptr, TreeRandom, EffectTestIndustry);
	uint8_t *counter = openttd_rust_tree_counter();
	openttd_rust_trees_initialize();
	CHECK(*counter == 0);
	*counter = 37; // Same boundary write made by DATE/TTD descriptors.
	openttd_rust_trees_tick({.tick_counter = 0, .size = 4096, .climate = 0, .extra = 2}, &tree_probe_services, &shared);
	CHECK(*counter == 36 && counter == openttd_rust_tree_counter());
	openttd_rust_trees_tick({.tick_counter = 0, .size = 4096 * 4096, .climate = 0, .extra = 2}, &tree_probe_services, &shared);
	CHECK(*counter == 36 && tree_probe.writes == 0);
	tree_probe.plant = {.tile_type = 0, .bridge = false, .ground = 0, .density = 2, .snow = false, .coast = false, .zone = 0};
	openttd_rust_trees_plant(1, 12, 2, 6, &tree_probe_services, &shared);
	CHECK(tree_probe.writes == 1 && openttd_rust_trees_can_plant(1, false, &tree_probe_services));
	tree_probe.plant.tile_type = 4;
	const auto command = openttd_rust_trees_cmd_plant(nullptr, 1, 1, 255, false,
		{.build_price = 15, .size = 4096, .climate = 0, .editor = false, .execute = false, .company_valid = false}, &tree_probe_services, &shared);
	CHECK(command.cost == 30 && command.status == 0);
	tree_probe.plant.tile_type = 0;
	openttd_rust_trees_tile_loop(1, {.tick_counter = 0, .size_x = 64, .climate = 0, .extra = 2, .ambient = false}, &tree_probe_services, &shared);
	CHECK(tree_probe.waters == 1 && tree_probe.clears == 1 && tree_probe.nested_cost == 40 && tree_probe.writes == 1);
	openttd_rust_trees_initialize();
	CHECK(*counter == 0 && counter == openttd_rust_tree_counter());
	tree_probe.abort = true;
	const OpenTTDTreeGenerateSettings generate{.size = 4096, .size_x = 64, .size_y = 64, .climate = 0, .placer = 1, .height_limit = 15, .editor = false, .freeform_edges = true};
	CHECK(openttd_rust_trees_generate(generate, &tree_probe_services, &shared));
	CHECK(openttd_rust_trees_place_randomly(generate, &tree_probe_services, &shared) && tree_probe.writes == 1);
	std::printf("tree_direct_services_counter_abort_and_reentry passed\n");
}

/* Complete typed tables exercise road cdecl calls without importing game globals.
 * These are boundary/lifetime checks, not a second simulation oracle. */
template <typename Function> struct VehicleUnused;
template <typename Result, typename... Args>
struct VehicleUnused<Result (OPENTTD_VEHICLE_CALL *)(Args...) noexcept> {
	static Result OPENTTD_VEHICLE_CALL Call(Args...) noexcept
	{
		CHECK(false);
		if constexpr (!std::is_void_v<Result>) return {};
	}
};
#define VEHICLE_UNUSED(table, field) table.field = VehicleUnused<decltype(table.field)>::Call
static OpenTTDVehicleMaintenanceServices _vehicle_maintenance_probe;
struct RoadTestTables {
	OpenTTDVehicleServices vehicle;
	OpenTTDGroundServices ground;
	OpenTTDSharedServices shared;
	OpenTTDRoadLeaves leaves;
	explicit RoadTestTables(const OpenTTDSharedServices *world) : shared(*world)
	{
		vehicle.maintenance = &_vehicle_maintenance_probe;
		VEHICLE_UNUSED(vehicle, write_counters);


		VEHICLE_UNUSED(vehicle, altitude_inputs);

		VEHICLE_UNUSED(vehicle, breakdown);
		VEHICLE_UNUSED(vehicle, check_orders_only);
		VEHICLE_UNUSED(vehicle, write_coordinates);
		VEHICLE_UNUSED(vehicle, subtype);
		VEHICLE_UNUSED(vehicle, x);
		VEHICLE_UNUSED(vehicle, y);
		VEHICLE_UNUSED(vehicle, z);
		VEHICLE_UNUSED(vehicle, tick);
		VEHICLE_UNUSED(vehicle, owner);
		VEHICLE_UNUSED(vehicle, speed);
		VEHICLE_UNUSED(vehicle, subspeed);
		VEHICLE_UNUSED(vehicle, running);
		VEHICLE_UNUSED(vehicle, order_time);
		VEHICLE_UNUSED(vehicle, vehicle_type);
		VEHICLE_UNUSED(vehicle, write_x);
		VEHICLE_UNUSED(vehicle, write_y);
		VEHICLE_UNUSED(vehicle, write_z);
		VEHICLE_UNUSED(vehicle, write_tile);
		VEHICLE_UNUSED(vehicle, write_status);
		VEHICLE_UNUSED(vehicle, write_subspeed);
		VEHICLE_UNUSED(vehicle, write_breakdown_counter);
		VEHICLE_UNUSED(vehicle, write_running);
		VEHICLE_UNUSED(vehicle, write_order_time);
		VEHICLE_UNUSED(vehicle, write_profit);
		VEHICLE_UNUSED(vehicle, write_economy_service);
		VEHICLE_UNUSED(vehicle, write_calendar_service);
		VEHICLE_UNUSED(vehicle, write_breakdowns);
		VEHICLE_UNUSED(vehicle, write_reliability);
		VEHICLE_UNUSED(vehicle, write_cargo_age);
		VEHICLE_UNUSED(vehicle, coordinates);
		VEHICLE_UNUSED(vehicle, xy);
		VEHICLE_UNUSED(vehicle, speed_inputs);
		VEHICLE_UNUSED(vehicle, direction_progress);
		VEHICLE_UNUSED(vehicle, tick_status);
		VEHICLE_UNUSED(vehicle, day_cost);
		VEHICLE_UNUSED(vehicle, make_dummy);
		VEHICLE_UNUSED(vehicle, loading);
		VEHICLE_UNUSED(vehicle, write_tick);
		VEHICLE_UNUSED(vehicle, direction);
		VEHICLE_UNUSED(vehicle, status);
		VEHICLE_UNUSED(vehicle, tile);
		VEHICLE_UNUSED(vehicle, progress);
		VEHICLE_UNUSED(vehicle, dest_tile);
		VEHICLE_UNUSED(vehicle, order_type);
		VEHICLE_UNUSED(vehicle, order_destination);
		VEHICLE_UNUSED(vehicle, order_nonstop);
		VEHICLE_UNUSED(ground, has_articulated);
		VEHICLE_UNUSED(leaves, tick_front);
		VEHICLE_UNUSED(leaves, max_depot_penalty);
		VEHICLE_UNUSED(vehicle, handle_breakdown);
		VEHICLE_UNUSED(leaves, process_orders_then_loading);
		VEHICLE_UNUSED(vehicle, waiting_for_unbunching);
		VEHICLE_UNUSED(vehicle, leave_unbunching_depot);
		VEHICLE_UNUSED(vehicle, reset_depot_unbunching);
		VEHICLE_UNUSED(vehicle, service_in_depot);
		VEHICLE_UNUSED(vehicle, needs_automatic_servicing);
		VEHICLE_UNUSED(vehicle, enter_depot);
		VEHICLE_UNUSED(vehicle, pathfinding_result);
		VEHICLE_UNUSED(vehicle, new_position);
		VEHICLE_UNUSED(vehicle, move_then_status);
		VEHICLE_UNUSED(vehicle, base_viewport);
		VEHICLE_UNUSED(vehicle, property);
		VEHICLE_UNUSED(vehicle, length_callback);
		VEHICLE_UNUSED(vehicle, unknown_length_result);
		VEHICLE_UNUSED(vehicle, length_changed);
		VEHICLE_UNUSED(vehicle, play_start_sound);
		VEHICLE_UNUSED(vehicle, play_sound);
		VEHICLE_UNUSED(vehicle, large_explosion);
		VEHICLE_UNUSED(vehicle, order_free);
		VEHICLE_UNUSED(vehicle, order_dummy);
		VEHICLE_UNUSED(vehicle, start_stop_dirty);
		VEHICLE_UNUSED(vehicle, destroy);
		VEHICLE_UNUSED(vehicle, age);
		VEHICLE_UNUSED(vehicle, economy_age);
		VEHICLE_UNUSED(vehicle, decrease_value);
		VEHICLE_UNUSED(vehicle, check_breakdown);
		VEHICLE_UNUSED(vehicle, check_orders);
		VEHICLE_UNUSED(vehicle, write_day);
		VEHICLE_UNUSED(vehicle, write_direction);
		VEHICLE_UNUSED(vehicle, write_speed);
		VEHICLE_UNUSED(vehicle, last_station);
		VEHICLE_UNUSED(vehicle, write_last_station);
		VEHICLE_UNUSED(vehicle, write_dest);
		VEHICLE_UNUSED(vehicle, write_progress);
		VEHICLE_UNUSED(ground, move_incline);
		VEHICLE_UNUSED(ground, update_viewport_then_next);
		VEHICLE_UNUSED(ground, turn);
		VEHICLE_UNUSED(ground, do_update_speed);
		VEHICLE_UNUSED(ground, set_last_speed);
		VEHICLE_UNUSED(ground, acceleration);
		VEHICLE_UNUSED(ground, crash);
		VEHICLE_UNUSED(ground, first_cargo_changed);
		VEHICLE_UNUSED(ground, go_to_depot_service);
		VEHICLE_UNUSED(ground, write_first_engine);
		VEHICLE_UNUSED(ground, write_length);
		VEHICLE_UNUSED(shared, tile_type);
		VEHICLE_UNUSED(shared, tile_owner);
		VEHICLE_UNUSED(shared, station_index);
		VEHICLE_UNUSED(shared, depot_index);
		VEHICLE_UNUSED(shared, is_level_crossing);
		VEHICLE_UNUSED(shared, tunnel_bridge_direction);
		VEHICLE_UNUSED(shared, bridge_speed);
		VEHICLE_UNUSED(leaves, visual_then_speed);
		VEHICLE_UNUSED(leaves, max_speed_read);
		VEHICLE_UNUSED(leaves, speed_part);
		VEHICLE_UNUSED(leaves, step);
		VEHICLE_UNUSED(leaves, crash_part);
		VEHICLE_UNUSED(leaves, part);
		VEHICLE_UNUSED(leaves, link);
		VEHICLE_UNUSED(leaves, link_tile);
		VEHICLE_UNUSED(leaves, enter_tile);
		VEHICLE_UNUSED(leaves, visit_close);
		VEHICLE_UNUSED(leaves, visit_trains);
		VEHICLE_UNUSED(leaves, visit_tile);
		VEHICLE_UNUSED(leaves, has_road);
		VEHICLE_UNUSED(leaves, track_status);
		VEHICLE_UNUSED(leaves, any_road_bits);
		VEHICLE_UNUSED(leaves, tram_bits);
		VEHICLE_UNUSED(leaves, is_road_depot_tile);
		VEHICLE_UNUSED(leaves, depot_direction);
		VEHICLE_UNUSED(leaves, is_normal_road);
		VEHICLE_UNUSED(leaves, has_road_works);
		VEHICLE_UNUSED(leaves, disallowed_directions);
		VEHICLE_UNUSED(leaves, is_bay_stop);
		VEHICLE_UNUSED(leaves, bay_direction);
		VEHICLE_UNUSED(leaves, is_drive_through);
		VEHICLE_UNUSED(leaves, is_station_road_stop);
		VEHICLE_UNUSED(leaves, stop_type);
		VEHICLE_UNUSED(leaves, has_free_bay);
		VEHICLE_UNUSED(leaves, continuation);
		VEHICLE_UNUSED(leaves, move_tile);
		VEHICLE_UNUSED(leaves, path_read);
		VEHICLE_UNUSED(leaves, choose_track);
		VEHICLE_UNUSED(leaves, find_depot);
		VEHICLE_UNUSED(leaves, can_build_tram);
		VEHICLE_UNUSED(vehicle, previous_tile);
		VEHICLE_UNUSED(leaves, disconnect);
		VEHICLE_UNUSED(leaves, stop_read);
		VEHICLE_UNUSED(leaves, should_stop);
		VEHICLE_UNUSED(leaves, stop_leave);
		VEHICLE_UNUSED(leaves, stop_entrance);
		VEHICLE_UNUSED(leaves, stop_entrance_busy);
		VEHICLE_UNUSED(leaves, arrival_read);
		VEHICLE_UNUSED(leaves, arrival_news);
		VEHICLE_UNUSED(leaves, begin_loading_at);
		VEHICLE_UNUSED(leaves, overtake_read);
		VEHICLE_UNUSED(leaves, sound_read);
		VEHICLE_UNUSED(leaves, depot_exit);
		VEHICLE_UNUSED(leaves, crash_news);
		VEHICLE_UNUSED(vehicle, detach_last);
		VEHICLE_UNUSED(ground, cache_begin);
		VEHICLE_UNUSED(vehicle, next);
		VEHICLE_UNUSED(vehicle, engine_id);
		VEHICLE_UNUSED(vehicle, engine_for_length);
		VEHICLE_UNUSED(ground, cached_length);
		VEHICLE_UNUSED(leaves, shorten_factor);
		VEHICLE_UNUSED(leaves, default_max_speed);
		VEHICLE_UNUSED(leaves, update_cargo_age);
		VEHICLE_UNUSED(vehicle, write_max_speed);
		VEHICLE_UNUSED(leaves, cost_read);
		VEHICLE_UNUSED(leaves, price);
		VEHICLE_UNUSED(leaves, pay_running);
		VEHICLE_UNUSED(leaves, service_read);
		VEHICLE_UNUSED(leaves, turn_read);
		VEHICLE_UNUSED(leaves, trackdir_read);
		leaves.vehicle = &vehicle;
		leaves.ground = &ground;
		leaves.shared = &shared;
	}
	RoadTestTables(const RoadTestTables &) = delete;
	RoadTestTables &operator=(const RoadTestTables &) = delete;
};
#undef VEHICLE_UNUSED
struct RoadAbiProbe {
	OpenTTDRoadState *state = openttd_rust_road_new();
	const OpenTTDRoadLeaves *leaves = nullptr;
	OpenTTDRoadEntry entry{.acceleration_model = 0, .road_side = 0, .queue = false, .map_log_x = 8};
	uint32_t dest = 0, nested = 0, destroyed = 0, departures = 0, progress = 255;
	uint8_t tick = 0, running = 0;
	uint32_t cache_properties = 0, cargo_ages = 0, length_changes = 0, viewports = 0, turns = 0, next_reads = 0;
	uint32_t progress_writes = 0;
	bool fallback_cache = false, engine_replaced = false;
	bool hidden_viewport_tail = false;
	int32_t order_time = 0;
};
static RoadAbiProbe *road_abi_probe;
static int road_abi_second, road_abi_hidden, road_abi_engine, road_abi_new_engine;
static int road_abi_vehicle; ///< Its address is the opaque vehicle handle.
static OpenTTDVehicle *RoadAbiHandle() { return reinterpret_cast<OpenTTDVehicle *>(&road_abi_vehicle); }
static OpenTTDRoadMaxSpeedRead RoadAbiMaxSpeed()
{
	return {.next = nullptr, .next_state = nullptr, .tile = 100, .max_track_speed = 80, .order_max_speed = 65535, .direction = 1, .status = 0};
}
static void RoadBoundary()
{
	RoadAbiProbe probe;
	road_abi_probe = &probe;
	EffectTestWorld world;
	effect_test_world = &world;
	const OpenTTDSharedServices services = OpenTTDFixtureSharedServices(&world, EffectTestRandom, EffectTestIndustry);
	RoadTestTables t(&services);
	probe.leaves = &t.leaves;
	OpenTTDVehicle *v = RoadAbiHandle();

	/* An articulated part only stores its counters. */
	t.vehicle.write_counters = [](OpenTTDVehicle *u, uint8_t tick, uint8_t running, int32_t order_time) noexcept {
		CHECK(u == RoadAbiHandle());
		auto &p = *road_abi_probe;
		p.tick = tick;
		p.running = running;
		p.order_time = order_time;
	};
	t.vehicle.write_tick = [](OpenTTDVehicle *u, uint8_t tick) noexcept { CHECK(u == RoadAbiHandle()); road_abi_probe->tick = tick; };
	const OpenTTDRoadTickRead part{.tick = 255, .front = false};
	CHECK(openttd_rust_road_tick(v, probe.state, &part, &t.leaves));
	CHECK(probe.tick == 0 && probe.running == 0 && probe.order_time == 0 && world.draws == 0);

	/* A direct callback changes canonical state before Rust reobserves it. */
	t.ground.crash = [](OpenTTDVehicle *u, bool flooded) noexcept -> OpenTTDGroundCrash {
		CHECK(u == RoadAbiHandle() && !flooded);
		openttd_rust_road_set(road_abi_probe->state, 0, 64);
		return {.victims = 7, .front = true};
	};
	t.leaves.stop_leave = [](OpenTTDVehicle *) noexcept { ++road_abi_probe->departures; };
	CHECK(openttd_rust_road_crash(v, probe.state, false, &t.leaves) == 8);
	CHECK(probe.departures == 1 && openttd_rust_road_get(probe.state, 5) == 1);

	/* Entries nested in a NewGRF property callback see and change the same owner. */
	t.leaves.trackdir_read = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadTrackdirRead { return {.tile = 100, .direction = 1, .status = 0}; };
	t.leaves.is_bay_stop = [](uint32_t) noexcept { return false; };
	t.vehicle.write_dest = [](OpenTTDVehicle *, uint32_t tile) noexcept { road_abi_probe->dest = tile; };
	t.ground.cache_begin = [](OpenTTDVehicle *) noexcept {};
	t.vehicle.engine_for_length = [](OpenTTDVehicle *) noexcept -> OpenTTDEngineForLength {
		auto &p = *road_abi_probe;
		CHECK(p.cache_properties == p.cargo_ages);
		return {.engine = reinterpret_cast<const OpenTTDEngine *>(p.engine_replaced ? &road_abi_new_engine : &road_abi_engine), .grf_version = 8};
	};
	t.vehicle.next = [](OpenTTDVehicle *u) noexcept -> OpenTTDVehicle * {
		auto &p = *road_abi_probe;
		CHECK(p.cargo_ages == (u == RoadAbiHandle() ? 1U : 2U));
		return p.fallback_cache && u == RoadAbiHandle() ? reinterpret_cast<OpenTTDVehicle *>(&road_abi_second) : nullptr;
	};
	t.vehicle.engine_id = [](OpenTTDVehicle *u) noexcept -> uint16_t {
		CHECK(u == RoadAbiHandle() && road_abi_probe->cargo_ages == 1);
		return 9; // The first part's callbacks changed the head engine.
	};
	t.leaves.shorten_factor = [](const OpenTTDEngine *e) noexcept -> uint8_t {
		CHECK(road_abi_probe->fallback_cache && e == reinterpret_cast<const OpenTTDEngine *>(&road_abi_engine) && road_abi_probe->cache_properties == 1);
		return 2; // Read through the engine captured before the property callback.
	};
	t.ground.cached_length = [](OpenTTDVehicle *) noexcept -> uint8_t {
		CHECK(road_abi_probe->fallback_cache && road_abi_probe->cache_properties > road_abi_probe->cargo_ages);
		return 8;
	};
	t.vehicle.length_changed = [](OpenTTDVehicle *) noexcept { ++road_abi_probe->length_changes; };
	t.vehicle.unknown_length_result = [](const OpenTTDEngine *e, uint16_t result) noexcept {
		CHECK(e == reinterpret_cast<const OpenTTDEngine *>(&road_abi_new_engine) && result == 9);
	};
	t.leaves.default_max_speed = [](OpenTTDVehicle *) noexcept -> uint16_t {
		CHECK(road_abi_probe->fallback_cache && road_abi_probe->cache_properties == 3);
		return 99;
	};
	t.ground.write_first_engine = [](OpenTTDVehicle *, uint16_t engine) noexcept { CHECK(engine == 0xFFFF || (road_abi_probe->fallback_cache && engine == 9)); };
	t.vehicle.property = [](OpenTTDVehicle *u, uint8_t property, int32_t fallback) noexcept -> int32_t {
		auto &p = *road_abi_probe;
		++p.cache_properties;
		if (p.fallback_cache) {
			p.engine_replaced = true;
			return property == 0x23 ? (u == RoadAbiHandle() ? 0xFFFF : 9) : 0;
		}
		const OpenTTDRoadMaxSpeedRead read = RoadAbiMaxSpeed();
		CHECK(openttd_rust_road_max_speed(u, p.state, p.entry.acceleration_model, &read, p.leaves) == 80);
		CHECK(openttd_rust_road_trackdir(u, p.state, p.leaves) == 0);
		++p.nested;
		openttd_rust_road_set_dest(u, p.state, 142 + p.nested, p.dest, p.leaves);
		CHECK(openttd_rust_road_path_size(p.state) == 0);
		openttd_rust_road_path_push(p.state, {10, 10815});
		return property == 0x23 ? 0 : property == 0x15 ? 20 : fallback;
	};
	t.ground.write_length = [](OpenTTDVehicle *u, uint8_t length, OpenTTDVehicle *, uint16_t total) noexcept {
		CHECK(road_abi_probe->fallback_cache ? (u == RoadAbiHandle() ? length == 6 && total == 6 : length == 1 && total == 7) : length == 8 && total == 8);
	};
	t.leaves.update_cargo_age = [](OpenTTDVehicle *) noexcept { ++road_abi_probe->cargo_ages; };
	t.vehicle.write_max_speed = [](OpenTTDVehicle *, uint16_t speed) noexcept { CHECK(speed == (road_abi_probe->fallback_cache ? 99 : 80)); };
	openttd_rust_road_update_cache(v, probe.state, false, &t.leaves);
	CHECK(probe.nested == 2 && probe.dest == 144 && openttd_rust_road_path_get(probe.state, 0).tile == 10815);

	/* Failed/invalid properties consume late fallback reads and the post-callback link.
	 * The first, !same_length run rejects conditional reads; a changed engine
	 * still leaves fallback/error reads attached to the captured engine context. */
	probe.fallback_cache = true;
	probe.cache_properties = probe.cargo_ages = 0;
	openttd_rust_road_update_cache(v, probe.state, true, &t.leaves);
	CHECK(probe.cargo_ages == 2 && probe.length_changes == 2);

	/* Running-cost property callbacks can replace the vehicle's engine. Price retains the original engine. */
	probe.engine_replaced = false;
	t.leaves.cost_read = [](OpenTTDVehicle *u) noexcept -> OpenTTDRoadCostRead {
		CHECK(u == RoadAbiHandle() && !road_abi_probe->engine_replaced);
		return {.engine = reinterpret_cast<const OpenTTDEngine *>(&road_abi_engine), .cost_class = 1, .cost_factor = 3};
	};
	t.vehicle.property = [](OpenTTDVehicle *u, uint8_t property, int32_t value) noexcept -> int32_t {
		CHECK(u == RoadAbiHandle() && property == 9 && value == 3);
		road_abi_probe->engine_replaced = true;
		return 4;
	};
	t.leaves.price = [](const OpenTTDEngine *e, uint32_t factor) noexcept -> int64_t {
		CHECK(road_abi_probe->engine_replaced && e == reinterpret_cast<const OpenTTDEngine *>(&road_abi_engine) && factor == 4);
		return 123;
	};
	CHECK(openttd_rust_road_running_cost(v, probe.state, &t.leaves) == 123);

	/* A non-station order must not read its destination at the bay stopping frame. */
	openttd_rust_road_set(probe.state, 0, 32);
	openttd_rust_road_set(probe.state, 1, 20);
	openttd_rust_road_set(probe.state, 6, 1); // Skip nearby-vehicle traversal.
	t.leaves.step = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadStep {
		uint16_t rd = openttd_rust_road_drive_entry(0, 32, 21);
		CHECK((rd & 0xC0) == 0 && (rd & 15) < 15);
		return {.tile = 100, .x = (rd & 15) + 1, .y = (rd >> 8) & 15, .z = 0, .cur_speed = 0, .direction = 1, .length = 8, .front = true, .tram = false};
	};
	t.leaves.link = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadLink { return {.next = nullptr, .next_state = nullptr}; };
	t.shared.station_index = [](uint32_t tile) noexcept -> uint16_t { CHECK(tile == 100); return 55; };
	t.leaves.is_drive_through = [](uint32_t tile) noexcept { CHECK(tile == 100); return false; };
	t.leaves.stop_entrance = [](OpenTTDVehicle *, bool busy) noexcept { CHECK(!busy); };
	t.vehicle.write_last_station = [](OpenTTDVehicle *, uint16_t station) noexcept { CHECK(station == 55); };
	t.vehicle.order_type = [](OpenTTDVehicle *) noexcept -> uint8_t { return 0; };
	t.leaves.is_bay_stop = [](uint32_t tile) noexcept { CHECK(tile == 100); return false; };
	t.vehicle.play_start_sound = [](OpenTTDVehicle *) noexcept { return true; };
	t.vehicle.start_stop_dirty = [](OpenTTDVehicle *) noexcept {};
	t.leaves.enter_tile = [](OpenTTDVehicle *, uint32_t, int32_t, int32_t) noexcept -> OpenTTDRoadEntered { return {.tile = 100, .cur_speed = 0, .flags = 4, .direction = 1, .order_type = 0}; };
	t.vehicle.write_speed = [](OpenTTDVehicle *, uint16_t speed) noexcept { CHECK(speed == 0); };
	CHECK(!openttd_rust_road_individual(v, probe.state, nullptr, nullptr, &probe.entry, &t.leaves));
	CHECK(openttd_rust_road_get(probe.state, 0) == 36);
	openttd_rust_road_set(probe.state, 0, 0);
	openttd_rust_road_set(probe.state, 6, 0);

	/* A front tick reenters the setter from ProcessOrders and the speed getter from ShowVisualEffect. */
	t.leaves.process_orders_then_loading = [](OpenTTDVehicle *u) noexcept -> uint8_t {
		auto &p = *road_abi_probe;
		openttd_rust_road_set_dest(u, p.state, 100, p.dest, p.leaves);
		return 0;
	};
	t.leaves.visual_then_speed = [](OpenTTDVehicle *u, bool realistic) noexcept -> OpenTTDRoadSpeedRead {
		auto &p = *road_abi_probe;
		CHECK(!realistic);
		const OpenTTDRoadMaxSpeedRead read = RoadAbiMaxSpeed();
		CHECK(openttd_rust_road_max_speed(u, p.state, p.entry.acceleration_model, &read, p.leaves) == 80);
		++p.nested;
		return {.max = read, .x = 0, .y = 0, .z = 0, .acceleration = 0, .length = 8, .tram = false};
	};
	t.ground.do_update_speed = [](OpenTTDVehicle *, uint32_t acceleration, int32_t minimum, int32_t maximum) noexcept -> OpenTTDGroundSpeed {
		CHECK(acceleration == 256 && minimum == 0 && maximum == 80);
		return {.distance = 0, .cur_speed = 0};
	};
	t.ground.set_last_speed = [](OpenTTDVehicle *) noexcept -> OpenTTDGroundLastSpeed { road_abi_probe->progress = 0; return {.status = 0}; };
	t.ground.update_viewport_then_next = [](OpenTTDVehicle *part, OpenTTDVehicle *front) noexcept -> OpenTTDGroundViewportNext {
		CHECK(part == RoadAbiHandle() && front == part);
		++road_abi_probe->viewports;
		road_abi_probe->progress = 7;
		++road_abi_probe->next_reads;
		return {.next = nullptr, .final_progress = static_cast<uint8_t>(road_abi_probe->progress)};
	};
	t.leaves.tick_front = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadFrontTickRead { return {.movement = road_abi_probe->entry, .order_time = 5, .running = 0, .status = 0}; };
	t.leaves.crash_part = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadCrashPart { return {.next = nullptr, .next_state = nullptr, .tile = 100, .z = 0, .crossing = false}; };
	const OpenTTDRoadTickRead front{.tick = 0, .front = true};
	CHECK(openttd_rust_road_tick(v, probe.state, &front, &t.leaves));
	CHECK(probe.nested == 3 && probe.dest == 100 && probe.order_time == 6 && probe.running == 1 && probe.progress == 7 && probe.next_reads == 1 && world.draws == 0);

	/* Hidden vehicles skip viewport callbacks, then use the direct canonical link. */
	t.ground.set_last_speed = [](OpenTTDVehicle *) noexcept -> OpenTTDGroundLastSpeed { return {.status = 1}; };
	t.vehicle.next = [](OpenTTDVehicle *) noexcept -> OpenTTDVehicle * { CHECK(road_abi_probe->viewports == 1); ++road_abi_probe->next_reads; return nullptr; };
	t.vehicle.progress = [](OpenTTDVehicle *) noexcept -> uint8_t { CHECK(road_abi_probe->viewports == 1 && road_abi_probe->next_reads == 2); return static_cast<uint8_t>(road_abi_probe->progress); };
	CHECK(openttd_rust_road_tick(v, probe.state, &front, &t.leaves));
	CHECK(probe.viewports == 1 && probe.next_reads == 2 && probe.progress == 7);

	/* Viewport reentry can add a later visible part. Its progress mutation is
	 * observed only after that viewport and any hidden final part's Next read. */
	t.ground.set_last_speed = [](OpenTTDVehicle *) noexcept -> OpenTTDGroundLastSpeed { road_abi_probe->progress = 0; return {.status = 0}; };
	t.ground.update_viewport_then_next = [](OpenTTDVehicle *part, OpenTTDVehicle *front) noexcept -> OpenTTDGroundViewportNext {
		auto &p = *road_abi_probe;
		CHECK(front == RoadAbiHandle());
		CHECK((p.viewports == 0 && part == front) || (p.viewports == 1 && part == reinterpret_cast<OpenTTDVehicle *>(&road_abi_second)));
		++p.viewports;
		p.progress = part == front ? 7 : 0;
		openttd_rust_road_set_dest(front, p.state, 200 + p.viewports, p.dest, p.leaves);
		++p.next_reads;
		OpenTTDVehicle *next = part == front ? reinterpret_cast<OpenTTDVehicle *>(&road_abi_second) : p.hidden_viewport_tail ? reinterpret_cast<OpenTTDVehicle *>(&road_abi_hidden) : nullptr;
		return {.next = next, .final_progress = next == nullptr ? uint8_t(p.progress) : uint8_t(99)};
	};
	t.vehicle.status = [](OpenTTDVehicle *part) noexcept -> uint8_t {
		bool hidden = part == reinterpret_cast<OpenTTDVehicle *>(&road_abi_hidden);
		CHECK(hidden ? road_abi_probe->hidden_viewport_tail && road_abi_probe->viewports == 2 : part == reinterpret_cast<OpenTTDVehicle *>(&road_abi_second) && road_abi_probe->viewports == 1);
		return hidden ? 1 : 0;
	};
	t.vehicle.next = [](OpenTTDVehicle *part) noexcept -> OpenTTDVehicle * {
		CHECK(part == reinterpret_cast<OpenTTDVehicle *>(&road_abi_hidden) && road_abi_probe->hidden_viewport_tail && road_abi_probe->viewports == 2);
		++road_abi_probe->next_reads;
		return nullptr;
	};
	t.vehicle.progress = [](OpenTTDVehicle *front) noexcept -> uint8_t {
		CHECK(front == RoadAbiHandle() && road_abi_probe->hidden_viewport_tail && road_abi_probe->viewports == 2 && road_abi_probe->next_reads == 3);
		return road_abi_probe->progress;
	};
	t.vehicle.write_progress = [](OpenTTDVehicle *front, uint8_t progress) noexcept {
		auto &p = *road_abi_probe;
		CHECK(front == RoadAbiHandle() && p.viewports == 2 && p.next_reads == (p.hidden_viewport_tail ? 3U : 2U) && progress == 0);
		++p.progress_writes;
		p.progress = progress;
	};
	for (bool hidden : {false, true}) {
		probe.hidden_viewport_tail = hidden;
		probe.viewports = probe.next_reads = probe.progress_writes = 0;
		CHECK(openttd_rust_road_tick(v, probe.state, &front, &t.leaves));
		CHECK(probe.viewports == 2 && probe.next_reads == (hidden ? 3U : 2U) && probe.progress_writes == 1 && probe.progress == 0 && probe.dest == 202);
	}

	/* Crash animation reads the next link after the turn/viewport callback. */
	openttd_rust_road_set(probe.state, 5, 0);
	t.leaves.tick_front = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadFrontTickRead { return {.movement = road_abi_probe->entry, .order_time = 0, .running = 0, .status = 128}; };
	t.vehicle.direction = [](OpenTTDVehicle *u) noexcept -> uint8_t { CHECK(u == RoadAbiHandle() && road_abi_probe->turns == 0); return 1; };
	t.ground.turn = [](OpenTTDVehicle *u, uint8_t direction) noexcept { CHECK(u == RoadAbiHandle() && direction < 8); ++road_abi_probe->turns; };
	t.vehicle.next = [](OpenTTDVehicle *u) noexcept -> OpenTTDVehicle * { CHECK(u == RoadAbiHandle() && road_abi_probe->turns == 1); return nullptr; };
	const OpenTTDRoadTickRead animation{.tick = 7, .front = true};
	CHECK(openttd_rust_road_tick(v, probe.state, &animation, &t.leaves));
	CHECK(probe.turns == 1 && world.draws == 1);

	/* The final-part deletion returns its copied alive result only. */
	openttd_rust_road_set(probe.state, 0, 0);
	openttd_rust_road_set(probe.state, 5, 2219);
	t.leaves.link = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadLink { return {.next = nullptr, .next_state = nullptr}; };
	t.vehicle.detach_last = [](OpenTTDVehicle *previous, OpenTTDVehicle *last, OpenTTDVehicle *first) noexcept {
		CHECK(previous == RoadAbiHandle() && last == previous && first == previous);
	};
	t.vehicle.destroy = [](OpenTTDVehicle *u) noexcept {
		CHECK(u == RoadAbiHandle());
		openttd_rust_road_destroy(road_abi_probe->state);
		road_abi_probe->state = nullptr;
		++road_abi_probe->destroyed;
	};
	t.leaves.tick_front = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadFrontTickRead { return {.movement = road_abi_probe->entry, .order_time = 0, .running = 0, .status = 128}; };
	const OpenTTDRoadTickRead crashed{.tick = 31, .front = true};
	CHECK(!openttd_rust_road_tick(v, probe.state, &crashed, &t.leaves));
	CHECK(probe.state == nullptr && probe.destroyed == 1 && world.draws == 1);
	std::printf("road_native_typed_reentry_path_and_owner_destruction passed\n");
}

/* Moving and partly entered consists cannot use IsChainInDepot's service path,
 * but FindClosestRoadDepot still selects their current depot at distance zero. */
struct RoadServiceProbe {
	OpenTTDRoadState *head = openttd_rust_road_new();
	OpenTTDRoadState *tail = openttd_rust_road_new();
	uint32_t tile = 3091, dest = 0;
	uint16_t speed = 5;
	bool depot_tile = true;
	uint32_t depot_orders = 0, services = 0, searches = 0;
};
static RoadServiceProbe *road_service_probe;
static int road_service_vehicles[2]; ///< Their addresses are the opaque head and tail handles.
static OpenTTDVehicle *RoadServiceHandle(int part) { return reinterpret_cast<OpenTTDVehicle *>(&road_service_vehicles[part]); }
static void RoadServiceBoundary()
{
	RoadServiceProbe probe;
	road_service_probe = &probe;
	EffectTestWorld world;
	effect_test_world = &world;
	const OpenTTDSharedServices services = OpenTTDFixtureSharedServices(&world, EffectTestRandom, EffectTestIndustry);
	RoadTestTables t(&services);
	t.leaves.max_depot_penalty = []() noexcept -> uint32_t { return 300; };
	t.vehicle.economy_age = [](OpenTTDVehicle *) noexcept -> uint8_t { return 0; };
	t.vehicle.write_day = [](OpenTTDVehicle *, uint8_t day) noexcept { CHECK(day == 1); };
	t.vehicle.check_breakdown = [](OpenTTDVehicle *) noexcept {};
	t.leaves.service_read = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadServiceRead {
		auto &p = *road_service_probe;
		return {.tile = p.tile, .servint = 1, .cur_speed = p.speed};
	};
	t.vehicle.needs_automatic_servicing = [](OpenTTDVehicle *) noexcept { return true; };
	t.leaves.is_road_depot_tile = [](uint32_t tile) noexcept { CHECK(tile == road_service_probe->tile); return road_service_probe->depot_tile; };
	t.leaves.link_tile = [](OpenTTDVehicle *u) noexcept -> OpenTTDRoadLinkTile {
		auto &p = *road_service_probe;
		bool head = u == RoadServiceHandle(0);
		return {.link = {.next = head ? RoadServiceHandle(1) : nullptr, .next_state = head ? p.tail : nullptr}, .tile = p.tile};
	};
	t.leaves.link = [](OpenTTDVehicle *u) noexcept -> OpenTTDRoadLink {
		bool head = u == RoadServiceHandle(0);
		return {.next = head ? RoadServiceHandle(1) : nullptr, .next_state = head ? road_service_probe->tail : nullptr};
	};
	t.leaves.find_depot = [](OpenTTDVehicle *, int32_t penalty) noexcept -> OpenTTDRoadDepotResult {
		CHECK(penalty == 300);
		++road_service_probe->searches;
		return {.tile = UINT32_MAX, .length = UINT32_MAX};
	};
	t.shared.depot_index = [](uint32_t tile) noexcept -> uint16_t { CHECK(tile == road_service_probe->tile); return 55; };
	t.vehicle.order_type = [](OpenTTDVehicle *) noexcept -> uint8_t { return 0; };
	t.ground.go_to_depot_service = [](OpenTTDVehicle *, uint16_t depot) noexcept { CHECK(depot == 55); ++road_service_probe->depot_orders; };
	t.vehicle.dest_tile = [](OpenTTDVehicle *) noexcept -> uint32_t { return road_service_probe->dest; };
	t.vehicle.write_dest = [](OpenTTDVehicle *, uint32_t tile) noexcept { CHECK(tile == road_service_probe->tile); road_service_probe->dest = tile; };
	t.vehicle.service_in_depot = [](OpenTTDVehicle *) noexcept { ++road_service_probe->services; };
	t.vehicle.start_stop_dirty = [](OpenTTDVehicle *) noexcept {};
	t.vehicle.check_orders = [](OpenTTDVehicle *) noexcept -> uint8_t { return 0; };
	auto day = [&]() { openttd_rust_road_economy_day(RoadServiceHandle(0), probe.head, true, &t.leaves); };
	for (bool moving : {true, false}) {
		probe.speed = moving ? 5 : 0;
		probe.dest = 42;
		openttd_rust_road_set(probe.head, 0, 254);
		openttd_rust_road_set(probe.tail, 0, moving ? 254 : 1);
		openttd_rust_road_path_push(probe.head, {10, 10815});
		day();
		CHECK(probe.dest == probe.tile && openttd_rust_road_path_size(probe.head) == 0);
	}
	CHECK(probe.depot_orders == 2 && probe.services == 0 && world.draws == 0);
	openttd_rust_road_set(probe.tail, 0, 254);
	day();
	CHECK(probe.services == 1 && probe.depot_orders == 2);
	probe.depot_tile = false;
	day();
	CHECK(probe.searches == 1);
	openttd_rust_road_destroy(probe.head);
	openttd_rust_road_destroy(probe.tail);
	std::printf("road_service_moving_partial_parked_and_search_branches passed\n");
}

struct DisasterProbe { uint32_t reads = 0, writes = 0, draws = 0, random = 0; };
static void OPENTTD_DISASTER_CALL DisasterProbeRead(void *context, uint32_t kind, uint32_t, int64_t, int64_t, int64_t *out) noexcept
{
	auto *probe = static_cast<DisasterProbe *>(context); probe->reads++;
	if (kind == 0) { out[0] = 64; out[1] = 128; out[2] = 63; out[3] = 127; out[4] = 8191; out[5] = 1935; out[6] = 0; out[9] = 1; }
	if (kind == 1) { out[0] = 13; out[7] = 8880; }
}
static void OPENTTD_DISASTER_CALL DisasterProbeWrite(void *context, uint32_t, uint32_t, int64_t) noexcept
{
	static_cast<DisasterProbe *>(context)->writes++;
}
static uint32_t DisasterProbeRandom(void *context) noexcept { auto *probe = static_cast<DisasterProbe *>(context); probe->draws++; return probe->random; }
static int64_t OPENTTD_DISASTER_CALL DisasterProbeService(void *, const OpenTTDDisasterAction *) noexcept { return 0; }
static uint32_t DisasterProbeIndustry(int32_t, int32_t, uint32_t *) noexcept { return 0; }
static void Disasters()
{
	using Owner = std::unique_ptr<OpenTTDDisasterRun, decltype(&openttd_rust_disaster_destroy)>;
	using StateOwner = std::unique_ptr<OpenTTDDisasterState, decltype(&openttd_rust_disaster_state_destroy)>;
	DisasterProbe probe;
	const OpenTTDSharedServices services = OpenTTDFixtureSharedServices(&probe, DisasterProbeRandom, DisasterProbeIndustry);
	auto make = [&](uint32_t operation, uint32_t id = 0) { return Owner(openttd_rust_disaster_create(operation, id, 0, 0, 0, 0, &probe, DisasterProbeRead, DisasterProbeWrite, &services, DisasterProbeService), openttd_rust_disaster_destroy); };
	StateOwner state(openttd_rust_disaster_state_create(1048575), openttd_rust_disaster_state_destroy);
	auto *address = state.get(); state->state = 65535; state->flags = 0xAB; state->image_override = UINT32_MAX;
	uint16_t *delay = openttd_rust_disaster_delay(); *delay = 0;
	{ auto daily = make(1); CHECK(openttd_rust_disaster_advance(daily.get(), 0).kind == 0 && *delay == 65535 && probe.draws == 0); }
	*delay = 1; probe.random = 511;
	{ auto daily = make(1); CHECK(openttd_rust_disaster_advance(daily.get(), 0).kind == 0 && *delay == 1241 && probe.draws == 1); }
	probe.random = 0;
	{ auto startup = make(2); CHECK(openttd_rust_disaster_advance(startup.get(), 0).kind == 0 && *delay == 730 && probe.draws == 2); }
	CHECK(delay == openttd_rust_disaster_delay() && state.get() == address && state->state == 65535 && state->flags == 0xAB && state->image_override == UINT32_MAX);
	auto expired = make(0, 17);
	CHECK(openttd_rust_disaster_advance(expired.get(), 0).kind == 2);
	uint32_t reads = probe.reads, writes = probe.writes;
	expired.reset(); CHECK(probe.reads == reads && probe.writes == writes && probe.draws == 2);
	std::printf("disaster_private_counter_direct_rng_delete_cancel passed\n");
}


/* Saves cannot observe NOSAVE rotation coordinates. Compare their full bit domain
 * with native C++ narrowing, and exercise real nested owner mutation in a callback. */
struct ShipProbe {
	uint32_t x = 32768, y = static_cast<uint32_t>(-32769), dest = 0;
	uint32_t order_time = UINT32_MAX;
	uint8_t tick = 255, running = 255, direction = 1;
	uint16_t speed = 33;
	uint32_t stage = 0, paths = 0, positions = 0;
	OpenTTDShipState *owner = nullptr;
	const OpenTTDShipLeaves *leaves = nullptr;
	OpenTTDSharedServices services{};
};
static ShipProbe ship_probe;
template <typename Result, typename... Args>
static Result ShipProbeUnused(Args...) noexcept
{
	if constexpr (!std::is_void_v<Result>) return {};
}
static void ShipBoundary()
{
	using Owner = std::unique_ptr<OpenTTDShipState, decltype(&openttd_rust_ship_state_destroy)>;
	Owner state(openttd_rust_ship_state_new(), openttd_rust_ship_state_destroy);
	CHECK(openttd_rust_ship_get_state(state.get()) == 0 && openttd_rust_ship_get_rotation(state.get()) == 255);
	for (uint32_t value = 0; value <= UINT16_MAX; ++value) {
		openttd_rust_ship_set_state(state.get(), static_cast<uint8_t>(value));
		openttd_rust_ship_set_rotation(state.get(), static_cast<uint8_t>(value));
		openttd_rust_ship_set_rotation_x(state.get(), static_cast<int16_t>(value));
		openttd_rust_ship_set_rotation_y(state.get(), static_cast<int16_t>(value));
		CHECK(openttd_rust_ship_get_state(state.get()) == static_cast<uint8_t>(value));
		CHECK(openttd_rust_ship_get_rotation(state.get()) == static_cast<uint8_t>(value));
		CHECK(openttd_rust_ship_get_rotation_x(state.get()) == static_cast<int16_t>(value));
		CHECK(openttd_rust_ship_get_rotation_y(state.get()) == static_cast<int16_t>(value));
	}
	/* Initialize every function pointer, including paths unused in this concrete gap. */
	OpenTTDShipLeaves leaves{
		.depot_dir = ShipProbeUnused<uint32_t, uint32_t>, .depot_axis = ShipProbeUnused<uint32_t, uint32_t>,
		.is_depot = ShipProbeUnused<bool, uint32_t>, .depot_index = ShipProbeUnused<uint32_t, uint32_t>,
		.wait_unbunch = ShipProbeUnused<bool, uint32_t>, .chain_depot = ShipProbeUnused<bool, uint32_t>,
		.servint = ShipProbeUnused<uint32_t, uint32_t>, .needs_service = ShipProbeUnused<bool, uint32_t>,
		.max_distance = ShipProbeUnused<uint32_t>, .tile_valid = ShipProbeUnused<bool, uint32_t>,
		.tile_type = ShipProbeUnused<uint32_t, uint32_t>, .water_class = ShipProbeUnused<uint32_t, uint32_t>,
		.lock_middle = ShipProbeUnused<bool, uint32_t>, .lock_dir = ShipProbeUnused<uint32_t, uint32_t>,
		.tile_min_z = ShipProbeUnused<uint32_t, uint32_t>, .tile_max_z = ShipProbeUnused<uint32_t, uint32_t>,
		.track_status = ShipProbeUnused<uint32_t, uint32_t, uint32_t>, .offset = ShipProbeUnused<uint32_t, uint32_t>,
		.diag_between = ShipProbeUnused<uint32_t, uint32_t, uint32_t>, .dist_square = ShipProbeUnused<uint32_t, uint32_t, uint32_t>,
		.dist_manhattan = ShipProbeUnused<uint32_t, uint32_t, uint32_t>, .docking = ShipProbeUnused<bool, uint32_t>,
		.dock = ShipProbeUnused<bool, uint32_t>, .dock_water = ShipProbeUnused<bool, uint32_t>,
		.station = ShipProbeUnused<uint32_t, uint32_t>, .industry_station = ShipProbeUnused<uint32_t, uint32_t>,
		.oilrig = ShipProbeUnused<bool, uint32_t>, .station_use = ShipProbeUnused<bool, uint32_t, uint32_t>,
		.station_xy = ShipProbeUnused<uint32_t, uint32_t>, .station_contains = ShipProbeUnused<bool, uint32_t, uint32_t>,
		.station_dock = ShipProbeUnused<bool, uint32_t>, .station_visits = ShipProbeUnused<uint32_t, uint32_t>,
		.visit_set = ShipProbeUnused<void, uint32_t, uint32_t>, .arrival = ShipProbeUnused<void, uint32_t, uint32_t>,
		.service = ShipProbeUnused<void, uint32_t>, .leave_unbunch = ShipProbeUnused<void, uint32_t>,
		.path_result = ShipProbeUnused<void, uint32_t, bool>, .order_free = ShipProbeUnused<void, uint32_t>,
		.order_dummy = ShipProbeUnused<void, uint32_t>, .order_depot = ShipProbeUnused<void, uint32_t, uint32_t>,
		.order_leave = ShipProbeUnused<void, uint32_t>, .order_increment = ShipProbeUnused<void, uint32_t>,
		.timetable = ShipProbeUnused<void, uint32_t>, .position = ShipProbeUnused<void, uint32_t>,
		.start_dirty = ShipProbeUnused<void, uint32_t>, .depot_dirty = ShipProbeUnused<void, uint32_t>,
		.depot_invalidate = ShipProbeUnused<void, uint32_t>, .ships_dirty = ShipProbeUnused<void>,
		.details_dirty = ShipProbeUnused<void, uint32_t>, .age = ShipProbeUnused<void, uint32_t>,
		.economy_age = ShipProbeUnused<void, uint32_t>, .decrease_value = ShipProbeUnused<void, uint32_t>,
		.check_breakdown = ShipProbeUnused<void, uint32_t>, .check_orders = ShipProbeUnused<void, uint32_t>,
		.running_cost = ShipProbeUnused<int64_t, uint32_t>, .cost_divisor = ShipProbeUnused<uint32_t>,
		.pay_running = ShipProbeUnused<void, uint32_t, int64_t>, .speed_default = ShipProbeUnused<uint32_t, uint32_t>,
		.age_default = ShipProbeUnused<uint32_t, uint32_t>, .speed_frac = ShipProbeUnused<uint32_t, uint32_t, bool>,
		.speed_property = ShipProbeUnused<uint32_t, uint32_t, uint32_t>, .age_property = ShipProbeUnused<uint32_t, uint32_t, uint32_t>,
		.update_visual = ShipProbeUnused<void, uint32_t>, .cache_invalidate = ShipProbeUnused<void, uint32_t>,
		.capacity = ShipProbeUnused<uint32_t, uint32_t>, .sprite_direction = ShipProbeUnused<void, uint32_t>,
		.tile_x = ShipProbeUnused<uint32_t, uint32_t>, .tile_y = ShipProbeUnused<uint32_t, uint32_t>,
		.build_flag = ShipProbeUnused<bool, uint32_t>, .build_random = ShipProbeUnused<void, uint32_t, uint16_t>,
		.new_position = ShipProbeUnused<OpenTTDShipPosition, uint32_t>, .exit_dir = ShipProbeUnused<uint32_t, uint32_t, uint32_t>,
		.track_direction = ShipProbeUnused<uint32_t, uint32_t, uint32_t>, .tracks_reach = ShipProbeUnused<uint32_t, uint32_t>,
		.busy_tile = ShipProbeUnused<bool, uint32_t>, .path_size = ShipProbeUnused<size_t, uint32_t>,
		.path_back = ShipProbeUnused<uint32_t, uint32_t>, .path_pop = ShipProbeUnused<void, uint32_t>,
		.path_clear = ShipProbeUnused<void, uint32_t>, .enter_tile = ShipProbeUnused<uint32_t, uint32_t, uint32_t, uint32_t, uint32_t>,
		.enter_depot = ShipProbeUnused<void, uint32_t>, .process_orders = ShipProbeUnused<bool, uint32_t>,
		.loading = ShipProbeUnused<void, uint32_t>, .begin_loading = ShipProbeUnused<void, uint32_t>,
		.breakdown = ShipProbeUnused<bool, uint32_t>, .viewport = ShipProbeUnused<void, uint32_t, bool, bool>,
		.base_viewport = ShipProbeUnused<void, uint32_t>, .visual = ShipProbeUnused<void, uint32_t>,
		.cache = ShipProbeUnused<void, uint32_t>, .play_sound = ShipProbeUnused<void, uint32_t>,
		.yapf_reverse = ShipProbeUnused<OpenTTDShipReverseResult, uint32_t, bool>, .yapf_choose = ShipProbeUnused<OpenTTDShipTrackResult, uint32_t, uint32_t>,
		.update_delta = ShipProbeUnused<void, uint32_t>, .build_owner = ShipProbeUnused<void, uint32_t>,
		.build_z = ShipProbeUnused<void, uint32_t>, .build_properties = ShipProbeUnused<void, uint32_t, uint32_t>,
		.build_dates = ShipProbeUnused<void, uint32_t>, .build_acceleration = ShipProbeUnused<void, uint32_t, uint32_t>,
		.build_prototype = ShipProbeUnused<void, uint32_t>, .build_interval_percent = ShipProbeUnused<void, uint32_t>,
		.build_capacity = ShipProbeUnused<void, uint32_t, uint32_t>, .set_tile = ShipProbeUnused<void, uint32_t, uint32_t>,
		.set_x = ShipProbeUnused<void, uint32_t, int32_t>, .set_y = ShipProbeUnused<void, uint32_t, int32_t>,
		.set_z = ShipProbeUnused<void, uint32_t, int32_t>, .set_direction = ShipProbeUnused<void, uint32_t, uint8_t>,
		.set_speed = ShipProbeUnused<void, uint32_t, uint16_t>, .set_tick = ShipProbeUnused<void, uint32_t, uint8_t>,
		.set_running = ShipProbeUnused<void, uint32_t, uint8_t>, .set_day = ShipProbeUnused<void, uint32_t, uint8_t>,
		.set_order_time = ShipProbeUnused<void, uint32_t, int32_t>, .set_progress = ShipProbeUnused<void, uint32_t, uint8_t>,
		.set_last_station = ShipProbeUnused<void, uint32_t, uint16_t>, .set_hidden = ShipProbeUnused<void, uint32_t, bool>,
		.set_max_speed = ShipProbeUnused<void, uint32_t, uint16_t>, .set_cargo_age = ShipProbeUnused<void, uint32_t, uint16_t>,
		.set_dest = ShipProbeUnused<void, uint32_t, uint32_t>, .tile = ShipProbeUnused<uint32_t, uint32_t>,
		.dest = ShipProbeUnused<uint32_t, uint32_t>, .x = ShipProbeUnused<uint32_t, uint32_t>,
		.y = ShipProbeUnused<uint32_t, uint32_t>, .z = ShipProbeUnused<uint32_t, uint32_t>,
		.direction = ShipProbeUnused<uint32_t, uint32_t>, .speed = ShipProbeUnused<uint32_t, uint32_t>,
		.tick = ShipProbeUnused<uint32_t, uint32_t>, .running = ShipProbeUnused<uint32_t, uint32_t>,
		.day = ShipProbeUnused<uint32_t, uint32_t>, .order_time = ShipProbeUnused<uint32_t, uint32_t>,
		.progress = ShipProbeUnused<uint32_t, uint32_t>, .status = ShipProbeUnused<uint32_t, uint32_t>,
		.owner = ShipProbeUnused<uint32_t, uint32_t>, .last_station = ShipProbeUnused<uint32_t, uint32_t>,
		.order_destination = ShipProbeUnused<uint32_t, uint32_t>, .order_type = ShipProbeUnused<uint32_t, uint32_t>,
		.order_max_speed = ShipProbeUnused<uint32_t, uint32_t>, .acceleration = ShipProbeUnused<uint32_t, uint32_t>,
		.max_speed = ShipProbeUnused<uint32_t, uint32_t>, .state_owner = ShipProbeUnused<OpenTTDShipState *, uint32_t>,
		.patch = ShipProbeUnused<OpenTTDWaterPatch, uint32_t>, .neighbours = ShipProbeUnused<size_t, OpenTTDWaterPatch, OpenTTDWaterPatch *>,
		.next_depot = ShipProbeUnused<bool, uint32_t, OpenTTDShipDepot *>
	};
	ship_probe = {};
	ship_probe.owner = state.get();
	ship_probe.leaves = &leaves;
	ship_probe.services = OpenTTDFixtureSharedServices(nullptr, ShipProbeUnused<uint32_t, void *>, ShipProbeUnused<uint32_t, int32_t, int32_t, uint32_t *>);
	leaves.state_owner = [](uint32_t) noexcept { return ship_probe.owner; };
	leaves.x = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.x); };
	leaves.y = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.y); };
	leaves.dest = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.dest); };
	leaves.order_time = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.order_time); };
	leaves.tick = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.tick); };
	leaves.running = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.running); };
	leaves.direction = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.direction); };
	leaves.speed = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.speed); };
	leaves.set_tick = [](uint32_t, uint8_t value) noexcept { ship_probe.tick = value; };
	leaves.set_running = [](uint32_t, uint8_t value) noexcept { ship_probe.running = value; };
	leaves.set_order_time = [](uint32_t, int32_t value) noexcept { ship_probe.order_time = static_cast<uint32_t>(value); };
	leaves.set_direction = [](uint32_t, uint8_t value) noexcept { ship_probe.direction = value; };
	leaves.set_speed = [](uint32_t, uint16_t value) noexcept { ship_probe.speed = value; };
	leaves.set_dest = [](uint32_t, uint32_t value) noexcept { ship_probe.dest = value; };
	leaves.path_clear = [](uint32_t) noexcept { ++ship_probe.paths; };
	leaves.position = [](uint32_t) noexcept { ++ship_probe.positions; };
	leaves.breakdown = [](uint32_t) noexcept {
		CHECK(ship_probe.stage++ == 0);
		CHECK(ship_probe.tick == 0 && ship_probe.running == 0 && ship_probe.order_time == 0);
		return false;
	};
	leaves.process_orders = [](uint32_t) noexcept { CHECK(ship_probe.stage++ == 1); return true; };
	leaves.yapf_reverse = [](uint32_t, bool trackdir) noexcept {
		CHECK(ship_probe.stage++ == 2 && !trackdir);
		return OpenTTDShipReverseResult{true, 255};
	};
	leaves.viewport = [](uint32_t id, bool update_delta, bool force) noexcept {
		CHECK(ship_probe.stage++ == 3 && update_delta && force);
		CHECK(ship_probe.direction == 5 && ship_probe.speed == 0 && ship_probe.paths == 1 && ship_probe.positions == 1);
		CHECK(openttd_rust_ship_get_rotation_x(ship_probe.owner) == static_cast<int16_t>(ship_probe.x));
		CHECK(openttd_rust_ship_get_rotation_y(ship_probe.owner) == static_cast<int16_t>(ship_probe.y));
		/* The outer call holds no Rust reference to this owner during nested mutation. */
		openttd_rust_ship_destination(id, 17, ship_probe.leaves, &ship_probe.services);
		CHECK(ship_probe.dest == 17 && ship_probe.paths == 2);
		openttd_rust_ship_set_rotation(ship_probe.owner, 3);
	};
	CHECK(openttd_rust_ship_tick(4, &leaves, &ship_probe.services));
	CHECK(ship_probe.stage == 4 && openttd_rust_ship_get_rotation(state.get()) == 3);
	CHECK(!openttd_rust_ship_find_depot(4, 80, &leaves, &ship_probe.services).valid);
	leaves.owner = [](uint32_t) noexcept { return uint32_t{3}; };
	leaves.next_depot = [](uint32_t first, OpenTTDShipDepot *out) noexcept {
		if (first != 0) return false;
		*out = {0x1234, 0x80000017, 3, 1};
		return true;
	};
	leaves.dist_square = [](uint32_t, uint32_t) noexcept { return uint32_t{49}; };
	auto depot = openttd_rust_ship_find_depot(4, 80, &leaves, &ship_probe.services);
	CHECK(depot.valid && depot.tile == 0x80000017 && depot.id == 0x1234);
	state.reset(openttd_rust_ship_state_new());
	CHECK(openttd_rust_ship_get_rotation_x(state.get()) == 0 && openttd_rust_ship_get_rotation(state.get()) == 255);
	std::printf("ship scalar widths, native transient narrowing, direct order and nested owner mutation passed\n");
}
/* Direct callbacks reenter with the same live scalar allocation. Deletion models
 * Aircraft::PreDestructor releasing airport blocks before freeing that owner. */
static struct AircraftBoundaryProbe {
	OpenTTDVehicleServices vehicle_services;
	OpenTTDSharedServices shared_services;
	OpenTTDAircraftLeaves leaves;
	OpenTTDAircraftVehicle vehicle;
	uint64_t blocks = UINT64_MAX;
	unsigned orders = 0, deleted = 0, ticks = 0, rotor_reads = 0, moves = 0;
	int32_t order_time = 0, x = 0, y = 0;
	uint8_t order_kind = 0, direction = 0, progress = 0;
	uint16_t maximum_speed = 0, cargo_age = 0;
	bool live = true;
} aircraft_probe;
static void AircraftDirectBoundary()
{
	auto &p = aircraft_probe;
	p.leaves = AircraftTestLeaves(p.vehicle_services, p.shared_services);
	CHECK(p.shared_services.is_water_tile != nullptr);
	auto &vehicle = p.vehicle_services;
	auto &shared = p.shared_services;
	auto &leaves = p.leaves;
	vehicle.status = [](OpenTTDVehicle *) noexcept { CHECK(aircraft_probe.live); return uint8_t{0}; };
	vehicle.order_type = [](OpenTTDVehicle *) noexcept { return uint8_t{3}; };
	vehicle.breakdown = [](OpenTTDVehicle *) noexcept { return uint8_t{0}; };
	vehicle.loading = [](OpenTTDVehicle *, bool) noexcept {};
	leaves.process_orders = [](OpenTTDAircraftVehicle v) noexcept {
		CHECK(aircraft_probe.live);
		aircraft_probe.orders++;
		openttd_rust_aircraft_cache(&aircraft_probe.leaves, v, true);
		v.state->pos = 19;
	};
	vehicle.property = [](OpenTTDVehicle *, uint8_t property, int32_t fallback) noexcept { CHECK(property == 0x0C && fallback == 0); return int32_t{200}; };
	leaves.cargo_age_property = [](OpenTTDAircraftVehicle) noexcept { return uint32_t{71}; };
	leaves.range_property = [](OpenTTDAircraftVehicle) noexcept { return uint32_t{41}; };
	vehicle.write_max_speed = [](OpenTTDVehicle *, uint16_t speed) noexcept { aircraft_probe.maximum_speed = speed; };
	vehicle.write_cargo_age = [](OpenTTDVehicle *, uint16_t age) noexcept { aircraft_probe.cargo_age = age; };
	leaves.next = [](OpenTTDAircraftVehicle v) noexcept { return v; };
	p.vehicle = {reinterpret_cast<OpenTTDVehicle *>(&p), openttd_rust_aircraft_state_new(), 17};
	CHECK(openttd_rust_aircraft_event(&leaves, p.vehicle, false));
	CHECK(p.orders == 1 && p.vehicle.state->pos == 19 && p.vehicle.state->cached_max_range == 41 && p.vehicle.state->cached_max_range_sqr == 1681);
	CHECK(p.maximum_speed == 2560 && p.cargo_age == 71);
	vehicle.status = [](OpenTTDVehicle *) noexcept { CHECK(aircraft_probe.live); return uint8_t{128}; };
	leaves.station = [](uint16_t) noexcept -> const OpenTTDStation * { CHECK(aircraft_probe.live); return reinterpret_cast<const OpenTTDStation *>(&aircraft_probe); };
	leaves.airport_tile = [](const OpenTTDStation *) noexcept { return uint32_t{7}; };
	leaves.airport_fta = [](const OpenTTDStation *s) noexcept -> const void * { return s; };
	leaves.airport_blocks = [](const OpenTTDStation *) noexcept { CHECK(aircraft_probe.live); return &aircraft_probe.blocks; };
	leaves.node = [](const void *ap, uint8_t) noexcept -> const void * { return ap; };
	leaves.fta = [](const void *) noexcept { return OpenTTDAircraftNode{nullptr, uint64_t{1} << 9, 0, 0, 0}; };
	leaves.layout_blocks = [](const void *, uint8_t) noexcept { return uint64_t{1} << 9; };
	vehicle.destroy = [](OpenTTDVehicle * handle) noexcept {
		CHECK(handle == aircraft_probe.vehicle.handle);
		auto v = aircraft_probe.vehicle;
		CHECK(aircraft_probe.live && v.state->crashed_counter == 10002);
		openttd_rust_aircraft_release_blocks(&aircraft_probe.leaves, v);
		CHECK((aircraft_probe.blocks & ((uint64_t{1} << 8) | (uint64_t{1} << 9) | (uint64_t{1} << 29))) == 0);
		openttd_rust_aircraft_state_destroy(v.state);
		aircraft_probe.live = false;
		aircraft_probe.deleted++;
	};
	p.vehicle.state->targetairport = 7;
	p.vehicle.state->crashed_counter = 9999;
	CHECK(!openttd_rust_aircraft_event(&leaves, p.vehicle, false));
	CHECK(!p.live && p.deleted == 1);
	/* A disaster has its own flags and no Aircraft state, even with subtype zero. */
	shared.map_log_x = []() noexcept { return uint32_t{6}; };
	shared.map_max_x = shared.map_max_y = []() noexcept { return uint32_t{63}; };
	vehicle.x = vehicle.y = vehicle.z = [](OpenTTDVehicle *) noexcept { CHECK(aircraft_probe.live == false); return int32_t{0}; };
	shared.tile_height = [](uint32_t) noexcept { return uint32_t{0}; };
	vehicle.xy = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleXY{0, 0}; };
	vehicle.subtype = [](OpenTTDVehicle *) noexcept { return uint8_t{0}; };
	vehicle.vehicle_type = [](OpenTTDVehicle *) noexcept { return uint8_t{4}; };
	vehicle.direction = [](OpenTTDVehicle *) noexcept { return uint8_t{4}; };
	vehicle.altitude_inputs = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleAltitudeInputs{1, 4}; };
	uint8_t disaster_flags = 0;
	int32_t min = 0, max = 0;
	OpenTTDAircraftVehicle disaster{reinterpret_cast<OpenTTDVehicle *>(&p), nullptr, 81};
	openttd_rust_aircraft_flight_bounds(&leaves, disaster, &min, &max);
	CHECK(min == 30 && max == 270);
	CHECK(openttd_rust_aircraft_flight_level(&leaves, disaster, &disaster_flags, false) == 1 && disaster_flags == 4);
	CHECK(openttd_rust_aircraft_flight_level(&leaves, disaster, &disaster_flags, true) == 2 && disaster_flags == 4);
	std::printf("aircraft direct nested mutation, live block release/deletion and disaster flight identity passed\n");
}
/* Concrete gaps: native callbacks can replace order fields/rotor counters and
 * direction/progress after the pre-speed observation. This probe forbids stale
 * snapshots at those original observation phases, using the production Rust entry. */
static void AircraftPhaseBoundary()
{
	auto &p = aircraft_probe;
	p = {};
	p.leaves = AircraftTestLeaves(p.vehicle_services, p.shared_services);
	auto &v = p.vehicle_services;
	auto &s = p.shared_services;
	auto &leaves = p.leaves;
	std::unique_ptr<OpenTTDAircraftState, decltype(&openttd_rust_aircraft_state_destroy)> state{openttd_rust_aircraft_state_new(), openttd_rust_aircraft_state_destroy};
	p.vehicle = {reinterpret_cast<OpenTTDVehicle *>(&p), state.get(), 17};
	v.subtype = [](OpenTTDVehicle *) noexcept { return uint8_t{0}; };
	v.tick_status = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleTickStatus{7, 0}; };
	v.write_tick = [](OpenTTDVehicle *, uint8_t tick) noexcept { CHECK(tick == 8); aircraft_probe.ticks++; };
	v.running = [](OpenTTDVehicle *) noexcept { return uint8_t{0}; };
	v.write_running = [](OpenTTDVehicle *, uint8_t running) noexcept { CHECK(running == 1); };
	leaves.next = [](OpenTTDAircraftVehicle u) noexcept {
		if (u.id == 18) { aircraft_probe.order_time = 40; aircraft_probe.rotor_reads++; }
		u.id++; u.handle = reinterpret_cast<OpenTTDVehicle *>(uintptr_t{u.id}); return u;
	};
	v.status = [](OpenTTDVehicle *u) noexcept { return static_cast<uint8_t>(u == aircraft_probe.vehicle.handle ? 0 : 1); };
	v.order_time = [](OpenTTDVehicle *) noexcept { CHECK(aircraft_probe.rotor_reads == 1); return aircraft_probe.order_time; };
	v.write_order_time = [](OpenTTDVehicle *, int32_t time) noexcept { CHECK(time == 41); aircraft_probe.order_time = time; };
	v.breakdown = [](OpenTTDVehicle *) noexcept { return uint8_t{0}; };
	leaves.process_orders = [](OpenTTDAircraftVehicle) noexcept { aircraft_probe.orders++; aircraft_probe.order_kind = 3; };
	v.loading = [](OpenTTDVehicle *, bool) noexcept { aircraft_probe.order_kind = 4; };
	v.order_type = [](OpenTTDVehicle *) noexcept { return aircraft_probe.order_kind; };
	CHECK(openttd_rust_aircraft_tick(&leaves, p.vehicle));
	CHECK(p.ticks == 1 && p.orders == 2 && p.rotor_reads == 1 && p.order_time == 41);

	/* A speed change dirties the native widget before direction/progress are
	 * read. Their mutation yields exactly one move; a stale snapshot yields none. */
	v.subtype = [](OpenTTDVehicle *) noexcept { return uint8_t{2}; };
	v.status = [](OpenTTDVehicle *) noexcept { return uint8_t{0}; };
	leaves.process_orders = [](OpenTTDAircraftVehicle) noexcept {};
	v.loading = [](OpenTTDVehicle *, bool) noexcept {};
	v.order_type = [](OpenTTDVehicle *) noexcept { return uint8_t{1}; };
	p.vehicle.state->state = 14; p.vehicle.state->targetairport = 7;
	leaves.station = [](uint16_t station) noexcept { CHECK(station == 7); return reinterpret_cast<const OpenTTDStation *>(&aircraft_probe); };
	leaves.geometry = [](const OpenTTDStation *) noexcept { return OpenTTDAircraftGeometry{0, 0, 8, 8, true}; };
	leaves.airport_fta = [](const OpenTTDStation *) noexcept -> const void * { return &aircraft_probe; };
	leaves.moving = [](const void *, uint8_t) noexcept { return OpenTTDAircraftMoving{16, 0, 16, 0}; };
	v.xy = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleXY{aircraft_probe.x, aircraft_probe.y}; };
	s.map_log_x = []() noexcept { return uint32_t{6}; };
	s.map_max_x = s.map_max_y = []() noexcept { return uint32_t{63}; };
	s.plane_speed = []() noexcept { return uint8_t{1}; };
	v.speed_inputs = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleSpeedInputs{0, 100, 8, 0, 0}; };
	v.write_subspeed = [](OpenTTDVehicle *, uint8_t sub) noexcept { CHECK(sub == 104); };
	v.write_speed = [](OpenTTDVehicle *, uint16_t speed) noexcept { CHECK(speed == 2); aircraft_probe.maximum_speed = speed; };
	v.start_stop_dirty = [](OpenTTDVehicle *) noexcept { aircraft_probe.direction = 1; aircraft_probe.progress = 255; };
	v.direction_progress = [](OpenTTDVehicle *) noexcept { CHECK(aircraft_probe.progress == 255); return OpenTTDVehicleDirectionProgress{aircraft_probe.direction, aircraft_probe.progress}; };
	v.write_progress = [](OpenTTDVehicle *, uint8_t progress) noexcept { CHECK(progress == 1); aircraft_probe.progress = progress; };
	leaves.direction_towards = [](OpenTTDAircraftVehicle, int32_t, int32_t) noexcept { return uint8_t{1}; };
	v.direction = [](OpenTTDVehicle *) noexcept { return aircraft_probe.direction; };
	v.new_position = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleNewPosition{aircraft_probe.x + 1, aircraft_probe.y, 0}; };
	v.write_tile = [](OpenTTDVehicle *, uint32_t) noexcept {};
	v.z = [](OpenTTDVehicle *) noexcept { return int32_t{1}; };
	v.write_coordinates = [](OpenTTDVehicle *, int32_t x, int32_t y, int32_t z) noexcept { CHECK(z == 1); aircraft_probe.x = x; aircraft_probe.y = y; };
	leaves.update_position = [](OpenTTDAircraftVehicle) noexcept { aircraft_probe.moves++; };
	leaves.next = [](OpenTTDAircraftVehicle u) noexcept { if (u.id != 17) return OpenTTDAircraftVehicle{nullptr, nullptr, 1048575}; u.id = 18; u.handle = reinterpret_cast<OpenTTDVehicle *>(uintptr_t{18}); return u; };
	v.write_x = [](OpenTTDVehicle *, int32_t x) noexcept { CHECK(x == 1); };
	v.write_y = [](OpenTTDVehicle *, int32_t y) noexcept { CHECK(y == 0); };
	v.y = [](OpenTTDVehicle *) noexcept { return int32_t{0}; };
	v.write_z = [](OpenTTDVehicle *, int32_t z) noexcept { CHECK(z == 0); };
	s.slope = [](int32_t, int32_t) noexcept { return int32_t{0}; };
	leaves.copy_sprite = [](OpenTTDAircraftVehicle, OpenTTDAircraftVehicle) noexcept {};
	leaves.position_viewport = [](OpenTTDAircraftVehicle) noexcept {};
	CHECK(openttd_rust_aircraft_event(&leaves, p.vehicle, false));
	CHECK(p.moves == 1 && p.x == 1 && p.y == 0 && p.progress == 1);
	std::printf("aircraft rotor/order/loading phases and post-dirty motion mutation passed\n");
}

static void AircraftOwnership()
{
	std::unique_ptr<OpenTTDAircraftState, decltype(&openttd_rust_aircraft_state_destroy)> state{openttd_rust_aircraft_state_new(), openttd_rust_aircraft_state_destroy};
	std::unique_ptr<uint64_t, decltype(&openttd_rust_airport_blocks_destroy)> airport{openttd_rust_airport_blocks_new(), openttd_rust_airport_blocks_destroy};
	CHECK(state->targetairport == 65535 && state->last_direction == 255 && state->cached_max_range_sqr == 0 && *airport == 0);
	uint8_t &position = state->pos; uint16_t &counter = state->crashed_counter; uint64_t &blocks = *airport;
	position = 255; counter = 65535; blocks = UINT64_MAX; state->cached_max_range = 65535; state->cached_max_range_sqr = UINT32_MAX;
	CHECK(state->pos == 255 && state->crashed_counter == 65535 && *airport == UINT64_MAX && state->cached_max_range == 65535 && state->cached_max_range_sqr == UINT32_MAX);
	std::printf("aircraft_airport_canonical_scalar_lifetime passed\n");
}

/* Explicit trivial Date and actual VehicleType carriers and aggregate return/write call ABI. Opaque
 * shells are never dereferenced; the game target checks real member stores. */
static void VehicleMaintenanceBoundary()
{
	static size_t dirty;
	static size_t resets;
	static size_t values;
	dirty = resets = values = 0;
	auto &maintenance = _vehicle_maintenance_probe;
	maintenance.renewal = AircraftUnexpectedService<decltype(maintenance.renewal)>::Call;
	maintenance.reset = AircraftUnexpectedService<decltype(maintenance.reset)>::Call;
	maintenance.reset_write = AircraftUnexpectedService<decltype(maintenance.reset_write)>::Call;
	maintenance.has_engine = AircraftUnexpectedService<decltype(maintenance.has_engine)>::Call;
	maintenance.interval = AircraftUnexpectedService<decltype(maintenance.interval)>::Call;
	maintenance.engine_reliability = AircraftUnexpectedService<decltype(maintenance.engine_reliability)>::Call;
	maintenance.budget = AircraftUnexpectedService<decltype(maintenance.budget)>::Call;
	maintenance.replacement = AircraftUnexpectedService<decltype(maintenance.replacement)>::Call;
	maintenance.replacement_available = AircraftUnexpectedService<decltype(maintenance.replacement_available)>::Call;
	maintenance.replacement_cost = AircraftUnexpectedService<decltype(maintenance.replacement_cost)>::Call;
	maintenance.automatic = AircraftUnexpectedService<decltype(maintenance.automatic)>::Call;
	maintenance.depot_service = AircraftUnexpectedService<decltype(maintenance.depot_service)>::Call;
	maintenance.check_breakdown = AircraftUnexpectedService<decltype(maintenance.check_breakdown)>::Call;
	maintenance.breakdown_guard = AircraftUnexpectedService<decltype(maintenance.breakdown_guard)>::Call;
	maintenance.breakdown_trigger = AircraftUnexpectedService<decltype(maintenance.breakdown_trigger)>::Call;
	maintenance.chance_write = AircraftUnexpectedService<decltype(maintenance.chance_write)>::Call;
	maintenance.trigger = AircraftUnexpectedService<decltype(maintenance.trigger)>::Call;
	maintenance.breakdown_counter = AircraftUnexpectedService<decltype(maintenance.breakdown_counter)>::Call;
	maintenance.breakdown_probe = [](OpenTTDVehicle *) noexcept -> OpenTTDVehicleMaintenanceBreakdownProbe { return {.counter = 0, .status = 0}; };
	maintenance.breakdown_start = AircraftUnexpectedService<decltype(maintenance.breakdown_start)>::Call;
	maintenance.breakdown_start_write = AircraftUnexpectedService<decltype(maintenance.breakdown_start_write)>::Call;
	maintenance.breakdown_active = AircraftUnexpectedService<decltype(maintenance.breakdown_active)>::Call;
	maintenance.breakdown_countdown = AircraftUnexpectedService<decltype(maintenance.breakdown_countdown)>::Call;
	maintenance.breakdown_sound_state = AircraftUnexpectedService<decltype(maintenance.breakdown_sound_state)>::Call;
	maintenance.breakdown_delay_write = AircraftUnexpectedService<decltype(maintenance.breakdown_delay_write)>::Call;
	maintenance.breakdown_sound = AircraftUnexpectedService<decltype(maintenance.breakdown_sound)>::Call;
	maintenance.smoke_disabled = AircraftUnexpectedService<decltype(maintenance.smoke_disabled)>::Call;
	maintenance.smoke = AircraftUnexpectedService<decltype(maintenance.smoke)>::Call;
	maintenance.smoke_animation = AircraftUnexpectedService<decltype(maintenance.smoke_animation)>::Call;
	maintenance.aircraft_broken = AircraftUnexpectedService<decltype(maintenance.aircraft_broken)>::Call;
	maintenance.mark_dirty = AircraftUnexpectedService<decltype(maintenance.mark_dirty)>::Call;
	maintenance.view_dirty = AircraftUnexpectedService<decltype(maintenance.view_dirty)>::Call;
	maintenance.details_dirty = AircraftUnexpectedService<decltype(maintenance.details_dirty)>::Call;
	maintenance.age = AircraftUnexpectedService<decltype(maintenance.age)>::Call;
	maintenance.age_write = AircraftUnexpectedService<decltype(maintenance.age_write)>::Call;
	maintenance.decay_write = AircraftUnexpectedService<decltype(maintenance.decay_write)>::Call;
	maintenance.warning = AircraftUnexpectedService<decltype(maintenance.warning)>::Call;
	maintenance.warning_company = AircraftUnexpectedService<decltype(maintenance.warning_company)>::Call;
	maintenance.engine_available_any = AircraftUnexpectedService<decltype(maintenance.engine_available_any)>::Call;
	maintenance.replacement_active = AircraftUnexpectedService<decltype(maintenance.replacement_active)>::Call;
	maintenance.old_news = AircraftUnexpectedService<decltype(maintenance.old_news)>::Call;
	maintenance.economy_age = AircraftUnexpectedService<decltype(maintenance.economy_age)>::Call;
	maintenance.economy_age_write = AircraftUnexpectedService<decltype(maintenance.economy_age_write)>::Call;
	maintenance.reached_min_age = AircraftUnexpectedService<decltype(maintenance.reached_min_age)>::Call;
	maintenance.value = AircraftUnexpectedService<decltype(maintenance.value)>::Call;
	maintenance.value_write = AircraftUnexpectedService<decltype(maintenance.value_write)>::Call;
	maintenance.details_dirty = [](OpenTTDVehicle *) noexcept { dirty++; };
	maintenance.reset = [](OpenTTDVehicle *) noexcept {
		return OpenTTDVehicleMaintenanceReset{.economy_date = OpenTTDEconomyDate{0x1020304}, .calendar_date = OpenTTDCalendarDate{-0x12345}, .reliability = 0x9876, .chance = 12, .breakdowns = 2};
	};
	maintenance.reset_write = [](OpenTTDVehicle *, OpenTTDVehicleMaintenanceResetWrite value) noexcept {
		CHECK(value.economy_date.value == 0x1020304);
		CHECK(value.calendar_date.value == -0x12345);
		CHECK(value.reliability == 0x9876 && value.chance == 3 && value.since == 0);
		resets++;
	};
	maintenance.value = [](OpenTTDVehicle *) noexcept { return int64_t{-257}; };
	maintenance.value_write = [](OpenTTDVehicle *, int64_t value) noexcept { CHECK(value == -255); values++; };
	maintenance.renewal = [](OpenTTDVehicle *, const Company *) noexcept {
		return OpenTTDVehicleMaintenanceRenewal{.age = OpenTTDCalendarDate{100}, .max_age = OpenTTDCalendarDate{100}, .months = -4, .enabled = false, .kind = static_cast<VehicleType>(254), .subtype = 0};
	};
	const auto shared = OpenTTDFixtureSharedServices(nullptr, TreeRandom, EffectTestIndustry);
	RoadTestTables tables(&shared);
	auto &base = tables.vehicle;
	base.maintenance = &maintenance;
	base.next = [](OpenTTDVehicle *) noexcept -> OpenTTDVehicle * { return nullptr; };
	uint8_t opaque;
	auto *shell = reinterpret_cast<OpenTTDVehicle *>(&opaque);
	openttd_rust_vehicle_service_depot(shell, &base);
	openttd_rust_vehicle_decrease_value(shell, &base);
	CHECK(openttd_rust_vehicle_needs_renew(shell, &base, nullptr, false));
	CHECK(!openttd_rust_vehicle_needs_renew(shell, &base, nullptr, true));
	CHECK(dirty == 2 && resets == 1 && values == 1);
}

/* Actual native storage type is checked by the game-side shell probe; this
 * standalone ABI target also exercises const BaseBitSet member identity and
 * native size_t/pointer cursor calls on every platform. */
static void VehicleSchedulerBoundary()
{
	using Status = decltype(std::declval<Vehicle>().vehstatus);
	static_assert(std::is_same_v<Status::BaseType, uint8_t>);
	static_assert(std::is_same_v<decltype(RustVehicleStatusAddress(nullptr)), const uint8_t *>);
	Status status;
	const uint8_t *address = status.base_address();
	static_assert(std::is_same_v<decltype(status.base_address()), const uint8_t *>);
	CHECK(address == status.base_address());
	status.Set(VehState::Hidden);
	CHECK(*address == status.base());
	status.Set(VehState::Crashed);
	CHECK(*address == 129);
	status.Reset(VehState::Hidden);
	CHECK(*address == 128);
	/* Opaque shells are never dereferenced: this target tests the pointer/size_t
	 * call ABI; the game target compares real pool mutations to PoolIterator. */
	static Vehicle *slots[5];
	static size_t live;
	static size_t size;
	std::array<uint8_t, 2> opaque{};
	slots[0] = reinterpret_cast<Vehicle *>(&opaque[0]);
	slots[1] = nullptr;
	slots[2] = reinterpret_cast<Vehicle *>(&opaque[1]);
	slots[3] = slots[4] = nullptr;
	live = size = 2;
	OpenTTDVehicleSchedulerServices services{};
	services.tick = AircraftUnexpectedService<decltype(services.tick)>::Call;
	services.calendar_day = AircraftUnexpectedService<decltype(services.calendar_day)>::Call;
	services.economy_day = AircraftUnexpectedService<decltype(services.economy_day)>::Call;
	services.day_callback_read = AircraftUnexpectedService<decltype(services.day_callback_read)>::Call;
	services.day_callback = AircraftUnexpectedService<decltype(services.day_callback)>::Call;
	services.randomise_callback = AircraftUnexpectedService<decltype(services.randomise_callback)>::Call;
	services.first_dirty = AircraftUnexpectedService<decltype(services.first_dirty)>::Call;
	services.unknown_day_callback = AircraftUnexpectedService<decltype(services.unknown_day_callback)>::Call;
	services.cargo_owner = AircraftUnexpectedService<decltype(services.cargo_owner)>::Call;
	services.running_sound = AircraftUnexpectedService<decltype(services.running_sound)>::Call;
	services.periodic_sound = AircraftUnexpectedService<decltype(services.periodic_sound)>::Call;
	services.next_station = AircraftUnexpectedService<decltype(services.next_station)>::Call;
	services.with_loading_timer = AircraftUnexpectedService<decltype(services.with_loading_timer)>::Call;
	services.reset_accumulators = AircraftUnexpectedService<decltype(services.reset_accumulators)>::Call;
	services.pool = []() noexcept { return OpenTTDVehiclePoolCursor{.slots = slots, .slots_len = size, .first_unused = &live}; };
	auto cursor = services.pool();
	size_t from = 0;
	CHECK(openttd_rust_vehicle_cursor_next(&services, &cursor, &from) == reinterpret_cast<OpenTTDVehicle *>(slots[0]));
	CHECK(from == 1);
	live = 3;
	size = 5;
	CHECK(openttd_rust_vehicle_cursor_next(&services, &cursor, &from) == reinterpret_cast<OpenTTDVehicle *>(slots[2]));
	CHECK(cursor.slots_len == 5);
	CHECK(cursor.first_unused == &live);
	CHECK(from == 3);
	CHECK(openttd_rust_vehicle_cursor_next(&services, &cursor, &from) == nullptr);
}
int main()
{
	Layouts();
	VehicleSchedulerBoundary();
	VehicleMaintenanceBoundary();
	ShipBoundary();
	RoadBoundary();
	RoadServiceBoundary();
	WaterProbe::Run([](bool result) { CHECK(result); });
	ShipYapfProbe::Run([](bool result) { CHECK(result); });
	CheckEffectProtocol([](bool result) { CHECK(result); });
	LinkGraphJob();
	Trees();
	Disasters();
	AircraftOwnership();
	AircraftDirectBoundary();
	AircraftPhaseBoundary();
	/* Opaque owner, native size_t, immutable byte borrow and complete UTF-8 output. */
	auto *townname = openttd_rust_townname_generate(1, UINT32_MAX);
	size_t name_length;
	const char *name = openttd_rust_townname_data(townname, &name_length);
	CHECK(name_length == 8 && std::memcmp(name, "Alen\xc3\xa7on", name_length) == 0);
	openttd_rust_townname_destroy(townname);
	Calls();
	Encoded();
	History();
	Math();
	CryptoPrimitives();
	Blake2b();
	X25519();
	StationCargo();
	PacketState();
	StringValidation();
	ScriptListControl();
	Locale();
	std::printf("ABI audit passed\n");
}
