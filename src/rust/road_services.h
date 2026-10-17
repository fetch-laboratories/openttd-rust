/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file road_services.h Typed road services; each resolves its vehicle once and reads only what it returns. */
static_assert(RVSB_IN_DEPOT == 254 && RVSB_WORMHOLE == 255 && RVSB_IN_ROAD_STOP == 32 && RVSB_IN_DT_ROAD_STOP == 64);
static_assert(to_underlying(RoadStopType::Bus) == 0 && to_underlying(RoadStopType::Truck) == 1);
static_assert(PROP_ROADVEH_SHORTEN_FACTOR == 0x23 && PROP_ROADVEH_CARGO_AGE_PERIOD == 0x22 && PROP_ROADVEH_SPEED == 0x15 && PROP_ROADVEH_RUNNING_COST_FACTOR == 0x09);

static RoadVehicle *RoadOf(OpenTTDVehicle *v) noexcept { return RoadVehicle::From(VehicleOf(v)); }
static OpenTTDRoadState *RoadStateOf(const RoadVehicle *u) noexcept { return u == nullptr ? nullptr : u->GetRustState(); }

static OpenTTDRoadFrontTickRead OPENTTD_VEHICLE_CALL RoadSvcTickFront(OpenTTDVehicle *v) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	return {.movement = GetRoadEntry(), .order_time = u->current_order_time, .running = u->running_ticks, .status = RustVehicleStatus(u)};
}

static OpenTTDRoadMaxSpeedRead RoadMaxSpeedRead(const RoadVehicle *u) noexcept
{
	const RoadVehicle *next = static_cast<const RoadVehicle *>(RustVehicleNext(u));
	return {.next = RustVehicle(next), .next_state = RoadStateOf(next), .tile = RustVehicleTile(u), .max_track_speed = u->gcache.cached_max_track_speed,
		.order_max_speed = u->current_order.GetMaxSpeed(), .direction = RustVehicleDirection(u), .status = RustVehicleStatus(u)};
}
static OpenTTDRoadSpeedRead OPENTTD_VEHICLE_CALL RoadSvcVisualThenSpeed(OpenTTDVehicle *v, bool realistic) noexcept
{
	RoadVehicle *u = RoadOf(v);
	u->ShowVisualEffect();
	return {.max = RoadMaxSpeedRead(u), .x = u->x_pos, .y = u->y_pos, .z = u->z_pos, .acceleration = realistic ? u->GetAcceleration() : 0,
		.length = RustGroundCachedLength(u), .tram = RoadTypeIsTram(u->roadtype)};
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
	return {.tile = RustVehicleTile(u), .x = u->x_pos, .y = u->y_pos, .z = u->z_pos,
		.cur_speed = u->cur_speed, .direction = RustVehicleDirection(u), .length = RustGroundCachedLength(u), .front = u->IsFrontEngine(), .tram = RoadTypeIsTram(u->roadtype)};
}
static OpenTTDRoadCrashPart OPENTTD_VEHICLE_CALL RoadSvcCrashPart(OpenTTDVehicle *v) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	const RoadVehicle *next = static_cast<const RoadVehicle *>(RustVehicleNext(u));
	return {.next = RustVehicle(next), .next_state = RoadStateOf(next), .tile = RustVehicleTile(u), .z = u->z_pos, .crossing = GetRustSharedServices().is_level_crossing(u->tile.base())};
}
static OpenTTDRoadPartRead OPENTTD_VEHICLE_CALL RoadSvcPart(OpenTTDVehicle *v) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	const RoadVehicle *next = static_cast<const RoadVehicle *>(RustVehicleNext(u));
	return {.next = RustVehicle(next), .next_state = RoadStateOf(next), .tile = RustVehicleTile(u), .z = u->z_pos, .tram = RoadTypeIsTram(u->roadtype)};
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
static OpenTTDRoadEntered OPENTTD_VEHICLE_CALL RoadSvcEnterTile(OpenTTDVehicle *v, uint32_t tile, int32_t x, int32_t y) noexcept
{
	RoadVehicle *u = RoadOf(v);
	uint8_t flags = VehicleEnterTile(u, TileIndex(tile), x, y).base();
	return {.tile = RustVehicleTile(u), .cur_speed = u->cur_speed, .flags = flags, .direction = RustVehicleDirection(u), .order_type = RustVehicleOrderType(u)};
}

