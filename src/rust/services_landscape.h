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

inline OpenTTDFlatTileHeight RustFlatTileHeight(uint32_t tile) noexcept { int height = 0; bool flat = IsTileFlat(TileIndex{tile}, &height); return {.height = height, .flat = flat}; }
inline void RustSetTileHeight(uint32_t tile, uint32_t height) noexcept { SetTileHeight(TileIndex{tile}, height); }
inline bool RustInnerTile(uint32_t tile) noexcept { return IsInnerTile(TileIndex{tile}); }
inline OpenTTDFoundationSlope RustTileSlopeZ(uint32_t tile) noexcept { auto [slope, z] = GetTileSlopeZ(TileIndex{tile}); return {.z = z, .slope = slope}; }
inline OpenTTDFoundationSlope RustApplyFoundation(Foundation foundation, Slope slope) noexcept { int z = ApplyFoundationToSlope(foundation, slope); return {.z = z, .slope = slope}; }
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
inline bool RustTunnelBridgeIsBridge(Tile tile) noexcept { return IsBridge(tile); }
inline bool RustAirportTile(uint32_t tile) noexcept { return IsAirportTile(TileIndex{tile}); }
inline EffectVehicle *RustCreateEffectAbove(int32_t x, int32_t y, int32_t z, EffectVehicleType type) noexcept { return CreateEffectVehicleAbove(x, y, z, type); }
inline EffectVehicle *RustCreateEffectAboveValue(int32_t x, int32_t y, int32_t z, uint8_t type) noexcept { return RustCreateEffectAbove(x, y, z, static_cast<EffectVehicleType>(type)); }
inline IndustryID RustFieldIndustry(Tile tile) noexcept { return GetIndustryIndexOfField(tile); }
inline bool RustFieldOfIndustry(Tile tile, IndustryID industry) noexcept { return RustClearFieldTile(tile) && RustFieldIndustry(tile) == industry; }
/* Existing industry ABI uses the underlying IndustryID16 wire value. */
inline bool RustFieldOfIndustryValue(uint32_t tile, uint16_t industry) noexcept { return RustFieldOfIndustry(Tile{TileIndex{tile}}, IndustryID{industry}); }
inline bool RustIsCoast(uint32_t tile) noexcept { return RustWaterTileType(tile) == WaterTileType::Coast; }
inline void RustMakeClear(uint32_t tile, ClearGround ground, uint32_t density) noexcept { MakeClear(TileIndex{tile}, ground, density); }
inline void RustMakeClearValue(uint32_t tile, uint8_t ground, uint32_t density) noexcept { RustMakeClear(tile, static_cast<ClearGround>(ground), density); }
inline void RustMakeSnow(uint32_t tile, uint32_t density) noexcept { MakeSnow(TileIndex{tile}, density); }
inline void RustSetTreeGroundDensity(uint32_t tile, TreeGround ground, uint32_t density) noexcept { SetTreeGroundDensity(TileIndex{tile}, ground, density); }
inline void RustSetTreeGroundDensityValue(uint32_t tile, uint8_t ground, uint32_t density) noexcept { RustSetTreeGroundDensity(tile, static_cast<TreeGround>(ground), density); }
inline bool RustAmbientSoundEnabled() noexcept { return HasGrfMiscBit(GrfMiscBit::AmbientSoundCallback); }
inline void RustAmbientSound(uint32_t tile) noexcept { AmbientSoundEffect(TileIndex{tile}); }
/* The existing tree ABI uses this post-callback flag to refresh its observation. */
inline bool RustAmbientSoundLegacy(uint32_t tile) noexcept { RustAmbientSound(tile); return RustAmbientSoundEnabled(); }
inline void RustPlayTileSound(uint32_t tile, uint16_t sound) noexcept { SndPlayTileFx(sound, TileIndex{tile}); }
inline void RustPlayTileSoundBySound(uint16_t sound, uint32_t tile) noexcept { RustPlayTileSound(tile, sound); }
inline void RustUpdateSignals() noexcept { UpdateSignalsInBuffer(); }
/* Necessary legacy signatures: context handles are unused and scalar return
 * widths are retained. All native work remains in the canonical typed leaves. */
template <typename Context>
inline void RustCrossingSound(Context, uint32_t tile) noexcept { RustPlayTileSound(tile, SND_0E_LEVEL_CROSSING); }

inline OpenTTDFoundationSlope RustTilePixelSlope(uint32_t tile) noexcept { auto [slope, z] = GetTilePixelSlope(TileIndex{tile}); return {.z = z, .slope = slope}; }
inline OpenTTDFoundationSlope RustOutsidePixelSlope(uint32_t x, uint32_t y) noexcept { auto [slope, z] = GetTilePixelSlopeOutsideMap(x, y); return {.z = z, .slope = slope}; }
inline OpenTTDMovementObjectFoundation RustObjectFoundation(uint32_t tile) noexcept
{
	const auto *spec = ObjectSpec::Get(GetObjectType(TileIndex{tile}));
	const bool enabled = spec->IsEnabled();
	return {.enabled = enabled, .no_foundation = enabled && spec->flags.Test(ObjectFlag::HasNoFoundation)};
}
inline OpenTTDMovementFoundation RustHouseFoundation(uint32_t tile) noexcept
{
	HouseID kind = GetHouseType(TileIndex{tile});
	const auto *spec = kind >= NEW_HOUSE_OFFSET ? HouseSpec::Get(kind) : nullptr;
	return {.spec = spec, .kind = kind, .callback = spec != nullptr && spec->callback_mask.Test(HouseCallbackMask::DrawFoundations)};
}
inline uint16_t RustHouseFoundationCallback(uint32_t tile, uint16_t kind) noexcept { return OpenTTDHouseCallback(CBID_HOUSE_DRAW_FOUNDATIONS, 0, 0, kind, Town::GetByTile(TileIndex{tile}), TileIndex{tile}, false); }
inline bool RustHouseFoundationBoolean(const void *spec, uint16_t result) noexcept { return RustSpecBooleanCallback(static_cast<const HouseSpec *>(spec), CBID_HOUSE_DRAW_FOUNDATIONS, result); }
inline OpenTTDMovementFoundation RustIndustryFoundation(uint32_t tile) noexcept
{
	IndustryGfx kind = GetIndustryGfx(TileIndex{tile});
	const auto *spec = kind >= NEW_INDUSTRYTILEOFFSET ? GetIndustryTileSpec(kind) : nullptr;
	return {.spec = spec, .kind = kind, .callback = spec != nullptr && spec->callback_mask.Test(IndustryTileCallbackMask::DrawFoundations)};
}
inline uint16_t RustIndustryFoundationCallback(uint32_t tile, uint16_t kind) noexcept { return OpenTTDIndustryTileCallback(tile, kind, CBID_INDTILE_DRAW_FOUNDATIONS, 0, 0); }
inline bool RustIndustryFoundationBoolean(const void *spec, uint16_t result) noexcept { return RustSpecBooleanCallback(static_cast<const IndustryTileSpec *>(spec), CBID_INDTILE_DRAW_FOUNDATIONS, result); }
#endif /* RUST_SERVICES_LANDSCAPE_H */
