/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `AlternatingIterator` empty and singleton ranges, copies, postfix, ordering, end `Base()`
//! values and live insertion into a node-stable range (#736).
//!
//! Moved from `src/tests/alternating_iterator_state.cpp`, whose expectations were
//! fork-written literals. The pinned original's `AlternatingView` ran the same four cases
//! over `std::vector` and `std::list`, and its observations are frozen in
//! `iterator-oracle.txt` (`python3 tools/oracle-capture.py alternating`). [`Iterator`] is
//! the candidate C++ `AlternatingIterator` glue over the production traversal: it reads the
//! live range length at each increment, moves the iterator [`advance`] selects and passes
//! [`complete`] the live boundary, and compares through [`compare`]. [`Range`] stands in for
//! the container: its iterators name nodes, so they stay valid across an insertion, as
//! `std::list`'s do and as `std::vector`'s do when nothing is inserted.
use super::{
    AlternatingState, MOVEMENT_AFTER, MOVEMENT_BEFORE, MOVEMENT_NONE, advance, compare, complete,
    initialize,
};
use crate::oracle;

const ORACLE: &str = include_str!("iterator-oracle.txt");

/// A container's nodes in order, as `(node, value)`.
struct Range(Vec<(usize, i32)>);

/// A container iterator: a node, or `None` past the end.
type Node = Option<usize>;

impl Range {
    fn new(values: &[i32]) -> Self {
        Self(values.iter().copied().enumerate().collect())
    }

    /// The node's current position; `end()` is the length.
    fn position(&self, node: Node) -> usize {
        node.map_or(self.0.len(), |node| {
            self.0.iter().position(|&(id, _)| id == node).unwrap()
        })
    }

    fn at(&self, position: usize) -> Node {
        self.0.get(position).map(|&(id, _)| id)
    }

    fn begin(&self) -> Node {
        self.at(0)
    }

    fn next(&self, node: Node) -> Node {
        self.at(self.position(node) + 1)
    }

    fn prev(&self, node: Node) -> Node {
        self.at(self.position(node) - 1)
    }

    fn distance(&self, from: Node, to: Node) -> usize {
        self.position(to) - self.position(from)
    }

    fn value(&self, node: Node) -> i32 {
        self.0[self.position(node)].1
    }

    /// `insert(pos, value)`: a new node before `pos`.
    fn insert(&mut self, pos: Node, value: i32) {
        let at = self.position(pos);
        let id = self.0.len();
        self.0.insert(at, (id, value));
    }
}

/// The candidate `AlternatingIterator`.
#[derive(Clone, Copy)]
struct Iterator {
    first: Node,
    last: Node,
    after: Node,
    before: Node,
    traversal: AlternatingState,
}

impl Iterator {
    /// `AlternatingIterator(first, last, middle, begin)`, as `AlternatingView` builds it.
    fn new(range: &Range, middle: Node, begin: bool) -> Self {
        let (first, last) = (range.begin(), None);
        // Starting from the end is not supported, unless the range is empty.
        assert!(first == last || middle != last);
        let initial_position = if begin {
            0
        } else {
            range.distance(first, last)
        };
        Self {
            first,
            last,
            after: middle,
            before: middle,
            traversal: initialize(initial_position, middle == first),
        }
    }

    fn same(self, other: Self) -> bool {
        compare(self.traversal, other.traversal) == 0
    }

    fn less(self, other: Self) -> bool {
        compare(self.traversal, other.traversal) < 0
    }

    fn greater(self, other: Self) -> bool {
        compare(self.traversal, other.traversal) > 0
    }

    fn base(self) -> Node {
        if self.traversal.state != 0 {
            self.after
        } else {
            self.before
        }
    }

    fn get(self, range: &Range) -> i32 {
        range.value(self.base())
    }

    /// Prefix `operator++`: Rust selects the movement; the glue queries only the requested
    /// live boundary after moving.
    fn increment(&mut self, range: &Range) -> &mut Self {
        let size = range.distance(self.first, self.last);
        assert!(self.traversal.position < size);
        let step = advance(self.traversal, size);
        self.traversal = step.state;
        match step.movement {
            MOVEMENT_NONE => {}
            MOVEMENT_AFTER => {
                assert!(self.after != self.last);
                self.after = range.next(self.after);
                self.traversal = complete(self.traversal, self.before == self.first);
            }
            MOVEMENT_BEFORE => {
                assert!(self.before != self.first);
                self.before = range.prev(self.before);
                let boundary = range.next(self.after) != self.last;
                self.traversal = complete(self.traversal, boundary);
            }
            _ => unreachable!(),
        }
        self
    }

