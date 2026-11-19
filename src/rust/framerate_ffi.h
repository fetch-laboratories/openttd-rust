/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file framerate_ffi.h Native performance measurements that Rust makes, bound once for the process. */
#ifndef RUST_FRAMERATE_FFI_H
#define RUST_FRAMERATE_FFI_H
#include <cstddef>
#include <cstdint>
#include "service_defaults.hpp"

/* A native measurer or accumulator is placement-constructed here, on its Rust caller's stack; Rust never interprets the bytes. */
struct alignas(8) OpenTTDPerformanceMeasurer { std::byte storage[16]; };
struct alignas(8) OpenTTDPerformanceAccumulator { std::byte storage[16]; };
struct OpenTTDFramerateServices {
	void (*accumulator_begin)(OpenTTDPerformanceAccumulator *storage, uint8_t elem) noexcept OPENTTD_SERVICE_DEFAULT(accumulator_begin);
	void (*accumulator_end)(OpenTTDPerformanceAccumulator *storage) noexcept OPENTTD_SERVICE_DEFAULT(accumulator_end);
	void (*accumulator_reset)(uint8_t elem) noexcept OPENTTD_SERVICE_DEFAULT(accumulator_reset);
	void (*measurer_begin)(OpenTTDPerformanceMeasurer *storage, uint8_t elem) noexcept OPENTTD_SERVICE_DEFAULT(measurer_begin);
	void (*measurer_end)(OpenTTDPerformanceMeasurer *storage) noexcept OPENTTD_SERVICE_DEFAULT(measurer_end);
	void (*measurer_paused)(uint8_t elem) noexcept OPENTTD_SERVICE_DEFAULT(measurer_paused);
	void (*measurer_set_inactive)(uint8_t elem) noexcept OPENTTD_SERVICE_DEFAULT(measurer_set_inactive);
};
const OpenTTDFramerateServices &GetRustFramerateServices() noexcept;
extern "C" const OpenTTDFramerateServices *openttd_rust_framerate_bind(const OpenTTDFramerateServices *services);
#endif /* RUST_FRAMERATE_FFI_H */
