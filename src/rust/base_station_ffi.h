/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file base_station_ffi.h Native readers of BaseStation members that Rust shares, bound once for the process. */
#ifndef RUST_BASE_STATION_FFI_H
#define RUST_BASE_STATION_FFI_H
#include <cstdint>
#include "service_defaults.hpp"

struct BaseStation;

struct OpenTTDBaseStationServices {
	uint32_t (*xy)(const BaseStation *st) noexcept OPENTTD_SERVICE_DEFAULT(xy);
};
extern "C" void openttd_rust_base_station_bind(const OpenTTDBaseStationServices *services);
#endif /* RUST_BASE_STATION_FFI_H */
