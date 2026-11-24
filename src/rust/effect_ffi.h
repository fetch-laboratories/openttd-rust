/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file effect_ffi.h Rust effect owners and direct synchronous services. */
#ifndef OPENTTD_RUST_EFFECT_FFI_H
#define OPENTTD_RUST_EFFECT_FFI_H
#include "service_defaults.hpp"
#include <stdint.h>
#include "services_ffi.h"
#include "call_ffi.h"
#ifdef __cplusplus
struct OpenTTDVehicleServices;
struct OpenTTDVehicle;
extern "C" {
#endif
typedef struct OpenTTDEffectState OpenTTDEffectState;
typedef struct EffectVehicle OpenTTDEffectVehicle;
struct OpenTTDEffectAllocation { OpenTTDVehicle *record; OpenTTDEffectState *owner; };
struct OpenTTDEffectCreationServices {
	OpenTTDEffectAllocation (*allocate)() noexcept OPENTTD_SERVICE_DEFAULT(allocate);
	const OpenTTDSharedServices *shared;
	const OpenTTDVehicleServices *vehicles;
};
const OpenTTDEffectCreationServices *GetRustEffectCreationServices() noexcept;
const OpenTTDEffectCreationServices *openttd_rust_effect_bind(const OpenTTDEffectCreationServices *);
OpenTTDEffectVehicle *openttd_rust_effect_create(int32_t, int32_t, int32_t, uint8_t);
OpenTTDEffectVehicle *openttd_rust_effect_create_above(int32_t, int32_t, int32_t, uint8_t);
OpenTTDEffectVehicle *openttd_rust_effect_create_rel(const OpenTTDVehicle *, int32_t, int32_t, int32_t, uint8_t);
/* One shell owns one zero-created state with the uint16 animation_state and uint8 animation_substate.
 * run is EffectVehicle::Tick over the given tables: 1 alive, 0 expiry, after which C++ deletes the shell.
 * Save/load get/set use the C++ staging. */
OpenTTDEffectState *OPENTTD_RUST_CALL openttd_rust_effect_new(void);
void OPENTTD_RUST_CALL openttd_rust_effect_destroy(OpenTTDEffectState *);
void OPENTTD_RUST_CALL openttd_rust_effect_set_animation(OpenTTDEffectState *, uint16_t value);
uint16_t OPENTTD_RUST_CALL openttd_rust_effect_get_animation(const OpenTTDEffectState *);
void OPENTTD_RUST_CALL openttd_rust_effect_set_substate(OpenTTDEffectState *, uint8_t value);
uint8_t OPENTTD_RUST_CALL openttd_rust_effect_get_substate(const OpenTTDEffectState *);
uint8_t OPENTTD_RUST_CALL openttd_rust_effect_run(OpenTTDEffectState *, OpenTTDVehicle *record, const OpenTTDVehicleServices *, const OpenTTDSharedServices *);
#ifdef __cplusplus
}
#endif
#endif /* OPENTTD_RUST_EFFECT_FFI_H */
