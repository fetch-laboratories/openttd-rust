/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Station simulation index. All accesses hold the game-state mutex. Coordinate
//! readers and radius visitors cannot mutate this index; no tree borrow spans
//! presentation reentry from the qualified native `BaseStation::MoveSign`.
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
use super::{BaseStation, CatchmentServices};
use crate::company_type::{CompanyID, Owner};
use crate::tile_type::TileIndex;
use crate::{game_state::GameState, kdtree::Kdtree, services::Services, station_type::StationID};
static TREE: GameState<Kdtree<StationID>> = GameState::new(Kdtree::new());
fn table(s: &Services) -> &CatchmentServices {
    unsafe { &*(s.station_catchment)() }
}
/// `Kdtree_StationXYFunc`: the sign tile's X or Y coordinate.
fn coordinate(c: &CatchmentServices, map: crate::map_func::Map, id: StationID, dim: usize) -> u16 {
    let tile = (c.location)((c.station_if_valid)(id));
    if dim == 0 {
        map.tile_x(TileIndex(tile)) as u16
    } else {
        map.tile_y(TileIndex(tile)) as u16
    }
}
/// `RebuildStationKdtree`: rebuild the station index from every station in
/// pool order.
pub(crate) fn rebuild_station_kdtree(s: &Services) {
    let c = table(s);
    let map = s.map();
    let mut st = (c.next_station)(u32::from(StationID::BEGIN.0));
    let ids = std::iter::from_fn(|| {
        if st.is_null() {
            return None;
        }
        let id = (c.id)(st);
        st = (c.next_station)(u32::from(id.0) + 1);
        Some(id)
    });
    unsafe {
        (*TREE.get()).build(ids, &|id, dim| coordinate(c, map, id, dim), &|keys| {
            crate::kdtree::native_split(s, keys)
        });
    }
}
/// `_station_kdtree.Insert(st->index)`.
pub(crate) fn insert(s: &Services, st: *mut BaseStation) {
    let c = table(s);
    let map = s.map();
    unsafe {
        (*TREE.get()).insert(
            (c.id)(st),
            &|id, dim| coordinate(c, map, id, dim),
            &|keys| crate::kdtree::native_split(s, keys),
        );
    }
}
/// `_station_kdtree.Remove(id)`.
pub(crate) fn remove(s: &Services, id: StationID) {
    let c = table(s);
    let map = s.map();
    unsafe {
        (*TREE.get()).remove(id, &|id, dim| coordinate(c, map, id, dim), &|keys| {
            crate::kdtree::native_split(s, keys)
        });
    }
}
/// `ForAllStationsRadius`: call a function on all stations whose sign is
/// within a radius of a center tile.
pub(crate) fn for_all_stations_radius(
    s: &Services,
    center: TileIndex,
    radius: u32,
    mut func: impl FnMut(*mut BaseStation),
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
            |id| func((c.station_if_valid)(id)),
        );
    }
}
/// `Station::MoveSign`, dispatching a waypoint to `Waypoint::MoveSign`.
pub(crate) fn station_move_sign(s: &Services, st: *mut BaseStation, new_xy: TileIndex) {
    let commands = unsafe { &*(s.station_commands)() };
    let c = table(s);
    let station = unsafe { &*commands.station };
    // The virtual `MoveSign` of a waypoint.
    if !(station.is_station)(st) {
        (commands.waypoint_move_sign)(st, new_xy.0);
        return;
    }
    if (c.location)(st) == new_xy.0 {
        return;
    }
    remove(s, (c.id)(st));
    (commands.base_move_sign)(st, new_xy.0);
    insert(s, st);
}
/// `ModifyStationRatingAround`: forcibly modify station ratings near a given
/// tile. Used when a crash hurts a company's station ratings nearby, or when
/// local authority actions affect nearby ratings.
pub(crate) fn modify_station_rating_around(
    s: &Services,
    tile: TileIndex,
    owner: Owner,
    amount: i32,
    radius: u32,
) {
    let c = table(s);
    let commands = unsafe { &*(s.station_commands)() };
    let station = unsafe { &*commands.station };
    let map = s.map();
    for_all_stations_radius(s, tile, radius, |st| {
        if CompanyID((station.station_owner)(st)) == owner
            && map.distance_manhattan(tile, TileIndex((c.location)(st))) <= radius
        {
            unsafe {
                crate::station_service::modify_rating((c.owner)(st), amount);
            }
        }
    });
}
/// Temporary raw adapter for callers outside workset 03.
pub(crate) fn modify_rating_around(s: &Services, tile: u32, owner: u8, amount: i32, distance: u32) {
    modify_station_rating_around(s, TileIndex(tile), CompanyID(owner), amount, distance);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_rebuild_index(s: *const Services) {
    rebuild_station_kdtree(unsafe { &*s });
}
