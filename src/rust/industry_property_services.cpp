/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file industry_property_services.cpp Native spec, graph and string storage. */
#include "../stdafx.h"
#ifdef WITH_RUST
#include "../newgrf.h"
#include "../debug.h"
#include "../sound_type.h"
#include "../newgrf/newgrf_internal.h"
#include "../newgrf/newgrf_stringmapping.h"
#include "industry_property_ffi.h"
#include "services_cargo.h"
#include "../table/build_industry.h"
#include "../table/strings.h"
#include "../safeguards.h"

static_assert(NUM_INDUSTRYTYPES_PER_GRF == 128 && NUM_INDUSTRYTILES_PER_GRF == 255);
static_assert(NEW_INDUSTRYOFFSET == 37 && NEW_INDUSTRYTILEOFFSET == 175 && NUM_INDUSTRYTYPES == 240 && NUM_INDUSTRYTILES == 512);
static_assert(INDUSTRY_NUM_INPUTS == 16 && INDUSTRY_NUM_OUTPUTS == 16 && INDUSTRY_ORIGINAL_NUM_INPUTS == 3 && INDUSTRY_ORIGINAL_NUM_OUTPUTS == 2);
static_assert(CIR_SUCCESS == 0 && CIR_DISABLED == 1 && CIR_UNKNOWN == 3 && CIR_INVALID_ID == 4);
static_assert(GSF_INDUSTRYTILES == 9 && GSF_INDUSTRIES == 10);

