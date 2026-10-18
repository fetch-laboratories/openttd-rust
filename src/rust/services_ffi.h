/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_ffi.h Synchronous shared game services for Rust owners. */
#ifndef RUST_SERVICES_FFI_H
#define RUST_SERVICES_FFI_H
#include "service_defaults.hpp"
#include <cstdint>
#include <type_traits>
#include "station_ffi.h"

enum class LandscapeType : uint8_t;

/* Distinct trivial date carriers. Strong native Date classes stay inside C++;
 * wrappers convert explicitly so MSVC class-return rules cannot alter this ABI. */
struct OpenTTDCalendarDate { int32_t value; };
struct OpenTTDEconomyDate { int32_t value; };
static_assert(std::is_trivially_copyable_v<OpenTTDCalendarDate> && std::is_standard_layout_v<OpenTTDCalendarDate>);
static_assert(std::is_trivially_copyable_v<OpenTTDEconomyDate> && std::is_standard_layout_v<OpenTTDEconomyDate>);

/* Immutable function table. All functions are noexcept; command/PBS/viewport effects may reenter Rust.
 * Context may be null in the game; test fixtures supply their own world.
 * No Rust references into the map/pools survive calls. Output is call-scoped.
 * Map predicates take a TileIndex value and return the original accessor's
 * underlying type: TileType, TropicZone and Slope are uint8_t, GetTileZ is int.
 * mark_dirty is MarkTileDirtyByTile(tile), set_tropic_zone is SetTropicZone and
 * clear_square is DoClearSquare(tile). free_train_reservation is
 * FreeTrainTrackReservation(v), try_path_reserve is TryPathReserve(v, mark_as_stuck,
 * first_tile_okay) and safe_waiting_position is IsSafeWaitingPosition(v, tile,
 * trackdir, true, forbid_90_deg); the last two can reenter Rust train owners.
 * Random logging still runs, but its debug source location names this wrapper.
 * Industry query returns 0 not industry,1 industry,2 bubble catcher and writes
 * TileVirtXY(x,y). It observes only type and industry graphics.
 * An environmental exception terminates rather than unwinding through Rust. */
struct OpenTTDTownHandle; // Opaque native Town shell; never dereferenced in Rust.
struct OpenTTDTownState;
struct Train; // Native Train shell; never dereferenced in Rust.
enum RoadType : uint8_t;
enum RoadBits : uint8_t;
enum Trackdir : uint8_t;
struct OpenTTDSharedServices {
	bool (*autoslope)() noexcept OPENTTD_SERVICE_DEFAULT(autoslope);
	uint8_t (*bay_stop_direction)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(bay_stop_direction);
	bool (*bridge_above)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(bridge_above);
	uint8_t (*bridge_axis)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(bridge_axis);
	uint16_t (*bridge_speed)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(bridge_speed);
	bool (*build_on_slopes)() noexcept OPENTTD_SERVICE_DEFAULT(build_on_slopes);
	int32_t (*calendar_date)() noexcept OPENTTD_SERVICE_DEFAULT(calendar_date);
	int32_t (*calendar_year)() noexcept OPENTTD_SERVICE_DEFAULT(calendar_year);
	void (*clear_square)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(clear_square);
	const OpenTTDTownHandle *(*closest_town)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(closest_town);
	bool (*company_valid)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(company_valid);
	void *context;
	RoadBits (*crossing_road_bits)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(crossing_road_bits);
	uint8_t (*current_company)() noexcept OPENTTD_SERVICE_DEFAULT(current_company);
	uint16_t (*depot_index)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_index);
	uint8_t (*disallowed_road_directions)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(disallowed_road_directions);
	bool (*disaster_sound)() noexcept OPENTTD_SERVICE_DEFAULT(disaster_sound);
	uint32_t (*distance_from_edge)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(distance_from_edge);
	uint8_t (*drive_through_axis)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(drive_through_axis);
	int32_t (*economy_date)() noexcept OPENTTD_SERVICE_DEFAULT(economy_date);
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
	uint8_t (*road_depot_direction)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(road_depot_direction);
	uint8_t (*road_owner)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(road_owner);
	uint8_t (*road_type)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(road_type);
	bool (*safe_waiting_position)(const Train *, uint32_t, uint8_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(safe_waiting_position);
	bool (*service_at_helipad)() noexcept OPENTTD_SERVICE_DEFAULT(service_at_helipad);
	void (*set_tropic_zone)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_tropic_zone);
	int32_t (*slope)(int32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(slope);
	uint8_t (*snow_line)() noexcept OPENTTD_SERVICE_DEFAULT(snow_line);
	uint16_t (*station_index)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_index);
	uint32_t (*tile_height)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_height);
	int32_t (*tile_max_z)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_max_z);
	uint8_t (*tile_owner)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_owner);
	uint8_t (*tile_slope)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_slope);
	uint8_t (*tile_type)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_type);
	bool (*tile_valid)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_valid);
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
	// Registry: house (#256).

	// Registry: landscape (#212).
};
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
struct OpenTTDRailMapServices {
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
	uint32_t (*rail_track_status)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_track_status);
	/* Rail command, depot and level-crossing leaves (#265). */
	TrackBits (*rail_tracks)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_tracks);
	bool (*has_signals)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(has_signals);
	SignalType (*signal_type)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(signal_type);
	RailType (*rail_type)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_type);
	void (*set_rail_type)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_rail_type);
	RailTileType (*rail_tile_type)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_tile_type);
	RailGroundType (*rail_ground)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_ground);
	void (*set_rail_ground)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_rail_ground);
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
	TrackBits (*reserved_tracks)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(reserved_tracks);
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
	void (*set_tile_owner)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_tile_owner);
};
const OpenTTDRailMapServices &GetRustRailMapServices() noexcept;

/* Explicit empty-world behavior for fixtures that need a constant callback. */
template <typename Result, typename... Args>
Result OpenTTDFixtureMapService(Args...) noexcept
{
	if constexpr (!std::is_void_v<Result>) return Result{};
}
inline OpenTTDSharedServices OpenTTDFixtureSharedServices(void *context, uint32_t (*random)(void *) noexcept, uint32_t (*industry)(int32_t, int32_t, uint32_t *) noexcept = OpenTTDAbortService<uint32_t (*)(int32_t, int32_t, uint32_t *) noexcept>::Call) noexcept
{
	return {
		.context = context,
		.current_company = []() noexcept -> uint8_t { return UINT8_MAX; },
		.industry = industry,
		.next_company = [](uint8_t) noexcept -> uint8_t { return UINT8_MAX; },
		.random = random,
		// Registry: house (#256).

		// Registry: landscape (#212).
	};
}
#endif /* RUST_SERVICES_FFI_H */
