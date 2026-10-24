// Monocypher version 4.0.2
//
// This file is dual-licensed.  Choose whichever licence you want from
// the two licences listed below.
//
// The first licence is a regular 2-clause BSD licence.  The second licence
// is the CC-0 from Creative Commons. It is intended to release Monocypher
// to the public domain.  The BSD licence serves as a fallback option.
//
// SPDX-License-Identifier: BSD-2-Clause OR CC0-1.0
//
// ------------------------------------------------------------------------
//
// Copyright (c) 2017-2020, Loup Vaillant
// All rights reserved.
//
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are
// met:
//
// 1. Redistributions of source code must retain the above copyright
//    notice, this list of conditions and the following disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright
//    notice, this list of conditions and the following disclaimer in the
//    documentation and/or other materials provided with the
//    distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
// "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
// LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
// A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
// HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
// LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
// DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
// THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
// (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
//
// ------------------------------------------------------------------------
//
// Written in 2017-2020 by Loup Vaillant
//
// To the extent possible under law, the author(s) have dedicated all copyright
// and related neighboring rights to this software to the public domain
// worldwide.  This software is distributed without any warranty.
//
// You should have received a copy of the CC0 Public Domain Dedication along
// with this software.  If not, see
// <https://creativecommons.org/publicdomain/zero/1.0/>

/** @file crypto_primitives_ffi.h Borrowed cipher/MAC contexts and primitive-only leaves. */
#ifndef OPENTTD_CRYPTO_PRIMITIVES_FFI_H
#define OPENTTD_CRYPTO_PRIMITIVES_FFI_H
#include "service_defaults.hpp"
#include <stddef.h>
#include <stdint.h>
#if defined(_MSC_VER)
#define OPENTTD_CRYPTO_CALL __cdecl
#elif defined(__i386__)
#define OPENTTD_CRYPTO_CALL __attribute__((cdecl))
#else
#define OPENTTD_CRYPTO_CALL
#endif
#ifdef __cplusplus
extern "C" {
#endif
struct OpenTTDCryptoLeaves {
	void (OPENTTD_CRYPTO_CALL *wipe)(void *, size_t) OPENTTD_SERVICE_DEFAULT(wipe);
	int32_t (OPENTTD_CRYPTO_CALL *verify16)(const uint8_t *, const uint8_t *) OPENTTD_SERVICE_DEFAULT(verify16);
};
struct OpenTTDPolyLayout {
	size_t size, alignment, c, c_idx, r, pad, h;
};
struct OpenTTDAeadLayout {
	size_t size, alignment, counter, key, nonce;
};
/* Leaves are the original nonthrowing wipe and constant-time compare. Context lifetime starts in
 * C++, which supplies its actual size and offsets; only initialized fields are read, and final
 * wipes cover the whole caller context. Text input and output are disjoint or identical, and key
 * and nonce loads precede cipher writes, including Elligator's overlapping key. Empty spans may be
 * NULL; raw cipher plaintext may also be NULL for a nonempty keystream.
 * Tables and layouts are live, aligned, initialized and immutable; calls are serialized and spans
 * have their original readable/writable extents <= PTRDIFF_MAX and vendor context/output alias rules.
 * Init preserves unwritten chunk and padding bytes; compiler spills need not be erased. */
void OPENTTD_CRYPTO_CALL openttd_rust_chacha_h(const struct OpenTTDCryptoLeaves *, uint8_t *, const uint8_t *, const uint8_t *);
uint64_t OPENTTD_CRYPTO_CALL openttd_rust_chacha_djb(const struct OpenTTDCryptoLeaves *, uint8_t *, const uint8_t *, size_t, const uint8_t *, const uint8_t *, uint64_t);
uint32_t OPENTTD_CRYPTO_CALL openttd_rust_chacha_ietf(const struct OpenTTDCryptoLeaves *, uint8_t *, const uint8_t *, size_t, const uint8_t *, const uint8_t *, uint32_t);
uint64_t OPENTTD_CRYPTO_CALL openttd_rust_chacha_x(const struct OpenTTDCryptoLeaves *, uint8_t *, const uint8_t *, size_t, const uint8_t *, const uint8_t *, uint64_t);
void OPENTTD_CRYPTO_CALL openttd_rust_poly_init(const struct OpenTTDPolyLayout *, void *, const uint8_t *);
void OPENTTD_CRYPTO_CALL openttd_rust_poly_update(const struct OpenTTDPolyLayout *, void *, const uint8_t *, size_t);
void OPENTTD_CRYPTO_CALL openttd_rust_poly_final(const struct OpenTTDCryptoLeaves *, const struct OpenTTDPolyLayout *, void *, uint8_t *);
void OPENTTD_CRYPTO_CALL openttd_rust_poly(const struct OpenTTDCryptoLeaves *, const struct OpenTTDPolyLayout *, void *, uint8_t *, const uint8_t *, size_t, const uint8_t *);
void OPENTTD_CRYPTO_CALL openttd_rust_aead_init_x(const struct OpenTTDCryptoLeaves *, const struct OpenTTDAeadLayout *, void *, const uint8_t *, const uint8_t *);
void OPENTTD_CRYPTO_CALL openttd_rust_aead_init_djb(const struct OpenTTDAeadLayout *, void *, const uint8_t *, const uint8_t *);
void OPENTTD_CRYPTO_CALL openttd_rust_aead_init_ietf(const struct OpenTTDAeadLayout *, void *, const uint8_t *, const uint8_t *);
void OPENTTD_CRYPTO_CALL openttd_rust_aead_write(const struct OpenTTDCryptoLeaves *, const struct OpenTTDAeadLayout *, void *, uint8_t *, uint8_t *, const uint8_t *, size_t, const uint8_t *, size_t);
int32_t OPENTTD_CRYPTO_CALL openttd_rust_aead_read(const struct OpenTTDCryptoLeaves *, const struct OpenTTDAeadLayout *, void *, uint8_t *, const uint8_t *, const uint8_t *, size_t, const uint8_t *, size_t);
void OPENTTD_CRYPTO_CALL openttd_rust_aead_lock(const struct OpenTTDCryptoLeaves *, const struct OpenTTDAeadLayout *, void *, uint8_t *, uint8_t *, const uint8_t *, const uint8_t *, const uint8_t *, size_t, const uint8_t *, size_t);
int32_t OPENTTD_CRYPTO_CALL openttd_rust_aead_unlock(const struct OpenTTDCryptoLeaves *, const struct OpenTTDAeadLayout *, void *, uint8_t *, const uint8_t *, const uint8_t *, const uint8_t *, const uint8_t *, size_t, const uint8_t *, size_t);
#ifdef __cplusplus
}
#endif
#endif /* OPENTTD_CRYPTO_PRIMITIVES_FFI_H */
