/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file landscape.cpp Functions related to the landscape (slopes etc.). */

/** @defgroup SnowLineGroup Snowline functions and data structures */

#include "stdafx.h"
#include "heightmap.h"
#include "clear_map.h"
#include "spritecache.h"
#include "station_map.h"
#include "viewport_func.h"
#include "command_func.h"
#include "landscape.h"
#include "void_map.h"
#include "tgp.h"
#include "genworld.h"
#include "fios.h"
#include "error_func.h"
#include "timer/timer_game_calendar.h"
#include "timer/timer_game_tick.h"
#include "water.h"
#include "effectvehicle_func.h"
#include "landscape_type.h"
#include "animated_tile_func.h"
#include "core/random_func.hpp"
#include "object_base.h"
#include "tree_cmd.h"
#include "company_func.h"
#include "company_gui.h"
#include "saveload/saveload.h"
#include "framerate_type.h"
#include "landscape_cmd.h"
#include "terraform_cmd.h"
#include "station_func.h"
#include "pathfinder/water_regions.h"
#include "pathfinder/yapf/yapf_river_builder.h"

#include "table/strings.h"
#include "table/sprites.h"

#include <unordered_set>

#include "rust/ffi.h"
#include "rust/snow_line_ffi.h"
#include "rust/landscape_loop_ffi.h"
#include "rust/station_lifecycle_ffi.h"
#include "rust/services_object.h"
#include "rust/tile_owner_services.h"
#include "rust/road_command_services.h"
#include "rust/ground_services.h"
#include "rust/environment_services.h"
#include "rust/services_landscape.h"
#include "rust/town_ffi.h"

#include "rust/landscape_command_ffi.h"
#include "rust/settings_services.h"
#include "tunnel_map.h"
#include "rust/water_area_services.h"
#include "rust/town_founding_ffi.h"
#include "rust/command_services.h"
#include "rust/environment_services.h"

#include "rust/movement_ffi.h"
#include "rust/services_map.h"
#include "rust/services_movement.h"
#include "rust/train_ffi.h"
#include "rust/services_station.h"

#include "safeguards.h"

extern const TileTypeProcs
	_tile_type_clear_procs,
	_tile_type_rail_procs,
	_tile_type_road_procs,
	_tile_type_town_procs,
	_tile_type_trees_procs,
	_tile_type_station_procs,
	_tile_type_water_procs,
	_tile_type_void_procs,
	_tile_type_industry_procs,
	_tile_type_tunnelbridge_procs,
	_tile_type_object_procs;

/**
 * Tile callback functions for each type of tile.
 * @ingroup TileCallbackGroup
 * @see TileType
 */
const TileTypeProcs * const _tile_type_procs[16] = {
	&_tile_type_clear_procs,        ///< Callback functions for MP_CLEAR tiles
	&_tile_type_rail_procs,         ///< Callback functions for MP_RAILWAY tiles
	&_tile_type_road_procs,         ///< Callback functions for MP_ROAD tiles
	&_tile_type_town_procs,         ///< Callback functions for MP_HOUSE tiles
	&_tile_type_trees_procs,        ///< Callback functions for MP_TREES tiles
	&_tile_type_station_procs,      ///< Callback functions for MP_STATION tiles
	&_tile_type_water_procs,        ///< Callback functions for MP_WATER tiles
	&_tile_type_void_procs,         ///< Callback functions for MP_VOID tiles
	&_tile_type_industry_procs,     ///< Callback functions for MP_INDUSTRY tiles
	&_tile_type_tunnelbridge_procs, ///< Callback functions for MP_TUNNELBRIDGE tiles
	&_tile_type_object_procs,       ///< Callback functions for MP_OBJECT tiles
};

/** landscape slope => sprite */
extern const uint8_t _slope_to_sprite_offset[32] = {
	0, 1, 2, 3, 4, 5, 6,  7, 8, 9, 10, 11, 12, 13, 14, 0,
	0, 0, 0, 0, 0, 0, 0, 16, 0, 0,  0, 17,  0, 15, 18, 0,
};

static const uint TILE_UPDATE_FREQUENCY_LOG = 8;  ///< The logarithm of how many ticks it takes between tile updates (log base 2).
static const uint TILE_UPDATE_FREQUENCY = 1 << TILE_UPDATE_FREQUENCY_LOG;  ///< How many ticks it takes between tile updates (has to be a power of 2).

/**
 * Description of the snow line throughout the year.
 *
 * If it is \c nullptr, a static snowline height is used, as set by \c _settings_game.game_creation.snow_line_height.
 * Otherwise it points to a table loaded from a newGRF file that describes the variable snowline.
 * @ingroup SnowLineGroup
 * @see GetSnowLine() GameCreationSettings
 */
/* Preserve the native static owner's shutdown point. */
static struct SnowLineCleanup {
	~SnowLineCleanup() { ClearSnowLine(); }
} _snow_line_cleanup;

/**
 * Map 2D viewport or smallmap coordinate to 3D world or tile coordinate.
 * Function takes into account height of tiles and foundations.
 *
 * @param x X viewport 2D coordinate.
 * @param y Y viewport 2D coordinate.
 * @param clamp_to_map Clamp the coordinate outside of the map to the closest, non-void tile within the map.
 * @param[out] clamped Whether coordinates were clamped.
 * @return 3D world coordinate of point visible at the given screen coordinate (3D perspective).
 *
 * @note Inverse of #RemapCoords2 function. Smaller values may get rounded.
 * @see InverseRemapCoords
 */
