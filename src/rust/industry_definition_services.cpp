/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file industry_definition_services.cpp Native industry definition storage and defaults. */
#include "../stdafx.h"
#include "../industrytype.h"
#include "../newgrf.h"
#include "../debug.h"
#include "../sound_type.h"
#include "../newgrf_cargo.h"
#include "../newgrf_config.h"
#include "../newgrf/newgrf_internal.h"
#include "industry_definition_ffi.h"
#include "industry_ffi.h"
#include "services_newgrf.h"
#include "services_cargo.h"
#include "../table/build_industry.h"
#include "../table/strings.h"
#include "../safeguards.h"

template <typename Slots, size_t N>
static Slots DefinitionCargo(uint8_t *cargo, const std::array<std::variant<CargoLabel, MixedCargoType>, N> &labels) noexcept
{
	Slots slots{.cargo = cargo, .labels = {}};
	for (size_t i = 0; i < std::size(slots.labels); ++i) {
		if (!IsValidCargoType(cargo[i])) slots.labels[i] = RustCargoLabel(labels[i]);
	}
	return slots;
}
const OpenTTDIndustryDefinitionServices &GetIndustryDefinitionServices() noexcept
{
	static const OpenTTDIndustryDefinitionServices services{
		.generation = &GetIndustryGenerationServices(),
		.shared = &GetRustSharedServices(),
		.industry = [](uint16_t id) noexcept { return &_industry_specs[id]; },
		.tile = [](uint16_t id) noexcept { return &_industry_tile_specs[id]; },
		.availability = [](const IndustrySpec *spec) noexcept { return OpenTTDIndustryDefinitionAvailability{RustSpecEnabled<IndustrySpec>(spec), RustSpecHasGRFFile(spec)}; },
		.publish = [](uint16_t id, IndustrySpec *spec) noexcept { _industry_specs[id] = std::move(*spec); },
		.publish_tile = [](uint16_t id, IndustryTileSpec *spec) noexcept { _industry_tile_specs[id] = std::move(*spec); },
		.industry_full = []() noexcept { GrfMsg(1, "Industry.SetEntitySpec: Too many industries allocated. Ignoring."); },
		.tile_full = []() noexcept { GrfMsg(1, "IndustryTile.SetEntitySpec: Too many industry tiles allocated. Ignoring."); },
		.reset_industry = [](uint16_t id) noexcept { _industry_specs[id] = _origin_industry_specs[id]; },
		.clear_industry = [](uint16_t id) noexcept { _industry_specs[id] = IndustrySpec{}; },
		.climate_available = [](const IndustrySpec *spec, uint8_t climate) noexcept { return spec->climate_availability.Test(static_cast<LandscapeType>(climate)); },
		.reset_tile = [](uint16_t id) noexcept { _industry_tile_specs[id] = _origin_industry_tile_specs[id]; },
		.clear_tile = [](uint16_t id) noexcept { _industry_tile_specs[id] = IndustryTileSpec{}; },
		.industry_count = [](const GRFFile *file) noexcept { return file->industryspec.size(); },
		.tile_count = [](const GRFFile *file) noexcept { return file->indtspec.size(); },
		.file_industry = [](GRFFile *file, size_t index) noexcept { return file->industryspec[index].get(); },
		.file_tile = [](GRFFile *file, size_t index) noexcept { return file->indtspec[index].get(); },
		.clear_file = [](GRFFile *file) noexcept { file->industryspec.clear(); file->industryspec.shrink_to_fit(); file->indtspec.clear(); file->indtspec.shrink_to_fit(); },
		.conflicting = [](IndustrySpec *spec) noexcept { return spec->conflicting; },
		.invalid_name = [](IndustrySpec *spec) noexcept { spec->name = STR_NEWGRF_INVALID_INDUSTRYTYPE; },
		.cargo = [](IndustrySpec *spec) noexcept { return OpenTTDIndustryDefinitionCargo{
			.produced = DefinitionCargo<OpenTTDIndustryProducedSlots>(spec->produced_cargo.data(), spec->produced_cargo_label),
			.accepted = DefinitionCargo<OpenTTDIndustryAcceptedSlots>(spec->accepts_cargo.data(), spec->accepts_cargo_label),
		}; },
		.tile_cargo = [](IndustryTileSpec *spec) noexcept { return DefinitionCargo<OpenTTDIndustryAcceptedSlots>(spec->accepts_cargo.data(), spec->accepts_cargo_label); },
		.set_produced = [](IndustrySpec *spec, size_t slot, uint8_t cargo) noexcept { spec->produced_cargo[slot] = cargo; },
		.set_accepted = [](IndustrySpec *spec, size_t slot, uint8_t cargo) noexcept { spec->accepts_cargo[slot] = cargo; },
		.set_tile_accepted = [](IndustryTileSpec *spec, size_t slot, uint8_t cargo) noexcept { spec->accepts_cargo[slot] = cargo; },
	};
	return services;
}
