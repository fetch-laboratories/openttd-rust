/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared definitions from `pathfinder/pathfinder_type.h`.
use crate::tile_type::TileIndex;

/// Length (penalty) of one tile with YAPF.
pub const YAPF_TILE_LENGTH: i32 = 100;

/// Length (penalty) of a corner with YAPF.
pub const YAPF_TILE_CORNER_LENGTH: i32 = 71;

/// Maximum segments of road vehicle path cache.
pub const YAPF_ROADVEH_PATH_CACHE_SEGMENTS: i32 = 8;

/// Distance from destination road stops to not cache any further.
pub const YAPF_ROADVEH_PATH_CACHE_DESTINATION_LIMIT: i32 = 8;

/// Helper container to find a depot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FindDepotData {
    /// The tile of the depot.
    pub tile: TileIndex,
    /// The distance towards the depot in penalty, or `UINT_MAX` if not found.
    pub best_length: u32,
    /// True if reversing is necessary for the train to get to this depot.
    pub reverse: bool,
}
impl FindDepotData {
    /// Create an instance of this structure.
    #[must_use]
    pub const fn new(tile: TileIndex, best_length: u32, reverse: bool) -> Self {
        Self {
            tile,
            best_length,
            reverse,
        }
    }
}
impl Default for FindDepotData {
    /// `FindDepotData()`: `INVALID_TILE`, `UINT_MAX` and no reversal.
    fn default() -> Self {
        Self::new(TileIndex::INVALID, u32::MAX, false)
    }
}
