/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services.cpp Direct shared-world leaves for Rust game owners. */
#include "../stdafx.h"
#include "command_post_ffi.h"
#include "../source_type.h"
#include "../effectvehicle_func.h"
#include "../airport.h"
#include "town_services.h"
#include "services_ffi.h"
#include "pbs_ffi.h"
#include "movement_ffi.h"
#include "services_bridge.h"
#include "services_viewport.h"
#include "station_command_ffi.h"
#include "station_catchment_ffi.h"
#include "road_command_ffi.h"
#include "road_stop_ffi.h"
#include "presentation_services.h"
#include "services_assert.h"
#include "tile_animation_services.h"
#include "vehicle_services.h"
#include "services_water.h"
#include "town_ffi.h"
#include "services_map.h"
#include "command_services.h"
#include "landscape_command_ffi.h"
#include "command_cost_native_services.h"
#include "company_services.h"
#include "company_adapter.h"
#include "../object_base.h"
#include "../strings_func.h"
#include "../table/strings.h"
#include "newgrf_bytereader_services.h"
#include "../aircraft.h"
#include "../airport.h"
#include "../engine_type.h"
#include "../industry_map.h"
#include "../industry.h"
#include "../bridge_map.h"
#include "services_rail.h"
#include "services_newgrf.h"
#include "spec_services.hpp"
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
#include "../linkgraph/linkgraph_type.h"
#include "../linkgraph/linkgraphjob.h"
#include "services_linkgraph.h"
#include "../company_base.h"
#include "../network/network_base.h"
#include "../company_func.h"
#include "../command_type.h"
#include "../command_func.h"
#include "../landscape_cmd.h"
#include "../vehicle_func.h"
#include "../vehiclelist.h"
#include "../tunnelbridge.h"
#include "../town.h"
#include "../timer/timer_game_calendar.h"
#include "../newgrf_callbacks.h"
#include "../slope_func.h"
#include "tile_type_access.hpp"
#include "../effectvehicle_func.h"
#include "clock_ffi.h"
#include "../error.h"
#include "../gfx_func.h"
#include "../gui.h"
#include "../texteff.hpp"
#include "newgrf_storage_ffi.h"
#include "signal_ffi.h"
#include "debug_ffi.h"
#include "engine_services.h"
#include "../debug.h"
#include "../newgrf.h"
#include "../safeguards.h"
static_assert(VE_OFFSET_START == 0);
static_assert(VE_OFFSET_CENTRE == 8);
static_assert(VE_TYPE_START == 4);
static_assert(VE_TYPE_DEFAULT == 0);
static_assert(VE_TYPE_STEAM == 1);
static_assert(VE_DISABLE_EFFECT == 6);
static_assert(VE_ADVANCED_EFFECT == 6);
static_assert(VE_DISABLE_WAGON_POWER == 7);
static_assert(VE_DEFAULT == 255);
static_assert(EC_STEAM == 0);
static_assert(EC_MONORAIL == 3);
static_assert(DIRDIFF_90RIGHT == 2);
static_assert(CBID_VEHICLE_VISUAL_EFFECT == 16);
static_assert(VSE_VISUAL_EFFECT == 6);

#include "settings_services.h"
#include "environment_services.h"
#include "type_availability_ffi.h"
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

/* The native API exposes its existing frame; all scope selection is Rust-owned. */
ScopeResolver *ResolverObject::GetScope(VarSpriteGroupScope scope, uint8_t relative) { return RustResolverBaseScope(this, scope, relative); }

OpenTTDIndustryBuilder *RustIndustryBuilderState() noexcept { return _industry_builder.owner.get(); }

extern "C" uint64_t openttd_orders_tick_counter() noexcept
{
	return TimerGameTick::counter;
}

static CommandCost *NativeCost(OpenTTDCommandCost *cost) noexcept { return reinterpret_cast<CommandCost *>(cost); }
static const CommandCost *NativeCost(const OpenTTDCommandCost *cost) noexcept { return reinterpret_cast<const CommandCost *>(cost); }
static_assert(sizeof(CommandCost) <= sizeof(OpenTTDCommandCostStorage));
static_assert(alignof(CommandCost) <= alignof(OpenTTDCommandCostStorage));
static OpenTTDCommandCost *CostConstruct(OpenTTDCommandCostStorage *p) noexcept { return reinterpret_cast<OpenTTDCommandCost *>(std::construct_at(reinterpret_cast<CommandCost *>(p))); }
static void CostDestroy(OpenTTDCommandCost *p) noexcept { std::destroy_at(reinterpret_cast<CommandCost *>(p)); }
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
static void CostEncodeOwnedBy(OpenTTDCommandCost *out, uint8_t owner, uint32_t tile) noexcept { RustEncodeOwnedBy(*NativeCost(out), Owner(owner), TileIndex(tile)); }
static void CostErrorOwner(OpenTTDCommandCost *out, uint8_t owner) noexcept { NativeCost(out)->SetErrorOwner(Owner(owner)); }
const OpenTTDCommandCostServices &GetRustCommandCostServices() noexcept
{
	static const OpenTTDCommandCostServices services{
		.construct = CostConstruct, .destroy = CostDestroy,
		.zero = CostZero, .error = CostError, .expense_money = CostExpenseMoney,
		.error_param = CostErrorParam, .add = CostAdd, .move = CostMove,
		.amount = CostAmount, .success = CostSuccess, .error_message = CostErrorMessage, .money = CostMoney,
		.expense = CostExpense, .make_error = CostMakeError,
		.encode_owned_by = CostEncodeOwnedBy, .error_owner = CostErrorOwner,
	};
	return services;
}

/* Command lifecycle leaves: settings, input and presentation only. None reenters a
 * command; allocation or error-formatting failure terminates under noexcept. */
