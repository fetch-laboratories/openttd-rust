/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file services_industry.h Canonical native industry town association. */
#ifndef RUST_SERVICES_INDUSTRY_H
#define RUST_SERVICES_INDUSTRY_H
#include "../industry.h"
#include "services_ffi.h"
inline Town *OpenTTDIndustryTown(const Industry *industry) noexcept { return industry->town; }
inline const OpenTTDTownHandle *OpenTTDIndustryTownHandle(Industry *industry) noexcept { return reinterpret_cast<const OpenTTDTownHandle *>(OpenTTDIndustryTown(industry)); }
inline Town *OpenTTDIndustryTownByTile(uint32_t tile) noexcept { return OpenTTDIndustryTown(Industry::GetByTile(TileIndex(tile))); }
#endif /* RUST_SERVICES_INDUSTRY_H */
