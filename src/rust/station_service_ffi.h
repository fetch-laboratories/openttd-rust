/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file station_service_ffi.h Canonical Rust station service state and direct world leaves. */
#ifndef RUST_STATION_SERVICE_FFI_H
#define RUST_STATION_SERVICE_FFI_H
#include "service_defaults.hpp"
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
enum OrderType : uint8_t;
struct OpenTTDOrderDestination {
	uint16_t destination;
	OrderType kind;
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
	OpenTTDLoadingStation (*station)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(station);
	uint8_t (*station_owner)(OpenTTDBaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(station_owner);
	OpenTTDStationExclusivity (*station_exclusivity)(OpenTTDBaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(station_exclusivity);
	OpenTTDBaseStation * (*station_next)(void *) noexcept OPENTTD_SERVICE_DEFAULT(station_next);
	OpenTTDRatingCounts (*rating_counts)(OpenTTDBaseStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(rating_counts);
	OpenTTDBaseStation * (*get)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(get);
	OpenTTDStationService * (*owner)(OpenTTDBaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(owner);
	size_t (*pool_size)() noexcept OPENTTD_SERVICE_DEFAULT(pool_size);
	OpenTTDEnvironmentServices environment;
	uint8_t (*facilities)(OpenTTDBaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(facilities);
	bool (*in_use)(OpenTTDBaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(in_use);
	bool (*is_station)(OpenTTDBaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(is_station);
	bool (*local_owner)(OpenTTDBaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(local_owner);
	bool (*rectangle_empty)(OpenTTDBaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(rectangle_empty);
	bool (*town_statue)(OpenTTDBaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(town_statue);
	bool (*select_goods)() noexcept OPENTTD_SERVICE_DEFAULT(select_goods);
	bool (*rating_cheat)() noexcept OPENTTD_SERVICE_DEFAULT(rating_cheat);
	uint32_t (*available)(OpenTTDBaseStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(available);
	bool (*cargo_valid)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_valid);
	bool (*rating_callback_enabled)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(rating_callback_enabled);
	bool (*passenger_cargo)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(passenger_cargo);
	bool (*packet_available)() noexcept OPENTTD_SERVICE_DEFAULT(packet_available);
	void (*rating_redraw)(OpenTTDBaseStation *, bool) noexcept OPENTTD_SERVICE_DEFAULT(rating_redraw);
	void (*acceptance_news)(OpenTTDBaseStation *, uint64_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(acceptance_news);
	void (*acceptance_redraw)(OpenTTDBaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(acceptance_redraw);
	void (*delete_station)(OpenTTDBaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(delete_station);
	void (*acceptance_animation)(OpenTTDBaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(acceptance_animation);
	void (*watched_animation)(uint32_t, uint64_t) noexcept OPENTTD_SERVICE_DEFAULT(watched_animation);
	void (*append_packet)(OpenTTDBaseStation *, uint8_t, uint32_t, OpenTTDCargoSource) noexcept OPENTTD_SERVICE_DEFAULT(append_packet);
	void (*station_list_invalidation)(OpenTTDBaseStation *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_list_invalidation);
	void (*new_cargo_animation)(OpenTTDBaseStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(new_cargo_animation);
	void (*waiting_redraw)(OpenTTDBaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(waiting_redraw);
	uint16_t (*rating_callback)(OpenTTDBaseStation *, uint8_t, uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rating_callback);
	void (*with_tiles)(OpenTTDBaseStation *, void *, OpenTTDStationScope) noexcept OPENTTD_SERVICE_DEFAULT(with_tiles);
	uint32_t (*tile_next)(void *) noexcept OPENTTD_SERVICE_DEFAULT(tile_next);
	void (*accept_tile)(void *, uint32_t, uint64_t *) noexcept OPENTTD_SERVICE_DEFAULT(accept_tile);
	const uint32_t * (*acceptance_amounts)(void *) noexcept OPENTTD_SERVICE_DEFAULT(acceptance_amounts);
	void (*with_truncation)(OpenTTDBaseStation *, uint8_t, uint32_t, void *, OpenTTDStationScope) noexcept OPENTTD_SERVICE_DEFAULT(with_truncation);
	OpenTTDTruncatedSource (*truncate_next)(void *) noexcept OPENTTD_SERVICE_DEFAULT(truncate_next);
	const OpenTTDSharedServices *shared;
};

struct OpenTTDStationEdge {
	OpenTTDBaseStation *destination;
	uint32_t distance;
};
struct OpenTTDStationLinks {
	OpenTTDLinkGraph * (*graph)(OpenTTDBaseStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(graph);
	bool (*automatic_distribution)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(automatic_distribution);
	uint16_t (*node)(OpenTTDBaseStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(node);
	int32_t (*date)() noexcept OPENTTD_SERVICE_DEFAULT(date);
	uint16_t (*station_id)(OpenTTDBaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(station_id);
	OpenTTDStationEdge (*edge)(OpenTTDBaseStation *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(edge);
	void (*delete_flows)(OpenTTDBaseStation *, uint8_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(delete_flows);
	void (*restrict_flows)(OpenTTDBaseStation *, uint8_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(restrict_flows);
	OpenTTDOrderList * (*order_list)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(order_list);
	size_t (*order_pool_size)() noexcept OPENTTD_SERVICE_DEFAULT(order_pool_size);
	size_t (*order_count)(OpenTTDOrderList *) noexcept OPENTTD_SERVICE_DEFAULT(order_count);
	OpenTTDOrderDestination (*order_destination)(OpenTTDOrderList *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(order_destination);
	OpenTTDVehicle * (*order_vehicle)(OpenTTDOrderList *) noexcept OPENTTD_SERVICE_DEFAULT(order_vehicle);
	OpenTTDVehicle * (*next_vehicle)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(next_vehicle);
	OpenTTDVehicle * (*next_shared)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(next_shared);
	bool (*stopped_in_depot)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(stopped_in_depot);
	int32_t (*last_service)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(last_service);
	uint8_t (*cargo_type)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_type);
	void (*reroute)(OpenTTDBaseStation *, OpenTTDVehicle *, uint8_t, uint16_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(reroute);
};

struct OpenTTDStationLoading {
	OpenTTDLoadingControl (*control)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(control);
	OpenTTDLoadingPart (*part)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(part);
	OpenTTDLoadingStats (*set_refit_capacity_and_stats)(OpenTTDVehicle *, OpenTTDVehicle *, uint8_t, uint16_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_refit_capacity_and_stats);
	OpenTTDLoadingWait (*wait_observation)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(wait_observation);
	OpenTTDLoadEngine (*load_engine)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(load_engine);
	uint8_t (*grf_version)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(grf_version);
	bool (*engine_load_callback)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_load_callback);
	bool (*engine_no_multiplier)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_no_multiplier);
	OpenTTDDeliveryRemaining (*delivery_remaining)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(delivery_remaining);
	OpenTTDCargoStored (*cargo_stored)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_stored);
	OpenTTDCargoReservation (*cargo_reservation)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_reservation);
	OpenTTDCargoCapacity (*cargo_capacity)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_capacity);
	uint8_t (*vehicle_type)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_type);
	uint8_t (*cargo_type)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_type);
	uint16_t (*cargo_cap)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_cap);
	uint32_t (*stored_count)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(stored_count);
	uint32_t (*unload_count)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(unload_count);
	uint32_t (*deliver_count)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(deliver_count);
	uint32_t (*transfer_count)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(transfer_count);
	uint32_t (*load_count)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(load_count);
	uint8_t (*unload_type)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(unload_type);
	bool (*has_orders)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(has_orders);
	uint16_t (*load_unload_ticks)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(load_unload_ticks);
	uint8_t (*status)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(status);
	bool (*has_articulated_part)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(has_articulated_part);
	bool (*is_articulated_part)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(is_articulated_part);
	bool (*is_rear_dualheaded)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(is_rear_dualheaded);
	bool (*is_normal_aircraft)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(is_normal_aircraft);
	bool (*is_multiheaded)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(is_multiheaded);
	uint16_t (*cargo_multiplier)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_multiplier);
	uint16_t (*cached_total_length)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cached_total_length);
	bool (*full_load_order)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(full_load_order);
	uint8_t (*vehicle_owner)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_owner);
	bool (*passenger_cargo)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(passenger_cargo);
	bool (*station_length_penalty)() noexcept OPENTTD_SERVICE_DEFAULT(station_length_penalty);
	uint8_t (*current_company)() noexcept OPENTTD_SERVICE_DEFAULT(current_company);
	const OpenTTDVehicleServices *vehicle;
	uint32_t (*total_count)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(total_count);
	int32_t (*order_time)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(order_time);
	uint64_t (*refit_mask)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(refit_mask);
	void (*set_cargo_unloading)(OpenTTDVehicle *, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_cargo_unloading);
	void (*set_loading_finished)(OpenTTDVehicle *, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_loading_finished);
	void (*set_stop)(OpenTTDVehicle *, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_stop);
	void (*set_load_ticks)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_load_ticks);
	void (*set_speed)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_speed);
	void (*subtract_profit)(OpenTTDVehicle *, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(subtract_profit);
	void (*unknown_load_result)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(unknown_load_result);
	void (*vehicle_dirty)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_dirty);
	bool (*indicator)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(indicator);
	void (*show_indicator)(OpenTTDVehicle *, uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(show_indicator);
	void (*set_current_company)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_current_company);
	OpenTTDVehicle * (*next_vehicle)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(next_vehicle);
	OpenTTDVehicle * (*next_articulated)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(next_articulated);
	OpenTTDVehicle * (*other_multiheaded)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(other_multiheaded);
	OpenTTDVehicle * (*first_engine_part)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(first_engine_part);
	OpenTTDVehicle * (*first_vehicle)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(first_vehicle);
	OpenTTDVehicle * (*previous_vehicle)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(previous_vehicle);
	void (*with_next_stations)(OpenTTDVehicle *, void *, OpenTTDStationScope) noexcept OPENTTD_SERVICE_DEFAULT(with_next_stations);
	void (*stage)(OpenTTDStation *, OpenTTDVehicle *, uint8_t, void *, OpenTTDNativeCargoPayment *) noexcept OPENTTD_SERVICE_DEFAULT(stage);
	void (*return_cargo)(OpenTTDStation *, OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(return_cargo);
	uint32_t (*unload_cargo)(OpenTTDStation *, OpenTTDVehicle *, uint32_t, OpenTTDNativeCargoPayment *) noexcept OPENTTD_SERVICE_DEFAULT(unload_cargo);
	uint32_t (*load_cargo)(OpenTTDStation *, OpenTTDVehicle *, uint32_t, void *) noexcept OPENTTD_SERVICE_DEFAULT(load_cargo);
	void (*reserve_cargo)(OpenTTDStation *, OpenTTDVehicle *, uint32_t, void *) noexcept OPENTTD_SERVICE_DEFAULT(reserve_cargo);
	void (*deliver_to_transfer)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(deliver_to_transfer);
	void (*deliver_to_keep)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(deliver_to_keep);
	bool (*has_cargo_for)(OpenTTDStation *, uint8_t, void *) noexcept OPENTTD_SERVICE_DEFAULT(has_cargo_for);
	uint32_t (*platform_length)(OpenTTDStation *, OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(platform_length);
	uint32_t (*vehicle_tile)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_tile);
	uint16_t (*station_index)(OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(station_index);
	uint32_t (*station_cargo_total)(OpenTTDStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(station_cargo_total);
	uint16_t (*load_property)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(load_property);
	uint16_t (*load_callback)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(load_callback);
	OpenTTDNativeCargoPayment * (*get_payment)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(get_payment);
	OpenTTDNativeCargoPayment * (*create_payment)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(create_payment);
	void (*delete_payment)(OpenTTDNativeCargoPayment *) noexcept OPENTTD_SERVICE_DEFAULT(delete_payment);
	void (*flush_delivery)() noexcept OPENTTD_SERVICE_DEFAULT(flush_delivery);
	void (*vehicle_new_cargo)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_new_cargo);
	void (*cargo_taken)(OpenTTDStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_taken);
	void (*train_loads)(OpenTTDStation *, OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(train_loads);
	void (*road_loads)(OpenTTDStation *, OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(road_loads);
	void (*vehicle_empty)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_empty);
	void (*station_dirty)(OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(station_dirty);
};

const OpenTTDStationServices &GetRustStationServices() noexcept;
const OpenTTDStationLoading &GetRustStationLoading() noexcept;
const OpenTTDStationLinks &GetRustStationLinks() noexcept;
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
struct OpenTTDVehicleVisits;
struct OpenTTDCommandCost;
void openttd_rust_station_load(OpenTTDStationService *, const OpenTTDStationServices *, const OpenTTDStationLoading *, const OpenTTDVehicleVisits *, OpenTTDCommandCost *, OpenTTDCommandCost *);
void openttd_rust_station_acceptance(OpenTTDBaseStation *, const OpenTTDStationServices *, uint8_t);
void openttd_rust_station_watched(OpenTTDBaseStation *, const OpenTTDStationServices *);
void openttd_rust_station_tick(const OpenTTDStationServices *, const OpenTTDStationLinks *, const OpenTTDVehicleVisits *);
void openttd_rust_station_stale(OpenTTDBaseStation *, const OpenTTDStationServices *, const OpenTTDStationLinks *, const OpenTTDVehicleVisits *);
void openttd_rust_station_reroute(OpenTTDBaseStation *, uint8_t, uint16_t, uint16_t, const OpenTTDStationServices *, const OpenTTDStationLinks *);
void openttd_rust_station_monthly(const OpenTTDStationServices *);
void openttd_rust_station_modify_rating(OpenTTDStationService *, int32_t);
uint32_t openttd_rust_station_distribute(uint8_t, uint32_t, OpenTTDCargoSource, void *, uint8_t, const OpenTTDStationServices *);

}
#endif
