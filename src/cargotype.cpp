/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file cargotype.cpp Implementation of cargoes. */

#include "stdafx.h"
#include "cargotype.h"
#include "core/geometry_func.hpp"
#include "newgrf_cargo.h"
#include "string_func.h"
#include "strings_func.h"
#include "settings_type.h"

#include "table/sprites.h"
#include "table/strings.h"
#include "table/cargo_const.h"

#include "rust/cargo_registry_ffi.h"
#include "rust/services_cargo.h"
#include "newgrf.h"
#include "newgrf_spritegroup.h"
#include "rust/newgrf_spritegroup_ffi.h"

#include "safeguards.h"

CargoSpec CargoSpec::array[NUM_CARGO];
std::array<std::span<const CargoSpec *>, NUM_TPE> CargoSpec::town_production_cargoes{};
CargoTypes &_cargo_mask = *openttd_rust_cargo_mask();
CargoTypes &_standard_cargo_mask = *openttd_rust_standard_cargo_mask();

/**
 * Set up the default cargo types for the given landscape type.
 * @param l Landscape
 */
void SetupCargoForClimate(LandscapeType l)
{
	openttd_rust_cargo_setup(&GetRustCargoRegistryServices(), to_underlying(l));
}

/**
 * Get default climate-dependent cargo translation table for a NewGRF, used if the NewGRF does not provide its own.
 * @return Default translation table for GRF version.
 */
std::span<const CargoLabel> GetClimateDependentCargoTranslationTable()
{
	auto labels = openttd_rust_cargo_climate_labels();
	return {reinterpret_cast<const CargoLabel *>(labels.data), labels.size};
}

/**
 * Get default climate-independent cargo translation table for a NewGRF, used if the NewGRF does not provide its own.
 * @return Default translation table for GRF version.
 */
std::span<const CargoLabel> GetClimateIndependentCargoTranslationTable()
{
	auto labels = openttd_rust_cargo_bitnum_labels();
	return {reinterpret_cast<const CargoLabel *>(labels.data), labels.size};
}

/**
 * Build cargo label map.
 * This is called multiple times during NewGRF initialization as cargos are defined, so that TranslateRefitMask() and
 * GetCargoTranslation(), also used during initialization, get the correct information.
 */
void BuildCargoLabelMap()
{
	openttd_rust_cargo_labels_rebuild();
}

/**
 * Test if a cargo is a default cargo type.
 * @param cargo_type Cargo type.
 * @returns true iff the cargo type is a default cargo type.
 */
bool IsDefaultCargo(CargoType cargo_type)
{
	return openttd_rust_cargo_is_default(cargo_type);
}

/**
 * Get dimensions of largest cargo icon.
 * @return Dimensions of largest cargo icon.
 */
Dimension GetLargestCargoIconSize()
{
	Dimension size = {0, 0};
	for (const CargoSpec *cs : _sorted_cargo_specs) {
		size = maxdim(size, GetSpriteSize(cs->GetCargoIcon()));
	}
	return size;
}

/**
 * Get sprite for showing cargo of this type.
 * @return Sprite number to use.
 */
SpriteID CargoSpec::GetCargoIcon() const
{
	SpriteID sprite = this->sprite;
	if (sprite == 0xFFFF) {
		/* A value of 0xFFFF indicates we should draw a custom icon */
		sprite = GetCustomCargoSprite(this);
	}

	if (sprite == 0) sprite = SPR_CARGO_GOODS;

	return sprite;
}

std::span<uint8_t, NUM_CARGO> _sorted_cargo_types{openttd_rust_cargo_reverse(), NUM_CARGO};
std::span<const CargoSpec *> _sorted_cargo_specs;
std::span<const CargoSpec *> _sorted_standard_cargo_specs; ///< Standard cargo specifications sorted alphabetically by name.

