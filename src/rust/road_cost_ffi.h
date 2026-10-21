/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
#ifndef RUST_ROAD_COST_FFI_H
#define RUST_ROAD_COST_FFI_H
#include "services_ffi.h"
extern "C" {
int64_t openttd_rust_road_build_cost(const OpenTTDSharedServices *, uint8_t);
int64_t openttd_rust_road_clear_cost(const OpenTTDSharedServices *, uint8_t);
int64_t openttd_rust_road_convert_cost(const OpenTTDSharedServices *, uint8_t, uint8_t);
}
#endif /* RUST_ROAD_COST_FFI_H */
