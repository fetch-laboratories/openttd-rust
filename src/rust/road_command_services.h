/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file road_command_services.h Canonical typed road-build command adapter for native callers. */
#ifndef RUST_ROAD_COMMAND_SERVICES_H
#define RUST_ROAD_COMMAND_SERVICES_H
#include "road_command_ffi.h"
#include "rail_command_services.h"
#include "command_cost_services.h"
#include "../roadveh.h"
#include <optional>
struct OpenTTDRoadConvertFrame { std::vector<RoadVehicle *> affected; };
struct RoadConversionScratch {
	OpenTTDRoadConvertFrame frame;
	OpenTTDRailConvertFrame area;
	CommandCost error, ret;
};
/* No world layout enters Rust. This caller-owned stack slot supplies only the
 * extra results a direct nested owner call needs. Conversion remains lazy. */
struct OpenTTDRoadCommandScratch {
	std::optional<CommandCost> ret;
	std::optional<RoadConversionScratch> conversion;
};
/* Each outer native entry owns one scratch and two results, with no table copies,
 * heap allocation or state. Conversion is lazy, as is a road vehicle slot's context. */
struct RustRoadCommandContext {
	OpenTTDRoadCommandScratch scratch;
	CommandCost cost, ret;
	OpenTTDRoadCommandContext view{&GetRustRoadCommandServices(), &scratch, RustCommandCost(&cost), RustCommandCost(&ret)};
};
struct OpenTTDRoadCommandSlot { std::optional<RustRoadCommandContext> road; };
#endif /* RUST_ROAD_COMMAND_SERVICES_H */
