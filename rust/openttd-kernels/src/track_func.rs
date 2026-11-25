/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! The inline track helpers of `track_func.h`, with their tables from `table/track_data.h`.
//!
//! Each helper takes and returns the original's types. A helper whose original asserts, or
//! calls a helper that asserts, takes the caller's native assertion policy `check` as its
//! last parameter and reports each failed `assert` there, at the original's position; with
//! assertions off the original's computation follows. The `_raw` adapters at the end keep
//! integer signatures for files no pass has finished.
use crate::bitmath_func::{clr_bit8, clr_bit16, find_first_bit, has_bit, kill_first_bit};
use crate::direction_func::{
    change_diag_dir, dir_to_diag_dir, is_valid_axis, is_valid_diag_direction, is_valid_direction,
};
use crate::direction_type::{Axis, DiagDirDiff, DiagDirection, Direction};
use crate::services_assert::native_check;
use crate::slope_func::{is_valid_corner, remove_halftile_slope};
use crate::slope_type::{Corner, Slope};
use crate::track_type::{Track, TrackBits, TrackStatus, Trackdir, TrackdirBits};

/// `SetTrackBitIterator`: each set `Track` of a `TrackBits`, in ascending order.
/// The bits are copied on construction, as the original ensemble does.
pub fn set_track_bit_iterator(bits: TrackBits) -> impl Iterator<Item = Track> {
    crate::bitmath_func::SetBitIterator::new(u64::from(bits.0)).map(Track)
}

/// `SetTrackdirBitIterator`: each set `Trackdir` of a `TrackdirBits`, in ascending order.
/// The bits are copied on construction, as the original ensemble does.
pub fn set_trackdir_bit_iterator(bits: TrackdirBits) -> impl Iterator<Item = Trackdir> {
    crate::bitmath_func::SetBitIterator::new(u64::from(bits.0)).map(Trackdir)
}

/// `IsValidTrack`: checks if a Track is valid. Use this in an `assert()`.
pub const fn is_valid_track(track: Track) -> bool {
    track.0 < Track::TRACK_END.0
}

/// `IsValidTrackdirForRoadVehicle`: checks if a Trackdir is valid for road vehicles.
/// Use this in an `assert()`.
pub const fn is_valid_trackdir_for_road_vehicle(trackdir: Trackdir) -> bool {
    trackdir.0 < Trackdir::TRACKDIR_END.0
}

/// `IsValidTrackdir`: checks if a Trackdir is valid for non-road vehicles.
/// Use this in an `assert()`.
pub const fn is_valid_trackdir(trackdir: Trackdir) -> bool {
    trackdir.0 != Trackdir::INVALID_TRACKDIR.0
        && (1_u32.wrapping_shl(trackdir.0 as u32) & TrackdirBits::TRACKDIR_BIT_MASK.0 as u32)
            != TrackdirBits::TRACKDIR_BIT_NONE.0 as u32
}

/// `AxisToTrack`: convert an Axis to the corresponding Track. `AXIS_X` -> `TRACK_X`,
/// `AXIS_Y` -> `TRACK_Y`. Uses the fact that they share the same internal encoding.
pub fn axis_to_track(a: Axis, check: impl Fn(bool) + Copy) -> Track {
    if !is_valid_axis(a) {
        check(false);
    }
    Track(a.0)
}

/// `TrackToTrackBits`: maps a Track to the corresponding `TrackBits` value.
#[allow(
    clippy::cast_possible_truncation,
    reason = "TrackToTrackBits' (TrackBits) cast of the promoted shift"
)]
pub fn track_to_track_bits(track: Track, check: impl Fn(bool) + Copy) -> TrackBits {
    if !is_valid_track(track) {
        check(false);
    }
    TrackBits(1_u32.wrapping_shl(u32::from(track.0)) as u8)
}

/// `AxisToTrackBits`: maps an Axis to the corresponding `TrackBits` value.
pub fn axis_to_track_bits(a: Axis, check: impl Fn(bool) + Copy) -> TrackBits {
    track_to_track_bits(axis_to_track(a, check), check)
}

/// `CornerToTrackBits`: returns a single horizontal/vertical trackbit that is in a specific
/// tile corner.
pub fn corner_to_track_bits(corner: Corner, check: impl Fn(bool) + Copy) -> TrackBits {
    /// `_corner_to_trackbits`.
    const CORNER_TO_TRACKBITS: [TrackBits; 4] = [
        TrackBits::TRACK_BIT_LEFT,
        TrackBits::TRACK_BIT_LOWER,
        TrackBits::TRACK_BIT_RIGHT,
        TrackBits::TRACK_BIT_UPPER,
    ];
    if !is_valid_corner(corner) {
        check(false);
    }
    CORNER_TO_TRACKBITS[usize::from(corner.0)]
}

/// `TrackdirToTrackdirBits`: maps a Trackdir to the corresponding `TrackdirBits` value.
#[allow(
    clippy::cast_possible_truncation,
    reason = "TrackdirToTrackdirBits' (TrackdirBits) cast of the promoted shift"
)]
pub fn trackdir_to_trackdir_bits(trackdir: Trackdir, check: impl Fn(bool) + Copy) -> TrackdirBits {
    if !is_valid_trackdir(trackdir) {
        check(false);
    }
    TrackdirBits(1_u32.wrapping_shl(u32::from(trackdir.0)) as u16)
}

/// `RemoveFirstTrack`: removes the first Track from `TrackBits` and returns it.
///
/// This function searches for the first bit in the `TrackBits`, remove this bit from the
/// parameter and returns the found bit as Track value. It returns `INVALID_TRACK` if the
/// parameter was `TRACK_BIT_NONE`. This is basically used in while-loops to get up to 6
/// possible tracks on a tile until the parameter becomes `TRACK_BIT_NONE`.
pub fn remove_first_track(tracks: &mut TrackBits, check: impl Fn(bool) + Copy) -> Track {
    if *tracks != TrackBits::TRACK_BIT_NONE {
        if *tracks & !TrackBits::TRACK_BIT_MASK != TrackBits::TRACK_BIT_NONE {
            check(false);
        }
        let first = Track(find_first_bit(u64::from(tracks.0)));
        clr_bit8(&mut tracks.0, first.0);
        return first;
    }
    Track::INVALID_TRACK
}

