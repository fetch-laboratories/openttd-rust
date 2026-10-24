/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file station_resolver_ffi.h Stack-owned station resolver and typed native leaves. */
#ifndef RUST_STATION_RESOLVER_FFI_H
#define RUST_STATION_RESOLVER_FFI_H
#include "station_command_ffi.h"
#include "animation_ffi.h"
#include "cargo_storage_ffi.h"
#include "../newgrf_airport.h"
struct Livery; struct AirportSpec; struct ResolverObject; struct RealSpriteGroup; struct SpriteGroup; struct GRFFile;
struct OpenTTDStationResolverState {
	BaseStation *station; const StationSpec *spec; const Town *town;
	uint32_t tile; uint8_t cargo; Axis axis; uint8_t valid; uint32_t cache[6];
};
struct OpenTTDStationSpecIdentity { uint32_t grfid; uint16_t local; };
struct OpenTTDStationReseed { uint32_t reseed, used; };
struct OpenTTDStationResolverSpec { uint16_t threshold; bool divide; };
struct OpenTTDStationSlopeResult { uint16_t callback; int32_t registers[16]; };
struct OpenTTDStationResolverServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDCargoStorageServices *cargo;
	const OpenTTDCargoList *(*cargo_owner)(BaseStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_owner);
	size_t (*group_count)(const StationSpec *) noexcept OPENTTD_SERVICE_DEFAULT(group_count);
	uint8_t (*group_cargo)(const StationSpec *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(group_cargo);
	const SpriteGroup *(*group)(const StationSpec *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(group);
	OpenTTDStationResolverSpec (*spec)(const StationSpec *) noexcept OPENTTD_SERVICE_DEFAULT(spec);
	size_t (*loading_count)(const RealSpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(loading_count);
	size_t (*loaded_count)(const RealSpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(loaded_count);
	const SpriteGroup *(*loading)(const RealSpriteGroup *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(loading);
	const SpriteGroup *(*loaded)(const RealSpriteGroup *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(loaded);
	Axis (*axis)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(axis);
	uint32_t (*tile_add)(uint32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_add);
	uint8_t (*tile_random)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_random);
	uint16_t (*station_random)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(station_random);
	uint8_t (*waiting)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(waiting);
	void (*set_waiting)(BaseStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_waiting);
	void (*set_random)(BaseStation *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_random);
	uint64_t (*cached_cargo)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(cached_cargo);
	uint64_t (*spec_cargo)(const StationSpec *) noexcept OPENTTD_SERVICE_DEFAULT(spec_cargo);
	OpenTTDStationReseed (*resolve_random)(const StationSpec *, BaseStation *, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(resolve_random);
	uint16_t (*resolve_callback)(uint16_t, uint32_t, uint32_t, const StationSpec *, BaseStation *, uint32_t, uint8_t, int32_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(resolve_callback);

	uint8_t (*file_version)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(file_version);
	uint32_t (*company)(uint8_t, const Livery *) noexcept OPENTTD_SERVICE_DEFAULT(company);
	uint8_t (*reverse_rail)(RailType, const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(reverse_rail);
	uint32_t (*nearby)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(nearby);
	uint32_t (*nearby_station)(uint8_t, uint32_t, Axis) noexcept OPENTTD_SERVICE_DEFAULT(nearby_station);
	uint32_t (*nearby_info)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(nearby_info);
	uint32_t (*terrain)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(terrain);
	uint32_t (*badges)(const ResolverObject *, const StationSpec *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(badges);
	uint8_t (*translate)(uint8_t, const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(translate);
	int32_t (*build_date)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(build_date);
	const AirportSpec *(*airport_spec)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(airport_spec);
	TTDPAirportType (*airport_ttd_type)(const AirportSpec *) noexcept OPENTTD_SERVICE_DEFAULT(airport_ttd_type);
	OpenTTDStationSpecIdentity (*identity)(BaseStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(identity);
	void (*unhandled)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(unhandled);
	void (*slope_error)(OpenTTDCommandCost *, const OpenTTDStationSlopeResult *, const StationSpec *) noexcept OPENTTD_SERVICE_DEFAULT(slope_error);
};
const OpenTTDStationResolverServices &GetStationResolverServices() noexcept;
extern "C" {
const OpenTTDStationResolverServices *openttd_rust_station_resolver_exchange(const OpenTTDStationResolverServices *);
uint64_t openttd_rust_station_empty_mask(BaseStation *);
uint8_t openttd_rust_goods_convert_state(uint8_t);
const Town *openttd_rust_station_resolver_town(OpenTTDStationResolverState *);
bool openttd_rust_station_scope_is_parent(OpenTTDStationResolverState *, uint8_t);
uint32_t openttd_rust_station_variable(OpenTTDStationResolverState *, const ResolverObject *, const GRFFile *, uint8_t, uint32_t, bool *);
uint32_t openttd_rust_station_base_variable(BaseStation *, const GRFFile *, uint8_t, uint8_t, bool *);
const SpriteGroup *openttd_rust_station_resolver_init(OpenTTDStationResolverState *, const StationSpec *, BaseStation *, uint32_t);
const SpriteGroup *openttd_rust_station_resolve_real(const OpenTTDStationResolverState *, const RealSpriteGroup *);
uint32_t openttd_rust_station_random_bits(const OpenTTDStationResolverState *);
uint32_t openttd_rust_station_random_triggers(const OpenTTDStationResolverState *);
uint32_t openttd_rust_platform_info(uint8_t, uint8_t, int32_t, int32_t, int32_t, int32_t, bool);
void openttd_rust_station_randomisation(BaseStation *, uint32_t, uint8_t, uint8_t);
uint16_t openttd_rust_station_callback(uint16_t, uint32_t, uint32_t, const StationSpec *, BaseStation *, uint32_t, int32_t *, size_t);
void openttd_rust_station_slope_check(OpenTTDCommandCost *, uint32_t, uint32_t, const StationSpec *, uint8_t, uint8_t, uint8_t);
}
#endif
