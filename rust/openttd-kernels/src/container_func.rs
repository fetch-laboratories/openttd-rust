/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared container helpers from `core/container_func.hpp`.

/// `include`: append an item if it is not already contained. Returns whether
/// the item was already present. The original container must support `emplace_back`;
/// consider a set, unordered set or flat set in new code.
pub(crate) fn include<T: PartialEq + Clone>(container: &mut Vec<T>, item: &T) -> bool {
    let is_member = container.contains(item);
    if !is_member {
        container.push(item.clone());
    }
    is_member
}

/// `find_index`: helper function to get the index of an item. Returns the index of the
/// first matching element if found, otherwise -1.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    reason = "find_index returns the iterator distance as int"
)]
pub(crate) fn find_index<T: PartialEq>(container: &[T], item: &T) -> i32 {
    container
        .iter()
        .position(|value| value == item)
        .map_or(-1, |index| index as i32)
}

/// `Slide`: move elements between `first` and `last` to a new `position`, rotating elements
/// in between as necessary. Returns the first and last indices after being moved.
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
