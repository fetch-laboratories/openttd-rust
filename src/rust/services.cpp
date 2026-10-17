/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services.cpp Direct shared-world leaves for Rust game owners. */
#include "../stdafx.h"
#include "services_ffi.h"
#include "command_services.h"
#include "command_cost_native_services.h"
#include "company_services.h"
#ifdef WITH_RUST
#include "../industry_map.h"
#include "../bridge_map.h"
#include "services_rail.h"
#include "../road_map.h"
#include "../station_map.h"
#include "../tunnelbridge_map.h"
#include "../water_map.h"
#include "../landscape.h"
#include "../viewport_func.h"
#include "../core/random_func.hpp"
#include "../depot_map.h"
#include "../bridge.h"
#include "../water_map.h"
#include "../pbs.h"
#include "../company_base.h"
#include "../company_func.h"
#include "../command_type.h"
#include "../command_func.h"
#include "../landscape_cmd.h"
#include "../town.h"
#include "../timer/timer_game_calendar.h"
#include "tile_type_access.hpp"
#include "../safeguards.h"
#include "settings_services.h"
#include "environment_services.h"
#include "services_landscape.h"
#include "station_services.h"
#include "../clear_map.h"
#include "../tree_map.h"
#include "../cheat_type.h"
#include "../timer/timer_game_economy.h"

static CommandCost *NativeCost(OpenTTDCommandCost *cost) noexcept { return reinterpret_cast<CommandCost *>(cost); }
static const CommandCost *NativeCost(const OpenTTDCommandCost *cost) noexcept { return reinterpret_cast<const CommandCost *>(cost); }
static void CostZero(OpenTTDCommandCost *out) noexcept { OpenTTDCommandCostZero(NativeCost(out)); }
static void CostError(OpenTTDCommandCost *out, uint32_t message) noexcept { OpenTTDCommandCostError(NativeCost(out), message); }
static void CostExpenseMoney(OpenTTDCommandCost *out, uint8_t expense, int64_t amount) noexcept { OpenTTDCommandCostExpenseMoney(NativeCost(out), static_cast<ExpensesType>(expense), Money(amount)); }
static void CostErrorParam(OpenTTDCommandCost *out, uint32_t message, uint64_t parameter) noexcept { *NativeCost(out) = CommandCostWithParam(message, parameter); }
static void CostAdd(OpenTTDCommandCost *out, OpenTTDCommandCost *item) noexcept { NativeCost(out)->AddCost(std::move(*NativeCost(item))); }
static void CostMove(OpenTTDCommandCost *out, OpenTTDCommandCost *item) noexcept { *NativeCost(out) = std::move(*NativeCost(item)); }
static void CostAmount(OpenTTDCommandCost *out, int64_t amount) noexcept { NativeCost(out)->AddCost(Money(amount)); }
static bool CostSuccess(const OpenTTDCommandCost *cost) noexcept { return OpenTTDCommandCostSuccess(NativeCost(cost)); }
static uint32_t CostErrorMessage(const OpenTTDCommandCost *cost) noexcept { return NativeCost(cost)->GetErrorMessage(); }
static int64_t CostMoney(const OpenTTDCommandCost *cost) noexcept { return NativeCost(cost)->GetCost(); }
const OpenTTDCommandCostServices &GetRustCommandCostServices() noexcept
{
	static const OpenTTDCommandCostServices services{
		.zero = CostZero, .error = CostError, .expense_money = CostExpenseMoney,
		.error_param = CostErrorParam, .add = CostAdd, .move = CostMove,
		.amount = CostAmount, .success = CostSuccess, .error_message = CostErrorMessage, .money = CostMoney,
	};
	return services;
}

/* Direct command execution and its tile callbacks do not run a script VM or
 * save/load code. They may reenter Rust owners, but ordinary play cannot unwind
 * these leaves; allocation/error-formatting failures terminate under noexcept.
 * The native returned object is moved whole into its caller's stack sink. */
static void LandscapeClear(OpenTTDCommandCost *out, uint32_t tile, uint16_t flags) noexcept
{
	*NativeCost(out) = Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlags(flags), TileIndex{tile});
}
static void EnsureNoVehicle(OpenTTDCommandCost *out, uint32_t tile) noexcept
{
	*NativeCost(out) = RustEnsureNoVehicleOnGround(tile);
}
const OpenTTDCommandServices &GetRustCommandServices() noexcept
{
	static const OpenTTDCommandServices services{.landscape_clear = LandscapeClear, .ensure_no_vehicle = EnsureNoVehicle};
	return services;
}

