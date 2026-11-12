/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `SpiralTileSequence` clipping on rectangular maps, live map dimensions, copies, postfix,
//! coordinate equality and wrapping hole initialisation (#736).
//!
//! Moved from `src/tests/spiral_iterator_state.cpp`, whose expectations were fork-written
//! literals. The pinned original's `SpiralTileIterator` ran the same four cases, and its
//! observations are frozen in `iterator-oracle.txt` (`python3 tools/oracle-capture.py
//! spiral`). [`Iterator`] is the candidate C++ `SpiralTileIterator` glue over the
//! production state: it passes the live map's dimensions to every construction and
//! increment, compares through [`equal`] and [`SpiralState::is_end`], and dereferences to
//! `TileXY` of the current map.
use super::{SpiralState, equal, hole, square};
use crate::map_func::Map;
use crate::oracle;
use crate::tile_type::TileIndex;

const ORACLE: &str = include_str!("iterator-oracle.txt");

/// The map dimensions `Map::Allocate` sets, read live by every glue operation.
#[derive(Clone, Copy)]
struct Live {
    log_x: u32,
    log_y: u32,
}

impl Live {
    /// `Map::Allocate(size_x, size_y)` for power-of-two sizes.
    fn allocate(size_x: u32, size_y: u32) -> Self {
        Self {
            log_x: size_x.trailing_zeros(),
            log_y: size_y.trailing_zeros(),
        }
    }

    fn map(self) -> Map {
        Map::from_log_x(self.log_x)
    }

    fn size(self) -> (u32, u32) {
        (1 << self.log_x, 1 << self.log_y)
    }

    fn tile_xy(self, x: u32, y: u32) -> TileIndex {
        self.map().tile_xy(x, y)
    }
}

/// The candidate `SpiralTileIterator`.
#[derive(Clone, Copy)]
struct Iterator {
    state: SpiralState,
}

impl Iterator {
    /// `SpiralTileIterator(center, diameter)`.
    fn square(live: Live, center: TileIndex, diameter: u32) -> Self {
        let (size_x, size_y) = live.size();
        let map = live.map();
        Self {
            state: square(
                map.tile_x(center),
                map.tile_y(center),
                diameter,
                size_x,
                size_y,
            ),
        }
    }

    /// `SpiralTileIterator(start_north, radius, w, h)`.
    fn hole(live: Live, north: TileIndex, radius: u32, width: u32, height: u32) -> Self {
        let (size_x, size_y) = live.size();
        let map = live.map();
        Self {
            state: hole(
                map.tile_x(north),
                map.tile_y(north),
                radius,
                width,
                height,
                size_x,
                size_y,
            ),
        }
    }

    /// `operator*`: `TileXY` of the current map.
    fn get(self, live: Live) -> u32 {
        live.tile_xy(self.state.x, self.state.y).0
    }

    /// Prefix `operator++`.
    fn increment(&mut self, live: Live) -> &mut Self {
        let (size_x, size_y) = live.size();
        self.state = self.state.advance(size_x, size_y);
        self
    }

    /// Postfix `operator++`: the copy before the increment.
    fn postfix(&mut self, live: Live) -> Self {
        let result = *self;
        self.increment(live);
        result
    }

    /// `operator==` between iterators: coordinates only.
    fn same(self, other: Self) -> bool {
        equal(self.state, other.state)
    }

    /// `operator==(std::default_sentinel_t)`.
    fn ended(self) -> bool {
        self.state.is_end()
    }

    /// A whole range-for traversal as its ordered tiles.
    fn sequence(mut self, live: Live) -> String {
        let mut tiles = Vec::new();
        while !self.ended() {
            tiles.push(self.get(live).to_string());
            self.increment(live);
        }
        tiles.join(",")
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
}

fn edges_case() -> Rows {
    let mut r = Rows::default();
    let live = Live::allocate(128, 64);
    let wide = Iterator::square(live, live.tile_xy(126, 1), 5);
    r.row("128x64 (126,1) diameter 5", wide.sequence(live));
    let clipped = Iterator::hole(live, live.tile_xy(126, 62), 2, 1, 0);
    r.row("128x64 (126,62) radius 2 1x0", clipped.sequence(live));
    let live = Live::allocate(64, 128);
    let tall = Iterator::square(live, live.tile_xy(1, 126), 4);
    r.row("64x128 (1,126) diameter 4", tall.sequence(live));
    r
}

fn live_case() -> Rows {
    let mut r = Rows::default();
    let live = Live::allocate(64, 64);
    let mut iterator = Iterator::square(live, live.tile_xy(63, 1), 3);
    let live = Live::allocate(128, 64);
    iterator.increment(live);
    r.row("after 128x64 increment", iterator.get(live).to_string());
    let live = Live::allocate(64, 64);
    iterator.increment(live);
    r.row("after 64x64 increment", iterator.get(live).to_string());
    r
}

fn copies_case() -> Rows {
    let mut r = Rows::default();
    let live = Live::allocate(64, 128);
    let center = live.tile_xy(4, 70);
    let begin = Iterator::square(live, center, 3);
    let mut first = begin;
    let mut copy = first;
    r.flag("first == copy", first.same(copy));
    // Radius limits differ, but the public comparison is coordinates only.
    r.flag(
        "diameter 5 begin",
        first.same(Iterator::square(live, center, 5)),
    );
    r.flag(
        "diameter 1 begin",
        first.same(Iterator::square(live, center, 1)),
    );
    let previous = first.postfix(live);
    r.flag("previous == copy", previous.same(copy));
    r.flag("previous != first", !previous.same(first));
    r.row("*previous", previous.get(live).to_string());
    r.row("*first", first.get(live).to_string());
    let address = std::ptr::from_mut(copy.increment(live));
    r.flag("&++copy == &copy", std::ptr::eq(address, &raw const copy));
    r.flag("copy == first", copy.same(first));
    r.row("*previous", previous.get(live).to_string());
    let mut ended = begin;
    let mut count = 0;
    while !ended.ended() {
        count += 1;
        ended.increment(live);
    }
    r.row("count", count.to_string());
    r.flag("ended == sentinel", ended.ended());
    let end_copy = ended;
    r.flag("end_copy == ended", end_copy.same(ended));
    r.flag("end_copy == sentinel", end_copy.ended());
    // The terminal coordinate is (6,68), distinct from end-state.
    let same_coordinate = Iterator::square(live, live.tile_xy(6, 68), 1);
    r.flag("ended == (6,68) begin", ended.same(same_coordinate));
    r.flag("(6,68) begin == sentinel", same_coordinate.ended());
    r
}

fn wrapping_case() -> Rows {
    let mut r = Rows::default();
    let live = Live::allocate(64, 64);
    // x + UINT32_MAX + 1 and the initial movement count wrap.
    let wrapped = Iterator::hole(live, live.tile_xy(0, 0), 1, u32::MAX, 0);
    r.row("*begin", wrapped.get(live).to_string());
    r.flag("begin == sentinel", wrapped.ended());
    let square = Iterator::square(live, live.tile_xy(0, 0), 1);
    r.flag("begin == diameter 1 begin", wrapped.same(square));
    r
}

#[test]
fn spiral_iterator_cases_match_original() {
    let sections = oracle::parse(ORACLE);
    for (name, case, checks) in [
        ("edges", edges_case as fn() -> Rows, 3),
        ("live", live_case, 2),
        ("copies", copies_case, 16),
        ("wrapping", wrapping_case, 3),
    ] {
        assert_eq!(oracle::check(&sections, name, case().0), checks, "{name}");
    }
}
