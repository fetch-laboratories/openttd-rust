/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_loading_ffi.h Canonical loading-state aliases. */
#ifndef RUST_NEWGRF_LOADING_FFI_H
#define RUST_NEWGRF_LOADING_FFI_H
#include "newgrf_global_ffi.h"
#include "newgrf_text_ffi.h"
#include "grf_bindings_ffi.h"
#include "station_definitions_ffi.h"
#include "newgrf_class_ffi.h"
#include "../newgrf_config.h"
#include "../road_type.h"
#include "byte_span_ffi.h"
struct SpriteGroup;
struct GRFFile;
struct GRFConfig;
class SpriteFile;
enum GrfLoadingStage : uint8_t;
enum GrfSpecFeature : uint8_t;
struct OpenTTDGrfExecution {
	GrfLoadingStage stage;
	uint32_t spriteid;
	SpriteFile *file;
	GRFFile *grffile;
	GRFConfig *grfconfig;
	uint32_t nfo_line;
	int32_t skip_sprites;
	std::array<const SpriteGroup *, 256> spritegroups;
};
extern "C" {
OpenTTDGrfExecution *openttd_rust_grf_execution();
void openttd_rust_grf_add_sets(uint8_t, uint32_t, uint32_t, uint32_t, uint32_t);
uint8_t openttd_rust_grf_has_sets_abi(uint8_t);
extern "C++" inline bool openttd_rust_grf_has_sets(uint8_t a0) { return openttd_rust_grf_has_sets_abi(a0) != 0; }
uint8_t openttd_rust_grf_has_set_abi(uint8_t, uint32_t);
extern "C++" inline bool openttd_rust_grf_has_set(uint8_t a0, uint32_t a1) { return openttd_rust_grf_has_set_abi(a0, a1) != 0; }
uint32_t openttd_rust_grf_set_sprite(uint8_t, uint32_t);
uint32_t openttd_rust_grf_set_size(uint8_t, uint32_t);
}
struct OpenTTDHouseDefinitionServices;
struct OpenTTDEngineLoadingServices;
struct OpenTTDIndustryDefinitionServices;
struct OpenTTDAirportDefinitionServices;
struct OpenTTDTypeRegistryServices;
struct OpenTTDGenericServices;
struct OpenTTDCargoRegistryServices;
struct OpenTTDGrfClock { uint64_t ticks; uint8_t display; };
struct OpenTTDGrfEngineRange { uint8_t offset, count; };
struct OpenTTDAction2Services;
struct OpenTTDClockServices;
struct OpenTTDCompanyFinanceServices;
struct OpenTTDGrfLoadingServices {
	const OpenTTDClockServices *clocks;
	const OpenTTDCompanyFinanceServices *finance;
	const OpenTTDGlobalVariableServices * global;
	const OpenTTDCargoRegistryServices *cargo;
	const OpenTTDEngineLoadingServices *engine;
	const OpenTTDHouseDefinitionServices *houses;
	const OpenTTDIndustryDefinitionServices *industry;
	const OpenTTDAirportDefinitionServices *airports;
	const OpenTTDTypeRegistryServices *types;
	const OpenTTDAction2Services *action2;
	const OpenTTDGrfTextServices *text;
	const OpenTTDGrfBindingServices *bindings;
	const OpenTTDStationDefinitionServices *stations;
	OpenTTDGrfEngineRange (*engine_range)(GrfSpecFeature) noexcept OPENTTD_SERVICE_DEFAULT(engine_range);
	uint32_t (*config_id)(const GRFConfig *) noexcept OPENTTD_SERVICE_DEFAULT(config_id);
	GRFStatus (*config_status)(const GRFConfig *) noexcept OPENTTD_SERVICE_DEFAULT(config_status);
	uint8_t (*config_flags)(const GRFConfig *) noexcept OPENTTD_SERVICE_DEFAULT(config_flags);
	void (*set_status)(GRFConfig *, GRFStatus) noexcept OPENTTD_SERVICE_DEFAULT(set_status);
	void (*set_flag)(GRFConfig *, uint8_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_flag);
	void (*set_config_id)(GRFConfig *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_config_id);
	void (*set_file_info)(GRFFile *, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_file_info);
	uint32_t (*config_version)(const GRFConfig *) noexcept OPENTTD_SERVICE_DEFAULT(config_version);
	OpenTTDByteSpan (*config_filename)(const GRFConfig *) noexcept OPENTTD_SERVICE_DEFAULT(config_filename);
	uint8_t (*config_palette)(const GRFConfig *) noexcept OPENTTD_SERVICE_DEFAULT(config_palette);
	size_t (*param_count)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(param_count);
	void (*set_parameter)(GRFFile *, uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_parameter);
	GRFConfig *(*find_config)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(find_config);
	GRFFile *(*file_for_config)(const GRFConfig *) noexcept OPENTTD_SERVICE_DEFAULT(file_for_config);
	GRFError *(*error)(GRFConfig *, uint32_t, StringID, bool) noexcept OPENTTD_SERVICE_DEFAULT(error);
	void (*error_name)(GRFError *, const GRFConfig *) noexcept OPENTTD_SERVICE_DEFAULT(error_name);
	void (*error_property)(GRFError *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(error_property);
	void (*error_after_translated_file)(GRFError *) noexcept OPENTTD_SERVICE_DEFAULT(error_after_translated_file);
	StringID load_after;
	uint8_t (*read_byte)(SpriteFile *) noexcept OPENTTD_SERVICE_DEFAULT(read_byte);
	uint16_t (*read_word)(SpriteFile *) noexcept OPENTTD_SERVICE_DEFAULT(read_word);
	uint32_t (*read_dword)(SpriteFile *) noexcept OPENTTD_SERVICE_DEFAULT(read_dword);
	void (*read_block)(SpriteFile *, uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(read_block);
	size_t (*position)(const SpriteFile *) noexcept OPENTTD_SERVICE_DEFAULT(position);
	void (*seek)(SpriteFile *, size_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(seek);
	void (*skip)(SpriteFile *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(skip);
	uint8_t (*container_version)(const SpriteFile *) noexcept OPENTTD_SERVICE_DEFAULT(container_version);
	void (*sprite_offsets)(SpriteFile *) noexcept OPENTTD_SERVICE_DEFAULT(sprite_offsets);
	bool (*skip_sprite_data)(SpriteFile *, uint8_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(skip_sprite_data);
	SpriteFile *(*open_cached)(const GRFConfig *, Subdirectory) noexcept OPENTTD_SERVICE_DEFAULT(open_cached);
	SpriteFile *(*open_temporary)(const GRFConfig *, Subdirectory, void *) noexcept OPENTTD_SERVICE_DEFAULT(open_temporary);
	void (*scan_name)(GRFConfig *, uint32_t, const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(scan_name);
	void (*scan_info)(GRFConfig *, uint32_t, const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(scan_info);
	void (*set_pitch)(GRFFile *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_pitch);
	void (*set_width)(GRFFile *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_width);
	void (*set_rail_cost)(RailType, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_rail_cost);
	void (*set_misc)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_misc);
	bool (*safe_change_info)(const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(safe_change_info);
	bool (*reserve_change_info)(const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(reserve_change_info);
	bool (*feature_change_info)(const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(feature_change_info);
	bool (*skip_act1)(const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(skip_act1);
	bool (*new_sprite_set)(const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(new_sprite_set);
	bool (*skip_act5)(const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(skip_act5);
	bool (*graphics_new)(const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(graphics_new);
	bool (*skip_act_a)(const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(skip_act_a);
	bool (*sprite_replace)(const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(sprite_replace);
	bool (*grf_load_error)(const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(grf_load_error);
	bool (*grf_comment)(const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(grf_comment);
	bool (*skip_act11)(const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(skip_act11);
	bool (*grf_sound)(const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(grf_sound);
	bool (*skip_act12)(const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(skip_act12);
	bool (*load_font_glyph)(const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(load_font_glyph);
	bool (*static_grf_info)(const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(static_grf_info);
	bool (*gradual_loading)() noexcept OPENTTD_SERVICE_DEFAULT(gradual_loading);
	uint8_t (*signal_side)() noexcept OPENTTD_SERVICE_DEFAULT(signal_side);
	uint8_t (*freight_trains)() noexcept OPENTTD_SERVICE_DEFAULT(freight_trains);
	bool (*wagon_speed_limits)() noexcept OPENTTD_SERVICE_DEFAULT(wagon_speed_limits);
	bool (*improved_load)() noexcept OPENTTD_SERVICE_DEFAULT(improved_load);
	bool (*inflation)() noexcept OPENTTD_SERVICE_DEFAULT(inflation);
	int32_t (*starting_year)() noexcept OPENTTD_SERVICE_DEFAULT(starting_year);
	uint8_t (*plane_speed)() noexcept OPENTTD_SERVICE_DEFAULT(plane_speed);
	StringID read_bounds;
	StringID unexpected_sprite;
	StringID multiple_action8;
	StringID grm_failed;
	StringID static_desync;
	StringID list_property_too_long;
	StringID invalid_id;
	StringID force_disabled;
	SpriteID colourmap_sprite;
	SpriteID slopes_sprite;
	SpriteID shore_sprite;
	const OpenTTDGenericServices *generic;
	OpenTTDGrfClock (*clock)() noexcept OPENTTD_SERVICE_DEFAULT(clock);
	void (*set_clock)(const OpenTTDGrfClock *) noexcept OPENTTD_SERVICE_DEFAULT(set_clock);
	GRFConfig *(*config_at)(size_t) noexcept OPENTTD_SERVICE_DEFAULT(config_at);
	bool (*exists)(const GRFConfig *, Subdirectory) noexcept OPENTTD_SERVICE_DEFAULT(exists);
	void (*init_file)(const GRFConfig *) noexcept OPENTTD_SERVICE_DEFAULT(init_file);
	void (*too_many)(GRFConfig *) noexcept OPENTTD_SERVICE_DEFAULT(too_many);
	void (*clear_errors)(GRFConfig *) noexcept OPENTTD_SERVICE_DEFAULT(clear_errors);
	void (*reset_badges)() noexcept OPENTTD_SERVICE_DEFAULT(reset_badges);
	void (*reset_bridges)() noexcept OPENTTD_SERVICE_DEFAULT(reset_bridges);
	void (*reset_prices)() noexcept OPENTTD_SERVICE_DEFAULT(reset_prices);
	void (*reset_currencies)() noexcept OPENTTD_SERVICE_DEFAULT(reset_currencies);
	const OpenTTDObjectClassServices *object_classes;
	void (*reset_objects)() noexcept OPENTTD_SERVICE_DEFAULT(reset_objects);
	const OpenTTDStationClassServices *station_classes;
	const OpenTTDAirportClassServices *airport_classes;
	const OpenTTDRoadStopClassServices *roadstop_classes;
	void (*reset_water_features)() noexcept OPENTTD_SERVICE_DEFAULT(reset_water_features);
	void (*reset_faces)() noexcept OPENTTD_SERVICE_DEFAULT(reset_faces);
	void (*reset_files)() noexcept OPENTTD_SERVICE_DEFAULT(reset_files);
	void (*reset_loaded_features)() noexcept OPENTTD_SERVICE_DEFAULT(reset_loaded_features);
	void (*initialize_sounds)() noexcept OPENTTD_SERVICE_DEFAULT(initialize_sounds);
	void (*clean_spritegroups)() noexcept OPENTTD_SERVICE_DEFAULT(clean_spritegroups);
	void (*finalise_strings)() noexcept OPENTTD_SERVICE_DEFAULT(finalise_strings);
	void (*finalise_badges)() noexcept OPENTTD_SERVICE_DEFAULT(finalise_badges);
	void (*finalise_canals)() noexcept OPENTTD_SERVICE_DEFAULT(finalise_canals);
	void (*finalise_objects)() noexcept OPENTTD_SERVICE_DEFAULT(finalise_objects);
	void (*sort_industries)() noexcept OPENTTD_SERVICE_DEFAULT(sort_industries);
	void (*industry_legend)() noexcept OPENTTD_SERVICE_DEFAULT(industry_legend);
	void (*linkstats_legend)() noexcept OPENTTD_SERVICE_DEFAULT(linkstats_legend);
	void (*activate_shore)() noexcept OPENTTD_SERVICE_DEFAULT(activate_shore);
	void (*activate_tram_depot)() noexcept OPENTTD_SERVICE_DEFAULT(activate_tram_depot);
	void (*aging_stops)() noexcept OPENTTD_SERVICE_DEFAULT(aging_stops);
	void (*finalise_prices)() noexcept OPENTTD_SERVICE_DEFAULT(finalise_prices);
	Subdirectory baseset_dir;
	Subdirectory newgrf_dir;
	void (*assert_override_size)(size_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(assert_override_size);
	void (*assert_file_match)(const GRFConfig *, const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(assert_file_match);
	StringID null_error;
	StringID undefined;
};
extern "C" {
void openttd_rust_grf_unsafe(const OpenTTDGrfLoadingServices *);
GRFError *openttd_rust_grf_disable(const OpenTTDGrfLoadingServices *, StringID, GRFConfig *);
void openttd_rust_grf_load(const OpenTTDGrfLoadingServices *, SpriteID, uint32_t);
void openttd_rust_grf_reset(const OpenTTDGrfLoadingServices *);
uint8_t openttd_rust_grf_reserved_sprite_abi(uint32_t, SpriteID, uint16_t);
extern "C++" inline bool openttd_rust_grf_reserved_sprite(uint32_t a0, SpriteID a1, uint16_t a2) { return openttd_rust_grf_reserved_sprite_abi(a0, a1, a2) != 0; }
void openttd_rust_grf_load_file(const OpenTTDGrfLoadingServices *, GRFConfig *, GrfLoadingStage, Subdirectory, void *);
}
const OpenTTDGrfLoadingServices &GetRustGrfLoadingServices() noexcept;
#endif
