/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file vehicle_articulated_services.hpp Native specification, pool and shell leaves. */
static uint16_t VehicleArticNext(uint32_t index, uint16_t engine, void *v, bool *flip) noexcept { return GetNextArticulatedPart(index, EngineID(engine), static_cast<Vehicle *>(v), flip).base(); }
static void *VehicleArticPreparePrevious(void *shell) noexcept { Vehicle *v = static_cast<Vehicle *>(shell); auto *cache = v->GetGroundVehicleCache(); RustGroundSetFirstEngine(cache, v->engine_type); return cache; }
template <class T> static void *VehicleArticNew(void *previous) noexcept { T *v = new T(); static_cast<Vehicle *>(previous)->SetNext(v); v->subtype = 0; return v; }
static void VehicleArticRoadLength(void *cache) noexcept { static_cast<GroundVehicleCache *>(cache)->cached_veh_length = VEHICLE_LENGTH; }
static void VehicleArticCopyRoadtype(void *shell, void *head) noexcept { RoadVehicle *v = static_cast<RoadVehicle *>(shell); const auto *first = static_cast<const RoadVehicle *>(head); v->roadtype = first->roadtype; v->compatible_roadtypes = first->compatible_roadtypes; }
template <class T> static OpenTTDArticulatedPart VehicleArticInfo(const OpenTTDEngine *engine, bool carry) noexcept { const auto &info = EngineOf(engine)->VehInfo<T>(); return {.image = info.image_index, .capacity = static_cast<uint16_t>(carry ? info.capacity : 0)}; }
static void VehicleArticSetPart(void *shell, uint8_t image, uint8_t cargo, uint16_t capacity, bool validate) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(shell);
	v->spritenum = image;
	RustVehicleSetCargoType(RustVehicle(v), cargo);
	if (validate) assert(OpenTTDIsValidCargo(RustVehicleCargoType(RustVehicle(v))));
	RustVehicleSetCargoCap(RustVehicle(v), capacity);
	RustVehicleSetRefitCap(RustVehicle(v), 0);
}
template <class T> static void VehicleArticMark(void *v) noexcept { static_cast<T *>(v)->SetArticulatedPart(); }
static void VehicleArticCopyCommon(void *shell, void *head, uint16_t engine) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(shell);
	const Vehicle *first = static_cast<Vehicle *>(head);
	v->direction = first->direction; v->owner = first->owner; v->tile = first->tile;
	v->x_pos = first->x_pos; v->y_pos = first->y_pos; v->z_pos = first->z_pos;
	v->date_of_last_service = first->date_of_last_service;
	v->date_of_last_service_newgrf = first->date_of_last_service_newgrf;
	v->build_year = first->build_year;
	v->vehstatus = first->vehstatus; v->vehstatus.Reset(VehState::Stopped);
	RustVehicleSetCargoSubtype(RustVehicle(v), 0);
	v->max_age = CalendarTime::MIN_DATE; v->engine_type = EngineID(engine); v->value = 0;
	v->sprite_cache.sprite_seq.Set(SPR_IMG_QUERY);
}
static void VehicleArticFlip(void *shell) noexcept { ++static_cast<Vehicle *>(shell)->spritenum; }
static OpenTTDArticulatedProbability VehicleArticReverse(void *v, uint16_t engine) noexcept { auto p = RustVehicleBuildProbability(static_cast<Vehicle *>(v), EngineID(engine), BuildProbabilityType::Reversed); return {.present = p.has_value(), .value = p.value_or(false)}; }
static uint64_t VehicleArticAvailableCargo(uint16_t engine, bool purchase) noexcept { return GetAvailableVehicleCargoTypes(EngineID(engine), purchase); }
static void VehicleArticWarning(const OpenTTDEngine *e) noexcept { ShowNewGrfVehicleError(EngineOf(e)->index, STR_NEWGRF_BUGGY, STR_NEWGRF_BUGGY_ARTICULATED_CARGO, GRFBug::VehRefit, false); }
const OpenTTDVehicleArticulatedServices &GetRustVehicleArticulatedServices() noexcept
{
	static const OpenTTDVehicleArticulatedServices services{
		.refit = &GetRustVehicleRefitServices(), .default_cargo = RustEngineDefaultCargo, .capacity = RustArticulatedCapacity,
		.pool_available = RustVehicleCanAllocate, .has_callback = RustEngineHasArticulatedCallback, .next_part = VehicleArticNext,
		.prepare_previous = VehicleArticPreparePrevious, .new_train = VehicleArticNew<Train>, .new_road = VehicleArticNew<RoadVehicle>,
		.road_length = VehicleArticRoadLength, .copy_roadtype = VehicleArticCopyRoadtype,
		.train_info = VehicleArticInfo<RailVehicleInfo>, .road_info = VehicleArticInfo<RoadVehicleInfo>,
		.set_part = VehicleArticSetPart, .mark_train = VehicleArticMark<Train>, .mark_road = VehicleArticMark<RoadVehicle>,
		.copy_common = VehicleArticCopyCommon, .random_bits = RustOpaqueVehicleInvoke<RustVehicleSetRandomBits, uint16_t>,
		.flip_image = VehicleArticFlip, .reverse = VehicleArticReverse, .position = RustOpaqueVehicleInvoke<RustVehicleUpdatePosition<Vehicle>>,
		.available_cargo = VehicleArticAvailableCargo, .warning = VehicleArticWarning,
		.has_articulated = RustOpaqueVehicleRead<bool, RustGroundHasArticulated<Vehicle>>,
		.next_articulated = RustOpaqueHandleRead<void *, RustVehicleNextArticulated>, .max_parts = MAX_ARTICULATED_PARTS, .all_cargo = ALL_CARGOTYPES,
	};
	return services;
}