static bool CommandInfiniteMoney() noexcept { return _settings_game.difficulty.infinite_money; }
static void CommandMoneyError(OpenTTDCommandCost *cost, int64_t amount) noexcept
{
	NativeCost(cost)->MakeError(STR_ERROR_NOT_ENOUGH_CASH_REQUIRES_CURRENCY);
	if (RustIsLocalCompany()) NativeCost(cost)->SetEncodedMessage(GetEncodedString(STR_ERROR_NOT_ENOUGH_CASH_REQUIRES_CURRENCY, Money(amount)));
}
static bool CommandShiftPressed() noexcept { return _shift_pressed; }
static uint8_t CommandPauseLevel() noexcept { return to_underlying(_settings_game.construction.command_pause_level); }
static void CommandLastBuild(uint8_t company, uint32_t tile) noexcept
{
	Company *c = Company::GetIfValid(CompanyID(company));
	if (c != nullptr) c->last_build_coordinate = TileIndex{tile};
}
static void CommandPausedError(uint32_t err_message, int32_t x, int32_t y) noexcept
{
	ShowErrorMessage(GetEncodedString(err_message), GetEncodedString(STR_ERROR_NOT_ALLOWED_WHILE_PAUSED), WL_INFO, x, y);
}
static void CommandErrorMessage(uint32_t err_message, int32_t x, int32_t y, const OpenTTDCommandCost *res) noexcept
{
	ShowErrorMessage(GetEncodedString(err_message), x, y, *const_cast<CommandCost *>(NativeCost(res)));
}
static void CommandEstimate(int64_t cost, int32_t x, int32_t y) noexcept { ShowEstimatedCostOrIncome(cost, x, y); }

const OpenTTDCommandServices &GetRustCommandServices() noexcept
{
	static const OpenTTDCommandServices services{
		.command_flags = [](uint8_t cmd) noexcept { return GetCommandFlags(static_cast<Commands>(cmd)).base(); },
		.command_type = [](uint8_t cmd) noexcept { return GetCommandType(static_cast<Commands>(cmd)); },
		.command_valid = [](uint8_t cmd) noexcept { return IsValidCommand(static_cast<Commands>(cmd)); },
		.finance = &GetRustCompanyFinanceServices(),
		.infinite_money = CommandInfiniteMoney, .money_error = CommandMoneyError,
		.infrastructure_dirty = RustInfrastructureDirty,
		.landscape_commands = []() noexcept { return &GetRustLandscapeCommandServices(); },
		.company = []() noexcept { return &GetRustCompanyServices(); },
		.occupancy = GetRustOccupancyCommands,
		.shared = &GetRustSharedServices(),
		.cost = &GetRustCommandCostServices(),
		.rail_map = &GetRustRailMapServices(),
		.signals = &GetRustSignalServices(),
		.shift_pressed = CommandShiftPressed,
		.pause_level = CommandPauseLevel,
		.last_build_coordinate = CommandLastBuild,
		.not_enough_cash = STR_ERROR_NOT_ENOUGH_CASH_REQUIRES_CURRENCY,
		.paused_error = CommandPausedError,
		.error_message = CommandErrorMessage,
		.estimate = CommandEstimate,
	};
	return services;
}

const OpenTTDEnvironmentServices &GetRustEnvironmentServices() noexcept
{
	static const OpenTTDEnvironmentServices services{.editor = OpenTTDEditor, .freeform_edges = OpenTTDFreeformEdges};
	return services;
}
const OpenTTDWaterRestorationServices &GetRustWaterRestorationServices() noexcept
{
	static const OpenTTDWaterRestorationServices services{
		.make_canal = RustMakeCanal, .make_river = RustMakeRiver,
	};
	return services;
}


static void DestroySnowLine(SnowLine *record) noexcept { delete record; }

static_assert(std::is_same_v<std::underlying_type_t<Colours>, uint8_t>);
static_assert(std::is_same_v<std::underlying_type_t<LandscapeType>, uint8_t>);
static_assert(std::is_same_v<std::underlying_type_t<EconomyType>, uint8_t>);
static_assert(std::is_same_v<std::underlying_type_t<ParticipateSurvey>, uint8_t>);
static_assert(LS_DEFAULT == 0);
static const OpenTTDSettingAddresses _rust_setting_addresses{
	.allow_town_level_crossings = &_settings_game.economy.allow_town_level_crossings,
	.allow_town_roads = &_settings_game.economy.allow_town_roads,
	.ambient = &_settings_client.sound.ambient,
	.climate = reinterpret_cast<const uint8_t *>(&_settings_game.game_creation.landscape),
	.competitor_interval = &_settings_game.difficulty.competitors_interval,
	.construction_limits = &_settings_game.construction.limits,
	.custom_industry_number = &_settings_game.game_creation.custom_industry_number,
	.disable_elrails = &_settings_game.vehicle.disable_elrails,
	.dist_local_authority = &_settings_game.economy.dist_local_authority,
	.economy_type = reinterpret_cast<const uint8_t *>(&_settings_game.economy.type),
	.extra_tree_placement = &_settings_game.construction.extra_tree_placement,
	.freeform_edges = &_settings_game.construction.freeform_edges,
	.generation_seed = &_settings_game.game_creation.generation_seed,
	.height_limit = &_settings_game.construction.map_height_limit,
	.heightmap_height = &_settings_game.game_creation.heightmap_height,
	.industry_cargo_scale = &_settings_game.economy.industry_cargo_scale,
	.industry_density = &_settings_game.difficulty.industry_density,
	.industry_platform = &_settings_game.construction.industry_platform,
	.land_generator = &_settings_game.game_creation.land_generator,
	.line_reverse_mode = &_settings_game.difficulty.line_reverse_mode,
	.lost_vehicle_warn = &_settings_client.gui.lost_vehicle_warn,
	.map_x = &_settings_game.game_creation.map_x,
	.map_y = &_settings_game.game_creation.map_y,
	.max_bridge_length = &_settings_game.construction.max_bridge_length,
	.max_companies = &_settings_client.network.max_companies,
	.max_competitors = &_settings_game.difficulty.max_no_competitors,
	.max_train_length = &_settings_game.vehicle.max_train_length,
	.max_trains = &_settings_game.vehicle.max_trains,
	.maximum_go_to_depot_penalty = &_settings_game.pf.yapf.maximum_go_to_depot_penalty,
	.multiple_industry_per_town = &_settings_game.economy.multiple_industry_per_town,
	.never_expire_airports = &_settings_game.station.never_expire_airports,
	.never_expire_vehicles = &_settings_game.vehicle.never_expire_vehicles,
	.oil_refinery_limit = &_settings_game.game_creation.oil_refinery_limit,
	.order_review_system = &_settings_client.gui.order_review_system,
	.participate_survey = reinterpret_cast<const uint8_t *>(&_settings_client.network.participate_survey),
	.pause_on_newgame = &_settings_client.gui.pause_on_newgame,
	.raw_industry_construction = &_settings_game.construction.raw_industry_construction,
	.recalc_interval = &_settings_game.linkgraph.recalc_interval,
	.reload_cfg = &_settings_client.network.reload_cfg,
	.reverse_at_signals = &_settings_game.pf.reverse_at_signals,
	.roadveh_acceleration_model = &_settings_game.vehicle.roadveh_acceleration_model,
	.se_flat_world_height = &_settings_game.game_creation.se_flat_world_height,
	.secondary_colour = reinterpret_cast<const uint8_t *>(&_settings_client.gui.starting_colour_secondary),
	.select_goods = &_settings_game.order.selectgoods,
	.starting_colour = reinterpret_cast<const uint8_t *>(&_settings_client.gui.starting_colour),
	.station_noise = &_settings_game.economy.station_noise_level,
	.town_council_tolerance = &_settings_game.difficulty.town_council_tolerance,
	.town_growth_rate = &_settings_game.economy.town_growth_rate,
	.train_acceleration_model = &_settings_game.vehicle.train_acceleration_model,
	.tree_placer = &_settings_game.game_creation.tree_placer,
	.used_liveries = &_loaded_newgrf_features.used_liveries,
	.wait_for_pbs_path = &_settings_game.pf.wait_for_pbs_path,
	.wait_oneway_signal = &_settings_game.pf.wait_oneway_signal,
	.wait_twoway_signal = &_settings_game.pf.wait_twoway_signal,
};
constinit const OpenTTDMapStatics _rust_map_statics = OpenTTDNativeTileTypeAccess::Statics();

