/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file vehicle_services.cpp Shared typed vehicle and map services for Rust ports. */
#include "../stdafx.h"
#include "vehicle_ffi.h"
#include "vehicle_services.h"
#include "orders_ffi.h"
#include "station_service_ffi.h"
#include "vehicle_visits_ffi.h"
#include "command_cost_ffi.h"
#include "tile_animation_services.h"
#include "engine_services.h"
#include "services_newgrf.h"
#include "../vehicle_func.h"
#include "../order_func.h"
#include "../newgrf_engine.h"
#include "../newgrf_text.h"
#include "../engine_base.h"
#include "../newgrf_sound.h"
#include "../newgrf_callbacks.h"
#include "../sound_func.h"
#include "../effectvehicle_func.h"
#include "../economy_type.h"
#include "../station_map.h"
#include "../depot_map.h"
#include "../rail_map.h"
#include "../tunnelbridge_map.h"
#include "../bridge.h"
#include "../tile_cmd.h"
#include "../track_type.h"
#include "../window_func.h"
#include "../widgets/vehicle_widget.h"
#include "../sound_type.h"
#include "../framerate_type.h"
#include "../aircraft.h"
#include "../ship.h"
#include "../train.h"
#include "../station_base.h"
#include "vehicle_maintenance_ffi.h"
#include "aircraft_ffi.h"
#include "fleet_ffi.h"
#include "services_ffi.h"
#include "company_identity_ffi.h"
#include "company_services.h"
#include "environment_services.h"
#include "../company_base.h"
#include "../company_func.h"
#include "../autoreplace_func.h"
#include "../effectvehicle_base.h"
#include "../group.h"
#include "../news_func.h"
#include "../strings_func.h"
#include "../table/strings.h"
OpenTTDAircraftState *RustVehicleAircraftOwner(const Vehicle *v) noexcept { return Aircraft::From(v)->rust_state.get(); }
OpenTTDShipState *RustVehicleShipOwner(const Vehicle *v) noexcept { return Ship::From(v)->GetRustState(); }

OpenTTDAircraftVehicle RustAircraftVehicle(Vehicle *v) noexcept
{
	if (v == nullptr) return {nullptr, nullptr, VehicleID::Invalid().base()};
	return {RustVehicle(v), v->GetType() == VEH_AIRCRAFT ? RustVehicleAircraftOwner(v) : nullptr, RustVehicleIndexOpaque(v)};
}
OpenTTDAircraftVehicle GetRustAircraftVehicle(OpenTTDVehicle *v) noexcept { return RustAircraftVehicle(VehicleOf(v)); }


#include "../safeguards.h"

int32_t RustGRFPriceShift(const GRFFile *grf, uint32_t price) noexcept { return grf == nullptr ? 0 : grf->price_base_multipliers[price]; }

OpenTTDByteSpan RustVehicleName(void *shell) noexcept
{
	const std::string &name = static_cast<Vehicle *>(shell)->name;
	return {reinterpret_cast<const uint8_t *>(name.data()), name.size()};
}
void RustVehicleClearName(void *shell) noexcept { static_cast<Vehicle *>(shell)->name.clear(); }
void RustVehicleSetName(void *shell, OpenTTDByteSpan name) noexcept { static_cast<Vehicle *>(shell)->name.assign(reinterpret_cast<const char *>(name.data), name.size); }

bool RustVehiclePrimary(const Vehicle *v) noexcept { return v->IsPrimaryVehicle(); }
bool RustVehicleInDepot(const Vehicle *v) noexcept { return v->IsInDepot(); }
Vehicle *RustTrainFirstEnginePart(Vehicle *v) noexcept { return v->GetFirstEnginePart(); }
Vehicle *RustTrainOtherMultihead(const Vehicle *v) noexcept { return Train::From(v)->GetOtherMultiheadedPart(); }

OpenTTDTrainHandle RustVehicleTrain(OpenTTDVehicle *v) noexcept { return TrainHandle(Train::From(VehicleOf(v))); }
bool RustTrainIsEngine(const Vehicle *v) noexcept { return Train::From(v)->IsEngine(); }
uint32_t RustGRFString(uint32_t grfid, uint32_t string) noexcept { return GetGRFStringID(grfid, GRFStringID(string)); }

