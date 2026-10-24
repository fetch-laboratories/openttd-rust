/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_spritegroup_services.cpp Immutable parsed nodes and shared native resolver frames. */
#include "../stdafx.h"
#ifdef WITH_RUST
#include "../newgrf_spritegroup.h"
#include "../newgrf_profiling.h"
#include "../newgrf.h"
#include "../debug.h"
#include "newgrf_global_ffi.h"
#include "services_timer.h"
#include "settings_services.h"
#include "services_rail.h"
#include "services_industry.h"
#include "environment_services.h"
#include "services_newgrf.h"
#include "../newgrf_engine.h"
#include "generic_resolver.h"
#include "../newgrf_station.h"
#include "../newgrf_industries.h"
#include "../newgrf_industrytiles.h"
#include "../newgrf_airport.h"
#include "../newgrf_airporttiles.h"
#include "../newgrf_object.h"
#include "../newgrf_roadstop.h"
#include "../industry.h"
#include "../industry_map.h"
#include "object_ffi.h"
#include "newgrf_storage_ffi.h"
#include "../safeguards.h"

static_assert(VSG_SCOPE_SELF == 0 && VSG_SCOPE_PARENT == 1 && VSG_SCOPE_RELATIVE == 2 && VSG_END == 3);
static_assert(DSG_SIZE_BYTE == 0 && DSG_SIZE_WORD == 1 && DSG_SIZE_DWORD == 2);
static_assert(DSGA_TYPE_NONE == 0 && DSGA_TYPE_DIV == 1 && DSGA_TYPE_MOD == 2);
static_assert(DSGA_OP_ADD == 0 && DSGA_OP_SUB == 1 && DSGA_OP_SMIN == 2 && DSGA_OP_SMAX == 3 && DSGA_OP_UMIN == 4 && DSGA_OP_UMAX == 5);
static_assert(DSGA_OP_SDIV == 6 && DSGA_OP_SMOD == 7 && DSGA_OP_UDIV == 8 && DSGA_OP_UMOD == 9 && DSGA_OP_MUL == 10 && DSGA_OP_AND == 11 && DSGA_OP_OR == 12 && DSGA_OP_XOR == 13);
static_assert(DSGA_OP_STO == 14 && DSGA_OP_RST == 15 && DSGA_OP_STOP == 16 && DSGA_OP_ROR == 17 && DSGA_OP_SCMP == 18 && DSGA_OP_UCMP == 19 && DSGA_OP_SHL == 20 && DSGA_OP_SHR == 21 && DSGA_OP_SAR == 22);
static_assert(RSG_CMP_ANY == 0 && RSG_CMP_ALL == 1 && SP_CUSTOM == 3 && VEHICLEINFO_FULL_VEHICLE_WIDTH == 32);
static_assert(RAILTYPE_RAIL == 0 && RAILTYPE_ELECTRIC == 1 && RAILTYPE_MONO == 2 && RAILTYPE_MAGLEV == 3 && to_underlying(GrfMiscBit::TrainWidth32Pixels) == 3);

