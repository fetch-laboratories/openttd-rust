/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file script_rail.cpp Implementation of ScriptRail. */

#include "../../stdafx.h"
#include "script_rail.hpp"
#include "script_map.hpp"
#include "script_station.hpp"
#include "script_industrytype.hpp"
#include "script_cargo.hpp"
#include "../../debug.h"
#include "../../station_base.h"
#include "../../newgrf_generic.h"
#include "../../newgrf_station.h"
#include "../../strings_func.h"
#include "../../rail_cmd.h"
#include "../../station_cmd.h"
#include "../../waypoint_cmd.h"

#include "../../safeguards.h"

/* static */ std::optional<std::string> ScriptRail::GetName(RailType rail_type)
{
	if (!IsRailTypeAvailable(rail_type)) return std::nullopt;

	return ::StrMakeValid(::GetString(GetRailTypeInfo((::RailType)rail_type)->strings.menu_text), {});
}

/* static */ bool ScriptRail::TrainHasPowerOnRail(ScriptRail::RailType engine_rail_type, ScriptRail::RailType track_rail_type)
{\
	if (!ScriptRail::IsRailTypeAvailable(engine_rail_type)) return false;
	if (!ScriptRail::IsRailTypeAvailable(track_rail_type)) return false;

	return ::HasPowerOnRail((::RailType)engine_rail_type, (::RailType)track_rail_type);
}

/* static */ bool ScriptRail::ConvertRailType(TileIndex start_tile, TileIndex end_tile, ScriptRail::RailType convert_to)
{
	EnforceCompanyModeValid(false);
	if (!openttd_rust_script_rail_convert(&GetRustScriptHostServices(), start_tile.base(), end_tile.base(), static_cast<uint32_t>(convert_to))) return false;

	return ScriptObject::Command<CMD_CONVERT_RAIL>::Do(start_tile, end_tile, (::RailType)convert_to, false);
}

/* static */ bool ScriptRail::BuildRailDepot(TileIndex tile, TileIndex front)
{
	EnforceCompanyModeValid(false);
	uint8_t entrance_dir = openttd_rust_script_rail_depot(&GetRustScriptHostServices(), tile.base(), front.base());
	if (entrance_dir == INVALID_DIAGDIR) return false;

	return ScriptObject::Command<CMD_BUILD_TRAIN_DEPOT>::Do(tile, (::RailType)ScriptObject::GetRailType(), static_cast<DiagDirection>(entrance_dir));
}

/* static */ bool ScriptRail::BuildRailStation(TileIndex tile, RailTrack direction, SQInteger num_platforms, SQInteger platform_length, StationID station_id)
{
	EnforceCompanyModeValid(false);
	if (!openttd_rust_script_rail_station(&GetRustScriptHostServices(), tile.base(), static_cast<uint32_t>(direction), num_platforms, platform_length)) return false;
	EnforcePrecondition(false, station_id == ScriptStation::STATION_NEW || station_id == ScriptStation::STATION_JOIN_ADJACENT || ScriptStation::IsValidStation(station_id));

	bool adjacent = station_id != ScriptStation::STATION_JOIN_ADJACENT;
	return ScriptObject::Command<CMD_BUILD_RAIL_STATION>::Do(tile, (::RailType)GetCurrentRailType(), direction == RAILTRACK_NW_SE ? AXIS_Y : AXIS_X, num_platforms, platform_length, STAT_CLASS_DFLT, 0, ScriptStation::IsValidStation(station_id) ? station_id : StationID::Invalid(), adjacent);
}

