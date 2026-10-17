/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_ffi.h Synchronous shared game services for Rust owners. */
#ifndef RUST_SERVICES_FFI_H
#define RUST_SERVICES_FFI_H
#include <cstdint>
#include <type_traits>
#include "station_ffi.h"

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
	void *context;
	uint32_t (*random)(void *) noexcept;
	uint32_t (*industry)(int32_t, int32_t, uint32_t *) noexcept;
	uint8_t (*tile_type)(uint32_t) noexcept;
	bool (*bridge_above)(uint32_t) noexcept;
	uint8_t (*tropic_zone)(uint32_t) noexcept;
	int32_t (*tile_z)(uint32_t) noexcept;
	uint8_t (*tile_slope)(uint32_t) noexcept;
	void (*mark_dirty)(uint32_t) noexcept;
	void (*set_tropic_zone)(uint32_t, uint8_t) noexcept;
	uint8_t (*tile_owner)(uint32_t) noexcept;
	uint16_t (*station_index)(uint32_t) noexcept;
	uint16_t (*depot_index)(uint32_t) noexcept;
	bool (*is_level_crossing)(uint32_t) noexcept;
	uint8_t (*tunnel_bridge_direction)(uint32_t) noexcept;
	uint16_t (*bridge_speed)(uint32_t) noexcept;
	bool (*is_water_tile)(uint32_t) noexcept;
	bool (*company_valid)(uint8_t) noexcept;
	uint8_t (*next_company)(uint8_t) noexcept;
	uint8_t (*current_company)() noexcept;
	const OpenTTDTownHandle *(*closest_town)(uint32_t, uint32_t) noexcept;
	OpenTTDTownState *(*town_owner)(const OpenTTDTownHandle *) noexcept;
	bool (*tile_valid)(uint32_t) noexcept;
	uint32_t (*distance_from_edge)(uint32_t) noexcept;
	int32_t (*tile_max_z)(uint32_t) noexcept;
	uint32_t (*tile_height)(uint32_t) noexcept;
	uint8_t (*bridge_axis)(uint32_t) noexcept;
	bool (*has_water_ground)(uint32_t) noexcept;
	bool (*is_sea)(uint32_t) noexcept;
	bool (*is_plain_rail)(uint32_t) noexcept;
	bool (*is_bridge)(uint32_t) noexcept;
	uint8_t (*tunnel_transport)(uint32_t) noexcept;
	uint32_t (*other_tunnel_end)(uint32_t) noexcept;
	bool (*is_road_depot)(uint32_t) noexcept;
	uint8_t (*road_depot_direction)(uint32_t) noexcept;
	bool (*is_normal_road)(uint32_t) noexcept;
	uint8_t (*disallowed_road_directions)(uint32_t) noexcept;
	bool (*is_bay_stop)(uint32_t) noexcept;
	uint8_t (*bay_stop_direction)(uint32_t) noexcept;
	bool (*is_drive_through_stop)(uint32_t) noexcept;
	uint8_t (*drive_through_axis)(uint32_t) noexcept;
	bool (*is_any_road_stop)(uint32_t) noexcept;
	uint8_t (*road_type)(uint32_t) noexcept;
	bool (*has_road_type)(uint32_t) noexcept;
	uint8_t (*road_owner)(uint32_t) noexcept;
	uint16_t (*town_index)(uint32_t) noexcept;
	uint8_t (*road_bits)(uint32_t, uint8_t, bool) noexcept;
	uint8_t (*snow_line)() noexcept;
	int32_t (*calendar_year)() noexcept;
	uint32_t (*map_log_x)() noexcept;
	uint32_t (*map_max_x)() noexcept;
	uint32_t (*map_max_y)() noexcept;
	uint8_t (*plane_speed)() noexcept;
	bool (*no_jetcrash)() noexcept;
	uint8_t (*plane_crashes)() noexcept;
	bool (*service_at_helipad)() noexcept;
	bool (*disaster_sound)() noexcept;
	int32_t (*economy_date)() noexcept;
	int32_t (*calendar_date)() noexcept;
	int32_t (*slope)(int32_t, int32_t) noexcept;
	bool (*build_on_slopes)() noexcept;
	bool (*autoslope)() noexcept;
	void (*clear_square)(uint32_t) noexcept;
	void (*free_train_reservation)(const Train *) noexcept;
	bool (*try_path_reserve)(Train *, bool, bool) noexcept;
	bool (*safe_waiting_position)(const Train *, uint32_t, uint8_t, bool) noexcept;
	bool (*has_road_works)(uint32_t) noexcept;
	RoadType (*tram_type)(uint32_t) noexcept;
	uint8_t (*tram_owner)(uint32_t) noexcept;
	RoadBits (*normal_road_bits)(uint32_t, uint8_t) noexcept;
	RoadBits (*crossing_road_bits)(uint32_t) noexcept;
	bool (*forbid_90_deg)() noexcept;
	void (*update_level_crossing)(uint32_t, bool) noexcept;
	void (*mark_bridge_dirty)(uint32_t) noexcept;
	Trackdir (*train_trackdir)(const Train *) noexcept;

};
const OpenTTDSharedServices &GetRustSharedServices() noexcept;
/** Shared save/reseed/restore of BOTH original RNG streams. */
struct OpenTTDRandomSeeds { uint32_t random[2], interactive[2]; };
struct OpenTTDRandomServices {
	OpenTTDRandomSeeds (*save)() noexcept;
	void (*restore)(OpenTTDRandomSeeds) noexcept;
	void (*set_seed)(uint32_t) noexcept;
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
	OpenTTDRailTraversal (*traversal)(uint32_t) noexcept;
	OpenTTDRailSignalPresence (*signal_presence)(uint32_t, uint8_t, uint8_t) noexcept;
	OpenTTDRailSignalUpdate (*signal_update)(uint32_t, uint8_t) noexcept;
	OpenTTDRailStationAdmission (*station_admission)(uint32_t, uint8_t, uint8_t) noexcept;
	OpenTTDRailCrossingAdmission (*crossing_admission)(uint32_t, uint8_t) noexcept;
	OpenTTDRailWormholeAdmission (*wormhole_admission)(uint32_t, uint8_t) noexcept;

