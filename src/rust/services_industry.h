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
template <typename ID> inline Industry *RustIndustryByID(ID id) noexcept { return Industry::Get(id); }
inline Industry *RustIndustryByTile(uint32_t tile) noexcept { return Industry::GetByTile(TileIndex{tile}); }
inline Industry *OpenTTDNextIndustry(uint32_t from) noexcept { auto range = Industry::Iterate(from); return range.empty() ? nullptr : *range.begin(); }
inline Town *OpenTTDIndustryTown(const Industry *industry) noexcept { return industry->town; }
inline const OpenTTDTownHandle *OpenTTDIndustryTownHandle(Industry *industry) noexcept { return reinterpret_cast<const OpenTTDTownHandle *>(OpenTTDIndustryTown(industry)); }
inline Town *OpenTTDIndustryTownByTile(uint32_t tile) noexcept { return OpenTTDIndustryTown(RustIndustryByTile(tile)); }
inline uint16_t OpenTTDIndustryTileCallback(uint32_t tile, uint16_t gfx, uint16_t callback, uint32_t parameter1, uint32_t parameter2) noexcept
{
	return GetIndustryTileCallback(static_cast<CallbackID>(callback), parameter1, parameter2, gfx, RustIndustryByTile(tile), TileIndex(tile));
}
inline uint16_t RustIndustryID(void *industry) noexcept { return static_cast<Industry *>(industry)->index.base(); }
inline uint32_t RustIndustrySpecBehaviour(const IndustrySpec *spec) noexcept { return spec->behaviour.base(); }
inline uint32_t RustIndustryBehaviour(void *industry) noexcept { return RustIndustrySpecBehaviour(GetIndustrySpec(static_cast<Industry *>(industry)->type)); }
inline uint32_t RustIndustryTile(void *industry) noexcept { return static_cast<Industry *>(industry)->location.tile.base(); }
inline uint16_t RustIndustryTileIndex(uint32_t tile) noexcept { return GetIndustryIndex(TileIndex{tile}).base(); }
inline bool RustIndustryTileBelongs(void *industry, uint32_t tile) noexcept { return static_cast<Industry *>(industry)->TileBelongsToIndustry(TileIndex{tile}); }
inline void RustResetIndustryConstruction(uint32_t tile) noexcept { ResetIndustryConstructionStage(TileIndex{tile}); }
#endif /* RUST_SERVICES_INDUSTRY_H */
