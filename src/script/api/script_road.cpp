/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file script_road.cpp Implementation of ScriptRoad. */

#include "../../stdafx.h"
#include "script_map.hpp"
#include "script_station.hpp"
#include "script_cargo.hpp"
#include "../../station_base.h"
#include "../../landscape_cmd.h"
#include "../../road_cmd.h"
#include "../../station_cmd.h"
#include "../../strings_func.h"
#include "../../newgrf_roadstop.h"
#include "../../script/squirrel_helper_type.hpp"

#include "../../safeguards.h"

/* static */ ScriptRoad::RoadVehicleType ScriptRoad::GetRoadVehicleTypeForCargo(CargoType cargo_type)
{
	return ScriptCargo::HasCargoClass(cargo_type, ScriptCargo::CC_PASSENGERS) ? ROADVEHTYPE_BUS : ROADVEHTYPE_TRUCK;
}

/* static */ std::optional<std::string> ScriptRoad::GetName(RoadType road_type)
{
	if (!IsRoadTypeAvailable(road_type)) return std::nullopt;

	return ::StrMakeValid(::GetString(GetRoadTypeInfo((::RoadType)road_type)->strings.name), {});
}

/* static */ bool ScriptRoad::ConvertRoadType(TileIndex start_tile, TileIndex end_tile, RoadType road_type)
{
	EnforceCompanyModeValid(false);
	if (!openttd_rust_script_road_convert(&GetRustScriptHostServices(), start_tile.base(), end_tile.base(), static_cast<uint32_t>(road_type))) return false;

	return ScriptObject::Command<CMD_CONVERT_ROAD>::Do(start_tile, end_tile, (::RoadType)road_type, false);
}

/* static */ bool ScriptRoad::_BuildRoadInternal(TileIndex start, TileIndex end, bool one_way, bool full)
{
	EnforceDeityOrCompanyModeValid(false);
	uint8_t axis = openttd_rust_script_road_build(&GetRustScriptHostServices(), start.base(), end.base(), one_way);
	if (axis == INVALID_AXIS) return false;

	return ScriptObject::Command<CMD_BUILD_LONG_ROAD>::Do(end, start, ScriptObject::GetRoadType(), static_cast<Axis>(axis), one_way ? DRD_NORTHBOUND : DRD_NONE, (start < end) == !full, (start < end) != !full, true);
}

/* static */ bool ScriptRoad::BuildRoad(TileIndex start, TileIndex end)
{
	return _BuildRoadInternal(start, end, false, false);
}

/* static */ bool ScriptRoad::BuildOneWayRoad(TileIndex start, TileIndex end)
{
	EnforceCompanyModeValid(false);
	return _BuildRoadInternal(start, end, true, false);
}

/* static */ bool ScriptRoad::BuildRoadFull(TileIndex start, TileIndex end)
{
	return _BuildRoadInternal(start, end, false, true);
}

/* static */ bool ScriptRoad::BuildOneWayRoadFull(TileIndex start, TileIndex end)
{
	EnforceCompanyModeValid(false);
	return _BuildRoadInternal(start, end, true, true);
}

/* static */ bool ScriptRoad::BuildRoadDepot(TileIndex tile, TileIndex front)
{
	EnforceCompanyModeValid(false);
	uint8_t entrance_dir = openttd_rust_script_road_depot(&GetRustScriptHostServices(), tile.base(), front.base());
	if (entrance_dir == INVALID_DIAGDIR) return false;

	return ScriptObject::Command<CMD_BUILD_ROAD_DEPOT>::Do(tile, ScriptObject::GetRoadType(), static_cast<DiagDirection>(entrance_dir));
}

