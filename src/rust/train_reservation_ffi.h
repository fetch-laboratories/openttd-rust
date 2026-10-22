/*
	* This file is part of OpenTTD.
	* OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
	* OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
	* See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
	*/

/** @file train_reservation_ffi.h Train controller reservation owner. */
#ifndef OPENTTD_RUST_TRAIN_RESERVATION_FFI_H
#define OPENTTD_RUST_TRAIN_RESERVATION_FFI_H
#include "service_defaults.hpp"
#include <stdint.h>
#include "train_ffi.h"
#include "vehicle_ffi.h"
#include "orders_ffi.h"
struct OpenTTDTrainReservationLeaves;
const OpenTTDTrainReservationLeaves &GetTrainReservationServices() noexcept;
extern "C" {
struct OpenTTDTrainState;
struct OpenTTDTrainReservationFollow {
	uint32_t old_tile, new_tile;
	int32_t skipped;
	uint16_t dirs;
	uint8_t old_td, exitdir, tunnel, bridge, station, error;
};
struct OpenTTDTrainReservationPbs {
	uint32_t tile;
	OpenTTDTrainHandle other;
	uint8_t td, okay;
};
struct OpenTTDTrainReservationSearch {
	uint8_t track;
	uint32_t tile, final_dest;
	uint8_t td, found, okay;
};
struct OpenTTDTrainReservationFreeRead {
	uint32_t tile;
};
struct OpenTTDTrainReservationFree1Read {
	uint32_t tile;
	OpenTTDTrainHandle next;
};
struct OpenTTDTrainReservationNewRead {
	uint32_t dest;
	uint16_t last_station;
	uint8_t order_index;
	uint8_t suppress;
};
struct OpenTTDTrainReservationChooseRead {
	uint32_t tile;
	uint32_t dest;
	uint16_t destination;
	uint8_t order;
};
struct OpenTTDTrainReservationChoose4Read {
	uint8_t order;
	uint8_t nearest;
};
struct OpenTTDTrainReservationCheckNextRead {
	uint32_t tile;
	uint32_t dest;
	uint16_t destination;
	uint8_t order;
	uint8_t num_orders;
};
struct OpenTTDTrainReservationChoice { uint8_t track, reserved; };
struct OpenTTDTrainReservationLeaves {
	uint32_t (*read_tile)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_tile);
	OpenTTDTrainHandle (*read_next)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_next);
	uint16_t (*read_last_station)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_last_station);
	uint8_t (*read_direction)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_direction);
	uint8_t (*read_order)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_order);
	uint8_t (*read_num_orders)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_num_orders);
	uint8_t (*read_order_index)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_order_index);
	OpenTTDTrainReservationFreeRead (*read_free)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_free);
	OpenTTDTrainReservationFree1Read (*read_free_1)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_free_1);
	OpenTTDTrainReservationNewRead (*read_new)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_new);
	OpenTTDTrainReservationChooseRead (*read_choose)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_choose);
	OpenTTDTrainReservationChoose4Read (*read_choose_4)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_choose_4);
	OpenTTDTrainReservationCheckNextRead (*read_check_next)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(read_check_next);

	uint64_t (*all_compat)(void *, OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(all_compat);
	uint64_t (*backoff)(void *, OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(backoff);
	uint64_t (*bits_track)(void *, OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(bits_track);
	uint64_t (*blocking)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(blocking);
	uint64_t (*check_reverse)(void *, OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(check_reverse);
	uint64_t (*compat_station)(void *, OpenTTDTrainHandle, uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(compat_station);
	uint64_t (*conditional)(void *, OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(conditional);
	void (*copy_order)(void *, OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(copy_order);
	uint64_t (*cross_dirs)(void *, OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(cross_dirs);
	uint64_t (*cross_tracks)(void *, OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(cross_tracks);
	uint64_t (*depot_dir)(void *, OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_dir);
	uint64_t (*depot_reserved)(void *, OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_reserved);
	uint64_t (*diag_reach_dirs)(void *, OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(diag_reach_dirs);
	uint64_t (*diag_track)(void *, OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(diag_track);
	uint64_t (*enter_td)(void *, OpenTTDTrainHandle, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(enter_td);
	uint64_t (*exit_dir)(void *, OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(exit_dir);
	uint64_t (*free)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(free);
	uint64_t (*green)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(green);
	uint64_t (*has_pbs)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(has_pbs);
	uint64_t (*has_reserved)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(has_reserved);
	uint64_t (*has_signal)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(has_signal);
	void (*increment_order)(void *, OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(increment_order);
	uint64_t (*is_bridge)(void *, OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_bridge);
	uint64_t (*is_depot)(void *, OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_depot);
	uint64_t (*is_pbs)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(is_pbs);
	uint64_t (*is_plain)(void *, OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_plain);
	uint64_t (*is_railway)(void *, OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_railway);
	uint64_t (*is_station)(void *, OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_station);
	uint64_t (*is_tunnel)(void *, OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_tunnel);
	uint64_t (*is_waypoint)(void *, OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_waypoint);
	uint64_t (*line_reverse)(void *, OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(line_reverse);
	void (*mark_tile)(void *, OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(mark_tile);
	uint64_t (*oneway)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(oneway);
	uint64_t (*order_service)(void *, OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(order_service);
	uint64_t (*order_stop)(void *, OpenTTDTrainHandle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(order_stop);
	uint64_t (*order_type)(void *, OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(order_type);
	uint64_t (*other_end)(void *, OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(other_end);
	uint64_t (*overlap)(void *, OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(overlap);
	void (*path_result)(void *, OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(path_result);
	uint64_t (*rail90)(void *, OpenTTDTrainHandle, uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rail90);
	uint64_t (*reach_dirs)(void *, OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(reach_dirs);
	uint64_t (*reach_tracks)(void *, OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(reach_tracks);
	uint64_t (*reserved)(void *, OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(reserved);
	uint64_t (*reserve_paths)(void *, OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(reserve_paths);
	void (*restore_order)(OpenTTDTrainHandle, const OpenTTDOrderFields *) noexcept OPENTTD_SERVICE_DEFAULT(restore_order);
	OpenTTDOrderFields (*save_order)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(save_order);
	void (*service)(void *, OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(service);
	void (*set_depot)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_depot);
	void (*set_depot_dest)(void *, OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_depot_dest);
	void (*set_platform)(void *, OpenTTDTrainHandle, uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_platform);
	void (*set_signal)(void *, OpenTTDTrainHandle, uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_signal);
	void (*set_tunnel)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_tunnel);
	uint64_t (*show_res)(void *, OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(show_res);
	OpenTTDSignalContext (*signal_context)() noexcept OPENTTD_SERVICE_DEFAULT(signal_context);
	const OpenTTDVehicleServices *vehicle;
	const OpenTTDSharedServices *shared;
	void (*start_stop)(void *, OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(start_stop);
	uint64_t (*station)(void *, OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station);
	uint64_t (*station_train)(void *, OpenTTDTrainHandle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(station_train);
	uint64_t (*station_xy)(void *, OpenTTDTrainHandle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(station_xy);
	void (*stuck)(void *, OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(stuck);
	uint64_t (*tile_add)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_add);
	uint64_t (*tile_offset)(void *, OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_offset);
	uint64_t (*trackdir)(void *, OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(trackdir);
	uint64_t (*track_status)(void *, OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(track_status);
	uint64_t (*try_track)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(try_track);
	uint64_t (*tunnel_dir)(void *, OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tunnel_dir);
	uint64_t (*tunnel_free)(void *, OpenTTDTrainHandle, uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tunnel_free);
	void (*unreserve)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(unreserve);
	void (*write_dest)(void *, OpenTTDTrainHandle, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_dest);
	void (*write_last)(void *, OpenTTDTrainHandle, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(write_last);
	void (*write_suppress)(void *, OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_suppress);

	uint8_t (*follow)(OpenTTDTrainHandle, uint64_t, OpenTTDTrainReservationFollow *) noexcept OPENTTD_SERVICE_DEFAULT(follow);
	OpenTTDTrainReservationPbs (*origin)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(origin);
	OpenTTDTrainReservationSearch (*pathfind)(OpenTTDTrainHandle, uint32_t, uint8_t, uint8_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(pathfind);
	uint8_t (*safe_track)(OpenTTDTrainHandle, uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(safe_track);
	uint8_t (*process_orders)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(process_orders);
	uint8_t (*update_order_dest)(OpenTTDTrainHandle, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(update_order_dest);
};
/* Synchronous serialized services retain no owner/world borrow. The native stack
 * owns temporary Order snapshots; nested entries get distinct contexts. Rust
 * controls restoration exactly once and keeps the original ordered signal rollback
 * scratch. Ordinary-play callbacks are direct noexcept calls; escaping exceptions
 * and Rust panics terminate. Immutable table/context pointers live until return. */
void openttd_rust_train_reservation_check_next(OpenTTDTrainHandle, OpenTTDVehicle *, const OpenTTDTrainReservationLeaves *, void *);
void openttd_rust_train_reservation_clear(OpenTTDTrainHandle, OpenTTDVehicle *, uint32_t, uint8_t, const OpenTTDTrainReservationLeaves *, void *);
void openttd_rust_train_reservation_free(OpenTTDTrainHandle, OpenTTDVehicle *, const OpenTTDTrainReservationLeaves *, void *);
OpenTTDTrainReservationChoice openttd_rust_train_reservation_choose(OpenTTDTrainHandle, OpenTTDVehicle *, uint32_t, uint8_t, uint8_t, uint8_t, uint8_t, const OpenTTDTrainReservationLeaves *, void *);
uint8_t openttd_rust_train_reservation_try_path(OpenTTDTrainHandle, OpenTTDVehicle *, uint8_t, uint8_t, const OpenTTDTrainReservationLeaves *, void *);
uint8_t openttd_rust_train_reservation_check_reverse(OpenTTDTrainHandle, OpenTTDVehicle *, const OpenTTDTrainReservationLeaves *, void *);
uint32_t openttd_rust_train_reservation_station_location(OpenTTDTrainHandle, OpenTTDVehicle *, uint16_t, const OpenTTDTrainReservationLeaves *, void *);
void openttd_rust_train_reservation_reserve_under(OpenTTDTrainHandle, OpenTTDVehicle *, const OpenTTDTrainReservationLeaves *, void *);
}
#endif
