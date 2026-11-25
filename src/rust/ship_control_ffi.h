/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file ship_control_ffi.h Canonical ship controller and private state. */
#ifndef OPENTTD_SHIP_CONTROL_FFI_H
#define OPENTTD_SHIP_CONTROL_FFI_H
#include "service_defaults.hpp"
#include "services_ffi.h"
struct OpenTTDVehicle;
struct OpenTTDVehicleServices;
struct OpenTTDOrdersClosest;
struct OpenTTDShipPath;
#include <cstdint>
struct OpenTTDShipState;
/** One step of Depot::Iterate. */
struct OpenTTDShipDepot { uint16_t index; uint32_t xy; };
struct OpenTTDShipLeaves {
	bool (*next_depot)(uint32_t, OpenTTDShipDepot *) noexcept OPENTTD_SERVICE_DEFAULT(next_depot);
	void (*update_viewport)(OpenTTDVehicle *, bool, bool) noexcept OPENTTD_SERVICE_DEFAULT(update_viewport);
	const OpenTTDVehicleServices *vehicle; ///< Base vehicle table, immutable for the process.
};
const OpenTTDShipLeaves &GetRustShipServices() noexcept;
extern "C" {
/* Game-thread shell ownership; new zeroes state/coordinates and sets INVALID_DIR.
 * Typed fields are state:u8, rotation:u8, rotation-x:i16, rotation-y:i16.
 * Accessors retain no borrow; callers narrow exactly like C++.
 * Modern/legacy staging commits partial loads on unwind outside Rust frames.
 * Panics/OOM abort. One destruction follows PreDestructor, including pool cleanup. */
void openttd_rust_ship_control_reload_rotation();
OpenTTDShipState *openttd_rust_ship_state_new();
void openttd_rust_ship_state_destroy(OpenTTDShipState *);
OpenTTDShipPath *openttd_rust_ship_state_path(OpenTTDShipState *);
uint8_t openttd_rust_ship_get_state(const OpenTTDShipState *);
void openttd_rust_ship_set_state(OpenTTDShipState *, uint8_t);
uint8_t openttd_rust_ship_get_rotation(const OpenTTDShipState *);
void openttd_rust_ship_set_rotation(OpenTTDShipState *, uint8_t);
int16_t openttd_rust_ship_get_rotation_x(const OpenTTDShipState *);
void openttd_rust_ship_set_rotation_x(OpenTTDShipState *, int16_t);
int16_t openttd_rust_ship_get_rotation_y(const OpenTTDShipState *);
void openttd_rust_ship_set_rotation_y(OpenTTDShipState *, int16_t);
/* Plain synchronous calls over a live ship record. No entry task allocation or retained
 * world/owner borrow. Every service is noexcept; unexpected environmental exceptions terminate.
 * The selected services do not unwind through script VMs or save/load during play. */
uint8_t openttd_rust_ship_tick_abi(OpenTTDVehicle *, const OpenTTDShipLeaves *, const OpenTTDSharedServices *);
extern "C++" inline bool openttd_rust_ship_tick(OpenTTDVehicle *a0, const OpenTTDShipLeaves *a1, const OpenTTDSharedServices *a2) { return openttd_rust_ship_tick_abi(a0, a1, a2) != 0; }
void openttd_rust_ship_cache(OpenTTDVehicle *, const OpenTTDShipLeaves *, const OpenTTDSharedServices *);
void openttd_rust_ship_destination(OpenTTDVehicle *, uint32_t);
void openttd_rust_ship_find_depot(OpenTTDVehicle *, const OpenTTDShipLeaves *, const OpenTTDSharedServices *, OpenTTDOrdersClosest *);
uint32_t openttd_rust_ship_station_destination(OpenTTDVehicle *, uint16_t, const OpenTTDShipLeaves *, const OpenTTDSharedServices *);
}
#endif
