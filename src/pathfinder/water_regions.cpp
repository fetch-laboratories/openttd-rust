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
#include "../rust/services_bridge.h"

static uint32_t WaterAqueduct(uint32_t tile) noexcept
{
	const TileIndex t{tile};
	return IsBridgeTile(t) && GetTunnelBridgeTransportType(t) == TRANSPORT_WATER ? RustOtherBridgeEnd(tile) : INVALID_TILE.base();
}

static void WaterDebug(uint8_t operation, int32_t x, int32_t y) noexcept
{
	if (operation == 0) Debug(map, 3, "Updating water region ({},{})", x, y);
	else Debug(map, 3, "Invalidated water region ({},{})", x, y);
}

static const OpenTTDWaterLeaves _water_leaves{.shared = &GetRustSharedServices(), .aqueduct = WaterAqueduct, .debug = WaterDebug};
using WaterOwner = std::unique_ptr<OpenTTDWaterRegions, decltype(&openttd_rust_water_destroy)>;
static WaterOwner _water_regions{nullptr, openttd_rust_water_destroy};

OpenTTDWaterRegionContext GetRustWaterRegionContext() noexcept { return {_water_regions.get(), &_water_leaves}; }

int CalculateWaterRegionPatchHash(const WaterRegionPatchDesc &patch)
{
	return patch.label.base() | (Map::SizeX() / WATER_REGION_EDGE_LENGTH * patch.y + patch.x) << 8;
}

TileIndex GetWaterRegionCenterTile(const WaterRegionDesc &region)
{
	return TileXY(region.x * WATER_REGION_EDGE_LENGTH + WATER_REGION_EDGE_LENGTH / 2, region.y * WATER_REGION_EDGE_LENGTH + WATER_REGION_EDGE_LENGTH / 2);
}

WaterRegionDesc GetWaterRegionInfo(TileIndex tile)
{
	return WaterRegionDesc{static_cast<int>(TileX(tile) / WATER_REGION_EDGE_LENGTH), static_cast<int>(TileY(tile) / WATER_REGION_EDGE_LENGTH)};
}

WaterRegionPatchDesc GetWaterRegionPatchInfo(TileIndex tile)
{
	const auto region = GetWaterRegionInfo(tile);
	return WaterRegionPatchDesc{region.x, region.y, WaterRegionPatchLabel{openttd_rust_water_label(_water_regions.get(), &_water_leaves, tile.base())}};
}

void InvalidateWaterRegion(TileIndex tile)
{
	if (!IsValidTile(tile)) return;
	openttd_rust_water_invalidate(_water_regions.get(), &_water_leaves, tile.base());
}

void VisitWaterRegionPatchNeighbours(const WaterRegionPatchDesc &patch, VisitWaterRegionPatchCallback &callback)
{
	using VisitOwner = std::unique_ptr<OpenTTDWaterVisit, decltype(&openttd_rust_water_visit_destroy)>;
	VisitOwner visit{openttd_rust_water_visit_new(_water_regions.get(), &_water_leaves, {patch.x, patch.y, patch.label.base()}), openttd_rust_water_visit_destroy};
	OpenTTDWaterPatch next{};
	while (openttd_rust_water_visit_next(_water_regions.get(), &_water_leaves, visit.get(), {patch.x, patch.y, patch.label.base()}, &next)) {
		callback(WaterRegionPatchDesc{next.x, next.y, WaterRegionPatchLabel{next.label}});
	}
}

/* Ship search calls the distinct cache visitor directly; no callback reenters its arena. */
void *ShipWaterVisitNew(OpenTTDWaterPatch patch) noexcept
{
	return openttd_rust_water_visit_new(_water_regions.get(), &_water_leaves, patch);
}
uint8_t ShipWaterVisitNext(void *cursor, OpenTTDWaterPatch patch, OpenTTDWaterPatch *out) noexcept
{
	return openttd_rust_water_visit_next(_water_regions.get(), &_water_leaves, static_cast<OpenTTDWaterVisit *>(cursor), patch, out);
}
void ShipWaterVisitDestroy(void *cursor) noexcept
{
	openttd_rust_water_visit_destroy(static_cast<OpenTTDWaterVisit *>(cursor));
}

void AllocateWaterRegions()
{
	_water_regions.reset(openttd_rust_water_new(Map::SizeX(), Map::SizeY()));
	Debug(map, 2, "Allocating {} x {} water regions", Map::SizeX() / WATER_REGION_EDGE_LENGTH, Map::SizeY() / WATER_REGION_EDGE_LENGTH);
}

void PrintWaterRegionDebugInfo(TileIndex tile)
{
	OpenTTDWaterSnapshot snapshot{};
	openttd_rust_water_snapshot(_water_regions.get(), &_water_leaves, tile.base(), &snapshot);
	const auto region = GetWaterRegionInfo(tile);
	Debug(map, 9, "Water region {},{} labels and edge traversability = ...", region.x, region.y);
	const size_t max_element_width = fmt::format("{}", snapshot.patches).size();
	std::string traversability = fmt::format("{:0{}b}", snapshot.edges[DIAGDIR_NW], WATER_REGION_EDGE_LENGTH);
	Debug(map, 9, "    {:{}}", fmt::join(traversability, " "), max_element_width);
	Debug(map, 9, "  +{:->{}}+", "", WATER_REGION_EDGE_LENGTH * (max_element_width + 1) + 1);
	for (int y = 0; y < WATER_REGION_EDGE_LENGTH; ++y) {
		std::string line{};
		for (int x = 0; x < WATER_REGION_EDGE_LENGTH; ++x) {
			const auto label = snapshot.labels[x + WATER_REGION_EDGE_LENGTH * y];
			if (label == 0) line = fmt::format("{:{}} {}", ".", max_element_width, line);
			else line = fmt::format("{:{}} {}", label, max_element_width, line);
		}
		Debug(map, 9, "{} | {}| {}", GB(snapshot.edges[DIAGDIR_SW], y, 1), line, GB(snapshot.edges[DIAGDIR_NE], y, 1));
	}
	Debug(map, 9, "  +{:->{}}+", "", WATER_REGION_EDGE_LENGTH * (max_element_width + 1) + 1);
	traversability = fmt::format("{:0{}b}", snapshot.edges[DIAGDIR_SE], WATER_REGION_EDGE_LENGTH);
	Debug(map, 9, "    {:{}}", fmt::join(traversability, " "), max_element_width);
}

#include "../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDWaterLeaves.hpp"
bool CheckAbi_water_regions() { return AbiServicesComplete(_water_leaves); }
