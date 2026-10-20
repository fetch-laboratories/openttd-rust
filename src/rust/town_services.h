/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file town_services.h Canonical typed native spatial town lookup. */
#ifndef RUST_TOWN_SERVICES_H
#define RUST_TOWN_SERVICES_H
#include "../town.h"

/** Uncached lookup; callers retain their original threshold and result policy. */
inline Town *RustCalcClosestTown(TileIndex tile, uint threshold) noexcept
{
	return CalcClosestTownFromTile(tile, threshold);
}
#endif /* RUST_TOWN_SERVICES_H */
