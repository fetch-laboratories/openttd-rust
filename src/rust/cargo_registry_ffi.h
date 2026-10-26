/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file cargo_registry_ffi.h Canonical cargo registry and synchronous invocation boundary.
 * Loading/reset is serialized outside callbacks. Native specs/files outlive calls;
 * spans alias Rust buffers until the next rebuild. No borrow spans Action2 reentry.
 * Parser and save/load exception frames stay native; noexcept leaves terminate on
 * environmental failures. Rust panics abort. */
#ifndef RUST_CARGO_REGISTRY_FFI_H
#define RUST_CARGO_REGISTRY_FFI_H
#include "services_ffi.h"
#include "cargo_registry_lookup.h"
struct CargoSpec;
struct GRFFile;
struct SpriteGroup;
struct ResolverObject;
struct OpenTTDResolverState;
struct OpenTTDSpriteGroupServices;
struct OpenTTDCargoSprite { uint32_t sprite; uint16_t count; };
struct OpenTTDCargoProduction { uint8_t cargo; uint16_t multiplier; };
struct OpenTTDCargoLabelSpec { uint32_t label; uint8_t bitnum; };
struct OpenTTDCargoOrderSpec { uint8_t cargo, effect; bool special; };
struct OpenTTDCargoLabels { const uint32_t *data; size_t size; };
struct OpenTTDCargoFile { OpenTTDCargoLabels labels; uint8_t version; bool fallback; };
struct OpenTTDCargoInvocation {
	uint32_t label;
	const GRFFile *file;
	const SpriteGroup *root;
	uint16_t callback;
	uint32_t param1, param2;
};
struct OpenTTDCargoRegistryServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDSpriteGroupServices *common;
	const CargoSpec *(*spec)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(spec);
	void (*copy_default)(uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(copy_default);
	void (*clear_specs)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(clear_specs);
	OpenTTDCargoLabelSpec (*label_spec)(const CargoSpec *) noexcept OPENTTD_SERVICE_DEFAULT(label_spec);
	OpenTTDCargoOrderSpec (*order_spec)(const CargoSpec *) noexcept OPENTTD_SERVICE_DEFAULT(order_spec);
	uint8_t (*index)(const CargoSpec *) noexcept OPENTTD_SERVICE_DEFAULT(index);
	OpenTTDCargoProduction (*production)(const CargoSpec *) noexcept OPENTTD_SERVICE_DEFAULT(production);
	void (*sort)(const CargoSpec **, size_t) noexcept OPENTTD_SERVICE_DEFAULT(sort);
	void (*publish_sorted)(const CargoSpec **, size_t, size_t) noexcept OPENTTD_SERVICE_DEFAULT(publish_sorted);
	void (*publish_town)(uint8_t, const CargoSpec **, size_t) noexcept OPENTTD_SERVICE_DEFAULT(publish_town);
	void (*set_effect)(const CargoSpec *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_effect);
	void (*invalidate_strings)(const CargoSpec *) noexcept OPENTTD_SERVICE_DEFAULT(invalidate_strings);
	OpenTTDCargoFile (*file)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(file);
	void (*assign_fallback)(GRFFile *, OpenTTDCargoLabels) noexcept OPENTTD_SERVICE_DEFAULT(assign_fallback);
	void (*resize_fallback)(GRFFile *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(resize_fallback);
	void (*set_fallback)(GRFFile *, uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_fallback);
	void (*clear_inverse)(GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(clear_inverse);
	void (*set_inverse)(GRFFile *, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_inverse);
	void (*invocation)(const CargoSpec *, OpenTTDCargoInvocation *) noexcept OPENTTD_SERVICE_DEFAULT(invocation);
	uint16_t (*callback_frame)(const OpenTTDCargoInvocation *, int32_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(callback_frame);
	uint32_t (*sprite_frame)(const OpenTTDCargoInvocation *) noexcept OPENTTD_SERVICE_DEFAULT(sprite_frame);
	OpenTTDCargoSprite (*sprite)(const SpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(sprite);
};
const OpenTTDCargoRegistryServices &GetRustCargoRegistryServices() noexcept;
extern "C" {
uint64_t *openttd_rust_cargo_mask();
uint64_t *openttd_rust_standard_cargo_mask();
uint8_t *openttd_rust_cargo_reverse();
void openttd_rust_cargo_setup(const OpenTTDCargoRegistryServices *, uint8_t);
void openttd_rust_cargo_labels_rebuild();
bool openttd_rust_cargo_is_default(uint8_t);
OpenTTDCargoLabels openttd_rust_cargo_climate_labels();
OpenTTDCargoLabels openttd_rust_cargo_bitnum_labels();
void openttd_rust_cargo_sort(const OpenTTDCargoRegistryServices *);
void openttd_rust_cargo_finalise();
OpenTTDCargoLabels openttd_rust_cargo_translation_table(const GRFFile *);
uint8_t openttd_rust_cargo_translate(uint8_t, const GRFFile *, bool);
void openttd_rust_cargo_build_inverse(GRFFile *);
void openttd_rust_cargo_fallback(GRFFile *, uint32_t, uint32_t, uint32_t);
uint32_t openttd_rust_cargo_custom_sprite(const CargoSpec *);
uint16_t openttd_rust_cargo_callback(uint16_t, uint32_t, uint32_t, const CargoSpec *, int32_t *, size_t);
uint16_t openttd_rust_cargo_evaluate_callback(const OpenTTDCargoInvocation *, ResolverObject *, OpenTTDResolverState *, int32_t *, size_t);
uint32_t openttd_rust_cargo_evaluate_sprite(const OpenTTDCargoInvocation *, ResolverObject *, OpenTTDResolverState *);
uint8_t openttd_rust_cargo_feature();
uint32_t openttd_rust_cargo_debug_id(const OpenTTDCargoInvocation *);
}
#endif
