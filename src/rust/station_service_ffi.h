/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file station_service_ffi.h Canonical Rust station service state and direct world leaves. */
#ifndef RUST_STATION_SERVICE_FFI_H
#define RUST_STATION_SERVICE_FFI_H
#include <cstdint>
#include <cstddef>
#include "services_ffi.h"
#include "vehicle_ffi.h"
#include "cargo_storage_ffi.h"
#include "station_ffi.h"
#include "linkgraph_ffi.h"
#include "orders_ffi.h"
#include "cargo_payment_ffi.h"
struct OpenTTDStationService;
struct OpenTTDStationCargo {
	uint32_t max_waiting_cargo;
	uint8_t status, time_since_pickup, rating, last_speed, last_age, amount_fract;
};
struct OpenTTDStationFields {
	uint64_t always_accepted;
	uint8_t delete_ctr, time_since_load, time_since_unload, last_vehicle_type;
};
/* Each BaseStation owns one stable allocation, created for ordinary and indexed
 * allocation and destroyed after its derived shell and cargo storage. Scalar
 * addresses stay valid until owner destruction; native facade references alias
 * this allocation rather than mirror it. All calls are serial. Rust must use
 * raw field-sized access, ending access before a world service; C++ may read or
 * mutate any metadata through callbacks. No reference/slice into state survives
 * a world call. Saving and legacy loading address the same canonical scalars.
 * Cargo status starts a trivially-copyable native EnumBitSet<uint8_t> lifetime
 * at shell construction; the ABI pins size/alignment and Rust accesses only its
 * byte representation through raw pointers. No C++ reference object is saved.
 * Stable queue nodes have std::list identity/erase semantics; first/next/value
 * require live entries, and erase_node requires an entry from that owner.
 * The native save facade alone uses a call-local std::list for partial loading
 * and pointer fixups, committing on native error unwind outside Rust frames.
 * Refitting returns CommandCost/capacities without ordinary exceptions. Its
 * capacity/NewGRF cache updates and command-scope window invalidations may read
 * canonical metadata; no reference to canonical owner fields, cargo metadata or queue spans the call.
 * Cargo Stage/Unload can reenter #117 payment/delivery and later industry owners;
 * copied observations and raw field accesses end before each operation. NewGRF
 * load/house/cargo/vehicle/station/airport/road resolvers execute sprite-group
 * programs, never script VMs or arbitrary game commands. Command-scope window
 * invalidation may read the owner; unrestricted GUI-scope runs later in C++.
 * Opaque world handles obey the original pool lifetimes. Panics and environmental
 * service failures abort. Packet, flow, geometry and map storage remain C++.
 * with_* bodies run synchronously once while native iterator/container frames
 * live on the caller's stack. Immutable engine/GRF handles retain native registry
 * lifetime; conditional fields are observed only at their original read points.
 */
struct OpenTTDRatingCounts {
	uint32_t available;
	uint32_t destinations;
};
struct OpenTTDStationExclusivity {
	uint8_t counter;
	uint8_t exclusive;
};
/* Stable Rust cargo owner and the native capacity/type used at this read point.
 * Rust reads remaining counts directly; refresh after refit or reservation. */
struct OpenTTDCargoCapacity {
	const OpenTTDCargoList *owner;
	uint16_t capacity;
	uint8_t cargo;
};
struct OpenTTDCargoReservation {
	uint64_t mask;
	uint32_t reserved;
	uint16_t capacity;
	uint8_t cargo;
};
struct OpenTTDCargoStored {
	uint32_t stored;
	uint16_t capacity;
};
struct OpenTTDDeliveryRemaining {
	uint32_t remaining;
	uint32_t deliver;
	uint16_t capacity;
};
struct OpenTTDLoadEngine {
	const OpenTTDEngine *engine;
	uint8_t amount;
};
struct OpenTTDLoadingWait {
	int32_t lateness;
	uint16_t wait;
};
/* Front order/type/settings stay unchanged during Reserve, refit and bounded
 * NewGRF/cargo callbacks. Captured after GetNextStoppingStation; StopLoading and
 * load_unload_ticks are written only after the part loop. */
