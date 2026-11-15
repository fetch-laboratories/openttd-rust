struct OpenTTDEngineCapabilitiesServices;
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
#include "vehicle_ffi.h"
#include "services_ffi.h"
#include "signal_ffi.h"
struct OpenTTDVehicleServices;
struct OpenTTDGroundPhysicsServices;
/* Handles carry the canonical record and train owner. Both remain live until
 * their source deletion point; no Rust borrow spans a native callback. */
struct OpenTTDTrainDepot { uint32_t tile, length; };
struct Train;
struct OpenTTDTrainServices;
const OpenTTDTrainServices &GetTrainServices() noexcept;
OpenTTDTrainHandle TrainHandle(const Train *) noexcept;
extern "C" {
struct OpenTTDTrainServices {
	const OpenTTDGroundPhysicsServices *physics;

	void (*write_colourmap)(OpenTTDTrainHandle, uint64_t) noexcept OPENTTD_SERVICE_DEFAULT(write_colourmap);

	uint64_t (*advance_distance)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(advance_distance);
	uint64_t (*all_powered)(OpenTTDTrainHandle, uint64_t) noexcept OPENTTD_SERVICE_DEFAULT(all_powered);
	void (*arrival_news)(OpenTTDTrainHandle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(arrival_news);

	uint64_t (*axis_diag)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(axis_diag);
	uint8_t (*backoff)() noexcept OPENTTD_SERVICE_DEFAULT(backoff);
	void (*base_viewport)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(base_viewport);
	const OpenTTDEngineCapabilitiesServices *capabilities;
	void (*capacity_error)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(capacity_error);
	void (*check_next)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(check_next);
	void (*check_orders)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(check_orders);
	uint64_t (*check_reverse)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(check_reverse);
	uint64_t (*choose_track)(OpenTTDTrainHandle, uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(choose_track);
	void (*clear_reservation)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(clear_reservation);
	void (*consist_windows)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(consist_windows);
	uint64_t (*cost_divisor)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(cost_divisor);
	uint64_t (*count_chain)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(count_chain);
	void (*crash_event)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(crash_event);
	void (*crash_news)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(crash_news);
	void (*crash_sound)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(crash_sound);
	uint64_t (*crossing_barred)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(crossing_barred);
	uint64_t (*crossing_rail_axis)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(crossing_rail_axis);
	uint64_t (*crossing_road_axis)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(crossing_road_axis);
	void (*crossing_sound)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(crossing_sound);
	uint64_t (*curve_advantage)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(curve_advantage);
	uint64_t (*day_ticks)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(day_ticks);
	void (*delete_vehicle)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(delete_vehicle);
	uint64_t (*depot_dir)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_dir);
	void (*depot_dirty)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(depot_dirty);
	uint64_t (*depot_track)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_track);
	void (*depot_window)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_window);
	uint64_t (*diag_axis)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(diag_axis);
	uint64_t (*diag_between)(OpenTTDTrainHandle, uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(diag_between);
	uint64_t (*diag_reaches_tracks)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(diag_reaches_tracks);
	uint64_t (*diag_trackdir)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(diag_trackdir);
	uint64_t (*dir_diag)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(dir_diag);
	void (*disconnect)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(disconnect);
	OpenTTDTrainDepot (*find_depot)(OpenTTDTrainHandle, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(find_depot);
	uint64_t (*first_track)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(first_track);
	uint64_t (*has_depot_res)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(has_depot_res);
	uint64_t (*has_signal)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(has_signal);
	uint64_t (*has_signals)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(has_signals);
	uint64_t (*has_signal_td)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(has_signal_td);
	void (*hide_fill)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(hide_fill);
	void (*invalidate_grf)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(invalidate_grf);
	uint64_t (*is_bridge)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_bridge);
	uint64_t (*is_crossing)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_crossing);
	uint64_t (*is_depot)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_depot);
	uint64_t (*is_plain_rail)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_plain_rail);
	uint64_t (*is_station)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_station);
	uint64_t (*is_station_any)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_station_any);
	void (*leave_sound)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(leave_sound);
	void (*leave_unbunch)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(leave_unbunch);
	uint64_t (*local_company)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(local_company);
	uint64_t (*no_90)(OpenTTDTrainHandle, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(no_90);
	uint64_t (*oneway_blocking)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(oneway_blocking);
	void (*order_depot_service)(OpenTTDTrainHandle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(order_depot_service);
	void (*order_dummy)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(order_dummy);
	void (*order_free)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(order_free);
	uint64_t (*order_max_speed)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(order_max_speed);
	uint64_t (*other_end)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(other_end);
	void (*pay_running)(OpenTTDTrainHandle, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(pay_running);
	uint64_t (*pbs_signal_type)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(pbs_signal_type);
	uint64_t (*rail_type)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_type);
	bool (*reserve_paths)() noexcept OPENTTD_SERVICE_DEFAULT(reserve_paths);
	void (*reserve_under)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(reserve_under);
	void (*reset_unbunch)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(reset_unbunch);
	void (*reverse_windows)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(reverse_windows);
	void (*running_windows)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(running_windows);
	uint64_t (*servint)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(servint);
	void (*set_depot_res)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_depot_res);
	void (*set_signal_state)(OpenTTDTrainHandle, uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_signal_state);
	void (*set_tunnel_res)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_tunnel_res);
	uint64_t (*show_reservation)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(show_reservation);
	OpenTTDSignalContext (*signal_context)() noexcept OPENTTD_SERVICE_DEFAULT(signal_context);
	uint64_t (*signal_has_pbs)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(signal_has_pbs);
	uint64_t (*signal_pbs)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(signal_pbs);
	uint64_t (*signal_type)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(signal_type);
	uint64_t (*sigseg_full)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(sigseg_full);
	uint64_t (*sigseg_pbs)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(sigseg_pbs);
	void (*start_stop_dirty)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(start_stop_dirty);
	uint64_t (*station_axis)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_axis);
	uint64_t (*station_compatible)(OpenTTDTrainHandle, uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_compatible);
	uint64_t (*station_dest)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(station_dest);
	uint64_t (*stopped_in_depot)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(stopped_in_depot);
	uint64_t (*stop_location)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(stop_location);
	void (*stuck_news)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(stuck_news);
	void (*suppress_implicit)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(suppress_implicit);
	uint64_t (*tile_offset_axis)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_offset_axis);
	uint64_t (*tile_offset_diag)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_offset_diag);
	uint64_t (*tile_rail_type)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_rail_type);
	uint32_t (*tile_virt)(int32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_virt);
	uint64_t (*trackdir_exit)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(trackdir_exit);
	uint64_t (*trackdir_reaches)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(trackdir_reaches);
	uint64_t (*track_bits)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(track_bits);
	uint64_t (*track_crosses)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(track_crosses);
	uint64_t (*track_direction)(OpenTTDTrainHandle, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(track_direction);
	void (*train_list)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(train_list);
	void (*truncate_cargo)(OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(truncate_cargo);
	uint64_t (*veh_exit_dir)(OpenTTDTrainHandle, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(veh_exit_dir);
	void (*viewport)(OpenTTDTrainHandle, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(viewport);
	uint64_t (*wagon_override)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(wagon_override);
	uint64_t (*wagon_speed_limits)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(wagon_speed_limits);
	uint64_t (*wait_unbunch)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(wait_unbunch);
	void (*write_crossing_bar)(OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_crossing_bar);

	const OpenTTDVehicleServices *vehicle; ///< Base vehicle table, immutable for the process.
};

void openttd_rust_train_consist_changed(OpenTTDTrainHandle, uint8_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
int32_t openttd_rust_train_current_max_speed(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
int32_t openttd_rust_train_update_speed(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_mark_dirty(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
uint8_t openttd_rust_train_tick(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_calendar_day(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_economy_day(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
int64_t openttd_rust_train_running_cost(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
uint8_t openttd_rust_train_trackdir(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_mark_stuck(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_reverse_swap(OpenTTDTrainHandle, int32_t, int32_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_update_crossing(uint32_t, uint8_t, uint8_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_adjacent_crossing_dirty(uint32_t, uint8_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_crossing_removed(uint32_t, uint8_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
uint8_t openttd_rust_train_controller(OpenTTDTrainHandle, OpenTTDTrainHandle, uint8_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
uint32_t openttd_rust_train_crash(OpenTTDTrainHandle, uint8_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
void openttd_rust_train_needs_service(OpenTTDTrainHandle, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
uint8_t openttd_rust_train_reverse_command(OpenTTDTrainHandle, uint8_t, uint8_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
uint8_t openttd_rust_train_force_command(OpenTTDTrainHandle, uint8_t, const OpenTTDTrainServices *, const OpenTTDSharedServices *);
}
#endif
