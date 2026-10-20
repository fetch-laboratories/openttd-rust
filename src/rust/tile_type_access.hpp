/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file tile_type_access.hpp Narrow allocation/layout facade; storage remains owned by Tile. */
#ifndef RUST_TILE_TYPE_ACCESS_HPP
#define RUST_TILE_TYPE_ACCESS_HPP
#include "../map_func.h"
#include "services_ffi.h"

/* This friend exposes allocation provenance and layout only. No TileBase value
 * is copied to Rust. The scheduler's game-state lock/no-reset proof bounds the
 * descriptor lifetime; other entries retain the original callback reader. */
struct OpenTTDNativeTileTypeAccess {
	using BaseTile = Tile::TileBase;
	static_assert(std::is_standard_layout_v<BaseTile>);
	static_assert(sizeof(BaseTile) == 8 && alignof(BaseTile) == 2);
	static_assert(offsetof(BaseTile, type) == 0 && offsetof(BaseTile, height) == 1 && offsetof(BaseTile, m2) == 2);
	static_assert(offsetof(BaseTile, m1) == 4 && offsetof(BaseTile, m3) == 5 && offsetof(BaseTile, m4) == 6 && offsetof(BaseTile, m5) == 7);
	/* Map::Allocate bounds both dimensions by MAX_MAP_SIZE. Even the largest
	 * allocation is 128 MiB, below Rust isize::MAX on a 32-bit target. */
	static_assert(uint64_t{MAX_MAP_SIZE} * MAX_MAP_SIZE * sizeof(BaseTile) <= INT32_MAX);

	static OpenTTDTileTypeAllocation Acquire() noexcept
	{
		return {.base = reinterpret_cast<const uint8_t *>(Tile::base_tiles.get()), .count = Map::Size()};
	}
};
#endif /* RUST_TILE_TYPE_ACCESS_HPP */
