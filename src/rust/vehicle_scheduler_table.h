/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
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
