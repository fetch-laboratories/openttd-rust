/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file yapf_road.cpp The road pathfinding. */

#include "../../stdafx.h"
#include "yapf.hpp"
#include "yapf_node_road.hpp"
#include "../../roadstop_base.h"
#include "../../rust/services_road.h"

#include "../../safeguards.h"


#include "../../rust/road_yapf_ffi.h"
#include "../../rust/services_yapf.h"
#include "../../rust/services_map.h"
#include "../../rust/services_movement.h"
#include "../../rust/vehicle_services.h"

static OpenTTDRoadYapfInput RoadInput(const RoadVehicle *v)
{
	const auto &s = _settings_game.pf.yapf;
	return {Map::SizeX(), Map::SizeY(), v->GetTile().base(), v->GetDestTile().base(), static_cast<int32_t>(s.max_search_nodes), static_cast<int32_t>(s.road_slope_penalty), static_cast<int32_t>(s.road_crossing_penalty), static_cast<int32_t>(s.road_stop_penalty), static_cast<int32_t>(s.road_stop_occupied_penalty), static_cast<int32_t>(s.road_stop_bay_occupied_penalty), static_cast<int32_t>(s.road_curve_penalty), v->current_order.GetDestination().base(), static_cast<uint8_t>(v->current_order.GetType()), v->IsBus(), v->HasArticulatedPart(), static_cast<uint8_t>(v->GetVehicleTrackdir())};
}

static OpenTTDRoadYapfTile RoadTile(const void *, uint32_t index, uint8_t td, bool cost) noexcept
{
	TileIndex tile(index);
	OpenTTDRoadYapfTile out{};
	out.type = SharedTileType(index);
	out.depot = Shared_is_road_depot(index);
	if (out.depot) out.depot_dir = Shared_road_depot_direction(index);
	if (cost && out.type == MP_ROAD && IsDiagonalTrackdir(static_cast<Trackdir>(td))) out.crossing = MapSvcRoadIsLevelCrossing(index);
	if (out.type == MP_STATION) {
		out.station = MapSvcStationIndex(index);
		out.station_type = static_cast<uint8_t>(GetStationType(tile));
		out.waypoint = IsRoadWaypoint(tile);
		out.drive_through = Shared_is_drive_through_stop(index);
	}
	return out;
}

static uint32_t RoadClosest(const void *ctx, uint16_t station, uint8_t type) noexcept
{
	return CalcClosestStationTile(StationID(station), VehicleOf(static_cast<const OpenTTDVehicle *>(ctx))->GetTile(), static_cast<StationType>(type)).base();
}

static const OpenTTDRoadYapfLeaves _road_yapf_leaves{.shared = &_rust_shared_services, .tile = RoadTile, .closest = RoadClosest};

static void RoadDebug(const RoadVehicle *v, const OpenTTDRoadYapfResult &result)
{
	if (result.rounds == 0 || _debug_yapf_level < 3) return;
	RustYapfDebug('r', result.found, v->unitnumber, result.rounds, result.open, result.closed, 0.0f,
		result.found ? result.cost : -1, result.found ? result.distance : -1);
}

Trackdir YapfRoadVehicleChooseTrack(const RoadVehicle *v, TileIndex tile, DiagDirection enterdir, TrackdirBits trackdirs, bool &path_found)
{
	const auto input = RoadInput(v);
	const auto result = openttd_rust_road_yapf_choose(v->GetRustState(), &input, &_road_yapf_leaves, RustVehicle(v), tile.base(), enterdir, trackdirs, path_found);
	RoadDebug(v, result);
	path_found = result.found;
	return static_cast<Trackdir>(result.direction);
}

FindDepotData YapfRoadVehicleFindNearestDepot(const RoadVehicle *v, int max_distance)
{
	const auto input = RoadInput(v);
	const auto result = openttd_rust_road_yapf_depot(&input, &_road_yapf_leaves, RustVehicle(v), max_distance);
	RoadDebug(v, result);
	return result.found ? FindDepotData(TileIndex(result.tile), result.cost) : FindDepotData();
}

#include "../../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDRoadYapfLeaves.hpp"
bool CheckAbi_yapf_road() { return AbiServicesComplete(_road_yapf_leaves); }