/// `RemoveFirstTrackdir`: removes the first Trackdir from `TrackdirBits` and returns it.
///
/// This function searches for the first bit in the `TrackdirBits` parameter, remove this
/// bit from the parameter and returns the found bit as Trackdir value. It returns
/// `INVALID_TRACKDIR` if the trackdirs is `TRACKDIR_BIT_NONE` or `INVALID_TRACKDIR_BIT`.
/// This is basically used in a while-loop to get all track-directions step by step until
/// the value reaches `TRACKDIR_BIT_NONE`.
pub fn remove_first_trackdir(
    trackdirs: &mut TrackdirBits,
    check: impl Fn(bool) + Copy,
) -> Trackdir {
    if *trackdirs != TrackdirBits::TRACKDIR_BIT_NONE
        && *trackdirs != TrackdirBits::INVALID_TRACKDIR_BIT
    {
        if *trackdirs & !TrackdirBits::TRACKDIR_BIT_MASK != TrackdirBits::TRACKDIR_BIT_NONE {
            check(false);
        }
        let first = Trackdir(find_first_bit(u64::from(trackdirs.0)));
        clr_bit16(&mut trackdirs.0, first.0);
        return first;
    }
    Trackdir::INVALID_TRACKDIR
}

/// `FindFirstTrack`: returns the first Track from `TrackBits` or `INVALID_TRACK`.
///
/// This function returns the first Track found in the `TrackBits` value as Track-value.
/// It returns `INVALID_TRACK` if the parameter is `TRACK_BIT_NONE`.
pub const fn find_first_track(tracks: TrackBits) -> Track {
    if tracks.0 == TrackBits::TRACK_BIT_NONE.0 {
        Track::INVALID_TRACK
    } else {
        Track(find_first_bit(tracks.0 as u64))
    }
}

/// `TrackBitsToTrack`: converts `TrackBits` to Track.
///
/// This function converts a `TrackBits` value to a Track value. As it is not possible to
/// convert two or more tracks to one track the parameter must contain only one track.
pub fn track_bits_to_track(tracks: TrackBits, check: impl Fn(bool) + Copy) -> Track {
    if !(tracks != TrackBits::TRACK_BIT_NONE
        && kill_first_bit((tracks & TrackBits::TRACK_BIT_MASK).0) == TrackBits::TRACK_BIT_NONE.0)
    {
        check(false);
    }
    Track(find_first_bit(u64::from(
        (tracks & TrackBits::TRACK_BIT_MASK).0,
    )))
}

/// `FindFirstTrackdir`: returns the first Trackdir from `TrackdirBits` or `INVALID_TRACKDIR`.
///
/// This function returns the first Trackdir in the given `TrackdirBits` value or
/// `INVALID_TRACKDIR` if the value is `TRACKDIR_BIT_NONE`. The `TrackdirBits` must not be
/// `INVALID_TRACKDIR_BIT`.
pub fn find_first_trackdir(trackdirs: TrackdirBits, check: impl Fn(bool) + Copy) -> Trackdir {
    if trackdirs & !TrackdirBits::TRACKDIR_BIT_MASK != TrackdirBits::TRACKDIR_BIT_NONE {
        check(false);
    }
    if trackdirs == TrackdirBits::TRACKDIR_BIT_NONE {
        Trackdir::INVALID_TRACKDIR
    } else {
        Trackdir(find_first_bit(u64::from(trackdirs.0)))
    }
}

/*
 * Functions describing logical relations between Tracks, TrackBits, Trackdirs
 * TrackdirBits, Direction and DiagDirections.
 */

/// `ReverseTrackdir`: maps a trackdir to the reverse trackdir.
///
/// Returns the reverse trackdir of a Trackdir value. The reverse trackdir is the same track
/// with the other direction on it.
pub fn reverse_trackdir(trackdir: Trackdir, check: impl Fn(bool) + Copy) -> Trackdir {
    if !is_valid_trackdir_for_road_vehicle(trackdir) {
        check(false);
    }
    Trackdir(trackdir.0 ^ 8)
}

/// `TrackdirToTrack`: returns the Track that a given Trackdir represents.
///
/// This function filters the Track which is used in the Trackdir value and returns it as a
/// Track value.
pub fn trackdir_to_track(trackdir: Trackdir, check: impl Fn(bool) + Copy) -> Track {
    if !is_valid_trackdir(trackdir) {
        check(false);
    }
    Track(trackdir.0 & 0x7)
}

/// `TrackToTrackdir`: returns a Trackdir for the given Track.
///
/// Since every Track corresponds to two Trackdirs, we choose the one which points between
/// NE and S. Note that the actual implementation is quite futile, but this might change in
/// the future.
pub fn track_to_trackdir(track: Track, check: impl Fn(bool) + Copy) -> Trackdir {
    if !is_valid_track(track) {
        check(false);
    }
    Trackdir(track.0)
}

/// `TrackToTrackdirBits`: returns a `TrackdirBit` mask from a given Track.
///
/// The `TrackdirBit` mask contains the two `TrackdirBits` that correspond with the given
/// Track (one for each direction).
pub fn track_to_trackdir_bits(track: Track, check: impl Fn(bool) + Copy) -> TrackdirBits {
    let td = track_to_trackdir(track, check);
    trackdir_to_trackdir_bits(td, check)
        | trackdir_to_trackdir_bits(reverse_trackdir(td, check), check)
}

/// `TrackdirBitsToTrackBits`: discards all directional information from a `TrackdirBits`
/// value. Any Track which is present in either direction will be present in the result.
#[allow(
    clippy::cast_possible_truncation,
    reason = "TrackdirBitsToTrackBits' (TrackBits) cast of the masked bits"
)]
pub const fn trackdir_bits_to_track_bits(bits: TrackdirBits) -> TrackBits {
    TrackBits(((bits.0 | (bits.0 >> 8)) & TrackBits::TRACK_BIT_MASK.0 as u16) as u8)
}

/// `TrackBitsToTrackdirBits`: converts `TrackBits` to `TrackdirBits` while allowing both
/// directions.
pub const fn track_bits_to_trackdir_bits(bits: TrackBits) -> TrackdirBits {
    TrackdirBits((bits.0 as u16).wrapping_mul(0x101))
}

/// `HasTrack`: checks whether a `TrackBits` has a given Track.
pub fn has_track(tracks: TrackBits, track: Track, check: impl Fn(bool) + Copy) -> bool {
    if !is_valid_track(track) {
        check(false);
    }
    has_bit(tracks.0, track.0)
}

/// `HasTrackdir`: checks whether a `TrackdirBits` has a given Trackdir.
pub fn has_trackdir(
    trackdirs: TrackdirBits,
    trackdir: Trackdir,
    check: impl Fn(bool) + Copy,
) -> bool {
    if !is_valid_trackdir(trackdir) {
        check(false);
    }
    has_bit(trackdirs.0, trackdir.0)
}