/* static */ bool ScriptRail::BuildNewGRFRailStation(TileIndex tile, RailTrack direction, SQInteger num_platforms, SQInteger platform_length, StationID station_id, CargoType cargo_type, IndustryType source_industry, IndustryType goal_industry, SQInteger distance, bool source_station)
{
	EnforceCompanyModeValid(false);
	if (!openttd_rust_script_rail_station(&GetRustScriptHostServices(), tile.base(), static_cast<uint32_t>(direction), num_platforms, platform_length)) return false;
	EnforcePrecondition(false, station_id == ScriptStation::STATION_NEW || station_id == ScriptStation::STATION_JOIN_ADJACENT || ScriptStation::IsValidStation(station_id));
	EnforcePrecondition(false, ScriptCargo::IsValidCargo(cargo_type));
	EnforcePrecondition(false, source_industry == ScriptIndustryType::INDUSTRYTYPE_UNKNOWN || source_industry == ScriptIndustryType::INDUSTRYTYPE_TOWN || ScriptIndustryType::IsValidIndustryType(source_industry));
	EnforcePrecondition(false, goal_industry   == ScriptIndustryType::INDUSTRYTYPE_UNKNOWN || goal_industry   == ScriptIndustryType::INDUSTRYTYPE_TOWN || ScriptIndustryType::IsValidIndustryType(goal_industry));

	auto res = GetAiPurchaseCallbackResult(
		GSF_STATIONS,
		cargo_type,
		0,
		source_industry,
		goal_industry,
		ClampTo<uint8_t>(distance / 2),
		AICE_STATION_GET_STATION_ID,
		source_station ? 0 : 1,
		std::min<SQInteger>(15u, num_platforms) << 4 | std::min<SQInteger>(15u, platform_length)
	);

	Axis axis = direction == RAILTRACK_NW_SE ? AXIS_Y : AXIS_X;
	bool adjacent = station_id != ScriptStation::STATION_JOIN_ADJACENT;
	StationID to_join = ScriptStation::IsValidStation(station_id) ? station_id : StationID::Invalid();
	if (res.second != CALLBACK_FAILED) {
		const StationSpec *spec = StationClass::GetByGrf(res.first->grfid, res.second);
		if (spec == nullptr) {
			Debug(grf, 1, "{} returned an invalid station ID for 'AI construction/purchase selection (18)' callback", res.first->filename);
		} else {
			/* We might have gotten an usable station spec. Try to build it, but if it fails we'll fall back to the original station. */
			if (ScriptObject::Command<CMD_BUILD_RAIL_STATION>::Do(tile, (::RailType)GetCurrentRailType(), axis, num_platforms, platform_length, spec->class_index, spec->index, to_join, adjacent)) return true;
		}
	}

	return ScriptObject::Command<CMD_BUILD_RAIL_STATION>::Do(tile, (::RailType)GetCurrentRailType(), axis, num_platforms, platform_length, STAT_CLASS_DFLT, 0, to_join, adjacent);
}

/* static */ bool ScriptRail::BuildRailWaypoint(TileIndex tile)
{
	EnforceCompanyModeValid(false);
	uint8_t axis = openttd_rust_script_rail_waypoint(&GetRustScriptHostServices(), tile.base());
	if (axis == INVALID_AXIS) return false;

	return ScriptObject::Command<CMD_BUILD_RAIL_WAYPOINT>::Do(tile, static_cast<Axis>(axis), 1, 1, STAT_CLASS_WAYP, 0, StationID::Invalid(), false);
}

/* static */ bool ScriptRail::RemoveRailWaypointTileRectangle(TileIndex tile, TileIndex tile2, bool keep_rail)
{
	EnforceCompanyModeValid(false);
	EnforcePrecondition(false, ::IsValidTile(tile));
	EnforcePrecondition(false, ::IsValidTile(tile2));

	return ScriptObject::Command<CMD_REMOVE_FROM_RAIL_WAYPOINT>::Do(tile, tile2, keep_rail);
}

/* static */ bool ScriptRail::RemoveRailStationTileRectangle(TileIndex tile, TileIndex tile2, bool keep_rail)
{
	EnforceCompanyModeValid(false);
	EnforcePrecondition(false, ::IsValidTile(tile));
	EnforcePrecondition(false, ::IsValidTile(tile2));

	return ScriptObject::Command<CMD_REMOVE_FROM_RAIL_STATION>::Do(tile, tile2, keep_rail);
}

/* static */ bool ScriptRail::BuildRailTrack(TileIndex tile, RailTrack rail_track)
{
	EnforceCompanyModeValid(false);
	Track track = static_cast<Track>(openttd_rust_script_rail_build_track(&GetRustScriptHostServices(), tile.base(), static_cast<uint32_t>(rail_track)));
	if (track == INVALID_TRACK) return false;

	return ScriptObject::Command<CMD_BUILD_RAILROAD_TRACK>::Do(tile, tile, (::RailType)GetCurrentRailType(), track, false, false);
}

