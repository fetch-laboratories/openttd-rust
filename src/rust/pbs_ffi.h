/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file pbs_ffi.h Synchronous PBS owner; native map storage remains canonical. */
#ifndef OPENTTD_RUST_PBS_FFI_H
#define OPENTTD_RUST_PBS_FFI_H
#include "services_ffi.h"
#include "train_ffi.h"
#include "train_reservation_ffi.h"
struct BaseStation;
struct OpenTTDPbsTrainOutput;
struct OpenTTDPbsServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDRailMapServices *rail;
	const OpenTTDTrainServices *train;
	TrackBits (*depot_bits)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_bits);
	TrackBits (*crossing_bits)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(crossing_bits);
	TrackBits (*station_bits)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_bits);
	TrackBits (*wormhole_bits)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(wormhole_bits);
	void (*set_depot)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_depot);
	void (*set_station)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_station);
	void (*set_wormhole)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_wormhole);
	bool (*try_track)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(try_track);
	void (*unreserve_track)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(unreserve_track);
	bool (*show_reservation)() noexcept OPENTTD_SERVICE_DEFAULT(show_reservation);
	BaseStation * (*station)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station);

	bool (*is_crossing)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_crossing);
	OpenTTDTrainHandle (*read_train_on_res)(const OpenTTDPbsTrainOutput *) noexcept OPENTTD_SERVICE_DEFAULT(read_train_on_res);
	void (*write_train_on_res)(OpenTTDPbsTrainOutput *, OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(write_train_on_res);
	bool assertions;
	OpenTTDTrainHandle (*handle)(const Train *) noexcept OPENTTD_SERVICE_DEFAULT(handle);
	uint64_t (*compatible)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(compatible);
	bool (*rail90)(uint8_t, uint8_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(rail90);
	bool (*blocking)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(blocking);
	bool (*has_pbs)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(has_pbs);
	uint32_t (*index)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(index);
};
const OpenTTDPbsServices &GetPbsServices() noexcept;
extern "C" {
uint8_t openttd_rust_pbs_reserved(const OpenTTDPbsServices *, uint32_t);
void openttd_rust_pbs_platform(const OpenTTDPbsServices *, uint32_t, uint8_t, bool);
uint8_t openttd_rust_pbs_try_reserve_abi(const OpenTTDPbsServices *, uint32_t, uint8_t, bool);
extern "C++" inline bool openttd_rust_pbs_try_reserve(const OpenTTDPbsServices *a0, uint32_t a1, uint8_t a2, bool a3) { return openttd_rust_pbs_try_reserve_abi(a0, a1, a2, a3) != 0; }
void openttd_rust_pbs_unreserve(const OpenTTDPbsServices *, uint32_t, uint8_t);
OpenTTDTrainReservationPbs openttd_rust_pbs_follow(const OpenTTDPbsServices *, uint8_t, uint64_t, uint32_t, uint8_t);
OpenTTDTrainReservationPbs openttd_rust_pbs_follow_train(const OpenTTDPbsServices *, OpenTTDTrainHandle, OpenTTDPbsTrainOutput *);
OpenTTDTrainHandle openttd_rust_pbs_train_for(const OpenTTDPbsServices *, uint32_t, uint8_t);
uint8_t openttd_rust_pbs_safe_abi(const OpenTTDPbsServices *, OpenTTDTrainHandle, uint32_t, uint8_t, bool, bool);
extern "C++" inline bool openttd_rust_pbs_safe(const OpenTTDPbsServices *a0, OpenTTDTrainHandle a1, uint32_t a2, uint8_t a3, bool a4, bool a5) { return openttd_rust_pbs_safe_abi(a0, a1, a2, a3, a4, a5) != 0; }
uint8_t openttd_rust_pbs_free_abi(const OpenTTDPbsServices *, OpenTTDTrainHandle, uint32_t, uint8_t, bool);
extern "C++" inline bool openttd_rust_pbs_free(const OpenTTDPbsServices *a0, OpenTTDTrainHandle a1, uint32_t a2, uint8_t a3, bool a4) { return openttd_rust_pbs_free_abi(a0, a1, a2, a3, a4) != 0; }
}
#endif
