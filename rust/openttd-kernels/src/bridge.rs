/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Bridge helpers declared in `bridge.h`.
#![allow(unsafe_code, missing_docs)]
use crate::services::{Axis, Foundation, Services, Slope};
use crate::slope_func::{
    FOUNDATION_NONE, SLOPE_FLAT, SLOPE_NE, SLOPE_NW, SLOPE_SE, SLOPE_SW, flattening_foundation,
    has_slope_highest_corner, inclined_foundation,
};
pub(crate) const MAX_BRIDGES: u32 = 13;
/// Original long-bridge price factor, including precondition-free small inputs.
pub(crate) fn calc_bridge_len_cost_factor(mut length: i32) -> i32 {
    if length < 2 {
        return length;
    }
    length = length.wrapping_sub(2);
    let mut sum: i32 = 2;
    let mut delta: i32 = 1;
    loop {
        for _ in 0..delta {
            if length == 0 {
                return sum;
            }
            sum = sum.wrapping_add(delta);
            length = length.wrapping_sub(1);
        }
        delta = delta.wrapping_add(1);
    }
}
pub(crate) fn get_bridge_foundation(slope: u8, axis: u8) -> u8 {
    if slope == SLOPE_FLAT
        || ((slope == SLOPE_NE || slope == SLOPE_SW) && axis == 0)
        || ((slope == SLOPE_NW || slope == SLOPE_SE) && axis == 1)
    {
        return FOUNDATION_NONE;
    }
    if has_slope_highest_corner(slope) {
        inclined_foundation(axis)
    } else {
        flattening_foundation(slope)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_bridge_length_cost(length: i32) -> i32 {
    calc_bridge_len_cost_factor(length)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_bridge_foundation(slope: u8, axis: Axis) -> Foundation {
    Foundation(get_bridge_foundation(slope, axis.0))
}
/// Original `HasBridgeFlatRamp`: if the foundation slope is flat the bridge has a
/// non-flat ramp and vice versa.
pub(crate) fn has_bridge_flat_ramp(s: &Services, slope: u8, axis: u8) -> bool {
    (s.apply_foundation)(Foundation(get_bridge_foundation(slope, axis)), Slope(slope))
        .slope
        .0
        != SLOPE_FLAT
}
/// # Safety
/// The native facade supplies its immutable process-lifetime shared table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_bridge_flat_ramp(
    s: *const Services,
    slope: u8,
    axis: Axis,
) -> bool {
    // SAFETY: canonical immutable table from the native facade.
    has_bridge_flat_ramp(unsafe { &*s }, slope, axis.0)
}
