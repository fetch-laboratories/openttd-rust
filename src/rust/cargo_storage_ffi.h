/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file cargo_storage_ffi.h Canonical packets, cargo lists and movement. */
#ifndef RUST_CARGO_STORAGE_FFI_H
#define RUST_CARGO_STORAGE_FFI_H
#include "service_defaults.hpp"
#include <cstdint>
#include <cstddef>
struct OpenTTDCargoList;
struct OpenTTDCargoFlow;
struct OpenTTDCargoFlowMap;
enum class SourceType : uint8_t;
/* Typed, call-local cargo Source value; the ID and source kind are never packed. */
struct OpenTTDCargoSource {
	uint16_t id;
	SourceType type;
};
#include "abi-record-cargo_storage.h"
struct OpenTTDCargoListFields {
	uint64_t cargo_periods_in_transit;
	int64_t feeder_share;
	uint32_t count, reserved_count, action_counts[4];
};
/* Handles are opaque owners; C++ pool shells keep IDs and allocation order, and list nodes hold
 * pool-shell handles. Payments run in Rust and may reenter packet and list getters; raw owner accesses
 * end before them, and metadata writes keep their source order. Save/load writes packet fields in place;
 * list import never dereferences unresolved IDs. Next-hop buffers borrow the caller's StationID span for one call.
 * Handles are live and serially accessed, create/destroy match once, and callbacks retain no arguments;
 * snapshot output is confined to local exports. */
struct CargoPacket;
struct GoodsEntry;
struct OpenTTDCargoDelivery;
struct OpenTTDCargoMonitorServices;
struct OpenTTDCargoPayment;
struct OpenTTDCargoServices;
struct OpenTTDSharedServices;
struct CargoPayment;
struct OpenTTDCargoStorageServices {
	uint8_t (*can_allocate)() noexcept OPENTTD_SERVICE_DEFAULT(can_allocate);
	OpenTTDCargoDelivery *(*cargo_delivery_state)() noexcept OPENTTD_SERVICE_DEFAULT(cargo_delivery_state);
	CargoPacket *(*create)(const OpenTTDCargoPacket *) noexcept OPENTTD_SERVICE_DEFAULT(create);
	void (*destroy)(CargoPacket *) noexcept OPENTTD_SERVICE_DEFAULT(destroy);
	OpenTTDCargoFlowMap *(*flow_map)(const GoodsEntry *) noexcept OPENTTD_SERVICE_DEFAULT(flow_map);
	const OpenTTDCargoMonitorServices *monitors;
	OpenTTDCargoPacket *(*packet)(CargoPacket *) noexcept OPENTTD_SERVICE_DEFAULT(packet);
	CargoPacket *(*packet_next)(CargoPacket *) noexcept OPENTTD_SERVICE_DEFAULT(packet_next);
	const OpenTTDCargoServices *payment;
	OpenTTDCargoPayment *(*payment_state)(CargoPayment *) noexcept OPENTTD_SERVICE_DEFAULT(payment_state);
	const OpenTTDSharedServices *shared;
};
extern "C" {
void openttd_rust_cargo_invalidate(const OpenTTDCargoStorageServices *, uint16_t, uint8_t);
/* Null initializes original packet defaults; non-null is Split's copy constructor and asserts count != 0. */
OpenTTDCargoPacket *openttd_rust_cargo_packet_new(const OpenTTDCargoPacket *);
void openttd_rust_cargo_packet_destroy(OpenTTDCargoPacket *);
void openttd_rust_cargo_packet_update_loading_tile(OpenTTDCargoPacket *, const OpenTTDCargoStorageServices *, uint32_t);
uint32_t openttd_rust_cargo_packet_distance(const OpenTTDCargoPacket *, const OpenTTDCargoStorageServices *, uint32_t);
OpenTTDCargoList *openttd_rust_cargo_list_new(uint8_t);
void openttd_rust_cargo_list_destroy(OpenTTDCargoList *, const OpenTTDCargoStorageServices *);
void openttd_rust_cargo_list_clear(OpenTTDCargoList *);
void openttd_rust_cargo_list_export(const OpenTTDCargoList *, OpenTTDCargoListFields *);
void openttd_rust_cargo_list_import(OpenTTDCargoList *, const OpenTTDCargoListFields *);
uint32_t openttd_rust_cargo_list_count(const OpenTTDCargoList *);
uint32_t openttd_rust_cargo_list_reserved(const OpenTTDCargoList *);
void openttd_rust_cargo_list_set_reserved(OpenTTDCargoList *, uint32_t);
uint32_t openttd_rust_cargo_list_station_total(const OpenTTDCargoList *);
uint32_t openttd_rust_cargo_list_periods(const OpenTTDCargoList *);
int64_t openttd_rust_cargo_list_feeder(const OpenTTDCargoList *);
uint32_t openttd_rust_cargo_list_action_count(const OpenTTDCargoList *, uint8_t);
uint32_t openttd_rust_cargo_list_stored(const OpenTTDCargoList *);
size_t openttd_rust_cargo_list_key_count(const OpenTTDCargoList *);
void openttd_rust_cargo_list_snapshot(const OpenTTDCargoList *, void *, void (*)(void *, uint16_t, CargoPacket *) noexcept);
void openttd_rust_cargo_list_insert(OpenTTDCargoList *, uint16_t, CargoPacket *);
void openttd_rust_cargo_list_rebuild(OpenTTDCargoList *, const OpenTTDCargoStorageServices *);
uint16_t openttd_rust_cargo_list_first(const OpenTTDCargoList *, const OpenTTDCargoStorageServices *);
void openttd_rust_cargo_list_keep(OpenTTDCargoList *);
void openttd_rust_vehicle_cargo_list_append(OpenTTDCargoList *, const OpenTTDCargoStorageServices *, CargoPacket *, uint8_t);
void openttd_rust_station_cargo_list_append(OpenTTDCargoList *, const OpenTTDCargoStorageServices *, CargoPacket *, uint16_t);
uint32_t openttd_rust_vehicle_cargo_list_truncate(OpenTTDCargoList *, const OpenTTDCargoStorageServices *, uint32_t);
uint32_t openttd_rust_station_cargo_list_truncate(OpenTTDCargoList *, const OpenTTDCargoStorageServices *, uint32_t);
/* For the orders console only: StationCargoList::Reserve over `length` station IDs. */
uint32_t openttd_rust_station_cargo_list_reserve(OpenTTDCargoList *, const OpenTTDCargoStorageServices *, uint32_t, OpenTTDCargoList *, const uint16_t *, size_t, uint32_t);
}
#endif /* RUST_CARGO_STORAGE_FFI_H */
