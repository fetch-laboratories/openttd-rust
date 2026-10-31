/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file road_cmd.cpp Commands related to road tiles. */

#include "stdafx.h"
#include "road.h"
#include "road_internal.h"
#include "viewport_func.h"
#include "command_func.h"
#include "company_func.h"
#include "pathfinder/yapf/yapf_cache.h"
#include "depot_base.h"
#include "newgrf.h"
#include "autoslope.h"
#include "tunnelbridge_map.h"
#include "strings_func.h"
#include "vehicle_func.h"
#include "sound_func.h"
#include "tunnelbridge.h"
#include "cheat_type.h"
#include "effectvehicle_func.h"
#include "effectvehicle_base.h"
#include "elrail_func.h"
#include "roadveh.h"
#include "train.h"
#include "town.h"
#include "company_base.h"
#include "core/random_func.hpp"
#include "core/container_func.hpp"
#include "newgrf_debug.h"
#include "newgrf_railtype.h"
#include "newgrf_roadtype.h"
#include "timer/timer_game_calendar.h"
#include "genworld.h"
#include "company_gui.h"
#include "road_func.h"
#include "road_cmd.h"
#include "landscape_cmd.h"
#include "rail_cmd.h"

#include "table/strings.h"
#include "table/roadtypes.h"

#include "rust/ground_services.h"
#include "rust/services_assert.h"
#include "rust/road_command_services.h"
#include "rust/services_road.h"
#include "rust/town_ffi.h"
#include "rust/services_landscape.h"
#include "rust/environment_services.h"

#include "rust/movement_ffi.h"

#include "rust/type_registry_ffi.h"
#include "rust/type_availability_services.h"

#include "safeguards.h"


/** Helper type for lists/vectors of road vehicles */
typedef std::vector<RoadVehicle *> RoadVehicleList;

RoadTypeInfo _roadtypes[ROADTYPE_END];
static_assert(sizeof(RoadTypes) == sizeof(uint64_t) && alignof(RoadTypes) == alignof(uint64_t));
std::span<RoadType> _sorted_roadtypes;
RoadTypes &_roadtypes_hidden_mask = *reinterpret_cast<RoadTypes *>(openttd_rust_road_hidden_mask());
RoadTypes &_roadtypes_road = *reinterpret_cast<RoadTypes *>(openttd_rust_road_mask());
RoadTypes &_roadtypes_tram = *reinterpret_cast<RoadTypes *>(openttd_rust_tram_mask());

/**
 * Reset all road type information to its default values.
 */

void ResolveRoadTypeGUISprites(RoadTypeInfo *rti)
{
	SpriteID cursors_base = GetCustomRoadSprite(rti, INVALID_TILE, ROTSG_CURSORS);
	if (cursors_base != 0) {
		rti->gui_sprites.build_y_road = cursors_base +  0;
		rti->gui_sprites.build_x_road = cursors_base +  1;
		rti->gui_sprites.auto_road    = cursors_base +  2;
		rti->gui_sprites.build_depot  = cursors_base +  3;
		rti->gui_sprites.build_tunnel = cursors_base +  4;
		rti->gui_sprites.convert_road = cursors_base +  5;
		rti->cursor.road_swne         = cursors_base +  6;
		rti->cursor.road_nwse         = cursors_base +  7;
		rti->cursor.autoroad          = cursors_base +  8;
		rti->cursor.depot             = cursors_base +  9;
		rti->cursor.tunnel            = cursors_base + 10;
		rti->cursor.convert_road      = cursors_base + 11;
	}
}

/**
 * Compare roadtypes based on their sorting order.
 * @param first  The roadtype to compare to.
 * @param second The roadtype to compare.
 * @return True iff the first should be sorted before the second.
 */
static bool CompareRoadTypes(const RoadType &first, const RoadType &second)
{
	if (RoadTypeIsRoad(first) == RoadTypeIsRoad(second)) {
		return GetRoadTypeInfo(first)->sorting_order < GetRoadTypeInfo(second)->sorting_order;
	}
	return RoadTypeIsTram(first) < RoadTypeIsTram(second);
}

/**
 * Resolve sprites of custom road types
 */

/**
 * Allocate a new road type label
 */

const OpenTTDRoadRegistryServices &GetRustRoadRegistryServices() noexcept
{
	static const OpenTTDRoadRegistryServices table = {
		.default_count = std::size(_original_roadtypes),
		.spec = [](RoadType rt) noexcept { return &_roadtypes[rt]; },
		.type_label = [](RoadType rt) noexcept { return RustRoadTypeLabel(GetRoadTypeInfo(rt)); },
		.labels = []() noexcept -> const RoadTypeLabel * const * {
			static const auto labels = [] {
				std::array<const RoadTypeLabel *, ROADTYPE_END> pointers{};
				for (size_t i = 0; i < pointers.size(); ++i) pointers[i] = &RustRoadTypeLabel(&_roadtypes[i]);
				return pointers;
			}();
			return labels.data();
		},
		.alternate = [](RoadType rt, uint32_t label) noexcept { return GetRoadTypeInfo(rt)->alternate_labels.contains(label); },
		.clear = [](RoadTypeInfo *info) noexcept { *info = RoadTypeInfo{}; },
		.copy_default = [](RoadTypeInfo *info, RoadType source) noexcept { *info = _original_roadtypes[source]; },
		.resolve_gui = [](RoadTypeInfo *info) noexcept { ResolveRoadTypeGUISprites(info); },
		.publication = [](const RoadTypeInfo *info) noexcept { return OpenTTDTypePublication{.label = RustRoadTypeLabel(info), .hidden = RustRoadTypeHidden(info)}; },
		.sort = [](RoadType *data, size_t count) noexcept { std::sort(data, data + count, CompareRoadTypes); },
		.publish_sorted = [](RoadType *data, size_t count) noexcept { _sorted_roadtypes = {data, count}; },
		.allocate = [](RoadTypeInfo *info, const OpenTTDTypeAllocation *value) noexcept {
			info->label = value->label;
			info->alternate_labels.clear();
			info->flags = {};
			info->introduction_date = CalendarTime::INVALID_DATE;
			info->powered_roadtypes = RoadTypes{value->self};
			info->introduces_roadtypes = RoadTypes{value->self};
			info->sorting_order = value->order;
		},
	};
	return table;
}




/**
 * Update road infrastructure counts for a company.
 * @param rt Road type to update count of.
 * @param o Owner of road piece.
 * @param count Number of road pieces to adjust.
 */


/** Invalid RoadBits on slopes.  */

static Foundation GetRoadFoundation(Slope tileh, RoadBits bits);

/**
 * Is it allowed to remove the given road bits from the given tile?
 * @param tile      the tile to remove the road from
 * @param remove    the roadbits that are going to be removed
 * @param owner     the actual owner of the roadbits of the tile
 * @param rt        the road type to remove the bits from
 * @param flags     command flags
 * @param town_check Shall the town rating checked/affected
 * @return A succeeded command when it is allowed to remove the road bits, a failed command otherwise.
 */


