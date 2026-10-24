/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file auth_ffi.h Opaque authentication owners and primitive-only host operations. */
#ifndef OPENTTD_RUST_AUTH_FFI_H
#define OPENTTD_RUST_AUTH_FFI_H
#include "service_defaults.hpp"

#include <stddef.h>
#include <stdint.h>

#if defined(_MSC_VER)
#define OPENTTD_AUTH_CALL __cdecl
#elif defined(__i386__)
#define OPENTTD_AUTH_CALL __attribute__((cdecl))
#else
#define OPENTTD_AUTH_CALL
#endif

#ifdef __cplusplus
extern "C" {
#endif

/* All callbacks are nonthrowing, synchronous primitive operations. They retain
 * no input pointer and invoke no Packet, RNG, logger, allocator, or UI callback.
 * The host starts the actual trivial vendor-context lifetime in init/copy.
 * Layouts describe the host vendor types, not a mirrored Rust representation. */
struct OpenTTDAuthPrimitives {
	uint32_t version;
	uint32_t reserved;
	size_t aead_size;
	size_t aead_alignment;
	size_t hash_size;
	size_t hash_alignment;
	void (OPENTTD_AUTH_CALL *wipe)(void *, size_t) OPENTTD_SERVICE_DEFAULT(wipe);
	void (OPENTTD_AUTH_CALL *x25519)(uint8_t *, const uint8_t *, const uint8_t *) OPENTTD_SERVICE_DEFAULT(x25519);
	void (OPENTTD_AUTH_CALL *public_key)(uint8_t *, const uint8_t *) OPENTTD_SERVICE_DEFAULT(public_key);
	void (OPENTTD_AUTH_CALL *hash_init)(void *, size_t) OPENTTD_SERVICE_DEFAULT(hash_init);
	void (OPENTTD_AUTH_CALL *hash_update)(void *, const uint8_t *, size_t) OPENTTD_SERVICE_DEFAULT(hash_update);
	void (OPENTTD_AUTH_CALL *hash_final)(void *, uint8_t *) OPENTTD_SERVICE_DEFAULT(hash_final);
	void (OPENTTD_AUTH_CALL *aead_init)(void *, const uint8_t *, const uint8_t *) OPENTTD_SERVICE_DEFAULT(aead_init);
	void (OPENTTD_AUTH_CALL *aead_copy)(void *, const void *) OPENTTD_SERVICE_DEFAULT(aead_copy);
	void (OPENTTD_AUTH_CALL *aead_write)(void *, uint8_t *, uint8_t *, const uint8_t *, size_t, const uint8_t *, size_t) OPENTTD_SERVICE_DEFAULT(aead_write);
	int32_t (OPENTTD_AUTH_CALL *aead_read)(void *, uint8_t *, const uint8_t *, const uint8_t *, size_t, const uint8_t *, size_t) OPENTTD_SERVICE_DEFAULT(aead_read);
	void (OPENTTD_AUTH_CALL *lock)(uint8_t *, uint8_t *, const uint8_t *, const uint8_t *, const uint8_t *, size_t, const uint8_t *, size_t) OPENTTD_SERVICE_DEFAULT(lock);
	int32_t (OPENTTD_AUTH_CALL *unlock)(uint8_t *, const uint8_t *, const uint8_t *, const uint8_t *, const uint8_t *, size_t, const uint8_t *, size_t) OPENTTD_SERVICE_DEFAULT(unlock);
};

struct OpenTTDAuthKeys;
struct OpenTTDAuthSession;
struct OpenTTDAuthStream;

/* Owners allocate and free only in Rust. Clone makes independent storage; assignment overwrites
 * fixed state like the original member assignment. Fixed owner views keep their address until
 * destruction. Exchange extra payload may alias derived-key bytes: hashing completes before
 * replacement. Fixed buffers are 32-byte keys, 24-byte nonces, 16-byte MACs and 8-byte auth
 * messages; MAC and message regions are disjoint, and encryption is in place. */
struct OpenTTDAuthKeys *OPENTTD_AUTH_CALL openttd_rust_auth_keys_new(const struct OpenTTDAuthPrimitives *);
struct OpenTTDAuthKeys *OPENTTD_AUTH_CALL openttd_rust_auth_keys_clone(const struct OpenTTDAuthKeys *);
void OPENTTD_AUTH_CALL openttd_rust_auth_keys_assign(struct OpenTTDAuthKeys *, const struct OpenTTDAuthKeys *);
void OPENTTD_AUTH_CALL openttd_rust_auth_keys_destroy(struct OpenTTDAuthKeys *);
const uint8_t *OPENTTD_AUTH_CALL openttd_rust_auth_keys_data(const struct OpenTTDAuthKeys *, uint8_t);
uint8_t OPENTTD_AUTH_CALL openttd_rust_auth_keys_exchange(struct OpenTTDAuthKeys *, const uint8_t *, uint8_t, const uint8_t *, const uint8_t *, const uint8_t *, size_t);

struct OpenTTDAuthSession *OPENTTD_AUTH_CALL openttd_rust_auth_session_new(const struct OpenTTDAuthPrimitives *, const uint8_t *);
struct OpenTTDAuthSession *OPENTTD_AUTH_CALL openttd_rust_auth_session_clone(const struct OpenTTDAuthSession *);
void OPENTTD_AUTH_CALL openttd_rust_auth_session_assign(struct OpenTTDAuthSession *, const struct OpenTTDAuthSession *);
void OPENTTD_AUTH_CALL openttd_rust_auth_session_destroy(struct OpenTTDAuthSession *);
/* Fields: 0 our public key, 1 peer public key, 2 exchange nonce, 3 stream nonce,
 * 4 client-to-server key, 5 server-to-client key. Only fields 1/2/3 are mutable. */
const uint8_t *OPENTTD_AUTH_CALL openttd_rust_auth_session_view(const struct OpenTTDAuthSession *, uint8_t);
uint8_t *OPENTTD_AUTH_CALL openttd_rust_auth_session_mut_view(struct OpenTTDAuthSession *, uint8_t);
uint8_t OPENTTD_AUTH_CALL openttd_rust_auth_session_exchange(struct OpenTTDAuthSession *, uint8_t, const uint8_t *, size_t);
void OPENTTD_AUTH_CALL openttd_rust_auth_session_encrypt_response(const struct OpenTTDAuthSession *, uint8_t *, uint8_t *);
uint8_t OPENTTD_AUTH_CALL openttd_rust_auth_session_decrypt_response(const struct OpenTTDAuthSession *, uint8_t *, const uint8_t *);

struct OpenTTDAuthStream *OPENTTD_AUTH_CALL openttd_rust_auth_stream_new(const struct OpenTTDAuthSession *, uint8_t);
struct OpenTTDAuthStream *OPENTTD_AUTH_CALL openttd_rust_auth_stream_clone(const struct OpenTTDAuthStream *);
void OPENTTD_AUTH_CALL openttd_rust_auth_stream_assign(struct OpenTTDAuthStream *, const struct OpenTTDAuthStream *);
void OPENTTD_AUTH_CALL openttd_rust_auth_stream_destroy(struct OpenTTDAuthStream *);
void OPENTTD_AUTH_CALL openttd_rust_auth_stream_encrypt(struct OpenTTDAuthStream *, uint8_t *, uint8_t *, size_t);
uint8_t OPENTTD_AUTH_CALL openttd_rust_auth_stream_decrypt(struct OpenTTDAuthStream *, const uint8_t *, uint8_t *, size_t);

#ifdef __cplusplus
}
static_assert(sizeof(OpenTTDAuthPrimitives) == 8 + 16 * sizeof(size_t));
static_assert(offsetof(OpenTTDAuthPrimitives, wipe) == 8 + 4 * sizeof(size_t));
static_assert(sizeof(int32_t) == sizeof(int));
#endif
#endif /* OPENTTD_RUST_AUTH_FFI_H */
