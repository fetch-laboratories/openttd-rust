struct OpenTTDEngineCapabilitiesServices;
struct OpenTTDCompanyFinanceServices;
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
using OpenTTDEngineYMD = OpenTTDCalendarYMD;
struct OpenTTDEngineStartSettings { int32_t starting_year, date; };
struct OpenTTDEngineCompany { const Company *shell; uint8_t id; };
struct OpenTTDEngineVehicle { OpenTTDVehicle *shell; uint32_t id; uint8_t owner; VehicleType type; };
struct OpenTTDEngineVehicleCargo { const OpenTTDEngine *engine; uint8_t cargo; };
/** Every callback is noexcept; script events only enqueue, no VM executes.
 * No owner borrow survives any callback, including native type availability,
 * articulated/NewGRF reads, windows, and recursively joined variants.
 * Pool traversal preserves IDs/order; prototypes arrive by shell, never ID. */
struct OpenTTDTypeAvailabilityServices;
struct OpenTTDEngineServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDCompanyFinanceServices *finance;
	const OpenTTDRandomServices *rng;
	const OpenTTDTypeAvailabilityServices *types;
	OpenTTDEngineRef (*resolve)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(resolve);
	uint16_t (*display_variant)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(display_variant);
	uint16_t (*engine_id)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_id);
	OpenTTDEngineStartSettings (*start_settings)() noexcept OPENTTD_SERVICE_DEFAULT(start_settings);
	uint16_t (*road_limit)() noexcept OPENTTD_SERVICE_DEFAULT(road_limit);
	uint16_t (*ship_limit)() noexcept OPENTTD_SERVICE_DEFAULT(ship_limit);
	uint16_t (*aircraft_limit)() noexcept OPENTTD_SERVICE_DEFAULT(aircraft_limit);
	bool (*ai_train_disabled)() noexcept OPENTTD_SERVICE_DEFAULT(ai_train_disabled);
	bool (*ai_road_disabled)() noexcept OPENTTD_SERVICE_DEFAULT(ai_road_disabled);
	bool (*ai_ship_disabled)() noexcept OPENTTD_SERVICE_DEFAULT(ai_ship_disabled);
	bool (*ai_aircraft_disabled)() noexcept OPENTTD_SERVICE_DEFAULT(ai_aircraft_disabled);
	void (*clear_last_variant)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(clear_last_variant);
	void (*refresh_type)(VehicleType) noexcept OPENTTD_SERVICE_DEFAULT(refresh_type);
	void (*refresh_lists)(bool) noexcept OPENTTD_SERVICE_DEFAULT(refresh_lists);
	OpenTTDEngineCompany (*company_next)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(company_next);
	OpenTTDEngineVehicle (*vehicle_next)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_next);
	OpenTTDEngineVehicleCargo (*vehicle_cargo)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_cargo);
	const OpenTTDEngineCapabilitiesServices *capabilities;
	uint (*company_engine_count)(uint8_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(company_engine_count);
	void (*toolbar_enable)(VehicleType) noexcept OPENTTD_SERVICE_DEFAULT(toolbar_enable);
	void (*toolbar_release)(VehicleType) noexcept OPENTTD_SERVICE_DEFAULT(toolbar_release);
	void (*close_preview)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(close_preview);
	void (*invalidate_preview)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(invalidate_preview);
	void (*offer_preview)(uint16_t, uint8_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(offer_preview);
	void (*available_event)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(available_event);
	void (*available_news)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(available_news);
	void (*no_engines)(int32_t) noexcept OPENTTD_SERVICE_DEFAULT(no_engines);
};
const OpenTTDEngineServices &EngineServices() noexcept;
extern "C" {
uint8_t openttd_rust_engine_enabled_abi(const OpenTTDEngineServices *, const OpenTTDEngine *);
extern "C++" inline bool openttd_rust_engine_enabled(const OpenTTDEngineServices *a0, const OpenTTDEngine *a1) { return openttd_rust_engine_enabled_abi(a0, a1) != 0; }
void openttd_rust_engine_reliability(const OpenTTDEngineServices *, OpenTTDEngineRef, bool);
void openttd_rust_engine_start_one(const OpenTTDEngineServices *, OpenTTDEngineRef, OpenTTDEngineYMD, uint32_t);
void openttd_rust_engine_aging_stops(const OpenTTDEngineServices *);
void openttd_rust_engine_start_all(const OpenTTDEngineServices *);
void openttd_rust_engine_monthly(const OpenTTDEngineServices *);
void openttd_rust_engine_clear_hidden(const OpenTTDEngineServices *, uint8_t);
uint8_t openttd_rust_engine_buildable_abi(const OpenTTDEngineServices *, OpenTTDEngineRef, VehicleType, uint8_t);
extern "C++" inline bool openttd_rust_engine_buildable(const OpenTTDEngineServices *a0, OpenTTDEngineRef a1, VehicleType a2, uint8_t a3) { return openttd_rust_engine_buildable_abi(a0, a1, a2, a3) != 0; }
uint8_t openttd_rust_engine_variant_hidden_abi(const OpenTTDEngineServices *, OpenTTDEngineRef, uint8_t);
extern "C++" inline bool openttd_rust_engine_variant_hidden(const OpenTTDEngineServices *a0, OpenTTDEngineRef a1, uint8_t a2) { return openttd_rust_engine_variant_hidden_abi(a0, a1, a2) != 0; }
uint8_t openttd_rust_engine_visibility(const OpenTTDEngineServices *, OpenTTDEngineRef, bool, bool);
uint8_t openttd_rust_engine_want_preview(const OpenTTDEngineServices *, OpenTTDEngineRef, bool);
uint8_t openttd_rust_engine_control(const OpenTTDEngineServices *, OpenTTDEngineRef, uint8_t, bool, bool);
uint8_t openttd_rust_engine_rename(const OpenTTDEngineServices *, OpenTTDEngineRef, OpenTTDByteSpan, bool);
void openttd_rust_engine_check(const OpenTTDEngineServices *);
}
#endif /* RUST_ENGINE_FFI_H */
