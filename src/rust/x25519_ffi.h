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

/** @file x25519_ffi.h Fixed byte spans and complete Montgomery ladder. */
#ifndef OPENTTD_X25519_FFI_H
#define OPENTTD_X25519_FFI_H
#include "service_defaults.hpp"
#include "crypto_primitives_ffi.h"
#ifdef __cplusplus
extern "C" {
#endif
struct OpenTTDX25519Leaves {
	void (OPENTTD_CRYPTO_CALL *wipe)(void *, size_t) OPENTTD_SERVICE_DEFAULT(wipe);
	int32_t (OPENTTD_CRYPTO_CALL *verify32)(const uint8_t *, const uint8_t *) OPENTTD_SERVICE_DEFAULT(verify32);
};
/* Immutable initialized table borrowed synchronously, containing only original
 * nonthrowing wipe/constant-time verify32 leaves. No global registration,
 * heap, application callback, direct vendor import or C++ unwind through Rust.
 * Buffers are original fixed readable/writable 32-byte spans; output may overlap
 * secret/scalar/point, including partial overlap. All inputs are read before
 * output serialization. Trim separately retains literal FORWARD byte-copy
 * behavior, including propagation for forward overlaps; it is not memmove.
 * Coarse ladder callers supply 255/256 bits. No C++ field representation crosses
 * ABI; internal ten-limb Rust storage preserves signed bounds/carry/shifts.
 * No pointer survives a call. Serialized operations, Rust panic aborts. Original
 * wipe points/order remain; no complete compiler-copy/spill erasure claim. */
void OPENTTD_CRYPTO_CALL openttd_rust_x25519_trim(uint8_t *, const uint8_t *);
void OPENTTD_CRYPTO_CALL openttd_rust_x25519_ladder(const struct OpenTTDX25519Leaves *, uint8_t *, const uint8_t *, const uint8_t *, int32_t);
void OPENTTD_CRYPTO_CALL openttd_rust_x25519(const struct OpenTTDX25519Leaves *, uint8_t *, const uint8_t *, const uint8_t *);
void OPENTTD_CRYPTO_CALL openttd_rust_x25519_public_key(const struct OpenTTDX25519Leaves *, uint8_t *, const uint8_t *);
#ifdef __cplusplus
}
#endif
#endif /* OPENTTD_X25519_FFI_H */
