struct OpenTTDEngineCapabilitiesServices;
/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file engine_ffi.h Engine lifecycle's synchronous typed boundary. */
#ifndef RUST_ENGINE_FFI_H
#define RUST_ENGINE_FFI_H
#include "service_defaults.hpp"
#include "services_ffi.h"
#include "engine_lifecycle.hpp"
#include "../engine_type.h"
#include "../vehicle_type.h"
struct OpenTTDEngine;
struct OpenTTDVehicle;
struct Company;
struct OpenTTDEngineRef { const OpenTTDEngine *shell; OpenTTDEngineLifecycle *state; uint16_t id; VehicleType type; };
struct OpenTTDEngineSyncVariant { uint16_t parent; bool sync; };
struct OpenTTDEngineDisplayVariant { uint16_t parent, display; };
struct OpenTTDEnginePreviewVariant { uint16_t parent; bool join; };
struct OpenTTDEngineReliability { int32_t base_life; int8_t retire_early; };
struct OpenTTDEngineStartup { int32_t base_intro; uint32_t grfid; int32_t base_life; uint8_t decay_speed; bool climate; };
struct OpenTTDEngineSeed { int32_t base_intro, lifelength; uint8_t decay_speed; int8_t retire_early; };
struct OpenTTDEngineYear { int32_t base_intro, lifelength; };
using OpenTTDEngineYMD = OpenTTDCalendarYMD;
struct OpenTTDEngineStartSettings { uint32_t generation_seed; int32_t starting_year, date; };
struct OpenTTDEngineCompany { const Company *shell; uint8_t id; };
struct OpenTTDEngineCompanyPreview { uint8_t blocked; int32_t performance; };
struct OpenTTDEngineVehicle { OpenTTDVehicle *shell; uint32_t id; uint8_t owner; VehicleType type; };
struct OpenTTDEngineVehicleCargo { const OpenTTDEngine *engine; uint8_t cargo; };
struct OpenTTDEngineCarrying { VehicleType kind; uint8_t ground_capacity, default_cargo; };
struct OpenTTDEngineCapacityConfig { uint16_t id, callbacks; uint8_t misc; };
struct OpenTTDEngineRailAvailability { bool climate; RailVehicleTypes kind; uint64_t railtypes; };
struct OpenTTDEngineRoadAvailability { bool climate; RoadType roadtype; };
/** Every callback is noexcept; script events only enqueue, no VM executes.
 * No owner borrow survives any callback, including native type availability,
 * articulated/NewGRF reads, windows, and recursively joined variants.
 * Pool traversal preserves IDs/order; prototypes arrive by shell, never ID. */
