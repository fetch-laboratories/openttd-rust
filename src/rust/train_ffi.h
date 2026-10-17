/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file train_ffi.h Complete train owner shared-world boundary. */
#ifndef RUST_TRAIN_FFI_H
#define RUST_TRAIN_FFI_H
#include <cstdint>
#include <cstddef>
#include "train_state_ffi.h"
#include "services_ffi.h"
#include "signal_ffi.h"
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
struct OpenTTDTrainCurrentMaxSpeed6Read {
	OpenTTDTrainHandle next;
	uint32_t tile;
	uint8_t status;
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
	OpenTTDTrainHandle (*read_first)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainHandle (*read_next)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainHandle (*read_previous)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainHandle (*read_next_unit)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainHandle (*read_last)(OpenTTDTrainHandle) noexcept;
	uint32_t (*read_tile)(OpenTTDTrainHandle) noexcept;
	uint32_t (*read_dest)(OpenTTDTrainHandle) noexcept;
	int32_t (*read_order_time)(OpenTTDTrainHandle) noexcept;
	uint16_t (*read_length)(OpenTTDTrainHandle) noexcept;
	uint16_t (*read_total_length)(OpenTTDTrainHandle) noexcept;
	uint16_t (*read_max_track_speed)(OpenTTDTrainHandle) noexcept;
	uint16_t (*read_speed)(OpenTTDTrainHandle) noexcept;
	uint16_t (*read_gv_flags)(OpenTTDTrainHandle) noexcept;
	uint16_t (*read_refit_cap)(OpenTTDTrainHandle) noexcept;
	uint16_t (*read_last_station)(OpenTTDTrainHandle) noexcept;
	uint8_t (*read_direction)(OpenTTDTrainHandle) noexcept;
	uint8_t (*read_status)(OpenTTDTrainHandle) noexcept;
	uint8_t (*read_tick)(OpenTTDTrainHandle) noexcept;
	uint8_t (*read_running)(OpenTTDTrainHandle) noexcept;
	uint8_t (*read_day)(OpenTTDTrainHandle) noexcept;
	uint8_t (*read_progress)(OpenTTDTrainHandle) noexcept;
	uint8_t (*read_order)(OpenTTDTrainHandle) noexcept;
	uint8_t (*read_front)(OpenTTDTrainHandle) noexcept;
	uint8_t (*read_articulated)(OpenTTDTrainHandle) noexcept;
	uint8_t (*read_multiheaded)(OpenTTDTrainHandle) noexcept;
	uint8_t (*read_owner)(OpenTTDTrainHandle) noexcept;
	uint8_t (*read_vis_effect)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainConsistChangedRead (*read_consist_changed)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainConsistChanged1Read (*read_consist_changed_1)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainConsistChanged2Read (*read_consist_changed_2)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainCurveLimitRead (*read_curve_limit)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainStopLocationRead (*read_stop_location)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainCurrentMaxSpeedRead (*read_current_max_speed)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainCurrentMaxSpeed6Read (*read_current_max_speed_6)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainUpdateAccelerationRead (*read_update_acceleration)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainUpdateSpeedRead (*read_update_speed)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainTrackdirRead (*read_trackdir)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainCanLeaveRead (*read_can_leave)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainCrossingApproachRead (*read_crossing_approach)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainNextOffsetRead (*read_next_offset)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainAfterSwapRead (*read_after_swap)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainReverseSwapRead (*read_reverse_swap)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainApproachingEndRead (*read_approaching_end)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainLineEndsRead (*read_line_ends)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainSpeedZRead (*read_speed_z)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainMoveVehicleRead (*read_move_vehicle)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainMoveVehicle20Read (*read_move_vehicle_20)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainCollisionOneRead (*read_collision_one)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainCollisionOne22Read (*read_collision_one_22)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainDeleteLastRead (*read_delete_last)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainStayDepotRead (*read_stay_depot)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainLocoRead (*read_loco)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainLoco26Read (*read_loco_26)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainTickRead (*read_tick_state)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainNeedsServiceRead (*read_needs_service)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainNextForceRead (*read_next_force)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainReverseCommandRead (*read_reverse_command)(OpenTTDTrainHandle) noexcept;

