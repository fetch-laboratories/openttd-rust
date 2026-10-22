/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file vehicle_services.h Game-side handle conversion and GroundVehicle services for Rust ports. */
#ifndef RUST_VEHICLE_SERVICES_H
#define RUST_VEHICLE_SERVICES_H
#include "vehicle_ffi.h"
#include "byte_span_ffi.h"
#include "../ground_vehicle.hpp"
#include "../engine_base.h"
#include "../vehicle_func.h"
#include "../newgrf.h"
#include "../newgrf_engine.h"
#include "../newgrf_commons.h"
#include "services_newgrf.h"

inline uint16_t RustVehicleCallback(CallbackID callback, uint32_t param1, uint32_t param2, EngineID engine, const Vehicle *v, std::span<int32_t> registers = {}) noexcept { return GetVehicleCallback(callback, param1, param2, engine, v, registers); }
uint16_t RustVehicleStartstopCallback(Vehicle *v, int32_t *reg) noexcept;
bool RustTrainIsEngine(const Vehicle *v) noexcept;
uint32_t RustTrainCachedPower(const Vehicle *v) noexcept;
uint8_t RustVehicleGRFVersion(const Vehicle *v) noexcept;
uint32_t RustVehicleGRFString(Vehicle *v, GRFStringID string) noexcept;
void RustVehicleFlipStopped(Vehicle *v) noexcept;


OpenTTDByteSpan RustVehicleName(void *shell) noexcept;
void RustVehicleClearName(void *shell) noexcept;
void RustVehicleSetName(void *shell, OpenTTDByteSpan name) noexcept;

/** The Rust handle of a live vehicle; null stays null. */
inline OpenTTDVehicle *RustVehicle(const Vehicle *v) noexcept { return reinterpret_cast<OpenTTDVehicle *>(const_cast<Vehicle *>(v)); }
/** The vehicle behind a Rust handle. */
inline Vehicle *VehicleOf(OpenTTDVehicle *v) noexcept { return reinterpret_cast<Vehicle *>(v); }

inline std::optional<bool> RustVehicleBuildProbability(Vehicle *v, EngineID engine, BuildProbabilityType type) noexcept { return TestVehicleBuildProbability(v, engine, type); }
inline bool RustVehicleHasReverseProbability(void *shell) noexcept { Vehicle *v = static_cast<Vehicle *>(shell); return RustVehicleBuildProbability(v, v->engine_type, BuildProbabilityType::Reversed).has_value(); }
inline void RustGroundSetFirstEngine(GroundVehicleCache *cache, EngineID engine) noexcept { cache->first_engine = engine; }
template <class T> inline void RustVehicleUpdatePosition(T *v) noexcept { v->UpdatePosition(); }
inline void RustVehicleSetRandomBits(Vehicle *v, uint16_t bits) noexcept { v->random_bits = bits; }
inline bool RustVehicleCanAllocate(size_t count) noexcept { return Vehicle::CanAllocateItem(count); }
inline VehiclesOnTile RustVehiclesOnTile(TileIndex tile) noexcept { return VehiclesOnTile(tile); }

/** Callback may reenter Rust but must preserve the current native shell. Advance
 * after it returns, matching the original live hash traversal. No allocation. */
inline void RustVisitVehiclesOnTile(uint32_t tile, void *context, void (*visit)(void *, OpenTTDVehicle *)) noexcept
{
	for (Vehicle *v : RustVehiclesOnTile(TileIndex(tile))) visit(context, RustVehicle(v));
}

/** Shared ascending vehicle pool traversal for lifecycle and fleet ports. */
inline OpenTTDVehicle *RustNextVehicle(uint32_t from) noexcept
{
	for (Vehicle *v : Vehicle::Iterate(from)) return RustVehicle(v);
	return nullptr;
}

