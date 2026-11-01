/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file water_cmd.cpp Handling of water tiles. */

#include "stdafx.h"
#include "landscape.h"
#include "viewport_func.h"
#include "command_func.h"
#include "town.h"
#include "news_func.h"
#include "depot_base.h"
#include "depot_func.h"
#include "water.h"
#include "industry_map.h"
#include "newgrf_canal.h"
#include "strings_func.h"
#include "vehicle_func.h"
#include "sound_func.h"
#include "company_func.h"
#include "clear_map.h"
#include "tree_map.h"
#include "aircraft.h"
#include "effectvehicle_func.h"
#include "tunnelbridge_map.h"
#include "station_base.h"
#include "ai/ai.hpp"
#include "game/game.hpp"
#include "core/random_func.hpp"
#include "core/backup_type.hpp"
#include "timer/timer_game_calendar.h"
#include "company_base.h"
#include "company_gui.h"
#include "newgrf_generic.h"
#include "industry.h"
#include "water_cmd.h"
#include "landscape_cmd.h"
#include "pathfinder/water_regions.h"

#include "table/strings.h"

#include "rust/station_command_ffi.h"
#include "rust/ground_services.h"
#include "rust/services_landscape.h"
#include "rust/environment_services.h"
#include "rust/company_adapter.h"
#include "rust/vehicle_services.h"
#include "rust/services_station.h"
#include "rail_cmd.h"
#include "rust/water_command_ffi.h"
#include "rust/rail_command_ffi.h"
#include "rust/command_services.h"
#include "rust/company_services.h"
#include "rust/services_rail.h"
#include <variant>
#include <optional>

#include "safeguards.h"


/**
 * Marks tile dirty if it is a canal or river tile.
 * Called to avoid glitches when flooding tiles next to canal tile.
 *
 * @param tile tile to check
 */
void ClearNeighbourNonFloodingStates(TileIndex tile)
{
	openttd_rust_clear_neighbour_flooding(tile.base(), Map::SizeX(), Map::SizeY(), &GetRustWaterGroundServices(), &GetRustSharedServices());
}

/**
 * Build a ship depot.
 * @param flags type of operation
 * @param tile tile where ship depot is built
 * @param axis depot orientation (Axis)
 * @return the cost of this operation or an error
 */
CommandCost CmdBuildShipDepot(DoCommandFlags flags, TileIndex tile, Axis axis)
{
	CommandCost cost, ret;
	openttd_rust_water_build_depot(&GetRustWaterCommandServices(), RustCommandCost(&cost), RustCommandCost(&ret), flags.base(), tile.base(), axis);
	return cost;
}


bool IsPossibleDockingTile(Tile t)
{
	return openttd_rust_station_possible_docking(&GetRustStationCommandServices(), TileIndex(t).base());
}

/**
 * Mark the supplied tile as a docking tile if it is suitable for docking.
 * Tiles surrounding the tile are tested to be docks with correct orientation.
 * @param t Tile to test.
 */
void CheckForDockingTile(TileIndex t)
{
	openttd_rust_station_check_docking(&GetRustStationCommandServices(), t.base());
}

void MakeWaterKeepingClass(TileIndex tile, Owner owner)
{
	openttd_rust_water_restore(tile.base(), owner.base(), &GetRustSharedServices(), &GetRustWaterGroundServices(), &GetRustWaterRestorationServices());
}
CommandCost CmdBuildLock(DoCommandFlags flags, TileIndex tile)
{
	CommandCost cost, ret;
	openttd_rust_water_build_lock(&GetRustWaterCommandServices(), RustCommandCost(&cost), RustCommandCost(&ret), flags.base(), tile.base());
	return cost;
}
void MakeRiverAndModifyDesertZoneAround(TileIndex tile)
{
	openttd_rust_water_make_river(&GetRustWaterCommandServices(), tile.base());
}
/* The selected command owns traversal; the established map iterator stays a
 * call-local native value. Variant construction has no per-command allocation. */
