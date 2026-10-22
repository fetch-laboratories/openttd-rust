/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_bridge.h Canonical bridge specification/map observations. */
#ifndef RUST_SERVICES_BRIDGE_H
#define RUST_SERVICES_BRIDGE_H
#include "../bridge.h"
#include "../tunnelbridge_map.h"
inline BridgeType RustBridgeType(uint32_t tile) noexcept { return GetBridgeType(TileIndex{tile}); }
inline uint16_t RustBridgeSpecSpeed(BridgeType type) noexcept { return GetBridgeSpec(type)->speed; }
#endif /* RUST_SERVICES_BRIDGE_H */
