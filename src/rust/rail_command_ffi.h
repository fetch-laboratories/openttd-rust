/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file rail_command_ffi.h Rail track, depot and signal command owner boundary (ABI 980-1019). */
#ifndef RUST_RAIL_COMMAND_FFI_H
#define RUST_RAIL_COMMAND_FFI_H
#include "service_defaults.hpp"
#include "services_ffi.h"
#include "command_services_ffi.h"
#include "signal_ffi.h"

/* Rust owns the command and tile-proc control flow; map, pools, company counters and every
 * CommandCost stay native. Nested commands, reservations and crossing updates may reenter Rust.
 * Strong IDs cross as TileIndex uint32_t, CompanyID uint8_t, DepotID uint16_t, TownID uint16_t;
 * enums as their uint8_t value. No entry draws Random. */
enum Foundation : uint8_t;
enum FloodingBehaviour : uint8_t;
struct Train;
struct OpenTTDVehicle;
struct OpenTTDRailConvertFrame; ///< Caller-owned TileIterator and ordered affected-train list.
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
	bool (*rail_type_valid)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_type_valid);
	bool (*compatible_rail)(uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(compatible_rail);
	bool (*powered_rail)(uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(powered_rail);
	bool (*train_powered)(Train *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(train_powered);
	bool (*rail_no_crossings)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_no_crossings);
	bool (*road_no_crossing)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(road_no_crossing);
	int64_t (*rail_build_cost)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_build_cost);
	int64_t (*rail_clear_cost)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_clear_cost);
	int64_t (*rail_convert_cost)(uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_convert_cost);
	bool (*crossing_with_competitor)() noexcept OPENTTD_SERVICE_DEFAULT(crossing_with_competitor);
	bool (*disable_elrails)() noexcept OPENTTD_SERVICE_DEFAULT(disable_elrails);
	int32_t (*slope_z_in_corner)(uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(slope_z_in_corner);
	bool (*entrance_edge)(uint32_t, int32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(entrance_edge);
	void (*build_rail)(OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, uint8_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(build_rail);
	void (*remove_rail)(OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(remove_rail);
	void (*build_signal)(OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, uint8_t, uint8_t, bool, bool, bool, uint8_t, uint8_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(build_signal);
	void (*remove_signal)(OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(remove_signal);
	void (*convert_rail)(OpenTTDCommandCost *, uint16_t, uint32_t, uint32_t, uint8_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(convert_rail);
	void (*rail_infrastructure)(uint8_t, uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_infrastructure);
	void (*road_infrastructure)(uint8_t, uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(road_infrastructure);
	void (*signal_infrastructure)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(signal_infrastructure);
	OpenTTDRailTrainParking (*parking)(Train *) noexcept OPENTTD_SERVICE_DEFAULT(parking);
	OpenTTDRailTrainPosition (*train_position)(Train *) noexcept OPENTTD_SERVICE_DEFAULT(train_position);
	void (*layout_changed)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(layout_changed);
	void (*mark_crossings)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(mark_crossings);
	void (*crossing_removed)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(crossing_removed);
	void (*make_crossing)(uint32_t, uint8_t, uint8_t, uint8_t, uint8_t, uint8_t, uint8_t, uint8_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(make_crossing);
	void (*make_road)(uint32_t, OpenTTDCrossingRoad) noexcept OPENTTD_SERVICE_DEFAULT(make_road);
	bool (*depot_available)() noexcept OPENTTD_SERVICE_DEFAULT(depot_available);
	uint16_t (*new_depot)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(new_depot);
	void (*depot_name)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_name);
	void (*delete_depot)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(delete_depot);
	void (*depot_windows)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_windows);
	void (*close_inspect)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(close_inspect);
	void (*make_shore)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(make_shore);
	void (*area_begin)(OpenTTDRailConvertFrame *, uint32_t, uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(area_begin);
	uint32_t (*area_tile)(OpenTTDRailConvertFrame *) noexcept OPENTTD_SERVICE_DEFAULT(area_tile);
	void (*area_next)(OpenTTDRailConvertFrame *) noexcept OPENTTD_SERVICE_DEFAULT(area_next);
	bool (*area_contains)(uint32_t, uint32_t, bool, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(area_contains);
	void (*include_train)(OpenTTDRailConvertFrame *, OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(include_train);
	void (*consist_changed)(OpenTTDRailConvertFrame *) noexcept OPENTTD_SERVICE_DEFAULT(consist_changed);
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
void openttd_rust_rail_build_above(const OpenTTDRailCommandServices *, OpenTTDCommandCost *, uint16_t, uint32_t);
}
#endif /* RUST_RAIL_COMMAND_FFI_H */
