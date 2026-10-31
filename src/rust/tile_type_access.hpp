/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file tile_type_access.hpp Canonical map static addresses and layout; storage remains owned by Tile. */
#ifndef RUST_TILE_TYPE_ACCESS_HPP
#define RUST_TILE_TYPE_ACCESS_HPP
#include "../map_func.h"
#include "services_ffi.h"

/* This friend exposes the addresses of the canonical statics and layout only.
 * No TileBase value is copied to Rust. Rust loads each static at entry; the
 * game-state lock/no-reset proof bounds what it derives from them. */
struct OpenTTDNativeTileTypeAccess {
	using BaseTile = Tile::TileBase;
	using ExtendedTile = Tile::TileExtended;
	static_assert(std::is_standard_layout_v<BaseTile> && std::is_standard_layout_v<ExtendedTile>);
	/* Map::Allocate bounds both dimensions by MAX_MAP_SIZE. Even the largest
	 * allocation is 128 MiB, below Rust isize::MAX on a 32-bit target. */
	static_assert(uint64_t{MAX_MAP_SIZE} * MAX_MAP_SIZE * sizeof(BaseTile) <= INT32_MAX);
	/* Rust loads each array pointer as the unique_ptr's sole representation. */
	static_assert(sizeof(Tile::base_tiles) == sizeof(BaseTile *) && sizeof(Tile::extended_tiles) == sizeof(ExtendedTile *));
	static_assert(std::is_same_v<decltype(Map::size), uint32_t> && std::is_same_v<decltype(Map::log_x), uint32_t>);

	static constexpr OpenTTDMapStatics Statics() noexcept
	{
		return {
			.base = &Tile::base_tiles, .extended = &Tile::extended_tiles, .size = &Map::size, .log_x = &Map::log_x,
#if !defined(NDEBUG) || defined(WITH_ASSERT)
			.assertions = true,
#else
			.assertions = false,
#endif
		};
	}
};
#endif /* RUST_TILE_TYPE_ACCESS_HPP */
