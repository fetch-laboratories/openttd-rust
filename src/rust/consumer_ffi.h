/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file consumer_ffi.h Borrowed byte algorithms and by-value cursor decisions. */
#ifndef RUST_CONSUMER_FFI_H
#define RUST_CONSUMER_FFI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Bounded view and delayed cursor commit; shortfall is 0/1. */
typedef struct {
	size_t length;
	size_t position;
	uint8_t shortfall;
} OpenTTDRustConsumerBound;

/** Unsigned bits plus successful width; zero length means no result. */
typedef struct {
	uint64_t value_bits;
	size_t length;
} OpenTTDRustConsumerByte;

/** Empty prefix matches (matched=1, length=0); unmatched length is also zero. */
typedef struct {
	size_t length;
	uint8_t matched;
} OpenTTDRustConsumerMatch;

/** Returned view and consumed bytes differ for separator SKIP policies. */
typedef struct {
	size_t result_length;
	size_t consumed_length;
} OpenTTDRustConsumerSeparator;

/**
 * size/position/requested use size_t/Rust usize; position <= size.
 * SIZE_MAX means all remaining bytes, never shortfall. No borrowed memory.
 * Returned position <= size; log a shortfall before committing the position.
 */
OpenTTDRustConsumerBound openttd_rust_consumer_bound(size_t size, size_t position, size_t requested);

/** Nonempty spans address readable initialized bytes, immutable for this call; empty spans permit
 * NULL and read-only spans may overlap. Each span lies in one allocation of length <= PTRDIFF_MAX.
 * No callback runs while borrowing; all results are by value. */
OpenTTDRustConsumerByte openttd_rust_consumer_little_endian(const uint8_t *data, size_t length, uint8_t width); ///< Width is 1/2/4/8 bytes.
OpenTTDRustConsumerMatch openttd_rust_consumer_prefix(const uint8_t *data, size_t length, const uint8_t *pattern, size_t pattern_length); ///< Empty patterns are valid.
size_t openttd_rust_consumer_find(const uint8_t *data, size_t length, const uint8_t *pattern, size_t pattern_length, uint8_t mode); ///< Nonempty pattern; mode 0 substring, 1 set membership, 2 nonmembership. SIZE_MAX is not found.
OpenTTDRustConsumerByte openttd_rust_consumer_character(const uint8_t *data, size_t length, const uint8_t *pattern, size_t pattern_length, uint8_t member); ///< Nonempty pattern; member is 0/1.
OpenTTDRustConsumerSeparator openttd_rust_consumer_separator(const uint8_t *data, size_t length, const uint8_t *pattern, size_t pattern_length, int32_t policy); ///< Nonempty separator. Policies 0 READ_ALL, 1 READ_ONE, 2 KEEP, 3 SKIP_ONE, 4 SKIP_ALL; others KEEP.

#ifdef __cplusplus
}
#endif
#endif /* RUST_CONSUMER_FFI_H */
