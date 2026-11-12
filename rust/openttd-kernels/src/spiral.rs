/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete scalar spiral traversal; map storage stays in C++.

use crate::direction_type::{
    DIAGDIR_BEGIN, DIAGDIR_END, DiagDirection, Direction, INVALID_DIAGDIR,
};
use crate::map_func::{tile_index_diff_c_by_diag_dir, tile_index_diff_c_by_dir};
use crate::map_type::TileIndexDiffC;

/// The canonical native assertion, for the map helpers' preconditions.
fn native_check(ok: bool) {
    crate::services_assert::check(ok);
}

/// Copyable traversal state, with all original unsigned arithmetic at 32 bits.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SpiralState {
    /// Shell limit; constant after initialization.
    pub max_radius: u32,
    /// Inner extents in NE, SE, SW, NW direction order.
    pub extent: [u32; 4],
    /// Current shell, before the end-state comparison.
    pub cur_radius: u32,
    /// Remaining movements in the current direction.
    pub position: u32,
    /// Tile x coordinate; outside positions preserve unsigned wrapping.
    pub x: u32,
    /// Tile y coordinate; outside positions preserve unsigned wrapping.
    pub y: u32,
    /// Direction 0..3, or 255 for the initial odd-diameter tile.
    pub direction: u8,
}

const _: () = {
    assert!(std::mem::size_of::<SpiralState>() == 40);
    assert!(std::mem::align_of::<SpiralState>() == 4);
    assert!(std::mem::offset_of!(SpiralState, direction) == 36);
};

impl SpiralState {
    /// End is exactly the radius limit with a non-invalid direction.
    #[must_use]
    pub fn is_end(self) -> bool {
        self.cur_radius == self.max_radius && self.direction != INVALID_DIAGDIR
    }

    /// `SpiralTileIterator::InitPosition`: initialise `position` after `direction` was changed.
    fn init_position(&mut self) {
        self.position = self.extent[usize::from(self.direction)]
            .wrapping_add(self.cur_radius.wrapping_mul(2))
            .wrapping_add(1);
    }

    /// Add a `TileIndexDiffC` to the unsigned coordinates.
    fn step(&mut self, diff: TileIndexDiffC) {
        self.x = self.x.wrapping_add_signed(i32::from(diff.x));
        self.y = self.y.wrapping_add_signed(i32::from(diff.y));
    }
    /// One step by `TileIndexDiffCByDir(DIR_W)`.
    fn west(&mut self) {
        self.step(tile_index_diff_c_by_dir(Direction::DIR_W, native_check));
    }

    /// `SpiralTileIterator::Increment`: advance the internal state to the next
    /// potential tile. The tile may be outside the map though.
    fn increment(&mut self) {
        if self.is_end() {
            crate::services_assert::check(false);
        }
        /* Special value for first tile in areas with odd diameter */
        if self.direction == INVALID_DIAGDIR {
            self.west();
            self.direction = DIAGDIR_BEGIN;
            self.init_position();
            return;
        }
        /* Step to the next 'neighbour' in the circular line */
        self.step(tile_index_diff_c_by_diag_dir(
            DiagDirection(self.direction),
            native_check,
        ));
        self.position = self.position.wrapping_sub(1);
        /* Corner reached, switch direction */
        if self.position > 0 {
            return;
        }
        // Valid directions are 0..3 before this increment.
        self.direction += 1;
        /* Jump to next circle */
        if self.direction == DIAGDIR_END {
            self.west();
            self.cur_radius = self.cur_radius.wrapping_add(1);
            self.direction = DIAGDIR_BEGIN;
        }
        self.init_position();
    }

    /// `SpiralTileIterator::SkipOutsideMap`: advance the internal state until it
    /// reaches a valid tile or the end.
    fn skip_outside(&mut self, size_x: u32, size_y: u32) {
        while !self.is_end() && (self.x >= size_x || self.y >= size_y) {
            self.increment();
        }
    }

    /// Advance a non-ended state, clipping against the current map dimensions.
    #[must_use]
    pub fn advance(mut self, size_x: u32, size_y: u32) -> Self {
        self.increment();
        self.skip_outside(size_x, size_y);
        self
    }
}

/// `SpiralTileIterator(center, diameter)`, from the center's coordinates and the
/// live map dimensions.
pub fn square(x: u32, y: u32, diameter: u32, size_x: u32, size_y: u32) -> SpiralState {
    if diameter == 0 {
        crate::services_assert::check(false);
    }
    let odd = diameter % 2 == 1;
    let mut state = SpiralState {
        max_radius: diameter / 2,
        extent: [u32::from(odd); 4],
        cur_radius: 0,
        position: 0,
        // Odd diameters start at the center (see increment); even ones at the
        // west corner of the center 2x2 rect.
        x: if odd { x } else { x.wrapping_add(1) },
        y,
        direction: if odd { INVALID_DIAGDIR } else { DIAGDIR_BEGIN },
    };
    if !odd {
        state.init_position();
    }
    state.skip_outside(size_x, size_y);
    state
}

/// `SpiralTileIterator(start_north, radius, w, h)`, from the north tile's
/// coordinates and the live map dimensions.
pub fn hole(
    x: u32,
    y: u32,
    radius: u32,
    width: u32,
    height: u32,
    size_x: u32,
    size_y: u32,
) -> SpiralState {
    if radius == 0 {
        crate::services_assert::check(false);
    }
    let mut state = SpiralState {
        max_radius: radius,
        extent: [width, height, width, height],
        cur_radius: 0,
        position: 0,
        /* first tile is the west corner */
        x: x.wrapping_add(width).wrapping_add(1),
        y,
        direction: DIAGDIR_BEGIN,
    };
    state.init_position();
    state.skip_outside(size_x, size_y);
    state
}

/// Whether two traversals are at the same tile.
pub fn equal(left: SpiralState, right: SpiralState) -> bool {
    left.x == right.x && left.y == right.y
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRustSpiralState" ["probe"] => SpiralState {
        max_radius,
        extent,
        cur_radius,
        position,
        x,
        y,
        direction,
    };
}
#[cfg(test)]
mod evidence_tests;
