/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_map.h Canonical native map reads shared by service bundles. */
#ifndef RUST_SERVICES_MAP_H
#define RUST_SERVICES_MAP_H

#include "../landscape.h"
#include "../road.h"
#include "../road_map.h"
#include "../roadstop_base.h"
#include "../station_map.h"

inline uint8_t SharedTileType(uint32_t tile) noexcept { return GetTileType(TileIndex{tile}); }
inline uint16_t MapSvcStationIndex(uint32_t tile) noexcept { return GetStationIndex(TileIndex(tile)).base(); }
inline bool MapSvcRoadIsLevelCrossing(uint32_t tile) noexcept { return IsLevelCrossing(TileIndex(tile)); }
inline bool MapSvcIsLevelCrossing(uint32_t tile) noexcept { return SharedTileType(tile) == MP_ROAD && MapSvcRoadIsLevelCrossing(tile); }
inline bool Shared_is_road_depot(uint32_t tile) noexcept { return IsRoadDepotTile(TileIndex{tile}); }
inline uint8_t Shared_road_depot_direction(uint32_t tile) noexcept { return GetRoadDepotDirection(TileIndex{tile}); }
inline bool Shared_is_drive_through_stop(uint32_t tile) noexcept { return IsDriveThroughStopTile(TileIndex{tile}); }
inline bool Shared_is_normal_road(uint32_t tile) noexcept { return IsNormalRoadTile(TileIndex{tile}); }
inline uint8_t Shared_disallowed_road_directions(uint32_t tile) noexcept { return GetDisallowedRoadDirections(TileIndex{tile}); }
inline bool Shared_is_bay_stop(uint32_t tile) noexcept { return IsBayRoadStopTile(TileIndex{tile}); }
inline uint8_t Shared_bay_stop_direction(uint32_t tile) noexcept { return GetBayRoadStopDir(TileIndex{tile}); }
inline bool SharedRoadTypeIsTram(RoadType roadtype) noexcept { return RoadTypeIsTram(roadtype); }
inline RoadStopType SharedRoadStopType(uint32_t tile) noexcept { return GetRoadStopType(TileIndex(tile)); }
/** Width adapter for the existing road service table; the typed read stays above. */
inline uint8_t SharedRoadStopTypeByte(uint32_t tile) noexcept { return to_underlying(SharedRoadStopType(tile)); }
inline RoadStop *RustRoadStopAt(TileIndex tile, RoadStopType type) noexcept { return RoadStop::GetByTile(tile, type); }
inline RoadStop *SharedRoadStopAt(uint32_t tile) noexcept { return RustRoadStopAt(TileIndex(tile), SharedRoadStopType(tile)); }
inline bool RoadSvcContinuation(uint32_t tile, uint32_t next) noexcept
{
	return RoadStop::IsDriveThroughRoadStopContinuation(TileIndex(tile), TileIndex(next));
}
inline int32_t SharedSlopePixelZ(int32_t x, int32_t y, bool ground_vehicle) noexcept { return GetSlopePixelZ(x, y, ground_vehicle); }

#endif
