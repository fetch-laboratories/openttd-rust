/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared inline helpers declared in `core/bitmath_func.hpp`.

/// `GB` for a 32-bit input, with source preconditions `count < 32` and
/// `start + count <= 32`.
pub const fn gb(value: u32, start: u8, count: u8) -> u32 {
    (value >> start) & ((1_u32 << count) - 1)
}

/// `KillFirstBit` for the byte-sized track bitsets.
pub const fn kill_first_bit(value: u8) -> u8 {
    value & value.wrapping_sub(1)
}
