/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `yapf_costbase.hpp`'s `CYapfCostBase`: the cost helpers the YAPF searches
//! share.
use crate::direction_func::diag_dir_to_axis;
use crate::direction_type::DiagDirection;
use crate::map_func::World;
use crate::tile_type::{TileIndex, TileType};
use crate::track_func::{is_diagonal_trackdir, is_uphill_trackdir, trackdir_to_exitdir};
use crate::track_type::Trackdir;

/// `CYapfCostBase::stSlopeCost`: does the given track direction on the given
/// tile yield an uphill penalty?
#[inline]
pub(crate) fn st_slope_cost(world: &World<'_>, tile: TileIndex, td: Trackdir) -> bool {
    if is_diagonal_trackdir(td.0) {
        if world.kind(tile) == TileType::MP_TUNNELBRIDGE && world.bridge(tile.0) {
            /* it is bridge ramp, check if we are entering the bridge */
            if world.tunnel_bridge_dir(tile.0) != trackdir_to_exitdir(td.0) {
                return false; // no, we are leaving it, no penalty
            }
            /* we are entering the bridge */
            let tile_slope = world.tile_slope(tile);
            let axis = diag_dir_to_axis(DiagDirection(world.tunnel_bridge_dir(tile.0)), |ok| {
                world.check(ok);
            });
            return !crate::bridge::has_bridge_flat_ramp(world.shared, tile_slope.0, axis.0);
        }
        /* not bridge ramp */
        if world.kind(tile) == TileType::MP_TUNNELBRIDGE && world.tunnel(tile.0) {
            return false; // tunnel entry/exit doesn't slope
        }
        let tile_slope = world.tile_slope(tile);
        return is_uphill_trackdir(tile_slope, td.0, |ok| world.check(ok)); // slopes uphill => apply penalty
    }
    false
}
