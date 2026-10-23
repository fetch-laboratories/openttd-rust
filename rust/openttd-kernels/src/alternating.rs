/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Alternating traversal decisions; typed iterators and their live boundaries stay in C++.

/// Copyable scalar state. Selectors are 0 for before-middle and 1 for after-middle.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlternatingState {
    /// Logical position; equality/ordering do not compare underlying iterators.
    pub position: usize,
    /// Side to move on the next nonterminal increment.
    pub next_after: u8,
    /// Side currently selected by `Base()`, including after reaching end.
    pub current_after: u8,
}

/// One logical increment and its typed iterator movement request.
#[repr(C)]
pub struct AlternatingStep {
    /// Updated logical state.
    pub state: AlternatingState,
    /// 0 means no movement, 1 increments after, 2 decrements before.
    pub movement: u8,
}

pub fn initialize(end_position: usize, before_at_first: bool) -> AlternatingState {
    let side = u8::from(before_at_first);
    AlternatingState {
        position: end_position,
        next_after: side,
        current_after: side,
    }
}

pub fn advance(mut state: AlternatingState, live_size: usize) -> AlternatingStep {
    assert!(state.position < live_size);
    state.position += 1;
    let movement = if state.position < live_size {
        state.current_after = state.next_after;
        if state.next_after != 0 { 1 } else { 2 }
    } else {
        // Original ++ skips Next() at end, preserving the last selected Base().
        0
    };
    AlternatingStep { state, movement }
}

pub fn complete(mut state: AlternatingState, boundary: bool) -> AlternatingState {
    // After moving after, boundary means before == first. After moving before,
    // it means next(after) != last. Both original cases select after iff true.
    state.next_after = u8::from(boundary);
    state
}

pub fn compare(left: AlternatingState, right: AlternatingState) -> i8 {
    match left.position.cmp(&right.position) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRustAlternatingState" ["probe"] => AlternatingState {
        position,
        next_after,
        current_after,
    };
    "OpenTTDRustAlternatingStep" ["probe"] => AlternatingStep {
        state,
        movement,
    };
}
