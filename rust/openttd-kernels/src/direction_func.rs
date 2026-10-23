/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Inline direction helpers from `direction_func.h`.
//! A failed header precondition calls the native assertion handler. The caller
//! supplies it lazily; valid inputs cross no boundary and asserts-off keeps the
//! original computation. No Rust debug/release assertion setting is consulted.
use crate::direction_type::{AXIS_END, DIR_END};

/// `DiagDirToAxis`.
pub fn diag_dir_to_axis(direction: u8, check: impl Fn(bool) + Copy) -> u8 {
    if direction >= 4 {
        check(false);
    }
    direction & 1
}
/// `AxisToDiagDir`.
pub fn axis_to_diag_dir(axis: u8, check: impl Fn(bool) + Copy) -> u8 {
    if axis >= 2 {
        check(false);
    }
    crate::direction_type::DIAGDIR_SW.wrapping_sub(axis)
}
/// `DirToDiagDir`.
pub fn dir_to_diag_dir(direction: u8, check: impl Fn(bool) + Copy) -> u8 {
    if direction >= DIR_END {
        check(false);
    }
    direction >> 1
}
/// `ReverseDiagDir`.
pub fn reverse_diag_dir(direction: u8, check: impl Fn(bool) + Copy) -> u8 {
    if direction >= 4 {
        check(false);
    }
    direction ^ 2
}

/// `DiagDirToDir`, with a valid native diagonal direction.
pub fn diag_dir_to_dir(d: u8, check: impl Fn(bool) + Copy) -> u8 {
    if d >= 4 {
        check(false);
    }
    d.wrapping_mul(2).wrapping_add(1)
}
/// `ReverseDir`.
pub fn reverse_dir(d: u8, check: impl Fn(bool) + Copy) -> u8 {
    if d >= DIR_END {
        check(false);
    }
    d ^ 4
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn native_assertion_policy_keeps_asserts_off_arithmetic() {
        let failures = Cell::new(0);
        let disabled = |ok: bool| {
            assert!(!ok);
            failures.set(failures.get() + 1);
        };
        assert_eq!(diag_dir_to_axis(4, disabled), 0);
        assert_eq!(axis_to_diag_dir(3, disabled), 255);
        assert_eq!(reverse_diag_dir(4, disabled), 6);
        assert_eq!(dir_to_diag_dir(8, disabled), 4);
        assert_eq!(diag_dir_to_dir(255, disabled), 255);
        assert_eq!(reverse_dir(8, disabled), 12);
        assert_eq!(failures.get(), 6);
        let enabled = |ok: bool| assert!(ok);
        for direction in 0..4 {
            assert_eq!(diag_dir_to_axis(direction, enabled), direction & 1);
            assert_eq!(diag_dir_to_dir(direction, enabled), direction * 2 + 1);
        }
    }
}

/// `ChangeDiagDir` modulo the four diagonal directions.
pub const fn change_diag_dir(direction: u8, difference: u8) -> u8 {
    direction.wrapping_add(difference) & 3
}

/// `IsValidAxis`.
pub const fn is_valid_axis(axis: u8) -> bool {
    axis < AXIS_END
}
/// `OtherAxis`.
pub fn other_axis(axis: u8, check: impl Fn(bool) + Copy) -> u8 {
    if !is_valid_axis(axis) {
        check(false);
    }
    axis ^ 1
}
