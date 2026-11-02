/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file industry_resolver_ffi.h Stack contexts and typed native resolver leaves. */
#ifndef RUST_INDUSTRY_RESOLVER_FFI_H
#define RUST_INDUSTRY_RESOLVER_FFI_H
struct GRFFile;
#include "industry_ffi.h"
#include "newgrf_storage_ffi.h"
#include "../town_type.h"
enum class HouseZone : uint8_t;
struct ResolverObject;
struct ScopeResolver;
struct TownScopeResolver;
struct Town;
struct IndustrySpec;
struct IndustryTileSpec;
struct SpriteGroup;
struct OpenTTDIndustryResolverContext {
	OpenTTDIndustry *owner;
	Industry *industry;
	Town *town;
	uint32_t tile, random_bits;
	uint16_t id;
	uint8_t type;
};
/** Strict pool lookup consumes only canonical owner and native town association. */
struct OpenTTDIndustryResolverRef { OpenTTDIndustry *owner; Town *town; };
struct OpenTTDIndustryFounderInfo { uint8_t colours; bool is_ai; };
struct OpenTTDIndustryResolverServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDIndustryGenerationServices *generation;
	const OpenTTDIndustryConstructionServices *construction;
	const OpenTTDIndustryLifecycleServices *lifecycle;
	const OpenTTDIndustryTileServices *tile;
	OpenTTDIndustryResolverRef (*get)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(get);
	uint16_t (*clean_gfx)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(clean_gfx);
	uint32_t (*nearby)(uint8_t, uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(nearby);
	uint32_t (*water_distance)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(water_distance);
	uint32_t (*town_tile)(const OpenTTDTownHandle *) noexcept OPENTTD_SERVICE_DEFAULT(town_tile);
	uint16_t (*town_id)(const OpenTTDTownHandle *) noexcept OPENTTD_SERVICE_DEFAULT(town_id);
	OpenTTDIndustryFounderInfo (*founder_info)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(founder_info);
	uint16_t (*callback_id)(const ResolverObject *) noexcept OPENTTD_SERVICE_DEFAULT(callback_id);
	OpenTTDStorage256 *(*storage_owner)(Industry *) noexcept OPENTTD_SERVICE_DEFAULT(storage_owner);
	void (*unhandled)(uint8_t, bool, bool, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(unhandled);
	void (*pointer_magic)() noexcept OPENTTD_SERVICE_DEFAULT(pointer_magic);
	ScopeResolver *(*base_scope)(ResolverObject *, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(base_scope);
	TownScopeResolver *(*existing_town)(ResolverObject *) noexcept OPENTTD_SERVICE_DEFAULT(existing_town);
	TownScopeResolver *(*make_town)(ResolverObject *, Town *, bool) noexcept OPENTTD_SERVICE_DEFAULT(make_town);
	uint16_t (*callback)(uint16_t, uint32_t, uint32_t, Industry *, uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(callback);
	void (*callback_error)(const IndustrySpec *, uint16_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(callback_error);
};
const OpenTTDIndustryResolverServices &GetIndustryResolverServices() noexcept;
extern "C" {
uint32_t openttd_rust_industry_variable(const OpenTTDIndustryResolverContext *, const ResolverObject *, uint8_t, uint32_t, bool *, OpenTTDIndustryBuilder *, const OpenTTDIndustryResolverServices *);
uint32_t openttd_rust_industry_tile_variable(const OpenTTDIndustryResolverContext *, const ResolverObject *, uint8_t, uint32_t, bool *, const OpenTTDIndustryResolverServices *);
uint32_t openttd_rust_industry_resolver_random(const OpenTTDIndustryResolverContext *);
uint32_t openttd_rust_industry_tile_random(const OpenTTDIndustryResolverContext *, const OpenTTDIndustryResolverServices *);
uint32_t openttd_rust_industry_tile_triggers(const OpenTTDIndustryResolverContext *, const OpenTTDIndustryResolverServices *);
const SpriteGroup *openttd_rust_industry_resolver_root(const OpenTTDIndustryResolverContext *, const OpenTTDIndustryResolverServices *);
const SpriteGroup *openttd_rust_industry_tile_root(const OpenTTDIndustryResolverContext *, uint16_t, const OpenTTDIndustryResolverServices *);
TownScopeResolver *openttd_rust_industry_resolver_town(const OpenTTDIndustryResolverContext *, ResolverObject *, const OpenTTDIndustryResolverServices *);
ScopeResolver *openttd_rust_industry_resolver_scope(const OpenTTDIndustryResolverContext *, ResolverObject *, ScopeResolver *, uint8_t, uint8_t, const OpenTTDIndustryResolverServices *);
ScopeResolver *openttd_rust_industry_tile_scope(ResolverObject *, ScopeResolver *, ScopeResolver *, uint8_t, uint8_t, const OpenTTDIndustryResolverServices *);
uint32_t openttd_rust_industry_probability(uint8_t, uint8_t, uint32_t, const OpenTTDIndustryResolverServices *);
bool openttd_rust_industry_refuses(Industry *, OpenTTDIndustry *, uint8_t, const OpenTTDIndustryResolverServices *);
}
#endif