/**
 * Delete a piece of road.
 * @param tile tile where to remove road from
 * @param flags operation to perform
 * @param pieces roadbits to remove
 * @param rt roadtype to remove
 * @param town_check should we check if the town allows removal?
 */


/**
 * Calculate the costs for roads on slopes
 *  Aside modify the RoadBits to fit on the slopes
 *
 * @note The RoadBits are modified too!
 * @param tileh The current slope
 * @param pieces The RoadBits we want to add
 * @param existing The existent RoadBits of the current type
 * @param other The other existent RoadBits
 * @return The costs for these RoadBits on this slope
 */

/**
 * Build a piece of road.
 * @param flags operation to perform
 * @param tile tile where to build road
 * @param pieces road pieces to build (RoadBits)
 * @param rt road type
 * @param toggle_drd disallowed directions to toggle
 * @param town_id the town that is building the road (TownID::Invalid() if not applicable)
 * @return the cost of this operation or an error
 */

/**
 * Checks whether a road or tram connection can be found when building a new road or tram.
 * @param tile Tile at which the road being built will end.
 * @param rt Roadtype of the road being built.
 * @param dir Direction that the road is following.
 * @return True if the next tile at dir direction is suitable for being connected directly by a second roadbit at the end of the road being built.
 */

/**
 * Build a long piece of road.
 * @param flags operation to perform
 * @param end_tile end tile of drag
 * @param start_tile start tile of drag
 * @param rt road type
 * @param axis direction
 * @param drd set road direction
 * @param start_half start tile starts in the 2nd half of tile (p2 & 1). Only used if \c is_ai is set or if we are building a single tile
 * @param end_half end tile starts in the 2nd half of tile (p2 & 2). Only used if \c is_ai is set or if we are building a single tile
 * @param is_ai defines two different behaviors for this command:
 *      - false = Build up to an obstacle. Do not build the first and last roadbits unless they can be connected to something, or if we are building a single tile
 *      - true = Fail if an obstacle is found. Always take into account start_half and end_half. This behavior is used for scripts
 * @return the cost of this operation or an error
 */

/**
 * Remove a long piece of road.
 * @param flags operation to perform
 * @param end_tile end tile of drag
 * @param start_tile start tile of drag
 * @param rt road type
 * @param axis direction
 * @param start_half start tile starts in the 2nd half of tile
 * @param end_half end tile starts in the 2nd half of tile (p2 & 2)
 * @return the cost of this operation or an error
 */

/**
 * Build a road depot.
 * @param tile tile where to build the depot
 * @param flags operation to perform
 * @param rt road type
 * @param dir entrance direction
 * @return the cost of this operation or an error
 *
 * @todo When checking for the tile slope,
 * distinguish between "Flat land required" and "land sloped in wrong direction"
 */




struct DrawRoadTileStruct {
	uint16_t image;
	uint8_t subcoord_x;
	uint8_t subcoord_y;
};

#include "table/road_land.h"

/**
 * Get the foundationtype of a RoadBits Slope combination
 *
 * @param tileh The Slope part
 * @param bits The RoadBits part
 * @return The resulting Foundation
 */
static Foundation GetRoadFoundation(Slope tileh, RoadBits bits)
{
	return openttd_rust_road_foundation(tileh, bits, RustAssertCondition);
}

const uint8_t _road_sloped_sprites[14] = {
	0,  0,  2,  0,
	0,  1,  0,  0,
	3,  0,  0,  0,
	0,  0
};

/**
 * Get the sprite offset within a spritegroup.
 * @param slope Slope
 * @param bits Roadbits
 * @return Offset for the sprite within the spritegroup.
 */
static uint GetRoadSpriteOffset(Slope slope, RoadBits bits)
{
	if (slope != SLOPE_FLAT) {
		switch (slope) {
			case SLOPE_NE: return 11;
			case SLOPE_SE: return 12;
			case SLOPE_SW: return 13;
			case SLOPE_NW: return 14;
			default: NOT_REACHED();
		}
	} else {
		static const uint offsets[] = {
			0, 18, 17, 7,
			16, 0, 10, 5,
			15, 8, 1, 4,
			9, 3, 6, 2
		};
		return offsets[bits];
	}
}

/**
 * Should the road be drawn as a unpaved snow/desert road?
 * By default, roads are always drawn as unpaved if they are on desert or
 * above the snow line, but NewGRFs can override this for desert.
 *
 * @param snow_or_desert Is snowy or desert tile
 * @param roadside What sort of road this is
 * @return True if snow/desert road sprites should be used.
 */
static bool DrawRoadAsSnowOrDesert(bool snow_or_desert, Roadside roadside)
{
	return (snow_or_desert &&
			!(_settings_game.game_creation.landscape == LandscapeType::Tropic && HasGrfMiscBit(GrfMiscBit::DesertPavedRoads) &&
				roadside != Roadside::Barren && roadside != Roadside::Grass && roadside != Roadside::GrassRoadWorks));
}

/**
 * Draws the catenary for the RoadType of the given tile
 * @param ti information about the tile (slopes, height etc)
 * @param rt road type to draw catenary for
 * @param rb the roadbits for the tram
 */
