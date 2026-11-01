/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file tilearea_ffi.h Rust tile-area geometry over native areas borrowed for one call. */
#ifndef RUST_TILEAREA_FFI_H
#define RUST_TILEAREA_FFI_H

#include <stdint.h>

struct OpenTTDSharedServices;
struct OrthogonalTileArea;
struct DiagonalTileArea;

/** DiagonalTileIterator fields after the inherited current tile. */
struct OpenTTDDiagonalTileCursor {
	uint32_t base_x;
	uint32_t base_y;
	int32_t a_cur;
	int32_t b_cur;
	int32_t a_max;
	int32_t b_max;
};

extern "C" {
void openttd_rust_tile_area_corners(OrthogonalTileArea *out, uint32_t start, uint32_t end, const OpenTTDSharedServices *shared);
void openttd_rust_tile_area_add(OrthogonalTileArea *area, uint32_t to_add, const OpenTTDSharedServices *shared);
bool openttd_rust_tile_area_intersects(const OrthogonalTileArea *area, const OrthogonalTileArea *other, const OpenTTDSharedServices *shared);
bool openttd_rust_tile_area_contains(const OrthogonalTileArea *area, uint32_t tile, const OpenTTDSharedServices *shared);
void openttd_rust_tile_area_expand(OrthogonalTileArea *area, int32_t radius, const OpenTTDSharedServices *shared);
void openttd_rust_tile_area_clamp_to_map(OrthogonalTileArea *area, const OpenTTDSharedServices *shared);
void openttd_rust_diagonal_tile_area_corners(DiagonalTileArea *out, uint32_t start, uint32_t end, const OpenTTDSharedServices *shared);
bool openttd_rust_diagonal_tile_area_contains(const DiagonalTileArea *area, uint32_t tile, const OpenTTDSharedServices *shared);
uint32_t openttd_rust_diagonal_tile_advance(OpenTTDDiagonalTileCursor *cursor, uint32_t tile, const OpenTTDSharedServices *shared); ///< Returns the next current tile.
}

#endif /* RUST_TILEAREA_FFI_H */
