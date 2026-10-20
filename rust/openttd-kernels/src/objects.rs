/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Object lifecycle owner. Native pools, specs, map and save records remain
//! canonical. Counts and cleared-area command scratch have one process-wide
//! Rust owner, serialized by the game-state mutex. No state borrow spans a
//! reentrant service. Panics abort; native exceptions never unwind through Rust.
#![allow(unsafe_code)]
use std::cell::UnsafeCell;

struct State {
    counts: [u16; 64000],
    areas: Vec<ClearArea>,
}
struct Global(UnsafeCell<State>);
// SAFETY: native callers serialize every entry under the game-state mutex.
// An access ends before any service can reenter; the owner is process-wide.
unsafe impl Sync for Global {}
static STATE: Global = Global(UnsafeCell::new(State {
    counts: [0; 64000],
    areas: Vec::new(),
}));

/// Value-only command scratch entry. Indices, never pointers, survive append.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ClearArea {
    pub first_tile: u32,
    pub tile: u32,
    pub width: u16,
    pub height: u16,
}
/// Industry transaction backup at the original copy point.
pub(crate) fn snapshot_areas() -> Vec<ClearArea> {
    // SAFETY: caller holds the game-state mutex; no callback in this scope.
    unsafe { (*STATE.0.get()).areas.clone() }
}
/// Industry transaction restore at the original move point.
pub(crate) fn restore_areas(areas: Vec<ClearArea>) {
    // SAFETY: caller holds the game-state mutex; no callback in this scope.
    unsafe { (*STATE.0.get()).areas = areas };
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_counts_reset() {
    // SAFETY: serialized native entry.
    unsafe { (*STATE.0.get()).counts.fill(0) };
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_count_increment(kind: u16) {
    // SAFETY: serialized entry; native source asserts kind < NUM_OBJECTS.
    let count = unsafe { &mut (*STATE.0.get()).counts[usize::from(kind)] };
    *count = count.wrapping_add(1);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_count_decrement(kind: u16) {
    // SAFETY: serialized entry; native source asserts kind < NUM_OBJECTS.
    let count = unsafe { &mut (*STATE.0.get()).counts[usize::from(kind)] };
    *count = count.wrapping_sub(1);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_count(kind: u16) -> u16 {
    // SAFETY: serialized native entry; original kind precondition.
    unsafe { (*STATE.0.get()).counts[usize::from(kind)] }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_areas_clear() {
    // SAFETY: serialized native entry.
    unsafe { (*STATE.0.get()).areas.clear() };
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_areas_find(tile: u32, log_x: u32) -> usize {
    let mask = (1 << log_x) - 1;
    let x = tile & mask;
    let y = tile >> log_x;
    // SAFETY: serialized entry, no callbacks while reading.
    unsafe { &(*STATE.0.get()).areas }
        .iter()
        .position(|a| {
            if a.width == 0 {
                return false;
            }
            assert!(a.height != 0);
            let left = a.tile & mask;
            let top = a.tile >> log_x;
            x >= left && x < left + u32::from(a.width) && y >= top && y < top + u32::from(a.height)
        })
        .unwrap_or(usize::MAX)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_areas_get(index: usize) -> ClearArea {
    // SAFETY: serialized entry; index was returned by find/append.
    unsafe { (&(*STATE.0.get()).areas)[index] }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_areas_append(area: ClearArea) -> usize {
    // SAFETY: serialized entry; allocation failure aborts.
    let areas = unsafe { &mut (*STATE.0.get()).areas };
    areas.push(area);
    areas.len() - 1
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_areas_set_first(index: usize, first: u32) {
    // SAFETY: serialized entry; append preserves indices returned by find/append.
    unsafe { (&mut (*STATE.0.get()).areas)[index].first_tile = first };
}
