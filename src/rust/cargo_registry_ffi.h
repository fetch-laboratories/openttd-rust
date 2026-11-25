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
struct OpenTTDCargoRegistryServices {
	const OpenTTDSpriteGroupServices *common;
	const OpenTTDSharedServices *shared;

	CargoSpec *(*spec)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(spec);
	void (*copy_default_cargo)(uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(copy_default_cargo);
	void (*fill_default)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(fill_default);

	uint8_t (*bitnum)(const CargoSpec *) noexcept OPENTTD_SERVICE_DEFAULT(bitnum);
	uint16_t (*classes)(const CargoSpec *) noexcept OPENTTD_SERVICE_DEFAULT(classes);
	uint8_t (*index)(const CargoSpec *) noexcept OPENTTD_SERVICE_DEFAULT(index);
	uint32_t (*label)(const CargoSpec *) noexcept OPENTTD_SERVICE_DEFAULT(label);
	uint8_t (*town_acceptance_effect)(const CargoSpec *) noexcept OPENTTD_SERVICE_DEFAULT(town_acceptance_effect);
	uint8_t (*town_production_effect)(const CargoSpec *) noexcept OPENTTD_SERVICE_DEFAULT(town_production_effect);
	uint16_t (*town_production_multiplier)(const CargoSpec *) noexcept OPENTTD_SERVICE_DEFAULT(town_production_multiplier);

	void (*set_abbrev)(CargoSpec *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_abbrev);
	void (*set_grffile)(CargoSpec *, const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(set_grffile);
	void (*set_name)(CargoSpec *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_name);
	void (*set_name_single)(CargoSpec *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_name_single);
	void (*set_quantifier)(CargoSpec *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_quantifier);
	void (*set_town_production_effect)(CargoSpec *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_town_production_effect);
	void (*set_units_volume)(CargoSpec *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_units_volume);

	void (*set_sorted_cargo_specs)(const CargoSpec **, size_t) noexcept OPENTTD_SERVICE_DEFAULT(set_sorted_cargo_specs);
	void (*set_sorted_standard_cargo_specs)(const CargoSpec **, size_t) noexcept OPENTTD_SERVICE_DEFAULT(set_sorted_standard_cargo_specs);
	void (*set_town_production_cargoes)(uint8_t, const CargoSpec **, size_t) noexcept OPENTTD_SERVICE_DEFAULT(set_town_production_cargoes);
	void (*sort)(const CargoSpec **, size_t) noexcept OPENTTD_SERVICE_DEFAULT(sort);

	uint16_t (*callback_frame)(const CargoSpec *, uint16_t, uint32_t, uint32_t, int32_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(callback_frame);
	uint16_t (*num_sprites)(const SpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(num_sprites);
	uint32_t (*sprite)(const SpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(sprite);
	uint32_t (*sprite_frame)(const CargoSpec *) noexcept OPENTTD_SERVICE_DEFAULT(sprite_frame);
};
const OpenTTDCargoRegistryServices &GetRustCargoRegistryServices() noexcept;
extern "C" {
uint64_t *openttd_rust_cargo_mask();
uint64_t *openttd_rust_standard_cargo_mask();
uint8_t *openttd_rust_cargo_reverse();
void openttd_rust_cargo_setup(const OpenTTDCargoRegistryServices *, uint8_t);
void openttd_rust_cargo_labels_rebuild();
void openttd_rust_cargo_bitnum_changed(const CargoSpec *, uint8_t, const GRFFile *);
uint8_t openttd_rust_cargo_is_default_abi(uint8_t);
extern "C++" inline bool openttd_rust_cargo_is_default(uint8_t a0) { return openttd_rust_cargo_is_default_abi(a0) != 0; }
OpenTTDCargoLabels openttd_rust_cargo_climate_labels();
void openttd_rust_cargo_sort(const OpenTTDCargoRegistryServices *);
void openttd_rust_cargo_finalise();
uint8_t openttd_rust_cargo_translate(uint8_t, const GRFFile *, bool);
void openttd_rust_cargo_build_inverse(const GRFFile *);
void openttd_rust_cargo_fallback(const GRFFile *, uint32_t, uint32_t, uint32_t);
uint32_t openttd_rust_cargo_custom_sprite(const CargoSpec *);
uint16_t openttd_rust_cargo_callback(uint16_t, uint32_t, uint32_t, const CargoSpec *, int32_t *, size_t);
uint16_t openttd_rust_cargo_evaluate_callback(const SpriteGroup *, ResolverObject *, OpenTTDResolverState *, const GRFFile *, int32_t *, size_t);
uint32_t openttd_rust_cargo_evaluate_sprite(const SpriteGroup *, ResolverObject *, OpenTTDResolverState *, const GRFFile *);
uint8_t openttd_rust_cargo_feature();
uint32_t openttd_rust_cargo_debug_id(const CargoSpec *);
}
#endif
