/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Consumed `tile_map.h` inline helpers over canonical services.
use crate::services::Services;
/// `IsTileOwner`.
pub fn is_tile_owner(s: &Services, tile: u32, owner: u8) -> bool {
    (s.tile_owner)(tile) == owner
}
