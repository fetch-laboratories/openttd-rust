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
struct OpenTTDEngineRefitInputs { CargoTypes mask; CargoType cargo; bool custom; };
struct OpenTTDEngineTrainCargo { uint8_t capacity; RailVehicleTypes kind; };
struct OpenTTDEngineCargoClass { uint16_t classes; CargoType cargo; };
struct OpenTTDEngineMappingView { const EngineIDMapping *data; size_t size; };
/** Native leaves do not throw. Load/reset excludes resolution; native error frames
 * retain ByteReader and EIDS unwinding. All IDs returned across the ABI are scalars. */
struct OpenTTDEngineLoadingServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDVehicleResolverServices *resolver;
	OpenTTDEngineRef (*next)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next);
	OpenTTDEngineRef (*resolve)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(resolve);
	bool (*dynamic)() noexcept OPENTTD_SERVICE_DEFAULT(dynamic);
	uint32_t (*file_id)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(file_id);
	GRFFile *(*find_file)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(find_file);
	const GRFFile *(*engine_file)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_file);
	bool (*has_file)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(has_file);
	void (*set_file)(const OpenTTDEngine *, const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(set_file);
	void (*mapping_log)(uint32_t, uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(mapping_log);
	void (*replaced_log)(uint16_t, uint32_t, VehicleType, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(replaced_log);
	void (*created_log)(uint16_t, uint32_t, VehicleType, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(created_log);
	void (*full_log)() noexcept OPENTTD_SERVICE_DEFAULT(full_log);
	bool (*can_allocate)() noexcept OPENTTD_SERVICE_DEFAULT(can_allocate);
	size_t (*pool_size)() noexcept OPENTTD_SERVICE_DEFAULT(pool_size);
	OpenTTDEngineRef (*construct)(VehicleType, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(construct);
	void (*construct_at)(VehicleType, uint16_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(construct_at);
	void (*close_previews)() noexcept OPENTTD_SERVICE_DEFAULT(close_previews);
	void (*clean_pool)() noexcept OPENTTD_SERVICE_DEFAULT(clean_pool);
	uint64_t (*railtypes)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(railtypes);
	uint32_t (*rail_label)(RailType) noexcept OPENTTD_SERVICE_DEFAULT(rail_label);
	OpenTTDEngineRefitInputs (*refit_inputs)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(refit_inputs);
	OpenTTDNewGRFCargoLabel (*cargo_label)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_label);
	OpenTTDEngineTrainCargo (*train_cargo)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(train_cargo);
	void (*set_cargo)(const OpenTTDEngine *, CargoType) noexcept OPENTTD_SERVICE_DEFAULT(set_cargo);
	void (*set_refit)(const OpenTTDEngine *, CargoTypes) noexcept OPENTTD_SERVICE_DEFAULT(set_refit);
	void (*disable)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(disable);
	bool (*old_refittable)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(old_refittable);
	void (*set_old_refittable)(const OpenTTDEngine *, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_old_refittable);
	OpenTTDEngineCargoClass (*cargo_next)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_next);
	uint8_t (*cargo_map)(const GRFFile *, CargoType) noexcept OPENTTD_SERVICE_DEFAULT(cargo_map);
	uint8_t (*file_version)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(file_version);
	size_t (*cargo_list_size)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_list_size);
	void (*unknown_refit)(uint32_t, uint16_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(unknown_refit);
	uint8_t (*climate)() noexcept OPENTTD_SERVICE_DEFAULT(climate);
	uint16_t (*variant)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(variant);
	void (*set_variant)(const OpenTTDEngine *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_variant);
	void (*invalid_string)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(invalid_string);
	bool (*engine_climate)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_climate);
	void (*rail_badges)(const OpenTTDEngine *, RailType) noexcept OPENTTD_SERVICE_DEFAULT(rail_badges);
	void (*road_badges)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(road_badges);
	RailVehicleTypes (*rail_kind)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(rail_kind);
	LiveryScheme (*livery)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(livery);
	void (*use_livery)(LiveryScheme) noexcept OPENTTD_SERVICE_DEFAULT(use_livery);
	void (*variant_flags)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(variant_flags);
	void (*variant_cycle_log)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(variant_cycle_log);
	bool (*is_tram)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(is_tram);
	void (*set_road_speed)(const OpenTTDEngine *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_road_speed);
	void (*set_roadtype)(const OpenTTDEngine *, RoadType) noexcept OPENTTD_SERVICE_DEFAULT(set_roadtype);
	size_t (*road_list_size)(const GRFFile *, bool) noexcept OPENTTD_SERVICE_DEFAULT(road_list_size);
	RoadTypeLabel (*road_label)(const GRFFile *, bool, size_t) noexcept OPENTTD_SERVICE_DEFAULT(road_label);
	RoadType (*road_by_label)(RoadTypeLabel) noexcept OPENTTD_SERVICE_DEFAULT(road_by_label);
	RailType (*rail_by_label)(RailTypeLabel) noexcept OPENTTD_SERVICE_DEFAULT(rail_by_label);
	void (*set_railtypes)(const OpenTTDEngine *, uint64_t) noexcept OPENTTD_SERVICE_DEFAULT(set_railtypes);
	void (*override_log)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(override_log);
	OpenTTDEngineLoadingVehicle (*vehicle_next)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_next);
	void (*reload)() noexcept OPENTTD_SERVICE_DEFAULT(reload);
	const uint8_t *(*default_counts)() noexcept OPENTTD_SERVICE_DEFAULT(default_counts);
};
const OpenTTDEngineLoadingServices &GetRustEngineLoadingServices() noexcept;
extern "C" {
bool openttd_rust_engine_reset_config(const OpenTTDEngineLoadingServices *);
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
uint16_t openttd_rust_engine_mapping_use(uint8_t, uint16_t, uint32_t, bool);
void openttd_rust_engine_mapping_set(uint8_t, uint16_t, uint32_t, uint8_t, uint16_t);
bool openttd_rust_engine_override_get(uint32_t, uint32_t *);
}
#endif