void DrawRoadTypeCatenary(const TileInfo *ti, RoadType rt, RoadBits rb)
{
	/* Don't draw the catenary under a low bridge */
	if (IsBridgeAbove(ti->tile) && !IsTransparencySet(TO_CATENARY)) {
		int height = GetBridgeHeight(GetNorthernBridgeEnd(ti->tile));

		if (height <= GetTileMaxZ(ti->tile) + 1) return;
	}

	if (CountBits(rb) > 2) {
		/* On junctions we check whether neighbouring tiles also have catenary, and possibly
		 * do not draw catenary towards those neighbours, which do not have catenary. */
		RoadBits rb_new = ROAD_NONE;
		for (DiagDirection dir = DIAGDIR_BEGIN; dir < DIAGDIR_END; dir++) {
			if (rb & DiagDirToRoadBits(dir)) {
				TileIndex neighbour = TileAddByDiagDir(ti->tile, dir);
				if (MayHaveRoad(neighbour)) {
					RoadType rt_road = GetRoadTypeRoad(neighbour);
					RoadType rt_tram = GetRoadTypeTram(neighbour);

					if ((rt_road != INVALID_ROADTYPE && HasRoadCatenary(rt_road)) ||
							(rt_tram != INVALID_ROADTYPE && HasRoadCatenary(rt_tram))) {
						rb_new |= DiagDirToRoadBits(dir);
					}
				}
			}
		}
		if (CountBits(rb_new) >= 2) rb = rb_new;
	}

	const RoadTypeInfo *rti = GetRoadTypeInfo(rt);
	SpriteID front = GetCustomRoadSprite(rti, ti->tile, ROTSG_CATENARY_FRONT);
	SpriteID back = GetCustomRoadSprite(rti, ti->tile, ROTSG_CATENARY_BACK);

	if (front != 0 || back != 0) {
		if (front != 0) front += GetRoadSpriteOffset(ti->tileh, rb);
		if (back != 0) back += GetRoadSpriteOffset(ti->tileh, rb);
	} else if (ti->tileh != SLOPE_FLAT) {
		back  = SPR_TRAMWAY_BACK_WIRES_SLOPED  + _road_sloped_sprites[ti->tileh - 1];
		front = SPR_TRAMWAY_FRONT_WIRES_SLOPED + _road_sloped_sprites[ti->tileh - 1];
	} else {
		back  = SPR_TRAMWAY_BASE + _road_backpole_sprites_1[rb];
		front = SPR_TRAMWAY_BASE + _road_frontwire_sprites_1[rb];
	}

	/* Catenary uses 1st company colour to help identify owner.
	 * For tiles with OWNER_TOWN or OWNER_NONE, recolour CC to grey as a neutral colour. */
	Owner owner = GetRoadOwner(ti->tile, GetRoadTramType(rt));
	PaletteID pal = (owner == OWNER_NONE || owner == OWNER_TOWN ? GetColourPalette(COLOUR_GREY) : GetCompanyPalette(owner));
	uint8_t z_wires = (ti->tileh == SLOPE_FLAT ? 0 : TILE_HEIGHT) + BB_HEIGHT_UNDER_BRIDGE;
	if (back != 0) {
		/* The "back" sprite contains the west, north and east pillars.
		 * We cut the sprite at 3/8 of the west/east edges to create 3 sprites.
		 * 3/8 is chosen so that sprites can somewhat graphically extend into the tile. */
		static const int INF = 1000; ///< big number compared to sprite size
		static const SubSprite west  = { -INF, -INF, -12, INF };
		static const SubSprite north = {  -12, -INF,  12, INF };
		static const SubSprite east  = {   12, -INF, INF, INF };
		int8_t west_z = GetSlopePixelZInCorner(ti->tileh, CORNER_W);
		int8_t north_z = GetSlopePixelZInCorner(ti->tileh, CORNER_N);
		int8_t east_z = GetSlopePixelZInCorner(ti->tileh, CORNER_E);
		AddSortableSpriteToDraw(back, pal, *ti, {{15, 0, west_z}, {1, 1, z_wires}, {-15, 0, static_cast<int8_t>(-west_z)}}, IsTransparencySet(TO_CATENARY), &west);
		AddSortableSpriteToDraw(back, pal, *ti, {{0, 0, north_z}, {1, 1, z_wires}, {0, 0, static_cast<int8_t>(-north_z)}}, IsTransparencySet(TO_CATENARY), &north);
		AddSortableSpriteToDraw(back, pal, *ti, {{0, 15, east_z}, {1, 1, z_wires}, {0, -15, static_cast<int8_t>(-east_z)}}, IsTransparencySet(TO_CATENARY), &east);
	}
	if (front != 0) {
		/* Draw the "front" sprite (containing south pillar and wires) at a Z height that is both above the vehicles and above the "back" pillars. */
		AddSortableSpriteToDraw(front, pal, *ti, {{0, 0, static_cast<int8_t>(z_wires)}, {TILE_SIZE, TILE_SIZE, 1}, {0, 0, static_cast<int8_t>(-z_wires)}}, IsTransparencySet(TO_CATENARY));
	}
}

/**
 * Draws the catenary for the given tile
 * @param ti information about the tile (slopes, height etc)
 */
void DrawRoadCatenary(const TileInfo *ti)
{
	RoadBits road = ROAD_NONE;
	RoadBits tram = ROAD_NONE;

	if (IsTileType(ti->tile, MP_ROAD)) {
		if (IsNormalRoad(ti->tile)) {
			road = GetRoadBits(ti->tile, RTT_ROAD);
			tram = GetRoadBits(ti->tile, RTT_TRAM);
		} else if (IsLevelCrossing(ti->tile)) {
			tram = road = (GetCrossingRailAxis(ti->tile) == AXIS_Y ? ROAD_X : ROAD_Y);
		}
	} else if (IsTileType(ti->tile, MP_STATION)) {
		if (IsAnyRoadStop(ti->tile)) {
			if (IsDriveThroughStopTile(ti->tile)) {
				Axis axis = GetDriveThroughStopAxis(ti->tile);
				tram = road = (axis == AXIS_X ? ROAD_X : ROAD_Y);
			} else {
				tram = road = DiagDirToRoadBits(GetBayRoadStopDir(ti->tile));
			}
		}
	} else {
		/* No road here, no catenary to draw */
		return;
	}

	RoadType rt = GetRoadTypeRoad(ti->tile);
	if (rt != INVALID_ROADTYPE && HasRoadCatenaryDrawn(rt)) {
		DrawRoadTypeCatenary(ti, rt, road);
	}

	rt = GetRoadTypeTram(ti->tile);
	if (rt != INVALID_ROADTYPE && HasRoadCatenaryDrawn(rt)) {
		DrawRoadTypeCatenary(ti, rt, tram);
	}
}

/**
 * Draws details on/around the road
 * @param img the sprite to draw
 * @param ti  the tile to draw on
 * @param dx  the offset from the top of the BB of the tile
 * @param dy  the offset from the top of the BB of the tile
 * @param h   the height of the sprite to draw
 * @param transparent  whether the sprite should be transparent (used for roadside trees)
 */
static void DrawRoadDetail(SpriteID img, const TileInfo *ti, int8_t dx, int8_t dy, uint8_t h, bool transparent)
{
	int x = ti->x | dx;
	int y = ti->y | dy;
	int z = ti->z;
	if (ti->tileh != SLOPE_FLAT) z = GetSlopePixelZ(x, y);
	AddSortableSpriteToDraw(img, PAL_NONE, ti->x, ti->y, z, {{dx, dy, 0}, {2, 2, h}, {}}, transparent);
}

/**
 * Draw road underlay and overlay sprites.
 * @param ti TileInfo
 * @param road_rti Road road type information
 * @param tram_rti Tram road type information
 * @param road_offset Road sprite offset (based on road bits)
 * @param tram_offset Tram sprite offset (based on road bits)
 * @param draw_underlay Whether to draw underlays
 */
void DrawRoadOverlays(const TileInfo *ti, PaletteID pal, const RoadTypeInfo *road_rti, const RoadTypeInfo *tram_rti, uint road_offset, uint tram_offset, bool draw_underlay)
{
	if (draw_underlay) {
		/* Road underlay takes precedence over tram */
		if (road_rti != nullptr) {
			if (road_rti->UsesOverlay()) {
				SpriteID ground = GetCustomRoadSprite(road_rti, ti->tile, ROTSG_GROUND);
				DrawGroundSprite(ground + road_offset, pal);
			}
		} else {
			if (tram_rti->UsesOverlay()) {
				SpriteID ground = GetCustomRoadSprite(tram_rti, ti->tile, ROTSG_GROUND);
				DrawGroundSprite(ground + tram_offset, pal);
			} else {
				DrawGroundSprite(SPR_TRAMWAY_TRAM + tram_offset, pal);
			}
		}
	}

	/* Draw road overlay */
	if (road_rti != nullptr) {
		if (road_rti->UsesOverlay()) {
			SpriteID ground = GetCustomRoadSprite(road_rti, ti->tile, ROTSG_OVERLAY);
			if (ground != 0) DrawGroundSprite(ground + road_offset, pal);
		}
	}

	/* Draw tram overlay */
	if (tram_rti != nullptr) {
		if (tram_rti->UsesOverlay()) {
			SpriteID ground = GetCustomRoadSprite(tram_rti, ti->tile, ROTSG_OVERLAY);
			if (ground != 0) DrawGroundSprite(ground + tram_offset, pal);
		} else if (road_rti != nullptr) {
			DrawGroundSprite(SPR_TRAMWAY_OVERLAY + tram_offset, pal);
		}
	}
}

