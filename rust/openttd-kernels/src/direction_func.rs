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

/// `IsValidDiagDirection`: checks if an integer value is a valid `DiagDirection`.
pub const fn is_valid_diag_direction(d: DiagDirection) -> bool {
    d.0 < DiagDirection::DIAGDIR_END.0
}
/// `IsValidDirection`: checks if an integer value is a valid `Direction`.
pub const fn is_valid_direction(d: Direction) -> bool {
    d.0 < Direction::DIR_END.0
}
/// `IsValidAxis`: checks if an integer value is a valid `Axis`.
pub const fn is_valid_axis(d: Axis) -> bool {
    d.0 < Axis::AXIS_END.0
}
/// `ReverseDir`: return the reverse of a direction.
pub fn reverse_dir(d: Direction, check: impl Fn(bool)) -> Direction {
    if !is_valid_direction(d) {
        check(false);
    }
    Direction(4 ^ d.0)
}
/// `DirDifference`: calculate the difference between two directions; how the
/// second direction drifts of the first one.
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
/// `ChangeDirDiff`: applies two differences together.
///
/// This function adds two differences together and returns the resulting
/// difference. So adding two `DIRDIFF_REVERSE` together results in the
/// `DIRDIFF_SAME` difference.
pub const fn change_dir_diff(d: DirDiff, delta: DirDiff) -> DirDiff {
    /* Cast to uint so compiler can use bitmask. Result can never be negative. */
    DirDiff(((d.0 as u32 + delta.0 as u32) % 8) as u8)
}
/// `ChangeDir`: change a direction by a given difference.
///
/// This functions returns a new direction of the given direction
/// which is rotated by the given difference.
pub fn change_dir(d: Direction, delta: DirDiff, check: impl Fn(bool)) -> Direction {
    if !is_valid_direction(d) {
        check(false);
    }
    /* Cast to uint so compiler can use bitmask. Result can never be negative. */
    Direction(((u32::from(d.0) + u32::from(delta.0)) % 8) as u8)
}
/// `ReverseDiagDir`: returns the reverse direction of the given `DiagDirection`.
pub fn reverse_diag_dir(d: DiagDirection, check: impl Fn(bool)) -> DiagDirection {
    if !is_valid_diag_direction(d) {
        check(false);
    }
    DiagDirection(2 ^ d.0)
}
/// `ChangeDiagDir`: applies a difference on a `DiagDirection`.
///
/// This function applies a difference on a `DiagDirection` and returns
/// the new `DiagDirection`.
pub fn change_diag_dir(
    d: DiagDirection,
    delta: DiagDirDiff,
    check: impl Fn(bool),
) -> DiagDirection {
    if !is_valid_diag_direction(d) {
        check(false);
    }
    /* Cast to uint so compiler can use bitmask. Result can never be negative. */
    DiagDirection(((u32::from(d.0) + u32::from(delta.0)) % 4) as u8)
}
/// `DiagDirDifference`: calculate the difference between two `DiagDirection`
/// values; how the second direction drifts of the first one.
#[allow(
    clippy::cast_possible_truncation,
    reason = "DiagDirDifference's (uint)(d0 - d1) % 4 fits its DiagDirDiff"
)]
pub fn diag_dir_difference(
    d0: DiagDirection,
    d1: DiagDirection,
    check: impl Fn(bool),
) -> DiagDirDiff {
    if !is_valid_diag_direction(d0) {
        check(false);
    }
    if !is_valid_diag_direction(d1) {
        check(false);
    }
    /* Cast to uint so compiler can use bitmask. Result can never be negative. */
    DiagDirDiff(((i32::from(d0.0) - i32::from(d1.0)).cast_unsigned() % 4) as u8)
}
/// `DirToDiagDir`: convert a `Direction` to a `DiagDirection`.
///
/// This function can be used to convert the 8-way `Direction` to
/// the 4-way `DiagDirection`. If the direction cannot be mapped its
/// "rounded clockwise". So `DIR_N` becomes `DIAGDIR_NE`.
pub fn dir_to_diag_dir(dir: Direction, check: impl Fn(bool)) -> DiagDirection {
    if !is_valid_direction(dir) {
        check(false);
    }
    DiagDirection(dir.0 >> 1)
}
/// `DiagDirToDir`: convert a `DiagDirection` to a `Direction`.
///
/// This function can be used to convert the 4-way `DiagDirection`
/// to the 8-way `Direction`. As 4-way are less than 8-way not all
/// possible directions can be calculated.
pub fn diag_dir_to_dir(dir: DiagDirection, check: impl Fn(bool)) -> Direction {
    if !is_valid_diag_direction(dir) {
        check(false);
    }
    Direction(dir.0.wrapping_mul(2).wrapping_add(1))
}
/// `OtherAxis`: select the other axis as provided.
///
/// This is basically the not-operator for the axis.
pub fn other_axis(a: Axis, check: impl Fn(bool)) -> Axis {
    if !is_valid_axis(a) {
        check(false);
    }
    Axis(a.0 ^ 1)
}
/// `DiagDirToAxis`: convert a `DiagDirection` to the axis.
///
/// This function returns the axis which belongs to the given
/// `DiagDirection`. The axis X belongs to the `DiagDirection`
/// north-east and south-west.
pub fn diag_dir_to_axis(d: DiagDirection, check: impl Fn(bool)) -> Axis {
    if !is_valid_diag_direction(d) {
        check(false);
    }
    Axis(d.0 & 1)
}
/// `AxisToDiagDir`: converts an `Axis` to a `DiagDirection`.
///
/// This function returns the `DiagDirection` which
/// belongs to the axis. As 2 directions are mapped to an axis
/// this function returns the one which points to south,
/// either south-west (on X axis) or south-east (on Y axis).
pub fn axis_to_diag_dir(a: Axis, check: impl Fn(bool)) -> DiagDirection {
    if !is_valid_axis(a) {
        check(false);
    }
    DiagDirection(2_u8.wrapping_sub(a.0))
}
/// `AxisToDiagDirs`: converts an `Axis` to `DiagDirections`.
///
/// This function returns both `DiagDirections` which
/// belong to the axis.
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
/// Raw adapter: `ReverseDir` over base values.
pub fn reverse_dir_raw(d: u8, check: impl Fn(bool)) -> u8 {
    reverse_dir(Direction(d), check).0
}
/// Raw adapter: `ChangeDiagDir` over base values.
pub fn change_diag_dir_raw(direction: u8, difference: u8, check: impl Fn(bool)) -> u8 {
    change_diag_dir(DiagDirection(direction), DiagDirDiff(difference), check).0
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
    fn diag_dir_difference_over_every_valid_pair() {
        // direction_func.h: how the second direction drifts of the first one,
        // DIAGDIRDIFF_SAME, _90RIGHT, _REVERSE or _90LEFT.
        let expected = [[0, 3, 2, 1], [1, 0, 3, 2], [2, 1, 0, 3], [3, 2, 1, 0]];
        for d0 in 0..4_u8 {
            for d1 in 0..4_u8 {
                assert_eq!(
                    diag_dir_difference(DiagDirection(d0), DiagDirection(d1), |ok| assert!(ok)),
                    DiagDirDiff(expected[usize::from(d0)][usize::from(d1)]),
                    "DiagDirDifference({d0}, {d1})"
                );
            }
        }
        assert_eq!(
            diag_dir_difference(
                DiagDirection::DIAGDIR_NE,
                DiagDirection::DIAGDIR_SE,
                |ok| assert!(ok)
            ),
            DiagDirDiff::DIAGDIRDIFF_90LEFT
        );
    }

    #[test]
    fn native_assertion_policy_keeps_asserts_off_arithmetic() {
        let failures = Cell::new(0);
        let disabled = |ok: bool| {
            assert!(!ok);
            failures.set(failures.get() + 1);
        };
        assert_eq!(diag_dir_to_axis(DiagDirection(4), disabled), Axis(0));
        assert_eq!(axis_to_diag_dir(Axis(3), disabled), DiagDirection(255));
        assert_eq!(
            reverse_diag_dir(DiagDirection(4), disabled),
            DiagDirection(6)
        );
        assert_eq!(dir_to_diag_dir(Direction(8), disabled), DiagDirection(4));
        assert_eq!(
            diag_dir_to_dir(DiagDirection(255), disabled),
            Direction(255)
        );
        assert_eq!(reverse_dir(Direction(8), disabled), Direction(12));
        assert_eq!(failures.get(), 6);
    }
}