struct OpenTTDLoadingControl {
	uint16_t ticks;
	uint8_t vehicle_type, unload_type, load_type, refit_cargo;
	bool refit, full_load, stop, gradual, improved;
};
struct OpenTTDLoadingPart {
	uint16_t capacity;
	uint8_t cargo, vehicle_type;
	bool unloading;
};
struct OpenTTDLoadingStats {
	uint32_t stored, load_count;
	int32_t speed, age;
	uint16_t capacity;
	uint8_t cargo;
};
using OpenTTDStationScope = void (*)(void *, void *) noexcept;
struct OpenTTDOrderDestination {
	uint16_t destination;
	bool is_station;
};
struct OpenTTDTruncatedSource {
	uint32_t amount;
	uint16_t source;
	bool present;
};
/* Station lookup identities and canonical owner, resolved together before
 * next-station discovery. No borrowed station fields cross a callback. */
struct OpenTTDLoadingStation {
	OpenTTDStation *native;
	OpenTTDBaseStation *base;
	OpenTTDStationService *state;
};
struct OpenTTDStationServices {
	OpenTTDLoadingStation (*station)(uint16_t) noexcept;
	uint8_t (*station_owner)(OpenTTDBaseStation *) noexcept;
	OpenTTDStationExclusivity (*station_exclusivity)(OpenTTDBaseStation *) noexcept;
	OpenTTDBaseStation * (*station_next)(void *) noexcept;
	OpenTTDRatingCounts (*rating_counts)(OpenTTDBaseStation *, uint8_t) noexcept;
	OpenTTDBaseStation * (*get)(uint16_t) noexcept;
	OpenTTDStationService * (*owner)(OpenTTDBaseStation *) noexcept;
	size_t (*pool_size)() noexcept;
	uint64_t (*tick_counter)() noexcept;
	bool (*editor)() noexcept;
	uint8_t (*facilities)(OpenTTDBaseStation *) noexcept;
	bool (*in_use)(OpenTTDBaseStation *) noexcept;
	bool (*is_station)(OpenTTDBaseStation *) noexcept;
	bool (*local_owner)(OpenTTDBaseStation *) noexcept;
	bool (*rectangle_empty)(OpenTTDBaseStation *) noexcept;
	bool (*town_statue)(OpenTTDBaseStation *) noexcept;
	bool (*select_goods)() noexcept;
	bool (*rating_cheat)() noexcept;
	uint32_t (*available)(OpenTTDBaseStation *, uint8_t) noexcept;
	bool (*cargo_valid)(uint8_t) noexcept;
	bool (*rating_callback_enabled)(uint8_t) noexcept;
	bool (*passenger_cargo)(uint8_t) noexcept;
	bool (*packet_available)() noexcept;
	void (*rating_redraw)(OpenTTDBaseStation *, bool) noexcept;
	void (*set_demand)(OpenTTDBaseStation *, uint8_t, uint32_t) noexcept;
	void (*acceptance_news)(OpenTTDBaseStation *, uint64_t, bool) noexcept;
	void (*acceptance_redraw)(OpenTTDBaseStation *) noexcept;
	void (*delete_station)(OpenTTDBaseStation *) noexcept;
	void (*acceptance_animation)(OpenTTDBaseStation *) noexcept;
	void (*watched_animation)(uint32_t, uint64_t) noexcept;
	void (*append_packet)(OpenTTDBaseStation *, uint8_t, uint32_t, OpenTTDCargoSource) noexcept;
	void (*update_supply)(OpenTTDBaseStation *, uint8_t, uint32_t) noexcept;
	void (*station_list_invalidation)(OpenTTDBaseStation *) noexcept;
	void (*new_cargo_animation)(OpenTTDBaseStation *, uint8_t) noexcept;
	void (*waiting_redraw)(OpenTTDBaseStation *) noexcept;
	uint16_t (*rating_callback)(OpenTTDBaseStation *, uint8_t, uint32_t, uint32_t) noexcept;
	void (*with_tiles)(OpenTTDBaseStation *, void *, OpenTTDStationScope) noexcept;
	uint32_t (*tile_next)(void *) noexcept;
	void (*accept_tile)(void *, uint32_t, uint64_t *) noexcept;
	const uint32_t * (*acceptance_amounts)(void *) noexcept;
	void (*with_truncation)(OpenTTDBaseStation *, uint8_t, uint32_t, void *, OpenTTDStationScope) noexcept;
	OpenTTDTruncatedSource (*truncate_next)(void *) noexcept;
	const OpenTTDSharedServices *shared;
};

