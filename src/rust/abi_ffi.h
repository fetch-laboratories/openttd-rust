/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file abi_ffi.h Bounded native layout agreement checks, not an algorithm interface. */
#ifndef RUST_ABI_FFI_H
#define RUST_ABI_FFI_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/**
 * Type IDs: 0 integer, 1 UTF8 encoded, 2 UTF8 decoded, 3 LE bytes, 4 formatted,
 * 5 alternating state, 6 alternating step, 7 consumer bound, 8 consumer byte,
 * 9 consumer match, 10 consumer separator, 11 encoded parameter, 12 encoded view,
 * 13 encoded diagnostic, 14 spiral state, 15 byte trim,
 * 16 history descriptor, 17 history step, 18 station cargo,
 * 19 cipher leaves, 20 Poly1305 layout, 21 AEAD layout, 22 BLAKE2b layout,
 * 23 Packet state, 24 Packet framing offsets, 25 X25519 leaves,
 * 26 string-validation step, 27 in-place write result.
 * 39 effect view, 40 retired effect cursor, 41 effect map leaves, 42 shared game services.
 * 80 road path entry, 81 speed limits, 82 road services, 83 retired action.
 * 46 water patch, 47 water diagnostic snapshot, 48 water shared-service leaves.
 * 49 cargo specification, 50 cargo payment saved fields, 51 cargo services.
 * 52 ship YAPF input, 53 leaves, 54 follower, 55 tile, 56 choice result.
 * 60-61 retired town action/leaves; 430 town command result, 431 town services; 432-438 canonical town records; 439-442 narrow town spec reads.
 * 110 station cargo metadata, 111 station scalars, 112 station services,
 * 113 station edge observations, 114 station links, 115 station loading.
 * 130-138 industry storage, observation, world services and production result.
 * 210 company history entry, 211 finances, 212 economy, 213 finance services,
 * 214 company services, 400-417 company records and stack continuations.
 * 215-244 narrow road reads, 245 position, 246 track choice, 247 depot result.
 * 260 order fields, 261 consist, 262 vehicle orders, 263 order list,
 * 264 order backup, 265 typed order services, 266 closest-depot result.
 * 320 ship position, 321 typed ship services, 322 reverse result, 323 depot,
 * 324 ship track-choice result, 325 ship depot result.
 * 386 tree clear observation; 387-389 reserved for tree additions.
 * 390 fleet group fields, 391 statistics fields, 392 renewal fields,
 * 393 fleet group services, 394 transaction services, 395 native costs,
 * 396 pending services; 397-399 reserved for fleet additions.
 * Existing train allocation spans 248-385; other existing low IDs remain.
 * 630-633 town founding references, name/render services, metadata and direct services.
 * Reserved component blocks: company 400-429, town 430-459,
 * shared vehicle/road services 460-499. Allocate within the component's block.
 * 820 signal services, 821 canonical rail-map services; 822 diagnostic snapshot, 823 tile/direction; 824-829 phase observations, 830 direct context; 831 fixed-set probe trace; 832-859 reserved.
 * 980 rail command services, 981 rail command strings, 982 foundation slope/height,
 * 983 train parking fields, 984 normal road fields, 985 crossing road fields; 986-1019 reserved.
 * Item 0 size, 1 alignment, then every field offset in declaration order.
 * Unknown type/item returns SIZE_MAX. No memory borrow, ownership or callback.
 * Native extern-C convention (cdecl on i686 MSVC); Rust panic never unwinds.
 */
size_t openttd_rust_abi_layout(uint16_t type_id, uint8_t item);
#ifdef __cplusplus
}
#endif
#endif /* RUST_ABI_FFI_H */