	bool (*is_rail_depot)(uint32_t) noexcept;
	DiagDirection (*rail_depot_direction)(uint32_t) noexcept;
	bool (*has_signal_trackdir)(uint32_t, uint8_t) noexcept;
	SignalState (*signal_state)(uint32_t, uint8_t) noexcept;
	void (*set_signal_state)(uint32_t, uint8_t, uint8_t) noexcept;
	uint32_t (*other_tunnel_bridge_end)(uint32_t) noexcept;
	uint32_t (*rail_track_status)(uint32_t, uint8_t) noexcept;
	/* Rail command, depot and level-crossing leaves (#265). */
	TrackBits (*rail_tracks)(uint32_t) noexcept;
	bool (*has_signals)(uint32_t) noexcept;
	SignalType (*signal_type)(uint32_t, uint8_t) noexcept;
	RailType (*rail_type)(uint32_t) noexcept;
	void (*set_rail_type)(uint32_t, uint8_t) noexcept;
	RailTileType (*rail_tile_type)(uint32_t) noexcept;
	RailGroundType (*rail_ground)(uint32_t) noexcept;
	void (*set_rail_ground)(uint32_t, uint8_t) noexcept;
	void (*set_track_bits)(uint32_t, uint8_t) noexcept;
	unsigned int (*present_signals)(uint32_t) noexcept;
	void (*set_present_signals)(uint32_t, uint8_t) noexcept;
	unsigned int (*signal_states)(uint32_t) noexcept;
	void (*set_signal_states)(uint32_t, uint8_t) noexcept;
	void (*set_has_signals)(uint32_t, bool) noexcept;
	void (*set_signal_type)(uint32_t, uint8_t, uint8_t) noexcept;
	SignalVariant (*signal_variant)(uint32_t, uint8_t) noexcept;
	void (*set_signal_variant)(uint32_t, uint8_t, uint8_t) noexcept;
	void (*cycle_signal_side)(uint32_t, uint8_t) noexcept;
	TrackBits (*reserved_tracks)(uint32_t) noexcept;
	TrackBits (*rail_reservation)(uint32_t) noexcept;
	void (*set_track_reservation)(uint32_t, uint8_t) noexcept;
	bool (*depot_reserved)(uint32_t) noexcept;
	void (*set_depot_exit)(uint32_t, uint8_t) noexcept;
	void (*make_rail_normal)(uint32_t, uint8_t, uint8_t, uint8_t) noexcept;
	void (*make_rail_depot)(uint32_t, uint8_t, uint16_t, uint8_t, uint8_t) noexcept;
	TrackBits (*crossing_rail_bits)(uint32_t) noexcept;
	Axis (*crossing_road_axis)(uint32_t) noexcept;
	bool (*has_station_rail)(uint32_t) noexcept;
	bool (*is_rail_station)(uint32_t) noexcept;
	bool (*station_blocked)(uint32_t) noexcept;
	Track (*station_track)(uint32_t) noexcept;
	bool (*wormhole_reserved)(uint32_t) noexcept;
	void (*set_tile_owner)(uint32_t, uint8_t) noexcept;
};
const OpenTTDRailMapServices &GetRustRailMapServices() noexcept;

