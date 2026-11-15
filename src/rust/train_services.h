/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

#include "services_road.h"
#include "services_ffi.h"
#include "services_rail.h"
#include "services_map.h"
#include "services_station.h"

/** @file train_services.h Direct shared-world operations; no train policy. */

#include "vehicle_services.h"
#include "engine_services.h"
#include "services_newgrf.h"
#include "services_landscape.h"
#include "settings_services.h"

static_assert(to_underlying(VehicleCallbackMask::Length) == 1);
static_assert(to_underlying(EngineMiscFlag::RailTilts) == 0);

template <typename Result, auto Read>
static Result TrainVehicleRead(OpenTTDTrainHandle id) noexcept
{
	return Read(static_cast<const Train *>(VehicleFromRecord(id.record)));
}

template <auto Operation>
static void TrainVehicleInvoke(OpenTTDTrainHandle id) noexcept
{
	Operation(static_cast<Train *>(VehicleFromRecord(id.record)));
}

static void TrainWriteColourmap(OpenTTDTrainHandle id, uint64_t value) noexcept
{Train *v=static_cast<Train *>(VehicleFromRecord(id.record)); v->colourmap = static_cast<PaletteID>(value);}
static uint64_t TrainAdvanceDistance(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	return v->GetAdvanceDistance();
}
static uint64_t TrainAllPowered(OpenTTDTrainHandle, uint64_t a) noexcept
{
	return GetAllPoweredRailTypes(RailTypes(a)).base();
}
static void TrainArrivalNews(OpenTTDTrainHandle id, uint16_t a) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	Station *st = Station::Get(StationID(a)); AddVehicleNewsItem(GetEncodedString(STR_NEWS_FIRST_TRAIN_ARRIVAL, st->index), v->GetOwner() == _local_company ? NewsType::ArrivalCompany : NewsType::ArrivalOther, v->index, st->index); AI::NewEvent(v->GetOwner(), new ScriptEventStationFirstVehicle(st->index, v->index)); Game::NewEvent(new ScriptEventStationFirstVehicle(st->index, v->index));
}
static uint64_t TrainAxisDiag(OpenTTDTrainHandle, uint8_t a) noexcept
{
	return AxisToDiagDir(static_cast<Axis>(a));
}
static void TrainBaseViewport(OpenTTDTrainHandle id, uint8_t a) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	v->Vehicle::UpdateViewport(a != 0);
}
static void TrainCapacityError(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	RustVehicleShowNewGRFError(v->GetEngineType(), STR_NEWGRF_BROKEN, STR_NEWGRF_BROKEN_CAPACITY, GRFBug::VehCapacity, true);
}
static void TrainCheckNext(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	CheckNextTrainTile(v);
}
static void TrainCheckOrders(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	CheckOrders(v);
}
static uint64_t TrainCheckReverse(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	return CheckReverseTrain(v);
}
static uint64_t TrainChooseTrack(OpenTTDTrainHandle id, uint32_t a, uint8_t b, uint8_t c) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	return ChooseTrainTrack(v, TileIndex(a), static_cast<DiagDirection>(b), static_cast<TrackBits>(c), false, nullptr, true);
}
static void TrainClearReservation(OpenTTDTrainHandle id, uint32_t a, uint8_t b) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	ClearPathReservation(v, TileIndex(a), static_cast<Trackdir>(b));
}
static void TrainConsistWindows(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	RustVehicleDetailsDirty(RustVehicle(v)); InvalidateWindowData(WC_VEHICLE_REFIT, v->index, VIWD_CONSIST_CHANGED); InvalidateWindowData(WC_VEHICLE_ORDERS, v->index, VIWD_CONSIST_CHANGED); InvalidateNewGRFInspectWindow(GSF_TRAINS, v->index); InvalidateWindowData(WC_VEHICLE_VIEW, v->index, VIWD_CONSIST_CHANGED);
}


static uint64_t TrainCostDivisor(OpenTTDTrainHandle) noexcept
{
	return CalendarTime::DAYS_IN_YEAR * Ticks::DAY_TICKS;
}
static uint64_t TrainCountChain(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	return CountVehiclesInChain(v);
}
static void TrainCrashEvent(OpenTTDTrainHandle id, uint32_t a) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	AI::NewEvent(v->GetOwner(), new ScriptEventVehicleCrashed(v->index, v->GetTile(), ScriptEventVehicleCrashed::CRASH_TRAIN, static_cast<uint>(a), v->GetOwner())); Game::NewEvent(new ScriptEventVehicleCrashed(v->index, v->GetTile(), ScriptEventVehicleCrashed::CRASH_TRAIN, static_cast<uint>(a), v->GetOwner()));
}
static void TrainCrashNews(OpenTTDTrainHandle id, uint32_t a) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	AddTileNewsItem(GetEncodedString(STR_NEWS_TRAIN_CRASH, static_cast<uint>(a)), NewsType::Accident, v->GetTile());
}
static void TrainCrashSound(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	SndPlayVehicleFx(SND_13_TRAIN_COLLISION, v);
}