static uint8_t SharedSnowLine() noexcept { return GetSnowLine(); }
static int32_t SharedCalendarYear() noexcept { return TimerGameCalendar::year.base(); }

OpenTTDTileTypeAllocation GetRustTileTypeAllocation() noexcept
{
	return OpenTTDNativeTileTypeAccess::Acquire();
}

static uint32_t SharedRandom(void *) noexcept { return Random(); }
const OpenTTDStationService *OpenTTDStationActivityOwner(const OpenTTDStation *station) noexcept
{
	return StationOf(station)->rust_service.get();
}

OpenTTDTownState *OpenTTDTownOwnerByID(uint32_t id) noexcept
{
	return Town::Get(TownID(id))->growth_owner.get();
}

void OpenTTDAssertTownGenerationContext() noexcept { assert(_game_mode == GM_EDITOR || _generating_world); }

bool OpenTTDIsTileFlat(uint32_t tile) noexcept { return IsTileFlat(TileIndex(tile)); }
ClearGround OpenTTDClearGround(uint32_t tile) noexcept { return RustClearGround(Tile{TileIndex{tile}}); }
TreeGround OpenTTDTreeGround(uint32_t tile) noexcept { return RustTreeGround(Tile{TileIndex{tile}}); }

static uint8_t SharedTileType(uint32_t tile) noexcept { return GetTileType(TileIndex{tile}); }
static bool SharedBridgeAbove(uint32_t tile) noexcept { return IsBridgeAbove(TileIndex{tile}); }
static uint8_t SharedTropicZone(uint32_t tile) noexcept { return GetTropicZone(TileIndex{tile}); }
static int32_t SharedTileZ(uint32_t tile) noexcept { return RustTileZ(TileIndex{tile}); }
static uint8_t SharedTileSlope(uint32_t tile) noexcept { return GetTileSlope(TileIndex{tile}); }
static void SharedMarkDirty(uint32_t tile) noexcept { MarkTileDirtyByTile(TileIndex{tile}); }
static void SharedSetTropicZone(uint32_t tile, uint8_t zone) noexcept { SetTropicZone(TileIndex{tile}, static_cast<TropicZone>(zone)); }

/** Pure map query; no allocation, reentry or C++ exception crosses into Rust. */
static uint32_t SharedIndustry(int32_t x, int32_t y, uint32_t *index) noexcept
{
	TileIndex tile = TileVirtXY(x, y);
	*index = tile.base();
	if (!IsTileType(tile, MP_INDUSTRY)) return 0;
	return GetIndustryGfx(tile) == GFX_BUBBLE_CATCHER ? 2 : 1;
}

static uint16_t MapSvcStationIndex(uint32_t tile) noexcept { return GetStationIndex(TileIndex(tile)).base(); }
static uint16_t MapSvcDepotIndex(uint32_t tile) noexcept { return GetDepotIndex(TileIndex(tile)).base(); }
static bool MapSvcRoadIsLevelCrossing(uint32_t tile) noexcept { return IsLevelCrossing(TileIndex(tile)); }
static bool MapSvcIsLevelCrossing(uint32_t tile) noexcept { return SharedTileType(tile) == MP_ROAD && MapSvcRoadIsLevelCrossing(tile); }
static uint8_t MapSvcTunnelBridgeDirection(uint32_t tile) noexcept { return GetTunnelBridgeDirection(TileIndex(tile)); }
static uint16_t MapSvcBridgeSpeed(uint32_t tile) noexcept { return GetBridgeSpec(GetBridgeType(TileIndex(tile)))->speed; }
static bool MapSvcIsWaterTile(uint32_t tile) noexcept { return IsWaterTile(TileIndex(tile)); }

static uint8_t SharedNextCompany(uint8_t from) noexcept
{
	/* The canonical iterator's UINT32_MAX sentinel narrows to this ABI's UINT8_MAX. */
	return static_cast<uint8_t>(RustNextCompanyID(from));
}

static const OpenTTDTownHandle *SharedClosestTown(uint32_t tile, uint32_t threshold) noexcept
{
	return reinterpret_cast<const OpenTTDTownHandle *>(ClosestTownFromTile(TileIndex{tile}, threshold));
}
static OpenTTDTownState *SharedTownOwner(const OpenTTDTownHandle *town) noexcept
{
	return reinterpret_cast<const Town *>(town)->growth_owner.get();
}

