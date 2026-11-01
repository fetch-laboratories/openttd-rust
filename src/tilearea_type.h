/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file tilearea_type.h Type for storing the 'area' of something uses on the map. */

#ifndef TILEAREA_TYPE_H
#define TILEAREA_TYPE_H

#include "map_func.h"

#include "rust/spiral_ffi.h"
#include "rust/tilearea_ffi.h"
static_assert(sizeof(uint) == sizeof(uint32_t) && std::is_unsigned_v<uint> && std::numeric_limits<uint>::digits == 32);
static_assert(sizeof(DiagDirection) == sizeof(uint8_t));
static_assert(DIAGDIR_BEGIN == 0 && DIAGDIR_NE == 0 && DIAGDIR_SE == 1 && DIAGDIR_SW == 2 && DIAGDIR_NW == 3 && DIAGDIR_END == 4 && INVALID_DIAGDIR == 255);

class OrthogonalTileIterator;

/** Represents the covered area of e.g. a rail station */
struct OrthogonalTileArea {
	TileIndex tile; ///< The base tile of the area
	uint16_t w;       ///< The width of the area
	uint16_t h;       ///< The height of the area

	/**
	 * Construct this tile area with some set values
	 * @param tile the base tile
	 * @param w the width
	 * @param h the height
	 */
	OrthogonalTileArea(TileIndex tile = INVALID_TILE, uint16_t w = 0, uint16_t h = 0) : tile(tile), w(w), h(h)
	{
	}

	OrthogonalTileArea(TileIndex start, TileIndex end);

	void Add(TileIndex to_add);

	/**
	 * Clears the 'tile area', i.e. make the tile invalid.
	 */
	void Clear()
	{
		this->tile = INVALID_TILE;
		this->w    = 0;
		this->h    = 0;
	}

	bool Contains(TileIndex tile) const;

	OrthogonalTileArea &Expand(int rad);

	void ClampToMap();

	/**
	 * Get the center tile.
	 * @return The tile at the center, or just north of it.
	 */
	TileIndex GetCenterTile() const
	{
		return TileAddXY(this->tile, this->w / 2, this->h / 2);
	}

	OrthogonalTileIterator begin() const;

	OrthogonalTileIterator end() const;
};

/** Shorthand for the much more common orthogonal tile area. */
typedef OrthogonalTileArea TileArea;

/** Base class for tile iterators. */
class TileIterator {
protected:
	TileIndex tile; ///< The current tile we are at.

	/**
	 * Initialise the iterator starting at this tile.
	 * @param tile The tile we start iterating from.
	 */
	TileIterator(TileIndex tile = INVALID_TILE) : tile(tile)
	{
	}

public:
	virtual ~TileIterator() = default;

	/**
	 * Get the tile we are currently at.
	 * @return The tile we are at, or INVALID_TILE when we're done.
	 */
	inline operator TileIndex () const
	{
		return this->tile;
	}

	/**
	 * Get the tile we are currently at.
	 * @return The tile we are at, or INVALID_TILE when we're done.
	 */
	inline TileIndex operator *() const
	{
		return this->tile;
	}

	/**
	 * Move ourselves to the next tile in the rectangle on the map.
	 */
	virtual TileIterator& operator ++() = 0;

	/**
	 * Allocate a new iterator that is a copy of this one.
	 */
	virtual std::unique_ptr<TileIterator> Clone() const = 0;

	/**
	 * Equality comparison.
	 */
	bool operator ==(const TileIterator &rhs) const
	{
		return this->tile == rhs.tile;
	}

	/**
	 * Equality comparison.
	 */
	bool operator ==(const TileIndex &rhs) const
	{
		return this->tile == rhs;
	}
};

/** Iterator to iterate over a tile area (rectangle) of the map. */
class OrthogonalTileIterator : public TileIterator {
private:
	int w;          ///< The width of the iterated area.
	int x;          ///< The current 'x' position in the rectangle.
	int y;          ///< The current 'y' position in the rectangle.

public:
	/**
	 * Construct the iterator.
	 * @param ta Area, i.e. begin point and width/height of to-be-iterated area.
	 */
	OrthogonalTileIterator(const OrthogonalTileArea &ta) : TileIterator(ta.w == 0 || ta.h == 0 ? INVALID_TILE : ta.tile), w(ta.w), x(ta.w), y(ta.h)
	{
	}

