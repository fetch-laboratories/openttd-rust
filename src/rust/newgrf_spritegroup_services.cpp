/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_spritegroup_services.cpp Immutable parsed nodes and shared native resolver frames. */
#include "../stdafx.h"
#ifdef WITH_RUST
#include "../newgrf_spritegroup.h"
#include "../newgrf_profiling.h"
#include "../newgrf.h"
#include "../debug.h"
#include "newgrf_global_ffi.h"
#include "timer_services.h"
#include "settings_services.h"
#include "services_rail.h"
#include "environment_services.h"
#include "services_newgrf.h"
#include "../newgrf_engine.h"
#include "../newgrf_station.h"
#include "../newgrf_industries.h"
#include "../newgrf_industrytiles.h"
#include "../newgrf_airport.h"
#include "../newgrf_object.h"
#include "../industry.h"
#include "../industry_map.h"
#include "object_ffi.h"
#include "newgrf_storage_ffi.h"
#include "../safeguards.h"

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
	*out = {.state = nullptr, .self = nullptr, .parent = nullptr, .relative = nullptr, .default_scope = &ro->default_scope, .kind = 0};
	if (auto *v = dynamic_cast<VehicleResolverObject *>(ro)) {
		out->state = &v->state; out->self = &v->self_scope; out->parent = &v->parent_scope; out->relative = &v->relative_scope; out->kind = 1;
	} else if (auto *st = dynamic_cast<StationResolverObject *>(ro)) {
		out->state = &st->station_scope.state; out->self = &st->station_scope; out->kind = 2;
	} else if (auto *i = dynamic_cast<IndustriesResolverObject *>(ro)) {
		out->state = &i->industries_scope.context; out->self = &i->industries_scope; out->kind = 3;
	} else if (auto *it = dynamic_cast<IndustryTileResolverObject *>(ro)) {
		out->self = &it->indtile_scope; out->parent = &it->ind_scope; out->kind = 4;
	} else if (typeid(*ro) != typeid(ResolverObject)) {
		out->kind = 5; // Remaining native feature scope-selection behavior.
	}
}
static void ScopeBinding(ScopeResolver *scope, OpenTTDScopeBinding *out) noexcept
{
	if (auto *v = dynamic_cast<VehicleScopeResolver *>(scope)) { out->kind = 1; out->data.vehicle = &v->state; }
	else if (auto *st = dynamic_cast<StationScopeResolver *>(scope)) { out->kind = 2; out->data.station = &st->state; }
	else if (auto *i = dynamic_cast<IndustriesScopeResolver *>(scope)) { out->kind = 3; out->data.industry = &i->context; }
	else if (auto *it = dynamic_cast<IndustryTileScopeResolver *>(scope)) { out->kind = 4; out->data.industry = &it->context; }
	else if (auto *o = dynamic_cast<ObjectScopeResolver *>(scope)) { out->kind = 5; out->data.object = {o->obj, o->spec, o->tile.base(), o->view}; }
	else if (auto *t = dynamic_cast<TownScopeResolver *>(scope)) { out->kind = 6; out->data.town = {t->t, t->readonly}; }
	else if (auto *a = dynamic_cast<AirportScopeResolver *>(scope)) { out->kind = 7; out->data.airport = a->st; }
	else { out->kind = typeid(*scope) == typeid(ScopeResolver) ? 0 : 8; }
}
const OpenTTDSpriteGroupServices &GetRustSpriteGroupServices() noexcept
{
	static const OpenTTDSpriteGroupServices services{
		.vehicle = &GetRustVehicleResolverServices(),
		.industry = &GetIndustryResolverServices(),
		.object = &GetRustObjectServices(),
		.storage = &GetRustStorageServices(),
		.resolver_binding = ResolverBinding,
		.scope_binding = ScopeBinding,
		.industry_builder = []() noexcept { return _industry_builder.owner.get(); },
		.tile_random_check = [](const OpenTTDIndustryResolverContext *c) noexcept {
			assert(c->industry != nullptr && IsValidTile(TileIndex(c->tile)));
			assert(c->id == IndustryID::Invalid().base() || IsTileType(TileIndex(c->tile), MP_INDUSTRY));
		},
		.loaded_count = RustRealLoadedCount,
		.loading_count = RustRealLoadingCount,
		.loaded = RustRealLoaded,
		.loading = RustRealLoading,
		.node = GroupNode,
		.adjust = GroupAdjust,
		.range = GroupRange,
		.random_group = [](const SpriteGroup *g, size_t index) noexcept { return static_cast<const RandomizedSpriteGroup *>(g)->groups[index]; },
		.scope = [](ResolverObject *ro, uint8_t scope, uint8_t relative) noexcept { return ro->GetScope(static_cast<VarSpriteGroupScope>(scope), relative); },
		.variable = [](const ScopeResolver *s, uint8_t var, uint32_t param, bool *available) noexcept { return s->GetVariable(var, param, *available); },
		.random_bits = [](const ScopeResolver *s) noexcept { return s->GetRandomBits(); },
		.random_triggers = [](const ScopeResolver *s) noexcept { return s->GetRandomTriggers(); },
		.store_psa = [](ScopeResolver *s, uint32_t reg, int32_t value) noexcept { s->StorePSA(reg, value); },
		.native = RustNativeSpriteGroup,
		.unhandled_scope = RustUnhandledScopeVariable,
		.parameter = RustGRFParameter,
		.global = &GetRustGlobalVariableServices(),
		.profile_enter = ProfileEnter,
		.profile_end = ProfileEnd,
	};
	return services;
}
const OpenTTDGlobalVariableServices &GetRustGlobalVariableServices() noexcept
{
	static const OpenTTDGlobalVariableServices services{
		.shared = &GetRustSharedServices(),
		.date_to_ymd = RustCalendarDateToYMD,
		.ymd_to_date = RustCalendarYMDToDate,
		.date_fraction = RustCalendarDateFraction,
		.climate = OpenTTDLandscapeValue,
		.road_side = RustRoadSide,
		.palette = RustGRFPalette,
		.pitch = RustGRFTrainPitch,
		.disable_elrails = RustDisableElrails,
		.rail_cost = RustRailCostMultiplier,
		.game_mode = RustGameMode,
		.misc_bits = RustGRFMiscBits,
		.width = RustGRFTrainWidth,
		.height_limit = RustMapHeightLimit,
		.file_version = RustGRFFileVersion,
		.newgrf_version = RustNewGRFVersion,
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
ScopeResolver *ResolverObject::GetScope(VarSpriteGroupScope, uint8_t) { return openttd_rust_scope_default(&this->default_scope); }
#include "service_table_check.hpp"
#include "abi-visit-OpenTTDSpriteGroupServices.hpp"
#include "abi-visit-OpenTTDGlobalVariableServices.hpp"
bool CheckAbi_spritegroup() { return AbiServicesComplete(GetRustSpriteGroupServices()) && AbiServicesComplete(GetRustGlobalVariableServices()); }
#endif
