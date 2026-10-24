/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file station_cargo_ffi.h Stack-owned station cargo reducer. */
#ifndef OPENTTD_RUST_STATION_CARGO_FFI_H
#define OPENTTD_RUST_STATION_CARGO_FFI_H
#include "script_list_ffi.h"
#ifdef __cplusplus
extern "C" {
#endif
/* No layout contains a pointer/allocator. State and opaque destination are live,
 * aligned, exclusive and disjoint during each call only. No pointer is retained;
 * world/VM/iterator calls and C++ exceptions occur between Rust returns. Destination
 * outlives collector. Finalize flushes once, including early return/unwinding.
 * uint32 runs/share differences wrap; original signed result/token overflow is
 * outside defined input. Rust panics/OOM abort under existing ScriptList limits. */
typedef struct {
	uint32_t amount;
	uint32_t previous;
	uint16_t last_key;
	uint16_t other;
	uint16_t origin;
	uint8_t selector;
	uint8_t finalized;
} OpenTTDCargoCollector;
/* Plan: 0 all waiting packets, 1 equal_range(via), 2 all origins, 3 find(from).
 * Invalid mode/selector returns255 for the original C++ fatal policy. */
uint8_t OPENTTD_RUST_CALL openttd_rust_cargo_plan(uint8_t mode, uint8_t selector);
void OPENTTD_RUST_CALL openttd_rust_cargo_init(OpenTTDCargoCollector *, uint8_t selector, uint16_t other);
void OPENTTD_RUST_CALL openttd_rust_cargo_packet(OpenTTDCargoCollector *, struct OpenTTDScriptList *, uint16_t from, uint16_t via, uint32_t amount);
void OPENTTD_RUST_CALL openttd_rust_cargo_origin(OpenTTDCargoCollector *, uint16_t origin);
void OPENTTD_RUST_CALL openttd_rust_cargo_share(OpenTTDCargoCollector *, struct OpenTTDScriptList *, uint16_t via, uint32_t cumulative);
void OPENTTD_RUST_CALL openttd_rust_cargo_finish(OpenTTDCargoCollector *, struct OpenTTDScriptList *);
#ifdef __cplusplus
}
#endif
#endif /* OPENTTD_RUST_STATION_CARGO_FFI_H */
