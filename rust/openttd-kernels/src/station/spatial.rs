/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Station simulation index. All accesses hold the game-state mutex. Coordinate
//! readers and radius visitors cannot mutate this index; no tree borrow spans
//! presentation reentry from the qualified native `BaseStation::MoveSign`.
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
use super::BaseStation;
use crate::{game_state::GameState, kdtree::Kdtree, services::Services, station_type::StationID};
static TREE: GameState<Kdtree<StationID>> = GameState::new(Kdtree::new());
fn coordinate(s: &Services, map: crate::map_func::Map, id: StationID, dim: usize) -> u16 {
    let tile = crate::raw_field::read_field!(crate::station_record::get_record(s, id), xy).0;
    if dim == 0 {
        map.tile_x(tile) as u16
    } else {
        map.tile_y(tile) as u16
    }
}
pub(crate) fn rebuild(s: &Services) {
    let map = s.map();
    let mut st = crate::station_record::next_station_shell(s, 0);
    let ids = std::iter::from_fn(|| {
        if st.is_null() {
            return None;
        }
        let id = crate::station_record::id(st);
        st = crate::station_record::next_station_shell(s, u32::from(id.0) + 1);
        Some(id)
    });
    unsafe {
        (*TREE.get()).build(ids, &|id, dim| coordinate(s, map, id, dim), &|keys| {
            crate::kdtree::native_split(s, keys)
        });
    }
}
pub(crate) fn insert(s: &Services, st: *mut BaseStation) {
    let map = s.map();
    unsafe {
        (*TREE.get()).insert(
            crate::station_record::id(st),
            &|id, dim| coordinate(s, map, id, dim),
            &|keys| crate::kdtree::native_split(s, keys),
        );
    }
}
pub(crate) fn remove(s: &Services, id: StationID) {
    let map = s.map();
    unsafe {
        (*TREE.get()).remove(id, &|id, dim| coordinate(s, map, id, dim), &|keys| {
            crate::kdtree::native_split(s, keys)
        });
    }
}
pub(crate) fn radius(
    s: &Services,
    center: u32,
    radius: u32,
    mut visit: impl FnMut(*mut BaseStation),
) {
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
            &|id, dim| coordinate(s, map, id, dim),
            |id| {
                let record = crate::station_record::get_record_if_valid(s, id);
                visit(if record.is_null() {
                    std::ptr::null_mut()
                } else {
                    (*record).shell
                });
            },
        );
    }
}
pub(crate) fn move_sign(s: &Services, st: *mut BaseStation, tile: u32) {
    let commands = unsafe { &*(s.station_commands)() };
    if !crate::station_record::is_station(st.cast()) {
        (commands.waypoint_move_sign)(st, tile);
        return;
    }
    if crate::station_record::xy(st.cast()) == tile {
        return;
    }
    remove(s, crate::station_record::id(st));
    (commands.base_move_sign)(st, tile);
    insert(s, st);
}
pub(crate) fn modify_rating_around(s: &Services, tile: u32, owner: u8, amount: i32, distance: u32) {
    let map = s.map();
    radius(s, tile, distance, |st| {
        if crate::station_record::owner_company(st.cast()) == owner
            && map.distance_manhattan(tile, crate::station_record::xy(st.cast())) <= distance
        {
            unsafe {
                crate::station_service::modify_rating(
                    crate::station_record::service_of(st.cast()),
                    amount,
                );
            }
        }
    });
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_rebuild_index(s: *const Services) {
    rebuild(unsafe { &*s });
}
