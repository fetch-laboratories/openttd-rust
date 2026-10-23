/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `core/bitmath_func.hpp` bit operations used by game owners.
/// `HasBit` for the source's byte-sized bitsets, with C++ integer promotion.
/// Out-of-width shifts in C++ have no defined behavior; source-valid bit indices
/// retain the promoted expression, without adding a Rust shift-overflow panic.
pub fn has_bit(value: u8, bit: u8) -> bool {
    u32::from(value) & 1_u32.wrapping_shl(u32::from(bit)) != 0
}

/// Original `SB` for a 32-bit word: clear the window, OR the unmasked data.
/// The original preconditions are `n < 32` and `s + n <= 32`.
pub(crate) fn sb(value: &mut u32, s: u8, n: u8, data: u32) {
    *value &= !(((1_u32 << n) - 1) << s);
    *value |= data << s;
}
