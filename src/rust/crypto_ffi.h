/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file crypto_ffi.h Rust cryptographic operations used by native facades. */
#ifndef OPENTTD_RUST_CRYPTO_FFI_H
#define OPENTTD_RUST_CRYPTO_FFI_H
#include "call_ffi.h"
#include <stddef.h>
#include <stdint.h>

/* Serialized synchronous calls on initialized byte spans, null only if empty.
 * Fixed inputs are 32-byte keys/random bytes, 64-byte signatures and 16-byte UID
 * or 32-byte file digests. Outputs exclude input storage. Callbacks cannot throw. */
extern "C" {
void OPENTTD_RUST_CALL openttd_rust_crypto_public_key(uint8_t *, const uint8_t *);
void OPENTTD_RUST_CALL openttd_rust_crypto_wipe(uint8_t *, size_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_crypto_check(const uint8_t *, const uint8_t *, const uint8_t *, size_t);
void OPENTTD_RUST_CALL openttd_rust_crypto_hash_uid(uint8_t *, const uint8_t *, const uint8_t *, size_t);
void OPENTTD_RUST_CALL openttd_rust_crypto_hash_file(uint8_t *, void *, uint8_t (*)(void *) noexcept, size_t (*)(void *, uint8_t *, size_t) noexcept);
}
#endif /* OPENTTD_RUST_CRYPTO_FFI_H */