Point InverseRemapCoords2(int x, int y, bool clamp_to_map, bool *clamped)
{
	if (clamped != nullptr) *clamped = false; // Not clamping yet.

	/* Initial x/y world coordinate is like if the landscape
	 * was completely flat on height 0. */
	Point pt = InverseRemapCoords(x, y);

	const uint min_coord = _settings_game.construction.freeform_edges ? TILE_SIZE : 0;
	const uint max_x = Map::MaxX() * TILE_SIZE - 1;
	const uint max_y = Map::MaxY() * TILE_SIZE - 1;

	if (clamp_to_map) {
		/* Bring the coordinates near to a valid range. At the top we allow a number
		 * of extra tiles. This is mostly due to the tiles on the north side of
		 * the map possibly being drawn higher due to the extra height levels. */
		int extra_tiles = CeilDiv(_settings_game.construction.map_height_limit * TILE_HEIGHT, TILE_PIXELS);
		Point old_pt = pt;
		pt.x = Clamp(pt.x, -extra_tiles * TILE_SIZE, max_x);
		pt.y = Clamp(pt.y, -extra_tiles * TILE_SIZE, max_y);
		if (clamped != nullptr) *clamped = (pt.x != old_pt.x) || (pt.y != old_pt.y);
	}

	/* Now find the Z-world coordinate by fix point iteration.
	 * This is a bit tricky because the tile height is non-continuous at foundations.
	 * The clicked point should be approached from the back, otherwise there are regions that are not clickable.
	 * (FOUNDATION_HALFTILE_LOWER on SLOPE_STEEP_S hides north halftile completely)
	 * So give it a z-malus of 4 in the first iterations. */
	int z = 0;
	if (clamp_to_map) {
		for (int i = 0; i < 5; i++) z = GetSlopePixelZ(Clamp(pt.x + std::max(z, 4) - 4, min_coord, max_x), Clamp(pt.y + std::max(z, 4) - 4, min_coord, max_y)) / 2;
		for (int m = 3; m > 0; m--) z = GetSlopePixelZ(Clamp(pt.x + std::max(z, m) - m, min_coord, max_x), Clamp(pt.y + std::max(z, m) - m, min_coord, max_y)) / 2;
		for (int i = 0; i < 5; i++) z = GetSlopePixelZ(Clamp(pt.x + z,             min_coord, max_x), Clamp(pt.y + z,             min_coord, max_y)) / 2;
	} else {
		for (int i = 0; i < 5; i++) z = GetSlopePixelZOutsideMap(pt.x + std::max(z, 4) - 4, pt.y + std::max(z, 4) - 4) / 2;
		for (int m = 3; m > 0; m--) z = GetSlopePixelZOutsideMap(pt.x + std::max(z, m) - m, pt.y + std::max(z, m) - m) / 2;
		for (int i = 0; i < 5; i++) z = GetSlopePixelZOutsideMap(pt.x + z,             pt.y + z            ) / 2;
	}

	pt.x += z;
	pt.y += z;
	if (clamp_to_map) {
		Point old_pt = pt;
		pt.x = Clamp(pt.x, min_coord, max_x);
		pt.y = Clamp(pt.y, min_coord, max_y);
		if (clamped != nullptr) *clamped = *clamped || (pt.x != old_pt.x) || (pt.y != old_pt.y);
	}

	return pt;
}

/**
 * Applies a foundation to a slope.
 *
 * @pre      Foundation and slope must be valid combined.
 * @param f  The #Foundation.
 * @param s  The #Slope to modify.
 * @return   Increment to the tile Z coordinate.
 */
uint ApplyFoundationToSlope(Foundation f, Slope &s)
{
	if (!IsFoundation(f)) return 0;

	if (IsLeveledFoundation(f)) {
		uint dz = 1 + (IsSteepSlope(s) ? 1 : 0);
		s = SLOPE_FLAT;
		return dz;
	}

	if (f != FOUNDATION_STEEP_BOTH && IsNonContinuousFoundation(f)) {
		s = HalftileSlope(s, GetHalftileFoundationCorner(f));
		return 0;
	}

	if (IsSpecialRailFoundation(f)) {
		s = SlopeWithThreeCornersRaised(OppositeCorner(GetRailFoundationCorner(f)));
		return 0;
	}

	uint dz = IsSteepSlope(s) ? 1 : 0;
	Corner highest_corner = GetHighestSlopeCorner(s);

	switch (f) {
		case FOUNDATION_INCLINED_X:
			s = (((highest_corner == CORNER_W) || (highest_corner == CORNER_S)) ? SLOPE_SW : SLOPE_NE);
			break;

		case FOUNDATION_INCLINED_Y:
			s = (((highest_corner == CORNER_S) || (highest_corner == CORNER_E)) ? SLOPE_SE : SLOPE_NW);
			break;

		case FOUNDATION_STEEP_LOWER:
			s = SlopeWithOneCornerRaised(highest_corner);
			break;

		case FOUNDATION_STEEP_BOTH:
			s = HalftileSlope(SlopeWithOneCornerRaised(highest_corner), highest_corner);
			break;

		default: NOT_REACHED();
	}
	return dz;
}


/**
 * Determines height at given coordinate of a slope.
 *
 * At the northern corner (0, 0) the result is always a multiple of TILE_HEIGHT.
 * When the height is a fractional Z, then the height is rounded down. For example,
 * when at the height is 0 at x = 0 and the height is 8 at x = 16 (actually x = 0
 * of the next tile), then height is 0 at x = 1, 1 at x = 2, and 7 at x = 15.
 * @param x x coordinate (value from 0 to 15)
 * @param y y coordinate (value from 0 to 15)
 * @param corners slope to examine
 * @return height of given point of given slope
 */
