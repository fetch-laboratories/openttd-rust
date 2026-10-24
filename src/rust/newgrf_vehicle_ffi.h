/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_vehicle_ffi.h Stack vehicle resolver and canonical query cache boundary. */
#ifndef RUST_NEWGRF_VEHICLE_FFI_H
#define RUST_NEWGRF_VEHICLE_FFI_H
#include "vehicle_ffi.h"
struct Vehicle;
struct NewGRFCache;
struct ResolverObject;
struct SpriteGroup;
struct RealSpriteGroup;
struct GRFFile;
struct Livery;
struct OpenTTDSharedServices;
/** Invocation-local state; no owner/reference survives common Action2 evaluation. */
struct OpenTTDVehicleScopeState { const Vehicle *v; uint16_t self_type; bool rotor_in_gui; };
struct OpenTTDVehicleResolverState {
	OpenTTDVehicleScopeState self_, parent, relative;
	uint8_t cached_relative_count;
	const SpriteGroup *root;
};
struct OpenTTDVehicleRandomFields { uint16_t bits; uint8_t waiting; };
struct OpenTTDVehicleRandomResult { uint32_t used, reseed; };
struct OpenTTDVehicleCargoQuery { uint8_t cargo, subtype; const OpenTTDEngine *engine; };
struct OpenTTDVehicleOverride { const SpriteGroup *group; uint8_t cargo; size_t count; };
struct OpenTTDVehicleOverrides { const OpenTTDEngine *engine; size_t count; };
struct OpenTTDVehicleResolverServices {
	const OpenTTDVehicleServices *vehicle;
	const OpenTTDSharedServices *shared;
	NewGRFCache *(*cache)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cache);
	OpenTTDVehicle *(*first)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(first);
	OpenTTDVehicle *(*move)(OpenTTDVehicle *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(move);
	uint8_t (*user_data)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(user_data);
	OpenTTDVehicle *(*previous)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(previous);
	OpenTTDVehicleCargoQuery (*cargo)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cargo);
	bool (*can_carry)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(can_carry);
	uint16_t (*cargo_classes)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_classes);
	uint8_t (*cargo_bitnum)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_bitnum);
	uint8_t (*cargo_translation)(const GRFFile *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_translation);
	uint8_t (*grf_version)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(grf_version);
	int32_t (*get_register)(const ResolverObject *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(get_register);
	uint8_t (*current_company)() noexcept OPENTTD_SERVICE_DEFAULT(current_company);
	bool (*valid_company)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(valid_company);
	const Livery *(*livery)(uint16_t, uint8_t, uint16_t, OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(livery);
	uint32_t (*company_info)(uint8_t, const Livery *) noexcept OPENTTD_SERVICE_DEFAULT(company_info);
	uint16_t (*first_engine)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(first_engine);
	bool (*articulated)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(articulated);
	bool (*has_articulated)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(has_articulated);
	OpenTTDVehicleOverrides (*overrides)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(overrides);
	OpenTTDVehicleOverride (*override_at)(const OpenTTDEngine *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(override_at);
	uint16_t (*override_engine)(const OpenTTDEngine *, size_t, size_t) noexcept OPENTTD_SERVICE_DEFAULT(override_engine);
	const SpriteGroup *(*cached_override)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cached_override);
	const SpriteGroup *(*spritegroup)(uint16_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(spritegroup);
	OpenTTDVehicleRandomFields (*random_fields)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(random_fields);
	void (*write_random)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(write_random);
	void (*write_waiting)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_waiting);
	uint16_t (*resolve_callback)(OpenTTDVehicleResolverState *, uint16_t, uint16_t, uint32_t, uint32_t, int32_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(resolve_callback);
	OpenTTDVehicleRandomResult (*resolve_random)(OpenTTDVehicleResolverState *, uint16_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(resolve_random);
	bool (*in_motion)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(in_motion);
	OpenTTDCargoList *(*cargo_owner)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_owner);
	uint16_t (*cargo_capacity)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_capacity);
	uint32_t (*real_count)(const RealSpriteGroup *, bool) noexcept OPENTTD_SERVICE_DEFAULT(real_count);
	const SpriteGroup *(*real_at)(const RealSpriteGroup *, bool, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(real_at);
	void (*check)(bool) noexcept OPENTTD_SERVICE_DEFAULT(check);
};
const OpenTTDVehicleResolverServices &GetRustVehicleResolverServices() noexcept;
extern "C" {
NewGRFCache *openttd_rust_vehicle_grf_cache_new();
void openttd_rust_vehicle_grf_cache_delete(NewGRFCache *);
void openttd_rust_vehicle_grf_invalidate(const OpenTTDVehicleResolverServices *, const Vehicle *, bool);
void openttd_rust_vehicle_grf_fill(const OpenTTDVehicleResolverServices *, const Vehicle *);
void openttd_rust_vehicle_resolver_init(const OpenTTDVehicleResolverServices *, OpenTTDVehicleResolverState *, uint16_t, const Vehicle *, uint8_t, bool);
uint8_t openttd_rust_vehicle_resolver_scope(const OpenTTDVehicleResolverServices *, OpenTTDVehicleResolverState *, const ResolverObject *, uint8_t, uint8_t);
void openttd_rust_vehicle_scope_set(OpenTTDVehicleScopeState *, const Vehicle *);
uint32_t openttd_rust_vehicle_scope_random(const OpenTTDVehicleResolverServices *, const OpenTTDVehicleScopeState *, bool);
uint32_t openttd_rust_vehicle_cached_variable(const OpenTTDVehicleResolverServices *, const Vehicle *, uint8_t, const GRFFile *);
const SpriteGroup *openttd_rust_vehicle_resolve_real(const OpenTTDVehicleResolverServices *, const OpenTTDVehicleResolverState *, const RealSpriteGroup *);
const SpriteGroup *openttd_rust_vehicle_wagon_override(const OpenTTDVehicleResolverServices *, uint16_t, uint8_t, uint16_t);
uint16_t openttd_rust_vehicle_callback(const OpenTTDVehicleResolverServices *, uint16_t, uint32_t, uint32_t, uint16_t, const Vehicle *, const Vehicle *, bool, int32_t *, size_t);
int32_t openttd_rust_vehicle_property(const OpenTTDVehicleResolverServices *, uint16_t, uint8_t, int32_t, const Vehicle *, bool);
int8_t openttd_rust_vehicle_build_probability(const OpenTTDVehicleResolverServices *, const Vehicle *, uint16_t, uint8_t);
void openttd_rust_vehicle_randomise(const OpenTTDVehicleResolverServices *, const Vehicle *, uint8_t);
}
#endif /* RUST_NEWGRF_VEHICLE_FFI_H */
