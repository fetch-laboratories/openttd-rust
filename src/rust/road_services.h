/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

#include "road_command_services.h"
#include "services_road.h"

/** @file road_services.h Typed road services; each resolves its vehicle once and reads only what it returns. */
#include "vehicle_effect_ffi.h"
#include "effect_ffi.h"
#include "services_map.h"
#include "engine_services.h"
#include "services_road.h"
#include "services_station.h"

static_assert(RVS_USING_SECOND_BAY == 1 && RVS_IN_ROAD_STOP == 5 && RVS_IN_DT_ROAD_STOP == 6);
static_assert(RVSB_IN_DEPOT == 254 && RVSB_WORMHOLE == 255 && RVSB_IN_ROAD_STOP == 32 && RVSB_IN_DT_ROAD_STOP == 64);
static_assert(to_underlying(RoadStopType::Bus) == 0 && to_underlying(RoadStopType::Truck) == 1);
static_assert(PROP_ROADVEH_SHORTEN_FACTOR == 0x23 && PROP_ROADVEH_CARGO_AGE_PERIOD == 0x22 && PROP_ROADVEH_SPEED == 0x15 && PROP_ROADVEH_RUNNING_COST_FACTOR == 0x09);

static RoadVehicle *RoadOf(OpenTTDVehicle *v) noexcept { return RoadVehicle::From(VehicleOf(v)); }



static uint8_t OPENTTD_RUST_CALL RoadSvcAnyRoadBits(OpenTTDVehicle *v, uint32_t tile, bool straight_only) noexcept
{
	return GetAnyRoadBits(TileIndex(tile), RustRoadTramType(RustRoadVehicleType(RoadOf(v))), straight_only);
}
static bool OPENTTD_RUST_CALL RoadSvcIsStationRoadStop(uint32_t tile) noexcept { return IsStationRoadStop(TileIndex(tile)); }
static OpenTTDRoadTrackChoice OPENTTD_RUST_CALL RoadSvcChooseTrack(OpenTTDVehicle *v, uint32_t tile, uint8_t entry, uint16_t trackdirs) noexcept
{
	bool found = true;
	Trackdir trackdir = YapfRoadVehicleChooseTrack(RoadOf(v), TileIndex(tile), static_cast<DiagDirection>(entry), static_cast<TrackdirBits>(trackdirs), found);
	return {.trackdir = static_cast<uint8_t>(trackdir), .found = found};
}
static OpenTTDRoadDepotResult OPENTTD_RUST_CALL RoadSvcFindDepot(OpenTTDVehicle *v, int32_t max_distance) noexcept
{
	FindDepotData result = YapfRoadVehicleFindNearestDepot(RoadOf(v), max_distance);
	return {.tile = result.tile.base(), .length = result.best_length};
}
static OpenTTDRoadTramBuildRead OPENTTD_RUST_CALL RoadSvcTramBuildRead(OpenTTDVehicle *v, OpenTTDRoadCommandSlot *slot) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	if (slot->built) slot->road.~RustRoadCommandContext();
	slot->built = true;
	return {.road_type = RustRoadVehicleType(u), .road = &std::construct_at(&slot->road)->view};
}
static void OPENTTD_RUST_CALL RoadSvcDisconnect() noexcept { FatalError("Disconnecting road vehicle."); }
static bool OPENTTD_RUST_CALL RoadSvcShouldStop(OpenTTDVehicle *v, uint32_t tile) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	return u->current_order.ShouldStopAtStation(u, StationID(GetRustSharedServices().station_index(tile)));
}
/** The first-arrival body of `RoadVehArrivesAt` after the station is marked: news, AI and game events. */
static void OPENTTD_RUST_CALL RoadSvcArrivalNews(OpenTTDVehicle *v, uint16_t station, uint8_t headline) noexcept
{
	static const StringID headlines[] = {STR_NEWS_FIRST_BUS_ARRIVAL, STR_NEWS_FIRST_PASSENGER_TRAM_ARRIVAL, STR_NEWS_FIRST_TRUCK_ARRIVAL, STR_NEWS_FIRST_CARGO_TRAM_ARRIVAL};
	const RoadVehicle *u = RoadOf(v);
	Station *st = Station::Get(StationID(station));
	AddVehicleNewsItem(GetEncodedString(headlines[headline], st->index), (u->GetOwner() == _local_company) ? NewsType::ArrivalCompany : NewsType::ArrivalOther, u->index, st->index);
	AI::NewEvent(u->GetOwner(), new ScriptEventStationFirstVehicle(st->index, u->index));
	Game::NewEvent(new ScriptEventStationFirstVehicle(st->index, u->index));
}
/** The road stop arrival triggers after `BeginLoading`. */
static void OPENTTD_RUST_CALL RoadSvcDepotExitDirty(OpenTTDVehicle *v) noexcept
{
	RustVehicleInvalidateDepot(RoadOf(v));
}
/** `RoadVehCrash` after `v->Crash()`. */
static void OPENTTD_RUST_CALL RoadSvcCrashNews(OpenTTDVehicle *v, uint32_t victims) noexcept
{
	RoadVehicle *u = RoadOf(v);
	AI::NewEvent(u->GetOwner(), new ScriptEventVehicleCrashed(u->index, u->GetTile(), ScriptEventVehicleCrashed::CRASH_RV_LEVEL_CROSSING, victims, u->GetOwner()));
	Game::NewEvent(new ScriptEventVehicleCrashed(u->index, u->GetTile(), ScriptEventVehicleCrashed::CRASH_RV_LEVEL_CROSSING, victims, u->GetOwner()));

	EncodedString headline = (victims == 1)
		? GetEncodedString(STR_NEWS_ROAD_VEHICLE_CRASH_DRIVER)
		: GetEncodedString(STR_NEWS_ROAD_VEHICLE_CRASH, victims);
	NewsType newstype = u->GetOwner() == _local_company ? NewsType::Accident : NewsType::AccidentOther;

	AddTileNewsItem(std::move(headline), newstype, u->GetTile());

}



