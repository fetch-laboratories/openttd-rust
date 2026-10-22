/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file vehicle_build_services.hpp Native pools, specification helpers and presentation leaves. */
static VehicleType VehicleBuildDepotKind(uint32_t tile) noexcept { return GetDepotVehicleType(TileIndex(tile)); }
static uint32_t VehicleBuildArticulatedCount(uint16_t engine, bool capacity) noexcept { return CountArticulatedParts(EngineID(engine), capacity); }
static uint16_t VehicleBuildFreeUnit(VehicleType type) noexcept { return GetFreeUnitNumber(type); }
static void *VehicleBuildRail(OpenTTDCommandCost *out, uint16_t flags, uint32_t tile, const OpenTTDEngine *e) noexcept
{
	Vehicle *v = nullptr;
	*reinterpret_cast<CommandCost *>(out) = CmdBuildRailVehicle(DoCommandFlags(flags), TileIndex(tile), EngineOf(e), &v);
	return v;
}
/* Road vehicle and aircraft construction leaves: shell allocation and field stores. */
static bool VehicleConstructRoadTile(uint32_t tile, uint64_t powered) noexcept { return HasTileAnyRoadType(TileIndex(tile), RoadTypes(powered)); }
static void *VehicleConstructNewRoad(uint32_t tile) noexcept
{
	RoadVehicle *v = new RoadVehicle();
	v->direction = DiagDirToDir(static_cast<DiagDirection>(Shared_road_depot_direction(tile)));
	v->owner = _current_company;
	v->tile = TileIndex(tile);
	int x = TileX(v->tile) * TILE_SIZE + TILE_SIZE / 2;
	int y = TileY(v->tile) * TILE_SIZE + TILE_SIZE / 2;
	v->x_pos = x;
	v->y_pos = y;
	v->z_pos = SharedSlopePixelZ(x, y, true);
	v->vehstatus = {VehState::Hidden, VehState::Stopped, VehState::DefaultPalette};
	return v;
}
static void VehicleConstructRoadFront(void *shell, const OpenTTDEngine *e) noexcept
{
	RoadVehicle *v = static_cast<RoadVehicle *>(shell);
	RoadType rt = EngineOf(e)->VehInfo<RoadVehicleInfo>().roadtype;
	v->gcache.first_engine = EngineID::Invalid(); // needs to be set before first callback
	v->SetFrontEngine();
	v->roadtype = rt;
	v->compatible_roadtypes = RoadTypes(RustEnginePoweredRoadTypes(EngineOf(e)));
	v->gcache.cached_veh_length = VEHICLE_LENGTH;
}
static const OpenTTDStation *VehicleConstructStation(uint32_t tile) noexcept { return RustStation(Station::GetByTile(TileIndex(tile))); }
static bool VehicleConstructCanUse(const OpenTTDEngine *e, const OpenTTDStation *st) noexcept { return CanVehicleUseStation(EngineOf(e)->index, StationOf(st)); }
static uint32_t VehicleConstructHangar(const OpenTTDStation *station, uint32_t tile) noexcept
{
	const Station *st = StationOf(station);
	uint hangar = st->airport.GetHangarNum(TileIndex(tile));
	return RustStationHangarTile(st, hangar).base();
}
static uint8_t VehicleConstructPosOnBuild(uint32_t tile) noexcept { return GetVehiclePosOnBuild(TileIndex(tile)); }
static void *VehicleConstructNewAircraft() noexcept { return new Aircraft(); }
static void VehicleConstructAircraftPlace(void *aircraft, void *shadow, uint32_t tile) noexcept
{
	Aircraft *v = static_cast<Aircraft *>(aircraft);
	Aircraft *u = static_cast<Aircraft *>(shadow);
	v->direction = DIR_SE;
	v->owner = u->owner = _current_company;
	v->tile = TileIndex(tile);
	uint x = TileX(v->tile) * TILE_SIZE + 5;
	uint y = TileY(v->tile) * TILE_SIZE + 3;
	v->x_pos = u->x_pos = x;
	v->y_pos = u->y_pos = y;
	u->z_pos = SharedSlopePixelZ(x, y, false);
	v->z_pos = u->z_pos + 1;
	v->vehstatus = {VehState::Hidden, VehState::Stopped, VehState::DefaultPalette};
	u->vehstatus = {VehState::Hidden, VehState::Unclickable, VehState::Shadow};
}
static OpenTTDAircraftInfo VehicleConstructAircraftInfo(const OpenTTDEngine *e) noexcept
{
	const AircraftVehicleInfo &avi = EngineOf(e)->VehInfo<AircraftVehicleInfo>();
	return {.image = avi.image_index, .passengers = avi.passenger_capacity, .mail = avi.mail_capacity, .subtype = avi.subtype};
}
static void VehicleConstructAircraftProperties(void *aircraft, void *shadow, const OpenTTDEngine *e, uint8_t subtype) noexcept
{
	Aircraft *v = static_cast<Aircraft *>(aircraft);
	Aircraft *u = static_cast<Aircraft *>(shadow);
	v->acceleration = EngineOf(e)->VehInfo<AircraftVehicleInfo>().acceleration;
	u->engine_type = EngineOf(e)->index;
	v->subtype = subtype;
	u->subtype = AIR_SHADOW;
	v->vehicle_flags = {};
	v->SetNext(u);
}
static void VehicleConstructRotor(void *rotor, void *aircraft, void *shadow) noexcept
{
	Aircraft *w = static_cast<Aircraft *>(rotor);
	const Aircraft *v = static_cast<Aircraft *>(aircraft);
	w->engine_type = v->engine_type;
	w->direction = DIR_N;
	w->owner = _current_company;
	w->x_pos = v->x_pos;
	w->y_pos = v->y_pos;
	w->z_pos = v->z_pos + ROTOR_Z_OFFSET;
	w->vehstatus = {VehState::Hidden, VehState::Unclickable};
	w->spritenum = 0xFF;
	w->subtype = AIR_ROTOR;
	w->sprite_cache.sprite_seq.Set(SPR_ROTOR_STOPPED);
	static_cast<Vehicle *>(shadow)->SetNext(w);
}
static void VehicleConstructAircraftCapacity(const OpenTTDEngine *e, void *aircraft, void *shadow) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(aircraft);
	RustVehicleInvalidateChain(RustVehicle(v));
	v->cargo_cap = RustEngineDetermineCapacity(e, RustVehicle(v), &static_cast<Vehicle *>(shadow)->cargo_cap);
	RustVehicleInvalidateChain(RustVehicle(v));
}
static void VehicleConstructRoadCapacity(void *front, void *part, uint16_t capacity) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(front);
	Vehicle *u = static_cast<Vehicle *>(part);
	u->cargo_cap = capacity;
	u->refit_cap = 0;
	v->InvalidateNewGRFCache();
	u->InvalidateNewGRFCache();
}
static void VehicleConstructEngineCommon(void *shell, const OpenTTDEngine *engine) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(shell);
	const Engine *e = EngineOf(engine);
	v->last_station_visited = StationID::Invalid();
	v->last_loading_station = StationID::Invalid();
	v->engine_type = e->index;
	v->reliability = RustEngineReliability(e);
	v->reliability_spd_dec = e->reliability_spd_dec;
	v->max_age = e->GetLifeLengthInDays();
}
static void VehicleConstructServiceDates(void *shell, uint16_t interval) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(shell);
	v->SetServiceInterval(interval);
	v->date_of_last_service = TimerGameEconomy::date;
	v->date_of_last_service_newgrf = TimerGameCalendar::date;
}
static void VehicleConstructStamp(void *shell) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(shell);
	v->build_year = TimerGameCalendar::year;
	v->sprite_cache.sprite_seq.Set(SPR_IMG_QUERY);
}
static void VehicleConstructServiceFlags(void *shell, bool prototype, bool percent) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(shell);
	if (prototype) v->vehicle_flags.Set(VehicleFlag::BuiltAsPrototype);
	v->SetServiceIntervalIsPercent(percent);
}
static const OpenTTDVehicleConstructServices &GetVehicleConstructServices() noexcept
{
	static const OpenTTDVehicleConstructServices services{
		.articulated = &GetRustVehicleArticulatedServices(),
		.powered_roadtypes = Engine_powered_roadtypes, .any_roadtype = VehicleConstructRoadTile,
		.new_road = VehicleConstructNewRoad, .road_front = VehicleConstructRoadFront,
		.station = VehicleConstructStation, .can_use_station = VehicleConstructCanUse, .hangar_tile = VehicleConstructHangar,
		.pos_on_build = VehicleConstructPosOnBuild, .new_aircraft = VehicleConstructNewAircraft,
		.aircraft_place = VehicleConstructAircraftPlace, .aircraft_info = VehicleConstructAircraftInfo,
		.aircraft_properties = VehicleConstructAircraftProperties, .rotor = VehicleConstructRotor,
		.aircraft_capacity = VehicleConstructAircraftCapacity, .road_capacity = VehicleConstructRoadCapacity,
		.set_cargo_type = RustVehicleSetCargoType, .engine_common = VehicleConstructEngineCommon,
		.service_dates = VehicleConstructServiceDates, .build_stamp = VehicleConstructStamp,
		.exclusive_preview = RustEngineExclusivePreview, .service_flags = VehicleConstructServiceFlags,
		.wrong_depot = STR_ERROR_DEPOT_WRONG_DEPOT_TYPE,
	};
	return services;
}
static uint32_t VehicleBuildShipTile(uint32_t tile) noexcept { return GetShipDepotNorthTile(TileIndex(tile)).base(); }
static void *VehicleBuildNewShip(uint32_t tile) noexcept { Ship *v = new Ship(); v->tile = TileIndex(tile); return v; }
static void VehicleBuildSetPurchase(void *shell, uint16_t unit, int64_t value) noexcept { Vehicle *v = static_cast<Vehicle *>(shell); v->unitnumber = unit; v->value = value; }
static uint32_t VehicleBuildDisplayCapacity(const OpenTTDEngine *e, uint16_t *mail) noexcept { return EngineOf(e)->GetDisplayDefaultCapacity(mail); }
static void VehicleBuildNormalize(void *v) noexcept { NormalizeTrainVehInDepot(Train::From(static_cast<Vehicle *>(v))); }
static void VehicleBuildPresentation(void *shell, VehicleType type) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(shell);
	InvalidateWindowData(WC_VEHICLE_DEPOT, v->tile);
	InvalidateWindowClassesData(GetWindowClassForVehicleType(type), 0);
	SetWindowDirty(WC_COMPANY, OpenTTDCurrentCompany());
	if (RustIsLocalCompany()) InvalidateAutoreplaceWindow(v->engine_type, v->group_id);
}
static void VehicleBuildUseUnit(void *shell) noexcept { Vehicle *v = static_cast<Vehicle *>(shell); Company::Get(v->owner)->freeunits[v->type].UseID(v->unitnumber); }
const OpenTTDVehicleBuildServices &GetRustVehicleBuildServices() noexcept
{
	static const OpenTTDVehicleBuildServices services{
		.refit = &GetRustVehicleRefitServices(), .mass = &GetVehicleMassServices(), .random = &GetRustRandomServices(),
		.depot_kind = VehicleBuildDepotKind, .engine_cost = RustEngineCost, .default_cargo = RustEngineDefaultCargo,
		.articulate_count = VehicleBuildArticulatedCount, .pool_available = RustVehicleCanAllocate, .free_unit = VehicleBuildFreeUnit,
		.construct = &GetVehicleConstructServices(), .rail = VehicleBuildRail,
		.ship_tile = VehicleBuildShipTile, .new_ship = VehicleBuildNewShip, .set_purchase = VehicleBuildSetPurchase,
		.articulated_capacity = RustArticulatedCapacity, .display_capacity = VehicleBuildDisplayCapacity,
		.is_engine = RustOpaqueVehicleRead<bool, RustTrainIsEngine>, .normalize = VehicleBuildNormalize, .presentation = VehicleBuildPresentation, .use_unit = VehicleBuildUseUnit,
		.too_many = STR_ERROR_TOO_MANY_VEHICLES_IN_GAME, .unavailable = STR_ERROR_RAIL_VEHICLE_NOT_AVAILABLE,
	};
	return services;
}

