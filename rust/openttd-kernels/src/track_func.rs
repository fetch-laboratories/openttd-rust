/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared inline track helpers declared in `track_func.h`.
use crate::track_type::{TRACK_BIT_LEFT, TRACK_BIT_LOWER, TRACK_BIT_RIGHT, TRACK_BIT_UPPER};
use crate::track_type::{TRACKDIR_X_NE, TRACKDIR_X_SW, TRACKDIR_Y_NW, TRACKDIR_Y_SE};

/// `AxisToTrack`.
pub fn axis_to_track(
    axis: crate::services::Axis,
    check: impl Fn(bool) + Copy,
) -> crate::services::Track {
    if axis.0 >= 2 {
        check(false);
    }
    crate::services::Track(axis.0)
}
/// `DiagDirToDiagTrack`.
pub fn diag_dir_to_diag_track(direction: u8, check: impl Fn(bool) + Copy) -> u8 {
    if direction >= 4 {
        check(false);
    }
    direction & 1
}

/// `CornerToTrackBits`, with a valid native corner.
pub fn corner_to_track_bits(c: u8, check: impl Fn(bool) + Copy) -> u8 {
    if c >= 4 {
        check(false);
    }
    [
        TRACK_BIT_LEFT,
        TRACK_BIT_LOWER,
        TRACK_BIT_RIGHT,
        TRACK_BIT_UPPER,
    ][c as usize]
}
/// `TracksOverlap`.
pub const fn tracks_overlap(b: u8) -> bool {
    b != crate::track_type::TRACK_BIT_NONE
        && crate::bitmath_func::kill_first_bit(b) != crate::track_type::TRACK_BIT_NONE
        && b != crate::track_type::TRACK_BIT_HORZ
        && b != crate::track_type::TRACK_BIT_VERT
}

/// `IsValidTrack`.
pub const fn is_valid_track(track: u8) -> bool {
    track < crate::track_type::TRACK_END
}
/// `HasTrack`, retaining its native assertion and `HasBit` callee.
pub fn has_track(bits: u8, track: u8, check: impl Fn(bool) + Copy) -> bool {
    if !is_valid_track(track) {
        check(false);
    }
    crate::bitmath_func::has_bit(bits, track)
}

/// `TrackToTrackBits`.
pub fn track_to_track_bits(track: u8, check: impl Fn(bool) + Copy) -> u8 {
    if !is_valid_track(track) {
        check(false);
    }
    (1_u32.wrapping_shl(u32::from(track)) & 0xff) as u8
}
/// `AxisToTrackBits`.
pub fn axis_to_track_bits(axis: u8, check: impl Fn(bool) + Copy) -> u8 {
    track_to_track_bits(axis_to_track(crate::services::Axis(axis), check).0, check)
}
/// `DiagDirToDiagTrackBits`.
pub fn diag_dir_to_diag_track_bits(dir: u8, check: impl Fn(bool) + Copy) -> u8 {
    if dir >= 4 {
        check(false);
    }
    track_to_track_bits(diag_dir_to_diag_track(dir, check), check)
}
/// `TrackBitsToTrackdirBits`.
pub const fn track_bits_to_trackdir_bits(bits: u8) -> u16 {
    (bits as u16) * 0x101
}
/// `CombineTrackStatus`.
pub const fn combine_track_status(bits: u16, red: u16) -> u32 {
    (bits as u32) | ((red as u32) << 16)
}
/// `DiagDirToDiagTrackdir`.
pub fn diag_dir_to_diag_trackdir(dir: u8, check: impl Fn(bool) + Copy) -> u8 {
    if dir >= 4 {
        check(false);
    }
    [TRACKDIR_X_NE, TRACKDIR_Y_SE, TRACKDIR_X_SW, TRACKDIR_Y_NW][dir as usize]
}

/// `IsReversingRoadTrackdir`, under its native valid-road-trackdir precondition.
pub fn is_reversing_road_trackdir(dir: u8, check: impl Fn(bool) + Copy) -> bool {
    if dir >= 16 {
        check(false);
    }
    (dir & 7) >= 6
}

/// `TrackStatusToTrackBits`.
pub const fn track_status_to_track_bits(status: u32) -> u8 {
    trackdir_bits_to_track_bits(track_status_to_trackdir_bits(status))
}

/// Native `TrackStatusToTrackdirBits`.
#[allow(clippy::cast_possible_truncation)] // Native mask and narrowing to TrackdirBits.
pub const fn track_status_to_trackdir_bits(status: u32) -> u16 {
    (status & crate::track_type::TRACKDIR_BIT_MASK) as u16
}

/// `TrackdirToTrack`.
pub const fn trackdir_to_track(td: u8) -> u8 {
    td & 7
}

/// `TrackdirBitsToTrackBits`, including the native track mask.
#[allow(clippy::cast_possible_truncation)]
pub const fn trackdir_bits_to_track_bits(bits: u16) -> u8 {
    ((bits | (bits >> 8)) & crate::track_type::TRACK_BIT_MASK as u16) as u8
}

/// `ReverseTrackdir`.
pub const fn reverse_trackdir(td: u8) -> u8 {
    td ^ 8
}

/// `TrackToTrackdir`.
pub const fn track_to_trackdir(track: u8) -> u8 {
    track
}

/// `TrackToTrackdirBits`.
pub const fn track_to_trackdir_bits(track: u8) -> u16 {
    let td = track_to_trackdir(track);
    (1 << td) | (1 << reverse_trackdir(td))
}

/// `TrackdirToExitdir`, table from `table/track_data.h`.
pub fn trackdir_to_exitdir(td: u8) -> u8 {
    [0, 1, 0, 1, 2, 1, 0, 0, 2, 3, 3, 2, 3, 0, 0, 0][usize::from(td)]
}

/// `DiagdirReachesTrackdirs`.
pub fn diagdir_reaches_trackdirs(dir: u8) -> u16 {
    [0x1009, 0x16, 0x520, 0x2a00][usize::from(dir)]
}

/// `TrackdirCrossesTrackdirs`.
pub fn trackdir_crosses_trackdirs(td: u8) -> u16 {
    [0x202, 0x101, 0x3030, 0x3030, 0xc0c, 0xc0c][usize::from(trackdir_to_track(td))]
}

/// `TrackOverlapsTracks`.
pub fn track_overlaps_tracks(bits: u8, track: u8, check: impl Fn(bool) + Copy) -> bool {
    bits & track_to_track_bits(track, check) != 0
        || tracks_overlap(bits | track_to_track_bits(track, check))
}

/// `IsDiagonalTrackdir`.
pub const fn is_diagonal_trackdir(td: u8) -> bool {
    trackdir_to_track(td) < 2
}

/// `DiagdirReachesTracks`.
pub fn diagdir_reaches_tracks(dir: u8) -> u8 {
    trackdir_bits_to_track_bits(diagdir_reaches_trackdirs(dir))
}

/// `FindFirstTrackdir`.
#[allow(clippy::cast_possible_truncation)]
pub const fn find_first_trackdir(bits: u16) -> u8 {
    if bits == 0 {
        0xff
    } else {
        bits.trailing_zeros() as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_asserts_off_retains_header_conversions() {
        let disabled = |ok: bool| assert!(!ok);
        assert_eq!(axis_to_track(crate::services::Axis(2), disabled).0, 2);
        assert_eq!(axis_to_track_bits(2, disabled), 4);
        assert_eq!(track_to_track_bits(8, disabled), 0);
        assert_eq!(diag_dir_to_diag_track(4, disabled), 0);
        assert!(is_reversing_road_trackdir(22, disabled));
    }
}
