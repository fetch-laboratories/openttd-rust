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
	VehicleType (*engine_kind)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_kind);
	uint8_t (*engine_subtype)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_subtype);
	RoadStop *(*primary_road_stop)(const OpenTTDVehicle *, const Station *) noexcept OPENTTD_SERVICE_DEFAULT(primary_road_stop);
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
	const AirportSpec *(*base_spec)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(base_spec);
	bool (*enabled)(const AirportSpec *) noexcept OPENTTD_SERVICE_DEFAULT(enabled);
	int32_t (*min_year)(const AirportSpec *) noexcept OPENTTD_SERVICE_DEFAULT(min_year);
	int32_t (*max_year)(const AirportSpec *) noexcept OPENTTD_SERVICE_DEFAULT(max_year);
	uint16_t (*override_id)(const AirportSpec *) noexcept OPENTTD_SERVICE_DEFAULT(override_id);
	bool (*never_expire)() noexcept OPENTTD_SERVICE_DEFAULT(never_expire);
	uint8_t (*station_layout)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(station_layout);
	uint16_t (*random)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(random);
	int32_t (*build_date)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(build_date);
	const SpriteGroup *(*root)(const AirportSpec *, bool) noexcept OPENTTD_SERVICE_DEFAULT(root);
	const SpriteGroup *(*tile_root)(const AirportTileSpec *, bool) noexcept OPENTTD_SERVICE_DEFAULT(tile_root);
	uint32_t (*badges)(const ResolverObject *, const AirportSpec *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(badges);
	uint32_t (*tile_badges)(const ResolverObject *, const AirportTileSpec *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_badges);
	const AirportTileSpec *(*tile_spec)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_spec);
	uint16_t (*tile_override)(const AirportTileSpec *) noexcept OPENTTD_SERVICE_DEFAULT(tile_override);
	uint16_t (*tile_local)(const AirportTileSpec *) noexcept OPENTTD_SERVICE_DEFAULT(tile_local);
	uint32_t (*tile_grfid)(const AirportTileSpec *) noexcept OPENTTD_SERVICE_DEFAULT(tile_grfid);
	bool (*tile_has_groups)(const AirportTileSpec *) noexcept OPENTTD_SERVICE_DEFAULT(tile_has_groups);
	uint16_t (*tile_substitute)(const AirportTileSpec *) noexcept OPENTTD_SERVICE_DEFAULT(tile_substitute);
	bool (*belongs)(const Station *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(belongs);
	uint8_t (*tile_random)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_random);
	uint32_t (*nearby)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(nearby);
	uint32_t (*nearby_info)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(nearby_info);
	uint32_t (*terrain)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(terrain);
	uint8_t (*frame)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(frame);
	uint8_t (*version)(const ResolverObject *) noexcept OPENTTD_SERVICE_DEFAULT(version);
	uint32_t (*grfid)(const ResolverObject *) noexcept OPENTTD_SERVICE_DEFAULT(grfid);
	ScopeResolver *(*base_scope)(ResolverObject *, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(base_scope);
	TownScopeResolver *(*town_scope)(AirportResolverObject *, const Town *, bool) noexcept OPENTTD_SERVICE_DEFAULT(town_scope);
	uint16_t (*evaluate_tile)(uint16_t, uint32_t, uint32_t, const AirportTileSpec *, Station *, uint32_t, int32_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(evaluate_tile);
	void (*unhandled)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(unhandled);
};
const OpenTTDAirportResolverServices &GetAirportResolverServices() noexcept;
extern "C" {
const OpenTTDAirportResolverServices *openttd_rust_airport_exchange(const OpenTTDAirportResolverServices *);
bool openttd_rust_can_engine_use_station(uint16_t, const Station *, const OpenTTDAirportResolverServices *);
bool openttd_rust_can_vehicle_use_station(const OpenTTDVehicle *, const Station *, const OpenTTDAirportResolverServices *);
bool openttd_rust_is_hangar(uint32_t, const OpenTTDAirportResolverServices *);
uint8_t openttd_rust_vehicle_pos_on_build(uint32_t, const OpenTTDAirportResolverServices *);
const AirportFTAClass *openttd_rust_airport_fta(uint8_t, const OpenTTDAirportResolverServices *);
const AirportSpec *openttd_rust_airport_get_spec(uint32_t, uint8_t, const OpenTTDAirportResolverServices *);
const AirportSpec *openttd_rust_airport_spec(uint8_t, const OpenTTDAirportResolverServices *);
bool openttd_rust_airport_available(const AirportSpec *, const OpenTTDAirportResolverServices *);
bool openttd_rust_airport_in_bounds(const AirportSpec *, uint8_t, uint32_t, const OpenTTDAirportResolverServices *);
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