void RustVehicleSetCargoCap(OpenTTDVehicle *v, uint16_t cap) noexcept { VehicleOf(v)->Record().cargo_cap = cap; }
void RustVehicleSetCargoType(OpenTTDVehicle *v, uint8_t cargo) noexcept { VehicleOf(v)->cargo_type = cargo; }
void RustVehicleSetCargoSubtype(OpenTTDVehicle *v, uint8_t subtype) noexcept { VehicleOf(v)->cargo_subtype = subtype; }
void RustVehicleSetCargoKind(OpenTTDVehicle *v, uint8_t cargo, uint8_t subtype) noexcept { RustVehicleSetCargoType(v, cargo); RustVehicleSetCargoSubtype(v, subtype); }
void RustVehicleInvalidateChain(OpenTTDVehicle *v) noexcept { VehicleOf(v)->InvalidateNewGRFCacheOfChain(); }
ExpensesType RustVehicleExpense(OpenTTDVehicle *v, bool income) noexcept { return VehicleOf(v)->GetExpenseType(income); }
/* Values the Rust side names in rust/openttd-kernels/src/vehicle.rs. */
static_assert(MP_ROAD == 2 && MP_STATION == 5 && MP_TUNNELBRIDGE == 9);
static_assert(OT_GOTO_STATION == 1 && OT_GOTO_DEPOT == 2 && OT_LOADING == 3 && OT_LEAVESTATION == 4);
static_assert(VehStates{VehState::Hidden}.base() == 1 && VehStates{VehState::Stopped}.base() == 2 && VehStates{VehState::AircraftBroken}.base() == 64 && VehStates{VehState::Crashed}.base() == 128);
static_assert(VehicleEnterTileStates{VehicleEnterTileState::EnteredWormhole}.base() == 2 && VehicleEnterTileStates{VehicleEnterTileState::CannotEnter}.base() == 4);
static_assert(AM_REALISTIC == 1 && INVALID_TRACKDIR == 0xFF && INVALID_PRICE == 0xFF && CALLBACK_FAILED == 0xFFFF && VEHICLE_LENGTH == 8);
static_assert(EngineID::Invalid().base() == 0xFFFF);
static_assert(CalendarTime::DAYS_IN_YEAR == 365 && CalendarTime::DAYS_IN_LEAP_YEAR == 366 && Ticks::DAY_TICKS == 74);
static_assert(SND_19_DEPARTURE_OLD_RV_1 == 23 && SND_1A_DEPARTURE_OLD_RV_2 == 24);
static_assert(PROP_AIRCRAFT_SPEED == 0x0C && CBID_VEHICLE_LENGTH == 0x11 && CBID_VEHICLE_32DAY_CALLBACK == 0x32);
static_assert(VSE_START == 1 && VSE_BREAKDOWN == 3 && VSE_RUNNING == 4 && VSE_TOUCHDOWN == 5 && VSE_RUNNING_16 == 7 && VSE_STOPPED_16 == 8);
static_assert(HVOT_BUS == 4 && HVOT_TRUCK == 8);

static void OPENTTD_RUST_CALL VehicleSvcCheckOrdersOnly(OpenTTDVehicle *v) noexcept { CheckOrders(VehicleOf(v)); }
static bool OPENTTD_RUST_CALL VehicleSvcWaitingForUnbunching(OpenTTDVehicle *v) noexcept { return VehicleOf(v)->IsWaitingForUnbunching(); }
static void OPENTTD_RUST_CALL VehicleSvcLeaveUnbunchingDepot(OpenTTDVehicle *v) noexcept { VehicleOf(v)->LeaveUnbunchingDepot(); }
static void OPENTTD_RUST_CALL VehicleSvcResetDepotUnbunching(OpenTTDVehicle *v) noexcept { VehicleOf(v)->ResetDepotUnbunching(); }
static void OPENTTD_RUST_CALL VehicleSvcPathfindingResult(OpenTTDVehicle *v, bool found) noexcept { VehicleOf(v)->HandlePathfindingResult(found); }
static OpenTTDVehicleNewPosition OPENTTD_RUST_CALL VehicleSvcNewPosition(OpenTTDVehicle *v) noexcept
{
	GetNewVehiclePosResult gp = GetNewVehiclePos(VehicleOf(v));
	return {.x = gp.x, .y = gp.y, .tile = gp.new_tile.base()};
}
static void OPENTTD_RUST_CALL VehicleSvcBaseViewport(OpenTTDVehicle *v) noexcept { VehicleOf(v)->Vehicle::UpdateViewport(true); }
uint32_t RustTrainCachedWeight(const Vehicle *v) noexcept { return Train::From(v)->gcache.cached_weight; }
uint64_t RustTrainRailTypes(const Vehicle *v) noexcept { return Train::From(v)->GetRailTypes().base(); }
Train *RustTrainGetIfValid(uint32_t id) noexcept { return Train::GetIfValid(VehicleID(id)); }
OpenTTDVehicle *RustShipGet(uint32_t id) noexcept { return RustVehicle(Ship::Get(VehicleID(id))); }
static const OpenTTDOrdersLeaves *VehicleOrders() noexcept { return &GetRustOrdersLeaves(); }