/// `TrackStatusToTrackdirBits`: returns the present-trackdir-information of a `TrackStatus`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "TrackStatusToTrackdirBits' (TrackdirBits) cast of the masked status"
)]
pub const fn track_status_to_trackdir_bits(ts: TrackStatus) -> TrackdirBits {
    TrackdirBits((ts & TrackdirBits::TRACKDIR_BIT_MASK.0 as u32) as u16)
}

/// `TrackStatusToTrackBits`: returns the present-track-information of a `TrackStatus`.
pub const fn track_status_to_track_bits(ts: TrackStatus) -> TrackBits {
    trackdir_bits_to_track_bits(track_status_to_trackdir_bits(ts))
}

/// `TrackStatusToRedSignals`: returns the red-signal-information of a `TrackStatus`.
///
/// Note: The result may contain red signals for non-present tracks.
#[allow(
    clippy::cast_possible_truncation,
    reason = "TrackStatusToRedSignals' (TrackdirBits) cast of the masked status"
)]
pub const fn track_status_to_red_signals(ts: TrackStatus) -> TrackdirBits {
    TrackdirBits(((ts >> 16) & TrackdirBits::TRACKDIR_BIT_MASK.0 as u32) as u16)
}

/// `CombineTrackStatus`: builds a `TrackStatus` from the present trackdirs and the red
/// signals.
pub const fn combine_track_status(
    trackdirbits: TrackdirBits,
    red_signals: TrackdirBits,
) -> TrackStatus {
    (trackdirbits.0 as u32) | ((red_signals.0 as u32) << 16)
}

/// `NextTrackdir`: maps a trackdir to the trackdir that you will end up on if you go
/// straight ahead.
///
/// This will be the same trackdir for diagonal trackdirs, but a different (alternating) one
/// for straight trackdirs.
pub fn next_trackdir(trackdir: Trackdir, check: impl Fn(bool) + Copy) -> Trackdir {
    /// `_next_trackdir[TRACKDIR_END]`, whose last two entries the original leaves zero.
    const NEXT_TRACKDIR: [Trackdir; Trackdir::TRACKDIR_END.0 as usize] = [
        Trackdir::TRACKDIR_X_NE,
        Trackdir::TRACKDIR_Y_SE,
        Trackdir::TRACKDIR_LOWER_E,
        Trackdir::TRACKDIR_UPPER_E,
        Trackdir::TRACKDIR_RIGHT_S,
        Trackdir::TRACKDIR_LEFT_S,
        Trackdir::INVALID_TRACKDIR,
        Trackdir::INVALID_TRACKDIR,
        Trackdir::TRACKDIR_X_SW,
        Trackdir::TRACKDIR_Y_NW,
        Trackdir::TRACKDIR_LOWER_W,
        Trackdir::TRACKDIR_UPPER_W,
        Trackdir::TRACKDIR_RIGHT_N,
        Trackdir::TRACKDIR_LEFT_N,
        Trackdir(0),
        Trackdir(0),
    ];
    if !is_valid_trackdir(trackdir) {
        check(false);
    }
    NEXT_TRACKDIR[usize::from(trackdir.0)]
}

/// `TrackCrossesTracks`: maps a track to all tracks that make 90 deg turns with it.
///
/// For the diagonal directions these are the complement of the direction, for the straight
/// directions these are the two vertical or horizontal tracks, depend on the given
/// direction.
pub fn track_crosses_tracks(track: Track, check: impl Fn(bool) + Copy) -> TrackBits {
    /// `_track_crosses_tracks`: maps a track to all tracks that make 90 deg turns with it.
    const TRACK_CROSSES_TRACKS: [TrackBits; Track::TRACK_END.0 as usize] = [
        TrackBits::TRACK_BIT_Y,    // TRACK_X
        TrackBits::TRACK_BIT_X,    // TRACK_Y
        TrackBits::TRACK_BIT_VERT, // TRACK_UPPER
        TrackBits::TRACK_BIT_VERT, // TRACK_LOWER
        TrackBits::TRACK_BIT_HORZ, // TRACK_LEFT
        TrackBits::TRACK_BIT_HORZ, // TRACK_RIGHT
    ];
    if !is_valid_track(track) {
        check(false);
    }
    TRACK_CROSSES_TRACKS[usize::from(track.0)]
}

/// `TrackdirToExitdir`: maps a trackdir to the (4-way) direction the tile is exited when
/// following that trackdir.
///
/// For the diagonal directions these are the same directions. For the straight directions
/// these are the directions from the imagined base-tile to the bordering tile which will be
/// joined if the given straight direction is leaved from the base-tile.
pub fn trackdir_to_exitdir(trackdir: Trackdir, check: impl Fn(bool) + Copy) -> DiagDirection {
    /* Maps a trackdir to the (4-way) direction the tile is exited when following
     * that trackdir. `_trackdir_to_exitdir[TRACKDIR_END]`, whose last two entries the
     * original leaves zero. */
    const TRACKDIR_TO_EXITDIR: [DiagDirection; Trackdir::TRACKDIR_END.0 as usize] = [
        DiagDirection::DIAGDIR_NE,
        DiagDirection::DIAGDIR_SE,
        DiagDirection::DIAGDIR_NE,
        DiagDirection::DIAGDIR_SE,
        DiagDirection::DIAGDIR_SW,
        DiagDirection::DIAGDIR_SE,
        DiagDirection::DIAGDIR_NE,
        DiagDirection::DIAGDIR_NE,
        DiagDirection::DIAGDIR_SW,
        DiagDirection::DIAGDIR_NW,
        DiagDirection::DIAGDIR_NW,
        DiagDirection::DIAGDIR_SW,
        DiagDirection::DIAGDIR_NW,
        DiagDirection::DIAGDIR_NE,
        DiagDirection(0),
        DiagDirection(0),
    ];
    if !is_valid_trackdir_for_road_vehicle(trackdir) {
        check(false);
    }
    TRACKDIR_TO_EXITDIR[usize::from(trackdir.0)]
}