	void (*write_tile)(OpenTTDTrainHandle, uint32_t) noexcept;
	void (*write_dest)(OpenTTDTrainHandle, uint32_t) noexcept;
	void (*write_x)(OpenTTDTrainHandle, int32_t) noexcept;
	void (*write_y)(OpenTTDTrainHandle, int32_t) noexcept;
	void (*write_z)(OpenTTDTrainHandle, int32_t) noexcept;
	void (*write_direction)(OpenTTDTrainHandle, uint8_t) noexcept;
	void (*write_speed)(OpenTTDTrainHandle, uint16_t) noexcept;
	void (*write_tick)(OpenTTDTrainHandle, uint8_t) noexcept;
	void (*write_running)(OpenTTDTrainHandle, uint8_t) noexcept;
	void (*write_day)(OpenTTDTrainHandle, uint8_t) noexcept;
	void (*write_order_time)(OpenTTDTrainHandle, int32_t) noexcept;
	void (*write_progress)(OpenTTDTrainHandle, uint8_t) noexcept;
	void (*write_subspeed)(OpenTTDTrainHandle, uint8_t) noexcept;
	void (*write_gv_flags)(OpenTTDTrainHandle, uint16_t) noexcept;
	void (*write_acceleration)(OpenTTDTrainHandle, uint8_t) noexcept;
	void (*write_length)(OpenTTDTrainHandle, uint8_t) noexcept;
	void (*write_total_length)(OpenTTDTrainHandle, uint16_t) noexcept;
	void (*write_first_engine)(OpenTTDTrainHandle, uint16_t) noexcept;
	void (*write_max_speed)(OpenTTDTrainHandle, uint16_t) noexcept;
	void (*write_cargo_cap)(OpenTTDTrainHandle, uint16_t) noexcept;
	void (*write_refit_cap)(OpenTTDTrainHandle, uint16_t) noexcept;
	void (*write_cargo_age)(OpenTTDTrainHandle, uint16_t) noexcept;
	void (*write_last_station)(OpenTTDTrainHandle, uint16_t) noexcept;
	void (*write_colourmap)(OpenTTDTrainHandle, uint64_t) noexcept;
	void (*write_status)(OpenTTDTrainHandle, uint8_t) noexcept;

