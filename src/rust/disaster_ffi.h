/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
/** @file disaster_ffi.h Disaster owners and return-to-C++ shared-service protocol. */
#ifndef OPENTTD_RUST_DISASTER_FFI_H
#define OPENTTD_RUST_DISASTER_FFI_H
#include <stdint.h>
#ifdef __cplusplus
#include "services_ffi.h"
#endif
#include "call_ffi.h"
#ifdef __cplusplus
extern "C" {
#endif
typedef struct { uint32_t image_override, target; uint16_t state; uint8_t flags; } OpenTTDDisasterState;
typedef struct { uint32_t kind, id, other; int64_t a, b, c, d; } OpenTTDDisasterAction;
struct OpenTTDOccupancyCommands;
struct OpenTTDVehicle;
typedef struct {
	const struct OpenTTDOccupancyCommands *commands;
	struct OpenTTDVehicle *(OPENTTD_RUST_CALL *vehicle)(void *, uint32_t) noexcept;
} OpenTTDDisasterOccupancy;
/* Rust state follows its shell's lifetime. Services are synchronous and noexcept;
 * private scalar accesses end before callbacks, including deletion and reentry. */
OpenTTDDisasterState *OPENTTD_RUST_CALL openttd_rust_disaster_state_create(uint32_t);
void OPENTTD_RUST_CALL openttd_rust_disaster_state_destroy(OpenTTDDisasterState *);
uint16_t *OPENTTD_RUST_CALL openttd_rust_disaster_delay(void);
int64_t OPENTTD_RUST_CALL openttd_rust_disaster_run(uint32_t, uint32_t, int64_t, int64_t, int64_t, int64_t, void *, void (OPENTTD_RUST_CALL *)(void *, uint32_t, uint32_t, int64_t, int64_t, int64_t *), void (OPENTTD_RUST_CALL *)(void *, uint32_t, uint32_t, int64_t), const OpenTTDSharedServices *, const OpenTTDDisasterOccupancy *, int64_t (OPENTTD_RUST_CALL *)(void *, const OpenTTDDisasterAction *) noexcept);
#ifdef __cplusplus
}
#endif
#endif /* OPENTTD_RUST_DISASTER_FFI_H */