struct OpenTTDStationEdge {
	OpenTTDBaseStation *destination;
	int32_t last_update;
	uint32_t distance;
};
struct OpenTTDStationLinks {
	OpenTTDLinkGraph * (*graph)(OpenTTDBaseStation *, uint8_t) noexcept;
	bool (*automatic_distribution)(uint8_t) noexcept;
	uint16_t (*node)(OpenTTDBaseStation *, uint8_t) noexcept;
	size_t (*edge_count)(OpenTTDLinkGraph *, uint16_t) noexcept;
	int32_t (*date)() noexcept;
	uint16_t (*station_id)(OpenTTDBaseStation *) noexcept;
	int32_t (*edge_last_update)(OpenTTDLinkGraph *, uint16_t, size_t) noexcept;
	int32_t (*last_compression)(OpenTTDLinkGraph *) noexcept;
	OpenTTDStationEdge (*edge)(OpenTTDLinkGraph *, uint16_t, size_t, OpenTTDBaseStation *) noexcept;
	int32_t (*edge_unrestricted)(OpenTTDLinkGraph *, uint16_t, size_t) noexcept;
	int32_t (*edge_restricted)(OpenTTDLinkGraph *, uint16_t, size_t) noexcept;
	void (*delete_flows)(OpenTTDBaseStation *, uint8_t, uint16_t) noexcept;
	void (*restrict_edge)(OpenTTDLinkGraph *, uint16_t, size_t) noexcept;
	void (*restrict_flows)(OpenTTDBaseStation *, uint8_t, uint16_t) noexcept;
	void (*release_edge)(OpenTTDLinkGraph *, uint16_t, size_t) noexcept;
	void (*remove_edge)(OpenTTDLinkGraph *, uint16_t, uint16_t) noexcept;
	void (*compress)(OpenTTDLinkGraph *) noexcept;
	OpenTTDOrderList * (*order_list)(uint16_t) noexcept;
	size_t (*order_pool_size)() noexcept;
	size_t (*order_count)(OpenTTDOrderList *) noexcept;
	OpenTTDOrderDestination (*order_destination)(OpenTTDOrderList *, size_t) noexcept;
	OpenTTDVehicle * (*order_vehicle)(OpenTTDOrderList *) noexcept;
	OpenTTDVehicle * (*next_vehicle)(OpenTTDVehicle *) noexcept;
	OpenTTDVehicle * (*next_shared)(OpenTTDVehicle *) noexcept;
	bool (*stopped_in_depot)(OpenTTDVehicle *) noexcept;
	int32_t (*last_service)(OpenTTDVehicle *) noexcept;
	uint8_t (*cargo_type)(OpenTTDVehicle *) noexcept;
	void (*refresh)(OpenTTDVehicle *) noexcept;
	void (*reroute)(OpenTTDBaseStation *, OpenTTDVehicle *, uint8_t, uint16_t, uint16_t) noexcept;
};

