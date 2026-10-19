/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file tile_loop_cursor.h Serialization alias for the canonical scheduler cursor. */
#ifndef RUST_TILE_LOOP_CURSOR_H
#define RUST_TILE_LOOP_CURSOR_H
#ifdef WITH_RUST
#include <cstdint>
extern "C" uint32_t *openttd_rust_tile_loop_cursor();
using TileLoopCursorType = uint32_t;
#define _cur_tileloop_tile (*openttd_rust_tile_loop_cursor())
#else
using TileLoopCursorType = TileIndex;
extern TileIndex _cur_tileloop_tile;
#endif
#endif /* RUST_TILE_LOOP_CURSOR_H */
