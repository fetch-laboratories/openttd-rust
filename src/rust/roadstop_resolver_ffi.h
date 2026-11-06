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
struct OpenTTDRoadStopSpecTriggers { uint16_t animation; CargoTypes cargo; };
struct OpenTTDRoadStopReseed { uint32_t reseed; uint8_t used; };
struct OpenTTDRoadStopResolverServices {
	ScopeResolver * (*base_scope)(ResolverObject *, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(base_scope);
	BaseStation * (*by_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(by_tile);
	uint32_t (*company)(uint8_t, const Livery *) noexcept OPENTTD_SERVICE_DEFAULT(company);
	bool (*custom)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(custom);
	const SpriteGroup * (*group)(const RoadStopSpec *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(group);
	uint8_t (*group_cargo)(const RoadStopSpec *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(group_cargo);
	size_t (*group_count)(const RoadStopSpec *) noexcept OPENTTD_SERVICE_DEFAULT(group_count);
	uint32_t (*index)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(index);
	TownScopeResolver * (*make_town)(ResolverObject *, Town *, bool) noexcept OPENTTD_SERVICE_DEFAULT(make_town);
	uint16_t (*resolve_callback)(const OpenTTDRoadStopResolverState *, uint16_t, uint32_t, uint32_t, int32_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(resolve_callback);
	OpenTTDRoadStopReseed (*resolve_reseed)(const OpenTTDRoadStopResolverState *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(resolve_reseed);
	const OpenTTDSharedServices *shared;
	CargoTypes (*spec_cargo)(const RoadStopSpec *) noexcept OPENTTD_SERVICE_DEFAULT(spec_cargo);
	OpenTTDRoadStopSpecTriggers (*spec_triggers)(const RoadStopSpec *) noexcept OPENTTD_SERVICE_DEFAULT(spec_triggers);
	uint32_t (*town_tile)(const OpenTTDTownHandle *) noexcept OPENTTD_SERVICE_DEFAULT(town_tile);
};
const OpenTTDRoadStopResolverServices &GetRoadStopResolverServices() noexcept;
extern "C" {
void openttd_rust_roadstop_init(OpenTTDRoadStopResolverState *, const OpenTTDRoadStopResolverServices *);
uint32_t openttd_rust_roadstop_variable(const OpenTTDRoadStopResolverState *, ResolverObject *, const GRFFile *, uint8_t, uint32_t, bool *, const OpenTTDRoadStopResolverServices *);
uint32_t openttd_rust_roadstop_random(const OpenTTDRoadStopResolverState *, bool);
TownScopeResolver *openttd_rust_roadstop_town(OpenTTDRoadStopResolverState *, ResolverObject *, const OpenTTDRoadStopResolverServices *);
ScopeResolver *openttd_rust_roadstop_scope(OpenTTDRoadStopResolverState *, ResolverObject *, ScopeResolver *, uint8_t, uint8_t, const OpenTTDRoadStopResolverServices *);
uint16_t openttd_rust_roadstop_callback(uint16_t, uint32_t, uint32_t, const RoadStopSpec *, BaseStation *, uint32_t, uint8_t, uint8_t, uint8_t, int32_t *, size_t, const OpenTTDRoadStopResolverServices *);
void openttd_rust_roadstop_randomise(BaseStation *, uint32_t, uint8_t, uint8_t, const OpenTTDRoadStopResolverServices *);
const RoadStopSpec *openttd_rust_roadstop_spec(uint32_t, const OpenTTDRoadStopResolverServices *);
int16_t openttd_rust_roadstop_allocate(const RoadStopSpec *, BaseStation *);
void openttd_rust_roadstop_assign(const RoadStopSpec *, BaseStation *, uint8_t, const OpenTTDRoadStopResolverServices *);
void openttd_rust_roadstop_deallocate(BaseStation *, uint8_t, const OpenTTDRoadStopResolverServices *);
void openttd_rust_roadstop_cache(BaseStation *, const OpenTTDRoadStopResolverServices *);
}
#endif
