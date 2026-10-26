/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file vehicle_build_services.hpp Native pools, specification helpers and presentation leaves. */
static VehicleType VehicleBuildDepotKind(uint32_t tile) noexcept { return GetDepotVehicleType(TileIndex(tile)); }
static uint16_t VehicleBuildFreeUnit(VehicleType type) noexcept { return GetFreeUnitNumber(type); }
/* Road vehicle and aircraft construction leaves: shell allocation and field stores. */
static bool VehicleConstructRoadTile(uint32_t tile, uint64_t powered) noexcept { return HasTileAnyRoadType(TileIndex(tile), RoadTypes(powered)); }
static void *VehicleConstructNewRoad(uint32_t tile) noexcept
{
	RoadVehicle *v = new RoadVehicle();
	v->direction = DiagDirToDir(static_cast<DiagDirection>(Shared_road_depot_direction(tile)));
	RustVehicleSetOwner(RustVehicle(v), RustCurrentCompanyID());
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
	RoadType rt = RustEngineRoadType(e);
	RustGroundSetFirstEngine(&v->gcache, EngineID::Invalid()); // needs to be set before first callback
	v->SetFrontEngine();
	v->roadtype = rt;
	v->compatible_roadtypes = RoadTypes(RustEnginePoweredRoadTypes(EngineOf(e)));
	v->gcache.cached_veh_length = VEHICLE_LENGTH;
}
static uint32_t VehicleConstructHangar(const OpenTTDStation *station, uint32_t tile, const AirportSpec *spec) noexcept
{
	const Station *st = StationOf(station);
	uint hangar = st->airport.GetHangarNum(TileIndex(tile), spec);
	return RustStationHangarTile(st, hangar, spec).base();
}
static void *VehicleConstructNewAircraft() noexcept { return new Aircraft(); }
static void VehicleConstructAircraftPlace(void *aircraft, void *shadow, uint32_t tile) noexcept
{
	Aircraft *v = static_cast<Aircraft *>(aircraft);
	Aircraft *u = static_cast<Aircraft *>(shadow);
	v->direction = DIR_SE;
	RustVehicleSetOwner(RustVehicle(u), RustCurrentCompanyID()); RustVehicleSetOwner(RustVehicle(v), RustVehicleOwner(u));
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
	return {.image = avi.image_index, .passengers = RustEnginePassengerCapacity(e), .mail = RustEngineMailCapacity(e), .subtype = RustAircraftInfoSubtype(&avi)};
}
static void VehicleConstructAircraftProperties(void *aircraft, void *shadow, const OpenTTDEngine *e, uint8_t subtype) noexcept
{
	Aircraft *v = static_cast<Aircraft *>(aircraft);
	Aircraft *u = static_cast<Aircraft *>(shadow);
	v->acceleration = EngineOf(e)->VehInfo<AircraftVehicleInfo>().acceleration;
	RustVehicleSetEngine(RustVehicle(u), RustEngineID(e));
	RustVehicleSetSubtype(v, subtype);
	RustVehicleSetSubtype(u, AIR_SHADOW);
	v->vehicle_flags = {};
	RustVehicleSetNext(v, u);
}
static void VehicleConstructRotor(void *rotor, void *aircraft, void *shadow) noexcept
{
	Aircraft *w = static_cast<Aircraft *>(rotor);
	const Aircraft *v = static_cast<Aircraft *>(aircraft);
	RustVehicleSetEngine(RustVehicle(w), RustVehicleEngineID(v));
	w->direction = DIR_N;
	RustVehicleSetOwner(RustVehicle(w), RustCurrentCompanyID());
	w->x_pos = v->x_pos;
	w->y_pos = v->y_pos;
	w->z_pos = v->z_pos + ROTOR_Z_OFFSET;
	w->vehstatus = {VehState::Hidden, VehState::Unclickable};
	w->spritenum = 0xFF;
	RustVehicleSetSubtype(w, AIR_ROTOR);
	w->sprite_cache.sprite_seq.Set(SPR_ROTOR_STOPPED);
	RustVehicleSetNext(static_cast<Vehicle *>(shadow), w);
}
static void VehicleConstructRoadCapacity(void *front, void *part, uint16_t capacity) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(front);
	Vehicle *u = static_cast<Vehicle *>(part);
	u->cargo_cap = capacity;
	u->refit_cap = 0;
	RustVehicleInvalidateNewGRFCache(v);
	RustVehicleInvalidateNewGRFCache(u);
}
static void VehicleConstructEngineCommon(void *shell, const OpenTTDEngine *engine) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(shell);
	const Engine *e = EngineOf(engine);
	v->last_station_visited = StationID::Invalid();
	RustVehicleSetLastLoadingStation(RustVehicle(v), StationID::Invalid().base());
	RustVehicleSetEngine(RustVehicle(v), e->index.base());
	v->reliability = RustEngineReliability(e);
	v->reliability_spd_dec = e->reliability_spd_dec;
	v->max_age = RustEngineLifeLength(e);
}
static void VehicleConstructServiceDates(void *shell, uint16_t interval) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(shell);
	RustVehicleSetServiceInterval(v, interval);
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
		.station = OpenTTDStationByTile, .hangar_tile = VehicleConstructHangar,
		.new_aircraft = VehicleConstructNewAircraft,
		.aircraft_place = VehicleConstructAircraftPlace, .aircraft_info = VehicleConstructAircraftInfo,
		.aircraft_properties = VehicleConstructAircraftProperties, .rotor = VehicleConstructRotor,
		.mail_capacity_address = RustOpaqueVehicleInvoke<RustVehicleCargoCapacityAddress>, .road_capacity = VehicleConstructRoadCapacity,
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
static void VehicleBuildPresentation(void *shell, VehicleType type) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(shell);
	RustVehicleInvalidateDepot(v);
	InvalidateWindowClassesData(GetWindowClassForVehicleType(type), 0);
	SetWindowDirty(WC_COMPANY, OpenTTDCurrentCompany());
	if (RustIsLocalCompany()) InvalidateAutoreplaceWindow(v->engine_type, v->group_id);
}
const OpenTTDVehicleBuildServices &GetRustVehicleBuildServices() noexcept
{
	static const OpenTTDVehicleBuildServices services{
		.refit = &GetRustVehicleRefitServices(), .mass = &GetVehicleMassServices(), .random = &GetRustRandomServices(),
		.depot_kind = VehicleBuildDepotKind, .engine_cost = RustEngineCost, .default_cargo = RustEngineDefaultCargo,
		.pool_available = RustVehicleCanAllocate, .free_unit = VehicleBuildFreeUnit,
		.construct = &GetVehicleConstructServices(), .rail = &GetRustRailConstructionServices(),
		.ship_tile = VehicleBuildShipTile, .new_ship = VehicleBuildNewShip, .set_purchase = VehicleBuildSetPurchase,

		.is_engine = RustOpaqueVehicleRead<bool, RustTrainIsEngine>, .presentation = VehicleBuildPresentation, .use_unit = RustVehicleUseCurrentUnit,
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
