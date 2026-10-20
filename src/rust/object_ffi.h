/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file object_ffi.h Canonical object counts and command scratch. */
#ifndef RUST_OBJECT_FFI_H
#define RUST_OBJECT_FFI_H
#include <cstddef>
#include <cstdint>
struct OpenTTDObjectClearArea { uint32_t first_tile, tile; uint16_t width, height; };
/* All calls are serialized by the game-state mutex, including across the GUI and
 * game threads. Indices survive append. No element pointer or allocation escapes;
 * callbacks may reenter only after every Rust state borrow has ended. */
extern "C" {
void openttd_rust_object_counts_reset();
void openttd_rust_object_count_increment(uint16_t);
void openttd_rust_object_count_decrement(uint16_t);
uint16_t openttd_rust_object_count(uint16_t);
void openttd_rust_object_areas_clear();
size_t openttd_rust_object_areas_find(uint32_t, uint32_t);
OpenTTDObjectClearArea openttd_rust_object_areas_get(size_t);
size_t openttd_rust_object_areas_append(OpenTTDObjectClearArea);
void openttd_rust_object_areas_set_first(size_t, uint32_t);
}
#endif /* RUST_OBJECT_FFI_H */
