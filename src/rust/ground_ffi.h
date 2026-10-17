/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file ground_ffi.h Consumed terrain fields and direct synchronous leaves. */
#ifndef RUST_GROUND_FFI_H
#define RUST_GROUND_FFI_H
#include "services_ffi.h"
/* Actual fixed-underlying native enums, not byte aliases. Rust transparent
 * carriers preserve every stored bit pattern without enum-discriminant guards. */
enum ClearGround : uint8_t;
enum class RailGroundType : uint8_t;
enum Slope : uint8_t;
enum TrackBits : uint8_t;
enum Track : uint8_t;
enum Foundation : uint8_t;
enum DiagDirection : uint8_t;
enum class Roadside : uint8_t;
enum RoadBits : uint8_t;
enum RoadType : uint8_t;
enum TreeGround : uint8_t;
enum class WaterClass : uint8_t;
enum FloodingBehaviour : uint8_t;
enum class LandscapeType : uint8_t;
enum VehicleType : uint8_t;
#include "command_services_ffi.h"
#include "command_cost_ffi.h"
/* Post-climate guarded reads only; inactive members contain neutral wire values.
 * Three bytes, returned in one register; density is GetClearDensity's two bits. */
struct OpenTTDClearSurface { ClearGround ground; uint8_t density; bool snow; };
struct OpenTTDFieldFences { uint32_t ne; uint32_t se; uint32_t sw; uint32_t nw; };
/* Strong IDs (IndustryID16, Owner/CompanyID8) and VehStates8 retain their
 * established underlying wire values;
 * nontrivial native ID classes are never passed by value across this ABI. */
