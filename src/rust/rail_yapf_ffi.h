/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file rail_yapf_ffi.h The rail pathfinder's train controller entries. */
#ifndef OPENTTD_RUST_RAIL_YAPF_FFI_H
#define OPENTTD_RUST_RAIL_YAPF_FFI_H
#include <stdint.h>
extern "C" {
/* The entry runs synchronously under the game-state mutex. */
void openttd_rust_rail_invalidate(uint32_t tile, uint8_t track);
}
/** The rail search's DumpState through the native DumpTarget, which #725 retires. */
void YapfRailDumpState(const uint32_t *words, uint32_t length) noexcept;
#endif
