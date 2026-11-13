/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Tree tables of `table/tree_land.h` that the simulation reads; the sprite layouts stay with
//! the native drawing code.

/// `_tree_base_by_landscape`: the first tree type of each landscape.
pub(crate) const TREE_BASE_BY_LANDSCAPE: [u8; 4] = [0, 12, 20, 32];
/// `_tree_count_by_landscape`: the number of tree types of each landscape.
pub(crate) const TREE_COUNT_BY_LANDSCAPE: [u8; 4] = [12, 8, 12, 9];
