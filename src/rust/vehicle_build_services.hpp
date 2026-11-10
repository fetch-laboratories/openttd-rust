/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file vehicle_build_services.hpp Native pools, specification helpers and presentation leaves. */
static VehicleType VehicleBuildDepotKind(uint32_t tile) noexcept { return GetDepotVehicleType(TileIndex(tile)); }
static uint16_t VehicleBuildFreeUnit(VehicleType type) noexcept { return GetFreeUnitNumber(type); }
/* Road vehicle and aircraft construction leaves: shell allocation and field stores. */
static void *VehicleConstructNewRoad(uint32_t tile) noexcept
{
	RoadVehicle *v = new RoadVehicle();
	v->SetDirection(DiagDirToDir(static_cast<DiagDirection>(Shared_road_depot_direction(tile))));
	RustVehicleSetOwner(RustVehicle(v), RustCurrentCompanyID());
	RustWriteVehicleTile(v, TileIndex(tile));
	int x = TileX(v->GetTile()) * TILE_SIZE + TILE_SIZE / 2;
	int y = TileY(v->GetTile()) * TILE_SIZE + TILE_SIZE / 2;
	RustWriteVehicleX(v, x);
	RustWriteVehicleY(v, y);
	RustWriteVehicleZ(v, SharedSlopePixelZ(x, y, true));
	RustWriteVehicleStatus(v, {VehState::Hidden, VehState::Stopped, VehState::DefaultPalette});
	return v;
}
static void VehicleConstructRoadFront(void *shell, RoadType rt, uint64_t powered) noexcept
{
	RoadVehicle *v = static_cast<RoadVehicle *>(shell);
	RustGroundSetFirstEngine(&v->gcache, EngineID::Invalid()); // needs to be set before first callback
	v->SetFrontEngine();
	v->roadtype = rt;
	v->compatible_roadtypes = RoadTypes(powered);
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
	v->SetDirection(DIR_SE);
	RustVehicleSetOwner(RustVehicle(u), RustCurrentCompanyID()); RustVehicleSetOwner(RustVehicle(v), RustVehicleOwner(u));
	RustWriteVehicleTile(v, TileIndex(tile));
	uint x = TileX(v->GetTile()) * TILE_SIZE + 5;
	uint y = TileY(v->GetTile()) * TILE_SIZE + 3;
	RustWriteVehicleX(u, x); RustWriteVehicleX(v, x);
	RustWriteVehicleY(u, y); RustWriteVehicleY(v, y);
	RustWriteVehicleZ(u, SharedSlopePixelZ(x, y, false));
	RustWriteVehicleZ(v, u->Record().z_pos + 1);
	RustWriteVehicleStatus(v, {VehState::Hidden, VehState::Stopped, VehState::DefaultPalette});
	RustWriteVehicleStatus(u, {VehState::Hidden, VehState::Unclickable, VehState::Shadow});
}
static void VehicleConstructAircraftProperties(void *aircraft, void *shadow, uint8_t acceleration, uint16_t engine, uint8_t subtype) noexcept
{
	Aircraft *v = static_cast<Aircraft *>(aircraft);
	Aircraft *u = static_cast<Aircraft *>(shadow);
	v->Record().acceleration = acceleration;
	RustVehicleSetEngine(RustVehicle(u), engine);
	RustVehicleSetSubtype(v, subtype);
	RustVehicleSetSubtype(u, AIR_SHADOW);
	v->vehicle_flags = {};
}
static void VehicleConstructRotor(void *rotor, void *aircraft, void *) noexcept
{
	Aircraft *w = static_cast<Aircraft *>(rotor);
	const Aircraft *v = static_cast<Aircraft *>(aircraft);
	RustVehicleSetEngine(RustVehicle(w), RustVehicleEngineID(v));
	w->SetDirection(DIR_N);
	RustVehicleSetOwner(RustVehicle(w), RustCurrentCompanyID());
	RustWriteVehicleX(w, v->Record().x_pos);
	RustWriteVehicleY(w, v->Record().y_pos);
	RustWriteVehicleZ(w, v->Record().z_pos + ROTOR_Z_OFFSET);
	RustWriteVehicleStatus(w, {VehState::Hidden, VehState::Unclickable});
	RustWriteVehicleSprite(w, 0xFF);
	RustVehicleSetSubtype(w, AIR_ROTOR);
	RustWriteVehicleSpriteSequence(w, SPR_ROTOR_STOPPED);
}
static void VehicleConstructRoadCapacity(void *front, void *part, uint16_t capacity) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(front);
	Vehicle *u = static_cast<Vehicle *>(part);
	u->Record().cargo_cap = capacity;
	u->Record().refit_cap = 0;
	RustVehicleInvalidateNewGRFCache(v);
	RustVehicleInvalidateNewGRFCache(u);
}
static void VehicleConstructEngineCommon(void *shell, uint16_t engine, uint16_t reliability, uint16_t decay) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(shell);
	v->SetLastStationVisited(StationID::Invalid());
	RustVehicleSetLastLoadingStation(RustVehicle(v), StationID::Invalid().base());
	RustVehicleSetEngine(RustVehicle(v), engine);
	v->reliability = reliability;
	v->reliability_spd_dec = decay;
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
	RustWriteVehicleSpriteSequence(v, SPR_IMG_QUERY);
}
static void VehicleConstructServiceFlags(void *shell, bool prototype, bool percent) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(shell);
	if (prototype) v->vehicle_flags.Set(VehicleFlag::BuiltAsPrototype);
	RustVehicleSetServiceIntervalIsPercent(v, percent);
}
static const OpenTTDVehicleConstructServices &GetVehicleConstructServices() noexcept
{
	static const OpenTTDVehicleConstructServices services{
		.articulated = &GetRustVehicleArticulatedServices(),
		.new_road = VehicleConstructNewRoad, .road_front = VehicleConstructRoadFront,
		.station = OpenTTDStationByTile, .hangar_tile = VehicleConstructHangar,
		.new_aircraft = VehicleConstructNewAircraft,
		.aircraft_place = VehicleConstructAircraftPlace,
		.aircraft_properties = VehicleConstructAircraftProperties, .rotor = VehicleConstructRotor,
		.mail_capacity_address = RustOpaqueVehicleInvoke<RustVehicleCargoCapacityAddress>, .road_capacity = VehicleConstructRoadCapacity,
		.set_cargo_type = RustVehicleSetCargoType, .engine_common = VehicleConstructEngineCommon,
		.service_dates = VehicleConstructServiceDates, .build_stamp = VehicleConstructStamp,
		.service_flags = VehicleConstructServiceFlags,
		.wrong_depot = STR_ERROR_DEPOT_WRONG_DEPOT_TYPE,
	};
	return services;
}
static uint32_t VehicleBuildShipTile(uint32_t tile) noexcept { return GetShipDepotNorthTile(TileIndex(tile)).base(); }
static void *VehicleBuildNewShip(uint32_t tile) noexcept { Ship *v = new Ship(); RustWriteVehicleTile(v, TileIndex(tile)); return v; }
static void VehicleBuildSetPurchase(void *shell, uint16_t unit, int64_t value) noexcept { Vehicle *v = static_cast<Vehicle *>(shell); v->unitnumber = unit; v->value = value; }
static void VehicleBuildPresentation(void *shell, VehicleType type) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(shell);
	RustVehicleInvalidateDepot(v);
	InvalidateWindowClassesData(GetWindowClassForVehicleType(type), 0);
	SetWindowDirty(WC_COMPANY, OpenTTDCurrentCompany());
	if (RustIsLocalCompany()) InvalidateAutoreplaceWindow(v->GetEngineType(), v->group_id);
}
const OpenTTDVehicleBuildServices &GetRustVehicleBuildServices() noexcept
{
	static const OpenTTDVehicleBuildServices services{
		.refit = &GetRustVehicleRefitServices(), .mass = &GetVehicleMassServices(), .random = &GetRustRandomServices(),
		.depot_kind = VehicleBuildDepotKind,
		.free_unit = VehicleBuildFreeUnit,
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
	RustVehicleSetServiceIntervalIsPercent(w, v->ServiceIntervalIsPercent());
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