uint GetPartialPixelZ(int x, int y, Slope corners)
{
	static_assert(sizeof(int) == sizeof(int32_t) && sizeof(uint) == sizeof(uint32_t));
	static_assert(sizeof(Slope) == sizeof(uint8_t));
	static_assert(TILE_SIZE == 16 && TILE_HEIGHT == 8);
	static_assert(CORNER_W == 0 && CORNER_S == 1 && CORNER_E == 2 && CORNER_N == 3);
	static_assert(SLOPE_FLAT == 0x00 && SLOPE_W == 0x01 && SLOPE_S == 0x02 && SLOPE_E == 0x04 && SLOPE_N == 0x08);
	static_assert(SLOPE_NW == 0x09 && SLOPE_SW == 0x03 && SLOPE_SE == 0x06 && SLOPE_NE == 0x0C);
	static_assert(SLOPE_EW == 0x05 && SLOPE_NS == 0x0A && SLOPE_ELEVATED == 0x0F);
	static_assert(SLOPE_NWS == 0x0B && SLOPE_WSE == 0x07 && SLOPE_SEN == 0x0E && SLOPE_ENW == 0x0D);
	static_assert(SLOPE_STEEP == 0x10 && SLOPE_HALFTILE == 0x20 && SLOPE_HALFTILE_MASK == 0xE0);
	static_assert(SLOPE_STEEP_W == 0x1B && SLOPE_STEEP_S == 0x17 && SLOPE_STEEP_E == 0x1E && SLOPE_STEEP_N == 0x1D);
	uint64_t result = openttd_rust_get_partial_pixel_z(x, y, static_cast<uint8_t>(corners));
	if (result == UINT64_MAX) NOT_REACHED();
	return static_cast<uint>(result);
}

/**
 * Return world \c Z coordinate of a given point of a tile. Normally this is the
 * Z of the ground/foundation at the given location, but in some cases the
 * ground/foundation can differ from the Z coordinate that the (ground) vehicle
 * passing over it would take. For example when entering a tunnel or bridge.
 *
 * @param x World X coordinate in tile "units".
 * @param y World Y coordinate in tile "units".
 * @param ground_vehicle Whether to get the Z coordinate of the ground vehicle, or the ground.
 * @return World Z coordinate at tile ground (vehicle) level, including slopes and foundations.
 */
int GetSlopePixelZ(int x, int y, bool ground_vehicle)
{
	static const OpenTTDMovementServices * const movement = &GetRustMovementServices();
	return openttd_rust_movement_pixel_z(movement, x, y, ground_vehicle);
}

/**
 * Return world \c z coordinate of a given point of a tile,
 * also for tiles outside the map (virtual "black" tiles).
 *
 * @param x World X coordinate in tile "units", may be outside the map.
 * @param y World Y coordinate in tile "units", may be outside the map.
 * @return World Z coordinate at tile ground level, including slopes and foundations.
 */
int GetSlopePixelZOutsideMap(int x, int y)
{
	return openttd_rust_movement_outside_z(&GetRustMovementServices(), x, y);
}

/**
 * Determine the Z height of a corner relative to TileZ.
 *
 * @pre The slope must not be a halftile slope.
 *
 * @param tileh The slope.
 * @param corner The corner.
 * @return Z position of corner relative to TileZ.
 */
int GetSlopeZInCorner(Slope tileh, Corner corner)
{
	assert(!IsHalftileSlope(tileh));
	return ((tileh & SlopeWithOneCornerRaised(corner)) != 0 ? 1 : 0) + (tileh == SteepSlope(corner) ? 1 : 0);
}

/**
 * Determine the Z height of the corners of a specific tile edge
 *
 * @note If a tile has a non-continuous halftile foundation, a corner can have different heights wrt. its edges.
 *
 * @pre z1 and z2 must be initialized (typ. with TileZ). The corner heights just get added.
 *
 * @param tileh The slope of the tile.
 * @param edge The edge of interest.
 * @param z1 Gets incremented by the height of the first corner of the edge. (near corner wrt. the camera)
 * @param z2 Gets incremented by the height of the second corner of the edge. (far corner wrt. the camera)
 */
void GetSlopePixelZOnEdge(Slope tileh, DiagDirection edge, int &z1, int &z2)
{
	static const Slope corners[4][4] = {
		/*    corner     |          steep slope
		 *  z1      z2   |       z1             z2        */
		{SLOPE_E, SLOPE_N, SLOPE_STEEP_E, SLOPE_STEEP_N}, // DIAGDIR_NE, z1 = E, z2 = N
		{SLOPE_S, SLOPE_E, SLOPE_STEEP_S, SLOPE_STEEP_E}, // DIAGDIR_SE, z1 = S, z2 = E
		{SLOPE_S, SLOPE_W, SLOPE_STEEP_S, SLOPE_STEEP_W}, // DIAGDIR_SW, z1 = S, z2 = W
		{SLOPE_W, SLOPE_N, SLOPE_STEEP_W, SLOPE_STEEP_N}, // DIAGDIR_NW, z1 = W, z2 = N
	};

	int halftile_test = (IsHalftileSlope(tileh) ? SlopeWithOneCornerRaised(GetHalftileSlopeCorner(tileh)) : 0);
	if (halftile_test == corners[edge][0]) z2 += TILE_HEIGHT; // The slope is non-continuous in z2. z2 is on the upper side.
	if (halftile_test == corners[edge][1]) z1 += TILE_HEIGHT; // The slope is non-continuous in z1. z1 is on the upper side.

	if ((tileh & corners[edge][0]) != 0) z1 += TILE_HEIGHT; // z1 is raised
	if ((tileh & corners[edge][1]) != 0) z2 += TILE_HEIGHT; // z2 is raised
	if (RemoveHalftileSlope(tileh) == corners[edge][2]) z1 += TILE_HEIGHT; // z1 is highest corner of a steep slope
	if (RemoveHalftileSlope(tileh) == corners[edge][3]) z2 += TILE_HEIGHT; // z2 is highest corner of a steep slope
}

