/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_viewport.h Canonical map redraw operation. */
#ifndef RUST_SERVICES_VIEWPORT_H
#define RUST_SERVICES_VIEWPORT_H
#include "../viewport_func.h"
#include "../window_func.h"
inline void RustInvalidateDepotWindow(TileIndex tile) noexcept { InvalidateWindowData(WC_VEHICLE_DEPOT, tile); }
inline void RustMarkTileDirty(uint32_t tile) noexcept { MarkTileDirtyByTile(TileIndex{tile}); }
inline void RustMarkTileDirtyHeight(uint32_t tile, int32_t offset) noexcept { MarkTileDirtyByTile(TileIndex{tile}, offset); }
/* MarkTileDirtyByTile(tile, 0, tile_height_override) */
inline void RustMarkTileDirtyAtHeight(uint32_t tile, int32_t height) noexcept { MarkTileDirtyByTile(TileIndex{tile}, 0, height); }
#endif /* RUST_SERVICES_VIEWPORT_H */
