/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file ffi.h Narrow scalar ABI for migrated Rust kernels. */

#ifndef RUST_FFI_H
#define RUST_FFI_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Height for original signed coordinates and the original eight-bit slope.
 * The uint32_t height is widened; UINT64_MAX requests the C++ fatal handler.
 * No pointers or ownership transfer. Rust never unwinds into C++; panic aborts.
 */
uint64_t openttd_rust_get_partial_pixel_z(int32_t x, int32_t y, uint8_t corners);

/** Integer result: width-sized value bits, parse length, and diagnostic byte spans. */
typedef struct OpenTTDRustIntegerResult {
	uint64_t value_bits;
	size_t length;
	size_t error_offset;
	size_t error_length;
	uint8_t error_kind; ///< 0 = none, 1 = invalid, 2 = range, 3 = negative-hex range.
} OpenTTDRustIntegerResult;

/** Borrow readable bytes for this call only; zero length permits NULL, and arbitrary bytes and
 * embedded NULs are accepted. Base is 0/8/10/16; width is the native integer bit width (at most
 * 64). Signed/clamp are 0/1. The span is one live immutable allocation of length <= PTRDIFF_MAX. */
OpenTTDRustIntegerResult openttd_rust_parse_integer(const uint8_t *src, size_t length, uint8_t base, uint8_t width, uint8_t is_signed, uint8_t clamp);

/** Independent lexical skip, with the same byte-borrow/base contract. */
size_t openttd_rust_skip_integer(const uint8_t *src, size_t length, uint8_t base);

#ifdef __cplusplus
}
#endif

#endif /* RUST_FFI_H */