/**
 * Get slope of a tile on top of a (possible) foundation
 * If a tile does not have a foundation, the function returns the same as GetTileSlope.
 *
 * @param tile The tile of interest.
 * @return The slope on top of the foundation and the z of the foundation slope.
 */
std::tuple<Slope, int> GetFoundationSlope(TileIndex tile)
{
	auto result = openttd_rust_movement_foundation_slope(&GetRustMovementServices(), tile.base());
	return {result.slope, result.z};
}


bool HasFoundationNW(TileIndex tile, Slope slope_here, uint z_here)
{
	int z_W_here = z_here;
	int z_N_here = z_here;
	GetSlopePixelZOnEdge(slope_here, DIAGDIR_NW, z_W_here, z_N_here);

	auto [slope, z] = GetFoundationPixelSlope(TileAddXY(tile, 0, -1));
	int z_W = z;
	int z_N = z;
	GetSlopePixelZOnEdge(slope, DIAGDIR_SE, z_W, z_N);

	return (z_N_here > z_N) || (z_W_here > z_W);
}


bool HasFoundationNE(TileIndex tile, Slope slope_here, uint z_here)
{
	int z_E_here = z_here;
	int z_N_here = z_here;
	GetSlopePixelZOnEdge(slope_here, DIAGDIR_NE, z_E_here, z_N_here);

	auto [slope, z] = GetFoundationPixelSlope(TileAddXY(tile, -1, 0));
	int z_E = z;
	int z_N = z;
	GetSlopePixelZOnEdge(slope, DIAGDIR_SW, z_E, z_N);

	return (z_N_here > z_N) || (z_E_here > z_E);
}

/**
 * Draw foundation \a f at tile \a ti. Updates \a ti.
 * @param ti Tile to draw foundation on
 * @param f  Foundation to draw
 */
void DrawFoundation(TileInfo *ti, Foundation f)
{
	if (!IsFoundation(f)) return;

	/* Two part foundations must be drawn separately */
	assert(f != FOUNDATION_STEEP_BOTH);

	uint sprite_block = 0;
	auto [slope, z] = GetFoundationPixelSlope(ti->tile);

	/* Select the needed block of foundations sprites
	 * Block 0: Walls at NW and NE edge
	 * Block 1: Wall  at        NE edge
	 * Block 2: Wall  at NW        edge
	 * Block 3: No walls at NW or NE edge
	 */
	if (!HasFoundationNW(ti->tile, slope, z)) sprite_block += 1;
	if (!HasFoundationNE(ti->tile, slope, z)) sprite_block += 2;

	/* Use the original slope sprites if NW and NE borders should be visible */
	SpriteID leveled_base = (sprite_block == 0 ? (int)SPR_FOUNDATION_BASE : (SPR_SLOPES_VIRTUAL_BASE + sprite_block * TRKFOUND_BLOCK_SIZE));
	SpriteID inclined_base = SPR_SLOPES_VIRTUAL_BASE + SLOPES_INCLINED_OFFSET + sprite_block * TRKFOUND_BLOCK_SIZE;
	SpriteID halftile_base = SPR_HALFTILE_FOUNDATION_BASE + sprite_block * HALFTILE_BLOCK_SIZE;

	if (IsSteepSlope(ti->tileh)) {
		if (!IsNonContinuousFoundation(f)) {
			/* Lower part of foundation */
			static constexpr SpriteBounds bounds{{}, {TILE_SIZE, TILE_SIZE, TILE_HEIGHT - 1}, {}};
			AddSortableSpriteToDraw(leveled_base + (ti->tileh & ~SLOPE_STEEP), PAL_NONE, *ti, bounds);
		}

		Corner highest_corner = GetHighestSlopeCorner(ti->tileh);
		ti->z += ApplyPixelFoundationToSlope(f, ti->tileh);

		if (IsInclinedFoundation(f)) {
			/* inclined foundation */
			uint8_t inclined = highest_corner * 2 + (f == FOUNDATION_INCLINED_Y ? 1 : 0);

			SpriteBounds bounds{{}, {1, 1, TILE_HEIGHT}, {}};
			if (f == FOUNDATION_INCLINED_X) bounds.extent.x = TILE_SIZE;
			if (f == FOUNDATION_INCLINED_Y) bounds.extent.y = TILE_SIZE;
			AddSortableSpriteToDraw(inclined_base + inclined, PAL_NONE, *ti, bounds);
			OffsetGroundSprite(0, 0);
		} else if (IsLeveledFoundation(f)) {
			static constexpr SpriteBounds bounds{{0, 0, -(int)TILE_HEIGHT}, {TILE_SIZE, TILE_SIZE, TILE_HEIGHT - 1}, {}};
			AddSortableSpriteToDraw(leveled_base + SlopeWithOneCornerRaised(highest_corner), PAL_NONE, *ti, bounds);
			OffsetGroundSprite(0, -(int)TILE_HEIGHT);
		} else if (f == FOUNDATION_STEEP_LOWER) {
			/* one corner raised */
			OffsetGroundSprite(0, -(int)TILE_HEIGHT);
		} else {
			/* halftile foundation */
			int8_t x_bb = (((highest_corner == CORNER_W) || (highest_corner == CORNER_S)) ? TILE_SIZE / 2 : 0);
			int8_t y_bb = (((highest_corner == CORNER_S) || (highest_corner == CORNER_E)) ? TILE_SIZE / 2 : 0);

			SpriteBounds bounds{{x_bb, y_bb, TILE_HEIGHT}, {TILE_SIZE / 2, TILE_SIZE / 2, TILE_HEIGHT - 1}, {}};
			AddSortableSpriteToDraw(halftile_base + highest_corner, PAL_NONE, *ti, bounds);
			/* Reposition ground sprite back to original position after bounding box change above. This is similar to
			 * RemapCoords() but without zoom scaling. */
			Point pt = {(y_bb - x_bb) * 2, y_bb + x_bb};
			OffsetGroundSprite(-pt.x, -pt.y);
		}
	} else {
		if (IsLeveledFoundation(f)) {
			/* leveled foundation */
			static constexpr SpriteBounds bounds{{}, {TILE_SIZE, TILE_SIZE, TILE_HEIGHT - 1}, {}};
			AddSortableSpriteToDraw(leveled_base + ti->tileh, PAL_NONE, *ti, bounds);
			OffsetGroundSprite(0, -(int)TILE_HEIGHT);
		} else if (IsNonContinuousFoundation(f)) {
			/* halftile foundation */
			Corner halftile_corner = GetHalftileFoundationCorner(f);
			int8_t x_bb = (((halftile_corner == CORNER_W) || (halftile_corner == CORNER_S)) ? TILE_SIZE / 2 : 0);
			int8_t y_bb = (((halftile_corner == CORNER_S) || (halftile_corner == CORNER_E)) ? TILE_SIZE / 2 : 0);

			SpriteBounds bounds{{x_bb, y_bb, 0}, {TILE_SIZE / 2, TILE_SIZE / 2, TILE_HEIGHT - 1}, {}};
			AddSortableSpriteToDraw(halftile_base + halftile_corner, PAL_NONE, *ti, bounds);
			/* Reposition ground sprite back to original position after bounding box change above. This is similar to
			 * RemapCoords() but without zoom scaling. */
			Point pt = {(y_bb - x_bb) * 2, y_bb + x_bb};
			OffsetGroundSprite(-pt.x, -pt.y);
		} else if (IsSpecialRailFoundation(f)) {
			/* anti-zig-zag foundation */
			SpriteID spr;
			if (ti->tileh == SLOPE_NS || ti->tileh == SLOPE_EW) {
				/* half of leveled foundation under track corner */
				spr = leveled_base + SlopeWithThreeCornersRaised(GetRailFoundationCorner(f));
			} else {
				/* tile-slope = sloped along X/Y, foundation-slope = three corners raised */
				spr = inclined_base + 2 * GetRailFoundationCorner(f) + ((ti->tileh == SLOPE_SW || ti->tileh == SLOPE_NE) ? 1 : 0);
			}
			static constexpr SpriteBounds bounds{{}, {TILE_SIZE, TILE_SIZE, TILE_HEIGHT - 1}, {}};
			AddSortableSpriteToDraw(spr, PAL_NONE, *ti, bounds);
			OffsetGroundSprite(0, 0);
		} else {
			/* inclined foundation */
			uint8_t inclined = GetHighestSlopeCorner(ti->tileh) * 2 + (f == FOUNDATION_INCLINED_Y ? 1 : 0);

			SpriteBounds bounds{{}, {1, 1, TILE_HEIGHT}, {}};
			if (f == FOUNDATION_INCLINED_X) bounds.extent.x = TILE_SIZE;
			if (f == FOUNDATION_INCLINED_Y) bounds.extent.y = TILE_SIZE;
			AddSortableSpriteToDraw(inclined_base + inclined, PAL_NONE, *ti, bounds);
			OffsetGroundSprite(0, 0);
		}
		ti->z += ApplyPixelFoundationToSlope(f, ti->tileh);
	}
}

