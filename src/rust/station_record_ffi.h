/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file station_record_ffi.h Lifetime of the shared station and road-stop records. */
#ifndef RUST_STATION_RECORD_FFI_H
#define RUST_STATION_RECORD_FFI_H
#include "call_ffi.h"
#include "abi-record-station.h"
extern "C" {
/* One record per BaseStation shell, created before the shell's members and
 * released after its whole base destructor. The record is the first member of
 * the canonical Rust station owner, so it also identifies that owner. */
OpenTTDStationRecord *OPENTTD_RUST_CALL openttd_rust_station_record_new(BaseStation *shell, uint32_t xy);
void OPENTTD_RUST_CALL openttd_rust_station_record_delete(OpenTTDStationRecord *);
}
#endif /* RUST_STATION_RECORD_FFI_H */
