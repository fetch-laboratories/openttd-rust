/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file type_registry_ffi.h The rail and road type registry: the tables C++ fills for
 * `type_registry.rs` and `type_definitions.rs`, and the registry's exports.
 * NewGRF loading resets and rebuilds the types outside the game loop and command execution.
 * The sorted-list spans alias Rust's buffers until the next rebuild republishes them, and the
 * four mask references alias Rust's statics. No Rust borrow crosses a callback. */
#ifndef RUST_TYPE_REGISTRY_FFI_H
#define RUST_TYPE_REGISTRY_FFI_H
#include "services_ffi.h"
#include "../rail_type.h"
#include "../road_type.h"
#include "../newgrf_badge_type.h"
#include "../strings_type.h"
#include <vector>
class RailTypeInfo;
class RoadTypeInfo;
struct GRFFile;
struct OpenTTDSharedServices;
/* The registry tables. Each element pointer is one `spec` returned (`&_railtypes[rt]`,
 * `&_roadtypes[rt]`); a member store writes that element in place. */
struct OpenTTDRailRegistryServices {
	void (*clear)(RailTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(clear);
	void (*copy_default)(RailTypeInfo *, RailType) noexcept OPENTTD_SERVICE_DEFAULT(copy_default);
	uint8_t default_count;
	void (*resolve_rail_type_gui_sprites)(RailTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(resolve_rail_type_gui_sprites);
	void (*set_sorted_railtypes)(RailType *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(set_sorted_railtypes);
	void (*sort)(RailType *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(sort);
	RailTypeInfo *(*spec)(RailType) noexcept OPENTTD_SERVICE_DEFAULT(spec);
	void (*alternate_labels_clear)(RailTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(alternate_labels_clear);
	bool (*alternate_labels_contains)(const RailTypeInfo *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(alternate_labels_contains);
	void (*alternate_labels_insert)(RailTypeInfo *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(alternate_labels_insert);
	std::vector<BadgeID> *(*badges)(RailTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(badges);
	void (*compatible_railtypes_set)(RailTypeInfo *, RailType) noexcept OPENTTD_SERVICE_DEFAULT(compatible_railtypes_set);
	void (*introduces_railtypes_set)(RailTypeInfo *, RailType) noexcept OPENTTD_SERVICE_DEFAULT(introduces_railtypes_set);
	void (*introduction_required_railtypes_set)(RailTypeInfo *, RailType) noexcept OPENTTD_SERVICE_DEFAULT(introduction_required_railtypes_set);
	void (*powered_railtypes_set)(RailTypeInfo *, RailType) noexcept OPENTTD_SERVICE_DEFAULT(powered_railtypes_set);
	void (*set_acceleration_type)(RailTypeInfo *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_acceleration_type);
	void (*set_compatible_railtypes)(RailTypeInfo *, uint64_t) noexcept OPENTTD_SERVICE_DEFAULT(set_compatible_railtypes);
	void (*set_cost_multiplier)(RailTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_cost_multiplier);
	void (*set_curve_speed)(RailTypeInfo *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_curve_speed);
	void (*set_fallback_railtype)(RailTypeInfo *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_fallback_railtype);
	void (*set_flags)(RailTypeInfo *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_flags);
	void (*set_introduces_railtypes)(RailTypeInfo *, uint64_t) noexcept OPENTTD_SERVICE_DEFAULT(set_introduces_railtypes);
	void (*set_introduction_date)(RailTypeInfo *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_introduction_date);
	void (*set_label)(RailTypeInfo *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_label);
	void (*set_maintenance_multiplier)(RailTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_maintenance_multiplier);
	void (*set_map_colour)(RailTypeInfo *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_map_colour);
	void (*set_max_speed)(RailTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_max_speed);
	void (*set_powered_railtypes)(RailTypeInfo *, uint64_t) noexcept OPENTTD_SERVICE_DEFAULT(set_powered_railtypes);
	void (*set_sorting_order)(RailTypeInfo *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_sorting_order);
	StringID *(*strings_build_caption)(RailTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(strings_build_caption);
	StringID *(*strings_menu_text)(RailTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(strings_menu_text);
	StringID *(*strings_name)(RailTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(strings_name);
	StringID *(*strings_new_loco)(RailTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(strings_new_loco);
	StringID *(*strings_replace_text)(RailTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(strings_replace_text);
	StringID *(*strings_toolbar_caption)(RailTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(strings_toolbar_caption);
};
struct OpenTTDRoadRegistryServices {
	void (*clear)(RoadTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(clear);
	void (*copy_default)(RoadTypeInfo *, RoadType) noexcept OPENTTD_SERVICE_DEFAULT(copy_default);
	uint8_t default_count;
	void (*resolve_road_type_gui_sprites)(RoadTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(resolve_road_type_gui_sprites);
	void (*set_sorted_roadtypes)(RoadType *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(set_sorted_roadtypes);
	void (*sort)(RoadType *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(sort);
	RoadTypeInfo *(*spec)(RoadType) noexcept OPENTTD_SERVICE_DEFAULT(spec);
	void (*alternate_labels_clear)(RoadTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(alternate_labels_clear);
	bool (*alternate_labels_contains)(const RoadTypeInfo *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(alternate_labels_contains);
	void (*alternate_labels_insert)(RoadTypeInfo *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(alternate_labels_insert);
	std::vector<BadgeID> *(*badges)(RoadTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(badges);
	void (*introduces_roadtypes_set)(RoadTypeInfo *, RoadType) noexcept OPENTTD_SERVICE_DEFAULT(introduces_roadtypes_set);
	void (*introduction_required_roadtypes_set)(RoadTypeInfo *, RoadType) noexcept OPENTTD_SERVICE_DEFAULT(introduction_required_roadtypes_set);
	void (*powered_roadtypes_set)(RoadTypeInfo *, RoadType) noexcept OPENTTD_SERVICE_DEFAULT(powered_roadtypes_set);
	void (*set_cost_multiplier)(RoadTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_cost_multiplier);
	void (*set_flags)(RoadTypeInfo *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_flags);
	void (*set_introduces_roadtypes)(RoadTypeInfo *, uint64_t) noexcept OPENTTD_SERVICE_DEFAULT(set_introduces_roadtypes);
	void (*set_introduction_date)(RoadTypeInfo *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_introduction_date);
	void (*set_label)(RoadTypeInfo *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_label);
	void (*set_maintenance_multiplier)(RoadTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_maintenance_multiplier);
	void (*set_map_colour)(RoadTypeInfo *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_map_colour);
	void (*set_max_speed)(RoadTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_max_speed);
	void (*set_powered_roadtypes)(RoadTypeInfo *, uint64_t) noexcept OPENTTD_SERVICE_DEFAULT(set_powered_roadtypes);
	void (*set_sorting_order)(RoadTypeInfo *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_sorting_order);
	StringID *(*strings_build_caption)(RoadTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(strings_build_caption);
	StringID *(*strings_menu_text)(RoadTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(strings_menu_text);
	StringID *(*strings_name)(RoadTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(strings_name);
	StringID *(*strings_new_engine)(RoadTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(strings_new_engine);
	StringID *(*strings_replace_text)(RoadTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(strings_replace_text);
	StringID *(*strings_toolbar_caption)(RoadTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(strings_toolbar_caption);
};
struct OpenTTDTypeRegistryServices {
	const OpenTTDRailRegistryServices *rail;
	const OpenTTDRoadRegistryServices *road;
	const OpenTTDSharedServices *shared;
};
const OpenTTDTypeRegistryServices &GetRustTypeRegistryServices() noexcept;
const OpenTTDRailRegistryServices &GetRustRailRegistryServices() noexcept;
const OpenTTDRoadRegistryServices &GetRustRoadRegistryServices() noexcept;
extern "C" {
uint64_t *openttd_rust_rail_hidden_mask();
uint64_t *openttd_rust_road_hidden_mask();
uint64_t *openttd_rust_road_mask();
uint64_t *openttd_rust_tram_mask();
RailType openttd_rust_rail_by_label(uint32_t, bool);
RoadType openttd_rust_road_by_label(uint32_t, bool);
RailType openttd_rust_rail_translate(uint8_t, const GRFFile *);
RoadType openttd_rust_road_translate(RoadTramType, uint8_t, const GRFFile *);
uint8_t openttd_rust_rail_reverse(RailType, const GRFFile *);
uint8_t openttd_rust_road_reverse(RoadType, const GRFFile *);
void openttd_rust_rail_saved_convert(const OpenTTDSharedServices *, const OpenTTDRailMapServices *, uint32_t);
void openttd_rust_road_saved_convert(const OpenTTDSharedServices *, uint32_t);
void openttd_rust_rail_saved_reserve();
void openttd_rust_road_saved_reserve();
void openttd_rust_rail_saved_append(uint32_t, uint8_t);
void openttd_rust_road_saved_append(uint32_t, uint8_t);
void openttd_rust_rail_saved_clear();
void openttd_rust_road_saved_clear();
void openttd_rust_rail_saved_current();
void openttd_rust_road_saved_current();
}
#endif
