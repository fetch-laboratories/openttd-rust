/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file road_command_ffi.h Whole road command policy boundary (named layout metadata). */
#ifndef RUST_ROAD_COMMAND_FFI_H
#define RUST_ROAD_COMMAND_FFI_H
#include "rail_command_ffi.h"
#include "town_ffi.h"
/* Immutable tables outlive synchronous calls. Costs and conversion frames are
 * native stack storage. Rust holds no world borrow across direct noexcept
 * services, including nested commands, vehicle updates and town policy reentry.
 * No Rust panic or native exception unwinds over FFI; environmental failures abort.
 * Tile/pool shells, packed map, CommandCost and ordered affected list stay native. */
struct OpenTTDRoadConvertFrame;
struct OpenTTDRoadCommandScratch;
struct OpenTTDRoadCommandServices;
struct OpenTTDRoadCommandContext {
	const OpenTTDRoadCommandServices *services;
	OpenTTDRoadCommandScratch *scratch;
	OpenTTDCommandCost *cost, *ret;
};
struct OpenTTDRoadConversionScratch {
	OpenTTDRoadConvertFrame *frame;
	OpenTTDRailConvertFrame *area;
	OpenTTDCommandCost *error, *ret;
};
struct OpenTTDRoadNearestTownTile { bool road, depot, town_owned; };
struct OpenTTDRoadTownEdge { uint32_t tile, radius; };
struct OpenTTDRoadWorksEffect { uint32_t id, tile; };
struct OpenTTDRoadCommandStrings { uint32_t no_road, no_tram, junction, remove_road, no_suitable_road, no_suitable_tram, authority; };
struct OpenTTDGroundPhysicsServices;
struct OpenTTDRoadCommandServices {
	const OpenTTDGroundPhysicsServices *physics;
	void (*do_build)(const OpenTTDRoadCommandServices *, OpenTTDRoadCommandScratch *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, uint8_t, uint8_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(do_build);
	const OpenTTDRailCommandServices *rail_commands;
	const OpenTTDTownLeaves *town;
	const OpenTTDTownAuthorityServices *authority;
	OpenTTDRoadCommandStrings strings;
	OpenTTDRoadNearestTownTile (*nearest_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(nearest_tile);
	bool (*town_valid)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(town_valid);
	OpenTTDRoadTownEdge (*town_edge)(const OpenTTDTownHandle *) noexcept OPENTTD_SERVICE_DEFAULT(town_edge);
	void (*set_town)(uint32_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_town);
	bool (*extra_dynamite)() noexcept OPENTTD_SERVICE_DEFAULT(extra_dynamite);
	bool (*depot_slope)(uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_slope);
	void (*make_depot)(uint32_t, uint8_t, uint16_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(make_depot);
	void (*depot_exit)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_exit);
	OpenTTDRoadWorksEffect (*next_effect)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next_effect);
	void (*delete_effect)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(delete_effect);
	OpenTTDCommandCost *(*scratch_cost)(OpenTTDRoadCommandScratch *) noexcept OPENTTD_SERVICE_DEFAULT(scratch_cost);
	OpenTTDRoadConversionScratch (*conversion_scratch)(OpenTTDRoadCommandScratch *) noexcept OPENTTD_SERVICE_DEFAULT(conversion_scratch);
	void (*include_vehicle)(OpenTTDRoadConvertFrame *, OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(include_vehicle);
	size_t (*affected_count)(OpenTTDRoadConvertFrame *) noexcept OPENTTD_SERVICE_DEFAULT(affected_count);
	OpenTTDVehicle *(*affected_at)(OpenTTDRoadConvertFrame *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(affected_at);
};
const OpenTTDRoadCommandServices &GetRustRoadCommandServices() noexcept;
extern "C" {
void openttd_rust_road_do_build(const OpenTTDRoadCommandServices *, OpenTTDRoadCommandScratch *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, uint8_t, uint8_t, uint16_t) noexcept;
void openttd_rust_road_build(const OpenTTDRoadCommandServices *, OpenTTDRoadCommandScratch *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, uint8_t, uint8_t, uint16_t);
void openttd_rust_road_build_long(const OpenTTDRoadCommandServices *, OpenTTDRoadCommandScratch *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint32_t, uint8_t, uint8_t, uint8_t, bool, bool, bool);
int64_t openttd_rust_road_remove_long(const OpenTTDRoadCommandServices *, OpenTTDRoadCommandScratch *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint32_t, uint8_t, uint8_t, bool, bool);
void openttd_rust_road_build_depot(const OpenTTDRoadCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, uint8_t);
void openttd_rust_road_clear(const OpenTTDRoadCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t);
void openttd_rust_road_nearest_towns(const OpenTTDRoadCommandServices *, bool);
void openttd_rust_road_terraform(const OpenTTDRoadCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, int32_t, uint8_t);
void openttd_rust_road_convert(const OpenTTDRoadCommandServices *, OpenTTDRoadConvertFrame *, OpenTTDRailConvertFrame *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint32_t, uint8_t, bool);
void openttd_rust_road_build_above(const OpenTTDRoadCommandServices *, OpenTTDCommandCost *, uint16_t, uint32_t);
}
#endif /* RUST_ROAD_COMMAND_FFI_H */
