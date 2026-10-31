/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `landscape.h` snow-line ownership. Native loader records remain native allocations.
#![allow(unsafe_code, missing_docs, clippy::missing_safety_doc)]
use crate::{game_state::GameState, services::Services};
use std::ptr;

// The game-state mutex serializes all entries, including GUI/game-thread handoff.
// A native allocation stays live until replacement; only scalar raw loads occur,
// and no record borrow crosses the native whole-record deletion operation.
#[repr(C)]
pub struct SnowLine {
    pub table: [[u8; 32]; 12],
    pub highest_value: u8,
    pub lowest_value: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Support {
    pub height: *const u8,
    pub destroy: unsafe extern "C" fn(*mut SnowLine),
}
static SNOW_LINE: GameState<*mut SnowLine> = GameState::new(ptr::null_mut());

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_snow_line_is_set() -> bool {
    unsafe { !SNOW_LINE.get().read().is_null() }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_snow_line_set(s: &Support, next: *mut SnowLine) {
    let old = unsafe { ptr::replace(SNOW_LINE.get(), next) };
    if !old.is_null() {
        unsafe { (s.destroy)(old) };
    }
}
pub(crate) fn clear(s: &Services) {
    unsafe { openttd_rust_snow_line_set(&s.snow_line_support, ptr::null_mut()) };
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_snow_line_clear(s: &Services) {
    clear(s);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_snow_line_get(s: &Services) -> u8 {
    let record = unsafe { SNOW_LINE.get().read() };
    if record.is_null() {
        return unsafe { s.snow_line_support.height.read() };
    }
    let ymd =
        crate::timer_game_common::calendar_date_to_ymd(crate::timer_game_common::calendar_date().0);
    unsafe { ptr::addr_of!((*record).table[usize::from(ymd.month)][usize::from(ymd.day)]).read() }
}
pub(crate) fn highest(s: &Services) -> u8 {
    let record = unsafe { SNOW_LINE.get().read() };
    if record.is_null() {
        unsafe { s.snow_line_support.height.read() }
    } else {
        unsafe { ptr::addr_of!((*record).highest_value).read() }
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_snow_line_lowest(s: &Services) -> u8 {
    let record = unsafe { SNOW_LINE.get().read() };
    if record.is_null() {
        unsafe { s.snow_line_support.height.read() }
    } else {
        unsafe { ptr::addr_of!((*record).lowest_value).read() }
    }
}
pub(crate) use openttd_rust_snow_line_get as get;
crate::abi::layouts! {
    fn abi_layout;
    "SnowLine" ["landscape"] => SnowLine { table, highest_value, lowest_value };
    "OpenTTDSnowLineSupport" ["landscape"] => Support { height, destroy };
}
