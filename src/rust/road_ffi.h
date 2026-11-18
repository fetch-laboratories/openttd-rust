/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file road_ffi.h Canonical road vehicle controller and private state. */
#ifndef RUST_ROAD_FFI_H
#define RUST_ROAD_FFI_H
#include "service_defaults.hpp"
#include <cstdint>
#include <cstddef>
#include "services_ffi.h"
#include "vehicle_ffi.h"
#include "abi-record-road_state.h"
/** Entries take a live vehicle handle and its road owner. Both languages access the common,
 * ground and road fields in place in their canonical records. Leaves are typed noexcept services.
 * No owner or path borrow spans a service; panics abort. */
struct OpenTTDOrdersClosest;
struct OpenTTDRoadCommandContext;
struct OpenTTDRoadCommandSlot;
struct OpenTTDRoadPathElement { uint8_t trackdir; uint32_t tile; };
struct OpenTTDRoadLeaves {
	const OpenTTDRoadCommandContext *(OPENTTD_RUST_CALL *build_road_context)(OpenTTDRoadCommandSlot *) noexcept OPENTTD_SERVICE_DEFAULT(build_road_context);
	const OpenTTDGroundServices *ground;
	void (OPENTTD_RUST_CALL *roadveh_list_dirty)() noexcept OPENTTD_SERVICE_DEFAULT(roadveh_list_dirty);
	const OpenTTDSharedServices *shared;
	const OpenTTDVehicleServices *vehicle;
};
const OpenTTDRoadLeaves &GetRustRoadLeaves() noexcept;
extern "C" {
uint8_t OPENTTD_RUST_CALL openttd_rust_road_tick_abi(OpenTTDVehicle *, OpenTTDRoadState *, const OpenTTDRoadLeaves *, OpenTTDRoadCommandSlot *);
extern "C++" inline bool OPENTTD_RUST_CALL openttd_rust_road_tick(OpenTTDVehicle *a0, OpenTTDRoadState *a1, const OpenTTDRoadLeaves *a2, OpenTTDRoadCommandSlot *a3) { return openttd_rust_road_tick_abi(a0, a1, a2, a3) != 0; }
uint8_t OPENTTD_RUST_CALL openttd_rust_road_individual_abi(OpenTTDVehicle *, OpenTTDRoadState *, OpenTTDVehicle *, OpenTTDRoadState *, const OpenTTDRoadLeaves *, OpenTTDRoadCommandSlot *);
extern "C++" inline bool OPENTTD_RUST_CALL openttd_rust_road_individual(OpenTTDVehicle *a0, OpenTTDRoadState *a1, OpenTTDVehicle *a2, OpenTTDRoadState *a3, const OpenTTDRoadLeaves *a4, OpenTTDRoadCommandSlot *a5) { return openttd_rust_road_individual_abi(a0, a1, a2, a3, a4, a5) != 0; }
uint8_t OPENTTD_RUST_CALL openttd_rust_road_leave_depot_abi(OpenTTDVehicle *, OpenTTDRoadState *, bool, const OpenTTDRoadLeaves *);
extern "C++" inline bool OPENTTD_RUST_CALL openttd_rust_road_leave_depot(OpenTTDVehicle *a0, OpenTTDRoadState *a1, bool a2, const OpenTTDRoadLeaves *a3) { return openttd_rust_road_leave_depot_abi(a0, a1, a2, a3) != 0; }
uint32_t OPENTTD_RUST_CALL openttd_rust_road_crash(OpenTTDVehicle *, OpenTTDRoadState *, bool, const OpenTTDRoadLeaves *);
void OPENTTD_RUST_CALL openttd_rust_road_update_cache(OpenTTDVehicle *, OpenTTDRoadState *, bool, const OpenTTDRoadLeaves *);
void OPENTTD_RUST_CALL openttd_rust_road_calendar_day(OpenTTDVehicle *, OpenTTDRoadState *, const OpenTTDRoadLeaves *);
void OPENTTD_RUST_CALL openttd_rust_road_economy_day(OpenTTDVehicle *, OpenTTDRoadState *, const OpenTTDRoadLeaves *);
int64_t OPENTTD_RUST_CALL openttd_rust_road_running_cost(OpenTTDVehicle *, OpenTTDRoadState *, const OpenTTDRoadLeaves *);
int32_t OPENTTD_RUST_CALL openttd_rust_road_max_speed(OpenTTDVehicle *, OpenTTDRoadState *, const OpenTTDRoadLeaves *);
void OPENTTD_RUST_CALL openttd_rust_road_set_dest(OpenTTDVehicle *, OpenTTDRoadState *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_road_turn_abi(OpenTTDVehicle *, OpenTTDRoadState *, uint16_t, const OpenTTDRoadLeaves *);
extern "C++" inline bool OPENTTD_RUST_CALL openttd_rust_road_turn(OpenTTDVehicle *a0, OpenTTDRoadState *a1, uint16_t a2, const OpenTTDRoadLeaves *a3) { return openttd_rust_road_turn_abi(a0, a1, a2, a3) != 0; }
uint8_t OPENTTD_RUST_CALL openttd_rust_road_trackdir(OpenTTDVehicle *, OpenTTDRoadState *, const OpenTTDRoadLeaves *);
void OPENTTD_RUST_CALL openttd_rust_road_find_closest_depot(OpenTTDVehicle *, OpenTTDRoadState *, const OpenTTDRoadLeaves *, OpenTTDOrdersClosest *);
/* Each ordinary or indexed-load shell owns one zero-created state, destroyed once after
 * PreDestructor, also during pool cleanup. The generated fields are the allocation's
 * canonical prefix; its path remains private to Rust.
 * Save/load stages fields and path outside Rust frames and commits partial loads on unwind.
 * Path access has std::vector's index/nonempty preconditions; replacement copies in forward order,
 * with a null input allowed for zero count. */
OpenTTDRoadState *OPENTTD_RUST_CALL openttd_rust_road_new();
void OPENTTD_RUST_CALL openttd_rust_road_destroy(OpenTTDRoadState *);
uint8_t OPENTTD_RUST_CALL openttd_rust_road_stop_frame(uint32_t);
size_t OPENTTD_RUST_CALL openttd_rust_road_path_size(const OpenTTDRoadState *);
OpenTTDRoadPathElement OPENTTD_RUST_CALL openttd_rust_road_path_get(const OpenTTDRoadState *, size_t);
void OPENTTD_RUST_CALL openttd_rust_road_path_replace(OpenTTDRoadState *, const OpenTTDRoadPathElement *, size_t);
void OPENTTD_RUST_CALL openttd_rust_road_path_clear(OpenTTDRoadState *);
void OPENTTD_RUST_CALL openttd_rust_road_path_push(OpenTTDRoadState *, OpenTTDRoadPathElement);
void OPENTTD_RUST_CALL openttd_rust_road_path_pop(OpenTTDRoadState *);
}
#endif /* RUST_ROAD_FFI_H */