#include "rust/water_area_services.h"
// Direct Rust economy reads use these source Price indices.
static_assert(PR_BUILD_DEPOT_SHIP == 7 && PR_CLEAR_DEPOT_SHIP == 32 && PR_BUILD_LOCK == 64 && PR_CLEAR_LOCK == 65);
static_assert(PR_BUILD_CANAL == 60 && PR_CLEAR_CANAL == 61 && PR_CLEAR_WATER == 34 && PR_CLEAR_ROUGH == 23);
/* Rust water.rs and water_cmd.rs keep these water.h values and the three-part lock table. */
static_assert(RIVER_OFFSET_DESERT_DISTANCE == 5 && to_underlying(LockPart::End) == 3);
const OpenTTDWaterCommandServices &GetRustWaterCommandServices() noexcept
{
	static const OpenTTDWaterCommandServices services{
		.shared = &GetRustSharedServices(), .cost = &GetRustCommandCostServices(), .command = &GetRustCommandServices(), .water = &GetRustWaterGroundServices(),
		.rail = &GetRustRailCommandServices(),
		.water_type = RustWaterTileType,
		.lock_part = RustLockPart,
		.lock_direction = [](uint32_t t) noexcept { return GetLockDirection(TileIndex{t}); },
		.other_depot_tile = [](uint32_t t) noexcept { return GetOtherShipDepotTile(TileIndex{t}).base(); },
		.restoration = GetRustWaterRestorationServices(),
		.environment = GetRustEnvironmentServices(),
		.make_depot = [](uint32_t t, uint8_t o, uint16_t id, DepotPart part, Axis axis, WaterClass wc) noexcept { MakeShipDepot(TileIndex{t}, Owner{o}, DepotID{id}, part, axis, wc); },
		.make_lock_tile = [](uint32_t t, uint8_t o, LockPart part, DiagDirection dir, WaterClass wc) noexcept { MakeLockTile(TileIndex{t}, Owner{o}, part, dir, wc); },
		.area_begin = [](OpenTTDWaterArea *a, uint32_t tile, uint32_t start, bool diagonal) noexcept {
			if (diagonal) a->iter.emplace(std::in_place_type<DiagonalTileIterator>, TileIndex{tile}, TileIndex{start});
			else a->iter.emplace(std::in_place_type<OrthogonalTileIterator>, TileIndex{tile}, TileIndex{start});
		},
		.area_tile = [](OpenTTDWaterArea *a) noexcept { return std::visit([](auto &i) { return (*i).base(); }, *a->iter); },
		.area_next = [](OpenTTDWaterArea *a) noexcept { std::visit([](auto &i) { ++i; }, *a->iter); },
		.strings = {
			.built_on_water = STR_ERROR_MUST_BE_BUILT_ON_WATER,
			.demolish_bridge = STR_ERROR_MUST_DEMOLISH_BRIDGE_FIRST,
			.unsuitable = STR_ERROR_SITE_UNSUITABLE,
			.sloped_wrong = STR_ERROR_LAND_SLOPED_IN_WRONG_DIRECTION,
			.edge = STR_ERROR_TOO_CLOSE_TO_EDGE_OF_MAP,
			.low_bridge = STR_ERROR_BRIDGE_TOO_LOW_FOR_LOCK,
			.flat = STR_ERROR_FLAT_LAND_REQUIRED,
			.already_built = STR_ERROR_ALREADY_BUILT,
			.build_on_water = STR_ERROR_CAN_T_BUILD_ON_WATER,
			.demolish_building = STR_ERROR_BUILDING_MUST_BE_DEMOLISHED,
			.demolish_canal = STR_ERROR_MUST_DEMOLISH_CANAL_FIRST,
		},
	};
	return services;
}
CommandCost CmdBuildCanal(DoCommandFlags flags, TileIndex tile, TileIndex start_tile, WaterClass wc, bool diagonal)
{
	OpenTTDWaterArea area;
	CommandCost cost, ret;
	openttd_rust_water_build_canal(&GetRustWaterCommandServices(), RustCommandCost(&cost), RustCommandCost(&ret), &area, flags.base(), tile.base(), start_tile.base(), wc, diagonal);
	return cost;
}
static CommandCost ClearTile_Water(TileIndex tile, DoCommandFlags flags)
{
	CommandCost cost, ret;
	openttd_rust_water_clear(&GetRustWaterCommandServices(), RustCommandCost(&cost), RustCommandCost(&ret), flags.base(), tile.base());
	return cost;
}


