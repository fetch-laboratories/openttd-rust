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
#include "services_viewport.h"
#include "byte_span_ffi.h"
#include "../ground_vehicle.hpp"
#include "../engine_base.h"
#include "../vehicle_func.h"
#include "../newgrf.h"
#include "../newgrf_engine.h"
#include "../newgrf_commons.h"
#include "services_newgrf.h"
#include "../window_func.h"
#include "../texteff.hpp"

struct OpenTTDShipState;
struct OpenTTDAircraftState;
OpenTTDAircraftState *RustVehicleAircraftOwner(const Vehicle *v) noexcept;
OpenTTDShipState *RustVehicleShipOwner(const Vehicle *v) noexcept;

OpenTTDTrainHandle RustVehicleTrain(OpenTTDVehicle *v) noexcept;
bool RustTrainIsEngine(const Vehicle *v) noexcept;
Train *RustTrainGetIfValid(uint32_t id) noexcept;
/** Strict Ship::Get lookup, preserving source pool assertions. */
OpenTTDVehicle *RustShipGet(uint32_t id) noexcept;
uint32_t RustTrainCachedPower(const Vehicle *v) noexcept;
uint32_t RustTrainCachedWeight(const Vehicle *v) noexcept;
uint64_t RustTrainRailTypes(const Vehicle *v) noexcept;
uint8_t RustVehicleGRFVersion(const Vehicle *v) noexcept;
inline uint32_t RustVehicleGRFID(const Vehicle *v) noexcept { return v->GetGRFID(); }
/** Fixed callback-error adapter for a live native vehicle shell. */
template <CallbackID Callback>
inline void RustVehicleCallbackError(void *shell, uint16_t result) noexcept { RustUnknownCallbackResult(RustVehicleGRFID(static_cast<const Vehicle *>(shell)), Callback, result); }
uint32_t RustVehicleGRFString(Vehicle *v, uint32_t string) noexcept;
void RustVehicleFlipStopped(Vehicle *v) noexcept;


OpenTTDByteSpan RustVehicleName(void *shell) noexcept;
void RustVehicleClearName(void *shell) noexcept;
void RustVehicleSetName(void *shell, OpenTTDByteSpan name) noexcept;

/** The Rust handle of a live vehicle; null stays null. */
inline OpenTTDVehicle *RustVehicle(const Vehicle *v) noexcept { return reinterpret_cast<OpenTTDVehicle *>(const_cast<Vehicle *>(v)); }
/** The vehicle behind a Rust handle. */
inline Vehicle *VehicleOf(OpenTTDVehicle *v) noexcept { return reinterpret_cast<Vehicle *>(v); }
inline const Vehicle *VehicleOf(const OpenTTDVehicle *v) noexcept { return reinterpret_cast<const Vehicle *>(v); }