/* static */ bool ScriptRail::RemoveRailTrack(TileIndex tile, RailTrack rail_track)
{
	EnforceCompanyModeValid(false);
	Track track = static_cast<Track>(openttd_rust_script_rail_remove_track(&GetRustScriptHostServices(), tile.base(), static_cast<uint32_t>(rail_track)));
	if (track == INVALID_TRACK) return false;

	return ScriptObject::Command<CMD_REMOVE_RAILROAD_TRACK>::Do(tile, tile, track);
}

/* static */ bool ScriptRail::BuildRail(TileIndex from, TileIndex tile, TileIndex to)
{
	EnforceCompanyModeValid(false);
	auto plan = openttd_rust_script_rail_drag(&GetRustScriptHostServices(), from.base(), tile.base(), to.base(), true);
	if (plan.run == 0) return false;

	return ScriptObject::Command<CMD_BUILD_RAILROAD_TRACK>::Do(TileIndex(plan.to), tile, (::RailType)ScriptRail::GetCurrentRailType(), static_cast<Track>(plan.track), false, true);
}

/* static */ bool ScriptRail::RemoveRail(TileIndex from, TileIndex tile, TileIndex to)
{
	EnforceCompanyModeValid(false);
	auto plan = openttd_rust_script_rail_drag(&GetRustScriptHostServices(), from.base(), tile.base(), to.base(), false);
	if (plan.run == 0) return false;

	return ScriptObject::Command<CMD_REMOVE_RAILROAD_TRACK>::Do(TileIndex(plan.to), tile, static_cast<Track>(plan.track));
}

/* static */ bool ScriptRail::BuildSignal(TileIndex tile, TileIndex front, SignalType signal)
{
	EnforceCompanyModeValid(false);
	auto plan = openttd_rust_script_rail_build_signal(&GetRustScriptHostServices(), tile.base(), front.base(), static_cast<uint32_t>(signal));
	if (plan.run == 0) return false;

	return ScriptObject::Command<CMD_BUILD_SINGLE_SIGNAL>::Do(tile, static_cast<Track>(plan.track), static_cast<::SignalType>(plan.signal), ::SIG_ELECTRIC, false, false, false, ::SIGTYPE_BLOCK, ::SIGTYPE_BLOCK, plan.cycles, 0);
}

/* static */ bool ScriptRail::RemoveSignal(TileIndex tile, TileIndex front)
{
	EnforceCompanyModeValid(false);
	Track track = static_cast<Track>(openttd_rust_script_rail_remove_signal(&GetRustScriptHostServices(), tile.base(), front.base()));
	if (track == INVALID_TRACK) return false;

	return ScriptObject::Command<CMD_REMOVE_SINGLE_SIGNAL>::Do(tile, track);
}

/* static */ Money ScriptRail::GetBuildCost(RailType railtype, BuildType build_type)
{
	if (!ScriptRail::IsRailTypeAvailable(railtype)) return -1;

	switch (build_type) {
		case BT_TRACK:    return ::RailBuildCost((::RailType)railtype);
		case BT_SIGNAL:   return ::GetPrice(PR_BUILD_SIGNALS, 1, nullptr);
		case BT_DEPOT:    return ::GetPrice(PR_BUILD_DEPOT_TRAIN, 1, nullptr);
		case BT_STATION:  return ::GetPrice(PR_BUILD_STATION_RAIL, 1, nullptr) + ::GetPrice(PR_BUILD_STATION_RAIL_LENGTH, 1, nullptr);
		case BT_WAYPOINT: return ::GetPrice(PR_BUILD_WAYPOINT_RAIL, 1, nullptr);
		default: return -1;
	}
}

/* static */ SQInteger ScriptRail::GetMaxSpeed(RailType railtype)
{
	if (!ScriptRail::IsRailTypeAvailable(railtype)) return -1;

	return ::GetRailTypeInfo((::RailType)railtype)->max_speed;
}

/* static */ SQInteger ScriptRail::GetMaintenanceCostFactor(RailType railtype)
{
	if (!ScriptRail::IsRailTypeAvailable(railtype)) return 0;

	return ::GetRailTypeInfo((::RailType)railtype)->maintenance_multiplier;
}
