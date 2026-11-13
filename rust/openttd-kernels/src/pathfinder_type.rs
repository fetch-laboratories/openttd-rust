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

/// This penalty is the equivalent of "infinite", which means that paths that
/// get this penalty will be chosen, but only if there is no other route
/// without it. Be careful with not applying this penalty too often, or the
/// total path cost might overflow.
pub const YAPF_INFINITE_PENALTY: i32 = 1000 * YAPF_TILE_LENGTH;

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

/// [`FindDepotData`] as it crosses to the native train controller: the tile as
/// its base integer and `reverse` as a 0-or-1 byte.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FindDepotDataRecord {
    /// [`FindDepotData::tile`].
    pub tile: u32,
    /// [`FindDepotData::best_length`].
    pub best_length: u32,
    /// [`FindDepotData::reverse`].
    pub reverse: u8,
}
impl From<FindDepotData> for FindDepotDataRecord {
    fn from(data: FindDepotData) -> Self {
        Self {
            tile: data.tile.0,
            best_length: data.best_length,
            reverse: u8::from(data.reverse),
        }
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDFindDepotData" ["probe"] => FindDepotDataRecord {
        tile,
        best_length,
        reverse,
    };
}