/**
 * return true if a tile is a water tile wrt. a certain direction.
 *
 * @param tile The tile of interest.
 * @param from The direction of interest.
 * @return true iff the tile is water in the view of 'from'.
 *
 */
bool IsWateredTile(TileIndex tile, Direction from)
{
	switch (GetTileType(tile)) {
		case MP_WATER:
			switch (GetWaterTileType(tile)) {
				default: NOT_REACHED();
				case WaterTileType::Depot: case WaterTileType::Clear: return true;
				case WaterTileType::Lock: return DiagDirToAxis(GetLockDirection(tile)) == DiagDirToAxis(DirToDiagDir(from));

				case WaterTileType::Coast:
					switch (GetTileSlope(tile)) {
						case SLOPE_W: return (from == DIR_SE) || (from == DIR_E) || (from == DIR_NE);
						case SLOPE_S: return (from == DIR_NE) || (from == DIR_N) || (from == DIR_NW);
						case SLOPE_E: return (from == DIR_NW) || (from == DIR_W) || (from == DIR_SW);
						case SLOPE_N: return (from == DIR_SW) || (from == DIR_S) || (from == DIR_SE);
						default: return false;
					}
			}

		case MP_RAILWAY:
			if (GetRailGroundType(tile) == RailGroundType::HalfTileWater) {
				assert(IsPlainRail(tile));
				switch (GetTileSlope(tile)) {
					case SLOPE_W: return (from == DIR_SE) || (from == DIR_E) || (from == DIR_NE);
					case SLOPE_S: return (from == DIR_NE) || (from == DIR_N) || (from == DIR_NW);
					case SLOPE_E: return (from == DIR_NW) || (from == DIR_W) || (from == DIR_SW);
					case SLOPE_N: return (from == DIR_SW) || (from == DIR_S) || (from == DIR_SE);
					default: return false;
				}
			}
			return false;

		case MP_STATION:
			if (IsOilRig(tile)) {
				/* Do not draw waterborders inside of industries.
				 * Note: There is no easy way to detect the industry of an oilrig tile. */
				TileIndex src_tile = tile + TileOffsByDir(from);
				if ((IsTileType(src_tile, MP_STATION) && IsOilRig(src_tile)) ||
				    (IsTileType(src_tile, MP_INDUSTRY))) return true;

				return IsTileOnWater(tile);
			}
			return (IsDock(tile) && IsTileFlat(tile)) || IsBuoy(tile);

		case MP_INDUSTRY: {
			/* Do not draw waterborders inside of industries.
			 * Note: There is no easy way to detect the industry of an oilrig tile. */
			TileIndex src_tile = tile + TileOffsByDir(from);
			if ((IsTileType(src_tile, MP_STATION) && IsOilRig(src_tile)) ||
			    (IsTileType(src_tile, MP_INDUSTRY) && GetIndustryIndex(src_tile) == GetIndustryIndex(tile))) return true;

			return IsTileOnWater(tile);
		}

		case MP_OBJECT: return IsTileOnWater(tile);

		case MP_TUNNELBRIDGE: return GetTunnelBridgeTransportType(tile) == TRANSPORT_WATER && ReverseDiagDir(GetTunnelBridgeDirection(tile)) == DirToDiagDir(from);

		case MP_VOID: return true; // consider map border as water, esp. for rivers

		default:          return false;
	}
}

/**
 * Draw a water sprite, potentially with a NewGRF-modified sprite offset.
 * @param base    Sprite base.
 * @param offset  Sprite offset.
 * @param feature The type of sprite that is drawn.
 * @param tile    Tile index to draw.
 */