const OpenTTDIndustryPropertyServices &GetIndustryPropertyServices() noexcept
{
	static const OpenTTDIndustryPropertyServices services{
		.resize_industries = [](GRFFile *file, size_t size) noexcept { file->industryspec.resize(size); },
		.resize_tiles = [](GRFFile *file, size_t size) noexcept { file->indtspec.resize(size); },
		.create_industry = [](GRFFile *file, uint16_t id, uint8_t substitute) noexcept -> IndustrySpec * { file->industryspec[id] = std::make_unique<IndustrySpec>(_origin_industry_specs[substitute]); return file->industryspec[id].get(); },
		.create_tile = [](GRFFile *file, uint16_t id, uint8_t substitute) noexcept -> IndustryTileSpec * { file->indtspec[id] = std::make_unique<IndustryTileSpec>(_industry_tile_specs[substitute]); return file->indtspec[id].get(); },
		.industry_identity = [](IndustrySpec *spec, GRFFile *file, uint16_t id, uint8_t substitute) noexcept { spec->grf_prop.local_id = id; spec->grf_prop.subst_id = substitute; spec->grf_prop.SetGRFFile(file); },
		.tile_identity = [](IndustryTileSpec *spec, GRFFile *file, uint16_t id, uint8_t substitute) noexcept { spec->grf_prop.local_id = id; spec->grf_prop.subst_id = substitute; spec->grf_prop.SetGRFFile(file); },
		.clear_placement_check = [](IndustrySpec *spec) noexcept { spec->check_proc = CHECK_NOTHING; },
		.clear_animation = [](IndustryTileSpec *spec) noexcept { spec->anim_production = INDUSTRYTILE_NOANIM; spec->anim_next = INDUSTRYTILE_NOANIM; },
		.callbacks = [](const IndustrySpec *spec) noexcept -> uint16_t { return spec->callback_mask.base(); },
		.set_rate = [](IndustrySpec *spec, size_t slot, uint8_t rate) noexcept { spec->production_rate[slot] = rate; },
		.set_multiplier = [](IndustrySpec *spec, size_t input, size_t output, uint16_t multiplier) noexcept { spec->input_cargo_multiplier[input][output] = multiplier; },
		.set_conflicting = [](IndustrySpec *spec, size_t slot, uint8_t id) noexcept { spec->conflicting[slot] = id; },
		.set_creation = [](IndustrySpec *spec, uint8_t climate, uint8_t chance) noexcept { spec->appear_creation[climate] = chance; },
		.set_ingame = [](IndustrySpec *spec, uint8_t climate, uint8_t chance) noexcept { spec->appear_ingame[climate] = chance; },
		.invalidate_produced = [](IndustrySpec *spec, size_t slot) noexcept { spec->produced_cargo_label[slot] = CT_INVALID; },
		.invalidate_accepted = [](IndustrySpec *spec, size_t slot) noexcept { spec->accepts_cargo_label[slot] = CT_INVALID; },
		.invalidate_tile_accepted = [](IndustryTileSpec *spec, size_t slot) noexcept { spec->accepts_cargo_label[slot] = CT_INVALID; },
		.set_acceptance = [](IndustryTileSpec *spec, size_t slot, int8_t amount) noexcept { spec->acceptance[slot] = amount; },
		.set_sounds = [](IndustrySpec *spec, const uint8_t *sounds, size_t count) noexcept { spec->random_sounds.assign(sounds, sounds + count); },
		.clear_station_name = [](IndustrySpec *spec) noexcept { spec->station_name = STR_NULL; },
		.set_life = [](IndustrySpec *spec, uint8_t value) noexcept { spec->life_type = IndustryLifeTypes{value}; },
		.set_cost = [](IndustrySpec *spec, uint8_t value) noexcept { spec->cost_multiplier = value; },
		.set_minimal_cargo = [](IndustrySpec *spec, uint8_t value) noexcept { spec->minimal_cargo = value; },
		.set_map_colour = [](IndustrySpec *spec, uint8_t value) noexcept { spec->map_colour = PixelColour{value}; },
		.set_behaviour = [](IndustrySpec *spec, uint32_t value) noexcept { spec->behaviour = IndustryBehaviours{value}; },
		.set_prospecting = [](IndustrySpec *spec, uint32_t value) noexcept { spec->prospecting_chance = value; },
		.set_removal_cost = [](IndustrySpec *spec, uint32_t value) noexcept { spec->removal_cost_multiplier = value; },
		.set_callbacks = [](IndustrySpec *spec, uint16_t value) noexcept { spec->callback_mask = IndustryCallbackMasks{value}; },
		.set_tile_slopes = [](IndustryTileSpec *spec, uint8_t value) noexcept { spec->slopes_refused = static_cast<Slope>(value); },
		.set_tile_callbacks = [](IndustryTileSpec *spec, uint8_t value) noexcept { spec->callback_mask = static_cast<IndustryTileCallbackMasks>(value); },
		.set_animation_frames = [](IndustryTileSpec *spec, uint8_t value) noexcept { spec->animation.frames = value; },
		.set_animation_status = [](IndustryTileSpec *spec, uint8_t value) noexcept { spec->animation.status = static_cast<AnimationStatus>(value); },
		.set_animation_speed = [](IndustryTileSpec *spec, uint8_t value) noexcept { spec->animation.speed = value; },
		.set_animation_triggers = [](IndustryTileSpec *spec, uint8_t value) noexcept { spec->animation.triggers = IndustryAnimationTriggers{value}; },
		.set_tile_flags = [](IndustryTileSpec *spec, uint8_t value) noexcept { spec->special_flags = IndustryTileSpecialFlags{value}; },
		.map_closure_text = [](IndustrySpec *spec, uint16_t string) noexcept { AddStringForMapping(GRFStringID{string}, &spec->closure_text); },
		.map_production_up_text = [](IndustrySpec *spec, uint16_t string) noexcept { AddStringForMapping(GRFStringID{string}, &spec->production_up_text); },
		.map_production_down_text = [](IndustrySpec *spec, uint16_t string) noexcept { AddStringForMapping(GRFStringID{string}, &spec->production_down_text); },
		.map_new_industry_text = [](IndustrySpec *spec, uint16_t string) noexcept { AddStringForMapping(GRFStringID{string}, &spec->new_industry_text); },
		.map_name = [](IndustrySpec *spec, uint16_t string) noexcept { AddStringForMapping(GRFStringID{string}, &spec->name); },
		.map_station_name = [](IndustrySpec *spec, uint16_t string) noexcept { AddStringForMapping(GRFStringID{string}, &spec->station_name); },
		.original_layout_count = [](uint8_t type) noexcept -> size_t { return _origin_industry_specs[type].layouts.size(); },
		.original_layout = [](uint8_t type, uint8_t layout) noexcept -> OpenTTDIndustryDefinitionLayout { const auto &tiles = _origin_industry_specs[type].layouts[layout]; return {tiles.data(), tiles.size()}; },
		.install_layouts = [](IndustrySpec *spec, const OpenTTDIndustryDefinitionLayoutBuffer *layouts, size_t count) noexcept { std::vector<IndustryTileLayout> result; result.reserve(count); for (size_t i = 0; i < count; ++i) result.emplace_back(layouts[i].data, layouts[i].data + layouts[i].size); spec->layouts = std::move(result); },
		.too_many_tiles = [](uint32_t last) noexcept { GrfMsg(1, "IndustryTilesChangeInfo: Too many industry tiles loaded ({}), max ({}). Ignoring.", last, NUM_INDUSTRYTILES_PER_GRF); },
		.too_many_industries = [](uint32_t last) noexcept { GrfMsg(1, "IndustriesChangeInfo: Too many industries loaded ({}), max ({}). Ignoring.", last, NUM_INDUSTRYTYPES_PER_GRF); },
		.tile_substitute_invalid = [](uint8_t substitute, uint32_t id) noexcept { GrfMsg(2, "IndustryTilesChangeInfo: Attempt to use new industry tile {} as substitute industry tile for {}. Ignoring.", substitute, id); },
		.tile_override_invalid = [](uint8_t override_id, uint32_t id) noexcept { GrfMsg(2, "IndustryTilesChangeInfo: Attempt to override new industry tile {} with industry tile id {}. Ignoring.", override_id, id); },
		.industry_substitute_invalid = [](uint8_t substitute, uint32_t id) noexcept { GrfMsg(2, "_industry_specs: Attempt to use new industry {} as substitute industry for {}. Ignoring.", substitute, id); },
		.industry_override_invalid = [](uint8_t override_id, uint32_t id) noexcept { GrfMsg(2, "IndustriesChangeInfo: Attempt to override new industry {} with industry id {}. Ignoring.", override_id, id); },
		.layout_size_mismatch = [](uint32_t id) noexcept { GrfMsg(3, "IndustriesChangeInfo: Incorrect size for industry tile layout definition for industry {}.", id); },
		.layout_import_type = [](uint32_t id) noexcept { GrfMsg(1, "IndustriesChangeInfo: Invalid original industry number for layout import, industry {}", id); },
		.layout_import_index = [](uint32_t id) noexcept { GrfMsg(1, "IndustriesChangeInfo: Invalid original industry layout index for layout import, industry {}", id); },
		.layout_tile_undefined = [](uint16_t tile, uint32_t id) noexcept { GrfMsg(2, "IndustriesChangeInfo: Attempt to use industry tile {} with industry id {}, not yet defined. Ignoring.", tile, id); },
		.layout_invalid = [](uint32_t id) noexcept { GrfMsg(1, "IndustriesChangeInfo: Invalid industry layout for industry id {}. Ignoring", id); },
		.list_too_long = [](int32_t prop) noexcept { GRFError *error = DisableGrf(STR_NEWGRF_ERROR_LIST_PROPERTY_TOO_LONG); error->param_value[1] = prop; },
		.invalid_import = []() noexcept { DisableGrf(STR_NEWGRF_ERROR_INVALID_ID); },
		.cargo_translation = OpenTTDCargoTranslation,
		.industry_badges = [](IndustrySpec *spec) noexcept { return &spec->badges; },
		.tile_badges = [](IndustryTileSpec *spec) noexcept { return &spec->badges; },
	};
	return services;
}
#endif
