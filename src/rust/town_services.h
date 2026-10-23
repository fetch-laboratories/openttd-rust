/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file town_services.h Canonical native town lookup services. */
#ifndef OPENTTD_RUST_TOWN_SERVICES_H
#define OPENTTD_RUST_TOWN_SERVICES_H
#include "../town.h"
#include "town_ffi.h"

inline Town *RustCalcClosestTown(TileIndex tile, uint threshold) noexcept
{
	return CalcClosestTownFromTile(tile, threshold);
}
inline uint16_t RustTownAuthorityIndex(const OpenTTDTownHandle *town) noexcept { return reinterpret_cast<const Town *>(town)->index.base(); }
#endif /* OPENTTD_RUST_TOWN_SERVICES_H */