static uint64_t TrainCrossingRailAxis(OpenTTDTrainHandle, uint32_t a) noexcept
{
	return GetCrossingRailAxis(TileIndex(a));
}
static uint64_t TrainCurveAdvantage(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	return GetRailTypeInfo(RustRailType(v->GetTile().base()))->curve_speed;
}
static uint64_t TrainDayTicks(OpenTTDTrainHandle) noexcept
{
	return Ticks::DAY_TICKS;
}
static void TrainDeleteVehicle(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	delete v;
}
static void TrainDepotDirty(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	RustVehicleInvalidateDepot(v);
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
static void TrainDisconnect(OpenTTDTrainHandle) noexcept
{
	FatalError("Disconnecting train");
}
static OpenTTDTrainDepot TrainFindDepot(OpenTTDTrainHandle id, int32_t a) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	auto result = FindClosestTrainDepot(v, static_cast<int>(a)); return {result.tile.base(), result.best_length};
}
static uint64_t TrainFirstTrack(OpenTTDTrainHandle, uint8_t a) noexcept
{
	return FindFirstTrack(static_cast<TrackBits>(a));
}

static void TrainHideFill(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	RustVehicleHideFilling(RustVehicle(v));
}

static uint64_t TrainIsBridge(OpenTTDTrainHandle, uint32_t a) noexcept
{
	return IsBridgeTile(TileIndex(a));
}

static uint64_t TrainIsStationAny(OpenTTDTrainHandle, uint32_t a) noexcept
{
	return IsTileType(TileIndex(a), MP_STATION);
}
static void TrainLeaveSound(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	RustVehiclePlayLeaveSound(RustVehicle(v));
}
static void TrainLeaveUnbunch(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	v->LeaveUnbunchingDepot();
}
static uint64_t TrainLocalCompany(OpenTTDTrainHandle) noexcept
{
	return _local_company.base();
}

static void TrainOrderDepotService(OpenTTDTrainHandle id, uint16_t a) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	v->current_order.MakeGoToDepot(DepotID(a), OrderDepotTypeFlag::Service, OrderNonStopFlag::NoIntermediate, OrderDepotActionFlag::NearestDepot);
}
static void TrainOrderDummy(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	v->current_order.MakeDummy();
}
static void TrainOrderFree(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	v->current_order.Free();
}

static void TrainPayRunning(OpenTTDTrainHandle id, int64_t a) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	CommandCost cost(EXPENSES_TRAIN_RUN, Money(static_cast<int64_t>(a))); v->profit_this_year -= cost.GetCost(); SubtractMoneyFromCompanyFract(v->GetOwner(), cost);
}
static uint64_t TrainPbsSignalType(OpenTTDTrainHandle) noexcept
{
	return SIGTYPE_PBS;
}


static void TrainReserveUnder(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	v->ReserveTrackUnderConsist();
}
static void TrainResetUnbunch(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	v->ResetDepotUnbunching();
}
static void TrainReverseWindows(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	RustVehicleDirtyDepot(v); RustVehicleDetailsDirty(RustVehicle(v)); SetWindowDirty(WC_VEHICLE_VIEW, v->index); SetWindowClassesDirty(WC_TRAINS_LIST);
}
static void TrainRunningWindows(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	RustVehicleDetailsDirty(RustVehicle(v)); SetWindowClassesDirty(WC_TRAINS_LIST);
}
static uint64_t TrainServint(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	return Company::Get(v->GetOwner())->settings.vehicle.servint_trains;
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
static void TrainStartStopDirty(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	SetWindowWidgetDirty(WC_VEHICLE_VIEW, v->index, WID_VV_START_STOP);
}
static uint64_t TrainStationDest(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	return RustStationTrainTile(Station::Get(v->current_order.GetDestination().ToStationID()));
}
static uint64_t TrainStoppedInDepot(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	return v->IsStoppedInDepot();
}
static uint64_t TrainStopLocation(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	return to_underlying(v->current_order.GetStopLocation());
}
static void TrainStuckNews(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	AddVehicleAdviceNewsItem(AdviceType::TrainStuck, GetEncodedString(STR_NEWS_TRAIN_IS_STUCK, v->index), v->index);
}
static void TrainSuppressImplicit(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	SetBit(v->gv_flags, GVF_SUPPRESS_IMPLICIT_ORDERS);
}


static uint64_t TrainTileOffsetAxis(OpenTTDTrainHandle, uint8_t a) noexcept
{
	return static_cast<uint32_t>(TileOffsByAxis(static_cast<Axis>(a)));
}
static uint64_t TrainTileOffsetDiag(OpenTTDTrainHandle, uint8_t a) noexcept
{
	return static_cast<uint32_t>(TileOffsByDiagDir(static_cast<DiagDirection>(a)));
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
static void TrainTruncateCargo(OpenTTDTrainHandle id, uint32_t a) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	v->cargo.Truncate(static_cast<uint>(a));
}


static uint64_t TrainVehExitDir(OpenTTDTrainHandle, uint8_t a, uint8_t b) noexcept
{
	return VehicleExitDir(static_cast<Direction>(a), static_cast<TrackBits>(b));
}
static void TrainViewport(OpenTTDTrainHandle id, uint8_t a, uint8_t b) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	RustVehicleUpdateViewport(v, a != 0, b != 0);
}

static uint64_t TrainWaitUnbunch(OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	return v->IsWaitingForUnbunching();
}
static void TrainWriteCrossingBar(OpenTTDTrainHandle, uint32_t a, uint8_t b) noexcept
{
	SetCrossingBarred(TileIndex(a), b != 0);
}
