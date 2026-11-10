/*
	* This file is part of OpenTTD.
	* OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
	* OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
	* See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
	*/

#include "services_landscape.h"
#include "services_ffi.h"
#include "command_services.h"
#include "services_rail.h"
#include "services_station.h"

/** @file train_reservation_services.h Direct map/PBS/follower services. */

static uint64_t TrainReservationBitsTrack(void *, OpenTTDTrainHandle, uint8_t a) noexcept
{
	return TrackBitsToTrack(static_cast<TrackBits>(a));
}

static uint64_t TrainReservationCheckReverse(void *, OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	return YapfTrainCheckReverse(v);
}
static uint64_t TrainReservationCompatStation(void *, OpenTTDTrainHandle, uint32_t a, uint32_t b) noexcept
{
	TileIndex t(static_cast<uint32_t>(a));
	return RustCompatibleStationTiles(t.base(), b);
}
static uint64_t TrainReservationConditional(void *, OpenTTDTrainHandle id, uint8_t a) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	return ProcessConditionalOrder(v->GetOrder(static_cast<VehicleOrderID>(a)), v);
}
static void TrainReservationCopyOrder(void *, OpenTTDTrainHandle id, uint8_t a) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
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



static void TrainReservationIncrementOrder(void *, OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	v->IncrementRealOrderIndex();
}

static uint64_t TrainReservationIsPbs(void *, OpenTTDTrainHandle, uint32_t a, uint8_t b) noexcept
{
	return IsPbsSignal(RustRailSignalType(a, b));
}

static uint64_t TrainReservationIsWaypoint(void *, OpenTTDTrainHandle, uint32_t a) noexcept
{
	TileIndex t(static_cast<uint32_t>(a));
	return IsRailWaypointTile(t);
}
static uint64_t TrainReservationOrderService(void *, OpenTTDTrainHandle id, uint8_t a) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	return v->GetOrder(static_cast<VehicleOrderID>(a))->GetDepotOrderType().Test(OrderDepotTypeFlag::Service);
}

static uint64_t TrainReservationOrderType(void *, OpenTTDTrainHandle id, uint8_t a) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	return v->GetOrder(static_cast<VehicleOrderID>(a))->GetType();
}
static uint64_t TrainReservationOverlap(void *, OpenTTDTrainHandle, uint8_t a) noexcept
{
	return TracksOverlap(static_cast<TrackBits>(a));
}
static void TrainReservationPathResult(void *, OpenTTDTrainHandle id, uint8_t a) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	v->HandlePathfindingResult(a != 0);
}
static uint64_t TrainReservationReachDirs(void *, OpenTTDTrainHandle, uint8_t a) noexcept
{
	return TrackdirReachesTrackdirs(static_cast<Trackdir>(a));
}
static uint64_t TrainReservationReachTracks(void *, OpenTTDTrainHandle, uint8_t a) noexcept
{
	return DiagdirReachesTracks(static_cast<DiagDirection>(a));
}
/* Order has the existing OrderFields layout and trivial copy/destruction. Rust
 * owns each nested snapshot on its stack; no native context survives a call. */
static void TrainReservationService(void *, OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	CheckIfTrainNeedsService(v);
}


static void TrainReservationStartStop(void *, OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	SetWindowWidgetDirty(WC_VEHICLE_VIEW, v->index, WID_VV_START_STOP);
}
static void TrainReservationStuck(void *, OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	MarkTrainAsStuck(v);
}

static uint64_t TrainReservationTileOffset(void *, OpenTTDTrainHandle, uint8_t a) noexcept
{
	return static_cast<uint64_t>(static_cast<int64_t>(TileOffsByDiagDir(static_cast<DiagDirection>(a))));
}
static uint64_t TrainReservationTrackdir(void *, OpenTTDTrainHandle id) noexcept
{
	Train *v = static_cast<Train *>(VehicleFromRecord(id.record));
	return GetRustSharedServices().train_trackdir(v);
}



static OpenTTDTrainReservationSearch TrainReservationPathfind(OpenTTDTrainHandle id, uint32_t tile, uint8_t dir, uint8_t tracks, uint8_t reserve, uint8_t want_final) noexcept
{
	Train *v=static_cast<Train *>(VehicleFromRecord(id.record)); bool found=true; PBSTileInfo dest; TileIndex final_dest=INVALID_TILE;
	Track track=YapfTrainChooseTrack(v,TileIndex(tile),static_cast<DiagDirection>(dir),static_cast<TrackBits>(tracks),found,reserve != 0,&dest,want_final ? &final_dest:nullptr);
	OpenTTDTrainReservationSearch result{}; result.track=track;result.found=found;result.tile=dest.tile.base();result.td=dest.trackdir;result.okay=dest.okay;result.final_dest=final_dest.base();return result;
}
static uint8_t TrainReservationSafeTrack(OpenTTDTrainHandle id, uint32_t tile, uint8_t td, uint8_t override_types) noexcept
{return YapfTrainFindNearestSafeTile(static_cast<Train *>(VehicleFromRecord(id.record)),TileIndex(tile),static_cast<Trackdir>(td),override_types != 0);}
static uint8_t TrainReservationProcessOrders(OpenTTDTrainHandle id) noexcept
{return ProcessOrders(static_cast<Train *>(VehicleFromRecord(id.record)));}
static uint8_t TrainReservationUpdateOrderDest(OpenTTDTrainHandle id, uint8_t index) noexcept
{Train *v=static_cast<Train *>(VehicleFromRecord(id.record));return UpdateOrderDest(v,v->GetOrder(static_cast<VehicleOrderID>(index)),0,true);}
