/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file alternating_ffi.h Scalar traversal ABI; no C++ iterators or pointers cross it. */

#ifndef RUST_ALTERNATING_FFI_H
#define RUST_ALTERNATING_FFI_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Selectors are 0 (before-middle) or 1 (after-middle). Copies are independent. */
typedef struct OpenTTDRustAlternatingState {
	size_t position; ///< Logical position, independent of the selected Base iterator.
	uint8_t next_after;
	uint8_t current_after;
} OpenTTDRustAlternatingState;

/** A movement request accompanying the updated scalar state. */
typedef struct OpenTTDRustAlternatingStep {
	OpenTTDRustAlternatingState state;
	uint8_t movement; ///< 0 = none, 1 = ++after, 2 = --before.
} OpenTTDRustAlternatingStep;

/**
 * end_position is 0 for a begin iterator, otherwise the live range distance.
 * before_at_first is 0/1. No allocation, ownership, borrow, or iterator crosses
 * this ABI. size_t maps to Rust usize; panic aborts and no unwinding occurs.
 */
OpenTTDRustAlternatingState openttd_rust_alternating_initialize(size_t end_position, uint8_t before_at_first);

/**
 * Choose movement from the current range distance; position < live_size.
 * Distance and typed movement satisfy the original valid iterator preconditions.
 * No cached length or random-access requirement. At logical end there is no
 * movement or completion: retain current_after and the last selected Base().
 */
OpenTTDRustAlternatingStep openttd_rust_alternating_advance(OpenTTDRustAlternatingState state, size_t live_size);

/**
 * Complete a requested typed move with its live boundary fact (0/1), in original
 * operation order. For movement 1 (++after), boundary means before == first;
 * for movement 2 (--before), boundary means std::next(after) != last.
 * Never complete a no-movement/end step. Rust selects the subsequent side.
 */
OpenTTDRustAlternatingState openttd_rust_alternating_complete(OpenTTDRustAlternatingState state, uint8_t boundary);

/** Compare logical positions only: -1, 0, or 1. C++ checks same range/middle. */
int8_t openttd_rust_alternating_compare(OpenTTDRustAlternatingState left, OpenTTDRustAlternatingState right);

#ifdef __cplusplus
}
#endif

#endif /* RUST_ALTERNATING_FFI_H */
