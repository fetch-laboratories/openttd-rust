/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file town_ffi.h Canonical town state and synchronous typed services. */
#ifndef RUST_TOWN_FFI_H
#define RUST_TOWN_FFI_H
#include "services_ffi.h"
#include "byte_span_ffi.h"
#include "station_activity_ffi.h"
#include <cstdint>
#include "command_cost_ffi.h"
#include <cstddef>
struct OpenTTDTownState;
struct Town;
/** One resolved town: native shell, Rust owner and pool index. */
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
	uint8_t (*council_tolerance)() noexcept;
	bool (*bulldozer)() noexcept;
	uint8_t (*authority_distance)() noexcept;
	uint16_t (*town_index)(const OpenTTDTownHandle *) noexcept;
	uint32_t refusal;
};
struct OpenTTDTownLeaves {
	uint32_t (*map_log_x)() noexcept;
	bool (*build_on_slopes)() noexcept;
	uint8_t (*growth_rate)() noexcept;
	uint8_t (*snow_line)() noexcept;
	bool (*editor)() noexcept;
	bool (*generating_world)() noexcept;
	bool (*allow_crossings)() noexcept;
	uint8_t (*climate)() noexcept;
	uint8_t (*highest_snow_line)() noexcept;
	int32_t (*calendar_year)() noexcept;
	bool (*allow_roads)() noexcept;
	uint8_t (*road_flags)(uint8_t) noexcept;
	OpenTTDTownHouseAvailability (*house_availability)(uint16_t) noexcept;
	OpenTTDTownHousePlacement (*house_placement)(uint16_t) noexcept;
	uint8_t (*house_population)(uint16_t) noexcept;
	uint8_t (*house_flags)(uint16_t) noexcept;
	uint8_t (*house_extra_flags)(uint16_t) noexcept;
	uint16_t (*house_callbacks)(uint16_t) noexcept;
	OpenTTDTownRoadChoice (*road_info)(uint8_t) noexcept;
	OpenTTDTownRef (*resolve_town)(uint32_t) noexcept;
	void (*stations)(uint32_t, uint32_t, void *, void (*)(void *, const OpenTTDStation *, uint32_t)) noexcept;
	const OpenTTDStationService *(*station_activity_owner)(const OpenTTDStation *) noexcept;
	uint8_t (*station_owner)(const OpenTTDStation *) noexcept;
	void (*dirty_authority)(uint32_t) noexcept;
	void (*dirty_town)(uint32_t town) noexcept;
	uint32_t (*foundation_slope)(uint32_t tile) noexcept;
	int64_t (*terraform_price)() noexcept;
	uint32_t (*clean_road_bits)(uint32_t tile, uint32_t bits) noexcept;
	void (*set_road_town)(uint32_t tile, uint32_t town) noexcept;
	uint32_t (*exchange_company)(uint32_t company) noexcept;
	uint16_t (*house_class)(uint16_t house) noexcept;
	void (*make_house)(uint32_t tile, uint32_t town, uint32_t house, uint32_t construction) noexcept;
	void (*add_animation)(uint32_t tile, bool) noexcept;
	void (*population_changed)(uint32_t town) noexcept;
	void (*nearby_stations)(uint32_t town, uint32_t tile, uint32_t size) noexcept;
	uint32_t (*house_count)() noexcept;
	void (*update_max_pass)(uint32_t town) noexcept;
	uint32_t (*next_town)(uint32_t from) noexcept;
	uint64_t (*road_mask)() noexcept;
	bool (*build_road)(uint32_t town, uint32_t tile, uint32_t bits, uint32_t road_type, uint32_t mode) noexcept;
	bool (*clear_land)(uint32_t tile, uint32_t execute) noexcept;
	OpenTTDTownCommandResult (*terraform_land)(uint32_t tile, uint32_t slope, uint32_t up, uint32_t execute) noexcept;
	bool (*build_bridge)(uint32_t tile, uint32_t end, uint32_t bridge, uint32_t road_type, uint32_t execute) noexcept;
	bool (*build_tunnel)(uint32_t tile, uint32_t road_type, uint32_t execute) noexcept;
	void (*clear_house_land)(uint32_t tile) noexcept;
	bool (*allow_construction)(uint32_t town, uint32_t tile, uint32_t house, uint32_t random) noexcept;
	const OpenTTDHouseServices *houses;
};
/* Services are synchronous noexcept calls. No owner reference spans one. Commands
 * and bounded NewGRF resolution may reenter; ordinary gameplay does not throw.
 * Environmental failures and Rust panics abort. Save/load staging stays native. */
extern "C" {
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
void openttd_rust_town_tick_all(const OpenTTDTownLeaves *, const OpenTTDSharedServices *);
void openttd_rust_town_update_growth(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, uint32_t town);
void openttd_rust_town_expand(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, uint32_t town, uint32_t amount, uint32_t modes);
void openttd_rust_town_custom_rate(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, uint32_t town, uint32_t rate);
void openttd_rust_town_fund(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, uint32_t town);
uint32_t openttd_rust_town_road_type(const OpenTTDTownLeaves *, const OpenTTDSharedServices *);
bool openttd_rust_town_tunnel(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, uint32_t town, uint32_t tile, uint32_t dir);
}
#endif /* RUST_TOWN_FFI_H */
