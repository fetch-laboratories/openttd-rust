/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file services_industry.h Canonical native industry town association. */
#ifndef RUST_SERVICES_INDUSTRY_H
#define RUST_SERVICES_INDUSTRY_H
#include "../industry.h"
#include "../newgrf_industrytiles.h"
#include "services_ffi.h"
inline Industry *OpenTTDNextIndustry(uint32_t from) noexcept { auto range = Industry::Iterate(from); return range.empty() ? nullptr : *range.begin(); }
inline Town *OpenTTDIndustryTown(const Industry *industry) noexcept { return industry->town; }
inline const OpenTTDTownHandle *OpenTTDIndustryTownHandle(Industry *industry) noexcept { return reinterpret_cast<const OpenTTDTownHandle *>(OpenTTDIndustryTown(industry)); }
inline Town *OpenTTDIndustryTownByTile(uint32_t tile) noexcept { return OpenTTDIndustryTown(Industry::GetByTile(TileIndex(tile))); }
inline uint16_t OpenTTDIndustryTileCallback(uint32_t tile, uint16_t gfx, uint16_t callback, uint32_t parameter1, uint32_t parameter2) noexcept
{
	return GetIndustryTileCallback(static_cast<CallbackID>(callback), parameter1, parameter2, gfx, Industry::GetByTile(TileIndex(tile)), TileIndex(tile));
}
inline uint16_t RustIndustryID(void *industry) noexcept { return static_cast<Industry *>(industry)->index.base(); }
#endif /* RUST_SERVICES_INDUSTRY_H */
