/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file rail_command_services.h Native conversion cursor shared by command owners. */
#ifndef RUST_RAIL_COMMAND_SERVICES_H
#define RUST_RAIL_COMMAND_SERVICES_H
#include "../tilearea_type.h"
#include "../train.h"
struct OpenTTDRailConvertFrame {
	std::unique_ptr<TileIterator> iter;
	std::vector<Train *> affected;
};
#endif /* RUST_RAIL_COMMAND_SERVICES_H */
