/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file waypoint_command_ffi.h Synchronous waypoint commands on native stack costs. */
#ifndef RUST_WAYPOINT_COMMAND_FFI_H
#define RUST_WAYPOINT_COMMAND_FFI_H
#include "station_command_ffi.h"
extern "C" {
uint8_t openttd_rust_waypoint_rail_axis(const OpenTTDStationCommandServices *, uint32_t);
uint8_t openttd_rust_waypoint_road_axis(const OpenTTDStationCommandServices *, uint32_t);
void openttd_rust_waypoint_rename(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, uint16_t, uint16_t, const uint8_t *, size_t);
uint16_t openttd_rust_waypoint_move_name(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, uint16_t, uint16_t, uint32_t);
void openttd_rust_waypoint_build_rail(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDWaypointLayout *, uint16_t, uint32_t, uint8_t, uint8_t, uint8_t, uint16_t, uint16_t, uint16_t, bool);
void openttd_rust_waypoint_build_road(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, uint8_t, uint8_t, uint16_t, uint16_t, uint16_t, bool);
void openttd_rust_waypoint_build_buoy(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t);
}
#endif /* RUST_WAYPOINT_COMMAND_FFI_H */
