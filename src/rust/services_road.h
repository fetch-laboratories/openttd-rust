/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_road.h Canonical road mutation and crossing reservation leaves. */
#ifndef RUST_SERVICES_ROAD_H
#define RUST_SERVICES_ROAD_H
#include "../road_map.h"
#include "../town_map.h"
inline RoadTramType RustRoadTramType(RoadType rt) noexcept { return GetRoadTramType(rt); }
inline bool RustRoadTypeIsRoad(uint8_t rt) noexcept { return RoadTypeIsRoad(static_cast<RoadType>(rt)); }
inline uint16_t RustRoadCostMultiplier(uint8_t rt) noexcept { return GetRoadTypeInfo(static_cast<RoadType>(rt))->cost_multiplier; }
inline void RustSetRoadType(TileIndex tile, RoadTramType rtt, RoadType rt) noexcept { SetRoadType(tile, rtt, rt); }
inline bool RustCrossingReserved(uint32_t tile) noexcept { return HasCrossingReservation(TileIndex{tile}); }
inline void RustSetCrossingReserved(uint32_t tile, bool reserved) noexcept { SetCrossingReservation(TileIndex{tile}, reserved); }
inline void RustSetTownIndex(uint32_t tile, uint16_t town) noexcept { SetTownIndex(TileIndex{tile}, TownID(town)); }
#endif /* RUST_SERVICES_ROAD_H */
