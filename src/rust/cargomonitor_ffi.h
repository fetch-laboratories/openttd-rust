/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file cargomonitor_ffi.h Process-wide cargo-monitor owner and incremental rows. */
#ifndef RUST_CARGOMONITOR_FFI_H
#define RUST_CARGOMONITOR_FFI_H
#include "service_defaults.hpp"
#include <cstdint>
struct Station;
struct Industry;
struct RustNearbyIndustryCursor;
/** Serialized by the game-state mutex. Native handles/cursors are call-scoped;
 * no map borrow crosses a leaf or SlObject. Leaves cannot reset catchments or
 * pools, yield, retain arguments or throw. Rust panics abort. Save/load keeps
 * SlObject/SlError native, inserting only complete rows, with no transaction. */
struct OpenTTDCargoMonitorServices {
	const Station *(*station)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(station);
	uint16_t (*station_town)(const Station *) noexcept OPENTTD_SERVICE_DEFAULT(station_town);
	void (*begin_nearby)(RustNearbyIndustryCursor *, const Station *) noexcept OPENTTD_SERVICE_DEFAULT(begin_nearby);
	Industry *(*next_nearby)(RustNearbyIndustryCursor *) noexcept OPENTTD_SERVICE_DEFAULT(next_nearby);
	uint16_t (*industry_id)(void *) noexcept OPENTTD_SERVICE_DEFAULT(industry_id);
	void (*check)(bool) noexcept OPENTTD_SERVICE_DEFAULT(check);
};
const OpenTTDCargoMonitorServices &GetRustCargoMonitorServices() noexcept;
extern "C" {
uint32_t openttd_rust_monitor_encode_industry(uint8_t, uint8_t, uint16_t, void (*)(bool) noexcept);
uint32_t openttd_rust_monitor_encode_town(uint8_t, uint8_t, uint16_t, void (*)(bool) noexcept);
uint8_t openttd_rust_monitor_company(uint32_t);
uint8_t openttd_rust_monitor_cargo(uint32_t);
bool openttd_rust_monitor_is_industry(uint32_t);
uint16_t openttd_rust_monitor_industry(uint32_t);
uint16_t openttd_rust_monitor_town(uint32_t);
void openttd_rust_monitor_clear_pickups(uint8_t);
void openttd_rust_monitor_clear_deliveries(uint8_t);
int32_t openttd_rust_monitor_pickup(uint32_t, bool);
int32_t openttd_rust_monitor_delivery(uint32_t, bool);
void openttd_rust_monitor_add(const OpenTTDCargoMonitorServices *, uint8_t, uint8_t, uint32_t, uint16_t, uint8_t, const Station *, uint16_t, RustNearbyIndustryCursor *);
bool openttd_rust_monitor_next_pickup(bool, uint32_t *, uint32_t *);
bool openttd_rust_monitor_next_delivery(bool, uint32_t *, uint32_t *);
void openttd_rust_monitor_insert_pickup(uint32_t, uint32_t);
void openttd_rust_monitor_insert_delivery(uint32_t, uint32_t);
}
#endif /* RUST_CARGOMONITOR_FFI_H */