struct OpenTTDFieldGround { uint16_t industry; uint32_t type; };
struct OpenTTDClearServices {
	bool (*field_tile)(uint32_t) noexcept;
	uint32_t (*density)(uint32_t) noexcept;
	uint32_t (*counter)(uint32_t) noexcept;
	bool (*snow)(uint32_t) noexcept;
	OpenTTDClearSurface (*surface)(uint32_t) noexcept;
	void (*set_ground)(uint32_t, ClearGround, uint32_t) noexcept;
	void (*set_density)(uint32_t, uint32_t) noexcept;
	void (*set_counter)(uint32_t, uint32_t) noexcept;
	void (*clear_snow)(uint32_t) noexcept;
	void (*make_clear)(uint32_t, ClearGround, uint32_t) noexcept;
	OpenTTDFieldFences (*fences)(uint32_t) noexcept;
	OpenTTDFieldGround (*field)(uint32_t) noexcept;
	void (*set_field)(uint32_t, uint32_t) noexcept;
	WaterClass (*water_class)(uint32_t) noexcept;
	void (*ambient)(uint32_t) noexcept;
};
struct OpenTTDRailSnowGround { int32_t z; Slope slope; TrackBits tracks; bool plain; };
struct OpenTTDRailFenceGround { TrackBits tracks; uint8_t owner; bool plain; };
struct OpenTTDRailServices {
	RailGroundType (*ground)(uint32_t) noexcept;
	void (*set_ground)(uint32_t, RailGroundType) noexcept;
	OpenTTDRailSnowGround (*snow)(uint32_t) noexcept;
	OpenTTDRailFenceGround (*fences)(uint32_t) noexcept;
	Foundation (*foundation)(Slope, TrackBits) noexcept;
	bool (*owned_land)(uint32_t) noexcept;
};
struct OpenTTDTunnelGround { int32_t z; bool snow; };
struct OpenTTDTunnelServices {
	OpenTTDTunnelGround (*snow)(uint32_t) noexcept;
	bool (*snow_or_desert)(uint32_t) noexcept;
	void (*set_snow_or_desert)(uint32_t, bool) noexcept;
};
struct OpenTTDGroundCall; // Native optional CommandCost, independently owned by each facade invocation.
struct OpenTTDGroundCostServices {
	const OpenTTDCommandServices *commands;
	const OpenTTDCommandCostServices *costs;
	OpenTTDCommandCost *(*prepare)(OpenTTDGroundCall *) noexcept;
	void (*release)(OpenTTDGroundCall *) noexcept;
};
struct OpenTTDRoadSnowGround { int32_t z; bool snow; };
struct OpenTTDRoadGroundServices {
	OpenTTDRoadSnowGround (*snow)(uint32_t) noexcept;
	bool (*snow_or_desert)(uint32_t) noexcept;
	void (*toggle_snow_or_desert)(uint32_t) noexcept;
	Roadside (*roadside)(uint32_t) noexcept;
	void (*set_roadside)(uint32_t, Roadside) noexcept;
	uint32_t (*work_counter)(uint32_t) noexcept;
	void (*set_work_counter)(uint32_t, uint32_t) noexcept;
	RoadBits (*all_bits)(uint32_t) noexcept;
	void (*bulldozer)(uint32_t) noexcept;
	RoadBits (*cleanup)(uint32_t, RoadBits) noexcept;
	void (*remove)(uint32_t, RoadBits) noexcept;
	RoadType (*town_road_type)() noexcept;
	void (*set_road_type)(uint32_t, RoadType) noexcept;
};
const OpenTTDGroundCostServices &GetRustGroundCostServices() noexcept;
const OpenTTDRoadGroundServices &GetRustRoadGroundServices() noexcept;
const OpenTTDClearServices &GetRustClearServices() noexcept;
const OpenTTDRailServices &GetRustRailGroundServices() noexcept;
const OpenTTDTunnelServices &GetRustTunnelGroundServices() noexcept;
struct OpenTTDWaterGroundServices;
extern "C" {
void openttd_rust_road_tile_loop(uint32_t, LandscapeType, bool, bool, uint32_t, OpenTTDGroundCall *, const OpenTTDGroundCostServices *, const OpenTTDRoadGroundServices *, const OpenTTDSharedServices *);
void openttd_rust_clear_tile_loop(uint32_t, LandscapeType, bool, bool, uint32_t, const OpenTTDClearServices *, const OpenTTDSharedServices *);
void openttd_rust_rail_tile_loop(uint32_t, LandscapeType, bool, uint32_t, uint32_t, OpenTTDGroundCall *, const OpenTTDWaterGroundServices *, const OpenTTDRailServices *, const OpenTTDSharedServices *);
void openttd_rust_tunnel_tile_loop(uint32_t, LandscapeType, const OpenTTDTunnelServices *, const OpenTTDSharedServices *);
}
struct OpenTTDVehicle;
struct OpenTTDVehicleServices;
struct OpenTTDAirportGround { const OpenTTDStation *station; uint32_t tile; uint16_t width; uint16_t height; };
/* Native placement construction uses two pointer-aligned words, with native size
 * and alignment assertions. Rust never inspects the iterator representation. */