/**
 * Get ground sprite to draw for a road tile.
 * @param ti TileInof
 * @param roadside Road side type
 * @param rti Road type info
 * @param offset Road sprite offset
 * @param snow_or_desert Whether to get snow/desert ground sprite
 * @param[out] pal Palette to draw.
 */
static SpriteID GetRoadGroundSprite(const TileInfo *ti, Roadside roadside, const RoadTypeInfo *rti, uint offset, bool snow_or_desert, PaletteID *pal)
{
	/* Draw bare ground sprite if no road or road uses overlay system. */
	if (rti == nullptr || rti->UsesOverlay()) {
		if (DrawRoadAsSnowOrDesert(snow_or_desert, roadside)) {
			return SPR_FLAT_SNOW_DESERT_TILE + SlopeToSpriteOffset(ti->tileh);
		}

		switch (roadside) {
			case Roadside::Barren:
				*pal = PALETTE_TO_BARE_LAND;
			    return SPR_FLAT_GRASS_TILE + SlopeToSpriteOffset(ti->tileh);

			case Roadside::Grass:
			case Roadside::GrassRoadWorks:
				return SPR_FLAT_GRASS_TILE + SlopeToSpriteOffset(ti->tileh);

			default:
				break; // Paved
		}
	}

	/* Draw original road base sprite */
	SpriteID image = SPR_ROAD_Y + offset;
	if (DrawRoadAsSnowOrDesert(snow_or_desert, roadside)) {
		image += 19;
	} else {
		switch (roadside) {
			case Roadside::Barren:
				*pal = PALETTE_TO_BARE_LAND;
				break;

			case Roadside::Grass:
			case Roadside::GrassRoadWorks:
				break;

			default:
				image -= 19;
				break; // Paved
		}
	}

	return image;
}

/**
 * Draw road ground sprites.
 * @param ti TileInfo
 * @param road Road bits
 * @param tram Tram bits
 * @param road_rti Road road type information
 * @param tram_rti Tram road type information
 * @param roadside Roadside type
 * @param snow_or_desert Whether to draw snow/desert ground sprites
 */
void DrawRoadGroundSprites(const TileInfo *ti, RoadBits road, RoadBits tram, const RoadTypeInfo *road_rti, const RoadTypeInfo *tram_rti, Roadside roadside, bool snow_or_desert)
{
	/* Determine sprite offsets */
	uint road_offset = GetRoadSpriteOffset(ti->tileh, road);
	uint tram_offset = GetRoadSpriteOffset(ti->tileh, tram);

	/* Draw baseset underlay */
	PaletteID pal = PAL_NONE;
	SpriteID image = GetRoadGroundSprite(ti, roadside, road_rti, road == ROAD_NONE ? tram_offset : road_offset, snow_or_desert, &pal);
	DrawGroundSprite(image, pal);

	DrawRoadOverlays(ti, pal, road_rti, tram_rti, road_offset, tram_offset);
}

/**
 * Draw ground sprite and road pieces
 * @param ti TileInfo
 */
static void DrawRoadBits(TileInfo *ti)
{
	RoadBits road = GetRoadBits(ti->tile, RTT_ROAD);
	RoadBits tram = GetRoadBits(ti->tile, RTT_TRAM);

	RoadType road_rt = GetRoadTypeRoad(ti->tile);
	RoadType tram_rt = GetRoadTypeTram(ti->tile);
	const RoadTypeInfo *road_rti = road_rt == INVALID_ROADTYPE ? nullptr : GetRoadTypeInfo(road_rt);
	const RoadTypeInfo *tram_rti = tram_rt == INVALID_ROADTYPE ? nullptr : GetRoadTypeInfo(tram_rt);

	if (ti->tileh != SLOPE_FLAT) {
		DrawFoundation(ti, GetRoadFoundation(ti->tileh, road | tram));
		/* DrawFoundation() modifies ti. */
	}

	DrawRoadGroundSprites(ti, road, tram, road_rti, tram_rti, GetRoadside(ti->tile), IsOnSnowOrDesert(ti->tile));

	/* Draw one way */
	if (road_rti != nullptr) {
		DisallowedRoadDirections drd = GetDisallowedRoadDirections(ti->tile);
		if (drd != DRD_NONE) {
			SpriteID oneway = GetCustomRoadSprite(road_rti, ti->tile, ROTSG_ONEWAY);

			if (oneway == 0) oneway = SPR_ONEWAY_BASE;

			if ((ti->tileh == SLOPE_NE) || (ti->tileh == SLOPE_NW)) {
				oneway += ONEWAY_SLOPE_N_OFFSET;
			} else if ((ti->tileh == SLOPE_SE) || (ti->tileh == SLOPE_SW)) {
				oneway += ONEWAY_SLOPE_S_OFFSET;
			}

			DrawGroundSpriteAt(oneway + drd - 1 + ((road == ROAD_X) ? 0 : 3), PAL_NONE, 8, 8, GetPartialPixelZ(8, 8, ti->tileh));
		}
	}

	if (HasRoadWorks(ti->tile)) {
		/* Road works */
		DrawGroundSprite((road | tram) & ROAD_X ? SPR_EXCAVATION_X : SPR_EXCAVATION_Y, PAL_NONE);
		return;
	}

	/* Draw road, tram catenary */
	DrawRoadCatenary(ti);

	/* Return if full detail is disabled, or we are zoomed fully out. */
	if (!HasBit(_display_opt, DO_FULL_DETAIL) || _cur_dpi->zoom > ZoomLevel::Detail) return;

	/* Do not draw details (street lights, trees) under low bridge */
	Roadside roadside = GetRoadside(ti->tile);
	if (IsBridgeAbove(ti->tile) && (roadside == Roadside::Trees || roadside == Roadside::StreetLights)) {
		int height = GetBridgeHeight(GetNorthernBridgeEnd(ti->tile));
		int minz = GetTileMaxZ(ti->tile) + 2;

		if (roadside == Roadside::Trees) minz++;

		if (height < minz) return;
	}

	/* If there are no road bits, return, as there is nothing left to do */
	if (HasAtMostOneBit(road)) return;

	/* Do not draw details when invisible. */
	if (roadside == Roadside::Trees && IsInvisibilitySet(TO_TREES)) return;
	if (roadside == Roadside::StreetLights && IsInvisibilitySet(TO_HOUSES)) return;

	/* Check whether details should be transparent. */
	bool is_transparent = false;
	if (roadside == Roadside::Trees && IsTransparencySet(TO_TREES)) {
		is_transparent = true;
	}
	if (roadside == Roadside::StreetLights && IsTransparencySet(TO_HOUSES)) {
		is_transparent = true;
	}

	/* Draw extra details. */
	for (const DrawRoadTileStruct *drts = _road_display_table[to_underlying(roadside)][road | tram]; drts->image != 0; drts++) {
		DrawRoadDetail(drts->image, ti, drts->subcoord_x, drts->subcoord_y, 0x10, is_transparent);
	}
}

