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
/** Front fields read by one link prediction. */
struct OpenTTDVisitPrediction {
	uint16_t last_loading;
	uint16_t last_station;
	uint16_t ticks;
};
struct OpenTTDVehicleVisits {
	const OpenTTDSharedServices *shared;
	const OpenTTDStationServices *station;
	const OpenTTDStationLinks *links;
	const OpenTTDStationLoading *loading;
	const OpenTTDVehicleRefitServices *refit;
	OpenTTDVisitCapacity (*capacity)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(capacity);
	OpenTTDVisitPrediction (*prediction)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(prediction);
	int32_t (*display_speed)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(display_speed);
	uint32_t (*location)(const OpenTTDBaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(location);
	bool (*graph_available)() noexcept OPENTTD_SERVICE_DEFAULT(graph_available);
	OpenTTDLinkGraph *(*allocate_graph)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(allocate_graph);
	void (*allocation_failure)() noexcept OPENTTD_SERVICE_DEFAULT(allocation_failure);
	void (*join_graph)(OpenTTDLinkGraph *, OpenTTDBaseStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(join_graph);
	uint16_t (*graph_size)(OpenTTDLinkGraph *) noexcept OPENTTD_SERVICE_DEFAULT(graph_size);
	void (*merge_graph)(OpenTTDLinkGraph *, OpenTTDLinkGraph *) noexcept OPENTTD_SERVICE_DEFAULT(merge_graph);
	void (*update_edge)(OpenTTDLinkGraph *, uint16_t, uint16_t, uint32_t, uint32_t, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(update_edge);
};
const OpenTTDVehicleVisits &GetRustVehicleVisits() noexcept;
extern "C" {
void openttd_rust_refresh_links(OpenTTDVehicle *, bool, bool, const OpenTTDVehicleVisits *);
void openttd_rust_increase_vehicle_stats(OpenTTDBaseStation *, OpenTTDVehicle *, uint16_t, uint32_t, const OpenTTDVehicleVisits *);
}
#endif /* RUST_VEHICLE_VISITS_FFI_H */
