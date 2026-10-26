/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file cargo_resolver.h Native virtual/profiler frame for the Rust cargo invocation. */
#ifndef RUST_CARGO_RESOLVER_H
#define RUST_CARGO_RESOLVER_H
#include "../newgrf_spritegroup.h"
#include "cargo_registry_ffi.h"
struct CargoResolverObject final : ResolverObject {
	const OpenTTDCargoInvocation &invocation;
	CargoResolverObject(const OpenTTDCargoInvocation &invocation) : ResolverObject(invocation.file, static_cast<CallbackID>(invocation.callback), invocation.param1, invocation.param2), invocation(invocation) { this->root_spritegroup = invocation.root; }
	GrfSpecFeature GetFeature() const override { return static_cast<GrfSpecFeature>(openttd_rust_cargo_feature()); }
	uint32_t GetDebugID() const override { return openttd_rust_cargo_debug_id(&this->invocation); }
};
#endif
