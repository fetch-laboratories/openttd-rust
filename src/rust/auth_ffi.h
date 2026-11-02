/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file auth_ffi.h Opaque authentication owners over Rust cryptographic primitives. */
#ifndef OPENTTD_RUST_AUTH_FFI_H
#define OPENTTD_RUST_AUTH_FFI_H
#include "service_defaults.hpp"

#include <stddef.h>
#include <stdint.h>

#include "call_ffi.h"

#ifdef __cplusplus
extern "C" {
#endif

struct OpenTTDAuthKeys;
struct OpenTTDAuthSession;
struct OpenTTDAuthStream;

/* Owners allocate and free only in Rust. Clone makes independent storage; assignment overwrites
 * fixed state like the original member assignment. Fixed owner views keep their address until
 * destruction. Exchange extra payload may alias derived-key bytes: hashing completes before
 * replacement. Fixed buffers are 32-byte keys, 24-byte nonces, 16-byte MACs and 8-byte auth
 * messages; MAC and message regions are disjoint, and encryption is in place.
 * Calls are serialized; live initialized buffers have lengths <= PTRDIFF_MAX (NULL only if empty),
 * output excludes owner storage, and owner views cannot be accessed during mutation. */
struct OpenTTDAuthKeys *OPENTTD_RUST_CALL openttd_rust_auth_keys_new();
struct OpenTTDAuthKeys *OPENTTD_RUST_CALL openttd_rust_auth_keys_clone(const struct OpenTTDAuthKeys *);
void OPENTTD_RUST_CALL openttd_rust_auth_keys_assign(struct OpenTTDAuthKeys *, const struct OpenTTDAuthKeys *);
void OPENTTD_RUST_CALL openttd_rust_auth_keys_destroy(struct OpenTTDAuthKeys *);
const uint8_t *OPENTTD_RUST_CALL openttd_rust_auth_keys_data(const struct OpenTTDAuthKeys *, uint8_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_auth_keys_exchange(struct OpenTTDAuthKeys *, const uint8_t *, uint8_t, const uint8_t *, const uint8_t *, const uint8_t *, size_t);

struct OpenTTDAuthSession *OPENTTD_RUST_CALL openttd_rust_auth_session_new(const uint8_t *);
struct OpenTTDAuthSession *OPENTTD_RUST_CALL openttd_rust_auth_session_clone(const struct OpenTTDAuthSession *);
void OPENTTD_RUST_CALL openttd_rust_auth_session_assign(struct OpenTTDAuthSession *, const struct OpenTTDAuthSession *);
void OPENTTD_RUST_CALL openttd_rust_auth_session_destroy(struct OpenTTDAuthSession *);
/* Fields: 0 our public key, 1 peer public key, 2 exchange nonce, 3 stream nonce,
 * 4 client-to-server key, 5 server-to-client key. Only fields 1/2/3 are mutable. */
const uint8_t *OPENTTD_RUST_CALL openttd_rust_auth_session_view(const struct OpenTTDAuthSession *, uint8_t);
uint8_t *OPENTTD_RUST_CALL openttd_rust_auth_session_mut_view(struct OpenTTDAuthSession *, uint8_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_auth_session_exchange(struct OpenTTDAuthSession *, uint8_t, const uint8_t *, size_t);
void OPENTTD_RUST_CALL openttd_rust_auth_session_encrypt_response(const struct OpenTTDAuthSession *, uint8_t *, uint8_t *);
uint8_t OPENTTD_RUST_CALL openttd_rust_auth_session_decrypt_response(const struct OpenTTDAuthSession *, uint8_t *, const uint8_t *);

struct OpenTTDAuthStream *OPENTTD_RUST_CALL openttd_rust_auth_stream_new(const struct OpenTTDAuthSession *, uint8_t);
struct OpenTTDAuthStream *OPENTTD_RUST_CALL openttd_rust_auth_stream_clone(const struct OpenTTDAuthStream *);
void OPENTTD_RUST_CALL openttd_rust_auth_stream_assign(struct OpenTTDAuthStream *, const struct OpenTTDAuthStream *);
void OPENTTD_RUST_CALL openttd_rust_auth_stream_destroy(struct OpenTTDAuthStream *);
void OPENTTD_RUST_CALL openttd_rust_auth_stream_encrypt(struct OpenTTDAuthStream *, uint8_t *, uint8_t *, size_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_auth_stream_decrypt(struct OpenTTDAuthStream *, const uint8_t *, uint8_t *, size_t);

#ifdef __cplusplus
}
#endif
#endif /* OPENTTD_RUST_AUTH_FFI_H */