/* static */ bool ScriptRoad::_BuildRoadStationInternal(TileIndex tile, TileIndex front, RoadVehicleType road_veh_type, bool drive_through, StationID station_id)
{
	EnforceCompanyModeValid(false);
	if (!openttd_rust_script_road_station_tiles(&GetRustScriptHostServices(), tile.base(), front.base())) return false;
	EnforcePrecondition(false, station_id == ScriptStation::STATION_NEW || station_id == ScriptStation::STATION_JOIN_ADJACENT || ScriptStation::IsValidStation(station_id));
	uint8_t entrance_dir = openttd_rust_script_road_station(&GetRustScriptHostServices(), tile.base(), front.base(), static_cast<uint32_t>(road_veh_type));
	if (entrance_dir == 0xFF) return false;

	RoadStopType stop_type = road_veh_type == ROADVEHTYPE_TRUCK ? RoadStopType::Truck : RoadStopType::Bus;
	StationID to_join = ScriptStation::IsValidStation(station_id) ? station_id : StationID::Invalid();
	return ScriptObject::Command<CMD_BUILD_ROAD_STOP>::Do(tile, 1, 1, stop_type, drive_through, static_cast<DiagDirection>(entrance_dir), ScriptObject::GetRoadType(), ROADSTOP_CLASS_DFLT, 0, to_join, station_id != ScriptStation::STATION_JOIN_ADJACENT);
}

/* static */ bool ScriptRoad::BuildRoadStation(TileIndex tile, TileIndex front, RoadVehicleType road_veh_type, StationID station_id)
{
	return _BuildRoadStationInternal(tile, front, road_veh_type, false, station_id);
}

/* static */ bool ScriptRoad::BuildDriveThroughRoadStation(TileIndex tile, TileIndex front, RoadVehicleType road_veh_type, StationID station_id)
{
	return _BuildRoadStationInternal(tile, front, road_veh_type, true, station_id);
}

/* static */ bool ScriptRoad::RemoveRoad(TileIndex start, TileIndex end)
{
	EnforceCompanyModeValid(false);
	if (!openttd_rust_script_road_remove(&GetRustScriptHostServices(), start.base(), end.base())) return false;

	return ScriptObject::Command<CMD_REMOVE_LONG_ROAD>::Do(end, start, ScriptObject::GetRoadType(), ::TileY(start) != ::TileY(end) ? AXIS_Y : AXIS_X, start < end, start >= end);
}

/* static */ bool ScriptRoad::RemoveRoadFull(TileIndex start, TileIndex end)
{
	EnforceCompanyModeValid(false);
	if (!openttd_rust_script_road_remove(&GetRustScriptHostServices(), start.base(), end.base())) return false;

	return ScriptObject::Command<CMD_REMOVE_LONG_ROAD>::Do(end, start, ScriptObject::GetRoadType(), ::TileY(start) != ::TileY(end) ? AXIS_Y : AXIS_X, start >= end, start < end);
}

/* static */ bool ScriptRoad::RemoveRoadDepot(TileIndex tile)
{
	EnforceCompanyModeValid(false);
	if (!openttd_rust_script_road_remove_depot(&GetRustScriptHostServices(), tile.base())) return false;

	return ScriptObject::Command<CMD_LANDSCAPE_CLEAR>::Do(tile);
}

/* static */ bool ScriptRoad::RemoveRoadStation(TileIndex tile)
{
	EnforceCompanyModeValid(false);
	uint8_t stop_type = openttd_rust_script_road_remove_station(&GetRustScriptHostServices(), tile.base());
	if (stop_type == 0xFF) return false;

	return ScriptObject::Command<CMD_REMOVE_ROAD_STOP>::Do(tile, 1, 1, static_cast<RoadStopType>(stop_type), false);
}

/* static */ ScriptRoad::RoadTramTypes ScriptRoad::GetRoadTramType(RoadType roadtype)
{
	return (RoadTramTypes)(1 << ::GetRoadTramType((::RoadType)roadtype));
}

/* static */ SQInteger ScriptRoad::GetMaintenanceCostFactor(RoadType roadtype)
{
	if (!ScriptRoad::IsRoadTypeAvailable(roadtype)) return 0;

	return GetRoadTypeInfo((::RoadType)roadtype)->maintenance_multiplier;
}
