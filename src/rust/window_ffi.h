/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file window_ffi.h Native window entries that Rust calls, bound once for the process. */
#ifndef RUST_WINDOW_FFI_H
#define RUST_WINDOW_FFI_H
#include <cstdint>
#include "service_defaults.hpp"

struct OpenTTDWindowServices {
	void (*close_window_by_id)(uint16_t cls, int32_t number, bool force, int32_t data) noexcept OPENTTD_SERVICE_DEFAULT(close_window_by_id);
	void (*delete_newgrf_inspect_window)(uint8_t feature, uint32_t index) noexcept OPENTTD_SERVICE_DEFAULT(delete_newgrf_inspect_window);
	void (*invalidate_newgrf_inspect_window)(uint8_t feature, uint32_t index) noexcept OPENTTD_SERVICE_DEFAULT(invalidate_newgrf_inspect_window);
	void (*invalidate_window_classes_data)(uint16_t cls, int32_t data, bool gui_scope) noexcept OPENTTD_SERVICE_DEFAULT(invalidate_window_classes_data);
	void (*invalidate_window_data)(uint16_t cls, int32_t number, int32_t data, bool gui_scope) noexcept OPENTTD_SERVICE_DEFAULT(invalidate_window_data);
	void (*set_window_classes_dirty)(uint16_t cls) noexcept OPENTTD_SERVICE_DEFAULT(set_window_classes_dirty);
	void (*set_window_dirty)(uint16_t cls, int32_t number) noexcept OPENTTD_SERVICE_DEFAULT(set_window_dirty);
	void (*set_window_widget_dirty)(uint16_t cls, int32_t number, int32_t widget_index) noexcept OPENTTD_SERVICE_DEFAULT(set_window_widget_dirty);
};
extern "C" void openttd_rust_window_bind(const OpenTTDWindowServices *services);
#endif /* RUST_WINDOW_FFI_H */
