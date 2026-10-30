/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file linkgraph_ffi.h Owned computation of a copied link graph job. */
#ifndef OPENTTD_RUST_LINKGRAPH_FFI_H
#define OPENTTD_RUST_LINKGRAPH_FFI_H

/** Opaque live native entity, distinct from Rust-owned boundary state. */
#ifdef __cplusplus
class LinkGraph;
struct BaseStation;
using OpenTTDBaseStation = BaseStation;
using OpenTTDLinkGraph = LinkGraph;
#endif
#include <stdint.h>
#include <stddef.h>
#include "call_ffi.h"
#ifdef __cplusplus
extern "C" {
#endif
typedef struct {
	uint32_t supply, demand, station, x, y, edge_begin, edge_count;
} OpenTTDLinkGraphNode;
typedef struct {
	uint32_t capacity, travel_time, dest;
} OpenTTDLinkGraphEdge;
typedef struct {
	uint32_t accuracy, demand_distance, demand_size, saturation, distribution, express, map_max_x, map_max_y, runtime;
} OpenTTDLinkGraphSettings;
typedef struct {
	uint32_t node, origin, via, cumulative, unrestricted, has_share;
} OpenTTDLinkGraphShare;
typedef struct OpenTTDLinkGraphResult OpenTTDLinkGraphResult;
/* The call copies plain input arrays into Rust-owned state; empty arrays permit null. Only the
 * nonthrowing abort callback reads the job's atomic flag. Result arrays borrow the opaque owner;
 * C++ copies them before destroying it once, also on allocation exceptions.
 * Inputs and settings are live, aligned and readable; array extents are <= PTRDIFF_MAX and obey
 * the original graph invariants. The abort callback cannot reenter Rust or mutate the snapshot. */
OpenTTDLinkGraphResult *OPENTTD_RUST_CALL openttd_rust_linkgraph_run(const OpenTTDLinkGraphNode *, size_t, const OpenTTDLinkGraphEdge *, size_t, const OpenTTDLinkGraphSettings *, const void *, uint8_t (OPENTTD_RUST_CALL *)(const void *));
const OpenTTDLinkGraphShare *OPENTTD_RUST_CALL openttd_rust_linkgraph_shares(const OpenTTDLinkGraphResult *, size_t *);
const uint32_t *OPENTTD_RUST_CALL openttd_rust_linkgraph_edges(const OpenTTDLinkGraphResult *);
void OPENTTD_RUST_CALL openttd_rust_linkgraph_destroy(OpenTTDLinkGraphResult *);
/* Contents owner. Canonical edge scalars have the BaseEdge layout; a borrowed
 * pointer expires on edge/vector mutation. Node handles never copy records.
 * Save addresses expire on graph resize/destruction. SlObject and all error
 * cleanup run after these synchronous calls return, entirely in C++ frames. */
struct OpenTTDLiveGraph;
struct OpenTTDLiveEdge;
struct OpenTTDLiveNodeFields {
	uint32_t *supply, *demand;
	uint16_t *station;
	uint32_t *xy;
	int32_t *last_update;
};
OpenTTDLiveGraph *openttd_rust_live_graph_new(uint8_t, int32_t);
OpenTTDLiveGraph *openttd_rust_live_graph_copy(const OpenTTDLiveGraph *);
void openttd_rust_live_graph_delete(OpenTTDLiveGraph *);
uint16_t openttd_rust_live_graph_size(const OpenTTDLiveGraph *);
uint8_t *openttd_rust_live_graph_cargo(OpenTTDLiveGraph *);
int32_t *openttd_rust_live_graph_date(OpenTTDLiveGraph *);
void openttd_rust_live_graph_init(OpenTTDLiveGraph *, uint32_t);
void openttd_rust_live_graph_shift(OpenTTDLiveGraph *, int32_t);
void openttd_rust_live_graph_compress(OpenTTDLiveGraph *);
uint32_t openttd_rust_live_graph_scale(uint32_t, int32_t, int32_t);
uint32_t openttd_rust_live_graph_monthly(const OpenTTDLiveGraph *, uint32_t, int32_t);
OpenTTDLiveNodeFields openttd_rust_live_node_fields(OpenTTDLiveGraph *, uint16_t);
uint32_t openttd_rust_live_node_supply(const OpenTTDLiveGraph *, uint16_t);
uint16_t openttd_rust_live_node_station(const OpenTTDLiveGraph *, uint16_t);
const uint16_t *openttd_rust_live_node_station_address(const OpenTTDLiveGraph *, uint16_t);
void openttd_rust_live_node_location(OpenTTDLiveGraph *, uint16_t, uint32_t);
void openttd_rust_live_node_update_supply(OpenTTDLiveGraph *, uint16_t, uint32_t, int32_t);
const OpenTTDLiveEdge *openttd_rust_live_node_edges(const OpenTTDLiveGraph *, uint16_t, size_t *);
const OpenTTDLiveEdge *openttd_rust_live_node_edge(const OpenTTDLiveGraph *, uint16_t, uint16_t);
void openttd_rust_live_node_update_edge(OpenTTDLiveGraph *, uint16_t, uint16_t, uint32_t, uint32_t, uint32_t, uint8_t, int32_t, bool);
OpenTTDLiveEdge *openttd_rust_live_node_append_edge(OpenTTDLiveGraph *, uint16_t);
uint32_t openttd_rust_live_edge_travel_time(const OpenTTDLiveEdge *);
int32_t openttd_rust_live_edge_last_update(const OpenTTDLiveEdge *);
void openttd_rust_live_node_add_edge(OpenTTDLiveGraph *, uint16_t, uint16_t, uint32_t, uint32_t, uint32_t, uint8_t, int32_t);
void openttd_rust_live_edge_init(OpenTTDLiveEdge *, uint16_t);
void openttd_rust_live_edge_update(OpenTTDLiveEdge *, uint32_t, uint32_t, uint32_t, uint8_t, int32_t);
void openttd_rust_live_edge_restrict(OpenTTDLiveEdge *);
void openttd_rust_live_edge_release(OpenTTDLiveEdge *);
struct OpenTTDLiveGraphAccess { OpenTTDLiveGraph *owner; uint16_t index; };
struct OpenTTDLiveStationInput { uint32_t xy, demand; };
struct OpenTTDLiveGraphServices {
	bool assertions;
	OpenTTDLinkGraph *(*get)(uint16_t) noexcept;
	OpenTTDLinkGraph *(*get_if_valid)(uint16_t) noexcept;
	uint16_t (*station_id)(const OpenTTDBaseStation *) noexcept;
	OpenTTDLiveGraph *(*owner)(OpenTTDLinkGraph *) noexcept;
	OpenTTDLiveGraphAccess (*access)(OpenTTDLinkGraph *) noexcept;
	OpenTTDLiveStationInput (*station)(uint16_t, uint8_t) noexcept;
	void (*attach)(uint16_t, uint8_t, uint16_t, uint16_t) noexcept;
	void (*set_node)(uint16_t, uint8_t, uint16_t) noexcept;
	void (*delete_graph)(OpenTTDLinkGraph *) noexcept;
	uint16_t (*graph_id)(OpenTTDBaseStation *, uint8_t) noexcept;
	uint16_t (*node)(OpenTTDBaseStation *, uint8_t) noexcept;
	bool (*available)() noexcept;
	OpenTTDLinkGraph *(*allocate)(uint8_t) noexcept;
	void (*allocation_failure)() noexcept;
};
OpenTTDLinkGraphResult *openttd_rust_live_graph_run(const OpenTTDLiveGraph *, const OpenTTDLinkGraphSettings *, const void *, uint8_t (*)(const void *));
void openttd_rust_live_graph_merge(OpenTTDLinkGraph *, OpenTTDLinkGraph *, const OpenTTDLiveGraphServices *);
void openttd_rust_live_graph_remove(OpenTTDLinkGraph *, uint16_t, const OpenTTDLiveGraphServices *);
uint16_t openttd_rust_live_graph_add_station(OpenTTDLinkGraph *, uint16_t, const OpenTTDLiveGraphServices *);
#ifdef __cplusplus
}
const OpenTTDLiveGraphServices &GetRustLiveGraphServices() noexcept;
#endif
#endif /* OPENTTD_RUST_LINKGRAPH_FFI_H */
