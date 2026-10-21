/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file ground_ffi.h Consumed terrain fields and direct synchronous leaves. */
#ifndef RUST_GROUND_FFI_H
#define RUST_GROUND_FFI_H
#include "service_defaults.hpp"
#include "services_ffi.h"
#include "water_regions_ffi.h"
#include "signal_ffi.h"
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
	bool (*field_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(field_tile);
	uint32_t (*density)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(density);
	uint32_t (*counter)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(counter);
	bool (*snow)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(snow);
	OpenTTDClearSurface (*surface)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(surface);
	void (*set_ground)(uint32_t, ClearGround, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_ground);
	void (*set_density)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_density);
	void (*set_counter)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_counter);
	void (*clear_snow)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(clear_snow);
	void (*make_clear)(uint32_t, ClearGround, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(make_clear);
	OpenTTDFieldFences (*fences)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(fences);
	OpenTTDFieldGround (*field)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(field);
	void (*set_field)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_field);
	WaterClass (*water_class)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(water_class);
	void (*ambient)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(ambient);
};
struct OpenTTDRailSnowGround { int32_t z; Slope slope; TrackBits tracks; bool plain; };
struct OpenTTDRailFenceGround { TrackBits tracks; uint8_t owner; bool plain; };
struct OpenTTDRailServices {
	TrackBits (*tracks)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tracks);
	RailGroundType (*ground)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(ground);
	void (*set_ground)(uint32_t, RailGroundType) noexcept OPENTTD_SERVICE_DEFAULT(set_ground);
	OpenTTDRailSnowGround (*snow)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(snow);
	OpenTTDRailFenceGround (*fences)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(fences);
	Foundation (*foundation)(Slope, TrackBits) noexcept OPENTTD_SERVICE_DEFAULT(foundation);
	bool (*owned_land)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(owned_land);
};
struct OpenTTDTunnelGround { int32_t z; bool snow; };
struct OpenTTDTunnelServices {
	OpenTTDTunnelGround (*snow)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(snow);
	bool (*snow_or_desert)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(snow_or_desert);
	void (*set_snow_or_desert)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_snow_or_desert);
};
struct OpenTTDGroundCall; // Native optional CommandCost, independently owned by each facade invocation.
struct OpenTTDGroundCostServices {
	const OpenTTDCommandServices *commands;
	const OpenTTDCommandCostServices *costs;
	OpenTTDCommandCost *(*prepare)(OpenTTDGroundCall *) noexcept OPENTTD_SERVICE_DEFAULT(prepare);
	void (*release)(OpenTTDGroundCall *) noexcept OPENTTD_SERVICE_DEFAULT(release);
};
struct OpenTTDRoadSnowGround { int32_t z; bool snow; };
struct OpenTTDTownLeaves;
struct OpenTTDRoadGroundServices {
	OpenTTDRoadSnowGround (*snow)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(snow);
	bool (*snow_or_desert)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(snow_or_desert);
	void (*toggle_snow_or_desert)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(toggle_snow_or_desert);
	Roadside (*roadside)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(roadside);
	void (*set_roadside)(uint32_t, Roadside) noexcept OPENTTD_SERVICE_DEFAULT(set_roadside);
	uint32_t (*work_counter)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(work_counter);
	void (*set_work_counter)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_work_counter);
	RoadBits (*all_bits)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(all_bits);
	void (*bulldozer)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(bulldozer);
	RoadBits (*cleanup)(uint32_t, RoadBits) noexcept OPENTTD_SERVICE_DEFAULT(cleanup);
	void (*remove)(uint32_t, RoadBits) noexcept OPENTTD_SERVICE_DEFAULT(remove);
	const OpenTTDTownLeaves *town;
	void (*set_road_type)(uint32_t, RoadType) noexcept OPENTTD_SERVICE_DEFAULT(set_road_type);
};
const OpenTTDGroundCostServices &GetRustGroundCostServices() noexcept;
const OpenTTDRoadGroundServices &GetRustRoadGroundServices() noexcept;
const OpenTTDClearServices &GetRustClearServices() noexcept;
const OpenTTDRailServices &GetRustRailGroundServices() noexcept;
const OpenTTDTunnelServices &GetRustTunnelGroundServices() noexcept;
struct OpenTTDWaterRestorationServices {
	OpenTTDFoundationSlope (*slope_z)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(slope_z);
	void (*infrastructure)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(infrastructure);
	void (*infrastructure_dirty)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(infrastructure_dirty);
	void (*make_canal)(uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(make_canal);
	void (*make_river)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(make_river);
};
const OpenTTDWaterRestorationServices &GetRustWaterRestorationServices() noexcept;
struct OpenTTDWaterGroundServices;
struct OpenTTDVehicle;
struct OpenTTDVehicleServices;
struct OpenTTDAirportGround { const OpenTTDStation *station; uint32_t tile; uint16_t width; uint16_t height; };
struct OpenTTDFloodVictim { uint8_t status; VehicleType type; };
struct OpenTTDAircraftFloodPose { uint32_t tile; int32_t z; uint8_t subtype; };
struct OpenTTDGroundFloodPose { int32_t z; OpenTTDVehicle *first; };
struct OpenTTDCompanyServices;
struct OpenTTDWaterGroundServices {
	const OpenTTDRailServices *rail;
	const OpenTTDClearServices *clear;
	const OpenTTDVehicleServices *vehicles;
	const OpenTTDGroundCostServices *costs;
	const OpenTTDCompanyServices *company; ///< Current-company exchange.
	OpenTTDSignalContext (*signal_context)() noexcept OPENTTD_SERVICE_DEFAULT(signal_context);
	bool (*non_flooding)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(non_flooding);
	void (*set_non_flooding)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_non_flooding);
	bool (*coast)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(coast);
	void (*set_tree_ground)(uint32_t, TreeGround, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_tree_ground);
	void (*make_sea)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(make_sea);
	bool (*buoy_or_dock)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(buoy_or_dock);
	bool (*canal_or_river)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(canal_or_river);
	OpenTTDWaterRegionContext (*region_context)() noexcept OPENTTD_SERVICE_DEFAULT(region_context);
	void (*remove_rail)(OpenTTDCommandCost *, uint32_t, Track) noexcept OPENTTD_SERVICE_DEFAULT(remove_rail);
	OpenTTDFoundationSlope (*apply_foundation)(Foundation, Slope) noexcept OPENTTD_SERVICE_DEFAULT(apply_foundation);
	bool (*airport_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(airport_tile);
	OpenTTDAirportGround (*airport)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(airport);
	bool (*airport_member)(const OpenTTDStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(airport_member);
	int32_t (*airport_z)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(airport_z);
	OpenTTDFloodVictim (*victim)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(victim);
	OpenTTDAircraftFloodPose (*aircraft_pose)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(aircraft_pose);
	OpenTTDGroundFloodPose (*ground_pose)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(ground_pose);
	uint32_t (*crash)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(crash);
	void (*ai_event)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(ai_event);
	void (*game_event)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(game_event);
	void (*news)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(news);
	int32_t (*bridge_height)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(bridge_height);
};
const OpenTTDWaterGroundServices &GetRustWaterGroundServices() noexcept;
extern "C" {
void openttd_rust_water_tile_loop(uint32_t, uint32_t, uint32_t, bool, OpenTTDGroundCall *, const OpenTTDWaterGroundServices *, const OpenTTDSharedServices *);
bool openttd_rust_flood_halftile(uint32_t, OpenTTDGroundCall *, const OpenTTDWaterGroundServices *, const OpenTTDSharedServices *);
FloodingBehaviour openttd_rust_flooding_behaviour(uint32_t, const OpenTTDWaterGroundServices *, const OpenTTDSharedServices *);
void openttd_rust_clear_neighbour_flooding(uint32_t, uint32_t, uint32_t, const OpenTTDWaterGroundServices *, const OpenTTDSharedServices *);
}
/* Default native and Windows ABI call fixture: actual record returns and enum
 * parameters, including all-byte values, not just size/offset metadata. */
struct OpenTTDGroundABICalls {
	OpenTTDFieldFences (*fences)() noexcept OPENTTD_SERVICE_DEFAULT(fences);
	OpenTTDClearSurface (*surface)() noexcept OPENTTD_SERVICE_DEFAULT(surface);
	OpenTTDFieldGround (*field)() noexcept OPENTTD_SERVICE_DEFAULT(field);
	OpenTTDRailSnowGround (*rail_snow)() noexcept OPENTTD_SERVICE_DEFAULT(rail_snow);
	OpenTTDRailFenceGround (*rail_fence)() noexcept OPENTTD_SERVICE_DEFAULT(rail_fence);
	OpenTTDFoundationSlope (*foundation)() noexcept OPENTTD_SERVICE_DEFAULT(foundation);
	OpenTTDRoadSnowGround (*road_snow)() noexcept OPENTTD_SERVICE_DEFAULT(road_snow);
	OpenTTDTunnelGround (*tunnel)() noexcept OPENTTD_SERVICE_DEFAULT(tunnel);
	OpenTTDAirportGround (*airport)() noexcept OPENTTD_SERVICE_DEFAULT(airport);
	OpenTTDFloodVictim (*victim)() noexcept OPENTTD_SERVICE_DEFAULT(victim);
	OpenTTDAircraftFloodPose (*aircraft)() noexcept OPENTTD_SERVICE_DEFAULT(aircraft);
	OpenTTDGroundFloodPose (*ground)() noexcept OPENTTD_SERVICE_DEFAULT(ground);
	bool (*enums)(ClearGround, RailGroundType, Slope, TrackBits, Track, Foundation, Roadside, RoadBits, RoadType, TreeGround, WaterClass, FloodingBehaviour, DiagDirection, LandscapeType, VehicleType) noexcept OPENTTD_SERVICE_DEFAULT(enums);
};
extern "C" bool openttd_rust_ground_abi_calls(const OpenTTDGroundABICalls *);
extern "C" void openttd_rust_water_restore(uint32_t, uint8_t, const OpenTTDSharedServices *, const OpenTTDWaterGroundServices *, const OpenTTDWaterRestorationServices *);
extern "C" void openttd_rust_clear_square(uint32_t, const OpenTTDSharedServices *);
#endif /* RUST_GROUND_FFI_H */
