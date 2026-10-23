/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_linkgraph.h Canonical native graph allocation and station membership leaves. */
#ifndef RUST_SERVICES_LINKGRAPH_H
#define RUST_SERVICES_LINKGRAPH_H
#include "../linkgraph/linkgraph.h"
#include "../linkgraph/linkgraphschedule.h"
#include "../station_base.h"
#include "../debug.h"
#include "linkgraph_ffi.h"
#include "station_service_ffi.h"

inline bool RustLinkGraphAvailable() noexcept { return LinkGraph::CanAllocateItem(); }
inline OpenTTDLinkGraph *RustLinkGraphAllocate(uint8_t cargo) noexcept
{
	LinkGraph *lg = new LinkGraph(cargo);
	LinkGraphSchedule::instance.Queue(lg);
	return lg;
}
inline void RustLinkGraphAllocationFailure() noexcept { Debug(misc, 0, "Can't allocate link graph"); }
#endif /* RUST_SERVICES_LINKGRAPH_H */
