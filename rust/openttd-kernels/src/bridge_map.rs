/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Bridge height helpers from `bridge_map.h`/`bridge_map.cpp`.
#![allow(unsafe_code)]
use crate::direction_func::diag_dir_to_axis;
use crate::services::{Foundation, Services};
/// `GetBridgeHeight`. Each original operation remains a named shared callee.
pub(crate) fn get_bridge_height(tile: u32, s: &Services) -> i32 {
    let slope_z = (s.tile_slope_z)(tile);
    let foundation = crate::bridge::get_bridge_foundation(
        slope_z.slope.0,
        diag_dir_to_axis((s.tunnel_bridge_direction)(tile), |ok| {
            crate::movement::native_assert(s, ok);
        }),
    );
    slope_z
        .z
        .wrapping_add(1)
        .wrapping_add((s.apply_foundation)(Foundation(foundation), slope_z.slope).z)
}
/// `GetBridgePixelHeight`.
pub(crate) fn get_bridge_pixel_height(tile: u32, s: &Services) -> i32 {
    get_bridge_height(tile, s).wrapping_mul(crate::tile_type::TILE_HEIGHT.cast_signed())
}
/// Thin facade for native rendering/map consumers.
/// # Safety
/// The table is immutable, live and serialized by the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_bridge_height(s: *const Services, tile: u32) -> i32 {
    // SAFETY: canonical native table; no world borrow is formed.
    get_bridge_height(tile, unsafe { &*s })
}

impl crate::map_func::World<'_> {
    pub(crate) fn bridge(&self, t: u32) -> bool {
        self.check(self.kind(t) == crate::tile_type::MP_TUNNELBRIDGE);
        self.byte(self.types.m5(t)) & 128 != 0
    }
}
