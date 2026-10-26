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
		.get = RustHouseSpec,
		.count = []() noexcept { return HouseSpec::Specs().size(); },
		.clear_specs = []() noexcept { HouseSpec::Specs().clear(); },
		.reserve_specs = [](size_t count) noexcept { HouseSpec::Specs().reserve(count); },
		.append_default = [](uint16_t id) noexcept { HouseSpec::Specs().push_back(_original_house_specs[id]); },
		.file_count = []() noexcept { return GetAllGRFFiles().size(); },
		.file_at = [](size_t index) noexcept { return const_cast<GRFFile *>(&GetAllGRFFiles()[index]); },
		.local_count = [](const GRFFile *file) noexcept { return file->housespec.size(); },
		.has_filename = [](const GRFFile *file) noexcept { return !file->filename.empty(); },
		.resize_local = [](GRFFile *file, size_t size) noexcept { file->housespec.resize(size); },
		.local = [](const GRFFile *file, size_t index) noexcept { return file->housespec[index].get(); },
		.clear_local = [](GRFFile *file) noexcept { file->housespec.clear(); file->housespec.shrink_to_fit(); },
		.copy_substitute = [](GRFFile *file, uint16_t id, const HouseSpec *source) noexcept { auto &spec = file->housespec[id]; spec = std::make_unique<HouseSpec>(*source); return spec.get(); },
		.set_identity = [](const HouseSpec *spec, GRFFile *file, uint16_t local, uint16_t substitute) noexcept { auto &props = MutableHouse(spec)->grf_prop; props.local_id = local; props.subst_id = substitute; props.SetGRFFile(file); },
		.identity = [](const HouseSpec *spec) noexcept -> OpenTTDHouseDefinitionIdentity { return {.grfid = RustSpecGRFID(spec), .local = RustSpecLocalID(spec), .substitute = spec->grf_prop.subst_id}; },
		.validity = [](const HouseSpec *spec) noexcept -> OpenTTDHouseDefinitionValidity { return {.availability = spec->building_availability.base(), .substitute = spec->grf_prop.subst_id, .flags = spec->building_flags.base(), .population = spec->population, .enabled = spec->enabled}; },
		.early = [](const HouseSpec *spec) noexcept -> OpenTTDHouseDefinitionEarly { return {.year = spec->min_year.base(), .availability = spec->building_availability.base(), .enabled = spec->enabled}; },
		.cargo = [](const HouseSpec *spec, uint8_t index) noexcept -> OpenTTDHouseDefinitionCargo { return {.label = spec->accepts_cargo_label[index], .cargo = spec->accepts_cargo[index]}; },
		.set_cargo = [](const HouseSpec *spec, uint8_t index, CargoType cargo) noexcept { MutableHouse(spec)->accepts_cargo[index] = cargo; },
		.set_label = [](const HouseSpec *spec, uint8_t index, uint32_t label) noexcept { MutableHouse(spec)->accepts_cargo_label[index] = label; },
		.set_acceptance = [](const HouseSpec *spec, uint8_t index, uint8_t amount) noexcept { MutableHouse(spec)->cargo_acceptance[index] = amount; },
		.set_colour = [](const HouseSpec *spec, uint8_t index, uint8_t colour) noexcept { MutableHouse(spec)->random_colour[index] = static_cast<Colours>(colour); },
		.callbacks = [](const HouseSpec *spec) noexcept { return spec->callback_mask.base(); },
		.watched = [](const HouseSpec *spec) noexcept { return spec->watched_cargoes; },
		.map_string = [](const HouseSpec *spec, uint16_t id) noexcept { AddStringForMapping(GRFStringID{id}, &MutableHouse(spec)->building_name); },
		.set_animation = [](const HouseSpec *spec, uint8_t frames, AnimationStatus status) noexcept { MutableHouse(spec)->animation.frames = frames; MutableHouse(spec)->animation.status = status; },
		.badges = [](const HouseSpec *spec) noexcept { return &MutableHouse(spec)->badges; },
		.file_id = RustGRFFileID,
		.add_override = [](uint16_t id, uint32_t grfid, uint16_t target) noexcept { _house_mngr.Add(id, grfid, target); },
		.add_entity = [](uint16_t id, uint32_t grfid, uint16_t substitute) noexcept { return _house_mngr.AddEntityID(id, grfid, substitute); },
		.pending_override = [](uint16_t id) noexcept -> OpenTTDHouseDefinitionOverride { return {.grfid = _house_mngr.grfid_overrides[id], .local = _house_mngr.entity_overrides[id]}; },
		.clear_override = [](uint16_t id) noexcept { _house_mngr.entity_overrides[id] = INVALID_HOUSE_ID; _house_mngr.grfid_overrides[id] = 0; },
		.reset_override = []() noexcept { _house_mngr.ResetOverride(); },
		.resize_specs = [](size_t count) noexcept { HouseSpec::Specs().resize(count); },
		.move_spec = [](const HouseSpec *spec, uint16_t id) noexcept { HouseSpec::Specs()[id] = std::move(*MutableHouse(spec)); },
		.set_override = [](const HouseSpec *spec, uint16_t id) noexcept { MutableHouse(spec)->grf_prop.override_id = id; },
		.climate_mask = []() noexcept { return GetClimateMaskForLandscape().base(); },
		.landscape = OpenTTDLandscape,
		.too_many = [](uint32_t last) noexcept { GrfMsg(1, "TownHouseChangeInfo: Too many houses loaded ({}), max ({}). Ignoring.", last, NUM_HOUSES_PER_GRF); },
		.invalid_substitute = [](uint8_t substitute, uint32_t id) noexcept { GrfMsg(2, "TownHouseChangeInfo: Attempt to use new house {} as substitute house for {}. Ignoring.", substitute, id); },
		.invalid_override = [](uint8_t target, uint32_t id) noexcept { GrfMsg(2, "TownHouseChangeInfo: Attempt to override new house {} with house id {}. Ignoring.", target, id); },
		.allocation_failed = []() noexcept { GrfMsg(1, "House.SetEntitySpec: Too many houses allocated. Ignoring."); },
		.disable_list = [](int prop) noexcept { DisableGrf(STR_NEWGRF_ERROR_LIST_PROPERTY_TOO_LONG)->param_value[1] = prop; },
		.invalid_missing = [](const GRFFile *file, const HouseSpec *spec) noexcept { Debug(grf, 1, "FinaliseHouseArray: {} defines house {} as multitile, but no suitable tiles follow. Disabling house.", file->filename, spec->grf_prop.local_id); },
		.invalid_population = [](const GRFFile *file, const HouseSpec *spec) noexcept { Debug(grf, 1, "FinaliseHouseArray: {} defines house {} as multitile with non-zero population on additional tiles. Disabling house.", file->filename, spec->grf_prop.local_id); },
		.invalid_size = [](const GRFFile *file, const HouseSpec *spec) noexcept { Debug(grf, 1, "FinaliseHouseArray: {} defines house {} with different house size then it's substitute type. Disabling house.", file->filename, spec->grf_prop.local_id); },
		.invalid_availability = [](const GRFFile *file, const HouseSpec *spec) noexcept { Debug(grf, 1, "FinaliseHouseArray: {} defines house {} without a size but marked it as available. Disabling house.", file->filename, spec->grf_prop.local_id); },
		.set_enabled = [](const HouseSpec *spec, bool value) noexcept { MutableHouse(spec)->enabled = value; },
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
		.set_animation_speed = [](const HouseSpec *spec, uint8_t value) noexcept { MutableHouse(spec)->animation.speed = value; },
		.set_class_id = [](const HouseSpec *spec, uint16_t value) noexcept { MutableHouse(spec)->class_id = value; },
		.set_minimum_life = [](const HouseSpec *spec, uint8_t value) noexcept { MutableHouse(spec)->minimum_life = value; },
		.set_watched = [](const HouseSpec *spec, uint64_t value) noexcept { MutableHouse(spec)->watched_cargoes = value; },
	};
	return services;
}
#endif
