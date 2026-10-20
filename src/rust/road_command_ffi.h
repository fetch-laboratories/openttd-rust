/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file road_command_ffi.h Whole road command policy boundary (ABI 1020-1059). */
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
struct OpenTTDRoadConversionScratch {
	OpenTTDRoadConvertFrame *frame;
	OpenTTDRailConvertFrame *area;
	OpenTTDCommandCost *error, *ret;
};
struct OpenTTDRoadTownEdge { uint32_t tile, radius; };
struct OpenTTDRoadWorksEffect { uint32_t id, tile; };
struct OpenTTDRoadCommandStrings { uint32_t no_road, no_tram, junction, remove_road, no_suitable_road, no_suitable_tram, authority; };
struct OpenTTDRoadCommandServices {
	const OpenTTDRailCommandServices *rail_commands;
	const OpenTTDTownLeaves *town;
	const OpenTTDTownAuthorityServices *authority;
	OpenTTDRoadCommandStrings strings;
	bool (*vehicles_built)() noexcept;
	bool (*town_valid)(uint16_t) noexcept;
	const OpenTTDTownHandle *(*nearest_town)(uint32_t) noexcept;
	OpenTTDRoadTownEdge (*town_edge)(const OpenTTDTownHandle *) noexcept;
	void (*set_town)(uint32_t, uint16_t) noexcept;
	Foundation (*foundation)(uint8_t, uint8_t) noexcept;
	int64_t (*clear_cost)(uint8_t) noexcept;
	int64_t (*convert_cost)(uint8_t, uint8_t) noexcept;
	bool (*extra_dynamite)() noexcept;
	bool (*depot_slope)(uint8_t, uint8_t) noexcept;
	void (*make_depot)(uint32_t, uint8_t, uint16_t, uint8_t, uint8_t) noexcept;
	void (*depot_exit)(uint32_t, uint8_t) noexcept;
	OpenTTDRoadWorksEffect (*next_effect)(uint32_t) noexcept;
	void (*delete_effect)(uint32_t) noexcept;
	void (*owned_by_town)(OpenTTDCommandCost *, uint32_t) noexcept;
	OpenTTDCommandCost *(*scratch_cost)(OpenTTDRoadCommandScratch *) noexcept;
	OpenTTDRoadConversionScratch (*conversion_scratch)(OpenTTDRoadCommandScratch *) noexcept;
	void (*collect_vehicles)(OpenTTDRoadConvertFrame *, uint32_t) noexcept;
	void (*cargo_changed)(OpenTTDRoadConvertFrame *) noexcept;
};
const OpenTTDRoadCommandServices &GetRustRoadCommandServices() noexcept;
extern "C" {
void openttd_rust_road_do_build(const OpenTTDRoadCommandServices *, OpenTTDRoadCommandScratch *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, uint8_t, uint8_t, uint16_t);
bool openttd_rust_road_vehicles_built(const OpenTTDRoadCommandServices *);
void openttd_rust_road_infrastructure(const OpenTTDRoadCommandServices *, uint8_t, uint8_t, int32_t);
void openttd_rust_road_allow_remove(const OpenTTDRoadCommandServices *, OpenTTDCommandCost *, uint32_t, uint8_t, uint8_t, uint8_t, uint16_t, bool);
void openttd_rust_road_remove(const OpenTTDRoadCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, uint8_t, bool);
void openttd_rust_road_build(const OpenTTDRoadCommandServices *, OpenTTDRoadCommandScratch *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, uint8_t, uint8_t, uint16_t);
void openttd_rust_road_build_long(const OpenTTDRoadCommandServices *, OpenTTDRoadCommandScratch *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint32_t, uint8_t, uint8_t, uint8_t, bool, bool, bool);
int64_t openttd_rust_road_remove_long(const OpenTTDRoadCommandServices *, OpenTTDRoadCommandScratch *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint32_t, uint8_t, uint8_t, bool, bool);
void openttd_rust_road_build_depot(const OpenTTDRoadCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, uint8_t);
void openttd_rust_road_clear(const OpenTTDRoadCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t);
void openttd_rust_road_nearest_towns(const OpenTTDRoadCommandServices *, bool);
void openttd_rust_road_change_owner(const OpenTTDRoadCommandServices *, OpenTTDCommandCost *, uint32_t, uint8_t, uint8_t);
void openttd_rust_road_terraform(const OpenTTDRoadCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, int32_t, uint8_t);
void openttd_rust_road_convert(const OpenTTDRoadCommandServices *, OpenTTDRoadConvertFrame *, OpenTTDRailConvertFrame *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint32_t, uint8_t, bool);
void openttd_rust_road_build_above(const OpenTTDRoadCommandServices *, OpenTTDCommandCost *, uint16_t, uint32_t);
}
#endif /* RUST_ROAD_COMMAND_FFI_H */
