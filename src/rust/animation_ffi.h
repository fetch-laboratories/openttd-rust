/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file animation_ffi.h Typed native leaves for the sole Rust animation owner. */
#ifndef RUST_ANIMATION_FFI_H
#define RUST_ANIMATION_FFI_H
#include "services_ffi.h"
#include "../animated_tile_map.h"
#include "../tile_cmd.h"
#include "town_ffi.h"
#include "industry_ffi.h"
struct OpenTTDAirportResolverServices;
struct HouseSpec; struct Town; struct IndustryTileSpec; struct Industry;
struct StationSpec; struct BaseStation; struct AirportTileSpec; struct Station;
struct ObjectSpec; struct Object; struct RoadStopSpec;
struct OpenTTDAnimationInfo { uint8_t speed, frames; bool looping, speed_callback, next_callback, random, enabled; };
template <typename Spec, typename Obj> struct OpenTTDAnimationEntry { const Spec *spec; Obj *object; };
#include "animation_types.hpp"
using OpenTTDHouseAnimation = OpenTTDAnimationBinding<HouseSpec, Town, uint64_t>;
using OpenTTDIndustryAnimation = OpenTTDAnimationBinding<IndustryTileSpec, Industry, int32_t>;
using OpenTTDStationAnimation = OpenTTDAnimationBinding<StationSpec, BaseStation, int32_t>;
using OpenTTDObjectAnimation = OpenTTDAnimationBinding<ObjectSpec, Object, int32_t>;
using OpenTTDRoadStopAnimation = OpenTTDAnimationBinding<RoadStopSpec, BaseStation, int32_t>;
using OpenTTDHouseAnimationEntry = OpenTTDAnimationEntry<HouseSpec, Town>;
using OpenTTDIndustryAnimationEntry = OpenTTDAnimationEntry<IndustryTileSpec, Industry>;
using OpenTTDStationAnimationEntry = OpenTTDAnimationEntry<StationSpec, BaseStation>;
using OpenTTDObjectAnimationEntry = OpenTTDAnimationEntry<ObjectSpec, Object>;
using OpenTTDRoadStopAnimationEntry = OpenTTDAnimationEntry<RoadStopSpec, BaseStation>;
struct OpenTTDAnimationArea { uint32_t tile; uint16_t width, height; };
struct OpenTTDAnimationServices {
	const OpenTTDSharedServices *shared;
	BaseStation *(*station_base)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(station_base);
	Station *(*as_station)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(as_station);
	uint32_t (*location)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(location);
	bool (*is_station)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(is_station);
	void (*road_randomisation)(BaseStation *, uint32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(road_randomisation);
	const OpenTTDTownLeaves *town;
	const OpenTTDIndustryServices *industries;
	const OpenTTDIndustryConstructionServices *construction;
	const OpenTTDIndustryTileServices *industry_tiles;
	const OpenTTDHouseAnimation *house;
	const OpenTTDIndustryAnimation *industry;
	const OpenTTDStationAnimation *station;
	const OpenTTDAirportAnimation *airport;
	const OpenTTDAirportResolverServices *airport_runtime;
	const OpenTTDObjectAnimation *object;
	const OpenTTDRoadStopAnimation *roadstop;
	AnimatedTileState (*state)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(state);
	void (*set_state)(uint32_t, AnimatedTileState) noexcept OPENTTD_SERVICE_DEFAULT(set_state);
	const HouseSpec *(*house_spec)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(house_spec);
	uint8_t (*road_frame)(BaseStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(road_frame);
	bool (*set_road_frame)(BaseStation *, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_road_frame);
	OpenTTDAnimationArea (*rail_area)(BaseStation *, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_area);
	bool (*rail_belongs)(BaseStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_belongs);
	bool (*station_triggers)(BaseStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(station_triggers);
	bool (*road_triggers)(BaseStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(road_triggers);
	const StationSpec *(*rail_spec)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_spec);
	const RoadStopSpec *(*road_spec)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(road_spec);
	Industry *(*industry_object)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(industry_object);
	uint16_t (*rail_spec_triggers)(const StationSpec *) noexcept OPENTTD_SERVICE_DEFAULT(rail_spec_triggers);
	uint16_t (*road_spec_triggers)(const RoadStopSpec *) noexcept OPENTTD_SERVICE_DEFAULT(road_spec_triggers);
	uint8_t (*airport_spec_triggers)(const AirportTileSpec *) noexcept OPENTTD_SERVICE_DEFAULT(airport_spec_triggers);
	uint8_t (*rail_cargo)(const StationSpec *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_cargo);
	uint8_t (*road_cargo)(const RoadStopSpec *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(road_cargo);
	uint8_t (*airport_cargo)(const AirportTileSpec *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(airport_cargo);
	bool (*airport_grf)(const AirportTileSpec *) noexcept OPENTTD_SERVICE_DEFAULT(airport_grf);
	OpenTTDAnimationArea (*airport_area)(Station *) noexcept OPENTTD_SERVICE_DEFAULT(airport_area);
	bool (*airport_belongs)(const Station *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(airport_belongs);
	size_t (*road_count)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(road_count);
	uint32_t (*road_tile)(BaseStation *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(road_tile);
};
OpenTTDAnimationArea OpenTTDAnimationRailArea(BaseStation *, uint32_t, uint8_t) noexcept;
const OpenTTDIndustryConstructionServices &GetIndustryConstructionServices();
const OpenTTDHouseAnimation &GetHouseAnimationBinding() noexcept;
const OpenTTDIndustryAnimation &GetIndustryAnimationBinding() noexcept;
const OpenTTDStationAnimation &GetStationAnimationBinding() noexcept;
const OpenTTDAirportAnimation &GetAirportAnimationBinding() noexcept;
const OpenTTDObjectAnimation &GetObjectAnimationBinding() noexcept;
const OpenTTDRoadStopAnimation &GetRoadStopAnimationBinding() noexcept;
extern "C" {
const OpenTTDAnimationServices *openttd_rust_animation_exchange(const OpenTTDAnimationServices *);
void openttd_rust_animation_add(uint32_t, bool);
void openttd_rust_animation_delete(uint32_t, bool);
void openttd_rust_animation_run();
void openttd_rust_animation_initialize();
size_t openttd_rust_animation_count();
uint32_t openttd_rust_animation_at(size_t);
void openttd_rust_animation_append(uint32_t);
void openttd_rust_animation_afterload(bool, bool);
void openttd_rust_animate_house(uint32_t);
void openttd_rust_animate_industry(uint32_t);
void openttd_rust_animate_station(uint32_t);
void openttd_rust_animate_airport(uint32_t);
void openttd_rust_animate_object(uint32_t);
void openttd_rust_animate_roadstop(uint32_t);
void openttd_rust_trigger_station(BaseStation *, uint32_t, uint8_t, uint8_t);
void openttd_rust_trigger_roadstop(BaseStation *, uint32_t, uint8_t, uint8_t);
bool openttd_rust_trigger_airport_tile(Station *, uint32_t, uint8_t);
bool openttd_rust_trigger_airport(Station *, uint8_t, uint8_t);
}
#endif /* RUST_ANIMATION_FFI_H */
