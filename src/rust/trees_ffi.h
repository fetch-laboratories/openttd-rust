/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file trees_ffi.h Rust tree generation, simulation and command ownership. */
#ifndef TREES_FFI_H
#define TREES_FFI_H
#include <cstdint>
#include "services_ffi.h"

/* Fields CanPlantTreesOnTile, GetRandomTreeType and PlantTreesOnTile observe. Clear
 * fields are set on clear tiles; coast is IsCoast && !IsSlopeWithOneCornerRaised and
 * is set on water tiles only, so the slope is read only on coast tiles. */
struct OpenTTDTreePlantObservation {
	uint8_t tile_type;
	bool bridge;
	uint8_t ground;
	uint8_t density;
	bool snow;
	bool coast;
	uint8_t zone;
};
/* Tree fields of one MP_TREES tile. */
struct OpenTTDTreeObservation {
	uint8_t ground, density, species, count, growth, zone;
};
/* Only the fields ClearTile_Trees uses, with GetTreeCount's uint return type. */
struct OpenTTDTreeClearObservation {
	uint32_t count;
	uint8_t species;
};
/* Settings read once per entry. Prices are Money values. */
struct OpenTTDTreeLoopSettings {
	uint64_t tick_counter;
	uint32_t size_x;
	uint8_t climate, extra;
	bool ambient;
};
struct OpenTTDTreeTickSettings {
	uint64_t tick_counter;
	uint32_t size;
	uint8_t climate, extra;
};
struct OpenTTDTreeGenerateSettings {
	uint32_t size, size_x, size_y;
	uint8_t climate, placer, height_limit;
	bool editor, freeform_edges;
};
struct OpenTTDTreeCommandSettings {
	int64_t build_price;
	uint32_t size;
	uint8_t climate;
	bool editor, execute, company_valid;
};
/* Status 0: the cost, or message index when the cost is zero; 1: CMD_ERROR;
 * 2: the failed nested landscape clear kept by the command context. */
struct OpenTTDTreeCommandResult {
	int64_t cost;
	uint8_t status, message;
};
struct OpenTTDTreeLandscapeClear {
	int64_t cost;
	bool failed;
};
/* Command flags, company, tile iterator and nested result; opaque to Rust. */
struct OpenTTDTreeCommand;

/* Designated, immutable table of noexcept services. Reads are pure. Writes are the
 * same-named tree_map.h/clear_map.h functions. tile_loop_water, ambient and
 * landscape_clear can reenter tree entries; Rust holds no borrow across them and
 * reads the tile again afterwards. ambient returns whether its NewGRF callback ran.
 * progress/progress_total return true when world generation was aborted: the C++
 * wrapper keeps the exception, Rust returns without another draw or write, and the
 * entry's caller rethrows it. No other C++ exception reaches these services. */
struct OpenTTDTreeServices {
	OpenTTDTreePlantObservation (*plant_observation)(uint32_t) noexcept;
	OpenTTDTreeObservation (*tree_observation)(uint32_t) noexcept;
	uint32_t (*tree_count)(uint32_t) noexcept;
	OpenTTDTreeClearObservation (*clear_observation)(uint32_t) noexcept;
	uint8_t (*snow_line)() noexcept;
	float (*sin)(float) noexcept;
	float (*cos)(float) noexcept;
	void (*make_tree)(uint32_t tile, uint8_t type, uint32_t count, uint8_t growth, uint8_t ground, uint32_t density) noexcept;
	void (*set_ground_density)(uint32_t tile, uint8_t ground, uint32_t density) noexcept;
	void (*add_count)(uint32_t tile, int32_t count) noexcept;
	void (*add_growth)(uint32_t tile, int32_t growth) noexcept;
	void (*set_growth)(uint32_t tile, uint8_t growth) noexcept;
	void (*make_clear)(uint32_t tile, uint8_t ground, uint32_t density) noexcept;
	void (*make_shore)(uint32_t tile) noexcept;
	void (*make_snow)(uint32_t tile, uint32_t density) noexcept;
	void (*clear_neighbour_flooding)(uint32_t tile) noexcept;
	void (*tile_loop_water)(uint32_t tile) noexcept;
	bool (*ambient)(uint32_t tile) noexcept;
	void (*play_sound)(uint32_t tile, uint16_t sound) noexcept;
	bool (*progress)() noexcept;
	bool (*progress_total)(uint32_t total) noexcept;
	void (*town_rating)(OpenTTDTreeCommand *, uint32_t tile, bool up) noexcept;
	int32_t (*command_begin)(OpenTTDTreeCommand *, uint32_t end, uint32_t start, bool diagonal) noexcept;
	uint32_t (*command_tile)(OpenTTDTreeCommand *) noexcept;
	uint32_t (*command_next)(OpenTTDTreeCommand *) noexcept;
	void (*command_debit)(OpenTTDTreeCommand *) noexcept;
	OpenTTDTreeLandscapeClear (*landscape_clear)(OpenTTDTreeCommand *, uint32_t tile) noexcept;
};

extern "C" {
/* Plain synchronous entries; original game preconditions apply. Panics abort. */
void openttd_rust_trees_tile_loop(uint32_t tile, OpenTTDTreeLoopSettings, const OpenTTDTreeServices *, const OpenTTDSharedServices *);
void openttd_rust_trees_tick(OpenTTDTreeTickSettings, const OpenTTDTreeServices *, const OpenTTDSharedServices *);
bool openttd_rust_trees_generate(OpenTTDTreeGenerateSettings, const OpenTTDTreeServices *, const OpenTTDSharedServices *);
bool openttd_rust_trees_place_randomly(OpenTTDTreeGenerateSettings, const OpenTTDTreeServices *, const OpenTTDSharedServices *);
void openttd_rust_trees_place(uint32_t tile, uint32_t r, bool keep_density, uint8_t climate, const OpenTTDTreeServices *, const OpenTTDSharedServices *);
void openttd_rust_trees_plant(uint32_t tile, uint8_t type, uint32_t count, uint8_t growth, const OpenTTDTreeServices *, const OpenTTDSharedServices *);
bool openttd_rust_trees_can_plant(uint32_t tile, bool allow_desert, const OpenTTDTreeServices *);
OpenTTDTreeCommandResult openttd_rust_trees_cmd_plant(OpenTTDTreeCommand *, uint32_t tile, uint32_t start, uint8_t tree, bool diagonal, OpenTTDTreeCommandSettings, const OpenTTDTreeServices *, const OpenTTDSharedServices *);
int64_t openttd_rust_trees_clear_tile(OpenTTDTreeCommand *, uint32_t tile, int64_t price, bool execute, bool company_valid, const OpenTTDTreeServices *, const OpenTTDSharedServices *);
/* Stable Rust-owned byte; game-thread initialization/ticks and serial save/load access only.
 * C++ never retains a C++ reference across a Rust call. DATE LoadCheck omits this field.
 * The unchanged modern and TTD/TTO descriptors access it only at serialization boundaries. */
uint8_t *openttd_rust_tree_counter();
void openttd_rust_trees_initialize();
}
#endif /* TREES_FFI_H */