	uint64_t (*acceleration)(OpenTTDTrainHandle) noexcept;
	uint64_t (*acc_model)(OpenTTDTrainHandle) noexcept;
	uint64_t (*acc_type)(OpenTTDTrainHandle) noexcept;
	uint64_t (*advance_distance)(OpenTTDTrainHandle) noexcept;
	void (*age)(OpenTTDTrainHandle) noexcept;
	uint64_t (*all_powered)(OpenTTDTrainHandle, uint64_t) noexcept;
	uint64_t (*ambient_sound)(OpenTTDTrainHandle) noexcept;
	void (*arrival_news)(OpenTTDTrainHandle, uint16_t) noexcept;
	void (*arrival_triggers)(OpenTTDTrainHandle, uint16_t) noexcept;
	uint64_t (*axis_diag)(OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*backoff)(OpenTTDTrainHandle) noexcept;
	void (*base_viewport)(OpenTTDTrainHandle, uint8_t) noexcept;
	void (*begin_loading)(OpenTTDTrainHandle) noexcept;
	uint64_t (*bridge_speed)(OpenTTDTrainHandle, uint32_t) noexcept;
	void (*cache_override)(OpenTTDTrainHandle) noexcept;
	uint64_t (*callback_length)(OpenTTDTrainHandle) noexcept;
	uint64_t (*capacity)(OpenTTDTrainHandle) noexcept;
	void (*capacity_error)(OpenTTDTrainHandle) noexcept;
	uint64_t (*cargo_age_default)(OpenTTDTrainHandle) noexcept;
	void (*cargo_changed)(OpenTTDTrainHandle) noexcept;
	uint64_t (*chain_depot)(OpenTTDTrainHandle) noexcept;
	void (*check_breakdown)(OpenTTDTrainHandle) noexcept;
	void (*check_next)(OpenTTDTrainHandle) noexcept;
	void (*check_orders)(OpenTTDTrainHandle) noexcept;
	uint64_t (*check_reverse)(OpenTTDTrainHandle) noexcept;
	uint64_t (*choose_track)(OpenTTDTrainHandle, uint32_t, uint8_t, uint8_t) noexcept;
	void (*clear_reservation)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	uint64_t (*compatible_rail_owner)(OpenTTDTrainHandle, uint32_t) noexcept;
	void (*consist_windows)(OpenTTDTrainHandle) noexcept;
	uint64_t (*cost_class)(OpenTTDTrainHandle) noexcept;
	uint64_t (*cost_default)(OpenTTDTrainHandle) noexcept;
	uint64_t (*cost_divisor)(OpenTTDTrainHandle) noexcept;
	uint64_t (*count_chain)(OpenTTDTrainHandle) noexcept;
	void (*crash_event)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*crash_ground)(OpenTTDTrainHandle, uint8_t) noexcept;
	void (*crash_news)(OpenTTDTrainHandle, uint32_t) noexcept;
	void (*crash_rating)(OpenTTDTrainHandle) noexcept;
	void (*crash_sound)(OpenTTDTrainHandle) noexcept;
	uint64_t (*crossing_barred)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*crossing_rail_axis)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*crossing_reserved)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*crossing_road_axis)(OpenTTDTrainHandle, uint32_t) noexcept;
	void (*crossing_sound)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*curve_advantage)(OpenTTDTrainHandle) noexcept;
	uint64_t (*curve_mod)(OpenTTDTrainHandle) noexcept;
	uint64_t (*day_ticks)(OpenTTDTrainHandle) noexcept;
	void (*decrease_value)(OpenTTDTrainHandle) noexcept;
	void (*delete_vehicle)(OpenTTDTrainHandle) noexcept;
	uint64_t (*depot_dir)(OpenTTDTrainHandle, uint32_t) noexcept;
	void (*depot_dirty)(OpenTTDTrainHandle) noexcept;
	uint64_t (*depot_index)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*depot_track)(OpenTTDTrainHandle, uint32_t) noexcept;
	void (*depot_window)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*diag_axis)(OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*diag_between)(OpenTTDTrainHandle, uint32_t, uint32_t) noexcept;
	uint64_t (*diag_reaches_tracks)(OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*diag_trackdir)(OpenTTDTrainHandle, uint8_t) noexcept;
	void (*dirty_tile)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*dir_diag)(OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*disaster_sound)(OpenTTDTrainHandle) noexcept;
	void (*disconnect)(OpenTTDTrainHandle) noexcept;
	void (*economy_age)(OpenTTDTrainHandle) noexcept;
	uint64_t (*engine_power)(OpenTTDTrainHandle) noexcept;
	void (*enter_depot)(OpenTTDTrainHandle) noexcept;
	uint64_t (*enter_tile)(OpenTTDTrainHandle, uint32_t, int32_t, int32_t) noexcept;
	OpenTTDTrainDepot (*find_depot)(OpenTTDTrainHandle, int32_t) noexcept;
	uint64_t (*first_track)(OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*grf_version)(OpenTTDTrainHandle) noexcept;
	uint64_t (*handle_breakdown)(OpenTTDTrainHandle) noexcept;
	uint64_t (*has_depot_res)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*has_reserved)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	uint64_t (*has_signal)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	uint64_t (*has_signals)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*has_signal_td)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	void (*hide_fill)(OpenTTDTrainHandle) noexcept;
	uint64_t (*inclination)(OpenTTDTrainHandle, uint8_t, uint8_t) noexcept;
	void (*invalidate_grf)(OpenTTDTrainHandle) noexcept;
	uint64_t (*invalid_price)(OpenTTDTrainHandle) noexcept;
	uint64_t (*is_bridge)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*is_crossing)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*is_depot)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*is_plain_rail)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*is_railway)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*is_station)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*is_station_any)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*is_tunnelbridge)(OpenTTDTrainHandle, uint32_t) noexcept;
	void (*large_explosion)(OpenTTDTrainHandle) noexcept;
	void (*last_speed)(OpenTTDTrainHandle) noexcept;
	void (*leave_sound)(OpenTTDTrainHandle) noexcept;
	void (*leave_station)(OpenTTDTrainHandle) noexcept;
	void (*leave_unbunch)(OpenTTDTrainHandle) noexcept;
	uint64_t (*length_callback)(OpenTTDTrainHandle) noexcept;
	void (*length_changed)(OpenTTDTrainHandle) noexcept;
	uint64_t (*length_default)(OpenTTDTrainHandle) noexcept;
	void (*length_error)(OpenTTDTrainHandle, uint16_t) noexcept;
	void (*loading)(OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*local_company)(OpenTTDTrainHandle) noexcept;
	uint64_t (*lost_warn)(OpenTTDTrainHandle) noexcept;
	uint64_t (*map_size)(OpenTTDTrainHandle) noexcept;
	uint64_t (*max_depot_penalty)(OpenTTDTrainHandle) noexcept;
	uint64_t (*needs_service)(OpenTTDTrainHandle) noexcept;
	uint64_t (*no_90)(OpenTTDTrainHandle, uint8_t, uint8_t) noexcept;
	uint64_t (*oneway_blocking)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	void (*order_depot_service)(OpenTTDTrainHandle, uint16_t) noexcept;
	void (*order_dummy)(OpenTTDTrainHandle) noexcept;
	void (*order_free)(OpenTTDTrainHandle) noexcept;
	uint64_t (*order_max_speed)(OpenTTDTrainHandle) noexcept;
	uint64_t (*order_stop)(OpenTTDTrainHandle, uint16_t) noexcept;
	uint64_t (*other_end)(OpenTTDTrainHandle, uint32_t) noexcept;
	void (*pay_running)(OpenTTDTrainHandle, int64_t) noexcept;
	uint64_t (*pbs_signal_type)(OpenTTDTrainHandle) noexcept;
	uint64_t (*platform_ahead)(OpenTTDTrainHandle, uint16_t, uint32_t) noexcept;
	uint64_t (*platform_length)(OpenTTDTrainHandle, uint16_t, uint32_t) noexcept;
	void (*position)(OpenTTDTrainHandle) noexcept;
	uint64_t (*pow_wag_power)(OpenTTDTrainHandle) noexcept;
	uint64_t (*price)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*process_orders)(OpenTTDTrainHandle) noexcept;
	uint64_t (*property)(OpenTTDTrainHandle, uint8_t, uint32_t) noexcept;
	uint64_t (*railveh_wagon)(OpenTTDTrainHandle) noexcept;
	uint64_t (*rail_tilt)(OpenTTDTrainHandle) noexcept;
	uint64_t (*rail_type)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*rail_types)(OpenTTDTrainHandle) noexcept;
	uint64_t (*reserve_paths)(OpenTTDTrainHandle) noexcept;
	void (*reserve_under)(OpenTTDTrainHandle) noexcept;
	void (*reset_unbunch)(OpenTTDTrainHandle) noexcept;
	uint64_t (*reverse_at_signals)(OpenTTDTrainHandle) noexcept;
	uint64_t (*reverse_single_blocked)(OpenTTDTrainHandle) noexcept;
	void (*reverse_windows)(OpenTTDTrainHandle) noexcept;
	void (*running_windows)(OpenTTDTrainHandle) noexcept;
	void (*service)(OpenTTDTrainHandle) noexcept;
	uint64_t (*servint)(OpenTTDTrainHandle) noexcept;
	void (*set_depot_res)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	void (*set_next)(OpenTTDTrainHandle, OpenTTDTrainHandle) noexcept;
	void (*set_platform_res)(OpenTTDTrainHandle, uint32_t, uint8_t, uint8_t) noexcept;
	void (*set_signal_state)(OpenTTDTrainHandle, uint32_t, uint8_t, uint8_t) noexcept;
	void (*set_tunnel_res)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	void (*show_effect)(OpenTTDTrainHandle) noexcept;
	uint64_t (*show_reservation)(OpenTTDTrainHandle) noexcept;
	OpenTTDSignalContext (*signal_context)() noexcept;
	uint64_t (*signal_has_pbs)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	uint64_t (*signal_pbs)(OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*signal_type)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	uint64_t (*sigseg_full)(OpenTTDTrainHandle) noexcept;
	uint64_t (*sigseg_pbs)(OpenTTDTrainHandle) noexcept;
	void (*small_explosion)(OpenTTDTrainHandle, int32_t, int32_t, int32_t) noexcept;
	uint64_t (*speed_default)(OpenTTDTrainHandle) noexcept;
	void (*start_stop_dirty)(OpenTTDTrainHandle) noexcept;
	uint64_t (*station)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*station_axis)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*station_compatible)(OpenTTDTrainHandle, uint32_t, uint32_t) noexcept;
	uint64_t (*station_dest)(OpenTTDTrainHandle) noexcept;
	uint64_t (*stopped_in_depot)(OpenTTDTrainHandle) noexcept;
	uint64_t (*stop_location)(OpenTTDTrainHandle) noexcept;
	void (*stuck_news)(OpenTTDTrainHandle) noexcept;
	void (*suppress_implicit)(OpenTTDTrainHandle) noexcept;
	uint64_t (*ticks_leave_depot)(OpenTTDTrainHandle) noexcept;
	uint64_t (*tile_add_diag)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	uint64_t (*tile_offset_axis)(OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*tile_offset_diag)(OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*tile_owner)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*tile_rail_type)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*tile_virt)(OpenTTDTrainHandle, int32_t, int32_t) noexcept;
	uint64_t (*trackdir_exit)(OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*trackdir_reaches)(OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*track_bits)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*track_crosses)(OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*track_direction)(OpenTTDTrainHandle, uint8_t, uint8_t) noexcept;
	uint64_t (*track_status)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	void (*train_list)(OpenTTDTrainHandle) noexcept;
	uint64_t (*train_visit)(OpenTTDTrainHandle) noexcept;
	void (*truncate_cargo)(OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*try_reserve)(OpenTTDTrainHandle, uint32_t, uint8_t, uint8_t) noexcept;
	uint64_t (*tunnel_dir)(OpenTTDTrainHandle, uint32_t) noexcept;
	void (*unreserve)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	void (*update_delta)(OpenTTDTrainHandle) noexcept;
	uint64_t (*update_speed)(OpenTTDTrainHandle, uint32_t, int32_t, int32_t) noexcept;
	uint64_t (*user_default)(OpenTTDTrainHandle) noexcept;
	uint64_t (*veh_exit_dir)(OpenTTDTrainHandle, uint8_t, uint8_t) noexcept;
	void (*viewport)(OpenTTDTrainHandle, uint8_t, uint8_t) noexcept;
	void (*view_window)(OpenTTDTrainHandle) noexcept;
	uint64_t (*visit_type)(OpenTTDTrainHandle, uint16_t) noexcept;
	void (*vis_effect)(OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*wagon_override)(OpenTTDTrainHandle) noexcept;
	uint64_t (*wagon_speed_limits)(OpenTTDTrainHandle) noexcept;
	uint64_t (*wait_oneway)(OpenTTDTrainHandle) noexcept;
	uint64_t (*wait_pbs)(OpenTTDTrainHandle) noexcept;
	uint64_t (*wait_twoway)(OpenTTDTrainHandle) noexcept;
	uint64_t (*wait_unbunch)(OpenTTDTrainHandle) noexcept;
	void (*write_crossing_bar)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	void (*write_crossing_res)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	void (*write_visit_type)(OpenTTDTrainHandle, uint16_t) noexcept;

	uint8_t (*visit_tile)(uint32_t, void *, uint8_t (*)(void *, OpenTTDTrainHandle)) noexcept;
	uint8_t (*visit_near)(int32_t, int32_t, void *, uint8_t (*)(void *, OpenTTDTrainHandle)) noexcept;
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