/** Tile callback function for rendering a road tile to the screen */
static void DrawTile_Road(TileInfo *ti)
{
	BridgePillarFlags blocked_pillars{};
	switch (GetRoadTileType(ti->tile)) {
		case RoadTileType::Normal:
			DrawRoadBits(ti);

			if (IsBridgeAbove(ti->tile)) {
				RoadBits bits = GetAllRoadBits(ti->tile);
				if ((bits & ROAD_NE) != 0) blocked_pillars.Set(BridgePillarFlag::EdgeNE);
				if ((bits & ROAD_SE) != 0) blocked_pillars.Set(BridgePillarFlag::EdgeSE);
				if ((bits & ROAD_SW) != 0) blocked_pillars.Set(BridgePillarFlag::EdgeSW);
				if ((bits & ROAD_NW) != 0) blocked_pillars.Set(BridgePillarFlag::EdgeNW);
			}
			break;

		case RoadTileType::Crossing: {
			if (ti->tileh != SLOPE_FLAT) DrawFoundation(ti, FOUNDATION_LEVELED);

			Axis axis = GetCrossingRailAxis(ti->tile);

			const RailTypeInfo *rti = GetRailTypeInfo(GetRailType(ti->tile));

			RoadType road_rt = GetRoadTypeRoad(ti->tile);
			RoadType tram_rt = GetRoadTypeTram(ti->tile);
			const RoadTypeInfo *road_rti = road_rt == INVALID_ROADTYPE ? nullptr : GetRoadTypeInfo(road_rt);
			const RoadTypeInfo *tram_rti = tram_rt == INVALID_ROADTYPE ? nullptr : GetRoadTypeInfo(tram_rt);

			PaletteID pal = PAL_NONE;

			/* Draw base ground */
			if (rti->UsesOverlay()) {
				SpriteID image = SPR_ROAD_Y + axis;

				Roadside roadside = GetRoadside(ti->tile);
				if (DrawRoadAsSnowOrDesert(IsOnSnowOrDesert(ti->tile), roadside)) {
					image += 19;
				} else {
					switch (roadside) {
						case Roadside::Barren:
							pal = PALETTE_TO_BARE_LAND;
							break;

						case Roadside::Grass:
							break;

						default:
							image -= 19;
							break; // Paved
					}
				}

				DrawGroundSprite(image, pal);
			} else {
				SpriteID image = rti->base_sprites.crossing + axis;
				if (IsCrossingBarred(ti->tile)) image += 2;

				Roadside roadside = GetRoadside(ti->tile);
				if (DrawRoadAsSnowOrDesert(IsOnSnowOrDesert(ti->tile), roadside)) {
					image += 8;
				} else {
					switch (roadside) {
						case Roadside::Barren:
							pal = PALETTE_TO_BARE_LAND;
							break;

						case Roadside::Grass:
							break;

						default:
							image += 4;
							break; // Paved
					}
				}

				DrawGroundSprite(image, pal);
			}

			DrawRoadOverlays(ti, pal, road_rti, tram_rti, axis, axis);

			/* Draw rail/PBS overlay */
			bool draw_pbs = _game_mode != GM_MENU && _settings_client.gui.show_track_reservation && HasCrossingReservation(ti->tile);
			if (rti->UsesOverlay()) {
				pal = draw_pbs ? PALETTE_CRASH : PAL_NONE;
				SpriteID rail = GetCustomRailSprite(rti, ti->tile, RTSG_CROSSING) + axis;
				DrawGroundSprite(rail, pal);

				const Axis road_axis = GetCrossingRoadAxis(ti->tile);
				const DiagDirection dir1 = AxisToDiagDir(road_axis);
				const DiagDirection dir2 = ReverseDiagDir(dir1);
				DiagDirections adjacent_diagdirs{};
				for (DiagDirection dir : { dir1, dir2 }) {
					const TileIndex t = TileAddByDiagDir(ti->tile, dir);
					if (t < Map::Size() && IsLevelCrossingTile(t) && GetCrossingRoadAxis(t) == road_axis) {
						adjacent_diagdirs.Set(dir);
					}
				}

				switch (adjacent_diagdirs.base()) {
					case DiagDirections{}.base():
						DrawRailTileSeq(ti, &_crossing_layout, TO_CATENARY, rail, 0, PAL_NONE);
						break;

					case DiagDirections{DIAGDIR_NE}.base():
						DrawRailTileSeq(ti, &_crossing_layout_SW, TO_CATENARY, rail, 0, PAL_NONE);
						break;

					case DiagDirections{DIAGDIR_SE}.base():
						DrawRailTileSeq(ti, &_crossing_layout_NW, TO_CATENARY, rail, 0, PAL_NONE);
						break;

					case DiagDirections{DIAGDIR_SW}.base():
						DrawRailTileSeq(ti, &_crossing_layout_NE, TO_CATENARY, rail, 0, PAL_NONE);
						break;

					case DiagDirections{DIAGDIR_NW}.base():
						DrawRailTileSeq(ti, &_crossing_layout_SE, TO_CATENARY, rail, 0, PAL_NONE);
						break;

					default:
						/* Show no sprites */
						break;
				}
			} else if (draw_pbs || tram_rti != nullptr || road_rti->UsesOverlay()) {
				/* Add another rail overlay, unless there is only the base road sprite. */
				pal = draw_pbs ? PALETTE_CRASH : PAL_NONE;
				SpriteID rail = GetCrossingRoadAxis(ti->tile) == AXIS_Y ? GetRailTypeInfo(GetRailType(ti->tile))->base_sprites.single_x : GetRailTypeInfo(GetRailType(ti->tile))->base_sprites.single_y;
				DrawGroundSprite(rail, pal);
			}

			/* Draw road, tram catenary */
			DrawRoadCatenary(ti);

			/* Draw rail catenary */
			if (HasRailCatenaryDrawn(GetRailType(ti->tile))) DrawRailCatenary(ti);
			blocked_pillars = {BridgePillarFlag::EdgeSW, BridgePillarFlag::EdgeNE, BridgePillarFlag::EdgeNW, BridgePillarFlag::EdgeSE};
			break;
		}

		default:
		case RoadTileType::Depot: {
			if (ti->tileh != SLOPE_FLAT) DrawFoundation(ti, FOUNDATION_LEVELED);

			PaletteID palette = GetCompanyPalette(GetTileOwner(ti->tile));

			RoadType road_rt = GetRoadTypeRoad(ti->tile);
			RoadType tram_rt = GetRoadTypeTram(ti->tile);
			const RoadTypeInfo *rti = GetRoadTypeInfo(road_rt == INVALID_ROADTYPE ? tram_rt : road_rt);

			int relocation = GetCustomRoadSprite(rti, ti->tile, ROTSG_DEPOT);
			bool default_gfx = relocation == 0;
			if (default_gfx) {
				if (rti->flags.Test(RoadTypeFlag::Catenary)) {
					if (_loaded_newgrf_features.tram == TRAMWAY_REPLACE_DEPOT_WITH_TRACK && road_rt == INVALID_ROADTYPE && !rti->UsesOverlay()) {
						/* Sprites with track only work for default tram */
						relocation = SPR_TRAMWAY_DEPOT_WITH_TRACK - SPR_ROAD_DEPOT;
						default_gfx = false;
					} else {
						/* Sprites without track are always better, if provided */
						relocation = SPR_TRAMWAY_DEPOT_NO_TRACK - SPR_ROAD_DEPOT;
					}
				}
			} else {
				relocation -= SPR_ROAD_DEPOT;
			}

			DiagDirection dir = GetRoadDepotDirection(ti->tile);
			const DrawTileSprites *dts = &_road_depot[dir];
			DrawGroundSprite(dts->ground.sprite, PAL_NONE);

			if (default_gfx) {
				uint offset = GetRoadSpriteOffset(SLOPE_FLAT, DiagDirToRoadBits(dir));
				if (rti->UsesOverlay()) {
					SpriteID ground = GetCustomRoadSprite(rti, ti->tile, ROTSG_OVERLAY);
					if (ground != 0) DrawGroundSprite(ground + offset, PAL_NONE);
				} else if (road_rt == INVALID_ROADTYPE) {
					DrawGroundSprite(SPR_TRAMWAY_OVERLAY + offset, PAL_NONE);
				}
			}

			DrawRailTileSeq(ti, dts, TO_BUILDINGS, relocation, 0, palette);
			/* Depots can't have bridges above so no blocked pillars. */
			break;
		}
	}
	DrawBridgeMiddle(ti, blocked_pillars);
}

