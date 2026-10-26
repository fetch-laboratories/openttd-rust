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
 * Records are copies taken at the original read points. Leaves are typed noexcept
 * services; visitors run synchronously in pool order and stop when they return
 * false. Ground pointers name separate stable allocations, surviving controller
 * callbacks that cannot destroy the current shell; advancing a changed chain
 * reacquires both owners. No owner or path borrow spans a service; panics abort. */
struct OpenTTDRoadState;
struct OpenTTDGroundState;
struct OpenTTDRoadCommandContext;
struct OpenTTDRoadCommandSlot;
struct OpenTTDRoadTramBuildRead { uint8_t owner, road_type; const OpenTTDRoadCommandContext *road; };
struct OpenTTDRoadPathElement { uint8_t trackdir; uint32_t tile; };
struct OpenTTDRoadEntry {
	OpenTTDGroundState *ground;
	uint8_t acceleration_model;
	uint8_t road_side;
	bool queue;
	uint8_t map_log_x;
};
struct OpenTTDRoadTickRead {
	uint8_t tick;
	bool front;
	OpenTTDRoadEntry movement;
	const int32_t *order_time;
	const uint8_t *running;
	const uint8_t *status;
};
struct OpenTTDRoadMaxSpeedRead {
	OpenTTDVehicle *next;
	OpenTTDRoadState *next_state;
	uint32_t tile;
	uint16_t max_track_speed;
	uint16_t order_max_speed;
	uint8_t direction;
	uint8_t status;
};
struct OpenTTDRoadSpeedRead {
	OpenTTDRoadMaxSpeedRead max;
	int32_t x;
	int32_t y;
	int32_t z;

