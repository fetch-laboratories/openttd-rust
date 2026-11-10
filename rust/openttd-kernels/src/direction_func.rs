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
use crate::direction_type::{Axis, DiagDirDiff, DiagDirection, DiagDirections, DirDiff, Direction};

/// `IsValidDiagDirection`.
pub const fn is_valid_diag_direction(d: DiagDirection) -> bool {
    d.0 < DiagDirection::DIAGDIR_END.0
}
/// `IsValidDirection`.
pub const fn is_valid_direction(d: Direction) -> bool {
    d.0 < Direction::DIR_END.0
}
/// `IsValidAxis`.
pub const fn is_valid_axis(d: Axis) -> bool {
    d.0 < Axis::AXIS_END.0
}
/// `ReverseDir`.
pub fn reverse_dir(d: Direction, check: impl Fn(bool)) -> Direction {
    if !is_valid_direction(d) {
        check(false);
    }
    Direction(4 ^ d.0)
}
/// `DirDifference`.
pub fn dir_difference(d0: Direction, d1: Direction, check: impl Fn(bool)) -> DirDiff {
    if !is_valid_direction(d0) {
        check(false);
    }
    if !is_valid_direction(d1) {
        check(false);
    }
    /* Cast to uint so compiler can use bitmask. If the difference is negative
     * and we used int instead of uint, further "+ 8" would have to be added. */
    DirDiff((u32::from(d0.0).wrapping_sub(u32::from(d1.0)) % 8) as u8)
}
/// `ChangeDirDiff`.
pub const fn change_dir_diff(d: DirDiff, delta: DirDiff) -> DirDiff {
    /* Cast to uint so compiler can use bitmask. Result can never be negative. */
    DirDiff(((d.0 as u32 + delta.0 as u32) % 8) as u8)
}
/// `ChangeDir`.
pub fn change_dir(d: Direction, delta: DirDiff, check: impl Fn(bool)) -> Direction {
    if !is_valid_direction(d) {
        check(false);
    }
    /* Cast to uint so compiler can use bitmask. Result can never be negative. */
    Direction(((u32::from(d.0) + u32::from(delta.0)) % 8) as u8)
}
/// `ReverseDiagDir`.
pub fn reverse_diag_dir(d: DiagDirection, check: impl Fn(bool)) -> DiagDirection {
    if !is_valid_diag_direction(d) {
        check(false);
    }
    DiagDirection(2 ^ d.0)
}
/// `ChangeDiagDir`.
pub const fn change_diag_dir(d: DiagDirection, delta: DiagDirDiff) -> DiagDirection {
    /* Cast to uint so compiler can use bitmask. Result can never be negative. */
    DiagDirection(((d.0 as u32 + delta.0 as u32) % 4) as u8)
}
/// `DirToDiagDir`.
pub fn dir_to_diag_dir(dir: Direction, check: impl Fn(bool)) -> DiagDirection {
    if !is_valid_direction(dir) {
        check(false);
    }
    DiagDirection(dir.0 >> 1)
}
/// `DiagDirToDir`.
pub fn diag_dir_to_dir(dir: DiagDirection, check: impl Fn(bool)) -> Direction {
    if !is_valid_diag_direction(dir) {
        check(false);
    }
    Direction(dir.0.wrapping_mul(2).wrapping_add(1))
}
/// `OtherAxis`.
pub fn other_axis(a: Axis, check: impl Fn(bool)) -> Axis {
    if !is_valid_axis(a) {
        check(false);
    }
    Axis(a.0 ^ 1)
}
/// `DiagDirToAxis`.
pub fn diag_dir_to_axis(d: DiagDirection, check: impl Fn(bool)) -> Axis {
    if !is_valid_diag_direction(d) {
        check(false);
    }
    Axis(d.0 & 1)
}
/// `AxisToDiagDir`.
pub fn axis_to_diag_dir(a: Axis, check: impl Fn(bool)) -> DiagDirection {
    if !is_valid_axis(a) {
        check(false);
    }
    DiagDirection(2_u8.wrapping_sub(a.0))
}
/// `AxisToDiagDirs`.
pub fn axis_to_diag_dirs(a: Axis, check: impl Fn(bool)) -> DiagDirections {
    if !is_valid_axis(a) {
        check(false);
    }
    if a == Axis::AXIS_X {
        DiagDirections::from_element(DiagDirection::DIAGDIR_NE)
            | DiagDirections::from_element(DiagDirection::DIAGDIR_SW)
    } else {
        DiagDirections::from_element(DiagDirection::DIAGDIR_SE)
            | DiagDirections::from_element(DiagDirection::DIAGDIR_NW)
    }
}

