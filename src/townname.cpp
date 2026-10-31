/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file townname.cpp %Town name generators. */

#include "stdafx.h"
#include "string_func.h"
#include "townname_type.h"
#include "town.h"
#include "strings_func.h"
#include "core/random_func.hpp"
#include "genworld.h"
#include "gfx_layout.h"
#include "strings_internal.h"

#include "rust/townname_ffi.h"
#include "rust/town_name_ffi.h"
#include "rust/presentation_services.h"

#include "safeguards.h"


/**
 * Initializes this struct from town data
 * @param t town for which we will be printing name later
 */
TownNameParams::TownNameParams(const Town *t)
{
	openttd_rust_town_name_params_for_town(this, t->growth_owner.get());
}


/**
 * Fills builder with specified town name.
 * @param builder       The string builder.
 * @param par           Town name parameters.
 * @param townnameparts 'Encoded' town name.
 */
static void GetTownName(StringBuilder &builder, const TownNameParams *par, uint32_t townnameparts)
{
	if (par->grfid == 0) {
		auto tmp_params = MakeParameters(townnameparts);
		GetStringWithArgs(builder, par->type, tmp_params);
		return;
	}

	GRFTownNameGenerate(builder, par->grfid, par->type, townnameparts);
}

/**
 * Get the town name for the given parameters and parts.
 * @param par Town name parameters.
 * @param townnameparts 'Encoded' town name.
 * @return The town name.
 */
std::string GetTownName(const TownNameParams *par, uint32_t townnameparts)
{
	std::string result;
	StringBuilder builder(result);
	GetTownName(builder, par, townnameparts);
	return result;
}

/**
 * Fills builder with town's name.
 * @param builder String builder.
 * @param t       The town to get the name from.
 */
void GetTownName(StringBuilder &builder, const Town *t)
{
	TownNameParams par(t);
	GetTownName(builder, &par, t->townnameparts);
}

/**
 * Get the name of the given town.
 * @param t The town to get the name for.
 * @return The town name.
 */
std::string GetTownName(const Town *t)
{
	TownNameParams par(t);
	return GetTownName(&par, t->townnameparts);
}


TownNames::TownNames() : owner(openttd_rust_town_names_new()) {}
TownNames::~TownNames() { openttd_rust_town_names_destroy(this->owner); }
void TownNames::clear() { openttd_rust_town_names_clear(this->owner); }
void TownNames::insert(std::string_view name) { openttd_rust_town_names_insert(this->owner, {reinterpret_cast<const uint8_t *>(name.data()), name.size()}); }
const OpenTTDTownNameServices &GetRustTownNameServices() noexcept
{
	static const OpenTTDTownNameServices services{
		.next = [](uint from) noexcept -> OpenTTDTownRef {
			for (Town *town : Town::Iterate(from)) return {.shell = town, .state = town->growth_owner.get(), .id = town->index.base()};
			return {.shell = nullptr, .state = nullptr, .id = TownID::Invalid().base()};
		},
		.render = [](OpenTTDTownNameFrame *frame, const TownNameParams *params, uint32_t seed, bool other) noexcept -> OpenTTDByteSpan {
			auto &text = other ? frame->other : frame->candidate;
			text = GetTownName(params, seed);
			return {reinterpret_cast<const uint8_t *>(text.data()), text.size()};
		},
		.shared = &GetRustSharedServices(),
		.utf8_length = GetRustSharedServices().utf8_length,
		.random_next = [](Randomizer *randomizer) noexcept -> uint32_t { return randomizer->Next(); },
	};
	return services;
}

/**
 * Verifies the town name is valid and unique.
 * @param r random bits
 * @param par town name parameters
 * @param town_names if a name is generated, check its uniqueness with the set
 * @return true iff name is valid and unique
 */
bool VerifyTownName(uint32_t r, const TownNameParams *par, TownNames *town_names)
{
	OpenTTDTownNameFrame frame;
	return openttd_rust_town_verify_name(&GetRustTownNameServices(), &frame, r, par, town_names == nullptr ? nullptr : town_names->owner);
}


/**
 * Generates valid town name.
 * @param randomizer the source of random data for generating the name
 * @param townnameparts if a name is generated, it's stored there
 * @param town_names if a name is generated, check its uniqueness with the set
 * @return true iff a name was generated
 */
bool GenerateTownName(Randomizer &randomizer, uint32_t *townnameparts, TownNames *town_names)
{
	TownNameParams par(_settings_game.game_creation.town_name);
	OpenTTDTownNameFrame frame;
	return openttd_rust_town_generate_name(&GetRustTownNameServices(), &frame, &randomizer, townnameparts, &par, town_names == nullptr ? nullptr : town_names->owner);
}




/**
 * Generates town name from given seed.
 * @param builder string builder to write to
 * @param lang    town name language
 * @param seed    generation seed
 */
void GenerateTownNameString(StringBuilder &builder, size_t lang, uint32_t seed)
{
	static_assert(BUILTIN_TOWNNAME_GENERATOR_COUNT == 21);
	assert(lang < BUILTIN_TOWNNAME_GENERATOR_COUNT);
	/* Append outside Rust; RAII also frees the result if the C++ allocation throws. */
	auto result = std::unique_ptr<OpenTTDTownNameResult, decltype(&openttd_rust_townname_destroy)>(openttd_rust_townname_generate(lang, seed), openttd_rust_townname_destroy);
	size_t length;
	const char *data = openttd_rust_townname_data(result.get(), &length);
	builder.Put(std::string_view(data, length));
}

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDTownNameServices.hpp"
bool CheckAbi_townname() { return AbiServicesComplete(GetRustTownNameServices()); }
