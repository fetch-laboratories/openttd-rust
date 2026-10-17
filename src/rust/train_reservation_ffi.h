/*
	* This file is part of OpenTTD.
	* OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
	* OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
	* See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
	*/

/** @file train_reservation_ffi.h Train controller reservation owner. */
#ifndef OPENTTD_RUST_TRAIN_RESERVATION_FFI_H
#define OPENTTD_RUST_TRAIN_RESERVATION_FFI_H
#include <stdint.h>
#include "train_ffi.h"
#include "vehicle_ffi.h"
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
	uint32_t (*read_tile)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainHandle (*read_next)(OpenTTDTrainHandle) noexcept;
	uint16_t (*read_last_station)(OpenTTDTrainHandle) noexcept;
	uint8_t (*read_direction)(OpenTTDTrainHandle) noexcept;
	uint8_t (*read_order)(OpenTTDTrainHandle) noexcept;
	uint8_t (*read_num_orders)(OpenTTDTrainHandle) noexcept;
	uint8_t (*read_order_index)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainReservationFreeRead (*read_free)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainReservationFree1Read (*read_free_1)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainReservationNewRead (*read_new)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainReservationChooseRead (*read_choose)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainReservationChoose4Read (*read_choose_4)(OpenTTDTrainHandle) noexcept;
	OpenTTDTrainReservationCheckNextRead (*read_check_next)(OpenTTDTrainHandle) noexcept;

	uint64_t (*all_compat)(void *, OpenTTDTrainHandle) noexcept;
	uint64_t (*backoff)(void *, OpenTTDTrainHandle) noexcept;
	uint64_t (*bits_track)(void *, OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*blocking)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	uint64_t (*check_reverse)(void *, OpenTTDTrainHandle) noexcept;
	uint64_t (*compat_station)(void *, OpenTTDTrainHandle, uint32_t, uint32_t) noexcept;
	uint64_t (*conditional)(void *, OpenTTDTrainHandle, uint8_t) noexcept;
	void (*copy_order)(void *, OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*cross_dirs)(void *, OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*cross_tracks)(void *, OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*depot_dir)(void *, OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*depot_reserved)(void *, OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*diag_reach_dirs)(void *, OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*diag_track)(void *, OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*enter_td)(void *, OpenTTDTrainHandle, uint8_t, uint8_t) noexcept;
	uint64_t (*exit_dir)(void *, OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*free)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	uint64_t (*green)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	uint64_t (*has_pbs)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	uint64_t (*has_reserved)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	uint64_t (*has_signal)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	void (*increment_order)(void *, OpenTTDTrainHandle) noexcept;
	uint64_t (*is_bridge)(void *, OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*is_depot)(void *, OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*is_pbs)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	uint64_t (*is_plain)(void *, OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*is_railway)(void *, OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*is_station)(void *, OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*is_tunnel)(void *, OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*is_waypoint)(void *, OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*line_reverse)(void *, OpenTTDTrainHandle) noexcept;
	void (*mark_tile)(void *, OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*needs_service)(void *, OpenTTDTrainHandle) noexcept;
	uint64_t (*oneway)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	uint64_t (*order_service)(void *, OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*order_stop)(void *, OpenTTDTrainHandle, uint16_t) noexcept;
	uint64_t (*order_type)(void *, OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*other_end)(void *, OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*overlap)(void *, OpenTTDTrainHandle, uint8_t) noexcept;
	void (*path_result)(void *, OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*rail90)(void *, OpenTTDTrainHandle, uint32_t, uint32_t) noexcept;
	uint64_t (*reach_dirs)(void *, OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*reach_tracks)(void *, OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*reserved)(void *, OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*reserve_paths)(void *, OpenTTDTrainHandle) noexcept;
	void (*restore_order)(void *, OpenTTDTrainHandle) noexcept;
	void (*save_order)(void *, OpenTTDTrainHandle) noexcept;
	void (*service)(void *, OpenTTDTrainHandle) noexcept;
	void (*set_depot)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	void (*set_depot_dest)(void *, OpenTTDTrainHandle, uint32_t) noexcept;
	void (*set_platform)(void *, OpenTTDTrainHandle, uint32_t, uint8_t, uint8_t) noexcept;
	void (*set_signal)(void *, OpenTTDTrainHandle, uint32_t, uint8_t, uint8_t) noexcept;
	void (*set_tunnel)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	uint64_t (*show_res)(void *, OpenTTDTrainHandle) noexcept;
	OpenTTDSignalContext (*signal_context)() noexcept;
	const OpenTTDVehicleServices *vehicle;
	const OpenTTDSharedServices *shared;
	void (*start_stop)(void *, OpenTTDTrainHandle) noexcept;
	uint64_t (*station)(void *, OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*station_train)(void *, OpenTTDTrainHandle, uint16_t) noexcept;
	uint64_t (*station_xy)(void *, OpenTTDTrainHandle, uint16_t) noexcept;
	void (*stuck)(void *, OpenTTDTrainHandle) noexcept;
	uint64_t (*tile_add)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	uint64_t (*tile_offset)(void *, OpenTTDTrainHandle, uint8_t) noexcept;
	uint64_t (*trackdir)(void *, OpenTTDTrainHandle) noexcept;
	uint64_t (*track_status)(void *, OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*try_track)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	uint64_t (*tunnel_dir)(void *, OpenTTDTrainHandle, uint32_t) noexcept;
	uint64_t (*tunnel_free)(void *, OpenTTDTrainHandle, uint32_t, uint32_t) noexcept;
	void (*unreserve)(void *, OpenTTDTrainHandle, uint32_t, uint8_t) noexcept;
	void (*write_dest)(void *, OpenTTDTrainHandle, uint32_t) noexcept;
	void (*write_last)(void *, OpenTTDTrainHandle, uint16_t) noexcept;
	void (*write_suppress)(void *, OpenTTDTrainHandle, uint8_t) noexcept;

	uint8_t (*follow)(OpenTTDTrainHandle, uint64_t, OpenTTDTrainReservationFollow *) noexcept;
	OpenTTDTrainReservationPbs (*origin)(OpenTTDTrainHandle, uint8_t) noexcept;
	OpenTTDTrainReservationSearch (*pathfind)(OpenTTDTrainHandle, uint32_t, uint8_t, uint8_t, uint8_t, uint8_t) noexcept;
	uint8_t (*safe_track)(OpenTTDTrainHandle, uint32_t, uint8_t, uint8_t) noexcept;
	uint8_t (*process_orders)(OpenTTDTrainHandle) noexcept;
	uint8_t (*update_order_dest)(OpenTTDTrainHandle, uint8_t) noexcept;
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
