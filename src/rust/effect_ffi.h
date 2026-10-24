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
#if defined(_MSC_VER)
#define OPENTTD_EFFECT_CALL __cdecl
#elif defined(__i386__)
#define OPENTTD_EFFECT_CALL __attribute__((cdecl))
#else
#define OPENTTD_EFFECT_CALL
#endif
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
	void (OPENTTD_EFFECT_CALL *observe)(void *, OpenTTDEffectView *) noexcept OPENTTD_SERVICE_DEFAULT(observe);
	void (OPENTTD_EFFECT_CALL *write)(void *, uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(write);
	void (OPENTTD_EFFECT_CALL *viewport)(void *) noexcept OPENTTD_SERVICE_DEFAULT(viewport);
	void (OPENTTD_EFFECT_CALL *sound)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(sound);

} OpenTTDEffectLeaves;
/* One shell owns one zero-created state. get/set: field 0 uint16 animation, field 1 uint8 substate.
 * Write fields: 0 x, 1 y, 2 z, 3 sprite sequence.Set, 4 scalar sprite, 5 progress, 6 spritenum,
 * written immediately in original order. Sound: 0 fail, 1 success. run returns 1 alive, 0 expiry;
 * C++ deletes only after return. Save/load get/set use the C++ staging. */
OpenTTDEffectState *OPENTTD_EFFECT_CALL openttd_rust_effect_new(void);
void OPENTTD_EFFECT_CALL openttd_rust_effect_destroy(OpenTTDEffectState *);
void OPENTTD_EFFECT_CALL openttd_rust_effect_set(OpenTTDEffectState *, uint8_t field, uint16_t value);
uint16_t OPENTTD_EFFECT_CALL openttd_rust_effect_get(const OpenTTDEffectState *, uint8_t field);
uint8_t OPENTTD_EFFECT_CALL openttd_rust_effect_run(OpenTTDEffectState *, void *context, const OpenTTDEffectLeaves *, const OpenTTDSharedServices *, uint8_t initialize);
#ifdef __cplusplus
}
#endif
#endif /* OPENTTD_RUST_EFFECT_FFI_H */
