/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_landscape.h Canonical typed terrain leaves shared by game owners. */
#ifndef RUST_SERVICES_LANDSCAPE_H
#define RUST_SERVICES_LANDSCAPE_H
#include "ground_ffi.h"
#include "environment_services.h"
#include "movement_ffi.h"
#include "../newgrf_house.h"
#include "../newgrf_industrytiles.h"
#include "../newgrf_object.h"
#include "../house.h"
#include "../town.h"
#include "../industry.h"
#include "../object_map.h"
#include "../clear_map.h"
#include "services_rail.h"
#include "services_viewport.h"
#include "services_newgrf.h"
#include "services_industry.h"
#include "services_water.h"
#include "../road_internal.h"
#include "../tree_map.h"
#include "../water_map.h"
#include "../landscape.h"
#include "../water.h"
#include "../newgrf_generic.h"
#include "../sound_func.h"
#include "../signal_func.h"
#include "../effectvehicle_func.h"
#include "../viewport_func.h"
#include "tile_animation_services.h"

inline OpenTTDFlatTileHeight RustFlatTileHeight(uint32_t tile) noexcept { int height = 0; bool flat = RustIsTileFlat(tile, &height); return {.height = height, .flat = flat}; }
inline void RustSetTileHeight(uint32_t tile, uint32_t height) noexcept { SetTileHeight(TileIndex{tile}, height); }
inline OpenTTDFoundationSlope RustApplyFoundation(Foundation foundation, Slope slope) noexcept { int z = ApplyFoundationToSlope(foundation, slope); return {.z = z, .slope = slope}; }
inline uint8_t RustTileOwnerValue(uint32_t tile) noexcept { return GetTileOwner(TileIndex{tile}).base(); }
inline bool RustPlainRailTile(uint32_t tile) noexcept { return IsPlainRailTile(TileIndex{tile}); }
inline ClearGround RustClearGround(Tile tile) noexcept { return GetClearGround(tile); }
inline bool RustClearFieldTile(Tile tile) noexcept { return IsTileType(tile, MP_CLEAR) && RustClearGround(tile) == CLEAR_FIELDS; }
inline bool RustClearFieldTile(uint32_t index) noexcept { return RustClearFieldTile(Tile{TileIndex{index}}); }
inline bool RustSnowTile(Tile tile) noexcept { return IsSnowTile(tile); }
inline bool RustSnowTile(uint32_t index) noexcept { return RustSnowTile(Tile{TileIndex{index}}); }
inline uint32_t RustFieldFence(Tile tile, DiagDirection direction) noexcept { return GetFence(tile, direction); }
inline void RustSetFieldFence(uint32_t tile, DiagDirection direction, uint32_t fence) noexcept { SetFence(TileIndex{tile}, direction, fence); }
inline void RustSetClearCounter(Tile tile, uint32_t counter) noexcept { SetClearCounter(tile, counter); }
inline TreeGround RustTreeGround(Tile tile) noexcept { return GetTreeGround(tile); }
inline int32_t RustTileZ(TileIndex tile) noexcept { return GetTileZ(tile); }
inline bool RustRoadSnowOrDesert(Tile tile) noexcept { return IsOnSnowOrDesert(tile); }
inline bool RustRoadSnowOrDesert(uint32_t tile) noexcept { return RustRoadSnowOrDesert(Tile{TileIndex{tile}}); }
inline bool RustTunnelSnowOrDesert(Tile tile) noexcept { return HasTunnelBridgeSnowOrDesert(tile); }
inline bool RustTunnelSnowOrDesert(uint32_t tile) noexcept { return RustTunnelSnowOrDesert(Tile{TileIndex{tile}}); }
inline bool RustTunnelBridgeIsBridge(Tile tile) noexcept { return IsBridge(tile); }
inline bool RustAirportTile(uint32_t tile) noexcept { return IsAirportTile(TileIndex{tile}); }
inline IndustryID RustFieldIndustry(Tile tile) noexcept { return GetIndustryIndexOfField(tile); }
inline bool RustFieldOfIndustry(Tile tile, IndustryID industry) noexcept { return RustClearFieldTile(tile) && RustFieldIndustry(tile) == industry; }
/* Existing industry ABI uses the underlying IndustryID16 wire value. */
inline bool RustFieldOfIndustryValue(uint32_t tile, uint16_t industry) noexcept { return RustFieldOfIndustry(Tile{TileIndex{tile}}, IndustryID{industry}); }
inline void RustMakeClear(uint32_t tile, ClearGround ground, uint32_t density) noexcept { MakeClear(TileIndex{tile}, ground, density); }
inline void RustMakeClearValue(uint32_t tile, uint8_t ground, uint32_t density) noexcept { RustMakeClear(tile, static_cast<ClearGround>(ground), density); }
inline void RustMakeSnow(uint32_t tile, uint32_t density) noexcept { MakeSnow(TileIndex{tile}, density); }
inline void RustSetTreeGroundDensity(uint32_t tile, TreeGround ground, uint32_t density) noexcept { SetTreeGroundDensity(TileIndex{tile}, ground, density); }
inline void RustSetTreeGroundDensityValue(uint32_t tile, uint8_t ground, uint32_t density) noexcept { RustSetTreeGroundDensity(tile, static_cast<TreeGround>(ground), density); }
inline bool RustAmbientSoundEnabled() noexcept { return HasGrfMiscBit(GrfMiscBit::AmbientSoundCallback); }
inline void RustPlayTileSound(uint32_t tile, uint16_t sound) noexcept { SndPlayTileFx(sound, TileIndex{tile}); }
inline void RustPlayTileSoundBySound(uint16_t sound, uint32_t tile) noexcept { RustPlayTileSound(tile, sound); }
inline void RustUpdateSignals() noexcept { UpdateSignalsInBuffer(); }
/* Necessary legacy signatures: context handles are unused and scalar return
 * widths are retained. All native work remains in the canonical typed leaves. */
template <typename Context>
inline void RustCrossingSound(Context, uint32_t tile) noexcept { RustPlayTileSound(tile, SND_0E_LEVEL_CROSSING); }

#endif /* RUST_SERVICES_LANDSCAPE_H */
