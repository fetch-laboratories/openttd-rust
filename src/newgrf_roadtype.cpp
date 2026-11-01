/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_roadtype.cpp NewGRF handling of road types. */

#include "stdafx.h"
#include "core/container_func.hpp"
#include "debug.h"
#include "newgrf_roadtype.h"
#include "newgrf_railtype.h"
#include "timer/timer_game_calendar.h"
#include "depot_base.h"
#include "town.h"
#include "tunnelbridge_map.h"

#include "rust/type_registry_ffi.h"

#include "safeguards.h"

/**
 * Variable 0x45 of road-/tram-/rail-types to query track types on a tile.
 *
 * Format: __RRttrr
 * - rr: Translated roadtype.
 * - tt: Translated tramtype.
 * - RR: Translated railtype.
 *
 * Special values for rr, tt, RR:
 * - 0xFF: Track not present on tile.
 * - 0xFE: Track present, but no matching entry in translation table.
 */
uint32_t GetTrackTypes(TileIndex tile, const GRFFile *grffile)
{
	uint8_t road = 0xFF;
	uint8_t tram = 0xFF;
	if (MayHaveRoad(tile)) {
		if (auto tt = GetRoadTypeRoad(tile); tt != INVALID_ROADTYPE) {
			road = GetReverseRoadTypeTranslation(tt, grffile);
			if (road == 0xFF) road = 0xFE;
		}
		if (auto tt = GetRoadTypeTram(tile); tt != INVALID_ROADTYPE) {
			tram = GetReverseRoadTypeTranslation(tt, grffile);
			if (tram == 0xFF) tram = 0xFE;
		}
	}
	uint8_t rail = 0xFF;
	if (auto tt = GetTileRailType(tile); tt != INVALID_RAILTYPE) {
		rail = GetReverseRailTypeTranslation(tt, grffile);
		if (rail == 0xFF) rail = 0xFE;
	}
	return road | tram << 8 | rail << 16;
}

/* virtual */ uint32_t RoadTypeScopeResolver::GetRandomBits() const
{
	uint tmp = CountBits(this->tile.base() + (TileX(this->tile) + TileY(this->tile)) * TILE_SIZE);
	return GB(tmp, 0, 2);
}

/* virtual */ uint32_t RoadTypeScopeResolver::GetVariable(uint8_t variable, [[maybe_unused]] uint32_t parameter, bool &available) const
{
	if (this->tile == INVALID_TILE) {
		switch (variable) {
			case 0x40: return 0;
			case 0x41: return 0;
			case 0x42: return 0;
			case 0x43: return TimerGameCalendar::date.base();
			case 0x44: return to_underlying(HouseZone::TownEdge);
			case 0x45: {
				RoadType rt = this->rti->Index();
				uint8_t local = GetReverseRoadTypeTranslation(rt, this->ro.grffile);
				if (local == 0xFF) local = 0xFE;
				if (RoadTypeIsRoad(rt)) {
					return 0xFFFF00 | local;
				} else {
					return 0xFF00FF | local << 8;
				}
			}
		}
	}

	switch (variable) {
		case 0x40: return GetTerrainType(this->tile, this->context);
		case 0x41: return 0;
		case 0x42: return IsLevelCrossingTile(this->tile) && IsCrossingBarred(this->tile);
		case 0x43:
			if (IsRoadDepotTile(this->tile)) return Depot::GetByTile(this->tile)->build_date.base();
			return TimerGameCalendar::date.base();
		case 0x44: {
			const Town *t = nullptr;
			if (IsRoadDepotTile(this->tile)) {
				t = Depot::GetByTile(this->tile)->town;
			} else {
				t = ClosestTownFromTile(this->tile, UINT_MAX);
			}
			return to_underlying(t != nullptr ? GetTownRadiusGroup(t, this->tile) : HouseZone::TownEdge);
		}
		case 0x45:
			return GetTrackTypes(this->tile, ro.grffile);
	}

	Debug(grf, 1, "Unhandled road type tile variable 0x{:X}", variable);

	available = false;
	return UINT_MAX;
}

