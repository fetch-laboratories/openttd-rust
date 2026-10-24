/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_spritegroup_services.cpp Immutable parsed nodes and shared native resolver frames. */
#include "../stdafx.h"
#ifdef WITH_RUST
#include "../newgrf_spritegroup.h"
#include "../newgrf_profiling.h"
#include "../newgrf.h"
#include "../debug.h"
#include "services_newgrf.h"
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
const OpenTTDSpriteGroupServices &GetRustSpriteGroupServices() noexcept
{
	static const OpenTTDSpriteGroupServices services{
		.node = GroupNode,
		.adjust = GroupAdjust,
		.range = GroupRange,
		.random_group = [](const SpriteGroup *g, size_t index) noexcept { return static_cast<const RandomizedSpriteGroup *>(g)->groups[index]; },
		.scope = [](ResolverObject *ro, uint8_t scope, uint8_t relative) noexcept { return ro->GetScope(static_cast<VarSpriteGroupScope>(scope), relative); },
		.variable = [](const ScopeResolver *s, uint8_t var, uint32_t param, bool *available) noexcept { return s->GetVariable(var, param, *available); },
		.random_bits = [](const ScopeResolver *s) noexcept { return s->GetRandomBits(); },
		.random_triggers = [](const ScopeResolver *s) noexcept { return s->GetRandomTriggers(); },
		.store_psa = [](ScopeResolver *s, uint32_t reg, int32_t value) noexcept { s->StorePSA(reg, value); },
		.real = [](ResolverObject *ro, const SpriteGroup *g) noexcept { return ro->ResolveReal(*static_cast<const RealSpriteGroup *>(g)); },
		.native = RustNativeSpriteGroup,
		.parameter = [](const GRFFile *file, uint32_t param) noexcept { return file->GetParam(param); },
		.global = [](uint8_t var, uint32_t *value, const GRFFile *file) noexcept { return GetGlobalVariable(var, value, file); },
		.profile_enter = ProfileEnter,
		.profile_end = ProfileEnd,
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
uint32_t ScopeResolver::GetRandomBits() const { return 0; }
uint32_t ScopeResolver::GetRandomTriggers() const { return 0; }
uint32_t ScopeResolver::GetVariable(uint8_t variable, uint32_t, bool &available) const
{
	Debug(grf, 1, "Unhandled scope variable 0x{:X}", variable);
	available = false;
	return UINT_MAX;
}
void ScopeResolver::StorePSA(uint, int32_t) {}
const SpriteGroup *ResolverObject::ResolveReal(const RealSpriteGroup &group) const
{
	if (!group.loaded.empty()) return group.loaded[0];
	if (!group.loading.empty()) return group.loading[0];
	return nullptr;
}
ScopeResolver *ResolverObject::GetScope(VarSpriteGroupScope, uint8_t) { return &this->default_scope; }
#endif
