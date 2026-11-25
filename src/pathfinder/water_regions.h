/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

 /** @file water_regions.h Handles dividing the water in the map into regions to assist pathfinding. */

#ifndef WATER_REGIONS_H
#define WATER_REGIONS_H

#include "../tile_type.h"
#include "../map_func.h"

constexpr int WATER_REGION_EDGE_LENGTH = 16;

void InvalidateWaterRegion(TileIndex tile);


void AllocateWaterRegions();

void PrintWaterRegionDebugInfo(TileIndex tile);

#endif /* WATER_REGIONS_H */
