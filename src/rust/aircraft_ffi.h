/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
/** @file aircraft_ffi.h Aircraft-private ownership and copied shared services. */
#ifndef OPENTTD_RUST_AIRCRAFT_FFI_H
#define OPENTTD_RUST_AIRCRAFT_FFI_H
#include "service_defaults.hpp"
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
struct Vehicle;
OpenTTDAircraftVehicle OPENTTD_AIRCRAFT_CALL RustAircraftNext(OpenTTDAircraftVehicle) noexcept;
uint16_t OPENTTD_AIRCRAFT_CALL RustAircraftEngineReliability(OpenTTDAircraftVehicle) noexcept;
OpenTTDAircraftVehicle RustAircraftVehicle(Vehicle *) noexcept;
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
	OpenTTDAircraftHangarCandidate (OPENTTD_AIRCRAFT_CALL *hangar_candidate)(const OpenTTDStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(hangar_candidate);
	OpenTTDAircraftGeometry (OPENTTD_AIRCRAFT_CALL *geometry)(const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(geometry);
	uint64_t (OPENTTD_AIRCRAFT_CALL *layout_blocks)(const void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(layout_blocks);
	uint8_t (OPENTTD_AIRCRAFT_CALL *layout_heading)(const void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(layout_heading);
	uint8_t (OPENTTD_AIRCRAFT_CALL *layout_next_position)(const void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(layout_next_position);
	const void *(OPENTTD_AIRCRAFT_CALL *layout_next)(const void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(layout_next);
	OpenTTDAircraftLayoutRoute (OPENTTD_AIRCRAFT_CALL *layout_route)(const void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(layout_route);
	const OpenTTDVehicleServices *vehicle;
	const OpenTTDSharedServices *shared;
	OpenTTDAircraftVehicle (OPENTTD_AIRCRAFT_CALL *next)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(next);
	const OpenTTDStation *(OPENTTD_AIRCRAFT_CALL *station)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(station);
	uint32_t (OPENTTD_AIRCRAFT_CALL *airport_tile)(const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(airport_tile);
	uint32_t (OPENTTD_AIRCRAFT_CALL *station_tile)(const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(station_tile);
	uint8_t (OPENTTD_AIRCRAFT_CALL *rotation)(const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(rotation);
	uint8_t (OPENTTD_AIRCRAFT_CALL *airport_type)(const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(airport_type);
	uint8_t (OPENTTD_AIRCRAFT_CALL *station_owner)(const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(station_owner);
	bool (OPENTTD_AIRCRAFT_CALL *has_hangar)(const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(has_hangar);
	const void * (OPENTTD_AIRCRAFT_CALL *airport_fta)(const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(airport_fta);
	uint64_t * (OPENTTD_AIRCRAFT_CALL *airport_blocks)(const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(airport_blocks);
	uint8_t (OPENTTD_AIRCRAFT_CALL *had_vehicle)(const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(had_vehicle);
	const void * (OPENTTD_AIRCRAFT_CALL *dummy_airport)() noexcept OPENTTD_SERVICE_DEFAULT(dummy_airport);
	uint8_t (OPENTTD_AIRCRAFT_CALL *airport_elements)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(airport_elements);
	uint8_t (OPENTTD_AIRCRAFT_CALL *helipads)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(helipads);
	uint8_t (OPENTTD_AIRCRAFT_CALL *airport_flags)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(airport_flags);
	uint8_t (OPENTTD_AIRCRAFT_CALL *airport_delta_z)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(airport_delta_z);
	const void * (OPENTTD_AIRCRAFT_CALL *node)(const void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(node);
	OpenTTDAircraftNode (OPENTTD_AIRCRAFT_CALL *fta)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(fta);
	OpenTTDAircraftMoving (OPENTTD_AIRCRAFT_CALL *moving)(const void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(moving);
	uint16_t (OPENTTD_AIRCRAFT_CALL *engine_speed)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(engine_speed);
	uint8_t (OPENTTD_AIRCRAFT_CALL *engine_subtype)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(engine_subtype);
	uint16_t (OPENTTD_AIRCRAFT_CALL *engine_sound)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(engine_sound);
	uint8_t (OPENTTD_AIRCRAFT_CALL *airport_entry)(const void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(airport_entry);
	uint8_t (OPENTTD_AIRCRAFT_CALL *direction_towards)(OpenTTDAircraftVehicle, int32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(direction_towards);
	int32_t (OPENTTD_AIRCRAFT_CALL *hangar_height)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(hangar_height);
	uint8_t (OPENTTD_AIRCRAFT_CALL *terminal_count)(const void *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(terminal_count);
	uint8_t (OPENTTD_AIRCRAFT_CALL *hangar_exit)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(hangar_exit);
	bool (OPENTTD_AIRCRAFT_CALL *can_use_station)(OpenTTDAircraftVehicle, const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(can_use_station);
	uint16_t (OPENTTD_AIRCRAFT_CALL *service_interval)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(service_interval);
	bool (OPENTTD_AIRCRAFT_CALL *chain_in_depot)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(chain_in_depot);
	bool (OPENTTD_AIRCRAFT_CALL *nearest_depot_order)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(nearest_depot_order);
	bool (OPENTTD_AIRCRAFT_CALL *part_of_orders)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(part_of_orders);
	OpenTTDAircraftVehicle (OPENTTD_AIRCRAFT_CALL *next_aircraft)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next_aircraft);
	void (OPENTTD_AIRCRAFT_CALL *update_position)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(update_position);
	void (OPENTTD_AIRCRAFT_CALL *rotor_image)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(rotor_image);
	void (OPENTTD_AIRCRAFT_CALL *copy_sprite)(OpenTTDAircraftVehicle, OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(copy_sprite);
	void (OPENTTD_AIRCRAFT_CALL *position_viewport)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(position_viewport);
	void (OPENTTD_AIRCRAFT_CALL *create_effect)(OpenTTDAircraftVehicle, uint8_t, int32_t, int32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(create_effect);
	void (OPENTTD_AIRCRAFT_CALL *truncate_cargo)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(truncate_cargo);
	void (OPENTTD_AIRCRAFT_CALL *crash_news)(OpenTTDAircraftVehicle, uint16_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(crash_news);
	void (OPENTTD_AIRCRAFT_CALL *station_rating)(uint32_t, uint8_t, int32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_rating);
	void (OPENTTD_AIRCRAFT_CALL *landing_rating)(uint16_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(landing_rating);
	void (OPENTTD_AIRCRAFT_CALL *dirty_depot)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(dirty_depot);
	void (OPENTTD_AIRCRAFT_CALL *first_arrival)(OpenTTDAircraftVehicle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(first_arrival);
	void (OPENTTD_AIRCRAFT_CALL *begin_loading)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(begin_loading);
	void (OPENTTD_AIRCRAFT_CALL *update_delta)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(update_delta);
	void (OPENTTD_AIRCRAFT_CALL *touchdown_animation)(OpenTTDAircraftVehicle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(touchdown_animation);
	void (OPENTTD_AIRCRAFT_CALL *destination_too_far)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(destination_too_far);
	void (OPENTTD_AIRCRAFT_CALL *delete_range_news)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(delete_range_news);
	void (OPENTTD_AIRCRAFT_CALL *service_order)(OpenTTDAircraftVehicle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(service_order);
	int64_t (OPENTTD_AIRCRAFT_CALL *running_cost)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(running_cost);
	void (OPENTTD_AIRCRAFT_CALL *subtract_cost)(OpenTTDAircraftVehicle, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(subtract_cost);
	void (OPENTTD_AIRCRAFT_CALL *dirty_lists)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(dirty_lists);
	uint16_t (OPENTTD_AIRCRAFT_CALL *next_stopping_station)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(next_stopping_station);
	void (OPENTTD_AIRCRAFT_CALL *remove_depot_orders)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(remove_depot_orders);
	void (OPENTTD_AIRCRAFT_CALL *assert_flying)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(assert_flying);
	void (OPENTTD_AIRCRAFT_CALL *invalid_movement)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(invalid_movement);
	void (OPENTTD_AIRCRAFT_CALL *invalid_position)(OpenTTDAircraftVehicle, const void *) noexcept OPENTTD_SERVICE_DEFAULT(invalid_position);
	void (OPENTTD_AIRCRAFT_CALL *invalid_scheme)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(invalid_scheme);
	void (OPENTTD_AIRCRAFT_CALL *unreachable)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(unreachable);
	uint32_t (OPENTTD_AIRCRAFT_CALL *cargo_age_property)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(cargo_age_property);
	uint32_t (OPENTTD_AIRCRAFT_CALL *range_property)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(range_property);
	bool (OPENTTD_AIRCRAFT_CALL *touchdown_sound)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(touchdown_sound);
	bool (OPENTTD_AIRCRAFT_CALL *rotor_image_if_changed)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(rotor_image_if_changed);
	void (OPENTTD_AIRCRAFT_CALL *update_rotor_image)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(update_rotor_image);
	void (OPENTTD_AIRCRAFT_CALL *process_orders)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(process_orders);
	uint32_t (OPENTTD_AIRCRAFT_CALL *vehicle_crash)(OpenTTDAircraftVehicle, bool) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_crash);
	bool (OPENTTD_AIRCRAFT_CALL *send_to_depot)(OpenTTDAircraftVehicle, bool) noexcept OPENTTD_SERVICE_DEFAULT(send_to_depot);
	uint8_t (OPENTTD_AIRCRAFT_CALL *fta_next_position)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(fta_next_position);
	OpenTTDAircraftBlockNode (OPENTTD_AIRCRAFT_CALL *block_node)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(block_node);
	OpenTTDAircraftRouteNode (OPENTTD_AIRCRAFT_CALL *route_node)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(route_node);
	OpenTTDAircraftBlockChoice (OPENTTD_AIRCRAFT_CALL *block_choice)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(block_choice);
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
