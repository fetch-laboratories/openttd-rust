/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file train_ffi.h Complete train owner shared-world boundary. */
#ifndef RUST_TRAIN_FFI_H
#define RUST_TRAIN_FFI_H
#include "service_defaults.hpp"
#include <cstdint>
#include <cstddef>
#include "train_state_ffi.h"
#include "services_ffi.h"
#include "signal_ffi.h"
struct OpenTTDVehicleServices;
/* Each shell owns one canonical Rust state allocation. A resolved handle copies
 * native identity and its owner once per entry or topology read. Narrow records
 * contain only fields consumed by the current policy; no world borrow spans a
 * service. Tables are immutable and borrowed for synchronous calls. Native
 * visitors preserve container order and filter non-trains before Rust policy.
 * Tile/depot, orders/loading, station callbacks and destruction are direct
 * noexcept services: none unwinds an ordinary-play script or save error here.
 * Escaping environmental exceptions and Rust panics abort. Destruction invalidates
 * both handle pointers; policy never reads them after the original deletion point. */
struct OpenTTDTrainDepot { uint32_t tile, length; };
struct OpenTTDTrainHandle {
	void *shell;
	OpenTTDTrainState *owner;
};
struct Train;
struct OpenTTDTrainServices;
const OpenTTDTrainServices &GetTrainServices() noexcept;
OpenTTDTrainHandle TrainHandle(const Train *) noexcept;
extern "C" {
struct OpenTTDTrainConsistChangedRead {
	uint16_t engine;
	uint8_t front;
};
struct OpenTTDTrainConsistChanged1Read {
	uint16_t engine;
	uint8_t engine_part;
};
struct OpenTTDTrainConsistChanged2Read {
	uint16_t cargo_cap;
};
struct OpenTTDTrainCurveLimitRead {
	OpenTTDTrainHandle next;
	uint8_t direction;
};
struct OpenTTDTrainStopLocationRead {
	uint16_t length;
	uint16_t total_length;
	uint16_t order_destination;
	uint8_t order;
};
struct OpenTTDTrainCurrentMaxSpeedRead {
	uint32_t tile;
	uint16_t max_track_speed;
	uint16_t speed;
};
struct OpenTTDTrainUpdateAccelerationRead {
	uint32_t power;
	uint32_t weight;
};
struct OpenTTDTrainUpdateSpeedRead {
	uint8_t status;
	uint8_t acceleration;
};
struct OpenTTDTrainTrackdirRead {
	uint32_t tile;
	uint8_t direction;
	uint8_t status;
};
struct OpenTTDTrainCanLeaveRead {
	uint32_t tile;
	uint8_t direction;
};
struct OpenTTDTrainCrossingApproachRead {
	uint8_t status;
	uint8_t front;
};
struct OpenTTDTrainNextOffsetRead {
	OpenTTDTrainHandle next;
	uint16_t length;
};
struct OpenTTDTrainAfterSwapRead {
	uint32_t tile;
	int32_t x;
	int32_t y;
};
struct OpenTTDTrainReverseSwapRead {
	uint32_t tile;
	int32_t x;
	int32_t y;
	int32_t z;
	uint8_t direction;
	uint8_t status;
};
struct OpenTTDTrainApproachingEndRead {
	int32_t x;
	int32_t y;
	uint16_t length;
	uint16_t speed;
	uint8_t direction;
};
struct OpenTTDTrainLineEndsRead {
	uint32_t tile;
	uint16_t speed;
	uint8_t breakdown;
};
struct OpenTTDTrainSpeedZRead {
	int32_t z;
	uint16_t max_track_speed;
	uint16_t speed;
};
struct OpenTTDTrainMoveVehicleRead {
	uint32_t tile;
	int32_t x;
	int32_t y;
	uint8_t direction;
	uint8_t front;
	uint8_t articulated;
};
struct OpenTTDTrainMoveVehicle20Read {
	uint16_t speed;
	uint8_t status;
	uint8_t front;
};
struct OpenTTDTrainCollisionOneRead {
	OpenTTDTrainHandle first;
	int32_t x;
	int32_t y;
	int32_t z;
	uint16_t length;
	uint8_t owner;
};
struct OpenTTDTrainCollisionOne22Read {
	int32_t x;
	int32_t y;
	int32_t z;
	uint16_t length;
	uint8_t owner;
};
struct OpenTTDTrainDeleteLastRead {
	uint32_t tile;
	uint8_t owner;
};
struct OpenTTDTrainStayDepotRead {
	uint32_t tile;
	uint32_t power;
};
struct OpenTTDTrainLocoRead {
	uint16_t speed;
	uint8_t status;
	uint8_t order;
};
struct OpenTTDTrainLoco26Read {
	uint32_t tile;
	uint16_t order_destination;
	uint8_t order;
	uint8_t nonstop;
};
struct OpenTTDTrainTickRead {
	uint16_t speed;
	uint8_t status;
	uint8_t running;
	uint8_t front;
	uint8_t free_wagon;
};
struct OpenTTDTrainNeedsServiceRead {
	uint16_t order_destination;
	uint8_t order;
};
struct OpenTTDTrainNextForceRead {
	uint32_t tile;
	uint8_t status;
};
struct OpenTTDTrainReverseCommandRead {
	uint8_t status;
	uint8_t breakdown;
	uint8_t front;
};
struct OpenTTDTrainServices {
	OpenTTDTrainHandle (*read_first)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_first);
	OpenTTDTrainHandle (*read_next)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_next);
	OpenTTDTrainHandle (*read_previous)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_previous);
	OpenTTDTrainHandle (*read_next_unit)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_next_unit);
	OpenTTDTrainHandle (*read_last)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_last);
	uint32_t (*read_tile)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_tile);
	uint32_t (*read_dest)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_dest);
	int32_t (*read_order_time)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_order_time);
	uint16_t (*read_length)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_length);
	uint16_t (*read_total_length)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_total_length);
	uint16_t (*read_max_track_speed)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_max_track_speed);
	uint16_t (*read_speed)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_speed);
	uint16_t (*read_gv_flags)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_gv_flags);
	uint16_t (*read_refit_cap)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_refit_cap);
	uint16_t (*read_last_station)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_last_station);
	uint8_t (*read_direction)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_direction);
	uint8_t (*read_status)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_status);
	uint8_t (*read_tick)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_tick);
	uint8_t (*read_running)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_running);
	uint8_t (*read_day)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_day);
	uint8_t (*read_progress)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_progress);
	uint8_t (*read_order)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_order);
	uint8_t (*read_front)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_front);
	uint8_t (*read_articulated)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_articulated);
	uint8_t (*read_multiheaded)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_multiheaded);
	uint8_t (*read_owner)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_owner);
	uint8_t (*read_vis_effect)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_vis_effect);
	OpenTTDTrainConsistChangedRead (*read_consist_changed)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_consist_changed);
	OpenTTDTrainConsistChanged1Read (*read_consist_changed_1)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_consist_changed_1);
	OpenTTDTrainConsistChanged2Read (*read_consist_changed_2)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_consist_changed_2);
	OpenTTDTrainCurveLimitRead (*read_curve_limit)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_curve_limit);
	OpenTTDTrainStopLocationRead (*read_stop_location)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_stop_location);
	OpenTTDTrainCurrentMaxSpeedRead (*read_current_max_speed)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_current_max_speed);
	OpenTTDTrainUpdateAccelerationRead (*read_update_acceleration)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_update_acceleration);
	OpenTTDTrainUpdateSpeedRead (*read_update_speed)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_update_speed);
	OpenTTDTrainTrackdirRead (*read_trackdir)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_trackdir);
	OpenTTDTrainCanLeaveRead (*read_can_leave)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_can_leave);
	OpenTTDTrainCrossingApproachRead (*read_crossing_approach)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_crossing_approach);
	OpenTTDTrainNextOffsetRead (*read_next_offset)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_next_offset);
	OpenTTDTrainAfterSwapRead (*read_after_swap)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_after_swap);
	OpenTTDTrainReverseSwapRead (*read_reverse_swap)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_reverse_swap);
	OpenTTDTrainApproachingEndRead (*read_approaching_end)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_approaching_end);
	OpenTTDTrainLineEndsRead (*read_line_ends)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_line_ends);
	OpenTTDTrainSpeedZRead (*read_speed_z)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_speed_z);
	OpenTTDTrainMoveVehicleRead (*read_move_vehicle)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_move_vehicle);
	OpenTTDTrainMoveVehicle20Read (*read_move_vehicle_20)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_move_vehicle_20);
	OpenTTDTrainCollisionOneRead (*read_collision_one)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_collision_one);
	OpenTTDTrainCollisionOne22Read (*read_collision_one_22)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_collision_one_22);
	OpenTTDTrainDeleteLastRead (*read_delete_last)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_delete_last);
	OpenTTDTrainStayDepotRead (*read_stay_depot)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_stay_depot);
	OpenTTDTrainLocoRead (*read_loco)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_loco);
	OpenTTDTrainLoco26Read (*read_loco_26)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_loco_26);
	OpenTTDTrainTickRead (*read_tick_state)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_tick_state);
	OpenTTDTrainNeedsServiceRead (*read_needs_service)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_needs_service);
	OpenTTDTrainNextForceRead (*read_next_force)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_next_force);
	OpenTTDTrainReverseCommandRead (*read_reverse_command)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_reverse_command);

	void (*write_tile)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_tile);
	void (*write_dest)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_dest);
	void (*write_x)(OpenTTDTrainHandle, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_x);
	void (*write_y)(OpenTTDTrainHandle, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_y);
	void (*write_z)(OpenTTDTrainHandle, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_z);
	void (*write_direction)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_direction);
	void (*write_speed)(OpenTTDTrainHandle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(write_speed);
	void (*write_tick)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_tick);
	void (*write_running)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_running);
	void (*write_day)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_day);
	void (*write_order_time)(OpenTTDTrainHandle, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_order_time);
	void (*write_progress)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_progress);
	void (*write_subspeed)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_subspeed);
	void (*write_gv_flags)(OpenTTDTrainHandle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(write_gv_flags);
	void (*write_acceleration)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_acceleration);
	void (*write_length)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_length);
	void (*write_total_length)(OpenTTDTrainHandle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(write_total_length);
	void (*write_first_engine)(OpenTTDTrainHandle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(write_first_engine);
	void (*write_max_speed)(OpenTTDTrainHandle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(write_max_speed);
	void (*write_cargo_cap)(OpenTTDTrainHandle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(write_cargo_cap);
	void (*write_refit_cap)(OpenTTDTrainHandle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(write_refit_cap);
	void (*write_cargo_age)(OpenTTDTrainHandle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(write_cargo_age);
	void (*write_last_station)(OpenTTDTrainHandle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(write_last_station);
	void (*write_colourmap)(OpenTTDTrainHandle, uint64_t) noexcept OPENTTD_SERVICE_DEFAULT(write_colourmap);
	void (*write_status)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_status);

	uint64_t (*acceleration)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(acceleration);
	uint64_t (*acc_model)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(acc_model);
	uint64_t (*acc_type)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(acc_type);
	uint64_t (*advance_distance)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(advance_distance);
	uint64_t (*all_powered)(OpenTTDTrainHandle, uint64_t) noexcept OPENTTD_SERVICE_DEFAULT(all_powered);
	uint64_t (*ambient_sound)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(ambient_sound);
	void (*arrival_news)(OpenTTDTrainHandle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(arrival_news);
	void (*arrival_triggers)(OpenTTDTrainHandle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(arrival_triggers);
	uint64_t (*axis_diag)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(axis_diag);
	uint64_t (*backoff)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(backoff);
	void (*base_viewport)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(base_viewport);
	uint64_t (*bridge_speed)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(bridge_speed);
	void (*cache_override)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(cache_override);
	uint64_t (*callback_length)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(callback_length);
	uint64_t (*capacity)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(capacity);
	void (*capacity_error)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(capacity_error);
	uint64_t (*cargo_age_default)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(cargo_age_default);
	void (*cargo_changed)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(cargo_changed);
	uint64_t (*chain_depot)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(chain_depot);
	void (*check_next)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(check_next);
	void (*check_orders)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(check_orders);
	uint64_t (*check_reverse)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(check_reverse);
	uint64_t (*choose_track)(OpenTTDTrainHandle, uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(choose_track);
	void (*clear_reservation)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(clear_reservation);
	void (*consist_windows)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(consist_windows);
	uint64_t (*cost_class)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(cost_class);
	uint64_t (*cost_default)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(cost_default);
	uint64_t (*cost_divisor)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(cost_divisor);
	uint64_t (*count_chain)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(count_chain);
	void (*crash_event)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(crash_event);
	uint64_t (*crash_ground)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(crash_ground);
	void (*crash_news)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(crash_news);
	void (*crash_rating)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(crash_rating);
	void (*crash_sound)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(crash_sound);
	uint64_t (*crossing_barred)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(crossing_barred);
	uint64_t (*crossing_rail_axis)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(crossing_rail_axis);
	uint64_t (*crossing_road_axis)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(crossing_road_axis);
	void (*crossing_sound)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(crossing_sound);
	uint64_t (*curve_advantage)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(curve_advantage);
	uint64_t (*curve_mod)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(curve_mod);
	uint64_t (*day_ticks)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(day_ticks);
	void (*delete_vehicle)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(delete_vehicle);
	uint64_t (*depot_dir)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_dir);
	void (*depot_dirty)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(depot_dirty);
	uint64_t (*depot_index)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_index);
	uint64_t (*depot_track)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_track);
	void (*depot_window)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_window);
	uint64_t (*diag_axis)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(diag_axis);
	uint64_t (*diag_between)(OpenTTDTrainHandle, uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(diag_between);
	uint64_t (*diag_reaches_tracks)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(diag_reaches_tracks);
	uint64_t (*diag_trackdir)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(diag_trackdir);
	void (*dirty_tile)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(dirty_tile);
	uint64_t (*dir_diag)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(dir_diag);
	uint64_t (*disaster_sound)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(disaster_sound);
	void (*disconnect)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(disconnect);
	uint64_t (*engine_power)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(engine_power);
	OpenTTDTrainDepot (*find_depot)(OpenTTDTrainHandle, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(find_depot);
	uint64_t (*first_track)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(first_track);
	uint64_t (*grf_version)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(grf_version);
	uint64_t (*has_depot_res)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(has_depot_res);
	uint64_t (*has_reserved)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(has_reserved);
	uint64_t (*has_signal)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(has_signal);
	uint64_t (*has_signals)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(has_signals);
	uint64_t (*has_signal_td)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(has_signal_td);
	void (*hide_fill)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(hide_fill);
	uint64_t (*inclination)(OpenTTDTrainHandle, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(inclination);
	void (*invalidate_grf)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(invalidate_grf);
	uint64_t (*invalid_price)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(invalid_price);
	uint64_t (*is_bridge)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_bridge);
	uint64_t (*is_crossing)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_crossing);
	uint64_t (*is_depot)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_depot);
	uint64_t (*is_plain_rail)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_plain_rail);
	uint64_t (*is_railway)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_railway);
	uint64_t (*is_station)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_station);
	uint64_t (*is_station_any)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_station_any);
	uint64_t (*is_tunnelbridge)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_tunnelbridge);
	void (*large_explosion)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(large_explosion);
	void (*last_speed)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(last_speed);
	void (*leave_sound)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(leave_sound);
	void (*leave_unbunch)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(leave_unbunch);
	uint64_t (*length_callback)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(length_callback);
	void (*length_changed)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(length_changed);
	uint64_t (*length_default)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(length_default);
	void (*length_error)(OpenTTDTrainHandle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(length_error);
	uint64_t (*local_company)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(local_company);
	uint64_t (*lost_warn)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(lost_warn);
	uint64_t (*map_size)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(map_size);
	uint64_t (*max_depot_penalty)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(max_depot_penalty);
	uint64_t (*no_90)(OpenTTDTrainHandle, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(no_90);
	uint64_t (*oneway_blocking)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(oneway_blocking);
	void (*order_depot_service)(OpenTTDTrainHandle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(order_depot_service);
	void (*order_dummy)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(order_dummy);
	void (*order_free)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(order_free);
	uint64_t (*order_max_speed)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(order_max_speed);
	uint64_t (*other_end)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(other_end);
	void (*pay_running)(OpenTTDTrainHandle, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(pay_running);
	uint64_t (*pbs_signal_type)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(pbs_signal_type);
	uint64_t (*platform_ahead)(OpenTTDTrainHandle, uint16_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(platform_ahead);
	uint64_t (*platform_length)(OpenTTDTrainHandle, uint16_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(platform_length);
	void (*position)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(position);
	uint64_t (*pow_wag_power)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(pow_wag_power);
	uint64_t (*price)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(price);
	uint64_t (*process_orders)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(process_orders);
	uint64_t (*property)(OpenTTDTrainHandle, uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(property);
	uint64_t (*railveh_wagon)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(railveh_wagon);
	uint64_t (*rail_tilt)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(rail_tilt);
	uint64_t (*rail_type)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_type);
	uint64_t (*rail_types)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(rail_types);
	uint64_t (*reserve_paths)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(reserve_paths);
	void (*reserve_under)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(reserve_under);
	void (*reset_unbunch)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(reset_unbunch);
	uint64_t (*reverse_at_signals)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(reverse_at_signals);
	uint64_t (*reverse_single_blocked)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(reverse_single_blocked);
	void (*reverse_windows)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(reverse_windows);
	void (*running_windows)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(running_windows);
	uint64_t (*servint)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(servint);
	void (*set_depot_res)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_depot_res);
	void (*set_next)(OpenTTDTrainHandle, OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(set_next);
	void (*set_platform_res)(OpenTTDTrainHandle, uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_platform_res);
	void (*set_signal_state)(OpenTTDTrainHandle, uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_signal_state);
	void (*set_tunnel_res)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_tunnel_res);
	void (*show_effect)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(show_effect);
	uint64_t (*show_reservation)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(show_reservation);
	OpenTTDSignalContext (*signal_context)() noexcept OPENTTD_SERVICE_DEFAULT(signal_context);
	uint64_t (*signal_has_pbs)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(signal_has_pbs);
	uint64_t (*signal_pbs)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(signal_pbs);
	uint64_t (*signal_type)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(signal_type);
	uint64_t (*sigseg_full)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(sigseg_full);
	uint64_t (*sigseg_pbs)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(sigseg_pbs);
	void (*small_explosion)(OpenTTDTrainHandle, int32_t, int32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(small_explosion);
	uint64_t (*speed_default)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(speed_default);
	void (*start_stop_dirty)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(start_stop_dirty);
	uint64_t (*station)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station);
	uint64_t (*station_axis)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_axis);
	uint64_t (*station_compatible)(OpenTTDTrainHandle, uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_compatible);
	uint64_t (*station_dest)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(station_dest);
	uint64_t (*stopped_in_depot)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(stopped_in_depot);
	uint64_t (*stop_location)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(stop_location);
	void (*stuck_news)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(stuck_news);
	void (*suppress_implicit)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(suppress_implicit);
	uint64_t (*tile_add_diag)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_add_diag);
	uint64_t (*tile_offset_axis)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_offset_axis);
	uint64_t (*tile_offset_diag)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_offset_diag);
	uint64_t (*tile_owner)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_owner);
	uint64_t (*tile_rail_type)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_rail_type);
	uint64_t (*tile_virt)(OpenTTDTrainHandle, int32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_virt);
	uint64_t (*trackdir_exit)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(trackdir_exit);
	uint64_t (*trackdir_reaches)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(trackdir_reaches);
	uint64_t (*track_bits)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(track_bits);
	uint64_t (*track_crosses)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(track_crosses);
	uint64_t (*track_direction)(OpenTTDTrainHandle, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(track_direction);
	void (*train_list)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(train_list);
	uint64_t (*train_visit)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(train_visit);
	void (*truncate_cargo)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(truncate_cargo);
	uint64_t (*try_reserve)(OpenTTDTrainHandle, uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(try_reserve);
	uint64_t (*tunnel_dir)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tunnel_dir);
	void (*unreserve)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(unreserve);
	void (*update_delta)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(update_delta);
	uint64_t (*update_speed)(OpenTTDTrainHandle, uint32_t, int32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(update_speed);
	uint64_t (*user_default)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(user_default);
	uint64_t (*veh_exit_dir)(OpenTTDTrainHandle, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(veh_exit_dir);
	void (*viewport)(OpenTTDTrainHandle, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(viewport);
	uint64_t (*visit_type)(OpenTTDTrainHandle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(visit_type);
	void (*vis_effect)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(vis_effect);
	uint64_t (*wagon_override)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(wagon_override);
	uint64_t (*wagon_speed_limits)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(wagon_speed_limits);
	uint64_t (*wait_oneway)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(wait_oneway);
	uint64_t (*wait_pbs)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(wait_pbs);
	uint64_t (*wait_twoway)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(wait_twoway);
	uint64_t (*wait_unbunch)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(wait_unbunch);
	void (*write_crossing_bar)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_crossing_bar);
	void (*write_visit_type)(OpenTTDTrainHandle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(write_visit_type);

	uint8_t (*visit_tile)(uint32_t, void *, uint8_t (*)(void *, OpenTTDTrainHandle)) noexcept OPENTTD_SERVICE_DEFAULT(visit_tile);
	uint8_t (*visit_near)(int32_t, int32_t, void *, uint8_t (*)(void *, OpenTTDTrainHandle)) noexcept OPENTTD_SERVICE_DEFAULT(visit_near);
	const OpenTTDVehicleServices *vehicle; ///< Base vehicle table, immutable for the process.
};

void openttd_rust_train_consist_changed(OpenTTDTrainHandle, uint8_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
uint16_t openttd_rust_train_curve_limit(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
int32_t openttd_rust_train_current_max_speed(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_update_acceleration(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
int32_t openttd_rust_train_update_speed(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_mark_dirty(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
uint8_t openttd_rust_train_tick(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_calendar_day(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_economy_day(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
int64_t openttd_rust_train_running_cost(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
uint8_t openttd_rust_train_trackdir(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_mark_stuck(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_reverse_swap(OpenTTDTrainHandle, int32_t, int32_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
uint8_t openttd_rust_train_train_on_tile(uint32_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_update_crossing(uint32_t, uint8_t, uint8_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_adjacent_crossing_dirty(uint32_t, uint8_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_crossing_removed(uint32_t, uint8_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_reverse(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
uint8_t openttd_rust_train_controller(OpenTTDTrainHandle, OpenTTDTrainHandle, uint8_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
uint32_t openttd_rust_train_crash(OpenTTDTrainHandle, uint8_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_needs_service(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
uint8_t openttd_rust_train_reverse_command(OpenTTDTrainHandle, uint8_t, uint8_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
uint8_t openttd_rust_train_force_command(OpenTTDTrainHandle, uint8_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
}
#endif