/// `TrackEnterdirToTrackdir`: maps a track and an (4-way) dir to the trackdir that
/// represents the track with the entry in the given direction.
///
/// For the diagonal tracks the return value is clear, its either the matching track
/// direction or `INVALID_TRACKDIR`. For the straight tracks this returns the track
/// direction which results if you follow the `DiagDirection` and then turn by 45 deg left
/// or right on the next tile. The new direction on the new track will be the returning
/// Trackdir value. If the parameters makes no sense like the track `TRACK_UPPER` and the
/// direction `DIAGDIR_NE` (target track cannot be reached) this function returns
/// `INVALID_TRACKDIR`.
pub fn track_enterdir_to_trackdir(
    track: Track,
    diagdir: DiagDirection,
    check: impl Fn(bool) + Copy,
) -> Trackdir {
    const I: Trackdir = Trackdir::INVALID_TRACKDIR;
    /// `_track_enterdir_to_trackdir`.
    const TRACK_ENTERDIR_TO_TRACKDIR: [[Trackdir; DiagDirection::DIAGDIR_END.0 as usize];
        Track::TRACK_END.0 as usize] = [
        [Trackdir::TRACKDIR_X_NE, I, Trackdir::TRACKDIR_X_SW, I],
        [I, Trackdir::TRACKDIR_Y_SE, I, Trackdir::TRACKDIR_Y_NW],
        [I, Trackdir::TRACKDIR_UPPER_E, Trackdir::TRACKDIR_UPPER_W, I],
        [Trackdir::TRACKDIR_LOWER_E, I, I, Trackdir::TRACKDIR_LOWER_W],
        [Trackdir::TRACKDIR_LEFT_N, Trackdir::TRACKDIR_LEFT_S, I, I],
        [I, I, Trackdir::TRACKDIR_RIGHT_S, Trackdir::TRACKDIR_RIGHT_N],
    ];
    if !is_valid_track(track) {
        check(false);
    }
    if !is_valid_diag_direction(diagdir) {
        check(false);
    }
    TRACK_ENTERDIR_TO_TRACKDIR[usize::from(track.0)][usize::from(diagdir.0)]
}

