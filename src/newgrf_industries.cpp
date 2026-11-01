/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_industries.cpp Handling of NewGRF industries. */

#include "stdafx.h"
#include "debug.h"
#include "industry.h"
#include "newgrf_badge.h"
#include "newgrf_industries.h"
#include "newgrf_town.h"
#include "newgrf_cargo.h"
#include "window_func.h"
#include "town.h"
#include "company_base.h"
#include "error.h"
#include "strings_func.h"
#include "core/random_func.hpp"
#include "timer/timer_game_calendar.h"

#include "table/strings.h"

#include "rust/services_industry.h"
#include "rust/industry_resolver_ffi.h"
#include "rust/services_newgrf.h"
#include "rust/services_cargo.h"
#include "rust/services_map.h"
#include "rust/industry_shell_services.h"
#include "rust/industry_generation_adapter.hpp"
#include "rust/town_services.h"
#include "rust/environment_services.h"
#include "rust/tile_animation_services.h"
#include "newgrf_industrytiles.h"
#include "rust/company_services.h"
#include "safeguards.h"

/* Since the industry IDs defined by the GRF file don't necessarily correlate
 * to those used by the game, the IDs used for overriding old industries must be
 * translated when the idustry spec is set. */
IndustryOverrideManager _industry_mngr(NEW_INDUSTRYOFFSET, NUM_INDUSTRYTYPES, IT_INVALID);
IndustryTileOverrideManager _industile_mngr(NEW_INDUSTRYTILEOFFSET, NUM_INDUSTRYTILES, INVALID_INDUSTRYTILE);

/**
 * Map the GRF local type to an industry type.
 * @param grf_type The GRF local type.
 * @param grf_id The GRF of the local type.
 * @return The industry type in the global scope.
 */

/**
 * Make an analysis of a tile and check for its belonging to the same
 * industry, and/or the same grf file
 * @param tile TileIndex of the tile to query
 * @param i Industry to which to compare the tile to
 * @param cur_grfid GRFID of the current callback chain
 * @return value encoded as per NFO specs
 */


/* virtual */ uint32_t IndustriesScopeResolver::GetVariable(uint8_t variable, [[maybe_unused]] uint32_t parameter, bool &available) const
{
	return openttd_rust_industry_variable(&this->context, &this->ro, variable, parameter, &available, RustIndustryBuilderState(), &GetIndustryResolverServices());
}

/* virtual */ uint32_t IndustriesScopeResolver::GetRandomBits() const
{
	return openttd_rust_industry_resolver_random(&this->context);
}

/* virtual */ uint32_t IndustriesScopeResolver::GetRandomTriggers() const
{
	return 0;
}

/* virtual */ void IndustriesScopeResolver::StorePSA(uint pos, int32_t value)
{
	openttd_rust_storage_industry(&GetRustStorageServices(), this->context.industry, pos, value);
}

/**
 * Get the grf file associated with the given industry type.
 * @param type Industry type to query.
 * @return The associated GRF file, if any.
 */
static const GRFFile *GetGrffile(IndustryType type)
{
	const IndustrySpec *indspec = GetIndustrySpec(type);
	return (indspec != nullptr) ? indspec->grf_prop.grffile : nullptr;
}

/**
 * Constructor of the industries resolver.
 * @param tile %Tile owned by the industry.
 * @param indus %Industry being resolved.
 * @param type Type of the industry.
 * @param random_bits Random bits of the new industry.
 * @param callback Callback ID.
 * @param callback_param1 First parameter (var 10) of the callback.
 * @param callback_param2 Second parameter (var 18) of the callback.
 */
IndustriesResolverObject::IndustriesResolverObject(TileIndex tile, Industry *indus, IndustryType type, uint32_t random_bits,
		CallbackID callback, uint32_t callback_param1, uint32_t callback_param2)
	: ResolverObject(GetGrffile(type), callback, callback_param1, callback_param2),
	industries_scope(*this, tile, indus, type, random_bits)
{
	this->root_spritegroup = openttd_rust_industry_resolver_root(&this->industries_scope.context, &GetIndustryResolverServices());
}

/**
 * Get or create the town scope object associated with the industry.
 * @return The associated town scope, if it exists.
 */
TownScopeResolver *IndustriesResolverObject::GetTown()
{
	return openttd_rust_industry_resolver_town(&this->industries_scope.context, this, &GetIndustryResolverServices());
}

GrfSpecFeature IndustriesResolverObject::GetFeature() const
{
	return GSF_INDUSTRIES;
}

uint32_t IndustriesResolverObject::GetDebugID() const
{
	return RustSpecLocalID(GetIndustrySpec(this->industries_scope.context.type));
}

