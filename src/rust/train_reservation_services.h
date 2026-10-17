/*
	* This file is part of OpenTTD.
	* OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
	* OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
	* See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
	*/

#include "services_ffi.h"
#include "command_services.h"
#include "services_rail.h"

/** @file train_reservation_services.h Direct map/PBS/follower services. */

template <typename Result, auto Member>
static Result TrainReservationSharedMap(void *, OpenTTDTrainHandle, uint32_t tile) noexcept
{
	return (GetRustSharedServices().*Member)(tile);
}

static uint64_t TrainReservationAllCompat(void *, OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return GetAllCompatibleRailTypes(v->GetRailTypes()).base();
}
static uint64_t TrainReservationBackoff(void *, OpenTTDTrainHandle) noexcept
{
	return _settings_game.pf.path_backoff_interval;
}
static uint64_t TrainReservationBitsTrack(void *, OpenTTDTrainHandle, uint8_t a) noexcept
{
	return TrackBitsToTrack(static_cast<TrackBits>(a));
}
static uint64_t TrainReservationBlocking(void *, OpenTTDTrainHandle, uint32_t a, uint8_t b) noexcept
{
	TileIndex t(static_cast<uint32_t>(a));
	return HasOnewaySignalBlockingTrackdir(t, static_cast<Trackdir>(b));
}
static uint64_t TrainReservationCheckReverse(void *, OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return YapfTrainCheckReverse(v);
}
static uint64_t TrainReservationCompatStation(void *, OpenTTDTrainHandle, uint32_t a, uint32_t b) noexcept
{
	TileIndex t(static_cast<uint32_t>(a));
	return IsCompatibleTrainStationTile(t, TileIndex(static_cast<uint32_t>(b)));
}
static uint64_t TrainReservationConditional(void *, OpenTTDTrainHandle id, uint8_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return ProcessConditionalOrder(v->GetOrder(static_cast<VehicleOrderID>(a)), v);
}
static void TrainReservationCopyOrder(void *, OpenTTDTrainHandle id, uint8_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->current_order = *v->GetOrder(static_cast<VehicleOrderID>(a));
}
static uint64_t TrainReservationCrossDirs(void *, OpenTTDTrainHandle, uint8_t a) noexcept
{
	return TrackdirCrossesTrackdirs(static_cast<Trackdir>(a));
}
static uint64_t TrainReservationCrossTracks(void *, OpenTTDTrainHandle, uint8_t a) noexcept
{
	return TrackCrossesTracks(static_cast<Track>(a));
}
static uint64_t TrainReservationDiagReachDirs(void *, OpenTTDTrainHandle, uint8_t a) noexcept
{
	return DiagdirReachesTrackdirs(static_cast<DiagDirection>(a));
}
static uint64_t TrainReservationDiagTrack(void *, OpenTTDTrainHandle, uint8_t a) noexcept
{
	return DiagDirToDiagTrack(static_cast<DiagDirection>(a));
}
static uint64_t TrainReservationEnterTd(void *, OpenTTDTrainHandle, uint8_t a, uint8_t b) noexcept
{
	return TrackEnterdirToTrackdir(static_cast<Track>(a), static_cast<DiagDirection>(b));
}
static uint64_t TrainReservationExitDir(void *, OpenTTDTrainHandle, uint8_t a) noexcept
{
	return TrackdirToExitdir(static_cast<Trackdir>(a));
}
static uint64_t TrainReservationFree(void *, OpenTTDTrainHandle id, uint32_t a, uint8_t b) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	TileIndex t(static_cast<uint32_t>(a));
	return IsWaitingPositionFree(v, t, static_cast<Trackdir>(b), _settings_game.pf.forbid_90_deg);
}
static uint64_t TrainReservationHasPbs(void *, OpenTTDTrainHandle, uint32_t a, uint8_t b) noexcept
{
	TileIndex t(static_cast<uint32_t>(a));
	return HasPbsSignalOnTrackdir(t, static_cast<Trackdir>(b));
}
static uint64_t TrainReservationHasReserved(void *, OpenTTDTrainHandle, uint32_t a, uint8_t b) noexcept
{
	TileIndex t(static_cast<uint32_t>(a));
	return HasReservedTracks(t, static_cast<TrackBits>(b));
}
static void TrainReservationIncrementOrder(void *, OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->IncrementRealOrderIndex();
}
static uint64_t TrainReservationIsBridge(void *, OpenTTDTrainHandle, uint32_t a) noexcept
{
	TileIndex t(static_cast<uint32_t>(a));
	return IsBridge(t);
}
static uint64_t TrainReservationIsDepot(void *, OpenTTDTrainHandle, uint32_t a) noexcept
{
	TileIndex t(static_cast<uint32_t>(a));
	return IsRailDepotTile(t);
}
static uint64_t TrainReservationIsPbs(void *, OpenTTDTrainHandle, uint32_t a, uint8_t b) noexcept
{
	return IsPbsSignal(RustRailSignalType(a, b));
}
static uint64_t TrainReservationIsPlain(void *, OpenTTDTrainHandle, uint32_t a) noexcept
{
	TileIndex t(static_cast<uint32_t>(a));
	return IsPlainRail(t);
}
static uint64_t TrainReservationIsRailway(void *, OpenTTDTrainHandle, uint32_t a) noexcept
{
	TileIndex t(static_cast<uint32_t>(a));
	return IsTileType(t, MP_RAILWAY);
}
static uint64_t TrainReservationIsTunnel(void *, OpenTTDTrainHandle, uint32_t a) noexcept
{
	TileIndex t(static_cast<uint32_t>(a));
	return IsTileType(t, MP_TUNNELBRIDGE);
}
static uint64_t TrainReservationIsWaypoint(void *, OpenTTDTrainHandle, uint32_t a) noexcept
{
	TileIndex t(static_cast<uint32_t>(a));
	return IsRailWaypointTile(t);
}
static uint64_t TrainReservationLineReverse(void *, OpenTTDTrainHandle) noexcept
{
	return _settings_game.difficulty.line_reverse_mode;
}
static uint64_t TrainReservationNeedsService(void *, OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return v->NeedsServicing();
}
static uint64_t TrainReservationOneway(void *, OpenTTDTrainHandle, uint32_t a, uint8_t b) noexcept
{
	TileIndex t(static_cast<uint32_t>(a));
	return IsOnewaySignal(t, static_cast<Track>(b));
}
static uint64_t TrainReservationOrderService(void *, OpenTTDTrainHandle id, uint8_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return v->GetOrder(static_cast<VehicleOrderID>(a))->GetDepotOrderType().Test(OrderDepotTypeFlag::Service);
}
static uint64_t TrainReservationOrderStop(void *, OpenTTDTrainHandle id, uint16_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return v->current_order.ShouldStopAtStation(v, StationID(static_cast<uint16_t>(a)));
}
static uint64_t TrainReservationOrderType(void *, OpenTTDTrainHandle id, uint8_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return v->GetOrder(static_cast<VehicleOrderID>(a))->GetType();
}
static uint64_t TrainReservationOverlap(void *, OpenTTDTrainHandle, uint8_t a) noexcept
{
	return TracksOverlap(static_cast<TrackBits>(a));
}
static void TrainReservationPathResult(void *, OpenTTDTrainHandle id, uint8_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->HandlePathfindingResult(a != 0);
}
static uint64_t TrainReservationRail90(void *, OpenTTDTrainHandle, uint32_t a, uint32_t b) noexcept
{
	TileIndex t(static_cast<uint32_t>(a));
	return Rail90DegTurnDisallowed(GetTileRailType(t), GetTileRailType(TileIndex(static_cast<uint32_t>(b))));
}
static uint64_t TrainReservationReachDirs(void *, OpenTTDTrainHandle, uint8_t a) noexcept
{
	return TrackdirReachesTrackdirs(static_cast<Trackdir>(a));
}
static uint64_t TrainReservationReachTracks(void *, OpenTTDTrainHandle, uint8_t a) noexcept
{
	return DiagdirReachesTracks(static_cast<DiagDirection>(a));
}
static uint64_t TrainReservationReservePaths(void *, OpenTTDTrainHandle) noexcept
{
	return _settings_game.pf.reserve_paths;
}
static void TrainReservationRestoreOrder(void *context, OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	auto *ctx = static_cast<TrainReservationContext *>(context);
	v->current_order = *ctx->saved_order;
}
static void TrainReservationSaveOrder(void *context, OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	auto *ctx = static_cast<TrainReservationContext *>(context);
	ctx->saved_order.emplace(v->current_order);
}
static void TrainReservationService(void *, OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	CheckIfTrainNeedsService(v);
}
static void TrainReservationSetDepot(void *, OpenTTDTrainHandle, uint32_t a, uint8_t b) noexcept
{
	TileIndex t(static_cast<uint32_t>(a));
	SetDepotReservation(t, b != 0);
}
static void TrainReservationSetDepotDest(void *, OpenTTDTrainHandle id, uint32_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	TileIndex t(static_cast<uint32_t>(a));
	v->current_order.SetDestination(GetDepotIndex(t));
}
static void TrainReservationSetPlatform(void *, OpenTTDTrainHandle, uint32_t a, uint8_t b, uint8_t c) noexcept
{
	TileIndex t(static_cast<uint32_t>(a));
	SetRailStationPlatformReservation(t, static_cast<DiagDirection>(b), c != 0);
}
static void TrainReservationSetTunnel(void *, OpenTTDTrainHandle, uint32_t a, uint8_t b) noexcept
{
	TileIndex t(static_cast<uint32_t>(a));
	SetTunnelBridgeReservation(t, b != 0);
}
static uint64_t TrainReservationShowRes(void *, OpenTTDTrainHandle) noexcept
{
	return _settings_client.gui.show_track_reservation;
}
static void TrainReservationStartStop(void *, OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	SetWindowWidgetDirty(WC_VEHICLE_VIEW, v->index, WID_VV_START_STOP);
}
static uint64_t TrainReservationStationTrain(void *, OpenTTDTrainHandle, uint16_t a) noexcept
{
	return Station::Get(StationID(static_cast<uint16_t>(a)))->facilities.Test(StationFacility::Train);
}
static uint64_t TrainReservationStationXy(void *, OpenTTDTrainHandle, uint16_t a) noexcept
{
	return Station::Get(StationID(static_cast<uint16_t>(a)))->xy.base();
}
static void TrainReservationStuck(void *, OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	MarkTrainAsStuck(v);
}
static uint64_t TrainReservationTileAdd(void *, OpenTTDTrainHandle, uint32_t a, uint8_t b) noexcept
{
	TileIndex t(static_cast<uint32_t>(a));
	return TileAddByDiagDir(t, static_cast<DiagDirection>(b)).base();
}
static uint64_t TrainReservationTileOffset(void *, OpenTTDTrainHandle, uint8_t a) noexcept
{
	return static_cast<uint64_t>(static_cast<int64_t>(TileOffsByDiagDir(static_cast<DiagDirection>(a))));
}
static uint64_t TrainReservationTrackdir(void *, OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	return GetRustSharedServices().train_trackdir(v);
}
static uint64_t TrainReservationTrackStatus(void *, OpenTTDTrainHandle, uint32_t a) noexcept
{
	return TrackStatusToTrackdirBits(RustRailTrackStatus(a, INVALID_DIAGDIR));
}
static uint64_t TrainReservationTryTrack(void *, OpenTTDTrainHandle, uint32_t a, uint8_t b) noexcept
{
	TileIndex t(static_cast<uint32_t>(a));
	return TryReserveRailTrack(t, static_cast<Track>(b));
}
static uint64_t TrainReservationTunnelFree(void *, OpenTTDTrainHandle id, uint32_t a, uint32_t b) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	TileIndex t(static_cast<uint32_t>(a));
	CommandCost result;
	GetRustCommandServices().wormhole_free(RustCommandCost(&result), t.base(), b, v);
	return result.Succeeded();
}
static void TrainReservationUnreserve(void *, OpenTTDTrainHandle, uint32_t a, uint8_t b) noexcept
{
	TileIndex t(static_cast<uint32_t>(a));
	UnreserveRailTrack(t, static_cast<Track>(b));
}
static void TrainReservationWriteDest(void *, OpenTTDTrainHandle id, uint32_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	TileIndex t(static_cast<uint32_t>(a));
	v->dest_tile = t;
}
static void TrainReservationWriteLast(void *, OpenTTDTrainHandle id, uint16_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	v->last_station_visited = StationID(static_cast<uint16_t>(a));
}
static void TrainReservationWriteSuppress(void *, OpenTTDTrainHandle id, uint8_t a) noexcept
{
	Train *v = static_cast<Train *>(id.shell);
	AssignBit(v->gv_flags, GVF_SUPPRESS_IMPLICIT_ORDERS, a != 0);
}
static uint8_t TrainReservationFollow(OpenTTDTrainHandle id, uint64_t railtypes, OpenTTDTrainReservationFollow *out) noexcept
{
	CFollowTrackRail ft(static_cast<Train *>(id.shell), RailTypes(railtypes));
	ft.new_tile = TileIndex(out->new_tile); ft.new_td_bits = static_cast<TrackdirBits>(out->dirs);
	ft.exitdir = static_cast<DiagDirection>(out->exitdir); ft.is_tunnel = out->tunnel;
	ft.is_bridge = out->bridge; ft.is_station = out->station; ft.tiles_skipped = out->skipped;
	bool followed = ft.Follow(TileIndex(out->old_tile), static_cast<Trackdir>(out->old_td));
	*out = {ft.old_tile.base(), ft.new_tile.base(), ft.tiles_skipped, ft.new_td_bits,
	ft.old_td, ft.exitdir, static_cast<uint8_t>(ft.is_tunnel), static_cast<uint8_t>(ft.is_bridge),
	static_cast<uint8_t>(ft.is_station), ft.err};
	return followed;
}
static OpenTTDTrainReservationPbs TrainReservationOrigin(OpenTTDTrainHandle id, uint8_t check_other) noexcept
{
	Vehicle *other = nullptr;
	PBSTileInfo pbs = FollowTrainReservation(static_cast<Train *>(id.shell), check_other ? &other : nullptr);
	return {pbs.tile.base(), TrainHandle(other == nullptr ? nullptr : Train::From(other)), pbs.trackdir, static_cast<uint8_t>(pbs.okay)};
}