static void OPENTTD_RUST_CALL VehicleSvcLengthChanged(OpenTTDVehicle *v) noexcept { VehicleLengthChanged(VehicleOf(v)); }
static bool OPENTTD_RUST_CALL VehicleSvcPlayVehicleSound(OpenTTDVehicle *v, VehicleSoundEvent event) noexcept { return PlayVehicleSound(VehicleOf(v), event); }
static void OPENTTD_RUST_CALL VehicleSvcPlaySound(OpenTTDVehicle *v, uint16_t sound) noexcept { SndPlayVehicleFx(static_cast<SoundID>(sound), VehicleOf(v)); }
static void OPENTTD_RUST_CALL VehicleSvcOrderFree(OpenTTDVehicle *v) noexcept { VehicleOf(v)->current_order.Free(); }
static void OPENTTD_RUST_CALL VehicleSvcOrderDummy(OpenTTDVehicle *v) noexcept
{
	Vehicle *u = VehicleOf(v);
	u->current_order.MakeDummy();
	SetWindowWidgetDirty(WC_VEHICLE_VIEW, u->index, WID_VV_START_STOP);
}
static void OPENTTD_RUST_CALL VehicleSvcStartStopDirty(OpenTTDVehicle *v) noexcept { SetWindowWidgetDirty(WC_VEHICLE_VIEW, VehicleOf(v)->index, WID_VV_START_STOP); }
static void OPENTTD_RUST_CALL VehicleSvcDestroy(OpenTTDVehicle *v) noexcept { delete VehicleOf(v); }
/* Legacy orders context adapters; the native operations have shared definitions. */
extern "C" uint64_t openttd_orders_primary_vehicle(void *ctx) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return RustVehiclePrimary(v);
}
extern "C" void openttd_orders_set_destination(void *ctx, uint32_t tile) noexcept
{
	RustVehicleSetDestTile(RustVehicle(static_cast<Vehicle *>(ctx)), tile);
}



static uint8_t OPENTTD_RUST_CALL VehicleSvcOrderType(OpenTTDVehicle *v) noexcept { return RustVehicleOrderType(VehicleOf(v)); }
static uint16_t OPENTTD_RUST_CALL VehicleSvcOrderDestination(OpenTTDVehicle *v) noexcept { return RustVehicleOrderDestination(VehicleOf(v)); }
static uint8_t OPENTTD_RUST_CALL VehicleSvcOrderNonStop(OpenTTDVehicle *v) noexcept { return RustVehicleOrderNonStop(VehicleOf(v)); }


static void OPENTTD_RUST_CALL VehicleSvcWriteProfit(OpenTTDVehicle *v, int64_t value) noexcept { VehicleOf(v)->profit_this_year = value; }
static void OPENTTD_RUST_CALL VehicleSvcWriteEconomyService(OpenTTDVehicle *v, int32_t value) noexcept { VehicleOf(v)->date_of_last_service = TimerGameEconomy::Date{value}; }
static void OPENTTD_RUST_CALL VehicleSvcWriteCalendarService(OpenTTDVehicle *v, int32_t value) noexcept { VehicleOf(v)->date_of_last_service_newgrf = TimerGameCalendar::Date{value}; }
static void OPENTTD_RUST_CALL VehicleSvcWriteBreakdowns(OpenTTDVehicle *v, uint8_t value) noexcept { VehicleOf(v)->breakdowns_since_last_service = value; }
static void OPENTTD_RUST_CALL VehicleSvcWriteReliability(OpenTTDVehicle *v, uint16_t value) noexcept { VehicleOf(v)->reliability = value; }
static int64_t OPENTTD_RUST_CALL VehicleSvcProfit(OpenTTDVehicle *v) noexcept { return RustVehicleProfit(VehicleOf(v)); }
static void OPENTTD_RUST_CALL VehicleSvcMakeDummy(OpenTTDVehicle *v) noexcept { VehicleOf(v)->current_order.MakeDummy(); }


