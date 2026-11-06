/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file engine_lifecycle.hpp Canonical engine lifecycle storage and shell lifetime. */
#ifndef RUST_ENGINE_LIFECYCLE_HPP
#define RUST_ENGINE_LIFECYCLE_HPP
#include "byte_span_ffi.h"
#include "../engine_type.h"
#include "../vehicle_type.h"
#include <utility>
#include "rust_vector.h"
#include "abi-record-engine.h"
extern "C" {
OpenTTDEngineLifecycle *openttd_rust_engine_new();
void openttd_rust_engine_construct(OpenTTDEngineLifecycle *, const OpenTTDEngineDefaults *, VehicleType, uint16_t);
void openttd_rust_engine_destroy(OpenTTDEngineLifecycle *);
void openttd_rust_engine_set_name(OpenTTDEngineLifecycle *, OpenTTDByteSpan);
}
/** Game-state mutex serializes access. Moves transfer ownership; no view survives mutation/drop. */
struct EngineLifecycleOwner {
	OpenTTDEngineLifecycle *state;
	EngineLifecycleOwner() : state(openttd_rust_engine_new()) {}
	EngineLifecycleOwner(EngineLifecycleOwner &&other) noexcept : state(std::exchange(other.state, nullptr)) {}
	EngineLifecycleOwner(const EngineLifecycleOwner &) = delete;
	EngineLifecycleOwner &operator=(const EngineLifecycleOwner &) = delete;
	~EngineLifecycleOwner() { if (this->state != nullptr) openttd_rust_engine_destroy(this->state); }
};
#endif /* RUST_ENGINE_LIFECYCLE_HPP */