static void VehicleCloneCopyService(void *source, void *destination) noexcept
{
	const Vehicle *v = static_cast<Vehicle *>(source);
	Vehicle *w = static_cast<Vehicle *>(destination);
	w->service_interval = v->service_interval;
	w->SetServiceIntervalIsCustom(v->ServiceIntervalIsCustom());
	w->SetServiceIntervalIsPercent(v->ServiceIntervalIsPercent());
}
static long VehicleCloneParseSuffix(OpenTTDByteSpan suffix) noexcept
{
	long num;
	const char *data = reinterpret_cast<const char *>(suffix.data);
	[[maybe_unused]] auto err = std::from_chars(data, data + suffix.size, num, 10).ec;
	assert(err == std::errc());
	return num;
}
const OpenTTDVehicleCloneServices &GetRustVehicleCloneServices() noexcept
{
	static const OpenTTDVehicleCloneServices services{
		.build = &GetRustVehicleBuildServices(),
		.next_vehicle = RustOpaqueVehicleRead<void *, RustVehicleNextVehicle<Vehicle>>,
		.has_articulated = RustOpaqueVehicleRead<bool, RustGroundHasArticulated<Vehicle>>,
		.next_articulated = RustOpaqueHandleRead<void *, RustVehicleNextArticulated>,
		.copy_service = VehicleCloneCopyService, .parse_suffix = VehicleCloneParseSuffix,
	};
	return services;
}
