/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file pbs.cpp PBS support routines */

#include "stdafx.h"
#include "viewport_func.h"
#include "vehicle_func.h"
#include "newgrf_station.h"
#include "pathfinder/follow_track.hpp"

#include "rust/pbs_ffi.h"
#include "rust/services_map.h"
#include "rust/services_rail.h"
#include "rust/services_station.h"

#include "safeguards.h"

const OpenTTDPbsServices &GetPbsServices() noexcept
{
	static const OpenTTDPbsServices s{
		.shared = &GetRustSharedServices(),
		.rail = &GetRustRailMapServices(),
		.train = &GetTrainServices(),
		.depot_bits = [](uint32_t t) noexcept { return GetDepotReservationTrackBits(TileIndex{t}); },
		.crossing_bits = [](uint32_t t) noexcept { return GetCrossingReservationTrackBits(TileIndex{t}); },
		.station_bits = [](uint32_t t) noexcept { return GetStationReservationTrackBits(TileIndex{t}); },
		.wormhole_bits = [](uint32_t t) noexcept { return GetTunnelBridgeReservationTrackBits(TileIndex{t}); },
		.set_depot = RustSetDepotReservation,
		.set_station = RustSetRailStationReservation,
		.set_wormhole = RustSetTunnelBridgeReservation,
		.try_track = [](uint32_t t, uint8_t track) noexcept { return TryReserveTrack(TileIndex{t}, static_cast<Track>(track)); },
		.unreserve_track = [](uint32_t t, uint8_t track) noexcept { UnreserveTrack(TileIndex{t}, static_cast<Track>(track)); },
		.show_reservation = RustShowTrackReservation,
		.assert_platform = []([[maybe_unused]] uint32_t tile, [[maybe_unused]] uint8_t dir) noexcept { assert(IsRailStationTile(TileIndex{tile})); assert(GetRailStationAxis(TileIndex{tile}) == DiagDirToAxis(static_cast<DiagDirection>(dir))); },
		.station = RustBaseStationByTile,

		.is_crossing = MapSvcRoadIsLevelCrossing,
		.read_train_on_res = [](const OpenTTDPbsTrainOutput *slot) noexcept -> OpenTTDTrainHandle { return {*reinterpret_cast<Vehicle *const *>(slot), nullptr}; },
		.write_train_on_res = [](OpenTTDPbsTrainOutput *slot, OpenTTDTrainHandle v) noexcept { *reinterpret_cast<Vehicle **>(slot) = static_cast<Train *>(v.shell); },
#if !defined(NDEBUG) || defined(WITH_ASSERT)
		.assertions = true,
#else
		.assertions = false,
#endif
		.handle = TrainHandle,
		.all_compatible = RustAllCompatibleRailTypes,
		.compatible = [](uint8_t rt) noexcept { return GetRailTypeInfo(static_cast<RailType>(rt))->compatible_railtypes.base(); },
		.rail90 = RustRail90Disallowed,
		.blocking = RustOnewaySignalBlocking,
		.has_pbs = RustHasPbsSignal,
		.index = RustVehicleIndexOpaque,
	};
	return s;
}
TrackBits GetReservedTrackbits(TileIndex t) { return static_cast<TrackBits>(openttd_rust_pbs_reserved(&GetPbsServices(), t.base())); }
void SetRailStationPlatformReservation(TileIndex t, DiagDirection dir, bool b) { openttd_rust_pbs_platform(&GetPbsServices(), t.base(), dir, b); }
bool TryReserveRailTrack(TileIndex tile, Track t, bool trigger) { return openttd_rust_pbs_try_reserve(&GetPbsServices(), tile.base(), t, trigger); }
void UnreserveRailTrack(TileIndex tile, Track t) { openttd_rust_pbs_unreserve(&GetPbsServices(), tile.base(), t); }

PBSTileInfo FollowTrainReservation(const Train *v, Vehicle **other)
{
	assert(v->GetType() == VEH_TRAIN);
	auto result = openttd_rust_pbs_follow_train(&GetPbsServices(), TrainHandle(v), reinterpret_cast<OpenTTDPbsTrainOutput *>(other));
	return {TileIndex{result.tile}, static_cast<Trackdir>(result.td), result.okay != 0};
}
Train *GetTrainForReservation(TileIndex tile, Track track) { return static_cast<Train *>(openttd_rust_pbs_train_for(&GetPbsServices(), tile.base(), track).shell); }
bool IsSafeWaitingPosition(const Train *v, TileIndex tile, Trackdir td, bool end, bool no90) { return openttd_rust_pbs_safe(&GetPbsServices(), TrainHandle(v), tile.base(), td, end, no90); }
bool IsWaitingPositionFree(const Train *v, TileIndex tile, Trackdir td, bool no90) { return openttd_rust_pbs_free(&GetPbsServices(), TrainHandle(v), tile.base(), td, no90); }
