/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file abi_ffi.h Bounded native layout agreement checks, not an algorithm interface. */
#ifndef RUST_ABI_FFI_H
#define RUST_ABI_FFI_H
#include <cstddef>
#include <cstdint>
extern "C" size_t openttd_rust_abi_layout_named(const char *name, size_t item);
#endif /* RUST_ABI_FFI_H */