/**
 * Draw the road depot sprite.
 * @param x   The x offset to draw at.
 * @param y   The y offset to draw at.
 * @param dir The direction the depot must be facing.
 * @param rt  The road type of the depot to draw.
 */
void DrawRoadDepotSprite(int x, int y, DiagDirection dir, RoadType rt)
{
	PaletteID palette = GetCompanyPalette(_local_company);

	const RoadTypeInfo *rti = GetRoadTypeInfo(rt);
	int relocation = GetCustomRoadSprite(rti, INVALID_TILE, ROTSG_DEPOT);
	bool default_gfx = relocation == 0;
	if (default_gfx) {
		if (rti->flags.Test(RoadTypeFlag::Catenary)) {
			if (_loaded_newgrf_features.tram == TRAMWAY_REPLACE_DEPOT_WITH_TRACK && RoadTypeIsTram(rt) && !rti->UsesOverlay()) {
				/* Sprites with track only work for default tram */
				relocation = SPR_TRAMWAY_DEPOT_WITH_TRACK - SPR_ROAD_DEPOT;
				default_gfx = false;
			} else {
				/* Sprites without track are always better, if provided */
				relocation = SPR_TRAMWAY_DEPOT_NO_TRACK - SPR_ROAD_DEPOT;
			}
		}
	} else {
		relocation -= SPR_ROAD_DEPOT;
	}

	const DrawTileSprites *dts = &_road_depot[dir];
	DrawSprite(dts->ground.sprite, PAL_NONE, x, y);

	if (default_gfx) {
		uint offset = GetRoadSpriteOffset(SLOPE_FLAT, DiagDirToRoadBits(dir));
		if (rti->UsesOverlay()) {
			SpriteID ground = GetCustomRoadSprite(rti, INVALID_TILE, ROTSG_OVERLAY);
			if (ground != 0) DrawSprite(ground + offset, PAL_NONE, x, y);
		} else if (RoadTypeIsTram(rt)) {
			DrawSprite(SPR_TRAMWAY_OVERLAY + offset, PAL_NONE, x, y);
		}
	}

	DrawRailTileSeqInGUI(x, y, dts, relocation, 0, palette);
}

/**
 * Updates cached nearest town for all road tiles
 * @param invalidate are we just invalidating cached data?
 * @pre invalidate == true implies _generating_world == true
 */



const OpenTTDRoadGroundServices &GetRustRoadGroundServices() noexcept
{
	static_assert(SND_21_ROAD_WORKS == 31 && OWNER_TOWN.base() == 15);
	static const OpenTTDRoadGroundServices services{
		.snow = [](uint32_t index, bool flat) noexcept { TileIndex t{index}; int z = flat ? OpenTTDTileMaxZ(t) : RustTileZ(t); return OpenTTDRoadSnowGround{.z = z, .snow = RustRoadSnowOrDesert(Tile{t})}; },
		.snow_or_desert = RustRoadSnowOrDesert,
		.toggle_snow_or_desert = [](uint32_t t) noexcept { ToggleSnowOrDesert(TileIndex{t}); },
		.roadside = [](uint32_t t) noexcept { return GetRoadside(TileIndex{t}); },
		.set_roadside = [](uint32_t t, Roadside side) noexcept { SetRoadside(TileIndex{t}, side); },
		.work_counter = [](uint32_t t) noexcept { return GB(Tile(TileIndex{t}).m7(), 0, 4); },
		.set_work_counter = [](uint32_t t, uint32_t counter) noexcept { SB(Tile(TileIndex{t}).m7(), 0, 4, counter); },
		.all_bits = [](uint32_t t) noexcept { return GetAllRoadBits(TileIndex{t}); },
		.cleanup = [](uint32_t t, RoadBits bits) noexcept { return RustCleanRoadBits(TileIndex{t}, bits); },
		.town = &GetRustTownServices(),
		.set_road_type = [](uint32_t t, RoadType type) noexcept { RustSetRoadType(TileIndex{t}, RTT_ROAD, type); },
	};
	return services;
}

static bool ClickTile_Road(TileIndex tile)
{
	if (!IsRoadDepot(tile)) return false;

	ShowDepotWindow(tile, VEH_ROAD);
	return true;
}

/* Converts RoadBits to TrackBits */


static const StringID _road_tile_strings[] = {
	STR_LAI_ROAD_DESCRIPTION_ROAD,
	STR_LAI_ROAD_DESCRIPTION_ROAD,
	STR_LAI_ROAD_DESCRIPTION_ROAD,
	STR_LAI_ROAD_DESCRIPTION_ROAD_WITH_STREETLIGHTS,
	STR_LAI_ROAD_DESCRIPTION_ROAD,
	STR_LAI_ROAD_DESCRIPTION_TREE_LINED_ROAD,
	STR_LAI_ROAD_DESCRIPTION_ROAD,
	STR_LAI_ROAD_DESCRIPTION_ROAD,
};