/** Shared native field definitions, also used by bundled observations. */
template <class T> inline auto RustVehicleFirst(T *v) noexcept { return v->First(); }
/** Canonical typed ground-vehicle cargo update for command/list adapters. */
template <class T> inline void RustGroundCargoChanged(T *v) noexcept { v->CargoChanged(); }
template <class T> inline auto RustVehicleLast(T *v) noexcept { return v->Last(); }
/** Signature adapter for existing opaque fleet and orders callbacks. */
inline void *RustVehicleFirstOpaque(void *v) noexcept { return RustVehicleFirst(static_cast<const Vehicle *>(v)); }
inline void OPENTTD_VEHICLE_CALL RustVehicleSetDestTile(OpenTTDVehicle *v, uint32_t tile) noexcept { VehicleOf(v)->SetDestTile(TileIndex(tile)); }
inline void RustVehicleLeaveStation(OpenTTDVehicle *v) noexcept { VehicleOf(v)->LeaveStation(); }
inline uint32_t RustVehicleOrderStationLocation(OpenTTDVehicle *v, uint16_t station) noexcept { return VehicleOf(v)->GetOrderStationLocation(StationID(station)).base(); }
inline uint64_t RustVehicleLegacyOrderStationLocation(void *v, uint16_t station) noexcept { return RustVehicleOrderStationLocation(RustVehicle(static_cast<Vehicle *>(v)), station); }
template <class T> inline auto RustVehicleNextVehicle(const T *v) noexcept { return v->GetNextVehicle(); }
inline OpenTTDVehicle *RustVehicleNextArticulated(OpenTTDVehicle *v) noexcept { return RustVehicle(VehicleOf(v)->GetNextArticulatedPart()); }
inline Vehicle *RustVehicleNext(const Vehicle *v) noexcept { return v->Next(); }
/** Chain, subtype and unit leaves shared by consist transactions and controllers. */
inline Vehicle *RustVehiclePrevious(const Vehicle *v) noexcept { return v->Previous(); }
inline void RustVehicleSetNext(Vehicle *v, Vehicle *next) noexcept { v->SetNext(next); }
inline void RustOpaqueVehicleSetNext(void *v, void *next) noexcept { RustVehicleSetNext(static_cast<Vehicle *>(v), static_cast<Vehicle *>(next)); }
inline Vehicle *RustVehicleLastEnginePart(Vehicle *v) noexcept { return v->GetLastEnginePart(); }
inline void RustVehicleSetSubtype(Vehicle *v, uint8_t subtype) noexcept { v->subtype = subtype; }
inline void RustVehicleInvalidateNewGRFCache(Vehicle *v) noexcept { v->InvalidateNewGRFCache(); }
inline uint16_t RustVehicleUnitNumber(const Vehicle *v) noexcept { return v->unitnumber; }
inline void RustVehicleReleaseUnitNumber(Vehicle *v) noexcept { v->ReleaseUnitNumber(); }
inline uint16_t RustGroundFirstEngine(const Vehicle *v) noexcept { return v->GetGroundVehicleCache()->first_engine.base(); }
#ifdef WITH_RUST
inline OpenTTDCargoList *RustVehicleCargoOwner(Vehicle *v) noexcept { return v->cargo.RustOwner(); }
#endif
inline bool RustVehicleHasEngine(const Vehicle *v) noexcept { return v->HasEngineType(); }
inline uint8_t RustVehicleDay(const Vehicle *v) noexcept { return v->day_counter; }
inline uint32_t RustVehicleTile(const Vehicle *v) noexcept { return v->tile.base(); }
inline uint32_t RustVehicleDestTile(const Vehicle *v) noexcept { return v->dest_tile.base(); }
inline OrderType RustVehicleOrderType(const Vehicle *v) noexcept { return v->current_order.GetType(); }
inline uint16_t RustVehicleOrderDestination(const Vehicle *v) noexcept { return v->current_order.GetDestination().base(); }
inline uint16_t RustVehicleOrderMaxSpeed(const Vehicle *v) noexcept { return v->current_order.GetMaxSpeed(); }
template <class T> inline int RustVehicleDisplayMaxSpeed(const T *v) noexcept { return v->GetDisplayMaxSpeed(); }
inline uint8_t RustVehicleOrderNonStop(const Vehicle *v) noexcept { return v->current_order.GetNonStopType().base(); }
bool RustVehicleInDepot(const Vehicle *v) noexcept;
bool RustTrainFreeWagon(const Vehicle *v) noexcept;
Vehicle *RustTrainFirstEnginePart(Vehicle *v) noexcept;
bool RustTrainMultiheaded(const Vehicle *v) noexcept;
Vehicle *RustTrainOtherMultihead(const Vehicle *v) noexcept;
bool RustVehiclePrimary(const Vehicle *v) noexcept;
inline uint8_t RustVehicleOwner(const Vehicle *v) noexcept { return v->owner.base(); }
inline const Direction *RustVehicleDirectionAddress(const Vehicle *v) noexcept { return &v->direction; }
inline Direction RustVehicleDirection(const Vehicle *v) noexcept { return *RustVehicleDirectionAddress(v); }
inline uint8_t RustVehicleProgress(const Vehicle *v) noexcept { return v->progress; }
inline void RustWriteVehicleProgress(Vehicle *v, uint8_t progress) noexcept { v->progress = progress; }
inline const uint8_t *RustVehicleTickAddress(const Vehicle *v) noexcept { return &v->tick_counter; }
inline uint8_t RustVehicleTick(const Vehicle *v) noexcept { return *RustVehicleTickAddress(v); }
inline const uint8_t *RustVehicleStatusAddress(const Vehicle *v) noexcept { return v->vehstatus.base_address(); }
inline uint8_t RustVehicleStatus(const Vehicle *v) noexcept { return *RustVehicleStatusAddress(v); }
inline void RustWriteVehicleTick(Vehicle *v, uint8_t tick) noexcept { v->tick_counter = tick; }
inline uint8_t RustVehicleBreakdownDelay(const Vehicle *v) noexcept { return v->breakdown_delay; }
inline uint8_t RustVehicleBreakdownChance(const Vehicle *v) noexcept { return v->breakdown_chance; }
inline uint8_t RustVehicleBreakdownsSinceService(const Vehicle *v) noexcept { return v->breakdowns_since_last_service; }
inline uint16_t RustVehicleReliability(const Vehicle *v) noexcept { return v->reliability; }
inline uint16_t RustVehicleReliabilityDecay(const Vehicle *v) noexcept { return v->reliability_spd_dec; }
inline TimerGameCalendar::Date RustVehicleAge(const Vehicle *v) noexcept { return v->age; }
inline TimerGameCalendar::Date RustVehicleMaxAge(const Vehicle *v) noexcept { return v->max_age; }
inline TimerGameEconomy::Date RustVehicleEconomyAge(const Vehicle *v) noexcept { return v->economy_age; }
inline TimerGameEconomy::Date RustVehicleLastService(const Vehicle *v) noexcept { return v->date_of_last_service; }
inline int32_t RustVehicleLastServiceOf(OpenTTDVehicle *v) noexcept { return RustVehicleLastService(VehicleOf(v)).base(); }
inline uint8_t RustVehicleBreakdownCounter(const Vehicle *v) noexcept { return v->breakdown_ctr; }
inline int64_t RustVehicleValue(const Vehicle *v) noexcept { return v->value.base(); }
inline uint16_t RustEngineReliability(const Engine *e) noexcept { return e->reliability; }
inline uint16_t RustVehicleEngineID(const Vehicle *v) noexcept { return v->engine_type.base(); }
inline const OpenTTDEngine *RustEngine(const Engine *e) noexcept { return reinterpret_cast<const OpenTTDEngine *>(e); }
inline const Engine *EngineOf(const OpenTTDEngine *e) noexcept { return reinterpret_cast<const Engine *>(e); }
inline const OpenTTDEngine *RustVehicleGetEngine(OpenTTDVehicle *v) noexcept { return RustEngine(VehicleOf(v)->GetEngine()); }
uint16_t RustVehicleRefitCap(OpenTTDVehicle *v) noexcept;
uint16_t RustVehicleCargoCap(OpenTTDVehicle *v) noexcept;
void RustVehicleSetRefitCap(OpenTTDVehicle *v, uint16_t cap) noexcept;
void RustVehicleSetCargoCap(OpenTTDVehicle *v, uint16_t cap) noexcept;
void RustVehicleSetCargoKind(OpenTTDVehicle *v, uint8_t cargo, uint8_t subtype) noexcept;
void RustVehicleSetCargoType(OpenTTDVehicle *v, uint8_t cargo) noexcept;
void RustVehicleSetCargoSubtype(OpenTTDVehicle *v, uint8_t subtype) noexcept;
void RustVehicleInvalidateChain(OpenTTDVehicle *v) noexcept;
ExpensesType RustVehicleExpense(OpenTTDVehicle *v, bool income) noexcept;
template <class T>
inline uint8_t RustGroundCachedLength(const T *v) noexcept { return v->gcache.cached_veh_length; }
template <class T>
inline int32_t RustGroundAcceleration(const T *v) noexcept { return v->GetAcceleration(); }
template <class T>
inline bool RustGroundHasArticulated(const T *v) noexcept { return v->HasArticulatedPart(); }

