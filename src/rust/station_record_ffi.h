/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file station_record_ffi.h Lifetime of the shared station and road-stop records. */
#ifndef RUST_STATION_RECORD_FFI_H
#define RUST_STATION_RECORD_FFI_H
#include "call_ffi.h"
#include "rust_vector.h"
#include "pool_ffi.h"
#include "abi-record-station.h"
extern "C" {
/* One record per BaseStation shell, created before the shell's members and
 * released after its whole base destructor. The record is the first member of
 * the canonical Rust station owner, so it also identifies that owner. A real
 * shell also publishes the offset of its record pointer; fixtures pass null. */
OpenTTDStationRecord *OPENTTD_RUST_CALL openttd_rust_station_record_new(BaseStation *shell, uint32_t xy, size_t record_offset);
void OPENTTD_RUST_CALL openttd_rust_station_record_delete(OpenTTDStationRecord *);
/* Optional attachment publication and destruction are Rust-owned. Clear nulls
 * the goods link before callbacks; borrowed native views end before clear. */
OpenTTDStationGoodsData *openttd_rust_station_goods_data_ensure(OpenTTDGoodsEntry *);
uint8_t openttd_rust_station_goods_data_is_empty(const OpenTTDStationGoodsData *);
void openttd_rust_station_goods_data_clear(OpenTTDGoodsEntry *, const struct OpenTTDCargoStorageServices *);
uint8_t openttd_rust_station_roadstop_data_set(OpenTTDStationRecord *, uint32_t tile, uint8_t value, uint8_t animation);
void openttd_rust_station_roadstop_data_remove(OpenTTDStationRecord *, uint32_t tile);
void openttd_rust_station_roadstop_data_clear(OpenTTDStationRecord *);
void openttd_rust_station_roadstop_data_reserve(OpenTTDStationRecord *, size_t count);
OpenTTDRoadStopTileData *openttd_rust_station_roadstop_data_append(OpenTTDStationRecord *);
void openttd_rust_station_specs_reserve(OpenTTDStationRecord *, uint8_t road, size_t count);
void *openttd_rust_station_specs_append(OpenTTDStationRecord *, uint8_t road);
void openttd_rust_station_name_set(OpenTTDStationRecord *, uint8_t cached, const uint8_t *text, size_t length);
}
#endif /* RUST_STATION_RECORD_FFI_H */