inline void RustGroundSetFirstEngine(GroundVehicleCache *cache, EngineID engine) noexcept { cache->first_engine = engine; }
inline void RustVehicleSetRandomBits(Vehicle *v, uint16_t bits) noexcept { v->random_bits = bits; }
inline uint16_t RustVehicleLastLoadingStation(OpenTTDVehicle *v) noexcept { return VehicleOf(v)->last_loading_station.base(); }
inline uint16_t RustVehicleLastStationVisited(OpenTTDVehicle *v) noexcept { return VehicleOf(v)->last_station_visited.base(); }
inline uint16_t RustVehicleLoadUnloadTicks(OpenTTDVehicle *v) noexcept { return VehicleOf(v)->load_unload_ticks; }
inline void RustVehicleSetLastLoadingStation(OpenTTDVehicle *v, uint16_t station) noexcept { VehicleOf(v)->last_loading_station = StationID(station); }
inline void RustVehicleSetLastLoadingTick(OpenTTDVehicle *v, uint64_t tick) noexcept { VehicleOf(v)->last_loading_tick = tick; }
inline void RustVehicleDirtyDepot(const Vehicle *v) noexcept { SetWindowDirty(WC_VEHICLE_DEPOT, v->tile); }
inline void RustVehicleDirtyDepotOpaque(void *v) noexcept { RustVehicleDirtyDepot(static_cast<Vehicle *>(v)); }
inline void RustVehicleInvalidateDepotTile(TileIndex tile) noexcept { InvalidateWindowData(WC_VEHICLE_DEPOT, tile); }
inline void RustVehicleInvalidateDepot(const Vehicle *v) noexcept { RustVehicleInvalidateDepotTile(v->tile); }
template <class T> inline void RustVehicleUpdateViewport(T *v, bool update_delta, bool force) noexcept { v->UpdateViewport(update_delta, force); }
template <typename Handle> inline void RustVehicleUpdateDelta(Handle handle) noexcept
{
	Vehicle *v;
	if constexpr (std::is_pointer_v<Handle>) { v = reinterpret_cast<Vehicle *>(handle); }
	else if constexpr (requires { handle.handle; }) { v = VehicleOf(handle.handle); }
	else { v = static_cast<Vehicle *>(handle.shell); }
	v->UpdateDeltaXY();
}
inline void RustVehiclePlayLeaveSound(OpenTTDVehicle *v) noexcept { VehicleOf(v)->PlayLeaveStationSound(); }
inline void RustVehicleHideFilling(OpenTTDVehicle *v) noexcept { HideFillingPercent(&VehicleOf(v)->fill_percent_te_id); }
inline void RustVehicleInvalidateView(VehicleID id) noexcept { InvalidateWindowData(WC_VEHICLE_VIEW, id); }
inline void RustVehicleVisitInvalidateView(OpenTTDVehicle *v) noexcept { RustVehicleInvalidateView(VehicleOf(v)->index); }
inline void RustVehicleSetTripOccupancy(OpenTTDVehicle *v, uint8_t occupancy) noexcept { VehicleOf(v)->trip_occupancy = occupancy; }
inline bool RustVehicleCanAllocate(size_t count) noexcept { return Vehicle::CanAllocateItem(count); }
inline void RustVehicleShowNewGRFError(EngineID engine, StringID first, StringID second, GRFBug bug, bool critical) noexcept { ShowNewGrfVehicleError(engine, first, second, bug, critical); }

/** Shared ascending vehicle pool traversal for lifecycle and fleet ports. */
inline OpenTTDVehicle *RustNextVehicle(uint32_t from) noexcept
{
	for (Vehicle *v : Vehicle::Iterate(from)) return RustVehicle(v);
	return nullptr;
}

