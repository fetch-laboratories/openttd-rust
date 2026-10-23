/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

#include "road_command_services.h"
#include "services_road.h"

/** @file road_services.h Typed road services; each resolves its vehicle once and reads only what it returns. */
#include "services_map.h"
#include "services_road.h"
#include "services_station.h"

static_assert(RVS_USING_SECOND_BAY == 1 && RVS_IN_ROAD_STOP == 5 && RVS_IN_DT_ROAD_STOP == 6);
static_assert(RVSB_IN_DEPOT == 254 && RVSB_WORMHOLE == 255 && RVSB_IN_ROAD_STOP == 32 && RVSB_IN_DT_ROAD_STOP == 64);
static_assert(to_underlying(RoadStopType::Bus) == 0 && to_underlying(RoadStopType::Truck) == 1);
static_assert(PROP_ROADVEH_SHORTEN_FACTOR == 0x23 && PROP_ROADVEH_CARGO_AGE_PERIOD == 0x22 && PROP_ROADVEH_SPEED == 0x15 && PROP_ROADVEH_RUNNING_COST_FACTOR == 0x09);

static RoadVehicle *RoadOf(OpenTTDVehicle *v) noexcept { return RoadVehicle::From(VehicleOf(v)); }

static OpenTTDRoadMaxSpeedRead RoadMaxSpeedRead(const RoadVehicle *u) noexcept
{
	const RoadVehicle *next = static_cast<const RoadVehicle *>(RustVehicleNext(u));
	return {.next = RustVehicle(next), .next_state = RoadStateOf(next), .tile = RustVehicleTile(u), .max_track_speed = u->gcache.cached_max_track_speed,
		.order_max_speed = RustVehicleOrderMaxSpeed(u), .direction = RustVehicleDirection(u), .status = RustVehicleStatus(u)};
}
static OpenTTDRoadVisualRead OPENTTD_VEHICLE_CALL RoadSvcVisualRead(OpenTTDVehicle *v) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	if (!u->VisualEffectEligible()) return {};
	return {.max = RoadMaxSpeedRead(u), .eligible = true};
}
static OpenTTDRoadSpeedRead RoadSpeedRead(const RoadVehicle *u, bool realistic) noexcept
{
	return {.max = RoadMaxSpeedRead(u), .x = RustVehicleX(u), .y = RustVehicleY(u), .z = RustVehicleZ(u), .acceleration = realistic ? RustGroundAcceleration(u) : 0,
		.length = RustGroundCachedLength(u), .tram = SharedRoadTypeIsTram(u->roadtype)};
}
static OpenTTDRoadSpeedRead OPENTTD_VEHICLE_CALL RoadSvcSpeedRead(OpenTTDVehicle *v, bool realistic) noexcept { return RoadSpeedRead(RoadOf(v), realistic); }
static OpenTTDRoadSpeedRead OPENTTD_VEHICLE_CALL RoadSvcVisualEffectThenSpeed(OpenTTDVehicle *v, uint32_t speed, bool realistic) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	u->ShowVisualEffectAtSpeed(speed);
	return RoadSpeedRead(u, realistic);
}
static OpenTTDRoadMaxSpeedRead OPENTTD_VEHICLE_CALL RoadSvcMaxSpeedRead(OpenTTDVehicle *v) noexcept { return RoadMaxSpeedRead(RoadOf(v)); }
static OpenTTDRoadSpeedPart OPENTTD_VEHICLE_CALL RoadSvcSpeedPart(OpenTTDVehicle *v) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	const RoadVehicle *next = static_cast<const RoadVehicle *>(RustVehicleNext(u));
	return {.next = RustVehicle(next), .next_state = RoadStateOf(next), .tile = RustVehicleTile(u), .direction = RustVehicleDirection(u), .status = RustVehicleStatus(u)};
}
static OpenTTDRoadStep OPENTTD_VEHICLE_CALL RoadSvcStep(OpenTTDVehicle *v) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	return {.tile = RustVehicleTile(u), .x = RustVehicleX(u), .y = RustVehicleY(u), .z = RustVehicleZ(u),
		.cur_speed = RustVehicleSpeed(u), .direction = RustVehicleDirection(u), .length = RustGroundCachedLength(u), .front = RustVehicleFrontEngine(u), .tram = SharedRoadTypeIsTram(u->roadtype)};
}
static OpenTTDRoadCrashPart OPENTTD_VEHICLE_CALL RoadSvcCrashPart(OpenTTDVehicle *v) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	const RoadVehicle *next = static_cast<const RoadVehicle *>(RustVehicleNext(u));
	return {.next = RustVehicle(next), .next_state = RoadStateOf(next), .tile = RustVehicleTile(u), .z = RustVehicleZ(u), .crossing = MapSvcIsLevelCrossing(u->tile.base())};
}
static OpenTTDRoadPartRead OPENTTD_VEHICLE_CALL RoadSvcPart(OpenTTDVehicle *v) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	const RoadVehicle *next = static_cast<const RoadVehicle *>(RustVehicleNext(u));
	return {.next = RustVehicle(next), .next_state = RoadStateOf(next), .tile = RustVehicleTile(u), .z = RustVehicleZ(u), .tram = SharedRoadTypeIsTram(u->roadtype)};
}
static OpenTTDRoadLink OPENTTD_VEHICLE_CALL RoadSvcLink(OpenTTDVehicle *v) noexcept
{
	const RoadVehicle *next = static_cast<const RoadVehicle *>(RustVehicleNext(RoadOf(v)));
	return {.next = RustVehicle(next), .next_state = RoadStateOf(next)};
}
static OpenTTDRoadLinkTile OPENTTD_VEHICLE_CALL RoadSvcLinkTile(OpenTTDVehicle *v) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	const RoadVehicle *next = static_cast<const RoadVehicle *>(RustVehicleNext(u));
	return {.link = {.next = RustVehicle(next), .next_state = RoadStateOf(next)}, .tile = RustVehicleTile(u)};
}
static OpenTTDRoadEntered OPENTTD_VEHICLE_CALL RoadSvcEnterTile(OpenTTDVehicle *v, uint8_t flags) noexcept
{
	RoadVehicle *u = RoadOf(v);
	return {.tile = RustVehicleTile(u), .cur_speed = RustVehicleSpeed(u), .flags = flags, .direction = RustVehicleDirection(u), .order_type = RustVehicleOrderType(u)};
}

