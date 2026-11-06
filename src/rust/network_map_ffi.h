/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_map_ffi.h Map payload ownership inside native save/load resource frames. */
#ifndef RUST_NETWORK_MAP_FFI_H
#define RUST_NETWORK_MAP_FFI_H
#include "packet_ffi.h"
#include "rust_vector.h"
#include "abi-record-network_map.h"
extern "C" {
OpenTTDNetworkMapWriter *openttd_rust_network_map_writer_new(void *, NetworkSocketHandler *);
void openttd_rust_network_map_writer_drop(OpenTTDNetworkMapWriter *);
void openttd_rust_network_map_write(OpenTTDNetworkMapWriter *, const OpenTTDPacketServices *, void (*)(bool) noexcept, const uint8_t *, size_t);
void openttd_rust_network_map_finish(OpenTTDNetworkMapWriter *, const OpenTTDPacketServices *, void (*)(bool) noexcept);
OpenTTDNetworkMapReader *openttd_rust_network_map_reader_new();
void openttd_rust_network_map_reader_drop(OpenTTDNetworkMapReader *);
size_t openttd_rust_network_map_read(OpenTTDNetworkMapReader *, uint8_t *, size_t);
}
#endif /* RUST_NETWORK_MAP_FFI_H */
