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

/** @file blake2b_ffi.h Borrowed actual BLAKE2b context and primitive wipe leaf. */
#ifndef OPENTTD_BLAKE2B_FFI_H
#define OPENTTD_BLAKE2B_FFI_H
#include "crypto_primitives_ffi.h"
#ifdef __cplusplus
extern "C" {
#endif
struct OpenTTDBlake2bLayout {
	size_t size, alignment, hash, input_offset, input, input_idx, hash_size;
};
/* Actual context lifetime starts in C++; immutable initialized layout/leaves
 * are borrowed synchronously. Fieldwise raw access handles actual u64 alignment;
 * no context padding is read and final wipes its complete actual storage.
 * Context is distinct from message/key/hash according to original incremental
 * preconditions. For hashes, one-shot output may overlap message/key, including
 * partial overlap: inputs are consumed before output writes. This deliberately
 * differs from cipher text aliasing. No external buffer becomes a Rust reference.
 * Empty message/key may be NULL; update(NULL, 0) accesses no context/layout.
 * Original readable/writable extents <=isize::MAX apply. No size validation is
 * added: init preserves supplied hash_size; final writes min(hash_size,64).
 * Keys <=128 reproduce defined source storage, though documented usage is <=64;
 * larger keys overrun original storage and have no compatibility guarantee.
 * No allocation, retained pointers, callback registration or C++ unwinding.
 * Panics abort. Original temporary wipe points are unchanged. */
void OPENTTD_RUST_CALL openttd_rust_blake2b_keyed_init(const struct OpenTTDBlake2bLayout *, void *, size_t, const uint8_t *, size_t);
void OPENTTD_RUST_CALL openttd_rust_blake2b_init(const struct OpenTTDBlake2bLayout *, void *, size_t);
void OPENTTD_RUST_CALL openttd_rust_blake2b_update(const struct OpenTTDBlake2bLayout *, void *, const uint8_t *, size_t);
void OPENTTD_RUST_CALL openttd_rust_blake2b_final(const struct OpenTTDCryptoLeaves *, const struct OpenTTDBlake2bLayout *, void *, uint8_t *);
void OPENTTD_RUST_CALL openttd_rust_blake2b_keyed(const struct OpenTTDCryptoLeaves *, const struct OpenTTDBlake2bLayout *, void *, uint8_t *, size_t, const uint8_t *, size_t, const uint8_t *, size_t);
void OPENTTD_RUST_CALL openttd_rust_blake2b(const struct OpenTTDCryptoLeaves *, const struct OpenTTDBlake2bLayout *, void *, uint8_t *, size_t, const uint8_t *, size_t);
#ifdef __cplusplus
}
#endif
#endif /* OPENTTD_BLAKE2B_FFI_H */
