/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared helpers from `pathfinder/yapf/yapf_common.hpp`.
use crate::map_func::Map;
use crate::pathfinder_type::{YAPF_TILE_CORNER_LENGTH, YAPF_TILE_LENGTH};
use crate::tile_type::TileIndex;
use crate::track_func::trackdir_to_exitdir;
use crate::track_type::Trackdir;

/// `OctileDistanceCost`: calculates the octile distance cost between a starting
/// tile / trackdir and a destination tile.
#[inline]
pub(crate) fn octile_distance_cost(
    map: Map,
    start_tile: TileIndex,
    start_td: Trackdir,
    destination_tile: TileIndex,
) -> i32 {
    const DG_DIR_TO_X_OFFS: [i32; 4] = [-1, 0, 1, 0];
    const DG_DIR_TO_Y_OFFS: [i32; 4] = [0, 1, 0, -1];

    let exitdir = usize::from(trackdir_to_exitdir(start_td.0));

    // `2 * TileX(..)` is unsigned; the sum converts back to `int`.
    let x1 = (2 * map.tile_x(start_tile))
        .wrapping_add_signed(DG_DIR_TO_X_OFFS[exitdir])
        .cast_signed();
    let y1 = (2 * map.tile_y(start_tile))
        .wrapping_add_signed(DG_DIR_TO_Y_OFFS[exitdir])
        .cast_signed();
    let x2 = (2 * map.tile_x(destination_tile)).cast_signed();
    let y2 = (2 * map.tile_y(destination_tile)).cast_signed();
    let dx = (x1 - x2).abs();
    let dy = (y1 - y2).abs();
    let dmin = dx.min(dy);
    let dxy = (dx - dy).abs();

    dmin * YAPF_TILE_CORNER_LENGTH + (dxy - 1) * (YAPF_TILE_LENGTH / 2)
}

#[cfg(test)]
mod tests {
    use super::octile_distance_cost;
    use crate::map_func::Map;
    use crate::tile_type::TileIndex;
    use crate::track_type::Trackdir;

    #[test]
    fn octile_distance_cost_follows_the_original_formula_per_exit_direction() {
        // A 64 by 64 map; the start (10, 20) and destination (13, 26).
        let map = Map::from_log_x(6);
        let start = TileIndex(10 + 20 * 64);
        let destination = TileIndex(13 + 26 * 64);
        // (trackdir, its exit direction's x1, y1): x2 = 26, y2 = 52.
        // NE: x1 19, y1 40 -> dx 7, dy 12: 7 * 71 + 4 * 50 = 697.
        // SE: x1 20, y1 41 -> dx 6, dy 11: 6 * 71 + 4 * 50 = 626.
        // SW: x1 21, y1 40 -> dx 5, dy 12: 5 * 71 + 6 * 50 = 655.
        // NW: x1 20, y1 39 -> dx 6, dy 13: 6 * 71 + 6 * 50 = 726.
        for (td, expected) in [
            (Trackdir::TRACKDIR_X_NE, 697),
            (Trackdir::TRACKDIR_Y_SE, 626),
            (Trackdir::TRACKDIR_X_SW, 655),
            (Trackdir::TRACKDIR_Y_NW, 726),
            // A curve's estimate starts from its exit direction too.
            (Trackdir::TRACKDIR_UPPER_E, 697),
        ] {
            assert_eq!(octile_distance_cost(map, start, td, destination), expected);
        }
    }
}