static bool Shared_tile_valid(uint32_t tile) noexcept { return IsValidTile(TileIndex{tile}); }
static uint32_t Shared_distance_from_edge(uint32_t tile) noexcept { return DistanceFromEdge(TileIndex{tile}); }
static int32_t Shared_tile_max_z(uint32_t tile) noexcept { return OpenTTDTileMaxZ(TileIndex{tile}); }
static_assert(TILE_HEIGHT == 8);
static uint32_t Shared_tile_height(uint32_t tile) noexcept { return TileHeight(TileIndex{tile}); }
static uint8_t Shared_bridge_axis(uint32_t tile) noexcept { return GetBridgeAxis(TileIndex{tile}); }
static bool Shared_has_water_ground(uint32_t tile) noexcept { return HasTileWaterGround(TileIndex{tile}); }
static bool Shared_is_sea(uint32_t tile) noexcept { return IsSea(TileIndex{tile}); }
static bool Shared_is_bridge(uint32_t tile) noexcept { return IsBridgeTile(TileIndex{tile}); }
static bool Shared_is_road_depot(uint32_t tile) noexcept { return IsRoadDepotTile(TileIndex{tile}); }
static uint8_t Shared_road_depot_direction(uint32_t tile) noexcept { return GetRoadDepotDirection(TileIndex{tile}); }
static bool Shared_is_normal_road(uint32_t tile) noexcept { return IsNormalRoadTile(TileIndex{tile}); }
static uint8_t Shared_disallowed_road_directions(uint32_t tile) noexcept { return GetDisallowedRoadDirections(TileIndex{tile}); }
static bool Shared_is_bay_stop(uint32_t tile) noexcept { return IsBayRoadStopTile(TileIndex{tile}); }
static uint8_t Shared_bay_stop_direction(uint32_t tile) noexcept { return GetBayRoadStopDir(TileIndex{tile}); }
static bool Shared_is_drive_through_stop(uint32_t tile) noexcept { return IsDriveThroughStopTile(TileIndex{tile}); }
static uint8_t Shared_drive_through_axis(uint32_t tile) noexcept { return GetDriveThroughStopAxis(TileIndex{tile}); }
static bool Shared_is_any_road_stop(uint32_t tile) noexcept { return IsAnyRoadStopTile(TileIndex{tile}); }
static uint8_t Shared_road_type(uint32_t tile) noexcept { return GetRoadTypeRoad(TileIndex{tile}); }
static bool Shared_has_road_type(uint32_t tile) noexcept { return HasTileRoadType(TileIndex{tile}, RTT_ROAD); }
static uint8_t Shared_road_owner(uint32_t tile) noexcept { return GetRoadOwner(TileIndex{tile}, RTT_ROAD).base(); }
static uint16_t Shared_town_index(uint32_t tile) noexcept { return GetTownIndex(TileIndex{tile}).base(); }
static uint8_t Shared_road_bits(uint32_t tile, uint8_t type, bool straight) noexcept { return GetAnyRoadBits(TileIndex{tile}, static_cast<RoadTramType>(type), straight); }
static uint32_t SharedMapMaxX() noexcept { return Map::MaxX(); }
static uint32_t SharedMapMaxY() noexcept { return Map::MaxY(); }
static uint8_t SharedPlaneSpeed() noexcept { return RustPlaneSpeed(); }
static bool SharedNoJetcrash() noexcept { return _cheats.no_jetcrash.value; }
static uint8_t SharedPlaneCrashes() noexcept { return RustPlaneCrashes(); }
static bool SharedServiceAtHelipad() noexcept { return RustServiceAtHelipad(); }
static bool SharedDisasterSound() noexcept { return RustDisasterSound(); }
static int32_t SharedEconomyDate() noexcept { return TimerGameEconomy::date.base(); }
static int32_t SharedCalendarDate() noexcept { return TimerGameCalendar::date.base(); }
static int32_t SharedSlope(int32_t x, int32_t y) noexcept { return GetSlopePixelZ(x, y); }