/** Pool-order traversals; C++ skips what the original predicate rejects by type or consist, Rust decides the rest. */
static void OPENTTD_VEHICLE_CALL RoadSvcVisitClose(OpenTTDVehicle *v, int32_t x, int32_t y, bool wormhole, OpenTTDRoadCloseVisitor visit, void *context) noexcept
{
	const RoadVehicle *front = RoadOf(v);
	auto offer = [&](const Vehicle *u) {
		if (u->type != VEH_ROAD || RustVehicleFirst(u) == front) return true;
		const RoadVehicle *r = RoadVehicle::From(u);
		const RoadVehicle *first = RustVehicleFirst(r);
		OpenTTDRoadCandidate candidate{.vehicle = RustVehicle(r), .state = RoadStateOf(r), .first = RustVehicle(first), .first_state = RoadStateOf(first),
			.index = r->index.base(), .x = RustVehicleX(r), .y = RustVehicleY(r), .z = RustVehicleZ(r), .first_speed = RustVehicleSpeed(first), .direction = RustVehicleDirection(r)};
		return visit(context, &candidate);
	};
	if (wormhole) {
		for (const Vehicle *u : RustVehiclesOnTile(front->tile)) if (!offer(u)) return;
		for (const Vehicle *u : RustVehiclesOnTile(GetOtherTunnelBridgeEnd(front->tile))) if (!offer(u)) return;
	} else {
		for (const Vehicle *u : VehiclesNearTileXY(x, y, 8)) if (!offer(u)) return;
	}
}
static void OPENTTD_VEHICLE_CALL RoadSvcVisitTrains(OpenTTDVehicle *v, OpenTTDRoadTrainVisitor visit, void *context) noexcept
{
	const RoadVehicle *front = RoadOf(v);
	for (const Vehicle *t : VehiclesNearTileXY(RustVehicleX(front), RustVehicleY(front), 4)) {
		if (t->type != VEH_TRAIN) continue;
		int32_t z = RustVehicleZ(t);
		if (!visit(context, &z)) return;
	}
}
static void OPENTTD_VEHICLE_CALL RoadSvcVisitTile(uint32_t tile, OpenTTDRoadTileVisitor visit, void *context) noexcept
{
	for (const Vehicle *u : RustVehiclesOnTile(TileIndex(tile))) {
		if (u->type != VEH_ROAD) continue;
		OpenTTDRoadTileVehicle item{.vehicle = RustVehicle(u), .first = RustVehicle(RustVehicleFirst(u))};
		if (!visit(context, &item)) return;
	}
}

