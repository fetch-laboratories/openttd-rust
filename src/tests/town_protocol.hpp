/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file town_protocol.hpp Narrow expansion-mode and callback-reentry gaps absent from console saves. */
#ifndef TOWN_PROTOCOL_HPP
#define TOWN_PROTOCOL_HPP
#include "../rust/town_ffi.h"
#include "../rust/type_registry_ffi.h"
#include "../core/backup_type.hpp"
#include "../rust/clock_state.h"
#include "../rust/road_command_ffi.h"
#include "../rust/house_ffi.h"
#include "../rust/house_resolver_ffi.h"
#include "../rust/command_services_ffi.h"
#include "../rust/ground_ffi.h"
#include "../rust/movement_ffi.h"
#include "../tile_storage_type.h"
#include "../rust/landscape_command_ffi.h"
#include "../rust/station_catchment_ffi.h"
#include <algorithm>
#include <array>
#include <memory>
#include <vector>

#include "../rust/animation_save.hpp"
struct TownProtocolWorld {
	OpenTTDTownState *owner;
	OpenTTDWaterRegions *regions;
	uint32_t population = 0, draws = 0, company = 7;
	uint32_t owner_reads = 0, map_reads = 0;
	uint32_t square_clears = 0;
	bool house_callback = false, clear_ok = true, placing = false;
	uint16_t house_class = 0;
	uint32_t clears = 0, animations = 0, roads = 0, landscape_clears = 0, house_count = 1;
	std::vector<std::array<uint32_t, 4>> writes;
};
static inline TownProtocolWorld *town_protocol_world;
static inline uint64_t TownProtocolLeaf(uint32_t op, uint32_t id, uint32_t a, uint32_t b, uint32_t c) noexcept
{
	auto &w = *town_protocol_world;
	switch (op) {
		case 8: { uint32_t previous = w.company; w.company = a; return previous; }
		case 10: w.writes.push_back({id, a, b, c}); break;
		case 17: return w.house_count;
		case 20: return UINT32_MAX;
	}
	return 0;
}
static inline OpenTTDTownState *TownProtocolResolveOwner(uint32_t) noexcept { town_protocol_world->owner_reads++; return town_protocol_world->owner; }
static inline void TownProtocolStations(uint32_t, uint32_t, void *, void (*)(void *, const OpenTTDStation *, uint32_t)) noexcept {}
static inline const OpenTTDStationService *TownProtocolStationActivityOwner(const OpenTTDStation *) noexcept { return nullptr; }
static inline uint8_t TownProtocolStationOwner(const OpenTTDStation *) noexcept { return UINT8_MAX; }
static inline uint32_t TownProtocolRandom(void *) noexcept { town_protocol_world->draws++; return 0xABCDEF01; }
static inline uint32_t TownProtocolIndustry(int32_t, int32_t, uint32_t *) noexcept { return 0; }

