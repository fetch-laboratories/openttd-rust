/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services.cpp Direct shared-world leaves for Rust game owners. */
#include "../stdafx.h"
#include "../airport.h"
#include "town_services.h"
#include "services_ffi.h"
#include "station_command_ffi.h"
#include "road_command_ffi.h"
#include "presentation_services.h"
#include "tile_animation_services.h"
#include "vehicle_services.h"
#include "services_water.h"
#include "town_ffi.h"
#include "services_map.h"
#include "command_services.h"
#include "command_cost_native_services.h"
#include "company_services.h"
#include "company_adapter.h"
#include "../object_base.h"
#include "../strings_func.h"
#include "../table/strings.h"
#ifdef WITH_RUST
#include "../industry_map.h"
#include "../bridge_map.h"
#include "services_rail.h"
#include "services_road.h"
#include "../road_map.h"
#include "../station_map.h"
#include "../tunnelbridge_map.h"
#include "../water_map.h"
#include "../landscape.h"
#include "../viewport_func.h"
#include "../core/random_func.hpp"
#include "../depot_map.h"
#include "../bridge.h"
#include "../road_func.h"
#include "../road_internal.h"
#include "../pbs.h"
#include "../train.h"
#include "../company_base.h"
#include "../company_func.h"
#include "../command_type.h"
#include "../command_func.h"
#include "../landscape_cmd.h"
#include "../vehicle_func.h"
#include "../tunnelbridge.h"
#include "../town.h"
#include "../timer/timer_game_calendar.h"
#include "../newgrf_callbacks.h"
#include "../slope_func.h"
#include "tile_type_access.hpp"
#include "../safeguards.h"
#include "settings_services.h"
#include "environment_services.h"
#include "services_station.h"
#include "services_landscape.h"
#include "ground_ffi.h"
#include "../clear_map.h"
#include "../tree_map.h"
#include "../cheat_type.h"
#include "../timer/timer_game_economy.h"
#include "../economy_func.h"
#include "orders_ffi.h"
#include "tile_animation_services.h"

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
static ExpensesType CostExpense(const OpenTTDCommandCost *cost) noexcept { const auto expense = NativeCost(cost)->GetExpensesType(); assert(expense != INVALID_EXPENSES); return expense; }
static void CostMakeError(OpenTTDCommandCost *cost, uint32_t message) noexcept { NativeCost(cost)->MakeError(message); }
const OpenTTDCommandCostServices &GetRustCommandCostServices() noexcept
{
	static const OpenTTDCommandCostServices services{
		.zero = CostZero, .error = CostError, .expense_money = CostExpenseMoney,
		.error_param = CostErrorParam, .add = CostAdd, .move = CostMove,
		.amount = CostAmount, .success = CostSuccess, .error_message = CostErrorMessage, .money = CostMoney,
		.expense = CostExpense, .make_error = CostMakeError,
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
static void CommandTileOwnership(OpenTTDCommandCost *out, uint32_t tile) noexcept { *NativeCost(out) = CheckTileOwnership(TileIndex{tile}); }
static void CommandOwnership(OpenTTDCommandCost *out, uint8_t owner, uint32_t tile) noexcept { *NativeCost(out) = CheckOwnership(Owner(owner), TileIndex{tile}); }
static void CommandNoVehicle(OpenTTDCommandCost *out, uint32_t tile) noexcept { *NativeCost(out) = EnsureNoVehicleOnGround(TileIndex{tile}); }
static void CommandNoTrain(OpenTTDCommandCost *out, uint32_t tile, uint8_t bits) noexcept { *NativeCost(out) = EnsureNoTrainOnTrackBits(TileIndex{tile}, static_cast<TrackBits>(bits)); }
static void CommandWormholeFree(OpenTTDCommandCost *out, uint32_t tile, uint32_t end, const Train *ignore) noexcept { *NativeCost(out) = TunnelBridgeIsFree(TileIndex{tile}, TileIndex{end}, ignore); }
static void CommandClearObjectAreas() noexcept { _cleared_object_areas.clear(); }
static bool CommandInfiniteMoney() noexcept { return _settings_game.difficulty.infinite_money; }
static void CommandMoneyError(OpenTTDCommandCost *cost, int64_t amount) noexcept
{
	NativeCost(cost)->MakeError(STR_ERROR_NOT_ENOUGH_CASH_REQUIRES_CURRENCY);
	if (RustIsLocalCompany()) NativeCost(cost)->SetEncodedMessage(GetEncodedString(STR_ERROR_NOT_ENOUGH_CASH_REQUIRES_CURRENCY, Money(amount)));
}
const OpenTTDEnvironmentServices &GetRustEnvironmentServices() noexcept
{
	static const OpenTTDEnvironmentServices services{.editor = OpenTTDEditor, .freeform_edges = OpenTTDFreeformEdges};
	return services;
}
const OpenTTDCommandServices &GetRustCommandServices() noexcept
{
	static const OpenTTDCommandServices services{
		.enter = RecursiveCommandCounter::Enter, .leave = RecursiveCommandCounter::Leave,
		.finance = &GetRustCompanyFinanceServices(),
		.clear_object_areas = CommandClearObjectAreas, .infinite_money = CommandInfiniteMoney, .money_error = CommandMoneyError,
		.infrastructure_dirty = RustInfrastructureDirty,
		.landscape_clear = LandscapeClear,
		.tile_ownership = CommandTileOwnership, .ownership = CommandOwnership,
		.no_vehicle = CommandNoVehicle, .no_train = CommandNoTrain, .wormhole_free = CommandWormholeFree,
	};
	return services;
}

const OpenTTDWaterRestorationServices &GetRustWaterRestorationServices() noexcept
{
	static const OpenTTDWaterRestorationServices services{
		.slope_z = RustTileSlopeZ, .infrastructure = RustAddWaterInfrastructure,
		.infrastructure_dirty = GetRustCommandServices().infrastructure_dirty,
		.make_canal = RustMakeCanal, .make_river = RustMakeRiver,
	};
	return services;
}

static RoadTramType Shared_road_tram_type(uint8_t type) noexcept { return RustRoadTramType(static_cast<RoadType>(type)); }

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

Town *OpenTTDTownShellByID(uint32_t id) noexcept
{
	return Town::Get(TownID(id));
}

void OpenTTDAssertTownGenerationContext() noexcept { assert(_game_mode == GM_EDITOR || _generating_world); }

bool OpenTTDIsTileFlat(uint32_t tile) noexcept { return IsTileFlat(TileIndex(tile)); }
ClearGround OpenTTDClearGround(uint32_t tile) noexcept { return RustClearGround(Tile{TileIndex{tile}}); }
TreeGround OpenTTDTreeGround(uint32_t tile) noexcept { return RustTreeGround(Tile{TileIndex{tile}}); }

static bool SharedBridgeAbove(uint32_t tile) noexcept { return IsBridgeAbove(TileIndex{tile}); }
static uint8_t SharedTropicZone(uint32_t tile) noexcept { return GetTropicZone(TileIndex{tile}); }
static int32_t SharedTileZ(uint32_t tile) noexcept { return RustTileZ(TileIndex{tile}); }
static uint8_t SharedTileSlope(uint32_t tile) noexcept { return GetTileSlope(TileIndex{tile}); }
static void SharedMarkDirty(uint32_t tile) noexcept { MarkTileDirtyByTile(TileIndex{tile}); }
static void SharedFreeTrainReservation(const Train *v) noexcept { FreeTrainTrackReservation(v); }
static bool SharedTryPathReserve(Train *v, bool mark_as_stuck, bool first_tile_okay) noexcept { return TryPathReserve(v, mark_as_stuck, first_tile_okay); }
static bool Shared_may_have_road(uint32_t tile) noexcept { return MayHaveRoad(TileIndex{tile}); }
static RoadTileType Shared_road_tile_kind(uint32_t tile) noexcept { return GetRoadTileType(TileIndex{tile}); }
static void Shared_set_road_bits(uint32_t tile, uint8_t bits, uint8_t rtt) noexcept { SetRoadBits(TileIndex{tile}, static_cast<RoadBits>(bits), static_cast<RoadTramType>(rtt)); }
static void Shared_set_road_type(uint32_t tile, uint8_t rtt, uint8_t rt) noexcept { RustSetRoadType(TileIndex{tile}, static_cast<RoadTramType>(rtt), static_cast<RoadType>(rt)); }
static void Shared_set_road_directions(uint32_t tile, uint8_t drd) noexcept { SetDisallowedRoadDirections(TileIndex{tile}, static_cast<DisallowedRoadDirections>(drd)); }
static bool Shared_road_powered(uint8_t from, uint8_t to) noexcept { return HasPowerOnRoad(static_cast<RoadType>(from), static_cast<RoadType>(to)); }
static bool Shared_road_type_valid(uint8_t rt) noexcept { return ValParamRoadType(static_cast<RoadType>(rt)); }
static bool SharedSafeWaitingPosition(const Train *v, uint32_t tile, uint8_t trackdir, bool forbid_90_deg) noexcept { return IsSafeWaitingPosition(v, TileIndex{tile}, static_cast<Trackdir>(trackdir), true, forbid_90_deg); }
static void SharedSetTropicZone(uint32_t tile, uint8_t zone) noexcept { SetTropicZone(TileIndex{tile}, static_cast<TropicZone>(zone)); }

/** Pure map query; no allocation, reentry or C++ exception crosses into Rust. */
static uint32_t SharedIndustry(int32_t x, int32_t y, uint32_t *index) noexcept
{
	TileIndex tile = TileVirtXY(x, y);
	*index = tile.base();
	if (!IsTileType(tile, MP_INDUSTRY)) return 0;
	return GetIndustryGfx(tile) == GFX_BUBBLE_CATCHER ? 2 : 1;
}

static uint16_t MapSvcDepotIndex(uint32_t tile) noexcept { return GetDepotIndex(TileIndex(tile)).base(); }
static uint8_t MapSvcTunnelBridgeDirection(uint32_t tile) noexcept { return GetTunnelBridgeDirection(TileIndex(tile)); }
static uint16_t MapSvcBridgeSpeed(uint32_t tile) noexcept { return GetBridgeSpec(GetBridgeType(TileIndex(tile)))->speed; }
static bool MapSvcIsWaterTile(uint32_t tile) noexcept { return IsWaterTile(TileIndex(tile)); }

static uint8_t SharedNextCompany(uint8_t from) noexcept
{
	/* The canonical iterator's UINT32_MAX sentinel narrows to this ABI's UINT8_MAX. */
	return static_cast<uint8_t>(RustNextCompanyID(from));
}

static const OpenTTDTownHandle *SharedCalcClosestTown(uint32_t tile, uint32_t threshold) noexcept
{
	return reinterpret_cast<const OpenTTDTownHandle *>(RustCalcClosestTown(TileIndex{tile}, threshold));
}
static const OpenTTDTownHandle *SharedClosestTown(uint32_t tile, uint32_t threshold) noexcept
{
	return reinterpret_cast<const OpenTTDTownHandle *>(ClosestTownFromTile(TileIndex{tile}, threshold));
}
static OpenTTDTownState *SharedTownOwner(const OpenTTDTownHandle *town) noexcept
{
	return reinterpret_cast<const Town *>(town)->growth_owner.get();
}

OpenTTDTownState *OpenTTDTownOwnerByID(uint32_t id) noexcept
{
	return SharedTownOwner(reinterpret_cast<const OpenTTDTownHandle *>(OpenTTDTownShellByID(id)));
}

OpenTTDTownRef OpenTTDTownResolveByID(uint32_t id) noexcept
{
	Town *t = OpenTTDTownShellByID(id);
	return {.shell = t, .state = SharedTownOwner(reinterpret_cast<const OpenTTDTownHandle *>(t)), .id = static_cast<uint16_t>(id)};
}

static bool Shared_tile_valid(uint32_t tile) noexcept { return IsValidTile(TileIndex{tile}); }
static uint32_t Shared_distance_from_edge(uint32_t tile) noexcept { return DistanceFromEdge(TileIndex{tile}); }
static int32_t Shared_tile_max_z(uint32_t tile) noexcept { return OpenTTDTileMaxZ(TileIndex{tile}); }
static_assert(TILE_HEIGHT == 8);
/* Values of the shared Rust header modules (tile_type.rs, slope_func.rs and the like). */
static_assert(MP_CLEAR == 0 && MP_RAILWAY == 1 && MP_ROAD == 2 && MP_HOUSE == 3 && MP_TREES == 4 && MP_STATION == 5 && MP_WATER == 6 && MP_TUNNELBRIDGE == 9);
static_assert(INVALID_TILE.base() == UINT32_MAX && TILE_HEIGHT_STEP == 50 && CALLBACK_FAILED == 0xFFFF && INVALID_STRING_ID == 0xFFFF);
static_assert(SLOPE_FLAT == 0 && SLOPE_W == 1 && SLOPE_S == 2 && SLOPE_E == 4 && SLOPE_N == 8 && SLOPE_STEEP == 0x10 && SLOPE_HALFTILE == 0x20 && SLOPE_HALFTILE_MASK == 0xE0);
static_assert(DIAGDIR_NE == 0 && DIAGDIR_SE == 1 && DIAGDIR_SW == 2 && DIAGDIR_NW == 3 && INVALID_DIAGDIR == 0xFF);
static_assert(OWNER_TOWN.base() == 0x0F && OWNER_NONE.base() == 0x10 && OWNER_WATER.base() == 0x11 && OWNER_DEITY.base() == 0x12 && INVALID_OWNER.base() == 0xFF);
static_assert(to_underlying(DoCommandFlag::Execute) == 0 && to_underlying(DoCommandFlag::Auto) == 1 && to_underlying(DoCommandFlag::QueryCost) == 2 && to_underlying(DoCommandFlag::NoWater) == 3);
static_assert(to_underlying(DoCommandFlag::NoTestTownRating) == 4 && to_underlying(DoCommandFlag::Bankrupt) == 5 && to_underlying(DoCommandFlag::NoModifyTownRating) == 9 && to_underlying(DoCommandFlag::ForceClearTile) == 10);
static_assert(std::is_same_v<DoCommandFlags::BaseType, uint16_t>);
static_assert(AT_OILRIG == 9 && OBJECT_STATUE == 2 && EXPENSES_OTHER == 12 && COMPANY_SPECTATOR.base() == 0xFF && MP_INDUSTRY == 8 && MP_OBJECT == 10);
static_assert(RATING_MINIMUM == -1000 && RATING_MAXIMUM == 1000 && RATING_BRIBE_UP_STEP == 200 && RATING_BRIBE_MAXIMUM == 800 && RATING_BRIBE_DOWN_TO == -50);
static_assert(PR_BUILD_SIGNALS == 3 && PR_BUILD_DEPOT_TRAIN == 5 && PR_CLEAR_ROUGH == 23 && PR_CLEAR_SIGNALS == 28 && PR_CLEAR_DEPOT_TRAIN == 30 && PR_CLEAR_WATER == 34 && PR_TOWN_ACTION == 56 && PR_BUILD_FOUNDATION == 57);
static uint32_t Shared_tile_height(uint32_t tile) noexcept { return TileHeight(TileIndex{tile}); }
static uint8_t Shared_bridge_axis(uint32_t tile) noexcept { return GetBridgeAxis(TileIndex{tile}); }
static bool Shared_has_water_ground(uint32_t tile) noexcept { return HasTileWaterGround(TileIndex{tile}); }
static bool Shared_is_sea(uint32_t tile) noexcept { return IsSea(TileIndex{tile}); }
static bool Shared_is_bridge(uint32_t tile) noexcept { return IsBridgeTile(TileIndex{tile}); }
static uint8_t Shared_drive_through_axis(uint32_t tile) noexcept { return GetDriveThroughStopAxis(TileIndex{tile}); }
static bool Shared_is_any_road_stop(uint32_t tile) noexcept { return IsAnyRoadStopTile(TileIndex{tile}); }
static uint8_t Shared_road_type(uint32_t tile) noexcept { return GetRoadTypeRoad(TileIndex{tile}); }
static bool Shared_has_road_type(uint32_t tile) noexcept { return HasTileRoadType(TileIndex{tile}, RTT_ROAD); }
static uint8_t Shared_road_owner(uint32_t tile) noexcept { return GetRoadOwner(TileIndex{tile}, RTT_ROAD).base(); }
static uint16_t Shared_town_index(uint32_t tile) noexcept { return GetTownIndex(TileIndex{tile}).base(); }
static uint8_t Shared_road_bits(uint32_t tile, uint8_t type, bool straight) noexcept { return GetAnyRoadBits(TileIndex{tile}, static_cast<RoadTramType>(type), straight); }
static void SharedUpdateLevelCrossing(uint32_t tile, bool sound) noexcept { UpdateLevelCrossing(TileIndex{tile}, sound); }
static void SharedMarkBridgeDirty(uint32_t tile) noexcept { MarkBridgeDirty(TileIndex{tile}); }
static Trackdir SharedTrainTrackdir(const Train *v) noexcept { return v->GetVehicleTrackdir(); }
static bool SharedHasRoadWorks(uint32_t tile) noexcept { return HasRoadWorks(TileIndex{tile}); }
static RoadType SharedTramType(uint32_t tile) noexcept { return GetRoadTypeTram(TileIndex{tile}); }
static uint8_t SharedTramOwner(uint32_t tile) noexcept { return GetRoadOwner(TileIndex{tile}, RTT_TRAM).base(); }
static RoadBits SharedNormalRoadBits(uint32_t tile, uint8_t type) noexcept { return GetRoadBits(TileIndex{tile}, static_cast<RoadTramType>(type)); }
static RoadBits SharedCrossingRoadBits(uint32_t tile) noexcept { return GetCrossingRoadBits(TileIndex{tile}); }
static uint32_t SharedMapMaxX() noexcept { return Map::MaxX(); }
static uint32_t SharedMapMaxY() noexcept { return Map::MaxY(); }
static uint8_t SharedPlaneSpeed() noexcept { return RustPlaneSpeed(); }
static bool SharedNoJetcrash() noexcept { return _cheats.no_jetcrash.value; }
static uint8_t SharedPlaneCrashes() noexcept { return RustPlaneCrashes(); }
static bool SharedServiceAtHelipad() noexcept { return RustServiceAtHelipad(); }
static bool SharedDisasterSound() noexcept { return RustDisasterSound(); }
static int32_t SharedEconomyDate() noexcept { return TimerGameEconomy::date.base(); }
static int32_t SharedCalendarDate() noexcept { return TimerGameCalendar::date.base(); }
static int32_t SharedSlope(int32_t x, int32_t y) noexcept { return SharedSlopePixelZ(x, y, false); }
static size_t SharedTownCount() noexcept { return Town::GetNumItems(); }
static bool SharedRecession() noexcept { return EconomyIsInRecession(); }
static void SharedRebuildTownTree() noexcept { RebuildTownKdtree(); }

constinit const OpenTTDSharedServices _rust_shared_services{
	.set_tile_owner = RustSetTileOwner,
	.autoslope = RustAutoslopeEnabled,
	.bay_stop_direction = Shared_bay_stop_direction,
	.bridge_above = SharedBridgeAbove,
	.bridge_axis = Shared_bridge_axis,
	.bridge_speed = MapSvcBridgeSpeed,
	.build_on_slopes = RustBuildOnSlopes,
	.calendar_date = SharedCalendarDate,
	.calendar_year = SharedCalendarYear,
	.clear_square_services = {
		.may_animate = OpenTTDMayAnimateTile, .delete_animated = OpenTTDDeleteAnimatedTileWithMode,
		.docking = OpenTTDIsDockingTile,
		.generating_world = OpenTTDGeneratingWorld,
	},
	.calc_closest_town = SharedCalcClosestTown,
	.closest_town = SharedClosestTown,
	.company_valid = OpenTTDCompanyIsValid,
	.context = nullptr,
	.crossing_road_bits = SharedCrossingRoadBits,
	.current_company = RustCurrentCompanyID,
	.depot_index = MapSvcDepotIndex,
	.disallowed_road_directions = Shared_disallowed_road_directions,
	.disaster_sound = SharedDisasterSound,
	.distance_from_edge = Shared_distance_from_edge,
	.drive_through_axis = Shared_drive_through_axis,
	.economy_date = SharedEconomyDate,
	.forbid_90_deg = RustForbid90Deg,
	.free_train_reservation = SharedFreeTrainReservation,
	.has_road_type = Shared_has_road_type,
	.has_road_works = SharedHasRoadWorks,
	.has_water_ground = Shared_has_water_ground,
	.industry = SharedIndustry,
	.is_any_road_stop = Shared_is_any_road_stop,
	.is_bay_stop = Shared_is_bay_stop,
	.is_bridge = Shared_is_bridge,
	.is_drive_through_stop = Shared_is_drive_through_stop,
	.is_level_crossing = MapSvcIsLevelCrossing,
	.is_normal_road = Shared_is_normal_road,
	.is_plain_rail = RustPlainRailTile,
	.is_road_depot = Shared_is_road_depot,
	.is_sea = Shared_is_sea,
	.is_water_tile = MapSvcIsWaterTile,
	.map_log_x = OpenTTDMapLogX,
	.map_max_x = SharedMapMaxX,
	.map_max_y = SharedMapMaxY,
	.mark_bridge_dirty = SharedMarkBridgeDirty,
	.mark_dirty = SharedMarkDirty,
	.next_company = SharedNextCompany,
	.no_jetcrash = SharedNoJetcrash,
	.normal_road_bits = SharedNormalRoadBits,
	.other_tunnel_end = RustOtherTunnelBridgeEnd,
	.plane_crashes = SharedPlaneCrashes,
	.plane_speed = SharedPlaneSpeed,
	.random = SharedRandom,
	.road_bits = Shared_road_bits,
	.road_tram_type = Shared_road_tram_type, .road_type_is_road = RustRoadTypeIsRoad, .road_cost_multiplier = RustRoadCostMultiplier,
	.road_depot_direction = Shared_road_depot_direction,
	.road_owner = Shared_road_owner,
	.road_type = Shared_road_type,
	.crossing_reserved = RustCrossingReserved,
	.set_crossing_reserved = RustSetCrossingReserved,
	.may_have_road = Shared_may_have_road,
	.road_tile_kind = Shared_road_tile_kind,
	.set_road_bits = Shared_set_road_bits,
	.set_road_type = Shared_set_road_type,
	.set_road_owner = RustSetRoadOwner,
	.set_road_directions = Shared_set_road_directions,
	.road_powered = Shared_road_powered,
	.road_type_valid = Shared_road_type_valid,
	.safe_waiting_position = SharedSafeWaitingPosition,
	.service_at_helipad = SharedServiceAtHelipad,
	.set_tropic_zone = SharedSetTropicZone,
	.slope = SharedSlope,
	.snow_line = SharedSnowLine,
	.station_index = MapSvcStationIndex,
	.road_commands = []() noexcept { return &GetRustRoadCommandServices(); },
	.station_commands = []() noexcept { return &GetRustStationCommandServices(); },
	.docking_tile = OpenTTDIsDockingTile,
	.station_gfx = [](uint32_t tile) noexcept { return GetStationGfx(TileIndex{tile}); },
	.station_serve_neutral = RustServeNeutralIndustries,
	.station_type = [](uint32_t tile) noexcept { return GetStationType(TileIndex{tile}); },
	.tile_animation_add = OpenTTDAddAnimatedTile,
	.tile_animation_delete = OpenTTDDeleteAnimatedTile,
	.tile_animation_set_frame = OpenTTDSetAnimationFrame,
	.town_next_id = OpenTTDNextTownID,
	.town_owner_by_id = OpenTTDTownOwnerByID,
	.utf8_length = RustUtf8Length,
	.vehicle_next = RustNextVehicle,
	.water_middle_lock = OpenTTDIsMiddleLock,
	.tile_height = Shared_tile_height,
	.tile_max_z = Shared_tile_max_z,
	.tile_owner = RustTileOwnerValue,
	.tile_slope = SharedTileSlope,
	.tile_type = SharedTileType,
	.tile_add_wrap = SharedTileAddWrap,
	.tile_valid = Shared_tile_valid,
	.tile_z = SharedTileZ,
	.town_index = Shared_town_index,
	.town_owner = SharedTownOwner,
	.train_trackdir = SharedTrainTrackdir,
	.tram_owner = SharedTramOwner,
	.tram_type = SharedTramType,
	.tropic_zone = SharedTropicZone,
	.try_path_reserve = SharedTryPathReserve,
	.tunnel_bridge_direction = MapSvcTunnelBridgeDirection,
	.tunnel_transport = OpenTTDMapQuery<uint8_t, RustTunnelBridgeTransport, uint32_t>,
	.update_level_crossing = SharedUpdateLevelCrossing,
	// Registry: house (#256).
	.rebuild_town_tree = SharedRebuildTownTree,
	.recession = SharedRecession,
	.tick_counter = openttd_orders_tick_counter,
	.town_count = SharedTownCount,

	// Registry: landscape (#212).
	.clear_ground = OpenTTDClearGround,
	.foundation = RustFoundationSlope,
	.has_water_class = RustHasWaterClass,
	.make_shore = OpenTTDMakeShore,
	.make_snow = RustMakeSnow,
	.play_sound = RustPlayTileSound,
	.set_fence = RustSetFieldFence,
	.tree_ground = OpenTTDTreeGround,
	.water_services = []() noexcept { return &GetRustWaterGroundServices(); },
};

const OpenTTDSharedServices &GetRustSharedServices() noexcept
{
	return _rust_shared_services;
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
	assert(RustRailHasSignalTrackdir(tile, td));
	SignalType kind = RustRailSignalType(tile, TrackdirToTrack(static_cast<Trackdir>(td)));
	OpenTTDRailSignalUpdate r{.kind = kind, .reservations = TRACK_BIT_NONE};
	if (IsPbsSignal(kind)) r.reservations = RustRailReservation(tile);
	return r;
}
static OpenTTDRailStationAdmission SharedRail_station_admission(uint32_t tile, uint8_t owner, uint8_t dir) noexcept
{
	TileIndex t(tile);
	OpenTTDRailStationAdmission r{.rail = RustHasStationRail(tile), .owner = INVALID_OWNER.base(), .axis = AXIS_X, .blocked = false};
	if (r.rail) {
		r.owner = RustTileOwnerValue(tile);
		if (r.owner == owner) {
			r.axis = RustRailStationAxis(tile);
			if (DiagDirToAxis(static_cast<DiagDirection>(dir)) == r.axis) r.blocked = RustStationBlocked(tile);
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
		.ground = &GetRustRailGroundServices(),
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
		.has_signals = RustRailHasSignals,
		.signal_type = RustRailSignalType,
		.invalid_rail_type = RustInvalidRailType,
		.rail_type = RustRailType,
		.set_rail_type = RustSetRailType,
		.rail_tile_type = RustRailTileType,
		.set_track_bits = RustSetTrackBits,
		.present_signals = RustRailPresentSignals,
		.set_present_signals = RustSetPresentSignals,
		.signal_states = RustSignalStates,
		.set_signal_states = RustSetSignalStates,
		.set_has_signals = RustSetHasSignals,
		.set_signal_type = RustSetSignalType,
		.signal_variant = RustSignalVariant,
		.set_signal_variant = RustSetSignalVariant,
		.cycle_signal_side = RustCycleSignalSide,
		.reserved_tracks = RustReservedTracks,
		.rail_reservation = RustRailReservation,
		.set_track_reservation = RustSetTrackReservation,
		.depot_reserved = RustDepotReserved,
		.set_depot_exit = RustSetDepotExit,
		.make_rail_normal = RustMakeRailNormal,
		.make_rail_depot = RustMakeRailDepot,
		.crossing_rail_bits = RustCrossingRailBits,
		.crossing_road_axis = RustCrossingRoadAxis,
		.has_station_rail = RustHasStationRail,
		.is_rail_station = RustIsRailStation,
		.station_blocked = RustStationBlocked,
		.station_track = RustRailStationTrack,
		.wormhole_reserved = RustWormholeReserved,
	};
	return services;
}
#endif /* WITH_RUST */

#ifdef WITH_RUST
#include "service_table_check.hpp"
#include "abi-visit-OpenTTDRandomServices.hpp"
#include "abi-visit-OpenTTDRailMapServices.hpp"
#include "abi-visit-OpenTTDCommandServices.hpp"
#include "abi-visit-OpenTTDCommandCostServices.hpp"
#include "abi-visit-OpenTTDSharedServices.hpp"
#include "abi-visit-OpenTTDEnvironmentServices.hpp"
#include "abi-visit-OpenTTDClearSquareServices.hpp"
#include "abi-visit-OpenTTDWaterRestorationServices.hpp"
bool CheckAbi_services()
{
	return AbiServicesComplete(GetRustEnvironmentServices()) &&
		AbiServicesComplete(GetRustRandomServices()) &&
		AbiServicesComplete(GetRustRailMapServices()) &&
		AbiServicesComplete(GetRustCommandServices()) &&
		AbiServicesComplete(GetRustCommandCostServices()) &&
		AbiServicesComplete(GetRustSharedServices()) &&
		AbiServicesComplete(GetRustSharedServices().clear_square_services) &&
		AbiServicesComplete(GetRustWaterRestorationServices());
}
#endif