const OpenTTDOccupancyCommands *GetRustOccupancyCommands() noexcept
{
	static const OpenTTDOccupancyCommands services{.vehicles = &GetRustVehicleServices(), .max_pixel_z = OpenTTDTileMaxPixelZ, .costs = &GetRustCommandCostServices(), .vehicle_error = STR_ERROR_TRAIN_IN_THE_WAY};
	return &services;
}
static_assert(AIR_SHADOW == 4);

static OpenTTDVehicleTileLinks *OPENTTD_RUST_CALL VehicleSvcTileLinks(OpenTTDVehicle *v) noexcept { return RustVehicleTileLinks(VehicleOf(v)); }

static OpenTTDVehicle *VehicleRecordFromShell(void *shell) noexcept { return shell == nullptr ? nullptr : static_cast<Vehicle *>(shell)->record; }
static void VehicleRecordAssertNext([[maybe_unused]] OpenTTDVehicle *v, [[maybe_unused]] OpenTTDVehicle *next) noexcept { assert(v != next); }

constinit const OpenTTDVehicleServices _rust_vehicle_services{
	.assert_next = VehicleRecordAssertNext,
	.record_from_shell = VehicleRecordFromShell,
	.tile_links = VehicleSvcTileLinks,
	.map_log_x = OpenTTDMapLogX,
#if !defined(NDEBUG) || defined(WITH_ASSERT)
	.assertions = true,
#else
	.assertions = false,
#endif
	.train_handle = RustVehicleTrain,
	.waiting_for_unbunching = VehicleSvcWaitingForUnbunching,
	.leave_unbunching_depot = VehicleSvcLeaveUnbunchingDepot,
	.reset_depot_unbunching = VehicleSvcResetDepotUnbunching,
	.enter_depot = openttd_rust_visit_enter_depot,
	.pathfinding_result = VehicleSvcPathfindingResult,
	.new_position = VehicleSvcNewPosition,
	.base_viewport = VehicleSvcBaseViewport,
	.newgrf = []() noexcept { return &GetRustVehicleResolverServices(); },
	.property = openttd_rust_vehicle_shell_property,
	.callback = openttd_rust_vehicle_shell_callback,
	.set_max_age = RustVehicleSetMaxAge,
	.length_changed = VehicleSvcLengthChanged,
	.play_vehicle_sound = VehicleSvcPlayVehicleSound,
	.play_sound = VehicleSvcPlaySound,
	.order_free = VehicleSvcOrderFree,
	.order_dummy = VehicleSvcOrderDummy,
	.start_stop_dirty = VehicleSvcStartStopDirty,
	.destroy = VehicleSvcDestroy,
	.set_dest_tile = RustVehicleSetDestTile,
	.engine = RustVehicleGetEngine,
	.order_type = VehicleSvcOrderType,
	.order_destination = VehicleSvcOrderDestination,
	.order_nonstop = VehicleSvcOrderNonStop,
	.write_profit = VehicleSvcWriteProfit,
	.write_economy_service = VehicleSvcWriteEconomyService,
	.write_calendar_service = VehicleSvcWriteCalendarService,
	.write_breakdowns = VehicleSvcWriteBreakdowns,
	.write_reliability = VehicleSvcWriteReliability,
	.profit = VehicleSvcProfit,
	.make_dummy = VehicleSvcMakeDummy,
	.check_orders_only = VehicleSvcCheckOrdersOnly,
	.maintenance = &_rust_vehicle_maintenance_services,
	.visits = RustVehicleVisits,
	.orders = VehicleOrders,
	.ship_get = RustShipGet,
	.first_sprite = RustVehicleRead<uint32_t, RustVehicleFirstSprite>,
	.update_delta = RustVehicleUpdateDelta<OpenTTDVehicle *>,
	.can_allocate = RustVehicleCanAllocate,
};

const OpenTTDVehicleServices &GetRustVehicleServices() noexcept
{
	return _rust_vehicle_services;
}


/* New scheduler constants consumed by Rust branches. */
static_assert(VEH_TRAIN == 0 && VEH_ROAD == 1 && VEH_SHIP == 2 && VEH_AIRCRAFT == 3 && VEH_EFFECT == 4 && VEH_DISASTER == 5 && VEH_END == 6 && VEH_INVALID == 255);
static_assert(GVSF_FRONT == 0 && GVSF_ENGINE == 3 && AIR_AIRCRAFT == 2);
static_assert(VehStates{VehState::TrainSlowing}.base() == 16 && Ticks::DAY_TICKS == 74);