void DoClearSquare(TileIndex tile)
{
	openttd_rust_clear_square(tile.base(), &GetRustSharedServices());
}

/**
 * Returns information about trackdirs and signal states.
 * If there is any trackbit at 'side', return all trackdirbits.
 * For TRANSPORT_ROAD, return no trackbits if there is no roadbit (of given subtype) at given side.
 * @param tile tile to get info about
 * @param mode transport type
 * @param sub_mode for TRANSPORT_ROAD, roadtypes to check
 * @param side side we are entering from, INVALID_DIAGDIR to return all trackbits
 * @return trackdirbits and other info depending on 'mode'
 */
TrackStatus GetTileTrackStatus(TileIndex tile, TransportType mode, uint sub_mode, DiagDirection side)
{
	static const OpenTTDMovementServices * const movement = &GetRustMovementServices();
	return openttd_rust_movement_track_status(movement, tile.base(), mode, sub_mode, side);
}

const OpenTTDTileOwnerServices &GetRustTileOwnerServices() noexcept
{
	static const OpenTTDTileOwnerServices services{
		.clear = [](uint32_t tile, uint8_t old, uint8_t next) noexcept { _tile_type_clear_procs.change_tile_owner_proc(TileIndex(tile), Owner(old), Owner(next)); },
		.house = [](uint32_t tile, uint8_t old, uint8_t next) noexcept { _tile_type_town_procs.change_tile_owner_proc(TileIndex(tile), Owner(old), Owner(next)); },
		.industry = &GetIndustryTileServices(),
		.objects = &GetRustObjectServices(),
		.rail = &GetRustRailCommandServices(),
		.shared = &GetRustSharedServices(),
		.station = &GetRustStationCommandServices(),
		.trees = [](uint32_t tile, uint8_t old, uint8_t next) noexcept { _tile_type_trees_procs.change_tile_owner_proc(TileIndex(tile), Owner(old), Owner(next)); },
		.tunnel_bridge = &GetRustTunnelBridgeServices(),
		.void_tile = [](uint32_t tile, uint8_t old, uint8_t next) noexcept { _tile_type_void_procs.change_tile_owner_proc(TileIndex(tile), Owner(old), Owner(next)); },
		.water = &GetRustWaterCommandServices(),
	};
	return services;
}

