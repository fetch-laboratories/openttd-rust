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

/// `IsValidTrackdirForRoadVehicle`: checks if a Trackdir is valid for road vehicles.
/// Use this in an `assert()`.
#[must_use]
pub const fn is_valid_trackdir_for_road_vehicle(trackdir: crate::track_type::Trackdir) -> bool {
    trackdir.0 < crate::track_type::Trackdir::TRACKDIR_END.0
}

/// `IsReversingRoadTrackdir`, under its native valid-road-trackdir precondition.
pub fn is_reversing_road_trackdir(dir: u8, check: impl Fn(bool) + Copy) -> bool {
    if !is_valid_trackdir_for_road_vehicle(crate::track_type::Trackdir(dir)) {
        check(false);
    }
    (dir & 7) >= 6
}

/// `IsStraightRoadTrackdir`: checks whether the given trackdir is a straight road.
pub fn is_straight_road_trackdir(
    dir: crate::track_type::Trackdir,
    check: impl Fn(bool) + Copy,
) -> bool {
    if !is_valid_trackdir_for_road_vehicle(dir) {
        check(false);
    }
    (dir.0 & 0x06) == 0
}

/// `TrackStatusToRedSignals`: returns the red-signal-information of a `TrackStatus`.
///
/// Note: The result may contain red signals for non-present tracks.
#[allow(
    clippy::cast_possible_truncation,
    reason = "TrackStatusToRedSignals' (TrackdirBits) cast of the masked status"
)]
pub const fn track_status_to_red_signals(
    ts: crate::track_type::TrackStatus,
) -> crate::track_type::TrackdirBits {
    crate::track_type::TrackdirBits(((ts >> 16) & crate::track_type::TRACKDIR_BIT_MASK) as u16)
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

/// `TrackdirToExitdir` with the original's `assert(IsValidTrackdirForRoadVehicle(trackdir))`
/// through the native assertion policy `check`: maps a trackdir to the direction in which
/// the train leaves the tile.
pub fn trackdir_to_exitdir_asserted(
    trackdir: crate::track_type::Trackdir,
    check: impl Fn(bool),
) -> crate::direction_type::DiagDirection {
    check(is_valid_trackdir_for_road_vehicle(trackdir));
    crate::direction_type::DiagDirection(trackdir_to_exitdir(trackdir.0))
}

/// `DiagdirReachesTrackdirs`.
pub fn diagdir_reaches_trackdirs(dir: u8) -> u16 {
    [0x1009, 0x16, 0x520, 0x2a00][usize::from(dir)]
}

/// `_track_crosses_trackdirs`: maps a track to all trackdirs that make 90 deg turns with it.
const TRACK_CROSSES_TRACKDIRS: [u16; 6] = [0x202, 0x101, 0x3030, 0x3030, 0xc0c, 0xc0c];

/// `TrackdirCrossesTrackdirs`.
pub fn trackdir_crosses_trackdirs(td: u8) -> u16 {
    TRACK_CROSSES_TRACKDIRS[usize::from(trackdir_to_track(td))]
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
    if !is_valid_trackdir_for_road_vehicle(crate::track_type::Trackdir(dir)) {
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

/// `TrackCrossesTracks`: maps a track to all tracks that make 90 deg turns with it. For the
/// diagonal directions these are the complement of the direction, for the straight
/// directions these are the two vertical or horizontal tracks, depend on the given direction.
pub fn track_crosses_tracks(
    track: crate::track_type::Track,
    check: impl Fn(bool) + Copy,
) -> crate::track_type::TrackBits {
    use crate::track_type::TrackBits;
    /// `_track_crosses_tracks`.
    const TRACK_CROSSES_TRACKS: [TrackBits; crate::track_type::Track::TRACK_END.0 as usize] = [
        TrackBits::TRACK_BIT_Y,    // TRACK_X
        TrackBits::TRACK_BIT_X,    // TRACK_Y
        TrackBits::TRACK_BIT_VERT, // TRACK_UPPER
        TrackBits::TRACK_BIT_VERT, // TRACK_LOWER
        TrackBits::TRACK_BIT_HORZ, // TRACK_LEFT
        TrackBits::TRACK_BIT_HORZ, // TRACK_RIGHT
    ];
    if !is_valid_track(track.0) {
        check(false);
    }
    TRACK_CROSSES_TRACKS[usize::from(track.0)]
}

/// `TrackdirToTrack` with the original's `assert(IsValidTrackdir(trackdir))` through the
/// native assertion policy `check`: returns the track which is used in the trackdir.
pub fn trackdir_to_track_asserted(
    trackdir: crate::track_type::Trackdir,
    check: impl Fn(bool),
) -> crate::track_type::Track {
    check(is_valid_trackdir(trackdir.0));
    crate::track_type::Track(trackdir_to_track(trackdir.0))
}

/// `ReverseTrackdir` with the original's `assert(IsValidTrackdirForRoadVehicle(trackdir))`:
/// returns the reverse trackdir, the same track with the other direction on it.
pub fn reverse_trackdir_asserted(
    trackdir: crate::track_type::Trackdir,
    check: impl Fn(bool),
) -> crate::track_type::Trackdir {
    check(is_valid_trackdir_for_road_vehicle(trackdir));
    crate::track_type::Trackdir(reverse_trackdir(trackdir.0))
}

/// `FindFirstTrackdir` with the original's `assert((trackdirs & ~TRACKDIR_BIT_MASK) ==
/// TRACKDIR_BIT_NONE)`: returns the first trackdir of the bits, or `INVALID_TRACKDIR` on
/// `TRACKDIR_BIT_NONE`. The bits must not be `INVALID_TRACKDIR_BIT`.
pub fn find_first_trackdir_asserted(
    trackdirs: crate::track_type::TrackdirBits,
    check: impl Fn(bool),
) -> crate::track_type::Trackdir {
    use crate::track_type::TrackdirBits;
    check(trackdirs & !TrackdirBits::TRACKDIR_BIT_MASK == TrackdirBits::TRACKDIR_BIT_NONE);
    crate::track_type::Trackdir(find_first_trackdir(trackdirs.0))
}

/// `DiagdirReachesTrackdirs` with the original's `assert(IsValidDiagDirection(diagdir))`:
/// returns all trackdirs that can be reached when entering a tile from a given (diagonal)
/// direction. This will obviously include 90 degree turns, since no information is
/// available about the exact angle of entering.
pub fn diagdir_reaches_trackdirs_asserted(
    diagdir: crate::direction_type::DiagDirection,
    check: impl Fn(bool),
) -> crate::track_type::TrackdirBits {
    check(crate::direction_func::is_valid_diag_direction(diagdir));
    crate::track_type::TrackdirBits(diagdir_reaches_trackdirs(diagdir.0))
}

/// `DiagdirReachesTracks`, through the asserting `DiagdirReachesTrackdirs`: returns all
/// tracks that can be reached when entering a tile from a given (diagonal) direction. This
/// will obviously include 90 degree turns, since no information is available about the
/// exact angle of entering.
pub fn diagdir_reaches_tracks_asserted(
    diagdir: crate::direction_type::DiagDirection,
    check: impl Fn(bool),
) -> crate::track_type::TrackBits {
    crate::track_type::TrackBits(trackdir_bits_to_track_bits(
        diagdir_reaches_trackdirs_asserted(diagdir, check).0,
    ))
}

/// `TrackdirCrossesTrackdirs` with the original's `assert(IsValidTrackdir(trackdir))` and its
/// asserting `TrackdirToTrack`: maps a trackdir to all trackdirs that make 90 deg turns with
/// it. For the diagonal tracks this returns the track direction bits of the other axis in
/// both directions, which cannot be joined by the given track direction. For the straight
/// tracks this returns all possible 90 deg turns either on the current tile (which no train
/// can joined) or on the bordering tiles.
pub fn trackdir_crosses_trackdirs_asserted(
    trackdir: crate::track_type::Trackdir,
    check: impl Fn(bool) + Copy,
) -> crate::track_type::TrackdirBits {
    check(is_valid_trackdir(trackdir.0));
    crate::track_type::TrackdirBits(
        TRACK_CROSSES_TRACKDIRS[usize::from(trackdir_to_track_asserted(trackdir, check).0)],
    )
}

/// `TrackEnterdirToTrackdir`: maps a track and an (4-way) dir to the trackdir that represents
/// the track with the entry in the given direction.
///
/// For the diagonal tracks the return value is clear, its either the matching track direction
/// or `INVALID_TRACKDIR`. For the straight tracks this returns the track direction which
/// results if you follow the `DiagDirection` and then turn by 45 deg left or right on the next
/// tile. The new direction on the new track will be the returning `Trackdir` value. If the
/// parameters makes no sense like the track `TRACK_UPPER` and the direction `DIAGDIR_NE`
/// (target track cannot be reached) this function returns `INVALID_TRACKDIR`.
pub fn track_enterdir_to_trackdir(
    track: crate::track_type::Track,
    diagdir: crate::direction_type::DiagDirection,
    check: impl Fn(bool) + Copy,
) -> crate::track_type::Trackdir {
    use crate::track_type::Trackdir;
    const I: Trackdir = Trackdir::INVALID_TRACKDIR;
    /// `_track_enterdir_to_trackdir`.
    const TRACK_ENTERDIR_TO_TRACKDIR: [[Trackdir;
        crate::direction_type::DiagDirection::DIAGDIR_END.0 as usize];
        crate::track_type::Track::TRACK_END.0 as usize] = [
        [Trackdir::TRACKDIR_X_NE, I, Trackdir::TRACKDIR_X_SW, I],
        [I, Trackdir::TRACKDIR_Y_SE, I, Trackdir::TRACKDIR_Y_NW],
        [I, Trackdir::TRACKDIR_UPPER_E, Trackdir::TRACKDIR_UPPER_W, I],
        [Trackdir::TRACKDIR_LOWER_E, I, I, Trackdir::TRACKDIR_LOWER_W],
        [Trackdir::TRACKDIR_LEFT_N, Trackdir::TRACKDIR_LEFT_S, I, I],
        [I, I, Trackdir::TRACKDIR_RIGHT_S, Trackdir::TRACKDIR_RIGHT_N],
    ];
    check(is_valid_track(track.0));
    check(crate::direction_func::is_valid_diag_direction(diagdir));
    TRACK_ENTERDIR_TO_TRACKDIR[usize::from(track.0)][usize::from(diagdir.0)]
}

/// `TrackdirReachesTrackdirs`: maps a trackdir to the trackdirs that can be reached from it
/// (ie, when entering the next tile). This will include 90 degree turns!
pub fn trackdir_reaches_trackdirs(
    trackdir: crate::track_type::Trackdir,
    check: impl Fn(bool) + Copy,
) -> crate::track_type::TrackdirBits {
    check(is_valid_trackdir(trackdir.0));
    /* Note that there is no direct table for this function (there used to be),
     * but it uses two simpler tables to achieve the result */
    crate::track_type::TrackdirBits(diagdir_reaches_trackdirs(
        trackdir_to_exitdir_asserted(trackdir, check).0,
    ))
}

/// `TrackDirectionToTrackdir`: maps a track and a full (8-way) direction to the trackdir that
/// represents the track running in the given direction.
pub fn track_direction_to_trackdir(
    track: crate::track_type::Track,
    dir: crate::direction_type::Direction,
    check: impl Fn(bool) + Copy,
) -> crate::track_type::Trackdir {
    use crate::track_type::Trackdir;
    const I: Trackdir = Trackdir::INVALID_TRACKDIR;
    /// `_track_direction_to_trackdir`.
    const TRACK_DIRECTION_TO_TRACKDIR: [[Trackdir;
        crate::direction_type::Direction::DIR_END.0 as usize];
        crate::track_type::Track::TRACK_END.0 as usize] = [
        [
            I,
            Trackdir::TRACKDIR_X_NE,
            I,
            I,
            I,
            Trackdir::TRACKDIR_X_SW,
            I,
            I,
        ],
        [
            I,
            I,
            I,
            Trackdir::TRACKDIR_Y_SE,
            I,
            I,
            I,
            Trackdir::TRACKDIR_Y_NW,
        ],
        [
            I,
            I,
            Trackdir::TRACKDIR_UPPER_E,
            I,
            I,
            I,
            Trackdir::TRACKDIR_UPPER_W,
            I,
        ],
        [
            I,
            I,
            Trackdir::TRACKDIR_LOWER_E,
            I,
            I,
            I,
            Trackdir::TRACKDIR_LOWER_W,
            I,
        ],
        [
            Trackdir::TRACKDIR_LEFT_N,
            I,
            I,
            I,
            Trackdir::TRACKDIR_LEFT_S,
            I,
            I,
            I,
        ],
        [
            Trackdir::TRACKDIR_RIGHT_N,
            I,
            I,
            I,
            Trackdir::TRACKDIR_RIGHT_S,
            I,
            I,
            I,
        ],
    ];
    if !is_valid_track(track.0) {
        check(false);
    }
    if !crate::direction_func::is_valid_direction(dir) {
        check(false);
    }
    TRACK_DIRECTION_TO_TRACKDIR[usize::from(track.0)][usize::from(dir.0)]
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
    if !crate::bitmath_func::has_bit(direction.0, 0)
        && track != STATE_DIR_TABLE[usize::from(diagdir.0)]
    {
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

/// `TrackCrossesTracks` and `TrackDirectionToTrackdir` against `table/track_data.h`, with the
/// native assertion recorded for an invalid track or direction.
#[cfg(test)]
mod track_table_tests {
    use crate::direction_type::Direction;
    use crate::track_type::{Track, TrackBits, Trackdir};
    use std::cell::Cell;

    /// `TrackdirToExitdir` reads `_trackdir_to_exitdir` (`table/track_data.h`) for every
    /// trackdir below `TRACKDIR_END`, and asserts `IsValidTrackdirForRoadVehicle` first:
    /// `INVALID_TRACKDIR` fails the assertion before the table read.
    #[test]
    fn trackdir_to_exitdir_asserted_follows_the_table_and_asserts_a_valid_trackdir() {
        use crate::direction_type::DiagDirection;
        // DIAGDIR_NE, SE, NE, SE, SW, SE, NE, NE, SW, NW, NW, SW, NW, NE, and the two
        // zero-filled entries.
        let expected: [u8; 16] = [0, 1, 0, 1, 2, 1, 0, 0, 2, 3, 3, 2, 3, 0, 0, 0];
        let failures = Cell::new(0);
        let check = |ok: bool| failures.set(failures.get() + u32::from(!ok));
        for (td, dir) in (0u8..).zip(expected) {
            assert_eq!(
                super::trackdir_to_exitdir_asserted(Trackdir(td), check),
                DiagDirection(dir)
            );
        }
        assert_eq!(failures.get(), 0);
        let failed = Cell::new(false);
        let record = |ok: bool| failed.set(failed.get() || !ok);
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            super::trackdir_to_exitdir_asserted(Trackdir::INVALID_TRACKDIR, |ok| {
                record(ok);
                assert!(ok);
            })
        }));
        assert!(failed.get());
    }

    #[test]
    fn track_crosses_tracks_follows_the_original_table() {
        let expected = [
            TrackBits(2),  // TRACK_X: TRACK_BIT_Y
            TrackBits(1),  // TRACK_Y: TRACK_BIT_X
            TrackBits(48), // TRACK_UPPER: TRACK_BIT_VERT
            TrackBits(48), // TRACK_LOWER: TRACK_BIT_VERT
            TrackBits(12), // TRACK_LEFT: TRACK_BIT_HORZ
            TrackBits(12), // TRACK_RIGHT: TRACK_BIT_HORZ
        ];
        let failures = Cell::new(0);
        let check = |ok: bool| failures.set(failures.get() + u32::from(!ok));
        for (track, bits) in (0u8..).zip(expected) {
            assert_eq!(super::track_crosses_tracks(Track(track), check), bits);
        }
        assert_eq!(failures.get(), 0);
        let failed = Cell::new(false);
        let record = |ok: bool| failed.set(failed.get() || !ok);
        // The assertion is reported before the table read, which the invalid index would
        // overrun; the recording check stops there.
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            super::track_crosses_tracks(Track(6), |ok| {
                record(ok);
                assert!(ok);
            })
        }));
        assert!(failed.get());
    }

    #[test]
    fn track_direction_to_trackdir_follows_the_original_table() {
        const I: u8 = 0xFF;
        // `_track_direction_to_trackdir[TRACK_END][DIR_END]`, rows by track, columns
        // DIR_N, DIR_NE, DIR_E, DIR_SE, DIR_S, DIR_SW, DIR_W, DIR_NW.
        let expected: [[u8; 8]; 6] = [
            [I, 0, I, I, I, 8, I, I],
            [I, I, I, 1, I, I, I, 9],
            [I, I, 2, I, I, I, 10, I],
            [I, I, 3, I, I, I, 11, I],
            [12, I, I, I, 4, I, I, I],
            [13, I, I, I, 5, I, I, I],
        ];
        let failures = Cell::new(0);
        let check = |ok: bool| failures.set(failures.get() + u32::from(!ok));
        for (track, row) in (0u8..).zip(expected.iter()) {
            for (dir, &td) in (0u8..).zip(row.iter()) {
                assert_eq!(
                    super::track_direction_to_trackdir(Track(track), Direction(dir), check),
                    Trackdir(td),
                    "track {track}, direction {dir}"
                );
            }
        }
        assert_eq!(failures.get(), 0);
        let failed = Cell::new(0);
        let record = |ok: bool| {
            failed.set(failed.get() + u32::from(!ok));
            assert!(ok);
        };
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            super::track_direction_to_trackdir(Track(0), Direction(8), record)
        }));
        assert_eq!(failed.get(), 1, "the direction assertion");
    }
}