/** Shared native field definitions, also used by bundled observations. */
/* Typed signature adapter for synchronous pathfinder vehicle contexts. */
template <typename Result, typename Shell, auto Query>
inline Result RustVehicleContextQuery(const void *context) noexcept { return Query(RustVehicle(static_cast<const Shell *>(context))); }
template <class T> inline auto RustVehicleFirst(T *v) noexcept { return v->First(); }
template <class T> inline auto RustVehicleLast(T *v) noexcept { return v->Last(); }
/** Signature adapter for existing opaque fleet and orders callbacks. */
inline void *RustVehicleFirstOpaque(void *v) noexcept { return RustVehicleFirst(static_cast<const Vehicle *>(v)); }
inline void OPENTTD_RUST_CALL RustVehicleSetDestTile(OpenTTDVehicle *v, uint32_t tile) noexcept { VehicleOf(v)->SetDestTile(TileIndex(tile)); }
inline uint32_t RustVehicleOrderStationLocation(OpenTTDVehicle *v, uint16_t station) noexcept { return VehicleOf(v)->GetOrderStationLocation(StationID(station)).base(); }
inline uint64_t RustVehicleLegacyOrderStationLocation(void *v, uint16_t station) noexcept { return RustVehicleOrderStationLocation(RustVehicle(static_cast<Vehicle *>(v)), station); }
template <class T> inline auto RustVehicleNextVehicle(const T *v) noexcept { return v->GetNextVehicle(); }
inline OpenTTDVehicle *RustVehicleNextArticulated(OpenTTDVehicle *v) noexcept { return RustVehicle(VehicleOf(v)->GetNextArticulatedPart()); }
inline Vehicle *RustVehicleNext(const Vehicle *v) noexcept { return v->Next(); }
/** Chain, subtype and unit leaves shared by consist transactions and controllers. */
inline Vehicle *RustVehiclePrevious(const Vehicle *v) noexcept { return v->Previous(); }
inline void RustVehicleSetNext(Vehicle *v, Vehicle *next) noexcept { v->SetNext(next); }
inline void RustTypedVehicleSetNext(OpenTTDVehicle *v, OpenTTDVehicle *next) noexcept { RustVehicleSetNext(VehicleOf(v), VehicleOf(next)); }
inline uint8_t RustVehicleDirectionTowards(OpenTTDVehicle *v, int32_t x, int32_t y) noexcept { return GetDirectionTowards(VehicleOf(v), x, y); }
inline void RustOpaqueVehicleSetNext(void *v, void *next) noexcept { RustTypedVehicleSetNext(RustVehicle(static_cast<Vehicle *>(v)), RustVehicle(static_cast<Vehicle *>(next))); }
inline Vehicle *RustVehicleLastEnginePart(Vehicle *v) noexcept { return v->GetLastEnginePart(); }
inline void RustVehicleSetServiceInterval(Vehicle *v, uint16_t interval) noexcept { v->SetServiceInterval(interval); }
inline void RustVehicleSetSubtype(Vehicle *v, uint8_t subtype) noexcept { v->subtype = subtype; }
inline void RustVehicleInvalidateNewGRFCache(Vehicle *v) noexcept { v->InvalidateNewGRFCache(); }
inline uint16_t RustVehicleUnitNumber(const Vehicle *v) noexcept { return v->unitnumber; }
inline void RustVehicleReleaseUnitNumber(Vehicle *v) noexcept { v->ReleaseUnitNumber(); }
inline uint16_t RustGroundFirstEngine(const Vehicle *v) noexcept { return v->GetGroundVehicleCache()->first_engine.base(); }
inline OpenTTDCargoList *RustVehicleCargoOwner(Vehicle *v) noexcept { return v->cargo.RustOwner(); }
inline OpenTTDVehicleTileLinks *RustVehicleTileLinks(Vehicle *v) noexcept { return &v->tile_links; }
inline bool RustVehicleHasEngine(const Vehicle *v) noexcept { return v->HasEngineType(); }
inline uint8_t RustVehicleDay(const Vehicle *v) noexcept { return v->day_counter; }
inline uint32_t RustVehicleTile(const Vehicle *v) noexcept { return v->tile.base(); }
inline uint32_t RustVehicleDestTile(const Vehicle *v) noexcept { return v->dest_tile.base(); }
inline OrderType RustVehicleOrderType(const Vehicle *v) noexcept { return v->current_order.GetType(); }
inline uint16_t RustVehicleOrderDestination(const Vehicle *v) noexcept { return v->current_order.GetDestination().base(); }
inline OrderUnloadType RustVehicleOrderUnloadType(const Vehicle *v) noexcept { return v->current_order.GetUnloadType(); }
inline OrderLoadType RustVehicleOrderLoadType(const Vehicle *v) noexcept { return v->current_order.GetLoadType(); }
inline TimerGameCalendar::Date RustVehicleCalendarService(const Vehicle *v) noexcept { return v->date_of_last_service_newgrf; }
inline TimerGameCalendar::Year RustVehicleBuildYear(const Vehicle *v) noexcept { return v->build_year; }
inline uint8_t RustVehicleSprite(const Vehicle *v) noexcept { return v->spritenum; }
inline void RustWriteVehicleSprite(Vehicle *v, uint8_t sprite) noexcept { v->spritenum = sprite; }
inline void RustVehicleFlipImage(Vehicle *v) noexcept { RustWriteVehicleSprite(v, RustVehicleSprite(v) + 1); }
inline SpriteID RustVehicleFirstSprite(const Vehicle *v) noexcept { return v->sprite_cache.sprite_seq.seq[0].sprite; }
inline void RustWriteVehicleFirstSprite(Vehicle *v, SpriteID sprite) noexcept { v->sprite_cache.sprite_seq.seq[0].sprite = sprite; }
inline void RustWriteVehicleSpriteSequence(Vehicle *v, SpriteID sprite) noexcept { v->sprite_cache.sprite_seq.Set(sprite); }
inline uint8_t RustVehicleNumOrders(const Vehicle *v) noexcept { return v->GetNumOrders(); }
inline uint8_t RustVehicleRealOrderIndex(const Vehicle *v) noexcept { return v->cur_real_order_index; }
inline uint16_t RustVehicleServiceInterval(const Vehicle *v) noexcept { return v->GetServiceInterval(); }
inline int64_t RustVehicleDisplayProfitLastYear(const Vehicle *v) noexcept { return v->GetDisplayProfitLastYear().base(); }
inline uint16_t RustVehicleOrderMaxSpeed(const Vehicle *v) noexcept { return v->current_order.GetMaxSpeed(); }
template <class T> inline int RustVehicleDisplayMaxSpeed(const T *v) noexcept { return v->GetDisplayMaxSpeed(); }
inline int32_t RustVehicleDisplaySpeed(OpenTTDVehicle *v) noexcept { return RustVehicleDisplayMaxSpeed(VehicleOf(v)); }
inline uint8_t RustVehicleOrderNonStop(const Vehicle *v) noexcept { return v->current_order.GetNonStopType().base(); }
bool RustVehicleInDepot(const Vehicle *v) noexcept;
inline uint32_t RustVehicleBaseCrash(OpenTTDVehicle *v, bool flooded) noexcept { return VehicleOf(v)->Vehicle::Crash(flooded); }
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
inline void RustWriteVehicleStatus(Vehicle *v, VehStates status) noexcept { v->vehstatus = status; }
inline void RustVehicleSetUnclickable(Vehicle *v) noexcept { RustWriteVehicleStatus(v, VehState::Unclickable); }
inline void RustWriteVehicleTile(Vehicle *v, TileIndex tile) noexcept { v->tile = tile; }
inline void RustWriteVehicleTick(Vehicle *v, uint8_t tick) noexcept { v->tick_counter = tick; }
inline uint8_t RustVehicleBreakdownDelay(const Vehicle *v) noexcept { return v->breakdown_delay; }
inline uint8_t RustVehicleBreakdownChance(const Vehicle *v) noexcept { return v->breakdown_chance; }
inline uint8_t RustVehicleBreakdownsSinceService(const Vehicle *v) noexcept { return v->breakdowns_since_last_service; }
inline uint16_t RustVehicleReliability(const Vehicle *v) noexcept { return v->reliability; }
inline uint16_t RustVehicleReliabilityDecay(const Vehicle *v) noexcept { return v->reliability_spd_dec; }
inline TimerGameCalendar::Date RustVehicleAge(const Vehicle *v) noexcept { return v->age; }
inline int32_t RustVehicleAgeValue(const Vehicle *v) noexcept { return RustVehicleAge(v).base(); }
inline void RustWriteVehicleAge(Vehicle *v, int32_t age) noexcept { v->age = TimerGameCalendar::Date{age}; }
inline void RustVehicleWriteBreakdown(Vehicle *v, uint8_t counter, uint8_t delay) noexcept { v->breakdown_ctr = counter; v->breakdown_delay = delay; }
inline bool RustVehicleIsGround(const Vehicle *v) noexcept { return v->IsGroundVehicle(); }
inline TimerGameCalendar::Date RustVehicleMaxAge(const Vehicle *v) noexcept { return v->max_age; }
inline TimerGameEconomy::Date RustVehicleEconomyAge(const Vehicle *v) noexcept { return v->economy_age; }
inline TimerGameEconomy::Date RustVehicleLastService(const Vehicle *v) noexcept { return v->date_of_last_service; }
inline int32_t RustVehicleLastServiceOf(OpenTTDVehicle *v) noexcept { return RustVehicleLastService(VehicleOf(v)).base(); }
inline uint8_t RustVehicleBreakdownCounter(const Vehicle *v) noexcept { return v->breakdown_ctr; }
inline int64_t RustVehicleValue(const Vehicle *v) noexcept { return v->value.base(); }
inline uint16_t RustEngineReliability(const Engine *e) noexcept { return e->reliability; }
template <typename Handle> inline uint32_t RustVehicleIndex(Handle v) noexcept { return static_cast<const Vehicle *>(static_cast<const void *>(v))->index.base(); }
inline uint32_t RustVehicleIndexOpaque(const void *v) noexcept { return RustVehicleIndex(v); }
/** The legacy company sentinel is UINT32_MAX, distinct from VehicleID::Invalid(). */
inline uint32_t RustVehiclePreviousID(const Vehicle *v) noexcept
{
	const Vehicle *previous = RustVehiclePrevious(v);
	return previous == nullptr ? UINT32_MAX : RustVehicleIndexOpaque(previous);
}
inline void RustVehicleSetEngine(OpenTTDVehicle *v, uint16_t engine) noexcept { VehicleOf(v)->engine_type = EngineID(engine); }
inline void RustVehicleSetOwner(OpenTTDVehicle *v, uint8_t owner) noexcept { VehicleOf(v)->owner = Owner(owner); }
inline uint16_t RustVehicleEngineID(const Vehicle *v) noexcept { return v->engine_type.base(); }
inline const OpenTTDEngine *RustEngine(const Engine *e) noexcept { return reinterpret_cast<const OpenTTDEngine *>(e); }
inline const Engine *EngineOf(const OpenTTDEngine *e) noexcept { return reinterpret_cast<const Engine *>(e); }
inline const GRFFile *RustEngineGRF(const OpenTTDEngine *e) noexcept { return EngineOf(e)->GetGRF(); }
inline const OpenTTDEngine *RustVehicleGetEngine(OpenTTDVehicle *v) noexcept { return RustEngine(VehicleOf(v)->GetEngine()); }
/** Shared engine observation for a live base shell, resolved once. */
template <typename Result, auto Read>
static Result RustVehicleEngineRead(OpenTTDVehicle *v) noexcept { return Read(RustVehicleGetEngine(v)); }
/** Signature adapter for a typed controller handle's shared engine observation. */
template <typename Result, typename Handle, auto Read>
static Result RustVehicleEngineObservation(Handle handle) noexcept { return RustVehicleEngineRead<Result, Read>(handle.handle); }
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
/** Canonical cached consist length, shared by narrow and compound observations. */
inline uint16_t RustGroundCachedTotalLength(const GroundVehicleCache *cache) noexcept { return cache->cached_total_length; }
template <class T>
inline bool RustGroundHasArticulated(const T *v) noexcept { return v->HasArticulatedPart(); }

