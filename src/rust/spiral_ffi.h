/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file spiral_ffi.h By-value spiral traversal without map ownership. */
#ifndef RUST_SPIRAL_FFI_H
#define RUST_SPIRAL_FFI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Copyable state: direction 0 NE, 1 SE, 2 SW, 3 NW, 255 initial odd tile. */
typedef struct OpenTTDRustSpiralState {
	uint32_t max_radius;
	uint32_t extent[4];
	uint32_t cur_radius;
	uint32_t position;
	uint32_t x;
	uint32_t y;
	uint8_t direction;
} OpenTTDRustSpiralState;

#ifdef __cplusplus
static_assert(sizeof(OpenTTDRustSpiralState) == 40 && alignof(OpenTTDRustSpiralState) == 4);
static_assert(offsetof(OpenTTDRustSpiralState, direction) == 36);
#endif

/** All arithmetic keeps the original uint32_t wrapping, including out-of-map coordinates.
 * Direction offsets are NE(-1,0), SE(0,1), SW(1,0), NW(0,-1); shell jumps are west(+1,-1). Map
 * dimensions are live operation inputs. State must come from initialization or advancement. */
OpenTTDRustSpiralState openttd_rust_spiral_square(uint32_t x, uint32_t y, uint32_t diameter, uint32_t size_x, uint32_t size_y); ///< diameter > 0.
OpenTTDRustSpiralState openttd_rust_spiral_hole(uint32_t x, uint32_t y, uint32_t radius, uint32_t width, uint32_t height, uint32_t size_x, uint32_t size_y); ///< radius > 0.
OpenTTDRustSpiralState openttd_rust_spiral_advance(OpenTTDRustSpiralState state, uint32_t size_x, uint32_t size_y); ///< State must not be at end.
uint8_t openttd_rust_spiral_end(OpenTTDRustSpiralState state); ///< 0/1; radius equality and non-invalid direction.
uint8_t openttd_rust_spiral_equal(OpenTTDRustSpiralState left, OpenTTDRustSpiralState right); ///< 0/1; compare x,y only.

#ifdef __cplusplus
}
#endif
#endif /* RUST_SPIRAL_FFI_H */
