/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Inline direction helpers from `direction_func.h`.
use crate::direction_type::DIR_END;

/// `DiagDirToAxis`.
pub const fn diag_dir_to_axis(direction: u8) -> u8 {
    assert!(direction < 4); // Native valid-direction/axis assertion.
    direction & 1
}
/// `AxisToDiagDir`.
pub const fn axis_to_diag_dir(axis: u8) -> u8 {
    assert!(axis < 2); // Native valid-direction/axis assertion.
    2 - axis
}
/// `DirToDiagDir`.
pub const fn dir_to_diag_dir(direction: u8) -> u8 {
    debug_assert!(direction < DIR_END);
    direction >> 1
}
/// `ReverseDiagDir`.
pub const fn reverse_diag_dir(direction: u8) -> u8 {
    assert!(direction < 4); // Native valid-direction/axis assertion.
    direction ^ 2
}

/// `DiagDirToDir`, with a valid native diagonal direction.
pub const fn diag_dir_to_dir(d: u8) -> u8 {
    assert!(d < 4); // Native valid-direction/axis assertion.
    2 * d + 1
}
/// `ReverseDir`.
pub const fn reverse_dir(d: u8) -> u8 {
    assert!(d < DIR_END); // Native valid-direction/axis assertion.
    d ^ 4
}
