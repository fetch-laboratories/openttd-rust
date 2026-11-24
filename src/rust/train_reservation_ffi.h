/*
	* This file is part of OpenTTD.
	* OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
	* OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
	* See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
	*/

/** @file train_reservation_ffi.h The train path reservation entries C++ still calls. */
#ifndef OPENTTD_RUST_TRAIN_RESERVATION_FFI_H
#define OPENTTD_RUST_TRAIN_RESERVATION_FFI_H
#include <stdint.h>
#include "services_ffi.h"
#include "train_ffi.h"
extern "C" {
/* Each entry runs synchronously under the game-state mutex on a live train part and the
 * process-lifetime train and shared tables. Callbacks may reenter; escaping exceptions
 * and Rust panics terminate. Bool results are 0 or 1. */
void openttd_rust_train_reservation_check_next(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_reservation_free(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
uint8_t openttd_rust_train_reservation_try_path(OpenTTDTrainHandle, uint8_t, uint8_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
uint32_t openttd_rust_train_reservation_station_location(OpenTTDTrainHandle, uint16_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_reservation_reserve_under(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
}
#endif