static bool OPENTTD_VEHICLE_CALL RoadSvcHasRoad(OpenTTDVehicle *v, uint32_t tile) noexcept { return HasTileAnyRoadType(TileIndex(tile), RoadOf(v)->compatible_roadtypes); }

static uint8_t OPENTTD_VEHICLE_CALL RoadSvcAnyRoadBits(OpenTTDVehicle *v, uint32_t tile, bool straight_only) noexcept
{
	return GetAnyRoadBits(TileIndex(tile), RustRoadTramType(RoadOf(v)->roadtype), straight_only);
}
static bool OPENTTD_VEHICLE_CALL RoadSvcIsStationRoadStop(uint32_t tile) noexcept { return IsStationRoadStop(TileIndex(tile)); }
/** `old_tile = v->tile; v->tile = tile;` then both road types of the vehicle's road/tram kind. */
static OpenTTDRoadRoadTypes OPENTTD_VEHICLE_CALL RoadSvcMoveTile(OpenTTDVehicle *v, uint32_t tile) noexcept
{
	RoadVehicle *u = RoadOf(v);
	TileIndex old_tile = u->tile;
	u->tile = TileIndex(tile);
	RoadTramType rtt = RustRoadTramType(u->roadtype);
	return {.before = GetRoadType(old_tile, rtt), .after = GetRoadType(u->tile, rtt)};
}
static OpenTTDRoadPathRead OPENTTD_VEHICLE_CALL RoadSvcPathRead(OpenTTDVehicle *v) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	return {.dest = RustVehicleDestTile(u), .owner = RustVehicleOwner(u), .articulated = RustGroundHasArticulated(u), .bus = u->IsBus()};
}
static OpenTTDRoadTrackChoice OPENTTD_VEHICLE_CALL RoadSvcChooseTrack(OpenTTDVehicle *v, uint32_t tile, uint8_t entry, uint16_t trackdirs) noexcept
{
	bool found = true;
	Trackdir trackdir = YapfRoadVehicleChooseTrack(RoadOf(v), TileIndex(tile), static_cast<DiagDirection>(entry), static_cast<TrackdirBits>(trackdirs), found);
	return {.trackdir = static_cast<uint8_t>(trackdir), .found = found};
}
static OpenTTDRoadDepotResult OPENTTD_VEHICLE_CALL RoadSvcFindDepot(OpenTTDVehicle *v, int32_t max_distance) noexcept
{
	FindDepotData result = YapfRoadVehicleFindNearestDepot(RoadOf(v), max_distance);
	return {.tile = result.tile.base(), .length = result.best_length};
}
static OpenTTDRoadTramBuildRead OPENTTD_VEHICLE_CALL RoadSvcTramBuildRead(OpenTTDVehicle *v, OpenTTDRoadCommandSlot *slot) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	if (slot->built) slot->road.~RustRoadCommandContext();
	slot->built = true;
	return {.owner = RustVehicleOwner(u), .road_type = u->roadtype, .road = &std::construct_at(&slot->road)->view};
}
static void OPENTTD_VEHICLE_CALL RoadSvcDisconnect() noexcept { FatalError("Disconnecting road vehicle."); }
static OpenTTDRoadStopRead OPENTTD_VEHICLE_CALL RoadSvcStopRead(OpenTTDVehicle *v) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	return {.owner = RustVehicleOwner(u), .bus = u->IsBus()};
}
static bool OPENTTD_VEHICLE_CALL RoadSvcShouldStop(OpenTTDVehicle *v, uint32_t tile) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	return u->current_order.ShouldStopAtStation(u, StationID(GetRustSharedServices().station_index(tile)));
}
static OpenTTDRoadArrivalRead OPENTTD_VEHICLE_CALL RoadSvcArrivalRead(OpenTTDVehicle *v, uint16_t station) noexcept
{
	return {.had_vehicle_of_type = Station::Get(StationID(station))->had_vehicle_of_type, .bus = RoadOf(v)->IsBus()};
}
/** The first-arrival body of `RoadVehArrivesAt`: mark, news, AI and game events. */
static void OPENTTD_VEHICLE_CALL RoadSvcArrivalNews(OpenTTDVehicle *v, uint16_t station, uint8_t vehicle_type, uint8_t headline) noexcept
{
	static const StringID headlines[] = {STR_NEWS_FIRST_BUS_ARRIVAL, STR_NEWS_FIRST_PASSENGER_TRAM_ARRIVAL, STR_NEWS_FIRST_TRUCK_ARRIVAL, STR_NEWS_FIRST_CARGO_TRAM_ARRIVAL};
	const RoadVehicle *u = RoadOf(v);
	Station *st = Station::Get(StationID(station));
	st->had_vehicle_of_type |= static_cast<StationHadVehicleOfType>(vehicle_type);
	AddVehicleNewsItem(GetEncodedString(headlines[headline], st->index), (u->owner == _local_company) ? NewsType::ArrivalCompany : NewsType::ArrivalOther, u->index, st->index);
	AI::NewEvent(u->owner, new ScriptEventStationFirstVehicle(st->index, u->index));
	Game::NewEvent(new ScriptEventStationFirstVehicle(st->index, u->index));
}
/** `v->BeginLoading()` and the road stop arrival triggers. */
static void OPENTTD_VEHICLE_CALL RoadSvcBeginLoadingAt(OpenTTDVehicle *v, uint16_t station) noexcept
{
	RoadVehicle *u = RoadOf(v);
	Station *st = Station::Get(StationID(station));
	u->BeginLoading();
	TriggerRoadStopRandomisation(st, u->tile, StationRandomTrigger::VehicleArrives);
	RustRoadStopAnimation(st, u->tile, StationAnimationTrigger::VehicleArrives);
}
static OpenTTDRoadOvertakeRead OPENTTD_VEHICLE_CALL RoadSvcOvertakeRead(OpenTTDVehicle *v) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	return {.tile = RustVehicleTile(u), .cur_speed = RustVehicleSpeed(u), .direction = RustVehicleDirection(u), .status = RustVehicleStatus(u)};
}
static OpenTTDRoadSoundRead OPENTTD_VEHICLE_CALL RoadSvcSoundRead(OpenTTDVehicle *v) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	return {.sound = RoadVehInfo(u->engine_type)->sfx, .tick = RustVehicleTick(u)};
}
/** The depot exit tail of `RoadVehLeaveDepot` after the state and frame stores. */
static void OPENTTD_VEHICLE_CALL RoadSvcDepotExit(OpenTTDVehicle *v, int32_t x, int32_t y) noexcept
{
	RoadVehicle *u = RoadOf(v);
	u->vehstatus.Reset(VehState::Hidden);
	u->x_pos = x;
	u->y_pos = y;
	RustVehicleUpdatePosition(u);
	u->UpdateInclination(true, true);
	RustInvalidateDepotWindow(u->tile);
}
/** `RoadVehCrash` after `v->Crash()`. */
static void OPENTTD_VEHICLE_CALL RoadSvcCrashNews(OpenTTDVehicle *v, uint32_t victims) noexcept
{
	RoadVehicle *u = RoadOf(v);
	AI::NewEvent(u->owner, new ScriptEventVehicleCrashed(u->index, u->tile, ScriptEventVehicleCrashed::CRASH_RV_LEVEL_CROSSING, victims, u->owner));
	Game::NewEvent(new ScriptEventVehicleCrashed(u->index, u->tile, ScriptEventVehicleCrashed::CRASH_RV_LEVEL_CROSSING, victims, u->owner));

	EncodedString headline = (victims == 1)
		? GetEncodedString(STR_NEWS_ROAD_VEHICLE_CRASH_DRIVER)
		: GetEncodedString(STR_NEWS_ROAD_VEHICLE_CRASH, victims);
	NewsType newstype = u->owner == _local_company ? NewsType::Accident : NewsType::AccidentOther;

	AddTileNewsItem(std::move(headline), newstype, u->tile);

	OpenTTDModifyStationRatingAround(u->tile.base(), u->owner.base(), -160, 22);
	if (_settings_client.sound.disaster) SndPlayVehicleFx(SND_12_EXPLOSION, u);
}
static uint8_t OPENTTD_VEHICLE_CALL RoadSvcShortenFactor(const OpenTTDEngine *e) noexcept
{
	return EngineOf(e)->VehInfo<RoadVehicleInfo>().shorten_factor;
}
static uint16_t OPENTTD_VEHICLE_CALL RoadSvcDefaultMaxSpeed(OpenTTDVehicle *v) noexcept
{
	return RoadVehInfo(EngineID(RustVehicleEngineID(VehicleOf(v))))->max_speed;
}
static void OPENTTD_VEHICLE_CALL RoadSvcUpdateCargoAge(OpenTTDVehicle *v) noexcept
{
	RoadVehicle *u = RoadOf(v);
	u->vcache.cached_cargo_age_period = GetVehicleProperty(u, PROP_ROADVEH_CARGO_AGE_PERIOD, EngInfo(u->engine_type)->cargo_age_period);
}
static OpenTTDRoadCostRead OPENTTD_VEHICLE_CALL RoadSvcCostRead(OpenTTDVehicle *v) noexcept
{
	const Engine *e = RoadOf(v)->GetEngine();
	const RoadVehicleInfo &info = e->VehInfo<RoadVehicleInfo>();
	return {.engine = RustEngine(e), .cost_class = info.running_cost_class, .cost_factor = info.running_cost};
}
static int64_t OPENTTD_VEHICLE_CALL RoadSvcPrice(const OpenTTDEngine *engine, uint32_t cost_factor) noexcept
{
	const Engine *e = EngineOf(engine);
	return static_cast<int64_t>(GetPrice(e->VehInfo<RoadVehicleInfo>().running_cost_class, cost_factor, e->GetGRF()));
}
/** The running cost tail of `RoadVehicle::OnNewEconomyDay`. */
static void OPENTTD_VEHICLE_CALL RoadSvcPayRunning(OpenTTDVehicle *v, int64_t amount) noexcept
{
	RoadVehicle *u = RoadOf(v);
	CommandCost cost(EXPENSES_ROADVEH_RUN, Money(amount));

	u->profit_this_year -= cost.GetCost();
	u->running_ticks = 0;

	SubtractMoneyFromCompanyFract(u->owner, cost);

	SetWindowDirty(WC_VEHICLE_DETAILS, u->index);
	SetWindowClassesDirty(WC_ROADVEH_LIST);
}
static OpenTTDRoadServiceRead OPENTTD_VEHICLE_CALL RoadSvcServiceRead(OpenTTDVehicle *v) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	return {.tile = RustVehicleTile(u), .servint = Company::Get(u->owner)->settings.vehicle.servint_roadveh, .cur_speed = RustVehicleSpeed(u)};
}
static OpenTTDRoadTurnRead OPENTTD_VEHICLE_CALL RoadSvcTurnRead(OpenTTDVehicle *v) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	return {.tile = RustVehicleTile(u), .breakdown_ctr = RustVehicleBreakdownCounter(u), .direction = RustVehicleDirection(u), .order_type = RustVehicleOrderType(u), .status = RustVehicleStatus(u)};
}
static OpenTTDRoadTrackdirRead OPENTTD_VEHICLE_CALL RoadSvcTrackdirRead(OpenTTDVehicle *v) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	return {.tile = RustVehicleTile(u), .direction = RustVehicleDirection(u), .status = RustVehicleStatus(u)};
}

