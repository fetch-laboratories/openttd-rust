/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file waypoint_layout.h Caller-owned native layout and iterator lifetimes. */
#ifndef RUST_WAYPOINT_LAYOUT_H
#define RUST_WAYPOINT_LAYOUT_H
#include "../station_layout_type.h"
/* The facade owns this frame; init occurs at the original layout construction
 * point, and the iterator is reset for each original traversal. Neither optional
 * allocates. Specs and their layout spans remain immutable during the command. */
struct OpenTTDWaypointLayout {
	std::optional<RailStationTileLayout> layout;
	std::optional<RailStationTileLayout::Iterator> iterator;
};
#endif /* RUST_WAYPOINT_LAYOUT_H */
