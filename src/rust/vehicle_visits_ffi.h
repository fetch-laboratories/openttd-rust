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

/** One consist part's cargo_cap, refit_cap and cargo_type. */
struct OpenTTDVisitCapacity {
	uint16_t capacity;
	uint16_t remaining;
	uint8_t cargo;
};
/** Front loading fields read by one link prediction or arrival. */
struct OpenTTDVisitPrediction {
	uint64_t loading_tick;
	uint16_t last_loading;
	uint16_t last_station;
	uint16_t ticks;
};
/** The loading indicator's colour strings. */
struct OpenTTDVisitPercentStrings {
	uint32_t up;
	uint32_t down;
	uint32_t up_down;
	uint32_t none;
};
struct OpenTTDCommandCost;
struct OpenTTDCargoList;
/** Two native CommandCost stack slots for one nested depot refit. */
struct OpenTTDVisitCosts {
	OpenTTDCommandCost *out;
	OpenTTDCommandCost *temporary;
};
struct OpenTTDScheduleServices;
struct OpenTTDVehicleVisits {
	const OpenTTDSharedServices *shared;
	const OpenTTDStationServices *station;
	const OpenTTDStationLinks *links;
	const OpenTTDStationLoading *loading;
	const OpenTTDVehicleRefitServices *refit;
	const OpenTTDScheduleServices *schedule;
	OpenTTDVisitCapacity (*capacity)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(capacity);
	OpenTTDVisitPrediction (*prediction)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(prediction);
	int32_t (*display_speed)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(display_speed);
	uint32_t (*location)(const OpenTTDBaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(location);
	OpenTTDVisitPercentStrings percent;
	void (*set_last_loading)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_last_loading);
	void (*set_loading_tick)(OpenTTDVehicle *, uint64_t) noexcept OPENTTD_SERVICE_DEFAULT(set_loading_tick);
	void (*arrival_dirty)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(arrival_dirty);
	OpenTTDCargoList *(*station_cargo)(OpenTTDStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(station_cargo);
	void (*reservation_log)() noexcept OPENTTD_SERVICE_DEFAULT(reservation_log);
	void (*hide_filling)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(hide_filling);
	void (*set_occupancy)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_occupancy);

	void (*leave_sound)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(leave_sound);
	void (*depot_windows)(OpenTTDVehicle *, bool) noexcept OPENTTD_SERVICE_DEFAULT(depot_windows);
	void (*ship_depot_viewport)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(ship_depot_viewport);
	void (*view_invalidate)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(view_invalidate);
	void (*with_costs)(void *, void (*)(void *, void *)) noexcept OPENTTD_SERVICE_DEFAULT(with_costs);
	void (*refit_failed_news)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(refit_failed_news);
	void (*cost_animation)(OpenTTDVehicle *, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(cost_animation);
	void (*waiting_news)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(waiting_news);
};
const OpenTTDVehicleVisits &GetRustVehicleVisits() noexcept;
inline const OpenTTDVehicleVisits *RustVehicleVisits() noexcept { return &GetRustVehicleVisits(); }
extern "C" {
uint8_t openttd_rust_percent_filled(OpenTTDVehicle *, uint32_t *, const OpenTTDVehicleVisits *);
void openttd_rust_visit_leave_station(OpenTTDVehicle *, const OpenTTDVehicleVisits *);
void openttd_rust_visit_handle_loading(OpenTTDVehicle *, bool, const OpenTTDVehicleVisits *) noexcept;
void openttd_rust_visit_enter_depot(OpenTTDVehicle *, const OpenTTDVehicleVisits *) noexcept;
void openttd_rust_visit_cancel_reservation(OpenTTDVehicle *, uint16_t, OpenTTDStation *, const OpenTTDVehicleVisits *);
}
#endif /* RUST_VEHICLE_VISITS_FFI_H */
