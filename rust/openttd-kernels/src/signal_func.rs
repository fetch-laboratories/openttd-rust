/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Helpers of `signal_func.h`.
/// Maps a trackdir to the bit that stores its status in the map arrays,
/// in the direction along with the trackdir (`SignalAlongTrackdir`).
pub fn signal_along_trackdir(trackdir: u8) -> u8 {
    [
        0x8, 0x8, 0x8, 0x2, 0x4, 0x1, 0, 0, 0x4, 0x4, 0x4, 0x1, 0x8, 0x2,
    ][usize::from(trackdir)]
}
/// Maps a track to the bits that store the status of the two signals that can
/// be present on the given track (`SignalOnTrack`).
pub const fn signal_on_track(track: u8) -> u8 {
    [0x0c, 0x0c, 0x0c, 0x03, 0x0c, 0x03][track as usize]
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
