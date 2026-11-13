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
#include "../../rust/vehicle_services.h"

static OpenTTDRoadYapfInput RoadInput(const RoadVehicle *v)
{
	const auto &s = _settings_game.pf.yapf;
	return {v->GetTile().base(), v->GetDestTile().base(), static_cast<int32_t>(s.max_search_nodes), static_cast<int32_t>(s.road_slope_penalty), static_cast<int32_t>(s.road_crossing_penalty), static_cast<int32_t>(s.road_stop_penalty), static_cast<int32_t>(s.road_stop_occupied_penalty), static_cast<int32_t>(s.road_stop_bay_occupied_penalty), static_cast<int32_t>(s.road_curve_penalty), v->current_order.GetDestination().base(), static_cast<uint8_t>(v->current_order.GetType()), v->IsBus(), v->HasArticulatedPart(), static_cast<uint8_t>(v->GetVehicleTrackdir())};
}

Trackdir YapfRoadVehicleChooseTrack(const RoadVehicle *v, TileIndex tile, DiagDirection enterdir, TrackdirBits trackdirs, bool &path_found)
{
	const auto input = RoadInput(v);
	const auto result = openttd_rust_road_yapf_choose(v->GetRustState(), &input, &_rust_shared_services, RustVehicle(v), tile.base(), enterdir, trackdirs, path_found);
	path_found = result.found;
	return static_cast<Trackdir>(result.direction);
}

FindDepotData YapfRoadVehicleFindNearestDepot(const RoadVehicle *v, int max_distance)
{
	const auto input = RoadInput(v);
	const auto result = openttd_rust_road_yapf_depot(&input, &_rust_shared_services, RustVehicle(v), max_distance);
	return result.found ? FindDepotData(TileIndex(result.tile), result.cost) : FindDepotData();
}
