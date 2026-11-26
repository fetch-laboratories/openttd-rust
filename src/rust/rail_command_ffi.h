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

/* Rust owns the command and tile-proc control flow and runs the nested rail commands; map
 * writers, pools, company counters and every CommandCost stay native. Reservations and
 * crossing updates may reenter Rust. Strong IDs cross as TileIndex uint32_t, CompanyID
 * uint8_t, DepotID uint16_t, TownID uint16_t; enums as their uint8_t value. No entry draws
 * Random. */
struct OpenTTDTrainServices;
enum RoadBits : uint8_t;
enum RoadType : uint8_t;
struct OpenTTDCrossingRoad { RoadBits bits; RoadType road_type, tram_type; uint16_t town; uint8_t road_owner, tram_owner; };
struct OpenTTDRailCommandServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDRailMapServices *rail;
	const OpenTTDCommandCostServices *cost;
	const OpenTTDCommandServices *command;
	const OpenTTDSignalServices *signal;
	const OpenTTDTrainServices *train;
	bool (*road_no_crossing)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(road_no_crossing);
	bool (*crossing_with_competitor)() noexcept OPENTTD_SERVICE_DEFAULT(crossing_with_competitor);
	bool (*entrance_edge)(uint32_t, int32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(entrance_edge);
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
void openttd_rust_rail_convert(const OpenTTDRailCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint32_t, uint8_t, bool);
void openttd_rust_rail_build_above(const OpenTTDRailCommandServices *, OpenTTDCommandCost *, uint16_t, uint32_t);
}
#endif /* RUST_RAIL_COMMAND_FFI_H */
