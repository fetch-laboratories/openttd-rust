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
#include "../aircraft.h"
#include "../ship.h"
#include "../train.h"
#include "../station_base.h"
#include "vehicle_maintenance_ffi.h"
#include "clock_ffi.h"
#include "aircraft_ffi.h"
#include "fleet_ffi.h"
#include "services_ffi.h"
#include "company_identity_ffi.h"
#include "company_services.h"
#include "environment_services.h"
#include "../company_base.h"
#include "../company_func.h"
#include "../effectvehicle_base.h"
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

bool RustTrainIsEngine(const Vehicle *v) noexcept { return Train::From(v)->IsEngine(); }
uint32_t RustGRFString(uint32_t grfid, uint32_t string) noexcept { return GetGRFStringID(grfid, GRFStringID(string)); }

void RustVehicleSetCargoCap(OpenTTDVehicle *v, uint16_t cap) noexcept { VehicleOf(v)->Record().cargo_cap = cap; }
void RustVehicleSetCargoType(OpenTTDVehicle *v, uint8_t cargo) noexcept { VehicleOf(v)->cargo_type = cargo; }
void RustVehicleSetCargoSubtype(OpenTTDVehicle *v, uint8_t subtype) noexcept { VehicleOf(v)->cargo_subtype = subtype; }
void RustVehicleSetCargoKind(OpenTTDVehicle *v, uint8_t cargo, uint8_t subtype) noexcept { RustVehicleSetCargoType(v, cargo); RustVehicleSetCargoSubtype(v, subtype); }
void RustVehicleInvalidateChain(OpenTTDVehicle *v) noexcept { VehicleOf(v)->InvalidateNewGRFCacheOfChain(); }
ExpensesType RustVehicleExpense(OpenTTDVehicle *v, bool income) noexcept { return VehicleOf(v)->GetExpenseType(income); }
/* Values the Rust vehicle ports name. */
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

static void OPENTTD_RUST_CALL VehicleSvcPathfindingResult(OpenTTDVehicle *v, bool found) noexcept { VehicleOf(v)->HandlePathfindingResult(found); }
static void OPENTTD_RUST_CALL VehicleSvcBaseViewport(OpenTTDVehicle *v) noexcept { VehicleOf(v)->Vehicle::UpdateViewport(true); }
uint64_t RustTrainRailTypes(const Vehicle *v) noexcept { return Train::From(v)->GetRailTypes().base(); }
Train *RustTrainGetIfValid(uint32_t id) noexcept { return Train::GetIfValid(VehicleID(id)); }
OpenTTDVehicle *RustShipGet(uint32_t id) noexcept { return RustVehicle(Ship::Get(VehicleID(id))); }
static const OpenTTDOrdersLeaves *VehicleOrders() noexcept { return &GetRustOrdersLeaves(); }


static void OPENTTD_RUST_CALL VehicleSvcLengthChanged(OpenTTDVehicle *v) noexcept { VehicleLengthChanged(VehicleOf(v)); }
static bool OPENTTD_RUST_CALL VehicleSvcPlayVehicleSound(OpenTTDVehicle *v, VehicleSoundEvent event) noexcept { return PlayVehicleSound(VehicleOf(v), event); }
static void OPENTTD_RUST_CALL VehicleSvcPlaySound(OpenTTDVehicle *v, uint16_t sound) noexcept { SndPlayVehicleFx(static_cast<SoundID>(sound), VehicleOf(v)); }
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





static void OPENTTD_RUST_CALL VehicleSvcWriteProfit(OpenTTDVehicle *v, int64_t value) noexcept { VehicleOf(v)->profit_this_year = value; }
static void OPENTTD_RUST_CALL VehicleSvcWriteEconomyService(OpenTTDVehicle *v, int32_t value) noexcept { VehicleOf(v)->date_of_last_service = TimerGameEconomy::Date{value}; }
static void OPENTTD_RUST_CALL VehicleSvcWriteCalendarService(OpenTTDVehicle *v, int32_t value) noexcept { VehicleOf(v)->date_of_last_service_newgrf = TimerGameCalendar::Date{value}; }
static void OPENTTD_RUST_CALL VehicleSvcWriteBreakdowns(OpenTTDVehicle *v, uint8_t value) noexcept { VehicleOf(v)->breakdowns_since_last_service = value; }
static void OPENTTD_RUST_CALL VehicleSvcWriteReliability(OpenTTDVehicle *v, uint16_t value) noexcept { VehicleOf(v)->reliability = value; }
static int64_t OPENTTD_RUST_CALL VehicleSvcProfit(OpenTTDVehicle *v) noexcept { return RustVehicleProfit(VehicleOf(v)); }


const OpenTTDOccupancyCommands *GetRustOccupancyCommands() noexcept
{
	static const OpenTTDOccupancyCommands services{.vehicles = &GetRustVehicleServices(), .shared = &GetRustSharedServices(), .max_pixel_z = OpenTTDTileMaxPixelZ, .costs = &GetRustCommandCostServices(), .vehicle_error = STR_ERROR_TRAIN_IN_THE_WAY};
	return &services;
}
static_assert(AIR_SHADOW == 4);