/// The asserting `track_func.h` helpers against the original's tables over their whole
/// domain, and each assertion on an invalid input: a check that stops at the failed
/// assertion shows it precedes the table read the assertion guards.
#[cfg(test)]
mod asserting_helper_tests {
    use crate::direction_type::DiagDirection;
    use crate::track_type::{Track, TrackBits, Trackdir, TrackdirBits};
    use std::cell::Cell;

    /// `INVALID_TRACKDIR` and the reversing road trackdirs fail `IsValidTrackdir`.
    fn valid_trackdirs() -> impl Iterator<Item = Trackdir> {
        [0, 1, 2, 3, 4, 5, 8, 9, 10, 11, 12, 13]
            .into_iter()
            .map(Trackdir)
    }
    /// A check that counts failed assertions.
    fn counting(failures: &Cell<u32>) -> impl Fn(bool) + Copy + '_ {
        |ok| failures.set(failures.get() + u32::from(!ok))
    }
    /// Run `f` with a check that stops at its first failed assertion, and return how
    /// many assertions failed: one when `f` asserts before anything else can fail.
    fn stopping_failures(f: impl FnOnce(&dyn Fn(bool))) -> u32 {
        let failures = Cell::new(0);
        let check = |ok: bool| {
            failures.set(failures.get() + u32::from(!ok));
            assert!(ok, "the failed native assertion");
        };
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&check)));
        failures.get()
    }

    #[test]
    fn track_enterdir_to_trackdir_follows_the_original_table() {
        const I: u8 = 0xFF;
        // `_track_enterdir_to_trackdir[TRACK_END][DIAGDIR_END]`, rows by track, columns
        // DIAGDIR_NE, DIAGDIR_SE, DIAGDIR_SW, DIAGDIR_NW.
        let expected: [[u8; 4]; 6] = [
            [0, I, 8, I],
            [I, 1, I, 9],
            [I, 2, 10, I],
            [3, I, I, 11],
            [12, 4, I, I],
            [I, I, 5, 13],
        ];
        let failures = Cell::new(0);
        for (track, row) in (0u8..).zip(expected.iter()) {
            for (dir, &td) in (0u8..).zip(row.iter()) {
                assert_eq!(
                    super::track_enterdir_to_trackdir(
                        Track(track),
                        DiagDirection(dir),
                        counting(&failures)
                    ),
                    Trackdir(td),
                    "track {track}, direction {dir}"
                );
            }
        }
        assert_eq!(failures.get(), 0);
        let invalid_track = stopping_failures(|check| {
            super::track_enterdir_to_trackdir(Track::INVALID_TRACK, DiagDirection(0), check);
        });
        assert_eq!(invalid_track, 1, "the track assertion");
        let invalid_dir = stopping_failures(|check| {
            super::track_enterdir_to_trackdir(
                Track::TRACK_X,
                DiagDirection::INVALID_DIAGDIR,
                check,
            );
        });
        assert_eq!(invalid_dir, 1, "the direction assertion");
    }

    #[test]
    fn trackdir_reaches_trackdirs_follows_the_exit_direction_table() {
        // `_exitdir_reaches_trackdirs[_trackdir_to_exitdir[trackdir]]`: the exit directions
        // NE, SE, NE, SE, SW, SE and SW, NW, NW, SW, NW, NE of the valid trackdirs, and
        // `X_NE | LOWER_E | LEFT_N`, `Y_SE | LEFT_S | UPPER_E`, `X_SW | UPPER_W | RIGHT_S`,
        // `Y_NW | RIGHT_N | LOWER_W` for those directions.
        let reaches = [0x1009_u16, 0x0016, 0x0520, 0x2a00];
        let exitdirs = [0, 1, 0, 1, 2, 1, 2, 3, 3, 2, 3, 0];
        let failures = Cell::new(0);
        for (td, exitdir) in valid_trackdirs().zip(exitdirs) {
            assert_eq!(
                super::trackdir_reaches_trackdirs(td, counting(&failures)),
                TrackdirBits(reaches[exitdir])
            );
        }
        assert_eq!(failures.get(), 0);
        // `TRACKDIR_RVREV_NE` fails `IsValidTrackdir` but passes `TrackdirToExitdir`'s road
        // assertion; `INVALID_TRACKDIR` fails both, and the first stops the lookup.
        let rvrev = Cell::new(0);
        super::trackdir_reaches_trackdirs(Trackdir::TRACKDIR_RVREV_NE, counting(&rvrev));
        assert_eq!(rvrev.get(), 1);
        let invalid = stopping_failures(|check| {
            super::trackdir_reaches_trackdirs(Trackdir::INVALID_TRACKDIR, check);
        });
        assert_eq!(invalid, 1);
    }

    #[test]
    fn asserting_trackdir_conversions_follow_their_raw_forms() {
        let failures = Cell::new(0);
        for td in valid_trackdirs() {
            assert_eq!(
                super::trackdir_to_track_asserted(td, counting(&failures)),
                Track(td.0 & 7)
            );
            assert_eq!(
                super::reverse_trackdir_asserted(td, counting(&failures)),
                Trackdir(td.0 ^ 8)
            );
            // `_track_crosses_trackdirs[TrackdirToTrack(trackdir)]`.
            assert_eq!(
                super::trackdir_crosses_trackdirs_asserted(td, counting(&failures)),
                TrackdirBits([0x202, 0x101, 0x3030, 0x3030, 0x0c0c, 0x0c0c][usize::from(td.0 & 7)])
            );
        }
        // Every valid set of trackdir bits, including none.
        for bits in (0..=u16::MAX).filter(|bits| bits & !0x3f3f == 0) {
            let first = (0..16).find(|&td| bits & (1 << td) != 0).unwrap_or(0xff);
            assert_eq!(
                super::find_first_trackdir_asserted(TrackdirBits(bits), counting(&failures)),
                Trackdir(first)
            );
        }
        assert_eq!(failures.get(), 0);

        // Each conversion asserts its precondition: `TrackdirToTrack` and
        // `TrackdirCrossesTrackdirs` a valid rail trackdir, `ReverseTrackdir` a trackdir
        // below `TRACKDIR_END`, `FindFirstTrackdir` no bits outside the mask.
        let rvrev = Cell::new(0);
        super::trackdir_to_track_asserted(Trackdir::TRACKDIR_RVREV_NE, counting(&rvrev));
        assert_eq!(rvrev.get(), 1);
        let reversed = Cell::new(0);
        super::reverse_trackdir_asserted(Trackdir::TRACKDIR_RVREV_NE, counting(&reversed));
        assert_eq!(reversed.get(), 0);
        super::reverse_trackdir_asserted(Trackdir::INVALID_TRACKDIR, counting(&reversed));
        assert_eq!(reversed.get(), 1);
        let crossing = stopping_failures(|check| {
            super::trackdir_crosses_trackdirs_asserted(Trackdir::INVALID_TRACKDIR, check);
        });
        assert_eq!(crossing, 1);
        let masked = Cell::new(0);
        assert_eq!(
            super::find_first_trackdir_asserted(TrackdirBits(0x0040), counting(&masked)),
            Trackdir(6)
        );
        super::find_first_trackdir_asserted(TrackdirBits::INVALID_TRACKDIR_BIT, counting(&masked));
        assert_eq!(masked.get(), 2);
    }

    #[test]
    fn asserting_diagdir_reaches_follow_the_original_table() {
        // `_exitdir_reaches_trackdirs`, and its tracks.
        let trackdirs = [0x1009_u16, 0x0016, 0x0520, 0x2a00];
        let tracks = [0x19_u8, 0x16, 0x25, 0x2a];
        let failures = Cell::new(0);
        for dir in 0..4 {
            assert_eq!(
                super::diagdir_reaches_trackdirs_asserted(DiagDirection(dir), counting(&failures)),
                TrackdirBits(trackdirs[usize::from(dir)])
            );
            assert_eq!(
                super::diagdir_reaches_tracks_asserted(DiagDirection(dir), counting(&failures)),
                TrackBits(tracks[usize::from(dir)])
            );
        }
        assert_eq!(failures.get(), 0);
        let trackdir_lookup = stopping_failures(|check| {
            super::diagdir_reaches_trackdirs_asserted(DiagDirection::INVALID_DIAGDIR, check);
        });
        assert_eq!(trackdir_lookup, 1);
        let track_lookup = stopping_failures(|check| {
            super::diagdir_reaches_tracks_asserted(DiagDirection::INVALID_DIAGDIR, check);
        });
        assert_eq!(track_lookup, 1);
    }
}