inline const uint8_t *RustVehicleSubtypeAddress(const Vehicle *v) noexcept { return &v->subtype; }
inline uint8_t RustVehicleSubtype(const Vehicle *v) noexcept { return *RustVehicleSubtypeAddress(v); }
inline const int32_t *RustVehicleXAddress(const Vehicle *v) noexcept { return &v->x_pos; }
inline int32_t RustVehicleX(const Vehicle *v) noexcept { return *RustVehicleXAddress(v); }
inline const int32_t *RustVehicleYAddress(const Vehicle *v) noexcept { return &v->y_pos; }
inline int32_t RustVehicleY(const Vehicle *v) noexcept { return *RustVehicleYAddress(v); }
inline const int32_t *RustVehicleZAddress(const Vehicle *v) noexcept { return &v->z_pos; }
inline int32_t RustVehicleZ(const Vehicle *v) noexcept { return *RustVehicleZAddress(v); }
inline const uint16_t *RustVehicleSpeedAddress(const Vehicle *v) noexcept { return &v->cur_speed; }
inline uint16_t RustVehicleSpeed(const Vehicle *v) noexcept { return *RustVehicleSpeedAddress(v); }
inline uint8_t RustVehicleSubspeed(const Vehicle *v) noexcept { return v->subspeed; }
inline uint8_t RustVehicleAcceleration(const Vehicle *v) noexcept { return v->acceleration; }
inline uint16_t RustVehicleMaxSpeed(const Vehicle *v) noexcept { return v->vcache.cached_max_speed; }
inline const uint8_t *RustVehicleRunningAddress(const Vehicle *v) noexcept { return &v->running_ticks; }
inline uint8_t RustVehicleRunning(const Vehicle *v) noexcept { return *RustVehicleRunningAddress(v); }
inline const int32_t *RustVehicleOrderTimeAddress(const Vehicle *v) noexcept { return &v->current_order_time; }
inline int32_t RustVehicleOrderTime(const Vehicle *v) noexcept { return *RustVehicleOrderTimeAddress(v); }
inline int64_t RustVehicleProfit(const Vehicle *v) noexcept { return v->profit_this_year; }
inline const VehicleType *RustVehicleVehicleTypeAddress(const Vehicle *v) noexcept { return &v->type; }
inline VehicleType RustVehicleVehicleType(const Vehicle *v) noexcept { return *RustVehicleVehicleTypeAddress(v); }