static OpenTTDTrainReservationSearch TrainReservationPathfind(OpenTTDTrainHandle id, uint32_t tile, uint8_t dir, uint8_t tracks, uint8_t reserve, uint8_t want_final) noexcept
{
	Train *v=static_cast<Train *>(id.shell); bool found=true; PBSTileInfo dest; TileIndex final_dest=INVALID_TILE;
	Track track=YapfTrainChooseTrack(v,TileIndex(tile),static_cast<DiagDirection>(dir),static_cast<TrackBits>(tracks),found,reserve != 0,&dest,want_final ? &final_dest:nullptr);
	OpenTTDTrainReservationSearch result{}; result.track=track;result.found=found;result.tile=dest.tile.base();result.td=dest.trackdir;result.okay=dest.okay;result.final_dest=final_dest.base();return result;
}
static uint8_t TrainReservationSafeTrack(OpenTTDTrainHandle id, uint32_t tile, uint8_t td, uint8_t override_types) noexcept
{return YapfTrainFindNearestSafeTile(static_cast<Train *>(id.shell),TileIndex(tile),static_cast<Trackdir>(td),override_types != 0);}
static uint8_t TrainReservationProcessOrders(OpenTTDTrainHandle id) noexcept
{return ProcessOrders(static_cast<Train *>(id.shell));}
static uint8_t TrainReservationUpdateOrderDest(OpenTTDTrainHandle id, uint8_t index) noexcept
{Train *v=static_cast<Train *>(id.shell);return UpdateOrderDest(v,v->GetOrder(static_cast<VehicleOrderID>(index)),0,true);}

