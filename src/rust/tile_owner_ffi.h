/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file tile_owner_ffi.h Shared ownership dispatch and borrowed stack context. */
#ifndef RUST_TILE_OWNER_FFI_H
#define RUST_TILE_OWNER_FFI_H
#include "road_command_ffi.h"
#include "station_command_ffi.h"
#include "water_command_ffi.h"
#include "tunnelbridge_command_ffi.h"
#include "industry_ffi.h"
struct OpenTTDObjectServices;
struct OpenTTDTileOwnerServices {
	void (*clear)(uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(clear);
	void (*house)(uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(house);
	const OpenTTDIndustryTileServices *industry;
	const OpenTTDObjectServices *objects;
	const OpenTTDRailCommandServices *rail;
	const OpenTTDSharedServices *shared;
	const OpenTTDStationCommandServices *station;
	void (*trees)(uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(trees);
	const OpenTTDTunnelBridgeServices *tunnel_bridge;
	void (*void_tile)(uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(void_tile);
	const OpenTTDWaterCommandServices *water;
};
struct OpenTTDTileOwnerContext {
	const OpenTTDTileOwnerServices *services;
	OpenTTDCommandCost *cost;
	OpenTTDCommandCost *ret;
	OpenTTDCommandCost *error;
	const OpenTTDRoadCommandContext *road;
};
const OpenTTDTileOwnerServices &GetRustTileOwnerServices() noexcept;
extern "C" void openttd_rust_tile_change_owner(const OpenTTDTileOwnerContext *, uint32_t, uint8_t, uint8_t);
#endif /* RUST_TILE_OWNER_FFI_H */
