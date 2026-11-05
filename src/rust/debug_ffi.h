/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file debug_ffi.h Native debug levels and print leaves for Rust's Debug and GrfMsg. */
#ifndef RUST_DEBUG_FFI_H
#define RUST_DEBUG_FFI_H
#include <cstddef>
#include <cstdint>
struct OpenTTDDebugServices {
	const int *console;
	const int *desync;
	const int *driver;
	const int *fontcache;
	const int *gamelog;
	const int *grf;
	void (*grf_msg)(int severity, const char *message, size_t size) noexcept;
	const int *map;
	const int *misc;
	const int *net;
	const int *oldloader;
	void (*print)(const char *category, size_t category_size, int level, const char *message, size_t size) noexcept;
	const int *script;
	const int *sl;
	const int *sprite;
	const int *yapf;
};
extern "C" void openttd_rust_debug_bind(const OpenTTDDebugServices *services);
#endif /* RUST_DEBUG_FFI_H */
