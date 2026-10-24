/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `animated_tile_map.h` state values; canonical packed map storage stays native.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct AnimatedTileState(pub u8);
impl AnimatedTileState {
    pub const NONE: Self = Self(0);
    pub const DELETED: Self = Self(1);
    pub const ANIMATED: Self = Self(3);
}
crate::abi::layouts! {
    fn abi_layout;
    "AnimatedTileState" ["probe"] => AnimatedTileState {};
}

/// Canonical `GetAnimationFrame` service, shared with the animation owner.
pub(crate) fn animation_frame(services: &crate::animated_tile::Services, tile: u32) -> u8 {
    // SAFETY: InitializeAnimatedTiles installs this immutable native service
    // before map use. The callback reads no Rust owner or borrowed world view.
    unsafe { ((*services.industry_tiles).frame)(tile) }
}