static void DrawWaterSprite(SpriteID base, uint offset, CanalFeature feature, TileIndex tile)
{
	if (base != SPR_FLAT_WATER_TILE) {
		/* Only call offset callback if the sprite is NewGRF-provided. */
		offset = GetCanalSpriteOffset(feature, tile, offset);
	}
	DrawGroundSprite(base + offset, PAL_NONE);
}

/**
 * Draw canal or river edges.
 * @param canal  True if canal edges should be drawn, false for river edges.
 * @param offset Sprite offset.
 * @param tile   Tile to draw.
 */
static void DrawWaterEdges(bool canal, uint offset, TileIndex tile)
{
	CanalFeature feature;
	SpriteID base = 0;
	if (canal) {
		feature = CF_DIKES;
		base = GetCanalSprite(CF_DIKES, tile);
		if (base == 0) base = SPR_CANAL_DIKES_BASE;
	} else {
		feature = CF_RIVER_EDGE;
		base = GetCanalSprite(CF_RIVER_EDGE, tile);
		if (base == 0) return; // Don't draw if no sprites provided.
	}

	uint wa;

	/* determine the edges around with water. */
	wa  = IsWateredTile(TileAddXY(tile, -1,  0), DIR_SW) << 0;
	wa += IsWateredTile(TileAddXY(tile,  0,  1), DIR_NW) << 1;
	wa += IsWateredTile(TileAddXY(tile,  1,  0), DIR_NE) << 2;
	wa += IsWateredTile(TileAddXY(tile,  0, -1), DIR_SE) << 3;

	if (!(wa & 1)) DrawWaterSprite(base, offset,     feature, tile);
	if (!(wa & 2)) DrawWaterSprite(base, offset + 1, feature, tile);
	if (!(wa & 4)) DrawWaterSprite(base, offset + 2, feature, tile);
	if (!(wa & 8)) DrawWaterSprite(base, offset + 3, feature, tile);

	/* right corner */
	switch (wa & 0x03) {
		case 0: DrawWaterSprite(base, offset + 4, feature, tile); break;
		case 3: if (!IsWateredTile(TileAddXY(tile, -1, 1), DIR_W)) DrawWaterSprite(base, offset + 8, feature, tile); break;
	}

	/* bottom corner */
	switch (wa & 0x06) {
		case 0: DrawWaterSprite(base, offset + 5, feature, tile); break;
		case 6: if (!IsWateredTile(TileAddXY(tile, 1, 1), DIR_N)) DrawWaterSprite(base, offset + 9, feature, tile); break;
	}

	/* left corner */
	switch (wa & 0x0C) {
		case  0: DrawWaterSprite(base, offset + 6, feature, tile); break;
		case 12: if (!IsWateredTile(TileAddXY(tile, 1, -1), DIR_E)) DrawWaterSprite(base, offset + 10, feature, tile); break;
	}

	/* upper corner */
	switch (wa & 0x09) {
		case 0: DrawWaterSprite(base, offset + 7, feature, tile); break;
		case 9: if (!IsWateredTile(TileAddXY(tile, -1, -1), DIR_S)) DrawWaterSprite(base, offset + 11, feature, tile); break;
	}
}

/** Draw a plain sea water tile with no edges */
static void DrawSeaWater(TileIndex)
{
	DrawGroundSprite(SPR_FLAT_WATER_TILE, PAL_NONE);
}

/** draw a canal styled water tile with dikes around */
static void DrawCanalWater(TileIndex tile)
{
	SpriteID image = SPR_FLAT_WATER_TILE;
	if (HasBit(_water_feature[CF_WATERSLOPE].flags, CFF_HAS_FLAT_SPRITE)) {
		/* First water slope sprite is flat water. */
		image = GetCanalSprite(CF_WATERSLOPE, tile);
		if (image == 0) image = SPR_FLAT_WATER_TILE;
	}
	DrawWaterSprite(image, 0, CF_WATERSLOPE, tile);

	DrawWaterEdges(true, 0, tile);
}