/// Raw adapter: `DiagDirToAxis` over base values, for files no pass has finished yet.
pub fn diag_dir_to_axis_raw(direction: u8, check: impl Fn(bool)) -> u8 {
    diag_dir_to_axis(DiagDirection(direction), check).0
}
/// Raw adapter: `AxisToDiagDir` over base values.
pub fn axis_to_diag_dir_raw(axis: u8, check: impl Fn(bool)) -> u8 {
    axis_to_diag_dir(Axis(axis), check).0
}
/// Raw adapter: `DirToDiagDir` over base values.
pub fn dir_to_diag_dir_raw(direction: u8, check: impl Fn(bool)) -> u8 {
    dir_to_diag_dir(Direction(direction), check).0
}
/// Raw adapter: `ReverseDiagDir` over base values.
pub fn reverse_diag_dir_raw(direction: u8, check: impl Fn(bool)) -> u8 {
    reverse_diag_dir(DiagDirection(direction), check).0
}
/// Raw adapter: `DiagDirToDir` over base values.
pub fn diag_dir_to_dir_raw(d: u8, check: impl Fn(bool)) -> u8 {
    diag_dir_to_dir(DiagDirection(d), check).0
}
/// Raw adapter: `ReverseDir` over base values.
pub fn reverse_dir_raw(d: u8, check: impl Fn(bool)) -> u8 {
    reverse_dir(Direction(d), check).0
}
/// Raw adapter: `ChangeDiagDir` over base values.
pub const fn change_diag_dir_raw(direction: u8, difference: u8) -> u8 {
    change_diag_dir(DiagDirection(direction), DiagDirDiff(difference)).0
}
/// Raw adapter: `IsValidAxis` over a base value.
pub const fn is_valid_axis_raw(axis: u8) -> bool {
    is_valid_axis(Axis(axis))
}
/// Raw adapter: `OtherAxis` over base values.
pub fn other_axis_raw(axis: u8, check: impl Fn(bool)) -> u8 {
    other_axis(Axis(axis), check).0
}
/// Raw adapter: `DirDifference` over base values.
pub fn dir_difference_raw(a: u8, b: u8, check: impl Fn(bool)) -> u8 {
    dir_difference(Direction(a), Direction(b), check).0
}
/// Raw adapter: `ChangeDirDiff` over base values.
pub const fn change_dir_diff_raw(a: u8, b: u8) -> u8 {
    change_dir_diff(DirDiff(a), DirDiff(b)).0
}
/// Raw adapter: `ChangeDir` over base values.
pub fn change_dir_raw(direction: u8, difference: u8, check: impl Fn(bool)) -> u8 {
    change_dir(Direction(direction), DirDiff(difference), check).0
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
        assert_eq!(diag_dir_to_axis_raw(4, disabled), 0);
        assert_eq!(axis_to_diag_dir_raw(3, disabled), 255);
        assert_eq!(reverse_diag_dir_raw(4, disabled), 6);
        assert_eq!(dir_to_diag_dir_raw(8, disabled), 4);
        assert_eq!(diag_dir_to_dir_raw(255, disabled), 255);
        assert_eq!(reverse_dir_raw(8, disabled), 12);
        assert_eq!(failures.get(), 6);
    }
}