static void GetTileDesc_Road(TileIndex tile, TileDesc &td)
{
	Owner rail_owner = INVALID_OWNER;
	Owner road_owner = INVALID_OWNER;
	Owner tram_owner = INVALID_OWNER;

	RoadType road_rt = GetRoadTypeRoad(tile);
	RoadType tram_rt = GetRoadTypeTram(tile);
	if (road_rt != INVALID_ROADTYPE) {
		const RoadTypeInfo *rti = GetRoadTypeInfo(road_rt);
		td.roadtype = rti->strings.name;
		td.road_speed = rti->max_speed / 2;
		road_owner = GetRoadOwner(tile, RTT_ROAD);
	}
	if (tram_rt != INVALID_ROADTYPE) {
		const RoadTypeInfo *rti = GetRoadTypeInfo(tram_rt);
		td.tramtype = rti->strings.name;
		td.tram_speed = rti->max_speed / 2;
		tram_owner = GetRoadOwner(tile, RTT_TRAM);
	}

	switch (GetRoadTileType(tile)) {
		case RoadTileType::Crossing: {
			td.str = STR_LAI_ROAD_DESCRIPTION_ROAD_RAIL_LEVEL_CROSSING;
			rail_owner = GetTileOwner(tile);

			const RailTypeInfo *rti = GetRailTypeInfo(GetRailType(tile));
			td.railtype = rti->strings.name;
			td.rail_speed = rti->max_speed;

			break;
		}

		case RoadTileType::Depot:
			td.str = STR_LAI_ROAD_DESCRIPTION_ROAD_VEHICLE_DEPOT;
			td.build_date = Depot::GetByTile(tile)->build_date;
			break;

		default: {
			td.str = (road_rt != INVALID_ROADTYPE ? _road_tile_strings[to_underlying(GetRoadside(tile))] : STR_LAI_ROAD_DESCRIPTION_TRAMWAY);
			break;
		}
	}

	/* Now we have to discover, if the tile has only one owner or many:
	 *   - Find a first_owner of the tile. (Currently road or tram must be present, but this will break when the third type becomes available)
	 *   - Compare the found owner with the other owners, and test if they differ.
	 * Note: If road exists it will be the first_owner.
	 */
	Owner first_owner = (road_owner == INVALID_OWNER ? tram_owner : road_owner);
	bool mixed_owners = (tram_owner != INVALID_OWNER && tram_owner != first_owner) || (rail_owner != INVALID_OWNER && rail_owner != first_owner);

	if (mixed_owners) {
		/* Multiple owners */
		td.owner_type[0] = (rail_owner == INVALID_OWNER ? STR_NULL : STR_LAND_AREA_INFORMATION_RAIL_OWNER);
		td.owner[0] = rail_owner;
		td.owner_type[1] = (road_owner == INVALID_OWNER ? STR_NULL : STR_LAND_AREA_INFORMATION_ROAD_OWNER);
		td.owner[1] = road_owner;
		td.owner_type[2] = (tram_owner == INVALID_OWNER ? STR_NULL : STR_LAND_AREA_INFORMATION_TRAM_OWNER);
		td.owner[2] = tram_owner;
	} else {
		/* One to rule them all */
		td.owner[0] = first_owner;
	}
}

/**
 * Given the direction the road depot is pointing, this is the direction the
 * vehicle should be travelling in in order to enter the depot.
 */





/**
 * Checks the tile and returns whether the current player is allowed to convert the roadtype to another roadtype without taking ownership
 * @param owner the tile owner.
 * @param rtt Road/tram type.
 * @return whether the road is convertible
 */

/**
 * Convert the ownership of the RoadType of the tile if applicable
 * @param tile the tile of which convert ownership
 * @param num_pieces the count of the roadbits to assign to the new owner
 * @param owner the current owner of the RoadType
 * @param from_type the old road type
 * @param to_type the new road type
 */

/**
 * Convert one road subtype to another.
 * Not meant to convert from road to tram.
 *
 * @param flags operation to perform
 * @param tile end tile of road conversion drag
 * @param area_start start tile of drag
 * @param to_type new roadtype to convert to.
 * @return the cost of this operation or an error
 */


