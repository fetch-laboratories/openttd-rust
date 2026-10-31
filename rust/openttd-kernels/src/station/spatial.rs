/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Station simulation index. All accesses hold the game-state mutex. Coordinate
//! readers and radius visitors cannot mutate this index; no tree borrow spans
//! presentation reentry from the qualified native `BaseStation::MoveSign`.
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
use super::{BaseStation, CatchmentServices};
use crate::{game_state::GameState, kdtree::Kdtree, services::Services, station_type::StationID};
static TREE: GameState<Kdtree<StationID>> = GameState::new(Kdtree::new());
fn table(s: &Services) -> &CatchmentServices {
    unsafe { &*(s.station_catchment)() }
}
fn coordinate(c: &CatchmentServices, map: crate::map_func::Map, id: StationID, dim: usize) -> u16 {
    let tile = (c.location)((c.station_if_valid)(id));
    if dim == 0 {
        map.tile_x(tile) as u16
    } else {
        map.tile_y(tile) as u16
    }
}
pub(crate) fn rebuild(s: &Services) {
    let c = table(s);
    let map = s.map();
    let mut ids = Vec::with_capacity((c.station_count)());
    let mut st = (c.next_station)(0);
    while !st.is_null() {
        let id = (c.id)(st);
        ids.push(id);
        st = (c.next_station)(u32::from(id.0) + 1);
    }
    unsafe {
        (*TREE.get()).build(&mut ids, &|ids, dim| {
            crate::kdtree::native_split(s, ids, |id| coordinate(c, map, id, dim))
        });
    }
}
pub(crate) fn insert(s: &Services, st: *mut BaseStation) {
    let c = table(s);
    let map = s.map();
    unsafe {
        (*TREE.get()).insert(
            (c.id)(st),
            &|id, dim| coordinate(c, map, id, dim),
            &|ids, dim| crate::kdtree::native_split(s, ids, |id| coordinate(c, map, id, dim)),
        );
    }
}
pub(crate) fn remove(s: &Services, id: StationID) {
    let c = table(s);
    let map = s.map();
    unsafe {
        (*TREE.get()).remove(id, &|id, dim| coordinate(c, map, id, dim), &|ids, dim| {
            crate::kdtree::native_split(s, ids, |id| coordinate(c, map, id, dim))
        });
    }
}
pub(crate) fn radius(
    s: &Services,
    center: u32,
    radius: u32,
    mut visit: impl FnMut(*mut BaseStation),
) {
    let c = table(s);
    let map = s.map();
    // Native unsigned arithmetic converts to int before clipping, then narrows to u16.
    let bounds = |v: u32, size: u32| {
        [
            (v.wrapping_sub(radius).cast_signed().max(0)) as u16,
            (v.wrapping_add(radius)
                .wrapping_add(1)
                .cast_signed()
                .min(size.cast_signed())) as u16,
        ]
    };
    let x = bounds(map.tile_x(center), map.size_x());
    let y = bounds(map.tile_y(center), (s.map_max_y)().wrapping_add(1));
    unsafe {
        (*TREE.get()).contained(
            [x[0], y[0]],
            [x[1], y[1]],
            &|id, dim| coordinate(c, map, id, dim),
            |id| visit((c.station_if_valid)(id)),
        );
    }
}
pub(crate) fn move_sign(s: &Services, st: *mut BaseStation, tile: u32) {
    let commands = unsafe { &*(s.station_commands)() };
    let c = table(s);
    let station = unsafe { &*commands.station };
    if !(station.is_station)(st) {
        (commands.waypoint_move_sign)(st, tile);
        return;
    }
    if (c.location)(st) == tile {
        return;
    }
    remove(s, (c.id)(st));
    (commands.base_move_sign)(st, tile);
    insert(s, st);
}
pub(crate) fn modify_rating_around(s: &Services, tile: u32, owner: u8, amount: i32, distance: u32) {
    let c = table(s);
    let commands = unsafe { &*(s.station_commands)() };
    let station = unsafe { &*commands.station };
    let map = s.map();
    radius(s, tile, distance, |st| {
        if (station.station_owner)(st) == owner
            && map.distance_manhattan(tile, (c.location)(st)) <= distance
        {
            unsafe {
                crate::station_service::modify_rating((c.owner)(st), amount);
            }
        }
    });
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_rebuild_index(s: *const Services) {
    rebuild(unsafe { &*s });
}