inline const uint8_t *RustVehicleSubtypeAddress(const Vehicle *v) noexcept { return &v->subtype; }
inline uint8_t RustVehicleSubtype(const Vehicle *v) noexcept { return *RustVehicleSubtypeAddress(v); }
inline bool RustVehicleFront(const Vehicle *v) noexcept { return v->IsFrontEngine(); }
inline auto *RustVehicleVisualCache(const Vehicle *v) noexcept { return &v->vcache.cached_vis_effect; }
inline uint8_t RustVehicleVisualEffect(const Vehicle *v) noexcept { return *RustVehicleVisualCache(v); }
inline const int32_t *RustVehicleXAddress(const Vehicle *v) noexcept { return &v->x_pos; }
inline int32_t RustVehicleX(const Vehicle *v) noexcept { return *RustVehicleXAddress(v); }
inline const int32_t *RustVehicleYAddress(const Vehicle *v) noexcept { return &v->y_pos; }
inline int32_t RustVehicleY(const Vehicle *v) noexcept { return *RustVehicleYAddress(v); }
inline const int32_t *RustVehicleZAddress(const Vehicle *v) noexcept { return &v->z_pos; }
inline int32_t RustVehicleZ(const Vehicle *v) noexcept { return *RustVehicleZAddress(v); }
inline void RustWriteVehicleX(Vehicle *v, int32_t x) noexcept { v->x_pos = x; }
inline void RustWriteVehicleY(Vehicle *v, int32_t y) noexcept { v->y_pos = y; }
inline void RustWriteVehicleZ(Vehicle *v, int32_t z) noexcept { v->z_pos = z; }
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
inline uint16_t *RustVehicleCargoCapacityAddress(Vehicle *v) noexcept { return &v->cargo_cap; }
inline uint16_t *RustVehicleCargoCounterAddress(Vehicle *v) noexcept { return &v->cargo_age_counter; }
inline uint32_t *RustVehicleMotionAddress(Vehicle *v) noexcept { return &v->motion_counter; }

