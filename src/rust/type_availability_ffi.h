/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file type_availability_ffi.h Paired introduction and construction availability. */
#ifndef RUST_TYPE_AVAILABILITY_FFI_H
#define RUST_TYPE_AVAILABILITY_FFI_H
#include "engine_ffi.h"
#include "../rail_type.h"
#include "../road_type.h"
struct OpenTTDTypeIntroduction { uint32_t label; int32_t date; uint64_t required, introduces; };
struct OpenTTDRoadTypeConstruction { uint32_t label; int32_t date; bool hidden, town_build; };
/* Native registries stay canonical. These immutable typed leaves neither reset
 * pools nor yield. Engine lifecycle scalar reads end before every callback.
 * Panics and environmental exceptions abort; ordinary play has no throwing leaf. */
struct OpenTTDTypeAvailabilityServices {
	OpenTTDEngineRef (*next)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next);
	OpenTTDEngineRailAvailability (*rail_availability)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(rail_availability);
	OpenTTDEngineRoadAvailability (*road_availability)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(road_availability);
	uint64_t (*engine_railtypes)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_railtypes);
	RoadType (*engine_roadtype)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_roadtype);
	OpenTTDTypeIntroduction (*rail_introduction)(RailType) noexcept OPENTTD_SERVICE_DEFAULT(rail_introduction);
	OpenTTDTypeIntroduction (*road_introduction)(RoadType) noexcept OPENTTD_SERVICE_DEFAULT(road_introduction);
	uint64_t (*rail_introduces)(RailType) noexcept OPENTTD_SERVICE_DEFAULT(rail_introduces);
	uint64_t (*road_introduces)(RoadType) noexcept OPENTTD_SERVICE_DEFAULT(road_introduces);
	OpenTTDRoadTypeConstruction (*road_construction)(RoadType) noexcept OPENTTD_SERVICE_DEFAULT(road_construction);
	uint64_t (*rail_hidden)() noexcept OPENTTD_SERVICE_DEFAULT(rail_hidden);
	uint64_t (*road_hidden)() noexcept OPENTTD_SERVICE_DEFAULT(road_hidden);
	uint64_t (*road_tram_mask)(RoadTramType) noexcept OPENTTD_SERVICE_DEFAULT(road_tram_mask);
	uint64_t (*company_railtypes)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(company_railtypes);
	uint64_t (*company_roadtypes)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(company_roadtypes);
	void (*set_company_railtypes)(uint8_t, uint64_t) noexcept OPENTTD_SERVICE_DEFAULT(set_company_railtypes);
	void (*set_company_roadtypes)(uint8_t, uint64_t) noexcept OPENTTD_SERVICE_DEFAULT(set_company_roadtypes);
	bool (*editor)() noexcept OPENTTD_SERVICE_DEFAULT(editor);
	bool assertions;
};
const OpenTTDTypeAvailabilityServices &GetRustTypeAvailabilityServices() noexcept;
extern "C" {
bool openttd_rust_has_rail_type(const OpenTTDTypeAvailabilityServices *, uint8_t, RailType);
bool openttd_rust_has_any_rail_types(const OpenTTDTypeAvailabilityServices *, uint8_t);
bool openttd_rust_val_rail_type(const OpenTTDSharedServices *, RailType);
uint64_t openttd_rust_add_date_rail_types(const OpenTTDTypeAvailabilityServices *, uint64_t, int32_t);
uint64_t openttd_rust_company_rail_types(const OpenTTDSharedServices *, const OpenTTDTypeAvailabilityServices *, uint8_t, bool);
uint64_t openttd_rust_rail_types(const OpenTTDSharedServices *, const OpenTTDTypeAvailabilityServices *, bool);
bool openttd_rust_has_road_type(const OpenTTDSharedServices *, const OpenTTDTypeAvailabilityServices *, uint8_t, RoadType);
bool openttd_rust_has_any_road_types(const OpenTTDTypeAvailabilityServices *, uint8_t, RoadTramType);
bool openttd_rust_val_road_type(const OpenTTDSharedServices *, RoadType);
uint64_t openttd_rust_add_date_road_types(const OpenTTDTypeAvailabilityServices *, uint64_t, int32_t);
uint64_t openttd_rust_company_road_types(const OpenTTDSharedServices *, const OpenTTDTypeAvailabilityServices *, uint8_t, bool);
uint64_t openttd_rust_road_types(const OpenTTDSharedServices *, const OpenTTDTypeAvailabilityServices *, bool);
}
#endif
