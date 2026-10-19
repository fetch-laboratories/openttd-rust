/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
/** @file aircraft_ffi.h Aircraft-private ownership and copied shared services. */
#ifndef OPENTTD_RUST_AIRCRAFT_FFI_H
#define OPENTTD_RUST_AIRCRAFT_FFI_H
#include "services_ffi.h"
#include "vehicle_ffi.h"
#if defined(_MSC_VER)
#define OPENTTD_AIRCRAFT_CALL __cdecl
#elif defined(__i386__)
#define OPENTTD_AIRCRAFT_CALL __attribute__((cdecl))
#else
#define OPENTTD_AIRCRAFT_CALL
#endif
struct OpenTTDAircraftLeaves;
struct OpenTTDAircraftVehicle;
const OpenTTDAircraftLeaves &GetRustAircraftLeaves() noexcept;
OpenTTDAircraftVehicle GetRustAircraftVehicle(OpenTTDVehicle *) noexcept;

extern "C" {
struct OpenTTDAircraftState {
	uint32_t cached_max_range_sqr;
	uint16_t cached_max_range, cache_padding, crashed_counter, targetairport;
	uint8_t pos, previous_pos, state, last_direction, number_consecutive_turns, turn_counter, flags;
};
struct OpenTTDAircraftVehicle { OpenTTDVehicle *handle; OpenTTDAircraftState *state; uint32_t id; };
struct OpenTTDAircraftNode { const void *next; uint64_t blocks; uint8_t position, next_position, heading; };
struct OpenTTDAircraftMoving { int16_t x, y; uint16_t flags; uint8_t direction; };
struct OpenTTDAircraftBlockNode { uint64_t blocks; uint8_t position, next_position; };
struct OpenTTDAircraftRouteNode { const void *next; uint8_t next_position, heading; };
struct OpenTTDAircraftBlockChoice { const void *next; uint64_t blocks; uint8_t heading; };
/* Immutable callback table, borrowed for synchronous entries. Handles retain the
 * original pool identity; only raw field-sized State access occurs in Rust.
 * No borrow survives a callback, including reentry and PreDestructor deletion.
 * All native services are noexcept; environmental failures and Rust panics abort.
 * Flight helpers accept actual Vehicle identity, with null state for disasters. */
struct OpenTTDAircraftGeometry { uint32_t tile; uint8_t rotation; uint16_t width, height; bool airport; };
struct OpenTTDAircraftLayoutRoute { const void *node, *next; uint8_t next_position, heading; };
struct OpenTTDAircraftHangarCandidate { uint8_t owner; bool airport, hangar; };
struct OpenTTDAircraftLeaves {
	OpenTTDAircraftHangarCandidate (OPENTTD_AIRCRAFT_CALL *hangar_candidate)(const OpenTTDStation *, uint8_t) noexcept;
	OpenTTDAircraftGeometry (OPENTTD_AIRCRAFT_CALL *geometry)(const OpenTTDStation *) noexcept;
	uint64_t (OPENTTD_AIRCRAFT_CALL *layout_blocks)(const void *, uint8_t) noexcept;
	uint8_t (OPENTTD_AIRCRAFT_CALL *layout_heading)(const void *, uint8_t) noexcept;
	uint8_t (OPENTTD_AIRCRAFT_CALL *layout_next_position)(const void *, uint8_t) noexcept;
	const void *(OPENTTD_AIRCRAFT_CALL *layout_next)(const void *, uint8_t) noexcept;
	OpenTTDAircraftLayoutRoute (OPENTTD_AIRCRAFT_CALL *layout_route)(const void *, uint8_t) noexcept;
	const OpenTTDVehicleServices *vehicle;
	const OpenTTDSharedServices *shared;
	OpenTTDAircraftVehicle (OPENTTD_AIRCRAFT_CALL *next)(OpenTTDAircraftVehicle) noexcept;
	const OpenTTDStation *(OPENTTD_AIRCRAFT_CALL *station)(uint16_t) noexcept;
	uint32_t (OPENTTD_AIRCRAFT_CALL *airport_tile)(const OpenTTDStation *) noexcept;
	uint32_t (OPENTTD_AIRCRAFT_CALL *station_tile)(const OpenTTDStation *) noexcept;
	uint8_t (OPENTTD_AIRCRAFT_CALL *rotation)(const OpenTTDStation *) noexcept;
	uint8_t (OPENTTD_AIRCRAFT_CALL *airport_type)(const OpenTTDStation *) noexcept;
	uint8_t (OPENTTD_AIRCRAFT_CALL *station_owner)(const OpenTTDStation *) noexcept;
	bool (OPENTTD_AIRCRAFT_CALL *has_hangar)(const OpenTTDStation *) noexcept;
	const void * (OPENTTD_AIRCRAFT_CALL *airport_fta)(const OpenTTDStation *) noexcept;
	uint64_t * (OPENTTD_AIRCRAFT_CALL *airport_blocks)(const OpenTTDStation *) noexcept;
	uint8_t (OPENTTD_AIRCRAFT_CALL *had_vehicle)(const OpenTTDStation *) noexcept;
	const void * (OPENTTD_AIRCRAFT_CALL *dummy_airport)() noexcept;
	uint8_t (OPENTTD_AIRCRAFT_CALL *airport_elements)(const void *) noexcept;
	uint8_t (OPENTTD_AIRCRAFT_CALL *helipads)(const void *) noexcept;
	uint8_t (OPENTTD_AIRCRAFT_CALL *airport_flags)(const void *) noexcept;
	uint8_t (OPENTTD_AIRCRAFT_CALL *airport_delta_z)(const void *) noexcept;
	const void * (OPENTTD_AIRCRAFT_CALL *node)(const void *, uint8_t) noexcept;
	OpenTTDAircraftNode (OPENTTD_AIRCRAFT_CALL *fta)(const void *) noexcept;
	OpenTTDAircraftMoving (OPENTTD_AIRCRAFT_CALL *moving)(const void *, uint8_t) noexcept;
	uint16_t (OPENTTD_AIRCRAFT_CALL *engine_speed)(OpenTTDAircraftVehicle) noexcept;
	uint8_t (OPENTTD_AIRCRAFT_CALL *engine_subtype)(OpenTTDAircraftVehicle) noexcept;
	uint16_t (OPENTTD_AIRCRAFT_CALL *engine_sound)(OpenTTDAircraftVehicle) noexcept;
	uint16_t (OPENTTD_AIRCRAFT_CALL *engine_reliability)(OpenTTDAircraftVehicle) noexcept;
	uint8_t (OPENTTD_AIRCRAFT_CALL *airport_entry)(const void *, uint8_t) noexcept;
	uint8_t (OPENTTD_AIRCRAFT_CALL *direction_towards)(OpenTTDAircraftVehicle, int32_t, int32_t) noexcept;
	int32_t (OPENTTD_AIRCRAFT_CALL *hangar_height)(uint16_t) noexcept;
	uint8_t (OPENTTD_AIRCRAFT_CALL *terminal_count)(const void *, uint32_t) noexcept;
	uint8_t (OPENTTD_AIRCRAFT_CALL *hangar_exit)(OpenTTDAircraftVehicle) noexcept;
	bool (OPENTTD_AIRCRAFT_CALL *can_use_station)(OpenTTDAircraftVehicle, const OpenTTDStation *) noexcept;
	uint16_t (OPENTTD_AIRCRAFT_CALL *service_interval)(OpenTTDAircraftVehicle) noexcept;
	bool (OPENTTD_AIRCRAFT_CALL *chain_in_depot)(OpenTTDAircraftVehicle) noexcept;
	bool (OPENTTD_AIRCRAFT_CALL *nearest_depot_order)(OpenTTDAircraftVehicle) noexcept;
	bool (OPENTTD_AIRCRAFT_CALL *part_of_orders)(OpenTTDAircraftVehicle) noexcept;
	uint16_t (OPENTTD_AIRCRAFT_CALL *next_station)(uint16_t) noexcept;
	OpenTTDAircraftVehicle (OPENTTD_AIRCRAFT_CALL *next_aircraft)(uint32_t) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *update_position)(OpenTTDAircraftVehicle) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *rotor_image)(OpenTTDAircraftVehicle) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *copy_sprite)(OpenTTDAircraftVehicle, OpenTTDAircraftVehicle) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *position_viewport)(OpenTTDAircraftVehicle) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *create_effect)(OpenTTDAircraftVehicle, uint8_t, int32_t, int32_t, int32_t) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *truncate_cargo)(OpenTTDAircraftVehicle) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *crash_news)(OpenTTDAircraftVehicle, uint16_t, uint32_t) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *station_rating)(uint32_t, uint8_t, int32_t, uint32_t) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *landing_rating)(uint16_t, uint8_t) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *dirty_depot)(OpenTTDAircraftVehicle) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *first_arrival)(OpenTTDAircraftVehicle, uint16_t) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *begin_loading)(OpenTTDAircraftVehicle) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *dirty_details)(OpenTTDAircraftVehicle) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *update_delta)(OpenTTDAircraftVehicle) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *touchdown_animation)(OpenTTDAircraftVehicle, uint16_t) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *destination_too_far)(OpenTTDAircraftVehicle) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *delete_range_news)(OpenTTDAircraftVehicle) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *service_order)(OpenTTDAircraftVehicle, uint16_t) noexcept;
	int64_t (OPENTTD_AIRCRAFT_CALL *running_cost)(OpenTTDAircraftVehicle) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *subtract_cost)(OpenTTDAircraftVehicle, int64_t) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *dirty_lists)(OpenTTDAircraftVehicle) noexcept;
	uint16_t (OPENTTD_AIRCRAFT_CALL *next_stopping_station)(OpenTTDAircraftVehicle) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *remove_depot_orders)(uint16_t) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *assert_flying)(OpenTTDAircraftVehicle) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *invalid_movement)(OpenTTDAircraftVehicle) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *invalid_position)(OpenTTDAircraftVehicle, const void *) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *invalid_scheme)(OpenTTDAircraftVehicle) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *unreachable)(OpenTTDAircraftVehicle) noexcept;
	uint32_t (OPENTTD_AIRCRAFT_CALL *cargo_age_property)(OpenTTDAircraftVehicle) noexcept;
	uint32_t (OPENTTD_AIRCRAFT_CALL *range_property)(OpenTTDAircraftVehicle) noexcept;
	bool (OPENTTD_AIRCRAFT_CALL *touchdown_sound)(OpenTTDAircraftVehicle) noexcept;
	bool (OPENTTD_AIRCRAFT_CALL *rotor_image_if_changed)(OpenTTDAircraftVehicle) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *update_rotor_image)(OpenTTDAircraftVehicle) noexcept;
	void (OPENTTD_AIRCRAFT_CALL *process_orders)(OpenTTDAircraftVehicle) noexcept;
	uint32_t (OPENTTD_AIRCRAFT_CALL *vehicle_crash)(OpenTTDAircraftVehicle, bool) noexcept;
	bool (OPENTTD_AIRCRAFT_CALL *send_to_depot)(OpenTTDAircraftVehicle, bool) noexcept;
	uint8_t (OPENTTD_AIRCRAFT_CALL *fta_next_position)(const void *) noexcept;
	OpenTTDAircraftBlockNode (OPENTTD_AIRCRAFT_CALL *block_node)(const void *) noexcept;
	OpenTTDAircraftRouteNode (OPENTTD_AIRCRAFT_CALL *route_node)(const void *) noexcept;
	OpenTTDAircraftBlockChoice (OPENTTD_AIRCRAFT_CALL *block_choice)(const void *) noexcept;
};
bool OPENTTD_AIRCRAFT_CALL openttd_rust_aircraft_tick(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle);
void OPENTTD_AIRCRAFT_CALL openttd_rust_aircraft_calendar_day(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle);
void OPENTTD_AIRCRAFT_CALL openttd_rust_aircraft_economy_day(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle);
void OPENTTD_AIRCRAFT_CALL openttd_rust_aircraft_position(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle, int32_t, int32_t, int32_t);
void OPENTTD_AIRCRAFT_CALL openttd_rust_aircraft_enter_hangar(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle);
void OPENTTD_AIRCRAFT_CALL openttd_rust_aircraft_cache(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle, bool);
void OPENTTD_AIRCRAFT_CALL openttd_rust_aircraft_flight_bounds(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle, int32_t *, int32_t *);
int32_t OPENTTD_AIRCRAFT_CALL openttd_rust_aircraft_flight_level(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle, uint8_t *, bool);
int32_t OPENTTD_AIRCRAFT_CALL openttd_rust_aircraft_height(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle);
int32_t OPENTTD_AIRCRAFT_CALL openttd_rust_aircraft_hold_altitude(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle);
void OPENTTD_AIRCRAFT_CALL openttd_rust_aircraft_missing_orders(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle);
uint32_t OPENTTD_AIRCRAFT_CALL openttd_rust_aircraft_crash(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle, bool);
void OPENTTD_AIRCRAFT_CALL openttd_rust_aircraft_next_airport(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle);
void OPENTTD_AIRCRAFT_CALL openttd_rust_aircraft_leave_hangar(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle, uint8_t);
void OPENTTD_AIRCRAFT_CALL openttd_rust_aircraft_order_location(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle);
void OPENTTD_AIRCRAFT_CALL openttd_rust_aircraft_replacement(const OpenTTDAircraftLeaves *, uint16_t);
uint16_t OPENTTD_AIRCRAFT_CALL openttd_rust_aircraft_closest_depot(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle);
bool OPENTTD_AIRCRAFT_CALL openttd_rust_aircraft_event(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle, bool);
void OPENTTD_AIRCRAFT_CALL openttd_rust_aircraft_release_blocks(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle);
void OPENTTD_AIRCRAFT_CALL openttd_rust_aircraft_invalidate_target(const OpenTTDAircraftLeaves *, uint16_t);
OpenTTDAircraftState *OPENTTD_AIRCRAFT_CALL openttd_rust_aircraft_state_new();
void OPENTTD_AIRCRAFT_CALL openttd_rust_aircraft_state_destroy(OpenTTDAircraftState *);
uint64_t *OPENTTD_AIRCRAFT_CALL openttd_rust_airport_blocks_new();
void OPENTTD_AIRCRAFT_CALL openttd_rust_airport_blocks_destroy(uint64_t *);
}
#endif /* OPENTTD_RUST_AIRCRAFT_FFI_H */