    /// Postfix `operator++`: the copy before the increment.
    fn postfix(&mut self, range: &Range) -> Self {
        let result = *self;
        self.increment(range);
        result
    }
}

/// One case's observations in check order, each named by its operation.
#[derive(Default)]
struct Rows(Vec<(String, String)>);

impl Rows {
    fn row(&mut self, name: &str, value: String) {
        let check = self.0.len();
        self.0.push((format!("check {check} {name}"), value));
    }

    fn flag(&mut self, name: &str, value: bool) {
        self.row(name, u8::from(value).to_string());
    }

    fn value(&mut self, name: &str, value: i32) {
        self.row(name, value.to_string());
    }

    /// A `Base()` as its distance from the range's first element.
    fn base(&mut self, name: &str, range: &Range, iterator: Iterator) {
        let distance = range.distance(range.begin(), iterator.base());
        self.row(name, distance.to_string());
    }
}

fn singleton_case() -> Rows {
    let mut r = Rows::default();
    let empty = Range::new(&[]);
    let (begin, end) = (
        Iterator::new(&empty, empty.begin(), true),
        Iterator::new(&empty, empty.begin(), false),
    );
    r.flag("empty begin == end", begin.same(end));
    r.base("empty begin Base", &empty, begin);
    r.base("empty end Base", &empty, end);
    let singleton = Range::new(&[7]);
    let end = Iterator::new(&singleton, singleton.begin(), false);
    let mut iterator = Iterator::new(&singleton, singleton.begin(), true);
    r.value("*begin", iterator.get(&singleton));
    r.flag("begin != end", !iterator.same(end));
    iterator.increment(&singleton);
    r.flag("++begin == end", iterator.same(end));
    r.base("++begin Base", &singleton, iterator);
    r
}

fn copies_case() -> Rows {
    let mut r = Rows::default();
    let values = Range::new(&[0, 1, 2, 3, 4]);
    let middle = values.at(2);
    let mut first = Iterator::new(&values, middle, true);
    let mut copy = first;
    let old = copy.postfix(&values);
    r.flag("first == old", first.same(old));
    r.value("*first", first.get(&values));
    r.value("*old", old.get(&values));
    r.value("*copy", copy.get(&values));
    r.flag("first < copy", first.less(copy));
    r.flag("copy > first", copy.greater(first));
    let address = std::ptr::from_mut(first.increment(&values));
    r.flag(
        "&++first == &first",
        std::ptr::eq(address, &raw const first),
    );
    r.flag("first == copy", first.same(copy));
    copy.increment(&values);
    r.value("*first", first.get(&values));
    r.value("*copy", copy.get(&values));
    r.flag("first < copy", first.less(copy));
    r
}

fn end_case() -> Rows {
    let mut r = Rows::default();
    let values = Range::new(&[0, 1, 2, 3, 4]);
    let middle = values.at(2);
    let constructed_end = Iterator::new(&values, middle, false);
    let mut iterated_end = Iterator::new(&values, middle, true);
    while !iterated_end.same(constructed_end) {
        iterated_end.increment(&values);
    }
    r.flag(
        "iterated == constructed",
        iterated_end.same(constructed_end),
    );
    r.flag(
        "iterated <=> constructed is equal",
        compare(iterated_end.traversal, constructed_end.traversal) == 0,
    );
    r.base("constructed end Base", &values, constructed_end);
    r.base("iterated end Base", &values, iterated_end);
    r
}

fn insertion_case() -> Rows {
    let mut r = Rows::default();
    let mut values = Range::new(&[0, 1, 2, 3, 4]);
    let middle = values.at(2);
    let mut iterator = Iterator::new(&values, middle, true);
    let mut sequence = vec![iterator.get(&values)];
    iterator.increment(&values);
    sequence.push(iterator.get(&values));
    // Existing iterators stay valid; both traversal and range length stay live.
    let after_middle = values.next(middle);
    values.insert(after_middle, 25);
    iterator.increment(&values);
    while !iterator.same(Iterator::new(&values, middle, false)) {
        sequence.push(iterator.get(&values));
        iterator.increment(&values);
    }
    let sequence: Vec<String> = sequence.iter().map(i32::to_string).collect();
    r.row("sequence", sequence.join(","));
    r.base("end Base", &values, iterator);
    r
}

#[test]
fn alternating_iterator_cases_match_original() {
    let sections = oracle::parse(ORACLE);
    for (name, case, checks) in [
        ("singleton", singleton_case as fn() -> Rows, 7),
        ("copies", copies_case, 11),
        ("end", end_case, 4),
        ("insertion", insertion_case, 2),
    ] {
        assert_eq!(oracle::check(&sections, name, case().0), checks, "{name}");
    }
}
