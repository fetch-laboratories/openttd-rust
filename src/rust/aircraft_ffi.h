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
#include "call_ffi.h"
struct AirportSpec;
struct OpenTTDAircraftLeaves;
struct OpenTTDAircraftVehicle;
const OpenTTDAircraftLeaves &GetRustAircraftLeaves() noexcept;
struct Vehicle;
OpenTTDAircraftVehicle RustAircraftVehicle(Vehicle *) noexcept;
OpenTTDAircraftVehicle GetRustAircraftVehicle(OpenTTDVehicle *) noexcept;

extern "C" {
struct OpenTTDAircraftState {
	uint32_t cached_max_range_sqr;
	uint16_t cached_max_range, cache_padding, crashed_counter, targetairport;
	uint8_t pos, previous_pos, state, last_direction, number_consecutive_turns, turn_counter, flags;
};
struct OpenTTDAircraftVehicle { OpenTTDVehicle *handle; OpenTTDAircraftState *state; uint32_t id; };
struct OpenTTDAircraftMoving { int16_t x, y; uint16_t flags; uint8_t direction; };
struct OpenTTDAircraftStrings {
	uint32_t aircraft_crash;
	uint32_t aircraft_dest_too_far;
	uint32_t first_aircraft_arrival;
	uint32_t plane_crash_out_of_fuel;
};
struct OpenTTDVehicleMassServices;
/* Immutable callback table, borrowed for synchronous entries. Handles retain the
 * original pool identity; only raw field-sized State access occurs in Rust.
 * No borrow survives a callback, including reentry and PreDestructor deletion.
 * All native services are noexcept; environmental failures and Rust panics abort.
 * Flight helpers accept actual Vehicle identity, with null state for disasters.
 * Station and AirportFTAClass fields cross one typed leaf per field. */
struct OpenTTDAircraftLeaves {
	uint64_t *(OPENTTD_RUST_CALL *airport_blocks)(const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(airport_blocks);
	uint8_t (OPENTTD_RUST_CALL *airport_delta_z)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(airport_delta_z);
	uint8_t (OPENTTD_RUST_CALL *airport_elements)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(airport_elements);
	uint8_t (OPENTTD_RUST_CALL *airport_entry)(const void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(airport_entry);
	uint8_t (OPENTTD_RUST_CALL *airport_flags)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(airport_flags);
	uint8_t (OPENTTD_RUST_CALL *airport_height)(const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(airport_height);
	uint32_t (OPENTTD_RUST_CALL *airport_tile)(const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(airport_tile);
	uint8_t (OPENTTD_RUST_CALL *airport_type)(const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(airport_type);
	uint8_t (OPENTTD_RUST_CALL *airport_width)(const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(airport_width);
	void (OPENTTD_RUST_CALL *copy_sprite)(OpenTTDVehicle *, OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(copy_sprite);
	uint8_t (OPENTTD_RUST_CALL *direction_towards)(OpenTTDAircraftVehicle, int32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(direction_towards);
	void (OPENTTD_RUST_CALL *dirty_depot)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(dirty_depot);
	void (OPENTTD_RUST_CALL *dirty_lists)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(dirty_lists);
	uint64_t (OPENTTD_RUST_CALL *fta_blocks)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(fta_blocks);
	uint8_t (OPENTTD_RUST_CALL *fta_heading)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(fta_heading);
	const void *(OPENTTD_RUST_CALL *fta_next)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(fta_next);
	uint8_t (OPENTTD_RUST_CALL *fta_next_position)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(fta_next_position);
	uint8_t (OPENTTD_RUST_CALL *fta_position)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(fta_position);
	uint8_t (OPENTTD_RUST_CALL *had_vehicle)(const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(had_vehicle);
	uint8_t (OPENTTD_RUST_CALL *hangar_exit)(const OpenTTDStation *, uint32_t, const AirportSpec *) noexcept OPENTTD_SERVICE_DEFAULT(hangar_exit);
	uint8_t (OPENTTD_RUST_CALL *helipads)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(helipads);
	void (OPENTTD_RUST_CALL *landing_rating)(uint16_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(landing_rating);
	const OpenTTDVehicleMassServices *mass;
	OpenTTDAircraftMoving (OPENTTD_RUST_CALL *moving)(const void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(moving);
	OpenTTDAircraftVehicle (OPENTTD_RUST_CALL *next_aircraft)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next_aircraft);
	const void * (OPENTTD_RUST_CALL *node)(const void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(node);
	uint8_t (OPENTTD_RUST_CALL *rotation)(const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(rotation);
	void (OPENTTD_RUST_CALL *rotor_image)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(rotor_image);
	bool (OPENTTD_RUST_CALL *rotor_image_if_changed)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(rotor_image_if_changed);
	uint16_t (OPENTTD_RUST_CALL *service_interval)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(service_interval);
	void (OPENTTD_RUST_CALL *service_order)(OpenTTDAircraftVehicle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(service_order);
	const OpenTTDSharedServices *shared;
	const OpenTTDStation *(OPENTTD_RUST_CALL *station)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(station);
	uint8_t (OPENTTD_RUST_CALL *station_owner)(const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(station_owner);
	uint32_t (OPENTTD_RUST_CALL *station_tile)(const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(station_tile);
	OpenTTDAircraftStrings strings;
	void (OPENTTD_RUST_CALL *subtract_cost)(OpenTTDAircraftVehicle, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(subtract_cost);
	uint8_t (OPENTTD_RUST_CALL *terminal_count)(const void *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(terminal_count);
	void (OPENTTD_RUST_CALL *truncate_cargo)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(truncate_cargo);
	void (OPENTTD_RUST_CALL *update_rotor_image)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(update_rotor_image);
	const OpenTTDVehicleServices *vehicle;
	uint32_t (OPENTTD_RUST_CALL *vehicle_crash)(OpenTTDAircraftVehicle, bool) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_crash);
	void (OPENTTD_RUST_CALL *viewport)(OpenTTDAircraftVehicle) noexcept OPENTTD_SERVICE_DEFAULT(viewport);
};
uint8_t OPENTTD_RUST_CALL openttd_rust_aircraft_tick_abi(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle);
extern "C++" inline bool OPENTTD_RUST_CALL openttd_rust_aircraft_tick(const OpenTTDAircraftLeaves *a0, OpenTTDAircraftVehicle a1) { return openttd_rust_aircraft_tick_abi(a0, a1) != 0; }
void OPENTTD_RUST_CALL openttd_rust_aircraft_calendar_day(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle);
void OPENTTD_RUST_CALL openttd_rust_aircraft_economy_day(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle);
void OPENTTD_RUST_CALL openttd_rust_aircraft_position(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle, int32_t, int32_t, int32_t);
void OPENTTD_RUST_CALL openttd_rust_aircraft_cache(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle, bool);
void OPENTTD_RUST_CALL openttd_rust_aircraft_flight_bounds(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle, int32_t *, int32_t *);
int32_t OPENTTD_RUST_CALL openttd_rust_aircraft_flight_level(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle, uint8_t *, bool);
void OPENTTD_RUST_CALL openttd_rust_aircraft_missing_orders(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle);
uint32_t OPENTTD_RUST_CALL openttd_rust_aircraft_crash(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle, bool);
void OPENTTD_RUST_CALL openttd_rust_aircraft_next_airport(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle);
void OPENTTD_RUST_CALL openttd_rust_aircraft_leave_hangar(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle, uint8_t);
void OPENTTD_RUST_CALL openttd_rust_aircraft_order_location(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle);
uint16_t OPENTTD_RUST_CALL openttd_rust_aircraft_closest_depot(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle);
uint8_t OPENTTD_RUST_CALL openttd_test_aircraft_event(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle, bool);
void OPENTTD_RUST_CALL openttd_rust_aircraft_release_blocks(const OpenTTDAircraftLeaves *, OpenTTDAircraftVehicle);
OpenTTDAircraftState *OPENTTD_RUST_CALL openttd_rust_aircraft_state_new();
void OPENTTD_RUST_CALL openttd_rust_aircraft_state_destroy(OpenTTDAircraftState *);
uint64_t *OPENTTD_RUST_CALL openttd_rust_airport_blocks_new();
void OPENTTD_RUST_CALL openttd_rust_airport_blocks_destroy(uint64_t *);
}
#endif /* OPENTTD_RUST_AIRCRAFT_FFI_H */
