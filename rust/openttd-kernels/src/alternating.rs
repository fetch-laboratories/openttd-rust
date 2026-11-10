/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `AlternatingIterator` (`misc/alternating_iterator.hpp`): iterator that
//! alternately takes from the "middle" of a range. The traversal decisions live
//! here; the typed iterators and their live boundaries stay in C++.

/// The scalar state of an `AlternatingIterator`. A state flag is 1 to take from
/// after the middle and 0 to take from before it.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlternatingState {
    /// Position within the entire range.
    pub position: usize,
    /// Next state for advancing iterator. If true take from after middle, otherwise take from before middle.
    pub next_state: u8,
    /// Current state for reading iterator. If true take from after middle, otherwise take from before middle.
    pub state: u8,
}

/// No iterator movement: the increment reached the end.
pub const MOVEMENT_NONE: u8 = 0;
/// `++after`.
pub const MOVEMENT_AFTER: u8 = 1;
/// `--before`.
pub const MOVEMENT_BEFORE: u8 = 2;

/// One `operator++` and the iterator movement C++ makes for it.
#[repr(C)]
pub struct AlternatingStep {
    /// Updated state.
    pub state: AlternatingState,
    /// `MOVEMENT_NONE`, `MOVEMENT_AFTER` or `MOVEMENT_BEFORE`.
    pub movement: u8,
}

/// `AlternatingIterator::AlternatingIterator`'s state: `position` is 0 for a
/// begin iterator, otherwise the range's distance; `before_at_first` is
/// `before == first`.
pub fn initialize(position: usize, before_at_first: bool) -> AlternatingState {
    let next_state = u8::from(before_at_first);
    AlternatingState {
        position,
        next_state,
        state: next_state,
    }
}

/// `AlternatingIterator::operator++` up to `Next()`'s iterator movement.
pub fn advance(mut state: AlternatingState, size: usize) -> AlternatingStep {
    if state.position >= size {
        crate::services_assert::check(false);
    }
    state.position += 1;
    let movement = if state.position < size {
        /* Next(): */
        state.state = state.next_state;
        if state.next_state != 0 {
            MOVEMENT_AFTER
        } else {
            MOVEMENT_BEFORE
        }
    } else {
        MOVEMENT_NONE
    };
    AlternatingStep { state, movement }
}

/// The rest of `AlternatingIterator::Next()` after its iterator movement. After
/// `++after`, `boundary` is `before == first`; after `--before`, it is
/// `std::next(after) != last`. Both select after the middle when true.
pub fn complete(mut state: AlternatingState, boundary: bool) -> AlternatingState {
    state.next_state = u8::from(boundary);
    state
}

/// `AlternatingIterator`'s comparisons: they compare `position` only.
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
        next_state,
        state,
    };
    "OpenTTDRustAlternatingStep" ["probe"] => AlternatingStep {
        state,
        movement,
    };
}
