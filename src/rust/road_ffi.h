/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file road_ffi.h Canonical road vehicle controller and private state. */
#ifndef RUST_ROAD_FFI_H
#define RUST_ROAD_FFI_H
#include <cstdint>
#include <cstddef>
#include "services_ffi.h"
#include "vehicle_ffi.h"
/** Entries take a live vehicle handle and its owner; consist links carry both.
 * Records are copies taken at the original read points. Leaves are typed noexcept
 * services; visitors run synchronously in pool order and stop when they return
 * false. No owner or path borrow spans a service; panics abort. */
struct OpenTTDRoadState;
struct OpenTTDRoadPathElement { uint8_t trackdir; uint32_t tile; };
struct OpenTTDRoadEntry {
	uint8_t acceleration_model;
	uint8_t road_side;
	bool queue;
	uint8_t map_log_x;
};
struct OpenTTDRoadTickRead {
	uint8_t tick;
	bool front;
};
struct OpenTTDRoadFrontTickRead {
	OpenTTDRoadEntry movement;
	int32_t order_time;
	uint8_t running;
	uint8_t status;
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
	int32_t acceleration;
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
	uint32_t tile;
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
	uint32_t index;
	int32_t x;
	int32_t y;
	int32_t z;
	uint16_t first_speed;
	uint8_t direction;
};
struct OpenTTDRoadTileVehicle {
	OpenTTDVehicle *vehicle;
	OpenTTDVehicle *first;
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
using OpenTTDRoadCloseVisitor = bool (OPENTTD_VEHICLE_CALL *)(void *, const OpenTTDRoadCandidate *);
using OpenTTDRoadTrainVisitor = bool (OPENTTD_VEHICLE_CALL *)(void *, const int32_t *);
using OpenTTDRoadTileVisitor = bool (OPENTTD_VEHICLE_CALL *)(void *, const OpenTTDRoadTileVehicle *);
struct OpenTTDRoadLeaves {
	const OpenTTDVehicleServices *vehicle;
	const OpenTTDGroundServices *ground;
	const OpenTTDSharedServices *shared;
	OpenTTDRoadFrontTickRead (OPENTTD_VEHICLE_CALL *tick_front)(OpenTTDVehicle *) noexcept;
	uint32_t (OPENTTD_VEHICLE_CALL *max_depot_penalty)() noexcept;
	uint8_t (OPENTTD_VEHICLE_CALL *process_orders_then_loading)(OpenTTDVehicle *) noexcept;
	OpenTTDRoadSpeedRead (OPENTTD_VEHICLE_CALL *visual_then_speed)(OpenTTDVehicle *, bool) noexcept;
	OpenTTDRoadMaxSpeedRead (OPENTTD_VEHICLE_CALL *max_speed_read)(OpenTTDVehicle *) noexcept;
	OpenTTDRoadSpeedPart (OPENTTD_VEHICLE_CALL *speed_part)(OpenTTDVehicle *) noexcept;
	OpenTTDRoadStep (OPENTTD_VEHICLE_CALL *step)(OpenTTDVehicle *) noexcept;
	OpenTTDRoadCrashPart (OPENTTD_VEHICLE_CALL *crash_part)(OpenTTDVehicle *) noexcept;
	OpenTTDRoadPartRead (OPENTTD_VEHICLE_CALL *part)(OpenTTDVehicle *) noexcept;
	OpenTTDRoadLink (OPENTTD_VEHICLE_CALL *link)(OpenTTDVehicle *) noexcept;
	OpenTTDRoadLinkTile (OPENTTD_VEHICLE_CALL *link_tile)(OpenTTDVehicle *) noexcept;
	OpenTTDRoadEntered (OPENTTD_VEHICLE_CALL *enter_tile)(OpenTTDVehicle *, uint32_t, int32_t, int32_t) noexcept;
	void (OPENTTD_VEHICLE_CALL *visit_close)(OpenTTDVehicle *, int32_t, int32_t, bool, OpenTTDRoadCloseVisitor, void *) noexcept;
	void (OPENTTD_VEHICLE_CALL *visit_trains)(OpenTTDVehicle *, OpenTTDRoadTrainVisitor, void *) noexcept;
	void (OPENTTD_VEHICLE_CALL *visit_tile)(uint32_t, OpenTTDRoadTileVisitor, void *) noexcept;
	bool (OPENTTD_VEHICLE_CALL *has_road)(OpenTTDVehicle *, uint32_t) noexcept;
	uint32_t (OPENTTD_VEHICLE_CALL *track_status)(OpenTTDVehicle *, uint32_t) noexcept;
	uint8_t (OPENTTD_VEHICLE_CALL *any_road_bits)(OpenTTDVehicle *, uint32_t, bool) noexcept;
	bool (OPENTTD_VEHICLE_CALL *is_road_depot_tile)(uint32_t) noexcept;
	uint8_t (OPENTTD_VEHICLE_CALL *depot_direction)(uint32_t) noexcept;
	bool (OPENTTD_VEHICLE_CALL *is_normal_road)(uint32_t) noexcept;
	uint8_t (OPENTTD_VEHICLE_CALL *disallowed_directions)(uint32_t) noexcept;
	bool (OPENTTD_VEHICLE_CALL *is_bay_stop)(uint32_t) noexcept;
	uint8_t (OPENTTD_VEHICLE_CALL *bay_direction)(uint32_t) noexcept;
	bool (OPENTTD_VEHICLE_CALL *is_drive_through)(uint32_t) noexcept;
	bool (OPENTTD_VEHICLE_CALL *is_station_road_stop)(uint32_t) noexcept;
	uint8_t (OPENTTD_VEHICLE_CALL *stop_type)(uint32_t) noexcept;
	bool (OPENTTD_VEHICLE_CALL *has_free_bay)(uint32_t) noexcept;
	bool (OPENTTD_VEHICLE_CALL *continuation)(uint32_t, uint32_t) noexcept;
	OpenTTDRoadRoadTypes (OPENTTD_VEHICLE_CALL *move_tile)(OpenTTDVehicle *, uint32_t) noexcept;
	OpenTTDRoadPathRead (OPENTTD_VEHICLE_CALL *path_read)(OpenTTDVehicle *) noexcept;
	OpenTTDRoadTrackChoice (OPENTTD_VEHICLE_CALL *choose_track)(OpenTTDVehicle *, uint32_t, uint8_t, uint16_t) noexcept;
	OpenTTDRoadDepotResult (OPENTTD_VEHICLE_CALL *find_depot)(OpenTTDVehicle *, int32_t) noexcept;
	bool (OPENTTD_VEHICLE_CALL *can_build_tram)(OpenTTDVehicle *, uint32_t, uint8_t) noexcept;
	void (OPENTTD_VEHICLE_CALL *disconnect)() noexcept;
	OpenTTDRoadStopRead (OPENTTD_VEHICLE_CALL *stop_read)(OpenTTDVehicle *) noexcept;
	bool (OPENTTD_VEHICLE_CALL *should_stop)(OpenTTDVehicle *, uint32_t) noexcept;
	void (OPENTTD_VEHICLE_CALL *stop_leave)(OpenTTDVehicle *) noexcept;
	void (OPENTTD_VEHICLE_CALL *stop_entrance)(OpenTTDVehicle *, bool) noexcept;
	bool (OPENTTD_VEHICLE_CALL *stop_entrance_busy)(OpenTTDVehicle *) noexcept;
	OpenTTDRoadArrivalRead (OPENTTD_VEHICLE_CALL *arrival_read)(OpenTTDVehicle *, uint16_t) noexcept;
	void (OPENTTD_VEHICLE_CALL *arrival_news)(OpenTTDVehicle *, uint16_t, uint8_t, uint8_t) noexcept;
	void (OPENTTD_VEHICLE_CALL *begin_loading_at)(OpenTTDVehicle *, uint16_t) noexcept;
	OpenTTDRoadOvertakeRead (OPENTTD_VEHICLE_CALL *overtake_read)(OpenTTDVehicle *) noexcept;
	OpenTTDRoadSoundRead (OPENTTD_VEHICLE_CALL *sound_read)(OpenTTDVehicle *) noexcept;
	void (OPENTTD_VEHICLE_CALL *depot_exit)(OpenTTDVehicle *, int32_t, int32_t) noexcept;
	void (OPENTTD_VEHICLE_CALL *crash_news)(OpenTTDVehicle *, uint32_t) noexcept;
	uint8_t (OPENTTD_VEHICLE_CALL *shorten_factor)(const OpenTTDEngine *) noexcept;
	uint16_t (OPENTTD_VEHICLE_CALL *default_max_speed)(OpenTTDVehicle *) noexcept;
	void (OPENTTD_VEHICLE_CALL *update_cargo_age)(OpenTTDVehicle *) noexcept;
	OpenTTDRoadCostRead (OPENTTD_VEHICLE_CALL *cost_read)(OpenTTDVehicle *) noexcept;
	int64_t (OPENTTD_VEHICLE_CALL *price)(const OpenTTDEngine *, uint32_t) noexcept;
	void (OPENTTD_VEHICLE_CALL *pay_running)(OpenTTDVehicle *, int64_t) noexcept;
	OpenTTDRoadServiceRead (OPENTTD_VEHICLE_CALL *service_read)(OpenTTDVehicle *) noexcept;
	OpenTTDRoadTurnRead (OPENTTD_VEHICLE_CALL *turn_read)(OpenTTDVehicle *) noexcept;
	OpenTTDRoadTrackdirRead (OPENTTD_VEHICLE_CALL *trackdir_read)(OpenTTDVehicle *) noexcept;
};
extern "C" {
bool OPENTTD_VEHICLE_CALL openttd_rust_road_tick(OpenTTDVehicle *, OpenTTDRoadState *, const OpenTTDRoadTickRead *, const OpenTTDRoadLeaves *);
bool OPENTTD_VEHICLE_CALL openttd_rust_road_individual(OpenTTDVehicle *, OpenTTDRoadState *, OpenTTDVehicle *, OpenTTDRoadState *, const OpenTTDRoadEntry *, const OpenTTDRoadLeaves *);
bool OPENTTD_VEHICLE_CALL openttd_rust_road_leave_depot(OpenTTDVehicle *, OpenTTDRoadState *, bool, const OpenTTDRoadEntry *, const OpenTTDRoadLeaves *);
uint32_t OPENTTD_VEHICLE_CALL openttd_rust_road_crash(OpenTTDVehicle *, OpenTTDRoadState *, bool, const OpenTTDRoadLeaves *);
void OPENTTD_VEHICLE_CALL openttd_rust_road_update_cache(OpenTTDVehicle *, OpenTTDRoadState *, bool, const OpenTTDRoadLeaves *);
void OPENTTD_VEHICLE_CALL openttd_rust_road_calendar_day(OpenTTDVehicle *, bool, const OpenTTDRoadLeaves *);
void OPENTTD_VEHICLE_CALL openttd_rust_road_economy_day(OpenTTDVehicle *, OpenTTDRoadState *, bool, const OpenTTDRoadLeaves *);
int64_t OPENTTD_VEHICLE_CALL openttd_rust_road_running_cost(OpenTTDVehicle *, OpenTTDRoadState *, const OpenTTDRoadLeaves *);
int32_t OPENTTD_VEHICLE_CALL openttd_rust_road_max_speed(OpenTTDVehicle *, OpenTTDRoadState *, uint8_t, const OpenTTDRoadMaxSpeedRead *, const OpenTTDRoadLeaves *);
void OPENTTD_VEHICLE_CALL openttd_rust_road_set_dest(OpenTTDVehicle *, OpenTTDRoadState *, uint32_t, uint32_t, const OpenTTDRoadLeaves *);
bool OPENTTD_VEHICLE_CALL openttd_rust_road_turn(OpenTTDVehicle *, OpenTTDRoadState *, bool, const OpenTTDRoadLeaves *);
uint8_t OPENTTD_VEHICLE_CALL openttd_rust_road_trackdir(OpenTTDVehicle *, OpenTTDRoadState *, const OpenTTDRoadLeaves *);
bool OPENTTD_VEHICLE_CALL openttd_rust_road_slope_pixel(OpenTTDVehicle *, OpenTTDVehicle *, OpenTTDRoadState *, uint8_t, const OpenTTDRoadLeaves *);
/* Each ordinary or indexed-load shell owns one zero-created state. Destruction
 * follows PreDestructor and occurs exactly once, also during pool cleanup.
 * Scalars 0..6: state:u8, frame:u8, blocked:u16, overtaking:u8,
 * overtaking-counter:u8, crashed:u16, reverse:u8. Set narrows to source widths.
 * No returned references or persistent C++ cache. Save/load stages fields and
 * path outside Rust frames, commits partial loads on unwind and preserves nesting.
 * Path access requires the same index/nonempty preconditions as std::vector.
 * Replacing copies in forward order; count zero permits a null input.
 * Game-thread access only; panic/OOM abort and no exception crosses this ABI. */
OpenTTDRoadState *OPENTTD_VEHICLE_CALL openttd_rust_road_new();
void OPENTTD_VEHICLE_CALL openttd_rust_road_destroy(OpenTTDRoadState *);
uint16_t OPENTTD_VEHICLE_CALL openttd_rust_road_get(const OpenTTDRoadState *, uint8_t);
void OPENTTD_VEHICLE_CALL openttd_rust_road_set(OpenTTDRoadState *, uint8_t, uint16_t);
uint8_t OPENTTD_VEHICLE_CALL openttd_rust_road_stop_frame(uint32_t);
uint16_t OPENTTD_VEHICLE_CALL openttd_rust_road_drive_entry(uint8_t, uint8_t, uint8_t);
size_t OPENTTD_VEHICLE_CALL openttd_rust_road_path_size(const OpenTTDRoadState *);
OpenTTDRoadPathElement OPENTTD_VEHICLE_CALL openttd_rust_road_path_get(const OpenTTDRoadState *, size_t);
void OPENTTD_VEHICLE_CALL openttd_rust_road_path_replace(OpenTTDRoadState *, const OpenTTDRoadPathElement *, size_t);
void OPENTTD_VEHICLE_CALL openttd_rust_road_path_clear(OpenTTDRoadState *);
void OPENTTD_VEHICLE_CALL openttd_rust_road_path_push(OpenTTDRoadState *, OpenTTDRoadPathElement);
void OPENTTD_VEHICLE_CALL openttd_rust_road_path_pop(OpenTTDRoadState *);
}
#endif /* RUST_ROAD_FFI_H */
