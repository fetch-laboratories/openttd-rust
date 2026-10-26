/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file train_state_ffi.h Canonical train-private state owner. */
#ifndef RUST_TRAIN_STATE_FFI_H
#define RUST_TRAIN_STATE_FFI_H
#include <cstdint>
#include "call_ffi.h"
struct OpenTTDTrainState;
extern "C" {
/* One zero-created allocation per ordinary/indexed-load shell. Destroy exactly
 * once after PreDestructor, including pool cleanup. Game-thread scalar access;
 * no returned references, no owner borrow across callbacks, panic/OOM abort.
 * Typed scalar entries retain original widths, including signed curve modifiers. */
OpenTTDTrainState *OPENTTD_RUST_CALL openttd_rust_train_state_new();
void OPENTTD_RUST_CALL openttd_rust_train_state_destroy(OpenTTDTrainState *);
uint16_t OPENTTD_RUST_CALL openttd_rust_train_state_get_flags(const OpenTTDTrainState *);
void OPENTTD_RUST_CALL openttd_rust_train_state_set_flags(OpenTTDTrainState *, uint16_t);
uint16_t OPENTTD_RUST_CALL openttd_rust_train_state_get_crash_anim_pos(const OpenTTDTrainState *);
void OPENTTD_RUST_CALL openttd_rust_train_state_set_crash_anim_pos(OpenTTDTrainState *, uint16_t);
uint16_t OPENTTD_RUST_CALL openttd_rust_train_state_get_wait_counter(const OpenTTDTrainState *);
void OPENTTD_RUST_CALL openttd_rust_train_state_set_wait_counter(OpenTTDTrainState *, uint16_t);
uint64_t OPENTTD_RUST_CALL openttd_rust_train_state_get_compatible_railtypes(const OpenTTDTrainState *);
void OPENTTD_RUST_CALL openttd_rust_train_state_set_compatible_railtypes(OpenTTDTrainState *, uint64_t);
uint64_t OPENTTD_RUST_CALL openttd_rust_train_state_get_railtypes(const OpenTTDTrainState *);
void OPENTTD_RUST_CALL openttd_rust_train_state_set_railtypes(OpenTTDTrainState *, uint64_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_train_state_get_track(const OpenTTDTrainState *);
void OPENTTD_RUST_CALL openttd_rust_train_state_set_track(OpenTTDTrainState *, uint8_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_train_state_get_force_proceed(const OpenTTDTrainState *);
void OPENTTD_RUST_CALL openttd_rust_train_state_set_force_proceed(OpenTTDTrainState *, uint8_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_train_state_get_cached_tilt(const OpenTTDTrainState *);
void OPENTTD_RUST_CALL openttd_rust_train_state_set_cached_tilt(OpenTTDTrainState *, uint8_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_train_state_get_user_def_data(const OpenTTDTrainState *);
int16_t OPENTTD_RUST_CALL openttd_rust_train_state_get_cached_curve_speed_mod(const OpenTTDTrainState *);
void OPENTTD_RUST_CALL openttd_rust_train_state_set_cached_curve_speed_mod(OpenTTDTrainState *, int16_t);
uint16_t OPENTTD_RUST_CALL openttd_rust_train_state_get_cached_max_curve_speed(const OpenTTDTrainState *);
void OPENTTD_RUST_CALL openttd_rust_train_state_set_cached_max_curve_speed(OpenTTDTrainState *, uint16_t);

}
#endif /* RUST_TRAIN_STATE_FFI_H */