#include "table/water_land.h"

/**
 * Draw a build sprite sequence for water tiles.
 * If buildings are invisible, nothing will be drawn.
 * @param ti      Tile info.
 * @param seq Sprite sequence to draw.
 * @param base    Base sprite.
 * @param offset  Additional sprite offset.
 * @param palette Palette to use.
 */
static void DrawWaterTileStruct(const TileInfo *ti, std::span<const DrawTileSeqStruct> seq, SpriteID base, uint offset, PaletteID palette, CanalFeature feature)
{
	/* Don't draw if buildings are invisible. */
	if (IsInvisibilitySet(TO_BUILDINGS)) return;

	for (const DrawTileSeqStruct &dtss : seq) {
		uint tile_offs = offset + dtss.image.sprite;
		if (feature < CF_END) tile_offs = GetCanalSpriteOffset(feature, ti->tile, tile_offs);
		AddSortableSpriteToDraw(base + tile_offs, palette, *ti, dtss, IsTransparencySet(TO_BUILDINGS));
	}
}

/** Draw a lock tile. */
static void DrawWaterLock(const TileInfo *ti)
{
	LockPart part = GetLockPart(ti->tile);
	const DrawTileSprites &dts = _lock_display_data[to_underlying(part)][GetLockDirection(ti->tile)];

	/* Draw ground sprite. */
	SpriteID image = dts.ground.sprite;

	SpriteID water_base = GetCanalSprite(CF_WATERSLOPE, ti->tile);
	if (water_base == 0) {
		/* Use default sprites. */
		water_base = SPR_CANALS_BASE;
	} else if (HasBit(_water_feature[CF_WATERSLOPE].flags, CFF_HAS_FLAT_SPRITE)) {
		/* NewGRF supplies a flat sprite as first sprite. */
		if (image == SPR_FLAT_WATER_TILE) {
			image = water_base;
		} else {
			image++;
		}
	}

	if (image < 5) image += water_base;
	DrawGroundSprite(image, PAL_NONE);

	/* Draw structures. */
	uint     zoffs = 0;
	SpriteID base  = GetCanalSprite(CF_LOCKS, ti->tile);

	if (base == 0) {
		/* If no custom graphics, use defaults. */
		base = SPR_LOCK_BASE;
		uint8_t z_threshold = part == LockPart::Upper ? 8 : 0;
		zoffs = ti->z > z_threshold ? 24 : 0;
	}

	DrawWaterTileStruct(ti, dts.GetSequence(), base, zoffs, PAL_NONE, CF_LOCKS);
}

/** Draw a ship depot tile. */
static void DrawWaterDepot(const TileInfo *ti)
{
	DrawWaterClassGround(ti);
	DrawWaterTileStruct(ti, _shipdepot_display_data[GetShipDepotAxis(ti->tile)][to_underlying(GetShipDepotPart(ti->tile))].seq, 0, 0, GetCompanyPalette(GetTileOwner(ti->tile)), CF_END);
}

