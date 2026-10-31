/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file roadstop_resolver_ffi.h Stack invocation state and typed native leaves. */
#ifndef RUST_ROADSTOP_RESOLVER_FFI_H
#define RUST_ROADSTOP_RESOLVER_FFI_H
#include "services_ffi.h"
#include "../cargo_type.h"
struct RoadStopSpec;
struct SpriteGroup;
struct ResolverObject;
struct ScopeResolver;
struct TownScopeResolver;
struct Town;
struct GRFFile;
struct Livery;
enum class HouseZone : uint8_t;
struct OpenTTDRoadStopResolverState {
	BaseStation *station;
	const RoadStopSpec *spec;
	const SpriteGroup *root;
	TownScopeResolver *town_scope;
	uint32_t tile;
	uint8_t roadtype, type, view, cargo;
};
struct OpenTTDRoadStopSpecKey { uint32_t grfid; uint16_t local_id; };
struct OpenTTDRoadStopSpecVacancy { const RoadStopSpec *spec; uint32_t grfid; };
struct OpenTTDRoadStopSpecTriggers { uint16_t animation; CargoTypes cargo; };
struct OpenTTDRoadStopReseed { uint32_t reseed; uint8_t used; };
struct OpenTTDRoadStopResolverServices {
	void (*rebind)(BaseStation *, size_t, const RoadStopSpec *) noexcept OPENTTD_SERVICE_DEFAULT(rebind);
	uint32_t (*badges)(const ResolverObject *, const RoadStopSpec *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(badges);
	ScopeResolver * (*base_scope)(ResolverObject *, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(base_scope);
	int32_t (*build_date)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(build_date);
	BaseStation * (*by_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(by_tile);
	CargoTypes (*cached_cargo)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(cached_cargo);
	void (*clear)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(clear);
	uint32_t (*company)(uint8_t, const Livery *) noexcept OPENTTD_SERVICE_DEFAULT(company);
	bool (*custom)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(custom);
	uint8_t (*facilities)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(facilities);
	uint8_t (*frame)(BaseStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(frame);
	const SpriteGroup * (*group)(const RoadStopSpec *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(group);
	uint8_t (*group_cargo)(const RoadStopSpec *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(group_cargo);
	size_t (*group_count)(const RoadStopSpec *) noexcept OPENTTD_SERVICE_DEFAULT(group_count);
	uint32_t (*index)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(index);
	bool (*is_station)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(is_station);
	TownScopeResolver * (*make_town)(ResolverObject *, Town *, bool) noexcept OPENTTD_SERVICE_DEFAULT(make_town);
	uint32_t (*nearby)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(nearby);
	uint32_t (*nearby_info)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(nearby_info);
	uint8_t (*owner)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(owner);
	uint16_t (*random_bits)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(random_bits);
	void (*resize)(BaseStation *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(resize);
	uint16_t (*resolve_callback)(const OpenTTDRoadStopResolverState *, uint16_t, uint32_t, uint32_t, int32_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(resolve_callback);
	OpenTTDRoadStopReseed (*resolve_reseed)(const OpenTTDRoadStopResolverState *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(resolve_reseed);
	void (*set_cached)(BaseStation *, uint16_t, CargoTypes) noexcept OPENTTD_SERVICE_DEFAULT(set_cached);
	void (*set_random_bits)(BaseStation *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_random_bits);
	void (*set_slot)(BaseStation *, uint8_t, const RoadStopSpec *, uint32_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_slot);
	void (*set_tile_random)(BaseStation *, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_tile_random);
	void (*set_waiting)(BaseStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_waiting);
	const OpenTTDSharedServices *shared;
	uint32_t (*slot_grfid)(const BaseStation *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(slot_grfid);
	OpenTTDRoadStopSpecKey (*slot_key)(const BaseStation *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(slot_key);
	const RoadStopSpec *(*slot_spec)(const BaseStation *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(slot_spec);
	OpenTTDRoadStopSpecVacancy (*slot_vacancy)(const BaseStation *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(slot_vacancy);
	CargoTypes (*spec_cargo)(const RoadStopSpec *) noexcept OPENTTD_SERVICE_DEFAULT(spec_cargo);
	size_t (*spec_count)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(spec_count);
	const GRFFile * (*spec_file)(const RoadStopSpec *) noexcept OPENTTD_SERVICE_DEFAULT(spec_file);
	uint32_t (*spec_grfid)(const RoadStopSpec *) noexcept OPENTTD_SERVICE_DEFAULT(spec_grfid);
	uint16_t (*spec_local_id)(const RoadStopSpec *) noexcept OPENTTD_SERVICE_DEFAULT(spec_local_id);
	OpenTTDRoadStopSpecTriggers (*spec_triggers)(const RoadStopSpec *) noexcept OPENTTD_SERVICE_DEFAULT(spec_triggers);
	uint16_t (*station_id)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(station_id);
	uint32_t (*terrain)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(terrain);
	uint32_t (*tile)(BaseStation *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(tile);
	size_t (*tile_count)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(tile_count);
	uint8_t (*tile_random)(const BaseStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_random);
	Town * (*town)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(town);
	uint32_t (*town_tile)(const OpenTTDTownHandle *) noexcept OPENTTD_SERVICE_DEFAULT(town_tile);
	uint8_t (*version)(const ResolverObject *) noexcept OPENTTD_SERVICE_DEFAULT(version);
	uint8_t (*waiting)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(waiting);
};
const OpenTTDRoadStopResolverServices &GetRoadStopResolverServices() noexcept;
extern "C" {
void openttd_rust_roadstop_init(OpenTTDRoadStopResolverState *, const OpenTTDRoadStopResolverServices *);
uint32_t openttd_rust_roadstop_variable(const OpenTTDRoadStopResolverState *, ResolverObject *, const GRFFile *, uint8_t, uint32_t, bool *, const OpenTTDRoadStopResolverServices *);
uint32_t openttd_rust_roadstop_random(const OpenTTDRoadStopResolverState *, bool, const OpenTTDRoadStopResolverServices *);
TownScopeResolver *openttd_rust_roadstop_town(OpenTTDRoadStopResolverState *, ResolverObject *, const OpenTTDRoadStopResolverServices *);
ScopeResolver *openttd_rust_roadstop_scope(OpenTTDRoadStopResolverState *, ResolverObject *, ScopeResolver *, uint8_t, uint8_t, const OpenTTDRoadStopResolverServices *);
uint16_t openttd_rust_roadstop_callback(uint16_t, uint32_t, uint32_t, const RoadStopSpec *, BaseStation *, uint32_t, uint8_t, uint8_t, uint8_t, int32_t *, size_t, const OpenTTDRoadStopResolverServices *);
void openttd_rust_roadstop_randomise(BaseStation *, uint32_t, uint8_t, uint8_t, const OpenTTDRoadStopResolverServices *);
const RoadStopSpec *openttd_rust_roadstop_spec(uint32_t, const OpenTTDRoadStopResolverServices *);
int16_t openttd_rust_roadstop_allocate(const RoadStopSpec *, BaseStation *, const OpenTTDRoadStopResolverServices *);
void openttd_rust_roadstop_assign(const RoadStopSpec *, BaseStation *, uint8_t, const OpenTTDRoadStopResolverServices *);
void openttd_rust_roadstop_deallocate(BaseStation *, uint8_t, const OpenTTDRoadStopResolverServices *);
void openttd_rust_roadstop_cache(BaseStation *, const OpenTTDRoadStopResolverServices *);
}
#endif
