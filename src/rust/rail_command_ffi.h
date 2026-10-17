/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file rail_command_ffi.h Rail track, depot and signal command owner boundary (ABI 980-1019). */
#ifndef RUST_RAIL_COMMAND_FFI_H
#define RUST_RAIL_COMMAND_FFI_H
#include "services_ffi.h"
#include "command_services_ffi.h"
#include "signal_ffi.h"

/* Rust owns the command and tile-proc control flow; map, pools, company
 * counters and every CommandCost stay native. Each service is a direct
 * noexcept call. Nested commands, reservations and crossing updates may
 * reenter Rust; Rust holds no borrow of native state across any call.
 * Strong IDs cross as TileIndex uint32_t, CompanyID uint8_t, DepotID uint16_t,
 * TownID uint16_t; enums as their uint8_t value. No entry draws Random. */
enum Foundation : uint8_t;
enum FloodingBehaviour : uint8_t;
struct Train;
struct OpenTTDRailConvertFrame; ///< Caller-owned TileIterator and ordered affected-train list.
struct OpenTTDSlopeZ { int32_t z; uint8_t slope; };
struct OpenTTDRailTrainParking { uint8_t track; uint8_t status; uint16_t speed; uint8_t order; };
struct OpenTTDRailTrainPosition { uint32_t tile; uint8_t trackdir; };
struct OpenTTDCrossingRoad { uint8_t bits, road_type, tram_type; uint16_t town; uint8_t road_owner, tram_owner; };
struct OpenTTDRailCommandStrings {
	uint32_t impossible_combination, already_built, build_on_water, sloped_wrong, remove_signals_first;
	uint32_t road_works, oneway_road, crossing_rail, crossing_road, no_track, owned_by, flat_land;
	uint32_t demolish_bridge, no_suitable_track, no_signals, owned_by_another, remove_track;
	uint32_t demolish_building, ship_in_way;
};
struct OpenTTDRailCommandServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDRailMapServices *rail;
	const OpenTTDCommandCostServices *cost;
	const OpenTTDCommandServices *command;
	const OpenTTDSignalServices *signal;
	OpenTTDRailCommandStrings strings;
	bool (*rail_type_valid)(uint8_t) noexcept;
	bool (*compatible_rail)(uint8_t, uint8_t) noexcept;
	bool (*powered_rail)(uint8_t, uint8_t) noexcept;
	bool (*train_powered)(Train *, uint8_t) noexcept;
	bool (*rail_no_crossings)(uint8_t) noexcept;
	bool (*road_no_crossing)(uint8_t) noexcept;
	int64_t (*rail_build_cost)(uint8_t) noexcept;
	int64_t (*rail_clear_cost)(uint8_t) noexcept;
	int64_t (*rail_convert_cost)(uint8_t, uint8_t) noexcept;
	int64_t (*road_build_cost)(uint8_t) noexcept;
	bool (*crossing_with_competitor)() noexcept;
	bool (*disable_elrails)() noexcept;
	Foundation (*rail_foundation)(uint8_t, uint8_t) noexcept;
	OpenTTDSlopeZ (*apply_foundation)(uint8_t, uint8_t) noexcept;
	int32_t (*slope_z_in_corner)(uint8_t, uint8_t) noexcept;
	FloodingBehaviour (*flooding)(uint32_t) noexcept;
	bool (*entrance_edge)(uint32_t, int32_t, uint8_t, uint8_t) noexcept;
	bool (*ship_on_tile)(uint32_t) noexcept;
	void (*build_rail)(OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, uint8_t, bool) noexcept;
	void (*remove_rail)(OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t) noexcept;
	void (*build_signal)(OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, uint8_t, uint8_t, bool, bool, bool, uint8_t, uint8_t, uint8_t, uint8_t) noexcept;
	void (*remove_signal)(OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t) noexcept;
	void (*convert_rail)(OpenTTDCommandCost *, uint16_t, uint32_t, uint32_t, uint8_t, bool) noexcept;
	void (*rail_infrastructure)(uint8_t, uint8_t, uint32_t) noexcept;
	void (*road_infrastructure)(uint8_t, uint8_t, uint32_t) noexcept;
	void (*signal_infrastructure)(uint8_t, uint32_t) noexcept;
	void (*infrastructure_dirty)(uint8_t) noexcept;
	Train *(*reserving_train)(uint32_t, uint8_t) noexcept;
	OpenTTDRailTrainParking (*parking)(Train *) noexcept;
	OpenTTDRailTrainPosition (*train_position)(Train *) noexcept;
	void (*layout_changed)(uint32_t, uint8_t) noexcept;
	void (*mark_crossings)(uint32_t, uint8_t) noexcept;
	void (*crossing_removed)(uint32_t, uint8_t) noexcept;
	void (*make_crossing)(uint32_t, uint8_t, uint8_t, uint8_t, uint8_t, uint8_t, uint8_t, uint8_t, uint16_t) noexcept;
	void (*make_road)(uint32_t, OpenTTDCrossingRoad) noexcept;
	bool (*depot_available)() noexcept;
	uint16_t (*new_depot)(uint32_t) noexcept;
	void (*depot_name)(uint16_t) noexcept;
	void (*delete_depot)(uint32_t) noexcept;
	void (*depot_windows)(uint32_t) noexcept;
	void (*close_inspect)(uint32_t) noexcept;
	void (*make_shore)(uint32_t) noexcept;
	void (*docking_check)(uint32_t) noexcept;
	void (*area_begin)(OpenTTDRailConvertFrame *, uint32_t, uint32_t, bool) noexcept;
	uint32_t (*area_tile)(OpenTTDRailConvertFrame *) noexcept;
	void (*area_next)(OpenTTDRailConvertFrame *) noexcept;
	bool (*area_contains)(uint32_t, uint32_t, bool, uint32_t) noexcept;
	void (*collect_trains)(OpenTTDRailConvertFrame *, uint32_t) noexcept;
	void (*consist_changed)(OpenTTDRailConvertFrame *) noexcept;
};
const OpenTTDRailCommandServices &GetRustRailCommandServices() noexcept;