/** Typed shell adapter; canonical reads keep their actual return types.
 * Result also preserves existing wider legacy table signatures. */
template <typename Result, auto Read>
static Result OPENTTD_RUST_CALL RustVehicleRead(OpenTTDVehicle *shell) noexcept { return static_cast<Result>(Read(VehicleOf(shell))); }

/** Typed signature adapter; each operation keeps its canonical definition. */
template <auto Operation, typename... Args>
static auto OPENTTD_RUST_CALL RustVehicleInvoke(OpenTTDVehicle *shell, Args... args) noexcept { return Operation(VehicleOf(shell), args...); }
/** Native compound handles retain their existing signature around a shared leaf. */
template <auto Operation, typename Handle, typename... Args>
static auto OPENTTD_RUST_CALL RustVehicleHandleInvoke(Handle vehicle, Args... args) noexcept { return Operation(vehicle.handle, args...); }
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
inline bool RustVehicleUsesWagonOverride(const Vehicle *v) noexcept { return UsesWagonOverride(v); }
inline bool RustVehicleArticulated(const Vehicle *v) noexcept { return v->IsArticulatedPart(); }
inline uint8_t RustVehicleCargoSubtype(const Vehicle *v) noexcept { return v->cargo_subtype; }
inline uint8_t RustVehicleCargoType(OpenTTDVehicle *handle) noexcept { return VehicleOf(handle)->cargo_type; }
inline uint8_t RustVehicleOwnerOf(OpenTTDVehicle *handle) noexcept { return RustVehicleOwner(VehicleOf(handle)); }
inline void RustVehicleDetachPayment(OpenTTDVehicle *handle) noexcept { VehicleOf(handle)->cargo_payment = nullptr; }
inline bool RustVehicleHasArticulated(OpenTTDVehicle *handle) noexcept { return RustGroundHasArticulated(VehicleOf(handle)); }