/** The running cost tail of `RoadVehicle::OnNewEconomyDay`. */
static void OPENTTD_RUST_CALL RoadSvcPayRunning(OpenTTDVehicle *v, int64_t amount) noexcept
{
	RoadVehicle *u = RoadOf(v);
	CommandCost cost(EXPENSES_ROADVEH_RUN, Money(amount));

	SubtractMoneyFromCompanyFract(u->GetOwner(), cost);

	RustVehicleDetailsDirty(RustVehicle(u));
	SetWindowClassesDirty(WC_ROADVEH_LIST);
}


static constinit const OpenTTDRoadLeaves _rust_road_leaves{
	.vehicle = &_rust_vehicle_services,
	.ground = &_rust_ground_services<RoadVehicle>,
	.shared = &_rust_shared_services,

	.any_road_bits = RoadSvcAnyRoadBits,
	.is_road_depot_tile = Shared_is_road_depot,
	.depot_direction = Shared_road_depot_direction,
	.disallowed_directions = Shared_disallowed_road_directions,
	.is_bay_stop = Shared_is_bay_stop,
	.bay_direction = Shared_bay_stop_direction,
	.is_station_road_stop = RoadSvcIsStationRoadStop,
	.choose_track = RoadSvcChooseTrack,
	.find_depot = RoadSvcFindDepot,
	.tram_build_read = RoadSvcTramBuildRead,
	.disconnect = RoadSvcDisconnect,
	.should_stop = RoadSvcShouldStop,
	.arrival_news = RoadSvcArrivalNews,
	.depot_exit_dirty = RoadSvcDepotExitDirty,
	.crash_news = RoadSvcCrashNews,
	.pay_running = RoadSvcPayRunning,
};

static const OpenTTDRoadLeaves &GetRoadLeaves()
{
	return _rust_road_leaves;
}
