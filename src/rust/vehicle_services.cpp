/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file vehicle_services.cpp Shared typed vehicle and map services for Rust ports. */
#include "../stdafx.h"
#include "vehicle_ffi.h"
#ifdef WITH_RUST
#include "vehicle_services.h"
#include "../vehicle_func.h"
#include "../order_func.h"
#include "../newgrf_engine.h"
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
#include "../station_base.h"
#include "vehicle_maintenance_ffi.h"
#include "fleet_ffi.h"
#include "services_ffi.h"
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
#include "../safeguards.h"

/* Values the Rust side names in rust/openttd-kernels/src/vehicle.rs. */
static_assert(MP_ROAD == 2 && MP_STATION == 5 && MP_TUNNELBRIDGE == 9);
static_assert(OT_GOTO_STATION == 1 && OT_GOTO_DEPOT == 2 && OT_LOADING == 3 && OT_LEAVESTATION == 4);
static_assert(VehStates{VehState::Hidden}.base() == 1 && VehStates{VehState::Stopped}.base() == 2 && VehStates{VehState::Crashed}.base() == 128);
static_assert(VehicleEnterTileStates{VehicleEnterTileState::EnteredWormhole}.base() == 2 && VehicleEnterTileStates{VehicleEnterTileState::CannotEnter}.base() == 4);
static_assert(AM_REALISTIC == 1 && INVALID_TRACKDIR == 0xFF && INVALID_PRICE == 0xFF && CALLBACK_FAILED == 0xFFFF && VEHICLE_LENGTH == 8);
static_assert(EngineID::Invalid().base() == 0xFFFF);
static_assert(CalendarTime::DAYS_IN_YEAR * Ticks::DAY_TICKS == 365 * 74);
static_assert(SND_19_DEPARTURE_OLD_RV_1 == 23 && SND_1A_DEPARTURE_OLD_RV_2 == 24);
static_assert(PROP_AIRCRAFT_SPEED == 0x0C);
static_assert(HVOT_BUS == 4 && HVOT_TRUCK == 8);

