/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file train_state_ffi.h Canonical train-private state owner. */
#ifndef RUST_TRAIN_STATE_FFI_H
#define RUST_TRAIN_STATE_FFI_H
#include "abi-record-train_state.h"
#include "call_ffi.h"
extern "C" {
/* One zero-created allocation per ordinary/indexed-load shell. Destroy exactly
 * once after PreDestructor, including pool cleanup. Game-thread scalar access;
 * no owner borrow across callbacks, panic/OOM abort. */
OpenTTDTrainState *OPENTTD_RUST_CALL openttd_rust_train_state_new();
void OPENTTD_RUST_CALL openttd_rust_train_state_destroy(OpenTTDTrainState *);

}
#endif /* RUST_TRAIN_STATE_FFI_H */