inline const GRFFile *RustEngineGRF(const Engine *engine) noexcept { return RustEngineGRF(RustEngine(engine)); }

/** GRF version of a live engine; the original default-engine value is zero. */
inline uint8_t RustEngineGRFVersion(const Engine *engine) noexcept
{
	const GRFFile *grf = RustEngineGRF(engine);
	return grf == nullptr ? uint8_t(0) : RustGRFFileVersion(grf);
}

template <class T>
inline OpenTTDGroundViewportNext OPENTTD_RUST_CALL GroundSvcUpdateViewportThenNext(OpenTTDVehicle *part, OpenTTDVehicle *front) noexcept
{
	T *u = T::From(VehicleOf(part));
	/* UpdateViewport may reenter Rust; no Rust borrow survives this call. */
	RustVehicleUpdateViewport(u, false, false);
	Vehicle *next = RustVehicleNext(u);
	/* Only the final iteration consumes front progress, after every viewport. */
	return {.next = RustVehicle(next), .final_progress = next == nullptr ? RustVehicleProgress(VehicleOf(front)) : uint8_t(0)};
}
template <class T>
inline void OPENTTD_RUST_CALL GroundSvcTurn(OpenTTDVehicle *v, uint8_t direction) noexcept
{
	T *u = T::From(VehicleOf(v));
	u->direction = static_cast<Direction>(direction);
	RustVehicleUpdateViewport(u, true, true);
}
template <class T>
inline void OPENTTD_RUST_CALL GroundSvcGoToDepotService(OpenTTDVehicle *v, uint16_t depot) noexcept
{
	T *u = T::From(VehicleOf(v));
	SetBit(u->gv_flags, GVF_SUPPRESS_IMPLICIT_ORDERS);
	u->current_order.MakeGoToDepot(DepotID(depot), OrderDepotTypeFlag::Service);
}
template <class T>
inline void OPENTTD_RUST_CALL GroundSvcWriteFirstEngine(OpenTTDVehicle *v, uint16_t engine) noexcept
{
	RustGroundSetFirstEngine(&T::From(VehicleOf(v))->gcache, EngineID(engine));
}
template <class T>
inline void OPENTTD_RUST_CALL GroundSvcWriteLength(OpenTTDVehicle *part, uint8_t length, OpenTTDVehicle *front, uint16_t total_length) noexcept
{
	T *u = T::From(VehicleOf(part));
	u->gcache.cached_veh_length = length;
	T::From(VehicleOf(front))->gcache.cached_total_length = total_length;
}

