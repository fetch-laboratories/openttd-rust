/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_ffi.h Synchronous shared game services for Rust owners. */
#ifndef RUST_SERVICES_FFI_H
#define RUST_SERVICES_FFI_H
#include "../tile_storage_type.h"
#include "service_defaults.hpp"
#include <cstdint>
#include <cstddef>
#include <type_traits>
#include "station_ffi.h"

enum class LandscapeType : uint8_t;
enum class StationType : uint8_t;
struct OpenTTDStationService;
enum class RoadStopType : uint8_t;
struct OpenTTDStationCommandServices;
struct OpenTTDStationCatchmentServices;
struct OpenTTDRoadStopServices;
struct OpenTTDRoadCommandServices;
enum class RoadTileType : uint8_t;

/* Distinct trivial date carriers. Strong native Date classes stay inside C++;
 * wrappers convert explicitly so MSVC class-return rules cannot alter this ABI. */
struct OpenTTDCalendarDate { int32_t value; };
struct OpenTTDEconomyDate { int32_t value; };
static_assert(std::is_trivially_copyable_v<OpenTTDCalendarDate> && std::is_standard_layout_v<OpenTTDCalendarDate>);
static_assert(std::is_trivially_copyable_v<OpenTTDEconomyDate> && std::is_standard_layout_v<OpenTTDEconomyDate>);

/* Canonical typed allocation addresses, never references to tile zero's fields.
 * Only an audited serialized entry may retain it. It expires before map reset or
 * mutex yield. Only field-sized raw reads in audited scheduler entries are allowed;
 * C++ owns allocation/construction/reset/destruction and every write. */
struct OpenTTDTileTypeAllocation {
	const TileBaseStorage *base;
	const TileExtendedStorage *extended;
	uint32_t count;
	bool assertions;
};
OpenTTDTileTypeAllocation GetRustTileTypeAllocation() noexcept;

/* Immutable table of world services; context may be null in the game, while test fixtures supply
 * their own world. Map predicates take a TileIndex value and return the original accessor's
 * underlying type (TileType, TropicZone and Slope are uint8_t, GetTileZ is int). Train reservation
 * services can reenter the Rust train owners. Random debug logging names this wrapper's location.
 * mark_dirty is MarkTileDirtyByTile, free_train_reservation FreeTrainTrackReservation and
 * try_path_reserve TryPathReserve(v, mark_as_stuck, first_tile_okay).
 * Industry query returns 0 not industry, 1 industry, 2 bubble catcher and writes TileVirtXY(x, y). */
enum ClearGround : uint8_t;
enum TreeGround : uint8_t;
enum Foundation : uint8_t;
enum Slope : uint8_t;
enum DiagDirection : uint8_t;
struct OpenTTDCalendarYMD { int32_t year; uint8_t month, day; };
struct OpenTTDPixelLocation { uint32_t tile; uint8_t kind; };
struct OpenTTDFoundationSlope { int32_t z; Slope slope; };
struct OpenTTDTownHandle; // Opaque native Town shell; never dereferenced in Rust.
struct OpenTTDTownState;
struct OpenTTDVehicle;
struct Train; // Native Train shell; never dereferenced in Rust.
enum RoadType : uint8_t;
enum RoadBits : uint8_t;
enum Trackdir : uint8_t;
struct OpenTTDWaterGroundServices;
struct OpenTTDEnvironmentServices {
	bool (*editor)() noexcept OPENTTD_SERVICE_DEFAULT(editor);
	bool (*freeform_edges)() noexcept OPENTTD_SERVICE_DEFAULT(freeform_edges);
};
const OpenTTDEnvironmentServices &GetRustEnvironmentServices() noexcept;
enum RoadTramType : bool;
struct OpenTTDClearSquareServices {

