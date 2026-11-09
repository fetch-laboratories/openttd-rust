/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `core/bitmath_func.hpp` bit operations used by game owners.
/// `HasBit`: checks if a bit in a value is set. The `bit` value specifies the position of
/// the bit, started at the LSB and count from 0. For byte and uint32 bitsets, with C++
/// integer promotion.
/// Out-of-width shifts in C++ have no defined behavior; source-valid bit indices
/// retain the promoted expression, without adding a Rust shift-overflow panic.
pub fn has_bit(value: impl Into<u32>, bit: u8) -> bool {
    value.into() & 1_u32.wrapping_shl(u32::from(bit)) != 0
}

/// `SetBit`: set a bit in a variable. The variable is changed and the value is also
/// returned. For uint32 fields.
pub fn set_bit(value: &mut u32, bit: u8) -> u32 {
    *value |= 1_u32.wrapping_shl(u32::from(bit));
    *value
}

/// `ClrBit`: clears a bit in a variable. The variable is changed and the value is also
/// returned. For uint32 fields.
pub fn clr_bit(value: &mut u32, bit: u8) -> u32 {
    *value &= !1_u32.wrapping_shl(u32::from(bit));
    *value
}

/// `HasExactlyOneBit`: test whether `value` has exactly 1 bit set. For byte-sized counts,
/// with C++ integer promotion.
pub const fn has_exactly_one_bit(value: u8) -> bool {
    value != 0 && value & value.wrapping_sub(1) == 0
}

/// `SB`: set `length` bits in `value` starting at bit `start` to `data`. For uint32 fields:
/// clear the destination window, then OR the entire shifted operand. The original
/// deliberately does not mask the operand ("This is not a bug, its a feature.").
pub fn sb(value: &mut u32, start: u8, length: u8, data: u32) {
    *value &= !((1_u32.wrapping_shl(u32::from(length)) - 1).wrapping_shl(u32::from(start)));
    *value |= data.wrapping_shl(u32::from(start));
}

/// `AssignBit`: assigns a single bit in a variable. The variable is changed and the value is
/// also returned. For promoted integer fields.
pub fn assign_bit(value: &mut u32, bit: u8, enabled: bool) -> u32 {
    sb(value, bit, 1, u32::from(enabled));
    *value
}

/// `GB`: fetch `count` bits from `value`, started at bit `start`. For a 32-bit promoted
/// input. Native defined inputs retain their result;
/// wrapping shifts avoid introducing a panic for the native out-of-width domain.
pub const fn gb(value: u32, start: u8, count: u8) -> u32 {
    value.wrapping_shr(start as u32) & (1_u32.wrapping_shl(count as u32).wrapping_sub(1))
}

/// `KillFirstBit`: clear the first bit in an integer. This function returns a value where
/// the first bit (from LSB) is cleared. So, 110100 returns 110000, 000001 returns 000000,
/// etc. For the byte-sized track bitsets.
#[allow(
    clippy::cast_possible_truncation,
    reason = "KillFirstBit returns its own T; clearing a bit cannot widen the input byte"
)]
pub const fn kill_first_bit(value: u8) -> u8 {
    kill_first_bit_u32(value as u32) as u8
}

/// `KillFirstBit` for script-visible unsigned track masks.
pub const fn kill_first_bit_u32(value: u32) -> u32 {
    value & value.wrapping_sub(1)
}

/// `FindFirstBit`: search the first set bit in a value. When no bit is set, it returns 0.
/// For a cargo bitmask.
#[allow(
    clippy::cast_possible_truncation,
    reason = "FindFirstBit returns uint8_t; a uint64 bit position is at most 63"
)]
pub const fn find_first_bit(value: u64) -> u8 {
    if value == 0 {
        0
    } else {
        value.trailing_zeros() as u8
    }
}

/// `ToggleBit`: toggles a bit in a variable. The variable is changed and the value is also
/// returned. For promoted integer fields.
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
