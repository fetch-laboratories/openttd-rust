struct OpenTTDEngineCapabilitiesServices;
/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_vehicle_ffi.h Stack vehicle resolver and canonical query cache boundary. */
#ifndef RUST_NEWGRF_VEHICLE_FFI_H
#define RUST_NEWGRF_VEHICLE_FFI_H
#include "spec_services_ffi.h"
#include "vehicle_ffi.h"
#include "station_ffi.h"
struct Vehicle;
struct NewGRFCache;
struct ResolverObject;
struct SpriteGroup;
struct RealSpriteGroup;
struct GRFFile;
struct Livery;
struct OpenTTDSharedServices;
struct OpenTTDAircraftState;
struct OpenTTDAircraftLeaves;
struct AirportSpec;
enum TTDPAirportType : uint8_t;
struct OpenTTDGroundPhysicsServices;
struct OpenTTDTrainServices;
struct OpenTTDCargoStorageServices;
struct OpenTTDShipState;
enum RoadTramType : bool;
enum RailType : uint8_t;
enum RoadType : uint8_t;
/** Invocation-local state; no owner/reference survives common Action2 evaluation. */
struct OpenTTDVehicleScopeState { const OpenTTDVehicle *v; uint16_t self_type; bool rotor_in_gui; };
struct OpenTTDVehicleResolverState {
	OpenTTDVehicleScopeState self_, parent, relative;
	uint8_t cached_relative_count;
	const SpriteGroup *root;
};
struct OpenTTDVehicleLoadingOrder { uint8_t unload, load; bool loading; };
struct OpenTTDVehicleRandomResult { uint32_t used, reseed; };
struct OpenTTDVehicleCargoInfo { uint16_t classes; uint8_t weight; };
struct OpenTTDVehicleResolverServices {
	const OpenTTDVehicleServices *vehicle;
	const OpenTTDSharedServices *shared;
	NewGRFCache *(*cache)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cache);
	const OpenTTDEngineCapabilitiesServices *capabilities;
	uint16_t (*cargo_classes)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_classes);
	uint8_t (*cargo_bitnum)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_bitnum);
	const Livery *(*livery)(uint16_t, uint8_t, uint16_t, OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(livery);
	uint32_t (*company_info)(uint8_t, const Livery *) noexcept OPENTTD_SERVICE_DEFAULT(company_info);
	const SpriteGroup *(*cached_override)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cached_override);
	void (*write_override)(OpenTTDVehicle *, const SpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(write_override);
	uint16_t (*random_bits)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(random_bits);
	uint8_t (*waiting)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(waiting);
	void (*write_random)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(write_random);
	void (*write_waiting)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_waiting);
	uint16_t (*resolve_callback)(OpenTTDVehicleResolverState *, uint16_t, uint16_t, uint32_t, uint32_t, int32_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(resolve_callback);
	OpenTTDVehicleRandomResult (*resolve_random)(OpenTTDVehicleResolverState *, uint16_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(resolve_random);
	OpenTTDVehicleLoadingOrder (*order)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(order);
	OpenTTDCargoList *(*cargo_owner)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_owner);
	uint32_t (*real_count)(const RealSpriteGroup *, bool) noexcept OPENTTD_SERVICE_DEFAULT(real_count);
	const SpriteGroup *(*real_at)(const RealSpriteGroup *, bool, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(real_at);
	const OpenTTDEngine * (*engine_by_id)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(engine_by_id);
	OpenTTDVehicleCargoInfo (*cargo_info)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_info);
	int32_t (*build_year)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(build_year);
	int32_t (*last_service)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(last_service);
	bool (*primary)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(primary);
	const OpenTTDTrainServices *(*train_services)() noexcept OPENTTD_SERVICE_DEFAULT(train_services);
	uint16_t (*order_max_speed)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(order_max_speed);
	uint32_t (*id)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(id);
	uint16_t (*old_order)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(old_order);
	uint8_t (*num_orders)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(num_orders);
	uint8_t (*real_order)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(real_order);
	uint16_t (*load_ticks)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(load_ticks);
	uint16_t (*service_interval)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(service_interval);
	uint8_t (*cargo_type)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_type);
	uint8_t (*cargo_subtype)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_subtype);
	int32_t (*age)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(age);
	int32_t (*max_age)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(max_age);
	uint16_t (*unit)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(unit);
	uint8_t (*sprite)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(sprite);
	uint8_t (*breakdowns)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(breakdowns);
	uint8_t (*breakdown_delay)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(breakdown_delay);
	uint8_t (*breakdown_chance)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(breakdown_chance);
	uint16_t (*reliability)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(reliability);
	uint16_t (*reliability_decay)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(reliability_decay);
	int64_t (*value)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(value);
	int64_t (*profit_this)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(profit_this);
	int64_t (*profit_last)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(profit_last);
	uint16_t (*vehicle_flags)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_flags);
	const OpenTTDStation * (*target_airport)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(target_airport);
	TTDPAirportType (*airport_ttd_type)(const AirportSpec *) noexcept OPENTTD_SERVICE_DEFAULT(airport_ttd_type);
	uint16_t (*moving_flags)(const void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(moving_flags);
	RoadType (*roadtype)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(roadtype);
	RoadType (*tile_roadtype)(uint32_t, RoadTramType) noexcept OPENTTD_SERVICE_DEFAULT(tile_roadtype);
	bool (*rail_catenary)(RailType) noexcept OPENTTD_SERVICE_DEFAULT(rail_catenary);
	bool (*road_catenary)(RoadType) noexcept OPENTTD_SERVICE_DEFAULT(road_catenary);
	bool (*compatible_rail)(RailType, RailType) noexcept OPENTTD_SERVICE_DEFAULT(compatible_rail);
	bool (*powered_rail)(uint64_t, RailType) noexcept OPENTTD_SERVICE_DEFAULT(powered_rail);
	bool (*powered_single_rail)(RailType, RailType) noexcept OPENTTD_SERVICE_DEFAULT(powered_single_rail);
	size_t (*badge_count)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(badge_count);
	uint32_t (*badge_at)(const GRFFile *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(badge_at);
	OpenTTDBadgeSpan (*rail_badge)(RailType) noexcept OPENTTD_SERVICE_DEFAULT(rail_badge);
	OpenTTDBadgeSpan (*road_badge)(RoadType) noexcept OPENTTD_SERVICE_DEFAULT(road_badge);
	OpenTTDShipState * (*ship_owner)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(ship_owner);
	const OpenTTDAircraftLeaves * (*aircraft_leaves)() noexcept OPENTTD_SERVICE_DEFAULT(aircraft_leaves);
	const OpenTTDGroundPhysicsServices * (*ground)() noexcept OPENTTD_SERVICE_DEFAULT(ground);
	const OpenTTDCargoStorageServices * (*cargo_services)() noexcept OPENTTD_SERVICE_DEFAULT(cargo_services);
};
const OpenTTDVehicleResolverServices &GetRustVehicleResolverServices() noexcept;
extern "C" {
int32_t openttd_rust_vehicle_shell_property(const OpenTTDVehicleResolverServices *, OpenTTDVehicle *, uint8_t, int32_t) noexcept;
uint16_t openttd_rust_vehicle_shell_callback(const OpenTTDVehicleResolverServices *, OpenTTDVehicle *, CallbackID) noexcept;
NewGRFCache *openttd_rust_vehicle_grf_cache_new();
void openttd_rust_vehicle_grf_cache_delete(NewGRFCache *);
uint8_t *openttd_rust_vehicle_visual_cache(NewGRFCache *);
void openttd_rust_vehicle_grf_invalidate(const OpenTTDVehicleResolverServices *, const OpenTTDVehicle *, bool);
void openttd_rust_vehicle_grf_fill(const OpenTTDVehicleResolverServices *, const OpenTTDVehicle *);
void openttd_rust_vehicle_resolver_init(const OpenTTDVehicleResolverServices *, OpenTTDVehicleResolverState *, uint16_t, const OpenTTDVehicle *, uint8_t, bool);
uint8_t openttd_rust_vehicle_resolver_scope(const OpenTTDVehicleResolverServices *, OpenTTDVehicleResolverState *, const ResolverObject *, uint8_t, uint8_t);
void openttd_rust_vehicle_scope_set(OpenTTDVehicleScopeState *, const OpenTTDVehicle *);
uint32_t openttd_rust_vehicle_scope_variable(const OpenTTDVehicleResolverServices *, const OpenTTDVehicleScopeState *, const ResolverObject *, const GRFFile *, uint16_t, uint8_t, uint32_t, bool *);
uint32_t openttd_rust_vehicle_scope_random(const OpenTTDVehicleResolverServices *, const OpenTTDVehicleScopeState *, bool);
const SpriteGroup *openttd_rust_vehicle_wagon_override(const OpenTTDVehicleResolverServices *, uint16_t, uint8_t, uint16_t);
uint16_t openttd_rust_vehicle_callback(const OpenTTDVehicleResolverServices *, uint16_t, uint32_t, uint32_t, uint16_t, const OpenTTDVehicle *, const OpenTTDVehicle *, bool, int32_t *, size_t);
int32_t openttd_rust_vehicle_property(const OpenTTDVehicleResolverServices *, uint16_t, uint8_t, int32_t, const OpenTTDVehicle *, bool);
int8_t openttd_rust_vehicle_build_probability(const OpenTTDVehicleResolverServices *, const OpenTTDVehicle *, uint16_t, uint8_t);
void openttd_rust_vehicle_randomise(const OpenTTDVehicleResolverServices *, const OpenTTDVehicle *, uint8_t);
}
#endif /* RUST_NEWGRF_VEHICLE_FFI_H */
