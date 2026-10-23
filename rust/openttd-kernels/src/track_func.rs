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
/// `TrackStatusToTrackBits`.
pub const fn track_status_to_track_bits(status: u32) -> u8 {
    trackdir_bits_to_track_bits(track_status_to_trackdir_bits(status))
}

/// `TrackdirToTrack`.
pub const fn trackdir_to_track(td: u8) -> u8 {
    td & 7
}
/// `TrackToTrackBits`.
pub const fn track_to_track_bits(track: u8) -> u8 {
    1 << track
}
/// `TrackBitsToTrackdirBits`.
pub const fn track_bits_to_trackdir_bits(bits: u8) -> u16 {
    (bits as u16) * 0x101
}
/// `TrackdirBitsToTrackBits`, including the native track mask.
#[allow(clippy::cast_possible_truncation)]
pub const fn trackdir_bits_to_track_bits(bits: u16) -> u8 {
    ((bits | (bits >> 8)) & crate::track_type::TRACK_BIT_MASK as u16) as u8
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
/// `DiagDirToDiagTrackdir`.
pub fn diag_dir_to_diag_trackdir(dir: u8) -> u8 {
    [0, 1, 8, 9][usize::from(dir)]
}
/// `DiagdirReachesTrackdirs`.
pub fn diagdir_reaches_trackdirs(dir: u8) -> u16 {
    [0x1009, 0x16, 0x520, 0x2a00][usize::from(dir)]
}
/// `TrackdirCrossesTrackdirs`.
pub fn trackdir_crosses_trackdirs(td: u8) -> u16 {
    [0x202, 0x101, 0x3030, 0x3030, 0xc0c, 0xc0c][usize::from(trackdir_to_track(td))]
}
/// `TracksOverlap`.
pub const fn tracks_overlap(bits: u8) -> bool {
    bits != 0
        && bits & bits.wrapping_sub(1) != 0
        && bits != crate::track_type::TRACK_BIT_HORZ
        && bits != crate::track_type::TRACK_BIT_VERT
}
/// `TrackOverlapsTracks`.
pub const fn track_overlaps_tracks(bits: u8, track: u8) -> bool {
    bits & track_to_track_bits(track) != 0 || tracks_overlap(bits | track_to_track_bits(track))
}

/// `IsDiagonalTrackdir`.
pub const fn is_diagonal_trackdir(td: u8) -> bool {
    trackdir_to_track(td) < 2
}
/// `DiagdirReachesTracks`.
pub fn diagdir_reaches_tracks(dir: u8) -> u8 {
    trackdir_bits_to_track_bits(diagdir_reaches_trackdirs(dir))
}
