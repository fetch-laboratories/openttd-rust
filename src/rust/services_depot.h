/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file depot_services.h Shared native depot pool operations. */
#ifndef RUST_DEPOT_SERVICES_H
#define RUST_DEPOT_SERVICES_H
#include "../depot_base.h"
#include "../depot_func.h"
inline bool RustDepotAvailable() noexcept { return Depot::CanAllocateItem(); }
inline uint16_t RustNewDepot(uint32_t tile) noexcept { return (new Depot(TileIndex{tile}))->index.base(); }
inline void RustDepotName(uint16_t id) noexcept { MakeDefaultName(Depot::Get(DepotID(id))); }
inline void RustDeleteDepot(uint32_t tile) noexcept { delete Depot::GetByTile(TileIndex{tile}); }
#endif /* RUST_DEPOT_SERVICES_H */
