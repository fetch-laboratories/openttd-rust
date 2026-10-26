/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete scalar spiral traversal; map storage stays in C++.

use crate::direction_type::{DIAGDIR_BEGIN, DIAGDIR_END, INVALID_DIAGDIR};

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

    fn init_position(&mut self) {
        self.position = self.extent[usize::from(self.direction)]
            .wrapping_add(self.cur_radius.wrapping_mul(2))
            .wrapping_add(1);
    }

    fn west(&mut self) {
        self.x = self.x.wrapping_add(1);
        self.y = self.y.wrapping_sub(1);
    }

    fn increment(&mut self) {
        assert!(!self.is_end());
        if self.direction == INVALID_DIAGDIR {
            self.west();
            self.direction = DIAGDIR_BEGIN;
            self.init_position();
            return;
        }
        match self.direction {
            0 => self.x = self.x.wrapping_sub(1),
            1 => self.y = self.y.wrapping_add(1),
            2 => self.x = self.x.wrapping_add(1),
            3 => self.y = self.y.wrapping_sub(1),
            _ => unreachable!(),
        }
        self.position = self.position.wrapping_sub(1);
        if self.position > 0 {
            return;
        }
        self.direction += 1; // Valid directions are 0..3 before this increment.
        if self.direction == DIAGDIR_END {
            self.west();
            self.cur_radius = self.cur_radius.wrapping_add(1);
            self.direction = DIAGDIR_BEGIN;
        }
        self.init_position();
    }

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

pub fn square(x: u32, y: u32, diameter: u32, size_x: u32, size_y: u32) -> SpiralState {
    assert!(diameter > 0);
    let odd = diameter % 2 == 1;
    let mut state = SpiralState {
        max_radius: diameter / 2,
        extent: [u32::from(odd); 4],
        cur_radius: 0,
        position: 0,
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

pub fn hole(
    x: u32,
    y: u32,
    radius: u32,
    width: u32,
    height: u32,
    size_x: u32,
    size_y: u32,
) -> SpiralState {
    assert!(radius > 0);
    let mut state = SpiralState {
        max_radius: radius,
        extent: [width, height, width, height],
        cur_radius: 0,
        position: 0,
        x: x.wrapping_add(width).wrapping_add(1),
        y,
        direction: DIAGDIR_BEGIN,
    };
    state.init_position();
    state.skip_outside(size_x, size_y);
    state
}

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
