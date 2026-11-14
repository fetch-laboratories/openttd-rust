/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file rail_yapf_ffi.h The rail pathfinder's train controller entries. */
#ifndef OPENTTD_RUST_RAIL_YAPF_FFI_H
#define OPENTTD_RUST_RAIL_YAPF_FFI_H
#include <stdint.h>
#include "services_ffi.h"
#include "train_reservation_ffi.h"
extern "C" {
/** FindDepotData with the tile as its base integer. */
struct OpenTTDFindDepotData {
	uint32_t tile, best_length;
	uint8_t reverse;
};
/* Each entry runs synchronously under the game-state mutex. A non-null target or dest is read
 * before and written after the search. Bool results are 0 or 1. */
uint8_t openttd_rust_yapf_train_choose_track(const OpenTTDSharedServices *, const OpenTTDVehicle *, uint32_t tile, uint8_t enterdir, uint8_t tracks, bool *path_found, uint8_t reserve_track, OpenTTDTrainReservationPbs *target, uint32_t *dest);
uint8_t openttd_rust_yapf_train_check_reverse(const OpenTTDSharedServices *, const OpenTTDVehicle *);
OpenTTDFindDepotData openttd_rust_yapf_train_find_nearest_depot(const OpenTTDSharedServices *, const OpenTTDVehicle *, int32_t max_penalty);
uint8_t openttd_rust_yapf_train_find_nearest_safe_tile(const OpenTTDSharedServices *, const OpenTTDVehicle *, uint32_t tile, uint8_t td, uint8_t override_railtype);
void openttd_rust_rail_invalidate(uint32_t tile, uint8_t track);
}
/** The rail search's DumpState through the native DumpTarget, which #725 retires. */
void YapfRailDumpState(const uint32_t *words, uint32_t length) noexcept;
#endif
