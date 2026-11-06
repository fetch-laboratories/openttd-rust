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
struct OpenTTDStationServices;
struct OpenTTDStationLinks;
struct OpenTTDLiveGraph;
struct OpenTTDLinkGraphJobRecord;
typedef struct OpenTTDCargoFlowServices {
	OpenTTDLinkGraphJobRecord *job;
	const OpenTTDLiveGraph *live_graph;
	void *context;
	const OpenTTDStationServices *stations;
	const OpenTTDStationLinks *links;
	bool (*data_is_empty)(void *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(data_is_empty);
	OpenTTDCargoFlowMap *(*live_flows)(void *, uint16_t) OPENTTD_SERVICE_DEFAULT(live_flows);
	void (*reroute)(void *, uint16_t, uint16_t) OPENTTD_SERVICE_DEFAULT(reroute);
	void (*finish)(void *, uint16_t, uint8_t) OPENTTD_SERVICE_DEFAULT(finish);
} OpenTTDCargoFlowServices;
/* Each facade holds one opaque owner or a nonowning borrow of a map entry, valid until that entry is
 * erased. Maps own raw-allocated flow nodes that insertion and unrelated erasure leave in place; job
 * maps are private to their worker until join, and rerouting cannot delete graph shells.
 * RerouteCargo only changes this station's cargo list and staged loading-vehicle cargo.
 * It cannot clear/replace station data or flows, delete/reuse the station, reset pools or
 * yield the game-state mutex. Live raw map/entry pointers therefore remain valid across
 * this audited leaf, as in the pinned native iterator loop; no Rust owner borrow crosses it.
 * All station/data/flow handles end before finish, which may clear the attachment.
 * Station validity, membership and live distribution use the existing station/graph services.
 * Only native-owned data emptiness remains a typed field leaf; job snapshots/results are direct.
 * finish clears data when requested, then invalidates the station window, after the thread join.
 * Calls are serial; clone and insertion copy into independent owners, destroyed once. Graph owners
 * stay live under the game-state mutex during application, and callbacks retain no owner references. */
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