static void DrawRiverWater(const TileInfo *ti)
{
	SpriteID image = SPR_FLAT_WATER_TILE;
	uint     offset = 0;
	uint     edges_offset = 0;

	if (ti->tileh != SLOPE_FLAT || HasBit(_water_feature[CF_RIVER_SLOPE].flags, CFF_HAS_FLAT_SPRITE)) {
		image = GetCanalSprite(CF_RIVER_SLOPE, ti->tile);
		if (image == 0) {
			switch (ti->tileh) {
				case SLOPE_NW: image = SPR_WATER_SLOPE_Y_DOWN; break;
				case SLOPE_SW: image = SPR_WATER_SLOPE_X_UP;   break;
				case SLOPE_SE: image = SPR_WATER_SLOPE_Y_UP;   break;
				case SLOPE_NE: image = SPR_WATER_SLOPE_X_DOWN; break;
				default:       image = SPR_FLAT_WATER_TILE;    break;
			}
		} else {
			/* Flag bit 0 indicates that the first sprite is flat water. */
			offset = HasBit(_water_feature[CF_RIVER_SLOPE].flags, CFF_HAS_FLAT_SPRITE) ? 1 : 0;

			switch (ti->tileh) {
				case SLOPE_SE:              edges_offset += 12; break;
				case SLOPE_NE: offset += 1; edges_offset += 24; break;
				case SLOPE_SW: offset += 2; edges_offset += 36; break;
				case SLOPE_NW: offset += 3; edges_offset += 48; break;
				default:       offset  = 0; break;
			}

			offset = GetCanalSpriteOffset(CF_RIVER_SLOPE, ti->tile, offset);
		}
	}

	DrawGroundSprite(image + offset, PAL_NONE);

	/* Draw river edges if available. */
	DrawWaterEdges(false, edges_offset, ti->tile);
}

void DrawShoreTile(Slope tileh)
{
	/* Converts the enum Slope into an offset based on SPR_SHORE_BASE.
	 * This allows to calculate the proper sprite to display for this Slope */
	static const uint8_t tileh_to_shoresprite[32] = {
		0, 1, 2, 3, 4, 16, 6, 7, 8, 9, 17, 11, 12, 13, 14, 0,
		0, 0, 0, 0, 0,  0, 0, 0, 0, 0,  0,  5,  0, 10, 15, 0,
	};

	assert(!IsHalftileSlope(tileh)); // Halftile slopes need to get handled earlier.
	assert(tileh != SLOPE_FLAT);     // Shore is never flat

	assert((tileh != SLOPE_EW) && (tileh != SLOPE_NS)); // No suitable sprites for current flooding behaviour

	DrawGroundSprite(SPR_SHORE_BASE + tileh_to_shoresprite[tileh], PAL_NONE);
}

void DrawWaterClassGround(const TileInfo *ti)
{
	switch (GetWaterClass(ti->tile)) {
		case WaterClass::Sea:   DrawSeaWater(ti->tile); break;
		case WaterClass::Canal: DrawCanalWater(ti->tile); break;
		case WaterClass::River: DrawRiverWater(ti); break;
		default: NOT_REACHED();
	}
}

static void DrawTile_Water(TileInfo *ti)
{
	switch (GetWaterTileType(ti->tile)) {
		case WaterTileType::Clear:
			DrawWaterClassGround(ti);
			/* A plain water tile can be traversed in any direction, so setting blocked pillars here would mean all bridges
			 * with edges would have no pillars above water. Instead prefer current behaviour of ships passing through. */
			DrawBridgeMiddle(ti, {});
			break;

		case WaterTileType::Coast: {
			DrawShoreTile(ti->tileh);
			DrawBridgeMiddle(ti, {});
			break;
		}

		case WaterTileType::Lock:
			DrawWaterLock(ti);
			DrawBridgeMiddle(ti, DiagDirToAxis(GetLockDirection(ti->tile)) == AXIS_X
				? BridgePillarFlags{BridgePillarFlag::EdgeNE, BridgePillarFlag::EdgeSW}
				: BridgePillarFlags{BridgePillarFlag::EdgeNW, BridgePillarFlag::EdgeSE});
			break;

		case WaterTileType::Depot:
			DrawWaterDepot(ti);
			break;
	}
}

void DrawShipDepotSprite(int x, int y, Axis axis, DepotPart part)
{
	const DrawTileSprites &dts = _shipdepot_display_data[axis][to_underlying(part)];

	DrawSprite(dts.ground.sprite, dts.ground.pal, x, y);
	DrawOrigTileSeqInGUI(x, y, &dts, GetCompanyPalette(_local_company));
}