static uint8_t OPENTTD_VEHICLE_CALL RoadSvcProcessOrdersThenLoading(OpenTTDVehicle *v) noexcept
{
	Vehicle *u = VehicleOf(v);
	ProcessOrders(u);
	u->HandleLoading();
	return RustVehicleOrderType(u);
}

static constinit const OpenTTDRoadLeaves _rust_road_leaves{
	.vehicle = &_rust_vehicle_services,
	.ground = &_rust_ground_services<RoadVehicle>,
	.shared = &_rust_shared_services,
	.max_depot_penalty = RustMaximumDepotPenalty,
	.process_orders_then_loading = RoadSvcProcessOrdersThenLoading,
	.visual_read = RoadSvcVisualRead,
	.visual_effect_then_speed = RoadSvcVisualEffectThenSpeed,
	.speed_read = RoadSvcSpeedRead,
	.max_speed_read = RoadSvcMaxSpeedRead,
	.speed_part = RoadSvcSpeedPart,
	.step = RoadSvcStep,
	.crash_part = RoadSvcCrashPart,
	.part = RoadSvcPart,
	.link = RoadSvcLink,
	.link_tile = RoadSvcLinkTile,
	.enter_tile = RoadSvcEnterTile,
	.visit_close = RoadSvcVisitClose,
	.visit_trains = RoadSvcVisitTrains,
	.visit_tile = RoadSvcVisitTile,
	.has_road = RoadSvcHasRoad,

	.any_road_bits = RoadSvcAnyRoadBits,
	.is_road_depot_tile = Shared_is_road_depot,
	.depot_direction = Shared_road_depot_direction,
	.is_normal_road = Shared_is_normal_road,
	.disallowed_directions = Shared_disallowed_road_directions,
	.is_bay_stop = Shared_is_bay_stop,
	.bay_direction = Shared_bay_stop_direction,
	.is_drive_through = Shared_is_drive_through_stop,
	.is_station_road_stop = RoadSvcIsStationRoadStop,
	.move_tile = RoadSvcMoveTile,
	.path_read = RoadSvcPathRead,
	.choose_track = RoadSvcChooseTrack,
	.find_depot = RoadSvcFindDepot,
	.tram_build_read = RoadSvcTramBuildRead,
	.disconnect = RoadSvcDisconnect,
	.stop_read = RoadSvcStopRead,
	.should_stop = RoadSvcShouldStop,
	.arrival_read = RoadSvcArrivalRead,
	.arrival_news = RoadSvcArrivalNews,
	.begin_loading_at = RoadSvcBeginLoadingAt,
	.overtake_read = RoadSvcOvertakeRead,
	.sound_read = RoadSvcSoundRead,
	.depot_exit = RoadSvcDepotExit,
	.crash_news = RoadSvcCrashNews,
	.shorten_factor = RoadSvcShortenFactor,
	.default_max_speed = RoadSvcDefaultMaxSpeed,
	.update_cargo_age = RoadSvcUpdateCargoAge,
	.cost_read = RoadSvcCostRead,
	.price = RoadSvcPrice,
	.pay_running = RoadSvcPayRunning,
	.service_read = RoadSvcServiceRead,
	.turn_read = RoadSvcTurnRead,
	.trackdir_read = RoadSvcTrackdirRead,
};

static const OpenTTDRoadLeaves &GetRoadLeaves()
{
	return _rust_road_leaves;
}
