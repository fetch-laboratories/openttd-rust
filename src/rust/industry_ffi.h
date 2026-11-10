/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file industry_ffi.h Canonical industry storage and periodic control. */
#ifndef RUST_INDUSTRY_FFI_H
#define RUST_INDUSTRY_FFI_H
struct GRFFile;
struct OpenTTDGrfTextServices;
#include "service_defaults.hpp"
struct OpenTTDSubsidyServices;
struct OpenTTDIndustryResolverServices;
#include "command_services_ffi.h"
#include <stddef.h>
#include <stdbool.h>
#include <stdint.h>
#include "byte_span_ffi.h"
#include "services_ffi.h"
#include "command_services_ffi.h"
#ifdef __cplusplus
extern "C" {
#endif
typedef struct Industry OpenTTDIndustryShell;
typedef struct IndustryTileSpec OpenTTDIndustryTileSpec;
struct IndustrySpec;
typedef struct OpenTTDIndustryStationHandle OpenTTDIndustryStationHandle;
typedef struct OpenTTDIndustry OpenTTDIndustry;
typedef struct OpenTTDIndustryBuilder OpenTTDIndustryBuilder;
typedef struct OpenTTDIndustryFields {
	uint64_t valid_history;
	int32_t last_prod_year;
	uint16_t counter;
	uint8_t prod_level, was_cargo_delivered, ctlflags;
} OpenTTDIndustryFields;
typedef struct OpenTTDIndustryBuildFields {
	uint32_t probability;
	uint8_t min_number;
	uint16_t target_count, max_wait, wait_count;
} OpenTTDIndustryBuildFields;
typedef struct OpenTTDIndustryBuilderFields {
	OpenTTDIndustryBuildFields builddata[240];
	uint32_t wanted_inds, daily_counter, daily_increment, sound_tile;
	uint8_t sound_ctr;
} OpenTTDIndustryBuilderFields;
typedef struct OpenTTDIndustrySlots { void *data; size_t size; } OpenTTDIndustrySlots;
typedef struct OpenTTDIndustryMetadata {
	uint32_t tile;
	uint16_t width, height;
	int32_t construction_date;
	uint16_t random;
	uint8_t type, owner, random_colour, founder, construction_type, selected_layout, exclusive_supplier, exclusive_consumer;
} OpenTTDIndustryMetadata;
OpenTTDIndustryMetadata *openttd_rust_industry_metadata(OpenTTDIndustry *owner);
/* Borrowed until text mutation or owner destruction; consume before any callback. */
OpenTTDByteSpan openttd_rust_industry_text(const OpenTTDIndustry *owner);
void openttd_rust_industry_set_text(OpenTTDIndustry *owner, const uint8_t *text, size_t length);
OpenTTDIndustrySlots openttd_rust_industry_registry_view(OpenTTDIndustryBuilder *owner, uint8_t type);
void openttd_rust_industry_registry_insert(OpenTTDIndustryBuilder *owner, uint8_t type, uint16_t id);
void openttd_rust_industry_registry_erase(OpenTTDIndustryBuilder *owner, uint8_t type, uint16_t id);
void openttd_rust_industry_registry_clear(OpenTTDIndustryBuilder *owner, uint8_t type);
OpenTTDIndustry *openttd_rust_industry_new(void);
void openttd_rust_industry_destroy(OpenTTDIndustry *owner);
OpenTTDIndustryFields *openttd_rust_industry_fields(OpenTTDIndustry *owner);
/* One entry per vector operation; any mutation invalidates outstanding views. */
OpenTTDIndustrySlots openttd_rust_industry_produced_view(OpenTTDIndustry *owner);
OpenTTDIndustrySlots openttd_rust_industry_accepted_view(OpenTTDIndustry *owner);
void openttd_rust_industry_produced_reserve(OpenTTDIndustry *owner, size_t count);
void openttd_rust_industry_accepted_reserve(OpenTTDIndustry *owner, size_t count);
void openttd_rust_industry_produced_resize(OpenTTDIndustry *owner, size_t count);
void openttd_rust_industry_accepted_resize(OpenTTDIndustry *owner, size_t count);
OpenTTDIndustrySlots openttd_rust_industry_produced_emplace_back(OpenTTDIndustry *owner);
OpenTTDIndustrySlots openttd_rust_industry_accepted_emplace_back(OpenTTDIndustry *owner);
void openttd_rust_industry_produced_shrink_to_fit(OpenTTDIndustry *owner);
void openttd_rust_industry_accepted_shrink_to_fit(OpenTTDIndustry *owner);
void *openttd_rust_industry_history(void *accepted);
OpenTTDIndustryBuilder *openttd_rust_industry_builder_new(void);
void openttd_rust_industry_builder_destroy(OpenTTDIndustryBuilder *owner);
OpenTTDIndustryBuilderFields *openttd_rust_industry_builder_fields(OpenTTDIndustryBuilder *owner);

/** Map facts constant for one entry; landscape is the original enum value. */
typedef struct OpenTTDIndustryMap {
	uint32_t size_x, size_y;
	uint8_t landscape;
} OpenTTDIndustryMap;
/** Next pool industry from an index; industry is null at the end. */
typedef struct OpenTTDIndustryEntry {
	void *industry;
	OpenTTDIndustry *owner;
	uint16_t id;
} OpenTTDIndustryEntry;
typedef struct OpenTTDIndustryLocation {
	uint32_t tile;
	uint16_t width, height;
} OpenTTDIndustryLocation;
/** Snow and ground for clear tiles; ground and grown (at least TreeGrowthStage::Grown) for tree tiles; no height. */
typedef struct OpenTTDIndustryFarmTile {
	uint8_t type, snow, ground, grown;
} OpenTTDIndustryFarmTile;
typedef struct OpenTTDIndustryChange {
	uint16_t result;
	int32_t reg;
} OpenTTDIndustryChange;
#ifdef __cplusplus
#	define OPENTTD_INDUSTRY_NOEXCEPT noexcept
#else
#	define OPENTTD_INDUSTRY_NOEXCEPT
#endif
enum class OpenTTDIndustryServicing : uint8_t { Nobody, Other, Company };
/** Direct typed services, one per original operation. No Rust borrow survives
 * any call; construction, clearing, distribution and destruction may reenter. */
typedef struct OpenTTDIndustryServices {
	const OpenTTDSharedServices *shared;
	/* Periodic tick. */
	OpenTTDIndustryEntry (*next_tick)(uint32_t from) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(next_tick);
	void (*industry_sound)(void *industry, uint32_t index) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(industry_sound);
	void (*play_sound)(uint16_t sound, uint32_t tile) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(play_sound);
	uint16_t (*special_effect)(void *industry, uint32_t random, uint32_t parameter) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(special_effect);
	void (*tick_trigger)(void *industry) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(tick_trigger);
	void (*production_callback)(void *industry, uint8_t reason) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(production_callback);
	uint32_t (*scale_cargo)(uint32_t amount) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(scale_cargo);
	const OpenTTDEnvironmentServices *environment;
	/* Farm fields and lumber mills. */
	OpenTTDIndustryFarmTile (*farm_tile)(uint32_t tile) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(farm_tile);
	void (*make_field)(uint32_t tile, uint8_t field_type, uint8_t counter, uint16_t industry) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(make_field);
	bool (*fence_wanted)(uint32_t tile, uint8_t side) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(fence_wanted);
	bool (*tile_completed)(void *industry, uint32_t tile) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(tile_completed);
	void (*harvest)(uint32_t tile) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(harvest);
	/* Monthly, daily, builder and command entries. */
	OpenTTDIndustryChange (*change_callback)(void *industry, bool monthly, uint32_t random) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(change_callback);
	const OpenTTDGrfTextServices *text;
	void (*news)(void *industry, uint32_t text, bool close, OpenTTDIndustryServicing servicing) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(news);
	void (*production_news)(void *industry, uint8_t cargo, int32_t percent, OpenTTDIndustryServicing servicing) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(production_news);
	void (*callback_error)(void *industry, bool invalid_cargo) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(callback_error);
	void (*set_dirty)(void *industry) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(set_dirty);
	void (*destroy)(void *industry) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(destroy);
	void (*opening_news)(void *industry) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(opening_news);
	void (*opening_events)(void *industry) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(opening_events);
	void *(*create)(uint8_t type, uint8_t creation) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(create);
	uint32_t (*num_items)(void) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(num_items);
	void *(*get)(uint16_t id) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(get);
	const struct OpenTTDIndustryResolverServices *resolver;
	uint8_t (*company_none)(void) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(company_none);
	void (*restore_company)(uint8_t company) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(restore_company);
	void (*directory_dirty)(uint8_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(directory_dirty);
} OpenTTDIndustryServices;
uint8_t openttd_rust_industry_forest_abi(uint32_t, const OpenTTDIndustryServices *);
extern "C++" inline bool openttd_rust_industry_forest(uint32_t a0, const OpenTTDIndustryServices *a1) { return openttd_rust_industry_forest_abi(a0, a1) != 0; }
uint8_t openttd_rust_industry_spec_raw_abi(const IndustrySpec *);
extern "C++" inline bool openttd_rust_industry_spec_raw(const IndustrySpec *a0) { return openttd_rust_industry_spec_raw_abi(a0) != 0; }
uint8_t openttd_rust_industry_spec_processing_abi(const IndustrySpec *);
extern "C++" inline bool openttd_rust_industry_spec_processing(const IndustrySpec *a0) { return openttd_rust_industry_spec_processing_abi(a0) != 0; }
uint8_t openttd_rust_industry_original_economy_abi(const IndustrySpec *, const OpenTTDSharedServices *shared);
extern "C++" inline bool openttd_rust_industry_original_economy(const IndustrySpec *a0, const OpenTTDSharedServices *a1) { return openttd_rust_industry_original_economy_abi(a0, a1) != 0; }
int64_t openttd_rust_industry_construction_cost(const IndustrySpec *, const OpenTTDSharedServices *shared);
IndustrySpec *openttd_rust_industry_specs();
IndustryTileSpec *openttd_rust_industry_tile_specs();
struct OpenTTDIndustryPair { uint16_t first, second; };
OpenTTDIndustryPair openttd_rust_industry_sum_history(const OpenTTDIndustryPair *, size_t);


/** Minimal destruction map facts, queried at the original observation point. */
typedef struct OpenTTDIndustryCleanupTile { uint16_t industry; uint8_t type; bool oilrig; } OpenTTDIndustryCleanupTile;
/** NewGRF resolver output. Sprite groups/resolvers remain native shared services. */
typedef struct OpenTTDIndustryReseed { uint32_t self, parent; uint8_t used; } OpenTTDIndustryReseed;
typedef struct OpenTTDIndustryRandomisationSpec { uint16_t gfx; bool enabled; } OpenTTDIndustryRandomisationSpec;
typedef struct OpenTTDIndustryAnimationSpec { const void *spec; uint8_t triggers; } OpenTTDIndustryAnimationSpec;
struct OpenTTDWaterRestorationServices;
typedef struct OpenTTDIndustryLifecycleServices {
	OpenTTDIndustryStationHandle *(*neutral_station)(void *) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(neutral_station);
	OpenTTDIndustryCleanupTile (*cleanup_tile)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(cleanup_tile);
	void (*delete_tile_inspect)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(delete_tile_inspect);
	const OpenTTDWaterRestorationServices *restoration;
	void (*delete_oilrig)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(delete_oilrig);
	void (*clear_docking)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(clear_docking);
	bool (*industry_field)(uint32_t, uint16_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(industry_field);
	void (*disassociate_field)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(disassociate_field);
	void (*release_disasters)(uint16_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(release_disasters);
	void (*delete_storage)(void *) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(delete_storage);
	void (*delete_news)(uint16_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(delete_news);
	void (*close_view)(uint16_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(close_view);
	void (*close_production)(uint16_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(close_production);
	void (*delete_inspect)(uint16_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(delete_inspect);
	const OpenTTDSubsidyServices *(*subsidies)() OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(subsidies);
	void (*invalidate_cargo)(uint16_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(invalidate_cargo);
	bool (*belongs)(uint32_t, uint16_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(belongs);
	OpenTTDIndustryRandomisationSpec (*randomisation_spec)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(randomisation_spec);
	uint8_t (*random_triggers)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(random_triggers);
	void (*set_random_triggers)(uint32_t, uint8_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(set_random_triggers);
	OpenTTDIndustryReseed (*resolve_reseed)(uint32_t, uint16_t, void *, uint8_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(resolve_reseed);
	uint8_t (*random_bits)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(random_bits);
	void (*set_random_bits)(uint32_t, uint8_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(set_random_bits);
	OpenTTDIndustryAnimationSpec (*animation_spec)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(animation_spec);

} OpenTTDIndustryLifecycleServices;
void openttd_rust_industry_cleanup(void *, OpenTTDIndustry *, OpenTTDIndustryBuilder *, uint16_t, bool, OpenTTDIndustryMap, const OpenTTDIndustryServices *, const OpenTTDIndustryLifecycleServices *);
void openttd_rust_industry_randomise_tile(uint32_t, uint8_t, void *, OpenTTDIndustry *, const OpenTTDIndustryServices *, const OpenTTDIndustryLifecycleServices *);
void openttd_rust_industry_randomise(void *, OpenTTDIndustry *, uint16_t, uint8_t, uint32_t, const OpenTTDIndustryServices *, const OpenTTDIndustryLifecycleServices *);
uint8_t openttd_rust_industry_animate_tile_abi(uint32_t, uint8_t, uint32_t, const OpenTTDIndustryServices *, const OpenTTDIndustryLifecycleServices *);
extern "C++" inline bool openttd_rust_industry_animate_tile(uint32_t a0, uint8_t a1, uint32_t a2, const OpenTTDIndustryServices *a3, const OpenTTDIndustryLifecycleServices *a4) { return openttd_rust_industry_animate_tile_abi(a0, a1, a2, a3, a4) != 0; }
uint8_t openttd_rust_industry_animate_abi(OpenTTDIndustry *, uint16_t, uint8_t, uint32_t, const OpenTTDIndustryServices *, const OpenTTDIndustryLifecycleServices *);
extern "C++" inline bool openttd_rust_industry_animate(OpenTTDIndustry *a0, uint16_t a1, uint8_t a2, uint32_t a3, const OpenTTDIndustryServices *a4, const OpenTTDIndustryLifecycleServices *a5) { return openttd_rust_industry_animate_abi(a0, a1, a2, a3, a4, a5) != 0; }
typedef struct OpenTTDIndustrySpecHandle OpenTTDIndustrySpecHandle;
typedef struct OpenTTDIndustryGenerationContext OpenTTDIndustryGenerationContext;
typedef struct OpenTTDIndustryGenerationServices {
	OpenTTDEnvironmentServices environment;
	uint32_t (*scale_by_size_1d)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(scale_by_size_1d);
	uint32_t (*scale_by_land_proportion)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(scale_by_land_proportion);
	bool (*set_progress)(OpenTTDIndustryGenerationContext *, bool, uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(set_progress);
	bool (*increase_progress)(OpenTTDIndustryGenerationContext *, bool) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(increase_progress);
	void (*missing_warning)(const OpenTTDIndustrySpecHandle *) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(missing_warning);
} OpenTTDIndustryGenerationServices;
uint8_t openttd_rust_industry_generate_abi(OpenTTDIndustryBuilder *, OpenTTDIndustryGenerationContext *, const OpenTTDIndustryServices *, const OpenTTDIndustryGenerationServices *);
extern "C++" inline bool openttd_rust_industry_generate(OpenTTDIndustryBuilder *a0, OpenTTDIndustryGenerationContext *a1, const OpenTTDIndustryServices *a2, const OpenTTDIndustryGenerationServices *a3) { return openttd_rust_industry_generate_abi(a0, a1, a2, a3) != 0; }
void openttd_rust_industry_check(OpenTTDIndustryBuilder *, const OpenTTDIndustryServices *, const OpenTTDIndustryGenerationServices *);
typedef struct OpenTTDTownHandle OpenTTDTownHandle;
typedef struct OpenTTDIndustryLayoutHandle OpenTTDIndustryLayoutHandle;
typedef struct OpenTTDIndustryCargoLists { uint8_t produced[16], accepted[16]; } OpenTTDIndustryCargoLists;
typedef struct OpenTTDIndustryLayoutTile { int16_t x, y; uint16_t gfx; } OpenTTDIndustryLayoutTile;
typedef struct OpenTTDIndustryConstructionRequest {
	const OpenTTDIndustryLayoutHandle *layout;
	const OpenTTDTownHandle *town;
	size_t layout_index;
	uint32_t tile, size_x, size_y;
	uint16_t id, random_bits;
	uint8_t type, founder;
} OpenTTDIndustryConstructionRequest;
typedef struct OpenTTDIndustryConstructionServices {
	void (*set_town)(void *, const OpenTTDTownHandle *) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(set_town);
	bool (*generating)(void) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(generating);
	uint16_t (*callback)(void *, uint16_t, uint32_t, uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(callback);
	void (*callback_error)(const OpenTTDIndustrySpecHandle *, uint16_t, uint16_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(callback_error);
	uint8_t (*water_class)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(water_class);
	void (*clear_tile)(uint32_t, uint16_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(clear_tile);
	void (*make_industry)(uint32_t, uint16_t, uint16_t, uint32_t, uint8_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(make_industry);
	void (*set_construction_counter)(uint32_t, uint8_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(set_construction_counter);
	void (*set_construction_stage)(uint32_t, uint8_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(set_construction_stage);
	uint16_t (*translated_gfx)(uint16_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(translated_gfx);

	void (*build_window_dirty)(void) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(build_window_dirty);
	bool (*serve_neutral)(void) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(serve_neutral);
} OpenTTDIndustryConstructionServices;
uint8_t openttd_rust_industry_transport_abi(void *industry, OpenTTDIndustry *owner, uint8_t minimal_cargo, bool recession, const OpenTTDIndustryServices *services);
extern "C++" inline bool openttd_rust_industry_transport(void *a0, OpenTTDIndustry *a1, uint8_t a2, bool a3, const OpenTTDIndustryServices *a4) { return openttd_rust_industry_transport_abi(a0, a1, a2, a3, a4) != 0; }
void openttd_rust_industry_farm(OpenTTDIndustry *owner, uint16_t id, const OpenTTDIndustryServices *services);
void openttd_rust_industry_recompute(OpenTTDIndustry *owner);
void openttd_rust_industry_daily_start(OpenTTDIndustryBuilder *builder, uint32_t map_bits, uint8_t init_counter);
void openttd_rust_industry_build_type_reset(OpenTTDIndustryBuildFields *fields);
/** Native stack cost operations consumed by industry policy. */
#ifdef __cplusplus
typedef class CommandCost OpenTTDIndustryCost;
#else
typedef struct CommandCost OpenTTDIndustryCost;
#endif
typedef struct OpenTTDIndustryCostServices {
	void (*zero)(OpenTTDIndustryCost *) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(zero);
	void (*error)(OpenTTDIndustryCost *, uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(error);
	void (*expense_money)(OpenTTDIndustryCost *, uint8_t, int64_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(expense_money);
	bool (*success)(const OpenTTDIndustryCost *) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(success);
} OpenTTDIndustryCostServices;
typedef struct OpenTTDIndustryTilePolicyServices {
	bool (*magic_bulldozer)(void) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(magic_bulldozer);
	void (*object_in_way)(OpenTTDIndustryCost *, const OpenTTDIndustrySpecHandle *) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(object_in_way);
	void (*close_ai)(OpenTTDIndustryShell *) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(close_ai);
	void (*close_game)(OpenTTDIndustryShell *) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(close_game);
	void (*destroy)(OpenTTDIndustryShell *) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(destroy);
	int32_t (*tile_max_z)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(tile_max_z);
	const OpenTTDCommandServices *command_checks;
} OpenTTDIndustryTilePolicyServices;
typedef struct OpenTTDIndustryCommandServices {
	const OpenTTDSharedServices *shared;
	OpenTTDIndustryShell * (*get)(uint16_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(get);
	OpenTTDIndustry * (*owner)(OpenTTDIndustryShell *) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(owner);
	bool (*valid_company)(uint8_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(valid_company);
	void (*invalidate_view)(uint16_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(invalidate_view);
} OpenTTDIndustryCommandServices;
uint8_t openttd_rust_industry_set_flags_command_abi(uint16_t id, bool execute, uint8_t value, const OpenTTDIndustryCommandServices *);
extern "C++" inline bool openttd_rust_industry_set_flags_command(uint16_t a0, bool a1, uint8_t a2, const OpenTTDIndustryCommandServices *a3) { return openttd_rust_industry_set_flags_command_abi(a0, a1, a2, a3) != 0; }
uint8_t openttd_rust_industry_set_production_command_abi(uint16_t id, bool execute, uint8_t level, const OpenTTDIndustryCommandServices *, const OpenTTDIndustryServices *);
extern "C++" inline bool openttd_rust_industry_set_production_command(uint16_t a0, bool a1, uint8_t a2, const OpenTTDIndustryCommandServices *a3, const OpenTTDIndustryServices *a4) { return openttd_rust_industry_set_production_command_abi(a0, a1, a2, a3, a4) != 0; }
uint8_t openttd_rust_industry_set_exclusivity_command_abi(uint16_t id, bool execute, uint8_t company, bool consumer, const OpenTTDIndustryCommandServices *);
extern "C++" inline bool openttd_rust_industry_set_exclusivity_command(uint16_t a0, bool a1, uint8_t a2, bool a3, const OpenTTDIndustryCommandServices *a4) { return openttd_rust_industry_set_exclusivity_command_abi(a0, a1, a2, a3, a4) != 0; }
uint8_t openttd_rust_industry_set_text_command_abi(uint16_t id, bool execute, OpenTTDByteSpan text, const OpenTTDIndustryCommandServices *);
extern "C++" inline bool openttd_rust_industry_set_text_command(uint16_t a0, bool a1, OpenTTDByteSpan a2, const OpenTTDIndustryCommandServices *a3) { return openttd_rust_industry_set_text_command_abi(a0, a1, a2, a3) != 0; }
void openttd_rust_industry_build_reset(OpenTTDIndustryBuilder *builder, const OpenTTDIndustryServices *services);
void openttd_rust_industry_build_monthly(OpenTTDIndustryBuilder *builder, const OpenTTDIndustryServices *services);
void openttd_rust_industry_build_targets(OpenTTDIndustryBuilder *builder, const OpenTTDIndustryServices *services);
void openttd_rust_industry_build_try(OpenTTDIndustryBuilder *builder, const OpenTTDIndustryServices *services);
uint8_t openttd_rust_industry_build_type(OpenTTDIndustryBuildFields *fields, uint8_t type, const OpenTTDIndustryServices *services);
void openttd_rust_industry_trim(OpenTTDIndustry *industry);
typedef struct OpenTTDIndustryProductionResult {
	int32_t subtract[256], add[256], again;
	uint8_t cargo_input[256], cargo_output[256], version, num_input, num_output, present;
} OpenTTDIndustryProductionResult;
/** A native resolver lives here from construct to destroy; no pointer escapes this call. */
struct alignas(16) OpenTTDIndustryProductionStorage { uint8_t storage[1024]; };
struct OpenTTDIndustryProductionContext;
struct OpenTTDIndustrySpecHandle;
struct OpenTTDIndustryDeliveryServices {
	OpenTTDIndustryProductionContext *(*construct)(OpenTTDIndustryProductionStorage *, Industry *) noexcept OPENTTD_SERVICE_DEFAULT(construct);
	void (*destroy)(OpenTTDIndustryProductionContext *) noexcept OPENTTD_SERVICE_DEFAULT(destroy);
	void (*resolve)(OpenTTDIndustryProductionContext *, uint32_t, uint32_t, OpenTTDIndustryProductionResult *) noexcept OPENTTD_SERVICE_DEFAULT(resolve);
};
typedef void (*OpenTTDIndustryResolve)(void *context, uint32_t random, uint32_t parameter, OpenTTDIndustryProductionResult *out);
void openttd_rust_industry_production_callback(void *industry, OpenTTDIndustry *owner, void *context, uint32_t behaviour, uint8_t reason, OpenTTDIndustryResolve resolve, const OpenTTDIndustryServices *services);
/** Minimal live tile lookup; no persistent shell observation. */
typedef struct OpenTTDIndustryTileOwner {
	OpenTTDIndustryShell *shell;
	OpenTTDIndustry *owner;
} OpenTTDIndustryTileOwner;
/** Consumed immutable tile acceptance properties, not a live industry view. */
typedef struct OpenTTDIndustryTileAcceptance {
	uint8_t cargo[16];
	int8_t amount[16];
	bool accepts_all, accepts_callback, amount_callback;
} OpenTTDIndustryTileAcceptance;
/** Consumed direct leaves; generic animation/map leaves share native definitions. */
typedef struct OpenTTDIndustryTileServices {
	const OpenTTDSharedServices *shared;
	uint16_t (*gfx)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(gfx);
	void (*set_gfx)(uint32_t, uint16_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(set_gfx);
	uint8_t (*frame)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(frame);
	void (*set_frame)(uint32_t, uint8_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(set_frame);
	uint8_t (*animation_loop)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(animation_loop);
	void (*set_animation_loop)(uint32_t, uint8_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(set_animation_loop);

	uint8_t (*construction_counter)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(construction_counter);
	uint8_t (*construction_stage)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(construction_stage);
	bool (*completed)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(completed);
	void (*set_completed)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(set_completed);
	void (*reset_construction)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(reset_construction);
	OpenTTDIndustryTileOwner (*industry)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(industry);
	uint16_t (*industry_index)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(industry_index);
	uint8_t (*toffee_offset)(uint8_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(toffee_offset);
	bool (*on_water)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(on_water);
	bool (*transport)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(transport);
	void (*build_oilrig)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(build_oilrig);
	int32_t (*pixel_max_z)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(pixel_max_z);
	uint16_t (*callback)(uint32_t, uint16_t, uint16_t, uint32_t, uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(callback);
} OpenTTDIndustryTileServices;
void openttd_rust_industry_tile_acceptance(uint32_t tile, uint32_t *acceptance, uint64_t *always_accepted, const OpenTTDIndustryTileServices *);
/** Industry-owned scratch value, matching the actual shared cleared-object entry. */
typedef struct OpenTTDIndustryAllocation { OpenTTDIndustryShell *shell; OpenTTDIndustry *owner; uint16_t id; } OpenTTDIndustryAllocation;
typedef struct OpenTTDIndustryConflictKinds { uint8_t kinds[3]; } OpenTTDIndustryConflictKinds;
typedef struct OpenTTDIndustryConstructionErrors {
	uint32_t site, forest, positioned, desert, rainforest, lowland, one_per_town, town, population, near_town, conflict, too_many;
} OpenTTDIndustryConstructionErrors;
typedef struct OpenTTDIndustryLocationResult { uint16_t callback; int32_t registers[16]; } OpenTTDIndustryLocationResult;
typedef void (*OpenTTDIndustryTemporaryVisit)(void *, OpenTTDIndustryShell *);
OpenTTDIndustryLocationResult openttd_industry_resolve_location(OpenTTDIndustryShell *, uint32_t tile, uint8_t type, uint32_t seed, uint8_t creation) OPENTTD_INDUSTRY_NOEXCEPT;
OpenTTDIndustryLocationResult openttd_industry_resolve_shape(OpenTTDIndustryShell *, uint32_t tile, uint16_t gfx, uint32_t parameter) OPENTTD_INDUSTRY_NOEXCEPT;
void openttd_industry_location_error(OpenTTDIndustryCost *, const OpenTTDIndustryLocationResult *, const OpenTTDIndustrySpecHandle *) OPENTTD_INDUSTRY_NOEXCEPT;
void openttd_industry_shape_error(OpenTTDIndustryCost *, const OpenTTDIndustryLocationResult *, const OpenTTDIndustryTileSpec *) OPENTTD_INDUSTRY_NOEXCEPT;
typedef struct OpenTTDIndustryTransactionServices {
	const OpenTTDTownHandle * (*town)(OpenTTDIndustryShell *) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(town);
	uint32_t (*population)(const OpenTTDTownHandle *) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(population);
	uint32_t (*town_position)(const OpenTTDTownHandle *) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(town_position);
	OpenTTDIndustryShell * (*industry)(uint16_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(industry);
	uint32_t (*edge_distance)(uint32_t, uint8_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(edge_distance);
	uint32_t (*tile_height)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(tile_height);
	void (*set_company)(uint8_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(set_company);
	/* Owner/context live on the Rust stack; visit and resolver cannot retain them. */
	void (*temporary_shell)(OpenTTDIndustry *, void *, OpenTTDIndustryTemporaryVisit) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(temporary_shell);
	OpenTTDIndustryLocationResult (*resolve_location)(OpenTTDIndustryShell *, uint32_t, uint8_t, uint32_t, uint8_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(resolve_location);
	OpenTTDIndustryLocationResult (*resolve_shape)(OpenTTDIndustryShell *, uint32_t, uint16_t, uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(resolve_shape);
	void (*location_error)(OpenTTDIndustryCost *, const OpenTTDIndustryLocationResult *, const OpenTTDIndustrySpecHandle *) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(location_error);
	void (*shape_error)(OpenTTDIndustryCost *, const OpenTTDIndustryLocationResult *, const OpenTTDIndustryTileSpec *) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(shape_error);
	bool (*ignore_restrictions)(void) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(ignore_restrictions);
	bool (*can_allocate)(void) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(can_allocate);
	OpenTTDIndustryAllocation (*allocate)(uint32_t) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(allocate);
	bool (*local_company)(void) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(local_company);
	void (*prospect_no_place)(void) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(prospect_no_place);
	void (*prospect_unlucky)(void) OPENTTD_INDUSTRY_NOEXCEPT OPENTTD_SERVICE_DEFAULT(prospect_unlucky);
} OpenTTDIndustryTransactionServices;
/** Stack-only immutable service context; no live owner observation. */
typedef struct OpenTTDIndustryTransactionContext {
	OpenTTDIndustryMap map;
	OpenTTDIndustryBuilder *builder;
	const OpenTTDIndustryServices *production;
	const OpenTTDIndustryGenerationServices *generation;
	const OpenTTDIndustryConstructionServices *construction;
	const OpenTTDIndustryLifecycleServices *lifecycle;
	const OpenTTDIndustryTileServices *tiles;
	const OpenTTDIndustryCommandServices *commands;
	const OpenTTDCommandServices *command_checks;
	const OpenTTDIndustryCostServices *costs;
	const OpenTTDIndustryTransactionServices *transaction;
	const OpenTTDIndustryConstructionErrors *errors;
} OpenTTDIndustryTransactionContext;
void openttd_rust_industry_build(OpenTTDIndustryCost *, uint32_t tile, uint8_t type, uint16_t flags, uint32_t first_layout, bool fund, uint32_t seed, const OpenTTDIndustryTransactionContext *);
void openttd_rust_industry_create(OpenTTDIndustryCost *, OpenTTDIndustryShell **industry, uint32_t tile, uint8_t type, uint8_t creation_type, const OpenTTDIndustryTransactionContext *);
void openttd_rust_industry_location_check(OpenTTDIndustryCost *, uint32_t tile, uint8_t type, size_t layout, uint32_t random, uint16_t bits, uint8_t founder, uint8_t creation, const OpenTTDIndustryTransactionContext *);
void openttd_rust_industry_shape_check(OpenTTDIndustryCost *, uint32_t tile, uint32_t current, const OpenTTDIndustryTileSpec *, uint8_t type, uint16_t gfx, size_t layout, uint16_t bits, uint8_t founder, uint8_t creation, const OpenTTDIndustryTransactionContext *);
uint8_t openttd_rust_industry_slope_refused_abi(uint8_t current, uint8_t refused);
extern "C++" inline bool openttd_rust_industry_slope_refused(uint8_t a0, uint8_t a1) { return openttd_rust_industry_slope_refused_abi(a0, a1) != 0; }
#ifdef __cplusplus
}
#endif
#ifdef __cplusplus
const OpenTTDIndustryDeliveryServices &GetIndustryDeliveryServices() noexcept;
const OpenTTDIndustryTileServices &GetIndustryTileServices() noexcept;
const OpenTTDIndustryTilePolicyServices &GetIndustryTilePolicyServices() noexcept;
const OpenTTDIndustryCostServices &GetIndustryCostServices() noexcept;
const OpenTTDIndustryGenerationServices &GetIndustryGenerationServices();
const OpenTTDIndustryConstructionServices &GetIndustryConstructionServices();
#endif
#endif
