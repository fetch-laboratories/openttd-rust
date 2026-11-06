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
class LinkGraphJob;
struct BaseStation;
using OpenTTDBaseStation = BaseStation;
using OpenTTDLinkGraph = LinkGraph;
#endif
#include <stdint.h>
#include <stddef.h>
#include "call_ffi.h"
#include "../timer/timer_game_economy.h"
#include "rust_vector.h"
#include <atomic>
struct OpenTTDCargoFlowMap;
struct RustPoolDescriptor;
#include "abi-record-station_service.h"
#ifdef __cplusplus
extern "C" {
#endif
OpenTTDLinkGraphJobRecord *openttd_rust_linkgraph_job_new(const OpenTTDLiveGraph *, uint16_t, const OpenTTDLinkGraphJobSettings *);
void openttd_rust_linkgraph_job_delete(OpenTTDLinkGraphJobRecord *);
/* Contents owner. Canonical edge scalars have the BaseEdge layout; a borrowed
 * pointer expires on edge/vector mutation. Node handles never copy records.
 * Save addresses expire on graph resize/destruction. SlObject and all error
 * cleanup run after these synchronous calls return, entirely in C++ frames. */
struct OpenTTDLiveGraph;
struct OpenTTDLiveEdge;
OpenTTDLiveGraph *openttd_rust_live_graph_new(uint8_t);
OpenTTDLiveGraph *openttd_rust_live_graph_new_empty();
void openttd_rust_live_graph_delete(OpenTTDLiveGraph *);
void openttd_rust_live_graph_init(OpenTTDLiveGraph *, uint32_t);
void openttd_rust_live_graph_shift(OpenTTDLiveGraph *, int32_t);
void openttd_rust_live_graph_compress(OpenTTDLiveGraph *);
uint32_t openttd_rust_live_graph_scale(uint32_t, int32_t, int32_t);
uint32_t openttd_rust_live_graph_monthly(const OpenTTDLiveGraph *, uint32_t);
void openttd_rust_live_node_location(OpenTTDLiveGraph *, uint16_t, uint32_t);
void openttd_rust_live_node_update_supply(OpenTTDLiveGraph *, uint16_t, uint32_t);
void openttd_rust_live_node_update_edge(OpenTTDLiveGraph *, uint16_t, uint16_t, uint32_t, uint32_t, uint32_t, uint8_t, bool);
OpenTTDLiveEdge *openttd_rust_live_node_append_edge(OpenTTDLiveGraph *, uint16_t);
void openttd_rust_live_node_add_edge(OpenTTDLiveGraph *, uint16_t, uint16_t, uint32_t, uint32_t, uint32_t, uint8_t);
void openttd_rust_live_edge_init(OpenTTDLiveEdge *, uint16_t);
struct OpenTTDLiveStationInput { uint32_t xy, demand; };
struct OpenTTDLiveGraphServices {
	bool assertions;
	/* Canonical member address: load at game entry, after construction and before teardown. */
	const RustPoolDescriptor *const *pool;
	uint16_t (*station_id)(const OpenTTDBaseStation *) noexcept;
	uint16_t (*index)(const OpenTTDLiveGraph *) noexcept;
	OpenTTDLiveStationInput (*station)(uint16_t, uint8_t) noexcept;
	void (*attach)(uint16_t, uint8_t, uint16_t, uint16_t) noexcept;
	void (*set_node)(uint16_t, uint8_t, uint16_t) noexcept;
	void (*delete_graph)(OpenTTDLiveGraph *) noexcept;
	bool (*available)() noexcept;
	OpenTTDLiveGraph *(*allocate)(uint8_t) noexcept;
};
void openttd_rust_live_graph_merge(OpenTTDLiveGraph *, OpenTTDLiveGraph *, const OpenTTDLiveGraphServices *);
void openttd_rust_live_graph_remove(OpenTTDLiveGraph *, uint16_t, const OpenTTDLiveGraphServices *);
uint16_t openttd_rust_live_graph_add_station(OpenTTDLiveGraph *, uint16_t, const OpenTTDLiveGraphServices *);
#ifdef __cplusplus
}
const OpenTTDLiveGraphServices &GetRustLiveGraphServices() noexcept;
#endif
#endif /* OPENTTD_RUST_LINKGRAPH_FFI_H */
