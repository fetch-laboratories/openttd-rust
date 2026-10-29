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
enum class SourceType : uint8_t;
/* Typed, call-local cargo Source value; the ID and source kind are never packed. */
struct OpenTTDCargoSource {
	uint16_t id;
	SourceType type;
};
struct OpenTTDCargoPacketFields {
	int64_t feeder_share = 0;
	uint32_t source_xy = UINT32_MAX;
	uint16_t count = 0, periods_in_transit = 0, first_station = UINT16_MAX, next_hop = UINT16_MAX, source_id = UINT16_MAX;
	int16_t travelled_x = 0, travelled_y = 0;
	uint8_t source_type = 0, in_vehicle = 0;
};
/* Rust-allocated repr(C) packet with exactly these fields (ABI layout 280);
 * CAPA save/load addresses them in place between Rust calls. */
struct OpenTTDCargoPacket : OpenTTDCargoPacketFields {};
struct OpenTTDCargoListFields {
	uint64_t cargo_periods_in_transit;
	int64_t feeder_share;
	uint32_t count, reserved_count, action_counts[4];
};
/* Handles are opaque owners; C++ pool shells keep IDs and allocation order, and list nodes hold
 * pool-shell handles. Payment callbacks may reenter packet and list getters; raw owner accesses end
 * before them, and metadata writes keep their source order. Save/load writes packet fields in place;
 * import never dereferences
 * unresolved IDs. Next-hop buffers borrow the caller's StationID span for one call. Flow mode:
 * 0 normal/restricted, 1 forced transfer (excludes current and next reverse), 2 reroute (excludes
 * avoid/avoid2); selector 3 asks for the first flow origin, selector 4 whether any flows exist.
 * Handles are live and serially accessed, create/destroy match once, and callbacks retain no arguments;
 * next-hop storage stays live and stable for the call, with snapshot output confined to local exports. */
