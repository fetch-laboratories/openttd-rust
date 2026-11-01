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
#include "call_ffi.h"
#include "service_defaults.hpp"
#include "abi-record-packet.h"
struct NetworkEncryptionHandler;
struct OpenTTDAuthStream;
struct OpenTTDPacketServices {
	bool (*decrypt)(NetworkEncryptionHandler *, uint8_t *, size_t, uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(decrypt);
	void (*encrypt)(NetworkEncryptionHandler *, uint8_t *, size_t, uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(encrypt);
	bool (*has_quit)(const NetworkSocketHandler *) noexcept OPENTTD_SERVICE_DEFAULT(has_quit);
	size_t (*mac_size)(const NetworkEncryptionHandler *) noexcept OPENTTD_SERVICE_DEFAULT(mac_size);
	void (*mark_closed)(NetworkSocketHandler *) noexcept OPENTTD_SERVICE_DEFAULT(mark_closed);
	NetworkEncryptionHandler *(*receive_handler)(const NetworkSocketHandler *) noexcept OPENTTD_SERVICE_DEFAULT(receive_handler);
	NetworkEncryptionHandler *(*send_handler)(const NetworkSocketHandler *) noexcept OPENTTD_SERVICE_DEFAULT(send_handler);
	OpenTTDAuthStream *(*stream)(const NetworkEncryptionHandler *) noexcept OPENTTD_SERVICE_DEFAULT(stream);
};
/* Rust owns the allocation. State/byte spans must expire before growth or a callback.
 * Socket and encryption services are synchronous and nonthrowing. Native-only throwing virtual
 * handlers stay on the caller's C++ frame. Receive bytes retains #41's short-source precondition. */
extern "C" {
uint8_t OPENTTD_RUST_CALL openttd_rust_packet_get_type(const OpenTTDPacketState *, const OpenTTDPacketServices *);
void OPENTTD_RUST_CALL openttd_rust_packet_init_outgoing(OpenTTDPacketState *, NetworkSocketHandler *, const OpenTTDPacketServices *, uint8_t, size_t);
void OPENTTD_RUST_CALL openttd_rust_packet_init_receive(OpenTTDPacketState *, NetworkSocketHandler *, size_t, size_t);
void OPENTTD_RUST_CALL openttd_rust_packet_init_send(OpenTTDPacketState *, NetworkSocketHandler *, uint8_t, size_t, size_t);
void OPENTTD_RUST_CALL openttd_rust_packet_clone(OpenTTDPacketState *, const OpenTTDPacketState *);
void OPENTTD_RUST_CALL openttd_rust_packet_assign(OpenTTDPacketState *, const OpenTTDPacketState *);
void OPENTTD_RUST_CALL openttd_rust_packet_destroy(OpenTTDPacketState *);
bool OPENTTD_RUST_CALL openttd_rust_packet_can_write(const OpenTTDPacketState *, size_t);
bool OPENTTD_RUST_CALL openttd_rust_packet_can_read(OpenTTDPacketState *, const OpenTTDPacketServices *, size_t, bool);
bool OPENTTD_RUST_CALL openttd_rust_packet_parse_size(OpenTTDPacketState *);
void OPENTTD_RUST_CALL openttd_rust_packet_begin_send(OpenTTDPacketState *);
void OPENTTD_RUST_CALL openttd_rust_packet_finish_send(OpenTTDPacketState *);
void OPENTTD_RUST_CALL openttd_rust_packet_begin_read(OpenTTDPacketState *);
void OPENTTD_RUST_CALL openttd_rust_packet_finish_read(OpenTTDPacketState *, size_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_packet_packet_type(const OpenTTDPacketState *, size_t);
size_t OPENTTD_RUST_CALL openttd_rust_packet_transfer_amount(const OpenTTDPacketState *, size_t);
void OPENTTD_RUST_CALL openttd_rust_packet_transfer_commit(OpenTTDPacketState *, intptr_t);
bool OPENTTD_RUST_CALL openttd_rust_packet_frame(uint16_t, size_t, size_t, OpenTTDPacketFrame *);
void OPENTTD_RUST_CALL openttd_rust_packet_prepare_to_send(OpenTTDPacketState *, const OpenTTDPacketServices *);
bool OPENTTD_RUST_CALL openttd_rust_packet_prepare_to_read(OpenTTDPacketState *, const OpenTTDPacketServices *);
size_t OPENTTD_RUST_CALL openttd_rust_packet_recv_buffer_size(OpenTTDPacketState *, const OpenTTDPacketServices *);
void OPENTTD_RUST_CALL openttd_rust_packet_recv_buffer_data(OpenTTDPacketState *, uint8_t *, size_t);
size_t OPENTTD_RUST_CALL openttd_rust_packet_recv_bytes(OpenTTDPacketState *, uint8_t *, size_t);
void OPENTTD_RUST_CALL openttd_rust_packet_send_string(OpenTTDPacketState *, const uint8_t *, size_t);
void OPENTTD_RUST_CALL openttd_rust_packet_send_buffer(OpenTTDPacketState *, const uint8_t *, size_t);
size_t OPENTTD_RUST_CALL openttd_rust_packet_send_bytes(OpenTTDPacketState *, const uint8_t *, size_t);
void OPENTTD_RUST_CALL openttd_rust_packet_recv_string(OpenTTDPacketState *, const OpenTTDPacketServices *, size_t, uint8_t, void *, void (*)(void *, const uint8_t *, size_t) noexcept);
void OPENTTD_RUST_CALL openttd_rust_packet_send_bool(OpenTTDPacketState *, bool);
bool OPENTTD_RUST_CALL openttd_rust_packet_recv_bool(OpenTTDPacketState *, const OpenTTDPacketServices *);
void OPENTTD_RUST_CALL openttd_rust_packet_send_uint8(OpenTTDPacketState *, uint8_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_packet_recv_uint8(OpenTTDPacketState *, const OpenTTDPacketServices *);
void OPENTTD_RUST_CALL openttd_rust_packet_send_uint16(OpenTTDPacketState *, uint16_t);
uint16_t OPENTTD_RUST_CALL openttd_rust_packet_recv_uint16(OpenTTDPacketState *, const OpenTTDPacketServices *);
void OPENTTD_RUST_CALL openttd_rust_packet_send_uint32(OpenTTDPacketState *, uint32_t);
uint32_t OPENTTD_RUST_CALL openttd_rust_packet_recv_uint32(OpenTTDPacketState *, const OpenTTDPacketServices *);
void OPENTTD_RUST_CALL openttd_rust_packet_send_uint64(OpenTTDPacketState *, uint64_t);
uint64_t OPENTTD_RUST_CALL openttd_rust_packet_recv_uint64(OpenTTDPacketState *, const OpenTTDPacketServices *);
}
#endif /* OPENTTD_RUST_PACKET_FFI_H */
