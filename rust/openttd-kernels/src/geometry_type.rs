/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared inline geometry helpers declared in `core/geometry_type.hpp`.

/// `Rect::Contains`, with the original unsigned comparison of coordinate deltas.
#[allow(clippy::cast_sign_loss)]
pub fn rect_contains(left: i32, top: i32, right: i32, bottom: i32, x: i32, y: i32) -> bool {
    x.wrapping_sub(left) as u32 <= right.wrapping_sub(left) as u32
        && y.wrapping_sub(top) as u32 <= bottom.wrapping_sub(top) as u32
}
