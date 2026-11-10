/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file engine_loading_ffi.h Canonical engine loading state and synchronous native leaves. */
#ifndef RUST_ENGINE_LOADING_FFI_H
#define RUST_ENGINE_LOADING_FFI_H
#include "../engine_base.h"
#include "engine_ffi.h"
#include "../livery.h"
#include "../road_type.h"
#include "newgrf_vehicle_ffi.h"
#include "newgrf_global_ffi.h"
struct OpenTTDEngineLoadingVehicle { uint32_t id; VehicleType type; };
struct OpenTTDEngineCargoClass { uint16_t classes; CargoType cargo; };
struct OpenTTDEngineMappingView { const EngineIDMapping *data; size_t size; };
/** Native leaves do not throw. Load/reset excludes resolution; native error frames
 * retain ByteReader and EIDS unwinding. All IDs returned across the ABI are scalars. */
struct OpenTTDCompanyIdentityServices;
struct OpenTTDTypeAvailabilityServices;
struct OpenTTDEngineLoadingServices {
	const OpenTTDCompanyIdentityServices *identity;
	const OpenTTDSharedServices *shared;
	const OpenTTDVehicleResolverServices *resolver;
	const OpenTTDTypeAvailabilityServices *types;
	OpenTTDEngineRef (*resolve)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(resolve);
	bool (*dynamic)() noexcept OPENTTD_SERVICE_DEFAULT(dynamic);
	GRFFile *(*find_file)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(find_file);
	void (*mapping_log)(uint32_t, uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(mapping_log);
	bool (*can_allocate)() noexcept OPENTTD_SERVICE_DEFAULT(can_allocate);
	size_t (*pool_size)() noexcept OPENTTD_SERVICE_DEFAULT(pool_size);
	OpenTTDEngineRef (*construct)(VehicleType, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(construct);
	void (*construct_at)(VehicleType, uint16_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(construct_at);
	void (*close_previews)() noexcept OPENTTD_SERVICE_DEFAULT(close_previews);
	void (*clean_pool)() noexcept OPENTTD_SERVICE_DEFAULT(clean_pool);
	OpenTTDEngineCargoClass (*cargo_next)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_next);
	size_t (*cargo_list_size)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_list_size);
	void (*unknown_refit)(uint32_t, uint16_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(unknown_refit);
	void (*use_livery)(LiveryScheme) noexcept OPENTTD_SERVICE_DEFAULT(use_livery);
	void (*variant_flags)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(variant_flags);
	void (*override_log)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(override_log);
	OpenTTDEngineLoadingVehicle (*vehicle_next)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_next);
	void (*reload)() noexcept OPENTTD_SERVICE_DEFAULT(reload);
	const uint8_t *(*default_counts)() noexcept OPENTTD_SERVICE_DEFAULT(default_counts);
	void (*order_sort)(uint16_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(order_sort);
	size_t (*order_count)() noexcept OPENTTD_SERVICE_DEFAULT(order_count);
};
const OpenTTDEngineLoadingServices &GetRustEngineLoadingServices() noexcept;
void RustVehicleOrderSort(uint16_t *, size_t) noexcept;
extern "C" {
uint32_t openttd_rust_engine_file_badge(const OpenTTDEngineLoadingServices *, const GRFFile *, BadgeID);
uint8_t openttd_rust_engine_reset_config_abi(const OpenTTDEngineLoadingServices *);
extern "C++" inline bool openttd_rust_engine_reset_config(const OpenTTDEngineLoadingServices *a0) { return openttd_rust_engine_reset_config_abi(a0) != 0; }
const GRFFile *openttd_rust_engine_current_override(const OpenTTDEngineLoadingServices *, const GRFFile *);
void openttd_rust_engine_set_override(const OpenTTDEngineLoadingServices *, uint32_t, uint32_t);
uint64_t openttd_rust_engine_translate_refit(uint32_t, const GRFFile *);
void openttd_rust_engine_setup(const OpenTTDEngineLoadingServices *);
const OpenTTDEngine *openttd_rust_engine_definition(const OpenTTDEngineLoadingServices *, const GRFFile *, VehicleType, uint16_t, bool);
uint16_t openttd_rust_engine_new_id(const OpenTTDEngineLoadingServices *, const GRFFile *, VehicleType, uint16_t);
OpenTTDEngineMappingView openttd_rust_engine_mappings(uint8_t);
void openttd_rust_engine_mapping_clear();
void openttd_rust_engine_mapping_defaults(const uint8_t *);
uint16_t openttd_rust_engine_mapping_get(uint8_t, uint16_t, uint32_t);
void openttd_rust_engine_mapping_set(uint8_t, uint16_t, uint32_t, uint8_t, uint16_t);
uint8_t openttd_rust_engine_override_get_abi(uint32_t, uint32_t *);
extern "C++" inline bool openttd_rust_engine_override_get(uint32_t a0, uint32_t *a1) { return openttd_rust_engine_override_get_abi(a0, a1) != 0; }
}
#endif