static inline uint32_t TownProtocol_map_log_x() noexcept { town_protocol_world->map_reads++; return 6; }
static inline uint8_t TownProtocol_growth_rate() noexcept { return 2; }
static inline uint8_t TownProtocol_snow_line() noexcept { return 0; }
static inline bool TownProtocol_editor() noexcept { return false; }
static inline bool TownProtocol_generating_world() noexcept { return false; }
static inline bool TownProtocol_allow_crossings() noexcept { return false; }
static inline uint8_t TownProtocol_climate() noexcept { return 0; }
static inline uint8_t TownProtocol_highest_snow_line() noexcept { return 0; }
static inline bool TownProtocol_allow_roads() noexcept { return false; }
static inline uint8_t TownProtocol_road_flags(uint8_t) noexcept { return 0; }
static inline OpenTTDTownHouseAvailability TownProtocol_house_availability(uint16_t) noexcept { return {.zones = 0xFFFF, .override_id = UINT16_MAX, .class_id = town_protocol_world->house_class, .probability = 1, .enabled = true}; }
static inline uint8_t TownProtocol_house_flags(uint16_t id) noexcept { return id == 0 || id == 2 ? 1 : 16; }
static inline OpenTTDTownHousePlacement TownProtocol_house_placement(uint16_t) noexcept { return {.min_year = 0, .max_year = 5000}; }
static inline uint8_t TownProtocol_house_population(uint16_t house) noexcept { return house == 2 ? 0 : 10; }
static inline uint8_t TownProtocol_house_extra_flags(uint16_t) noexcept { return 0; }
static inline OpenTTDTownRoadChoice TownProtocol_road_info(uint8_t) noexcept { return {}; }
static inline void TownProtocol_dirty_town(uint32_t town) noexcept { TownProtocolLeaf(1, town, 0, 0, 0); }
static inline int64_t TownProtocol_terraform_price() noexcept { return TownProtocolLeaf(5, 0, 0, 0, 0); }
static inline uint32_t TownProtocol_clean_road_bits(uint32_t tile, uint32_t bits) noexcept { return static_cast<uint32_t>(TownProtocolLeaf(6, tile, bits, 0, 0)); }
static inline void TownProtocol_set_road_town(uint32_t tile, uint32_t town) noexcept { TownProtocolLeaf(7, tile, town, 0, 0); }
static inline uint32_t TownProtocol_exchange_company(uint32_t company) noexcept { return static_cast<uint32_t>(TownProtocolLeaf(8, 0, company, 0, 0)); }
static inline uint16_t TownProtocol_house_class(uint16_t id) noexcept { return id == 2 ? 0 : town_protocol_world->house_class; }
static inline void TownProtocol_make_house(uint32_t tile, uint32_t town, uint32_t house, uint32_t construction) noexcept { TownProtocolLeaf(10, tile, town, house, construction); }
static inline void TownProtocol_population_changed(uint32_t) noexcept { town_protocol_world->population = openttd_rust_town_simulation(town_protocol_world->owner)->population; }
static inline uint32_t TownProtocol_house_count() noexcept { return static_cast<uint32_t>(TownProtocolLeaf(17, 0, 0, 0, 0)); }
static inline void TownProtocol_update_max_pass(uint32_t town) noexcept { TownProtocolLeaf(19, town, 0, 0, 0); }
static inline uint32_t TownProtocol_next_town(uint32_t from) noexcept { return static_cast<uint32_t>(TownProtocolLeaf(20, 0, from, 0, 0)); }
static inline bool TownProtocol_build_road(uint32_t, uint32_t, uint32_t, uint32_t, uint32_t mode) noexcept
{
	auto &w = *town_protocol_world; w.roads++; return mode == 0;
}
static inline bool TownProtocol_build_bridge(uint32_t, uint32_t, uint32_t, uint32_t, uint32_t) noexcept { return false; }
static inline bool TownProtocol_build_tunnel(uint32_t, uint32_t, uint32_t) noexcept { return false; }
static inline void TownProtocol_assert_clear_house(bool) noexcept {}
/* House 2 is an unpopulated, completed old 1x1 house; its rebuild reaches TryBuildTownHouse. */
static inline OpenTTDHouseLoopTile TownProtocol_house_tile(uint32_t) noexcept { return {.age = 255, .completed = true, .lift_has_destination = false}; }
static inline uint16_t TownProtocol_house_callbacks(uint16_t) noexcept { return (1U << 3) | (town_protocol_world->house_callback ? 1U : 0U); }
struct TownProtocolCost { bool ok; };
static inline TownProtocolCost *TownProtocolCostOf(const OpenTTDCommandCost *cost) noexcept { return reinterpret_cast<TownProtocolCost *>(const_cast<OpenTTDCommandCost *>(cost)); }
static inline OpenTTDCommandCostServices TownProtocolCosts()
{
	auto costs = OpenTTDCommandCostServices{};
	costs.construct = [](OpenTTDCommandCostStorage *p) noexcept { return reinterpret_cast<OpenTTDCommandCost *>(std::construct_at(reinterpret_cast<TownProtocolCost *>(p))); };
	costs.destroy = [](OpenTTDCommandCost *p) noexcept { std::destroy_at(TownProtocolCostOf(p)); };
	costs.expense_money = [](OpenTTDCommandCost *p, uint8_t, int64_t) noexcept { TownProtocolCostOf(p)->ok = true; };
	costs.add = [](OpenTTDCommandCost *p, OpenTTDCommandCost *other) noexcept { TownProtocolCostOf(p)->ok &= TownProtocolCostOf(other)->ok; };
	costs.money = [](const OpenTTDCommandCost *) noexcept -> int64_t { return 0; };
	costs.zero = [](OpenTTDCommandCost *cost) noexcept { TownProtocolCostOf(cost)->ok = true; };
	costs.error = [](OpenTTDCommandCost *cost, uint32_t) noexcept { TownProtocolCostOf(cost)->ok = false; };
	costs.success = [](const OpenTTDCommandCost *cost) noexcept { return TownProtocolCostOf(cost)->ok; };
	return costs;
}
static inline const OpenTTDCommandCostServices town_protocol_costs = TownProtocolCosts();
static inline OpenTTDLandscapeCommandServices town_protocol_landscape;
static inline const OpenTTDCommandServices town_protocol_commands = [] {
	auto commands = OpenTTDCommandServices{};
	commands.landscape_commands = []() noexcept -> const OpenTTDLandscapeCommandServices * { return &town_protocol_landscape; };
	commands.enter = []() noexcept { return false; }; commands.leave = []() noexcept {};
	return commands;
}();
static inline const OpenTTDTownAuthorityServices town_protocol_authority{
	.council_tolerance = nullptr,
	.bulldozer = nullptr,
	.authority_distance = nullptr,
	.town_index = [](const OpenTTDTownHandle *) noexcept -> uint16_t { return 0; },
	.refusal = 0,
};
/* Services reached by the house tile-loop rebuild and CmdPlaceHouse; unused callbacks abort. */
static inline OpenTTDHouseServices TownProtocolHouses()
{
	auto houses = OpenTTDHouseServices{};
	houses.cost = &town_protocol_costs;
	houses.commands = &town_protocol_commands;
	houses.authority = &town_protocol_authority;
	houses.entry = [](uint32_t) noexcept -> OpenTTDHouseEntry { town_protocol_world->owner_reads++; return {{nullptr, town_protocol_world->owner, 0}, 2}; };
	houses.loop_tile = TownProtocol_house_tile;
	houses.house_type = [](uint32_t) noexcept -> uint16_t { return 2; };
	houses.completed = [](uint32_t) noexcept { return true; };
	houses.town_shell = [](uint32_t) noexcept -> Town * { return nullptr; };
	houses.town = [](uint32_t) noexcept -> OpenTTDTownRef { town_protocol_world->owner_reads++; return {nullptr, town_protocol_world->owner, 0}; };
	houses.loop_spec = [](uint16_t id) noexcept -> OpenTTDHouseLoopSpec { return {.callbacks = TownProtocol_house_callbacks(id), .flags = TownProtocol_house_flags(id), .population = TownProtocol_house_population(id), .mail = 0, .minimum_life = 0}; };
	houses.delete_inspect = [](uint32_t) noexcept {};
	houses.cargogen_mode = []() noexcept -> uint8_t { return 0; };
	houses.place_houses = []() noexcept -> uint8_t { return 2; };
	houses.company_human = [](uint8_t) noexcept { return true; };
	return houses;
}
static inline const OpenTTDHouseServices town_protocol_houses = TownProtocolHouses();
/* The synthetic map has no animated or docking tiles and no water neighbours. */
static inline const OpenTTDClearServices town_protocol_clear = [] {
	auto clear = OpenTTDClearServices{};
	clear.make_clear = [](uint32_t, ClearGround, uint32_t) noexcept { town_protocol_world->square_clears++; };
	return clear;
}();
static inline const OpenTTDWaterLeaves town_protocol_water_leaves = [] {
	auto leaves = OpenTTDWaterLeaves{};
	leaves.debug = [](uint8_t, int32_t, int32_t) noexcept {};
	return leaves;
}();
static inline const OpenTTDWaterGroundServices town_protocol_water = [] {
	auto water = OpenTTDWaterGroundServices{};
	water.clear = &town_protocol_clear;
	water.region_context = []() noexcept -> OpenTTDWaterRegionContext { return {town_protocol_world->regions, &town_protocol_water_leaves}; };
	return water;
}();
static inline OpenTTDTownLeaves TownProtocolServices()
{
	return {
		.map_log_x = TownProtocol_map_log_x,
		.growth_rate = TownProtocol_growth_rate,
		.snow_line = TownProtocol_snow_line,
		.environment = {.editor = TownProtocol_editor, .freeform_edges = TownProtocol_editor},
		.generating_world = TownProtocol_generating_world,
		.allow_crossings = TownProtocol_allow_crossings,
		.climate = TownProtocol_climate,
		.highest_snow_line = TownProtocol_highest_snow_line,
		.allow_roads = TownProtocol_allow_roads,
		.road_flags = TownProtocol_road_flags,
		.house_availability = TownProtocol_house_availability,
		.house_placement = TownProtocol_house_placement,
		.house_population = TownProtocol_house_population,
		.house_flags = TownProtocol_house_flags,
		.house_extra_flags = TownProtocol_house_extra_flags,
		.house_callbacks = TownProtocol_house_callbacks,
		.road_info = TownProtocol_road_info,
		.resolve_owner = TownProtocolResolveOwner,
		.stations = TownProtocolStations,
		.station_activity_owner = TownProtocolStationActivityOwner,
		.station_owner = TownProtocolStationOwner,
		.dirty_authority = TownProtocol_dirty_town,
		.dirty_town = TownProtocol_dirty_town,

		.terraform_price = TownProtocol_terraform_price,
		.clean_road_bits = TownProtocol_clean_road_bits,
		.set_road_town = TownProtocol_set_road_town,
		.exchange_company = TownProtocol_exchange_company,
		.house_class = TownProtocol_house_class,
		.make_house = TownProtocol_make_house,

		.population_changed = TownProtocol_population_changed,
		.house_count = TownProtocol_house_count,
		.update_max_pass = TownProtocol_update_max_pass,
		.next_town = TownProtocol_next_town,
		.build_bridge = TownProtocol_build_bridge,
		.build_tunnel = TownProtocol_build_tunnel,
		.assert_clear_house = TownProtocol_assert_clear_house,
		.houses = &town_protocol_houses,
	};
}
template <typename Check> void CheckTownProtocol(Check check)
{
	AutoRestoreBackup<decltype(GetRustClockState().calendar_year)> calendar_year_scope(GetRustClockState().calendar_year, decltype(GetRustClockState().calendar_year){1950});
	AutoRestoreBackup<uint64_t> road_mask(*openttd_rust_road_mask(), 0);
	/* Source quirks: first-road execute failure still returns success; a failed
	 * expansion tries 25 times; reentry mutates the canonical owner's flags. */
	auto owner = std::unique_ptr<OpenTTDTownState, decltype(&openttd_rust_town_destroy)>(openttd_rust_town_new(), openttd_rust_town_destroy);
	auto regions = std::unique_ptr<OpenTTDWaterRegions, decltype(&openttd_rust_water_destroy)>(openttd_rust_water_new(64, 64), openttd_rust_water_destroy);
	/* Preserve the process-wide game counters around the isolated fixture. */
	const auto global_ids = openttd_rust_town_global_id_counts();
	const auto global_classes = openttd_rust_town_global_class_counts();
	std::vector<uint32_t> saved_ids(global_ids.size), saved_classes(global_classes.size);
	std::copy_n(static_cast<const uint32_t *>(global_ids.data), global_ids.size, saved_ids.begin());
	std::copy_n(static_cast<const uint32_t *>(global_classes.data), global_classes.size, saved_classes.begin());
	openttd_rust_town_id_counts_resize(owner.get(), 5);
	openttd_rust_town_class_counts_resize(owner.get(), 2);
	openttd_rust_town_global_id_counts_resize(std::max<size_t>(5, saved_ids.size()));
	openttd_rust_town_global_class_counts_resize(std::max<size_t>(2, saved_classes.size()));
	TownProtocolWorld w{}; w.owner = owner.get(); w.regions = regions.get(); town_protocol_world = &w;
	auto *fields = openttd_rust_town_simulation(owner.get()); fields->xy = 2080; std::fill_n(fields->radius, 5, 100);
	const auto leaves = TownProtocolServices();
	OpenTTDSharedServices services = OpenTTDFixtureSharedServices(nullptr, TownProtocolRandom, TownProtocolIndustry);
	/* The callback reenters the town owner through the direct resolver entry;
	 * preserve the original fixture's mutation and success. */
	auto resolver = OpenTTDHouseResolverServices{};
	resolver.town_by_id = [](uint32_t) noexcept -> Town * { return nullptr; };
	resolver.evaluate = [](const OpenTTDHouseCallbackArgs *, int32_t *, size_t) noexcept -> uint16_t {
		openttd_rust_town_growth(town_protocol_world->owner)->flags = 0x88;
		return UINT16_MAX;
	};
	const auto *previous_resolver = openttd_rust_house_resolver_exchange(&resolver);

	/* This fixture models flat, empty ground without roads, depots or bridges. */
	services.is_road_depot = [](uint32_t) noexcept { return false; };
	services.is_any_road_stop = [](uint32_t) noexcept { return false; };
	services.is_bay_stop = [](uint32_t) noexcept { return false; };
	services.road_bits = [](uint32_t, uint8_t, bool) noexcept -> uint8_t { return 0; };
	services.tile_type = [](uint32_t) noexcept -> uint8_t { return 0; };
	services.tile_slope = [](uint32_t) noexcept -> uint8_t { return 0; };
	static OpenTTDSharedServices movement_world{};
	movement_world.tile_type = [](uint32_t) noexcept -> uint8_t { return 0; };
	movement_world.map_log_x = []() noexcept -> uint32_t { return 6; };
	static OpenTTDMovementServices movement{};
	movement.shared = &movement_world;
	movement.tile_types = []() noexcept -> OpenTTDTileTypeAllocation {
		static const std::array<TileBaseStorage, 4096> tiles{};
		static const std::array<TileExtendedStorage, 4096> extended{};
		return {tiles.data(), extended.data(), 4096, true};
	};
	services.movement_services = []() noexcept -> const OpenTTDMovementServices * { return &movement; };
	services.tile_max_z = [](uint32_t) noexcept -> int32_t { return 0; };
	services.tile_z = [](uint32_t) noexcept -> int32_t { return 0; };
	AutoRestoreBackup<decltype(GetRustClockState().calendar_date)> calendar_date_scope(GetRustClockState().calendar_date, decltype(GetRustClockState().calendar_date){0});
	services.bridge_above = [](uint32_t) noexcept { return false; };
	services.has_water_ground = [](uint32_t) noexcept { return false; };
	services.mark_dirty = [](uint32_t) noexcept {};
	services.clear_square_services.docking = [](uint32_t) noexcept { return false; };
	services.clear_square_services.generating_world = []() noexcept { return false; };
	services.water_services = []() noexcept { return &town_protocol_water; };
	/* No stations exist, so placed houses find no nearby station. */
	static OpenTTDStationCatchmentServices catchment{};
	catchment.shared = &services;
	catchment.station_count = []() noexcept -> size_t { return 0; };
	services.station_catchment = []() noexcept -> const OpenTTDStationCatchmentServices * { return &catchment; };
	services.map_log_x = []() noexcept -> uint32_t { return 6; };
	services.map_max_x = []() noexcept -> uint32_t { return 63; };
	services.map_max_y = []() noexcept -> uint32_t { return 63; };
	services.closest_town = [](uint32_t, uint32_t) noexcept -> const OpenTTDTownHandle * { return nullptr; };
	services.tile_valid = [](uint32_t) noexcept { return true; };
	services.road_type = [](uint32_t) noexcept { return uint8_t(63); };
	static const OpenTTDTileOwnerServices owners{};
	static const OpenTTDLandscapeTileServices tiles = [] {
		auto t = OpenTTDLandscapeTileServices{};
		t.clear = [](OpenTTDCommandCost *p, uint32_t, uint16_t flags) noexcept { auto &w = *town_protocol_world; if (w.placing && !(flags & 1)) w.landscape_clears++; else w.clears++; TownProtocolCostOf(p)->ok = w.clear_ok; };
		t.owners = &owners;
		return t;
	}();
	town_protocol_landscape.shared = &services; town_protocol_landscape.costs = &town_protocol_costs;
	town_protocol_landscape.commands = &town_protocol_commands; town_protocol_landscape.tile_procs = &tiles;
	const OpenTTDHouseAnimation animation_house{
		.callback = [](uint16_t, uint32_t, uint32_t, const HouseSpec *, Town *, uint32_t, uint64_t) noexcept -> uint16_t { town_protocol_world->animations++; return UINT16_MAX; },
	};
	auto animation = OpenTTDAnimationServices{};
	animation.shared = &services;
	animation.house = &animation_house;
	animation.state = [](uint32_t) noexcept -> AnimatedTileState { return AnimatedTileState::Deleted; };
	animation.set_state = [](uint32_t tile, AnimatedTileState) noexcept { TownProtocolLeaf(11, tile, 0, 0, 0); };
	animation.house_spec = [](uint16_t) noexcept -> const HouseSpec * { return nullptr; };
	RustAnimationScope animation_scope(animation);

	bool road_ok = false;
	auto costs = OpenTTDCommandCostServices{};
	costs.success = [](const OpenTTDCommandCost *out) noexcept { return *reinterpret_cast<const bool *>(out); };
	OpenTTDRailCommandServices rail{}; rail.cost = &costs;
	OpenTTDRoadCommandServices roads{}; roads.do_build = [](const OpenTTDRoadCommandServices *, OpenTTDRoadCommandScratch *, OpenTTDCommandCost *out, OpenTTDCommandCost *, uint16_t flags, uint32_t tile, uint8_t bits, uint8_t rt, uint8_t, uint16_t town) noexcept {
		*reinterpret_cast<bool *>(out) = TownProtocol_build_road(town, tile, bits, rt, (flags & 1) == 0 ? 0 : (flags & 8) != 0 ? 1 : 2);
	}; roads.rail_commands = &rail;
	OpenTTDRoadCommandContext road{&roads, nullptr, reinterpret_cast<OpenTTDCommandCost *>(&road_ok), nullptr};
	openttd_rust_town_expand(&leaves, &services, &road, 0, 1, 1);
	check(w.roads == 0 && w.draws == 0 && w.company == 7);
	openttd_rust_town_expand(&leaves, &services, &road, 0, 1, 2);
	check(w.roads == 1 && w.draws == 1 && w.company == 7);
	/* Expansion updates radii; restore the isolated house-placement setup. */
	std::fill_n(fields->radius, 5, 100);
	w.clears = 0; w.clear_ok = false;
	openttd_rust_town_expand(&leaves, &services, &road, 0, 1, 2);
	check(w.clears == 25 * 13 && w.company == 7 && w.draws == 1);
	w.clear_ok = true; w.house_callback = true;
	auto *id_counts = static_cast<uint16_t *>(openttd_rust_town_id_counts(owner.get()).data);
	auto *class_counts = static_cast<uint16_t *>(openttd_rust_town_class_counts(owner.get()).data);
	/* A growing town rebuilds the expired house: one draw, clear, then TryBuildTownHouse. */
	const auto rebuild = [&] {
		fields->time_until_rebuild = 1; fields->num_houses++;
		openttd_rust_town_growth(owner.get())->flags |= 1;
		w.owner_reads = 0; w.map_reads = 0;
		openttd_rust_house_tile_loop(&leaves, &services, 2080);
		check(w.owner_reads == 1 && w.map_reads == 1);
	};
	id_counts[0] = UINT16_MAX;
	rebuild();
	check(w.writes.empty() && w.draws == 2 && w.company == 7 && fields->time_until_rebuild == 0xCD + 192);
	id_counts[0] = 0; w.house_class = 1; class_counts[1] = UINT16_MAX;
	rebuild();
	check(w.writes.empty() && w.draws == 3);
	w.house_class = 0; class_counts[1] = 0;
	rebuild();
	check(fields->num_houses == 1 && w.writes.size() == 1 && w.animations == 1 && openttd_rust_town_growth(owner.get())->flags == 0x88);
	check(w.square_clears == 3);
	/* CmdPlaceHouse executes a completed, protected 2x2 house through BuildTownHouse. */
	w.placing = true; w.writes.clear(); w.house_callback = false; w.clears = 0; w.animations = 0; w.house_count = 5;
	TownProtocolCost cost{};
	services.town_count = []() noexcept -> size_t { return 1; };
	services.town_owner = [](const OpenTTDTownHandle *) noexcept { town_protocol_world->owner_reads++; return town_protocol_world->owner; };
	w.owner_reads = 0; w.map_reads = 0;
	openttd_rust_house_place(&leaves, &services, reinterpret_cast<OpenTTDCommandCost *>(&cost), 1, 2080, 1, true, false);
	check(w.owner_reads == 1 && w.map_reads == 1);
	check(cost.ok && w.landscape_clears == 4 && w.clears == 4 && w.animations == 4 && w.writes.size() == 4);
	check(w.writes[0][0] == 2080 && w.writes[1][0] == 2144 && w.writes[2][0] == 2081 && w.writes[3][0] == 2145);
	check(w.writes[0][2] == 1 && w.writes[1][2] == 2 && w.writes[2][2] == 3 && w.writes[3][2] == 4);
	check(w.population == 10 && fields->num_houses == 2);
	const auto counts = openttd_rust_town_id_counts(owner.get());
	const auto *ids = static_cast<const uint16_t *>(counts.data);
	check(ids[0] == 1 && ids[1] == 1 && ids[2] == 1 && ids[3] == 1 && ids[4] == 1);
	/* A skipped builtin lift tick must not observe lift state, owner or map.
	 * This fixture leaves the lift observation at its abort default, making eager reads fail. */
	services.tick_counter = []() noexcept -> uint64_t { return 1; };
	w.owner_reads = 0; w.map_reads = 0;
	openttd_rust_house_animate(&leaves, &services, 2080);
	check(w.owner_reads == 0 && w.map_reads == 0);
	/* Ordinary growth binds its town before walking the road; construction
	 * triggers must reuse that context rather than resolve the owner again. */
	services.road_bits = [](uint32_t tile, uint8_t, bool) noexcept -> uint8_t { return tile == 2080 ? 1 : 0; };
	services.tile_type = [](uint32_t tile) noexcept -> uint8_t { return tile == 2080 ? 2 : 0; };
	services.road_type = [](uint32_t tile) noexcept -> uint8_t { return tile == 2080 ? 0 : 63; };
	w.writes.clear(); w.animations = 0; w.house_count = 1;
	w.owner_reads = 0; w.map_reads = 0;
	openttd_rust_town_expand(&leaves, &services, &road, 0, 1, 1);
	check(w.writes.size() == 1 && w.animations == 1 && w.owner_reads == 1 && w.map_reads == 1);
	openttd_rust_town_global_id_counts_resize(saved_ids.size());
	openttd_rust_town_global_class_counts_resize(saved_classes.size());
	std::copy(saved_ids.begin(), saved_ids.end(), static_cast<uint32_t *>(openttd_rust_town_global_id_counts().data));
	std::copy(saved_classes.begin(), saved_classes.end(), static_cast<uint32_t *>(openttd_rust_town_global_class_counts().data));
	openttd_rust_house_resolver_exchange(previous_resolver);
	town_protocol_world = nullptr;
}
#endif /* TOWN_PROTOCOL_HPP */
