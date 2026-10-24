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
typedef struct OpenTTDDisasterRun OpenTTDDisasterRun;
struct OpenTTDOccupancyCommands;
struct OpenTTDVehicle;
typedef struct {
	const struct OpenTTDOccupancyCommands *commands;
	struct OpenTTDVehicle *(OPENTTD_RUST_CALL *vehicle)(void *, uint32_t) noexcept;
} OpenTTDDisasterOccupancy;
/* Rust state follows its shell's lifetime; scalar serialization addresses are stable until
 * destruction. Keep the shared service and occupancy tables live and immutable through run
 * destruction, including suspended actions. Only delete (2), RoadVehicle::Crash (14) and
 * CMD_LANDSCAPE_CLEAR (17) return actions, for lifetime, NewGRF and nested release hooks; C++
 * completes them and owns continuation cleanup on exceptions.
 * Read/write leaves cannot reenter; occupancy pointees stay live and immutable through suspension
 * and nested World clones. Shared flight helpers access flags only within their call. */
OpenTTDDisasterState *OPENTTD_RUST_CALL openttd_rust_disaster_state_create(uint32_t);
void OPENTTD_RUST_CALL openttd_rust_disaster_state_destroy(OpenTTDDisasterState *);
uint16_t *OPENTTD_RUST_CALL openttd_rust_disaster_delay(void);
OpenTTDDisasterRun *OPENTTD_RUST_CALL openttd_rust_disaster_create(uint32_t, uint32_t, int64_t, int64_t, int64_t, int64_t, void *, void (OPENTTD_RUST_CALL *)(void *, uint32_t, uint32_t, int64_t, int64_t, int64_t *), void (OPENTTD_RUST_CALL *)(void *, uint32_t, uint32_t, int64_t), const OpenTTDSharedServices *, const OpenTTDDisasterOccupancy *, int64_t (OPENTTD_RUST_CALL *)(void *, const OpenTTDDisasterAction *));
OpenTTDDisasterAction OPENTTD_RUST_CALL openttd_rust_disaster_advance(OpenTTDDisasterRun *, int64_t);
void OPENTTD_RUST_CALL openttd_rust_disaster_destroy(OpenTTDDisasterRun *);
#ifdef __cplusplus
}
#endif
#endif /* OPENTTD_RUST_DISASTER_FFI_H */