/**
 * Perform an industry callback.
 * @param callback The callback to perform.
 * @param param1 The first parameter.
 * @param param2 The second parameter.
 * @param industry The industry to do the callback for.
 * @param type The type of industry to do the callback for.
 * @param tile The tile associated with the callback.
 * @param[out] regs100 Additional result values from registers 100+
 * @return The callback result.
 */
uint16_t GetIndustryCallback(CallbackID callback, uint32_t param1, uint32_t param2, Industry *industry, IndustryType type, TileIndex tile, std::span<int32_t> regs100)
{
	IndustriesResolverObject object(tile, industry, type, 0, callback, param1, param2);
	return object.ResolveCallback(regs100);
}

/**
 * Check that the industry callback allows creation of the industry.
 * @param tile %Tile to build the industry.
 * @param type Type of industry to build.
 * @param layout Layout number.
 * @param seed Seed for the random generator.
 * @param initial_random_bits The random bits the industry is going to have after construction.
 * @param founder Industry founder
 * @param creation_type The circumstances the industry is created under.
 * @return Succeeded or failed command.
 */
extern "C" OpenTTDIndustryLocationResult openttd_industry_resolve_location(Industry *industry, uint32_t tile, uint8_t type, uint32_t seed, uint8_t creation) noexcept
{
	industry->psa = nullptr;
	IndustriesResolverObject object(TileIndex(tile), industry, type, seed, CBID_INDUSTRY_LOCATION, 0, creation);
	OpenTTDIndustryLocationResult result{};
	result.callback = object.ResolveCallback(result.registers);
	return result;
}


/**
 * Check with callback #CBID_INDUSTRY_PROBABILITY whether the industry can be built.
 * @param type Industry type to check.
 * @param creation_type Reason to construct a new industry.
 * @return If the industry has no callback or allows building, \c true is returned. Otherwise, \c false is returned.
 */
uint32_t GetIndustryProbabilityCallback(IndustryType type, IndustryAvailabilityCallType creation_type, uint32_t default_prob)
{
	return openttd_rust_industry_probability(type, creation_type, default_prob, &GetIndustryResolverServices());
}

static void RustIndustryResolveProduction(void *context, uint32_t random, uint32_t parameter, OpenTTDIndustryProductionResult *out) noexcept
{
	auto &object = *static_cast<IndustriesResolverObject *>(context);
	object.callback_param1 = random;
	object.callback_param2 = parameter;
	const auto *group = object.Resolve<IndustryProductionSpriteGroup>();
	out->present = group != nullptr;
	if (group == nullptr) return;
	out->version = group->version;
	if (group->version == 0xFF) return;
	out->num_input = group->num_input;
	out->num_output = group->num_output;
	auto deref = [&](int field) { return group->version >= 1 ? object.GetRegister(field) : field; };
	for (size_t n = 0; n < group->num_input; n++) {
		out->subtract[n] = deref(group->subtract_input[n]);
		if (group->version >= 2) out->cargo_input[n] = group->cargo_input[n];
	}
	for (size_t n = 0; n < group->num_output; n++) {
		out->add[n] = deref(group->add_output[n]);
		if (group->version >= 2) out->cargo_output[n] = group->cargo_output[n];
	}
	out->again = deref(group->again);
}
const OpenTTDIndustryDeliveryServices &GetIndustryDeliveryServices() noexcept
{
	static_assert(sizeof(IndustriesResolverObject) <= sizeof(OpenTTDIndustryProductionStorage));
	static_assert(alignof(IndustriesResolverObject) <= alignof(OpenTTDIndustryProductionStorage));
	static const OpenTTDIndustryDeliveryServices services = {
		.construct = [](OpenTTDIndustryProductionStorage *storage, Industry *ind) noexcept {
			auto *object = std::construct_at(reinterpret_cast<IndustriesResolverObject *>(storage), ind->location.tile, ind, ind->type);
			return reinterpret_cast<OpenTTDIndustryProductionContext *>(object);
		},
		.destroy = [](OpenTTDIndustryProductionContext *context) noexcept { reinterpret_cast<IndustriesResolverObject *>(context)->~IndustriesResolverObject(); },
		.resolve = [](OpenTTDIndustryProductionContext *context, uint32_t random, uint32_t parameter, OpenTTDIndustryProductionResult *out) noexcept { RustIndustryResolveProduction(context, random, parameter, out); },
		.input_multiplier = [](const OpenTTDIndustrySpecHandle *spec, size_t input, size_t output) noexcept { return reinterpret_cast<const IndustrySpec *>(spec)->input_cargo_multiplier[input][output]; },
	};
	return services;
}

/**
 * Get the industry production callback and apply it to the industry.
 * @param ind    the industry this callback has to be called for
 * @param reason the reason it is called (0 = incoming cargo, 1 = periodic tick callback)
 */