template <class T>
/** The head of `RoadVehUpdateCache`. */
inline void OPENTTD_RUST_CALL GroundSvcCacheBegin(OpenTTDVehicle *v) noexcept
{
	T *u = T::From(VehicleOf(v));
	u->InvalidateNewGRFCacheOfChain();
	u->gcache.cached_total_length = 0;
}

template <class T>
inline uint8_t OPENTTD_RUST_CALL GroundSvcCachedLength(OpenTTDVehicle *v) noexcept
{
	return RustGroundCachedLength(T::From(VehicleOf(v)));
}

template <class T>
inline bool OPENTTD_RUST_CALL GroundSvcHasArticulated(OpenTTDVehicle *v) noexcept
{
	return RustGroundHasArticulated(T::From(VehicleOf(v)));
}

/** One immutable process-wide GroundVehicle table per vehicle type. */
template <class T>
inline const OpenTTDGroundServices _rust_ground_services{
	.physics = &GetRustGroundPhysicsServices(),
	.cache_begin = GroundSvcCacheBegin<T>,
	.update_viewport_then_next = GroundSvcUpdateViewportThenNext<T>,
	.turn = GroundSvcTurn<T>,
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

inline OpenTTDVehicle *RustGroundFirst(OpenTTDVehicle *v) noexcept { return RustVehicle(RustVehicleFirst(VehicleOf(v))); }

inline void RustVehicleDetailsDirty(OpenTTDVehicle *shell) noexcept { SetWindowDirty(WC_VEHICLE_DETAILS, VehicleOf(shell)->index); }

template <typename Handle>
inline uint32_t RustVehicleBaseCrashHandle(Handle vehicle, bool flooded) noexcept { return RustVehicleBaseCrash(vehicle.handle, flooded); }
inline void RustVehicleShiftDates(OpenTTDVehicle *v, int32_t days) noexcept { VehicleOf(v)->ShiftDates(TimerGameEconomy::Date{days}); }
#endif /* RUST_VEHICLE_SERVICES_H */
