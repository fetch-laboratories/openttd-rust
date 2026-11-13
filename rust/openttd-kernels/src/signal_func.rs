/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Helpers of `signal_func.h`, with the tables of `table/signal_data.h`.
use crate::track_type::{Track, Trackdir};

/// `_signal_along_trackdir`: maps a trackdir to the bit that stores its status in
/// the map arrays, in the direction along with the trackdir. The original declares
/// it with `TRACKDIR_END` entries, so the two reversing road trackdirs read the
/// zero-initialized tail.
const SIGNAL_ALONG_TRACKDIR: [u8; Trackdir::TRACKDIR_END.0 as usize] = [
    0x8, 0x8, 0x8, 0x2, 0x4, 0x1, 0, 0, 0x4, 0x4, 0x4, 0x1, 0x8, 0x2, 0, 0,
];
/// `_signal_on_track`: maps a track to the bits that store the status of the two
/// signals that can be present on the given track.
const SIGNAL_ON_TRACK: [u8; Track::TRACK_END.0 as usize] = [0xC, 0xC, 0xC, 0x3, 0xC, 0x3];

/// Maps a trackdir to the bit that stores its status in the map arrays, in the
/// direction along with the trackdir (`SignalAlongTrackdir`).
pub fn signal_along_trackdir(trackdir: Trackdir) -> u8 {
    SIGNAL_ALONG_TRACKDIR[usize::from(trackdir.0)]
}
/// Maps a Track to the bits that store the status of the two signals that can
/// be present on the given track (`SignalOnTrack`).
pub const fn signal_on_track(track: Track) -> u8 {
    SIGNAL_ON_TRACK[track.0 as usize]
}

/// Temporary raw adapter of [`signal_along_trackdir`] for `rail_map.rs` until its
/// pass.
pub fn signal_along_trackdir_raw(trackdir: u8) -> u8 {
    signal_along_trackdir(Trackdir(trackdir))
}
/// Temporary raw adapter of [`signal_on_track`] for the files no pass has
/// finished yet.
pub const fn signal_on_track_raw(track: u8) -> u8 {
    signal_on_track(Track(track))
}

/// State of the signal segment (`signal_func.h`), retaining every raw byte.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SigSegState(pub u8);
impl SigSegState {
    /// Free and has no pre-signal exits or at least one green exit.
    pub const SIGSEG_FREE: Self = Self(0);
    /// Occupied by a train.
    pub const SIGSEG_FULL: Self = Self(1);
    /// Segment is a PBS segment.
    pub const SIGSEG_PBS: Self = Self(2);
}

#[cfg(test)]
mod tests {
    use super::signal_along_trackdir;
    use crate::track_type::Trackdir;

    /// `table/signal_data.h` gives `_signal_along_trackdir` `TRACKDIR_END` entries
    /// and initializes fourteen, so the reversing road trackdirs read zero.
    #[test]
    fn reversing_road_trackdirs_have_no_signal_bit() {
        for trackdir in [
            Trackdir::TRACKDIR_RVREV_NE,
            Trackdir::TRACKDIR_RVREV_SE,
            Trackdir::TRACKDIR_RVREV_SW,
            Trackdir::TRACKDIR_RVREV_NW,
        ] {
            assert_eq!(signal_along_trackdir(trackdir), 0);
        }
        assert_eq!(signal_along_trackdir(Trackdir::TRACKDIR_LEFT_N), 0x8);
        assert_eq!(signal_along_trackdir(Trackdir::TRACKDIR_RIGHT_N), 0x2);
    }
}
