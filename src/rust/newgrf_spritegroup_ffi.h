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
struct OpenTTDSpriteGroupServices {
	void (*node)(const SpriteGroup *, OpenTTDGroupNode *) noexcept OPENTTD_SERVICE_DEFAULT(node);
	OpenTTDGroupAdjust (*adjust)(const SpriteGroup *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(adjust);
	OpenTTDGroupRange (*range)(const SpriteGroup *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(range);
	const SpriteGroup *(*random_group)(const SpriteGroup *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(random_group);
	ScopeResolver *(*scope)(ResolverObject *, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(scope);
	uint32_t (*variable)(const ScopeResolver *, uint8_t, uint32_t, bool *) noexcept OPENTTD_SERVICE_DEFAULT(variable);
	uint32_t (*random_bits)(const ScopeResolver *) noexcept OPENTTD_SERVICE_DEFAULT(random_bits);
	uint32_t (*random_triggers)(const ScopeResolver *) noexcept OPENTTD_SERVICE_DEFAULT(random_triggers);
	void (*store_psa)(ScopeResolver *, uint32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(store_psa);
	const SpriteGroup *(*real)(ResolverObject *, const SpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(real);
	OpenTTDResolverResult (*native)(const SpriteGroup *, ResolverObject *) noexcept OPENTTD_SERVICE_DEFAULT(native);
	uint32_t (*parameter)(const GRFFile *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(parameter);
	bool (*global)(uint8_t, uint32_t *, const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(global);
	NewGRFProfiler *(*profile_enter)(ResolverObject *, bool) noexcept OPENTTD_SERVICE_DEFAULT(profile_enter);
	void (*profile_end)(NewGRFProfiler *, const OpenTTDResolverResult *) noexcept OPENTTD_SERVICE_DEFAULT(profile_end);
};
const OpenTTDSpriteGroupServices &GetRustSpriteGroupServices() noexcept;
extern "C" {
void openttd_rust_resolver_init(OpenTTDResolverState *, CallbackID, uint32_t, uint32_t);
OpenTTDResolverResult openttd_rust_spritegroup_resolve(const OpenTTDSpriteGroupServices *, const SpriteGroup *, ResolverObject *, OpenTTDResolverState *, const GRFFile *, bool);
OpenTTDResolverResult openttd_rust_resolver_resolve(const OpenTTDSpriteGroupServices *, const SpriteGroup *, ResolverObject *, OpenTTDResolverState *, const GRFFile *);
uint16_t openttd_rust_resolver_callback(const OpenTTDSpriteGroupServices *, const SpriteGroup *, ResolverObject *, OpenTTDResolverState *, const GRFFile *, int32_t *, size_t);
}
#endif
