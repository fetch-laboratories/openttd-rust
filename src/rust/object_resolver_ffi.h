/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file object_resolver_ffi.h Rust-owned stack invocation and eligibility leaves.
 * Game-state serialization pins object/spec/town shells for synchronous Action2
 * and storage reentry: no reset, load or yield occurs. No context borrow crosses
 * evaluation or a native leaf. Leaves are noexcept; panics/environmental failures
 * terminate. Generation cancellation and save/load frames remain native. */
#ifndef RUST_OBJECT_RESOLVER_FFI_H
#define RUST_OBJECT_RESOLVER_FFI_H
#include "service_defaults.hpp"
#include <cstdint>
#include <cstddef>
struct Object;
struct OpenTTDSharedServices;
struct ObjectSpec;
struct Town;
struct GRFFile;
struct SpriteGroup;
struct ScopeResolver;
struct TownScopeResolver;
struct ResolverObject;
struct ObjectResolverObject;
struct OpenTTDObjectServices;
struct OpenTTDObjectScope {
	Object *object;
	const ObjectSpec *spec;
	const Town *parent;
	uint32_t tile;
	uint8_t view;
};
struct OpenTTDObjectEligibility { uint16_t flags; uint8_t climate, views; };
struct OpenTTDObjectLifetime { int32_t introduction, end; };
struct OpenTTDObjectResolverServices {
	const OpenTTDSharedServices *shared;
	uint8_t (*views)(const ObjectSpec *) noexcept OPENTTD_SERVICE_DEFAULT(views);
	OpenTTDObjectEligibility (*eligibility)(const ObjectSpec *) noexcept OPENTTD_SERVICE_DEFAULT(eligibility);
	int32_t (*introduction)(const ObjectSpec *) noexcept OPENTTD_SERVICE_DEFAULT(introduction);
	OpenTTDObjectLifetime (*lifetime)(const ObjectSpec *) noexcept OPENTTD_SERVICE_DEFAULT(lifetime);
	TownScopeResolver *(*town_scope)(ObjectResolverObject *, const Town *, bool) noexcept OPENTTD_SERVICE_DEFAULT(town_scope);
	ScopeResolver *(*base_scope)(ResolverObject *, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(base_scope);
	uint16_t (*evaluate)(const OpenTTDObjectScope *, uint16_t, uint32_t, uint32_t, int32_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(evaluate);
};
const OpenTTDObjectServices &GetRustObjectServices() noexcept;
const OpenTTDObjectResolverServices &GetObjectResolverServices() noexcept;
extern "C" {
const OpenTTDObjectServices *openttd_rust_object_services_exchange(const OpenTTDObjectServices *);
OpenTTDObjectScope openttd_rust_object_scope_init(Object *, const ObjectSpec *, uint32_t, uint8_t);
const GRFFile *openttd_rust_object_file(const ObjectSpec *);
const SpriteGroup *openttd_rust_object_root(const OpenTTDObjectScope *);
uint32_t openttd_rust_object_debug_id(const OpenTTDObjectScope *);
uint8_t openttd_rust_object_feature();
ScopeResolver *openttd_rust_object_scope(OpenTTDObjectScope *, ObjectResolverObject *, ScopeResolver *, uint8_t, uint8_t);
uint16_t openttd_rust_object_callback(const OpenTTDObjectServices *, uint16_t, uint32_t, uint32_t, const ObjectSpec *, Object *, uint32_t, int32_t *, size_t, uint8_t);
uint16_t openttd_rust_object_animation_callback(uint16_t, uint32_t, uint32_t, const ObjectSpec *, Object *, uint32_t, int32_t) noexcept;
bool openttd_rust_object_enabled(const OpenTTDObjectServices *, const ObjectSpec *);
bool openttd_rust_object_ever_available(const OpenTTDObjectServices *, const ObjectSpec *);
bool openttd_rust_object_was_available(const OpenTTDObjectServices *, const ObjectSpec *);
bool openttd_rust_object_available(const OpenTTDObjectServices *, const ObjectSpec *);
}
#endif
