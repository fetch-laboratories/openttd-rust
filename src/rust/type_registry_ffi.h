/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file type_registry_ffi.h Rail/road definition ownership and native storage leaves.
 * Serialized loading/reset excludes runtime access. Native specs and file vectors
 * outlive synchronous calls; published spans alias Rust until the next rebuild.
 * No Rust borrow crosses a callback. Native save/parser error frames retain
 * ordinary exceptions. Environmental leaf failures terminate; Rust panics abort. */
#ifndef RUST_TYPE_REGISTRY_FFI_H
#define RUST_TYPE_REGISTRY_FFI_H
#include "services_ffi.h"
#include "../rail_type.h"
#include "../road_type.h"
class RailTypeInfo;
class RoadTypeInfo;
struct GRFFile;
struct OpenTTDTypePublication { uint32_t label; bool hidden; };
struct OpenTTDTypeAllocation { uint32_t label; uint64_t self; uint8_t order; };
struct OpenTTDTypeLabelTable { uint32_t values[RAILTYPE_END]; };
struct OpenTTDTypeLabels { const uint32_t *data; size_t size; };
struct OpenTTDTypeMap { uint8_t *data; size_t size; };
struct OpenTTDRailRegistryServices {
	uint8_t default_count;
	RailTypeInfo * (*spec)(RailType) noexcept OPENTTD_SERVICE_DEFAULT(spec);
	RailTypeLabel (*type_label)(RailType) noexcept OPENTTD_SERVICE_DEFAULT(type_label);
	OpenTTDTypeLabelTable (*labels)() noexcept OPENTTD_SERVICE_DEFAULT(labels);
	bool (*alternate)(RailType, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(alternate);
	void (*clear)(RailTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(clear);
	void (*copy_default)(RailTypeInfo *, RailType) noexcept OPENTTD_SERVICE_DEFAULT(copy_default);
	void (*resolve_gui)(RailTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(resolve_gui);
	OpenTTDTypePublication (*publication)(const RailTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(publication);
	void (*sort)(RailType *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(sort);
	void (*publish_sorted)(RailType *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(publish_sorted);
	void (*allocate)(RailTypeInfo *, const OpenTTDTypeAllocation *) noexcept OPENTTD_SERVICE_DEFAULT(allocate);
};
struct OpenTTDRoadRegistryServices {
	uint8_t default_count;
	RoadTypeInfo * (*spec)(RoadType) noexcept OPENTTD_SERVICE_DEFAULT(spec);
	RoadTypeLabel (*type_label)(RoadType) noexcept OPENTTD_SERVICE_DEFAULT(type_label);
	OpenTTDTypeLabelTable (*labels)() noexcept OPENTTD_SERVICE_DEFAULT(labels);
	bool (*alternate)(RoadType, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(alternate);
	void (*clear)(RoadTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(clear);
	void (*copy_default)(RoadTypeInfo *, RoadType) noexcept OPENTTD_SERVICE_DEFAULT(copy_default);
	void (*resolve_gui)(RoadTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(resolve_gui);
	OpenTTDTypePublication (*publication)(const RoadTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(publication);
	void (*sort)(RoadType *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(sort);
	void (*publish_sorted)(RoadType *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(publish_sorted);
	void (*allocate)(RoadTypeInfo *, const OpenTTDTypeAllocation *) noexcept OPENTTD_SERVICE_DEFAULT(allocate);
};
struct OpenTTDTypeRegistryServices {
	const OpenTTDRailRegistryServices *rail;
	const OpenTTDRoadRegistryServices *road;
	OpenTTDTypeLabels (*rail_labels)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(rail_labels);
	OpenTTDTypeLabels (*road_labels)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(road_labels);
	OpenTTDTypeLabels (*tram_labels)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(tram_labels);
	OpenTTDTypeMap (*rail_map)(GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(rail_map);
	OpenTTDTypeMap (*road_map)(GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(road_map);
	OpenTTDTypeMap (*tram_map)(GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(tram_map);
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
