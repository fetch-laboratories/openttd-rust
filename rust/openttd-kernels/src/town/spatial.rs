/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Simulation town index. The game-state mutex serializes every access, including
//! load rebuilds. Coordinate services read live records and cannot reenter this index.
use super::TownRef;
use crate::tile_type::{TileIndex, TileType};
use crate::{game_state::GameState, kdtree::Kdtree, services::Services, town_type::TownId};
use std::ptr;
static TREE: GameState<Kdtree<TownId>> = GameState::new(Kdtree::new());

/// Kd-tree coordinate of a town: `TileX` or `TileY` of its `xy`.
fn coordinate(s: &Services, map: crate::map_func::Map, id: TownId, dim: usize) -> u16 {
    let owner = (s.town_owner_by_id)(u32::from(id.0));
    let tile = unsafe { super::position(owner) };
    if dim == 0 {
        map.tile_x(tile.0) as u16
    } else {
        map.tile_y(tile.0) as u16
    }
}
/// `RebuildTownKdtree`.
pub(crate) fn rebuild_town_kdtree(s: &Services) {
    let map = s.map();
    let ids = super::iterate(|from| (s.town_next_id)(from));
    unsafe {
        (*TREE.get()).build(ids, &|id, dim| coordinate(s, map, id, dim), &|keys| {
            crate::kdtree::native_split(s, keys)
        });
    }
}
/// `_town_kdtree.Insert`.
pub(crate) fn insert(s: &Services, id: TownId) {
    let map = s.map();
    unsafe {
        (*TREE.get()).insert(id, &|id, dim| coordinate(s, map, id, dim), &|keys| {
            crate::kdtree::native_split(s, keys)
        });
    }
}
/// `_town_kdtree.Remove`.
pub(crate) fn remove(s: &Services, id: TownId) {
    let map = s.map();
    unsafe {
        (*TREE.get()).remove(id, &|id, dim| coordinate(s, map, id, dim), &|keys| {
            crate::kdtree::native_split(s, keys)
        });
    }
}
/// `_town_kdtree.FindNearest(TileX(tile), TileY(tile))`.
fn find_nearest(s: &Services, tile: TileIndex) -> TownId {
    let map = s.map();
    unsafe {
        (*TREE.get()).nearest(
            [map.tile_x(tile.0) as u16, map.tile_y(tile.0) as u16],
            &|id, dim| coordinate(s, map, id, dim),
        )
    }
}
/// Determines if a town is close to a tile (`IsCloseToTown`).
/// `dist` is the maximum distance to be accepted.
/// Return true if the tile is within the specified distance.
pub(crate) fn is_close_to_town(s: &Services, tile: TileIndex, dist: u32) -> bool {
    if unsafe { (*TREE.get()).count() } == 0 {
        return false;
    }
    let t = find_nearest(s, tile);
    let xy = unsafe { super::position((s.town_owner_by_id)(u32::from(t.0))) };
    s.map().distance_manhattan(tile.0, xy.0) < dist
}
/// Return the town closest to the given tile within `threshold`
/// (`CalcClosestTownFromTile`), or null if there is no such town.
/// This function only uses distance; `closest_town_from_tile` also takes town
/// ownership into account.
pub(crate) fn calc_closest_town_from_tile(
    s: &Services,
    tile: TileIndex,
    threshold: u32,
) -> *const crate::services::TownHandle {
    if (s.town_count)() == 0 {
        return ptr::null();
    }
    let tid = find_nearest(s, tile);
    let xy = unsafe { super::position((s.town_owner_by_id)(u32::from(tid.0))) };
    if s.map().distance_manhattan(tile.0, xy.0) < threshold {
        (s.town_by_id)(u32::from(tid.0))
    } else {
        ptr::null()
    }
}
/// Raw adapter for `calc_closest_town_from_tile`, retained for callers outside this pass.
pub(crate) fn calc_closest(
    s: &Services,
    tile: u32,
    threshold: u32,
) -> *const crate::services::TownHandle {
    calc_closest_town_from_tile(s, TileIndex(tile), threshold)
}
/// Return the town closest (in distance or ownership) to a given tile, within a
/// given threshold (`ClosestTownFromTile`), or null if there is no such town.
/// If you only care about distance, use `calc_closest_town_from_tile`.
pub(crate) fn closest_town_from_tile(
    s: &Services,
    tile: TileIndex,
    threshold: u32,
) -> *const crate::services::TownHandle {
    let world = crate::map_func::World::new(s);
    match TileType(world.kind(tile.0)) {
        TileType::MP_ROAD => {
            if world.road_depot(tile.0) {
                return calc_closest_town_from_tile(s, tile, threshold);
            }
            if !crate::road_map::has_town_owned_road(&world, tile.0) {
                let tid = TownId((s.town_index)(tile.0));
                if tid == TownId::INVALID {
                    // In the case we are generating "many random towns", this value may be
                    // TownID::Invalid().
                    if (s.clear_square_services.generating_world)() {
                        return calc_closest_town_from_tile(s, tile, threshold);
                    }
                    world.check((s.town_count)() == 0);
                    return ptr::null();
                }
                world.check((s.town_next_id)(u32::from(tid.0)) == u32::from(tid.0));
                let town = (s.town_by_id)(u32::from(tid.0));
                let xy = unsafe { super::position((s.town_owner)(town)) };
                if world.dimensions.distance_manhattan(tile.0, xy.0) >= threshold {
                    return ptr::null();
                }
                return town;
            }
            // Fall through to `MP_HOUSE`.
            (s.town_by_id)(u32::from((s.town_index)(tile.0)))
        }
        TileType::MP_HOUSE => (s.town_by_id)(u32::from((s.town_index)(tile.0))),
        _ => calc_closest_town_from_tile(s, tile, threshold),
    }
}
/// Raw adapter for `closest_town_from_tile`, retained for callers outside this pass.
pub(crate) fn closest(
    s: &Services,
    tile: u32,
    threshold: u32,
) -> *const crate::services::TownHandle {
    closest_town_from_tile(s, TileIndex(tile), threshold)
}
/// `GetRandomTown`: a uniformly chosen live town, or an empty reference without towns.
pub(crate) fn get_random_town(s: &Services) -> TownRef {
    let count = (s.town_count)();
    if count == 0 {
        return TownRef {
            shell: ptr::null_mut(),
            state: ptr::null_mut(),
            id: TownId::INVALID,
        };
    }
    let mut num = s.random_range(u32::from(count as u16));
    let mut id = (s.town_next_id)(u32::from(TownId::BEGIN.0));
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
    rebuild_town_kdtree(unsafe { &*s });
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_calc_closest(
    s: *const Services,
    tile: u32,
    threshold: u32,
) -> *const crate::services::TownHandle {
    calc_closest_town_from_tile(unsafe { &*s }, TileIndex(tile), threshold)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_closest(
    s: *const Services,
    tile: u32,
    threshold: u32,
) -> *const crate::services::TownHandle {
    closest_town_from_tile(unsafe { &*s }, TileIndex(tile), threshold)
}