struct OpenTTDStationLoading {
	OpenTTDLoadingControl (*control)(OpenTTDVehicle *) noexcept;
	OpenTTDLoadingPart (*part)(OpenTTDVehicle *) noexcept;
	OpenTTDLoadingStats (*set_refit_capacity_and_stats)(OpenTTDVehicle *, OpenTTDVehicle *, uint8_t, uint16_t, bool) noexcept;
	OpenTTDLoadingWait (*wait_observation)(OpenTTDVehicle *) noexcept;
	OpenTTDLoadEngine (*load_engine)(OpenTTDVehicle *) noexcept;
	uint8_t (*grf_version)(const OpenTTDEngine *) noexcept;
	bool (*engine_load_callback)(const OpenTTDEngine *) noexcept;
	bool (*engine_no_multiplier)(const OpenTTDEngine *) noexcept;
	OpenTTDDeliveryRemaining (*delivery_remaining)(OpenTTDVehicle *) noexcept;
	OpenTTDCargoStored (*cargo_stored)(OpenTTDVehicle *) noexcept;
	OpenTTDCargoReservation (*cargo_reservation)(OpenTTDVehicle *) noexcept;
	OpenTTDCargoCapacity (*cargo_capacity)(OpenTTDVehicle *) noexcept;
	uint8_t (*vehicle_type)(OpenTTDVehicle *) noexcept;
	uint8_t (*cargo_type)(OpenTTDVehicle *) noexcept;
	uint16_t (*cargo_cap)(OpenTTDVehicle *) noexcept;
	uint32_t (*stored_count)(OpenTTDVehicle *) noexcept;
	uint32_t (*unload_count)(OpenTTDVehicle *) noexcept;
	uint32_t (*deliver_count)(OpenTTDVehicle *) noexcept;
	uint32_t (*transfer_count)(OpenTTDVehicle *) noexcept;
	uint32_t (*load_count)(OpenTTDVehicle *) noexcept;
	uint8_t (*unload_type)(OpenTTDVehicle *) noexcept;
	bool (*has_orders)(OpenTTDVehicle *) noexcept;
	uint16_t (*load_unload_ticks)(OpenTTDVehicle *) noexcept;
	uint8_t (*status)(OpenTTDVehicle *) noexcept;
	bool (*has_articulated_part)(OpenTTDVehicle *) noexcept;
	bool (*is_articulated_part)(OpenTTDVehicle *) noexcept;
	bool (*is_rear_dualheaded)(OpenTTDVehicle *) noexcept;
	bool (*is_normal_aircraft)(OpenTTDVehicle *) noexcept;
	bool (*is_multiheaded)(OpenTTDVehicle *) noexcept;
	uint16_t (*cargo_multiplier)(OpenTTDVehicle *) noexcept;
	uint16_t (*cached_total_length)(OpenTTDVehicle *) noexcept;
	bool (*full_load_order)(OpenTTDVehicle *) noexcept;
	uint8_t (*vehicle_owner)(OpenTTDVehicle *) noexcept;
	bool (*passenger_cargo)(OpenTTDVehicle *) noexcept;
	bool (*station_length_penalty)() noexcept;
	uint8_t (*current_company)() noexcept;
	const OpenTTDVehicleServices *vehicle;
	uint32_t (*total_count)(OpenTTDVehicle *) noexcept;
	int32_t (*order_time)(OpenTTDVehicle *) noexcept;
	uint64_t (*refit_mask)(OpenTTDVehicle *) noexcept;
	void (*set_cargo_unloading)(OpenTTDVehicle *, bool) noexcept;
	void (*set_loading_finished)(OpenTTDVehicle *, bool) noexcept;
	void (*set_stop)(OpenTTDVehicle *, bool) noexcept;
	void (*set_load_ticks)(OpenTTDVehicle *, uint16_t) noexcept;
	void (*set_speed)(OpenTTDVehicle *, uint16_t) noexcept;
	void (*subtract_profit)(OpenTTDVehicle *, int64_t) noexcept;
	void (*unknown_load_result)(OpenTTDVehicle *, uint16_t) noexcept;
	void (*refresh_capacity)(OpenTTDVehicle *) noexcept;
	void (*vehicle_dirty)(OpenTTDVehicle *) noexcept;
	void (*update_indicator)(OpenTTDVehicle *) noexcept;
	void (*set_current_company)(uint8_t) noexcept;
	OpenTTDVehicle * (*next_vehicle)(OpenTTDVehicle *) noexcept;
	OpenTTDVehicle * (*next_articulated)(OpenTTDVehicle *) noexcept;
	OpenTTDVehicle * (*other_multiheaded)(OpenTTDVehicle *) noexcept;
	OpenTTDVehicle * (*first_engine_part)(OpenTTDVehicle *) noexcept;
	OpenTTDVehicle * (*first_vehicle)(OpenTTDVehicle *) noexcept;
	OpenTTDVehicle * (*previous_vehicle)(OpenTTDVehicle *) noexcept;
	void (*with_next_stations)(OpenTTDVehicle *, void *, OpenTTDStationScope) noexcept;
	void (*stage)(OpenTTDStation *, OpenTTDVehicle *, uint8_t, void *, OpenTTDNativeCargoPayment *) noexcept;
	void (*return_cargo)(OpenTTDStation *, OpenTTDVehicle *, uint32_t) noexcept;
	uint32_t (*unload_cargo)(OpenTTDStation *, OpenTTDVehicle *, uint32_t, OpenTTDNativeCargoPayment *) noexcept;
	uint32_t (*load_cargo)(OpenTTDStation *, OpenTTDVehicle *, uint32_t, void *) noexcept;
	void (*reserve_cargo)(OpenTTDStation *, OpenTTDVehicle *, uint32_t, void *) noexcept;
	void (*deliver_to_transfer)(OpenTTDVehicle *, uint32_t) noexcept;
	void (*deliver_to_keep)(OpenTTDVehicle *, uint32_t) noexcept;
	bool (*has_cargo_for)(OpenTTDStation *, uint8_t, void *) noexcept;
	uint32_t (*platform_length)(OpenTTDStation *, OpenTTDVehicle *) noexcept;
	uint32_t (*vehicle_tile)(OpenTTDVehicle *) noexcept;
	uint16_t (*station_index)(OpenTTDStation *) noexcept;
	uint32_t (*station_cargo_total)(OpenTTDStation *, uint8_t) noexcept;
	uint16_t (*load_property)(OpenTTDVehicle *) noexcept;
	uint16_t (*load_callback)(OpenTTDVehicle *) noexcept;
	OpenTTDNativeCargoPayment * (*get_payment)(OpenTTDVehicle *) noexcept;
	OpenTTDNativeCargoPayment * (*create_payment)(OpenTTDVehicle *) noexcept;
	void (*delete_payment)(OpenTTDNativeCargoPayment *) noexcept;
	void (*flush_delivery)() noexcept;
	void (*vehicle_new_cargo)(OpenTTDVehicle *) noexcept;
	void (*cargo_taken)(OpenTTDStation *, uint8_t) noexcept;
	void (*train_loads)(OpenTTDStation *, OpenTTDVehicle *) noexcept;
	void (*road_loads)(OpenTTDStation *, OpenTTDVehicle *) noexcept;
	void (*vehicle_empty)(OpenTTDVehicle *) noexcept;
	void (*station_dirty)(OpenTTDStation *) noexcept;
	uint32_t (*test_refit)(OpenTTDVehicle *, uint8_t) noexcept;
	bool (*execute_refit)(OpenTTDVehicle *, uint8_t, int64_t *) noexcept;
};

