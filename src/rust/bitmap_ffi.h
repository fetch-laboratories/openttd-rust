/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file bitmap_ffi.h Rust-owned BitmapTileArea storage; its leading geometry is a TileArea. */
#ifndef RUST_BITMAP_FFI_H
#define RUST_BITMAP_FFI_H
#include "services_ffi.h"
#include <cstdint>
struct OpenTTDBitmapTileArea;
struct OpenTTDBitmapArea { uint32_t tile; uint16_t w, h; };
extern "C" {
OpenTTDBitmapTileArea *openttd_rust_bitmap_new();
void openttd_rust_bitmap_destroy(OpenTTDBitmapTileArea *);
void openttd_rust_bitmap_initialize(OpenTTDBitmapTileArea *, OpenTTDBitmapArea);
void openttd_rust_bitmap_set(OpenTTDBitmapTileArea *, uint32_t, const OpenTTDSharedServices *);
bool openttd_rust_bitmap_has(const OpenTTDBitmapTileArea *, uint32_t, const OpenTTDSharedServices *);
}
#endif /* RUST_BITMAP_FFI_H */
