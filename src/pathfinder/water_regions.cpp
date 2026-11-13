/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

 /** @file water_regions.cpp Handles dividing the water in the map into square regions to assist pathfinding. */

#include "../stdafx.h"
#include "../map_func.h"
#include "water_regions.h"
#include "../tilearea_type.h"
#include "../track_func.h"
#include "../transport_type.h"
#include "../landscape.h"
#include "../tunnelbridge_map.h"
#include "follow_track.hpp"
#include "../ship.h"
#include "../debug.h"
#include "../3rdparty/fmt/ranges.h"
#include "../core/convertible_through_base.hpp"
#include "../safeguards.h"

#include "../rust/water_regions_ffi.h"
#include "../rust/services_ffi.h"

using WaterOwner = std::unique_ptr<OpenTTDWaterRegions, decltype(&openttd_rust_water_destroy)>;
static WaterOwner _water_regions{nullptr, openttd_rust_water_destroy};

OpenTTDWaterRegions *GetRustWaterRegions() noexcept { return _water_regions.get(); }

WaterRegionDesc GetWaterRegionInfo(TileIndex tile)
{
	return WaterRegionDesc{static_cast<int>(TileX(tile) / WATER_REGION_EDGE_LENGTH), static_cast<int>(TileY(tile) / WATER_REGION_EDGE_LENGTH)};
}

WaterRegionPatchDesc GetWaterRegionPatchInfo(TileIndex tile)
{
	const auto region = GetWaterRegionInfo(tile);
	return WaterRegionPatchDesc{region.x, region.y, WaterRegionPatchLabel{openttd_rust_water_label(_water_regions.get(), &GetRustSharedServices(), tile.base())}};
}

void InvalidateWaterRegion(TileIndex tile)
{
	openttd_rust_water_invalidate(_water_regions.get(), &GetRustSharedServices(), tile.base());
}

void VisitWaterRegionPatchNeighbours(const WaterRegionPatchDesc &patch, VisitWaterRegionPatchCallback &callback)
{
	OpenTTDWaterVisit visit{};
	OpenTTDWaterPatch next{};
	while (openttd_rust_water_visit_next(_water_regions.get(), &GetRustSharedServices(), &visit, {patch.x, patch.y, patch.label.base()}, &next)) {
		callback(WaterRegionPatchDesc{next.x, next.y, WaterRegionPatchLabel{next.label}});
	}
}

void AllocateWaterRegions()
{
	_water_regions.reset(openttd_rust_water_new(&GetRustSharedServices(), Map::SizeX(), Map::SizeY()));
	Debug(map, 2, "Allocating {} x {} water regions", Map::SizeX() / WATER_REGION_EDGE_LENGTH, Map::SizeY() / WATER_REGION_EDGE_LENGTH);
}

void PrintWaterRegionDebugInfo(TileIndex tile)
{
	openttd_rust_water_print_debug_info(_water_regions.get(), &GetRustSharedServices(), tile.base());
}
