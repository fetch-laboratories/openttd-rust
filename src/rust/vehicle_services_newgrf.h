/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file vehicle_services_newgrf.h Shared vehicle NewGRF observations and evaluator adapters. */
#ifndef RUST_VEHICLE_SERVICES_NEWGRF_H
#define RUST_VEHICLE_SERVICES_NEWGRF_H
#include "vehicle_services.h"
#include "engine_services.h"
#include "services_ffi.h"
#include "services_cargo.h"
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
#include "cargo_storage_ffi.h"
#include "services_station.h"

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
		.first = [](OpenTTDVehicle *v) noexcept { return RustVehicle(RustVehicleFirst(VehicleOf(v))); },
		.move = [](OpenTTDVehicle *v, int32_t count) noexcept { return RustVehicle(VehicleOf(v)->Move(count)); },
		.previous = [](OpenTTDVehicle *v) noexcept { return RustVehicle(RustVehiclePrevious(VehicleOf(v))); },
		.can_carry = Engine_can_carry,
		.cargo_classes = RustCargoClasses,
		.cargo_bitnum = [](uint8_t cargo) noexcept { return CargoSpec::Get(cargo)->bitnum; },
		.cargo_translation = RustGRFCargoMap,
		.grf_version = RustGRFVersion,
		.get_register = [](const ResolverObject *ro, uint16_t reg) noexcept { return ro->GetRegister(reg); },
		.current_company = RustCurrentCompanyID,
		.valid_company = OpenTTDCompanyIsValid,
		.livery = [](uint16_t engine, uint8_t owner, uint16_t first, OpenTTDVehicle *v) noexcept { return GetEngineLivery(EngineID(engine), CompanyID(owner), EngineID(first), VehicleOf(v), LIT_ALL); },
		.company_info = [](uint8_t owner, const Livery *livery) noexcept { return GetCompanyInfo(CompanyID(owner), livery); },
		.first_engine = [](OpenTTDVehicle *v) noexcept { return RustGroundFirstEngine(VehicleOf(v)); },
		.articulated = [](OpenTTDVehicle *v) noexcept { return RustVehicleArticulated(VehicleOf(v)); },
		.has_articulated = RustVehicleHasArticulated,
		.overrides = [](uint16_t engine) noexcept -> OpenTTDVehicleOverrides { const Engine *e = Engine::Get(EngineID(engine)); return {RustEngine(e), e->overrides.size()}; },
		.override_at = [](const OpenTTDEngine *engine, size_t i) noexcept -> OpenTTDVehicleOverride { const auto &o = EngineOf(engine)->overrides[i]; return {o.group, o.cargo, o.engines.size()}; },
		.override_engine = [](const OpenTTDEngine *engine, size_t i, size_t j) noexcept { return EngineOf(engine)->overrides[i].engines[j].base(); },
		.cached_override = [](OpenTTDVehicle *v) noexcept { return Train::From(VehicleOf(v))->tcache.cached_override; },
		.spritegroup = [](uint16_t engine, uint8_t cargo) noexcept { return Engine::Get(EngineID(engine))->grf_prop.GetSpriteGroup(cargo); },
		.random_bits = [](OpenTTDVehicle *v) noexcept { return VehicleOf(v)->random_bits; },
		.waiting = [](OpenTTDVehicle *v) noexcept { return VehicleOf(v)->waiting_random_triggers.base(); },
		.write_random = [](OpenTTDVehicle *v, uint16_t bits) noexcept { RustVehicleSetRandomBits(VehicleOf(v), bits); },
		.write_waiting = [](OpenTTDVehicle *v, uint8_t waiting) noexcept { VehicleOf(v)->waiting_random_triggers = VehicleRandomTriggers(waiting); },
		.resolve_callback = VehicleResolverCallback, .resolve_random = VehicleResolverRandom,
		.order = [](OpenTTDVehicle *v) noexcept -> OpenTTDVehicleLoadingOrder { const Order &o = VehicleOf(v)->current_order; return {to_underlying(o.GetUnloadType()), to_underlying(o.GetLoadType()), o.IsType(OT_LOADING)}; },
		.cargo_owner = [](OpenTTDVehicle *v) noexcept { return RustVehicleCargoOwner(VehicleOf(v)); },
		.cargo_capacity = RustVehicleCargoCap,
		.real_count = [](const RealSpriteGroup *group, bool motion) noexcept { return static_cast<uint32_t>(motion ? group->loaded.size() : group->loading.size()); },
		.real_at = [](const RealSpriteGroup *group, bool motion, uint32_t index) noexcept { return motion ? group->loaded[index] : group->loading[index]; },
		.vehicle_grfid = [](OpenTTDVehicle * v) noexcept -> uint32_t { return VehicleOf(v)->GetGRFID(); },
		.engine_by_id = [](uint16_t id) noexcept -> const OpenTTDEngine * { return RustEngineByID(id); },
		.default_cargo = [](const OpenTTDEngine * e) noexcept -> uint8_t { return RustEngineDefaultCargo(e); },
		.engine_flags = [](const OpenTTDEngine * e) noexcept -> uint8_t { return EngineOf(e)->flags.base(); },
		.local_id = [](const OpenTTDEngine * e) noexcept -> uint16_t { return EngineOf(e)->grf_prop.local_id; },
		.cargo_weight = [](uint8_t cargo) noexcept -> uint16_t { return CargoSpec::Get(cargo)->weight; },
		.motion = [](OpenTTDVehicle * v) noexcept -> uint32_t { return *RustVehicleMotionAddress(VehicleOf(v)); },
		.build_year = [](OpenTTDVehicle * v) noexcept -> int32_t { return VehicleOf(v)->build_year.base(); },
		.last_service = [](OpenTTDVehicle * v) noexcept -> int32_t { return VehicleOf(v)->date_of_last_service_newgrf.base(); },
		.primary = [](OpenTTDVehicle * v) noexcept -> bool { return RustVehiclePrimary(VehicleOf(v)); },
		.current_max_speed = [](OpenTTDVehicle * v) noexcept -> int32_t { return VehicleOf(v)->GetCurrentMaxSpeed(); },
		.id = [](OpenTTDVehicle * v) noexcept -> uint32_t { return RustVehicleIndexOpaque(VehicleOf(v)); },
		.old_order = [](OpenTTDVehicle * v) noexcept -> uint16_t { return VehicleOf(v)->current_order.MapOldOrder(); },
		.num_orders = [](OpenTTDVehicle * v) noexcept -> uint8_t { return VehicleOf(v)->GetNumOrders(); },
		.real_order = [](OpenTTDVehicle * v) noexcept -> uint8_t { return VehicleOf(v)->cur_real_order_index; },
		.load_ticks = [](OpenTTDVehicle * v) noexcept -> uint16_t { return RustVehicleLoadUnloadTicks(v); },
		.service_interval = [](OpenTTDVehicle * v) noexcept -> uint16_t { return VehicleOf(v)->GetServiceInterval(); },
		.acceleration = [](OpenTTDVehicle * v) noexcept -> uint8_t { return RustVehicleAcceleration(VehicleOf(v)); },
		.cargo_type = [](OpenTTDVehicle * v) noexcept -> uint8_t { return RustVehicleCargoType(v); },
		.cargo_subtype = [](OpenTTDVehicle * v) noexcept -> uint8_t { return RustVehicleCargoSubtype(VehicleOf(v)); },
		.age = [](OpenTTDVehicle * v) noexcept -> int32_t { return RustVehicleAge(VehicleOf(v)).base(); },
		.max_age = [](OpenTTDVehicle * v) noexcept -> int32_t { return RustVehicleMaxAge(VehicleOf(v)).base(); },
		.unit = [](OpenTTDVehicle * v) noexcept -> uint16_t { return RustVehicleUnitNumber(VehicleOf(v)); },
		.sprite = [](OpenTTDVehicle * v) noexcept -> uint8_t { return VehicleOf(v)->spritenum; },
		.breakdowns = [](OpenTTDVehicle * v) noexcept -> uint8_t { return RustVehicleBreakdownsSinceService(VehicleOf(v)); },
		.breakdown_counter = [](OpenTTDVehicle * v) noexcept -> uint8_t { return RustVehicleBreakdownCounter(VehicleOf(v)); },
		.breakdown_delay = [](OpenTTDVehicle * v) noexcept -> uint8_t { return RustVehicleBreakdownDelay(VehicleOf(v)); },
		.breakdown_chance = [](OpenTTDVehicle * v) noexcept -> uint8_t { return RustVehicleBreakdownChance(VehicleOf(v)); },
		.reliability = [](OpenTTDVehicle * v) noexcept -> uint16_t { return RustVehicleReliability(VehicleOf(v)); },
		.reliability_decay = [](OpenTTDVehicle * v) noexcept -> uint16_t { return RustVehicleReliabilityDecay(VehicleOf(v)); },
		.value = [](OpenTTDVehicle * v) noexcept -> int64_t { return RustVehicleValue(VehicleOf(v)); },
		.profit_this = [](OpenTTDVehicle * v) noexcept -> int64_t { return VehicleOf(v)->GetDisplayProfitThisYear().base(); },
		.profit_last = [](OpenTTDVehicle * v) noexcept -> int64_t { return VehicleOf(v)->GetDisplayProfitLastYear().base(); },
		.vehicle_flags = [](OpenTTDVehicle * v) noexcept -> uint16_t { return VehicleOf(v)->vehicle_flags.base(); },
		.aircraft = [](OpenTTDVehicle * v) noexcept -> OpenTTDAircraftState * { return GetRustAircraftVehicle(v).state; },
		.target_airport = [](OpenTTDVehicle * v) noexcept -> const OpenTTDStation * { return RustStation(GetTargetAirportIfValid(Aircraft::From(VehicleOf(v)))); },
		.airport_ttd_type = [](const OpenTTDStation * st) noexcept -> uint8_t { return StationOf(st)->airport.GetSpec()->ttd_airport_type; },
		.moving_flags = [](const void * airport, uint8_t pos) noexcept -> uint16_t { return static_cast<const AirportFTAClass *>(airport)->MovingData(pos)->flags.base(); },
		.roadtype = [](OpenTTDVehicle * v) noexcept -> RoadType { return RoadVehicle::From(VehicleOf(v))->roadtype; },
		.tile_roadtype = [](uint32_t tile, RoadTramType rtt) noexcept -> RoadType { return GetRoadType(TileIndex(tile), rtt); },
		.tile_railtype = [](uint32_t tile) noexcept -> RailType { return RustTileRailType(tile); },
		.rail_catenary = [](RailType rt) noexcept -> bool { return GetRailTypeInfo(rt)->flags.Test(RailTypeFlag::Catenary); },
		.road_catenary = [](RoadType rt) noexcept -> bool { return GetRoadTypeInfo(rt)->flags.Test(RoadTypeFlag::Catenary); },
		.reverse_rail = [](RailType rt, const GRFFile * grf) noexcept -> uint8_t { return GetReverseRailTypeTranslation(rt, grf); },
		.reverse_road = [](RoadType rt, const GRFFile * grf) noexcept -> uint8_t { return GetReverseRoadTypeTranslation(rt, grf); },
		.translate_rail = [](uint32_t type, const GRFFile * grf) noexcept -> RailType { return GetRailTypeTranslation(type, grf); },
		.translate_road = [](RoadTramType rtt, uint32_t type, const GRFFile * grf) noexcept -> RoadType { return GetRoadTypeTranslation(rtt, type, grf); },
		.compatible_rail = [](RailType a, RailType b) noexcept -> bool { return RustRailCompatible(a, b); },
		.powered_rail = [](uint64_t types, RailType rt) noexcept -> bool { return RustRailPowered(RailTypes(types), rt); },
		.powered_single_rail = [](RailType a, RailType b) noexcept -> bool { return RustRailPowered(a, b); },
		.badge_count = [](const GRFFile * grf) noexcept -> size_t { return grf->badge_list.size(); },
		.badge_at = [](const GRFFile * grf, uint32_t index) noexcept -> uint16_t { return grf->badge_list[index].base(); },
		.engine_has_badge = [](const OpenTTDEngine * e, uint16_t badge) noexcept -> bool { return std::ranges::find(EngineOf(e)->badges, BadgeID(badge)) != EngineOf(e)->badges.end(); },
		.engine_badge = [](const GRFFile * grf, const OpenTTDEngine * e, uint32_t p) noexcept -> uint32_t { return GetBadgeVariableResult(*grf, EngineOf(e)->badges, p); },
		.rail_badge = [](const GRFFile * grf, RailType rt, uint32_t p) noexcept -> uint32_t { return GetBadgeVariableResult(*grf, GetRailTypeInfo(rt)->badges, p); },
		.road_badge = [](const GRFFile * grf, RoadType rt, uint32_t p) noexcept -> uint32_t { return GetBadgeVariableResult(*grf, GetRoadTypeInfo(rt)->badges, p); },
		.ship_owner = [](OpenTTDVehicle * v) noexcept -> OpenTTDShipState * { return Ship::From(VehicleOf(v))->GetRustState(); },
		.unhandled = [](uint8_t variable, VehicleType type) noexcept -> void { Debug(grf, 1, "Unhandled vehicle variable 0x{:X}, type 0x{:X}", variable, (uint)type); },
		.aircraft_leaves = []() noexcept { return &GetRustAircraftLeaves(); },
		.ground = []() noexcept { return &GetRustGroundPhysicsServices(); },
		.cargo_services = []() noexcept { return GetRustCargoCapacityServices().cargo; },
		.check = []([[maybe_unused]] bool value) noexcept { assert(value); },
	};
	return services;
}

#endif /* RUST_VEHICLE_SERVICES_NEWGRF_H */
