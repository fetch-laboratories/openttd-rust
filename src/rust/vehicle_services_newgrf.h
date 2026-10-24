/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file vehicle_services_newgrf.h Shared vehicle NewGRF observations and evaluator adapters. */
#ifndef RUST_VEHICLE_SERVICES_NEWGRF_H
#define RUST_VEHICLE_SERVICES_NEWGRF_H
#include "vehicle_services.h"
#include "services_assert.h"
#include "engine_services.h"
#include "services_ffi.h"
#include "services_cargo.h"
#include "services_road.h"
#include "../newgrf_engine.h"
#include "../newgrf.h"
#include "../company_base.h"
#include "../aircraft.h"
#include "../ship.h"
#include "../newgrf_badge.h"
#include "../newgrf_railtype.h"
#include "../newgrf_roadtype.h"
#include "../debug.h"
#include "aircraft_ffi.h"
#include "ground_vehicle_ffi.h"
#include "settings_services.h"
#include "cargo_storage_ffi.h"
#include "services_station.h"

static_assert(CalendarTime::ORIGINAL_BASE_YEAR.base() == 1920 && CalendarTime::ORIGINAL_MAX_YEAR.base() == 2090);
static_assert(CalendarTime::DAYS_TILL_ORIGINAL_BASE_YEAR.base() == 701265);
static_assert(INVALID_RAILTYPE == 255 && INVALID_ROADTYPE == 63);
static_assert(ATP_TTDP_LARGE == 1 && DIR_W == 6 && DIRDIFF_REVERSE == 4);
static_assert(to_underlying(VehicleRailFlag::PoweredWagon) == 3 && to_underlying(VehicleRailFlag::Flipped) == 4 && to_underlying(VehicleRailFlag::Reversed) == 7);
/* Native observations and common evaluator adapters. No Vehicle layout crosses
 * this boundary; aliases into Rust's cache remain canonical until shell death. */
