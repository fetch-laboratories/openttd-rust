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

/// `TrackStatusToTrackdirBits`.
pub const fn track_status_to_trackdir_bits(status: u32) -> u16 {
    (status & crate::track_type::TRACKDIR_BIT_MASK) as u16
}
/// `TrackdirBitsToTrackBits`.
pub const fn trackdir_bits_to_track_bits(bits: u16) -> u8 {
    ((bits | (bits >> 8)) & crate::track_type::TRACK_BIT_MASK) as u8
}
/// `TrackStatusToTrackBits`.
pub const fn track_status_to_track_bits(status: u32) -> u8 {
    trackdir_bits_to_track_bits(track_status_to_trackdir_bits(status))
}

/// `TracksOverlap`.
pub fn tracks_overlap(bits: u8) -> bool {
    use crate::track_type::{TRACK_BIT_HORZ, TRACK_BIT_VERT};
    if bits == 0 || bits & bits.wrapping_sub(1) == 0 {
        return false;
    }
    bits != TRACK_BIT_HORZ && bits != TRACK_BIT_VERT
}