inline const uint16_t *RustVehicleCargoPeriodAddress(const Vehicle *v) noexcept { return &v->vcache.cached_cargo_age_period; }
inline uint16_t *RustVehicleCargoCounterAddress(Vehicle *v) noexcept { return &v->cargo_age_counter; }
inline uint32_t *RustVehicleMotionAddress(Vehicle *v) noexcept { return &v->motion_counter; }

/** Shared legacy opaque-owner adapter; the actual typed read is defined once. */
template <typename Result, auto Read>
static Result RustOpaqueVehicleRead(void *shell) noexcept { return Read(static_cast<Vehicle *>(shell)); }
template <typename Result, auto Read>
static Result RustOpaqueHandleRead(void *shell) noexcept { return Read(RustVehicle(static_cast<Vehicle *>(shell))); }
/** Typed legacy-context adapter; the native operation remains defined once. */
template <auto Operation, typename... Args>
static auto RustOpaqueVehicleInvoke(void *shell, Args... args) noexcept { return Operation(static_cast<Vehicle *>(shell), args...); }
/** Legacy orders width/handle adapter for a shared map read at a vehicle's tile. */
template <typename Result, auto Read>
static Result RustOpaqueVehicleTileRead(void *shell) noexcept { return Read(RustVehicleTile(static_cast<const Vehicle *>(shell))); }
inline bool RustVehicleOldEnough(const Vehicle *v) noexcept { return RustVehicleEconomyAge(v) > VEHICLE_PROFIT_MIN_AGE; }

