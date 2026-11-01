/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_generic.cpp Handling of generic feature callbacks. */

#include "stdafx.h"
#include "debug.h"
#include "newgrf_spritegroup.h"
#include "industrytype.h"
#include "core/random_func.hpp"
#include "newgrf_sound.h"
#include "water_map.h"

#include "rust/generic_resolver.h"
#include "rust/services_newgrf.h"
#include "rust/services_cargo.h"
#include "rust/services_landscape.h"
#include "rust/services_sound.h"
#include "rust/services_water.h"

#include "safeguards.h"

static_assert(GSF_SOUNDFX == 12 && GSF_END == 22);
static_assert(CBID_GENERIC_AI_PURCHASE_SELECTION == 0x18 && CBID_SOUNDS_AMBIENT_EFFECT == 0x144);
static_assert(IT_AI_UNKNOWN == 0xFE && IT_AI_TOWN == 0xFF);
static uint16_t EvaluateGeneric(const OpenTTDGenericInvocation *invocation, int32_t *regs, size_t count) noexcept
{
	GenericResolverObject object(*invocation);
	// This leaf constructs the required native virtual/profiler frame only.
	// Evaluation and all generic dispatch return directly to their Rust owners.
	return openttd_rust_generic_evaluate(invocation, &object, object.CommonState(), regs, count);
}
const OpenTTDGenericServices &GetRustGenericServices() noexcept
{
	static const OpenTTDGenericServices services{
		.shared = &GetRustSharedServices(),
		.cargo_bitnum = RustCargoBitnum,
		.industry_substitute = RustIndustrySubstitute,
		.evaluate = EvaluateGeneric,
		.common = &GetRustSpriteGroupServices(),
		.enabled = RustAmbientSoundEnabled,
		.ambient = OpenTTDAmbientSound,
		.water_class = RustWaterClass,
		.sound = RustGRFPlayTileSound,
		.unsupported = [](uint8_t feature) noexcept { GrfMsg(5, "AddGenericCallback: Unsupported feature 0x{:02X}", feature); },
		.unhandled = [](uint8_t variable) noexcept { Debug(grf, 1, "Unhandled generic feature variable 0x{:02X}", variable); },
	};
	return services;
}
void AddGenericCallback(GrfSpecFeature feature, const GRFFile *file, const SpriteGroup *group) { openttd_rust_generic_add(feature, file, group); }
std::pair<const GRFFile *, uint16_t> GetAiPurchaseCallbackResult(GrfSpecFeature feature, CargoType cargo, uint8_t selection, IndustryType source, IndustryType destination, uint8_t distance, AIConstructionEvent event, uint8_t count, uint8_t station_size)
{
	auto result = openttd_rust_generic_ai(feature, cargo, selection, source, destination, distance, event, count, station_size);
	return {result.file, result.result};
}

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDGenericServices.hpp"
bool CheckAbi_generic() { return AbiServicesComplete(GetRustGenericServices()); }
