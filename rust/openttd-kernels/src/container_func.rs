/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared container helpers from `core/container_func.hpp`.
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]

/// `find_index` returns the first match, narrowing the iterator distance to native int.
pub(crate) fn find_index<T: PartialEq>(container: &[T], item: &T) -> i32 {
    container
        .iter()
        .position(|value| value == item)
        .map_or(-1, |index| index as i32)
}
