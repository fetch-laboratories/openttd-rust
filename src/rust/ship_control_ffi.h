/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file ship_control_ffi.h Canonical ship controller and private state. */
#ifndef OPENTTD_SHIP_CONTROL_FFI_H
#define OPENTTD_SHIP_CONTROL_FFI_H
#include "service_defaults.hpp"
#include "services_ffi.h"
#include "water_regions_ffi.h"
struct OpenTTDVehicle;
struct OpenTTDVehicleServices;
#include <cstdint>
#include <cstddef>
struct OpenTTDShipState;
struct OpenTTDShipDepot { uint32_t id, tile, owner, ship; };
struct OpenTTDShipPosition { int32_t x, y; uint32_t old_tile, new_tile; };
struct OpenTTDShipReverseResult { bool reverse; uint8_t trackdir; };
struct OpenTTDShipTrackResult { uint8_t track; bool found; };
struct OpenTTDShipDepotResult { uint32_t tile; uint16_t id; bool valid; };
struct OpenTTDShipLeaves {
	uint32_t (*depot_dir)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_dir);
	uint32_t (*depot_axis)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_axis);
	bool (*is_depot)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_depot);
	uint32_t (*depot_index)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_index);
	bool (*wait_unbunch)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(wait_unbunch);
	bool (*chain_depot)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(chain_depot);
	uint32_t (*servint)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(servint);
	uint32_t (*max_distance)() noexcept OPENTTD_SERVICE_DEFAULT(max_distance);
	bool (*tile_valid)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_valid);
	uint32_t (*tile_type)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_type);
	uint32_t (*water_class)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(water_class);
	bool (*lock_middle)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(lock_middle);
	uint32_t (*lock_dir)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(lock_dir);
	uint32_t (*tile_min_z)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_min_z);
	uint32_t (*tile_max_z)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_max_z);
	uint32_t (*track_status)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(track_status);
	uint32_t (*offset)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(offset);
	uint32_t (*diag_between)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(diag_between);
	uint32_t (*dist_square)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(dist_square);
	uint32_t (*dist_manhattan)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(dist_manhattan);
	bool (*docking)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(docking);
	bool (*dock)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(dock);
	bool (*dock_water)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(dock_water);
	uint32_t (*station)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station);
	uint32_t (*industry_station)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(industry_station);
	bool (*oilrig)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(oilrig);
	bool (*station_use)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_use);
	uint32_t (*station_xy)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_xy);
	bool (*station_contains)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_contains);
	bool (*station_dock)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_dock);
	uint32_t (*station_visits)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_visits);
	void (*visit_set)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(visit_set);
	void (*arrival)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(arrival);
	void (*leave_unbunch)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(leave_unbunch);
	void (*path_result)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(path_result);
	void (*order_free)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(order_free);
	void (*order_dummy)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(order_dummy);
	void (*order_depot)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(order_depot);
	void (*order_leave)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(order_leave);
	void (*order_increment)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(order_increment);
	void (*timetable)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(timetable);
	void (*start_dirty)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(start_dirty);
	void (*depot_dirty)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_dirty);
	void (*depot_invalidate)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_invalidate);
	void (*ships_dirty)() noexcept OPENTTD_SERVICE_DEFAULT(ships_dirty);
	void (*check_orders)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(check_orders);
	int64_t (*running_cost)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(running_cost);
	uint32_t (*cost_divisor)() noexcept OPENTTD_SERVICE_DEFAULT(cost_divisor);
	void (*pay_running)(uint32_t, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(pay_running);
	uint32_t (*speed_default)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(speed_default);
	uint32_t (*age_default)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(age_default);
	uint32_t (*speed_frac)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(speed_frac);
	uint32_t (*speed_property)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(speed_property);
	uint32_t (*age_property)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(age_property);
	void (*update_visual)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(update_visual);
	void (*cache_invalidate)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(cache_invalidate);
	uint32_t (*capacity)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(capacity);
	void (*sprite_direction)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(sprite_direction);
	uint32_t (*tile_x)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_x);
	uint32_t (*tile_y)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_y);
	bool (*build_flag)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(build_flag);
	void (*build_random)(uint32_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(build_random);
	OpenTTDShipPosition (*new_position)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(new_position);
	uint32_t (*exit_dir)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(exit_dir);
	uint32_t (*track_direction)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(track_direction);
	uint32_t (*tracks_reach)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tracks_reach);
	size_t (*path_size)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(path_size);
	uint32_t (*path_back)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(path_back);
	void (*path_pop)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(path_pop);
	void (*path_clear)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(path_clear);
	uint32_t (*enter_tile)(uint32_t, uint32_t, uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(enter_tile);
	bool (*process_orders)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(process_orders);
	void (*viewport)(uint32_t, bool, bool) noexcept OPENTTD_SERVICE_DEFAULT(viewport);
	void (*base_viewport)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(base_viewport);
	void (*visual)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(visual);
	void (*cache)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(cache);
	void (*play_sound)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(play_sound);
	OpenTTDShipReverseResult (*yapf_reverse)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(yapf_reverse);
	OpenTTDShipTrackResult (*yapf_choose)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(yapf_choose);
	void (*update_delta)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(update_delta);
	void (*build_owner)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(build_owner);
	void (*build_z)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(build_z);
	void (*build_properties)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(build_properties);
	void (*build_dates)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(build_dates);
	void (*build_acceleration)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(build_acceleration);
	void (*build_prototype)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(build_prototype);
	void (*build_interval_percent)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(build_interval_percent);
	void (*build_capacity)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(build_capacity);
	void (*set_tile)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_tile);
	void (*set_x)(uint32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_x);
	void (*set_y)(uint32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_y);
	void (*set_z)(uint32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_z);
	void (*set_direction)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_direction);
	void (*set_speed)(uint32_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_speed);
	void (*set_tick)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_tick);
	void (*set_running)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_running);
	void (*set_day)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_day);
	void (*set_order_time)(uint32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_order_time);
	void (*set_progress)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_progress);
	void (*set_last_station)(uint32_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_last_station);
	void (*set_hidden)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_hidden);
	void (*set_max_speed)(uint32_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_max_speed);
	void (*set_cargo_age)(uint32_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_cargo_age);
	void (*set_dest)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_dest);
	uint32_t (*tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile);
	uint32_t (*dest)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(dest);
	uint32_t (*x)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(x);
	uint32_t (*y)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(y);
	uint32_t (*z)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(z);
	uint32_t (*direction)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(direction);
	uint32_t (*speed)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(speed);
	uint32_t (*tick)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tick);
	uint32_t (*running)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(running);
	uint32_t (*day)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(day);
	uint32_t (*order_time)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(order_time);
	uint32_t (*progress)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(progress);
	uint32_t (*status)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(status);
	uint32_t (*owner)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(owner);
	uint32_t (*last_station)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(last_station);
	uint32_t (*order_destination)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(order_destination);
	uint32_t (*order_type)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(order_type);
	uint32_t (*order_max_speed)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(order_max_speed);
	uint32_t (*acceleration)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(acceleration);
	uint32_t (*max_speed)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(max_speed);
	OpenTTDShipState *(*state_owner)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(state_owner);
	OpenTTDWaterPatch (*patch)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(patch);
	size_t (*neighbours)(OpenTTDWaterPatch, OpenTTDWaterPatch *) noexcept OPENTTD_SERVICE_DEFAULT(neighbours);
	bool (*next_depot)(uint32_t, OpenTTDShipDepot *) noexcept OPENTTD_SERVICE_DEFAULT(next_depot);
	const OpenTTDVehicleServices *vehicle; ///< Base vehicle table, immutable for the process.
};
struct OpenTTDShipLeaves;
const OpenTTDShipLeaves &GetRustShipServices() noexcept;
extern "C" {
/* Game-thread shell ownership; new zeroes state/coordinates and sets INVALID_DIR.
 * Typed fields are state:u8, rotation:u8, rotation-x:i16, rotation-y:i16.
 * Accessors retain no borrow; callers narrow exactly like C++.
 * Modern/legacy staging commits partial loads on unwind outside Rust frames.
 * Panics/OOM abort. One destruction follows PreDestructor, including pool cleanup. */
void openttd_rust_ship_control_reload_rotation();
OpenTTDShipState *openttd_rust_ship_state_new();
void openttd_rust_ship_state_destroy(OpenTTDShipState *);
uint8_t openttd_rust_ship_get_state(const OpenTTDShipState *);
void openttd_rust_ship_set_state(OpenTTDShipState *, uint8_t);
uint8_t openttd_rust_ship_get_rotation(const OpenTTDShipState *);
void openttd_rust_ship_set_rotation(OpenTTDShipState *, uint8_t);
int16_t openttd_rust_ship_get_rotation_x(const OpenTTDShipState *);
void openttd_rust_ship_set_rotation_x(OpenTTDShipState *, int16_t);
int16_t openttd_rust_ship_get_rotation_y(const OpenTTDShipState *);
void openttd_rust_ship_set_rotation_y(OpenTTDShipState *, int16_t);
/* Plain synchronous calls. No entry task allocation or retained world/owner borrow.
 * Every service is noexcept; unexpected environmental exceptions terminate.
 * The selected services do not unwind through script VMs or save/load during play.
 * Depot search owns its inherent BFS scratch; neighbour output spans 320 entries. */
bool openttd_rust_ship_tick(uint32_t, OpenTTDVehicle *, const OpenTTDShipLeaves *, const OpenTTDSharedServices *);
void openttd_rust_ship_calendar_day(uint32_t, OpenTTDVehicle *, const OpenTTDShipLeaves *, const OpenTTDSharedServices *);
void openttd_rust_ship_economy_day(uint32_t, OpenTTDVehicle *, const OpenTTDShipLeaves *, const OpenTTDSharedServices *);
void openttd_rust_ship_cache(uint32_t, const OpenTTDShipLeaves *, const OpenTTDSharedServices *);
void openttd_rust_ship_destination(uint32_t, uint32_t, const OpenTTDShipLeaves *, const OpenTTDSharedServices *);
OpenTTDShipDepotResult openttd_rust_ship_find_depot(uint32_t, uint32_t, const OpenTTDShipLeaves *, const OpenTTDSharedServices *);
bool openttd_rust_ship_is_destination(uint32_t, uint32_t, const OpenTTDShipLeaves *, const OpenTTDSharedServices *);
uint32_t openttd_rust_ship_station_destination(uint32_t, uint32_t, const OpenTTDShipLeaves *, const OpenTTDSharedServices *);
uint8_t openttd_rust_ship_trackdir(uint32_t, const OpenTTDShipLeaves *, const OpenTTDSharedServices *);
}
#endif