	bool (*docking)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(docking);
	bool (*generating_world)() noexcept OPENTTD_SERVICE_DEFAULT(generating_world);
};
struct OpenTTDTypeAvailabilityServices;
struct OpenTTDPbsServices;
struct OpenTTDMovementServices;
struct OpenTTDSharedServices {
	const OpenTTDMovementServices *(*movement_services)() noexcept OPENTTD_SERVICE_DEFAULT(movement_services);
	void (*set_tile_owner)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_tile_owner);
	bool (*autoslope)() noexcept OPENTTD_SERVICE_DEFAULT(autoslope);
	uint8_t (*bay_stop_direction)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(bay_stop_direction);
	bool (*bridge_above)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(bridge_above);
	uint8_t (*bridge_axis)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(bridge_axis);
	uint16_t (*bridge_speed)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(bridge_speed);
	bool (*build_on_slopes)() noexcept OPENTTD_SERVICE_DEFAULT(build_on_slopes);
	OpenTTDClearSquareServices clear_square_services;
	const OpenTTDTownHandle *(*calc_closest_town)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(calc_closest_town);
	const OpenTTDTownHandle *(*closest_town)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(closest_town);
	bool (*company_valid)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(company_valid);
	void *context;
	RoadBits (*crossing_road_bits)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(crossing_road_bits);
	uint8_t (*current_company)() noexcept OPENTTD_SERVICE_DEFAULT(current_company);
	uint16_t (*depot_index)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_index);
	uint8_t (*disallowed_road_directions)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(disallowed_road_directions);
	bool (*disaster_sound)() noexcept OPENTTD_SERVICE_DEFAULT(disaster_sound);
	uint8_t (*drive_through_axis)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(drive_through_axis);
	bool (*forbid_90_deg)() noexcept OPENTTD_SERVICE_DEFAULT(forbid_90_deg);
	void (*free_train_reservation)(const Train *) noexcept OPENTTD_SERVICE_DEFAULT(free_train_reservation);
	bool (*has_road_type)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(has_road_type);
	bool (*has_road_works)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(has_road_works);
	bool (*has_water_ground)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(has_water_ground);
	uint32_t (*industry)(int32_t, int32_t, uint32_t *) noexcept OPENTTD_SERVICE_DEFAULT(industry);
	bool (*is_any_road_stop)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_any_road_stop);
	bool (*is_bay_stop)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_bay_stop);
	bool (*is_bridge)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_bridge);
	bool (*is_drive_through_stop)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_drive_through_stop);
	bool (*is_level_crossing)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_level_crossing);
	bool (*is_normal_road)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_normal_road);
	bool (*is_plain_rail)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_plain_rail);
	bool (*is_road_depot)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_road_depot);
	bool (*is_sea)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_sea);
	bool (*is_water_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_water_tile);
	uint32_t (*map_log_x)() noexcept OPENTTD_SERVICE_DEFAULT(map_log_x);
	uint32_t (*map_max_x)() noexcept OPENTTD_SERVICE_DEFAULT(map_max_x);
	uint32_t (*map_max_y)() noexcept OPENTTD_SERVICE_DEFAULT(map_max_y);
	void (*mark_bridge_dirty)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(mark_bridge_dirty);
	void (*mark_dirty)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(mark_dirty);
	uint8_t (*next_company)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(next_company);
	bool (*no_jetcrash)() noexcept OPENTTD_SERVICE_DEFAULT(no_jetcrash);
	RoadBits (*normal_road_bits)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(normal_road_bits);
	uint32_t (*other_tunnel_end)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(other_tunnel_end);
	uint8_t (*plane_crashes)() noexcept OPENTTD_SERVICE_DEFAULT(plane_crashes);
	uint8_t (*plane_speed)() noexcept OPENTTD_SERVICE_DEFAULT(plane_speed);
	uint32_t (*random)(void *) noexcept OPENTTD_SERVICE_DEFAULT(random);
	uint8_t (*road_bits)(uint32_t, uint8_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(road_bits);
	uint16_t (*road_cost_multiplier)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(road_cost_multiplier);
	uint8_t (*road_depot_direction)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(road_depot_direction);
	uint8_t (*road_owner)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(road_owner);
	uint8_t (*road_type)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(road_type);
	bool (*crossing_reserved)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(crossing_reserved);
	void (*set_crossing_reserved)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_crossing_reserved);
	bool (*may_have_road)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(may_have_road);
	RoadTileType (*road_tile_kind)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(road_tile_kind);
	void (*set_road_bits)(uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_road_bits);
	void (*set_road_type)(uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_road_type);
	void (*set_road_owner)(uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_road_owner);
	void (*set_road_directions)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_road_directions);
	bool (*road_powered)(uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(road_powered);
	const OpenTTDTypeAvailabilityServices *(*type_availability)() noexcept OPENTTD_SERVICE_DEFAULT(type_availability);
	const OpenTTDPbsServices *(*pbs_services)(const OpenTTDSharedServices *) noexcept OPENTTD_SERVICE_DEFAULT(pbs_services);
	bool (*service_at_helipad)() noexcept OPENTTD_SERVICE_DEFAULT(service_at_helipad);
	void (*set_tropic_zone)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_tropic_zone);
	uint8_t (*snow_line)() noexcept OPENTTD_SERVICE_DEFAULT(snow_line);
	uint16_t (*station_index)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_index);
	const OpenTTDRoadCommandServices *(*road_commands)() noexcept OPENTTD_SERVICE_DEFAULT(road_commands);
	const OpenTTDStationCommandServices *(*station_commands)() noexcept OPENTTD_SERVICE_DEFAULT(station_commands);
	const OpenTTDStationCatchmentServices *(*station_catchment)() noexcept OPENTTD_SERVICE_DEFAULT(station_catchment);
	bool (*docking_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(docking_tile);
	uint8_t (*station_gfx)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_gfx);
	bool (*station_serve_neutral)() noexcept OPENTTD_SERVICE_DEFAULT(station_serve_neutral);
	StationType (*station_type)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_type);

	void (*tile_animation_set_frame)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_animation_set_frame);
	uint32_t (*town_next_id)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(town_next_id);
	OpenTTDTownState *(*town_owner_by_id)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(town_owner_by_id);
	size_t (*utf8_length)(const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(utf8_length);
	OpenTTDVehicle *(*vehicle_next)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_next);
	bool (*water_middle_lock)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(water_middle_lock);
	uint32_t (*tile_height)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_height);
	int32_t (*tile_max_z)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_max_z);
	uint8_t (*tile_owner)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_owner);
	uint8_t (*tile_slope)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_slope);
	uint8_t (*tile_type)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_type);
	bool (*tile_valid)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_valid);
	OpenTTDFoundationSlope (*tile_slope_z)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_slope_z);
	int32_t (*tile_z)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_z);
	uint16_t (*town_index)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(town_index);
	OpenTTDTownState *(*town_owner)(const OpenTTDTownHandle *) noexcept OPENTTD_SERVICE_DEFAULT(town_owner);
	Trackdir (*train_trackdir)(const Train *) noexcept OPENTTD_SERVICE_DEFAULT(train_trackdir);
	uint8_t (*tram_owner)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tram_owner);
	RoadType (*tram_type)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tram_type);
	uint8_t (*tropic_zone)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tropic_zone);
	bool (*try_path_reserve)(Train *, bool, bool) noexcept OPENTTD_SERVICE_DEFAULT(try_path_reserve);
	uint8_t (*tunnel_bridge_direction)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tunnel_bridge_direction);
	uint8_t (*tunnel_transport)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tunnel_transport);
	void (*update_level_crossing)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(update_level_crossing);
	void (*rebuild_town_tree)() noexcept OPENTTD_SERVICE_DEFAULT(rebuild_town_tree);
	bool (*recession)() noexcept OPENTTD_SERVICE_DEFAULT(recession);
	uint64_t (*tick_counter)() noexcept OPENTTD_SERVICE_DEFAULT(tick_counter);
	size_t (*town_count)() noexcept OPENTTD_SERVICE_DEFAULT(town_count);

	OpenTTDFoundationSlope (*apply_foundation)(Foundation, Slope) noexcept OPENTTD_SERVICE_DEFAULT(apply_foundation);
	ClearGround (*clear_ground)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(clear_ground);
	bool (*has_water_class)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(has_water_class);
	void (*make_shore)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(make_shore);
	void (*make_snow)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(make_snow);
	void (*play_sound)(uint32_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(play_sound);
	void (*set_fence)(uint32_t, DiagDirection, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_fence);
	TreeGround (*tree_ground)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tree_ground);
	const OpenTTDWaterGroundServices *(*water_services)() noexcept OPENTTD_SERVICE_DEFAULT(water_services); ///< Immutable table; getter avoids recursive static initialization.
	void (*for_each_station)(void *, void (*)(void *, const OpenTTDStation *, OpenTTDStationService *, uint16_t)) noexcept OPENTTD_SERVICE_DEFAULT(for_each_station); ///< Live Station::Iterate(); resolves the next slot after each visit, including reentry.
	RoadStopType (*road_stop_type)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(road_stop_type);
	const OpenTTDRoadStopServices *(*road_stops)() noexcept OPENTTD_SERVICE_DEFAULT(road_stops);
};
extern const OpenTTDSharedServices _rust_shared_services;
struct OpenTTDSubsidyServices;
const OpenTTDSubsidyServices &GetRustSubsidyServices() noexcept;
inline const OpenTTDSubsidyServices *RustSubsidyServices() noexcept { return &GetRustSubsidyServices(); }
const OpenTTDSharedServices &GetRustSharedServices() noexcept;
/** Shared save/reseed/restore of BOTH original RNG streams. */
struct OpenTTDRandomSeeds { uint32_t random[2], interactive[2]; };
struct OpenTTDRandomServices {
	OpenTTDRandomSeeds (*save)() noexcept OPENTTD_SERVICE_DEFAULT(save);
	void (*restore)(OpenTTDRandomSeeds) noexcept OPENTTD_SERVICE_DEFAULT(restore);
	void (*set_seed)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_seed);
};
const OpenTTDRandomServices &GetRustRandomServices() noexcept;

