/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file type_definitions_ffi.h The rail and road type Action 0 handlers. Each takes the
 * loader's reader table, its current file and its reader, and returns the ChangeInfoResult,
 * or a negative value for a read error the caller rethrows. */
#ifndef RUST_TYPE_DEFINITIONS_FFI_H
#define RUST_TYPE_DEFINITIONS_FFI_H
#include "type_registry_ffi.h"
#include "newgrf_bytereader_ffi.h"
extern "C" int32_t openttd_rust_rail_change(const OpenTTDByteReaderServices *, GRFFile *, uint32_t, uint32_t, int32_t, ByteReader *);
extern "C" int32_t openttd_rust_rail_reserve(const OpenTTDByteReaderServices *, GRFFile *, uint32_t, uint32_t, int32_t, ByteReader *);
extern "C" int32_t openttd_rust_road_change(const OpenTTDByteReaderServices *, GRFFile *, uint32_t, uint32_t, int32_t, ByteReader *, RoadTramType);
extern "C" int32_t openttd_rust_road_reserve(const OpenTTDByteReaderServices *, GRFFile *, uint32_t, uint32_t, int32_t, ByteReader *, RoadTramType);
#endif
