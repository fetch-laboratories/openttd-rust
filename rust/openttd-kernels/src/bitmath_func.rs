/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `core/bitmath_func.hpp` bit operations used by game owners.
/// `HasBit` for byte and uint32 bitsets, with C++ integer promotion.
/// Out-of-width shifts in C++ have no defined behavior; source-valid bit indices
/// retain the promoted expression, without adding a Rust shift-overflow panic.
pub fn has_bit(value: impl Into<u32>, bit: u8) -> bool {
    value.into() & 1_u32.wrapping_shl(u32::from(bit)) != 0
}

/// `SetBit` for uint32 fields; return the updated value as in the native helper.
pub fn set_bit(value: &mut u32, bit: u8) -> u32 {
    *value |= 1_u32.wrapping_shl(u32::from(bit));
    *value
}

/// `SB` for uint32 fields: clear the destination window, then OR the entire
/// shifted operand. The original deliberately does not mask the operand.
pub fn sb(value: &mut u32, start: u8, length: u8, data: u32) {
    *value &= !((1_u32.wrapping_shl(u32::from(length)) - 1).wrapping_shl(u32::from(start)));
    *value |= data.wrapping_shl(u32::from(start));
}

/// `GB` for a 32-bit input, with source preconditions `count < 32` and
/// `start + count <= 32`.
pub const fn gb(value: u32, start: u8, count: u8) -> u32 {
    (value >> start) & ((1_u32 << count) - 1)
}

/// `KillFirstBit` for the byte-sized track bitsets.
pub const fn kill_first_bit(value: u8) -> u8 {
    value & value.wrapping_sub(1)
}
