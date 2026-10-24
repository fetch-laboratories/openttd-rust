/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file vehicle_services_newgrf.h Shared vehicle NewGRF observations and evaluator adapters. */
#ifndef RUST_VEHICLE_SERVICES_NEWGRF_H
#define RUST_VEHICLE_SERVICES_NEWGRF_H
#include "vehicle_services.h"
#include "engine_services.h"
#include "services_ffi.h"
#include "../newgrf_engine.h"
#include "../newgrf.h"
#include "../company_base.h"

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
		.user_data = [](OpenTTDVehicle *v) noexcept { return Train::From(VehicleOf(v))->GetUserDefData(); },
		.previous = [](OpenTTDVehicle *v) noexcept { return RustVehicle(RustVehiclePrevious(VehicleOf(v))); },
		.cargo = [](OpenTTDVehicle *v) noexcept -> OpenTTDVehicleCargoQuery { return {RustVehicleCargoType(v), RustVehicleCargoSubtype(VehicleOf(v)), RustVehicleGetEngine(v)}; },
		.can_carry = Engine_can_carry,
		.cargo_classes = [](uint8_t cargo) noexcept { return CargoSpec::Get(cargo)->classes.base(); },
		.cargo_bitnum = [](uint8_t cargo) noexcept { return CargoSpec::Get(cargo)->bitnum; },
		.cargo_translation = [](const GRFFile *grf, uint8_t cargo) noexcept { return grf->cargo_map[cargo]; },
		.grf_version = [](const GRFFile *grf) noexcept { return grf->grf_version; },
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
		.random_fields = [](OpenTTDVehicle *v) noexcept -> OpenTTDVehicleRandomFields { return {VehicleOf(v)->random_bits, VehicleOf(v)->waiting_random_triggers.base()}; },
		.write_random = [](OpenTTDVehicle *v, uint16_t bits) noexcept { RustVehicleSetRandomBits(VehicleOf(v), bits); },
		.write_waiting = [](OpenTTDVehicle *v, uint8_t waiting) noexcept { VehicleOf(v)->waiting_random_triggers = VehicleRandomTriggers(waiting); },
		.resolve_callback = VehicleResolverCallback, .resolve_random = VehicleResolverRandom,
		.in_motion = [](OpenTTDVehicle *v) noexcept { const Order &o = RustVehicleFirst(VehicleOf(v))->current_order; return !o.IsType(OT_LOADING) || (o.GetUnloadType() == OrderUnloadType::NoUnload && o.GetLoadType() == OrderLoadType::NoLoad); },
		.cargo_owner = [](OpenTTDVehicle *v) noexcept { return RustVehicleCargoOwner(VehicleOf(v)); },
		.cargo_capacity = RustVehicleCargoCap,
		.real_count = [](const RealSpriteGroup *group, bool motion) noexcept { return static_cast<uint32_t>(motion ? group->loaded.size() : group->loading.size()); },
		.real_at = [](const RealSpriteGroup *group, bool motion, uint32_t index) noexcept { return motion ? group->loaded[index] : group->loading[index]; },
		.check = [](bool value) noexcept { assert(value); },
	};
	return services;
}

extern "C" const OpenTTDVehicleResolverServices *openttd_vehicle_resolver_services() noexcept
{
	return &GetRustVehicleResolverServices();
}
#endif /* RUST_VEHICLE_SERVICES_NEWGRF_H */