static void GetTileDesc_Water(TileIndex tile, TileDesc &td)
{
	switch (GetWaterTileType(tile)) {
		case WaterTileType::Clear:
			switch (GetWaterClass(tile)) {
				case WaterClass::Sea:   td.str = STR_LAI_WATER_DESCRIPTION_WATER; break;
				case WaterClass::Canal: td.str = STR_LAI_WATER_DESCRIPTION_CANAL; break;
				case WaterClass::River: td.str = STR_LAI_WATER_DESCRIPTION_RIVER; break;
				default: NOT_REACHED();
			}
			break;
		case WaterTileType::Coast: td.str = STR_LAI_WATER_DESCRIPTION_COAST_OR_RIVERBANK; break;
		case WaterTileType::Lock : td.str = STR_LAI_WATER_DESCRIPTION_LOCK;               break;
		case WaterTileType::Depot:
			td.str = STR_LAI_WATER_DESCRIPTION_SHIP_DEPOT;
			td.build_date = Depot::GetByTile(tile)->build_date;
			break;
		default: NOT_REACHED();
	}

	td.owner[0] = GetTileOwner(tile);
}

/**
 * Handle the flooding of a vehicle. This sets the vehicle state to crashed,
 * creates a newsitem and dirties the necessary windows.
 * @param v The vehicle to flood.
 */
/* Only leaves remain native. Virtual Crash and event insertion can reenter Rust,
 * but queueing AI/Game events never executes a script VM; allocation exceptions
 * terminate under noexcept. Commands use independent lazy stack result contexts. */
const OpenTTDWaterGroundServices &GetRustWaterGroundServices() noexcept
{
	static_assert(AIR_SHADOW == 4 && to_underlying(VehState::Crashed) == 7 && SND_12_EXPLOSION == 16 && OWNER_WATER.base() == 17 && VEH_AIRCRAFT == 3 && TREE_GROUND_SHORE == 3);
	static const OpenTTDWaterGroundServices services{
		.rail = &GetRustRailGroundServices(), .clear = &GetRustClearServices(), .vehicles = &GetRustVehicleServices(), .costs = &GetRustGroundCostServices(), .company = &GetRustCompanyServices(),
		.signal_context = GetRustSignalContext,
		.non_flooding = [](uint32_t t) noexcept { return IsNonFloodingWaterTile(TileIndex{t}); },
		.set_non_flooding = [](uint32_t t, bool value) noexcept { SetNonFloodingWaterTile(TileIndex{t}, value); },
		.coast = RustIsCoast,
		.set_tree_ground = RustSetTreeGroundDensity,
		.make_sea = [](uint32_t t) noexcept { MakeSea(TileIndex{t}); },
		.buoy_or_dock = [](uint32_t t) noexcept { return IsBuoy(TileIndex{t}) || IsDock(TileIndex{t}); },
		.canal_or_river = RustCanalOrRiver,
		.region_context = GetRustWaterRegionContext,
		.remove_rail = [](OpenTTDCommandCost *out, uint32_t t, Track track) noexcept { *NativeGroundCost(out) = Command<CMD_REMOVE_SINGLE_RAIL>::Do(DoCommandFlag::Execute, TileIndex{t}, track); },
		.airport_tile = RustAirportTile,
		.airport = [](uint32_t t) noexcept { const Station *st = RustStationByTile(t); return OpenTTDAirportGround{.station = st, .tile = RustStationAirportTile(st), .width = RustStationAirportWidth(st), .height = RustStationAirportHeight(st)}; },
		.airport_member = RustStationAirportMember,
		.airport_z = [](const AirportFTAClass *fta) noexcept { return int32_t{RustAirportDeltaZ(fta)} + 1; },
		.victim = [](OpenTTDVehicle *v) noexcept { Vehicle *u = VehicleOf(v); return OpenTTDFloodVictim{.status = RustVehicleStatus(u), .type = RustVehicleVehicleType(u)}; },
		.aircraft_pose = [](OpenTTDVehicle *v) noexcept { Vehicle *u = VehicleOf(v); return OpenTTDAircraftFloodPose{.tile = RustVehicleTile(u), .z = RustVehicleZ(u), .subtype = RustVehicleSubtype(u)}; },
		.ground_pose = [](OpenTTDVehicle *v) noexcept { Vehicle *u = VehicleOf(v); return OpenTTDGroundFloodPose{.z = RustVehicleZ(u), .first = RustVehicle(RustVehicleFirst(u))}; },
		.crash = [](OpenTTDVehicle *v) noexcept { return VehicleOf(v)->Crash(true); },
		.ai_event = [](OpenTTDVehicle *v, uint32_t victims) noexcept { Vehicle *u = VehicleOf(v); AI::NewEvent(u->GetOwner(), new ScriptEventVehicleCrashed(u->index, u->GetTile(), ScriptEventVehicleCrashed::CRASH_FLOODED, victims, u->GetOwner())); },
		.game_event = [](OpenTTDVehicle *v, uint32_t victims) noexcept { Vehicle *u = VehicleOf(v); Game::NewEvent(new ScriptEventVehicleCrashed(u->index, u->GetTile(), ScriptEventVehicleCrashed::CRASH_FLOODED, victims, u->GetOwner())); },
		.news = [](OpenTTDVehicle *v, uint32_t victims) noexcept { AddTileNewsItem(GetEncodedString(STR_NEWS_DISASTER_FLOOD_VEHICLE, victims), NewsType::Accident, TileIndex{RustVehicleTile(VehicleOf(v))}); },
	};
	return services;
}
FloodingBehaviour GetFloodingBehaviour(TileIndex tile)
{
	return openttd_rust_flooding_behaviour(tile.base(), &GetRustWaterGroundServices(), &GetRustSharedServices());
}
void TileLoop_Water(uint32_t tile) noexcept
{
	OpenTTDGroundCall call;
	openttd_rust_water_tile_loop(tile, Map::SizeX(), Map::SizeY(), &call, &GetRustWaterGroundServices(), &GetRustSharedServices());
}