static OpenTTDVehiclePoolCursor OPENTTD_RUST_CALL VehicleSchedulerPool() noexcept
{
	return {.slots = _vehicle_pool.descriptor->slots.items, .slots_len = _vehicle_pool.descriptor->slots.size(), .first_unused = &_vehicle_pool.descriptor->first_unused};
}
/* Existing virtual facades own the original controller timing scopes. The
 * timing destructor completes before this callback returns to Rust assertions;
 * Rust then reads the survivor's type and First() from its records. */
static bool OPENTTD_RUST_CALL VehicleSchedulerTick(OpenTTDVehicle *shell) noexcept { return VehicleOf(shell)->Tick(); }
static void OPENTTD_RUST_CALL VehicleSchedulerCalendarDay(OpenTTDVehicle *shell) noexcept { VehicleOf(shell)->OnNewCalendarDay(); }
static void OPENTTD_RUST_CALL VehicleSchedulerEconomyDay(OpenTTDVehicle *shell) noexcept { VehicleOf(shell)->OnNewEconomyDay(); }
static void OPENTTD_RUST_CALL VehicleSchedulerWithLoadingTimer(const OpenTTDVehicleSchedulerTables *tables, void (OPENTTD_RUST_CALL *load)(const OpenTTDVehicleSchedulerTables *)) noexcept
{
	PerformanceMeasurer framerate(PFE_GL_ECONOMY);
	load(tables);
}
static void OPENTTD_RUST_CALL VehicleSchedulerResetAccumulators() noexcept
{
	PerformanceAccumulator::Reset(PFE_GL_TRAINS);
	PerformanceAccumulator::Reset(PFE_GL_ROADVEHS);
	PerformanceAccumulator::Reset(PFE_GL_SHIPS);
	PerformanceAccumulator::Reset(PFE_GL_AIRCRAFT);
}

const OpenTTDVehicleSchedulerServices &GetRustVehicleSchedulerServices() noexcept
{
	static const OpenTTDVehicleSchedulerServices services{
		.pool = VehicleSchedulerPool,
#if !defined(NDEBUG) || defined(WITH_ASSERT)
		.assertions = true,
#else
		.assertions = false,
#endif
		.tick = VehicleSchedulerTick,
		.calendar_day = VehicleSchedulerCalendarDay,
		.economy_day = VehicleSchedulerEconomyDay,
		.cargo_owner = RustVehicleRead<OpenTTDCargoList *, RustVehicleCargoOwner>,
		.vehicle = &_rust_vehicle_services,
		.shared = &GetRustSharedServices(),
		.with_loading_timer = VehicleSchedulerWithLoadingTimer,
		.reset_accumulators = VehicleSchedulerResetAccumulators,
	};
	return services;
}

/* Operation-sized native observations and shared-service adapters. The complete
 * transition, traversal and decision policy lives in vehicle/maintenance.rs. */
