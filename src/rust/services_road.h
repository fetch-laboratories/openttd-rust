/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_road.h Canonical packed road map mutations. */
#ifndef RUST_SERVICES_ROAD_H
#define RUST_SERVICES_ROAD_H
#include "../road_map.h"
#include "../town_map.h"
#include "../road.h"
#include "../roadveh.h"
OpenTTDRoadTickRead RustRoadTickRead(const RoadVehicle *v) noexcept;
inline OpenTTDRoadState *RoadStateOf(const RoadVehicle *u) noexcept { return u == nullptr ? nullptr : u->GetRustState(); }
inline void RustSetRoadOwner(uint32_t tile, uint8_t kind, uint8_t owner) noexcept { SetRoadOwner(TileIndex{tile}, static_cast<RoadTramType>(kind), Owner(owner)); }
inline RoadType RustRoadTileType(uint32_t tile, RoadTramType kind) noexcept { return GetRoadType(TileIndex(tile), kind); }
inline RoadType RustRoadVehicleType(const RoadVehicle *v) noexcept { return v->roadtype; }
inline RoadTramType RustRoadTramType(RoadType type) noexcept { return GetRoadTramType(type); }
inline bool RustRoadTypeIsRoad(uint8_t rt) noexcept { return RoadTypeIsRoad(static_cast<RoadType>(rt)); }
inline uint16_t RustRoadCostMultiplier(uint8_t type) noexcept { return GetRoadTypeInfo(static_cast<RoadType>(type))->cost_multiplier; }
inline void RustSetRoadType(TileIndex tile, RoadTramType kind, RoadType type) noexcept { SetRoadType(tile, kind, type); }
inline bool RustCrossingReserved(uint32_t tile) noexcept { return HasCrossingReservation(TileIndex{tile}); }
inline void RustSetCrossingReserved(uint32_t tile, bool reserved) noexcept { SetCrossingReservation(TileIndex{tile}, reserved); }
inline void RustSetTownIndex(uint32_t tile, uint16_t town) noexcept { SetTownIndex(TileIndex{tile}, TownID(town)); }
inline bool OpenTTDHasTownOwnedRoad(uint32_t tile) noexcept { return HasTownOwnedRoad(TileIndex(tile)); }
inline bool RustMovementNormalRoad(uint32_t tile) noexcept { return IsNormalRoad(TileIndex{tile}); }
inline bool RustHasTileRoadType(uint32_t tile, RoadTramType kind) noexcept { return HasTileRoadType(TileIndex{tile}, kind); }
#endif /* RUST_SERVICES_ROAD_H */