/* Standalone fixtures without a game map: map predicates return zero and writes do nothing. */
template <typename Result, typename... Args>
Result OpenTTDFixtureMapService(Args...) noexcept
{
	if constexpr (!std::is_void_v<Result>) return Result{};
}
inline OpenTTDSharedServices OpenTTDFixtureSharedServices(void *context, uint32_t (*random)(void *) noexcept, uint32_t (*industry)(int32_t, int32_t, uint32_t *) noexcept) noexcept
{
	return {
		.context = context,
		.random = random,
		.industry = industry,
		.tile_type = OpenTTDFixtureMapService<uint8_t, uint32_t>,
		.bridge_above = OpenTTDFixtureMapService<bool, uint32_t>,
		.tropic_zone = OpenTTDFixtureMapService<uint8_t, uint32_t>,
		.tile_z = OpenTTDFixtureMapService<int32_t, uint32_t>,
		.tile_slope = OpenTTDFixtureMapService<uint8_t, uint32_t>,
		.mark_dirty = OpenTTDFixtureMapService<void, uint32_t>,
		.set_tropic_zone = OpenTTDFixtureMapService<void, uint32_t, uint8_t>,
		.tile_owner = OpenTTDFixtureMapService<uint8_t, uint32_t>,
		.station_index = OpenTTDFixtureMapService<uint16_t, uint32_t>,
		.depot_index = OpenTTDFixtureMapService<uint16_t, uint32_t>,
		.is_level_crossing = OpenTTDFixtureMapService<bool, uint32_t>,
		.tunnel_bridge_direction = OpenTTDFixtureMapService<uint8_t, uint32_t>,
		.bridge_speed = OpenTTDFixtureMapService<uint16_t, uint32_t>,
		.is_water_tile = OpenTTDFixtureMapService<bool, uint32_t>,
		.company_valid = OpenTTDFixtureMapService<bool, uint8_t>,
		.next_company = [](uint8_t) noexcept -> uint8_t { return UINT8_MAX; },
		.current_company = []() noexcept -> uint8_t { return UINT8_MAX; },
		.closest_town = OpenTTDFixtureMapService<const OpenTTDTownHandle *, uint32_t, uint32_t>,
		.town_owner = OpenTTDFixtureMapService<OpenTTDTownState *, const OpenTTDTownHandle *>,
		.tile_valid = OpenTTDFixtureMapService<bool, uint32_t>,
		.distance_from_edge = OpenTTDFixtureMapService<uint32_t, uint32_t>,
		.tile_max_z = OpenTTDFixtureMapService<int32_t, uint32_t>,
		.tile_height = OpenTTDFixtureMapService<uint32_t, uint32_t>,
		.bridge_axis = OpenTTDFixtureMapService<uint8_t, uint32_t>,
		.has_water_ground = OpenTTDFixtureMapService<bool, uint32_t>,
		.is_sea = OpenTTDFixtureMapService<bool, uint32_t>,
		.is_plain_rail = OpenTTDFixtureMapService<bool, uint32_t>,
		.is_bridge = OpenTTDFixtureMapService<bool, uint32_t>,
		.tunnel_transport = OpenTTDFixtureMapService<uint8_t, uint32_t>,
		.other_tunnel_end = OpenTTDFixtureMapService<uint32_t, uint32_t>,
		.is_road_depot = OpenTTDFixtureMapService<bool, uint32_t>,
		.road_depot_direction = OpenTTDFixtureMapService<uint8_t, uint32_t>,
		.is_normal_road = OpenTTDFixtureMapService<bool, uint32_t>,
		.disallowed_road_directions = OpenTTDFixtureMapService<uint8_t, uint32_t>,
		.is_bay_stop = OpenTTDFixtureMapService<bool, uint32_t>,
		.bay_stop_direction = OpenTTDFixtureMapService<uint8_t, uint32_t>,
		.is_drive_through_stop = OpenTTDFixtureMapService<bool, uint32_t>,
		.drive_through_axis = OpenTTDFixtureMapService<uint8_t, uint32_t>,
		.is_any_road_stop = OpenTTDFixtureMapService<bool, uint32_t>,
		.road_type = OpenTTDFixtureMapService<uint8_t, uint32_t>,
		.has_road_type = OpenTTDFixtureMapService<bool, uint32_t>,
		.road_owner = OpenTTDFixtureMapService<uint8_t, uint32_t>,
		.town_index = OpenTTDFixtureMapService<uint16_t, uint32_t>,
		.road_bits = OpenTTDFixtureMapService<uint8_t, uint32_t, uint8_t, bool>,
		.snow_line = OpenTTDFixtureMapService<uint8_t>,
		.calendar_year = OpenTTDFixtureMapService<int32_t>,
		.map_log_x = OpenTTDFixtureMapService<uint32_t>,
		.map_max_x = OpenTTDFixtureMapService<uint32_t>,
		.map_max_y = OpenTTDFixtureMapService<uint32_t>,
		.plane_speed = OpenTTDFixtureMapService<uint8_t>,
		.no_jetcrash = OpenTTDFixtureMapService<bool>,
		.plane_crashes = OpenTTDFixtureMapService<uint8_t>,
		.service_at_helipad = OpenTTDFixtureMapService<bool>,
		.disaster_sound = OpenTTDFixtureMapService<bool>,
		.economy_date = OpenTTDFixtureMapService<int32_t>,
		.calendar_date = OpenTTDFixtureMapService<int32_t>,
		.slope = OpenTTDFixtureMapService<int32_t, int32_t, int32_t>,
		.build_on_slopes = OpenTTDFixtureMapService<bool>,
		.autoslope = OpenTTDFixtureMapService<bool>,
		.clear_square = OpenTTDFixtureMapService<void, uint32_t>,
		.free_train_reservation = OpenTTDFixtureMapService<void, const Train *>,
		.try_path_reserve = OpenTTDFixtureMapService<bool, Train *, bool, bool>,
		.safe_waiting_position = OpenTTDFixtureMapService<bool, const Train *, uint32_t, uint8_t, bool>,
		.has_road_works = OpenTTDFixtureMapService<bool, uint32_t>,
		.tram_type = OpenTTDFixtureMapService<RoadType, uint32_t>,
		.tram_owner = OpenTTDFixtureMapService<uint8_t, uint32_t>,
		.normal_road_bits = OpenTTDFixtureMapService<RoadBits, uint32_t, uint8_t>,
		.crossing_road_bits = OpenTTDFixtureMapService<RoadBits, uint32_t>,
		.forbid_90_deg = OpenTTDFixtureMapService<bool>,
		.update_level_crossing = OpenTTDFixtureMapService<void, uint32_t, bool>,
		.mark_bridge_dirty = OpenTTDFixtureMapService<void, uint32_t>,
		.train_trackdir = OpenTTDFixtureMapService<Trackdir, const Train *>,
	};
}
#endif /* RUST_SERVICES_FFI_H */
