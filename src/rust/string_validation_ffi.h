/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file string_validation_ffi.h Borrowed sanitation and in-place copy decisions. */
#ifndef OPENTTD_RUST_STRING_VALIDATION_FFI_H
#define OPENTTD_RUST_STRING_VALIDATION_FFI_H
#include "utf8_ffi.h"
#include "call_ffi.h"
#ifdef __cplusplus
extern "C" {
#endif
/** No heap owner: C++ consumes input before its next output/allocation. */
typedef struct {
	size_t consumed;
	OpenTTDRustUtf8Encoded output;
	uint8_t stopped;
} OpenTTDValidationStep;
typedef struct {
	size_t position;
	uint8_t accepted;
} OpenTTDInplaceWrite;
/* Input is one readable allocation <= PTRDIFF_MAX, valid only during the call;
 * zero length accepts null. Result bytes own their storage. Unknown settings
 * bits are ignored. No callback or C++ allocation occurs in Rust; panic aborts. */
OpenTTDValidationStep OPENTTD_RUST_CALL openttd_rust_validation_step(const uint8_t *, size_t length, uint8_t settings);
uint8_t OPENTTD_RUST_CALL openttd_rust_validation_valid(const uint8_t *, size_t length);
/* Original ranges::copy preconditions apply. On accepted live-consumer capacity,
 * source length bytes are readable and dest[position..position+length] writable
 * in their live allocations (<= PTRDIFF_MAX). Output start must be outside the
 * nonempty input range; defined left overlap is allowed. C++ views may remain
 * alive without concurrent access. Raw reads precede writes; no Rust references
 * or overlapping slices are constructed.
 * Zero length accesses neither pointer. Overtake leaves output/position unchanged;
 * C++ fatal dispatch happens only after return. No borrow survives a call. */
OpenTTDInplaceWrite OPENTTD_RUST_CALL openttd_rust_inplace_write(uint8_t *dest, size_t position, size_t consumed, const uint8_t *source, size_t length);
#ifdef __cplusplus
}
#endif
#endif /* OPENTTD_RUST_STRING_VALIDATION_FFI_H */
