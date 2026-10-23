/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared inline track helpers declared in `track_func.h`.
/// `AxisToTrack`.
pub const fn axis_to_track(axis: crate::services::Axis) -> crate::services::Track {
    crate::services::Track(axis.0)
}
/// `DiagDirToDiagTrack`.
pub const fn diag_dir_to_diag_track(direction: u8) -> u8 {
    direction & 1
}

/// `CornerToTrackBits`, with a valid native corner.
pub const fn corner_to_track_bits(c: u8) -> u8 {
    [16, 8, 32, 4][c as usize]
}
/// `TracksOverlap`.
pub const fn tracks_overlap(b: u8) -> bool {
    b != 0 && b & b.wrapping_sub(1) != 0 && b != 12 && b != 48
}