/* Fixed-underlying native enum declarations keep standalone ABI consumers lean. */
enum DiagDirection : uint8_t;
enum TrackBits : uint8_t;
enum SignalType : uint8_t;
enum SignalState : uint8_t;
enum Axis : uint8_t;
enum TransportType : uint8_t;
enum Track : uint8_t;
enum RailType : uint8_t;
enum class RailTileType : uint8_t;
enum class RailGroundType : uint8_t;
enum SignalVariant : uint8_t;

struct OpenTTDRailTraversal { bool depot; TrackBits tracks; };
struct OpenTTDRailSignalPresence { bool signals; bool track; SignalType kind; bool forward; bool reverse; };
struct OpenTTDRailSignalUpdate { SignalType kind; TrackBits reservations; };
struct OpenTTDRailStationAdmission { bool rail; uint8_t owner; Axis axis; bool blocked; };
struct OpenTTDRailCrossingAdmission { bool crossing; uint8_t owner; Axis axis; };
struct OpenTTDRailWormholeAdmission { uint8_t owner; TransportType transport; DiagDirection direction; };

/* Canonical rail/signal map leaves. A consuming owner calls each phase only at
 * the source observation point; signal writes are immediate. No map pointer,
 * whole tile record, opcode or allocation crosses this boundary. */
