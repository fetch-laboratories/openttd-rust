/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Canonical road price operations shared by native and Rust command owners.
use crate::services::Services;
pub(crate) fn build_cost(s: &Services, road_type: u8) -> i64 {
    crate::company::price(2).saturating_mul(i64::from((s.road_cost_multiplier)(road_type))) >> 3
}
pub(crate) fn clear_cost(s: &Services, road_type: u8) -> i64 {
    let clear = crate::company::price(41);
    if crate::road::road_type_is_road(road_type) {
        return clear;
    }
    clear.saturating_sub(build_cost(s, road_type).saturating_mul(3) / 4)
}
pub(crate) fn convert_cost(s: &Services, from: u8, to: u8) -> i64 {
    if from == to {
        return 0;
    }
    build_cost(s, to).saturating_add(clear_cost(s, from))
}
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_build_cost(s: &Services, road_type: u8) -> i64 {
    build_cost(s, road_type)
}
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_clear_cost(s: &Services, road_type: u8) -> i64 {
    clear_cost(s, road_type)
}
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_convert_cost(s: &Services, from: u8, to: u8) -> i64 {
    convert_cost(s, from, to)
}
