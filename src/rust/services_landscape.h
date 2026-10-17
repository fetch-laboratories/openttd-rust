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
#include "../clear_map.h"
#include "services_rail.h"
#include "../road_internal.h"
#include "../tree_map.h"
#include "../water_map.h"
#include "../landscape.h"
#include "../water.h"
#include "../newgrf_generic.h"
#include "../sound_func.h"
#include "../signal_func.h"
#include "../effectvehicle_func.h"
#include "tile_animation_services.h"

inline OpenTTDFoundationSlope RustFoundationSlope(uint32_t tile) noexcept { auto [slope, z] = GetFoundationSlope(TileIndex{tile}); return {.z = z, .slope = slope}; }
inline RoadBits RustCleanRoadBits(TileIndex tile, RoadBits bits) noexcept { return CleanUpRoadBits(tile, bits); }
inline uint8_t RustTileOwnerValue(uint32_t tile) noexcept { return GetTileOwner(TileIndex{tile}).base(); }
inline bool RustPlainRailTile(uint32_t tile) noexcept { return IsPlainRailTile(TileIndex{tile}); }
inline ClearGround RustClearGround(Tile tile) noexcept { return GetClearGround(tile); }
inline bool RustClearFieldTile(Tile tile) noexcept { return IsTileType(tile, MP_CLEAR) && RustClearGround(tile) == CLEAR_FIELDS; }
inline bool RustClearFieldTile(uint32_t index) noexcept { return RustClearFieldTile(Tile{TileIndex{index}}); }
inline uint32_t RustClearDensity(Tile tile) noexcept { return GetClearDensity(tile); }
inline uint32_t RustClearDensity(uint32_t index) noexcept { return RustClearDensity(Tile{TileIndex{index}}); }
inline uint32_t RustClearCounter(uint32_t tile) noexcept { return GetClearCounter(TileIndex{tile}); }
inline bool RustSnowTile(Tile tile) noexcept { return IsSnowTile(tile); }
inline bool RustSnowTile(uint32_t index) noexcept { return RustSnowTile(Tile{TileIndex{index}}); }
inline OpenTTDClearSurface RustClearSurface(uint32_t index) noexcept
{
	Tile tile{TileIndex{index}};
	OpenTTDClearSurface surface{.ground = CLEAR_GRASS, .density = 0, .snow = RustSnowTile(tile)};
	if (!surface.snow) {
		surface.ground = RustClearGround(tile);
		if (surface.ground == CLEAR_GRASS) surface.density = static_cast<uint8_t>(RustClearDensity(tile));
	}
	return surface;
}
inline uint32_t RustFieldFence(Tile tile, DiagDirection direction) noexcept { return GetFence(tile, direction); }
inline void RustSetFieldFence(uint32_t tile, DiagDirection direction, uint32_t fence) noexcept { SetFence(TileIndex{tile}, direction, fence); }
inline void RustSetClearCounter(Tile tile, uint32_t counter) noexcept { SetClearCounter(tile, counter); }
inline TreeGround RustTreeGround(Tile tile) noexcept { return GetTreeGround(tile); }
inline int32_t RustTileZ(TileIndex tile) noexcept { return GetTileZ(tile); }
inline bool RustRoadSnowOrDesert(Tile tile) noexcept { return IsOnSnowOrDesert(tile); }
inline bool RustRoadSnowOrDesert(uint32_t tile) noexcept { return RustRoadSnowOrDesert(Tile{TileIndex{tile}}); }
inline bool RustTunnelSnowOrDesert(Tile tile) noexcept { return HasTunnelBridgeSnowOrDesert(tile); }
inline bool RustTunnelSnowOrDesert(uint32_t tile) noexcept { return RustTunnelSnowOrDesert(Tile{TileIndex{tile}}); }
inline bool RustAirportTile(uint32_t tile) noexcept { return IsAirportTile(TileIndex{tile}); }
inline EffectVehicle *RustCreateEffectAbove(int32_t x, int32_t y, int32_t z, EffectVehicleType type) noexcept { return CreateEffectVehicleAbove(x, y, z, type); }
inline EffectVehicle *RustCreateEffectAboveValue(int32_t x, int32_t y, int32_t z, uint8_t type) noexcept { return RustCreateEffectAbove(x, y, z, static_cast<EffectVehicleType>(type)); }
inline IndustryID RustFieldIndustry(Tile tile) noexcept { return GetIndustryIndexOfField(tile); }
inline bool RustFieldOfIndustry(Tile tile, IndustryID industry) noexcept { return RustClearFieldTile(tile) && RustFieldIndustry(tile) == industry; }
/* Existing industry ABI uses the underlying IndustryID16 wire value. */
inline bool RustFieldOfIndustryValue(uint32_t tile, uint16_t industry) noexcept { return RustFieldOfIndustry(Tile{TileIndex{tile}}, IndustryID{industry}); }
inline bool RustHasWaterClass(uint32_t tile) noexcept { return HasTileWaterClass(TileIndex{tile}); }
inline WaterClass RustWaterClass(uint32_t tile) noexcept { return GetWaterClass(TileIndex{tile}); }
inline bool RustIsCoast(uint32_t tile) noexcept { return IsCoast(TileIndex{tile}); }
inline void RustMakeClear(uint32_t tile, ClearGround ground, uint32_t density) noexcept { MakeClear(TileIndex{tile}, ground, density); }
inline void RustMakeClearValue(uint32_t tile, uint8_t ground, uint32_t density) noexcept { RustMakeClear(tile, static_cast<ClearGround>(ground), density); }
inline void RustMakeShore(uint32_t tile) noexcept { MakeShore(TileIndex{tile}); }
inline void RustMakeSnow(uint32_t tile, uint32_t density) noexcept { MakeSnow(TileIndex{tile}, density); }
inline void RustSetTreeGroundDensity(uint32_t tile, TreeGround ground, uint32_t density) noexcept { SetTreeGroundDensity(TileIndex{tile}, ground, density); }
inline void RustSetTreeGroundDensityValue(uint32_t tile, uint8_t ground, uint32_t density) noexcept { RustSetTreeGroundDensity(tile, static_cast<TreeGround>(ground), density); }
inline void RustClearNeighbourFlooding(uint32_t tile) noexcept { ClearNeighbourNonFloodingStates(TileIndex{tile}); }
inline void RustTileLoopWater(uint32_t tile) noexcept { TileLoop_Water(tile); }
inline bool RustAmbientSoundEnabled() noexcept { return HasGrfMiscBit(GrfMiscBit::AmbientSoundCallback); }
inline void RustAmbientSound(uint32_t tile) noexcept { AmbientSoundEffect(TileIndex{tile}); }
/* The existing tree ABI uses this post-callback flag to refresh its observation. */
inline bool RustAmbientSoundLegacy(uint32_t tile) noexcept { RustAmbientSound(tile); return RustAmbientSoundEnabled(); }
inline void RustPlayTileSound(uint32_t tile, uint16_t sound) noexcept { SndPlayTileFx(sound, TileIndex{tile}); }
inline void RustPlayTileSoundBySound(uint16_t sound, uint32_t tile) noexcept { RustPlayTileSound(tile, sound); }
inline void RustClearSquare(uint32_t tile) noexcept { DoClearSquare(TileIndex{tile}); }
inline void RustUpdateSignals() noexcept { UpdateSignalsInBuffer(); }
/* Necessary legacy signatures: context handles are unused and scalar return
 * widths are retained. All native work remains in the canonical typed leaves. */
inline uint32_t RustFoundationSlopeValue(uint32_t tile) noexcept { return RustFoundationSlope(tile).slope; }
template <typename Context>
inline void RustCrossingSound(Context, uint32_t tile) noexcept { RustPlayTileSound(tile, SND_0E_LEVEL_CROSSING); }
#endif /* RUST_SERVICES_LANDSCAPE_H */
