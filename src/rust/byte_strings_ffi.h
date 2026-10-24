/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file byte_strings_ffi.h Native-locale byte algorithms and trim offsets. */
#ifndef RUST_BYTE_STRINGS_FFI_H
#define RUST_BYTE_STRINGS_FFI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
	size_t offset;
	size_t length;
} OpenTTDRustByteTrim;

/** Read-only spans are initialized bytes in one live allocation each, empty spans permit NULL, and
 * shared spans may overlap. Case mapping keeps native toupper(char), including negative chars
 * outside portable C's domain; signed_char is the native char sign. mode: 0 compare, 1 equal,
 * 2 prefix, 3 suffix, 4 contains. length_order supplies the C++ library's equal-prefix result. */
int32_t openttd_rust_bytes_case(const uint8_t *left, size_t left_length, const uint8_t *right, size_t right_length, uint8_t mode, uint8_t signed_char, int32_t length_order);

/** Exclusive initialized writable span; offset <= length. Empty permits NULL. */
uint8_t openttd_rust_bytes_lower(uint8_t *data, size_t length, size_t offset);

/** Readable input and exclusive initialized output are disjoint; output is 2*length bytes. */
void openttd_rust_bytes_hex_encode(const uint8_t *data, size_t length, uint8_t *output);

/**
 * Initialized readable input and live writable output may overlap, without other
 * accesses during the call. Output may be uninitialized except bytes also read
 * through input. Both nibbles are read before each write; earlier writes affect later
 * reads. Bad lengths write nothing; invalid pairs preserve prior writes. No Rust
 * references alias these spans. Empty spans allow NULL, length <= PTRDIFF_MAX.
 */
uint8_t openttd_rust_bytes_hex_decode(const uint8_t *hex, size_t length, uint8_t *output, size_t output_length);

/** Zero length means default null-data view; otherwise take source.substr(offset,length). */
OpenTTDRustByteTrim openttd_rust_bytes_trim(const uint8_t *data, size_t length, const uint8_t *set, size_t set_length);

#ifdef __cplusplus
}
#endif
#endif /* RUST_BYTE_STRINGS_FFI_H */
