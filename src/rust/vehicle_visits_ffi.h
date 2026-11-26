/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file vehicle_visits_ffi.h Route-link prediction and link statistics owned by Rust. */
#ifndef RUST_VEHICLE_VISITS_FFI_H
#define RUST_VEHICLE_VISITS_FFI_H
#include "station_service_ffi.h"
#include "vehicle_refit_ffi.h"

struct OpenTTDScheduleServices;
struct OpenTTDVehicleVisits {
	void (*hide_filling_percent)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(hide_filling_percent);
	uint16_t (*last_loading_station)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(last_loading_station);
	uint64_t (*last_loading_tick)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(last_loading_tick);
	const OpenTTDStationLinks *links;
	const OpenTTDStationLoading *loading;
	const OpenTTDVehicleRefitServices *refit;
	const OpenTTDScheduleServices *schedule;
	void (*set_last_loading_station)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_last_loading_station);
	void (*set_last_loading_tick)(OpenTTDVehicle *, uint64_t) noexcept OPENTTD_SERVICE_DEFAULT(set_last_loading_tick);
	void (*set_trip_occupancy)(OpenTTDVehicle *, int8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_trip_occupancy);
	const OpenTTDSharedServices *shared;
	const OpenTTDStationServices *station;
};
const OpenTTDVehicleVisits &GetRustVehicleVisits() noexcept;
inline const OpenTTDVehicleVisits *RustVehicleVisits() noexcept { return &GetRustVehicleVisits(); }
extern "C" {
uint8_t openttd_rust_percent_filled(OpenTTDVehicle *, uint32_t *, const OpenTTDVehicleVisits *);
void openttd_rust_visit_leave_station(OpenTTDVehicle *, const OpenTTDVehicleVisits *);
void openttd_rust_visit_enter_depot(OpenTTDVehicle *, const OpenTTDVehicleVisits *) noexcept;
void openttd_rust_visit_cancel_reservation(OpenTTDVehicle *, uint16_t, OpenTTDStation *, const OpenTTDVehicleVisits *);
}
#endif /* RUST_VEHICLE_VISITS_FFI_H */