#include "rust/road_command_ffi.h"
#include "rust/road_command_services.h"
#include "rust/town_services.h"
#include "rust/rail_command_services.h"
#include "rust/command_cost_services.h"
#include "rust/command_cost_native_services.h"
#include "rust/company_services.h"
#include "rust/vehicle_services.h"
#include "rust/environment_services.h"
#include "rust/settings_services.h"
#include "rust/services_road.h"
#include "window_func.h"
static_assert(MP_VOID == 7 && DoCommandFlags{DoCommandFlag::AllTiles}.base() == 256);
static_assert(ROAD_DEPOT_TRACKBIT_FACTOR == 2 && ROAD_STOP_TRACKBIT_FACTOR == 2 && MAX_COMPANIES == 15);
static_assert(PR_BUILD_DEPOT_ROAD == 6 && PR_CLEAR_DEPOT_ROAD == 31 && PR_BUILD_FOUNDATION == 57);
static_assert(OWNER_TOWN.base() == 15 && OWNER_NONE.base() == 16 && INVALID_OWNER.base() == 255);
static_assert(FOUNDATION_NONE == 0 && FOUNDATION_LEVELED == 1 && FOUNDATION_INCLINED_X == 2 && FOUNDATION_INCLINED_Y == 3);
static CommandCost &RoadCost(OpenTTDCommandCost *cost) noexcept { return *reinterpret_cast<CommandCost *>(cost); }
static void RoadOwnedByTown(OpenTTDCommandCost *out, uint32_t tile) noexcept
{
	RoadCost(out).MakeError(STR_ERROR_OWNED_BY);
	if (RustIsLocalCompany()) RustEncodeOwnedBy(RoadCost(out), OWNER_TOWN, TileIndex{tile});
}
static OpenTTDRoadWorksEffect RoadNextEffect(uint32_t from) noexcept
{
	for (const EffectVehicle *v : EffectVehicle::Iterate(from)) return {.id = v->index.base(), .tile = TileVirtXY(v->x_pos, v->y_pos).base()};
	return {.id = UINT32_MAX, .tile = 0};
}
static void RoadIncludeVehicle(OpenTTDRoadConvertFrame *frame, OpenTTDVehicle *v) noexcept { include(frame->affected, v); }
const OpenTTDRoadCommandServices &GetRustRoadCommandServices() noexcept
{
	static const OpenTTDRoadCommandServices services{
		.physics = &GetRustGroundPhysicsServices(),
		.do_build = openttd_rust_road_do_build,
		.rail_commands = &GetRustRailCommandServices(), .town = &GetRustTownServices(), .authority = &GetRustTownAuthorityServices(),
		.strings = { .no_road = STR_ERROR_THERE_IS_NO_ROAD, .no_tram = STR_ERROR_THERE_IS_NO_TRAMWAY,
			.junction = STR_ERROR_ONEWAY_ROADS_CAN_T_HAVE_JUNCTION, .remove_road = STR_ERROR_MUST_REMOVE_ROAD_FIRST,
			.no_suitable_road = STR_ERROR_NO_SUITABLE_ROAD, .no_suitable_tram = STR_ERROR_NO_SUITABLE_TRAMWAY,
			.authority = STR_ERROR_LOCAL_AUTHORITY_REFUSES_TO_ALLOW_THIS },
		.nearest_tile = [](uint32_t index) noexcept -> OpenTTDRoadNearestTownTile {
			TileIndex tile{index};
			bool road = IsTileType(tile, MP_ROAD);
			bool depot = road && IsRoadDepot(tile);
			return {.road = road, .depot = depot, .town_owned = road && !depot && HasTownOwnedRoad(tile)};
		},
		.town_valid = [](uint16_t town) noexcept { return Town::IsValidID(TownID(town)); },
		.town_edge = [](const OpenTTDTownHandle *handle) noexcept -> OpenTTDRoadTownEdge {
			const Town *town = reinterpret_cast<const Town *>(handle);
			return {.tile = town->xy.base(), .radius = town->cache.squared_town_zone_radius[to_underlying(HouseZone::TownEdge)]};
		},
		.set_town = RustSetTownIndex,

		.extra_dynamite = RustExtraDynamite,
		.depot_slope = [](uint8_t dir, uint8_t slope) noexcept { return CanBuildDepotByTileh(static_cast<DiagDirection>(dir), static_cast<Slope>(slope)); },
		.make_depot = [](uint32_t tile, uint8_t owner, uint16_t depot, uint8_t dir, uint8_t rt) noexcept { MakeRoadDepot(TileIndex{tile}, Owner(owner), DepotID(depot), static_cast<DiagDirection>(dir), static_cast<RoadType>(rt)); },
		.depot_exit = [](uint32_t tile, uint8_t dir) noexcept { SetRoadDepotExitDirection(TileIndex{tile}, static_cast<DiagDirection>(dir)); },
		.next_effect = RoadNextEffect,
		.delete_effect = [](uint32_t id) noexcept { delete EffectVehicle::Get(VehicleID(id)); },
		.owned_by_town = RoadOwnedByTown,
		.scratch_cost = [](OpenTTDRoadCommandScratch *scratch) noexcept { return RustCommandCost(&scratch->ret.emplace()); },
		.conversion_scratch = [](OpenTTDRoadCommandScratch *scratch) noexcept -> OpenTTDRoadConversionScratch {
			auto &s = scratch->conversion.emplace();
			return {.frame = &s.frame, .area = &s.area, .error = RustCommandCost(&s.error), .ret = RustCommandCost(&s.ret)};
		},
		.include_vehicle = RoadIncludeVehicle,
		.affected_count = [](OpenTTDRoadConvertFrame *frame) noexcept { return frame->affected.size(); },
		.affected_at = [](OpenTTDRoadConvertFrame *frame, size_t i) noexcept { return frame->affected[i]; },
	};
	return services;
}
static const OpenTTDRoadCommandServices *RoadServices() noexcept { return &GetRustRoadCommandServices(); }
CommandCost CmdBuildRoad(DoCommandFlags flags, TileIndex tile, RoadBits bits, RoadType rt, DisallowedRoadDirections drd, TownID town)
{
	OpenTTDRoadCommandScratch nested;
	CommandCost cost, ret;
	openttd_rust_road_build(RoadServices(), &nested, RustCommandCost(&cost), RustCommandCost(&ret), flags.base(), tile.base(), bits, rt, drd, town.base());
	return cost;
}
CommandCost CmdBuildLongRoad(DoCommandFlags flags, TileIndex tile, TileIndex start, RoadType rt, Axis axis, DisallowedRoadDirections drd, bool start_half, bool end_half, bool is_ai)
{
	OpenTTDRoadCommandScratch nested;
	CommandCost cost, last, ret;
	openttd_rust_road_build_long(RoadServices(), &nested, RustCommandCost(&cost), RustCommandCost(&last), RustCommandCost(&ret), flags.base(), tile.base(), start.base(), rt, axis, drd, start_half, end_half, is_ai);
	return cost;
}
std::tuple<CommandCost, Money> CmdRemoveLongRoad(DoCommandFlags flags, TileIndex tile, TileIndex start, RoadType rt, Axis axis, bool start_half, bool end_half)
{
	OpenTTDRoadCommandScratch nested;
	CommandCost cost, last, ret, scratch;
	Money extra(openttd_rust_road_remove_long(RoadServices(), &nested, RustCommandCost(&cost), RustCommandCost(&last), RustCommandCost(&ret), RustCommandCost(&scratch), flags.base(), tile.base(), start.base(), rt, axis, start_half, end_half));
	return {std::move(cost), extra};
}
CommandCost CmdBuildRoadDepot(DoCommandFlags flags, TileIndex tile, RoadType rt, DiagDirection dir)
{
	CommandCost cost, ret;
	openttd_rust_road_build_depot(RoadServices(), RustCommandCost(&cost), RustCommandCost(&ret), flags.base(), tile.base(), rt, dir);
	return cost;
}
static CommandCost ClearTile_Road(TileIndex tile, DoCommandFlags flags)
{
	CommandCost cost, ret, scratch;
	openttd_rust_road_clear(RoadServices(), RustCommandCost(&cost), RustCommandCost(&ret), RustCommandCost(&scratch), flags.base(), tile.base());
	return cost;
}
void UpdateNearestTownForRoadTiles(bool invalidate)
{
	assert(!invalidate || _generating_world);
	openttd_rust_road_nearest_towns(RoadServices(), invalidate);
}
static CommandCost TerraformTile_Road(TileIndex tile, DoCommandFlags flags, int z_new, Slope slope)
{
	CommandCost cost, scratch;
	openttd_rust_road_terraform(RoadServices(), RustCommandCost(&cost), RustCommandCost(&scratch), flags.base(), tile.base(), z_new, slope);
	return cost;
}
CommandCost CmdConvertRoad(DoCommandFlags flags, TileIndex tile, TileIndex start, RoadType rt, bool diagonal)
{
	CommandCost cost, error, ret;
	OpenTTDRoadConvertFrame frame;
	OpenTTDRailConvertFrame area;
	openttd_rust_road_convert(RoadServices(), &frame, &area, RustCommandCost(&cost), RustCommandCost(&error), RustCommandCost(&ret), flags.base(), tile.base(), start.base(), rt, diagonal);
	return cost;
}
static CommandCost CheckBuildAbove_Road(TileIndex tile, DoCommandFlags flags, Axis, int)
{
	CommandCost cost;
	openttd_rust_road_build_above(RoadServices(), RustCommandCost(&cost), flags.base(), tile.base());
	return cost;
}

/** Tile callback functions for road tiles */
extern const TileTypeProcs _tile_type_road_procs = {
	DrawTile_Road,           // draw_tile_proc
	nullptr, // Rust movement owner dispatches directly.
	ClearTile_Road,          // clear_tile_proc
	nullptr,                    // add_accepted_cargo_proc
	GetTileDesc_Road,        // get_tile_desc_proc
	nullptr, // Rust movement owner dispatches directly.
	ClickTile_Road,          // click_tile_proc
	nullptr,                 // Rust scheduler calls the owner directly.
	nullptr,                 // Rust ownership dispatcher calls the owner directly.
	nullptr,                    // add_produced_cargo_proc
	nullptr, // Rust movement owner dispatches directly.
	nullptr, // Rust movement owner dispatches directly.
	TerraformTile_Road,      // terraform_tile_proc
	CheckBuildAbove_Road, // check_build_above_proc
};

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDRoadCommandServices.hpp"
bool CheckAbi_road_cmd() { return AbiServicesComplete(GetRustRoadCommandServices()); }
