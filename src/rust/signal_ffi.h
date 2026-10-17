/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file signal_ffi.h Rail signal-block owner boundary (ABI 820-859). */
#ifndef RUST_SIGNAL_FFI_H
#define RUST_SIGNAL_FFI_H
#include "services_ffi.h"

/* Signal work sets have process lifetime. The game-state mutex serializes all
 * entries, including calls from main and game threads. Each native service is
 * synchronous and noexcept. Rust releases every state borrow before a callback;
 * map, occupancy and dirtying cannot unwind ordinary-play exceptions. */
struct OpenTTDSignalServices {
	bool assertions;
	void (*assert_condition)(bool) noexcept;
	void (*assert_wormhole_start)(uint32_t, uint8_t) noexcept;
	bool (*train_on_tile)(uint32_t) noexcept;
	bool (*train_on_tracks)(uint32_t, uint8_t) noexcept;
	void (*overflow_tbu)() noexcept;
	void (*overflow_tbd)() noexcept;
	void (*overflow_global)() noexcept;
	void (*assert_company)(uint8_t) noexcept;
};

struct OpenTTDSignalItem { uint32_t tile; uint8_t dir; };
struct OpenTTDSignalSnapshot {
	unsigned int tbu_count, tbd_count, global_count;
	bool overflow[3];
	uint8_t last_owner;
	OpenTTDSignalItem tbu[64], tbd[256], global[128];
};

struct OpenTTDSignalContext {
	const OpenTTDSharedServices *shared;
	const OpenTTDRailMapServices *rail;
	const OpenTTDSignalServices *native;
	unsigned int log_x;
};
OpenTTDSignalContext GetRustSignalContext() noexcept;
const OpenTTDSignalServices &GetRustSignalServices() noexcept;

struct OpenTTDSignalSetTrace {
	unsigned int duplicate_count;
	OpenTTDSignalItem duplicate_pop[3];
	bool full_failed, overflow_full;
	OpenTTDSignalItem first_after_remove;
	bool refill_succeeded;
	OpenTTDSignalItem refill_pop;
	unsigned int drained_count;
	bool overflow_drained, overflow_reset;
};

extern "C" {
void openttd_rust_signal_count_occupancy(uint64_t);
void openttd_rust_signal_workset_probe(const OpenTTDSignalServices *, OpenTTDSignalSetTrace *);
void openttd_rust_signal_inspect(OpenTTDSignalSnapshot *);
void openttd_rust_signal_flush(const OpenTTDSharedServices *, const OpenTTDRailMapServices *, const OpenTTDSignalServices *, unsigned int);
void openttd_rust_signal_add_track(const OpenTTDSharedServices *, const OpenTTDRailMapServices *, const OpenTTDSignalServices *, unsigned int, uint32_t, uint8_t, uint8_t);
void openttd_rust_signal_add_side(const OpenTTDSharedServices *, const OpenTTDRailMapServices *, const OpenTTDSignalServices *, unsigned int, uint32_t, uint8_t, uint8_t);
uint8_t openttd_rust_signal_segment(const OpenTTDSharedServices *, const OpenTTDRailMapServices *, const OpenTTDSignalServices *, unsigned int, uint32_t, uint8_t, uint8_t);
void openttd_rust_signal_both(const OpenTTDSharedServices *, const OpenTTDRailMapServices *, const OpenTTDSignalServices *, unsigned int, uint32_t, uint8_t, uint8_t);
}
#endif /* RUST_SIGNAL_FFI_H */
