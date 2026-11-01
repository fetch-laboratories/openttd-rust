/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file industry_property_ffi.h Native industry property storage leaves. */
#ifndef RUST_INDUSTRY_PROPERTY_FFI_H
#define RUST_INDUSTRY_PROPERTY_FFI_H
#include "industry_definition_ffi.h"
#include "../industrytype.h"
#include "newgrf_bytereader_ffi.h"
struct OpenTTDGrfLoadingServices;
struct OpenTTDIndustryDefinitionLayout { const IndustryTileLayoutTile *data; size_t size; };
struct OpenTTDIndustryDefinitionLayoutBuffer { const IndustryTileLayoutTile *data; size_t size, capacity; };
struct OpenTTDIndustryPropertyServices {
	void (*resize_industries)(GRFFile *file, size_t size) noexcept OPENTTD_SERVICE_DEFAULT(resize_industries);
	void (*resize_tiles)(GRFFile *file, size_t size) noexcept OPENTTD_SERVICE_DEFAULT(resize_tiles);
	IndustrySpec * (*create_industry)(GRFFile *file, uint16_t id, uint8_t substitute) noexcept OPENTTD_SERVICE_DEFAULT(create_industry);
	IndustryTileSpec * (*create_tile)(GRFFile *file, uint16_t id, uint8_t substitute) noexcept OPENTTD_SERVICE_DEFAULT(create_tile);
	void (*industry_identity)(IndustrySpec *spec, GRFFile *file, uint16_t id, uint8_t substitute) noexcept OPENTTD_SERVICE_DEFAULT(industry_identity);
	void (*tile_identity)(IndustryTileSpec *spec, GRFFile *file, uint16_t id, uint8_t substitute) noexcept OPENTTD_SERVICE_DEFAULT(tile_identity);
	void (*clear_placement_check)(IndustrySpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(clear_placement_check);
	void (*clear_animation)(IndustryTileSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(clear_animation);
	uint16_t (*callbacks)(const IndustrySpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(callbacks);
	void (*set_rate)(IndustrySpec *spec, size_t slot, uint8_t rate) noexcept OPENTTD_SERVICE_DEFAULT(set_rate);
	void (*set_multiplier)(IndustrySpec *spec, size_t input, size_t output, uint16_t multiplier) noexcept OPENTTD_SERVICE_DEFAULT(set_multiplier);
	void (*set_conflicting)(IndustrySpec *spec, size_t slot, uint8_t id) noexcept OPENTTD_SERVICE_DEFAULT(set_conflicting);
	void (*set_creation)(IndustrySpec *spec, uint8_t climate, uint8_t chance) noexcept OPENTTD_SERVICE_DEFAULT(set_creation);
	void (*set_ingame)(IndustrySpec *spec, uint8_t climate, uint8_t chance) noexcept OPENTTD_SERVICE_DEFAULT(set_ingame);
	void (*invalidate_produced)(IndustrySpec *spec, size_t slot) noexcept OPENTTD_SERVICE_DEFAULT(invalidate_produced);
	void (*invalidate_accepted)(IndustrySpec *spec, size_t slot) noexcept OPENTTD_SERVICE_DEFAULT(invalidate_accepted);
	void (*invalidate_tile_accepted)(IndustryTileSpec *spec, size_t slot) noexcept OPENTTD_SERVICE_DEFAULT(invalidate_tile_accepted);
	void (*set_acceptance)(IndustryTileSpec *spec, size_t slot, int8_t amount) noexcept OPENTTD_SERVICE_DEFAULT(set_acceptance);
	void (*set_sounds)(IndustrySpec *spec, const uint8_t *sounds, size_t count) noexcept OPENTTD_SERVICE_DEFAULT(set_sounds);
	void (*clear_station_name)(IndustrySpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(clear_station_name);
	void (*set_life)(IndustrySpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_life);
	void (*set_cost)(IndustrySpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_cost);
	void (*set_minimal_cargo)(IndustrySpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_minimal_cargo);
	void (*set_map_colour)(IndustrySpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_map_colour);
	void (*set_behaviour)(IndustrySpec *spec, uint32_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_behaviour);
	void (*set_prospecting)(IndustrySpec *spec, uint32_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_prospecting);
	void (*set_removal_cost)(IndustrySpec *spec, uint32_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_removal_cost);
	void (*set_callbacks)(IndustrySpec *spec, uint16_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_callbacks);
	void (*set_tile_slopes)(IndustryTileSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_tile_slopes);
	void (*set_tile_callbacks)(IndustryTileSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_tile_callbacks);
	void (*set_tile_flags)(IndustryTileSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_tile_flags);
	void (*map_closure_text)(IndustrySpec *spec, uint16_t string) noexcept OPENTTD_SERVICE_DEFAULT(map_closure_text);
	void (*map_production_up_text)(IndustrySpec *spec, uint16_t string) noexcept OPENTTD_SERVICE_DEFAULT(map_production_up_text);
	void (*map_production_down_text)(IndustrySpec *spec, uint16_t string) noexcept OPENTTD_SERVICE_DEFAULT(map_production_down_text);
	void (*map_new_industry_text)(IndustrySpec *spec, uint16_t string) noexcept OPENTTD_SERVICE_DEFAULT(map_new_industry_text);
	void (*map_name)(IndustrySpec *spec, uint16_t string) noexcept OPENTTD_SERVICE_DEFAULT(map_name);
	void (*map_station_name)(IndustrySpec *spec, uint16_t string) noexcept OPENTTD_SERVICE_DEFAULT(map_station_name);
	size_t (*original_layout_count)(uint8_t type) noexcept OPENTTD_SERVICE_DEFAULT(original_layout_count);
	OpenTTDIndustryDefinitionLayout (*original_layout)(uint8_t type, uint8_t layout) noexcept OPENTTD_SERVICE_DEFAULT(original_layout);
	void (*install_layouts)(IndustrySpec *spec, const OpenTTDIndustryDefinitionLayoutBuffer *layouts, size_t count) noexcept OPENTTD_SERVICE_DEFAULT(install_layouts);
	void (*too_many_tiles)(uint32_t last) noexcept OPENTTD_SERVICE_DEFAULT(too_many_tiles);
	void (*too_many_industries)(uint32_t last) noexcept OPENTTD_SERVICE_DEFAULT(too_many_industries);
	void (*tile_substitute_invalid)(uint8_t substitute, uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(tile_substitute_invalid);
	void (*tile_override_invalid)(uint8_t override_id, uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(tile_override_invalid);
	void (*industry_substitute_invalid)(uint8_t substitute, uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(industry_substitute_invalid);
	void (*industry_override_invalid)(uint8_t override_id, uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(industry_override_invalid);
	void (*layout_size_mismatch)(uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(layout_size_mismatch);
	void (*layout_import_type)(uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(layout_import_type);
	void (*layout_import_index)(uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(layout_import_index);
	void (*layout_tile_undefined)(uint16_t tile, uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(layout_tile_undefined);
	void (*layout_invalid)(uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(layout_invalid);
};
struct OpenTTDIndustryPropertyContext {
	GRFFile *file;
	uint32_t grfid;
	uint8_t version, climate;
};
extern "C" bool openttd_rust_industry_properties(uint32_t, uint32_t, int32_t, ByteReader *, const OpenTTDIndustryPropertyContext *, const OpenTTDByteReaderServices *, const OpenTTDIndustryDefinitionServices *, const OpenTTDIndustryPropertyServices *, const OpenTTDGrfLoadingServices *, int32_t *);
extern "C" bool openttd_rust_industry_tile_properties(uint32_t, uint32_t, int32_t, ByteReader *, const OpenTTDIndustryPropertyContext *, const OpenTTDByteReaderServices *, const OpenTTDIndustryDefinitionServices *, const OpenTTDIndustryPropertyServices *, const OpenTTDGrfLoadingServices *, int32_t *);
const OpenTTDIndustryPropertyServices &GetIndustryPropertyServices() noexcept;
#endif