const OpenTTDSharedServices &GetRustSharedServices() noexcept
{
	static const OpenTTDSharedServices services{
		.context = nullptr,
		.random = SharedRandom,
		.industry = SharedIndustry,
		.tile_type = SharedTileType,
		.bridge_above = SharedBridgeAbove,
		.tropic_zone = SharedTropicZone,
		.tile_z = SharedTileZ,
		.tile_slope = SharedTileSlope,
		.mark_dirty = SharedMarkDirty,
		.set_tropic_zone = SharedSetTropicZone,
		.tile_owner = RustTileOwnerValue,
		.station_index = MapSvcStationIndex,
		.depot_index = MapSvcDepotIndex,
		.is_level_crossing = MapSvcIsLevelCrossing,
		.tunnel_bridge_direction = MapSvcTunnelBridgeDirection,
		.bridge_speed = MapSvcBridgeSpeed,
		.is_water_tile = MapSvcIsWaterTile,
		.company_valid = OpenTTDCompanyIsValid, .next_company = SharedNextCompany, .current_company = RustCurrentCompanyID,
		.closest_town = SharedClosestTown, .town_owner = SharedTownOwner,
		.tile_valid = Shared_tile_valid,
		.distance_from_edge = Shared_distance_from_edge,
		.tile_max_z = Shared_tile_max_z,
		.tile_height = Shared_tile_height,
		.bridge_axis = Shared_bridge_axis,
		.has_water_ground = Shared_has_water_ground,
		.is_sea = Shared_is_sea,
		.is_plain_rail = RustPlainRailTile,
		.is_bridge = Shared_is_bridge,
		.tunnel_transport = OpenTTDMapQuery<uint8_t, RustTunnelBridgeTransport, uint32_t>,
		.other_tunnel_end = RustOtherTunnelBridgeEnd,
		.is_road_depot = Shared_is_road_depot,
		.road_depot_direction = Shared_road_depot_direction,
		.is_normal_road = Shared_is_normal_road,
		.disallowed_road_directions = Shared_disallowed_road_directions,
		.is_bay_stop = Shared_is_bay_stop,
		.bay_stop_direction = Shared_bay_stop_direction,
		.is_drive_through_stop = Shared_is_drive_through_stop,
		.drive_through_axis = Shared_drive_through_axis,
		.is_any_road_stop = Shared_is_any_road_stop,
		.road_type = Shared_road_type,
		.has_road_type = Shared_has_road_type,
		.road_owner = Shared_road_owner,
		.town_index = Shared_town_index,
		.road_bits = Shared_road_bits,
		.snow_line = SharedSnowLine,
		.calendar_year = SharedCalendarYear,
		.map_log_x = OpenTTDMapLogX,
		.map_max_x = SharedMapMaxX,
		.map_max_y = SharedMapMaxY,
		.plane_speed = SharedPlaneSpeed,
		.no_jetcrash = SharedNoJetcrash,
		.plane_crashes = SharedPlaneCrashes,
		.service_at_helipad = SharedServiceAtHelipad,
		.disaster_sound = SharedDisasterSound,
		.economy_date = SharedEconomyDate,
		.calendar_date = SharedCalendarDate,
		.slope = SharedSlope,
		.clear_ground = OpenTTDClearGround,
		.tree_ground = OpenTTDTreeGround,
		.make_snow = RustMakeSnow,
		.make_shore = RustMakeShore,
		.tile_loop_water = RustTileLoopWater,
		.play_sound = RustPlayTileSound,
		.foundation = RustFoundationSlope,
		.set_fence = RustSetFieldFence,
		.has_water_class = RustHasWaterClass,
	};
	return services;
}
static OpenTTDRandomSeeds SharedSaveRandomSeeds() noexcept
{
	SavedRandomSeeds seeds;
	SaveRandomSeeds(&seeds);
	return {.random = {seeds.random.state[0], seeds.random.state[1]}, .interactive = {seeds.interactive_random.state[0], seeds.interactive_random.state[1]}};
}
static void SharedRestoreRandomSeeds(OpenTTDRandomSeeds seeds) noexcept
{
	RestoreRandomSeeds({{seeds.random[0], seeds.random[1]}, {seeds.interactive[0], seeds.interactive[1]}});
}
static void SharedSetRandomSeed(uint32_t seed) noexcept { SetRandomSeed(seed); }
const OpenTTDRandomServices &GetRustRandomServices() noexcept
{
	static const OpenTTDRandomServices services{.save = SharedSaveRandomSeeds, .restore = SharedRestoreRandomSeeds, .set_seed = SharedSetRandomSeed};
	return services;
}