static OpenTTDCalendarDate MaintenanceDate(TimerGameCalendar::Date date) noexcept { return {date.base()}; }
static OpenTTDEconomyDate MaintenanceDate(TimerGameEconomy::Date date) noexcept { return {date.base()}; }
static OpenTTDVehicleMaintenanceRenewal OPENTTD_RUST_CALL MaintenanceRenewal(OpenTTDVehicle *handle, const Company *company) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	assert(company == Company::Get(v->GetOwner()));
	return {.age = MaintenanceDate(RustVehicleAge(v)), .max_age = MaintenanceDate(RustVehicleMaxAge(v)), .months = company->settings.engine_renew_months, .enabled = company->settings.engine_renew, .kind = RustVehicleVehicleType(v), .subtype = RustVehicleSubtype(v)};
}
static_assert(to_underlying(EngineMiscFlag::NoBreakdownSmoke) == 6);
static OpenTTDVehicleMaintenanceReset OPENTTD_RUST_CALL MaintenanceReset(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.economy_date = MaintenanceDate(TimerGameEconomy::date), .calendar_date = MaintenanceDate(TimerGameCalendar::date), .chance = RustVehicleBreakdownChance(v), .breakdowns = _settings_game.difficulty.vehicle_breakdowns};
}
static bool OPENTTD_RUST_CALL MaintenanceHasEngine(OpenTTDVehicle *handle) noexcept { return RustVehicleHasEngine(VehicleOf(handle)); }
static OpenTTDVehicleMaintenanceInterval OPENTTD_RUST_CALL MaintenanceInterval(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	const Company *company = Company::Get(v->GetOwner());
	return {.company = company, .last_service = MaintenanceDate(RustVehicleLastService(v)), .now = MaintenanceDate(TimerGameEconomy::date), .interval = RustVehicleServiceInterval(v), .reliability = RustVehicleReliability(v), .percent = v->ServiceIntervalIsPercent(), .wallclock = TimerGameEconomy::UsingWallclockUnits(), .no_service = _settings_game.order.no_servicing_if_no_breakdowns, .breakdowns = _settings_game.difficulty.vehicle_breakdowns};
}
static OpenTTDVehicleMaintenanceReplacement OPENTTD_RUST_CALL MaintenanceReplacement(OpenTTDVehicle *handle, const Company *company) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	bool when_old = false;
	EngineID engine = EngineReplacementForCompany(company, v->GetEngineType(), v->group_id, &when_old);
	return {.engine = engine.base(), .when_old = when_old};
}
static OpenTTDVehicleMaintenanceAutomatic OPENTTD_RUST_CALL MaintenanceAutomatic(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.has_depot_order = v->HasDepotOrder(), .order_type = RustVehicleOrderType(v)};
}
static bool OPENTTD_RUST_CALL MaintenanceDepotService(OpenTTDVehicle *handle) noexcept { return VehicleOf(handle)->current_order.GetDepotOrderType().Test(OrderDepotTypeFlag::Service); }
static OpenTTDVehicleMaintenanceCheckBreakdown OPENTTD_RUST_CALL MaintenanceCheckBreakdown(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.reliability = RustVehicleReliability(v), .decay = RustVehicleReliabilityDecay(v), .no_service = _settings_game.order.no_servicing_if_no_breakdowns, .breakdowns = _settings_game.difficulty.vehicle_breakdowns};
}
static OpenTTDVehicleMaintenanceBreakdownGuard OPENTTD_RUST_CALL MaintenanceBreakdownGuard(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.speed = RustVehicleSpeed(v), .counter = RustVehicleBreakdownCounter(v), .status = RustVehicleStatus(v), .breakdowns = _settings_game.difficulty.vehicle_breakdowns, .menu = _game_mode == GM_MENU};
}
static OpenTTDVehicleMaintenanceBreakdownTrigger OPENTTD_RUST_CALL MaintenanceBreakdownTrigger(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.reliability = RustVehicleReliability(v), .chance = RustVehicleBreakdownChance(v), .kind = RustVehicleVehicleType(v), .breakdowns = _settings_game.difficulty.vehicle_breakdowns};
}
static void OPENTTD_RUST_CALL MaintenanceChanceWrite(OpenTTDVehicle *handle, uint8_t chance) noexcept { VehicleOf(handle)->breakdown_chance = chance; }
static void OPENTTD_RUST_CALL MaintenanceTrigger(OpenTTDVehicle *handle, uint8_t counter, uint8_t delay) noexcept
{
	Vehicle *v = VehicleOf(handle);
	RustVehicleWriteBreakdown(v, counter, delay);
	v->breakdown_chance = 0;
}
static OpenTTDVehicleMaintenanceBreakdownProbe OPENTTD_RUST_CALL MaintenanceBreakdownProbe(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.counter = RustVehicleBreakdownCounter(v), .status = RustVehicleStatus(v)};
}
static OpenTTDVehicleMaintenanceBreakdownStart OPENTTD_RUST_CALL MaintenanceBreakdownStart(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.since = RustVehicleBreakdownsSinceService(v), .kind = RustVehicleVehicleType(v)};
}
static void OPENTTD_RUST_CALL MaintenanceBreakdownStartWrite(OpenTTDVehicle *handle, uint8_t counter, uint8_t since) noexcept
{
	Vehicle *v = VehicleOf(handle);
	v->Record().breakdown_ctr = counter;
	v->breakdowns_since_last_service = since;
}
static OpenTTDVehicleMaintenanceBreakdownActive OPENTTD_RUST_CALL MaintenanceBreakdownActive(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.kind = RustVehicleVehicleType(v), .tick = RustVehicleTick(v), .delay = RustVehicleBreakdownDelay(v)};
}
static OpenTTDVehicleMaintenanceBreakdownCountdown OPENTTD_RUST_CALL MaintenanceBreakdownCountdown(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.order_type = RustVehicleOrderType(v), .counter = RustVehicleBreakdownCounter(v)};
}
static void OPENTTD_RUST_CALL MaintenanceBreakdownDelayWrite(OpenTTDVehicle *handle, uint8_t delay) noexcept { VehicleOf(handle)->breakdown_delay = delay; }
static void OPENTTD_RUST_CALL MaintenanceAircraftBroken(OpenTTDVehicle *handle) noexcept { VehicleOf(handle)->SetVehStatus(VehicleOf(handle)->GetVehStatus().Set(VehState::AircraftBroken)); }
static void OPENTTD_RUST_CALL MaintenanceMarkDirty(OpenTTDVehicle *handle) noexcept { VehicleOf(handle)->MarkDirty(); }
static void OPENTTD_RUST_CALL MaintenanceViewDirty(OpenTTDVehicle *handle) noexcept { SetWindowDirty(WC_VEHICLE_VIEW, VehicleOf(handle)->index); }
static OpenTTDVehicleMaintenanceAge OPENTTD_RUST_CALL MaintenanceAge(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.age = MaintenanceDate(RustVehicleAge(v)), .max_age = MaintenanceDate(RustVehicleMaxAge(v)), .decay = RustVehicleReliabilityDecay(v)};
}
static void OPENTTD_RUST_CALL MaintenanceAgeWrite(OpenTTDVehicle *handle, OpenTTDCalendarDate age) noexcept { RustWriteVehicleAge(VehicleOf(handle), age.value); }
static void OPENTTD_RUST_CALL MaintenanceDecayWrite(OpenTTDVehicle *handle, uint16_t decay) noexcept { VehicleOf(handle)->reliability_spd_dec = decay; }
static OpenTTDVehicleMaintenanceWarning OPENTTD_RUST_CALL MaintenanceWarning(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.enabled = _settings_client.gui.old_vehicle_warn, .has_previous = RustVehiclePrevious(v) != nullptr, .local = RustVehicleOwner(v) == RustLocalCompanyID(), .status = RustVehicleStatus(v)};
}
static OpenTTDVehicleMaintenanceWarningCompany OPENTTD_RUST_CALL MaintenanceWarningCompany(OpenTTDVehicle *handle) noexcept
{
	const Company *company = Company::Get(VehicleOf(handle)->GetOwner());
	return {.company = company, .renewal = company->settings.engine_renew};
}
static bool OPENTTD_RUST_CALL MaintenanceReplacementActive(OpenTTDVehicle *handle, const Company *company) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return EngineHasReplacementForCompany(company, v->GetEngineType(), v->group_id);
}
static void OPENTTD_RUST_CALL MaintenanceOldNews(OpenTTDVehicle *handle, StringID message) noexcept
{
	VehicleID id = VehicleOf(handle)->index;
	AddVehicleAdviceNewsItem(AdviceType::VehicleOld, GetEncodedString(message, id), id);
}
static OpenTTDEconomyDate OPENTTD_RUST_CALL MaintenanceEconomyAge(OpenTTDVehicle *handle) noexcept { return MaintenanceDate(RustVehicleEconomyAge(VehicleOf(handle))); }
static void OPENTTD_RUST_CALL MaintenanceEconomyAgeWrite(OpenTTDVehicle *handle, OpenTTDEconomyDate age) noexcept { VehicleOf(handle)->economy_age = TimerGameEconomy::Date{age.value}; }
static void OPENTTD_RUST_CALL MaintenanceReachedMinAge(OpenTTDVehicle *handle) noexcept { GroupStatistics::VehicleReachedMinAge(VehicleOf(handle)); }
static void OPENTTD_RUST_CALL MaintenanceValueWrite(OpenTTDVehicle *handle, int64_t value) noexcept { VehicleOf(handle)->value = Money(value); }
static_assert(to_underlying(LandscapeType::Toyland) == 3);
static_assert(EconomyTime::DAYS_IN_ECONOMY_MONTH == 30 && CalendarTime::DAYS_IN_LEAP_YEAR == 366 && NUM_CARGO == 64);
static_assert(SND_0F_BREAKDOWN_ROADVEHICLE == 13 && SND_10_BREAKDOWN_TRAIN_SHIP == 14 && SND_35_BREAKDOWN_ROADVEHICLE_TOYLAND == 53 && SND_3A_BREAKDOWN_TRAIN_SHIP_TOYLAND == 58);
const OpenTTDVehicleMaintenanceServices _rust_vehicle_maintenance_services{
	.shared = &GetRustSharedServices(),
	.transaction = &FleetTransactionServices(),
	.group = &FleetGroupServices(),
	.company = &GetRustCompanyIdentityServices(),
	.renewal = MaintenanceRenewal,
	.reset = MaintenanceReset,
	.has_engine = MaintenanceHasEngine,
	.interval = MaintenanceInterval,
	.replacement = MaintenanceReplacement,
	.automatic = MaintenanceAutomatic,
	.depot_service = MaintenanceDepotService,
	.check_breakdown = MaintenanceCheckBreakdown,
	.breakdown_guard = MaintenanceBreakdownGuard,
	.breakdown_trigger = MaintenanceBreakdownTrigger,
	.chance_write = MaintenanceChanceWrite,
	.trigger = MaintenanceTrigger,
	.breakdown_probe = MaintenanceBreakdownProbe,
	.breakdown_start = MaintenanceBreakdownStart,
	.breakdown_start_write = MaintenanceBreakdownStartWrite,
	.breakdown_active = MaintenanceBreakdownActive,
	.breakdown_countdown = MaintenanceBreakdownCountdown,
	.breakdown_delay_write = MaintenanceBreakdownDelayWrite,
	.smoke_delay = RustVehicleRead<uint8_t, RustVehicleBreakdownDelay>,
	.aircraft_broken = MaintenanceAircraftBroken,
	.mark_dirty = MaintenanceMarkDirty,
	.view_dirty = MaintenanceViewDirty,
	.details_dirty = RustVehicleDetailsDirty,
	.age = MaintenanceAge,
	.age_write = MaintenanceAgeWrite,
	.decay_write = MaintenanceDecayWrite,
	.warning = MaintenanceWarning,
	.warning_company = MaintenanceWarningCompany,
	.replacement_active = MaintenanceReplacementActive,
	.old_news = MaintenanceOldNews,
	.economy_age = MaintenanceEconomyAge,
	.economy_age_write = MaintenanceEconomyAgeWrite,
	.reached_min_age = MaintenanceReachedMinAge,
	.value = RustVehicleRead<int64_t, RustVehicleValue>,
	.value_write = MaintenanceValueWrite,
	.max_calendar_date = MaintenanceDate(CalendarTime::MAX_DATE),
	.max_economy_date = MaintenanceDate(EconomyTime::MAX_DATE),
	.profit_min_age = MaintenanceDate(VEHICLE_PROFIT_MIN_AGE),
	.anniversaries = {MaintenanceDate(TimerGameCalendar::DateAtStartOfYear(TimerGameCalendar::Year{0})), MaintenanceDate(TimerGameCalendar::DateAtStartOfYear(TimerGameCalendar::Year{1})), MaintenanceDate(TimerGameCalendar::DateAtStartOfYear(TimerGameCalendar::Year{2})), MaintenanceDate(TimerGameCalendar::DateAtStartOfYear(TimerGameCalendar::Year{3})), MaintenanceDate(TimerGameCalendar::DateAtStartOfYear(TimerGameCalendar::Year{4}))},
	.before_zero = MaintenanceDate(TimerGameCalendar::DateAtStartOfYear(TimerGameCalendar::Year{-1})),
	.news_getting_old = STR_NEWS_VEHICLE_IS_GETTING_OLD,
	.news_very_old = STR_NEWS_VEHICLE_IS_GETTING_VERY_OLD,
	.news_very_old_and = STR_NEWS_VEHICLE_IS_GETTING_VERY_OLD_AND,
};

/* Existing order ABI adapters share the canonical typed observations here. */
extern "C" uint64_t openttd_orders_age_years(void *ctx) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return TimerGameCalendar::DateToYear(RustVehicleAge(v)).base();
}

extern "C" uint64_t openttd_orders_remaining_years(void *ctx) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return std::max(TimerGameCalendar::DateToYear(RustVehicleMaxAge(v) - RustVehicleAge(v) + CalendarTime::DAYS_IN_LEAP_YEAR - 1), TimerGameCalendar::Year(0)).base();
}

#include "service_table_check.hpp"
#include "abi-visit-OpenTTDVehicleServices.hpp"
#include "abi-visit-OpenTTDVehicleSchedulerServices.hpp"
#include "abi-visit-OpenTTDVehicleMaintenanceServices.hpp"
bool CheckAbi_vehicle_services()
{
	return AbiServicesComplete(GetRustVehicleServices()) &&
		AbiServicesComplete(GetRustVehicleSchedulerServices()) &&
		AbiServicesComplete(_rust_vehicle_maintenance_services);
}
