/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_spritegroup_ffi.h Common Action2 stack state and immutable native node observations. */
#ifndef RUST_NEWGRF_SPRITEGROUP_FFI_H
#define RUST_NEWGRF_SPRITEGROUP_FFI_H
#include "service_defaults.hpp"
#include <array>
#include <cstddef>
#include <cstdint>
struct SpriteGroup;
struct ResolverObject;
struct ScopeResolver;
struct GRFFile;
struct NewGRFProfiler;
struct OpenTTDGlobalVariableServices;
enum CallbackID : uint16_t;
/** The sole common invocation state, embedded for the actual ResolverObject lifetime.
 * Native compatibility aliases address these fields. Calls hold the game-state
 * mutex; no Rust borrow of this carrier spans a scope/procedure/storage callback. */
struct OpenTTDResolverState {
	CallbackID callback;
	uint32_t callback_param1, callback_param2, last_value, waiting_random_triggers, used_random_triggers;
	std::array<uint32_t, 3> reseed;
};
struct OpenTTDResolverResult { const SpriteGroup *group; uint16_t callback; uint8_t kind; };
struct OpenTTDGroupResult { const SpriteGroup *group; bool calculated_result; };
struct OpenTTDDeterministicNode {
	size_t adjusts, ranges;
	OpenTTDGroupResult default_result;
	const SpriteGroup *error_group;
	uint8_t scope, size;
};
struct OpenTTDRandomizedNode { size_t groups; uint8_t scope, compare, triggers, count, lowest_bit; };
union OpenTTDGroupData { OpenTTDDeterministicNode deterministic; OpenTTDRandomizedNode randomized; uint16_t callback; };
struct OpenTTDGroupNode { OpenTTDGroupData data; uint8_t kind; };
struct OpenTTDGroupAdjust {
	const SpriteGroup *subroutine;
	uint32_t and_mask, add_val, divmod_val;
	uint8_t operation, type, variable, parameter, shift_num;
};
struct OpenTTDGroupRange { OpenTTDGroupResult result; uint32_t low, high; };
struct OpenTTDAirportResolverServices;
struct OpenTTDAirportResolverState;
struct OpenTTDVehicleResolverServices;
struct OpenTTDIndustryResolverServices;
struct OpenTTDObjectServices;
struct OpenTTDStorageServices;
struct OpenTTDRoadStopResolverServices;
struct OpenTTDHouseResolverServices;
struct OpenTTDHouseScope;
struct OpenTTDTownScope;
struct OpenTTDRoadStopResolverState;
struct OpenTTDIndustryBuilder;
struct OpenTTDVehicleScopeState;
struct OpenTTDIndustryResolverContext;
struct OpenTTDGenericInvocation;
struct Town;
struct OpenTTDObjectScope;
struct OpenTTDResolverBinding { void *state; ScopeResolver *self, *parent, *relative, *default_scope; uint8_t kind; };
union OpenTTDScopeData { const OpenTTDVehicleScopeState *vehicle; const OpenTTDIndustryResolverContext *industry; void *station; const OpenTTDObjectScope *object; const OpenTTDTownScope *town; const OpenTTDAirportResolverState *airport; const OpenTTDHouseScope *house; const OpenTTDRoadStopResolverState *roadstop; const OpenTTDGenericInvocation *generic; };
struct OpenTTDScopeBinding { OpenTTDScopeData data; uint8_t kind; };
struct OpenTTDSpriteGroupServices {
	OpenTTDGroupAdjust (*adjust)(const SpriteGroup *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(adjust);
	const OpenTTDAirportResolverServices *airport;
	const OpenTTDGlobalVariableServices *global;
	const OpenTTDHouseResolverServices *house;
	const OpenTTDIndustryResolverServices *industry;
	OpenTTDIndustryBuilder *(*industry_builder)() noexcept OPENTTD_SERVICE_DEFAULT(industry_builder);
	const SpriteGroup *(*loaded)(const struct RealSpriteGroup *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(loaded);
	size_t (*loaded_count)(const struct RealSpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(loaded_count);
	const SpriteGroup *(*loading)(const struct RealSpriteGroup *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(loading);
	size_t (*loading_count)(const struct RealSpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(loading_count);
	OpenTTDResolverResult (*native)(const SpriteGroup *, ResolverObject *) noexcept OPENTTD_SERVICE_DEFAULT(native);
	void (*node)(const SpriteGroup *, OpenTTDGroupNode *) noexcept OPENTTD_SERVICE_DEFAULT(node);
	const OpenTTDObjectServices *object;
	uint32_t (*parameter)(const GRFFile *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(parameter);
	void (*profile_end)(NewGRFProfiler *, const OpenTTDResolverResult *) noexcept OPENTTD_SERVICE_DEFAULT(profile_end);
	NewGRFProfiler *(*profile_enter)(ResolverObject *, bool) noexcept OPENTTD_SERVICE_DEFAULT(profile_enter);
	uint32_t (*random_bits)(const ScopeResolver *) noexcept OPENTTD_SERVICE_DEFAULT(random_bits);
	const SpriteGroup *(*random_group)(const SpriteGroup *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(random_group);
	uint32_t (*random_triggers)(const ScopeResolver *) noexcept OPENTTD_SERVICE_DEFAULT(random_triggers);
	OpenTTDGroupRange (*range)(const SpriteGroup *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(range);
	void (*resolver_binding)(ResolverObject *, OpenTTDResolverBinding *) noexcept OPENTTD_SERVICE_DEFAULT(resolver_binding);
	const OpenTTDRoadStopResolverServices *roadstop;
	ScopeResolver *(*scope)(ResolverObject *, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(scope);
	void (*scope_binding)(ScopeResolver *, OpenTTDScopeBinding *) noexcept OPENTTD_SERVICE_DEFAULT(scope_binding);
	const OpenTTDStorageServices *storage;
	void (*store_psa)(ScopeResolver *, uint32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(store_psa);
	void (*tile_random_check)(const OpenTTDIndustryResolverContext *) noexcept OPENTTD_SERVICE_DEFAULT(tile_random_check);
	void (*unhandled_scope)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(unhandled_scope);
	uint32_t (*variable)(const ScopeResolver *, uint8_t, uint32_t, bool *) noexcept OPENTTD_SERVICE_DEFAULT(variable);
	const OpenTTDVehicleResolverServices *vehicle;
};
const OpenTTDSpriteGroupServices &GetRustSpriteGroupServices() noexcept;
extern "C" {
const SpriteGroup *openttd_rust_resolver_default_real(const OpenTTDSpriteGroupServices *, const struct RealSpriteGroup *);
uint32_t openttd_rust_resolver_reseed_sum(const OpenTTDResolverState *);
uint32_t openttd_rust_scope_default_random();
uint32_t openttd_rust_scope_default_variable(const OpenTTDSpriteGroupServices *, uint8_t, bool *);
void openttd_rust_resolver_init(OpenTTDResolverState *, CallbackID, uint32_t, uint32_t);
OpenTTDResolverResult openttd_rust_spritegroup_resolve(const OpenTTDSpriteGroupServices *, const SpriteGroup *, ResolverObject *, OpenTTDResolverState *, const GRFFile *, bool);
OpenTTDResolverResult openttd_rust_resolver_resolve(const OpenTTDSpriteGroupServices *, const SpriteGroup *, ResolverObject *, OpenTTDResolverState *, const GRFFile *);
uint16_t openttd_rust_resolver_callback(const OpenTTDSpriteGroupServices *, const SpriteGroup *, ResolverObject *, OpenTTDResolverState *, const GRFFile *, int32_t *, size_t);
}
#endif