static OpenTTDRailTraversal SharedRail_traversal(uint32_t tile) noexcept
{
	OpenTTDRailTraversal r{.depot = RustRailIsDepot(tile), .tracks = TRACK_BIT_NONE};
	if (!r.depot) r.tracks = RustRailTracks(tile);
	return r;
}
static OpenTTDRailSignalPresence SharedRail_signal_presence(uint32_t tile, uint8_t masked_tracks, uint8_t td) noexcept
{
	Trackdir dir = static_cast<Trackdir>(td);
	OpenTTDRailSignalPresence r{.signals = RustRailHasSignals(tile), .track = false, .kind = SIGTYPE_BLOCK, .forward = false, .reverse = false};
	if (r.signals) {
		Track track = TrackBitsToTrack(static_cast<TrackBits>(masked_tracks));
		uint present = RustRailPresentSignals(tile);
		r.track = (present & SignalOnTrack(track)) != 0;
		if (r.track) {
			r.kind = RustRailSignalType(tile, track);
			r.forward = (present & SignalAlongTrackdir(dir)) != 0;
			r.reverse = (present & SignalAlongTrackdir(ReverseTrackdir(dir))) != 0;
		}
	}
	return r;
}
static OpenTTDRailSignalUpdate SharedRail_signal_update(uint32_t tile, uint8_t td) noexcept
{
	TileIndex t(tile);
	assert(RustRailHasSignalTrackdir(tile, td));
	SignalType kind = RustRailSignalType(tile, TrackdirToTrack(static_cast<Trackdir>(td)));
	OpenTTDRailSignalUpdate r{.kind = kind, .reservations = TRACK_BIT_NONE};
	if (IsPbsSignal(kind)) r.reservations = GetRailReservationTrackBits(t);
	return r;
}
static OpenTTDRailStationAdmission SharedRail_station_admission(uint32_t tile, uint8_t owner, uint8_t dir) noexcept
{
	TileIndex t(tile);
	OpenTTDRailStationAdmission r{.rail = HasStationRail(t), .owner = INVALID_OWNER.base(), .axis = AXIS_X, .blocked = false};
	if (r.rail) {
		r.owner = RustTileOwnerValue(tile);
		if (r.owner == owner) {
			r.axis = RustRailStationAxis(tile);
			if (DiagDirToAxis(static_cast<DiagDirection>(dir)) == r.axis) r.blocked = IsStationTileBlocked(t);
		}
	}
	return r;
}
static OpenTTDRailCrossingAdmission SharedRail_crossing_admission(uint32_t tile, uint8_t owner) noexcept
{
	OpenTTDRailCrossingAdmission r{.crossing = MapSvcRoadIsLevelCrossing(tile), .owner = INVALID_OWNER.base(), .axis = AXIS_X};
	if (r.crossing) {
		r.owner = RustTileOwnerValue(tile);
		if (r.owner == owner) r.axis = RustCrossingRoadAxis(tile);
	}
	return r;
}
static OpenTTDRailWormholeAdmission SharedRail_wormhole_admission(uint32_t tile, uint8_t owner) noexcept
{
	OpenTTDRailWormholeAdmission r{.owner = RustTileOwnerValue(tile), .transport = TRANSPORT_RAIL, .direction = DIAGDIR_NE};
	if (r.owner == owner) {
		r.transport = RustTunnelBridgeTransport(tile);
		if (r.transport == TRANSPORT_RAIL) r.direction = static_cast<DiagDirection>(MapSvcTunnelBridgeDirection(tile));
	}
	return r;
}

const OpenTTDRailMapServices &GetRustRailMapServices() noexcept
{
	static const OpenTTDRailMapServices services{
		.traversal = SharedRail_traversal,
		.signal_presence = SharedRail_signal_presence,
		.signal_update = SharedRail_signal_update,
		.station_admission = SharedRail_station_admission,
		.crossing_admission = SharedRail_crossing_admission,
		.wormhole_admission = SharedRail_wormhole_admission,

		.is_rail_depot = RustRailIsDepot,
		.rail_depot_direction = RustRailDepotDirection,
		.has_signal_trackdir = RustRailHasSignalTrackdir,
		.signal_state = RustRailSignalState,
		.set_signal_state = RustRailSetSignalState,
		.other_tunnel_bridge_end = RustOtherTunnelBridgeEnd,
		.rail_track_status = RustRailTrackStatus,
	};
	return services;
}
#endif /* WITH_RUST */
