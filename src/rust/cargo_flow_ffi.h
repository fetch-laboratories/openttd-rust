/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file cargo_flow_ffi.h Canonical flow owners and completed-job live application. */
#ifndef RUST_CARGO_FLOW_FFI_H
#define RUST_CARGO_FLOW_FFI_H
#include "service_defaults.hpp"
#include <stdint.h>
#include <stddef.h>
#ifdef __cplusplus
extern "C" {
#endif
typedef struct OpenTTDCargoFlow OpenTTDCargoFlow;
typedef struct OpenTTDCargoFlowMap OpenTTDCargoFlowMap;
typedef struct {
	uint32_t cumulative;
	uint16_t station;
	uint8_t found;
} OpenTTDCargoShare;
typedef struct {
	OpenTTDCargoFlow *flow;
	uint16_t origin;
	uint8_t found;
} OpenTTDCargoOrigin;
struct OpenTTDLiveGraph;
typedef struct OpenTTDCargoFlowServices {
	const OpenTTDLiveGraph *snapshot;
	const OpenTTDLiveGraph *live_graph;
	void *context;
	uint32_t (*read)(void *, uint8_t, uint16_t, uint16_t) OPENTTD_SERVICE_DEFAULT(read);
	OpenTTDCargoFlowMap *(*job_flows)(void *, uint16_t) OPENTTD_SERVICE_DEFAULT(job_flows);
	OpenTTDCargoFlowMap *(*live_flows)(void *, uint16_t) OPENTTD_SERVICE_DEFAULT(live_flows);
	void (*reroute)(void *, uint16_t, uint16_t) OPENTTD_SERVICE_DEFAULT(reroute);
	void (*finish)(void *, uint16_t, uint8_t) OPENTTD_SERVICE_DEFAULT(finish);
} OpenTTDCargoFlowServices;
/* Each facade has one opaque owner or a nonowning borrow of a stable map entry.
 * Maps own raw-allocated flow nodes, unchanged by insertion or unrelated erase.
 * Share/origin iterators retain scalar keys only; each read borrows for that call.
 * All operations are serial; job maps are private to their worker until join.
 * No C++ STL object is viewed by Rust. Clone returns an independent owner;
 * map insertion copies the supplied flow. Destroy each owned handle once.
 * A map borrow ends when that entry is erased; no retained Rust reference crosses
 * a service. Services are noexcept, never run script VMs, and may read/mutate
 * canonical cargo through rerouting; raw owner accesses end before each call.
 * Save/load errors remain entirely in C++ adapters. Panics/alloc failures abort.
 * Snapshot and live graph pointers are canonical Rust owners, stable for this
 * application under the game-state mutex; rerouting cannot delete graph shells.
 * read: 0 graph valid,3 live station valid,4 edge count,5 edge flow,6 destination
 * node,8 manual distribution,9 live data empty,10 live graph ID,11 live node ID,
 * 12 job graph ID. Graph queries execute directly in Rust. finish clears data when requested
 * then invalidates the station window. Native thread join precedes application.
 */
OpenTTDCargoFlow *openttd_rust_flow_new(uint16_t, uint32_t, uint8_t);
OpenTTDCargoFlow *openttd_rust_flow_clone(const OpenTTDCargoFlow *);
void openttd_rust_flow_destroy(OpenTTDCargoFlow *);
uint32_t openttd_rust_flow_read(const OpenTTDCargoFlow *, uint8_t, uint16_t);
void openttd_rust_flow_change(OpenTTDCargoFlow *, uint8_t, uint16_t, uint32_t);
void openttd_rust_flow_swap(OpenTTDCargoFlow *, OpenTTDCargoFlow *);
OpenTTDCargoShare openttd_rust_flow_share(const OpenTTDCargoFlow *, uint64_t, uint8_t);
uint16_t openttd_rust_flow_via(const OpenTTDCargoFlow *, uint8_t, uint16_t, uint16_t, uint8_t *, uint32_t (*)());
OpenTTDCargoFlowMap *openttd_rust_flow_map_new();
OpenTTDCargoFlowMap *openttd_rust_flow_map_clone(const OpenTTDCargoFlowMap *);
void openttd_rust_flow_map_destroy(OpenTTDCargoFlowMap *);
OpenTTDCargoOrigin openttd_rust_flow_map_at(const OpenTTDCargoFlowMap *, uint32_t, uint8_t);
uint8_t openttd_rust_flow_map_insert(OpenTTDCargoFlowMap *, uint16_t, const OpenTTDCargoFlow *);
void openttd_rust_flow_map_erase(OpenTTDCargoFlowMap *, uint16_t);
uint32_t openttd_rust_flow_map_read(const OpenTTDCargoFlowMap *, uint8_t, uint16_t, uint16_t);
void openttd_rust_flow_map_change(OpenTTDCargoFlowMap *, uint8_t, uint16_t, uint16_t, uint32_t);
/* Deleted origin IDs are visited synchronously, in ascending order. */
void openttd_rust_flow_map_delete(OpenTTDCargoFlowMap *, uint16_t, void *, void (*)(void *, uint16_t));
void openttd_rust_flow_apply(const OpenTTDCargoFlowServices *);
#ifdef __cplusplus
}
#endif
#endif /* RUST_CARGO_FLOW_FFI_H */
