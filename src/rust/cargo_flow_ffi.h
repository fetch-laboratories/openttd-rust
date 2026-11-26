/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file cargo_flow_ffi.h Rust flows, flow maps and the finished link graph job's application. */
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
/** A share of a flow, as its shares iterator returns it; found is 0 at end(). */
typedef struct {
	uint32_t cumulative;
	uint16_t station;
	uint8_t found;
} OpenTTDCargoShare;
/** An entry of a flow map, as its iterator returns it; found is 0 at end(). */
typedef struct {
	OpenTTDCargoFlow *flow;
	uint16_t origin;
	uint8_t found;
} OpenTTDCargoOrigin;
struct GoodsEntry;
struct OpenTTDStationServices;
struct OpenTTDStationLinks;
struct OpenTTDLinkGraphJobRecord;
typedef struct OpenTTDCargoFlowServices {
	void (*clear_data)(GoodsEntry *) noexcept OPENTTD_SERVICE_DEFAULT(clear_data);
	bool (*data_is_empty)(const GoodsEntry *) noexcept OPENTTD_SERVICE_DEFAULT(data_is_empty);
	OpenTTDCargoFlowMap *(*get_or_create_data_flows)(GoodsEntry *) noexcept OPENTTD_SERVICE_DEFAULT(get_or_create_data_flows);
	OpenTTDLinkGraphJobRecord *job;
	const OpenTTDStationLinks *links;
	const OpenTTDStationServices *stations;
} OpenTTDCargoFlowServices;
/* A map owns each flow in its own allocation, which insertion and the erasure of another origin
 * leave in place, so a borrowed flow stays valid until its own entry is erased. Calls are serial:
 * the game-state mutex for a station's flows, and the joined job for its maps. */
OpenTTDCargoFlow *openttd_rust_flow_new(uint16_t st, uint32_t flow, uint8_t restricted);
void openttd_rust_flow_destroy(OpenTTDCargoFlow *flow);
void openttd_rust_flow_append_share(OpenTTDCargoFlow *flow, uint16_t st, uint32_t amount, uint8_t restricted);
uint32_t openttd_rust_flow_get_unrestricted(const OpenTTDCargoFlow *flow);
OpenTTDCargoShare openttd_rust_flow_shares_begin(const OpenTTDCargoFlow *flow);
OpenTTDCargoShare openttd_rust_flow_shares_next(const OpenTTDCargoFlow *flow, uint32_t key);
uint16_t openttd_rust_flow_get_via(const OpenTTDCargoFlow *flow, uint32_t (*random_range)(uint32_t) noexcept);
OpenTTDCargoFlowMap *openttd_rust_flow_map_new();
void openttd_rust_flow_map_destroy(OpenTTDCargoFlowMap *map);
OpenTTDCargoOrigin openttd_rust_flow_map_begin(const OpenTTDCargoFlowMap *map);
OpenTTDCargoOrigin openttd_rust_flow_map_find(const OpenTTDCargoFlowMap *map, uint16_t origin);
OpenTTDCargoOrigin openttd_rust_flow_map_next(const OpenTTDCargoFlowMap *map, uint16_t origin);
/* Copies flow, which the caller keeps, into a new entry where origin has none; returns 1 if it did. */
uint8_t openttd_rust_flow_map_insert(OpenTTDCargoFlowMap *map, uint16_t origin, const OpenTTDCargoFlow *flow);
uint32_t openttd_rust_flow_map_get_flow(const OpenTTDCargoFlowMap *map);
uint32_t openttd_rust_flow_map_get_flow_via(const OpenTTDCargoFlowMap *map, uint16_t via);
uint32_t openttd_rust_flow_map_get_flow_from(const OpenTTDCargoFlowMap *map, uint16_t from);
uint32_t openttd_rust_flow_map_get_flow_from_via(const OpenTTDCargoFlowMap *map, uint16_t from, uint16_t via);
/* LinkGraphJob::~LinkGraphJob after the join, the CleaningPool() return and the abort assertion. */
void openttd_rust_flow_apply(const OpenTTDCargoFlowServices *services);
#ifdef __cplusplus
}
#endif
#endif /* RUST_CARGO_FLOW_FFI_H */
