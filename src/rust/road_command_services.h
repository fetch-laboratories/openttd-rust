/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file road_command_services.h Canonical typed road-build command adapter for native callers. */
#ifndef RUST_ROAD_COMMAND_SERVICES_H
#define RUST_ROAD_COMMAND_SERVICES_H
#include "../road_cmd.h"
CommandCost RustRoadBuildCommand(DoCommandFlags, TileIndex, RoadBits, RoadType, DisallowedRoadDirections, TownID) noexcept;
#endif /* RUST_ROAD_COMMAND_SERVICES_H */
