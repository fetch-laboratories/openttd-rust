/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared `pathfinder/pathfinder_func.h` road track helper.
use crate::direction_type::INVALID_DIAGDIR;
use crate::services::Services;
use crate::track_func::track_status_to_trackdir_bits;
use crate::track_type::{
    TRACKDIR_BIT_X_NE, TRACKDIR_BIT_X_SW, TRACKDIR_BIT_Y_NW, TRACKDIR_BIT_Y_SE,
};
use crate::transport_type::TRANSPORT_ROAD;

pub(crate) fn road_trackdirs(shared: &Services, tile: u32, tram: bool) -> u16 {
    let mut bits = track_status_to_trackdir_bits(crate::movement::track_status(
        shared,
        tile,
        TRANSPORT_ROAD,
        u32::from(tram),
        INVALID_DIAGDIR,
    ));
    if tram && bits == 0 && (shared.is_normal_road)(tile) {
        bits = match (shared.normal_road_bits)(tile, 1).0 {
            crate::road_type::ROAD_NE | crate::road_type::ROAD_SW => {
                TRACKDIR_BIT_X_NE | TRACKDIR_BIT_X_SW
            }
            crate::road_type::ROAD_NW | crate::road_type::ROAD_SE => {
                TRACKDIR_BIT_Y_NW | TRACKDIR_BIT_Y_SE
            }
            _ => bits,
        };
    }
    bits
}
/// # Safety
/// The immutable shared table and map remain live and serialized for this call.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_trackdirs(
    shared: *const Services,
    tile: u32,
    tram: crate::services::RoadTramType,
) -> u16 {
    road_trackdirs(unsafe { &*shared }, tile, tram.0)
}