/// `TrackDirectionToTrackdir`: maps a track and a full (8-way) direction to the trackdir
/// that represents the track running in the given direction.
pub fn track_direction_to_trackdir(
    track: Track,
    dir: Direction,
    check: impl Fn(bool) + Copy,
) -> Trackdir {
    const I: Trackdir = Trackdir::INVALID_TRACKDIR;
    /// `_track_direction_to_trackdir`.
    const TRACK_DIRECTION_TO_TRACKDIR: [[Trackdir; Direction::DIR_END.0 as usize];
        Track::TRACK_END.0 as usize] = [
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
    if !is_valid_track(track) {
        check(false);
    }
    if !is_valid_direction(dir) {
        check(false);
    }
    TRACK_DIRECTION_TO_TRACKDIR[usize::from(track.0)][usize::from(dir.0)]
}

/// `DiagDirToDiagTrack`: maps a (4-way) direction to the diagonal track incidating with
/// that diagdir.
pub fn diag_dir_to_diag_track(diagdir: DiagDirection, check: impl Fn(bool) + Copy) -> Track {
    if !is_valid_diag_direction(diagdir) {
        check(false);
    }
    Track(diagdir.0 & 1)
}

/// `DiagDirToDiagTrackBits`: maps a (4-way) direction to the diagonal track bits
/// incidating with that diagdir.
pub fn diag_dir_to_diag_track_bits(
    diagdir: DiagDirection,
    check: impl Fn(bool) + Copy,
) -> TrackBits {
    if !is_valid_diag_direction(diagdir) {
        check(false);
    }
    track_to_track_bits(diag_dir_to_diag_track(diagdir, check), check)
}

/// `DiagDirToDiagTrackdir`: maps a (4-way) direction to the diagonal trackdir that runs in
/// that direction.
pub fn diag_dir_to_diag_trackdir(diagdir: DiagDirection, check: impl Fn(bool) + Copy) -> Trackdir {
    /// `_dir_to_diag_trackdir`.
    const DIR_TO_DIAG_TRACKDIR: [Trackdir; DiagDirection::DIAGDIR_END.0 as usize] = [
        Trackdir::TRACKDIR_X_NE,
        Trackdir::TRACKDIR_Y_SE,
        Trackdir::TRACKDIR_X_SW,
        Trackdir::TRACKDIR_Y_NW,
    ];
    if !is_valid_diag_direction(diagdir) {
        check(false);
    }
    DIR_TO_DIAG_TRACKDIR[usize::from(diagdir.0)]
}

/// `_exitdir_reaches_trackdirs`: maps a diagonal direction to the all trackdirs that are
/// connected to any track entering in this direction (including those making 90 degree
/// turns).
const EXITDIR_REACHES_TRACKDIRS: [TrackdirBits; DiagDirection::DIAGDIR_END.0 as usize] = [
    TrackdirBits(
        TrackdirBits::TRACKDIR_BIT_X_NE.0
            | TrackdirBits::TRACKDIR_BIT_LOWER_E.0
            | TrackdirBits::TRACKDIR_BIT_LEFT_N.0,
    ), // DIAGDIR_NE
    TrackdirBits(
        TrackdirBits::TRACKDIR_BIT_Y_SE.0
            | TrackdirBits::TRACKDIR_BIT_LEFT_S.0
            | TrackdirBits::TRACKDIR_BIT_UPPER_E.0,
    ), // DIAGDIR_SE
    TrackdirBits(
        TrackdirBits::TRACKDIR_BIT_X_SW.0
            | TrackdirBits::TRACKDIR_BIT_UPPER_W.0
            | TrackdirBits::TRACKDIR_BIT_RIGHT_S.0,
    ), // DIAGDIR_SW
    TrackdirBits(
        TrackdirBits::TRACKDIR_BIT_Y_NW.0
            | TrackdirBits::TRACKDIR_BIT_RIGHT_N.0
            | TrackdirBits::TRACKDIR_BIT_LOWER_W.0,
    ), // DIAGDIR_NW
];

/// `DiagdirReachesTrackdirs`: returns all trackdirs that can be reached when entering a
/// tile from a given (diagonal) direction.
///
/// This will obviously include 90 degree turns, since no information is available about the
/// exact angle of entering.
pub fn diagdir_reaches_trackdirs(
    diagdir: DiagDirection,
    check: impl Fn(bool) + Copy,
) -> TrackdirBits {
    if !is_valid_diag_direction(diagdir) {
        check(false);
    }
    EXITDIR_REACHES_TRACKDIRS[usize::from(diagdir.0)]
}

/// `DiagdirReachesTracks`: returns all tracks that can be reached when entering a tile from
/// a given (diagonal) direction.
///
/// This will obviously include 90 degree turns, since no information is available about the
/// exact angle of entering.
pub fn diagdir_reaches_tracks(diagdir: DiagDirection, check: impl Fn(bool) + Copy) -> TrackBits {
    trackdir_bits_to_track_bits(diagdir_reaches_trackdirs(diagdir, check))
}

/// `TrackdirReachesTrackdirs`: maps a trackdir to the trackdirs that can be reached from it
/// (ie, when entering the next tile). This will include 90 degree turns!
pub fn trackdir_reaches_trackdirs(trackdir: Trackdir, check: impl Fn(bool) + Copy) -> TrackdirBits {
    if !is_valid_trackdir(trackdir) {
        check(false);
    }
    /* Note that there is no direct table for this function (there used to be),
     * but it uses two simpler tables to achieve the result */
    EXITDIR_REACHES_TRACKDIRS[usize::from(trackdir_to_exitdir(trackdir, check).0)]
}

/// `TrackdirCrossesTrackdirs`: maps a trackdir to all trackdirs that make 90 deg turns with
/// it.
///
/// For the diagonal tracks this returns the track direction bits of the other axis in both
/// directions, which cannot be joined by the given track direction. For the straight tracks
/// this returns all possible 90 deg turns either on the current tile (which no train can
/// joined) or on the bordering tiles.
pub fn trackdir_crosses_trackdirs(trackdir: Trackdir, check: impl Fn(bool) + Copy) -> TrackdirBits {
    use TrackdirBits as B;
    /// `_track_crosses_trackdirs`: maps a trackdir to all trackdirs that make 90 deg turns
    /// with it.
    const TRACK_CROSSES_TRACKDIRS: [TrackdirBits; Track::TRACK_END.0 as usize] = [
        B(B::TRACKDIR_BIT_Y_SE.0 | B::TRACKDIR_BIT_Y_NW.0), // TRACK_X
        B(B::TRACKDIR_BIT_X_NE.0 | B::TRACKDIR_BIT_X_SW.0), // TRACK_Y
        B(B::TRACKDIR_BIT_RIGHT_N.0
            | B::TRACKDIR_BIT_RIGHT_S.0
            | B::TRACKDIR_BIT_LEFT_N.0
            | B::TRACKDIR_BIT_LEFT_S.0), // TRACK_UPPER
        B(B::TRACKDIR_BIT_RIGHT_N.0
            | B::TRACKDIR_BIT_RIGHT_S.0
            | B::TRACKDIR_BIT_LEFT_N.0
            | B::TRACKDIR_BIT_LEFT_S.0), // TRACK_LOWER
        B(B::TRACKDIR_BIT_UPPER_W.0
            | B::TRACKDIR_BIT_UPPER_E.0
            | B::TRACKDIR_BIT_LOWER_W.0
            | B::TRACKDIR_BIT_LOWER_E.0), // TRACK_LEFT
        B(B::TRACKDIR_BIT_UPPER_W.0
            | B::TRACKDIR_BIT_UPPER_E.0
            | B::TRACKDIR_BIT_LOWER_W.0
            | B::TRACKDIR_BIT_LOWER_E.0), // TRACK_RIGHT
    ];
    if !is_valid_trackdir(trackdir) {
        check(false);
    }
    TRACK_CROSSES_TRACKDIRS[usize::from(trackdir_to_track(trackdir, check).0)]
}

/// `IsDiagonalTrack`: checks if a given Track is diagonal.
pub fn is_diagonal_track(track: Track, check: impl Fn(bool) + Copy) -> bool {
    if !is_valid_track(track) {
        check(false);
    }
    track == Track::TRACK_X || track == Track::TRACK_Y
}

/// `IsDiagonalTrackdir`: checks if a given Trackdir is diagonal.
pub fn is_diagonal_trackdir(trackdir: Trackdir, check: impl Fn(bool) + Copy) -> bool {
    if !is_valid_trackdir(trackdir) {
        check(false);
    }
    is_diagonal_track(trackdir_to_track(trackdir, check), check)
}

/// `TracksOverlap`: checks if the given tracks overlap, ie form a crossing. Basically this
/// means when there is more than one track on the tile, except when there are two parallel
/// tracks.
pub const fn tracks_overlap(bits: TrackBits) -> bool {
    /* With no, or only one track, there is no overlap */
    if bits.0 == TrackBits::TRACK_BIT_NONE.0
        || kill_first_bit(bits.0) == TrackBits::TRACK_BIT_NONE.0
    {
        return false;
    }
    /* We know that there are at least two tracks present. When there are more
     * than 2 tracks, they will surely overlap. When there are two, they will
     * always overlap unless they are lower & upper or right & left. */
    bits.0 != TrackBits::TRACK_BIT_HORZ.0 && bits.0 != TrackBits::TRACK_BIT_VERT.0
}

/// `TrackOverlapsTracks`: check if a given track is contained within or overlaps some other
/// tracks.
pub fn track_overlaps_tracks(tracks: TrackBits, track: Track, check: impl Fn(bool) + Copy) -> bool {
    if has_bit(tracks.0, track.0) {
        return true;
    }
    tracks_overlap(tracks | track_to_track_bits(track, check))
}

/// `IsReversingRoadTrackdir`: checks whether the trackdir means that we are reversing.
pub fn is_reversing_road_trackdir(dir: Trackdir, check: impl Fn(bool) + Copy) -> bool {
    if !is_valid_trackdir_for_road_vehicle(dir) {
        check(false);
    }
    (dir.0 & 0x07) >= 6
}

/// `IsStraightRoadTrackdir`: checks whether the given trackdir is a straight road.
pub fn is_straight_road_trackdir(dir: Trackdir, check: impl Fn(bool) + Copy) -> bool {
    if !is_valid_trackdir_for_road_vehicle(dir) {
        check(false);
    }
    (dir.0 & 0x06) == 0
}

/// `IsUphillTrackdir`: checks whether a trackdir on a specific slope is going uphill.
///
/// Valid for rail and road tracks. Valid for tile-slopes (under foundation) and
/// foundation-slopes (on foundation).
pub fn is_uphill_trackdir(slope: Slope, dir: Trackdir, check: impl Fn(bool) + Copy) -> bool {
    use TrackdirBits as B;
    /// `_uphill_trackdirs`, from `table/track_data.h`.
    const UPHILL_TRACKDIRS: [TrackdirBits; 31] = [
        B::TRACKDIR_BIT_NONE,                               //  0 SLOPE_FLAT
        B(B::TRACKDIR_BIT_X_SW.0 | B::TRACKDIR_BIT_Y_NW.0), //  1 SLOPE_W   -> inclined for diagonal track
        B(B::TRACKDIR_BIT_X_SW.0 | B::TRACKDIR_BIT_Y_SE.0), //  2 SLOPE_S   -> inclined for diagonal track
        B::TRACKDIR_BIT_X_SW,                               //  3 SLOPE_SW
        B(B::TRACKDIR_BIT_X_NE.0 | B::TRACKDIR_BIT_Y_SE.0), //  4 SLOPE_E   -> inclined for diagonal track
        B::TRACKDIR_BIT_NONE,                               //  5 SLOPE_EW
        B::TRACKDIR_BIT_Y_SE,                               //  6 SLOPE_SE
        B::TRACKDIR_BIT_NONE,                               //  7 SLOPE_WSE -> leveled
        B(B::TRACKDIR_BIT_X_NE.0 | B::TRACKDIR_BIT_Y_NW.0), //  8 SLOPE_N   -> inclined for diagonal track
        B::TRACKDIR_BIT_Y_NW,                               //  9 SLOPE_NW
        B::TRACKDIR_BIT_NONE,                               // 10 SLOPE_NS
        B::TRACKDIR_BIT_NONE,                               // 11 SLOPE_NWS -> leveled
        B::TRACKDIR_BIT_X_NE,                               // 12 SLOPE_NE
        B::TRACKDIR_BIT_NONE,                               // 13 SLOPE_ENW -> leveled
        B::TRACKDIR_BIT_NONE,                               // 14 SLOPE_SEN -> leveled
        B::TRACKDIR_BIT_NONE,                               // 15 invalid
        B::TRACKDIR_BIT_NONE,                               // 16 invalid
        B::TRACKDIR_BIT_NONE,                               // 17 invalid
        B::TRACKDIR_BIT_NONE,                               // 18 invalid
        B::TRACKDIR_BIT_NONE,                               // 19 invalid
        B::TRACKDIR_BIT_NONE,                               // 20 invalid
        B::TRACKDIR_BIT_NONE,                               // 21 invalid
        B::TRACKDIR_BIT_NONE,                               // 22 invalid
        B(B::TRACKDIR_BIT_X_SW.0 | B::TRACKDIR_BIT_Y_SE.0), // 23 SLOPE_STEEP_S -> inclined for diagonal track
        B::TRACKDIR_BIT_NONE,                               // 24 invalid
        B::TRACKDIR_BIT_NONE,                               // 25 invalid
        B::TRACKDIR_BIT_NONE,                               // 26 invalid
        B(B::TRACKDIR_BIT_X_SW.0 | B::TRACKDIR_BIT_Y_NW.0), // 27 SLOPE_STEEP_W -> inclined for diagonal track
        B::TRACKDIR_BIT_NONE,                               // 28 invalid
        B(B::TRACKDIR_BIT_X_NE.0 | B::TRACKDIR_BIT_Y_NW.0), // 29 SLOPE_STEEP_N -> inclined for diagonal track
        B(B::TRACKDIR_BIT_X_NE.0 | B::TRACKDIR_BIT_Y_SE.0), // 30 SLOPE_STEEP_E -> inclined for diagonal track
    ];
    if !is_valid_trackdir_for_road_vehicle(dir) {
        check(false);
    }
    has_bit(
        UPHILL_TRACKDIRS[usize::from(remove_halftile_slope(slope).0)].0,
        dir.0,
    )
}

/// `VehicleExitDir`: determine the side in which the vehicle will leave the tile.
pub fn vehicle_exit_dir(
    direction: Direction,
    track: TrackBits,
    check: impl Fn(bool) + Copy,
) -> DiagDirection {
    const STATE_DIR_TABLE: [TrackBits; DiagDirection::DIAGDIR_END.0 as usize] = [
        TrackBits::TRACK_BIT_RIGHT,
        TrackBits::TRACK_BIT_LOWER,
        TrackBits::TRACK_BIT_LEFT,
        TrackBits::TRACK_BIT_UPPER,
    ];

    let mut diagdir = dir_to_diag_dir(direction, check);

    /* Determine the diagonal direction in which we will exit this tile */
    if !has_bit(direction.0, 0) && track != STATE_DIR_TABLE[usize::from(diagdir.0)] {
        diagdir = change_diag_dir(diagdir, DiagDirDiff::DIAGDIRDIFF_90LEFT, check);
    }

    diagdir
}

/* Raw adapters with integer signatures for callers in files no pass has finished. Each
 * wraps its arguments, calls the typed helper above and unwraps its result. */

/// Raw adapter: `DiagDirToDiagTrack`.
pub fn diag_dir_to_diag_track_raw(direction: u8, check: impl Fn(bool) + Copy) -> u8 {
    diag_dir_to_diag_track(DiagDirection(direction), check).0
}
/// Raw adapter: `CornerToTrackBits`.
pub fn corner_to_track_bits_raw(c: u8, check: impl Fn(bool) + Copy) -> u8 {
    corner_to_track_bits(Corner(c), check).0
}
/// Raw adapter: `TracksOverlap`.
pub const fn tracks_overlap_raw(b: u8) -> bool {
    tracks_overlap(TrackBits(b))
}
/// Raw adapter: `TrackToTrackBits`.
pub fn track_to_track_bits_raw(track: u8, check: impl Fn(bool) + Copy) -> u8 {
    track_to_track_bits(Track(track), check).0
}
/// Raw adapter: `AxisToTrackBits`.
pub fn axis_to_track_bits_raw(axis: u8, check: impl Fn(bool) + Copy) -> u8 {
    axis_to_track_bits(Axis(axis), check).0
}
/// Raw adapter: `TrackStatusToTrackBits`.
pub const fn track_status_to_track_bits_raw(status: u32) -> u8 {
    track_status_to_track_bits(status).0
}
/// Raw adapter: `TrackStatusToTrackdirBits`.
pub const fn track_status_to_trackdir_bits_raw(status: u32) -> u16 {
    track_status_to_trackdir_bits(status).0
}
/// Raw adapter: `FindFirstTrack`.
pub const fn find_first_track_raw(tracks: u8) -> u8 {
    find_first_track(TrackBits(tracks)).0
}
/// Raw adapter: `FindFirstTrack` for the ship controller's promoted `TrackBits`, whose
/// values fit the byte the original takes.
#[allow(
    clippy::cast_possible_truncation,
    reason = "the ship controller promotes a TrackBits byte to its uint32 slot"
)]
pub const fn find_first_track_u32_raw(tracks: u32) -> u8 {
    find_first_track(TrackBits(tracks as u8)).0
}
/// Raw adapter: `TrackdirBitsToTrackBits`.
pub const fn trackdir_bits_to_track_bits_raw(bits: u16) -> u8 {
    trackdir_bits_to_track_bits(TrackdirBits(bits)).0
}
/// Raw adapter: `HasTrack`.
pub fn has_track_raw(bits: u8, track: u8, check: impl Fn(bool) + Copy) -> bool {
    has_track(TrackBits(bits), Track(track), check)
}
/// Raw adapter: `IsDiagonalTrackdir`, on the process-wide native assertion policy.
pub fn is_diagonal_trackdir_raw(td: u8) -> bool {
    is_diagonal_trackdir(Trackdir(td), native_check)
}
/// Raw adapter: `ReverseTrackdir`, on the process-wide native assertion policy.
pub fn reverse_trackdir_raw(td: u8) -> u8 {
    reverse_trackdir(Trackdir(td), native_check).0
}
/// Raw adapter: `TrackdirToExitdir`, on the process-wide native assertion policy.
pub fn trackdir_to_exitdir_raw(td: u8) -> u8 {
    trackdir_to_exitdir(Trackdir(td), native_check).0
}
/// Raw adapter: `DiagdirReachesTrackdirs`, on the process-wide native assertion policy.
pub fn diagdir_reaches_trackdirs_raw(dir: u8) -> u16 {
    diagdir_reaches_trackdirs(DiagDirection(dir), native_check).0
}
/// Raw adapter: `DiagdirReachesTracks`, on the process-wide native assertion policy.
pub fn diagdir_reaches_tracks_raw(dir: u8) -> u8 {
    diagdir_reaches_tracks(DiagDirection(dir), native_check).0
}

