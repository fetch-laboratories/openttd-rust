/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared road construction, removal and conversion price policy.
#![allow(unsafe_code)]
use crate::company::price;
use crate::services::Services;

/// `RoadBuildCost`: Money multiplication saturates before the arithmetic shift.
pub(crate) fn build_cost(shared: &Services, road_type: u8) -> i64 {
    price(2).saturating_mul(i64::from((shared.road_cost_multiplier)(road_type))) >> 3
}
/// `RoadClearCost`: ordinary roads do not read construction metadata.
pub(crate) fn clear_cost(shared: &Services, road_type: u8) -> i64 {
    if (shared.road_type_is_road)(road_type) {
        return price(41);
    }
    price(41).saturating_sub(build_cost(shared, road_type).saturating_mul(3) / 4)
}
pub(crate) fn convert_cost(shared: &Services, from: u8, to: u8) -> i64 {
    if from == to {
        return 0;
    }
    build_cost(shared, to).saturating_add(clear_cost(shared, from))
}
/// Native callers share the Rust road price policy and source Money arithmetic.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_build_cost(shared: &Services, road_type: u8) -> i64 {
    build_cost(shared, road_type)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_clear_cost(shared: &Services, road_type: u8) -> i64 {
    clear_cost(shared, road_type)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_convert_cost(shared: &Services, from: u8, to: u8) -> i64 {
    convert_cost(shared, from, to)
}