/** Pool-order traversals; C++ skips what the original predicate rejects by type or consist, Rust decides the rest. */
static void OPENTTD_VEHICLE_CALL RoadSvcVisitClose(OpenTTDVehicle *v, int32_t x, int32_t y, bool wormhole, OpenTTDRoadCloseVisitor visit, void *context) noexcept
{
	const RoadVehicle *front = RoadOf(v);
	auto offer = [&](const Vehicle *u) {
		if (u->type != VEH_ROAD || RustVehicleFirst(u) == front) return true;
		const RoadVehicle *r = RoadVehicle::From(u);
		const RoadVehicle *first = r->First();
		OpenTTDRoadCandidate candidate{.vehicle = RustVehicle(r), .state = r->GetRustState(), .first = RustVehicle(first), .first_state = first->GetRustState(),
			.index = r->index.base(), .x = r->x_pos, .y = r->y_pos, .z = r->z_pos, .first_speed = first->cur_speed, .direction = RustVehicleDirection(r)};
		return visit(context, &candidate);
	};
	if (wormhole) {
		for (const Vehicle *u : VehiclesOnTile(front->tile)) if (!offer(u)) return;
		for (const Vehicle *u : VehiclesOnTile(GetOtherTunnelBridgeEnd(front->tile))) if (!offer(u)) return;
	} else {
		for (const Vehicle *u : VehiclesNearTileXY(x, y, 8)) if (!offer(u)) return;
	}
}
static void OPENTTD_VEHICLE_CALL RoadSvcVisitTrains(OpenTTDVehicle *v, OpenTTDRoadTrainVisitor visit, void *context) noexcept
{
	const RoadVehicle *front = RoadOf(v);
	for (const Vehicle *t : VehiclesNearTileXY(front->x_pos, front->y_pos, 4)) {
		if (t->type != VEH_TRAIN) continue;
		int32_t z = t->z_pos;
		if (!visit(context, &z)) return;
	}
}
static void OPENTTD_VEHICLE_CALL RoadSvcVisitTile(uint32_t tile, OpenTTDRoadTileVisitor visit, void *context) noexcept
{
	for (const Vehicle *u : VehiclesOnTile(TileIndex(tile))) {
		if (u->type != VEH_ROAD) continue;
		OpenTTDRoadTileVehicle item{.vehicle = RustVehicle(u), .first = RustVehicle(RustVehicleFirst(u))};
		if (!visit(context, &item)) return;
	}
}

