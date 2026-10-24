/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file generic_resolver.h Native generic virtual-frame identity for typed common bindings. */
#ifndef RUST_GENERIC_RESOLVER_H
#define RUST_GENERIC_RESOLVER_H
#include "../newgrf_spritegroup.h"
#include "generic_ffi.h"

/** Native virtual frame; the invocation reference addresses the sole Rust stack owner. */
struct GenericScopeResolver final : ScopeResolver {
	const OpenTTDGenericInvocation &invocation;
	GenericScopeResolver(ResolverObject &ro, const OpenTTDGenericInvocation &invocation) : ScopeResolver(ro), invocation(invocation) {}
	uint32_t GetVariable(uint8_t variable, uint32_t, bool &available) const override { return openttd_rust_generic_variable(&this->invocation, variable, &available); }
};
struct GenericResolverObject final : ResolverObject {
	GenericScopeResolver generic_scope;
	GenericResolverObject(const OpenTTDGenericInvocation &invocation) : ResolverObject(invocation.file, static_cast<CallbackID>(invocation.callback), invocation.param1), generic_scope(*this, invocation) { this->root_spritegroup = invocation.root; }
	ScopeResolver *GetScope(VarSpriteGroupScope scope, uint8_t) override { return openttd_rust_generic_scope(&this->generic_scope, &this->default_scope, scope); }
	GrfSpecFeature GetFeature() const override { return static_cast<GrfSpecFeature>(this->generic_scope.invocation.scope.feature); }
	uint32_t GetDebugID() const override { return 0; }
};
#endif
