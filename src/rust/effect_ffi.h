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
extern "C" {
#endif
typedef struct OpenTTDEffectState OpenTTDEffectState;
typedef struct {
	int32_t x, y, z;
	uint32_t sprite;
	uint8_t progress, spritenum, subtype, ambient;
} OpenTTDEffectView;
typedef struct OpenTTDEffectLeaves {
	void (OPENTTD_RUST_CALL *observe)(void *, OpenTTDEffectView *) noexcept OPENTTD_SERVICE_DEFAULT(observe);
	void (OPENTTD_RUST_CALL *write_x)(void *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_x);
	void (OPENTTD_RUST_CALL *write_y)(void *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_y);
	void (OPENTTD_RUST_CALL *write_z)(void *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_z);
	void (OPENTTD_RUST_CALL *sprite_sequence)(void *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(sprite_sequence);
	void (OPENTTD_RUST_CALL *sprite)(void *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(sprite);
	void (OPENTTD_RUST_CALL *progress)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(progress);
	void (OPENTTD_RUST_CALL *spritenum)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(spritenum);
	void (OPENTTD_RUST_CALL *viewport)(void *) noexcept OPENTTD_SERVICE_DEFAULT(viewport);
	void (OPENTTD_RUST_CALL *sound)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(sound);

} OpenTTDEffectLeaves;
struct OpenTTDVehicle;
typedef struct EffectVehicle OpenTTDEffectVehicle;
struct OpenTTDEffectAllocation { void *shell; OpenTTDEffectState *owner; };
struct OpenTTDEffectPosition { int32_t x, y, z; };
struct OpenTTDEffectCreationServices {
	const OpenTTDEffectLeaves *effect;
	const OpenTTDSharedServices *shared;
	bool (*can_allocate)(size_t) noexcept OPENTTD_SERVICE_DEFAULT(can_allocate);
	OpenTTDEffectAllocation (*allocate)() noexcept OPENTTD_SERVICE_DEFAULT(allocate);
	OpenTTDEffectState *(*owner)(void *) noexcept OPENTTD_SERVICE_DEFAULT(owner);
	void (*place)(void *, int32_t, int32_t, int32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(place);
	void (*delta)(void *) noexcept OPENTTD_SERVICE_DEFAULT(delta);
	void (*unclickable)(void *) noexcept OPENTTD_SERVICE_DEFAULT(unclickable);
	OpenTTDEffectPosition (*position)(const OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(position);
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
 * is its live shell. Leaves cannot reenter, retain pointers or destroy the shell, and observe output
 * is call-scoped. Original subtype, table-index and coordinate preconditions apply. */
OpenTTDEffectState *OPENTTD_RUST_CALL openttd_rust_effect_new(void);
void OPENTTD_RUST_CALL openttd_rust_effect_destroy(OpenTTDEffectState *);
void OPENTTD_RUST_CALL openttd_rust_effect_set_animation(OpenTTDEffectState *, uint16_t value);
uint16_t OPENTTD_RUST_CALL openttd_rust_effect_get_animation(const OpenTTDEffectState *);
void OPENTTD_RUST_CALL openttd_rust_effect_set_substate(OpenTTDEffectState *, uint8_t value);
uint8_t OPENTTD_RUST_CALL openttd_rust_effect_get_substate(const OpenTTDEffectState *);
uint8_t OPENTTD_RUST_CALL openttd_rust_effect_run(OpenTTDEffectState *, void *context, const OpenTTDEffectLeaves *, const OpenTTDSharedServices *);
void OPENTTD_RUST_CALL openttd_rust_effect_initialize(OpenTTDEffectState *, void *, uint8_t, const OpenTTDEffectLeaves *, const OpenTTDSharedServices *);
#ifdef __cplusplus
}
#endif
#endif /* OPENTTD_RUST_EFFECT_FFI_H */
