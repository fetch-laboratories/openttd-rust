/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file rail_construction_services.hpp Native rail shell allocation, field stores and presentation leaves. */

/* Original `CmdBuildRailWagon` statements from the allocation through the sprite
 * cache, in source order; Rust draws the random bits that follow. */
static void *RailNewWagon(uint32_t tile_index, const OpenTTDEngine *engine) noexcept
{
	const Engine *e = EngineOf(engine);
	const RailVehicleInfo *rvi = &e->VehInfo<RailVehicleInfo>();
	TileIndex tile(tile_index);
	Train *v = new Train();
	v->spritenum = rvi->image_index;

	v->engine_type = e->index;
	RustGroundSetFirstEngine(&v->gcache, EngineID::Invalid()); // needs to be set before first callback

	DiagDirection dir = RustRailDepotDirection(tile.base());

	v->direction = DiagDirToDir(dir);
	v->tile = tile;

	int x = TileX(tile) * TILE_SIZE | _vehicle_initial_x_fract[dir];
	int y = TileY(tile) * TILE_SIZE | _vehicle_initial_y_fract[dir];

	v->x_pos = x;
	v->y_pos = y;
	v->z_pos = SharedSlopePixelZ(x, y, true);
	v->owner = _current_company;
	v->SetTrack(TRACK_BIT_DEPOT);
	v->vehstatus = {VehState::Hidden, VehState::DefaultPalette};

	v->SetWagon();

	v->SetFreeWagon();
	RustVehicleInvalidateDepot(v);

	v->cargo_type = RustEngineDefaultCargo(engine);
	assert(IsValidCargoType(v->cargo_type));
	v->cargo_cap = rvi->capacity;
	v->refit_cap = 0;

	v->SetRailTypes(RailTypes{RustEngineRailTypes(e)});

	v->date_of_last_service = TimerGameEconomy::date;
	v->date_of_last_service_newgrf = TimerGameCalendar::date;
	v->build_year = TimerGameCalendar::year;
	v->sprite_cache.sprite_seq.Set(SPR_IMG_QUERY);
	return v;
}
/* Original `CmdBuildRailVehicle` statements through the sprite cache. */
static void *RailNewEngine(uint32_t tile_index, const OpenTTDEngine *engine) noexcept
{
	const Engine *e = EngineOf(engine);
	const RailVehicleInfo *rvi = &e->VehInfo<RailVehicleInfo>();
	TileIndex tile(tile_index);
	DiagDirection dir = RustRailDepotDirection(tile.base());
	int x = TileX(tile) * TILE_SIZE + _vehicle_initial_x_fract[dir];
	int y = TileY(tile) * TILE_SIZE + _vehicle_initial_y_fract[dir];

	Train *v = new Train();
	v->direction = DiagDirToDir(dir);
	v->tile = tile;
	v->owner = _current_company;
	v->x_pos = x;
	v->y_pos = y;
	v->z_pos = SharedSlopePixelZ(x, y, true);
	v->SetTrack(TRACK_BIT_DEPOT);
	v->vehstatus = {VehState::Hidden, VehState::Stopped, VehState::DefaultPalette};
	v->spritenum = rvi->image_index;
	v->cargo_type = RustEngineDefaultCargo(engine);
	assert(IsValidCargoType(v->cargo_type));
	v->cargo_cap = rvi->capacity;
	v->refit_cap = 0;
	v->last_station_visited = StationID::Invalid();
	RustVehicleSetLastLoadingStation(RustVehicle(v), StationID::Invalid().base());

	v->engine_type = e->index;
	RustGroundSetFirstEngine(&v->gcache, EngineID::Invalid()); // needs to be set before first callback

	v->reliability = e->reliability;
	v->reliability_spd_dec = e->reliability_spd_dec;
	v->max_age = RustEngineLifeLength(e);

	v->SetRailTypes(RailTypes{RustEngineRailTypes(e)});

	RustVehicleSetServiceInterval(v, Company::Get(_current_company)->settings.vehicle.servint_trains);
	v->date_of_last_service = TimerGameEconomy::date;
	v->date_of_last_service_newgrf = TimerGameCalendar::date;
	v->build_year = TimerGameCalendar::year;
	v->sprite_cache.sprite_seq.Set(SPR_IMG_QUERY);
	return v;
}
/* Original `AddRearEngineToMultiheadedTrain` statements through the sprite cache,
 * including the front's `Money` value halving. */
