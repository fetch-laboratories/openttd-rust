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
static void *VehicleBuildRoad(OpenTTDCommandCost *out, uint16_t flags, uint32_t tile, const OpenTTDEngine *e) noexcept
{
	Vehicle *v = nullptr;
	*reinterpret_cast<CommandCost *>(out) = CmdBuildRoadVehicle(DoCommandFlags(flags), TileIndex(tile), EngineOf(e), &v);
	return v;
}
static void *VehicleBuildAircraft(OpenTTDCommandCost *out, uint16_t flags, uint32_t tile, const OpenTTDEngine *e) noexcept
{
	Vehicle *v = nullptr;
	*reinterpret_cast<CommandCost *>(out) = CmdBuildAircraft(DoCommandFlags(flags), TileIndex(tile), EngineOf(e), &v);
	return v;
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
		.rail = VehicleBuildRail, .road = VehicleBuildRoad, .aircraft = VehicleBuildAircraft,
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
		.next_articulated = RustOpaqueVehicleRead<void *, RustVehicleNextArticulated<Vehicle>>,
		.copy_service = VehicleCloneCopyService, .parse_suffix = VehicleCloneParseSuffix,
	};
	return services;
}
