/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file vehicle_record_ffi.h Lifetime and chain operations of the shared vehicle record. */
#ifndef RUST_VEHICLE_RECORD_FFI_H
#define RUST_VEHICLE_RECORD_FFI_H
#include "call_ffi.h"
#include "abi-record-vehicle.h"
extern "C" {
/* One record per shell, created before the shell's members and released after
 * its whole base destructor; first points to the record itself. */
OpenTTDVehicle *OPENTTD_RUST_CALL openttd_rust_vehicle_record_new(OpenTTDVehicleShell *, uint8_t type);
void OPENTTD_RUST_CALL openttd_rust_vehicle_record_delete(OpenTTDVehicle *);
/* Vehicle::SetNext after its self-link assertion. */
void OPENTTD_RUST_CALL openttd_rust_vehicle_set_next(OpenTTDVehicle *, OpenTTDVehicle *);
}
#endif /* RUST_VEHICLE_RECORD_FFI_H */