static void OPENTTD_VEHICLE_CALL VehicleSvcWriteCounters(OpenTTDVehicle *v, uint8_t tick, uint8_t running, int32_t order_time) noexcept
{
	Vehicle *u = VehicleOf(v);
	RustWriteVehicleTick(u, tick);
	u->running_ticks = running;
	u->current_order_time = order_time;
}
static uint8_t OPENTTD_VEHICLE_CALL VehicleSvcBreakdown(OpenTTDVehicle *v) noexcept
{
	Vehicle *u = VehicleOf(v);
	u->HandleBreakdown();
	return RustVehicleStatus(u);
}
static void OPENTTD_VEHICLE_CALL VehicleSvcCheckOrdersOnly(OpenTTDVehicle *v) noexcept { CheckOrders(VehicleOf(v)); }
static void OPENTTD_VEHICLE_CALL VehicleSvcWriteCoordinates(OpenTTDVehicle *v, int32_t x, int32_t y, int32_t z) noexcept
{
	Vehicle *u = VehicleOf(v);
	u->x_pos = x;
	u->y_pos = y;
	u->z_pos = z;
}
static OpenTTDVehicleBreakdown OPENTTD_VEHICLE_CALL VehicleSvcHandleBreakdown(OpenTTDVehicle *v) noexcept
{
	Vehicle *u = VehicleOf(v);
	bool broken = u->HandleBreakdown();
	return {.broken = broken, .status = RustVehicleStatus(u)};
}
static bool OPENTTD_VEHICLE_CALL VehicleSvcWaitingForUnbunching(OpenTTDVehicle *v) noexcept { return VehicleOf(v)->IsWaitingForUnbunching(); }
static void OPENTTD_VEHICLE_CALL VehicleSvcLeaveUnbunchingDepot(OpenTTDVehicle *v) noexcept { VehicleOf(v)->LeaveUnbunchingDepot(); }
static void OPENTTD_VEHICLE_CALL VehicleSvcResetDepotUnbunching(OpenTTDVehicle *v) noexcept { VehicleOf(v)->ResetDepotUnbunching(); }
static void OPENTTD_VEHICLE_CALL VehicleSvcServiceInDepot(OpenTTDVehicle *v) noexcept { VehicleServiceInDepot(VehicleOf(v)); }
static bool OPENTTD_VEHICLE_CALL VehicleSvcNeedsAutomaticServicing(OpenTTDVehicle *v) noexcept { return VehicleOf(v)->NeedsAutomaticServicing(); }
static void OPENTTD_VEHICLE_CALL VehicleSvcEnterDepot(OpenTTDVehicle *v) noexcept { VehicleEnterDepot(VehicleOf(v)); }
static void OPENTTD_VEHICLE_CALL VehicleSvcPathfindingResult(OpenTTDVehicle *v, bool found) noexcept { VehicleOf(v)->HandlePathfindingResult(found); }
static OpenTTDVehicleNewPosition OPENTTD_VEHICLE_CALL VehicleSvcNewPosition(OpenTTDVehicle *v) noexcept
{
	GetNewVehiclePosResult gp = GetNewVehiclePos(VehicleOf(v));
	return {.x = gp.x, .y = gp.y, .tile = gp.new_tile.base()};
}
static uint8_t OPENTTD_VEHICLE_CALL VehicleSvcMoveThenStatus(OpenTTDVehicle *v, int32_t x, int32_t y) noexcept
{
	Vehicle *u = VehicleOf(v);
	u->x_pos = x;
	u->y_pos = y;
	u->UpdatePosition();
	return RustVehicleStatus(u);
}
static void OPENTTD_VEHICLE_CALL VehicleSvcBaseViewport(OpenTTDVehicle *v) noexcept { VehicleOf(v)->Vehicle::UpdateViewport(true); }
static int32_t OPENTTD_VEHICLE_CALL VehicleSvcProperty(OpenTTDVehicle *v, uint8_t property, int32_t fallback) noexcept
{
	return GetVehicleProperty(VehicleOf(v), static_cast<PropertyID>(property), fallback);
}
static uint16_t OPENTTD_VEHICLE_CALL VehicleSvcLengthCallback(OpenTTDVehicle *v) noexcept
{
	Vehicle *u = VehicleOf(v);
	return GetVehicleCallback(CBID_VEHICLE_LENGTH, 0, 0, u->engine_type, u);
}
static void OPENTTD_VEHICLE_CALL VehicleSvcUnknownLengthResult(const OpenTTDEngine *e, uint16_t result) noexcept
{
	ErrorUnknownCallbackResult(EngineOf(e)->GetGRFID(), CBID_VEHICLE_LENGTH, result);
}
static void OPENTTD_VEHICLE_CALL VehicleSvcLengthChanged(OpenTTDVehicle *v) noexcept { VehicleLengthChanged(VehicleOf(v)); }
static bool OPENTTD_VEHICLE_CALL VehicleSvcPlayStartSound(OpenTTDVehicle *v) noexcept { return PlayVehicleSound(VehicleOf(v), VSE_START); }
static void OPENTTD_VEHICLE_CALL VehicleSvcPlaySound(OpenTTDVehicle *v, uint16_t sound) noexcept { SndPlayVehicleFx(static_cast<SoundID>(sound), VehicleOf(v)); }
static void OPENTTD_VEHICLE_CALL VehicleSvcLargeExplosion(OpenTTDVehicle *v) noexcept { CreateEffectVehicleRel(VehicleOf(v), 4, 4, 8, EV_EXPLOSION_LARGE); }
static void OPENTTD_VEHICLE_CALL VehicleSvcOrderFree(OpenTTDVehicle *v) noexcept { VehicleOf(v)->current_order.Free(); }
static void OPENTTD_VEHICLE_CALL VehicleSvcOrderDummy(OpenTTDVehicle *v) noexcept
{
	Vehicle *u = VehicleOf(v);
	u->current_order.MakeDummy();
	SetWindowWidgetDirty(WC_VEHICLE_VIEW, u->index, WID_VV_START_STOP);
}
static void OPENTTD_VEHICLE_CALL VehicleSvcStartStopDirty(OpenTTDVehicle *v) noexcept { SetWindowWidgetDirty(WC_VEHICLE_VIEW, VehicleOf(v)->index, WID_VV_START_STOP); }
static void OPENTTD_VEHICLE_CALL VehicleSvcDestroy(OpenTTDVehicle *v) noexcept { delete VehicleOf(v); }
static void OPENTTD_VEHICLE_CALL VehicleSvcAge(OpenTTDVehicle *v) noexcept { AgeVehicle(VehicleOf(v)); }
static uint8_t OPENTTD_VEHICLE_CALL VehicleSvcEconomyAge(OpenTTDVehicle *v) noexcept
{
	Vehicle *u = VehicleOf(v);
	EconomyAgeVehicle(u);
	return RustVehicleDay(u);
}
static void OPENTTD_VEHICLE_CALL VehicleSvcDecreaseValue(OpenTTDVehicle *v) noexcept { DecreaseVehicleValue(VehicleOf(v)); }
static void OPENTTD_VEHICLE_CALL VehicleSvcCheckBreakdown(OpenTTDVehicle *v) noexcept { CheckVehicleBreakdown(VehicleOf(v)); }
static uint8_t OPENTTD_VEHICLE_CALL VehicleSvcCheckOrders(OpenTTDVehicle *v) noexcept
{
	Vehicle *u = VehicleOf(v);
	CheckOrders(u);
	return u->running_ticks;
}
static void OPENTTD_VEHICLE_CALL VehicleSvcWriteDay(OpenTTDVehicle *v, uint8_t day) noexcept { VehicleOf(v)->day_counter = day; }
static void OPENTTD_VEHICLE_CALL VehicleSvcWriteDirection(OpenTTDVehicle *v, uint8_t direction) noexcept { VehicleOf(v)->direction = static_cast<Direction>(direction); }
static void OPENTTD_VEHICLE_CALL VehicleSvcWriteSpeed(OpenTTDVehicle *v, uint16_t speed) noexcept { VehicleOf(v)->cur_speed = speed; }
static uint16_t OPENTTD_VEHICLE_CALL VehicleSvcLastStation(OpenTTDVehicle *v) noexcept { return VehicleOf(v)->last_station_visited.base(); }
static void OPENTTD_VEHICLE_CALL VehicleSvcWriteLastStation(OpenTTDVehicle *v, uint16_t station) noexcept { VehicleOf(v)->last_station_visited = StationID(station); }
static void OPENTTD_VEHICLE_CALL VehicleSvcWriteDest(OpenTTDVehicle *v, uint32_t tile) noexcept { VehicleOf(v)->dest_tile = TileIndex(tile); }
static void OPENTTD_VEHICLE_CALL VehicleSvcWriteProgress(OpenTTDVehicle *v, uint8_t progress) noexcept { RustWriteVehicleProgress(VehicleOf(v), progress); }

static OpenTTDVehiclePreviousTile OPENTTD_VEHICLE_CALL VehicleSvcPreviousTile(OpenTTDVehicle *v) noexcept
{
	const Vehicle *previous = VehicleOf(v)->Previous();
	return {.tile = previous == nullptr ? 0 : RustVehicleTile(previous), .exists = previous != nullptr};
}
/** `DeleteLastRoadVeh` before the road stop check: unlink the last part and copy the station. */
static void OPENTTD_VEHICLE_CALL VehicleSvcDetachLast(OpenTTDVehicle *previous, OpenTTDVehicle *last, OpenTTDVehicle *first) noexcept
{
	VehicleOf(previous)->SetNext(nullptr);
	VehicleOf(last)->last_station_visited = VehicleOf(first)->last_station_visited;
}
static void OPENTTD_VEHICLE_CALL VehicleSvcWriteMaxSpeed(OpenTTDVehicle *v, uint16_t speed) noexcept { VehicleOf(v)->vcache.cached_max_speed = speed; }