/** Legacy indexed callback adapter; canonical read stays shared, owner resolves once. */
template <typename Result, auto Read>
static Result RustIndexedVehicleRead(uint32_t id) noexcept { return Read(Vehicle::Get(VehicleID(id))); }

/** Base vehicle cargo and articulated fields shared by loading and link refresh. */
inline uint8_t RustVehicleCargoType(OpenTTDVehicle *handle) noexcept { return VehicleOf(handle)->cargo_type; }
inline uint8_t RustVehicleOwnerOf(OpenTTDVehicle *handle) noexcept { return RustVehicleOwner(VehicleOf(handle)); }
inline bool RustVehicleHasArticulated(OpenTTDVehicle *handle) noexcept { return RustGroundHasArticulated(VehicleOf(handle)); }

/** GRF version of a live engine; the original default-engine value is zero. */
inline uint8_t RustEngineGRFVersion(const Engine *engine) noexcept
{
	const GRFFile *grf = engine->GetGRF();
	return grf == nullptr ? uint8_t(0) : grf->grf_version;
}

template <class T>
inline OpenTTDGroundMoved OPENTTD_VEHICLE_CALL GroundSvcMoveIncline(OpenTTDVehicle *v, int32_t x, int32_t y, bool new_tile, bool update_delta) noexcept
{
	T *u = T::From(VehicleOf(v));
	u->x_pos = x;
	u->y_pos = y;
	RustVehicleUpdatePosition(u);
	int old_z = u->UpdateInclination(new_tile, update_delta);
	return {.old_z = old_z, .z = RustVehicleZ(u), .cur_speed = RustVehicleSpeed(u), .max_track_speed = u->gcache.cached_max_track_speed};
}
template <class T>
inline OpenTTDGroundViewportNext OPENTTD_VEHICLE_CALL GroundSvcUpdateViewportThenNext(OpenTTDVehicle *part, OpenTTDVehicle *front) noexcept
{
	T *u = T::From(VehicleOf(part));
	/* UpdateViewport may reenter Rust; no Rust borrow survives this call. */
	u->UpdateViewport(false, false);
	Vehicle *next = RustVehicleNext(u);
	/* Only the final iteration consumes front progress, after every viewport. */
	return {.next = RustVehicle(next), .final_progress = next == nullptr ? RustVehicleProgress(VehicleOf(front)) : uint8_t(0)};
}
template <class T>
inline void OPENTTD_VEHICLE_CALL GroundSvcTurn(OpenTTDVehicle *v, uint8_t direction) noexcept
{
	T *u = T::From(VehicleOf(v));
	u->direction = static_cast<Direction>(direction);
	u->UpdateViewport(true, true);
}
template <class T>
inline OpenTTDGroundSpeed OPENTTD_VEHICLE_CALL GroundSvcDoUpdateSpeed(OpenTTDVehicle *v, uint32_t accel, int32_t min_speed, int32_t max_speed) noexcept
{
	T *u = T::From(VehicleOf(v));
	int distance = u->RustDoUpdateSpeed(accel, min_speed, max_speed);
	return {.distance = distance, .cur_speed = RustVehicleSpeed(u)};
}
template <class T>
inline OpenTTDGroundLastSpeed OPENTTD_VEHICLE_CALL GroundSvcSetLastSpeed(OpenTTDVehicle *v) noexcept
{
	T *u = T::From(VehicleOf(v));
	u->SetLastSpeed();
	return {.status = RustVehicleStatus(u)};
}
template <class T>
inline int32_t OPENTTD_VEHICLE_CALL GroundSvcAcceleration(OpenTTDVehicle *v) noexcept
{
	return RustGroundAcceleration(T::From(VehicleOf(v)));
}
template <class T>
inline OpenTTDGroundCrash OPENTTD_VEHICLE_CALL GroundSvcCrash(OpenTTDVehicle *v, bool flooded) noexcept
{
	T *u = T::From(VehicleOf(v));
	uint victims = u->T::GroundVehicleBase::Crash(flooded);
	return {.victims = victims, .front = u->IsFrontEngine()};
}
template <class T>
inline void OPENTTD_VEHICLE_CALL GroundSvcFirstCargoChanged(OpenTTDVehicle *v) noexcept
{
	RustGroundCargoChanged(RustVehicleFirst(T::From(VehicleOf(v))));
}
template <class T>
inline void OPENTTD_VEHICLE_CALL GroundSvcGoToDepotService(OpenTTDVehicle *v, uint16_t depot) noexcept
{
	T *u = T::From(VehicleOf(v));
	SetBit(u->gv_flags, GVF_SUPPRESS_IMPLICIT_ORDERS);
	u->current_order.MakeGoToDepot(DepotID(depot), OrderDepotTypeFlag::Service);
}
template <class T>
inline void OPENTTD_VEHICLE_CALL GroundSvcWriteFirstEngine(OpenTTDVehicle *v, uint16_t engine) noexcept
{
	RustGroundSetFirstEngine(&T::From(VehicleOf(v))->gcache, EngineID(engine));
}
template <class T>
inline void OPENTTD_VEHICLE_CALL GroundSvcWriteLength(OpenTTDVehicle *part, uint8_t length, OpenTTDVehicle *front, uint16_t total_length) noexcept
{
	T *u = T::From(VehicleOf(part));
	u->gcache.cached_veh_length = length;
	T::From(VehicleOf(front))->gcache.cached_total_length = total_length;
	u->UpdateVisualEffect();
}

