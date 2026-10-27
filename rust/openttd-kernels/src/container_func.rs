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

/// `Slide` moves `[first, last)` before `position` and returns its new bounds.
pub(crate) fn slide<T>(
    container: &mut [T],
    first: usize,
    last: usize,
    position: usize,
) -> (usize, usize) {
    if last < position {
        container[first..position].rotate_left(last - first);
        return (position - (last - first), position);
    }
    if position < first {
        container[position..last].rotate_right(last - first);
        return (position, position + (last - first));
    }
    (first, last)
}

#[cfg(test)]
mod tests {
    use super::slide;

    #[test]
    fn slide_ranges_and_returned_bounds() {
        for (first, last, position, expected, bounds) in [
            (1, 3, 5, [0, 3, 4, 1, 2], (3, 5)),
            (2, 4, 0, [2, 3, 0, 1, 4], (0, 2)),
            (0, 1, 4, [1, 2, 3, 0, 4], (3, 4)),
            (4, 5, 1, [0, 4, 1, 2, 3], (1, 2)),
            (1, 4, 1, [0, 1, 2, 3, 4], (1, 4)),
            (1, 4, 2, [0, 1, 2, 3, 4], (1, 4)),
            (1, 4, 4, [0, 1, 2, 3, 4], (1, 4)),
            (2, 2, 5, [0, 1, 2, 3, 4], (5, 5)),
            (2, 2, 0, [0, 1, 2, 3, 4], (0, 0)),
            (0, 5, 5, [0, 1, 2, 3, 4], (0, 5)),
        ] {
            let mut values = [0, 1, 2, 3, 4];
            assert_eq!(slide(&mut values, first, last, position), bounds);
            assert_eq!(values, expected);
        }
    }
}