/** Sort cargo specifications by their name. */
static bool CargoSpecNameSorter(const CargoSpec * const &a, const CargoSpec * const &b)
{
	std::string a_name = GetString(a->name);
	std::string b_name = GetString(b->name);

	int res = StrNaturalCompare(a_name, b_name); // Sort by name (natural sorting).

	/* If the names are equal, sort by cargo bitnum. */
	return (res != 0) ? res < 0 : (a->bitnum < b->bitnum);
}

/** Sort cargo specifications by their cargo class. */
static bool CargoSpecClassSorter(const CargoSpec * const &a, const CargoSpec * const &b)
{
	int res = b->classes.Test(CargoClass::Passengers) - a->classes.Test(CargoClass::Passengers);
	if (res == 0) {
		res = b->classes.Test(CargoClass::Mail) - a->classes.Test(CargoClass::Mail);
		if (res == 0) {
			res = a->classes.Test(CargoClass::Special) - b->classes.Test(CargoClass::Special);
			if (res == 0) {
				return CargoSpecNameSorter(a, b);
			}
		}
	}

	return res < 0;
}

/** Initialize the list of sorted cargo specifications. */
void InitializeSortedCargoSpecs()
{
	openttd_rust_cargo_sort(&GetRustCargoRegistryServices());
}

uint64_t CargoSpec::WeightOfNUnitsInTrain(uint32_t n) const
{
	if (this->is_freight) n *= _settings_game.vehicle.freight_trains;
	return this->WeightOfNUnits(n);
}

/**
 * Build comma-separated cargo acceptance string.
 * @param acceptance CargoArray filled with accepted cargo.
 * @param label Label to prefix cargo acceptance list.
 * @return String of accepted cargo, or nullopt if no cargo is accepted.
 */
std::optional<std::string> BuildCargoAcceptanceString(const CargoArray &acceptance, StringID label)
{
	std::string_view list_separator = GetListSeparator();

	std::stringstream line;

	bool found = false;
	for (const CargoSpec *cs : _sorted_cargo_specs) {
		CargoType cargo_type = cs->Index();
		if (acceptance[cargo_type] > 0) {
			/* Add a comma between each item. */
			if (found) line << list_separator;
			found = true;

			/* If the accepted value is less than 8, show it in 1/8:ths */
			if (acceptance[cargo_type] < 8) {
				line << GetString(STR_LAND_AREA_INFORMATION_CARGO_EIGHTS, acceptance[cargo_type], cs->name);
			} else {
				line << GetString(cs->name);
			}
		}
	}

	if (found) return GetString(label, line.str());

	return std::nullopt;
}

