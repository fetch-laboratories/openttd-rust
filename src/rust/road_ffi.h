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
/** Entries take a live vehicle handle and its owner; consist links carry both.
 * Both languages access common and ground fields in their canonical records. Leaves are typed noexcept
 * services; visitors run synchronously in pool order and stop when they return
 * false. Ground pointers name separate stable allocations, surviving controller
 * callbacks that cannot destroy the current shell; advancing a changed chain
 * reacquires both owners. No owner or path borrow spans a service; panics abort. */
struct OpenTTDRoadState;
struct OpenTTDGroundState;
struct OpenTTDRoadCommandContext;
struct OpenTTDRoadCommandSlot;
struct OpenTTDRoadTramBuildRead { uint8_t road_type; const OpenTTDRoadCommandContext *road; };
struct OpenTTDRoadPathElement { uint8_t trackdir; uint32_t tile; };
struct OpenTTDRoadEntry {
	OpenTTDGroundState *ground;
	uint8_t acceleration_model;
	uint8_t road_side;
	bool queue;
	uint8_t map_log_x;
};
struct OpenTTDRoadRoadTypes {
	uint8_t before;
	uint8_t after;
};
struct OpenTTDRoadTrackChoice {
	uint8_t trackdir;
	bool found;
};
struct OpenTTDRoadDepotResult {
	uint32_t tile;
	uint32_t length;
};
struct OpenTTDRoadLeaves {
	const OpenTTDVehicleServices *vehicle;
	const OpenTTDGroundServices *ground;
	const OpenTTDSharedServices *shared;
	uint32_t (OPENTTD_RUST_CALL *max_depot_penalty)() noexcept OPENTTD_SERVICE_DEFAULT(max_depot_penalty);
	uint8_t (OPENTTD_RUST_CALL *any_road_bits)(OpenTTDVehicle *, uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(any_road_bits);
	bool (OPENTTD_RUST_CALL *is_road_depot_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_road_depot_tile);
	uint8_t (OPENTTD_RUST_CALL *depot_direction)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_direction);
	uint8_t (OPENTTD_RUST_CALL *disallowed_directions)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(disallowed_directions);
	bool (OPENTTD_RUST_CALL *is_bay_stop)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_bay_stop);
	uint8_t (OPENTTD_RUST_CALL *bay_direction)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(bay_direction);
	bool (OPENTTD_RUST_CALL *is_drive_through)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_drive_through);
	bool (OPENTTD_RUST_CALL *is_station_road_stop)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_station_road_stop);
	OpenTTDRoadRoadTypes (OPENTTD_RUST_CALL *tile_road_types)(OpenTTDVehicle *, uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_road_types);
	OpenTTDRoadTrackChoice (OPENTTD_RUST_CALL *choose_track)(OpenTTDVehicle *, uint32_t, uint8_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(choose_track);
	OpenTTDRoadDepotResult (OPENTTD_RUST_CALL *find_depot)(OpenTTDVehicle *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(find_depot);
	OpenTTDRoadTramBuildRead (OPENTTD_RUST_CALL *tram_build_read)(OpenTTDVehicle *, OpenTTDRoadCommandSlot *) noexcept OPENTTD_SERVICE_DEFAULT(tram_build_read);
	void (OPENTTD_RUST_CALL *disconnect)() noexcept OPENTTD_SERVICE_DEFAULT(disconnect);
	bool (OPENTTD_RUST_CALL *should_stop)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(should_stop);
	void (OPENTTD_RUST_CALL *arrival_news)(OpenTTDVehicle *, uint16_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(arrival_news);

	void (OPENTTD_RUST_CALL *depot_exit_dirty)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(depot_exit_dirty);
	void (OPENTTD_RUST_CALL *crash_news)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(crash_news);
	uint8_t (OPENTTD_RUST_CALL *shorten_factor)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(shorten_factor);
	uint16_t (OPENTTD_RUST_CALL *default_max_speed)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(default_max_speed);
	uint16_t (OPENTTD_RUST_CALL *cargo_age_default)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_age_default);
	void (OPENTTD_RUST_CALL *pay_running)(OpenTTDVehicle *, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(pay_running);
};
const OpenTTDRoadLeaves &GetRustRoadLeaves() noexcept;
extern "C" {
bool OPENTTD_RUST_CALL openttd_rust_road_tick(OpenTTDVehicle *, OpenTTDRoadState *, const OpenTTDRoadEntry *, const OpenTTDRoadLeaves *, OpenTTDRoadCommandSlot *);
bool OPENTTD_RUST_CALL openttd_rust_road_individual(OpenTTDVehicle *, OpenTTDRoadState *, OpenTTDVehicle *, OpenTTDRoadState *, const OpenTTDRoadEntry *, const OpenTTDRoadLeaves *, OpenTTDRoadCommandSlot *);
bool OPENTTD_RUST_CALL openttd_rust_road_leave_depot(OpenTTDVehicle *, OpenTTDRoadState *, bool, const OpenTTDRoadEntry *, const OpenTTDRoadLeaves *);
uint32_t OPENTTD_RUST_CALL openttd_rust_road_crash(OpenTTDVehicle *, OpenTTDRoadState *, bool, const OpenTTDRoadLeaves *);
void OPENTTD_RUST_CALL openttd_rust_road_update_cache(OpenTTDVehicle *, OpenTTDRoadState *, bool, const OpenTTDRoadLeaves *);
void OPENTTD_RUST_CALL openttd_rust_road_calendar_day(OpenTTDVehicle *, bool, const OpenTTDRoadLeaves *);
void OPENTTD_RUST_CALL openttd_rust_road_economy_day(OpenTTDVehicle *, OpenTTDRoadState *, bool, const OpenTTDRoadLeaves *);
int64_t OPENTTD_RUST_CALL openttd_rust_road_running_cost(OpenTTDVehicle *, OpenTTDRoadState *, const OpenTTDRoadLeaves *);
int32_t OPENTTD_RUST_CALL openttd_rust_road_max_speed(OpenTTDVehicle *, OpenTTDRoadState *, uint8_t, const OpenTTDRoadLeaves *);
void OPENTTD_RUST_CALL openttd_rust_road_set_dest(OpenTTDVehicle *, OpenTTDRoadState *, uint32_t);
bool OPENTTD_RUST_CALL openttd_rust_road_turn(OpenTTDVehicle *, OpenTTDRoadState *, bool, const OpenTTDRoadLeaves *);
uint8_t OPENTTD_RUST_CALL openttd_rust_road_trackdir(OpenTTDVehicle *, OpenTTDRoadState *, const OpenTTDRoadLeaves *);
/* Each ordinary or indexed-load shell owns one zero-created state, destroyed once after
 * PreDestructor, also during pool cleanup. Scalars 0..6: state:u8, frame:u8, blocked:u16,
 * overtaking:u8, overtaking-counter:u8, crashed:u16, reverse:u8; set narrows to source widths.
 * Save/load stages fields and path outside Rust frames and commits partial loads on unwind.
 * Path access has std::vector's index/nonempty preconditions; replacement copies in forward order,
 * with a null input allowed for zero count. */
OpenTTDRoadState *OPENTTD_RUST_CALL openttd_rust_road_new();
void OPENTTD_RUST_CALL openttd_rust_road_destroy(OpenTTDRoadState *);
uint16_t OPENTTD_RUST_CALL openttd_rust_road_get(const OpenTTDRoadState *, uint8_t);
void OPENTTD_RUST_CALL openttd_rust_road_set(OpenTTDRoadState *, uint8_t, uint16_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_road_stop_frame(uint32_t);
uint16_t OPENTTD_RUST_CALL openttd_rust_road_drive_entry(uint8_t, uint8_t, uint8_t);
size_t OPENTTD_RUST_CALL openttd_rust_road_path_size(const OpenTTDRoadState *);
OpenTTDRoadPathElement OPENTTD_RUST_CALL openttd_rust_road_path_get(const OpenTTDRoadState *, size_t);
void OPENTTD_RUST_CALL openttd_rust_road_path_replace(OpenTTDRoadState *, const OpenTTDRoadPathElement *, size_t);
void OPENTTD_RUST_CALL openttd_rust_road_path_clear(OpenTTDRoadState *);
void OPENTTD_RUST_CALL openttd_rust_road_path_push(OpenTTDRoadState *, OpenTTDRoadPathElement);
void OPENTTD_RUST_CALL openttd_rust_road_path_pop(OpenTTDRoadState *);
}
#endif /* RUST_ROAD_FFI_H */
