/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file packet_ffi.h Rust-owned packet storage and byte operations. */
#ifndef OPENTTD_RUST_PACKET_FFI_H
#define OPENTTD_RUST_PACKET_FFI_H
#include <stddef.h>
#include <stdint.h>
#include "builder_ffi.h"
#include "call_ffi.h"
#include "abi-record-packet.h"
#ifdef __cplusplus
extern "C" {
#endif
typedef struct {
	size_t message;
	size_t payload;
} OpenTTDPacketFrame;
void OPENTTD_RUST_CALL openttd_rust_packet_resize(OpenTTDPacketState *, size_t);
void OPENTTD_RUST_CALL openttd_rust_packet_append(OpenTTDPacketState *, const uint8_t *, size_t);
void OPENTTD_RUST_CALL openttd_rust_packet_shrink(OpenTTDPacketState *);
void OPENTTD_RUST_CALL openttd_rust_packet_clone(OpenTTDPacketState *, const OpenTTDPacketState *);
void OPENTTD_RUST_CALL openttd_rust_packet_assign(OpenTTDPacketState *, const OpenTTDPacketState *);
void OPENTTD_RUST_CALL openttd_rust_packet_destroy(OpenTTDPacketState *);
/* Rust allocates and releases the byte buffer. Native spans expire before resizing or destruction.
 * State and byte storage are disjoint, and no borrow survives a handler or transfer call.
 * The persistent cursor narrows to uint16 after each consumption and positive transfer result.
 * Pointers are live and aligned, exclusive for writes; bytes occupy one allocation <= PTRDIFF_MAX. */
void OPENTTD_RUST_CALL openttd_rust_packet_init(OpenTTDPacketState *, size_t limit);
uint8_t OPENTTD_RUST_CALL openttd_rust_packet_boolean(uint8_t value);
uint8_t OPENTTD_RUST_CALL openttd_rust_packet_can_write(const OpenTTDPacketState *, size_t size, size_t amount);
uint8_t OPENTTD_RUST_CALL openttd_rust_packet_can_read(const OpenTTDPacketState *, size_t size, size_t amount);
uint8_t OPENTTD_RUST_CALL openttd_rust_packet_has_size(const OpenTTDPacketState *);
size_t OPENTTD_RUST_CALL openttd_rust_packet_remaining(const OpenTTDPacketState *, size_t size);
size_t OPENTTD_RUST_CALL openttd_rust_packet_transfer_amount(const OpenTTDPacketState *, size_t size, size_t limit);
void OPENTTD_RUST_CALL openttd_rust_packet_transfer_commit(OpenTTDPacketState *, intptr_t result);
size_t OPENTTD_RUST_CALL openttd_rust_packet_send_amount(const OpenTTDPacketState *, size_t size, size_t input);
size_t OPENTTD_RUST_CALL openttd_rust_packet_buffer_size(size_t length);
uint16_t OPENTTD_RUST_CALL openttd_rust_packet_prefix(size_t length);
uint64_t OPENTTD_RUST_CALL openttd_rust_packet_recv(OpenTTDPacketState *, const uint8_t *, size_t length, uint8_t width);
uint8_t OPENTTD_RUST_CALL openttd_rust_packet_buffer_next(OpenTTDPacketState *, const uint8_t *, size_t length, uint16_t *remaining, uint8_t *output);
uint16_t OPENTTD_RUST_CALL openttd_rust_packet_parse_size(const OpenTTDPacketState *, const uint8_t *, size_t length);
void OPENTTD_RUST_CALL openttd_rust_packet_read_start(OpenTTDPacketState *);
void OPENTTD_RUST_CALL openttd_rust_packet_send_reset(OpenTTDPacketState *);
uint8_t OPENTTD_RUST_CALL openttd_rust_packet_frame(uint16_t position, size_t size, size_t mac, OpenTTDPacketFrame *);
void OPENTTD_RUST_CALL openttd_rust_packet_skip_mac(OpenTTDPacketState *, size_t mac);
void OPENTTD_RUST_CALL openttd_rust_packet_write_header(uint8_t *, size_t length);
#ifdef __cplusplus
}
#endif
#endif /* OPENTTD_RUST_PACKET_FFI_H */