const OpenTTDStationServices &GetRustStationServices() noexcept;
const OpenTTDStationLoading &GetRustStationLoading() noexcept;
extern "C" {
OpenTTDStationService *openttd_rust_station_service_new();
void openttd_rust_station_service_destroy(OpenTTDStationService *);
OpenTTDStationFields *openttd_rust_station_service_fields(OpenTTDStationService *);
OpenTTDStationCargo *openttd_rust_station_service_cargo(OpenTTDStationService *, uint8_t);
size_t openttd_rust_station_queue_size(const OpenTTDStationService *);
void openttd_rust_station_queue_push(OpenTTDStationService *, void *);
void openttd_rust_station_queue_remove(OpenTTDStationService *, void *);
void openttd_rust_station_queue_clear(OpenTTDStationService *);
OpenTTDStationQueueEntry *openttd_rust_station_queue_first(const OpenTTDStationService *);
OpenTTDStationQueueEntry *openttd_rust_station_queue_next(const OpenTTDStationService *, const OpenTTDStationQueueEntry *);
void *openttd_rust_station_queue_value(const OpenTTDStationQueueEntry *);
OpenTTDStationQueueEntry *openttd_rust_station_queue_erase_node(OpenTTDStationService *, OpenTTDStationQueueEntry *);
void openttd_rust_station_prepare(OpenTTDVehicle *, const OpenTTDStationServices *, const OpenTTDStationLoading *);
/* An empty queue touches only its Rust owner; service tables may be null then. */
void openttd_rust_station_load(OpenTTDStationService *, const OpenTTDStationServices *, const OpenTTDStationLoading *);
void openttd_rust_station_acceptance(OpenTTDBaseStation *, const OpenTTDStationServices *, uint8_t);
void openttd_rust_station_watched(OpenTTDBaseStation *, const OpenTTDStationServices *);
void openttd_rust_station_tick(const OpenTTDStationServices *, const OpenTTDStationLinks *);
void openttd_rust_station_stale(OpenTTDBaseStation *, const OpenTTDStationServices *, const OpenTTDStationLinks *);
void openttd_rust_station_reroute(OpenTTDBaseStation *, uint8_t, uint16_t, uint16_t, const OpenTTDStationServices *, const OpenTTDStationLinks *);
void openttd_rust_station_monthly(const OpenTTDStationServices *);
void openttd_rust_station_modify_rating(OpenTTDStationService *, int32_t);
uint32_t openttd_rust_station_distribute(uint8_t, uint32_t, OpenTTDCargoSource, void *, uint8_t, const OpenTTDStationServices *);

}
#endif
