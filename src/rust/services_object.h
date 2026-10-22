/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_object.h Native stack frames for synchronous object entries. */
#ifndef RUST_SERVICES_OBJECT_H
#define RUST_SERVICES_OBJECT_H
#include "object_ffi.h"
#include "../station_func.h"
#include "../newgrf_object.h"
#include "generation_services.hpp"
#include <optional>
struct OpenTTDObjectLoopFrame { std::optional<StationFinder> stations; };
struct OpenTTDObjectGenerationFrame { std::exception_ptr cancellation; };
inline uint16_t RustObjectSpecFlags(const ObjectSpec *spec) noexcept { return spec->flags.base(); }
inline uint8_t RustObjectSpecCallbacks(const ObjectSpec *spec) noexcept { return spec->callback_mask.base(); }
inline bool RustObjectEverAvailable(const ObjectSpec *spec) noexcept { return spec->WasEverAvailable(); }
#endif /* RUST_SERVICES_OBJECT_H */