struct OpenTTDTileVehicleCursor { uintptr_t storage[2]; };
struct OpenTTDFloodVictim { uint8_t status; VehicleType type; };
struct OpenTTDAircraftFloodPose { uint32_t tile; int32_t z; uint8_t subtype; };
struct OpenTTDGroundFloodPose { int32_t z; OpenTTDVehicle *first; };
struct OpenTTDCompanyServices;
struct OpenTTDWaterGroundServices {
	const OpenTTDRailServices *rail;
	const OpenTTDClearServices *clear;
	const OpenTTDVehicleServices *vehicles;
	const OpenTTDGroundCostServices *costs;
	const OpenTTDCompanyServices *company; ///< Current-company exchange and signal-buffer update.
	bool (*non_flooding)(uint32_t) noexcept;
	void (*set_non_flooding)(uint32_t, bool) noexcept;
	bool (*coast)(uint32_t) noexcept;
	void (*set_tree_ground)(uint32_t, TreeGround, uint32_t) noexcept;
	void (*make_sea)(uint32_t) noexcept;
	bool (*buoy_or_dock)(uint32_t) noexcept;
	bool (*canal_or_river)(uint32_t) noexcept;
	bool (*possible_docking)(uint32_t) noexcept;
	void (*check_docking)(uint32_t) noexcept;
	void (*invalidate_region)(uint32_t) noexcept;
	TrackBits (*tracks)(uint32_t) noexcept;
	void (*remove_rail)(OpenTTDCommandCost *, uint32_t, Track) noexcept;
	OpenTTDFoundationSlope (*apply_foundation)(Foundation, Slope) noexcept;
	void (*iterator_begin)(OpenTTDTileVehicleCursor *, uint32_t) noexcept;
	OpenTTDVehicle *(*iterator_current)(OpenTTDTileVehicleCursor *) noexcept;
	void (*iterator_next)(OpenTTDTileVehicleCursor *) noexcept;
	void (*iterator_end)(OpenTTDTileVehicleCursor *) noexcept;
	bool (*airport_tile)(uint32_t) noexcept;
	OpenTTDAirportGround (*airport)(uint32_t) noexcept;
	bool (*airport_member)(const OpenTTDStation *, uint32_t) noexcept;
	int32_t (*airport_z)(uint32_t) noexcept;
	OpenTTDFloodVictim (*victim)(OpenTTDVehicle *) noexcept;
	OpenTTDAircraftFloodPose (*aircraft_pose)(OpenTTDVehicle *) noexcept;
	OpenTTDGroundFloodPose (*ground_pose)(OpenTTDVehicle *) noexcept;
	uint32_t (*crash)(OpenTTDVehicle *) noexcept;
	void (*ai_event)(OpenTTDVehicle *, uint32_t) noexcept;
	void (*game_event)(OpenTTDVehicle *, uint32_t) noexcept;
	void (*news)(OpenTTDVehicle *, uint32_t) noexcept;
	int32_t (*bridge_height)(uint32_t) noexcept;
};
const OpenTTDWaterGroundServices &GetRustWaterGroundServices() noexcept;
extern "C" {
void openttd_rust_water_tile_loop(uint32_t, uint32_t, uint32_t, bool, OpenTTDGroundCall *, const OpenTTDWaterGroundServices *, const OpenTTDSharedServices *);
bool openttd_rust_flood_halftile(uint32_t, OpenTTDGroundCall *, const OpenTTDWaterGroundServices *, const OpenTTDSharedServices *);
FloodingBehaviour openttd_rust_flooding_behaviour(uint32_t, const OpenTTDWaterGroundServices *, const OpenTTDSharedServices *);
void openttd_rust_clear_neighbour_flooding(uint32_t, uint32_t, uint32_t, const OpenTTDWaterGroundServices *, const OpenTTDSharedServices *);
void openttd_rust_dirty_water_neighbours(uint32_t, uint32_t, uint32_t, const OpenTTDWaterGroundServices *, const OpenTTDSharedServices *);
}
/* Default native and Windows ABI call fixture: actual record returns and enum
 * parameters, including all-byte values, not just size/offset metadata. */
struct OpenTTDGroundABICalls {
	OpenTTDFieldFences (*fences)() noexcept;
	OpenTTDClearSurface (*surface)() noexcept;
	OpenTTDFieldGround (*field)() noexcept;
	OpenTTDRailSnowGround (*rail_snow)() noexcept;
	OpenTTDRailFenceGround (*rail_fence)() noexcept;
	OpenTTDFoundationSlope (*foundation)() noexcept;
	OpenTTDRoadSnowGround (*road_snow)() noexcept;
	OpenTTDTunnelGround (*tunnel)() noexcept;
	OpenTTDAirportGround (*airport)() noexcept;
	OpenTTDFloodVictim (*victim)() noexcept;
	OpenTTDAircraftFloodPose (*aircraft)() noexcept;
	OpenTTDGroundFloodPose (*ground)() noexcept;
	bool (*enums)(ClearGround, RailGroundType, Slope, TrackBits, Track, Foundation, Roadside, RoadBits, RoadType, TreeGround, WaterClass, FloodingBehaviour, DiagDirection, LandscapeType, VehicleType) noexcept;
};
extern "C" bool openttd_rust_ground_abi_calls(const OpenTTDGroundABICalls *);
#endif /* RUST_GROUND_FFI_H */
