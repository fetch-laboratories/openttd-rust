/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file script_map.hpp Everything to query and manipulate map metadata. */

#ifndef SCRIPT_MAP_HPP
#define SCRIPT_MAP_HPP

#include "script_object.hpp"
#include "../../tile_type.h"
#include "../../rust/script_tile_ffi.h"

/**
 * Class that handles all map related functions.
 * @api ai game
 */
class ScriptMap : public ScriptObject {
public:
	static const int TILE_INVALID = INVALID_TILE.base(); ///< Invalid TileIndex.

	/**
	 * Checks whether the given tile is valid.
	 * @param tile The tile to check.
	 * @return True is the tile it within the boundaries of the map.
	 */
	static bool IsValidTile(TileIndex tile) { return openttd_rust_script_map_is_valid_tile(&GetRustScriptHostServices(), tile.base()); }

	/**
	 * Gets the number of tiles in the map.
	 * @return The size of the map in tiles.
	 * @post Return value is always positive.
	 */
	static SQInteger GetMapSize() { return openttd_rust_script_map_size(&GetRustScriptHostServices()); }

	/**
	 * Gets the amount of tiles along the SW and NE border.
	 * @return The length along the SW and NE borders.
	 * @post Return value is always positive.
	 */
	static SQInteger GetMapSizeX() { return openttd_rust_script_map_size_x(&GetRustScriptHostServices()); }

	/**
	 * Gets the amount of tiles along the SE and NW border.
	 * @return The length along the SE and NW borders.
	 * @post Return value is always positive.
	 */
	static SQInteger GetMapSizeY() { return openttd_rust_script_map_size_y(&GetRustScriptHostServices()); }

	/**
	 * Gets the place along the SW/NE border (X-value).
	 * @param tile The tile to get the X-value of.
	 * @pre IsValidTile(tile).
	 * @return The X-value.
	 * @post Return value is always lower than GetMapSizeX().
	 */
	static SQInteger GetTileX(TileIndex tile) { return openttd_rust_script_map_tile_x(&GetRustScriptHostServices(), tile.base()); }

	/**
	 * Gets the place along the SE/NW border (Y-value).
	 * @param tile The tile to get the Y-value of.
	 * @pre IsValidTile(tile).
	 * @return The Y-value.
	 * @post Return value is always lower than GetMapSizeY().
	 */
	static SQInteger GetTileY(TileIndex tile) { return openttd_rust_script_map_tile_y(&GetRustScriptHostServices(), tile.base()); }

	/**
	 * Gets the TileIndex given a x,y-coordinate.
	 * @param x The X coordinate.
	 * @param y The Y coordinate.
	 * @return The TileIndex for the given (x,y) coordinate.
	 * @post When 0 <= x && x < GetMapSizeX() && 0 <= y && y < GetMapSizeY(), then a valid tile index is returned.
	 *       Otherwise it may be invalid, but could be used to calculated neighbouring tiles, e.g. tile + AIMap.GetTileIndex(-1, -1) gets
	 *       the tile index of the tile to the north. But be aware that even when tile is a valid tile, the result might not be a valid tile.
	 */
	static TileIndex GetTileIndex(SQInteger x, SQInteger y) { return TileIndex(openttd_rust_script_map_tile_index(&GetRustScriptHostServices(), x, y)); }

	/**
	 * Calculates the Manhattan distance; the difference of
	 *  the X and Y added together.
	 * @param tile_from The start tile.
	 * @param tile_to The destination tile.
	 * @pre IsValidTile(tile_from).
	 * @pre IsValidTile(tile_to).
	 * @return The Manhattan distance between the tiles.
	 */
	static SQInteger DistanceManhattan(TileIndex tile_from, TileIndex tile_to) { return openttd_rust_script_map_distance_manhattan(&GetRustScriptHostServices(), tile_from.base(), tile_to.base()); }

	/**
	 * Calculates the distance between two tiles via 1D calculation.
	 *  This means the distance between X or the distance between Y, depending
	 *  on which one is bigger.
	 * @param tile_from The start tile.
	 * @param tile_to The destination tile.
	 * @pre IsValidTile(tile_from).
	 * @pre IsValidTile(tile_to).
	 * @return The maximum distance between the tiles.
	 */
	static SQInteger DistanceMax(TileIndex tile_from, TileIndex tile_to) { return openttd_rust_script_map_distance_max(&GetRustScriptHostServices(), tile_from.base(), tile_to.base()); }

	/**
	 * The squared distance between the two tiles.
	 *  This is the distance is the length of the shortest straight line
	 *  between both points.
	 * @param tile_from The start tile.
	 * @param tile_to The destination tile.
	 * @pre IsValidTile(tile_from).
	 * @pre IsValidTile(tile_to).
	 * @return The squared distance between the tiles.
	 */
	static SQInteger DistanceSquare(TileIndex tile_from, TileIndex tile_to) { return openttd_rust_script_map_distance_square(&GetRustScriptHostServices(), tile_from.base(), tile_to.base()); }

	/**
	 * Calculates the shortest distance to the edge.
	 * @param tile From where the distance has to be calculated.
	 * @pre IsValidTile(tile).
	 * @return The distances to the closest edge.
	 */
	static SQInteger DistanceFromEdge(TileIndex tile) { return openttd_rust_script_map_distance_from_edge(&GetRustScriptHostServices(), tile.base()); }
};

#endif /* SCRIPT_MAP_HPP */