static uint8_t OPENTTD_VEHICLE_CALL VehicleSvcProgress(OpenTTDVehicle *v) noexcept { return RustVehicleProgress(VehicleOf(v)); }
static uint32_t OPENTTD_VEHICLE_CALL VehicleSvcDestTile(OpenTTDVehicle *v) noexcept { return RustVehicleDestTile(VehicleOf(v)); }
static uint8_t OPENTTD_VEHICLE_CALL VehicleSvcOrderType(OpenTTDVehicle *v) noexcept { return RustVehicleOrderType(VehicleOf(v)); }
static uint16_t OPENTTD_VEHICLE_CALL VehicleSvcOrderDestination(OpenTTDVehicle *v) noexcept { return RustVehicleOrderDestination(VehicleOf(v)); }
static uint8_t OPENTTD_VEHICLE_CALL VehicleSvcOrderNonStop(OpenTTDVehicle *v) noexcept { return RustVehicleOrderNonStop(VehicleOf(v)); }
static uint8_t OPENTTD_VEHICLE_CALL VehicleSvcDirection(OpenTTDVehicle *v) noexcept { return RustVehicleDirection(VehicleOf(v)); }
static uint8_t OPENTTD_VEHICLE_CALL VehicleSvcStatus(OpenTTDVehicle *v) noexcept { return RustVehicleStatus(VehicleOf(v)); }
static uint32_t OPENTTD_VEHICLE_CALL VehicleSvcTile(OpenTTDVehicle *v) noexcept { return RustVehicleTile(VehicleOf(v)); }
static OpenTTDVehicle *OPENTTD_VEHICLE_CALL VehicleSvcNext(OpenTTDVehicle *v) noexcept { return RustVehicle(RustVehicleNext(VehicleOf(v))); }
static uint16_t OPENTTD_VEHICLE_CALL VehicleSvcEngineID(OpenTTDVehicle *v) noexcept { return RustVehicleEngineID(VehicleOf(v)); }
static OpenTTDEngineForLength OPENTTD_VEHICLE_CALL VehicleSvcEngineForLength(OpenTTDVehicle *v) noexcept
{
	const Engine *e = VehicleOf(v)->GetEngine();
	return {.engine = RustEngine(e), .grf_version = RustEngineGRFVersion(e)};
}

static void OPENTTD_VEHICLE_CALL VehicleSvcWriteTick(OpenTTDVehicle *v, uint8_t tick) noexcept { RustWriteVehicleTick(VehicleOf(v), tick); }

static uint8_t OPENTTD_VEHICLE_CALL VehicleSvcSubtype(OpenTTDVehicle *v) noexcept { return RustVehicleSubtype(VehicleOf(v)); }
static int32_t OPENTTD_VEHICLE_CALL VehicleSvcX(OpenTTDVehicle *v) noexcept { return RustVehicleX(VehicleOf(v)); }
static int32_t OPENTTD_VEHICLE_CALL VehicleSvcY(OpenTTDVehicle *v) noexcept { return RustVehicleY(VehicleOf(v)); }
static int32_t OPENTTD_VEHICLE_CALL VehicleSvcZ(OpenTTDVehicle *v) noexcept { return RustVehicleZ(VehicleOf(v)); }
static uint8_t OPENTTD_VEHICLE_CALL VehicleSvcTick(OpenTTDVehicle *v) noexcept { return RustVehicleTick(VehicleOf(v)); }
static uint8_t OPENTTD_VEHICLE_CALL VehicleSvcOwner(OpenTTDVehicle *v) noexcept { return RustVehicleOwner(VehicleOf(v)); }
static uint16_t OPENTTD_VEHICLE_CALL VehicleSvcSpeed(OpenTTDVehicle *v) noexcept { return RustVehicleSpeed(VehicleOf(v)); }
static uint8_t OPENTTD_VEHICLE_CALL VehicleSvcSubspeed(OpenTTDVehicle *v) noexcept { return RustVehicleSubspeed(VehicleOf(v)); }
static uint8_t OPENTTD_VEHICLE_CALL VehicleSvcRunning(OpenTTDVehicle *v) noexcept { return RustVehicleRunning(VehicleOf(v)); }
static int32_t OPENTTD_VEHICLE_CALL VehicleSvcOrderTime(OpenTTDVehicle *v) noexcept { return RustVehicleOrderTime(VehicleOf(v)); }
static uint8_t OPENTTD_VEHICLE_CALL VehicleSvcVehicleType(OpenTTDVehicle *v) noexcept { return static_cast<uint8_t>(RustVehicleVehicleType(VehicleOf(v))); }
static void OPENTTD_VEHICLE_CALL VehicleSvcWriteX(OpenTTDVehicle *v, int32_t value) noexcept { VehicleOf(v)->x_pos = value; }
static void OPENTTD_VEHICLE_CALL VehicleSvcWriteY(OpenTTDVehicle *v, int32_t value) noexcept { VehicleOf(v)->y_pos = value; }
static void OPENTTD_VEHICLE_CALL VehicleSvcWriteZ(OpenTTDVehicle *v, int32_t value) noexcept { VehicleOf(v)->z_pos = value; }
static void OPENTTD_VEHICLE_CALL VehicleSvcWriteTile(OpenTTDVehicle *v, uint32_t value) noexcept { VehicleOf(v)->tile = TileIndex(value); }
static void OPENTTD_VEHICLE_CALL VehicleSvcWriteStatus(OpenTTDVehicle *v, uint8_t value) noexcept { VehicleOf(v)->vehstatus = VehStates{value}; }
static void OPENTTD_VEHICLE_CALL VehicleSvcWriteSubspeed(OpenTTDVehicle *v, uint8_t value) noexcept { VehicleOf(v)->subspeed = value; }
static void OPENTTD_VEHICLE_CALL VehicleSvcWriteBreakdownCounter(OpenTTDVehicle *v, uint8_t value) noexcept { VehicleOf(v)->breakdown_ctr = value; }
static void OPENTTD_VEHICLE_CALL VehicleSvcWriteRunning(OpenTTDVehicle *v, uint8_t value) noexcept { VehicleOf(v)->running_ticks = value; }
static void OPENTTD_VEHICLE_CALL VehicleSvcWriteOrderTime(OpenTTDVehicle *v, int32_t value) noexcept { VehicleOf(v)->current_order_time = value; }
static void OPENTTD_VEHICLE_CALL VehicleSvcWriteProfit(OpenTTDVehicle *v, int64_t value) noexcept { VehicleOf(v)->profit_this_year = value; }
static void OPENTTD_VEHICLE_CALL VehicleSvcWriteEconomyService(OpenTTDVehicle *v, int32_t value) noexcept { VehicleOf(v)->date_of_last_service = TimerGameEconomy::Date{value}; }
static void OPENTTD_VEHICLE_CALL VehicleSvcWriteCalendarService(OpenTTDVehicle *v, int32_t value) noexcept { VehicleOf(v)->date_of_last_service_newgrf = TimerGameCalendar::Date{value}; }
static void OPENTTD_VEHICLE_CALL VehicleSvcWriteBreakdowns(OpenTTDVehicle *v, uint8_t value) noexcept { VehicleOf(v)->breakdowns_since_last_service = value; }
static void OPENTTD_VEHICLE_CALL VehicleSvcWriteReliability(OpenTTDVehicle *v, uint16_t value) noexcept { VehicleOf(v)->reliability = value; }
static void OPENTTD_VEHICLE_CALL VehicleSvcWriteCargoAge(OpenTTDVehicle *v, uint16_t value) noexcept { VehicleOf(v)->vcache.cached_cargo_age_period = value; }
static OpenTTDVehicleCoordinates OPENTTD_VEHICLE_CALL VehicleSvcCoordinates(OpenTTDVehicle *v) noexcept
{
	const Vehicle *u = VehicleOf(v);
	return {.x = RustVehicleX(u), .y = RustVehicleY(u), .z = RustVehicleZ(u)};
}
static OpenTTDVehicleXY OPENTTD_VEHICLE_CALL VehicleSvcXy(OpenTTDVehicle *v) noexcept
{
	const Vehicle *u = VehicleOf(v);
	return {.x = RustVehicleX(u), .y = RustVehicleY(u)};
}
static OpenTTDVehicleSpeedInputs OPENTTD_VEHICLE_CALL VehicleSvcSpeedInputs(OpenTTDVehicle *v) noexcept
{
	const Vehicle *u = VehicleOf(v);
	return {.speed = RustVehicleSpeed(u), .max_speed = RustVehicleMaxSpeed(u), .acceleration = RustVehicleAcceleration(u), .status = RustVehicleStatus(u), .subspeed = RustVehicleSubspeed(u)};
}
static OpenTTDVehicleDirectionProgress OPENTTD_VEHICLE_CALL VehicleSvcDirectionProgress(OpenTTDVehicle *v) noexcept
{
	const Vehicle *u = VehicleOf(v);
	return {.direction = RustVehicleDirection(u), .progress = RustVehicleProgress(u)};
}
static OpenTTDVehicleTickStatus OPENTTD_VEHICLE_CALL VehicleSvcTickStatus(OpenTTDVehicle *v) noexcept
{
	const Vehicle *u = VehicleOf(v);
	return {.tick = RustVehicleTick(u), .status = RustVehicleStatus(u)};
}
static OpenTTDVehicleDayCost OPENTTD_VEHICLE_CALL VehicleSvcDayCost(OpenTTDVehicle *v) noexcept
{
	const Vehicle *u = VehicleOf(v);
	return {.profit = RustVehicleProfit(u), .running = RustVehicleRunning(u)};
}
static void OPENTTD_VEHICLE_CALL VehicleSvcMakeDummy(OpenTTDVehicle *v) noexcept { VehicleOf(v)->current_order.MakeDummy(); }
static void OPENTTD_VEHICLE_CALL VehicleSvcLoading(OpenTTDVehicle *v, bool second) noexcept { VehicleOf(v)->HandleLoading(second); }

