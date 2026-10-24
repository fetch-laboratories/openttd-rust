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
struct OpenTTDCargoList;
struct OpenTTDRoadStopResolverState {
	BaseStation *station;
	const RoadStopSpec *spec;
	const SpriteGroup *root;
	TownScopeResolver *town_scope;
	uint32_t tile;
	uint8_t roadtype, type, view, cargo;
};
struct OpenTTDRoadStopSpecSlot { const RoadStopSpec *spec; uint32_t grfid; uint16_t local_id; };
struct OpenTTDRoadStopReseed { uint32_t reseed; uint8_t used; };
struct OpenTTDRoadStopResolverServices {
	const OpenTTDSharedServices *shared;
	BaseStation * (*by_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(by_tile);
	bool (*is_station)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(is_station);
	uint16_t (*station_id)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(station_id);
	Town * (*town)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(town);
	uint8_t (*owner)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(owner);
	uint8_t (*facilities)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(facilities);
	int32_t (*build_date)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(build_date);
	const OpenTTDCargoList * (*cargo_owner)(BaseStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_owner);
	uint16_t (*random_bits)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(random_bits);
	void (*set_random_bits)(BaseStation *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_random_bits);
	uint8_t (*waiting)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(waiting);
	void (*set_waiting)(BaseStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_waiting);
	uint8_t (*tile_random)(const BaseStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_random);
	void (*set_tile_random)(BaseStation *, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_tile_random);
	uint8_t (*frame)(BaseStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(frame);
	size_t (*tile_count)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(tile_count);
	uint32_t (*tile)(BaseStation *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(tile);
	size_t (*spec_count)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(spec_count);
	OpenTTDRoadStopSpecSlot (*slot)(const BaseStation *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(slot);
	void (*resize)(BaseStation *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(resize);
	void (*clear)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(clear);
	void (*set_slot)(BaseStation *, uint8_t, const RoadStopSpec *, uint32_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_slot);
	bool (*custom)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(custom);
	uint32_t (*index)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(index);
	CargoTypes (*cached_cargo)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(cached_cargo);
	void (*set_cached)(BaseStation *, uint16_t, CargoTypes) noexcept OPENTTD_SERVICE_DEFAULT(set_cached);
	CargoTypes (*spec_cargo)(const RoadStopSpec *) noexcept OPENTTD_SERVICE_DEFAULT(spec_cargo);
	uint16_t (*spec_animation)(const RoadStopSpec *) noexcept OPENTTD_SERVICE_DEFAULT(spec_animation);
	uint32_t (*spec_grfid)(const RoadStopSpec *) noexcept OPENTTD_SERVICE_DEFAULT(spec_grfid);
	uint16_t (*spec_local_id)(const RoadStopSpec *) noexcept OPENTTD_SERVICE_DEFAULT(spec_local_id);
	size_t (*group_count)(const RoadStopSpec *) noexcept OPENTTD_SERVICE_DEFAULT(group_count);
	uint8_t (*group_cargo)(const RoadStopSpec *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(group_cargo);
	const SpriteGroup * (*group)(const RoadStopSpec *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(group);
	uint8_t (*version)(const ResolverObject *) noexcept OPENTTD_SERVICE_DEFAULT(version);
	uint32_t (*badges)(const ResolverObject *, const RoadStopSpec *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(badges);
	uint32_t (*nearby)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(nearby);
	uint32_t (*nearby_info)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(nearby_info);
	uint32_t (*terrain)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(terrain);
	HouseZone (*town_zone)(const Town *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(town_zone);
	uint32_t (*town_tile)(const OpenTTDTownHandle *) noexcept OPENTTD_SERVICE_DEFAULT(town_tile);
	uint32_t (*company)(uint8_t, const Livery *) noexcept OPENTTD_SERVICE_DEFAULT(company);
	uint8_t (*reverse_road)(RoadType, const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(reverse_road);
	const GRFFile * (*spec_file)(const RoadStopSpec *) noexcept OPENTTD_SERVICE_DEFAULT(spec_file);
	uint32_t (*station_variable)(BaseStation *, ResolverObject *, uint8_t, uint8_t, bool *) noexcept OPENTTD_SERVICE_DEFAULT(station_variable);
	TownScopeResolver * (*make_town)(ResolverObject *, Town *, bool) noexcept OPENTTD_SERVICE_DEFAULT(make_town);
	ScopeResolver * (*base_scope)(ResolverObject *, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(base_scope);
	uint16_t (*resolve_callback)(const OpenTTDRoadStopResolverState *, uint16_t, uint32_t, uint32_t, int32_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(resolve_callback);
	OpenTTDRoadStopReseed (*resolve_reseed)(const OpenTTDRoadStopResolverState *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(resolve_reseed);
};
const OpenTTDRoadStopResolverServices &GetRoadStopResolverServices() noexcept;
extern "C" {
void openttd_rust_roadstop_init(OpenTTDRoadStopResolverState *, const OpenTTDRoadStopResolverServices *);
uint32_t openttd_rust_roadstop_variable(const OpenTTDRoadStopResolverState *, ResolverObject *, uint8_t, uint32_t, bool *, const OpenTTDRoadStopResolverServices *);
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