struct OpenTTDRailServices;
struct OpenTTDRailMapServices {
	const OpenTTDRailServices *ground;
	OpenTTDRailTraversal (*traversal)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(traversal);
	OpenTTDRailSignalPresence (*signal_presence)(uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(signal_presence);
	OpenTTDRailSignalUpdate (*signal_update)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(signal_update);
	OpenTTDRailStationAdmission (*station_admission)(uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(station_admission);
	OpenTTDRailCrossingAdmission (*crossing_admission)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(crossing_admission);
	OpenTTDRailWormholeAdmission (*wormhole_admission)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(wormhole_admission);

	bool (*is_rail_depot)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_rail_depot);
	DiagDirection (*rail_depot_direction)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_depot_direction);
	bool (*has_signal_trackdir)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(has_signal_trackdir);
	SignalState (*signal_state)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(signal_state);
	void (*set_signal_state)(uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_signal_state);
	uint32_t (*other_tunnel_bridge_end)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(other_tunnel_bridge_end);
	/* Rail command, depot and level-crossing leaves (rail_cmd.rs). */
	bool (*has_signals)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(has_signals);
	SignalType (*signal_type)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(signal_type);
	RailType (*invalid_rail_type)() noexcept OPENTTD_SERVICE_DEFAULT(invalid_rail_type);
	RailType (*rail_type)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_type);
	void (*set_rail_type)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_rail_type);
	RailTileType (*rail_tile_type)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_tile_type);
	void (*set_track_bits)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_track_bits);
	unsigned int (*present_signals)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(present_signals);
	void (*set_present_signals)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_present_signals);
	unsigned int (*signal_states)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(signal_states);
	void (*set_signal_states)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_signal_states);
	void (*set_has_signals)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_has_signals);
	void (*set_signal_type)(uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_signal_type);
	SignalVariant (*signal_variant)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(signal_variant);
	void (*set_signal_variant)(uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_signal_variant);
	void (*cycle_signal_side)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(cycle_signal_side);
	TrackBits (*rail_reservation)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_reservation);
	void (*set_track_reservation)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_track_reservation);
	bool (*depot_reserved)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_reserved);
	void (*set_depot_exit)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_depot_exit);
	void (*make_rail_normal)(uint32_t, uint8_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(make_rail_normal);
	void (*make_rail_depot)(uint32_t, uint8_t, uint16_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(make_rail_depot);
	TrackBits (*crossing_rail_bits)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(crossing_rail_bits);
	Axis (*crossing_road_axis)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(crossing_road_axis);
	bool (*has_station_rail)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(has_station_rail);
	bool (*is_rail_station)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_rail_station);
	bool (*station_blocked)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_blocked);
	Track (*station_track)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_track);
	bool (*wormhole_reserved)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(wormhole_reserved);
};
const OpenTTDRailMapServices &GetRustRailMapServices() noexcept;

/* Standalone fixtures without a game map: map predicates return zero and writes do nothing. */
template <typename Result, typename... Args>
Result OpenTTDFixtureMapService(Args...) noexcept
{
	if constexpr (!std::is_void_v<Result>) return Result{};
}
inline OpenTTDSharedServices OpenTTDFixtureSharedServices(void *context, uint32_t (*random)(void *) noexcept, uint32_t (*industry)(int32_t, int32_t, uint32_t *) noexcept = OpenTTDAbortService<uint32_t (*)(int32_t, int32_t, uint32_t *) noexcept>::Call) noexcept
{
	return {
		.clear_square_services = {},
		.context = context,
		.current_company = []() noexcept -> uint8_t { return UINT8_MAX; },
		.industry = industry,
		.next_company = [](uint8_t) noexcept -> uint8_t { return UINT8_MAX; },
		.random = random,
		.rebuild_town_tree = OpenTTDFixtureMapService<void>,
		.recession = OpenTTDFixtureMapService<bool>,
		.tick_counter = OpenTTDFixtureMapService<uint64_t>,
		.town_count = OpenTTDFixtureMapService<size_t>,
	};
}
#endif /* RUST_SERVICES_FFI_H */
