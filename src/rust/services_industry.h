/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file services_industry.h Canonical native industry town association. */
#ifndef RUST_SERVICES_INDUSTRY_H
#define RUST_SERVICES_INDUSTRY_H
#include "../industry.h"
#include "../newgrf_industrytiles.h"
#include "services_ffi.h"
#include "../industry_map.h"
inline void RustIndustryTileRandomCheck([[maybe_unused]] const OpenTTDIndustryResolverContext *context) noexcept
{
	assert(context->industry != nullptr && IsValidTile(TileIndex(context->tile)));
	assert(context->id == IndustryID::Invalid().base() || IsTileType(TileIndex(context->tile), MP_INDUSTRY));
}
inline Industry *RustIndustryByTile(uint32_t tile) noexcept { return Industry::GetByTile(TileIndex{tile}); }
inline uint16_t OpenTTDIndustryTileCallback(uint32_t tile, uint16_t gfx, uint16_t callback, uint32_t parameter1, uint32_t parameter2) noexcept
{
	return GetIndustryTileCallback(static_cast<CallbackID>(callback), parameter1, parameter2, gfx, RustIndustryByTile(tile), TileIndex(tile));
}
template <class Handle> inline uint16_t RustIndustryID(Handle industry) noexcept { return static_cast<Industry *>(industry)->index.base(); }
inline uint32_t RustIndustryTile(void *industry) noexcept { return static_cast<Industry *>(industry)->Record().location.tile.base(); }
inline uint16_t RustIndustryTileIndex(uint32_t tile) noexcept { return GetIndustryIndex(TileIndex{tile}).base(); }
inline bool RustIndustryTileBelongs(void *industry, uint32_t tile) noexcept { return static_cast<Industry *>(industry)->TileBelongsToIndustry(TileIndex{tile}); }
inline void RustResetIndustryConstruction(uint32_t tile) noexcept { ResetIndustryConstructionStage(TileIndex{tile}); }
#endif /* RUST_SERVICES_INDUSTRY_H */