extern "C" {
void openttd_rust_rail_build_single(const OpenTTDRailCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, uint8_t, bool);
void openttd_rust_rail_remove_single(const OpenTTDRailCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t);
void openttd_rust_rail_track_drag(const OpenTTDRailCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint32_t, uint8_t, uint8_t, bool, bool, bool);
void openttd_rust_rail_build_depot(const OpenTTDRailCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, uint8_t);
void openttd_rust_rail_build_signal(const OpenTTDRailCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, uint8_t, uint8_t, bool, bool, bool, uint8_t, uint8_t, uint8_t, uint8_t);
void openttd_rust_rail_signal_drag(const OpenTTDRailCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint32_t, uint8_t, uint8_t, uint8_t, bool, bool, bool, bool, int32_t);
void openttd_rust_rail_remove_signal(const OpenTTDRailCommandServices *, OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t);
void openttd_rust_rail_convert(const OpenTTDRailCommandServices *, OpenTTDRailConvertFrame *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint32_t, uint8_t, bool);
void openttd_rust_rail_clear_tile(const OpenTTDRailCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t);
void openttd_rust_rail_change_owner(const OpenTTDRailCommandServices *, OpenTTDCommandCost *, uint32_t, uint8_t, uint8_t);
void openttd_rust_rail_terraform(const OpenTTDRailCommandServices *, OpenTTDCommandCost *, uint16_t, uint32_t, int32_t, uint8_t);
void openttd_rust_rail_build_above(const OpenTTDRailCommandServices *, OpenTTDCommandCost *, uint16_t, uint32_t);
}
#endif /* RUST_RAIL_COMMAND_FFI_H */
