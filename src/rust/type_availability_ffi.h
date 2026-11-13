/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file type_availability_ffi.h Paired introduction and construction availability. */
#ifndef RUST_TYPE_AVAILABILITY_FFI_H
#define RUST_TYPE_AVAILABILITY_FFI_H
#include "engine_ffi.h"
#include "../rail_type.h"
#include "../road_type.h"
struct OpenTTDTypeIntroduction { uint32_t label; int32_t date; uint64_t required, introduces; };
class RoadTypeInfo;
/* Native TypeInfo storage supplies immutable metadata. These leaves neither reset
 * pools nor yield. Engine lifecycle scalar reads end before every callback.
 * Panics and environmental exceptions abort; ordinary play has no throwing leaf. */
struct OpenTTDTypeAvailabilityServices {
	bool assertions;
	uint64_t (*company_railtypes)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(company_railtypes);
	uint64_t (*company_roadtypes)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(company_roadtypes);
	bool (*editor)() noexcept OPENTTD_SERVICE_DEFAULT(editor);
	OpenTTDEngineRef (*next)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next);
	uint64_t (*rail_introduces)(RailType) noexcept OPENTTD_SERVICE_DEFAULT(rail_introduces);
	OpenTTDTypeIntroduction (*rail_introduction)(RailType) noexcept OPENTTD_SERVICE_DEFAULT(rail_introduction);
	uint8_t (*road_flags)(const RoadTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(road_flags);
	const RoadTypeInfo *(*road_info)(RoadType) noexcept OPENTTD_SERVICE_DEFAULT(road_info);
	uint64_t (*road_introduces)(const RoadTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(road_introduces);
	int32_t (*road_introduction_date)(const RoadTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(road_introduction_date);
	uint64_t (*road_introduction_required)(const RoadTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(road_introduction_required);
	uint32_t (*road_label)(const RoadTypeInfo *) noexcept OPENTTD_SERVICE_DEFAULT(road_label);
	void (*set_company_railtypes)(uint8_t, uint64_t) noexcept OPENTTD_SERVICE_DEFAULT(set_company_railtypes);
	void (*set_company_roadtypes)(uint8_t, uint64_t) noexcept OPENTTD_SERVICE_DEFAULT(set_company_roadtypes);
	const OpenTTDSharedServices *shared;
};
const OpenTTDTypeAvailabilityServices &GetRustTypeAvailabilityServices() noexcept;
extern "C" {
/* Pinned Rust 1.99 can return noncanonical bool bytes for the four Has/Val bit predicates.
 * Their wire results are uint8_t 0/1; logical APIs and callbacks remain bool. */
uint8_t openttd_rust_has_rail_type(const OpenTTDTypeAvailabilityServices *, uint8_t, RailType);
uint8_t openttd_rust_has_any_rail_types_abi(const OpenTTDTypeAvailabilityServices *, uint8_t);
extern "C++" inline bool openttd_rust_has_any_rail_types(const OpenTTDTypeAvailabilityServices *a0, uint8_t a1) { return openttd_rust_has_any_rail_types_abi(a0, a1) != 0; }
uint8_t openttd_rust_val_rail_type(const OpenTTDSharedServices *, RailType);
uint64_t openttd_rust_add_date_rail_types(const OpenTTDTypeAvailabilityServices *, uint64_t, int32_t);
uint64_t openttd_rust_company_rail_types( const OpenTTDTypeAvailabilityServices *, uint8_t, bool);
uint64_t openttd_rust_rail_types( const OpenTTDTypeAvailabilityServices *, bool);
uint8_t openttd_rust_has_road_type(const OpenTTDSharedServices *, const OpenTTDTypeAvailabilityServices *, uint8_t, RoadType);
uint8_t openttd_rust_has_any_road_types_abi(const OpenTTDTypeAvailabilityServices *, uint8_t, RoadTramType);
extern "C++" inline bool openttd_rust_has_any_road_types(const OpenTTDTypeAvailabilityServices *a0, uint8_t a1, RoadTramType a2) { return openttd_rust_has_any_road_types_abi(a0, a1, a2) != 0; }
uint8_t openttd_rust_val_road_type(const OpenTTDSharedServices *, RoadType);
uint64_t openttd_rust_add_date_road_types(const OpenTTDTypeAvailabilityServices *, uint64_t, int32_t);
uint64_t openttd_rust_company_road_types( const OpenTTDTypeAvailabilityServices *, uint8_t, bool);
uint64_t openttd_rust_road_types( const OpenTTDTypeAvailabilityServices *, bool);
}
#endif
