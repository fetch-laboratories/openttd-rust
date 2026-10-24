/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file history_ffi.h Staged scalar history structure; typed execution stays in C++. */
#ifndef RUST_HISTORY_FFI_H
#define RUST_HISTORY_FFI_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

typedef struct OpenTTDRustHistoryEngine OpenTTDRustHistoryEngine;

typedef struct {
	uintptr_t child;
	uint8_t periods, records, first, last, division, total_division;
	uint8_t child_periods, child_division;
} OpenTTDRustHistoryDescriptor;

typedef struct {
	uint8_t kind, count;
	uint32_t first, last, target;
	uintptr_t token;
	uint64_t value;
} OpenTTDRustHistoryStep;

/** Mode 0 update, 1 first-child validity, 2 rotation, 3 OR-validity query. Tokens are opaque
 * uintptr_t identities that only C++ round-trips; the identified HistoryRange objects form a live
 * acyclic chain until destroy. Typed history operations and exceptions run between calls. Age and
 * phase use native uint32 arithmetic. Each engine and frame is destroyed once by its function. */
OpenTTDRustHistoryEngine *openttd_rust_history_create(uint8_t mode, uintptr_t root, uint64_t mask, uint32_t age, uint32_t month);

/**
 * 0 done, 1 describe, 2 shift, 3 copy, 4 reset, 5 rotation reduction,
 * 6 construct all scratch elements, 7 child query, 8 query reduction, 9 leaf,
 * 10 original invalid-age fatal path. Execute typed instructions after return.
 */
OpenTTDRustHistoryStep openttd_rust_history_next(OpenTTDRustHistoryEngine *engine);
void openttd_rust_history_describe(OpenTTDRustHistoryEngine *engine, OpenTTDRustHistoryDescriptor descriptor); ///< Supply requested descriptor fields by value.
void openttd_rust_history_phase(OpenTTDRustHistoryEngine *engine, uint32_t month); ///< Live global phase AFTER scratch construction.
uint8_t openttd_rust_history_complete(OpenTTDRustHistoryEngine *engine); ///< After successful typed leaf/reduction; destroy on exceptions.
void openttd_rust_history_destroy(OpenTTDRustHistoryEngine *engine); ///< Exactly once, including typed C++ unwinding; no live handle access.
#ifdef __cplusplus
}
#endif
#endif /* RUST_HISTORY_FFI_H */