static OpenTTDVehicleAltitudeInputs OPENTTD_VEHICLE_CALL VehicleSvcAltitudeInputs(OpenTTDVehicle *v) noexcept
{
	const Vehicle *u = VehicleOf(v);
	return {.max_speed = RustVehicleMaxSpeed(u), .direction = RustVehicleDirection(u)};
}

const OpenTTDVehicleServices &GetRustVehicleServices() noexcept
{
	static const OpenTTDVehicleServices services{
		.write_counters = VehicleSvcWriteCounters,
		.handle_breakdown = VehicleSvcHandleBreakdown,
		.waiting_for_unbunching = VehicleSvcWaitingForUnbunching,
		.leave_unbunching_depot = VehicleSvcLeaveUnbunchingDepot,
		.reset_depot_unbunching = VehicleSvcResetDepotUnbunching,
		.service_in_depot = VehicleSvcServiceInDepot,
		.needs_automatic_servicing = VehicleSvcNeedsAutomaticServicing,
		.enter_depot = VehicleSvcEnterDepot,
		.pathfinding_result = VehicleSvcPathfindingResult,
		.new_position = VehicleSvcNewPosition,
		.move_then_status = VehicleSvcMoveThenStatus,
		.base_viewport = VehicleSvcBaseViewport,
		.property = VehicleSvcProperty,
		.length_callback = VehicleSvcLengthCallback,
		.unknown_length_result = VehicleSvcUnknownLengthResult,
		.length_changed = VehicleSvcLengthChanged,
		.play_start_sound = VehicleSvcPlayStartSound,
		.play_sound = VehicleSvcPlaySound,
		.large_explosion = VehicleSvcLargeExplosion,
		.order_free = VehicleSvcOrderFree,
		.order_dummy = VehicleSvcOrderDummy,
		.start_stop_dirty = VehicleSvcStartStopDirty,
		.destroy = VehicleSvcDestroy,
		.age = VehicleSvcAge,
		.economy_age = VehicleSvcEconomyAge,
		.decrease_value = VehicleSvcDecreaseValue,
		.check_breakdown = VehicleSvcCheckBreakdown,
		.check_orders = VehicleSvcCheckOrders,
		.write_day = VehicleSvcWriteDay,
		.write_direction = VehicleSvcWriteDirection,
		.write_speed = VehicleSvcWriteSpeed,
		.last_station = VehicleSvcLastStation,
		.write_last_station = VehicleSvcWriteLastStation,
		.write_dest = VehicleSvcWriteDest,
		.write_progress = VehicleSvcWriteProgress,
		.write_max_speed = VehicleSvcWriteMaxSpeed,
		.previous_tile = VehicleSvcPreviousTile,
		.detach_last = VehicleSvcDetachLast,
		.next = VehicleSvcNext,
		.engine_id = VehicleSvcEngineID,
		.engine_for_length = VehicleSvcEngineForLength,
		.write_tick = VehicleSvcWriteTick,
		.direction = VehicleSvcDirection,
		.status = VehicleSvcStatus,
		.tile = VehicleSvcTile,
		.progress = VehicleSvcProgress,
		.dest_tile = VehicleSvcDestTile,
		.order_type = VehicleSvcOrderType,
		.order_destination = VehicleSvcOrderDestination,
		.order_nonstop = VehicleSvcOrderNonStop,
		.subtype = VehicleSvcSubtype,
		.x = VehicleSvcX,
		.y = VehicleSvcY,
		.z = VehicleSvcZ,
		.tick = VehicleSvcTick,
		.owner = VehicleSvcOwner,
		.speed = VehicleSvcSpeed,
		.subspeed = VehicleSvcSubspeed,
		.running = VehicleSvcRunning,
		.order_time = VehicleSvcOrderTime,
		.vehicle_type = VehicleSvcVehicleType,
		.write_x = VehicleSvcWriteX,
		.write_y = VehicleSvcWriteY,
		.write_z = VehicleSvcWriteZ,
		.write_tile = VehicleSvcWriteTile,
		.write_status = VehicleSvcWriteStatus,
		.write_subspeed = VehicleSvcWriteSubspeed,
		.write_breakdown_counter = VehicleSvcWriteBreakdownCounter,
		.write_running = VehicleSvcWriteRunning,
		.write_order_time = VehicleSvcWriteOrderTime,
		.write_profit = VehicleSvcWriteProfit,
		.write_economy_service = VehicleSvcWriteEconomyService,
		.write_calendar_service = VehicleSvcWriteCalendarService,
		.write_breakdowns = VehicleSvcWriteBreakdowns,
		.write_reliability = VehicleSvcWriteReliability,
		.write_cargo_age = VehicleSvcWriteCargoAge,
		.coordinates = VehicleSvcCoordinates,
		.xy = VehicleSvcXy,
		.speed_inputs = VehicleSvcSpeedInputs,
		.direction_progress = VehicleSvcDirectionProgress,
		.tick_status = VehicleSvcTickStatus,
		.day_cost = VehicleSvcDayCost,
		.make_dummy = VehicleSvcMakeDummy,
		.loading = VehicleSvcLoading,
		.breakdown = VehicleSvcBreakdown,
		.check_orders_only = VehicleSvcCheckOrdersOnly,
		.write_coordinates = VehicleSvcWriteCoordinates,
		.altitude_inputs = VehicleSvcAltitudeInputs,
		.maintenance = &_rust_vehicle_maintenance_services,
	};
	return services;
}