struct OpenTTDCargoStorageServices {
	uint8_t (*can_allocate)() noexcept OPENTTD_SERVICE_DEFAULT(can_allocate);
	void *(*create)(const OpenTTDCargoPacketFields *) noexcept OPENTTD_SERVICE_DEFAULT(create);
	OpenTTDCargoPacket *(*packet)(void *) noexcept OPENTTD_SERVICE_DEFAULT(packet);
	void (*destroy)(void *) noexcept OPENTTD_SERVICE_DEFAULT(destroy);
	uint32_t (*random)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(random);
	uint32_t (*coordinate)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(coordinate);
	uint16_t (*flow)(const void *, uint8_t, uint16_t, uint16_t, uint16_t, const uint16_t *, size_t, uint8_t *) noexcept OPENTTD_SERVICE_DEFAULT(flow);
	int64_t (*pay)(void *, uint8_t, uint8_t, const void *, uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(pay);
	void (*origin)(void *, uint16_t, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(origin);
	OpenTTDCargoFlowMap *(*flow_map)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(flow_map);
	uint32_t (*random_draw)() noexcept OPENTTD_SERVICE_DEFAULT(random_draw);
	void *(*packet_next)(void *) noexcept OPENTTD_SERVICE_DEFAULT(packet_next);
};
struct OpenTTDCargoCapacityVehicle {
	OpenTTDCargoList *list;
	uint16_t capacity;
	uint8_t cargo, train, articulated;
};
struct OpenTTDCargoCapacityServices {
	void (*read)(void *, OpenTTDCargoCapacityVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(read);
	void *(*next_part)(void *) noexcept OPENTTD_SERVICE_DEFAULT(next_part);
	void *(*last_engine_part)(void *) noexcept OPENTTD_SERVICE_DEFAULT(last_engine_part);
	void *(*other_multiheaded_part)(void *) noexcept OPENTTD_SERVICE_DEFAULT(other_multiheaded_part);
	const OpenTTDCargoStorageServices *cargo;
};
const OpenTTDCargoCapacityServices &GetRustCargoCapacityServices() noexcept;
/* capacity mode 0 spreads/shrinks, mode 1 transfers. Return 1 requires the native
 * ConsistChanged callback after Rust returns (no borrowed part/list survives). */
extern "C" {
uint8_t openttd_rust_cargo_capacity(const OpenTTDCargoCapacityServices *, void *, void *, uint8_t, uint8_t);
void openttd_rust_cargo_invalidate(const OpenTTDCargoStorageServices *, uint16_t, uint8_t);
OpenTTDCargoPacket *openttd_rust_cargo_packet_new(const OpenTTDCargoPacketFields *);
void openttd_rust_cargo_packet_destroy(OpenTTDCargoPacket *);
void openttd_rust_cargo_packet_export(const OpenTTDCargoPacket *, OpenTTDCargoPacketFields *);
void openttd_rust_cargo_packet_import(OpenTTDCargoPacket *, const OpenTTDCargoPacketFields *);
void openttd_rust_cargo_packet_set(OpenTTDCargoPacket *, uint8_t, uint32_t);
void openttd_rust_cargo_packet_tile(OpenTTDCargoPacket *, const OpenTTDCargoStorageServices *, uint32_t, uint8_t);
void openttd_rust_cargo_packet_feeder(OpenTTDCargoPacket *, int64_t);
int64_t openttd_rust_cargo_packet_share(const OpenTTDCargoPacket *, uint32_t);
uint32_t openttd_rust_cargo_packet_distance(const OpenTTDCargoPacket *, const OpenTTDCargoStorageServices *, uint32_t);
void *openttd_rust_cargo_packet_split(OpenTTDCargoPacket *, const OpenTTDCargoStorageServices *, uint32_t);
void openttd_rust_cargo_packet_merge(OpenTTDCargoPacket *, const OpenTTDCargoStorageServices *, void *);
void openttd_rust_cargo_packet_reduce(OpenTTDCargoPacket *, uint32_t);
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
uint32_t openttd_rust_cargo_list_unload(const OpenTTDCargoList *);
uint32_t openttd_rust_cargo_list_remaining(const OpenTTDCargoList *);
size_t openttd_rust_cargo_list_key_count(const OpenTTDCargoList *);
void openttd_rust_cargo_list_snapshot(const OpenTTDCargoList *, void *, void (*)(void *, uint16_t, void *) noexcept);
void openttd_rust_cargo_list_insert(OpenTTDCargoList *, uint16_t, void *);
void openttd_rust_cargo_list_rebuild(OpenTTDCargoList *, const OpenTTDCargoStorageServices *);
void openttd_rust_cargo_list_append(OpenTTDCargoList *, const OpenTTDCargoStorageServices *, void *, uint16_t);
uint16_t openttd_rust_cargo_list_first(const OpenTTDCargoList *, const OpenTTDCargoStorageServices *);
uint8_t openttd_rust_cargo_list_has(const OpenTTDCargoList *, const uint16_t *, size_t);
void openttd_rust_cargo_list_age(OpenTTDCargoList *, const OpenTTDCargoStorageServices *);
void openttd_rust_cargo_list_keep(OpenTTDCargoList *);
uint32_t openttd_rust_cargo_list_reassign(OpenTTDCargoList *, const OpenTTDCargoStorageServices *, uint8_t, uint8_t, uint32_t);
uint8_t openttd_rust_cargo_list_stage(OpenTTDCargoList *, const OpenTTDCargoStorageServices *, uint8_t, uint16_t, const uint16_t *, size_t, uint8_t, const void *, uint8_t, void *, uint32_t);
uint32_t openttd_rust_cargo_list_move(OpenTTDCargoList *, OpenTTDCargoList *, const OpenTTDCargoStorageServices *, uint8_t, uint32_t, uint16_t, uint16_t, const uint16_t *, size_t, const void *, uint8_t, void *, uint32_t);
}
#endif /* RUST_CARGO_STORAGE_FFI_H */
