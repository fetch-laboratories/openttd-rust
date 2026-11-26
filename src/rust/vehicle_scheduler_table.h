/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file vehicle_scheduler_table.h The vehicle scheduler tables C++ fills for Rust; only the C++ that fills or calls them includes this header. */
#ifndef RUST_VEHICLE_SCHEDULER_TABLE_H
#define RUST_VEHICLE_SCHEDULER_TABLE_H
#include "vehicle_ffi.h"

struct Vehicle;
struct OpenTTDCargoStorageServices;
struct OpenTTDSharedServices;
struct OpenTTDStationServices;
struct OpenTTDStationLoading;
struct OpenTTDFleetPendingServices;
struct OpenTTDFleetTransactionServices;
struct OpenTTDFleetGroupServices;
struct OpenTTDAircraftLeaves;
struct OpenTTDDisasterServices;
struct OpenTTDRoadCommandSlot;
struct OpenTTDRoadLeaves;
struct OpenTTDShipLeaves;
struct OpenTTDTrainServices;
struct OpenTTDVehicleSchedulerServices {
	const OpenTTDAircraftLeaves *aircraft; ///< Aircraft controller table, immutable for the process.
	const OpenTTDDisasterServices *disaster; ///< Disaster vehicle table, immutable for the process.
	const RustPoolDescriptor *const *pool; ///< The vehicle pool's descriptor.
	const OpenTTDRoadLeaves *road; ///< Road vehicle controller table, immutable for the process.
	const OpenTTDSharedServices *shared; ///< Shared table, immutable for the process.
	const OpenTTDShipLeaves *ship; ///< Ship controller table, immutable for the process.
	const OpenTTDTrainServices *train; ///< Train controller table, immutable for the process.
	const OpenTTDVehicleServices *vehicle; ///< Base vehicle table, immutable for the process.
};
struct OpenTTDCommandCost;
struct OpenTTDVehicleVisits;
struct OpenTTDVehicleSchedulerTables {
	const OpenTTDCargoStorageServices *cargo;
	const OpenTTDFleetGroupServices *group;
	const OpenTTDStationLoading *loading;
	const OpenTTDFleetPendingServices *pending;
	OpenTTDCommandCost *refit_cost;
	OpenTTDCommandCost *refit_temporary;
	OpenTTDRoadCommandSlot *road_slot; ///< The session frame's road command slot for the vehicle tick; null on calendar day.
	const OpenTTDVehicleSchedulerServices *scheduler;
	const OpenTTDStationServices *station;
	const OpenTTDFleetTransactionServices *transaction;
	const OpenTTDVehicleVisits *visits;
};
#endif /* RUST_VEHICLE_SCHEDULER_TABLE_H */
