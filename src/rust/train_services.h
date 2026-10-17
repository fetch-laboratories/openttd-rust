/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

#include "services_ffi.h"
#include "services_rail.h"

/** @file train_services.h Direct shared-world operations; no train policy. */
#include "sound_services.h"

#include "vehicle_services.h"
#include "settings_services.h"

/* Compile-time adapters retain the legacy callback widths while sharing the actual reads. */
template <typename Result, auto Read>
static Result TrainVehicleRead(OpenTTDTrainHandle id) noexcept
{
	return Read(static_cast<const Train *>(id.shell));
}

template <typename Result, auto Member>
static Result TrainSharedMap(OpenTTDTrainHandle, uint32_t tile) noexcept
{
	return (GetRustSharedServices().*Member)(tile);
}

static void TrainWriteTile(OpenTTDTrainHandle id, uint32_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); v->tile = TileIndex(static_cast<uint32_t>(value));}
static void TrainWriteDest(OpenTTDTrainHandle id, uint32_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); v->dest_tile = TileIndex(static_cast<uint32_t>(value));}
static void TrainWriteX(OpenTTDTrainHandle id, int32_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); v->x_pos = static_cast<int32_t>(value);}
static void TrainWriteY(OpenTTDTrainHandle id, int32_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); v->y_pos = static_cast<int32_t>(value);}
static void TrainWriteZ(OpenTTDTrainHandle id, int32_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); v->z_pos = static_cast<int32_t>(value);}
static void TrainWriteDirection(OpenTTDTrainHandle id, uint8_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); v->direction = static_cast<Direction>(value);}
static void TrainWriteSpeed(OpenTTDTrainHandle id, uint16_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); v->cur_speed = static_cast<uint16_t>(value);}
static void TrainWriteTick(OpenTTDTrainHandle id, uint8_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); RustWriteVehicleTick(v, static_cast<uint8_t>(value));}
static void TrainWriteRunning(OpenTTDTrainHandle id, uint8_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); v->running_ticks = static_cast<uint8_t>(value);}
static void TrainWriteDay(OpenTTDTrainHandle id, uint8_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); v->day_counter = static_cast<uint8_t>(value);}
static void TrainWriteOrderTime(OpenTTDTrainHandle id, int32_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); v->current_order_time = static_cast<int32_t>(value);}
static void TrainWriteProgress(OpenTTDTrainHandle id, uint8_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); RustWriteVehicleProgress(v, static_cast<uint8_t>(value));}
static void TrainWriteSubspeed(OpenTTDTrainHandle id, uint8_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); v->subspeed = static_cast<uint8_t>(value);}
static void TrainWriteGvFlags(OpenTTDTrainHandle id, uint16_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); v->gv_flags = static_cast<uint16_t>(value);}
static void TrainWriteAcceleration(OpenTTDTrainHandle id, uint8_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); v->acceleration = static_cast<uint8_t>(value);}
static void TrainWriteLength(OpenTTDTrainHandle id, uint8_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); v->gcache.cached_veh_length = static_cast<uint8_t>(value);}
static void TrainWriteTotalLength(OpenTTDTrainHandle id, uint16_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); v->gcache.cached_total_length = static_cast<uint16_t>(value);}
static void TrainWriteFirstEngine(OpenTTDTrainHandle id, uint16_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); v->gcache.first_engine = EngineID(static_cast<uint16_t>(value));}
static void TrainWriteMaxSpeed(OpenTTDTrainHandle id, uint16_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); v->vcache.cached_max_speed = static_cast<uint16_t>(value);}
static void TrainWriteCargoCap(OpenTTDTrainHandle id, uint16_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); v->cargo_cap = static_cast<uint16_t>(value);}
static void TrainWriteRefitCap(OpenTTDTrainHandle id, uint16_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); v->refit_cap = static_cast<uint16_t>(value);}
static void TrainWriteCargoAge(OpenTTDTrainHandle id, uint16_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); v->vcache.cached_cargo_age_period = static_cast<uint16_t>(value);}
static void TrainWriteLastStation(OpenTTDTrainHandle id, uint16_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); v->last_station_visited = StationID(static_cast<uint16_t>(value));}
static void TrainWriteColourmap(OpenTTDTrainHandle id, uint64_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); v->colourmap = static_cast<PaletteID>(value);}
static void TrainWriteStatus(OpenTTDTrainHandle id, uint8_t value) noexcept
{Train *v=static_cast<Train *>(id.shell); v->vehstatus = VehStates(static_cast<uint8_t>(value));}
static uint64_t TrainAcceleration(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return static_cast<uint64_t>(v->GetAcceleration());
}
static uint64_t TrainAccModel(OpenTTDTrainHandle) noexcept
{
	return _settings_game.vehicle.train_acceleration_model;
}
static uint64_t TrainAccType(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return static_cast<uint8_t>(GetRailTypeInfo(GetRailType(v->tile))->acceleration_type);
}
static uint64_t TrainAdvanceDistance(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return v->GetAdvanceDistance();
}
static void TrainAge(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	AgeVehicle(v);
}
static uint64_t TrainAllPowered(OpenTTDTrainHandle, uint64_t a) noexcept
{
	return GetAllPoweredRailTypes(RailTypes(a)).base();
}
static uint64_t TrainAmbientSound(OpenTTDTrainHandle) noexcept
{
	return OpenTTDAmbientSound();
}
static void TrainArrivalNews(OpenTTDTrainHandle id, uint16_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	Station *st = Station::Get(StationID(a)); AddVehicleNewsItem(GetEncodedString(STR_NEWS_FIRST_TRAIN_ARRIVAL, st->index), v->owner == _local_company ? NewsType::ArrivalCompany : NewsType::ArrivalOther, v->index, st->index); AI::NewEvent(v->owner, new ScriptEventStationFirstVehicle(st->index, v->index)); Game::NewEvent(new ScriptEventStationFirstVehicle(st->index, v->index));
}
static void TrainArrivalTriggers(OpenTTDTrainHandle id, uint16_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	Station *st = Station::Get(StationID(a)); TriggerStationRandomisation(st, v->tile, StationRandomTrigger::VehicleArrives); TriggerStationAnimation(st, v->tile, StationAnimationTrigger::VehicleArrives);
}
static uint64_t TrainAxisDiag(OpenTTDTrainHandle, uint8_t a) noexcept
{
	return AxisToDiagDir(static_cast<Axis>(a));
}
static uint64_t TrainBackoff(OpenTTDTrainHandle) noexcept
{
	return _settings_game.pf.path_backoff_interval;
}
static void TrainBaseViewport(OpenTTDTrainHandle id, uint8_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->Vehicle::UpdateViewport(a != 0);
}
static void TrainBeginLoading(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->BeginLoading();
}
static void TrainCacheOverride(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->tcache.cached_override = GetWagonOverrideSpriteSet(v->engine_type, v->cargo_type, v->gcache.first_engine);
}
static uint64_t TrainCallbackLength(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return GetVehicleCallback(CBID_VEHICLE_LENGTH, 0, 0, v->engine_type, v);
}
static uint64_t TrainCapacity(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return v->GetEngine()->DetermineCapacity(v);
}
static void TrainCapacityError(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	ShowNewGrfVehicleError(v->engine_type, STR_NEWGRF_BROKEN, STR_NEWGRF_BROKEN_CAPACITY, GRFBug::VehCapacity, true);
}
static uint64_t TrainCargoAgeDefault(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return v->GetEngine()->info.cargo_age_period;
}
static void TrainCargoChanged(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->CargoChanged();
}
static uint64_t TrainChainDepot(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return v->IsChainInDepot();
}
static void TrainCheckBreakdown(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	CheckVehicleBreakdown(v);
}
static void TrainCheckNext(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	CheckNextTrainTile(v);
}
static void TrainCheckOrders(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	CheckOrders(v);
}
static uint64_t TrainCheckReverse(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return CheckReverseTrain(v);
}
static uint64_t TrainChooseTrack(OpenTTDTrainHandle id, uint32_t a, uint8_t b, uint8_t c) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return ChooseTrainTrack(v, TileIndex(a), static_cast<DiagDirection>(b), static_cast<TrackBits>(c), false, nullptr, true);
}
static void TrainClearReservation(OpenTTDTrainHandle id, uint32_t a, uint8_t b) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	ClearPathReservation(v, TileIndex(a), static_cast<Trackdir>(b));
}
static uint64_t TrainCompatibleRailOwner(OpenTTDTrainHandle id, uint32_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return IsTileOwner(TileIndex(a), v->owner);
}
static void TrainConsistWindows(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	SetWindowDirty(WC_VEHICLE_DETAILS, v->index); InvalidateWindowData(WC_VEHICLE_REFIT, v->index, VIWD_CONSIST_CHANGED); InvalidateWindowData(WC_VEHICLE_ORDERS, v->index, VIWD_CONSIST_CHANGED); InvalidateNewGRFInspectWindow(GSF_TRAINS, v->index); InvalidateWindowData(WC_VEHICLE_VIEW, v->index, VIWD_CONSIST_CHANGED);
}
static uint64_t TrainCostClass(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return static_cast<uint32_t>(v->GetEngine()->VehInfo<RailVehicleInfo>().running_cost_class);
}
static uint64_t TrainCostDefault(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return v->GetEngine()->VehInfo<RailVehicleInfo>().running_cost;
}
static uint64_t TrainCostDivisor(OpenTTDTrainHandle) noexcept
{
	return CalendarTime::DAYS_IN_YEAR * Ticks::DAY_TICKS;
}
static uint64_t TrainCountChain(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return CountVehiclesInChain(v);
}
static void TrainCrashEvent(OpenTTDTrainHandle id, uint32_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	AI::NewEvent(v->owner, new ScriptEventVehicleCrashed(v->index, v->tile, ScriptEventVehicleCrashed::CRASH_TRAIN, static_cast<uint>(a), v->owner)); Game::NewEvent(new ScriptEventVehicleCrashed(v->index, v->tile, ScriptEventVehicleCrashed::CRASH_TRAIN, static_cast<uint>(a), v->owner));
}
static uint64_t TrainCrashGround(OpenTTDTrainHandle id, uint8_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return v->GroundVehicleBase::Crash(a != 0);
}
static void TrainCrashNews(OpenTTDTrainHandle id, uint32_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	AddTileNewsItem(GetEncodedString(STR_NEWS_TRAIN_CRASH, static_cast<uint>(a)), NewsType::Accident, v->tile);
}
static void TrainCrashRating(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	ModifyStationRatingAround(v->tile, v->owner, -160, 30);
}
static void TrainCrashSound(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	SndPlayVehicleFx(SND_13_TRAIN_COLLISION, v);
}
static uint64_t TrainCrossingBarred(OpenTTDTrainHandle, uint32_t a) noexcept
{
	return IsCrossingBarred(TileIndex(a));
}
static uint64_t TrainCrossingRailAxis(OpenTTDTrainHandle, uint32_t a) noexcept
{
	return GetCrossingRailAxis(TileIndex(a));
}
static uint64_t TrainCrossingReserved(OpenTTDTrainHandle, uint32_t a) noexcept
{
	return HasCrossingReservation(TileIndex(a));
}
static void TrainCrossingSound(OpenTTDTrainHandle, uint32_t a) noexcept
{
	SndPlayTileFx(SND_0E_LEVEL_CROSSING, TileIndex(a));
}
static uint64_t TrainCurveAdvantage(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return GetRailTypeInfo(GetRailType(v->tile))->curve_speed;
}
static uint64_t TrainCurveMod(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return static_cast<uint16_t>(GetVehicleProperty(v, PROP_TRAIN_CURVE_SPEED_MOD, RailVehInfo(v->engine_type)->curve_speed_mod, true));
}
static uint64_t TrainDayTicks(OpenTTDTrainHandle) noexcept
{
	return Ticks::DAY_TICKS;
}
static void TrainDecreaseValue(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	DecreaseVehicleValue(v);
}
static void TrainDeleteVehicle(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	delete v;
}
static void TrainDepotDirty(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	InvalidateWindowData(WC_VEHICLE_DEPOT, v->tile);
}
static uint64_t TrainDepotTrack(OpenTTDTrainHandle, uint32_t a) noexcept
{
	return GetRailDepotTrack(TileIndex(a));
}
static void TrainDepotWindow(OpenTTDTrainHandle, uint32_t a) noexcept
{
	SetWindowDirty(WC_VEHICLE_DEPOT, TileIndex(a));
}
static uint64_t TrainDiagAxis(OpenTTDTrainHandle, uint8_t a) noexcept
{
	return DiagDirToAxis(static_cast<DiagDirection>(a));
}
static uint64_t TrainDiagBetween(OpenTTDTrainHandle, uint32_t a, uint32_t b) noexcept
{
	return DiagdirBetweenTiles(TileIndex(a), TileIndex(b));
}
static uint64_t TrainDiagReachesTracks(OpenTTDTrainHandle, uint8_t a) noexcept
{
	return DiagdirReachesTracks(static_cast<DiagDirection>(a));
}
static uint64_t TrainDiagTrackdir(OpenTTDTrainHandle, uint8_t a) noexcept
{
	return DiagDirToDiagTrackdir(static_cast<DiagDirection>(a));
}
static uint64_t TrainDirDiag(OpenTTDTrainHandle, uint8_t a) noexcept
{
	return DirToDiagDir(static_cast<Direction>(a));
}
static uint64_t TrainDisasterSound(OpenTTDTrainHandle) noexcept
{
	return _settings_client.sound.disaster;
}
static void TrainDisconnect(OpenTTDTrainHandle) noexcept
{
	FatalError("Disconnecting train");
}
static void TrainEconomyAge(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	EconomyAgeVehicle(v);
}
static uint64_t TrainEnginePower(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return RailVehInfo(v->engine_type)->power;
}
static void TrainEnterDepot(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	VehicleEnterDepot(v);
}
static uint64_t TrainEnterTile(OpenTTDTrainHandle id, uint32_t a, int32_t b, int32_t c) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return VehicleEnterTile(v, TileIndex(a), static_cast<int>(b), static_cast<int>(c)).base();
}
static OpenTTDTrainDepot TrainFindDepot(OpenTTDTrainHandle id, int32_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	auto result = FindClosestTrainDepot(v, static_cast<int>(a)); return {result.tile.base(), result.best_length};
}
static uint64_t TrainFirstTrack(OpenTTDTrainHandle, uint8_t a) noexcept
{
	return FindFirstTrack(static_cast<TrackBits>(a));
}
static void TrainFreeReservation(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	FreeTrainTrackReservation(v);
}
static uint64_t TrainGrfVersion(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return RustEngineGRFVersion(v->GetEngine());
}
static uint64_t TrainHandleBreakdown(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return v->HandleBreakdown();
}
static uint64_t TrainHasDepotRes(OpenTTDTrainHandle, uint32_t a) noexcept
{
	return HasDepotReservation(TileIndex(a));
}
static uint64_t TrainHasReserved(OpenTTDTrainHandle, uint32_t a, uint8_t b) noexcept
{
	return HasReservedTracks(TileIndex(a), static_cast<TrackBits>(b));
}
static void TrainHideFill(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	HideFillingPercent(&v->fill_percent_te_id);
}
static uint64_t TrainInclination(OpenTTDTrainHandle id, uint8_t a, uint8_t b) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return static_cast<uint64_t>(v->UpdateInclination(a != 0, b != 0));
}
static void TrainInvalidateGrf(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->InvalidateNewGRFCache();
}
static uint64_t TrainInvalidPrice(OpenTTDTrainHandle) noexcept
{
	return static_cast<uint32_t>(INVALID_PRICE);
}
static uint64_t TrainIsBridge(OpenTTDTrainHandle, uint32_t a) noexcept
{
	return IsBridgeTile(TileIndex(a));
}
static uint64_t TrainIsDepot(OpenTTDTrainHandle, uint32_t a) noexcept
{
	return IsRailDepotTile(TileIndex(a));
}
static uint64_t TrainIsPlainRail(OpenTTDTrainHandle, uint32_t a) noexcept
{
	return IsPlainRailTile(TileIndex(a));
}
static uint64_t TrainIsRailway(OpenTTDTrainHandle, uint32_t a) noexcept
{
	return IsTileType(TileIndex(a), MP_RAILWAY);
}
static uint64_t TrainIsStation(OpenTTDTrainHandle, uint32_t a) noexcept
{
	return IsRailStationTile(TileIndex(a));
}
static uint64_t TrainIsStationAny(OpenTTDTrainHandle, uint32_t a) noexcept
{
	return IsTileType(TileIndex(a), MP_STATION);
}
static uint64_t TrainIsTunnelbridge(OpenTTDTrainHandle, uint32_t a) noexcept
{
	return IsTileType(TileIndex(a), MP_TUNNELBRIDGE);
}
static void TrainLargeExplosion(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	CreateEffectVehicleRel(v, 4, 4, 8, EV_EXPLOSION_LARGE);
}
static void TrainLastSpeed(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->SetLastSpeed();
}
static void TrainLeaveSound(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->PlayLeaveStationSound();
}
static void TrainLeaveStation(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->LeaveStation();
}
static void TrainLeaveUnbunch(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->LeaveUnbunchingDepot();
}
static uint64_t TrainLengthCallback(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return v->GetEngine()->info.callback_mask.Test(VehicleCallbackMask::Length);
}
static void TrainLengthChanged(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	VehicleLengthChanged(v);
}
static uint64_t TrainLengthDefault(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return RailVehInfo(v->engine_type)->shorten_factor;
}
static void TrainLengthError(OpenTTDTrainHandle id, uint16_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	ErrorUnknownCallbackResult(v->GetEngine()->GetGRFID(), CBID_VEHICLE_LENGTH, static_cast<uint16_t>(a));
}
static void TrainLoading(OpenTTDTrainHandle id, uint8_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->HandleLoading(a != 0);
}
static uint64_t TrainLocalCompany(OpenTTDTrainHandle) noexcept
{
	return _local_company.base();
}
static uint64_t TrainLostWarn(OpenTTDTrainHandle) noexcept
{
	return _settings_client.gui.lost_vehicle_warn;
}
static uint64_t TrainMapSize(OpenTTDTrainHandle) noexcept
{
	return Map::Size();
}
static uint64_t TrainMaxDepotPenalty(OpenTTDTrainHandle) noexcept
{
	return RustMaximumDepotPenalty();
}
static uint64_t TrainNeedsService(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return v->NeedsAutomaticServicing();
}
static uint64_t TrainNo90(OpenTTDTrainHandle, uint8_t a, uint8_t b) noexcept
{
	return Rail90DegTurnDisallowed(static_cast<RailType>(a), static_cast<RailType>(b));
}
static uint64_t TrainOnewayBlocking(OpenTTDTrainHandle, uint32_t a, uint8_t b) noexcept
{
	return HasOnewaySignalBlockingTrackdir(TileIndex(a), static_cast<Trackdir>(b));
}
static void TrainOrderDepotService(OpenTTDTrainHandle id, uint16_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->current_order.MakeGoToDepot(DepotID(a), OrderDepotTypeFlag::Service, OrderNonStopFlag::NoIntermediate, OrderDepotActionFlag::NearestDepot);
}
static void TrainOrderDummy(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->current_order.MakeDummy();
}
static void TrainOrderFree(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->current_order.Free();
}
static uint64_t TrainOrderMaxSpeed(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return v->current_order.GetMaxSpeed();
}
static uint64_t TrainOrderStop(OpenTTDTrainHandle id, uint16_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return v->current_order.ShouldStopAtStation(v, StationID(a));
}
static void TrainPayRunning(OpenTTDTrainHandle id, int64_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	CommandCost cost(EXPENSES_TRAIN_RUN, Money(static_cast<int64_t>(a))); v->profit_this_year -= cost.GetCost(); v->running_ticks = 0; SubtractMoneyFromCompanyFract(v->owner, cost);
}
static uint64_t TrainPbsSignalType(OpenTTDTrainHandle) noexcept
{
	return SIGTYPE_PBS;
}
static uint64_t TrainPlatformAhead(OpenTTDTrainHandle id, uint16_t a, uint32_t b) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return Station::Get(StationID(a))->GetPlatformLength(TileIndex(b), DirToDiagDir(RustVehicleDirection(v)));
}
static uint64_t TrainPlatformLength(OpenTTDTrainHandle, uint16_t a, uint32_t b) noexcept
{
	return Station::Get(StationID(a))->GetPlatformLength(TileIndex(b));
}
static void TrainPosition(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->UpdatePosition();
}
static uint64_t TrainPowWagPower(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return RailVehInfo(v->engine_type)->pow_wag_power;
}
static uint64_t TrainPrice(OpenTTDTrainHandle id, uint32_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return static_cast<uint64_t>(GetPrice(v->GetEngine()->VehInfo<RailVehicleInfo>().running_cost_class, static_cast<uint>(a), v->GetEngine()->GetGRF()));
}
static uint64_t TrainProcessOrders(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return ProcessOrders(v);
}
static uint64_t TrainProperty(OpenTTDTrainHandle id, uint8_t a, uint32_t b) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return GetVehicleProperty(v, static_cast<PropertyID>(a), static_cast<uint32_t>(b));
}
static uint64_t TrainRailvehWagon(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return RailVehInfo(v->engine_type)->railveh_type == RAILVEH_WAGON;
}
static uint64_t TrainRailTilt(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return v->GetEngine()->info.misc_flags.Test(EngineMiscFlag::RailTilts);
}
static uint64_t TrainRailType(OpenTTDTrainHandle, uint32_t a) noexcept
{
	return GetRailType(TileIndex(a));
}
static uint64_t TrainRailTypes(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return RailVehInfo(v->engine_type)->railtypes.base();
}
static uint64_t TrainReservePaths(OpenTTDTrainHandle) noexcept
{
	return _settings_game.pf.reserve_paths;
}
static void TrainReserveUnder(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->ReserveTrackUnderConsist();
}
static void TrainResetUnbunch(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->ResetDepotUnbunching();
}
static uint64_t TrainReverseAtSignals(OpenTTDTrainHandle) noexcept
{
	return _settings_game.pf.reverse_at_signals;
}
static uint64_t TrainReverseSingleBlocked(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return EngInfo(v->engine_type)->callback_mask.Test(VehicleCallbackMask::ArticEngine);
}
static void TrainReverseWindows(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	SetWindowDirty(WC_VEHICLE_DEPOT, v->tile); SetWindowDirty(WC_VEHICLE_DETAILS, v->index); SetWindowDirty(WC_VEHICLE_VIEW, v->index); SetWindowClassesDirty(WC_TRAINS_LIST);
}
static void TrainRunningWindows(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	SetWindowDirty(WC_VEHICLE_DETAILS, v->index); SetWindowClassesDirty(WC_TRAINS_LIST);
}
static void TrainService(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	VehicleServiceInDepot(v);
}
static uint64_t TrainServint(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return Company::Get(v->owner)->settings.vehicle.servint_trains;
}
static void TrainSetDepotRes(OpenTTDTrainHandle, uint32_t a, uint8_t b) noexcept
{
	SetDepotReservation(TileIndex(a), b != 0);
}
static void TrainSetNext(OpenTTDTrainHandle id, OpenTTDTrainHandle a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->SetNext(static_cast<Train *>(a.shell));
}
static void TrainSetPlatformRes(OpenTTDTrainHandle, uint32_t a, uint8_t b, uint8_t c) noexcept
{
	SetRailStationPlatformReservation(TileIndex(a), static_cast<DiagDirection>(b), c != 0);
}
static void TrainSetTunnelRes(OpenTTDTrainHandle, uint32_t a, uint8_t b) noexcept
{
	SetTunnelBridgeReservation(TileIndex(a), b != 0);
}
static void TrainShowEffect(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->ShowVisualEffect();
}
static uint64_t TrainShowReservation(OpenTTDTrainHandle) noexcept
{
	return _settings_client.gui.show_track_reservation;
}
static uint64_t TrainSignalHasPbs(OpenTTDTrainHandle, uint32_t a, uint8_t b) noexcept
{
	return HasPbsSignalOnTrackdir(TileIndex(a), static_cast<Trackdir>(b));
}
static uint64_t TrainSignalPbs(OpenTTDTrainHandle, uint8_t a) noexcept
{
	return IsPbsSignal(static_cast<SignalType>(a));
}
static uint64_t TrainSigsegFull(OpenTTDTrainHandle) noexcept
{
	return SIGSEG_FULL;
}
static uint64_t TrainSigsegPbs(OpenTTDTrainHandle) noexcept
{
	return SIGSEG_PBS;
}
static void TrainSmallExplosion(OpenTTDTrainHandle id, int32_t a, int32_t b, int32_t c) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	CreateEffectVehicleRel(v, static_cast<int>(a), static_cast<int>(b), static_cast<int>(c), EV_EXPLOSION_SMALL);
}
static uint64_t TrainSpeedDefault(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return RailVehInfo(v->engine_type)->max_speed;
}
static void TrainStartStopDirty(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	SetWindowWidgetDirty(WC_VEHICLE_VIEW, v->index, WID_VV_START_STOP);
}
static uint64_t TrainStationCompatible(OpenTTDTrainHandle, uint32_t a, uint32_t b) noexcept
{
	return IsCompatibleTrainStationTile(TileIndex(a), TileIndex(b));
}
static uint64_t TrainStationDest(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return Station::Get(v->current_order.GetDestination().ToStationID())->train_station.tile.base();
}
static uint64_t TrainStoppedInDepot(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return v->IsStoppedInDepot();
}
static uint64_t TrainStopLocation(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return to_underlying(v->current_order.GetStopLocation());
}
static void TrainStuckNews(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	AddVehicleAdviceNewsItem(AdviceType::TrainStuck, GetEncodedString(STR_NEWS_TRAIN_IS_STUCK, v->index), v->index);
}
static void TrainSuppressImplicit(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	SetBit(v->gv_flags, GVF_SUPPRESS_IMPLICIT_ORDERS);
}
static uint64_t TrainTicksLeaveDepot(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return TicksToLeaveDepot(v);
}
static uint64_t TrainTileAddDiag(OpenTTDTrainHandle, uint32_t a, uint8_t b) noexcept
{
	return TileAddByDiagDir(TileIndex(a), static_cast<DiagDirection>(b)).base();
}
static uint64_t TrainTileOffsetAxis(OpenTTDTrainHandle, uint8_t a) noexcept
{
	return static_cast<uint32_t>(TileOffsByAxis(static_cast<Axis>(a)));
}
static uint64_t TrainTileOffsetDiag(OpenTTDTrainHandle, uint8_t a) noexcept
{
	return static_cast<uint32_t>(TileOffsByDiagDir(static_cast<DiagDirection>(a)));
}
static uint64_t TrainTileRailType(OpenTTDTrainHandle, uint32_t a) noexcept
{
	return GetTileRailType(TileIndex(a));
}
static uint64_t TrainTileVirt(OpenTTDTrainHandle, int32_t a, int32_t b) noexcept
{
	return TileVirtXY(static_cast<int>(a), static_cast<int>(b)).base();
}
static uint64_t TrainTrackdirExit(OpenTTDTrainHandle, uint8_t a) noexcept
{
	return TrackdirToExitdir(static_cast<Trackdir>(a));
}
static uint64_t TrainTrackdirReaches(OpenTTDTrainHandle, uint8_t a) noexcept
{
	return DiagdirReachesTrackdirs(static_cast<DiagDirection>(a));
}
static uint64_t TrainTrackCrosses(OpenTTDTrainHandle, uint8_t a) noexcept
{
	return TrackCrossesTracks(static_cast<Track>(a));
}
static uint64_t TrainTrackDirection(OpenTTDTrainHandle, uint8_t a, uint8_t b) noexcept
{
	return TrackDirectionToTrackdir(static_cast<Track>(a), static_cast<Direction>(b));
}
static void TrainTrainList(OpenTTDTrainHandle) noexcept
{
	SetWindowClassesDirty(WC_TRAINS_LIST);
}
static uint64_t TrainTrainVisit(OpenTTDTrainHandle) noexcept
{
	return HVOT_TRAIN;
}
static void TrainTruncateCargo(OpenTTDTrainHandle id, uint32_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->cargo.Truncate(static_cast<uint>(a));
}
static uint64_t TrainTryPath(OpenTTDTrainHandle id, uint8_t a, uint8_t b) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return TryPathReserve(v, a != 0, b != 0);
}
static uint64_t TrainTryReserve(OpenTTDTrainHandle, uint32_t a, uint8_t b, uint8_t c) noexcept
{
	return TryReserveRailTrack(TileIndex(a), static_cast<Track>(b), c != 0);
}
static void TrainUnreserve(OpenTTDTrainHandle, uint32_t a, uint8_t b) noexcept
{
	UnreserveRailTrack(TileIndex(a), static_cast<Track>(b));
}
static void TrainUpdateDelta(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->UpdateDeltaXY();
}
static uint64_t TrainUpdateSpeed(OpenTTDTrainHandle id, uint32_t a, int32_t b, int32_t c) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return v->RustDoUpdateSpeed(static_cast<uint>(a), static_cast<int>(b), static_cast<int>(c));
}
static uint64_t TrainUserDefault(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return RailVehInfo(v->engine_type)->user_def_data;
}
static uint64_t TrainVehExitDir(OpenTTDTrainHandle, uint8_t a, uint8_t b) noexcept
{
	return VehicleExitDir(static_cast<Direction>(a), static_cast<TrackBits>(b));
}
static void TrainViewport(OpenTTDTrainHandle id, uint8_t a, uint8_t b) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->UpdateViewport(a != 0, b != 0);
}
static void TrainViewWindow(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	SetWindowDirty(WC_VEHICLE_VIEW, v->index);
}
static uint64_t TrainVisitType(OpenTTDTrainHandle, uint16_t a) noexcept
{
	return Station::Get(StationID(a))->had_vehicle_of_type;
}
static void TrainVisEffect(OpenTTDTrainHandle id, uint8_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->UpdateVisualEffect(a != 0);
}
static uint64_t TrainWagonOverride(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return UsesWagonOverride(v);
}
static uint64_t TrainWagonSpeedLimits(OpenTTDTrainHandle) noexcept
{
	return _settings_game.vehicle.wagon_speed_limits;
}
static uint64_t TrainWaitOneway(OpenTTDTrainHandle) noexcept
{
	return _settings_game.pf.wait_oneway_signal;
}
static uint64_t TrainWaitPbs(OpenTTDTrainHandle) noexcept
{
	return _settings_game.pf.wait_for_pbs_path;
}
static uint64_t TrainWaitTwoway(OpenTTDTrainHandle) noexcept
{
	return _settings_game.pf.wait_twoway_signal;
}
static uint64_t TrainWaitUnbunch(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return v->IsWaitingForUnbunching();
}
static void TrainWriteCrossingBar(OpenTTDTrainHandle, uint32_t a, uint8_t b) noexcept
{
	SetCrossingBarred(TileIndex(a), b != 0);
}
static void TrainWriteCrossingRes(OpenTTDTrainHandle, uint32_t a, uint8_t b) noexcept
{
	SetCrossingReservation(TileIndex(a), b != 0);
}
static void TrainWriteVisitType(OpenTTDTrainHandle, uint16_t a) noexcept
{
	Station::Get(StationID(a))->had_vehicle_of_type |= HVOT_TRAIN;
}
static uint8_t TrainVisitTile(uint32_t tile, void *context, uint8_t (*visit)(void *, OpenTTDTrainHandle)) noexcept
{
	for (const Vehicle *v : VehiclesOnTile(TileIndex(tile))) {
		if (v->type == VEH_TRAIN && visit(context, TrainHandle(Train::From(v))) != 0) return 1;
	}
	return 0;
}
static uint8_t TrainVisitNear(int32_t x, int32_t y, void *context, uint8_t (*visit)(void *, OpenTTDTrainHandle)) noexcept
{
	for (const Vehicle *v : VehiclesNearTileXY(x, y, 7)) {
		if (v->type == VEH_TRAIN && visit(context, TrainHandle(Train::From(v))) != 0) return 1;
	}
	return 0;
}

