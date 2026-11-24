struct OpenTTDEngineCapabilitiesServices;
/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file train_ffi.h Complete train owner shared-world boundary. */
#ifndef RUST_TRAIN_FFI_H
#define RUST_TRAIN_FFI_H
#include "service_defaults.hpp"
#include <cstdint>
#include <cstddef>
#include "train_state_ffi.h"
#include "vehicle_ffi.h"
#include "services_ffi.h"
struct OpenTTDVehicleServices;
struct OpenTTDTrainReservationLeaves;
struct OpenTTDOrdersClosest;
struct OpenTTDVehicleBuildServices;
struct OpenTTDCommandCost;
struct OpenTTDGroundPhysicsServices;
/* Handles carry the canonical record and train owner. Both remain live until
 * their source deletion point; no Rust borrow spans a native callback. */
struct Train;
struct OpenTTDTrainServices;
const OpenTTDTrainServices &GetTrainServices() noexcept;
OpenTTDTrainHandle TrainHandle(const Train *) noexcept;
extern "C" {
struct OpenTTDTrainServices {
	const OpenTTDEngineCapabilitiesServices *capabilities;
	const OpenTTDGroundPhysicsServices *physics;
	const OpenTTDTrainReservationLeaves *reservation;
	const OpenTTDVehicleServices *vehicle; ///< Base vehicle table, immutable for the process.
};

void openttd_rust_train_consist_changed(OpenTTDTrainHandle, uint8_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
int32_t openttd_rust_train_current_max_speed(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
int32_t openttd_rust_train_update_speed(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_mark_dirty(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
uint8_t openttd_rust_train_tick(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_calendar_day(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_economy_day(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
int64_t openttd_rust_train_running_cost(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
uint8_t openttd_rust_train_trackdir(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_mark_stuck(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_reverse_swap(OpenTTDTrainHandle, int32_t, int32_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_update_crossing(uint32_t, uint8_t, uint8_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_adjacent_crossing_dirty(uint32_t, uint8_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_crossing_removed(uint32_t, uint8_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
uint8_t openttd_rust_train_controller(OpenTTDTrainHandle, OpenTTDTrainHandle, uint8_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
uint32_t openttd_rust_train_crash(OpenTTDTrainHandle, uint8_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_needs_service(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_find_closest_depot(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *, OpenTTDOrdersClosest *);
void openttd_rust_train_reverse_direction(const OpenTTDVehicleBuildServices *, OpenTTDCommandCost *, uint16_t, uint32_t, bool, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_force_proceed(const OpenTTDVehicleBuildServices *, OpenTTDCommandCost *, uint16_t, uint32_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
}
#endif