/* New scheduler constants consumed by Rust branches. */
static_assert(VEH_TRAIN == 0 && VEH_ROAD == 1 && VEH_SHIP == 2 && VEH_AIRCRAFT == 3 && VEH_EFFECT == 4 && VEH_DISASTER == 5 && VEH_END == 6 && VEH_INVALID == 255);
static_assert(GVSF_FRONT == 0 && GVSF_ENGINE == 3 && AIR_AIRCRAFT == 2);
static_assert(VehStates{VehState::TrainSlowing}.base() == 16 && Ticks::DAY_TICKS == 74);

static OpenTTDVehiclePoolCursor OPENTTD_VEHICLE_CALL VehicleSchedulerPool() noexcept
{
	return {.slots = _vehicle_pool.data.data(), .slots_len = _vehicle_pool.data.size(), .first_unused = &_vehicle_pool.first_unused};
}
/* Existing virtual facades own the original controller timing scopes. The
 * timing destructor completes before this callback returns to Rust assertions.
 * Only a surviving vehicle fills the post-Tick address bundle; the fill reads
 * type and First() without side effects, and Rust reads it after its assertion. */
static bool OPENTTD_VEHICLE_CALL VehicleSchedulerTick(OpenTTDVehicle *shell, OpenTTDVehiclePostTick *out) noexcept
{
	Vehicle *v = VehicleOf(shell);
	if (!v->Tick()) return false;
	const VehicleType type = v->type; // Fresh after Tick survives.
	if (type > VEH_AIRCRAFT) {
		*out = {.type = type, .period = nullptr, .counter = nullptr, .motion = nullptr, .speed = nullptr, .front_type = nullptr, .subtype = nullptr, .tick = nullptr, .front_status = nullptr, .status = nullptr};
		return true;
	}
	Vehicle *front = RustVehicleFirst(v);
	*out = {.type = type, .period = RustVehicleCargoPeriodAddress(v), .counter = RustVehicleCargoCounterAddress(v), .motion = RustVehicleMotionAddress(v), .speed = RustVehicleSpeedAddress(front), .front_type = RustVehicleVehicleTypeAddress(front), .subtype = RustVehicleSubtypeAddress(v), .tick = RustVehicleTickAddress(v), .front_status = RustVehicleStatusAddress(front), .status = RustVehicleStatusAddress(v)};
	return true;
}
static void OPENTTD_VEHICLE_CALL VehicleSchedulerCalendarDay(OpenTTDVehicle *shell) noexcept { VehicleOf(shell)->OnNewCalendarDay(); }
static void OPENTTD_VEHICLE_CALL VehicleSchedulerEconomyDay(OpenTTDVehicle *shell) noexcept { VehicleOf(shell)->OnNewEconomyDay(); }
static OpenTTDCargoList *OPENTTD_VEHICLE_CALL VehicleSchedulerCargoOwner(OpenTTDVehicle *shell) noexcept { return RustVehicleCargoOwner(VehicleOf(shell)); }
static OpenTTDVehicleDayCallback OPENTTD_VEHICLE_CALL VehicleSchedulerDayCallbackRead(OpenTTDVehicle *shell) noexcept
{
	const Vehicle *v = VehicleOf(shell);
	return {.day = RustVehicleDay(v), .has_engine = RustVehicleHasEngine(v)};
}
static uint16_t OPENTTD_VEHICLE_CALL VehicleSchedulerDayCallback(OpenTTDVehicle *shell) noexcept
{
	Vehicle *v = VehicleOf(shell);
	return GetVehicleCallback(CBID_VEHICLE_32DAY_CALLBACK, 0, 0, v->engine_type, v);
}
static void OPENTTD_VEHICLE_CALL VehicleSchedulerRandomiseCallback(OpenTTDVehicle *shell) noexcept
{
	TriggerVehicleRandomisation(VehicleOf(shell), VehicleRandomTrigger::Callback32);
}
static void OPENTTD_VEHICLE_CALL VehicleSchedulerFirstDirty(OpenTTDVehicle *shell) noexcept { RustVehicleFirst(VehicleOf(shell))->MarkDirty(); }
static void OPENTTD_VEHICLE_CALL VehicleSchedulerUnknownDayCallback(OpenTTDVehicle *shell, uint16_t result) noexcept
{
	ErrorUnknownCallbackResult(VehicleOf(shell)->GetGRFID(), CBID_VEHICLE_32DAY_CALLBACK, result);
}
static void OPENTTD_VEHICLE_CALL VehicleSchedulerRunningSound(OpenTTDVehicle *shell) noexcept { PlayVehicleSound(VehicleOf(shell), VSE_RUNNING); }
static void OPENTTD_VEHICLE_CALL VehicleSchedulerPeriodicSound(OpenTTDVehicle *shell, bool running) noexcept { PlayVehicleSound(VehicleOf(shell), running ? VSE_RUNNING_16 : VSE_STOPPED_16); }
static OpenTTDVehicleStationLookup OPENTTD_VEHICLE_CALL VehicleSchedulerNextStation(uint32_t from) noexcept
{
	auto range = Station::Iterate(from);
	auto it = range.begin();
	if (it == range.end()) return {};
	Station *st = *it;
	return {.station = st->rust_service.get(), .id = st->index.base()};
}
static void OPENTTD_VEHICLE_CALL VehicleSchedulerWithLoadingTimer(const OpenTTDVehicleSchedulerTables *tables, void (OPENTTD_VEHICLE_CALL *load)(const OpenTTDVehicleSchedulerTables *)) noexcept
{
	PerformanceMeasurer framerate(PFE_GL_ECONOMY);
	load(tables);
}
static void OPENTTD_VEHICLE_CALL VehicleSchedulerResetAccumulators() noexcept
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
		.day_callback_read = VehicleSchedulerDayCallbackRead,
		.day_callback = VehicleSchedulerDayCallback,
		.randomise_callback = VehicleSchedulerRandomiseCallback,
		.first_dirty = VehicleSchedulerFirstDirty,
		.unknown_day_callback = VehicleSchedulerUnknownDayCallback,
		.cargo_owner = VehicleSchedulerCargoOwner,
		.running_sound = VehicleSchedulerRunningSound,
		.periodic_sound = VehicleSchedulerPeriodicSound,
		.next_station = VehicleSchedulerNextStation,
		.with_loading_timer = VehicleSchedulerWithLoadingTimer,
		.reset_accumulators = VehicleSchedulerResetAccumulators,
	};
	return services;
}

