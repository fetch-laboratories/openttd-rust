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
