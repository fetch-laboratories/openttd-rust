/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared inline track helpers declared in `track_func.h`.
use crate::track_type::{TRACK_BIT_LEFT, TRACK_BIT_LOWER, TRACK_BIT_RIGHT, TRACK_BIT_UPPER};
use crate::track_type::{TRACKDIR_X_NE, TRACKDIR_X_SW, TRACKDIR_Y_NW, TRACKDIR_Y_SE};

/// `RemoveFirstTrack`: removes the lowest track from `tracks` and returns it,
/// or `INVALID_TRACK` when `tracks` is empty.
pub fn remove_first_track(
    tracks: &mut crate::track_type::TrackBits,
    check: impl Fn(bool) + Copy,
) -> crate::track_type::Track {
    use crate::track_type::{Track, TrackBits};
    if *tracks != TrackBits::TRACK_BIT_NONE {
        if tracks.0 & !TrackBits::TRACK_BIT_MASK.0 != TrackBits::TRACK_BIT_NONE.0 {
            check(false);
        }
        let first = Track(crate::bitmath_func::find_first_bit(u64::from(tracks.0)));
        crate::bitmath_func::clr_bit8(&mut tracks.0, first.0);
        return first;
    }
    Track::INVALID_TRACK
}
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
    if !crate::slope_func::is_valid_corner_raw(c) {
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

/// `TrackBitsToTrack`, including its single-track assertion and masked lookup.
pub fn track_bits_to_track(tracks: u8, check: impl Fn(bool) + Copy) -> u8 {
    let masked = tracks & crate::track_type::TRACK_BIT_MASK;
    check(
        tracks != crate::track_type::TRACK_BIT_NONE
            && crate::bitmath_func::kill_first_bit(masked) == crate::track_type::TRACK_BIT_NONE,
    );
    crate::bitmath_func::find_first_bit(u64::from(masked))
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

/// `IsValidTrackdir`.
pub const fn is_valid_trackdir(trackdir: u8) -> bool {
    trackdir != crate::track_type::INVALID_TRACKDIR
        && (1_u32.wrapping_shl(trackdir as u32) & crate::track_type::TRACKDIR_BIT_MASK) != 0
}

/// `FindFirstTrack`.
pub const fn find_first_track(tracks: u8) -> u8 {
    find_first_track_u32(tracks as u32)
}

/// `FindFirstTrack` for the ship controller's promoted track masks.
pub const fn find_first_track_u32(tracks: u32) -> u8 {
    if tracks == 0 {
        crate::track_type::INVALID_TRACK
    } else {
        crate::bitmath_func::find_first_bit(tracks as u64)
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

/// `HasTrackdir`: checks whether a `TrackdirBits` has a given `Trackdir`, under
/// its native valid-trackdir precondition.
pub fn has_trackdir(trackdirs: u16, trackdir: u8, check: impl Fn(bool)) -> bool {
    if !is_valid_trackdir(trackdir) {
        check(false);
    }
    trackdirs & 1_u16.wrapping_shl(u32::from(trackdir)) != 0
}

/// `NextTrackdir`: maps a trackdir to the trackdir that you will end up on if
/// you go straight ahead. This will be the same trackdir for diagonal
/// trackdirs, but a different (alternating) one for straight trackdirs.
pub fn next_trackdir(trackdir: u8, check: impl Fn(bool)) -> u8 {
    use crate::track_type::Trackdir as T;
    /// `_next_trackdir`, from `table/track_data.h`.
    const NEXT_TRACKDIR: [T; 14] = [
        T::TRACKDIR_X_NE,
        T::TRACKDIR_Y_SE,
        T::TRACKDIR_LOWER_E,
        T::TRACKDIR_UPPER_E,
        T::TRACKDIR_RIGHT_S,
        T::TRACKDIR_LEFT_S,
        T::INVALID_TRACKDIR,
        T::INVALID_TRACKDIR,
        T::TRACKDIR_X_SW,
        T::TRACKDIR_Y_NW,
        T::TRACKDIR_LOWER_W,
        T::TRACKDIR_UPPER_W,
        T::TRACKDIR_RIGHT_N,
        T::TRACKDIR_LEFT_N,
    ];
    if !is_valid_trackdir(trackdir) {
        check(false);
    }
    NEXT_TRACKDIR[usize::from(trackdir)].0
}

/// `IsUphillTrackdir`: checks whether a trackdir on a specific slope is going
/// uphill. Valid for rail and road tracks, and for tile-slopes (under
/// foundation) and foundation-slopes (on foundation).
pub fn is_uphill_trackdir(slope: crate::slope_type::Slope, dir: u8, check: impl Fn(bool)) -> bool {
    use crate::track_type::TrackdirBits as B;
    /// `_uphill_trackdirs`, from `table/track_data.h`.
    const UPHILL_TRACKDIRS: [u16; 31] = [
        B::TRACKDIR_BIT_NONE.0,                          //  0 SLOPE_FLAT
        B::TRACKDIR_BIT_X_SW.0 | B::TRACKDIR_BIT_Y_NW.0, //  1 SLOPE_W   -> inclined for diagonal track
        B::TRACKDIR_BIT_X_SW.0 | B::TRACKDIR_BIT_Y_SE.0, //  2 SLOPE_S   -> inclined for diagonal track
        B::TRACKDIR_BIT_X_SW.0,                          //  3 SLOPE_SW
        B::TRACKDIR_BIT_X_NE.0 | B::TRACKDIR_BIT_Y_SE.0, //  4 SLOPE_E   -> inclined for diagonal track
        B::TRACKDIR_BIT_NONE.0,                          //  5 SLOPE_EW
        B::TRACKDIR_BIT_Y_SE.0,                          //  6 SLOPE_SE
        B::TRACKDIR_BIT_NONE.0,                          //  7 SLOPE_WSE -> leveled
        B::TRACKDIR_BIT_X_NE.0 | B::TRACKDIR_BIT_Y_NW.0, //  8 SLOPE_N   -> inclined for diagonal track
        B::TRACKDIR_BIT_Y_NW.0,                          //  9 SLOPE_NW
        B::TRACKDIR_BIT_NONE.0,                          // 10 SLOPE_NS
        B::TRACKDIR_BIT_NONE.0,                          // 11 SLOPE_NWS -> leveled
        B::TRACKDIR_BIT_X_NE.0,                          // 12 SLOPE_NE
        B::TRACKDIR_BIT_NONE.0,                          // 13 SLOPE_ENW -> leveled
        B::TRACKDIR_BIT_NONE.0,                          // 14 SLOPE_SEN -> leveled
        B::TRACKDIR_BIT_NONE.0,                          // 15 invalid
        B::TRACKDIR_BIT_NONE.0,                          // 16 invalid
        B::TRACKDIR_BIT_NONE.0,                          // 17 invalid
        B::TRACKDIR_BIT_NONE.0,                          // 18 invalid
        B::TRACKDIR_BIT_NONE.0,                          // 19 invalid
        B::TRACKDIR_BIT_NONE.0,                          // 20 invalid
        B::TRACKDIR_BIT_NONE.0,                          // 21 invalid
        B::TRACKDIR_BIT_NONE.0,                          // 22 invalid
        B::TRACKDIR_BIT_X_SW.0 | B::TRACKDIR_BIT_Y_SE.0, // 23 SLOPE_STEEP_S -> inclined for diagonal track
        B::TRACKDIR_BIT_NONE.0,                          // 24 invalid
        B::TRACKDIR_BIT_NONE.0,                          // 25 invalid
        B::TRACKDIR_BIT_NONE.0,                          // 26 invalid
        B::TRACKDIR_BIT_X_SW.0 | B::TRACKDIR_BIT_Y_NW.0, // 27 SLOPE_STEEP_W -> inclined for diagonal track
        B::TRACKDIR_BIT_NONE.0,                          // 28 invalid
        B::TRACKDIR_BIT_X_NE.0 | B::TRACKDIR_BIT_Y_NW.0, // 29 SLOPE_STEEP_N -> inclined for diagonal track
        B::TRACKDIR_BIT_X_NE.0 | B::TRACKDIR_BIT_Y_SE.0, // 30 SLOPE_STEEP_E -> inclined for diagonal track
    ];
    // `IsValidTrackdirForRoadVehicle`.
    if dir >= crate::track_type::Trackdir::TRACKDIR_END.0 {
        check(false);
    }
    crate::bitmath_func::has_bit(
        UPHILL_TRACKDIRS[usize::from(crate::slope_func::remove_halftile_slope(slope).0)],
        dir,
    )
}

/// `TrackdirToTrackdirBits`.
pub const fn trackdir_to_trackdir_bits(td: u8) -> u16 {
    1_u16 << td
}

/// `RemoveFirstTrackdir`: removes the first `Trackdir` from `TrackdirBits` and
/// returns it, or `INVALID_TRACKDIR` when the bits hold none.
pub fn remove_first_trackdir(
    trackdirs: &mut crate::track_type::TrackdirBits,
    check: impl Fn(bool),
) -> crate::track_type::Trackdir {
    use crate::track_type::{Trackdir, TrackdirBits};
    if *trackdirs != TrackdirBits::TRACKDIR_BIT_NONE
        && *trackdirs != TrackdirBits::INVALID_TRACKDIR_BIT
    {
        if trackdirs.0 & !TrackdirBits::TRACKDIR_BIT_MASK.0 != TrackdirBits::TRACKDIR_BIT_NONE.0 {
            check(false);
        }
        let first = Trackdir(crate::bitmath_func::find_first_bit(u64::from(trackdirs.0)));
        crate::bitmath_func::clr_bit16(&mut trackdirs.0, first.0);
        return first;
    }
    Trackdir::INVALID_TRACKDIR
}

/// `VehicleExitDir`: determine the side in which the vehicle will leave the tile.
pub fn vehicle_exit_dir(
    direction: crate::direction_type::Direction,
    track: crate::track_type::TrackBits,
    check: impl Fn(bool) + Copy,
) -> crate::direction_type::DiagDirection {
    use crate::direction_type::DiagDirDiff;
    use crate::track_type::TrackBits;
    const STATE_DIR_TABLE: [TrackBits; 4] = [
        TrackBits::TRACK_BIT_RIGHT,
        TrackBits::TRACK_BIT_LOWER,
        TrackBits::TRACK_BIT_LEFT,
        TrackBits::TRACK_BIT_UPPER,
    ];

    let mut diagdir = crate::direction_func::dir_to_diag_dir(direction, check);

    /* Determine the diagonal direction in which we will exit this tile */
    if direction.0 & 1 == 0 && track != STATE_DIR_TABLE[usize::from(diagdir.0)] {
        diagdir =
            crate::direction_func::change_diag_dir(diagdir, DiagDirDiff::DIAGDIRDIFF_90LEFT, check);
    }

    diagdir
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn native_asserts_off_retains_header_conversions() {
        let failures = Cell::new(0);
        let disabled = |ok: bool| {
            assert!(!ok);
            failures.set(failures.get() + 1);
        };
        assert_eq!(axis_to_track(crate::services::Axis(2), disabled).0, 2);
        assert_eq!(failures.get(), 1);
        // `AxisToTrackBits` asserts only the axis: the resulting `TRACK_UPPER` is valid.
        assert_eq!(axis_to_track_bits(2, disabled), 4);
        assert_eq!(failures.get(), 2);
        assert_eq!(track_to_track_bits(8, disabled), 0);
        assert_eq!(diag_dir_to_diag_track(4, disabled), 0);
        assert!(is_reversing_road_trackdir(22, disabled));
        assert_eq!(failures.get(), 5);
    }
}

#[cfg(test)]
mod track_bits_conversion_tests {
    #[test]
    fn native_mask_and_single_track_assertion() {
        use std::cell::Cell;
        for (bits, track, failed) in [
            (0, 0, true),
            (255, 0, true),
            (128, 0, false),
            (65, 0, false),
            (2, 1, false),
            (3, 0, true),
        ] {
            let failures = Cell::new(0);
            assert_eq!(
                super::track_bits_to_track(bits, |ok| failures.set(failures.get() + u8::from(!ok))),
                track
            );
            assert_eq!(failures.get(), u8::from(failed));
        }
    }
}

/// `SetTrackBitIterator`: each set `Track` of a `TrackBits`, in ascending order.
/// The bits are copied on construction, as the original ensemble does.
pub fn set_track_bit_iterator(
    bits: crate::track_type::TrackBits,
) -> impl Iterator<Item = crate::track_type::Track> {
    crate::bitmath_func::SetBitIterator::new(u64::from(bits.0)).map(crate::track_type::Track)
}

/// `SetTrackdirBitIterator`: each set `Trackdir` of a `TrackdirBits`, in ascending order.
/// The bits are copied on construction, as the original ensemble does.
pub fn set_trackdir_bit_iterator(
    bits: crate::track_type::TrackdirBits,
) -> impl Iterator<Item = crate::track_type::Trackdir> {
    crate::bitmath_func::SetBitIterator::new(u64::from(bits.0)).map(crate::track_type::Trackdir)
}
