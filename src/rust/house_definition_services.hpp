/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file house_definition_services.hpp Native shell operations for serialized GRF loading. */
#ifndef RUST_HOUSE_DEFINITION_SERVICES_HPP
#define RUST_HOUSE_DEFINITION_SERVICES_HPP
#include "house_definition_ffi.h"
#include "services_newgrf.h"
#include "environment_services.h"
#include "../newgrf/newgrf_stringmapping.h"
#include "../table/strings.h"
extern const HouseSpec _original_house_specs[NEW_HOUSE_OFFSET];
/* HouseSpec shells are mutable during serialized definition loading; no shell
 * pointer survives vector growth or reset. Native field references end per leaf. */
inline HouseSpec *MutableHouse(const HouseSpec *spec) noexcept { return const_cast<HouseSpec *>(spec); }
inline const OpenTTDHouseDefinitionServices &GetHouseDefinitionServices() noexcept
{
	static const OpenTTDHouseDefinitionServices services{
		.resolver = &GetHouseResolverServices(),
		.get = RustHouseSpec,
		.count = RustHouseSpecCount,
		.clear_specs = []() noexcept { HouseSpec::Specs().clear(); },
		.reserve_specs = [](size_t count) noexcept { HouseSpec::Specs().reserve(count); },
		.append_default = [](uint16_t id) noexcept { HouseSpec::Specs().push_back(_original_house_specs[id]); },
		.file_count = RustGRFFileCount,
		.file_at = RustGRFFileAt,
		.local_count = [](const GRFFile *file) noexcept { return file->housespec.size(); },
		.has_filename = [](const GRFFile *file) noexcept { return !file->filename.empty(); },
		.resize_local = [](GRFFile *file, size_t size) noexcept { file->housespec.resize(size); },
		.local = [](const GRFFile *file, size_t index) noexcept -> const HouseSpec * { return file->housespec[index].get(); },
		.clear_local = [](GRFFile *file) noexcept { file->housespec.clear(); file->housespec.shrink_to_fit(); },
		.copy_substitute = [](GRFFile *file, uint16_t id, const HouseSpec *source) noexcept -> const HouseSpec * { auto &spec = file->housespec[id]; spec = std::make_unique<HouseSpec>(*source); return spec.get(); },
		.set_identity = [](const HouseSpec *spec, GRFFile *file, uint16_t local, uint16_t substitute) noexcept { RustSpecSetLocalID(MutableHouse(spec), local); RustSpecSetSubstitute(MutableHouse(spec), substitute); RustSpecSetGRFFile(MutableHouse(spec), file); },
		.identity = [](const HouseSpec *spec) noexcept -> OpenTTDHouseDefinitionIdentity { auto identity = RustSpecSubstituteIdentity(spec); return {.grfid = identity.grfid, .local = identity.local, .substitute = identity.substitute}; },
		.validity = [](const HouseSpec *spec) noexcept -> OpenTTDHouseDefinitionValidity { return {.availability = RustHouseAvailability(spec), .substitute = RustSpecSubstitute(spec), .flags = RustTownHouseFlags(spec)}; },
		.part = [](const HouseSpec *spec) noexcept -> OpenTTDHouseDefinitionPart { return {.flags = RustTownHouseFlags(spec), .population = RustTownHousePopulation(spec), .enabled = RustSpecEnabled<HouseSpec>(spec)}; },
		.flags = RustTownHouseFlags,
		.early = [](const HouseSpec *spec) noexcept -> OpenTTDHouseDefinitionEarly { return {.year = RustHouseMinYear(spec), .availability = RustHouseAvailability(spec), .enabled = RustSpecEnabled<HouseSpec>(spec)}; },
		.cargo = [](const HouseSpec *spec, uint8_t index) noexcept -> OpenTTDHouseDefinitionCargo { return {.label = spec->accepts_cargo_label[index].base(), .cargo = spec->accepts_cargo[index]}; },
		.set_cargo = [](const HouseSpec *spec, uint8_t index, CargoType cargo) noexcept { MutableHouse(spec)->accepts_cargo[index] = cargo; },
		.set_label = [](const HouseSpec *spec, uint8_t index, uint32_t label) noexcept { MutableHouse(spec)->accepts_cargo_label[index] = CargoLabel{label}; },
		.set_acceptance = [](const HouseSpec *spec, uint8_t index, uint8_t amount) noexcept { MutableHouse(spec)->cargo_acceptance[index] = amount; },
		.set_colour = [](const HouseSpec *spec, uint8_t index, uint8_t colour) noexcept { MutableHouse(spec)->random_colour[index] = static_cast<Colours>(colour); },
		.callbacks = RustTownHouseCallbacks,
		.watched = RustHouseWatched,
		.map_string = [](const HouseSpec *spec, uint16_t id) noexcept { AddStringForMapping(GRFStringID{id}, &MutableHouse(spec)->building_name); },
		.set_animation = [](const HouseSpec *spec, uint8_t frames, AnimationStatus status) noexcept { RustSpecAnimationFrames(MutableHouse(spec), frames); RustSpecAnimationStatus(MutableHouse(spec), static_cast<uint8_t>(status)); },
		.badges = [](const HouseSpec *spec) noexcept { return RustSpecBadgeList(MutableHouse(spec)); },
		.file_id = RustGRFFileID,
		.resize_specs = [](size_t count) noexcept { HouseSpec::Specs().resize(count); },
		.move_spec = [](const HouseSpec *spec, uint16_t id) noexcept { HouseSpec::Specs()[id] = std::move(*MutableHouse(spec)); },
		.set_override = [](const HouseSpec *spec, uint16_t id) noexcept { RustSpecSetOverride(MutableHouse(spec), id); },
		.landscape = OpenTTDLandscape,
		.too_many = [](uint32_t last) noexcept { GrfMsg(1, "TownHouseChangeInfo: Too many houses loaded ({}), max ({}). Ignoring.", last, NUM_HOUSES_PER_GRF); },
		.invalid_substitute = [](uint8_t substitute, uint32_t id) noexcept { GrfMsg(2, "TownHouseChangeInfo: Attempt to use new house {} as substitute house for {}. Ignoring.", substitute, id); },
		.invalid_override = [](uint8_t target, uint32_t id) noexcept { GrfMsg(2, "TownHouseChangeInfo: Attempt to override new house {} with house id {}. Ignoring.", target, id); },
		.allocation_failed = []() noexcept { GrfMsg(1, "House.SetEntitySpec: Too many houses allocated. Ignoring."); },
		.invalid_missing = [](const GRFFile *file, const HouseSpec *spec) noexcept { Debug(grf, 1, "FinaliseHouseArray: {} defines house {} as multitile, but no suitable tiles follow. Disabling house.", file->filename, spec->grf_prop.local_id); },
		.invalid_population = [](const GRFFile *file, const HouseSpec *spec) noexcept { Debug(grf, 1, "FinaliseHouseArray: {} defines multitile house {} with non-zero population on additional tiles. Disabling house.", file->filename, spec->grf_prop.local_id); },
		.invalid_size = [](const GRFFile *file, const HouseSpec *spec) noexcept { Debug(grf, 1, "FinaliseHouseArray: {} defines house {} with different house size then it's substitute type. Disabling house.", file->filename, spec->grf_prop.local_id); },
		.invalid_availability = [](const GRFFile *file, const HouseSpec *spec) noexcept { Debug(grf, 1, "FinaliseHouseArray: {} defines house {} without a size but marked it as available. Disabling house.", file->filename, spec->grf_prop.local_id); },
		.set_enabled = [](const HouseSpec *spec, bool value) noexcept { RustSpecSetEnabled(MutableHouse(spec), value); },
		.set_flags = [](const HouseSpec *spec, uint8_t value) noexcept { MutableHouse(spec)->building_flags = BuildingFlags{value}; },
		.set_min_year = [](const HouseSpec *spec, int32_t value) noexcept { MutableHouse(spec)->min_year = TimerGameCalendar::Year{value}; },
		.set_max_year = [](const HouseSpec *spec, int32_t value) noexcept { MutableHouse(spec)->max_year = TimerGameCalendar::Year{value}; },
		.set_population = [](const HouseSpec *spec, uint8_t value) noexcept { MutableHouse(spec)->population = value; },
		.set_mail = [](const HouseSpec *spec, uint8_t value) noexcept { MutableHouse(spec)->mail_generation = value; },
		.set_rating = [](const HouseSpec *spec, uint16_t value) noexcept { MutableHouse(spec)->remove_rating_decrease = value; },
		.set_removal = [](const HouseSpec *spec, uint8_t value) noexcept { MutableHouse(spec)->removal_cost = value; },
		.set_availability = [](const HouseSpec *spec, uint16_t value) noexcept { MutableHouse(spec)->building_availability = HouseZones{value}; },
		.set_callbacks = [](const HouseSpec *spec, uint16_t value) noexcept { MutableHouse(spec)->callback_mask = HouseCallbackMasks{value}; },
		.set_processing = [](const HouseSpec *spec, uint8_t value) noexcept { MutableHouse(spec)->processing_time = value; },
		.set_probability = [](const HouseSpec *spec, uint8_t value) noexcept { MutableHouse(spec)->probability = value; },
		.set_extra = [](const HouseSpec *spec, uint8_t value) noexcept { MutableHouse(spec)->extra_flags = HouseExtraFlags{value}; },
		.set_animation_speed = [](const HouseSpec *spec, uint8_t value) noexcept { RustSpecAnimationSpeed(MutableHouse(spec), value); },
		.set_class_id = [](const HouseSpec *spec, uint16_t value) noexcept { MutableHouse(spec)->class_id = value; },
		.set_minimum_life = [](const HouseSpec *spec, uint8_t value) noexcept { MutableHouse(spec)->minimum_life = value; },
		.set_watched = [](const HouseSpec *spec, uint64_t value) noexcept { MutableHouse(spec)->watched_cargoes = value; },
	};
	return services;
}
#endif
