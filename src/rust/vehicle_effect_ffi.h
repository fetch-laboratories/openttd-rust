/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file vehicle_effect_ffi.h The visual effect exports that the fleet harness calls. */
#ifndef RUST_VEHICLE_EFFECT_FFI_H
#define RUST_VEHICLE_EFFECT_FFI_H
#include "vehicle_ffi.h"
#include "services_ffi.h"
extern "C" {
void openttd_rust_visual_update(OpenTTDVehicle *, bool, const OpenTTDVehicleServices *, const OpenTTDSharedServices *);
void openttd_rust_visual_show(OpenTTDVehicle *, const OpenTTDVehicleServices *, const OpenTTDSharedServices *, int32_t (*)(OpenTTDVehicle *) noexcept);
}
#endif
