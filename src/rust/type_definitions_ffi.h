/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file type_definitions_ffi.h Typed storage operations for rail/road Action0. */
#ifndef RUST_TYPE_DEFINITIONS_FFI_H
#define RUST_TYPE_DEFINITIONS_FFI_H
#include "type_registry_ffi.h"
#include "newgrf_bytereader_ffi.h"
struct OpenTTDRailProperties {
	void (*toolbar_caption)(RailTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(toolbar_caption);
	void (*menu_text)(RailTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(menu_text);
	void (*build_caption)(RailTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(build_caption);
	void (*replace_text)(RailTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(replace_text);
	void (*name)(RailTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(name);
	void (*cost_multiplier)(RailTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(cost_multiplier);
	void (*max_speed)(RailTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(max_speed);
	void (*map_colour)(RailTypeInfo *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(map_colour);
	void (*introduction_date)(RailTypeInfo *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(introduction_date);
	void (*sorting_order)(RailTypeInfo *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(sorting_order);
	void (*maintenance_multiplier)(RailTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(maintenance_multiplier);
	void (*alternate_label)(RailTypeInfo *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(alternate_label);
	void (*new_engine)(RailTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(new_engine);
	void (*flags)(RailTypeInfo *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(flags);
	void (*powered)(RailTypeInfo *, RailType) noexcept OPENTTD_SERVICE_DEFAULT(powered);
	void (*required)(RailTypeInfo *, RailType) noexcept OPENTTD_SERVICE_DEFAULT(required);
	void (*introduces)(RailTypeInfo *, RailType) noexcept OPENTTD_SERVICE_DEFAULT(introduces);
	void (*compatible)(RailTypeInfo *, RailType) noexcept OPENTTD_SERVICE_DEFAULT(compatible);
	void (*curve_speed)(RailTypeInfo *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(curve_speed);
	void (*fallback)(RailTypeInfo *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(fallback);
	void (*acceleration)(RailTypeInfo *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(acceleration);
	std::vector<BadgeID> *(*badges)(RailTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(badges);
	void (*invalid_reserve)(uint32_t, size_t) noexcept OPENTTD_SERVICE_DEFAULT(invalid_reserve);
	void (*invalid_change)(uint32_t, size_t) noexcept OPENTTD_SERVICE_DEFAULT(invalid_change);
	void (*no_label)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(no_label);
};
extern "C" int32_t openttd_rust_rail_change(const OpenTTDRailProperties *, const OpenTTDByteReaderServices *, OpenTTDTypeMap, uint8_t, uint32_t, uint32_t, int32_t, ByteReader *);
extern "C" int32_t openttd_rust_rail_reserve(const OpenTTDRailProperties *, const OpenTTDByteReaderServices *, OpenTTDTypeMap, uint32_t, uint32_t, int32_t, ByteReader *);
struct OpenTTDRoadProperties {
	void (*toolbar_caption)(RoadTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(toolbar_caption);
	void (*menu_text)(RoadTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(menu_text);
	void (*build_caption)(RoadTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(build_caption);
	void (*replace_text)(RoadTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(replace_text);
	void (*name)(RoadTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(name);
	void (*cost_multiplier)(RoadTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(cost_multiplier);
	void (*max_speed)(RoadTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(max_speed);
	void (*map_colour)(RoadTypeInfo *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(map_colour);
	void (*introduction_date)(RoadTypeInfo *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(introduction_date);
	void (*sorting_order)(RoadTypeInfo *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(sorting_order);
	void (*maintenance_multiplier)(RoadTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(maintenance_multiplier);
	void (*alternate_label)(RoadTypeInfo *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(alternate_label);
	void (*new_engine)(RoadTypeInfo *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(new_engine);
	void (*flags)(RoadTypeInfo *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(flags);
	void (*powered)(RoadTypeInfo *, RoadType) noexcept OPENTTD_SERVICE_DEFAULT(powered);
	void (*required)(RoadTypeInfo *, RoadType) noexcept OPENTTD_SERVICE_DEFAULT(required);
	void (*introduces)(RoadTypeInfo *, RoadType) noexcept OPENTTD_SERVICE_DEFAULT(introduces);
	std::vector<BadgeID> *(*badges)(RoadTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(badges);
	void (*invalid_reserve)(uint32_t, size_t) noexcept OPENTTD_SERVICE_DEFAULT(invalid_reserve);
	void (*invalid_change)(uint32_t, size_t) noexcept OPENTTD_SERVICE_DEFAULT(invalid_change);
	void (*no_label)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(no_label);
	void (*reserve_mismatch)(uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(reserve_mismatch);
	void (*power_mismatch)(uint8_t resolved, uint8_t current) noexcept OPENTTD_SERVICE_DEFAULT(power_mismatch);
};
extern "C" int32_t openttd_rust_road_change(const OpenTTDRoadProperties *, const OpenTTDByteReaderServices *, OpenTTDTypeMap, uint32_t, uint32_t, int32_t, ByteReader *, RoadTramType);
extern "C" int32_t openttd_rust_road_reserve(const OpenTTDRoadProperties *, const OpenTTDByteReaderServices *, OpenTTDTypeMap, uint32_t, uint32_t, int32_t, ByteReader *, RoadTramType);
#endif
