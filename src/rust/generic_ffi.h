/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file generic_ffi.h Generic callback invocation and native Action2 facade.
 * Registration/reset run only during serialized GRF loading, outside evaluation.
 * Files/graphs outlive every invocation; no registry borrow spans evaluation or
 * playback. Nested calls have independent stack state. No ordinary-play leaf
 * throws; environmental failures terminate inside noexcept wrappers. */
#ifndef RUST_GENERIC_FFI_H
#define RUST_GENERIC_FFI_H
#include "services_ffi.h"
enum class WaterClass : uint8_t;
struct GRFFile;
struct SpriteGroup;
struct ResolverObject;
struct OpenTTDResolverState;
struct OpenTTDSpriteGroupServices;
struct ScopeResolver;
struct OpenTTDGenericScope {
	uint8_t cargo, selection, source, destination, distance, event, count, station_size, feature;
	bool ai;
};
struct OpenTTDGenericInvocation {
	OpenTTDGenericScope scope;
	uint16_t callback;
	uint32_t param1;
	const GRFFile *file;
	const SpriteGroup *root;
};
struct OpenTTDGenericResult { const GRFFile *file; uint16_t result; };
struct OpenTTDGenericServices {
	const OpenTTDSharedServices *shared;
	uint8_t (*version)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(version);
	uint8_t (*cargo_map)(const GRFFile *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_map);
	uint8_t (*cargo_bitnum)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_bitnum);
	uint16_t (*industry_substitute)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(industry_substitute);
	uint16_t (*evaluate)(const OpenTTDGenericInvocation *, int32_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(evaluate);
	const OpenTTDSpriteGroupServices *common;
	bool (*enabled)() noexcept OPENTTD_SERVICE_DEFAULT(enabled);
	bool (*ambient)() noexcept OPENTTD_SERVICE_DEFAULT(ambient);
	WaterClass (*water_class)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(water_class);
	void (*sound)(const GRFFile *, uint16_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(sound);
	void (*unsupported)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(unsupported);
	void (*unhandled)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(unhandled);
};
const OpenTTDGenericServices &GetRustGenericServices() noexcept;
extern "C" {
void openttd_rust_generic_reset(const OpenTTDGenericServices *);
void openttd_rust_generic_add(uint8_t, const GRFFile *, const SpriteGroup *);
OpenTTDGenericResult openttd_rust_generic_ai(uint8_t, uint8_t, uint8_t, uint8_t, uint8_t, uint8_t, uint8_t, uint8_t, uint8_t);
uint32_t openttd_rust_generic_variable(const OpenTTDGenericInvocation *, uint8_t, bool *);
ScopeResolver *openttd_rust_generic_scope(ScopeResolver *, ScopeResolver *, uint8_t);
uint16_t openttd_rust_generic_evaluate(const OpenTTDGenericInvocation *, ResolverObject *, OpenTTDResolverState *, int32_t *, size_t);
}
#endif
