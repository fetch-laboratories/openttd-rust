/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file tilearea.cpp Handling of tile areas. */

#include "stdafx.h"

#include "rust/services_ffi.h"
#include "tilearea_type.h"

#include "safeguards.h"

/**
 * Construct this tile area based on two points.
 * @param start the start of the area
 * @param end   the end of the area
 */
OrthogonalTileArea::OrthogonalTileArea(TileIndex start, TileIndex end)
{
	openttd_rust_tile_area_corners(this, start.base(), end.base(), &GetRustSharedServices());
}

/**
 * Add a single tile to a tile area; enlarge if needed.
 * @param to_add The tile to add
 */
void OrthogonalTileArea::Add(TileIndex to_add)
{
	openttd_rust_tile_area_add(this, to_add.base(), &GetRustSharedServices());
}

/**
 * Does this tile area intersect with another?
 * @param ta the other tile area to check against.
 * @return true if they intersect.
 */
bool OrthogonalTileArea::Intersects(const OrthogonalTileArea &ta) const
{
	return openttd_rust_tile_area_intersects(this, &ta, &GetRustSharedServices());
}

/**
 * Does this tile area contain a tile?
 * @param tile Tile to test for.
 * @return True if the tile is inside the area.
 */
bool OrthogonalTileArea::Contains(TileIndex tile) const
{
	return openttd_rust_tile_area_contains(this, tile.base(), &GetRustSharedServices());
}

/**
 * Expand a tile area by rad tiles in each direction, keeping within map bounds.
 * @param rad Number of tiles to expand
 * @return The OrthogonalTileArea.
 */
OrthogonalTileArea &OrthogonalTileArea::Expand(int rad)
{
	openttd_rust_tile_area_expand(this, rad, &GetRustSharedServices());
	return *this;
}

/**
 * Clamp the tile area to map borders.
 */
void OrthogonalTileArea::ClampToMap()
{
	openttd_rust_tile_area_clamp_to_map(this, &GetRustSharedServices());
}

/**
 * Returns an iterator to the beginning of the tile area.
 * @return The OrthogonalTileIterator.
 */
OrthogonalTileIterator OrthogonalTileArea::begin() const
{
	return OrthogonalTileIterator(*this);
}

/**
 * Returns an iterator to the end of the tile area.
 * @return The OrthogonalTileIterator.
 */
OrthogonalTileIterator OrthogonalTileArea::end() const
{
	return OrthogonalTileIterator(OrthogonalTileArea());
}

/**
 * Create a diagonal tile area from two corners.
 * @param start First corner of the area.
 * @param end Second corner of the area.
 */
DiagonalTileArea::DiagonalTileArea(TileIndex start, TileIndex end)
{
	openttd_rust_diagonal_tile_area_corners(this, start.base(), end.base(), &GetRustSharedServices());
}

/**
 * Does this tile area contain a tile?
 * @param tile Tile to test for.
 * @return True if the tile is inside the area.
 */
bool DiagonalTileArea::Contains(TileIndex tile) const
{
	return openttd_rust_diagonal_tile_area_contains(this, tile.base(), &GetRustSharedServices());
}

/**
 * Move ourselves to the next tile in the rectangle on the map.
 */
TileIterator &DiagonalTileIterator::operator++()
{
	this->tile = TileIndex{openttd_rust_diagonal_tile_advance(&this->state, this->tile.base(), &GetRustSharedServices())};
	return *this;
}

/**
 * Create either an OrthogonalTileIterator or DiagonalTileIterator given the diagonal parameter.
 * @param corner1 Tile from where to begin iterating.
 * @param corner2 Tile where to end the iterating.
 * @param diagonal Whether to create a DiagonalTileIterator or OrthogonalTileIterator.
 * @return unique_ptr to the allocated TileIterator.
 */
/* static */ std::unique_ptr<TileIterator> TileIterator::Create(TileIndex corner1, TileIndex corner2, bool diagonal)
{
	if (diagonal) {
		return std::make_unique<DiagonalTileIterator>(corner1, corner2);
	}
	return std::make_unique<OrthogonalTileIterator>(corner1, corner2);
}

/**
 * See SpiralTileSequence constructor for description.
 */
SpiralTileIterator::SpiralTileIterator(TileIndex center, uint diameter)
{
	assert(diameter > 0);
	this->state = openttd_rust_spiral_square(TileX(center), TileY(center), diameter, Map::SizeX(), Map::SizeY());
}

SpiralTileIterator::SpiralTileIterator(TileIndex start_north, uint radius, uint w, uint h)
{
	assert(radius > 0);
	this->state = openttd_rust_spiral_hole(TileX(start_north), TileY(start_north), radius, w, h, Map::SizeX(), Map::SizeY());
}