static OpenTTDTrainHandle TrainReadFirst(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return TrainHandle(v->First());
}
static OpenTTDTrainHandle TrainReadNext(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return TrainHandle(static_cast<Train *>(RustVehicleNext(v)));
}
static OpenTTDTrainHandle TrainReadPrevious(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return TrainHandle(v->Previous());
}
static OpenTTDTrainHandle TrainReadNextUnit(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return TrainHandle(v->GetNextVehicle());
}
static OpenTTDTrainHandle TrainReadLast(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return TrainHandle(v->Last());
}
static int32_t TrainReadOrderTime(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return v->current_order_time;
}
static uint16_t TrainReadTotalLength(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return v->gcache.cached_total_length;
}
static uint16_t TrainReadMaxTrackSpeed(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return v->gcache.cached_max_track_speed;
}
static uint16_t TrainReadGvFlags(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return v->gv_flags;
}
static uint16_t TrainReadRefitCap(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return v->refit_cap;
}
static uint16_t TrainReadLastStation(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return v->last_station_visited.base();
}
static uint8_t TrainReadRunning(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return v->running_ticks;
}
static uint8_t TrainReadFront(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return v->IsFrontEngine();
}
static uint8_t TrainReadArticulated(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return v->IsArticulatedPart();
}
static uint8_t TrainReadMultiheaded(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return v->IsMultiheaded();
}
static uint8_t TrainReadVisEffect(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return v->vcache.cached_vis_effect;
}
static OpenTTDTrainConsistChangedRead TrainReadConsistChanged(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleEngineID(v), v->IsFrontEngine()};
}
static OpenTTDTrainConsistChanged1Read TrainReadConsistChanged1(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleEngineID(v), v->IsEngine()};
}
static OpenTTDTrainConsistChanged2Read TrainReadConsistChanged2(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {v->cargo_cap};
}
static OpenTTDTrainCurveLimitRead TrainReadCurveLimit(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {TrainHandle(static_cast<Train *>(RustVehicleNext(v))), RustVehicleDirection(v)};
}
static OpenTTDTrainStopLocationRead TrainReadStopLocation(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustGroundCachedLength(v), v->gcache.cached_total_length, RustVehicleOrderDestination(v), RustVehicleOrderType(v)};
}
static OpenTTDTrainCurrentMaxSpeedRead TrainReadCurrentMaxSpeed(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleTile(v), v->gcache.cached_max_track_speed, RustVehicleSpeed(v)};
}
static OpenTTDTrainCurrentMaxSpeed6Read TrainReadCurrentMaxSpeed6(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {TrainHandle(static_cast<Train *>(RustVehicleNext(v))), RustVehicleTile(v), RustVehicleStatus(v)};
}
static OpenTTDTrainUpdateAccelerationRead TrainReadUpdateAcceleration(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {v->gcache.cached_power, v->gcache.cached_weight};
}
static OpenTTDTrainUpdateSpeedRead TrainReadUpdateSpeed(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleStatus(v), v->acceleration};
}
static OpenTTDTrainTrackdirRead TrainReadTrackdir(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleTile(v), RustVehicleDirection(v), RustVehicleStatus(v)};
}
static OpenTTDTrainCanLeaveRead TrainReadCanLeave(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleTile(v), RustVehicleDirection(v)};
}
static OpenTTDTrainCrossingApproachRead TrainReadCrossingApproach(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleStatus(v), v->IsFrontEngine()};
}
static OpenTTDTrainNextOffsetRead TrainReadNextOffset(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {TrainHandle(static_cast<Train *>(RustVehicleNext(v))), RustGroundCachedLength(v)};
}
static OpenTTDTrainAfterSwapRead TrainReadAfterSwap(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleTile(v), v->x_pos, v->y_pos};
}
static OpenTTDTrainReverseSwapRead TrainReadReverseSwap(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleTile(v), v->x_pos, v->y_pos, v->z_pos, RustVehicleDirection(v), RustVehicleStatus(v)};
}
static OpenTTDTrainApproachingEndRead TrainReadApproachingEnd(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {v->x_pos, v->y_pos, RustGroundCachedLength(v), RustVehicleSpeed(v), RustVehicleDirection(v)};
}
static OpenTTDTrainLineEndsRead TrainReadLineEnds(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleTile(v), RustVehicleSpeed(v), RustVehicleBreakdownCounter(v)};
}
static OpenTTDTrainSpeedZRead TrainReadSpeedZ(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {v->z_pos, v->gcache.cached_max_track_speed, RustVehicleSpeed(v)};
}
static OpenTTDTrainMoveVehicleRead TrainReadMoveVehicle(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleTile(v), v->x_pos, v->y_pos, RustVehicleDirection(v), v->IsFrontEngine(), v->IsArticulatedPart()};
}
static OpenTTDTrainMoveVehicle20Read TrainReadMoveVehicle20(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleSpeed(v), RustVehicleStatus(v), v->IsFrontEngine()};
}
static OpenTTDTrainCollisionOneRead TrainReadCollisionOne(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {TrainHandle(v->First()), v->x_pos, v->y_pos, v->z_pos, RustGroundCachedLength(v), v->owner.base()};
}
static OpenTTDTrainCollisionOne22Read TrainReadCollisionOne22(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {v->x_pos, v->y_pos, v->z_pos, RustGroundCachedLength(v), v->owner.base()};
}
static OpenTTDTrainDeleteLastRead TrainReadDeleteLast(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleTile(v), v->owner.base()};
}
static OpenTTDTrainStayDepotRead TrainReadStayDepot(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleTile(v), v->gcache.cached_power};
}
static OpenTTDTrainLocoRead TrainReadLoco(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleSpeed(v), RustVehicleStatus(v), RustVehicleOrderType(v)};
}
static OpenTTDTrainLoco26Read TrainReadLoco26(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleTile(v), RustVehicleOrderDestination(v), RustVehicleOrderType(v), RustVehicleOrderNonStop(v)};
}
static OpenTTDTrainTickRead TrainReadTickState(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleSpeed(v), RustVehicleStatus(v), v->running_ticks, v->IsFrontEngine(), v->IsFreeWagon()};
}
static OpenTTDTrainNeedsServiceRead TrainReadNeedsService(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleOrderDestination(v), RustVehicleOrderType(v)};
}
static OpenTTDTrainNextForceRead TrainReadNextForce(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleTile(v), RustVehicleStatus(v)};
}
static OpenTTDTrainReverseCommandRead TrainReadReverseCommand(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleStatus(v), RustVehicleBreakdownCounter(v), v->IsFrontEngine()};
}
