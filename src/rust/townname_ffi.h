/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file townname_ffi.h Complete built-in town names with Rust-owned output. */
#ifndef OPENTTD_RUST_TOWNNAME_FFI_H
#define OPENTTD_RUST_TOWNNAME_FFI_H
#include <stddef.h>
#include <stdint.h>
#if defined(_MSC_VER)
#define OPENTTD_TOWNNAME_CALL __cdecl
#elif defined(__i386__)
#define OPENTTD_TOWNNAME_CALL __attribute__((cdecl))
#else
#define OPENTTD_TOWNNAME_CALL
#endif
#ifdef __cplusplus
extern "C" {
#endif
typedef struct OpenTTDTownNameResult OpenTTDTownNameResult;
/* lang is an original valid built-in ID (0..20), seed any uint32_t. Generate owns its output in an
 * opaque Rust allocation. Data returns immutable bytes (not NUL terminated), valid until destroy;
 * length is a writable size_t. Copy or append in C++ before destroying once, also on a C++
 * allocation exception. Rust allocation failure timing differs from the original. */
OpenTTDTownNameResult *OPENTTD_TOWNNAME_CALL openttd_rust_townname_generate(size_t lang, uint32_t seed);
const char *OPENTTD_TOWNNAME_CALL openttd_rust_townname_data(const OpenTTDTownNameResult *, size_t *length);
void OPENTTD_TOWNNAME_CALL openttd_rust_townname_destroy(OpenTTDTownNameResult *);
#ifdef __cplusplus
}
#endif
#endif /* OPENTTD_RUST_TOWNNAME_FFI_H */
