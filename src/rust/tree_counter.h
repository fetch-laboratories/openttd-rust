/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file tree_counter.h Serialization address of the Rust-owned tree tick counter. */
#ifndef TREE_COUNTER_H
#define TREE_COUNTER_H
#include "trees_ffi.h"
#define _trees_tick_ctr (*openttd_rust_tree_counter())
#endif /* TREE_COUNTER_H */