void IndustryProductionCallback(Industry *ind, int reason)
{
	const IndustrySpec *spec = GetIndustrySpec(ind->type);
	IndustriesResolverObject object(ind->location.tile, ind, ind->type);
	/* Resolution is bounded and does not run scripts/commands. Rust owns random,
	 * repeat limit, version policy and all slot mutations; this leaf only resolves
	 * and copies register values while the native resolver context is live. */
	openttd_rust_industry_production_callback(ind, ind->production_owner.get(), &object, spec->behaviour.base(), reason,
		RustIndustryResolveProduction, &GetIndustryRustServices());
}

/**
 * Check whether an industry temporarily refuses to accept a certain cargo.
 * @param ind The industry to query.
 * @param cargo_type The cargo to get information about.
 * @pre cargo_type is in ind->accepts_cargo.
 * @return Whether the given industry refuses to accept this cargo type.
 */
bool IndustryTemporarilyRefusesCargo(Industry *ind, CargoType cargo_type)
{
	assert(ind->IsCargoAccepted(cargo_type));
	return openttd_rust_industry_refuses(ind, ind->production_owner.get(), cargo_type, &GetIndustryResolverServices());
}

IndustriesScopeResolver::IndustriesScopeResolver(ResolverObject &ro, TileIndex tile, Industry *industry, IndustryType type, uint32_t random_bits)
	: ScopeResolver(ro), context{industry == nullptr ? nullptr : industry->production_owner.get(), industry, industry == nullptr ? nullptr : industry->town, tile.base(), random_bits, industry == nullptr ? IndustryID::Invalid().base() : industry->index.base(), type}
{
}
ScopeResolver *IndustriesResolverObject::GetScope(VarSpriteGroupScope scope, uint8_t relative)
{
	return openttd_rust_industry_resolver_scope(&this->industries_scope.context, this, &this->industries_scope, scope, relative, &GetIndustryResolverServices());
}

/* Spec, Action2 and native associations remain live for synchronous resolution.
 * No ordinary-play leaf throws; debug/allocation failures terminate in noexcept.
 * Action2 and town/storage operations may reenter Rust; no owner borrow spans them. */
const OpenTTDIndustryResolverServices &GetIndustryResolverServices() noexcept
{
	static const OpenTTDIndustryResolverServices services{
		.shared = &GetRustSharedServices(),
		.generation = &GetIndustryGenerationServices(),
		.construction = &GetIndustryConstructionServices(),
		.lifecycle = &GetIndustryLifecycleRustServices(),
		.tile = &GetIndustryTileServices(),
		.get = [](uint16_t id) noexcept -> OpenTTDIndustryResolverRef { Industry *i = RustIndustryByID(id); return {RustIndustryCanonicalOwner(i), OpenTTDIndustryTown(i)}; },
		.clean_gfx = [](uint32_t tile) noexcept { return GetCleanIndustryGfx(TileIndex(tile)); },
		.nearby = RustNearbyTile,
		.water_distance = RustClosestWaterDistance,
		.town_tile = OpenTTDTownTile,
		.town_id = RustTownAuthorityIndex,
		.founder_info = [](uint8_t founder) noexcept -> OpenTTDIndustryFounderInfo {
			const Company *company = Company::GetIfValid(CompanyID(founder));
			return {company == nullptr ? uint8_t{0} : RustCompanyRecolour(company, true), company != nullptr && company->is_ai};
		},
		.callback_id = RustResolverCallback,
		.storage_owner = RustIndustryStorageOwner,
		.unhandled = [](uint8_t variable, bool tile, bool missing, uint16_t callback) noexcept {
			if (missing) Debug(grf, 1, "Unhandled variable 0x{:X} (no available industry) in callback 0x{:x}", variable, callback);
			else if (tile) Debug(grf, 1, "Unhandled industry tile variable 0x{:X}", variable);
			else Debug(grf, 1, "Unhandled industry variable 0x{:X}", variable);
		},
		.pointer_magic = []() noexcept { Debug(grf, 0, "NewGRFs shouldn't be doing pointer magic"); },
		.base_scope = RustResolverBaseScope,
		.existing_town = [](ResolverObject *r) noexcept -> TownScopeResolver * { auto &town = static_cast<IndustriesResolverObject *>(r)->town_scope; return town.has_value() ? &*town : nullptr; },
		.make_town = [](ResolverObject *r, Town *t, bool readonly) noexcept -> TownScopeResolver * { return &static_cast<IndustriesResolverObject *>(r)->town_scope.emplace(*r, t, readonly); },
		.callback = [](uint16_t cb, uint32_t p1, uint32_t p2, Industry *i, uint8_t type, uint32_t tile) noexcept { return GetIndustryCallback(static_cast<CallbackID>(cb), p1, p2, i, type, TileIndex(tile)); },
		.callback_error = [](const IndustrySpec *spec, uint16_t cb, uint16_t result) noexcept { RustSpecCallbackError<IndustrySpec, IndustrySpec>(spec, cb, result); },
	};
	return services;
}