static uint16_t VehicleResolverCallback(OpenTTDVehicleResolverState *state, uint16_t engine, uint16_t callback, uint32_t p1, uint32_t p2, int32_t *regs, size_t count) noexcept
{
	VehicleResolverObject object(*state, EngineID(engine), static_cast<CallbackID>(callback), p1, p2);
	return object.ResolveCallback({regs, count});
}
static OpenTTDVehicleRandomResult VehicleResolverRandom(OpenTTDVehicleResolverState *state, uint16_t engine, uint8_t waiting) noexcept
{
	VehicleResolverObject object(*state, EngineID(engine), CBID_RANDOM_TRIGGER, 0, 0);
	object.SetWaitingRandomTriggers(VehicleRandomTriggers(waiting));
	object.ResolveRerandomisation();
	return {object.GetUsedRandomTriggers().base(), object.GetReseedSum()};
}
const OpenTTDVehicleResolverServices &GetRustVehicleResolverServices() noexcept
{
	static const OpenTTDVehicleResolverServices services{
		.vehicle = &GetRustVehicleServices(), .shared = &GetRustSharedServices(),
		.cache = [](OpenTTDVehicle *v) noexcept { return &VehicleOf(v)->grf_cache; },
		.first = RustGroundFirst,
		.move = [](OpenTTDVehicle *v, int32_t count) noexcept { return RustVehicle(VehicleOf(v)->Move(count)); },
		.previous = [](OpenTTDVehicle *v) noexcept { return RustVehicle(RustVehiclePrevious(VehicleOf(v))); },
		.can_carry = Engine_can_carry,
		.cargo_classes = RustCargoClasses,
		.cargo_bitnum = RustCargoBitnum,
		.cargo_translation = RustGRFCargoMap,
		.grf_version = RustGRFFileVersion,
		.get_register = RustResolverRegister,
		.current_company = RustCurrentCompanyID,
		.valid_company = OpenTTDCompanyIsValid,
		.livery = [](uint16_t engine, uint8_t owner, uint16_t first, OpenTTDVehicle *v) noexcept { return GetEngineLivery(EngineID(engine), CompanyID(owner), EngineID(first), VehicleOf(v), LIT_ALL); },
		.company_info = RustCompanyInfo,
		.first_engine = [](OpenTTDVehicle *v) noexcept { return RustGroundFirstEngine(VehicleOf(v)); },
		.articulated = RustVehicleRead<bool, RustVehicleArticulated>,
		.has_articulated = RustVehicleHasArticulated,
		.overrides = [](uint16_t engine) noexcept -> OpenTTDVehicleOverrides { const Engine *e = Engine::Get(EngineID(engine)); return {RustEngine(e), e->overrides.size()}; },
		.override_at = [](const OpenTTDEngine *engine, size_t i) noexcept -> OpenTTDVehicleOverride { const auto &o = EngineOf(engine)->overrides[i]; return {o.group, o.cargo, o.engines.size()}; },
		.override_engine = [](const OpenTTDEngine *engine, size_t i, size_t j) noexcept { return EngineOf(engine)->overrides[i].engines[j].base(); },
		.cached_override = [](OpenTTDVehicle *v) noexcept { return Train::From(VehicleOf(v))->tcache.cached_override; },
		.write_override = [](OpenTTDVehicle *v, const SpriteGroup *group) noexcept { Train::From(VehicleOf(v))->tcache.cached_override = group; },
		.spritegroup = [](uint16_t engine, uint8_t cargo) noexcept { return RustSpecCargoSpriteGroup(Engine::Get(EngineID(engine)), cargo); },
		.random_bits = [](OpenTTDVehicle *v) noexcept { return VehicleOf(v)->random_bits; },
		.waiting = [](OpenTTDVehicle *v) noexcept { return VehicleOf(v)->waiting_random_triggers.base(); },
		.write_random = [](OpenTTDVehicle *v, uint16_t bits) noexcept { RustVehicleSetRandomBits(VehicleOf(v), bits); },
		.write_waiting = [](OpenTTDVehicle *v, uint8_t waiting) noexcept { VehicleOf(v)->waiting_random_triggers = VehicleRandomTriggers(waiting); },
		.resolve_callback = VehicleResolverCallback, .resolve_random = VehicleResolverRandom,
		.order = [](OpenTTDVehicle *v) noexcept -> OpenTTDVehicleLoadingOrder { const Vehicle *u = VehicleOf(v); return {to_underlying(RustVehicleOrderUnloadType(u)), to_underlying(RustVehicleOrderLoadType(u)), RustVehicleOrderType(u) == OT_LOADING}; },
		.cargo_owner = RustVehicleRead<OpenTTDCargoList *, RustVehicleCargoOwner>,
		.cargo_capacity = RustVehicleCargoCap,
		.real_count = [](const RealSpriteGroup *group, bool motion) noexcept { return static_cast<uint32_t>(motion ? RustRealLoadedCount(group) : RustRealLoadingCount(group)); },
		.real_at = [](const RealSpriteGroup *group, bool motion, uint32_t index) noexcept { return motion ? RustRealLoaded(group, index) : RustRealLoading(group, index); },
		.vehicle_grfid = RustVehicleRead<uint32_t, RustVehicleGRFID>,
		.engine_by_id = RustEngineByID,
		.default_cargo = RustEngineDefaultCargo,
		.engine_flags = [](const OpenTTDEngine * e) noexcept -> uint8_t { return EngineOf(e)->flags.base(); },
		.local_id = [](const OpenTTDEngine * e) noexcept -> uint16_t { return EngineOf(e)->grf_prop.local_id; },
		.cargo_info = [](uint8_t cargo) noexcept -> OpenTTDVehicleCargoInfo { const CargoSpec *cs = CargoSpec::Get(cargo); return {RustCargoSpecClasses(cs), cs->weight}; },
		.motion = [](OpenTTDVehicle * v) noexcept -> uint32_t { return *RustVehicleMotionAddress(VehicleOf(v)); },
		.build_year = [](OpenTTDVehicle * v) noexcept -> int32_t { return RustVehicleBuildYear(VehicleOf(v)).base(); },
		.last_service = [](OpenTTDVehicle * v) noexcept -> int32_t { return RustVehicleCalendarService(VehicleOf(v)).base(); },
		.primary = [](OpenTTDVehicle * v) noexcept -> bool { return RustVehiclePrimary(VehicleOf(v)); },
		.train_services = []() noexcept { return &GetTrainServices(); },
		.road_acceleration = RustRoadAccelerationModel,
		.order_max_speed = RustVehicleRead<uint16_t, RustVehicleOrderMaxSpeed>,
		.id = [](OpenTTDVehicle * v) noexcept -> uint32_t { return RustVehicleIndexOpaque(VehicleOf(v)); },
		.old_order = [](OpenTTDVehicle * v) noexcept -> uint16_t { return VehicleOf(v)->current_order.MapOldOrder(); },
		.num_orders = RustVehicleRead<uint8_t, RustVehicleNumOrders>,
		.real_order = RustVehicleRead<uint8_t, RustVehicleRealOrderIndex>,
		.load_ticks = RustVehicleLoadUnloadTicks,
		.service_interval = RustVehicleRead<uint16_t, RustVehicleServiceInterval>,
		.acceleration = [](OpenTTDVehicle * v) noexcept -> uint8_t { return RustVehicleAcceleration(VehicleOf(v)); },
		.cargo_type = [](OpenTTDVehicle * v) noexcept -> uint8_t { return RustVehicleCargoType(v); },
		.cargo_subtype = [](OpenTTDVehicle * v) noexcept -> uint8_t { return RustVehicleCargoSubtype(VehicleOf(v)); },
		.age = [](OpenTTDVehicle * v) noexcept -> int32_t { return RustVehicleAge(VehicleOf(v)).base(); },
		.max_age = [](OpenTTDVehicle * v) noexcept -> int32_t { return RustVehicleMaxAge(VehicleOf(v)).base(); },
		.unit = RustVehicleRead<uint16_t, RustVehicleUnitNumber>,
		.sprite = RustVehicleRead<uint8_t, RustVehicleSprite>,
		.breakdowns = [](OpenTTDVehicle * v) noexcept -> uint8_t { return RustVehicleBreakdownsSinceService(VehicleOf(v)); },
		.breakdown_counter = RustVehicleRead<uint8_t, RustVehicleBreakdownCounter>,
		.breakdown_delay = [](OpenTTDVehicle * v) noexcept -> uint8_t { return RustVehicleBreakdownDelay(VehicleOf(v)); },
		.breakdown_chance = [](OpenTTDVehicle * v) noexcept -> uint8_t { return RustVehicleBreakdownChance(VehicleOf(v)); },
		.reliability = RustVehicleRead<uint16_t, RustVehicleReliability>,
		.reliability_decay = [](OpenTTDVehicle * v) noexcept -> uint16_t { return RustVehicleReliabilityDecay(VehicleOf(v)); },
		.value = RustVehicleRead<int64_t, RustVehicleValue>,
		.profit_this = [](OpenTTDVehicle * v) noexcept -> int64_t { return VehicleOf(v)->GetDisplayProfitThisYear().base(); },
		.profit_last = RustVehicleRead<int64_t, RustVehicleDisplayProfitLastYear>,
		.vehicle_flags = [](OpenTTDVehicle * v) noexcept -> uint16_t { return VehicleOf(v)->vehicle_flags.base(); },
		.aircraft = RustVehicleRead<OpenTTDAircraftState *, RustVehicleAircraftOwner>,
		.target_airport = [](OpenTTDVehicle * v) noexcept -> const OpenTTDStation * { return RustStation(GetTargetAirportIfValid(Aircraft::From(VehicleOf(v)))); },
		.airport_ttd_type = RustStationAirportTTDType,
		.moving_flags = [](const void * airport, uint8_t pos) noexcept -> uint16_t { return RustAirportMovingFlags(static_cast<const AirportFTAClass *>(airport)->MovingData(pos)); },
		.roadtype = [](OpenTTDVehicle * v) noexcept -> RoadType { return RustRoadVehicleType(RoadVehicle::From(VehicleOf(v))); },
		.tile_roadtype = RustRoadTileType,
		.tile_railtype = RustTileRailType,
		.rail_catenary = [](RailType rt) noexcept -> bool { return GetRailTypeInfo(rt)->flags.Test(RailTypeFlag::Catenary); },
		.road_catenary = [](RoadType rt) noexcept -> bool { return GetRoadTypeInfo(rt)->flags.Test(RoadTypeFlag::Catenary); },
		.reverse_rail = RustReverseRailTypeTranslation,
		.reverse_road = RustReverseRoadTypeTranslation,
		.translate_rail = [](uint8_t type, const GRFFile * grf) noexcept -> RailType { return GetRailTypeTranslation(type, grf); },
		.translate_road = [](RoadTramType rtt, uint8_t type, const GRFFile * grf) noexcept -> RoadType { return GetRoadTypeTranslation(rtt, type, grf); },
		.compatible_rail = RustRailCompatible<RailType>,
		.powered_rail = RustRailPoweredMask,
		.powered_single_rail = RustRailPowered<RailType>,
		.badge_count = [](const GRFFile * grf) noexcept -> size_t { return grf->badge_list.size(); },
		.badge_at = [](const GRFFile * grf, uint32_t index) noexcept -> uint32_t { return grf->badge_list[index].base(); },
		.engine_has_badge = [](const OpenTTDEngine * e, uint32_t badge) noexcept -> bool { return std::ranges::find(EngineOf(e)->badges, BadgeID(badge)) != EngineOf(e)->badges.end(); },
		.engine_badge = [](const ResolverObject *ro, const OpenTTDEngine *e, uint32_t p) noexcept -> uint32_t { return RustSpecBadges(ro, EngineOf(e), p); },
		.rail_badge = [](const ResolverObject *ro, RailType rt, uint32_t p) noexcept -> uint32_t { return RustSpecBadges(ro, GetRailTypeInfo(rt), p); },
		.road_badge = [](const ResolverObject *ro, RoadType rt, uint32_t p) noexcept -> uint32_t { return RustSpecBadges(ro, GetRoadTypeInfo(rt), p); },
		.ship_owner = RustVehicleRead<OpenTTDShipState *, RustVehicleShipOwner>,
		.unhandled = [](uint8_t variable, VehicleType type) noexcept -> void { Debug(grf, 1, "Unhandled vehicle variable 0x{:X}, type 0x{:X}", variable, (uint)type); },
		.aircraft_leaves = []() noexcept { return &GetRustAircraftLeaves(); },
		.ground = []() noexcept { return &GetRustGroundPhysicsServices(); },
		.cargo_services = []() noexcept { return GetRustCargoCapacityServices().cargo; },
		.check = RustAssertCondition,
	};
	return services;
}

#endif /* RUST_VEHICLE_SERVICES_NEWGRF_H */
