/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_map.h Canonical native map reads shared by service bundles. */
#ifndef RUST_SERVICES_MAP_H
#define RUST_SERVICES_MAP_H

#include "services_ffi.h"
#include "../landscape.h"
#include "../animated_tile_func.h"
#include "../depot_map.h"
#include "../road.h"
#include "../road_map.h"
#include "../station_map.h"
#include "../tunnel_map.h"

inline uint8_t OpenTTDAnimationFrame(uint32_t tile) noexcept { return GetAnimationFrame(TileIndex(tile)); }
inline uint RustClosestWaterDistance(uint32_t tile, bool water) noexcept { return GetClosestWaterDistance(TileIndex(tile), water); }
inline uint32_t RustTileAddByDiagDir(uint32_t tile, uint8_t dir) noexcept { return TileAddByDiagDir(TileIndex{tile}, static_cast<DiagDirection>(dir)).base(); }
inline bool RustIsTunnelTile(uint32_t tile) noexcept { return IsTunnelTile(TileIndex(tile)); }
inline bool SharedIsDepotTile(uint32_t tile) noexcept { return IsDepotTile(TileIndex(tile)); }
inline uint32_t SharedTileAddWrap(uint32_t tile, int32_t x, int32_t y) noexcept { return TileAddWrap(TileIndex(tile), x, y).base(); }
inline uint8_t SharedTileType(uint32_t tile) noexcept { return GetTileType(TileIndex{tile}); }
inline uint32_t RustTileVirtXY(int32_t x, int32_t y) noexcept { return TileVirtXY(static_cast<uint>(x), static_cast<uint>(y)).base(); }
inline bool SharedHasRoadWorks(uint32_t tile) noexcept { return HasRoadWorks(TileIndex{tile}); }
inline uint16_t MapSvcStationIndex(uint32_t tile) noexcept { return GetStationIndex(TileIndex(tile)).base(); }
inline bool MapSvcRoadIsLevelCrossing(uint32_t tile) noexcept { return IsLevelCrossing(TileIndex(tile)); }
inline bool MapSvcIsLevelCrossing(uint32_t tile) noexcept { return SharedTileType(tile) == MP_ROAD && MapSvcRoadIsLevelCrossing(tile); }
inline bool Shared_is_road_depot(uint32_t tile) noexcept { return IsRoadDepotTile(TileIndex{tile}); }
inline uint8_t Shared_road_depot_direction(uint32_t tile) noexcept { return GetRoadDepotDirection(TileIndex{tile}); }
inline bool Shared_is_drive_through_stop(uint32_t tile) noexcept { return IsDriveThroughStopTile(TileIndex{tile}); }
inline bool Shared_is_normal_road_tile(uint32_t tile) noexcept { return IsNormalRoadTile(TileIndex{tile}); }
inline uint8_t Shared_disallowed_road_directions(uint32_t tile) noexcept { return GetDisallowedRoadDirections(TileIndex{tile}); }
inline bool Shared_is_bay_stop(uint32_t tile) noexcept { return IsBayRoadStopTile(TileIndex{tile}); }
inline uint8_t Shared_bay_stop_direction(uint32_t tile) noexcept { return GetBayRoadStopDir(TileIndex{tile}); }
inline bool SharedRoadTypeIsTram(RoadType roadtype) noexcept { return RoadTypeIsTram(roadtype); }
inline RoadStopType SharedRoadStopType(uint32_t tile) noexcept { return GetRoadStopType(TileIndex(tile)); }
inline int32_t SharedSlopePixelZ(int32_t x, int32_t y, bool ground_vehicle) noexcept { return GetSlopePixelZ(x, y, ground_vehicle); }

inline uint32_t OpenTTDDistanceManhattan(uint32_t from, uint32_t to) noexcept { return DistanceManhattan(TileIndex(from), TileIndex(to)); }
inline uint32_t OpenTTDDistanceSquare(uint32_t from, uint32_t to) noexcept { return DistanceSquare(TileIndex(from), TileIndex(to)); }
/* Retain the existing orders-table carrier while sharing the typed leaves. */
inline uint64_t OpenTTDLegacyOrderDistance(uint32_t from, uint32_t to, uint8_t square) noexcept { return square ? OpenTTDDistanceSquare(from, to) : OpenTTDDistanceManhattan(from, to); }
inline uint32_t RustTileAdd(uint32_t tile, int32_t delta) noexcept { return TileAdd(TileIndex{tile}, delta).base(); }
inline void RustCountLandTiles() noexcept { Map::CountLandTiles(); }
#endif
