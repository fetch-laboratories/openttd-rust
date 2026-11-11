/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `ScriptMarine` (`script_marine.cpp`): water depot, dock, buoy, lock and canal
//! tile queries, water connectivity, build costs and the water-depot
//! preconditions. The module contract is `script_tile.rs`'s.
#![allow(unsafe_code)] // Exported symbol attributes only.

use crate::map_func::World;
use crate::script_host::Services;
use crate::script_tile::{ERR_PRECONDITION_FAILED, enforce, valid, world};
use crate::tile_type::{MP_STATION, MP_WATER};
use crate::water_map::{WATER_TILE_DEPOT, WATER_TILE_LOCK};

/// The refused result of the byte-valued command entries.
const REFUSED: u8 = 0xFF;

/// `IsTileType(tile, kind)` and `read` on a valid tile.
fn on_type(s: &Services, tile: u32, kind: u8, read: impl Fn(&World<'_>) -> bool) -> bool {
    let w = world(s);
    valid(&w, tile) && w.kind(tile) == kind && read(&w)
}

fn water_tile_type(w: &World<'_>, tile: u32) -> u8 {
    w.asserted(|m| crate::water_map::get_water_tile_type(m, tile))
}

fn water_depot_tile(s: &Services, tile: u32) -> bool {
    on_type(s, tile, MP_WATER, |w| {
        water_tile_type(w, tile) == WATER_TILE_DEPOT
    })
}

fn dock_tile(s: &Services, tile: u32) -> bool {
    on_type(s, tile, MP_STATION, |w| {
        w.asserted(|m| crate::station_map::is_dock(m, crate::tile_type::TileIndex(tile)))
    })
}

fn buoy_tile(s: &Services, tile: u32) -> bool {
    on_type(s, tile, MP_STATION, |w| {
        w.asserted(|m| crate::station_map::is_buoy(m, crate::tile_type::TileIndex(tile)))
    })
}

fn lock_tile(s: &Services, tile: u32) -> bool {
    on_type(s, tile, MP_WATER, |w| {
        water_tile_type(w, tile) == WATER_TILE_LOCK
    })
}

/// `IsCanal`: `IsWater(t) && GetWaterClass(t) == WaterClass::Canal`.
fn canal_tile(s: &Services, tile: u32) -> bool {
    on_type(s, tile, MP_WATER, |w| {
        w.asserted(|m| crate::water_map::is_canal(m, tile))
    })
}

/// `ScriptMarine::AreWaterTilesConnected`.
fn water_tiles_connected(s: &Services, t1: u32, t2: u32) -> bool {
    use crate::track_func::{diagdir_reaches_tracks, track_status_to_track_bits};
    let w = world(s);
    if !valid(&w, t1) || !valid(&w, t2) {
        return false;
    }
    /* Tiles not neighbouring */
    if w.dimensions.distance_manhattan(t1, t2) != 1 {
        return false;
    }
    let check = |ok| w.check(ok);
    let water = crate::transport_type::TRANSPORT_WATER;
    let reachable = |tile, to_other: u8| {
        let side = crate::direction_func::reverse_diag_dir_raw(to_other, check);
        let status = crate::movement::track_status(s.command.shared, tile, water, 0, side);
        track_status_to_track_bits(status) & diagdir_reaches_tracks(to_other)
    };
    let to_other_tile = w
        .dimensions
        .diagdir_between_tiles(
            crate::tile_type::TileIndex(t2),
            crate::tile_type::TileIndex(t1),
        )
        .0;
    /* Determine the reachable tracks from the shared edge */
    if reachable(t1, to_other_tile) == crate::track_type::TRACK_BIT_NONE {
        return false;
    }
    let to_other_tile = crate::direction_func::reverse_diag_dir_raw(to_other_tile, check);
    reachable(t2, to_other_tile) != crate::track_type::TRACK_BIT_NONE
}

/// `ScriptMarine::GetBuildCost`: `GetPrice(price, 1, nullptr)`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_marine_build_cost(build_type: u32) -> i64 {
    use crate::economy_type::{
        PR_BUILD_CANAL, PR_BUILD_DEPOT_SHIP, PR_BUILD_LOCK, PR_BUILD_STATION_DOCK,
        PR_BUILD_WAYPOINT_BUOY,
    };
    /// Prices of `BT_DOCK` to `BT_CANAL`.
    const PRICES: [usize; 5] = [
        PR_BUILD_STATION_DOCK,
        PR_BUILD_DEPOT_SHIP,
        PR_BUILD_WAYPOINT_BUOY,
        PR_BUILD_LOCK,
        PR_BUILD_CANAL,
    ];
    match PRICES.get(build_type as usize) {
        #[allow(clippy::cast_possible_truncation)]
        Some(&price) => crate::economy_func::get_price(price as u32, 1, 0, 0),
        None => -1,
    }
}

/// `ScriptMarine::BuildWaterDepot` preconditions after the native company-mode
/// check; the depot `Axis`, or `REFUSED`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_marine_water_depot(
    s: &Services,
    tile: u32,
    front: u32,
) -> u8 {
    let w = world(s);
    let m = w.dimensions;
    if !enforce(valid(&w, tile), ERR_PRECONDITION_FAILED)
        || !enforce(valid(&w, front), ERR_PRECONDITION_FAILED)
        || !enforce(
            (m.tile_x(front) == m.tile_x(tile)) != (m.tile_y(front) == m.tile_y(tile)),
            ERR_PRECONDITION_FAILED,
        )
    {
        return REFUSED;
    }
    if m.tile_x(front) == m.tile_x(tile) {
        crate::direction_type::AXIS_Y
    } else {
        crate::direction_type::AXIS_X
    }
}

/// C byte exports of the boolean entries.
macro_rules! byte_entries {
    ($($(#[$doc:meta])* $name:ident => $inner:ident($($arg:ident: $ty:ty),*);)*) => {$(
        $(#[$doc])*
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(s: &Services, $($arg: $ty),*) -> u8 {
            u8::from($inner(s, $($arg),*))
        }
    )*};
}

byte_entries! {
    /// `ScriptMarine::IsWaterDepotTile`.
    openttd_rust_script_marine_is_water_depot_tile => water_depot_tile(tile: u32);
    /// `ScriptMarine::IsDockTile`.
    openttd_rust_script_marine_is_dock_tile => dock_tile(tile: u32);
    /// `ScriptMarine::IsBuoyTile`.
    openttd_rust_script_marine_is_buoy_tile => buoy_tile(tile: u32);
    /// `ScriptMarine::IsLockTile`.
    openttd_rust_script_marine_is_lock_tile => lock_tile(tile: u32);
    /// `ScriptMarine::IsCanalTile`.
    openttd_rust_script_marine_is_canal_tile => canal_tile(tile: u32);
    /// `ScriptMarine::AreWaterTilesConnected`.
    openttd_rust_script_marine_tiles_connected => water_tiles_connected(t1: u32, t2: u32);
}