static bool ClickTile_Water(TileIndex tile)
{
	if (GetWaterTileType(tile) == WaterTileType::Depot) {
		ShowDepotWindow(GetShipDepotNorthTile(tile), VEH_SHIP);
		return true;
	}
	return false;
}



static CommandCost TerraformTile_Water(TileIndex tile, DoCommandFlags flags, int, Slope)
{
	CommandCost cost;
	openttd_rust_water_terraform(&GetRustWaterCommandServices(), RustCommandCost(&cost), flags.base(), tile.base());
	return cost;
}
static CommandCost CheckBuildAbove_Water(TileIndex tile, DoCommandFlags flags, Axis, int height)
{
	CommandCost cost;
	openttd_rust_water_build_above(&GetRustWaterCommandServices(), RustCommandCost(&cost), flags.base(), tile.base(), height);
	return cost;
}


extern const TileTypeProcs _tile_type_water_procs = {
	DrawTile_Water,           // draw_tile_proc
	nullptr, // Rust movement owner dispatches directly.
	ClearTile_Water,          // clear_tile_proc
	nullptr,                     // add_accepted_cargo_proc
	GetTileDesc_Water,        // get_tile_desc_proc
	nullptr, // Rust movement owner dispatches directly.
	ClickTile_Water,          // click_tile_proc
	TileLoop_Water,           // tile_loop_proc
	nullptr,                 // Rust ownership dispatcher calls the owner directly.
	nullptr,                     // add_produced_cargo_proc
	nullptr, // Rust movement owner dispatches directly.
	nullptr, // Rust movement owner dispatches directly.
	TerraformTile_Water,      // terraform_tile_proc
	CheckBuildAbove_Water, // check_build_above_proc
};

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDWaterCommandServices.hpp"
#include "abi-visit-OpenTTDEnvironmentServices.hpp"
#include "abi-visit-OpenTTDWaterRestorationServices.hpp"
bool CheckAbi_water_cmd()
{
	return AbiServicesComplete(GetRustWaterCommandServices().restoration) && AbiServicesComplete(GetRustWaterCommandServices()) && AbiServicesComplete(GetRustWaterCommandServices().environment);
}
