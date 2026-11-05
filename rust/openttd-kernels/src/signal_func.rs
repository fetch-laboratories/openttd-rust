/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Helpers of `signal_func.h`.
/// `SignalAlongTrackdir` (`_signal_along_trackdir`, `table/signal_data.h`).
pub fn signal_along_trackdir(trackdir: u8) -> u8 {
    [
        0x8, 0x8, 0x8, 0x2, 0x4, 0x1, 0, 0, 0x4, 0x4, 0x4, 0x1, 0x8, 0x2,
    ][usize::from(trackdir)]
}
/// `SignalOnTrack` on the native valid-track domain.
pub const fn signal_on_track(track: u8) -> u8 {
    [0x0c, 0x0c, 0x0c, 0x03, 0x0c, 0x03][track as usize]
}