/**
 * Change the owner of a tile
 * @param tile      Tile to change
 * @param old_owner Current owner of the tile
 * @param new_owner New owner of the tile
 */
void ChangeTileOwner(TileIndex tile, Owner old_owner, Owner new_owner)
{
	RustTileOwnerContext ownership;
	openttd_rust_tile_change_owner(&ownership.view, tile.base(), old_owner.base(), new_owner.base());
}

void GetTileDesc(TileIndex tile, TileDesc &td)
{
	_tile_type_procs[GetTileType(tile)]->get_tile_desc_proc(tile, td);
}

/**
 * Has a snow line table already been loaded.
 * @return true if the table has been loaded already.
 * @ingroup SnowLineGroup
 */
bool IsSnowLineSet()
{
	return openttd_rust_snow_line_is_set();
}

/**
 * Set a variable snow line, as loaded from a newgrf file.
 * @param snow_line The new snow line configuration.
 * @ingroup SnowLineGroup
 */
void SetSnowLine(std::unique_ptr<SnowLine> &&snow_line)
{
	openttd_rust_snow_line_set(&GetRustSharedServices().snow_line_support, snow_line.release());
}

/**
 * Get the current snow line, either variable or static.
 * @return the snow line height.
 * @ingroup SnowLineGroup
 */
uint8_t GetSnowLine()
{
	return openttd_rust_snow_line_get(&GetRustSharedServices());
}

/**
 * Get the highest possible snow line height, either variable or static.
 * @return the highest snow line height.
 * @ingroup SnowLineGroup
 */

/**
 * Get the lowest possible snow line height, either variable or static.
 * @return the lowest snow line height.
 * @ingroup SnowLineGroup
 */
uint8_t LowestSnowLine()
{
	return openttd_rust_snow_line_lowest(&GetRustSharedServices());
}

/**
 * Clear the variable snow line table and free the memory.
 * @ingroup SnowLineGroup
 */
void ClearSnowLine()
{
	openttd_rust_snow_line_clear(&GetRustSharedServices());
}

/**
 * Check if all tiles on the map edge should be considered water borders.
 * @return true If the edge of the map is flat and height 0, allowing for infinite water borders.
 */

/**
 * Clear a piece of landscape
 * @param flags of operation to conduct
 * @param tile tile to clear
 * @return the cost of this operation or an error
 */
CommandCost CmdLandscapeClear(DoCommandFlags flags, TileIndex tile)
{
	CommandCost result;
	openttd_rust_landscape_clear(&GetRustLandscapeCommandServices(), RustCommandCost(&result), tile.base(), flags.base());
	return result;
}

/**
 * Clear a big piece of landscape
 * @param flags of operation to conduct
 * @param tile end tile of area dragging
 * @param start_tile start tile of area dragging
 * @param diagonal Whether to use the Orthogonal (false) or Diagonal (true) iterator.
 * @return the cost of this operation or an error
 */
std::tuple<CommandCost, Money> CmdClearArea(DoCommandFlags flags, TileIndex tile, TileIndex start_tile, bool diagonal)
{
	CommandCost result, last, ret;
	OpenTTDWaterArea area;
	Money extra(openttd_rust_landscape_clear_area(&GetRustLandscapeCommandServices(), RustCommandCost(&result), RustCommandCost(&last), RustCommandCost(&ret), tile.base(), start_tile.base(), diagonal, flags.base(), &area));
	return {std::move(result), extra};
}


const OpenTTDLandscapeCommandServices &GetRustLandscapeCommandServices() noexcept
{
	static_assert(to_underlying(WaterClass::Canal) == 1);
	static const OpenTTDLandscapeTileServices tiles{
		.clear = [](OpenTTDCommandCost *out, uint32_t tile, uint16_t flags) noexcept {
			*reinterpret_cast<CommandCost *>(out) = _tile_type_procs[GetTileType(TileIndex{tile})]->clear_tile_proc(TileIndex{tile}, DoCommandFlags{flags});
		},
		.terraform = [](OpenTTDCommandCost *out, uint32_t tile, uint16_t flags, int32_t z, uint8_t slope) noexcept {
			*reinterpret_cast<CommandCost *>(out) = _tile_type_procs[GetTileType(TileIndex{tile})]->terraform_tile_proc(TileIndex{tile}, DoCommandFlags{flags}, z, static_cast<Slope>(slope));
		},
		.owners = &GetRustTileOwnerServices(), .trees = &GetRustTreeServices(),
		.industry_policy = &GetIndustryTilePolicyServices(), .industry_costs = &GetIndustryCostServices(), .industry_generation = &GetIndustryGenerationServices(),
	};
	static const OpenTTDLandscapeCommandServices services{
		.shared = &GetRustSharedServices(), .costs = &GetRustCommandCostServices(), .commands = &GetRustCommandServices(),
		.strings = {STR_ERROR_MUST_DEMOLISH_CANAL_FIRST, STR_ERROR_CLEARING_LIMIT_REACHED, STR_ERROR_CAN_T_BUILD_ON_WATER, STR_ERROR_ALREADY_AT_SEA_LEVEL, STR_ERROR_TOO_HIGH, STR_ERROR_TOO_CLOSE_TO_EDGE_OF_MAP, STR_ERROR_MUST_DEMOLISH_BRIDGE_FIRST, STR_ERROR_BRIDGE_TOO_HIGH_AFTER_LOWER_LAND, STR_ERROR_EXCAVATION_WOULD_DAMAGE, STR_ERROR_TERRAFORM_LIMIT_REACHED, STR_ERROR_ALREADY_LEVELLED},
		.tile_procs = &tiles, .water = &GetRustWaterCommandServices(),
		.on_water = RustOnWater,
		.coast = [](uint32_t t) noexcept { return IsCoastTile(TileIndex{t}); },
		.buoy = [](uint32_t t) noexcept { return IsBuoyTile(TileIndex{t}); },
		.canal = RustIsCanal,
		.height_limit = RustMapHeightLimit,
		.max_bridge_height = RustMaxBridgeHeight,
		.freeform_edges = OpenTTDFreeformEdges, .editor = OpenTTDEditor,
		.exchange_generating = GetRustTownFoundingServices().exchange_generating,
		.tunnel_in_way = [](uint32_t t, int32_t z) noexcept { return IsTunnelInWay(TileIndex{t}, z); },
		.mark_dirty_height = RustMarkTileDirtyAtHeight,
		.set_height = RustSetTileHeight,
		.paused = []() noexcept { return _pause_mode.Any(); },
	};
	return services;
}