	uint8_t length;
	bool tram;
};
struct OpenTTDRoadSpeedPart {
	OpenTTDVehicle *next;
	OpenTTDRoadState *next_state;
	uint32_t tile;
	uint8_t direction;
	uint8_t status;
};
struct OpenTTDRoadLink {
	OpenTTDVehicle *next;
	OpenTTDRoadState *next_state;
	OpenTTDGroundState *next_ground;
};
struct OpenTTDRoadLinkTile {
	OpenTTDRoadLink link;
	uint32_t tile;
};
struct OpenTTDRoadStep {
	uint32_t tile;
	int32_t x;
	int32_t y;
	int32_t z;
	uint16_t cur_speed;
	uint8_t direction;
	uint8_t length;
	bool front;
	bool tram;
};
struct OpenTTDRoadCrashPart {
	OpenTTDVehicle *next;
	OpenTTDRoadState *next_state;
	int32_t z;
	bool crossing;
};
struct OpenTTDRoadEntered {
	uint32_t tile;
	uint16_t cur_speed;
	uint8_t flags;
	uint8_t direction;
	uint8_t order_type;
};
struct OpenTTDRoadCandidate {
	OpenTTDVehicle *vehicle;
	OpenTTDRoadState *state;
	OpenTTDVehicle *first;
	OpenTTDRoadState *first_state;
	OpenTTDGroundState *first_ground;
	uint32_t index;
	int32_t x;
	int32_t y;
	int32_t z;
	uint16_t first_speed;
	uint8_t direction;
};
struct OpenTTDRoadPartRead {
	OpenTTDVehicle *next;
	OpenTTDRoadState *next_state;
	uint32_t tile;
	int32_t z;
	bool tram;
};
struct OpenTTDRoadPathRead {
	uint32_t dest;
	uint8_t owner;
	bool articulated;
	bool bus;
};
struct OpenTTDRoadStopRead {
	uint8_t owner;
	bool bus;
};
struct OpenTTDRoadOvertakeRead {
	uint32_t tile;
	uint16_t cur_speed;
	uint8_t direction;
	uint8_t status;
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
struct OpenTTDRoadArrivalRead {
	uint8_t had_vehicle_of_type;
	bool bus;
};
struct OpenTTDRoadSoundRead {
	uint16_t sound;
	uint8_t tick;
};
struct OpenTTDRoadCostRead {
	const OpenTTDEngine *engine;
	uint8_t cost_class;
	uint8_t cost_factor;
};
struct OpenTTDRoadServiceRead {
	uint32_t tile;
	uint16_t servint;
	uint16_t cur_speed;
};
struct OpenTTDRoadTurnRead {
	uint32_t tile;
	uint8_t breakdown_ctr;
	uint8_t direction;
	uint8_t order_type;
	uint8_t status;
};
struct OpenTTDRoadTrackdirRead {
	uint32_t tile;
	uint8_t direction;
	uint8_t status;
};
struct OpenTTDRoadLeaves {
	bool (OPENTTD_RUST_CALL *candidate)(OpenTTDVehicle *, OpenTTDVehicle *, OpenTTDRoadCandidate *) noexcept OPENTTD_SERVICE_DEFAULT(candidate);
	const OpenTTDVehicleServices *vehicle;
	const OpenTTDGroundServices *ground;
	const OpenTTDSharedServices *shared;
	uint32_t (OPENTTD_RUST_CALL *max_depot_penalty)() noexcept OPENTTD_SERVICE_DEFAULT(max_depot_penalty);
	OpenTTDRoadSpeedRead (OPENTTD_RUST_CALL *speed_read)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(speed_read);
	OpenTTDRoadMaxSpeedRead (OPENTTD_RUST_CALL *max_speed_read)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(max_speed_read);
	OpenTTDRoadSpeedPart (OPENTTD_RUST_CALL *speed_part)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(speed_part);
	OpenTTDRoadStep (OPENTTD_RUST_CALL *step)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(step);
	OpenTTDRoadCrashPart (OPENTTD_RUST_CALL *crash_part)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(crash_part);
	OpenTTDRoadPartRead (OPENTTD_RUST_CALL *part)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(part);
	OpenTTDRoadLink (OPENTTD_RUST_CALL *link)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(link);
	OpenTTDRoadLinkTile (OPENTTD_RUST_CALL *link_tile)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(link_tile);
	OpenTTDRoadEntered (OPENTTD_RUST_CALL *after_enter_tile)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(after_enter_tile);
	bool (OPENTTD_RUST_CALL *has_road)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(has_road);
	uint8_t (OPENTTD_RUST_CALL *any_road_bits)(OpenTTDVehicle *, uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(any_road_bits);
	bool (OPENTTD_RUST_CALL *is_road_depot_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_road_depot_tile);
	uint8_t (OPENTTD_RUST_CALL *depot_direction)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_direction);
	bool (OPENTTD_RUST_CALL *is_normal_road)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_normal_road);
	uint8_t (OPENTTD_RUST_CALL *disallowed_directions)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(disallowed_directions);
	bool (OPENTTD_RUST_CALL *is_bay_stop)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_bay_stop);
	uint8_t (OPENTTD_RUST_CALL *bay_direction)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(bay_direction);
	bool (OPENTTD_RUST_CALL *is_drive_through)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_drive_through);
	bool (OPENTTD_RUST_CALL *is_station_road_stop)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_station_road_stop);
	OpenTTDRoadRoadTypes (OPENTTD_RUST_CALL *move_tile)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(move_tile);
	OpenTTDRoadPathRead (OPENTTD_RUST_CALL *path_read)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(path_read);
	OpenTTDRoadTrackChoice (OPENTTD_RUST_CALL *choose_track)(OpenTTDVehicle *, uint32_t, uint8_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(choose_track);
	OpenTTDRoadDepotResult (OPENTTD_RUST_CALL *find_depot)(OpenTTDVehicle *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(find_depot);
	OpenTTDRoadTramBuildRead (OPENTTD_RUST_CALL *tram_build_read)(OpenTTDVehicle *, OpenTTDRoadCommandSlot *) noexcept OPENTTD_SERVICE_DEFAULT(tram_build_read);
	void (OPENTTD_RUST_CALL *disconnect)() noexcept OPENTTD_SERVICE_DEFAULT(disconnect);
	OpenTTDRoadStopRead (OPENTTD_RUST_CALL *stop_read)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(stop_read);
	bool (OPENTTD_RUST_CALL *should_stop)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(should_stop);
	OpenTTDRoadArrivalRead (OPENTTD_RUST_CALL *arrival_read)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(arrival_read);
	void (OPENTTD_RUST_CALL *arrival_news)(OpenTTDVehicle *, uint16_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(arrival_news);

	OpenTTDRoadOvertakeRead (OPENTTD_RUST_CALL *overtake_read)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(overtake_read);
	OpenTTDRoadSoundRead (OPENTTD_RUST_CALL *sound_read)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(sound_read);
	void (OPENTTD_RUST_CALL *depot_exit_dirty)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(depot_exit_dirty);
	void (OPENTTD_RUST_CALL *depot_exit)(OpenTTDVehicle *, int32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_exit);
	void (OPENTTD_RUST_CALL *crash_news)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(crash_news);
	uint8_t (OPENTTD_RUST_CALL *shorten_factor)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(shorten_factor);
	uint16_t (OPENTTD_RUST_CALL *default_max_speed)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(default_max_speed);
	uint16_t (OPENTTD_RUST_CALL *cargo_age_default)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_age_default);
	OpenTTDRoadCostRead (OPENTTD_RUST_CALL *cost_read)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cost_read);
	int64_t (OPENTTD_RUST_CALL *price)(const OpenTTDEngine *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(price);
	void (OPENTTD_RUST_CALL *pay_running)(OpenTTDVehicle *, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(pay_running);
	OpenTTDRoadServiceRead (OPENTTD_RUST_CALL *service_read)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(service_read);
	OpenTTDRoadTurnRead (OPENTTD_RUST_CALL *turn_read)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(turn_read);
	OpenTTDRoadTrackdirRead (OPENTTD_RUST_CALL *trackdir_read)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(trackdir_read);
};
const OpenTTDRoadLeaves &GetRustRoadLeaves() noexcept;
extern "C" {
bool OPENTTD_RUST_CALL openttd_rust_road_tick(OpenTTDVehicle *, OpenTTDRoadState *, const OpenTTDRoadTickRead *, const OpenTTDRoadLeaves *, OpenTTDRoadCommandSlot *);
bool OPENTTD_RUST_CALL openttd_rust_road_individual(OpenTTDVehicle *, OpenTTDRoadState *, OpenTTDVehicle *, OpenTTDRoadState *, const OpenTTDRoadEntry *, const OpenTTDRoadLeaves *, OpenTTDRoadCommandSlot *);
bool OPENTTD_RUST_CALL openttd_rust_road_leave_depot(OpenTTDVehicle *, OpenTTDRoadState *, bool, const OpenTTDRoadEntry *, const OpenTTDRoadLeaves *);
uint32_t OPENTTD_RUST_CALL openttd_rust_road_crash(OpenTTDVehicle *, OpenTTDRoadState *, bool, const OpenTTDRoadLeaves *);
void OPENTTD_RUST_CALL openttd_rust_road_update_cache(OpenTTDVehicle *, OpenTTDRoadState *, bool, const OpenTTDRoadLeaves *);
void OPENTTD_RUST_CALL openttd_rust_road_calendar_day(OpenTTDVehicle *, bool, const OpenTTDRoadLeaves *);
void OPENTTD_RUST_CALL openttd_rust_road_economy_day(OpenTTDVehicle *, OpenTTDRoadState *, bool, const OpenTTDRoadLeaves *);
int64_t OPENTTD_RUST_CALL openttd_rust_road_running_cost(OpenTTDVehicle *, OpenTTDRoadState *, const OpenTTDRoadLeaves *);
int32_t OPENTTD_RUST_CALL openttd_rust_road_max_speed(OpenTTDVehicle *, OpenTTDRoadState *, uint8_t, const OpenTTDRoadMaxSpeedRead *, const OpenTTDRoadLeaves *);
void OPENTTD_RUST_CALL openttd_rust_road_set_dest(OpenTTDVehicle *, OpenTTDRoadState *, uint32_t, uint32_t, const OpenTTDRoadLeaves *);
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
