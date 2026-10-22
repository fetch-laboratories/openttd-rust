/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file town_ffi.h Canonical town state and synchronous typed services. */
#ifndef RUST_TOWN_FFI_H
#define RUST_TOWN_FFI_H
#include "service_defaults.hpp"
#include "services_ffi.h"
#include "byte_span_ffi.h"
#include "station_activity_ffi.h"
#include <cstdint>
#include "command_cost_ffi.h"
#include <cstddef>
struct OpenTTDTownState;
struct OpenTTDRoadCommandContext;
struct Town;
/** Pool IDs cross as their uint16_t wire value; native PoolID classes stay native. */
struct OpenTTDTownRef { Town *shell; OpenTTDTownState *state; uint16_t id; };

struct OpenTTDHouseServices;
struct OpenTTDTownGrowthFields { uint16_t counter, rate; uint8_t funding, road, flags; };
struct OpenTTDTownReceived { uint16_t old_max, new_max, old_act, new_act; };
struct OpenTTDTownSimulationFields {
	uint64_t valid_history;
	uint32_t xy, num_houses, population, radius[5], goal[6];
	int16_t ratings[15];
	OpenTTDTownReceived received[6];
	uint16_t noise_reached, time_until_rebuild, statues, have_ratings;
	uint8_t unwanted[15], exclusivity, exclusive_counter;
	bool larger_town;
	uint8_t layout;
	uint32_t townnamegrfid;
	uint16_t townnametype;
	uint32_t townnameparts;
};
struct OpenTTDTownSuppliedHistory { uint32_t production, transported; };
struct OpenTTDTownSupplied { uint8_t cargo; OpenTTDTownSuppliedHistory history[61]; };
struct OpenTTDTownSuppliedEntry { OpenTTDTownSupplied *entry; bool created; };
struct OpenTTDTownSlots { void *data; size_t size; };
using OpenTTDTownStationActivity = OpenTTDStationActivity;
struct OpenTTDTownCommandResult { int64_t cost; bool success; };
struct OpenTTDTownRoadChoice { int32_t introduction_date; uint16_t max_speed; bool town_build; };
struct OpenTTDTownHouseAvailability { uint16_t zones; uint16_t override_id; uint16_t class_id; uint8_t probability; bool enabled; };
struct OpenTTDTownHousePlacement { int32_t min_year; int32_t max_year; };
struct OpenTTDTownAuthorityServices {
	uint8_t (*council_tolerance)() noexcept OPENTTD_SERVICE_DEFAULT(council_tolerance);
	bool (*bulldozer)() noexcept OPENTTD_SERVICE_DEFAULT(bulldozer);
	uint8_t (*authority_distance)() noexcept OPENTTD_SERVICE_DEFAULT(authority_distance);
	uint16_t (*town_index)(const OpenTTDTownHandle *) noexcept OPENTTD_SERVICE_DEFAULT(town_index);
	uint32_t refusal;
};
struct OpenTTDTownLeaves {
	uint32_t (*map_log_x)() noexcept OPENTTD_SERVICE_DEFAULT(map_log_x);
	uint8_t (*growth_rate)() noexcept OPENTTD_SERVICE_DEFAULT(growth_rate);
	uint8_t (*snow_line)() noexcept OPENTTD_SERVICE_DEFAULT(snow_line);
	OpenTTDEnvironmentServices environment;
	bool (*generating_world)() noexcept OPENTTD_SERVICE_DEFAULT(generating_world);
	bool (*allow_crossings)() noexcept OPENTTD_SERVICE_DEFAULT(allow_crossings);
	uint8_t (*climate)() noexcept OPENTTD_SERVICE_DEFAULT(climate);
	uint8_t (*highest_snow_line)() noexcept OPENTTD_SERVICE_DEFAULT(highest_snow_line);
	int32_t (*calendar_year)() noexcept OPENTTD_SERVICE_DEFAULT(calendar_year);
	bool (*allow_roads)() noexcept OPENTTD_SERVICE_DEFAULT(allow_roads);
	uint8_t (*road_flags)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(road_flags);
	OpenTTDTownHouseAvailability (*house_availability)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(house_availability);
	OpenTTDTownHousePlacement (*house_placement)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(house_placement);
	uint8_t (*house_population)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(house_population);
	uint8_t (*house_flags)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(house_flags);
	uint8_t (*house_extra_flags)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(house_extra_flags);
	uint16_t (*house_callbacks)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(house_callbacks);
	OpenTTDTownRoadChoice (*road_info)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(road_info);
	OpenTTDTownState *(*resolve_owner)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(resolve_owner);
	void (*stations)(uint32_t, uint32_t, void *, void (*)(void *, const OpenTTDStation *, uint32_t)) noexcept OPENTTD_SERVICE_DEFAULT(stations);
	const OpenTTDStationService *(*station_activity_owner)(const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(station_activity_owner);
	uint8_t (*station_owner)(const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(station_owner);
	void (*dirty_authority)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(dirty_authority);
	void (*dirty_town)(uint32_t town) noexcept OPENTTD_SERVICE_DEFAULT(dirty_town);
	uint32_t (*foundation_slope)(uint32_t tile) noexcept OPENTTD_SERVICE_DEFAULT(foundation_slope);
	int64_t (*terraform_price)() noexcept OPENTTD_SERVICE_DEFAULT(terraform_price);
	uint32_t (*clean_road_bits)(uint32_t tile, uint32_t bits) noexcept OPENTTD_SERVICE_DEFAULT(clean_road_bits);
	void (*set_road_town)(uint32_t tile, uint32_t town) noexcept OPENTTD_SERVICE_DEFAULT(set_road_town);
	uint32_t (*exchange_company)(uint32_t company) noexcept OPENTTD_SERVICE_DEFAULT(exchange_company);
	uint16_t (*house_class)(uint16_t house) noexcept OPENTTD_SERVICE_DEFAULT(house_class);
	void (*make_house)(uint32_t tile, uint32_t town, uint32_t house, uint32_t construction) noexcept OPENTTD_SERVICE_DEFAULT(make_house);
	void (*add_animation)(uint32_t tile, bool) noexcept OPENTTD_SERVICE_DEFAULT(add_animation);
	void (*population_changed)(uint32_t town) noexcept OPENTTD_SERVICE_DEFAULT(population_changed);
	void (*nearby_stations)(uint32_t town, uint32_t tile, uint32_t size) noexcept OPENTTD_SERVICE_DEFAULT(nearby_stations);
	uint32_t (*house_count)() noexcept OPENTTD_SERVICE_DEFAULT(house_count);
	void (*update_max_pass)(uint32_t town) noexcept OPENTTD_SERVICE_DEFAULT(update_max_pass);
	uint32_t (*next_town)(uint32_t from) noexcept OPENTTD_SERVICE_DEFAULT(next_town);
	uint64_t (*road_mask)() noexcept OPENTTD_SERVICE_DEFAULT(road_mask);
	bool (*clear_land)(uint32_t tile, uint32_t execute) noexcept OPENTTD_SERVICE_DEFAULT(clear_land);
	OpenTTDTownCommandResult (*terraform_land)(uint32_t tile, uint32_t slope, uint32_t up, uint32_t execute) noexcept OPENTTD_SERVICE_DEFAULT(terraform_land);
	bool (*build_bridge)(uint32_t tile, uint32_t end, uint32_t bridge, uint32_t road_type, uint32_t execute) noexcept OPENTTD_SERVICE_DEFAULT(build_bridge);
	bool (*build_tunnel)(uint32_t tile, uint32_t road_type, uint32_t execute) noexcept OPENTTD_SERVICE_DEFAULT(build_tunnel);
	void (*clear_house_land)(uint32_t tile) noexcept OPENTTD_SERVICE_DEFAULT(clear_house_land);
	bool (*allow_construction)(uint32_t town, uint32_t tile, uint32_t house, uint32_t random) noexcept OPENTTD_SERVICE_DEFAULT(allow_construction);
	const OpenTTDHouseServices *houses;
};
/* Services are synchronous noexcept calls. No owner reference spans one. Commands
 * and bounded NewGRF resolution may reenter; ordinary gameplay does not throw.
 * Environmental failures and Rust panics abort. Save/load staging stays native. */
uint32_t OpenTTDNextTownID(uint32_t from) noexcept;
const OpenTTDTownAuthorityServices &GetRustTownAuthorityServices() noexcept;

extern "C" {
OpenTTDByteSpan openttd_rust_town_text(const OpenTTDTownState *);
void openttd_rust_town_set_text(OpenTTDTownState *, OpenTTDByteSpan);
OpenTTDByteSpan openttd_rust_town_name(const OpenTTDTownState *);
void openttd_rust_town_set_name(OpenTTDTownState *, OpenTTDByteSpan);
void openttd_rust_town_authority_station(const OpenTTDTownAuthorityServices *, const OpenTTDSharedServices *, const OpenTTDCommandCostServices *, OpenTTDCommandCost *, uint32_t, bool);
void openttd_rust_town_authority_remove(const OpenTTDTownAuthorityServices *, const OpenTTDSharedServices *, const OpenTTDCommandCostServices *, OpenTTDCommandCost *, const OpenTTDTownHandle *, bool, uint8_t);
void openttd_rust_town_update_radius(OpenTTDTownState *);
uint32_t openttd_rust_town_world_population(const OpenTTDTownLeaves *);
OpenTTDTownSimulationFields *openttd_rust_town_simulation(OpenTTDTownState *);
OpenTTDTownSlots openttd_rust_town_supplied_view(OpenTTDTownState *);
OpenTTDTownSuppliedEntry openttd_rust_town_supplied_entry(OpenTTDTownState *, uint8_t);
size_t openttd_rust_town_supplied_find(const OpenTTDTownState *, uint8_t);
uint8_t openttd_rust_town_percent_transported(const OpenTTDTownSuppliedHistory *);
void openttd_rust_town_supplied_reserve(OpenTTDTownState *, size_t);
OpenTTDTownSupplied *openttd_rust_town_supplied_insert(OpenTTDTownState *, size_t, uint8_t);
OpenTTDTownSlots openttd_rust_town_id_counts(OpenTTDTownState *);
void openttd_rust_town_id_counts_resize(OpenTTDTownState *, size_t);
OpenTTDTownSlots openttd_rust_town_global_id_counts();
void openttd_rust_town_global_id_counts_resize(size_t);
OpenTTDTownSlots openttd_rust_town_class_counts(OpenTTDTownState *);
void openttd_rust_town_class_counts_resize(OpenTTDTownState *, size_t);
OpenTTDTownSlots openttd_rust_town_global_class_counts();
void openttd_rust_town_global_class_counts_resize(size_t);
OpenTTDTownSuppliedHistory openttd_rust_town_sum_history(const OpenTTDTownSuppliedHistory *, size_t);
void openttd_rust_town_rating_test(bool);
int32_t openttd_rust_town_rating(const OpenTTDTownState *, uintptr_t, uint8_t);
void openttd_rust_town_change_rating(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, OpenTTDTownState *, uint32_t, uintptr_t, uint8_t, int32_t, int32_t, bool, bool);
void openttd_rust_town_monthly_all(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, uint8_t);
OpenTTDTownState *openttd_rust_town_new();
void openttd_rust_town_destroy(OpenTTDTownState *);
OpenTTDTownGrowthFields *openttd_rust_town_growth(OpenTTDTownState *);
void openttd_rust_town_tick_all(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, const OpenTTDRoadCommandContext *);
void openttd_rust_town_expand(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, const OpenTTDRoadCommandContext *, uint32_t town, uint32_t amount, uint32_t modes);
uint32_t openttd_rust_town_road_type(const OpenTTDTownLeaves *, const OpenTTDSharedServices *);
bool openttd_rust_town_tunnel(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, const OpenTTDRoadCommandContext *, uint32_t town, uint32_t tile, uint32_t dir);
}
const OpenTTDTownLeaves &GetRustTownServices() noexcept;
#endif /* RUST_TOWN_FFI_H */