struct OpenTTDTypeAvailabilityServices;
struct OpenTTDEngineServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDRandomServices *rng;
	const OpenTTDTypeAvailabilityServices *types;
	OpenTTDEngineRef (*next)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next);
	OpenTTDEngineRef (*resolve)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(resolve);
	OpenTTDEngineSyncVariant (*sync_variant)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(sync_variant);
	OpenTTDEngineDisplayVariant (*display_variant)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(display_variant);
	OpenTTDEnginePreviewVariant (*preview_variant)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(preview_variant);
	uint8_t (*extra_flags)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(extra_flags);
	uint16_t (*engine_id)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_id);
	OpenTTDEngineReliability (*reliability)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(reliability);
	OpenTTDEngineStartup (*startup)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(startup);
	OpenTTDEngineSeed (*seed_config)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(seed_config);
	int32_t (*base_intro)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(base_intro);
	OpenTTDEngineYear (*aging_year)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(aging_year);
	int32_t (*calendar_year)() noexcept OPENTTD_SERVICE_DEFAULT(calendar_year);
	bool (*never_expire)() noexcept OPENTTD_SERVICE_DEFAULT(never_expire);
	OpenTTDEngineStartSettings (*start_settings)() noexcept OPENTTD_SERVICE_DEFAULT(start_settings);
	uint16_t (*train_limit)() noexcept OPENTTD_SERVICE_DEFAULT(train_limit);
	uint16_t (*road_limit)() noexcept OPENTTD_SERVICE_DEFAULT(road_limit);
	uint16_t (*ship_limit)() noexcept OPENTTD_SERVICE_DEFAULT(ship_limit);
	uint16_t (*aircraft_limit)() noexcept OPENTTD_SERVICE_DEFAULT(aircraft_limit);
	bool (*ai_train_disabled)() noexcept OPENTTD_SERVICE_DEFAULT(ai_train_disabled);
	bool (*ai_road_disabled)() noexcept OPENTTD_SERVICE_DEFAULT(ai_road_disabled);
	bool (*ai_ship_disabled)() noexcept OPENTTD_SERVICE_DEFAULT(ai_ship_disabled);
	bool (*ai_aircraft_disabled)() noexcept OPENTTD_SERVICE_DEFAULT(ai_aircraft_disabled);
	bool (*enabled)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(enabled);
	bool (*climate)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(climate);
	RailVehicleTypes (*rail_kind)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(rail_kind);
	void (*clear_last_variant)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(clear_last_variant);
	void (*refresh_type)(VehicleType) noexcept OPENTTD_SERVICE_DEFAULT(refresh_type);
	void (*refresh_lists)(bool) noexcept OPENTTD_SERVICE_DEFAULT(refresh_lists);
	OpenTTDEngineCompany (*company_next)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(company_next);
	OpenTTDEngineCompanyPreview (*company_preview)(const Company *) noexcept OPENTTD_SERVICE_DEFAULT(company_preview);
	OpenTTDEngineVehicle (*vehicle_next)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_next);
	OpenTTDEngineVehicleCargo (*vehicle_cargo)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_cargo);
	const OpenTTDEngineCapabilitiesServices *capabilities;
	bool (*company_valid)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(company_valid);
	uint64_t (*compatible_railtypes)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(compatible_railtypes);
	uint64_t (*powered_roadtypes)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(powered_roadtypes);
	uint64_t (*company_railtypes)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(company_railtypes);
	uint64_t (*company_roadtypes)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(company_roadtypes);
	uint (*company_engine_count)(uint8_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(company_engine_count);
	void (*block_preview)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(block_preview);
	uint8_t (*current_company)() noexcept OPENTTD_SERVICE_DEFAULT(current_company);
	uint8_t (*local_company)() noexcept OPENTTD_SERVICE_DEFAULT(local_company);
	void (*toolbar_enable)(VehicleType) noexcept OPENTTD_SERVICE_DEFAULT(toolbar_enable);
	void (*toolbar_release)(VehicleType) noexcept OPENTTD_SERVICE_DEFAULT(toolbar_release);
	void (*close_preview)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(close_preview);
	void (*invalidate_preview)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(invalidate_preview);
	void (*offer_preview)(uint16_t, uint8_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(offer_preview);
	void (*available_event)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(available_event);
	void (*available_news)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(available_news);
	void (*no_engines)(int32_t) noexcept OPENTTD_SERVICE_DEFAULT(no_engines);
	void (*screen_dirty)() noexcept OPENTTD_SERVICE_DEFAULT(screen_dirty);
	size_t (*utf8_length)(const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(utf8_length);
};
extern "C" {
void openttd_rust_engine_reliability(const OpenTTDEngineServices *, OpenTTDEngineRef, bool);
void openttd_rust_engine_start_one(const OpenTTDEngineServices *, OpenTTDEngineRef, OpenTTDEngineYMD, uint32_t);
void openttd_rust_engine_aging_stops(const OpenTTDEngineServices *);
void openttd_rust_engine_start_all(const OpenTTDEngineServices *);
void openttd_rust_engine_monthly(const OpenTTDEngineServices *);
void openttd_rust_engine_clear_hidden(const OpenTTDEngineServices *, uint8_t);
bool openttd_rust_engine_buildable(const OpenTTDEngineServices *, OpenTTDEngineRef, VehicleType, uint8_t);
bool openttd_rust_engine_variant_hidden(const OpenTTDEngineServices *, OpenTTDEngineRef, uint8_t);
uint8_t openttd_rust_engine_visibility(const OpenTTDEngineServices *, OpenTTDEngineRef, bool, bool);
uint8_t openttd_rust_engine_want_preview(const OpenTTDEngineServices *, OpenTTDEngineRef, bool);
uint8_t openttd_rust_engine_control(const OpenTTDEngineServices *, OpenTTDEngineRef, uint8_t, bool, bool);
uint8_t openttd_rust_engine_rename(const OpenTTDEngineServices *, OpenTTDEngineRef, OpenTTDByteSpan, bool);
void openttd_rust_engine_check(const OpenTTDEngineServices *);
}
#endif /* RUST_ENGINE_FFI_H */
