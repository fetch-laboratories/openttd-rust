/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Reservation masking from `CFollowTrackT` in `follow_track.hpp`.
use crate::pbs::Pbs;
use crate::track_func::{
    track_bits_to_trackdir_bits, track_to_track_bits, track_to_trackdir_bits,
    trackdir_bits_to_track_bits, tracks_overlap,
};
use crate::train_reservation::Follow;

/// `CFollowTrackT::EC_RESERVED`.
const EC_RESERVED: u8 = 5;

/// `MaskReservedTracks` for a rail follower with `DoTrackMasking()` enabled.
/// The caller invokes it only after native Follow succeeds. All accesses are
/// call-local scalars and typed services; no world borrow spans a service.
#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
pub(crate) fn mask_reserved_tracks(pbs: Pbs<'_>, ft: &mut Follow) -> bool {
    if ft.station != 0 {
        let diff = pbs.map().offs_by_diag_dir(ft.exitdir);
        let mut tile = ft
            .new_tile
            .wrapping_sub(diff.wrapping_mul(ft.skipped as u32));
        while tile != ft.new_tile {
            if (pbs.0.station_reserved)(tile) {
                ft.dirs = 0;
                ft.error = EC_RESERVED;
                return false;
            }
            tile = tile.wrapping_add(diff);
        }
    }
    let reserved = pbs.reserved(ft.new_tile);
    ft.dirs &= !track_bits_to_trackdir_bits(reserved);
    let mut tracks = trackdir_bits_to_track_bits(ft.dirs);
    while tracks != 0 {
        let track = tracks.trailing_zeros() as u8;
        tracks &= tracks.wrapping_sub(1);
        if tracks_overlap(reserved | track_to_track_bits(track)) {
            ft.dirs &= !track_to_trackdir_bits(track);
        }
    }
    if ft.dirs == 0 {
        ft.error = EC_RESERVED;
        return false;
    }
    true
}

/// Default unchanged-body probe for transient masking outputs absent from saves.
/// # Safety
/// Native world/services and the call-local follower output remain live, and
/// Follow's original preconditions hold. No pointer is retained; panics abort.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_follow_mask(
    services: *const crate::pbs::PbsServices,
    ft: *mut Follow,
) -> bool {
    mask_reserved_tracks(Pbs(unsafe { &*services }), unsafe { &mut *ft })
}