/* Operation-sized native observations and shared-service adapters. The complete
 * transition, traversal and decision policy lives in vehicle/maintenance.rs. */
static OpenTTDCalendarDate MaintenanceDate(TimerGameCalendar::Date date) noexcept { return {date.base()}; }
static OpenTTDEconomyDate MaintenanceDate(TimerGameEconomy::Date date) noexcept { return {date.base()}; }
static OpenTTDVehicleMaintenanceRenewal OPENTTD_VEHICLE_CALL MaintenanceRenewal(OpenTTDVehicle *handle, const Company *company) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	assert(company == Company::Get(v->owner));
	return {.age = MaintenanceDate(RustVehicleAge(v)), .max_age = MaintenanceDate(RustVehicleMaxAge(v)), .months = company->settings.engine_renew_months, .enabled = company->settings.engine_renew, .kind = RustVehicleVehicleType(v), .subtype = RustVehicleSubtype(v)};
}
static OpenTTDVehicleMaintenanceReset OPENTTD_VEHICLE_CALL MaintenanceReset(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.economy_date = MaintenanceDate(TimerGameEconomy::date), .calendar_date = MaintenanceDate(TimerGameCalendar::date), .reliability = RustEngineReliability(v->GetEngine()), .chance = RustVehicleBreakdownChance(v), .breakdowns = _settings_game.difficulty.vehicle_breakdowns};
}
static void OPENTTD_VEHICLE_CALL MaintenanceResetWrite(OpenTTDVehicle *handle, OpenTTDVehicleMaintenanceResetWrite value) noexcept
{
	Vehicle *v = VehicleOf(handle);
	v->date_of_last_service = TimerGameEconomy::Date{value.economy_date.value};
	v->date_of_last_service_newgrf = TimerGameCalendar::Date{value.calendar_date.value};
	v->breakdowns_since_last_service = value.since;
	v->reliability = value.reliability;
	v->breakdown_chance = value.chance;
}
static bool OPENTTD_VEHICLE_CALL MaintenanceHasEngine(OpenTTDVehicle *handle) noexcept { return RustVehicleHasEngine(VehicleOf(handle)); }
static OpenTTDVehicleMaintenanceInterval OPENTTD_VEHICLE_CALL MaintenanceInterval(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	const Company *company = Company::Get(v->owner);
	return {.company = company, .last_service = MaintenanceDate(RustVehicleLastService(v)), .now = MaintenanceDate(TimerGameEconomy::date), .interval = v->GetServiceInterval(), .reliability = RustVehicleReliability(v), .percent = v->ServiceIntervalIsPercent(), .wallclock = TimerGameEconomy::UsingWallclockUnits(), .no_service = _settings_game.order.no_servicing_if_no_breakdowns, .breakdowns = _settings_game.difficulty.vehicle_breakdowns};
}
static uint16_t OPENTTD_VEHICLE_CALL MaintenanceEngineReliability(OpenTTDVehicle *handle) noexcept { return RustEngineReliability(VehicleOf(handle)->GetEngine()); }
static OpenTTDVehicleMaintenanceBudget OPENTTD_VEHICLE_CALL MaintenanceBudget(const Company *company) noexcept { return {.needed = RustCompanyRenewReserve(company), .available = GetAvailableMoney(company->index).base()}; }
static OpenTTDVehicleMaintenanceReplacement OPENTTD_VEHICLE_CALL MaintenanceReplacement(OpenTTDVehicle *handle, const Company *company) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	bool when_old = false;
	EngineID engine = EngineReplacementForCompany(company, v->engine_type, v->group_id, &when_old);
	return {.engine = engine.base(), .when_old = when_old};
}
static bool OPENTTD_VEHICLE_CALL MaintenanceReplacementAvailable(OpenTTDVehicle *handle, uint16_t engine) noexcept { return Engine::Get(EngineID(engine))->company_avail.Test(VehicleOf(handle)->owner); }
static OpenTTDVehicleMaintenanceReplacementCost OPENTTD_VEHICLE_CALL MaintenanceReplacementCost(const Company *company, uint16_t engine) noexcept { return {.cost = Engine::Get(EngineID(engine))->GetCost().base(), .available = GetAvailableMoney(company->index).base()}; }
static OpenTTDVehicleMaintenanceAutomatic OPENTTD_VEHICLE_CALL MaintenanceAutomatic(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.has_depot_order = v->HasDepotOrder(), .order_type = RustVehicleOrderType(v)};
}
static bool OPENTTD_VEHICLE_CALL MaintenanceDepotService(OpenTTDVehicle *handle) noexcept { return VehicleOf(handle)->current_order.GetDepotOrderType().Test(OrderDepotTypeFlag::Service); }
static OpenTTDVehicleMaintenanceCheckBreakdown OPENTTD_VEHICLE_CALL MaintenanceCheckBreakdown(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.reliability = RustVehicleReliability(v), .decay = RustVehicleReliabilityDecay(v), .no_service = _settings_game.order.no_servicing_if_no_breakdowns, .breakdowns = _settings_game.difficulty.vehicle_breakdowns};
}
static OpenTTDVehicleMaintenanceBreakdownGuard OPENTTD_VEHICLE_CALL MaintenanceBreakdownGuard(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.speed = RustVehicleSpeed(v), .counter = RustVehicleBreakdownCounter(v), .status = RustVehicleStatus(v), .breakdowns = _settings_game.difficulty.vehicle_breakdowns, .menu = _game_mode == GM_MENU};
}
static OpenTTDVehicleMaintenanceBreakdownTrigger OPENTTD_VEHICLE_CALL MaintenanceBreakdownTrigger(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.reliability = RustVehicleReliability(v), .chance = RustVehicleBreakdownChance(v), .kind = RustVehicleVehicleType(v), .breakdowns = _settings_game.difficulty.vehicle_breakdowns};
}
static void OPENTTD_VEHICLE_CALL MaintenanceChanceWrite(OpenTTDVehicle *handle, uint8_t chance) noexcept { VehicleOf(handle)->breakdown_chance = chance; }
static void OPENTTD_VEHICLE_CALL MaintenanceTrigger(OpenTTDVehicle *handle, uint8_t counter, uint8_t delay) noexcept
{
	Vehicle *v = VehicleOf(handle);
	v->breakdown_ctr = counter;
	v->breakdown_delay = delay;
	v->breakdown_chance = 0;
}
static uint8_t OPENTTD_VEHICLE_CALL MaintenanceBreakdownCounter(OpenTTDVehicle *handle) noexcept { return RustVehicleBreakdownCounter(VehicleOf(handle)); }
static OpenTTDVehicleMaintenanceBreakdownProbe OPENTTD_VEHICLE_CALL MaintenanceBreakdownProbe(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.counter = RustVehicleBreakdownCounter(v), .status = RustVehicleStatus(v)};
}
static OpenTTDVehicleMaintenanceBreakdownStart OPENTTD_VEHICLE_CALL MaintenanceBreakdownStart(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.since = RustVehicleBreakdownsSinceService(v), .kind = RustVehicleVehicleType(v)};
}
static void OPENTTD_VEHICLE_CALL MaintenanceBreakdownStartWrite(OpenTTDVehicle *handle, uint8_t counter, uint8_t since) noexcept
{
	Vehicle *v = VehicleOf(handle);
	v->breakdown_ctr = counter;
	v->breakdowns_since_last_service = since;
}
static OpenTTDVehicleMaintenanceBreakdownActive OPENTTD_VEHICLE_CALL MaintenanceBreakdownActive(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.kind = RustVehicleVehicleType(v), .tick = RustVehicleTick(v), .delay = RustVehicleBreakdownDelay(v)};
}
static OpenTTDVehicleMaintenanceBreakdownCountdown OPENTTD_VEHICLE_CALL MaintenanceBreakdownCountdown(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.order_type = RustVehicleOrderType(v), .counter = RustVehicleBreakdownCounter(v)};
}
static OpenTTDVehicleMaintenanceBreakdownSoundState OPENTTD_VEHICLE_CALL MaintenanceBreakdownSoundState(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.kind = RustVehicleVehicleType(v), .landscape = OpenTTDLandscape()};
}
static void OPENTTD_VEHICLE_CALL MaintenanceBreakdownDelayWrite(OpenTTDVehicle *handle, uint8_t delay) noexcept { VehicleOf(handle)->breakdown_delay = delay; }
static bool OPENTTD_VEHICLE_CALL MaintenanceBreakdownSound(OpenTTDVehicle *handle) noexcept { return PlayVehicleSound(VehicleOf(handle), VSE_BREAKDOWN); }
static bool OPENTTD_VEHICLE_CALL MaintenanceSmokeDisabled(OpenTTDVehicle *handle) noexcept { return EngInfo(VehicleOf(handle)->engine_type)->misc_flags.Test(EngineMiscFlag::NoBreakdownSmoke); }
static OpenTTDVehicleMaintenanceBreakdownSmoke OPENTTD_VEHICLE_CALL MaintenanceSmoke(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	EffectVehicle *smoke = CreateEffectVehicleRel(v, 4, 4, 5, EV_BREAKDOWN_SMOKE);
	return {.shell = RustVehicle(smoke), .delay = RustVehicleBreakdownDelay(v)};
}
static void OPENTTD_VEHICLE_CALL MaintenanceSmokeAnimation(OpenTTDVehicle *handle, uint16_t duration) noexcept { EffectVehicle::From(VehicleOf(handle))->SetAnimationState(duration); }
static void OPENTTD_VEHICLE_CALL MaintenanceAircraftBroken(OpenTTDVehicle *handle) noexcept { VehicleOf(handle)->vehstatus.Set(VehState::AircraftBroken); }
static void OPENTTD_VEHICLE_CALL MaintenanceMarkDirty(OpenTTDVehicle *handle) noexcept { VehicleOf(handle)->MarkDirty(); }
static void OPENTTD_VEHICLE_CALL MaintenanceViewDirty(OpenTTDVehicle *handle) noexcept { SetWindowDirty(WC_VEHICLE_VIEW, VehicleOf(handle)->index); }
static void OPENTTD_VEHICLE_CALL MaintenanceDetailsDirty(OpenTTDVehicle *handle) noexcept { SetWindowDirty(WC_VEHICLE_DETAILS, VehicleOf(handle)->index); }
static OpenTTDVehicleMaintenanceAge OPENTTD_VEHICLE_CALL MaintenanceAge(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.age = MaintenanceDate(RustVehicleAge(v)), .max_age = MaintenanceDate(RustVehicleMaxAge(v)), .decay = RustVehicleReliabilityDecay(v)};
}
static void OPENTTD_VEHICLE_CALL MaintenanceAgeWrite(OpenTTDVehicle *handle, OpenTTDCalendarDate age) noexcept { VehicleOf(handle)->age = TimerGameCalendar::Date{age.value}; }
static void OPENTTD_VEHICLE_CALL MaintenanceDecayWrite(OpenTTDVehicle *handle, uint16_t decay) noexcept { VehicleOf(handle)->reliability_spd_dec = decay; }
static OpenTTDVehicleMaintenanceWarning OPENTTD_VEHICLE_CALL MaintenanceWarning(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.enabled = _settings_client.gui.old_vehicle_warn, .has_previous = v->Previous() != nullptr, .local = RustVehicleOwner(v) == RustLocalCompanyID(), .status = RustVehicleStatus(v)};
}
static OpenTTDVehicleMaintenanceWarningCompany OPENTTD_VEHICLE_CALL MaintenanceWarningCompany(OpenTTDVehicle *handle) noexcept
{
	const Company *company = Company::Get(VehicleOf(handle)->owner);
	return {.company = company, .renewal = company->settings.engine_renew};
}
static bool OPENTTD_VEHICLE_CALL MaintenanceEngineAvailableAny(OpenTTDVehicle *handle) noexcept { return VehicleOf(handle)->GetEngine()->company_avail.Any(); }
static bool OPENTTD_VEHICLE_CALL MaintenanceReplacementActive(OpenTTDVehicle *handle, const Company *company) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return EngineHasReplacementForCompany(company, v->engine_type, v->group_id);
}
static void OPENTTD_VEHICLE_CALL MaintenanceOldNews(OpenTTDVehicle *handle, StringID message) noexcept
{
	VehicleID id = VehicleOf(handle)->index;
	AddVehicleAdviceNewsItem(AdviceType::VehicleOld, GetEncodedString(message, id), id);
}
static OpenTTDEconomyDate OPENTTD_VEHICLE_CALL MaintenanceEconomyAge(OpenTTDVehicle *handle) noexcept { return MaintenanceDate(RustVehicleEconomyAge(VehicleOf(handle))); }
static void OPENTTD_VEHICLE_CALL MaintenanceEconomyAgeWrite(OpenTTDVehicle *handle, OpenTTDEconomyDate age) noexcept { VehicleOf(handle)->economy_age = TimerGameEconomy::Date{age.value}; }
static void OPENTTD_VEHICLE_CALL MaintenanceReachedMinAge(OpenTTDVehicle *handle) noexcept { GroupStatistics::VehicleReachedMinAge(VehicleOf(handle)); }
static int64_t OPENTTD_VEHICLE_CALL MaintenanceValue(OpenTTDVehicle *handle) noexcept { return RustVehicleValue(VehicleOf(handle)); }
static void OPENTTD_VEHICLE_CALL MaintenanceValueWrite(OpenTTDVehicle *handle, int64_t value) noexcept { VehicleOf(handle)->value = Money(value); }
static_assert(to_underlying(LandscapeType::Toyland) == 3);
static_assert(EconomyTime::DAYS_IN_ECONOMY_MONTH == 30 && CalendarTime::DAYS_IN_LEAP_YEAR == 366 && NUM_CARGO == 64);
static_assert(SND_0F_BREAKDOWN_ROADVEHICLE == 13 && SND_10_BREAKDOWN_TRAIN_SHIP == 14 && SND_35_BREAKDOWN_ROADVEHICLE_TOYLAND == 53 && SND_3A_BREAKDOWN_TRAIN_SHIP_TOYLAND == 58);
const OpenTTDVehicleMaintenanceServices _rust_vehicle_maintenance_services{
	.shared = &GetRustSharedServices(),
	.transaction = &FleetTransactionServices(),
	.group = &FleetGroupServices(),
	.renewal = MaintenanceRenewal,
	.reset = MaintenanceReset,
	.reset_write = MaintenanceResetWrite,
	.has_engine = MaintenanceHasEngine,
	.interval = MaintenanceInterval,
	.engine_reliability = MaintenanceEngineReliability,
	.budget = MaintenanceBudget,
	.replacement = MaintenanceReplacement,
	.replacement_available = MaintenanceReplacementAvailable,
	.replacement_cost = MaintenanceReplacementCost,
	.automatic = MaintenanceAutomatic,
	.depot_service = MaintenanceDepotService,
	.check_breakdown = MaintenanceCheckBreakdown,
	.breakdown_guard = MaintenanceBreakdownGuard,
	.breakdown_trigger = MaintenanceBreakdownTrigger,
	.chance_write = MaintenanceChanceWrite,
	.trigger = MaintenanceTrigger,
	.breakdown_counter = MaintenanceBreakdownCounter,
	.breakdown_probe = MaintenanceBreakdownProbe,
	.breakdown_start = MaintenanceBreakdownStart,
	.breakdown_start_write = MaintenanceBreakdownStartWrite,
	.breakdown_active = MaintenanceBreakdownActive,
	.breakdown_countdown = MaintenanceBreakdownCountdown,
	.breakdown_sound_state = MaintenanceBreakdownSoundState,
	.breakdown_delay_write = MaintenanceBreakdownDelayWrite,
	.breakdown_sound = MaintenanceBreakdownSound,
	.smoke_disabled = MaintenanceSmokeDisabled,
	.smoke = MaintenanceSmoke,
	.smoke_animation = MaintenanceSmokeAnimation,
	.aircraft_broken = MaintenanceAircraftBroken,
	.mark_dirty = MaintenanceMarkDirty,
	.view_dirty = MaintenanceViewDirty,
	.details_dirty = MaintenanceDetailsDirty,
	.age = MaintenanceAge,
	.age_write = MaintenanceAgeWrite,
	.decay_write = MaintenanceDecayWrite,
	.warning = MaintenanceWarning,
	.warning_company = MaintenanceWarningCompany,
	.engine_available_any = MaintenanceEngineAvailableAny,
	.replacement_active = MaintenanceReplacementActive,
	.old_news = MaintenanceOldNews,
	.economy_age = MaintenanceEconomyAge,
	.economy_age_write = MaintenanceEconomyAgeWrite,
	.reached_min_age = MaintenanceReachedMinAge,
	.value = MaintenanceValue,
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
extern "C" uint64_t openttd_orders_day_counter(void *ctx) noexcept
{
	return RustVehicleDay(static_cast<const Vehicle *>(ctx));
}

extern "C" uint64_t openttd_orders_reliability(void *ctx) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return RustVehicleReliability(v);
}

extern "C" uint64_t openttd_orders_engine_reliability(void *ctx) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return RustEngineReliability(v->GetEngine());
}

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
#endif /* WITH_RUST */