static uint8_t TrainReservationReadNumOrders(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return v->GetNumOrders();
}
static uint8_t TrainReservationReadOrderIndex(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return v->cur_real_order_index;
}
static OpenTTDTrainReservationFreeRead TrainReservationReadFree(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleTile(v)};
}
static OpenTTDTrainReservationFree1Read TrainReservationReadFree1(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleTile(v), TrainHandle(static_cast<Train *>(RustVehicleNext(v)))};
}
static OpenTTDTrainReservationNewRead TrainReservationReadNew(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleDestTile(v), v->last_station_visited.base(), v->cur_real_order_index, static_cast<uint8_t>(HasBit(v->gv_flags, GVF_SUPPRESS_IMPLICIT_ORDERS))};
}
static OpenTTDTrainReservationChooseRead TrainReservationReadChoose(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleTile(v), RustVehicleDestTile(v), RustVehicleOrderDestination(v), RustVehicleOrderType(v)};
}
static OpenTTDTrainReservationChoose4Read TrainReservationReadChoose4(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleOrderType(v), static_cast<uint8_t>(v->current_order.GetDepotActionType().Test(OrderDepotActionFlag::NearestDepot))};
}
static OpenTTDTrainReservationCheckNextRead TrainReservationReadCheckNext(OpenTTDTrainHandle id) noexcept
{
	const Train *v=static_cast<Train *>(id.shell);
	return {RustVehicleTile(v), RustVehicleDestTile(v), RustVehicleOrderDestination(v), RustVehicleOrderType(v), v->GetNumOrders()};
}