extern uint16_t CargoCallbackFrame(const OpenTTDCargoInvocation *, int32_t *, size_t) noexcept;
extern uint32_t CargoSpriteFrame(const OpenTTDCargoInvocation *) noexcept;
const OpenTTDCargoRegistryServices &GetRustCargoRegistryServices() noexcept
{
	static_assert(sizeof(CargoLabel) == sizeof(uint32_t) && alignof(CargoLabel) == alignof(uint32_t));
	static_assert(INVALID_TPE == 4 && NUM_TPE == 3 && INVALID_CARGO_BITNUM == 255);
	static const OpenTTDCargoRegistryServices services{
		.shared = &GetRustSharedServices(),
		.common = &GetRustSpriteGroupServices(),
		.spec = RustCargoSpec,
		.copy_default = [](uint8_t climate, uint8_t slot) noexcept {
			auto *cs = RustCargoSpec(slot);
			std::visit([cs](const auto &entry) {
				if constexpr (std::is_same_v<std::decay_t<decltype(entry)>, int>) {
					*cs = _default_cargo[entry];
				} else {
					auto found = std::ranges::find(_default_cargo, entry, &CargoSpec::label);
					if (found == std::end(_default_cargo)) NOT_REACHED();
					*cs = *found;
				}
			}, _default_climate_cargo[climate][slot]);
		},
		.clear_specs = [](uint8_t first) noexcept { std::fill(RustCargoSpec(first), RustCargoSpec(0) + CargoSpec::GetArraySize(), CargoSpec{}); },
		.label_spec = [](const CargoSpec *cs) noexcept -> OpenTTDCargoLabelSpec { return {RustCargoSpecLabel(cs), RustCargoSpecBitnum(cs)}; },
		.bitnum = RustCargoSpecBitnum,
		.town_acceptance_effect = RustCargoTownAcceptanceEffect,
		.finalise_spec = [](const CargoSpec *cs) noexcept -> OpenTTDCargoFinaliseSpec { return {RustCargoSpecLabel(cs), RustCargoSpecBitnum(cs), cs->town_production_effect}; },
		.order_spec = [](const CargoSpec *cs) noexcept -> OpenTTDCargoOrderSpec { return {RustCargoSpecIndex(cs), cs->town_production_effect, cs->classes.Test(CargoClass::Special)}; },
		.index = RustCargoSpecIndex,
		.production = [](const CargoSpec *cs) noexcept -> OpenTTDCargoProduction { return {RustCargoSpecIndex(cs), cs->town_production_multiplier}; },
		.sort = [](const CargoSpec **data, size_t count) noexcept { std::sort(data, data + count, &CargoSpecClassSorter); },
		.publish_sorted = [](const CargoSpec **data, size_t count) noexcept { _sorted_cargo_specs = {data, count}; },
		.publish_standard = [](const CargoSpec **data, size_t count) noexcept { _sorted_standard_cargo_specs = {data, count}; },
		.publish_town = [](uint8_t effect, const CargoSpec **data, size_t count) noexcept { CargoSpec::town_production_cargoes[effect] = {data, count}; },
		.set_file = [](const CargoSpec *cs, const GRFFile *file) noexcept { const_cast<CargoSpec *>(cs)->grffile = file; },
		.set_effect = [](const CargoSpec *cs, uint8_t effect) noexcept { const_cast<CargoSpec *>(cs)->town_production_effect = static_cast<TownProductionEffect>(effect); },
		.invalidate_strings = [](const CargoSpec *spec) noexcept {
			auto cs = const_cast<CargoSpec *>(spec);
			cs->name = cs->name_single = cs->units_volume = STR_NEWGRF_INVALID_CARGO;
			cs->quantifier = STR_NEWGRF_INVALID_CARGO_QUANTITY;
			cs->abbrev = STR_NEWGRF_INVALID_CARGO_ABBREV;
		},
		.file = [](const GRFFile *file) noexcept -> OpenTTDCargoFile { return {{reinterpret_cast<const uint32_t *>(file->cargo_list.data()), file->cargo_list.size()}, RustGRFFileVersion(file), file->cargo_list_is_fallback}; },
		.assign_fallback = [](GRFFile *file, OpenTTDCargoLabels labels) noexcept { auto data = reinterpret_cast<const CargoLabel *>(labels.data); file->cargo_list.assign(data, data + labels.size); file->cargo_list_is_fallback = true; },
		.resize_fallback = [](GRFFile *file, size_t size) noexcept { file->cargo_list.resize(size, CT_INVALID); },
		.set_fallback = [](GRFFile *file, uint32_t id, uint32_t label) noexcept { file->cargo_list[id] = CargoLabel{label}; },
		.clear_inverse = [](GRFFile *file) noexcept { file->cargo_map.fill(UINT8_MAX); },
		.set_inverse = [](GRFFile *file, uint8_t cargo, uint8_t index) noexcept { file->cargo_map[cargo] = index; },
		.invocation = [](const CargoSpec *cs, OpenTTDCargoInvocation *out) noexcept { out->label = RustCargoSpecLabel(cs); out->file = cs->grffile; out->root = cs->group; },
		.callback_frame = CargoCallbackFrame,
		.sprite_frame = CargoSpriteFrame,
		.sprite = [](const SpriteGroup *group) noexcept -> OpenTTDCargoSprite { auto result = static_cast<const ResultSpriteGroup *>(group); return {result->sprite, result->num_sprites}; },
	};
	return services;
}

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDCargoRegistryServices.hpp"
bool CheckAbi_cargo_registry() { return AbiServicesComplete(GetRustCargoRegistryServices()); }
