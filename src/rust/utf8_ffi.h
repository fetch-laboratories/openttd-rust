/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file utf8_ffi.h Borrowed byte-span ABI for historical UTF-8 algorithms. */
#ifndef RUST_UTF8_FFI_H
#define RUST_UTF8_FFI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Encoded bytes by value: all unused bytes are zero; length zero means invalid. */
typedef struct {
	uint8_t bytes[4];
	size_t length;
} OpenTTDRustUtf8Encoded;

/** First decoded sequence by value: both fields zero mean invalid or truncated. */
typedef struct {
	size_t length;
	uint32_t codepoint;
} OpenTTDRustUtf8Decoded;

OpenTTDRustUtf8Encoded openttd_rust_encode_utf8(uint32_t codepoint);
uint8_t openttd_rust_is_utf8_part(uint8_t byte);

/** Byte spans are borrowed, initialized and immutable for the call; zero length accepts a null
 * pointer and read-only spans may overlap. Results are returned by value, so no output aliases input. */
OpenTTDRustUtf8Decoded openttd_rust_decode_utf8(const uint8_t *data, size_t length);
/** Position < length; the caller preserves the original iterator assertion. */
size_t openttd_rust_utf8_next(const uint8_t *data, size_t length, size_t position);
/** 0 < position <= length; the caller preserves the original assertion. */
size_t openttd_rust_utf8_previous(const uint8_t *data, size_t length, size_t position);
/** Any offset; offset >= length returns length (the facade retains its assertion). */
size_t openttd_rust_utf8_at_byte(const uint8_t *data, size_t length, size_t offset);

#ifdef __cplusplus
}
#endif
#endif /* RUST_UTF8_FFI_H */
