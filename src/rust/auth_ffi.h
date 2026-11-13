/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file auth_ffi.h Opaque Rust authentication handlers and encryption streams. */
#ifndef OPENTTD_RUST_AUTH_FFI_H
#define OPENTTD_RUST_AUTH_FFI_H
#include "call_ffi.h"
#include "service_defaults.hpp"
#include <stddef.h>
#include <stdint.h>

class NetworkAuthenticationPasswordProvider;
class NetworkAuthenticationAuthorizedKeyHandler;
class RustAuthenticationClientHandler;
struct OpenTTDPacketServices;
struct OpenTTDPacketState;
struct OpenTTDAuthHandler;
struct OpenTTDAuthStream;

struct OpenTTDAuthHandlerServices {
	uint8_t (*allowed)(const NetworkAuthenticationAuthorizedKeyHandler *, const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(allowed);
	void (*ask_password)(RustAuthenticationClientHandler *) noexcept OPENTTD_SERVICE_DEFAULT(ask_password);
	uint8_t (*authorized_available)(const NetworkAuthenticationAuthorizedKeyHandler *) noexcept OPENTTD_SERVICE_DEFAULT(authorized_available);
	const uint8_t *(*client_password)(const RustAuthenticationClientHandler *, size_t *) noexcept OPENTTD_SERVICE_DEFAULT(client_password);
	const OpenTTDPacketServices *packet;
	void (*random_bytes)(uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(random_bytes);
	const uint8_t *(*server_password)(const NetworkAuthenticationPasswordProvider *, size_t *) noexcept OPENTTD_SERVICE_DEFAULT(server_password);
};
const OpenTTDAuthHandlerServices &GetAuthHandlerServices();

/* Calls are serialized. Services, native providers and password callback objects
 * remain live until handler destruction. Password requests may synchronously
 * reenter. Byte spans are initialized, null only if empty, and do not overlap an
 * owner. Packet::Recv_bytes keeps its original nonempty-short precondition.
 * Owners and streams allocate/free in Rust. Native success values are uint8_t. */
extern "C" {
OpenTTDAuthHandler *OPENTTD_RUST_CALL openttd_rust_auth_client_new(const OpenTTDAuthHandlerServices *, const uint8_t *, RustAuthenticationClientHandler *);
OpenTTDAuthHandler *OPENTTD_RUST_CALL openttd_rust_auth_server_new(const OpenTTDAuthHandlerServices *, const NetworkAuthenticationPasswordProvider *, const NetworkAuthenticationAuthorizedKeyHandler *, uint16_t);
void OPENTTD_RUST_CALL openttd_rust_auth_handler_destroy(OpenTTDAuthHandler *);
const uint8_t *OPENTTD_RUST_CALL openttd_rust_auth_handler_name(const OpenTTDAuthHandler *, size_t *);
const uint8_t *OPENTTD_RUST_CALL openttd_rust_auth_handler_peer(const OpenTTDAuthHandler *);
uint8_t OPENTTD_RUST_CALL openttd_rust_auth_handler_can_be_used(const OpenTTDAuthHandler *);
void OPENTTD_RUST_CALL openttd_rust_auth_handler_send_request(OpenTTDAuthHandler *, OpenTTDPacketState *, void (*)(bool) noexcept);
uint8_t OPENTTD_RUST_CALL openttd_rust_auth_handler_receive_request(OpenTTDAuthHandler *, OpenTTDPacketState *);
uint8_t OPENTTD_RUST_CALL openttd_rust_auth_handler_send_response(OpenTTDAuthHandler *, OpenTTDPacketState *);
uint8_t OPENTTD_RUST_CALL openttd_rust_auth_handler_receive_response(OpenTTDAuthHandler *, OpenTTDPacketState *);
void OPENTTD_RUST_CALL openttd_rust_auth_handler_send_enable(OpenTTDAuthHandler *, OpenTTDPacketState *);
uint8_t OPENTTD_RUST_CALL openttd_rust_auth_handler_receive_enable(OpenTTDAuthHandler *, OpenTTDPacketState *);
OpenTTDAuthStream *OPENTTD_RUST_CALL openttd_rust_auth_stream_new(const OpenTTDAuthHandler *, uint8_t);
void OPENTTD_RUST_CALL openttd_rust_auth_stream_destroy(OpenTTDAuthStream *);
void OPENTTD_RUST_CALL openttd_rust_auth_stream_encrypt(OpenTTDAuthStream *, uint8_t *, uint8_t *, size_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_auth_stream_decrypt(OpenTTDAuthStream *, const uint8_t *, uint8_t *, size_t);
}
#endif /* OPENTTD_RUST_AUTH_FFI_H */
