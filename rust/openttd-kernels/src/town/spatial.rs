/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Simulation town index. The game-state mutex serializes every access, including
//! load rebuilds. Coordinate services read live records and cannot reenter this index.
use super::TownRef;
use crate::{game_state::GameState, kdtree::Kdtree, services::Services, town_type::TownId};
use std::ptr;
static TREE: GameState<Kdtree<TownId>> = GameState::new(Kdtree::new());

fn coordinate(s: &Services, map: crate::map_func::Map, id: TownId, dim: usize) -> u16 {
    let owner = (s.town_owner_by_id)(u32::from(id.0));
    let tile = unsafe { super::position(owner) };
    if dim == 0 {
        map.tile_x(tile) as u16
    } else {
        map.tile_y(tile) as u16
    }
}
pub(crate) fn rebuild(s: &Services) {
    let map = s.map();
    let mut id = (s.town_next_id)(0);
    let ids = std::iter::from_fn(|| {
        if id == u32::MAX {
            return None;
        }
        let town = TownId(id as u16);
        id = (s.town_next_id)(id + 1);
        Some(town)
    });
    unsafe {
        (*TREE.get()).build(ids, &|id, dim| coordinate(s, map, id, dim), &|keys| {
            crate::kdtree::native_split(s, keys)
        });
    }
}
pub(crate) fn insert(s: &Services, id: TownId) {
    let map = s.map();
    unsafe {
        (*TREE.get()).insert(id, &|id, dim| coordinate(s, map, id, dim), &|keys| {
            crate::kdtree::native_split(s, keys)
        });
    }
}
pub(crate) fn remove(s: &Services, id: TownId) {
    let map = s.map();
    unsafe {
        (*TREE.get()).remove(id, &|id, dim| coordinate(s, map, id, dim), &|keys| {
            crate::kdtree::native_split(s, keys)
        });
    }
}
fn nearest(s: &Services, tile: u32) -> TownId {
    let map = s.map();
    unsafe {
        (*TREE.get()).nearest(
            [map.tile_x(tile) as u16, map.tile_y(tile) as u16],
            &|id, dim| coordinate(s, map, id, dim),
        )
    }
}
pub(crate) fn close(s: &Services, tile: u32, distance: u32) -> bool {
    if unsafe { (*TREE.get()).count() } == 0 {
        return false;
    }
    let id = nearest(s, tile);
    let xy = unsafe { super::position((s.town_owner_by_id)(u32::from(id.0))) };
    s.map().distance_manhattan(tile, xy) < distance
}
pub(crate) fn calc_closest(
    s: &Services,
    tile: u32,
    threshold: u32,
) -> *const crate::services::TownHandle {
    if (s.town_count)() == 0 {
        return ptr::null();
    }
    let id = nearest(s, tile);
    let xy = unsafe { super::position((s.town_owner_by_id)(u32::from(id.0))) };
    if s.map().distance_manhattan(tile, xy) < threshold {
        (s.town_by_id)(u32::from(id.0))
    } else {
        ptr::null()
    }
}
pub(crate) fn closest(
    s: &Services,
    tile: u32,
    threshold: u32,
) -> *const crate::services::TownHandle {
    use crate::tile_type::{MP_HOUSE, MP_ROAD};
    let world = crate::map_func::World::new(s);
    match world.kind(tile) {
        MP_ROAD => {
            if world.road_depot(tile) {
                return calc_closest(s, tile, threshold);
            }
            if !crate::road_map::has_town_owned_road(&world, tile) {
                let id = (s.town_index)(tile);
                if id == TownId::INVALID.0 {
                    if (s.clear_square_services.generating_world)() {
                        return calc_closest(s, tile, threshold);
                    }
                    world.check((s.town_count)() == 0);
                    return ptr::null();
                }
                world.check((s.town_next_id)(u32::from(id)) == u32::from(id));
                let town = (s.town_by_id)(u32::from(id));
                let xy = unsafe { super::position((s.town_owner)(town)) };
                if world.dimensions.distance_manhattan(tile, xy) >= threshold {
                    return ptr::null();
                }
                return town;
            }
            (s.town_by_id)(u32::from((s.town_index)(tile)))
        }
        MP_HOUSE => (s.town_by_id)(u32::from((s.town_index)(tile))),
        _ => calc_closest(s, tile, threshold),
    }
}
pub(crate) fn random(s: &Services) -> TownRef {
    let count = (s.town_count)();
    if count == 0 {
        return TownRef {
            shell: ptr::null_mut(),
            state: ptr::null_mut(),
            id: TownId::INVALID,
        };
    }
    let mut num = s.random_range(u32::from(count as u16));
    let mut id = (s.town_next_id)(0);
    while num != 0 {
        num -= 1;
        id = (s.town_next_id)(id + 1);
    }
    TownRef {
        shell: (s.town_by_id)(id),
        state: (s.town_owner_by_id)(id),
        id: TownId(id as u16),
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_rebuild_index(s: *const Services) {
    rebuild(unsafe { &*s });
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_calc_closest(
    s: *const Services,
    tile: u32,
    threshold: u32,
) -> *const crate::services::TownHandle {
    calc_closest(unsafe { &*s }, tile, threshold)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_closest(
    s: *const Services,
    tile: u32,
    threshold: u32,
) -> *const crate::services::TownHandle {
    closest(unsafe { &*s }, tile, threshold)
}
