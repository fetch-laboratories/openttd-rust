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

/* Allocation-derived base array address, never a reference to tile zero's byte.
 * Only an audited serialized entry may retain it. It expires before map reset or
 * mutex yield. Raw reads consume only type@0 of each actual eight-byte base tile;
 * C++ owns allocation/construction/reset/destruction and every write. */
struct OpenTTDTileTypeAllocation {
	const uint8_t *base;
	uint32_t count;
};
OpenTTDTileTypeAllocation GetRustTileTypeAllocation() noexcept;

/* Immutable function table. All functions are noexcept; world operations may
 * reenter Rust owners after the caller ends its world/owner accesses.
 * Context may be null in the game; test fixtures supply their own world.
 * No Rust references into the map/pools survive calls. Output is call-scoped.
 * Map predicates take a TileIndex value and return the original accessor's
 * underlying type: TileType, TropicZone and Slope are uint8_t, GetTileZ is int.
 * mark_dirty is MarkTileDirtyByTile(tile) and set_tropic_zone is SetTropicZone.
 * Random logging still runs, but its debug source location names this wrapper.
 * Industry query returns 0 not industry,1 industry,2 bubble catcher and writes
 * TileVirtXY(x,y). It observes only type and industry graphics.
 * An environmental exception terminates rather than unwinding through Rust. */
enum ClearGround : uint8_t;
enum TreeGround : uint8_t;
enum Slope : uint8_t;
enum DiagDirection : uint8_t;
struct OpenTTDFoundationSlope { int32_t z; Slope slope; };
struct OpenTTDTownHandle; // Opaque native Town shell; never dereferenced in Rust.
struct OpenTTDTownState;
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
	ClearGround (*clear_ground)(uint32_t) noexcept;
	TreeGround (*tree_ground)(uint32_t) noexcept;
	void (*make_snow)(uint32_t, uint32_t) noexcept;
	void (*make_shore)(uint32_t) noexcept;
	void (*tile_loop_water)(uint32_t) noexcept; ///< Native TileLoop_Water; may reenter Rust owners.
	void (*play_sound)(uint32_t, uint16_t) noexcept;
	OpenTTDFoundationSlope (*foundation)(uint32_t) noexcept;
	void (*set_fence)(uint32_t, DiagDirection, uint32_t) noexcept;
	bool (*has_water_class)(uint32_t) noexcept;
};
const OpenTTDSharedServices &GetRustSharedServices() noexcept;
/* Shared reader ABI used by default allocation/layout/oracle tests, not game
 * dispatch. Zero count selects the existing callback backend. No pointer retained. */
extern "C" uint8_t openttd_rust_shared_tile_type(OpenTTDTileTypeAllocation, uint8_t (*callback)(uint32_t) noexcept, uint32_t tile);
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
		.clear_ground = OpenTTDFixtureMapService<ClearGround, uint32_t>,
		.tree_ground = OpenTTDFixtureMapService<TreeGround, uint32_t>,
		.make_snow = OpenTTDFixtureMapService<void, uint32_t, uint32_t>,
		.make_shore = OpenTTDFixtureMapService<void, uint32_t>,
		.tile_loop_water = OpenTTDFixtureMapService<void, uint32_t>,
		.play_sound = OpenTTDFixtureMapService<void, uint32_t, uint16_t>,
		.foundation = OpenTTDFixtureMapService<OpenTTDFoundationSlope, uint32_t>,
		.set_fence = OpenTTDFixtureMapService<void, uint32_t, DiagDirection, uint32_t>,
		.has_water_class = OpenTTDFixtureMapService<bool, uint32_t>,
	};
}
#endif /* RUST_SERVICES_FFI_H */
