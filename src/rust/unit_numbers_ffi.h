/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file unit_numbers_ffi.h Company-owned vehicle and group number bitmap. */
#ifndef RUST_UNIT_NUMBERS_FFI_H
#define RUST_UNIT_NUMBERS_FFI_H
#include "call_ffi.h"
#include "rust_vector.h"
#include "abi-record-vehicle_maintenance.h"
extern "C" {
void OPENTTD_RUST_CALL openttd_rust_unit_numbers_copy(OpenTTDUnitNumberState *, const OpenTTDUnitNumberState *);
void OPENTTD_RUST_CALL openttd_rust_unit_numbers_drop(OpenTTDUnitNumberState *);
uint16_t OPENTTD_RUST_CALL openttd_rust_unit_numbers_next(const OpenTTDUnitNumberState *);
uint16_t OPENTTD_RUST_CALL openttd_rust_unit_numbers_use(OpenTTDUnitNumberState *, uint16_t);
void OPENTTD_RUST_CALL openttd_rust_unit_numbers_release(OpenTTDUnitNumberState *, uint16_t, void (OPENTTD_RUST_CALL *)(bool) noexcept);
}
#endif /* RUST_UNIT_NUMBERS_FFI_H */
