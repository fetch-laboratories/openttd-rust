/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file trees_ffi.h Rust tree generation, simulation and command ownership. */
#ifndef TREES_FFI_H
#define TREES_FFI_H
#include "service_defaults.hpp"
#include <cstdint>
#include "services_ffi.h"

/* Settings of one tile-loop batch. climate is a LandscapeType and extra an ExtraTreePlacement. */
struct OpenTTDTreeLoopSettings {
	uint64_t tick_counter;
	uint32_t size_x;
	uint8_t climate, extra;
	bool ambient, ambient_callback;
};
/* CmdPlantTree's error messages. */
struct OpenTTDTreeStrings {
	uint32_t already_here;
	uint32_t on_water;
	uint32_t plant_limit;
	uint32_t site_unsuitable;
	uint32_t wrong_terrain;
};
/* The same-named tree_map.h/clear_map.h writers, the native sinf/cosf, and the generation
 * progress updates. progress/progress_total return true when world generation was aborted: the
 * C++ wrapper keeps the exception, Rust returns without another draw or write, and the entry's
 * caller rethrows it. */
struct OpenTTDCommandCost;
struct OpenTTDCommandServices;
struct OpenTTDTownLeaves;
struct OpenTTDTownAuthorityServices;
struct OpenTTDTreeServices {
	void (*add_count)(uint32_t tile, int32_t count) noexcept OPENTTD_SERVICE_DEFAULT(add_count);
	void (*add_growth)(uint32_t tile, int32_t growth) noexcept OPENTTD_SERVICE_DEFAULT(add_growth);
	const OpenTTDTownAuthorityServices *authority;
	float (*cos)(float) noexcept OPENTTD_SERVICE_DEFAULT(cos);
	void (*make_clear)(uint32_t tile, uint8_t ground, uint32_t density) noexcept OPENTTD_SERVICE_DEFAULT(make_clear);
	void (*make_tree)(uint32_t tile, uint8_t type, uint32_t count, uint8_t growth, uint8_t ground, uint32_t density) noexcept OPENTTD_SERVICE_DEFAULT(make_tree);
	bool (*progress)() noexcept OPENTTD_SERVICE_DEFAULT(progress);
	bool (*progress_total)(uint32_t total) noexcept OPENTTD_SERVICE_DEFAULT(progress_total);
	void (*set_ground_density)(uint32_t tile, uint8_t ground, uint32_t density) noexcept OPENTTD_SERVICE_DEFAULT(set_ground_density);
	void (*set_growth)(uint32_t tile, uint8_t growth) noexcept OPENTTD_SERVICE_DEFAULT(set_growth);
	float (*sin)(float) noexcept OPENTTD_SERVICE_DEFAULT(sin);
	OpenTTDTreeStrings strings;
	const OpenTTDTownLeaves *town;
};

struct OpenTTDWorldServices;
extern "C" {
/* Plain synchronous entries; original game preconditions apply. Panics abort. */
uint32_t openttd_rust_trees_place_group(uint32_t tile, uint8_t type, uint32_t radius, uint32_t count, bool set_zone, const OpenTTDTreeServices *, const OpenTTDSharedServices *, const OpenTTDWorldServices *);
uint8_t openttd_test_trees_generate(const OpenTTDTreeServices *, const OpenTTDSharedServices *);
uint8_t openttd_rust_trees_place_randomly_abi(const OpenTTDTreeServices *, const OpenTTDSharedServices *);
extern "C++" inline bool openttd_rust_trees_place_randomly(const OpenTTDTreeServices *a0, const OpenTTDSharedServices *a1) { return openttd_rust_trees_place_randomly_abi(a0, a1) != 0; }
/* CmdPlantTree: writes the command's result to the native slot. */
void openttd_rust_trees_cmd_plant(uint16_t flags, const OpenTTDPlantTreeArgs *, OpenTTDCommandCost *, const OpenTTDTreeServices *, const OpenTTDSharedServices *, const OpenTTDCommandServices *);
/* Stable Rust-owned byte; game-thread initialization/ticks and serial save/load access only.
 * C++ never retains a C++ reference across a Rust call. DATE LoadCheck omits this field.
 * The unchanged modern and TTD/TTO descriptors access it only at serialization boundaries. */
uint8_t *openttd_rust_tree_counter();
void openttd_rust_trees_initialize();
}
#endif /* TREES_FFI_H */