extern "C" uint64_t openttd_orders_ship_station_tile(uint16_t destination) noexcept
{
	return RustStationShipTile(RustStationGet<Station *>(destination));
}

static uint32_t SharedRandom(void *) noexcept { return Random(); }
const OpenTTDStationService *OpenTTDStationActivityOwner(const OpenTTDStation *station) noexcept
{
	return RustStationServiceOwner(StationOf(station));
}

Town *OpenTTDTownShellByID(uint32_t id) noexcept
{
	return Town::Get(TownID(id));
}

void OpenTTDAssertTownGenerationContext() noexcept { assert(_game_mode == GM_EDITOR || _generating_world); }

bool OpenTTDIsTileFlat(uint32_t tile) noexcept { return RustIsTileFlat(tile); }
ClearGround OpenTTDClearGround(uint32_t tile) noexcept { return RustClearGround(Tile{TileIndex{tile}}); }
TreeGround OpenTTDTreeGround(uint32_t tile) noexcept { return RustTreeGround(Tile{TileIndex{tile}}); }

static uint8_t SharedTropicZone(uint32_t tile) noexcept { return GetTropicZone(TileIndex{tile}); }
static int32_t SharedTileZ(uint32_t tile) noexcept { return RustTileZ(TileIndex{tile}); }
static uint8_t SharedTileSlope(uint32_t tile) noexcept { return GetTileSlope(TileIndex{tile}); }
static void SharedFreeTrainReservation(const Train *v) noexcept { FreeTrainTrackReservation(v); }
static bool SharedTryPathReserve(Train *v, bool mark_as_stuck, bool first_tile_okay) noexcept { return TryPathReserve(v, mark_as_stuck, first_tile_okay); }
static bool Shared_may_have_road(uint32_t tile) noexcept { return MayHaveRoad(TileIndex{tile}); }
static void Shared_set_road_bits(uint32_t tile, uint8_t bits, uint8_t rtt) noexcept { SetRoadBits(TileIndex{tile}, static_cast<RoadBits>(bits), static_cast<RoadTramType>(rtt)); }
static void Shared_set_road_type(uint32_t tile, uint8_t rtt, uint8_t rt) noexcept { RustSetRoadType(TileIndex{tile}, static_cast<RoadTramType>(rtt), static_cast<RoadType>(rt)); }
static void Shared_set_road_directions(uint32_t tile, uint8_t drd) noexcept { SetDisallowedRoadDirections(TileIndex{tile}, static_cast<DisallowedRoadDirections>(drd)); }
static bool Shared_road_powered(uint8_t from, uint8_t to) noexcept { return HasPowerOnRoad(static_cast<RoadType>(from), static_cast<RoadType>(to)); }
static void SharedSetTropicZone(uint32_t tile, uint8_t zone) noexcept { SetTropicZone(TileIndex{tile}, static_cast<TropicZone>(zone)); }

/** Pure map query; no allocation, reentry or C++ exception crosses into Rust. */
static uint32_t SharedIndustry(int32_t x, int32_t y, uint32_t *index) noexcept
{
	TileIndex tile{RustTileVirtXY(x, y)};
	*index = tile.base();
	if (!IsTileType(tile, MP_INDUSTRY)) return 0;
	return GetIndustryGfx(tile) == GFX_BUBBLE_CATCHER ? 2 : 1;
}

static uint16_t MapSvcDepotIndex(uint32_t tile) noexcept { return GetDepotIndex(TileIndex(tile)).base(); }
static uint8_t MapSvcTunnelBridgeDirection(uint32_t tile) noexcept { return GetTunnelBridgeDirection(TileIndex(tile)); }
static uint16_t MapSvcBridgeSpeed(uint32_t tile) noexcept { return RustBridgeSpecSpeed(RustBridgeType(tile)); }
static bool MapSvcIsWaterTile(uint32_t tile) noexcept { return IsWaterTile(TileIndex(tile)); }