static OpenTTDVehicleTileLinks *OPENTTD_RUST_CALL VehicleSvcTileLinks(OpenTTDVehicle *v) noexcept { return RustVehicleTileLinks(VehicleOf(v)); }

static OpenTTDVehicle *VehicleRecordFromShell(void *shell) noexcept { return shell == nullptr ? nullptr : static_cast<Vehicle *>(shell)->record; }

constinit const OpenTTDVehicleServices _rust_vehicle_services{
	.base_viewport = VehicleSvcBaseViewport,
	.can_allocate = RustVehicleCanAllocate,
	.destroy = VehicleSvcDestroy,
	.engine = RustVehicleGetEngine,
	.first_sprite = RustVehicleRead<uint32_t, RustVehicleFirstSprite>,
	.length_changed = VehicleSvcLengthChanged,
	.maintenance = &_rust_vehicle_maintenance_services,
	.newgrf = []() noexcept { return &GetRustVehicleResolverServices(); },
	.orders = VehicleOrders,
	.pathfinding_result = VehicleSvcPathfindingResult,
	.play_sound = VehicleSvcPlaySound,
	.play_vehicle_sound = VehicleSvcPlayVehicleSound,
	.profit = VehicleSvcProfit,
	.record_from_shell = VehicleRecordFromShell,
	.set_max_age = RustVehicleSetMaxAge,
	.ship_get = RustShipGet,
	.start_stop_dirty = VehicleSvcStartStopDirty,
	.tile_links = VehicleSvcTileLinks,
	.update_delta = RustVehicleUpdateDelta<OpenTTDVehicle *>,
	.visits = RustVehicleVisits,
	.write_breakdowns = VehicleSvcWriteBreakdowns,
	.write_calendar_service = VehicleSvcWriteCalendarService,
	.write_economy_service = VehicleSvcWriteEconomyService,
	.write_profit = VehicleSvcWriteProfit,
	.write_reliability = VehicleSvcWriteReliability,
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
	};
	return services;
}

/* The members vehicle/maintenance.rs reads and writes that the shell still stores, one slot each. */
static int32_t OPENTTD_RUST_CALL MaintenanceEconomyAge(OpenTTDVehicle *handle) noexcept { return RustVehicleEconomyAge(VehicleOf(handle)).base(); }
static void OPENTTD_RUST_CALL MaintenanceMarkDirty(OpenTTDVehicle *handle) noexcept { VehicleOf(handle)->MarkDirty(); }
static void OPENTTD_RUST_CALL MaintenanceSetAge(OpenTTDVehicle *handle, int32_t age) noexcept { RustWriteVehicleAge(VehicleOf(handle), age); }
static void OPENTTD_RUST_CALL MaintenanceSetBreakdownChance(OpenTTDVehicle *handle, uint8_t chance) noexcept { VehicleOf(handle)->breakdown_chance = chance; }
static void OPENTTD_RUST_CALL MaintenanceSetBreakdownDelay(OpenTTDVehicle *handle, uint8_t delay) noexcept { VehicleOf(handle)->breakdown_delay = delay; }
static void OPENTTD_RUST_CALL MaintenanceSetEconomyAge(OpenTTDVehicle *handle, int32_t age) noexcept { VehicleOf(handle)->economy_age = TimerGameEconomy::Date{age}; }
static void OPENTTD_RUST_CALL MaintenanceSetReliabilitySpdDec(OpenTTDVehicle *handle, uint16_t decay) noexcept { VehicleOf(handle)->reliability_spd_dec = decay; }
static void OPENTTD_RUST_CALL MaintenanceSetValue(OpenTTDVehicle *handle, int64_t value) noexcept { VehicleOf(handle)->value = Money(value); }
static_assert(to_underlying(EngineMiscFlag::NoBreakdownSmoke) == 6);
static_assert(to_underlying(LandscapeType::Toyland) == 3);
static_assert(EconomyTime::DAYS_IN_ECONOMY_MONTH == 30 && CalendarTime::DAYS_IN_LEAP_YEAR == 366);
static_assert(SND_0F_BREAKDOWN_ROADVEHICLE == 13 && SND_10_BREAKDOWN_TRAIN_SHIP == 14 && SND_35_BREAKDOWN_ROADVEHICLE_TOYLAND == 53 && SND_3A_BREAKDOWN_TRAIN_SHIP_TOYLAND == 58);
const OpenTTDVehicleMaintenanceServices _rust_vehicle_maintenance_services{
	.clock = &GetRustClockServices(),
	.company = &GetRustCompanyIdentityServices(),
	.date_of_last_service = RustVehicleLastServiceOf,
	.economy_age = MaintenanceEconomyAge,
	.group = &FleetGroupServices(),
	.mark_dirty = MaintenanceMarkDirty,
	.set_age = MaintenanceSetAge,
	.set_breakdown_chance = MaintenanceSetBreakdownChance,
	.set_breakdown_delay = MaintenanceSetBreakdownDelay,
	.set_economy_age = MaintenanceSetEconomyAge,
	.set_reliability_spd_dec = MaintenanceSetReliabilitySpdDec,
	.set_value = MaintenanceSetValue,
	.shared = &GetRustSharedServices(),
	.transaction = &FleetTransactionServices(),
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
