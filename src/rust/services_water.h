/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_water.h Canonical water-map leaves for native Rust adapters. */
#ifndef RUST_SERVICES_WATER_H
#define RUST_SERVICES_WATER_H
#include "../water_map.h"
#include "../water.h"
#include "../track_func.h"
#include "../landscape.h"

inline WaterTileType RustWaterTileType(uint32_t tile) noexcept { return GetWaterTileType(TileIndex{tile}); }
inline LockPart RustLockPart(uint32_t tile) noexcept { return GetLockPart(TileIndex{tile}); }
inline bool RustHasWaterClass(uint32_t tile) noexcept { return HasTileWaterClass(TileIndex{tile}); }
inline WaterClass RustWaterClass(uint32_t tile) noexcept { return GetWaterClass(TileIndex{tile}); }
inline bool OpenTTDIsDockingTile(uint32_t tile) noexcept { return IsDockingTile(TileIndex(tile)); }
inline void OpenTTDMakeShore(uint32_t tile) noexcept { MakeShore(TileIndex(tile)); }
inline bool OpenTTDIsMiddleLock(uint32_t tile) noexcept { return IsTileType(TileIndex{tile}, MP_WATER) && RustWaterTileType(tile) == WaterTileType::Lock && RustLockPart(tile) == LockPart::Middle; }
inline void OpenTTDSetDockingTile(uint32_t tile, bool docking) noexcept { SetDockingTile(TileIndex{tile}, docking); }
inline void RustMakeCanal(uint32_t tile, uint8_t owner, uint8_t random) noexcept { MakeCanal(TileIndex{tile}, Owner(owner), random); }
inline void RustMakeRiver(uint32_t tile, uint8_t random) noexcept { MakeRiver(TileIndex{tile}, random); }
inline bool RustOnWater(uint32_t tile) noexcept { return IsTileOnWater(TileIndex(tile)); }
inline bool RustIsCanal(uint32_t tile) noexcept { return IsCanal(TileIndex{tile}); }

inline Axis RustShipDepotAxis(uint32_t tile) noexcept { return GetShipDepotAxis(TileIndex{tile}); }
#endif /* RUST_SERVICES_WATER_H */
