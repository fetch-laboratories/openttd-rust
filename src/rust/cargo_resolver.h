/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file cargo_resolver.h Native virtual/profiler frame for the Rust cargo invocation. */
#ifndef RUST_CARGO_RESOLVER_H
#define RUST_CARGO_RESOLVER_H
#include "../newgrf_spritegroup.h"
#include "cargo_registry_ffi.h"
struct CargoResolverObject final : ResolverObject {
	const CargoSpec *cargospec;
	CargoResolverObject(const CargoSpec *cs, CallbackID callback = CBID_NO_CALLBACK, uint32_t callback_param1 = 0, uint32_t callback_param2 = 0) : ResolverObject(cs->grffile, callback, callback_param1, callback_param2), cargospec(cs) { this->root_spritegroup = cs->group; }
	GrfSpecFeature GetFeature() const override { return static_cast<GrfSpecFeature>(openttd_rust_cargo_feature()); }
	uint32_t GetDebugID() const override { return openttd_rust_cargo_debug_id(this->cargospec); }
};
#endif