	/**
	 * Move ourselves to the next tile in the rectangle on the map.
	 */
	inline TileIterator& operator ++() override
	{
		assert(this->tile != INVALID_TILE);

		if (--this->x > 0) {
			this->tile++;
		} else if (--this->y > 0) {
			this->x = this->w;
			this->tile += TileDiffXY(1 - this->w, 1);
		} else {
			this->tile = INVALID_TILE;
		}
		return *this;
	}

	std::unique_ptr<TileIterator> Clone() const override
	{
		return std::make_unique<OrthogonalTileIterator>(*this);
	}
};

/**
 * Helper class for SpiralTileSequence.
 */
class SpiralTileIterator {
public:
	using value_type = TileIndex;
	using difference_type = std::ptrdiff_t;
	using iterator_category = std::forward_iterator_tag;
	using pointer = void;
	using reference = void;

	SpiralTileIterator(TileIndex center, uint diameter);
	SpiralTileIterator(TileIndex start_north, uint radius, uint w, uint h);

	bool operator==(const SpiralTileIterator &rhs) const
	{
		return openttd_rust_spiral_equal(this->state, rhs.state) != 0;
	}
	bool operator==(const std::default_sentinel_t &) const { return this->IsEnd(); }

	TileIndex operator*() const
	{
		return TileXY(this->state.x, this->state.y);
	}

	SpiralTileIterator &operator++()
	{
		assert(!this->IsEnd());
		this->state = openttd_rust_spiral_advance(this->state, Map::SizeX(), Map::SizeY());
		return *this;
	}

	SpiralTileIterator operator++(int)
	{
		SpiralTileIterator result = *this;
		++*this;
		return result;
	}

private:
	OpenTTDRustSpiralState state;

	/**
	 * Test whether the iterator reached the end.
	 */
	bool IsEnd() const
	{
		return openttd_rust_spiral_end(this->state) != 0;
	}
};

/**
 * Generate TileIndices around a center tile or tile area, with increasing distance.
 */
class SpiralTileSequence {
public:
	/**
	 * Generate TileIndices for a square area around a center tile.
	 *
	 * The size of the square is given by the length of the edge.
	 * If the size is even, the south extent will be larger than the north extent.
	 *
	 * Example for diameter=4, [ ] is the "center":
	 *        1
	 *      1   1
	 *    1  [0]  1
	 *  1   0   0   1
	 *    1   0   1
	 *      1   1
	 *        1
	 * The sequence starts with the "0" tiles, and continues with the shells around it.
	 *
	 * @param center Center of the square area.
	 * @param diameter Edge length of the square.
	 * @pre diameter > 0
	 * @note This constructor uses a "diameter", unlike the other constructor using a "radius".
	 */
	SpiralTileSequence(TileIndex center, uint diameter) : start(center, diameter) {}

	/**
	 * Generate TileIndices for a rectangular area with an optional rectangular hole in the center.
	 * The TileIndices will be sorted by increasing distance from the center (hole).
	 *
	 * Example for radius=2, w=2, h=1, [ ] is "start_north":
	 *            1
	 *          1   1
	 *        1  [0]  1
	 *      1   0   0   1
	 *    1   0   H   0   1
	 *  1   0   H   0   1
	 *    1   0   0   1
	 *      1   0   1
	 *        1   1
	 *          1
	 * The sequence starts with the "0" tiles, and continues with the shells around it.
	 *
	 * @param start_north Tile directly north from the center hole.
	 * @param radius Radial distance between outer rectangle and center hole.
	 * @param w Width of the inner rectangular hole.
	 * @param h Height of the inner rectangular hole.
	 * @pre radius > 0
	 * @note This constructor uses a "radius", unlike the other constructor using a "diameter".
	 */
	SpiralTileSequence(TileIndex start_north, uint radius, uint w, uint h) : start(start_north, radius, w, h) {}

	SpiralTileIterator begin() const { return start; }
	std::default_sentinel_t end() const { return std::default_sentinel_t(); }

private:
	SpiralTileIterator start;
};

#endif /* TILEAREA_TYPE_H */