static bool OPENTTD_VEHICLE_CALL RoadSvcHasRoad(OpenTTDVehicle *v, uint32_t tile) noexcept { return HasTileAnyRoadType(TileIndex(tile), RoadOf(v)->compatible_roadtypes); }
static uint32_t OPENTTD_VEHICLE_CALL RoadSvcTrackStatus(OpenTTDVehicle *v, uint32_t tile) noexcept
{
	return GetTileTrackStatus(TileIndex(tile), TRANSPORT_ROAD, GetRoadTramType(RoadOf(v)->roadtype));
}
static uint8_t OPENTTD_VEHICLE_CALL RoadSvcAnyRoadBits(OpenTTDVehicle *v, uint32_t tile, bool straight_only) noexcept
{
	return GetAnyRoadBits(TileIndex(tile), GetRoadTramType(RoadOf(v)->roadtype), straight_only);
}
static uint8_t OPENTTD_VEHICLE_CALL RoadSvcTramBits(uint32_t tile) noexcept { return GetRoadBits(TileIndex(tile), RTT_TRAM); }
static bool OPENTTD_VEHICLE_CALL RoadSvcHasRoadWorks(uint32_t tile) noexcept { return HasRoadWorks(TileIndex(tile)); }
static bool OPENTTD_VEHICLE_CALL RoadSvcIsStationRoadStop(uint32_t tile) noexcept { return IsStationRoadStop(TileIndex(tile)); }
static uint8_t OPENTTD_VEHICLE_CALL RoadSvcStopType(uint32_t tile) noexcept { return to_underlying(GetRoadStopType(TileIndex(tile))); }
static bool OPENTTD_VEHICLE_CALL RoadSvcHasFreeBay(uint32_t tile) noexcept
{
	TileIndex t(tile);
	return RoadStop::GetByTile(t, GetRoadStopType(t))->HasFreeBay();
}
static bool OPENTTD_VEHICLE_CALL RoadSvcContinuation(uint32_t tile, uint32_t next) noexcept
{
	return RoadStop::IsDriveThroughRoadStopContinuation(TileIndex(tile), TileIndex(next));
}
/** `old_tile = v->tile; v->tile = tile;` then both road types of the vehicle's road/tram kind. */
static OpenTTDRoadRoadTypes OPENTTD_VEHICLE_CALL RoadSvcMoveTile(OpenTTDVehicle *v, uint32_t tile) noexcept
{
	RoadVehicle *u = RoadOf(v);
	TileIndex old_tile = u->tile;
	u->tile = TileIndex(tile);
	RoadTramType rtt = GetRoadTramType(u->roadtype);
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
/** `CanBuildTramTrackOnTile(v->owner, tile, v->roadtype, bits)`. */
static bool OPENTTD_VEHICLE_CALL RoadSvcCanBuildTram(OpenTTDVehicle *v, uint32_t tile, uint8_t bits) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	Backup<CompanyID> cur_company(_current_company, u->owner);
	CommandCost ret = Command<CMD_BUILD_ROAD>::Do(DoCommandFlag::NoWater, TileIndex(tile), static_cast<RoadBits>(bits), u->roadtype, DRD_NONE, TownID::Invalid());
	cur_company.Restore();
	return ret.Succeeded();
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
static void OPENTTD_VEHICLE_CALL RoadSvcStopLeave(OpenTTDVehicle *v) noexcept
{
	RoadVehicle *u = RoadOf(v);
	RoadStop::GetByTile(u->tile, GetRoadStopType(u->tile))->Leave(u);
}
static void OPENTTD_VEHICLE_CALL RoadSvcStopEntrance(OpenTTDVehicle *v, bool busy) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	RoadStop::GetByTile(u->tile, GetRoadStopType(u->tile))->SetEntranceBusy(busy);
}
static bool OPENTTD_VEHICLE_CALL RoadSvcStopEntranceBusy(OpenTTDVehicle *v) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	return RoadStop::GetByTile(u->tile, GetRoadStopType(u->tile))->IsEntranceBusy();
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
	TriggerRoadStopAnimation(st, u->tile, StationAnimationTrigger::VehicleArrives);
}
static OpenTTDRoadOvertakeRead OPENTTD_VEHICLE_CALL RoadSvcOvertakeRead(OpenTTDVehicle *v) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	return {.tile = RustVehicleTile(u), .cur_speed = u->cur_speed, .direction = RustVehicleDirection(u), .status = RustVehicleStatus(u)};
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
	u->UpdatePosition();
	u->UpdateInclination(true, true);
	InvalidateWindowData(WC_VEHICLE_DEPOT, u->tile);
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

	ModifyStationRatingAround(u->tile, u->owner, -160, 22);
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
	return {.tile = RustVehicleTile(u), .servint = Company::Get(u->owner)->settings.vehicle.servint_roadveh, .cur_speed = u->cur_speed};
}
static OpenTTDRoadTurnRead OPENTTD_VEHICLE_CALL RoadSvcTurnRead(OpenTTDVehicle *v) noexcept
{
	const RoadVehicle *u = RoadOf(v);
	return {.tile = RustVehicleTile(u), .breakdown_ctr = u->breakdown_ctr, .direction = RustVehicleDirection(u), .order_type = RustVehicleOrderType(u), .status = RustVehicleStatus(u)};
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

static const OpenTTDRoadLeaves &GetRoadLeaves()
{
	static const OpenTTDRoadLeaves leaves{
		.vehicle = &GetRustVehicleServices(),
		.ground = &GetRustGroundServices<RoadVehicle>(),
		.shared = &GetRustSharedServices(),
		.tick_front = RoadSvcTickFront,
		.max_depot_penalty = RustMaximumDepotPenalty,
		.process_orders_then_loading = RoadSvcProcessOrdersThenLoading,
		.visual_then_speed = RoadSvcVisualThenSpeed,
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
		.track_status = RoadSvcTrackStatus,
		.any_road_bits = RoadSvcAnyRoadBits,
		.tram_bits = RoadSvcTramBits,
		.is_road_depot_tile = GetRustSharedServices().is_road_depot,
		.depot_direction = GetRustSharedServices().road_depot_direction,
		.is_normal_road = GetRustSharedServices().is_normal_road,
		.has_road_works = RoadSvcHasRoadWorks,
		.disallowed_directions = GetRustSharedServices().disallowed_road_directions,
		.is_bay_stop = GetRustSharedServices().is_bay_stop,
		.bay_direction = GetRustSharedServices().bay_stop_direction,
		.is_drive_through = GetRustSharedServices().is_drive_through_stop,
		.is_station_road_stop = RoadSvcIsStationRoadStop,
		.stop_type = RoadSvcStopType,
		.has_free_bay = RoadSvcHasFreeBay,
		.continuation = RoadSvcContinuation,
		.move_tile = RoadSvcMoveTile,
		.path_read = RoadSvcPathRead,
		.choose_track = RoadSvcChooseTrack,
		.find_depot = RoadSvcFindDepot,
		.can_build_tram = RoadSvcCanBuildTram,
		.disconnect = RoadSvcDisconnect,
		.stop_read = RoadSvcStopRead,
		.should_stop = RoadSvcShouldStop,
		.stop_leave = RoadSvcStopLeave,
		.stop_entrance = RoadSvcStopEntrance,
		.stop_entrance_busy = RoadSvcStopEntranceBusy,
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
	return leaves;
}
