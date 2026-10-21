/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file water_command_ffi.h Water construction, clearing and restoration owner. */
#ifndef RUST_WATER_COMMAND_FFI_H
#define RUST_WATER_COMMAND_FFI_H
#include "ground_ffi.h"
struct OpenTTDRailCommandServices;
enum class WaterTileType : uint8_t;
enum class LockPart : uint8_t;
enum class DepotPart : uint8_t;
enum Axis : uint8_t;
struct OpenTTDWaterArea;
/* All result slots and map/pool identities remain native. Calls are serialized
 * by the game-state mutex, synchronous and noexcept. No native map/pool borrow
 * survives a callback. Nested landscape commands retain Command::Do lifecycle;
 * ordinary play calls no throwing script VM/save service. Environmental errors
 * terminate, and Rust panics abort. The area is native stack storage, no heap. */
struct OpenTTDWaterCommandStrings {
	uint32_t built_on_water, demolish_bridge, unsuitable, sloped_wrong, edge, low_bridge, flat, already_built, build_on_water, demolish_building, demolish_canal;
};
struct OpenTTDWaterCommandServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDCommandCostServices *cost;
	const OpenTTDCommandServices *command;
	const OpenTTDWaterGroundServices *water;
	const OpenTTDRailCommandServices *rail;
	WaterTileType (*water_type)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(water_type);
	LockPart (*lock_part)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(lock_part);
	DiagDirection (*lock_direction)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(lock_direction);
	uint32_t (*other_depot_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(other_depot_tile);
	OpenTTDWaterRestorationServices restoration;
	OpenTTDEnvironmentServices environment;
	void (*make_depot)(uint32_t, uint8_t, uint16_t, DepotPart, Axis, WaterClass) noexcept OPENTTD_SERVICE_DEFAULT(make_depot);
	void (*make_lock_tile)(uint32_t, uint8_t, LockPart, DiagDirection, WaterClass) noexcept OPENTTD_SERVICE_DEFAULT(make_lock_tile);
	void (*area_begin)(OpenTTDWaterArea *, uint32_t, uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(area_begin);
	uint32_t (*area_tile)(OpenTTDWaterArea *) noexcept OPENTTD_SERVICE_DEFAULT(area_tile);
	void (*area_next)(OpenTTDWaterArea *) noexcept OPENTTD_SERVICE_DEFAULT(area_next);
	OpenTTDWaterCommandStrings strings;
};
const OpenTTDWaterCommandServices &GetRustWaterCommandServices() noexcept;
extern "C" {
void openttd_rust_water_build_depot(const OpenTTDWaterCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, Axis);
void openttd_rust_water_build_lock(const OpenTTDWaterCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t);
void openttd_rust_water_build_canal(const OpenTTDWaterCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDWaterArea *, uint16_t, uint32_t, uint32_t, WaterClass, bool);
void openttd_rust_water_make_river(const OpenTTDWaterCommandServices *, uint32_t);
void openttd_rust_water_clear(const OpenTTDWaterCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t);
void openttd_rust_water_change_owner(const OpenTTDWaterCommandServices *, OpenTTDCommandCost *, uint32_t, uint8_t, uint8_t);
void openttd_rust_water_terraform(const OpenTTDWaterCommandServices *, OpenTTDCommandCost *, uint16_t, uint32_t);
void openttd_rust_water_build_above(const OpenTTDWaterCommandServices *, OpenTTDCommandCost *, uint16_t, uint32_t, int32_t);
}
#endif /* RUST_WATER_COMMAND_FFI_H */
