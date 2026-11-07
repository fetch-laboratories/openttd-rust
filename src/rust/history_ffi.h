/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file history_ffi.h Direct operations on native history ranges and canonical elements. */
#ifndef RUST_HISTORY_FFI_H
#define RUST_HISTORY_FFI_H
#include <stdint.h>
#include <stdbool.h>
struct HistoryRange;
struct OpenTTDIndustryPair;
struct OpenTTDTownSuppliedHistory;
#ifdef __cplusplus
extern "C" {
#endif

/** Legacy ABI fixture compatibility, removed with its protocol consumer. */
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
 * phase use native uint32 arithmetic. Each engine and frame is destroyed once by its function.
 * Handles are live, nonnull and exclusive; the chain is immutable with the original valid-index,
 * divisor and bit-shift preconditions. */
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

/** Invalid ages reach the native fatal path after Rust returns; validity is independent. */
struct OpenTTDRustHistoryQuery { bool is_valid, in_range; };

/** Ranges form a live immutable acyclic chain with the original index/divisor/shift
 * preconditions. Arrays contain HISTORY_RECORDS initialized elements in their
 * canonical layout. Result may alias an element. Calls allocate nothing, retain
 * no pointers, and never unwind. Native fatal handling stays outside Rust. */
void openttd_rust_history_update_valid(uint64_t *mask, const HistoryRange *hr, uint32_t month) noexcept;
uint8_t openttd_rust_history_is_valid(uint64_t mask, const HistoryRange *hr, uint32_t age) noexcept;
void openttd_rust_history_rotate_industry(OpenTTDIndustryPair *history, uint64_t mask, const HistoryRange *hr, uint32_t month) noexcept;
OpenTTDRustHistoryQuery openttd_rust_history_get_industry(const OpenTTDIndustryPair *history, uint64_t mask, const HistoryRange *hr, uint32_t age, OpenTTDIndustryPair *result) noexcept;
void openttd_rust_history_rotate_town(OpenTTDTownSuppliedHistory *history, uint64_t mask, const HistoryRange *hr, uint32_t month) noexcept;
OpenTTDRustHistoryQuery openttd_rust_history_get_town(const OpenTTDTownSuppliedHistory *history, uint64_t mask, const HistoryRange *hr, uint32_t age, OpenTTDTownSuppliedHistory *result) noexcept;
#ifdef __cplusplus
}
#endif
#endif /* RUST_HISTORY_FFI_H */
