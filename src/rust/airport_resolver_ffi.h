/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file airport_resolver_ffi.h Stack contexts and typed airport runtime services. */
#ifndef RUST_AIRPORT_RESOLVER_FFI_H
#define RUST_AIRPORT_RESOLVER_FFI_H
#include "station_command_ffi.h"
#include "newgrf_storage_ffi.h"
struct AirportResolverObject; struct AirportTileResolverObject; struct ScopeResolver; struct TownScopeResolver; struct ResolverObject; struct SpriteGroup;
struct OpenTTDAirportResolverState {
	Station *station; const AirportSpec *spec; const AirportTileSpec *tile_spec;
	uint32_t tile; uint8_t layout; TownScopeResolver *town;
};
struct OpenTTDAirportAvailability { bool enabled; int32_t min_year, max_year; };
struct OpenTTDAirportResolverServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDStorageServices *storage;
	const AirportSpec *(*base_spec)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(base_spec);
	OpenTTDAirportAvailability (*availability)(const AirportSpec *) noexcept OPENTTD_SERVICE_DEFAULT(availability);
	uint16_t (*override_id)(const AirportSpec *) noexcept OPENTTD_SERVICE_DEFAULT(override_id);
	uint32_t (*manager_grfid)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(manager_grfid);
	uint8_t (*substitute)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(substitute);
	bool (*never_expire)() noexcept OPENTTD_SERVICE_DEFAULT(never_expire);
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
	TownScopeResolver *(*town_scope)(AirportResolverObject *, Town *, bool) noexcept OPENTTD_SERVICE_DEFAULT(town_scope);
	uint16_t (*evaluate_tile)(uint16_t, uint32_t, uint32_t, const AirportTileSpec *, Station *, uint32_t, int32_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(evaluate_tile);
	void (*unhandled)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(unhandled);
};
const OpenTTDAirportResolverServices &GetAirportResolverServices() noexcept;
extern "C" {
const AirportSpec *openttd_rust_airport_spec(uint8_t, const OpenTTDAirportResolverServices *);
bool openttd_rust_airport_available(const AirportSpec *, const OpenTTDAirportResolverServices *);
bool openttd_rust_airport_in_bounds(const AirportSpec *, uint8_t, uint32_t, const OpenTTDAirportResolverServices *);
const SpriteGroup *openttd_rust_airport_init(OpenTTDAirportResolverState *, Station *, const AirportSpec *, const AirportTileSpec *, uint32_t, uint8_t, const OpenTTDAirportResolverServices *);
TownScopeResolver *openttd_rust_airport_town(OpenTTDAirportResolverState *, AirportResolverObject *, const OpenTTDAirportResolverServices *);
ScopeResolver *openttd_rust_airport_scope(OpenTTDAirportResolverState *, ResolverObject *, ScopeResolver *, ScopeResolver *, uint8_t, uint8_t, const OpenTTDAirportResolverServices *);
uint32_t openttd_rust_airport_variable(const OpenTTDAirportResolverState *, const ResolverObject *, const GRFFile *, uint8_t, uint32_t, bool *, const OpenTTDAirportResolverServices *);
uint32_t openttd_rust_airport_tile_variable(const OpenTTDAirportResolverState *, const ResolverObject *, uint8_t, uint32_t, bool *, const OpenTTDAirportResolverServices *);
uint32_t openttd_rust_airport_random(const OpenTTDAirportResolverState *, bool, const OpenTTDAirportResolverServices *);
uint8_t openttd_rust_airport_translate_tile(uint8_t, const OpenTTDAirportResolverServices *);
}
#endif