static void *RailNewRear(void *front) noexcept
{
	Train *v = static_cast<Train *>(front);
	Train *u = new Train();
	v->value >>= 1;
	u->value = v->value;
	u->direction = v->direction;
	u->owner = v->owner;
	u->tile = v->tile;
	u->x_pos = v->x_pos;
	u->y_pos = v->y_pos;
	u->z_pos = v->z_pos;
	u->SetTrack(TRACK_BIT_DEPOT);
	u->vehstatus = v->vehstatus;
	u->vehstatus.Reset(VehState::Stopped);
	u->spritenum = RustVehicleSprite(v) + 1;
	u->cargo_type = v->cargo_type;
	u->cargo_subtype = v->cargo_subtype;
	u->cargo_cap = v->cargo_cap;
	u->refit_cap = v->refit_cap;
	u->SetRailTypes(v->GetRailTypes());
	u->engine_type = v->engine_type;
	u->date_of_last_service = v->date_of_last_service;
	u->date_of_last_service_newgrf = RustVehicleCalendarService(v);
	u->build_year = RustVehicleBuildYear(v);
	u->sprite_cache.sprite_seq.Set(SPR_IMG_QUERY);
	return u;
}
static void RailLinkMultihead(void *front, void *rear) noexcept
{
	Train *v = static_cast<Train *>(front);
	Train *u = static_cast<Train *>(rear);
	v->other_multiheaded_part = u;
	u->other_multiheaded_part = v;
}
static void RailHeadWindows(void *shell) noexcept
{
	const Vehicle *head = static_cast<Vehicle *>(shell);
	InvalidateWindowData(WC_VEHICLE_REFIT, head->index, VIWD_CONSIST_CHANGED);
	SetWindowWidgetDirty(WC_VEHICLE_VIEW, head->index, WID_VV_REFIT);
}
static void RailFrontWindows(void *shell) noexcept
{
	const Vehicle *src = static_cast<Vehicle *>(shell);
	CloseWindowById(WC_VEHICLE_VIEW, src->index);
	CloseWindowById(WC_VEHICLE_ORDERS, src->index);
	CloseWindowById(WC_VEHICLE_REFIT, src->index);
	CloseWindowById(WC_VEHICLE_DETAILS, src->index);
	CloseWindowById(WC_VEHICLE_TIMETABLE, src->index);
	DeleteNewGRFInspectWindow(GSF_TRAINS, src->index);
	SetWindowDirty(WC_COMPANY, _current_company);
}
static void RailCompanyDirty() noexcept { SetWindowDirty(WC_COMPANY, OpenTTDCurrentCompany()); }
static void RailDepotChanged(uint32_t tile) noexcept
{
	RustVehicleInvalidateDepotTile(TileIndex(tile));
	InvalidateWindowClassesData(WC_TRAINS_LIST, 0);
}
const OpenTTDRailConstructionServices &GetRustRailConstructionServices() noexcept
{
	static const OpenTTDRailConstructionServices services{
		.cargo_capacity = &GetRustCargoCapacityServices(),
		.strings = {.rear_follow = STR_ERROR_REAR_ENGINE_FOLLOW_FRONT, .depot_only = STR_ERROR_TRAINS_CAN_ONLY_BE_ALTERED_INSIDE_A_DEPOT, .no_orders = STR_ERROR_NO_MORE_SPACE_FOR_ORDERS},
		.train_if_valid = RustTrainGetIfValid,
		.compatible = RustEngineCompatibleRail,
		.powered = RustEngineHasPowerOnRail,
		.new_wagon = RailNewWagon,
		.new_engine = RailNewEngine,
		.new_rear = RailNewRear,
		.link_multihead = RailLinkMultihead,
		.subtype = RustOpaqueVehicleRead<uint8_t, RustVehicleSubtype>,
		.set_subtype = RustOpaqueVehicleInvoke<RustVehicleSetSubtype, uint8_t>,
		.previous = RustOpaqueVehicleRead<void *, RustVehiclePrevious>,
		.last = RustOpaqueVehicleRead<void *, RustVehicleLast<Vehicle>>,
		.set_next = RustOpaqueVehicleSetNext,
		.next_vehicle = RustOpaqueVehicleRead<void *, RustVehicleNextVehicle<Vehicle>>,
		.last_engine_part = RustOpaqueVehicleRead<void *, RustVehicleLastEnginePart>,
		.ground = &GetRustGroundServices<Train>(),
		.first_engine = RustOpaqueVehicleRead<uint16_t, RustGroundFirstEngine>,
		.invalidate_grf = RustOpaqueVehicleInvoke<RustVehicleInvalidateNewGRFCache>,
		.unknown_attach = RustVehicleCallbackError<CBID_TRAIN_ALLOW_WAGON_ATTACH>,
		.train_limit = RustTrainLimit,
		.unit = RustOpaqueVehicleRead<uint16_t, RustVehicleUnitNumber>,
		.use_unit = RustOpaqueVehicleInvoke<RustVehicleUseUnitNumber, uint16_t>,
		.release_unit = RustOpaqueVehicleInvoke<RustVehicleReleaseUnitNumber>,
		.head_windows = RailHeadWindows,
		.front_windows = RailFrontWindows,
		.company_dirty = RailCompanyDirty,
		.depot_changed = RailDepotChanged,
	};
	return services;
}