OpenTTDLandscapeLoopServices RustLandscapeLoopServices(OpenTTDGroundCall *call, const OpenTTDRoadCommandContext *road) noexcept
{
	static_assert(MIN_MAP_SIZE_BITS == 6 && MAX_MAP_SIZE_BITS == 12 && TILE_UPDATE_FREQUENCY_LOG == 8);
	static_assert(MP_CLEAR == 0 && MP_RAILWAY == 1 && MP_ROAD == 2 && MP_HOUSE == 3 && MP_TREES == 4 && MP_STATION == 5 && MP_WATER == 6 && MP_VOID == 7 && MP_INDUSTRY == 8 && MP_TUNNELBRIDGE == 9 && MP_OBJECT == 10);
	return {
		.shared = &GetRustSharedServices(),
		.trees = &GetRustTreeServices(),
		.industry = GetRustIndustryLoopServices(),
		.ground = {.clear = &GetRustClearServices(), .rail = &GetRustRailGroundServices(), .road = &GetRustRoadGroundServices(), .road_command = road, .tunnel = &GetRustTunnelGroundServices(), .water = &GetRustWaterGroundServices(), .costs = &GetRustGroundCostServices(), .call = call, .climate = OpenTTDLandscape(), .editor = OpenTTDEditor(), .rebuild = _settings_game.economy.mod_road_rebuild},
		.town = &GetRustTownServices(),
		.station = &GetRustStationLifecycleServices(),
		.objects = &GetRustObjectServices(),
	};
}

/**
 * Gradually iterate over all tiles on the map, calling their TileLoopProcs once every TILE_UPDATE_FREQUENCY ticks.
 */

#include "rust/landscape_generation_ffi.h"
#include "void_map.h"
#include "rust/presentation_services.h"

#include "rust/generation_services.hpp"
#include "rust/generation_sea_set.hpp"
#include "rust/services_bridge.h"
#include "rust/services_yapf.h"
#include "rust/services_assert.h"
#include "rust/heightmap_services.hpp"
#include "rust/tgp_ffi.h"

#include "rust/landscape_generation_frame.hpp"

const OpenTTDLandscapeGenerationServices &GetRustLandscapeGenerationServices() noexcept
{
	static const OpenTTDLandscapeGenerationServices services{
		.shared = &GetRustSharedServices(),
		.clear = &GetRustClearServices(),
		.water = &GetRustWaterGroundServices(),
		.trees = &GetRustTreeServices(),
		.landscape_commands = &GetRustLandscapeCommandServices(),
		.water_commands = &GetRustWaterCommandServices(),
		.set_progress = [](OpenTTDGenerationFrame *f, GenWorldProgress stage, uint32_t total) noexcept { return RunGenerationProgress(f, [=] { RustSetGeneratingWorldProgress(stage, total); }); },
		.increase_progress = [](OpenTTDGenerationFrame *f, GenWorldProgress stage) noexcept { return RunGenerationProgress(f, [=] { RustIncreaseGeneratingWorldProgress(stage); }); },
		.begin_loop = [](OpenTTDGenerationFrame *f) noexcept -> const OpenTTDGenerationLoop * { return &f->loop.emplace().view; },
		.end_loop = [](OpenTTDGenerationFrame *f) noexcept { f->loop.reset(); },
		.tgp_settings = RustTGPSettings,
		.terrain_template = [](uint32_t index) noexcept {
			const Sprite *templ = GetSprite(index, SpriteType::MapGen);
			if (templ == nullptr) UserError("Map generator sprites could not be loaded");
			return OpenTTDTerrainTemplate{.width = templ->width, .height = templ->height, .data = reinterpret_cast<const uint8_t *>(templ->data)};
		},
		.flat_height = RustFlatTileHeight,
		.river = RustIsRiver,
		.set_snow_line = RustSetSnowLineHeight,
		.river_debug = RustRiverYapfDebug,
		.assert_condition = RustAssertCondition,
		.sea_construct = [](OpenTTDGenerationFrame *f) noexcept { f->sea.Construct(); },
		.sea_destroy = [](OpenTTDGenerationFrame *f) noexcept { f->sea.Destroy(); },
		.sea_insert = [](OpenTTDGenerationFrame *f, uint32_t tile) noexcept { f->sea.Insert(tile); },
		.sea_contains = [](OpenTTDGenerationFrame *f, uint32_t tile) noexcept { return f->sea.Contains(tile); },
		.sea_size = [](OpenTTDGenerationFrame *f) noexcept { return f->sea.Size(); },
		.sea_first = [](OpenTTDGenerationFrame *f) noexcept { return f->sea.First(); },
		.sea_next = [](OpenTTDGenerationFrame *f) noexcept { return f->sea.Next(); },
		.set_height = RustSetTileHeight,
		.inner_tile = RustInnerTile,
		.make_void = [](uint32_t t) noexcept { MakeVoid(TileIndex{t}); },
		.screen_dirty = RustScreenDirty,
		.set_freeform = RustSetFreeformEdges,
		.clear_bridge_middle = RustClearBridgeMiddle,
	};
	return services;
}