static ResolverResult NativeResult(const OpenTTDResolverResult &result)
{
	switch (result.kind) {
		case 1: return result.callback;
		case 2: return static_cast<const ResultSpriteGroup *>(result.group);
		case 3: return static_cast<const TileLayoutSpriteGroup *>(result.group);
		case 4: return static_cast<const IndustryProductionSpriteGroup *>(result.group);
		default: return std::monostate{};
	}
}
static OpenTTDResolverResult RustResult(const ResolverResult &result)
{
	OpenTTDResolverResult out{nullptr, 0, static_cast<uint8_t>(result.index())};
	std::visit([&](auto value) {
		using T = decltype(value);
		if constexpr (std::is_same_v<T, CallbackResult>) out.callback = value;
		else if constexpr (!std::is_same_v<T, std::monostate>) out.group = value;
	}, result);
	return out;
}
OpenTTDResolverResult RustNativeSpriteGroup(const SpriteGroup *group, ResolverObject *resolver) noexcept { return RustResult(group->Resolve(*resolver)); }
static void GroupNode(const SpriteGroup *group, OpenTTDGroupNode *out) noexcept
{
	out->data.callback = 0; // The unused union alternative is still initialized.
	out->kind = static_cast<uint8_t>(group->kind);
	switch (group->kind) {
		case SpriteGroup::Kind::Deterministic: {
			const auto *g = static_cast<const DeterministicSpriteGroup *>(group);
			out->data.deterministic = {g->adjusts.size(), g->ranges.size(), {g->default_result.group, g->default_result.calculated_result}, g->error_group, g->var_scope, g->size};
			break;
		}
		case SpriteGroup::Kind::Randomized: {
			const auto *g = static_cast<const RandomizedSpriteGroup *>(group);
			out->data.randomized = {g->groups.size(), g->var_scope, g->cmp_mode, g->triggers, g->count, g->lowest_randbit};
			break;
		}
		case SpriteGroup::Kind::Callback: out->data.callback = static_cast<const CallbackResultSpriteGroup *>(group)->result; break;
		default: break;
	}
}
static OpenTTDGroupAdjust GroupAdjust(const SpriteGroup *group, size_t index) noexcept
{
	const auto &a = static_cast<const DeterministicSpriteGroup *>(group)->adjusts[index];
	return {a.subroutine, a.and_mask, a.add_val, a.divmod_val, a.operation, a.type, a.variable, a.parameter, a.shift_num};
}
static OpenTTDGroupRange GroupRange(const SpriteGroup *group, size_t index) noexcept
{
	const auto &r = static_cast<const DeterministicSpriteGroup *>(group)->ranges[index];
	return {{r.result.group, r.result.calculated_result}, r.low, r.high};
}
static NewGRFProfiler *ProfileEnter(ResolverObject *resolver, bool top) noexcept
{
	auto p = std::ranges::find(_newgrf_profilers, resolver->grffile, &NewGRFProfiler::grffile);
	if (p == _newgrf_profilers.end() || !p->active) return nullptr;
	if (top) p->BeginResolve(*resolver); else p->RecursiveResolve();
	return &*p;
}
static void ProfileEnd(NewGRFProfiler *profiler, const OpenTTDResolverResult *result) noexcept { profiler->EndResolve(NativeResult(*result)); }
static void ResolverBinding(ResolverObject *ro, OpenTTDResolverBinding *out) noexcept
{
	*out = {.state = nullptr, .self = nullptr, .parent = nullptr, .relative = nullptr, .default_scope = RustResolverBaseScope(ro, VSG_SCOPE_SELF, 0), .kind = 0};
	if (auto *v = dynamic_cast<VehicleResolverObject *>(ro)) {
		out->state = &v->state; out->self = &v->self_scope; out->parent = &v->parent_scope; out->relative = &v->relative_scope; out->kind = 1;
	} else if (auto *st = dynamic_cast<StationResolverObject *>(ro)) {
		out->state = &st->station_scope.state; out->self = &st->station_scope; out->kind = 2;
	} else if (auto *i = dynamic_cast<IndustriesResolverObject *>(ro)) {
		out->state = &i->industries_scope.context; out->self = &i->industries_scope; out->kind = 3;
	} else if (auto *it = dynamic_cast<IndustryTileResolverObject *>(ro)) {
		out->self = &it->indtile_scope; out->parent = &it->ind_scope; out->kind = 4;
	} else if (auto *rs = dynamic_cast<RoadStopResolverObject *>(ro)) {
		out->state = &rs->roadstop_scope.state; out->self = &rs->roadstop_scope; out->kind = 6;
	} else if (auto *h = dynamic_cast<HouseResolverObject *>(ro)) {
		out->self = &h->house_scope; out->parent = &h->town_scope; out->kind = 7;
	} else if (auto *t = dynamic_cast<TownResolverObject *>(ro)) {
		out->self = &t->town_scope; out->kind = 8;
	} else if (auto *a = dynamic_cast<AirportResolverObject *>(ro)) {
		out->state = &a->airport_scope.context; out->self = &a->airport_scope; out->kind = 9;
	} else if (auto *at = dynamic_cast<AirportTileResolverObject *>(ro)) {
		out->state = &at->tiles_scope.context; out->self = &at->tiles_scope; out->parent = &at->airport_scope; out->kind = 10;
	} else if (auto *g = dynamic_cast<GenericResolverObject *>(ro)) {
		out->self = &g->generic_scope; out->kind = 11;
	} else if (auto *o = dynamic_cast<ObjectResolverObject *>(ro)) {
		out->state = &o->object_scope.context; out->self = &o->object_scope; out->kind = 12;
	} else if (typeid(*ro) != typeid(ResolverObject)) {
		out->kind = 5; // Remaining native feature scope-selection behavior.
	}
}
static void ScopeBinding(ScopeResolver *scope, OpenTTDScopeBinding *out) noexcept
{
	out->data.station = nullptr;
	if (auto *v = dynamic_cast<VehicleScopeResolver *>(scope)) { out->kind = 1; out->data.vehicle = &v->state; }
	else if (auto *st = dynamic_cast<StationScopeResolver *>(scope)) { out->kind = 2; out->data.station = &st->state; }
	else if (auto *i = dynamic_cast<IndustriesScopeResolver *>(scope)) { out->kind = 3; out->data.industry = &i->context; }
	else if (auto *it = dynamic_cast<IndustryTileScopeResolver *>(scope)) { out->kind = 4; out->data.industry = &it->context; }
	else if (auto *o = dynamic_cast<ObjectScopeResolver *>(scope)) { out->kind = 5; out->data.object = &o->context; }
	else if (auto *t = dynamic_cast<TownScopeResolver *>(scope)) { out->kind = 6; out->data.town = &t->context; }
	else if (auto *a = dynamic_cast<AirportScopeResolver *>(scope)) { out->kind = 7; out->data.airport = &a->context; }
	else if (auto *at = dynamic_cast<AirportTileScopeResolver *>(scope)) { out->kind = 11; out->data.airport = &at->context; }
	else if (auto *rs = dynamic_cast<RoadStopScopeResolver *>(scope)) { out->kind = 9; out->data.roadstop = &rs->state; }
	else if (auto *h = dynamic_cast<HouseScopeResolver *>(scope)) { out->kind = 10; out->data.house = &h->context; }
	else if (auto *g = dynamic_cast<GenericScopeResolver *>(scope)) { out->kind = 12; out->data.generic = &g->invocation; }
	else { out->kind = typeid(*scope) == typeid(ScopeResolver) ? 0 : 8; }
}
const OpenTTDSpriteGroupServices &GetRustSpriteGroupServices() noexcept
{
	static const OpenTTDSpriteGroupServices services{
		.adjust = GroupAdjust,
		.airport = &GetAirportResolverServices(),
		.global = &GetRustGlobalVariableServices(),
		.house = &GetHouseResolverServices(),
		.industry = &GetIndustryResolverServices(),
		.industry_builder = RustIndustryBuilderState,
		.loaded = RustRealLoaded,
		.loaded_count = RustRealLoadedCount,
		.loading = RustRealLoading,
		.loading_count = RustRealLoadingCount,
		.native = RustNativeSpriteGroup,
		.node = GroupNode,
		.object = &GetRustObjectServices(),
		.parameter = RustGRFParameter,
		.profile_end = ProfileEnd,
		.profile_enter = ProfileEnter,
		.random_bits = [](const ScopeResolver *s) noexcept { return s->GetRandomBits(); },
		.random_group = [](const SpriteGroup *g, size_t index) noexcept { return static_cast<const RandomizedSpriteGroup *>(g)->groups[index]; },
		.random_triggers = [](const ScopeResolver *s) noexcept { return s->GetRandomTriggers(); },
		.range = GroupRange,
		.resolver_binding = ResolverBinding,
		.roadstop = &GetRoadStopResolverServices(),
		.scope = [](ResolverObject *ro, uint8_t scope, uint8_t relative) noexcept { return ro->GetScope(static_cast<VarSpriteGroupScope>(scope), relative); },
		.scope_binding = ScopeBinding,
		.storage = &GetRustStorageServices(),
		.store_psa = [](ScopeResolver *s, uint32_t reg, int32_t value) noexcept { s->StorePSA(reg, value); },
		.tile_random_check = RustIndustryTileRandomCheck,
		.unhandled_scope = RustUnhandledScopeVariable,
		.variable = [](const ScopeResolver *s, uint8_t var, uint32_t param, bool *available) noexcept { return s->GetVariable(var, param, *available); },
		.vehicle = &GetRustVehicleResolverServices(),
	};
	return services;
}
const OpenTTDGlobalVariableServices &GetRustGlobalVariableServices() noexcept
{
	static const OpenTTDGlobalVariableServices services{
		.climate = OpenTTDLandscapeValue,
		.date_fraction = RustCalendarDateFraction,
		.date_to_ymd = RustCalendarDateToYMD,
		.disable_elrails = RustDisableElrails,
		.file_version = RustGRFFileVersion,
		.game_mode = RustGameMode,
		.height_limit = RustMapHeightLimit,
		.misc_bits = RustGRFMiscBits,
		.newgrf_version = RustNewGRFVersion,
		.palette = RustGRFPalette,
		.pitch = RustGRFTrainPitch,
		.rail_cost = RustRailCostMultiplier,
		.road_side = RustRoadSide,
		.shared = &GetRustSharedServices(),
		.width = RustGRFTrainWidth,
		.ymd_to_date = RustCalendarYMDToDate,
	};
	return services;
}
ResolverResult SpriteGroup::Resolve(const SpriteGroup *group, ResolverObject &object, bool top_level)
{
	return NativeResult(openttd_rust_spritegroup_resolve(&GetRustSpriteGroupServices(), group, &object, object.CommonState(), object.grffile, top_level));
}
ResolverResult ResolverObject::DoResolve()
{
	return NativeResult(openttd_rust_resolver_resolve(&GetRustSpriteGroupServices(), this->root_spritegroup, this, this->CommonState(), this->grffile));
}
uint32_t ScopeResolver::GetRandomBits() const { return openttd_rust_scope_default_random(); }
uint32_t ScopeResolver::GetRandomTriggers() const { return openttd_rust_scope_default_random(); }
uint32_t ScopeResolver::GetVariable(uint8_t variable, uint32_t, bool &available) const
{
	return openttd_rust_scope_default_variable(&GetRustSpriteGroupServices(), variable, &available);
}
void ScopeResolver::StorePSA(uint, int32_t) {}
const SpriteGroup *ResolverObject::ResolveReal(const RealSpriteGroup &group) const
{
	return openttd_rust_resolver_default_real(&GetRustSpriteGroupServices(), &group);
}
#include "service_table_check.hpp"
#include "abi-visit-OpenTTDSpriteGroupServices.hpp"
#include "abi-visit-OpenTTDGlobalVariableServices.hpp"
bool CheckAbi_spritegroup() { return AbiServicesComplete(GetRustSpriteGroupServices()) && AbiServicesComplete(GetRustGlobalVariableServices()); }
#endif