static uint8_t SharedNextCompany(uint8_t from) noexcept
{
	/* The canonical iterator's UINT32_MAX sentinel narrows to this ABI's UINT8_MAX. */
	return static_cast<uint8_t>(RustNextCompanyID(from));
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
static int32_t Shared_tile_max_z(uint32_t tile) noexcept { return OpenTTDTileMaxZ(TileIndex{tile}); }
static_assert(TILE_HEIGHT == 8 && TILE_SIZE == 16);
/* Values of the shared Rust header modules (tile_type.rs, slope_func.rs and the like). */
static_assert(PR_CLEAR_HOUSE == 40);
static_assert(MP_CLEAR == 0 && MP_RAILWAY == 1 && MP_ROAD == 2 && MP_HOUSE == 3 && MP_TREES == 4 && MP_STATION == 5 && MP_WATER == 6 && MP_VOID == 7 && MP_INDUSTRY == 8 && MP_TUNNELBRIDGE == 9 && MP_OBJECT == 10);
static_assert(INVALID_TILE.base() == UINT32_MAX && TILE_HEIGHT_STEP == 50 && CALLBACK_FAILED == 0xFFFF && INVALID_STRING_ID == 0xFFFF && SPECSTR_TOWNNAME_START == 0x20C0 && BUILTIN_TOWNNAME_GENERATOR_COUNT == 21);
static_assert(SLOPE_FLAT == 0 && SLOPE_W == 1 && SLOPE_S == 2 && SLOPE_E == 4 && SLOPE_N == 8 && SLOPE_STEEP == 0x10 && SLOPE_HALFTILE == 0x20 && SLOPE_HALFTILE_MASK == 0xE0);
static_assert(DIR_END == 8 && AXIS_Y == 1 && DIAGDIR_BEGIN == 0 && DIAGDIR_END == 4 && DIAGDIR_NE == 0 && DIAGDIR_SE == 1 && DIAGDIR_SW == 2 && DIAGDIR_NW == 3 && INVALID_DIAGDIR == 0xFF);
static_assert(DIR_N == 0 && DIR_NE == 1 && DIR_E == 2 && DIR_SE == 3 && DIR_S == 4 && DIR_SW == 5 && DIR_W == 6 && DIR_NW == 7);
static_assert(DIAGDIRDIFF_SAME == 0 && DIAGDIRDIFF_90RIGHT == 1 && DIAGDIRDIFF_REVERSE == 2 && DIAGDIRDIFF_90LEFT == 3 && DIAGDIRDIFF_END == 4);
static_assert(OWNER_TOWN.base() == 0x0F && OWNER_NONE.base() == 0x10 && OWNER_WATER.base() == 0x11 && OWNER_DEITY.base() == 0x12 && INVALID_OWNER.base() == 0xFF);
static_assert(to_underlying(DoCommandFlag::Execute) == 0 && to_underlying(DoCommandFlag::Auto) == 1 && to_underlying(DoCommandFlag::QueryCost) == 2 && to_underlying(DoCommandFlag::NoWater) == 3);
static_assert(to_underlying(DoCommandFlag::NoTestTownRating) == 4 && to_underlying(DoCommandFlag::Bankrupt) == 5 && to_underlying(DoCommandFlag::NoModifyTownRating) == 9 && to_underlying(DoCommandFlag::ForceClearTile) == 10);
static_assert(std::is_same_v<DoCommandFlags::BaseType, uint16_t>);
static_assert(INVALID_RAILTYPES.base() == UINT64_MAX && INVALID_RAILTYPE == 255);
static_assert(GFX_TRUCK_BUS_DRIVETHROUGH_OFFSET == 4);
static_assert(NEW_STATION.base() == 0xFFFD && StationID::Invalid().base() == 0xFFFF);
static_assert(to_underlying(StationType::RailWaypoint) == 7 && to_underlying(StationType::RoadWaypoint) == 8 && to_underlying(StationType::Buoy) == 6);
static_assert(StationFacilities(StationFacility::Train).base() == 1 && StationFacilities(StationFacility::TruckStop).base() == 2 && StationFacilities(StationFacility::BusStop).base() == 4 && StationFacilities(StationFacility::Dock).base() == 16);
static_assert(MAX_LENGTH_STATION_NAME_CHARS == 32 && StationRect::ADD_TEST == 0 && StationRect::ADD_TRY == 1 && StationRect::ADD_FORCE == 2);
static_assert(CA_NONE == 0 && CA_BUS == 3 && CA_TRUCK == 3 && CA_TRAIN == 4 && CA_DOCK == 5 && CA_UNMODIFIED == 4);
static_assert(PR_BUILD_STATION_TRUCK == 13 && PR_BUILD_WAYPOINT_RAIL == 52 && PR_BUILD_WAYPOINT_BUOY == 54 && PR_CLEAR_WAYPOINT_BUOY == 55 && ROAD_STOP_TRACKBIT_FACTOR == 2);
static_assert(to_underlying(RailTileType::Normal) == 0 && TRACK_BIT_X == 1 && TRACK_BIT_Y == 2 && AXIS_END == 2);
static_assert(ROAD_X == 10 && ROAD_Y == 5);
static_assert(AXIS_X == 0 && AXIS_Y == 1 && INVALID_AXIS == 0xFF);
static_assert(PR_TERRAFORM == 21 && PR_CLEAR_CANAL == 61 && PR_CLEAR_TREES == 26);
static_assert(to_underlying(DoCommandFlag::AutoReplace) == 6 && to_underlying(DoCommandFlag::NoCargoCapacityCheck) == 7);
static_assert(to_underlying(DoCommandFlag::AllTiles) == 8 && CMD_COMPANY_CTRL == 90 && CMD_END == 145 && std::is_same_v<CommandFlags::BaseType, uint16_t>);
static_assert(to_underlying(CommandFlag::Server) == 0 && to_underlying(CommandFlag::Spectator) == 1 && to_underlying(CommandFlag::NoTest) == 5 && to_underlying(CommandFlag::Deity) == 8 && to_underlying(CommandFlag::NoEst) == 10);
static_assert(to_underlying(CommandType::ServerSetting) == 7 && to_underlying(CommandType::End) == 9);
static_assert(to_underlying(CommandPauseLevel::NoActions) == 0 && to_underlying(CommandPauseLevel::NoConstruction) == 1 && to_underlying(CommandPauseLevel::NoLandscaping) == 2 && to_underlying(CommandPauseLevel::AllActions) == 3);
static_assert(AIR_HELICOPTER == 0 && AIR_AIRCRAFT == 2 && AIR_CTOL == 1 && HANGAR == 1);
static_assert(OT_NOTHING == 0 && OT_GOTO_STATION == 1 && OT_GOTO_DEPOT == 2 && OT_LOADING == 3 && OT_LEAVESTATION == 4 && OT_DUMMY == 5 && OT_GOTO_WAYPOINT == 6 && OT_CONDITIONAL == 7 && OT_IMPLICIT == 8);
static_assert(INVALID_VEH_ORDER_ID == 0xFF && to_underlying(OrderLoadType::NoLoad) == 4 && to_underlying(OrderUnloadType::Unload) == 1 && to_underlying(OrderUnloadType::Transfer) == 2 && to_underlying(OrderUnloadType::NoUnload) == 4);
static_assert(to_underlying(StationType::Truck) == 2 && to_underlying(StationType::Bus) == 3);
static_assert(to_underlying(StationType::Rail) == 0 && to_underlying(StationType::Airport) == 1 && to_underlying(StationType::Oilrig) == 4 && to_underlying(StationType::Dock) == 5);
static_assert(CARGO_NO_REFIT == 0xFE && static_cast<uint8_t>(OrderDepotTypeFlag::Service) == 0 && static_cast<uint8_t>(OrderNonStopFlag::NoIntermediate) == 0);
static_assert(CARGO_AUTO_REFIT == 0xFD && StationID::Invalid().base() == 0xFFFF);
static_assert(LinkGraph::MIN_TIMEOUT_DISTANCE == 32 && LinkGraph::STALE_LINK_DEPOT_TIMEOUT.base() == 1024 && LinkGraph::COMPRESSION_INTERVAL.base() == 256);
static_assert(EconomyTime::INVALID_DATE.base() == -1 && INVALID_NODE == UINT16_MAX && LinkGraphID::Invalid().base() == UINT16_MAX);
static_assert(EdgeUpdateModes{EdgeUpdateMode::Increase}.base() == 1 && EdgeUpdateModes{EdgeUpdateMode::Refresh}.base() == 2 && EdgeUpdateModes{EdgeUpdateMode::Restricted}.base() == 4 && EdgeUpdateModes{EdgeUpdateMode::Unrestricted}.base() == 8);
static_assert(VL_STANDARD == 0 && VL_SHARED_ORDERS == 1 && VL_STATION_LIST == 2 && VL_DEPOT_LIST == 3 && VL_GROUP_LIST == 4);
static_assert(GVSF_FRONT == 0 && GVSF_ARTICULATED_PART == 1 && GVSF_WAGON == 2 && GVSF_ENGINE == 3 && GVSF_FREE_WAGON == 4 && GVSF_MULTIHEADED == 5);
static_assert(CCF_ARRANGE.base() == 3);
static_assert(PR_BUILD_SIGNALS == 3 && PR_BUILD_DEPOT_TRAIN == 5 && PR_CLEAR_ROUGH == 23 && PR_CLEAR_SIGNALS == 28 && PR_CLEAR_DEPOT_TRAIN == 30 && PR_CLEAR_WATER == 34 && PR_TOWN_ACTION == 56 && PR_BUILD_FOUNDATION == 57);
static_assert(to_underlying(SourceType::Town) == 1 && to_underlying(SourceType::Headquarters) == 2);
static_assert(AT_OILRIG == 9 && OBJECT_STATUE == 2 && EXPENSES_CONSTRUCTION == 0 && EXPENSES_OTHER == 12 && COMPANY_SPECTATOR.base() == 0xFF && MP_INDUSTRY == 8 && MP_OBJECT == 10);
static_assert(RATING_MINIMUM == -1000 && RATING_MAXIMUM == 1000 && RATING_BRIBE_UP_STEP == 200 && RATING_BRIBE_MAXIMUM == 800 && RATING_BRIBE_DOWN_TO == -50);
static uint8_t Shared_bridge_axis(uint32_t tile) noexcept { return GetBridgeAxis(TileIndex{tile}); }
static bool Shared_has_water_ground(uint32_t tile) noexcept { return HasTileWaterGround(TileIndex{tile}); }
static bool Shared_is_bridge(uint32_t tile) noexcept { return IsBridgeTile(TileIndex{tile}); }
static uint8_t Shared_drive_through_axis(uint32_t tile) noexcept { return GetDriveThroughStopAxis(TileIndex{tile}); }
static uint8_t Shared_road_type(uint32_t tile) noexcept { return GetRoadTypeRoad(TileIndex{tile}); }
static bool Shared_has_road_type(uint32_t tile) noexcept { return RustHasTileRoadType(tile, RTT_ROAD); }
static uint16_t Shared_town_index(uint32_t tile) noexcept { return GetTownIndex(TileIndex{tile}).base(); }
static uint8_t Shared_road_bits(uint32_t tile, uint8_t type, bool straight) noexcept { return GetAnyRoadBits(TileIndex{tile}, static_cast<RoadTramType>(type), straight); }
static void SharedUpdateLevelCrossing(uint32_t tile, bool sound) noexcept { UpdateLevelCrossing(TileIndex{tile}, sound); }
static void SharedMarkBridgeDirty(uint32_t tile) noexcept { MarkBridgeDirty(TileIndex{tile}); }
static Trackdir SharedTrainTrackdir(const Train *v) noexcept { return v->GetVehicleTrackdir(); }
static RoadType SharedTramType(uint32_t tile) noexcept { return GetRoadTypeTram(TileIndex{tile}); }
static RoadBits SharedCrossingRoadBits(uint32_t tile) noexcept { return GetCrossingRoadBits(TileIndex{tile}); }
static uint32_t SharedMapMaxX() noexcept { return Map::MaxX(); }
static uint32_t SharedMapMaxY() noexcept { return Map::MaxY(); }
static uint8_t SharedPlaneSpeed() noexcept { return RustPlaneSpeed(); }
static bool SharedNoJetcrash() noexcept { return _cheats.no_jetcrash.value; }
static uint8_t SharedPlaneCrashes() noexcept { return RustPlaneCrashes(); }
static bool SharedServiceAtHelipad() noexcept { return RustServiceAtHelipad(); }
static bool SharedDisasterSound() noexcept { return RustDisasterSound(); }

static size_t SharedTownCount() noexcept { return Town::GetNumItems(); }
static bool SharedRecession() noexcept { return EconomyIsInRecession(); }
static void SharedForEachStation(void *context, void (*visit)(void *, const OpenTTDStation *, OpenTTDStationService *, uint16_t)) noexcept
{
	for (Station *st : Station::Iterate()) {
		visit(context, RustStation(st), RustStationServiceOwner(st), RustBaseStationID(st));
	}
}

/** Stateless STL permutation primitive; storage and tree control flow belong to Rust. */
static size_t RustKdtreePartition(OpenTTDKdtreeKey *keys, size_t count) noexcept
{
	auto *end = keys + count;
	auto *mid = keys + count / 2;
	std::nth_element(keys, mid, end, [](const OpenTTDKdtreeKey &a, const OpenTTDKdtreeKey &b) { return a.coordinate < b.coordinate; });
	auto split = mid->coordinate;
	return std::partition(keys, end, [=](const OpenTTDKdtreeKey &k) { return k.coordinate < split; }) - keys;
}

constinit const OpenTTDSharedServices _rust_shared_services{
	.all_compatible_railtypes = RustAllCompatibleRailTypes,
	.command_post = GetRustCommandPostServices,
	.movement = &_rust_movement_services,
	.set_tile_owner = RustSetTileOwner,
	.autoslope = RustAutoslopeEnabled,
	.bay_stop_direction = Shared_bay_stop_direction,
	.bridge_axis = Shared_bridge_axis,
	.bridge_speed = MapSvcBridgeSpeed,
	.build_on_slopes = RustBuildOnSlopes,
	.clear_square_services = {

		.docking = OpenTTDIsDockingTile,
		.generating_world = OpenTTDGeneratingWorld,
	},
	.company_human = OpenTTDCompanyIsValidHuman,
	.company_infrastructure = RustCompanyInfrastructure,
	.company_valid = OpenTTDCompanyIsValid,
	.context = nullptr,
	.cost_animation = RustCostOrIncomeAnimation,
	.crossing_road_bits = SharedCrossingRoadBits,
	.current_company = RustCurrentCompanyID,
	.depot_index = MapSvcDepotIndex,
	.disaster_sound = SharedDisasterSound,
	.drive_through_axis = Shared_drive_through_axis,
	.forbid_90_deg = RustForbid90Deg,
	.free_train_reservation = SharedFreeTrainReservation,
	.has_road_type = Shared_has_road_type,
	.has_water_ground = Shared_has_water_ground,
	.industry = SharedIndustry,
	.is_bay_stop = Shared_is_bay_stop,
	.is_bridge = Shared_is_bridge,
	.is_drive_through_stop = Shared_is_drive_through_stop,
	.is_plain_rail = RustPlainRailTile,
	.is_road_depot = Shared_is_road_depot,
	.is_water_tile = MapSvcIsWaterTile,
	.map = &_rust_map_statics,
	.settings = &_rust_setting_addresses,
	.map_max_x = SharedMapMaxX,
	.map_max_y = SharedMapMaxY,
	.mark_bridge_dirty = SharedMarkBridgeDirty,
	.mark_dirty = RustMarkTileDirty,
	.next_company = SharedNextCompany,
	.no_jetcrash = SharedNoJetcrash,
	.other_tunnel_end = RustOtherTunnelBridgeEnd,
	.plane_crashes = SharedPlaneCrashes,
	.plane_speed = SharedPlaneSpeed,
	.powered_roadtypes = RustRoadTypePoweredRoadTypes,
	.random = SharedRandom,
	.road_bits = Shared_road_bits,
	.road_cost_multiplier = RustRoadCostMultiplier,
	.road_depot_direction = Shared_road_depot_direction,
	.road_type = Shared_road_type,
	.crossing_reserved = RustCrossingReserved,
	.set_crossing_reserved = RustSetCrossingReserved,
	.may_have_road = Shared_may_have_road,
	.set_road_bits = Shared_set_road_bits,
	.set_road_type = Shared_set_road_type,
	.set_road_owner = RustSetRoadOwner,
	.set_road_directions = Shared_set_road_directions,
	.road_powered = Shared_road_powered,
	.type_availability = []() noexcept { return &GetRustTypeAvailabilityServices(); },
	.pbs_services = [](const OpenTTDSharedServices *) noexcept { return &GetPbsServices(); },
	.service_at_helipad = SharedServiceAtHelipad,
	.set_tropic_zone = SharedSetTropicZone,

	.snow_line_support = { .height = &_settings_game.game_creation.snow_line_height, .destroy = DestroySnowLine },
	.station_index = MapSvcStationIndex,
	.station_had_vehicle_of_type = [](uint16_t station) noexcept -> uint8_t { return Station::Get(StationID(station))->had_vehicle_of_type; },
	.station_add_vehicle_of_type = [](uint16_t station, uint8_t type) noexcept { Station::Get(StationID(station))->had_vehicle_of_type |= static_cast<StationHadVehicleOfType>(type); },
	.station_xy = [](uint16_t station) noexcept { return Station::Get(StationID(station))->xy.base(); },
	.base_station = [](uint16_t station) noexcept { return BaseStation::Get(StationID(station)); },
	.base_station_xy = [](uint16_t station) noexcept { return BaseStation::Get(StationID(station))->xy.base(); },
	.station_facilities = [](uint16_t station) noexcept { return Station::Get(StationID(station))->facilities.base(); },
	.road_commands = []() noexcept { return &GetRustRoadCommandServices(); },
	.kdtree_partition = RustKdtreePartition,
	.station_commands = []() noexcept { return &GetRustStationCommandServices(); },
	.station_catchment = []() noexcept { return &GetRustStationCatchmentServices(); },
	.docking_tile = OpenTTDIsDockingTile,
	.station_gfx = [](uint32_t tile) noexcept { return GetStationGfx(TileIndex{tile}); },
	.station_serve_neutral = RustServeNeutralIndustries,
	.station_type = [](uint32_t tile) noexcept { return GetStationType(TileIndex{tile}); },
	.storage_discard = RustStorageDiscard,

	.tile_animation_set_frame = OpenTTDSetAnimationFrame,
	.town_next_id = OpenTTDNextTownID,
	.town_by_id = [](uint32_t id) noexcept { return reinterpret_cast<OpenTTDTownHandle *>(OpenTTDTownShellByID(id)); },
	.town_owner_by_id = OpenTTDTownOwnerByID,
	.utf8_length = RustUtf8Length,
	.vehicle_next = RustNextVehicle,
	.water_middle_lock = OpenTTDIsMiddleLock,
	.tile_max_z = Shared_tile_max_z,
	.tile_owner = RustTileOwnerValue,
	.tile_slope = SharedTileSlope,
	.tile_type = SharedTileType,
	.tile_valid = Shared_tile_valid,
	.tile_z = SharedTileZ,
	.town_index = Shared_town_index,
	.town_owner = SharedTownOwner,
	.train_trackdir = SharedTrainTrackdir,
	.tram_type = SharedTramType,
	.tropic_zone = SharedTropicZone,
	.try_path_reserve = SharedTryPathReserve,
	.tunnel_bridge_direction = MapSvcTunnelBridgeDirection,
	.tunnel_transport = OpenTTDMapQuery<uint8_t, RustTunnelBridgeTransport, uint32_t>,
	.update_level_crossing = SharedUpdateLevelCrossing,
	.recession = SharedRecession,
	.tick_counter = openttd_orders_tick_counter,
	.town_count = SharedTownCount,

	.apply_foundation = RustApplyFoundation,
	.clear_ground = OpenTTDClearGround,

	.has_water_class = RustHasWaterClass,
	.make_shore = OpenTTDMakeShore,
	.make_snow = RustMakeSnow,
	.play_sound = RustPlayTileSound,
	.set_fence = RustSetFieldFence,
	.tree_ground = OpenTTDTreeGround,
	.water_services = []() noexcept { return &GetRustWaterGroundServices(); },
	.for_each_station = SharedForEachStation,
	.road_stops = []() noexcept { return &GetRustRoadStopServices(); },
	.nearby_tile = RustSignedNearbyTile,
	.nearby_tile_information = RustNearbyTileInformation,
	.network_client_info = [](uint32_t id) noexcept { return NetworkClientInfo::GetByClientID(ClientID(id)); },
	.screen_dirty = RustScreenDirty,
	.string_data = [](const void *ptr, size_t *length) noexcept {
		const std::string &str = *static_cast<const std::string *>(ptr);
		*length = str.size();
		return reinterpret_cast<const uint8_t *>(str.data());
	},
	.terrain_type = RustTerrainType,
	.copyable_badges = RustCopyableBadges,
	.grf_file_version = RustGRFFileVersion,
	.grf_price_shift = RustGRFPriceShift,
	.grf_cargo_map = RustGRFCargoMap,
	.grf_boolean_callback = OpenTTDConvertBooleanCallback,
	.grf_file_at_or_null = RustGRFFileAtOrNull,
	.grf_file_id = RustGRFFileID,
	.grf_file_name = RustGRFFileName,
	.grf_parameter = RustGRFParameter,
	.newgrf_specs = &_rust_newgrf_specs,
	.resolver_grf_file = RustResolverGRFFile,
	.resolver_badges = [](const ResolverObject *resolver, const BadgeID *badges, size_t count, uint32_t parameter) noexcept { return GetBadgeVariableResult(*RustResolverGRFFile(resolver), std::span{badges, count}, parameter); },
	.resolver_grf_file_id = RustResolverGRFFileID,
	.resolver_grf_version = RustResolverGRFVersion,
	.show_error = RustShowErrorMessage,
	.not_reached = RustNotReached,
};

/* Game entries also bind the canonical fatal slot; tool-only links keep assertion binding. */
[[maybe_unused]] static const bool _rust_invariants_bound = (openttd_rust_assert_bind(RustAssertCondition, &_rust_shared_services), true);

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

const OpenTTDRailMapServices &GetRustRailMapServices() noexcept
{
	static const OpenTTDRailMapServices services{
		.ground = &GetRustRailGroundServices(),
		.set_signal_state = RustRailSetSignalState,
		.other_tunnel_bridge_end = RustOtherTunnelBridgeEnd,

		.invalid_rail_type = RustInvalidRailType,
		.set_rail_type = RustSetRailType,
		.set_track_bits = RustSetTrackBits,
		.set_present_signals = RustSetPresentSignals,
		.set_signal_states = RustSetSignalStates,
		.set_has_signals = RustSetHasSignals,
		.set_signal_type = RustSetSignalType,
		.signal_variant = RustSignalVariant,
		.set_signal_variant = RustSetSignalVariant,
		.cycle_signal_side = RustCycleSignalSide,

		.rail_reservation = RustRailReservation,
		.set_track_reservation = RustSetTrackReservation,
		.depot_reserved = RustDepotReserved,
		.set_depot_exit = RustSetDepotExit,
		.make_rail_normal = RustMakeRailNormal,
		.make_rail_depot = RustMakeRailDepot,
		.wormhole_reserved = RustWormholeReserved,
	};
	return services;
}

#include "service_table_check.hpp"
#include "abi-visit-OpenTTDRandomServices.hpp"
#include "abi-visit-OpenTTDRailMapServices.hpp"
#include "abi-visit-OpenTTDCommandServices.hpp"
#include "abi-visit-OpenTTDCommandCostServices.hpp"
#include "abi-visit-OpenTTDSharedServices.hpp"
#include "abi-visit-OpenTTDSnowLineSupport.hpp"
#include "abi-visit-OpenTTDPbsServices.hpp"
#include "abi-visit-OpenTTDLiveGraphServices.hpp"
#include "abi-visit-OpenTTDEnvironmentServices.hpp"
#include "abi-visit-OpenTTDClearSquareServices.hpp"
#include "abi-visit-OpenTTDWaterRestorationServices.hpp"
#include "abi-visit-OpenTTDByteReaderServices.hpp"
#include "abi-visit-OpenTTDAirportSpecServices.hpp"
#include "abi-visit-OpenTTDAirportTileSpecServices.hpp"
#include "abi-visit-OpenTTDHouseSpecServices.hpp"
#include "abi-visit-OpenTTDObjectSpecServices.hpp"
#include "abi-visit-OpenTTDRoadStopSpecServices.hpp"
#include "abi-visit-OpenTTDStationSpecServices.hpp"
#include "abi-visit-OpenTTDAirportTileSpecAnimation.hpp"
#include "abi-visit-OpenTTDRoadStopSpecAnimation.hpp"
#include "abi-visit-OpenTTDStationSpecAnimation.hpp"
#include "abi-visit-OpenTTDAirportSpecStandard.hpp"
#include "abi-visit-OpenTTDAirportTileSpecStandard.hpp"
#include "abi-visit-OpenTTDHouseSpecStandard.hpp"
#include "abi-visit-OpenTTDObjectSpecStandard.hpp"
#include "abi-visit-OpenTTDAirportSpecSubstitute.hpp"
#include "abi-visit-OpenTTDAirportTileSpecSubstitute.hpp"
#include "abi-visit-OpenTTDHouseSpecSubstitute.hpp"
/* Each capability table Rust reaches is present and complete. */
static bool NewGRFSpecsComplete()
{
	const OpenTTDNewGRFSpecs &s = _rust_newgrf_specs;
	auto complete = [](const auto *table) { return table != nullptr && AbiServicesComplete(*table); };
	return complete(s.airport) && complete(s.airport->standard) && complete(s.airport->substitute) &&
		complete(s.airport_tile) && complete(s.airport_tile->animation) && complete(s.airport_tile->standard) && complete(s.airport_tile->substitute) &&
		complete(s.house) && complete(s.house->standard) && complete(s.house->substitute) &&
		complete(s.object) && complete(s.object->standard) &&
		complete(s.road_stop) && complete(s.road_stop->animation) &&
		complete(s.station) && complete(s.station->animation);
}
bool CheckAbi_services()
{
	return AbiServicesComplete(GetPbsServices()) && AbiServicesComplete(GetRustLiveGraphServices()) &&
		AbiServicesComplete(GetRustEnvironmentServices()) &&
		AbiServicesComplete(GetRustRandomServices()) &&
		AbiServicesComplete(GetRustByteReaderServices()) &&
		AbiServicesComplete(GetRustRailMapServices()) &&
		AbiServicesComplete(GetRustCommandServices()) &&
		AbiServicesComplete(GetRustCommandCostServices()) &&
		AbiServicesComplete(GetRustSharedServices()) &&
		AbiServicesComplete(GetRustSharedServices().clear_square_services) &&
		AbiServicesComplete(GetRustSharedServices().snow_line_support) &&
		AbiServicesComplete(GetRustWaterRestorationServices()) &&
		GetRustSharedServices().newgrf_specs == &_rust_newgrf_specs && NewGRFSpecsComplete();
}

static OpenTTDLiveStationInput LiveGraphStation(uint16_t station, uint8_t cargo) noexcept
{
	Station *st = Station::Get(StationID(station));
	return {RustStationLocation(st), st->goods[cargo].status.Test(GoodsEntry::State::Acceptance)};
}
static void LiveGraphAttach(uint16_t station, uint8_t cargo, uint16_t graph, uint16_t node) noexcept
{
	GoodsEntry &ge = Station::Get(StationID(station))->goods[cargo];
	ge.link_graph = LinkGraphID(graph);
	ge.node = node;
}
static void LiveGraphSetNode(uint16_t station, uint8_t cargo, uint16_t node) noexcept { Station::Get(StationID(station))->goods[cargo].node = node; }
static void LiveGraphDelete(OpenTTDLiveGraph *graph) noexcept { delete graph->shell; }

static uint16_t LiveGraphStationNode(OpenTTDBaseStation *station, uint8_t cargo) noexcept { return static_cast<Station *>(station)->goods[cargo].node; }
const OpenTTDLiveGraphServices &GetRustLiveGraphServices() noexcept
{
	static const OpenTTDLiveGraphServices services{
#if !defined(NDEBUG) || defined(WITH_ASSERT)
		.assertions = true,
#else
		.assertions = false,
#endif
		.pool = &_link_graph_pool.descriptor,
		.station_id = RustBaseStationID<const BaseStation>,
		.index = RustLinkGraphIndex,
		.station = LiveGraphStation,
		.attach = LiveGraphAttach,
		.set_node = LiveGraphSetNode,
		.delete_graph = LiveGraphDelete,
		.graph_id = RustStationLinkGraphIndex,
		.node = LiveGraphStationNode,
		.available = RustLinkGraphAvailable,
		.allocate = RustLinkGraphAllocate,
	};
	return services;
}

static_assert(EV_CHIMNEY_SMOKE == 0);
static_assert(EV_STEAM_SMOKE == 1);
static_assert(EV_DIESEL_SMOKE == 2);
static_assert(EV_ELECTRIC_SPARK == 3);
static_assert(EV_CRASH_SMOKE == 4);
static_assert(EV_EXPLOSION_LARGE == 5);
static_assert(EV_BREAKDOWN_SMOKE == 6);
static_assert(EV_EXPLOSION_SMALL == 7);
static_assert(EV_BULLDOZER == 8);
static_assert(EV_BUBBLE == 9);
static_assert(EV_BREAKDOWN_SMOKE_AIRCRAFT == 10);
static_assert(EV_COPPER_MINE_SMOKE == 11);
static_assert(EV_END == 12);
static_assert(VESM_NONE == 0);
static_assert(VESM_STEAM == 1);
static_assert(VESM_DIESEL == 2);
static_assert(VESM_ELECTRIC == 3);
static_assert(VESM_END == 4);
static_assert(VE_OFFSET_COUNT == 4 && VE_TYPE_COUNT == 2);
static_assert(CalendarTime::MAX_DATE.base() == 1826212865 && CalendarTime::DAYS_IN_YEAR == 365);
static_assert(RAILTYPE_END == 64 && ROADTYPE_END == 63);
static_assert(NUM_AIRPORTS_PER_GRF == 128 && NUM_AIRPORTTILES_PER_GRF == 255);
static_assert(GSF_AIRPORTS == 0x0D && GSF_TRAMTYPES == 0x13);
static_assert(MAX_CATCHMENT == 10 && CalendarTime::MAX_YEAR.base() == 5000000);

static_assert(PR_BUILD_INDUSTRY == 48 && PR_CLEAR_INDUSTRY == 49 && PR_BUILD_INDUSTRY_RAW == 58);

static const OpenTTDDebugServices _rust_debug_services{
	.console = &_debug_console_level,
	.desync = &_debug_desync_level,
	.driver = &_debug_driver_level,
	.fontcache = &_debug_fontcache_level,
	.gamelog = &_debug_gamelog_level,
	.grf = &_debug_grf_level,
	.grf_msg = [](int severity, const char *message, size_t size) noexcept { GrfMsgI(severity, std::string(message, size)); },
	.map = &_debug_map_level,
	.misc = &_debug_misc_level,
	.net = &_debug_net_level,
	.oldloader = &_debug_oldloader_level,
	.print = [](const char *category, size_t category_size, int level, const char *message, size_t size) noexcept { DebugPrint({category, category_size}, level, std::string(message, size)); },
	.script = &_debug_script_level,
	.sl = &_debug_sl_level,
	.sprite = &_debug_sprite_level,
	.yapf = &_debug_yapf_level,
};
/* Bound before main, so Rust Debug and GrfMsg see the native levels from the first message. */
[[maybe_unused]] static const bool _rust_debug_bound = (openttd_rust_debug_bind(&_rust_debug_services), true);
#include "abi-visit-OpenTTDDebugServices.hpp"
bool CheckAbi_debug() { return AbiServicesComplete(_rust_debug_services); }
