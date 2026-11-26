/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file generic_ffi.h Generic callback invocation and native Action2 facade.
 * Registration/reset run only during serialized GRF loading, outside evaluation.
 * Files/graphs outlive every invocation; no registry borrow spans evaluation or
 * playback. Nested calls have independent stack state. No ordinary-play leaf
 * throws; environmental failures terminate inside noexcept wrappers. */
#ifndef RUST_GENERIC_FFI_H
#define RUST_GENERIC_FFI_H
#include "services_ffi.h"
struct GRFFile;
struct SpriteGroup;
struct ResolverObject;
struct OpenTTDResolverState;
struct OpenTTDSpriteGroupServices;
struct ScopeResolver;
struct OpenTTDGlobalVariableServices;
struct OpenTTDGenericScope {
	uint8_t cargo_type, default_selection, src_industry, dst_industry, distance, event, count, station_size, feature;
	bool ai_callback;
};
struct OpenTTDGenericInvocation {
	OpenTTDGenericScope generic_scope;
	uint16_t callback;
	uint32_t callback_param1;
	const GRFFile *grffile;
	const SpriteGroup *root_spritegroup;
};
struct OpenTTDGenericResult { const GRFFile *file; uint16_t result; };
struct OpenTTDGenericServices {
	uint16_t (*callback_frame)(const OpenTTDGenericInvocation *, int32_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(callback_frame);
	uint8_t (*cargo_bitnum)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_bitnum);
	const OpenTTDSpriteGroupServices *common;
	const OpenTTDGlobalVariableServices *global;
	void (*play_tile_sound)(const GRFFile *, uint16_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(play_tile_sound);
	const OpenTTDSharedServices *shared;
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