void InitializeLandscape()
{
	openttd_rust_initialize_landscape(&GetRustLandscapeGenerationServices(), _settings_game.construction.freeform_edges);
}

OpenTTDLandscapeGenerationCall::OpenTTDLandscapeGenerationCall()
{
	static_assert(GWP_LANDSCAPE == 1 && GWP_RIVER == 2 && GWM_HEIGHTMAP == 3 && LG_ORIGINAL == 0 && LG_TERRAGENESIS == 1 && MAX_TILE_HEIGHT == 255);
	static_assert(SPR_MAPGEN_BEGIN == 4845 && TILE_UPDATE_FREQUENCY == 256);
	const auto &g = _settings_game.game_creation;
	this->settings = {
		.max_search_nodes = static_cast<int32_t>(_settings_game.pf.yapf.max_search_nodes),
		.climate = to_underlying(g.landscape), .generator = g.land_generator,
		.rivers = g.amount_of_rivers, .min_river_length = g.min_river_length, .route_random = g.river_route_random,
		.terrain = _settings_game.difficulty.terrain_type, .sea = _settings_game.difficulty.quantity_sea_lakes,
		.snow_coverage = g.snow_coverage, .desert_coverage = g.desert_coverage, .tree_placer = g.tree_placer,
		.height_limit = _settings_game.construction.map_height_limit, .rotation = g.heightmap_rotation, .heightmap_height = g.heightmap_height,
		.freeform = _settings_game.construction.freeform_edges,
	};
}



#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDLandscapeGenerationServices.hpp"
#include "abi-visit-OpenTTDHeightmapInput.hpp"
#include "abi-visit-OpenTTDMovementServices.hpp"
#include "abi-visit-OpenTTDTileOwnerServices.hpp"
#include "abi-visit-OpenTTDClearServices.hpp"
#include "abi-visit-OpenTTDRailServices.hpp"
#include "abi-visit-OpenTTDRoadGroundServices.hpp"
#include "abi-visit-OpenTTDTunnelServices.hpp"
#include "abi-visit-OpenTTDWaterGroundServices.hpp"
#include "abi-visit-OpenTTDGroundCostServices.hpp"
bool CheckAbi_landscape()
{
	RustHeightmapInput heightmap;
	return AbiServicesComplete(GetRustLandscapeGenerationServices()) && AbiServicesComplete(heightmap.view) &&
		AbiServicesComplete(GetRustTileOwnerServices()) &&
		AbiServicesComplete(GetRustClearServices()) && AbiServicesComplete(GetRustRailGroundServices()) &&
		AbiServicesComplete(GetRustRoadGroundServices()) && AbiServicesComplete(GetRustTunnelGroundServices()) &&
		AbiServicesComplete(GetRustWaterGroundServices()) && AbiServicesComplete(GetRustGroundCostServices()) &&
		AbiServicesComplete(GetRustMovementServices());
}

#include "rust/services_assert.h"
/* Namespace scope: the constinit shared table holds this address, so Rust loads
 * it instead of calling. Its nested getters initialize before main. */
const OpenTTDMovementServices _rust_movement_services{
	.shared = &GetRustSharedServices(),
	.orders = &GetRustOrdersLeaves(),
	.rail = &GetRustRailGroundServices(),
	.road = &GetRustRoadGroundServices(),
	.rail_map = &GetRustRailMapServices(),
	.train = &GetTrainServices(),
	.vehicle = &GetRustVehicleServices(),
	.tunnel_bridge = &GetRustTunnelBridgeServices(),
	.road_compatible = RustVehicleRead<uint64_t, RustRoadCompatibleTypes>,
	.road_type_speed = RustRoadTypeMaxSpeed,
	.road_kind = RustMovementRoadKind,
	.road_vehicle = &GetRustRoadLeaves(),
	.resolve = RustMovementResolve,
	.pose = RustMovementPose,
	.depot_position = RustMovementDepotPosition,
	.incline = RustMovementIncline,
	.first = RustMovementFirst,
	.first_speed = RustMovementFirstSpeed,
	.write_ground_flags = RustMovementWriteGroundFlags,
	.train_engine_class = RustMovementTrainEngineClass,
	.depot_invalidate = RustMovementDepotInvalidate,
	.native_assert = RustAssertCondition,
	.plain_rail = RustMovementPlainRail,
	.is_tunnel = RustIsTunnel,
	.rail_station = RustMovementRailStation,
	.road_crossing = MapSvcRoadIsLevelCrossing,
	.tile_flat = GetRustTownFoundingServices().flat,
	.add_by_diag_dir = RustTileAddByDiagDir,
	.rail_depot_tile = RustRailDepotTile,
	.has_tile_road_type = RustHasTileRoadType,
	.oneway_signal = RustRailOnewaySignal,
	.crossing_barred = RustCrossingBarred,
	.buoy = RustStationBuoy,
	.water_type = GetRustWaterCommandServices().water_type,
	.lock_direction = GetRustWaterCommandServices().lock_direction,
	.ship_depot_axis = RustShipDepotAxis,
	.owned_land = GetRustRailGroundServices().owned_land,
	.object_foundation = RustObjectFoundation,
	.house_foundation = RustHouseFoundation,
	.house_boolean = RustHouseFoundationBoolean,
	.industry_foundation = RustIndustryFoundation,
	.industry_callback = RustIndustryFoundationCallback,
	.industry_boolean = RustIndustryFoundationBoolean,
};
const OpenTTDMovementServices &GetRustMovementServices() noexcept
{
	return _rust_movement_services;
}
