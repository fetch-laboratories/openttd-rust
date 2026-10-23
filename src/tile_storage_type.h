/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file tile_storage_type.h Canonical native map storage declarations. */
#ifndef TILE_STORAGE_TYPE_H
#define TILE_STORAGE_TYPE_H
#include <cstdint>

/**
 * Data that is stored per tile. Also used TileExtended for this.
 * Look at docs/landscape.html for the exact meaning of the members.
 */
struct TileBaseStorage {
	uint8_t type = 0; ///< The type (bits 4..7), bridges (2..3), rainforest/desert (0..1)
	uint8_t height = 0; ///< The height of the northern corner.
	uint16_t m2 = 0; ///< Primarily used for indices to towns, industries and stations
	uint8_t m1 = 0; ///< Primarily used for ownership information
	uint8_t m3 = 0; ///< General purpose
	uint8_t m4 = 0; ///< General purpose
	uint8_t m5 = 0; ///< General purpose
};

/**
 * Data that is stored per tile. Also used TileBase for this.
 * Look at docs/landscape.html for the exact meaning of the members.
 */
struct TileExtendedStorage {
	uint8_t m6 = 0; ///< General purpose
	uint8_t m7 = 0; ///< Primarily used for newgrf support
	uint16_t m8 = 0; ///< General purpose
};
static_assert(sizeof(TileBaseStorage) == 8);
#endif /* TILE_STORAGE_TYPE_H */
