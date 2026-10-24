/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file tile_animation_services.h Shared packed animation and tile height leaves. */
#ifndef RUST_TILE_ANIMATION_SERVICES_H
#define RUST_TILE_ANIMATION_SERVICES_H
#include "../animated_tile_func.h"
#include "../tile_cmd.h"
#include "../map_func.h"
#include "../landscape.h"
inline uint8_t OpenTTDAnimationFrame(uint32_t tile) noexcept { return GetAnimationFrame(TileIndex(tile)); }
inline void OpenTTDSetAnimationFrame(uint32_t tile, uint8_t frame) noexcept { SetAnimationFrame(TileIndex(tile), frame); }
inline int32_t OpenTTDTileMaxPixelZ(uint32_t tile) noexcept { return GetTileMaxPixelZ(TileIndex(tile)); }
inline int OpenTTDTileMaxZ(TileIndex tile) noexcept { return GetTileMaxZ(tile); }
#endif
