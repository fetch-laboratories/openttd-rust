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

/// `ClrBit` for uint32 fields, preserving the original return value.
pub fn clr_bit(value: &mut u32, bit: u8) -> u32 {
    *value &= !1_u32.wrapping_shl(u32::from(bit));
    *value
}

/// `HasExactlyOneBit` for byte-sized counts, with C++ integer promotion.
pub const fn has_exactly_one_bit(value: u8) -> bool {
    value != 0 && value & value.wrapping_sub(1) == 0
}

/// `SB` for uint32 fields: clear the destination window, then OR the entire
/// shifted operand. The original deliberately does not mask the operand.
pub fn sb(value: &mut u32, start: u8, length: u8, data: u32) {
    *value &= !((1_u32.wrapping_shl(u32::from(length)) - 1).wrapping_shl(u32::from(start)));
    *value |= data.wrapping_shl(u32::from(start));
}

/// `GB` for a 32-bit promoted input. Native defined inputs retain their result;
/// wrapping shifts avoid introducing a panic for the native out-of-width domain.
pub const fn gb(value: u32, start: u8, count: u8) -> u32 {
    value.wrapping_shr(start as u32) & (1_u32.wrapping_shl(count as u32).wrapping_sub(1))
}

/// `KillFirstBit` for the byte-sized track bitsets.
pub const fn kill_first_bit(value: u8) -> u8 {
    value & value.wrapping_sub(1)
}

/// `FindFirstBit` for a cargo bitmask, including its zero-input result.
#[allow(clippy::cast_possible_truncation)]
pub const fn find_first_bit(value: u64) -> u8 {
    if value == 0 {
        0
    } else {
        value.trailing_zeros() as u8
    }
}

/// `ToggleBit` for promoted integer fields.
pub fn toggle_bit(value: &mut u32, bit: u8) -> u32 {
    *value ^= 1_u32.wrapping_shl(u32::from(bit));
    *value
}

/// `SetBit` for native 64-bit cargo masks, preserving the original return value.
pub fn set_bit64(value: &mut u64, bit: u8) -> u64 {
    *value |= 1_u64.wrapping_shl(u32::from(bit));
    *value
}

/// `ClrBit` for uint64 cargo masks.
pub fn clr_bit64(value: &mut u64, bit: u8) -> u64 {
    *value &= !1_u64.wrapping_shl(u32::from(bit));
    *value
}
