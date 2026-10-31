/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file yapf_ship.cpp Implementation of YAPF for ships. */

#include "../../stdafx.h"
#include "../../ship.h"
#include "../../vehicle_func.h"

#include "yapf.hpp"
#include "yapf_node_ship.hpp"
#include "yapf_ship_regions.h"
#include "../water_regions.h"

#include "../../safeguards.h"

#include "../../rust/ship_yapf_ffi.h"
#include "../../rust/services_yapf.h"
#include "../../rust/services_water.h"

static OpenTTDShipYapfDestination ShipDestination(const void *context) noexcept
{
	const Ship *v = static_cast<const Ship *>(context);
	if (v->current_order.IsType(OT_GOTO_STATION)) {
		return {.tile = CalcClosestStationTile(v->current_order.GetDestination().ToStationID(), v->tile, StationType::Dock).base(), .station = true};
	}
	return {.tile = v->dest_tile.base(), .station = false};
}
static OpenTTDShipTile ShipTile(const void *context, uint32_t tile, uint8_t cost) noexcept
{
	const Ship *v = static_cast<const Ship *>(context);
	const TileIndex t{tile};
	const bool docking = OpenTTDIsDockingTile(tile);
	if (cost == 0) return {0, static_cast<uint8_t>(docking), 0, 0,
		static_cast<uint8_t>(docking && IsShipDestinationTile(t, v->current_order.GetDestination().ToStationID()))};
	uint count = 0;
	if (docking) count = std::ranges::count_if(VehiclesOnTile(t), [](const Vehicle *other) { return other->type == VEH_SHIP && !other->vehstatus.Test(VehState::Hidden); });
	return {count, static_cast<uint8_t>(docking), static_cast<uint8_t>(GetEffectiveWaterClass(t) == WaterClass::Sea),
		static_cast<uint8_t>(GetRustSharedServices().water_middle_lock(tile)),
		0};
}
static OpenTTDWaterPatch ShipPatch(uint32_t tile) noexcept
{
	const auto p = GetWaterRegionPatchInfo(TileIndex{tile});
	return {p.x, p.y, p.label.base()};
}

void *ShipWaterVisitNew(OpenTTDWaterPatch) noexcept;
uint8_t ShipWaterVisitNext(void *, OpenTTDWaterPatch, OpenTTDWaterPatch *) noexcept;
void ShipWaterVisitDestroy(void *) noexcept;

static const OpenTTDShipYapfLeaves _ship_yapf_leaves{
	.destination = ShipDestination,
	.tile = ShipTile,
	.patch = ShipPatch,
	.visit_new = ShipWaterVisitNew,
	.visit_next = ShipWaterVisitNext,
	.visit_destroy = ShipWaterVisitDestroy,
	.debug = RustShipYapfDebug,
};

static OpenTTDShipYapfInput ShipInput(const Ship *v, bool blocked = false)
{
	const auto *svi = ShipVehInfo(v->engine_type);
	const bool station = v->current_order.IsType(OT_GOTO_STATION);
	TrackdirBits reverse_dirs = TRACKDIR_BIT_NONE;
	if (blocked) {
		const DiagDirection entry = ReverseDiagDir(VehicleExitDir(v->direction, v->GetState()));
		reverse_dirs = DiagdirReachesTrackdirs(entry) & TrackStatusToTrackdirBits(GetTileTrackStatus(v->tile, TRANSPORT_WATER, 0, entry));
	}
	return {Map::SizeX(), Map::SizeY(), v->tile.base(), v->dest_tile.base(), static_cast<int32_t>(_settings_game.pf.yapf.ship_curve90_penalty),
		static_cast<int32_t>(_settings_game.pf.yapf.ship_curve45_penalty), svi->max_speed,
		0, static_cast<uint16_t>(reverse_dirs), static_cast<uint8_t>(v->GetVehicleTrackdir()), svi->ocean_speed_frac, svi->canal_speed_frac, static_cast<uint8_t>(station), v->unitnumber};
}
static std::vector<uint32_t> ShipOrigins(const Ship *v)
{
	std::vector<uint32_t> tiles;
	if (v->current_order.IsType(OT_GOTO_STATION)) {
		StationID station = v->current_order.GetDestination().ToStationID();
		for (const auto &tile : BaseStation::Get(station)->GetTileArea(StationType::Dock)) {
			if (OpenTTDIsDockingTile(tile.base()) && IsShipDestinationTile(tile, station)) tiles.push_back(tile.base());
		}
	} else {
		tiles.push_back(v->dest_tile.base());
	}
	return tiles;
}
Track YapfShipChooseTrack(const Ship *v, TileIndex tile, bool &found, ShipPathCache &cache)
{
	const auto input = ShipInput(v);
	const auto origins = ShipOrigins(v);
	const auto result = openttd_rust_ship_choose(cache.GetOwner(), &input, &_ship_yapf_leaves, &GetRustSharedServices(), v, tile.base(), TrackdirToTrackdirBits(v->GetVehicleTrackdir()), TRACKDIR_BIT_NONE, origins.data(), origins.size());
	found = result.found;
	return result.direction == INVALID_TRACKDIR ? INVALID_TRACK : TrackdirToTrack(static_cast<Trackdir>(result.direction));
}
bool YapfShipCheckReverse(const Ship *v, Trackdir *trackdir)
{
	const auto input = ShipInput(v, trackdir != nullptr);
	const auto origins = ShipOrigins(v);
	const auto result = openttd_rust_ship_reverse(&input, &_ship_yapf_leaves, &GetRustSharedServices(), v, trackdir != nullptr, origins.data(), origins.size());
	if (trackdir != nullptr) *trackdir = static_cast<Trackdir>(result.direction);
	return result.found;
}
std::vector<WaterRegionPatchDesc> YapfShipFindWaterRegionPath(const Ship *v, TileIndex start, int max_length)
{
	const auto input = ShipInput(v);
	const auto origins = ShipOrigins(v);
	using ResultOwner = std::unique_ptr<OpenTTDShipRegionPath, decltype(&openttd_rust_ship_regions_destroy)>;
	ResultOwner result{openttd_rust_ship_regions(&input, &_ship_yapf_leaves, start.base(), max_length, origins.data(), origins.size()), openttd_rust_ship_regions_destroy};
	std::vector<WaterRegionPatchDesc> path;
	for (size_t i = 0; i < openttd_rust_ship_regions_size(result.get()); ++i) {
		const auto p = openttd_rust_ship_regions_get(result.get(), i);
		path.push_back({p.x, p.y, WaterRegionPatchLabel{p.label}});
	}
	return path;
}

#include "../../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDShipYapfLeaves.hpp"
bool CheckAbi_yapf_ship() { return AbiServicesComplete(_ship_yapf_leaves); }
