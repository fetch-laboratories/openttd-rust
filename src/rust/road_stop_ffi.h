/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file road_stop_ffi.h Road stop bays, entrance status and drive-through entries owned by Rust. */
#ifndef RUST_ROAD_STOP_FFI_H
#define RUST_ROAD_STOP_FFI_H
#include "service_defaults.hpp"
#include <cstdint>
#include "services_ffi.h"
#include "vehicle_ffi.h"

struct RoadStop;
struct BaseStation;
struct OpenTTDRoadState;
/** One drive-through entry point; Rust owns every instance, native readers only load. */
struct OpenTTDRoadStopEntry {
	uint16_t length; ///< The length of the stop in tile 'units'
	uint16_t occupied; ///< The amount of occupied stop in tile 'units'
	inline int GetLength() const { return this->length; }
	inline int GetOccupied() const { return this->occupied; }
};
/** Both entry points of a drive-through line, owned by its BaseEntry stop. */
struct OpenTTDRoadStopEntries { OpenTTDRoadStopEntry east, west; };
/** One Rust allocation per native shell: the saved status byte and the shared entries. */
struct OpenTTDRoadStopState { uint8_t status; OpenTTDRoadStopEntries *entries; };
struct OpenTTDRoadStopPoolItem { OpenTTDRoadStopState *state; uint32_t xy, index; };
struct OpenTTDRoadStopDriveRead { uint16_t total_length; uint8_t direction; };
struct OpenTTDRoadStopRebuildRead { OpenTTDRoadState *road; uint16_t total_length; VehicleType vehicle_type; uint8_t direction, status; bool primary; };
/* Typed noexcept services; none reenters a road stop, runs scripts or throws. */
struct OpenTTDRoadStopServices {
	const OpenTTDVehicleServices *vehicle;
	const OpenTTDGroundServices *ground;
	RoadStop *(*primary)(uint32_t, RoadStopType) noexcept OPENTTD_SERVICE_DEFAULT(primary);
	RoadStop *(*head)(BaseStation *, bool) noexcept OPENTTD_SERVICE_DEFAULT(head);
	bool (*is_bus)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(is_bus);
	RoadStop *(*next)(const RoadStop *) noexcept OPENTTD_SERVICE_DEFAULT(next);
	uint32_t (*tile)(const RoadStop *) noexcept OPENTTD_SERVICE_DEFAULT(tile);
	OpenTTDRoadStopState *(*state)(const RoadStop *) noexcept OPENTTD_SERVICE_DEFAULT(state);
	OpenTTDRoadStopPoolItem (*pool_next)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(pool_next);
	OpenTTDRoadStopDriveRead (OPENTTD_RUST_CALL *drive_read)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(drive_read);
	OpenTTDRoadStopRebuildRead (OPENTTD_RUST_CALL *rebuild_read)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(rebuild_read);
};
const OpenTTDRoadStopServices &GetRustRoadStopServices() noexcept;
extern "C" {
/* Each ordinary or indexed-load shell owns one state with both bays free;
 * destroy frees the shared entries when the state holds BaseEntry. Entries
 * take live serialized owners; panics abort and no exception crosses. */
OpenTTDRoadStopState *openttd_rust_road_stop_new();
void openttd_rust_road_stop_destroy(OpenTTDRoadStopState *);
bool openttd_rust_road_stop_enter(const OpenTTDSharedServices *, uint32_t, OpenTTDVehicle *, OpenTTDRoadState *);
void openttd_rust_road_stop_leave(const OpenTTDSharedServices *, OpenTTDVehicle *, OpenTTDRoadState *);
void openttd_rust_road_stop_check_integrity(const OpenTTDSharedServices *, uint32_t, OpenTTDRoadStopState *, uint8_t);
void openttd_rust_road_stop_after_load(const OpenTTDSharedServices *);
}
#endif /* RUST_ROAD_STOP_FFI_H */
