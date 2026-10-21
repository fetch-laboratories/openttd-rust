/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file tile_owner_services.h Call-local, constructed native ownership scratch. */
#ifndef RUST_TILE_OWNER_SERVICES_H
#define RUST_TILE_OWNER_SERVICES_H
#include "tile_owner_ffi.h"
#include "road_command_services.h"
/* The road context supplies two costs; error is the third distinct live slot.
 * Only immutable tables and scratch addresses cross FFI; no map, pool or
 * object reference is retained. */
struct RustTileOwnerContext {
	RustRoadCommandContext road;
	CommandCost error;
	OpenTTDTileOwnerContext view{&GetRustTileOwnerServices(), RustCommandCost(&road.cost), RustCommandCost(&road.ret), RustCommandCost(&error), &road.view};
};
#endif /* RUST_TILE_OWNER_SERVICES_H */
