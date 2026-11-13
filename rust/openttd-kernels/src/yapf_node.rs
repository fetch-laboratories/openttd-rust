/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Node in the pathfinder's graph (`yapf_node.hpp`'s `CYapfNodeT`), with the
//! fields its users observe. Intrusive hash links belong to the native hash
//! table, and choice flags are unobserved by no-cache river search.
use crate::direction_type::DiagDirection;
use crate::tile_type::TileIndex;
use crate::track_func::trackdir_to_exitdir;
use crate::track_type::Trackdir;
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy)]
pub(crate) struct Node<K> {
    pub(crate) key: K,
    pub(crate) parent: Option<usize>,
    pub(crate) cost: i32,
    pub(crate) estimate: i32,
}
impl<K> Node<K> {
    /// `CYapfNodeT::Set`, for the fields this node keeps.
    pub(crate) fn set(parent: Option<usize>, key: K) -> Self {
        Self {
            key,
            parent,
            cost: 0,
            estimate: 0,
        }
    }
    /// `CYapfNodeT::operator<`.
    pub(crate) fn less(&self, other: &Self) -> bool {
        self.estimate < other.estimate
    }
}
impl Node<TileIndex> {
    /// `CYapfNodeT::GetTile`. River keys elide their constant `INVALID_TRACKDIR`.
    pub(crate) fn get_tile(self) -> TileIndex {
        self.key
    }
}

/// `CYapfNodeKeyExitDir`: yapf node key that evaluates hash from (and compares)
/// tile & exit dir.
#[derive(Clone, Copy)]
pub(crate) struct CYapfNodeKeyExitDir {
    pub(crate) tile: TileIndex,
    pub(crate) td: Trackdir,
    pub(crate) exitdir: DiagDirection,
}
impl CYapfNodeKeyExitDir {
    /// `CYapfNodeKeyExitDir::Set`.
    #[inline]
    pub(crate) fn set(tile: TileIndex, td: Trackdir) -> Self {
        Self {
            tile,
            td,
            exitdir: if td == Trackdir::INVALID_TRACKDIR {
                DiagDirection::INVALID_DIAGDIR
            } else {
                DiagDirection(trackdir_to_exitdir(td.0))
            },
        }
    }
}
/// `CYapfNodeKeyExitDir::operator==`: tile and exit direction.
impl PartialEq for CYapfNodeKeyExitDir {
    fn eq(&self, other: &Self) -> bool {
        self.tile == other.tile && self.exitdir == other.exitdir
    }
}
impl Eq for CYapfNodeKeyExitDir {}
/// `CYapfNodeKeyExitDir::CalcHash` hashes the fields `operator==` compares.
impl Hash for CYapfNodeKeyExitDir {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u32(self.tile.0);
        state.write_u8(self.exitdir.0);
    }
}
