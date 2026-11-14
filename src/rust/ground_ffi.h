/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file ground_ffi.h Consumed terrain fields and direct synchronous leaves. */
#ifndef RUST_GROUND_FFI_H
#define RUST_GROUND_FFI_H
struct AirportFTAClass;
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
struct OpenTTDFlatTileHeight { int32_t height; bool flat; };
struct OpenTTDClearServices {
	void (*set_ground)(uint32_t, ClearGround, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_ground);
	void (*set_counter)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_counter);
	void (*clear_snow)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(clear_snow);
	void (*make_clear)(uint32_t, ClearGround, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(make_clear);
	void (*set_field)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_field);
};
struct OpenTTDRailServices {
	TrackBits (*tracks)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tracks);
	RailGroundType (*ground)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(ground);
	void (*set_ground)(uint32_t, RailGroundType) noexcept OPENTTD_SERVICE_DEFAULT(set_ground);
	bool (*owned_land)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(owned_land);
};
struct OpenTTDTunnelServices {
	bool (*snow_or_desert)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(snow_or_desert);
	void (*set_snow_or_desert)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_snow_or_desert);
};
extern "C" void openttd_rust_clear_tile(OpenTTDCommandCost *, uint32_t, uint16_t, const OpenTTDCommandCostServices *, const OpenTTDSharedServices *);
struct OpenTTDGroundCall; // Native optional CommandCost, independently owned by each facade invocation.
struct OpenTTDGroundCostServices {
	const OpenTTDCommandServices *commands;
	const OpenTTDCommandCostServices *costs;
	OpenTTDCommandCost *(*prepare)(OpenTTDGroundCall *) noexcept OPENTTD_SERVICE_DEFAULT(prepare);
	void (*release)(OpenTTDGroundCall *) noexcept OPENTTD_SERVICE_DEFAULT(release);
};
struct OpenTTDTownLeaves;
struct OpenTTDRoadCommandContext;
struct OpenTTDRoadGroundServices {
	bool (*snow_or_desert)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(snow_or_desert);
	void (*toggle_snow_or_desert)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(toggle_snow_or_desert);
	Roadside (*roadside)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(roadside);
	void (*set_roadside)(uint32_t, Roadside) noexcept OPENTTD_SERVICE_DEFAULT(set_roadside);
	RoadBits (*all_bits)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(all_bits);
	const OpenTTDTownLeaves *town;
};
const OpenTTDGroundCostServices &GetRustGroundCostServices() noexcept;
const OpenTTDRoadGroundServices &GetRustRoadGroundServices() noexcept;
const OpenTTDClearServices &GetRustClearServices() noexcept;
const OpenTTDRailServices &GetRustRailGroundServices() noexcept;
const OpenTTDTunnelServices &GetRustTunnelGroundServices() noexcept;
struct OpenTTDWaterRestorationServices {
	void (*make_canal)(uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(make_canal);
	void (*make_river)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(make_river);
};
const OpenTTDWaterRestorationServices &GetRustWaterRestorationServices() noexcept;
struct OpenTTDWaterGroundServices;
struct OpenTTDVehicle;
struct OpenTTDVehicleServices;
struct OpenTTDAirportGround { const OpenTTDStation *station; uint32_t tile; uint16_t width; uint16_t height; };
struct OpenTTDCompanyServices;
struct OpenTTDWaterGroundServices {
	const OpenTTDRailServices *rail;
	const OpenTTDClearServices *clear;
	const OpenTTDVehicleServices *vehicles;
	const OpenTTDGroundCostServices *costs;
	const OpenTTDCompanyServices *company; ///< Current-company exchange.
	OpenTTDSignalContext (*signal_context)() noexcept OPENTTD_SERVICE_DEFAULT(signal_context);
	void (*set_non_flooding)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_non_flooding);
	void (*set_tree_ground)(uint32_t, TreeGround, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_tree_ground);
	void (*make_sea)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(make_sea);
	bool (*buoy_or_dock)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(buoy_or_dock);
	OpenTTDWaterRegions *(*water_regions)() noexcept OPENTTD_SERVICE_DEFAULT(water_regions);
	void (*remove_rail)(OpenTTDCommandCost *, uint32_t, Track) noexcept OPENTTD_SERVICE_DEFAULT(remove_rail);
	bool (*airport_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(airport_tile);
	OpenTTDAirportGround (*airport)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(airport);
	bool (*airport_member)(const OpenTTDStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(airport_member);
	int32_t (*airport_z)(const AirportFTAClass *) noexcept OPENTTD_SERVICE_DEFAULT(airport_z);
	uint32_t (*crash)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(crash);
	void (*ai_event)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(ai_event);
	void (*game_event)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(game_event);
	void (*news)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(news);
};
const OpenTTDWaterGroundServices &GetRustWaterGroundServices() noexcept;
extern "C" {
void openttd_rust_water_tile_loop(uint32_t, uint32_t, uint32_t, OpenTTDGroundCall *, const OpenTTDWaterGroundServices *, const OpenTTDSharedServices *);
}
/* Default native and Windows ABI call fixture: actual record returns and enum
 * parameters, including all-byte values, not just size/offset metadata. */
struct OpenTTDGroundABICalls {
	OpenTTDFoundationSlope (*foundation)() noexcept OPENTTD_SERVICE_DEFAULT(foundation);
	OpenTTDAirportGround (*airport)() noexcept OPENTTD_SERVICE_DEFAULT(airport);
	bool (*enums)(ClearGround, RailGroundType, Slope, TrackBits, Track, Foundation, Roadside, RoadBits, RoadType, TreeGround, WaterClass, FloodingBehaviour, DiagDirection, LandscapeType, VehicleType) noexcept OPENTTD_SERVICE_DEFAULT(enums);
};
extern "C" uint8_t openttd_test_ground_abi_calls(const OpenTTDGroundABICalls *);
extern "C" void openttd_rust_water_restore(uint32_t, uint8_t, const OpenTTDSharedServices *, const OpenTTDWaterGroundServices *, const OpenTTDWaterRestorationServices *);
extern "C" void openttd_rust_clear_square(uint32_t, const OpenTTDSharedServices *);
#endif /* RUST_GROUND_FFI_H */
