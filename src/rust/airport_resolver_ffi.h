/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file airport_resolver_ffi.h Stack contexts and typed airport runtime services. */
#ifndef RUST_AIRPORT_RESOLVER_FFI_H
#define RUST_AIRPORT_RESOLVER_FFI_H
#include "station_ffi.h"
#include "newgrf_storage_ffi.h"
#include "../map_type.h"
struct OpenTTDSharedServices;
struct AirportSpec; struct AirportTileSpec;
enum VehicleType : uint8_t;
struct AirportResolverObject; struct AirportTileResolverObject; struct ScopeResolver; struct TownScopeResolver; struct ResolverObject; struct SpriteGroup; struct AirportFTAClass; struct OpenTTDVehicleServices; struct OpenTTDEngine; struct RoadStop;
struct OpenTTDAirportResolverState {
	Station *station; const AirportSpec *spec; const AirportTileSpec *tile_spec;
	uint32_t tile; uint8_t layout; TownScopeResolver *town;
};
struct OpenTTDAirportResolverServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDStorageServices *storage;
	const OpenTTDVehicleServices *vehicles;
	Station *(*by_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(by_tile);
	uint16_t (*station_id)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(station_id);
	uint8_t (*facilities)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(facilities);
	Station *(*get_station)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(get_station);
	void (*check)(bool) noexcept OPENTTD_SERVICE_DEFAULT(check);
	const OpenTTDEngine *(*engine_if_valid)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(engine_if_valid);
	uint16_t (*engine_id)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_id);
	uint8_t (*fta_flags)(const AirportFTAClass *) noexcept OPENTTD_SERVICE_DEFAULT(fta_flags);
	bool (*has_hangar)(const BaseStation *, const AirportSpec *) noexcept OPENTTD_SERVICE_DEFAULT(has_hangar);
	uint32_t (*hangar_tile)(const BaseStation *, uint32_t, const AirportSpec *) noexcept OPENTTD_SERVICE_DEFAULT(hangar_tile);
	size_t (*depot_count)(const AirportSpec *) noexcept OPENTTD_SERVICE_DEFAULT(depot_count);
	TileIndexDiffC (*depot_offset)(const AirportSpec *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_offset);
	uint32_t (*rotated_offset)(const BaseStation *, TileIndexDiffC, const AirportSpec *) noexcept OPENTTD_SERVICE_DEFAULT(rotated_offset);
	uint8_t (*fta_heading)(const AirportFTAClass *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(fta_heading);
	uint8_t (*fta_position)(const AirportFTAClass *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(fta_position);
	const AirportSpec *dummy_spec;
	const AirportFTAClass *dummy_fta;
	const AirportFTAClass *(*fta)(const AirportSpec *) noexcept OPENTTD_SERVICE_DEFAULT(fta);
	uint8_t (*catchment)(const AirportSpec *) noexcept OPENTTD_SERVICE_DEFAULT(catchment);
	const AirportSpec *(*base_spec)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(base_spec);
	int32_t (*min_year)(const AirportSpec *) noexcept OPENTTD_SERVICE_DEFAULT(min_year);
	int32_t (*max_year)(const AirportSpec *) noexcept OPENTTD_SERVICE_DEFAULT(max_year);
	uint8_t (*station_layout)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(station_layout);
	uint16_t (*random)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(random);
	int32_t (*build_date)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(build_date);
	const AirportTileSpec *(*tile_spec)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_spec);
	bool (*belongs)(const Station *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(belongs);
	uint8_t (*tile_random)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_random);
	uint8_t (*frame)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(frame);
	ScopeResolver *(*base_scope)(ResolverObject *, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(base_scope);
	TownScopeResolver *(*town_scope)(AirportResolverObject *, const Town *, bool) noexcept OPENTTD_SERVICE_DEFAULT(town_scope);
	uint16_t (*evaluate_tile)(uint16_t, uint32_t, uint32_t, const AirportTileSpec *, Station *, uint32_t, int32_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(evaluate_tile);
};
const OpenTTDAirportResolverServices &GetAirportResolverServices() noexcept;
extern "C" {
const OpenTTDAirportResolverServices *openttd_rust_airport_exchange(const OpenTTDAirportResolverServices *);
uint8_t openttd_rust_can_engine_use_station_abi(uint16_t, const Station *, const OpenTTDAirportResolverServices *);
extern "C++" inline bool openttd_rust_can_engine_use_station(uint16_t a0, const Station *a1, const OpenTTDAirportResolverServices *a2) { return openttd_rust_can_engine_use_station_abi(a0, a1, a2) != 0; }
uint8_t openttd_rust_can_vehicle_use_station_abi(const OpenTTDVehicle *, const Station *, const OpenTTDAirportResolverServices *);
extern "C++" inline bool openttd_rust_can_vehicle_use_station(const OpenTTDVehicle *a0, const Station *a1, const OpenTTDAirportResolverServices *a2) { return openttd_rust_can_vehicle_use_station_abi(a0, a1, a2) != 0; }
uint8_t openttd_rust_is_hangar_abi(uint32_t, const OpenTTDAirportResolverServices *);
extern "C++" inline bool openttd_rust_is_hangar(uint32_t a0, const OpenTTDAirportResolverServices *a1) { return openttd_rust_is_hangar_abi(a0, a1) != 0; }
const AirportSpec *openttd_rust_airport_get_spec(uint32_t, uint8_t, const OpenTTDAirportResolverServices *);
const AirportSpec *openttd_rust_airport_spec(uint8_t, const OpenTTDAirportResolverServices *);
uint8_t openttd_rust_airport_available_abi(const AirportSpec *, const OpenTTDAirportResolverServices *);
extern "C++" inline bool openttd_rust_airport_available(const AirportSpec *a0, const OpenTTDAirportResolverServices *a1) { return openttd_rust_airport_available_abi(a0, a1) != 0; }
uint8_t openttd_rust_airport_in_bounds_abi(const AirportSpec *, uint8_t, uint32_t, const OpenTTDAirportResolverServices *);
extern "C++" inline bool openttd_rust_airport_in_bounds(const AirportSpec *a0, uint8_t a1, uint32_t a2, const OpenTTDAirportResolverServices *a3) { return openttd_rust_airport_in_bounds_abi(a0, a1, a2, a3) != 0; }
void openttd_rust_airport_init(OpenTTDAirportResolverState *, Station *, const AirportSpec *, const AirportTileSpec *, uint32_t, uint8_t, const OpenTTDAirportResolverServices *);
const SpriteGroup *openttd_rust_airport_tile_init(const OpenTTDAirportResolverState *, OpenTTDAirportResolverState *, const OpenTTDAirportResolverServices *);
const SpriteGroup *openttd_rust_airport_root(const OpenTTDAirportResolverState *, const OpenTTDAirportResolverServices *);
TownScopeResolver *openttd_rust_airport_town(OpenTTDAirportResolverState *, AirportResolverObject *, const OpenTTDAirportResolverServices *);
ScopeResolver *openttd_rust_airport_scope(OpenTTDAirportResolverState *, ResolverObject *, ScopeResolver *, ScopeResolver *, uint8_t, uint8_t, const OpenTTDAirportResolverServices *);
uint32_t openttd_rust_airport_variable(const OpenTTDAirportResolverState *, const ResolverObject *, const GRFFile *, uint8_t, uint32_t, bool *, const OpenTTDAirportResolverServices *);
uint32_t openttd_rust_airport_tile_variable(const OpenTTDAirportResolverState *, const ResolverObject *, uint8_t, uint32_t, bool *, const OpenTTDAirportResolverServices *);
uint32_t openttd_rust_airport_random(const OpenTTDAirportResolverState *, bool, const OpenTTDAirportResolverServices *);
uint8_t openttd_rust_airport_translate_tile(uint8_t, const OpenTTDAirportResolverServices *);
}
#endif