#[cfg(test)]
mod tests {
    use super::*;
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
    /// many assertions failed: one when `f` asserts before anything else can fail, so
    /// before the table read the assertion guards.
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
    fn native_asserts_off_retains_header_conversions() {
        let failures = Cell::new(0);
        let disabled = |ok: bool| {
            assert!(!ok);
            failures.set(failures.get() + 1);
        };
        assert_eq!(axis_to_track(Axis(2), disabled), Track(2));
        assert_eq!(failures.get(), 1);
        // `AxisToTrackBits` asserts only the axis: the resulting `TRACK_UPPER` is valid.
        assert_eq!(axis_to_track_bits(Axis(2), disabled), TrackBits(4));
        assert_eq!(failures.get(), 2);
        assert_eq!(track_to_track_bits(Track(8), disabled), TrackBits(0));
        assert_eq!(diag_dir_to_diag_track(DiagDirection(4), disabled), Track(0));
        assert!(is_reversing_road_trackdir(Trackdir(22), disabled));
        assert_eq!(failures.get(), 5);
    }

    #[test]
    fn track_bits_to_track_masks_and_asserts_a_single_track() {
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
                track_bits_to_track(TrackBits(bits), counting(&failures)),
                Track(track)
            );
            assert_eq!(failures.get(), u32::from(failed));
        }
    }

    /// The twelve helpers whose original asserts, against tables written from
    /// `track_func.h` and `table/track_data.h` over their whole valid domain.
    #[test]
    fn trackdir_conversions_follow_the_original_over_their_domain() {
        let failures = Cell::new(0);
        for td in valid_trackdirs() {
            // `(Track)(trackdir & 0x7)`.
            assert_eq!(trackdir_to_track(td, counting(&failures)), Track(td.0 & 7));
            // `(TrackdirBits)(1 << trackdir)`.
            assert_eq!(
                trackdir_to_trackdir_bits(td, counting(&failures)),
                TrackdirBits(1 << td.0)
            );
            // `_track_crosses_trackdirs[TrackdirToTrack(trackdir)]`.
            assert_eq!(
                trackdir_crosses_trackdirs(td, counting(&failures)),
                TrackdirBits([0x202, 0x101, 0x3030, 0x3030, 0x0c0c, 0x0c0c][usize::from(td.0 & 7)])
            );
            // `IsDiagonalTrack(TrackdirToTrack(trackdir))`: `TRACK_X` and `TRACK_Y`.
            assert_eq!(is_diagonal_trackdir(td, counting(&failures)), td.0 & 7 < 2);
        }
        // `ReverseTrackdir` and `TrackdirToExitdir` take every trackdir below `TRACKDIR_END`.
        // `_trackdir_to_exitdir`: DIAGDIR_NE, SE, NE, SE, SW, SE, NE, NE, SW, NW, NW, SW, NW,
        // NE, and the two zero-filled entries.
        let exitdirs: [u8; 16] = [0, 1, 0, 1, 2, 1, 0, 0, 2, 3, 3, 2, 3, 0, 0, 0];
        for (td, exitdir) in (0u8..).zip(exitdirs) {
            assert_eq!(
                reverse_trackdir(Trackdir(td), counting(&failures)),
                Trackdir(td ^ 8)
            );
            assert_eq!(
                trackdir_to_exitdir(Trackdir(td), counting(&failures)),
                DiagDirection(exitdir)
            );
        }
        for track in 0..6 {
            assert_eq!(
                track_to_trackdir(Track(track), counting(&failures)),
                Trackdir(track)
            );
            // `TrackdirToTrackdirBits(td) | TrackdirToTrackdirBits(ReverseTrackdir(td))`.
            assert_eq!(
                track_to_trackdir_bits(Track(track), counting(&failures)),
                TrackdirBits(0x101 << track)
            );
            assert_eq!(
                is_diagonal_track(Track(track), counting(&failures)),
                track < 2
            );
        }
        // Every valid set of trackdir bits, including none.
        for bits in (0..=u16::MAX).filter(|bits| bits & !0x3f3f == 0) {
            let first = (0..16).find(|&td| bits & (1 << td) != 0).unwrap_or(0xff);
            assert_eq!(
                find_first_trackdir(TrackdirBits(bits), counting(&failures)),
                Trackdir(first)
            );
        }
        assert_eq!(failures.get(), 0);
    }

    /// The same twelve helpers on an invalid input: each records one failed assertion
    /// before its lookup.
    #[test]
    fn trackdir_conversions_assert_an_invalid_input_first() {
        // The invalid inputs. Each helper that only computes records one failed check and
        // keeps the original's computation; each that indexes a table, or calls a helper
        // that asserts, stops at its own assertion first.
        let one = |f: &dyn Fn(&Cell<u32>)| {
            let failures = Cell::new(0);
            f(&failures);
            failures.get()
        };
        assert_eq!(
            one(&|f| assert_eq!(
                trackdir_to_track(Trackdir::TRACKDIR_RVREV_NE, counting(f)),
                Track(6)
            )),
            1
        );
        assert_eq!(
            one(&|f| assert_eq!(
                reverse_trackdir(Trackdir::INVALID_TRACKDIR, counting(f)),
                Trackdir(0xf7)
            )),
            1
        );
        assert_eq!(
            one(&|f| assert_eq!(track_to_trackdir(Track(6), counting(f)), Trackdir(6))),
            1
        );
        assert_eq!(
            one(&|f| assert_eq!(
                trackdir_to_trackdir_bits(Trackdir::TRACKDIR_RVREV_NE, counting(f)),
                TrackdirBits(0x40)
            )),
            1
        );
        assert_eq!(
            one(&|f| assert!(!is_diagonal_track(Track(6), counting(f)))),
            1
        );
        assert_eq!(
            one(&|f| assert_eq!(
                find_first_trackdir(TrackdirBits::INVALID_TRACKDIR_BIT, counting(f)),
                Trackdir(0)
            )),
            1
        );
        // An isolated road-reversal bit fails the mask assertion and is still found.
        assert_eq!(
            one(&|f| assert_eq!(
                find_first_trackdir(TrackdirBits(0x0040), counting(f)),
                Trackdir(6)
            )),
            1
        );
        assert_eq!(
            stopping_failures(|check| {
                trackdir_to_exitdir(Trackdir::INVALID_TRACKDIR, check);
            }),
            1
        );
        assert_eq!(
            stopping_failures(|check| {
                trackdir_crosses_trackdirs(Trackdir::INVALID_TRACKDIR, check);
            }),
            1
        );
        assert_eq!(
            stopping_failures(|check| {
                track_to_trackdir_bits(Track::INVALID_TRACK, check);
            }),
            1
        );
        assert_eq!(
            stopping_failures(|check| {
                is_diagonal_trackdir(Trackdir::TRACKDIR_RVREV_NE, check);
            }),
            1
        );
        // With assertions off, the nested callees assert too: `IsValidTrackdir`, then
        // `TrackdirToTrack` and `IsDiagonalTrack` on its track 6.
        assert_eq!(
            one(&|f| assert!(!is_diagonal_trackdir(
                Trackdir::TRACKDIR_RVREV_NE,
                counting(f)
            ))),
            3
        );
    }

    #[test]
    fn diagdir_reaches_follow_the_original_table_and_assert_the_direction() {
        // `_exitdir_reaches_trackdirs`, and its tracks.
        let trackdirs = [0x1009_u16, 0x0016, 0x0520, 0x2a00];
        let tracks = [0x19_u8, 0x16, 0x25, 0x2a];
        let failures = Cell::new(0);
        for dir in 0..4 {
            assert_eq!(
                diagdir_reaches_trackdirs(DiagDirection(dir), counting(&failures)),
                TrackdirBits(trackdirs[usize::from(dir)])
            );
            assert_eq!(
                diagdir_reaches_tracks(DiagDirection(dir), counting(&failures)),
                TrackBits(tracks[usize::from(dir)])
            );
        }
        assert_eq!(failures.get(), 0);
        let trackdir_lookup = stopping_failures(|check| {
            diagdir_reaches_trackdirs(DiagDirection::INVALID_DIAGDIR, check);
        });
        assert_eq!(trackdir_lookup, 1);
        let track_lookup = stopping_failures(|check| {
            diagdir_reaches_tracks(DiagDirection::INVALID_DIAGDIR, check);
        });
        assert_eq!(track_lookup, 1);
    }

    /// `TrackOverlapsTracks` tests `HasBit(tracks, track)` first, so a set bit answers before
    /// `TrackToTrackBits` can assert on the track.
    #[test]
    fn track_overlaps_tracks_tests_the_bit_before_converting_the_track() {
        let set = Cell::new(0);
        assert!(track_overlaps_tracks(
            TrackBits(1 << 6),
            Track(6),
            counting(&set)
        ));
        assert_eq!(set.get(), 0);
        let clear = Cell::new(0);
        assert!(!track_overlaps_tracks(
            TrackBits::TRACK_BIT_NONE,
            Track(6),
            counting(&clear)
        ));
        assert_eq!(clear.get(), 1);
        // Valid tracks: contained, crossing, and the parallel pairs that do not overlap.
        let none = Cell::new(0);
        assert!(track_overlaps_tracks(
            TrackBits::TRACK_BIT_X,
            Track::TRACK_X,
            counting(&none)
        ));
        assert!(track_overlaps_tracks(
            TrackBits::TRACK_BIT_X,
            Track::TRACK_Y,
            counting(&none)
        ));
        assert!(!track_overlaps_tracks(
            TrackBits::TRACK_BIT_UPPER,
            Track::TRACK_LOWER,
            counting(&none)
        ));
        assert!(!track_overlaps_tracks(
            TrackBits::TRACK_BIT_LEFT,
            Track::TRACK_RIGHT,
            counting(&none)
        ));
        assert_eq!(none.get(), 0);
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
        for (track, bits) in (0u8..).zip(expected) {
            assert_eq!(
                track_crosses_tracks(Track(track), counting(&failures)),
                bits
            );
        }
        assert_eq!(failures.get(), 0);
        let invalid = stopping_failures(|check| {
            track_crosses_tracks(Track(6), check);
        });
        assert_eq!(invalid, 1);
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
        for (track, row) in (0u8..).zip(expected.iter()) {
            for (dir, &td) in (0u8..).zip(row.iter()) {
                assert_eq!(
                    track_direction_to_trackdir(Track(track), Direction(dir), counting(&failures)),
                    Trackdir(td),
                    "track {track}, direction {dir}"
                );
            }
        }
        assert_eq!(failures.get(), 0);
        let invalid = stopping_failures(|check| {
            track_direction_to_trackdir(Track(0), Direction(8), check);
        });
        assert_eq!(invalid, 1, "the direction assertion");
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
                    track_enterdir_to_trackdir(
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
            track_enterdir_to_trackdir(Track::INVALID_TRACK, DiagDirection(0), check);
        });
        assert_eq!(invalid_track, 1, "the track assertion");
        let invalid_dir = stopping_failures(|check| {
            track_enterdir_to_trackdir(Track::TRACK_X, DiagDirection::INVALID_DIAGDIR, check);
        });
        assert_eq!(invalid_dir, 1, "the direction assertion");
    }

    #[test]
    fn trackdir_reaches_trackdirs_follows_the_exit_direction_table() {
        // `_exitdir_reaches_trackdirs[_trackdir_to_exitdir[trackdir]]`.
        let reaches = [0x1009_u16, 0x0016, 0x0520, 0x2a00];
        let exitdirs = [0, 1, 0, 1, 2, 1, 2, 3, 3, 2, 3, 0];
        let failures = Cell::new(0);
        for (td, exitdir) in valid_trackdirs().zip(exitdirs) {
            assert_eq!(
                trackdir_reaches_trackdirs(td, counting(&failures)),
                TrackdirBits(reaches[exitdir])
            );
        }
        assert_eq!(failures.get(), 0);
        // `TRACKDIR_RVREV_NE` fails `IsValidTrackdir` but passes `TrackdirToExitdir`'s road
        // assertion; `INVALID_TRACKDIR` fails both, and the first stops the lookup.
        let rvrev = Cell::new(0);
        trackdir_reaches_trackdirs(Trackdir::TRACKDIR_RVREV_NE, counting(&rvrev));
        assert_eq!(rvrev.get(), 1);
        let invalid = stopping_failures(|check| {
            trackdir_reaches_trackdirs(Trackdir::INVALID_TRACKDIR, check);
        });
        assert_eq!(invalid, 1);
    }
}