template <class T>
/** The head of `RoadVehUpdateCache`. */
inline void OPENTTD_VEHICLE_CALL GroundSvcCacheBegin(OpenTTDVehicle *v) noexcept
{
	T *u = T::From(VehicleOf(v));
	u->InvalidateNewGRFCacheOfChain();
	u->gcache.cached_total_length = 0;
}

template <class T>
inline uint8_t OPENTTD_VEHICLE_CALL GroundSvcCachedLength(OpenTTDVehicle *v) noexcept
{
	return RustGroundCachedLength(T::From(VehicleOf(v)));
}

template <class T>
inline bool OPENTTD_VEHICLE_CALL GroundSvcHasArticulated(OpenTTDVehicle *v) noexcept
{
	return RustGroundHasArticulated(T::From(VehicleOf(v)));
}

/** One immutable process-wide GroundVehicle table per vehicle type. */
template <class T>
inline constinit const OpenTTDGroundServices _rust_ground_services{
	.cache_begin = GroundSvcCacheBegin<T>,
	.move_incline = GroundSvcMoveIncline<T>,
	.update_viewport_then_next = GroundSvcUpdateViewportThenNext<T>,
	.turn = GroundSvcTurn<T>,
	.do_update_speed = GroundSvcDoUpdateSpeed<T>,
	.set_last_speed = GroundSvcSetLastSpeed<T>,
	.acceleration = GroundSvcAcceleration<T>,
	.crash = GroundSvcCrash<T>,
	.first_cargo_changed = GroundSvcFirstCargoChanged<T>,
	.go_to_depot_service = GroundSvcGoToDepotService<T>,
	.write_first_engine = GroundSvcWriteFirstEngine<T>,
	.write_length = GroundSvcWriteLength<T>,
	.cached_length = GroundSvcCachedLength<T>,
	.has_articulated = GroundSvcHasArticulated<T>,
};

template <class T>
const OpenTTDGroundServices &GetRustGroundServices() noexcept
{
	return _rust_ground_services<T>;
}
#endif /* RUST_VEHICLE_SERVICES_H */
