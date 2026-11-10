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
typedef struct OpenTTDEffectLeaves {
	const OpenTTDVehicleServices *vehicles;
	void (OPENTTD_RUST_CALL *sprite_sequence)(void *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(sprite_sequence);
	void (OPENTTD_RUST_CALL *sprite)(void *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(sprite);
	void (OPENTTD_RUST_CALL *spritenum)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(spritenum);
	void (OPENTTD_RUST_CALL *viewport)(void *) noexcept OPENTTD_SERVICE_DEFAULT(viewport);
	void (OPENTTD_RUST_CALL *sound)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(sound);

} OpenTTDEffectLeaves;
typedef struct EffectVehicle OpenTTDEffectVehicle;
struct OpenTTDEffectAllocation { void *shell; OpenTTDVehicle *record; OpenTTDEffectState *owner; };
struct OpenTTDEffectCreationServices {
	const OpenTTDEffectLeaves *effect;
	const OpenTTDSharedServices *shared;
	OpenTTDEffectAllocation (*allocate)() noexcept OPENTTD_SERVICE_DEFAULT(allocate);
	OpenTTDEffectState *(*owner)(void *) noexcept OPENTTD_SERVICE_DEFAULT(owner);
};
const OpenTTDEffectCreationServices *GetRustEffectCreationServices() noexcept;
const OpenTTDEffectCreationServices *openttd_rust_effect_bind(const OpenTTDEffectCreationServices *);
OpenTTDEffectVehicle *openttd_rust_effect_create(int32_t, int32_t, int32_t, uint8_t);
OpenTTDEffectVehicle *openttd_rust_effect_create_above(int32_t, int32_t, int32_t, uint8_t);
OpenTTDEffectVehicle *openttd_rust_effect_create_rel(const OpenTTDVehicle *, int32_t, int32_t, int32_t, uint8_t);
/* One shell owns one zero-created state with uint16 animation and uint8 substate.
 * Typed field writes run immediately in original order. Sound: 0 fail, 1 success. run returns 1 alive, 0 expiry;
 * C++ deletes only after return. Save/load get/set use the C++ staging.
 * Tables are live, aligned, immutable and disjoint from the exclusive owner until return; context
 * is its live shell. Leaves cannot reenter, retain pointers or destroy the shell, and field accesses
 * end before each leaf call. Original subtype, table-index and coordinate preconditions apply. */
OpenTTDEffectState *OPENTTD_RUST_CALL openttd_rust_effect_new(void);
void OPENTTD_RUST_CALL openttd_rust_effect_destroy(OpenTTDEffectState *);
void OPENTTD_RUST_CALL openttd_rust_effect_set_animation(OpenTTDEffectState *, uint16_t value);
uint16_t OPENTTD_RUST_CALL openttd_rust_effect_get_animation(const OpenTTDEffectState *);
void OPENTTD_RUST_CALL openttd_rust_effect_set_substate(OpenTTDEffectState *, uint8_t value);
uint8_t OPENTTD_RUST_CALL openttd_rust_effect_get_substate(const OpenTTDEffectState *);
uint8_t OPENTTD_RUST_CALL openttd_rust_effect_run(OpenTTDEffectState *, OpenTTDVehicle *record, const OpenTTDEffectLeaves *, const OpenTTDSharedServices *);
#ifdef __cplusplus
}
#endif
#endif /* OPENTTD_RUST_EFFECT_FFI_H */
