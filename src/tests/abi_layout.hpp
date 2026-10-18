/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file abi_layout.hpp Shared check of one Rust ABI layout list, used by the tests and the ABI probe. */
#ifndef ABI_LAYOUT_HPP
#define ABI_LAYOUT_HPP
#include "../rust/abi_ffi.h"
#include <initializer_list>

/**
 * Compare every Rust layout item of a type, then its SIZE_MAX terminator.
 * Lists and field aliases are generated from the Rust declarations.
 * @param type Native C++ type name.
 * @param values Expected size, alignment and field offsets, in order.
 * @return The first differing item, or -1 when all items and the terminator match.
 */
inline int AbiLayoutMismatch(const char *type, std::initializer_list<size_t> values)
{
	size_t item = 0;
	for (size_t expected : values) {
		if (openttd_rust_abi_layout_named(type, item) != expected) return item;
		item++;
	}
	return openttd_rust_abi_layout_named(type, item) == SIZE_MAX ? -1 : item;
}
#endif /* ABI_LAYOUT_HPP */