GrfSpecFeature RoadTypeResolverObject::GetFeature() const
{
	RoadType rt = GetRoadTypeByLabel(this->roadtype_scope.rti->label, false);
	switch (GetRoadTramType(rt)) {
		case RTT_ROAD: return GSF_ROADTYPES;
		case RTT_TRAM: return GSF_TRAMTYPES;
		default: return GSF_INVALID;
	}
}

uint32_t RoadTypeResolverObject::GetDebugID() const
{
	return this->roadtype_scope.rti->label;
}

/**
 * Resolver object for road types.
 * @param rti Roadtype. nullptr in NewGRF Inspect window.
 * @param tile %Tile containing the track. For track on a bridge this is the southern bridgehead.
 * @param context Are we resolving sprites for the upper halftile, or on a bridge?
 * @param rtsg Roadpart of interest
 * @param param1 Extra parameter (first parameter of the callback, except roadtypes do not have callbacks).
 * @param param2 Extra parameter (second parameter of the callback, except roadtypes do not have callbacks).
 */
RoadTypeResolverObject::RoadTypeResolverObject(const RoadTypeInfo *rti, TileIndex tile, TileContext context, RoadTypeSpriteGroup rtsg, uint32_t param1, uint32_t param2)
	: ResolverObject(rti != nullptr ? rti->grffile[rtsg] : nullptr, CBID_NO_CALLBACK, param1, param2), roadtype_scope(*this, rti, tile, context)
{
	this->root_spritegroup = rti != nullptr ? rti->group[rtsg] : nullptr;
}

/**
 * Get the sprite to draw for the given tile.
 * @param rti The road type data (spec).
 * @param tile The tile to get the sprite for.
 * @param rtsg The type of sprite to draw.
 * @param content Where are we drawing the tile?
 * @param [out] num_results If not nullptr, return the number of sprites in the spriteset.
 * @return The sprite to draw.
 */
SpriteID GetCustomRoadSprite(const RoadTypeInfo *rti, TileIndex tile, RoadTypeSpriteGroup rtsg, TileContext context, uint *num_results)
{
	assert(rtsg < ROTSG_END);

	if (rti->group[rtsg] == nullptr) return 0;

	RoadTypeResolverObject object(rti, tile, context, rtsg);
	const auto *group = object.Resolve<ResultSpriteGroup>();
	if (group == nullptr || group->num_sprites == 0) return 0;

	if (num_results) *num_results = group->num_sprites;

	return group->sprite;
}

/**
 * Translate an index to the GRF-local road/tramtype-translation table into a RoadType.
 * @param rtt       Whether to index the road- or tramtype-table.
 * @param tracktype Index into GRF-local translation table.
 * @param grffile   Originating GRF file.
 * @return RoadType or INVALID_ROADTYPE if the roadtype is unknown.
 */
RoadType GetRoadTypeTranslation(RoadTramType rtt, uint8_t tracktype, const GRFFile *grffile)
{
	return openttd_rust_road_translate(rtt, tracktype, grffile);
}

/**
 * Perform a reverse roadtype lookup to get the GRF internal ID.
 * @param roadtype The global (OpenTTD) roadtype.
 * @param grffile The GRF to do the lookup for.
 * @return the GRF internal ID.
 */
uint8_t GetReverseRoadTypeTranslation(RoadType roadtype, const GRFFile *grffile)
{
	return openttd_rust_road_reverse(roadtype, grffile);
}


/**
 * Test if any saved road type labels are different to the currently loaded
 * road types. Road types stored in the map will be converted if necessary.
 */
void ConvertRoadTypes()
{
	openttd_rust_road_saved_convert(&GetRustSharedServices(), Map::Size());
}

/** Populate road type label list with current values. */
void SetCurrentRoadTypeLabelList()
{
	openttd_rust_road_saved_current();
}

void ClearRoadTypeLabelList()
{
	openttd_rust_road_saved_clear();
}
