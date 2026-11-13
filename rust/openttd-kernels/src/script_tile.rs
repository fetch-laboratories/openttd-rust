/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `ScriptMap` and `ScriptTile` (`script_map.cpp`, `script_tile.cpp`): validation,
//! map predicates, enumeration and command preconditions.
//!
//! Script-visible enums cross as the bits of their C++ value (`uint32_t`) and are
//! truncated here where the original casts to an in-game `uint8_t` enum. Each entry
//! loads one `World` from the script host's shared table and reads canonical tile
//! fields at the original observation points; script API calls run under the
//! game-state mutex and cannot reset or load the map. Command entries return
//! whether the native typed-command shim runs; the shim may throw `Script_Suspend`,
//! so it never runs on a Rust frame.
#![allow(unsafe_code)] // Exported symbol attributes and one canonical setting load.

use crate::map_func::World;
use crate::script_host::{self, Services};
use crate::slope_func::{
    SLOPE_ELEVATED, SLOPE_HALFTILE_MASK, SLOPE_STEEP, complement_slope_raw, is_halftile_slope_raw,
    is_steep_slope_raw,
};
use crate::tile_type::TileIndex;
use crate::tile_type::TileType;
use crate::tile_type::{MP_CLEAR, MP_HOUSE, MP_INDUSTRY, MP_STATION, MP_TREES, MP_WATER};

/// `ScriptError::ERR_PRECONDITION_FAILED`.
pub(crate) const ERR_PRECONDITION_FAILED: u32 = 2;
/// `ScriptTile::SLOPE_INVALID`.
const SLOPE_INVALID: u32 = 0xFFFF;
/// `ScriptTile::TRANSPORT_ROAD`.
const TRANSPORT_ROAD: u32 = crate::transport_type::TRANSPORT_ROAD as u32;
/// `ScriptCompany::COMPANY_INVALID`.
const COMPANY_INVALID: i32 = -1;

pub(crate) fn world(s: &Services) -> World<'_> {
    World::new(s.command.shared)
}

/// `::IsValidTile`.
pub(crate) fn valid(w: &World<'_>, tile: u32) -> bool {
    w.asserted(|m| crate::tile_map::is_valid_tile(m, TileIndex(tile), m.count()))
}

/// `::IsTileType` after a successful `IsValidTile`.
fn is_type(w: &World<'_>, tile: u32, kind: u8) -> bool {
    w.kind(TileIndex(tile)) == TileType(kind)
}

/// `ScriptCompanyMode::IsValid`.
pub(crate) fn company_mode_valid(s: &Services) -> bool {
    (s.command.shared.company_valid)(script_host::active_company())
}

/// `ScriptCompanyMode::IsDeity() || ScriptCompanyMode::IsValid()`.
pub(crate) fn deity_or_company_mode_valid(s: &Services) -> bool {
    script_host::active_company() == crate::company_type::OWNER_DEITY || company_mode_valid(s)
}

/// `EnforcePreconditionCustomError`: record `error` when `condition` fails.
pub(crate) fn enforce(condition: bool, error: u32) -> bool {
    if !condition {
        script_host::set_active_error(error);
    }
    condition
}

fn enforce_deity_or_company(s: &Services) -> bool {
    enforce(
        deity_or_company_mode_valid(s),
        script_host::ERR_PRECONDITION_INVALID_COMPANY,
    )
}

/// `ScriptMap::IsValidTile`.
fn is_valid_tile(s: &Services, tile: u32) -> bool {
    valid(&world(s), tile)
}

/// `ScriptMap::GetMapSize`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_map_size(s: &Services) -> i64 {
    i64::from(s.command.shared.map_size())
}

/// `ScriptMap::GetMapSizeX`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_map_size_x(s: &Services) -> i64 {
    i64::from(world(s).size().0)
}

/// `ScriptMap::GetMapSizeY`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_map_size_y(s: &Services) -> i64 {
    i64::from(world(s).size().1)
}

/// `ScriptMap::GetTileX`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_map_tile_x(s: &Services, tile: u32) -> i64 {
    let w = world(s);
    if !valid(&w, tile) {
        return -1;
    }
    i64::from(w.dimensions.tile_x(TileIndex(tile)))
}

/// `ScriptMap::GetTileY`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_map_tile_y(s: &Services, tile: u32) -> i64 {
    let w = world(s);
    if !valid(&w, tile) {
        return -1;
    }
    i64::from(w.dimensions.tile_y(TileIndex(tile)))
}

/// `ScriptMap::GetTileIndex`: `TileXY` of the `uint` truncations.
#[unsafe(no_mangle)]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub extern "C" fn openttd_rust_script_map_tile_index(s: &Services, x: i64, y: i64) -> u32 {
    world(s).dimensions.tile_xy(x as u32, y as u32).0
}

/// Shared validation of the two-tile `ScriptMap` distances.
fn distance(s: &Services, a: u32, b: u32, metric: impl Fn(crate::map_func::Map) -> u32) -> i64 {
    let w = world(s);
    if !valid(&w, a) || !valid(&w, b) {
        return -1;
    }
    i64::from(metric(w.dimensions))
}

/// `ScriptMap::DistanceManhattan`, also `ScriptTile::GetDistanceManhattanToTile`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_map_distance_manhattan(s: &Services, a: u32, b: u32) -> i64 {
    distance(s, a, b, |m| {
        m.distance_manhattan(TileIndex(a), TileIndex(b))
    })
}

/// `ScriptMap::DistanceMax`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_map_distance_max(s: &Services, a: u32, b: u32) -> i64 {
    distance(s, a, b, |m| m.distance_max(TileIndex(a), TileIndex(b)))
}

/// `ScriptMap::DistanceSquare`, also `ScriptTile::GetDistanceSquareToTile`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_map_distance_square(s: &Services, a: u32, b: u32) -> i64 {
    distance(s, a, b, |m| m.distance_square(TileIndex(a), TileIndex(b)))
}

/// `ScriptMap::DistanceFromEdge`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_map_distance_from_edge(s: &Services, tile: u32) -> i64 {
    let w = world(s);
    if !valid(&w, tile) {
        return -1;
    }
    i64::from(w.dimensions.distance_from_edge(TileIndex(tile), w.size_y()))
}

/// `ScriptTile::IsBuildable`.
fn is_buildable(s: &Services, tile: u32) -> bool {
    if !enforce_deity_or_company(s) {
        return false;
    }
    let w = world(s);
    if !valid(&w, tile) {
        return false;
    }
    match w.kind(TileIndex(tile)) {
        TileType::MP_CLEAR | TileType::MP_TREES => true,
        TileType::MP_WATER => w.asserted(|m| crate::water_map::is_coast(m, tile)),
        TileType::MP_ROAD => {
            /* Tram bits aren't considered buildable */
            if w.road_type(tile, true) != crate::road_type::INVALID_ROADTYPE {
                return false;
            }
            /* Depots and crossings aren't considered buildable */
            if w.asserted(|m| crate::road_map::get_road_tile_type(m, tile))
                .0
                != crate::road_map::ROAD_TILE_NORMAL
            {
                return false;
            }
            let bits =
                w.asserted(|m| crate::road_map::get_road_bits(m, tile, crate::road_type::RTT_ROAD));
            if !crate::bitmath_func::has_exactly_one_bit(bits.0) {
                return false;
            }
            if w.road_owner(tile, false) == crate::company_type::OWNER_TOWN {
                return true;
            }
            w.road_owner(tile, false) == script_host::active_company()
        }
        _ => false,
    }
}

/// `ScriptTile::IsBuildableRectangle`. The bounds compare `SQInteger` sums with
/// `uint` coordinates that wrap, as the original loop does.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn is_buildable_rectangle(s: &Services, tile: u32, width: i64, height: i64) -> bool {
    /* Check whether we can extract valid X and Y */
    if !valid(&world(s), tile) || width < 1 || height < 1 {
        return false;
    }
    let tx = openttd_rust_script_map_tile_x(s, tile) as u32;
    let ty = openttd_rust_script_map_tile_y(s, tile) as u32;
    let mut x = tx;
    while i64::from(x) < width.wrapping_add(i64::from(tx)) {
        let mut y = ty;
        while i64::from(y) < height.wrapping_add(i64::from(ty)) {
            let index = openttd_rust_script_map_tile_index(s, i64::from(x), i64::from(y));
            if !is_buildable(s, index) {
                return false;
            }
            y = y.wrapping_add(1);
        }
        x = x.wrapping_add(1);
    }
    true
}

/// A `ScriptTile` predicate that is false for invalid tiles.
fn on_valid(s: &Services, tile: u32, read: impl Fn(&World<'_>) -> bool) -> bool {
    let w = world(s);
    valid(&w, tile) && read(&w)
}

/// `ScriptTile::IsSeaTile`.
fn is_sea(s: &Services, tile: u32) -> bool {
    on_valid(s, tile, |w| {
        is_type(w, tile, MP_WATER) && w.asserted(|m| crate::water_map::is_sea(m, tile))
    })
}

/// `ScriptTile::IsRiverTile`.
fn is_river(s: &Services, tile: u32) -> bool {
    on_valid(s, tile, |w| {
        is_type(w, tile, MP_WATER) && w.asserted(|m| crate::water_map::is_river(m, tile))
    })
}

/// `ScriptTile::IsWaterTile`.
fn is_water(s: &Services, tile: u32) -> bool {
    on_valid(s, tile, |w| {
        is_type(w, tile, MP_WATER) && !w.asserted(|m| crate::water_map::is_coast(m, tile))
    })
}

/// `ScriptTile::IsCoastTile`.
fn is_coast(s: &Services, tile: u32) -> bool {
    on_valid(s, tile, |w| {
        (is_type(w, tile, MP_WATER) && w.asserted(|m| crate::water_map::is_coast(m, tile)))
            || (is_type(w, tile, MP_TREES)
                && w.asserted(|m| crate::tree_map::get_tree_ground_raw(m, tile))
                    == crate::tree_map::TREE_GROUND_SHORE)
    })
}

/// `ScriptTile::IsStationTile`.
fn is_station(s: &Services, tile: u32) -> bool {
    on_valid(s, tile, |w| is_type(w, tile, MP_STATION))
}

/// The bits `IsSteepSlope` and `IsHalftileSlope` accept, as an in-game `Slope`.
#[allow(clippy::cast_possible_truncation)]
fn game_slope(slope: u32) -> Option<u8> {
    let known = u32::from(SLOPE_ELEVATED | SLOPE_STEEP | SLOPE_HALFTILE_MASK);
    (slope & !known == 0).then_some(slope as u8)
}

/// `ScriptTile::IsSteepSlope`.
fn script_is_steep_slope(slope: u32) -> bool {
    game_slope(slope).is_some_and(is_steep_slope_raw)
}

/// `ScriptTile::IsHalftileSlope`.
fn script_is_halftile_slope(slope: u32) -> bool {
    game_slope(slope).is_some_and(is_halftile_slope_raw)
}

/// `ScriptTile::HasTreeOnTile`.
fn has_tree(s: &Services, tile: u32) -> bool {
    on_valid(s, tile, |w| is_type(w, tile, MP_TREES))
}

/// `IsTileType(MP_CLEAR) && GetClearGround(tile) == ground` of the clear-ground predicates.
fn clear_ground(s: &Services, tile: u32, ground: u8) -> bool {
    on_valid(s, tile, |w| {
        is_type(w, tile, MP_CLEAR)
            && w.asserted(|m| crate::clear_map::get_clear_ground_raw(m, tile)) == ground
    })
}

/// `ScriptTile::IsFarmTile`.
fn is_farm(s: &Services, tile: u32) -> bool {
    clear_ground(s, tile, crate::clear_map::CLEAR_FIELDS)
}

/// `ScriptTile::IsRockTile`.
fn is_rock(s: &Services, tile: u32) -> bool {
    clear_ground(s, tile, crate::clear_map::CLEAR_ROCKS)
}

/// `ScriptTile::IsRoughTile`.
fn is_rough(s: &Services, tile: u32) -> bool {
    clear_ground(s, tile, crate::clear_map::CLEAR_ROUGH)
}

/// `ScriptTile::IsSnowTile`.
fn is_snow(s: &Services, tile: u32) -> bool {
    on_valid(s, tile, |w| {
        is_type(w, tile, MP_CLEAR) && w.asserted(|m| crate::clear_map::is_snow_tile_raw(m, tile))
    })
}

/// `ScriptTile::IsDesertTile`.
fn is_desert(s: &Services, tile: u32) -> bool {
    clear_ground(s, tile, crate::clear_map::CLEAR_DESERT)
}

/// `ScriptTile::IsHouseTile`.
fn is_house(s: &Services, tile: u32) -> bool {
    on_valid(s, tile, |w| is_type(w, tile, MP_HOUSE))
}

/// `ScriptTile::GetTerrainType` over the `NewGRF` `GetTerrainType` (`TCX_NORMAL`).
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tile_terrain_type(s: &Services, tile: u32) -> u32 {
    /// `ScriptTile::TERRAIN_NORMAL`, `TERRAIN_DESERT`, `TERRAIN_RAINFOREST` and `TERRAIN_SNOW`.
    const TERRAIN: [u32; 4] = [0, 1, 2, 3];
    if !valid(&world(s), tile) {
        return TERRAIN[0];
    }
    match (s.command.shared.terrain_type)(tile) {
        1 => TERRAIN[1],
        2 => TERRAIN[2],
        4 => TERRAIN[3],
        _ => TERRAIN[0],
    }
}

/// `ScriptTile::GetSlope`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tile_slope(s: &Services, tile: u32) -> u32 {
    let w = world(s);
    if !valid(&w, tile) {
        return SLOPE_INVALID;
    }
    let (x, y) = w.size();
    u32::from((w.asserted(|m| crate::tile_map::get_tile_slope(m, TileIndex(tile), x, y))).0)
}

/// `ScriptTile::GetComplementSlope`.
#[unsafe(no_mangle)]
#[allow(clippy::cast_possible_truncation)]
pub extern "C" fn openttd_rust_script_tile_complement_slope(slope: u32) -> u32 {
    if slope & !u32::from(SLOPE_ELEVATED) != 0 {
        return SLOPE_INVALID;
    }
    u32::from(complement_slope_raw(slope as u8, |ok| {
        crate::services_assert::check(ok);
    }))
}

/// `ScriptTile::GetMinHeight`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tile_min_height(s: &Services, tile: u32) -> i64 {
    let w = world(s);
    if !valid(&w, tile) {
        return -1;
    }
    let (x, y) = w.size();
    i64::from(w.asserted(|m| crate::tile_map::get_tile_z(m, TileIndex(tile), x, y)))
}

/// `ScriptTile::GetMaxHeight`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tile_max_height(s: &Services, tile: u32) -> i64 {
    let w = world(s);
    if !valid(&w, tile) {
        return -1;
    }
    let (x, y) = w.size();
    i64::from(w.asserted(|m| crate::tile_map::get_tile_max_z(m, TileIndex(tile), x, y)))
}

/// `ScriptTile::GetCornerHeight`; `(::Corner)corner` truncates to the `uint8_t` enum.
#[unsafe(no_mangle)]
#[allow(clippy::cast_possible_truncation)]
pub extern "C" fn openttd_rust_script_tile_corner_height(
    s: &Services,
    tile: u32,
    corner: u32,
) -> i64 {
    let corner = corner as u8;
    let map = world(s);
    if !valid(&map, tile) || !crate::slope_func::is_valid_corner_raw(corner) {
        return -1;
    }
    let (size_x, size_y) = map.size();
    let (slope, z) =
        map.asserted(|m| crate::tile_map::get_tile_slope_z(m, TileIndex(tile), size_x, size_y));
    let check = |condition| map.check(condition);
    i64::from(z.wrapping_add(crate::landscape::get_slope_z_in_corner(
        slope.0, corner, check,
    )))
}

/// `ScriptTile::GetOwner`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tile_owner(s: &Services, tile: u32) -> i32 {
    let w = world(s);
    if !valid(&w, tile) || is_type(&w, tile, MP_HOUSE) || is_type(&w, tile, MP_INDUSTRY) {
        return COMPANY_INVALID;
    }
    let owner = w.owner(TileIndex(tile));
    resolve_company_id(s, to_script_company_id(owner.0))
}

/// `ScriptCompany::ToScriptCompanyID`.
fn to_script_company_id(company: u8) -> i32 {
    if company == crate::company_type::INVALID_OWNER {
        return COMPANY_INVALID;
    }
    i32::from(company)
}

/// `ScriptCompany::COMPANY_SELF`.
const COMPANY_SELF: i32 = 254;
/// `ScriptCompany::COMPANY_SPECTATOR`.
const COMPANY_SPECTATOR: i32 = 255;

/// `ScriptCompany::FromScriptCompanyID`; `static_cast<::CompanyID>` keeps the low byte.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn from_script_company_id(s: &Services, company: i32) -> u8 {
    crate::movement::native_assert(
        s.command.shared,
        company != COMPANY_SELF && company != COMPANY_SPECTATOR,
    );
    if company == COMPANY_INVALID {
        return crate::company_type::INVALID_OWNER;
    }
    company as u8
}

/// `ScriptCompany::ResolveCompanyID`. `COMPANY_SELF` reads `_current_company`, not
/// the script's company mode.
fn resolve_company_id(s: &Services, company: i32) -> i32 {
    let shared = s.command.shared;
    if company == COMPANY_SELF {
        let current = (shared.current_company)();
        if !(shared.company_valid)(current) {
            return COMPANY_INVALID;
        }
        return to_script_company_id(current);
    }
    if (shared.company_valid)(from_script_company_id(s, company)) {
        company
    } else {
        COMPANY_INVALID
    }
}

/// `ScriptTile::HasTransportType`; `(::TransportType)` truncates to the `uint8_t` enum.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn has_transport_type(s: &Services, tile: u32, transport_type: u32) -> bool {
    use crate::track_func::track_status_to_trackdir_bits;
    if !valid(&world(s), tile) {
        return false;
    }
    let mode = transport_type as u8;
    let side = crate::direction_type::INVALID_DIAGDIR;
    let status = |sub| crate::movement::track_status(s.command.shared, tile, mode, sub, side);
    if transport_type == TRANSPORT_ROAD {
        track_status_to_trackdir_bits(status(0)) != 0
            || track_status_to_trackdir_bits(status(1)) != 0
    } else {
        track_status_to_trackdir_bits(status(0)) != 0
    }
}

/// `ScriptTile::GetBuildCost`: `GetPrice(price, 1, nullptr)`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tile_build_cost(build_type: u32) -> i64 {
    use crate::economy_type::{
        PR_BUILD_FOUNDATION, PR_BUILD_TREES, PR_CLEAR_FIELDS, PR_CLEAR_GRASS, PR_CLEAR_HOUSE,
        PR_CLEAR_ROCKS, PR_CLEAR_ROUGH, PR_CLEAR_WATER, PR_TERRAFORM,
    };
    /// Prices of `BT_FOUNDATION` to `BT_CLEAR_WATER`.
    const PRICES: [usize; 9] = [
        PR_BUILD_FOUNDATION,
        PR_TERRAFORM,
        PR_BUILD_TREES,
        PR_CLEAR_GRASS,
        PR_CLEAR_ROUGH,
        PR_CLEAR_ROCKS,
        PR_CLEAR_FIELDS,
        PR_CLEAR_HOUSE,
        PR_CLEAR_WATER,
    ];
    match PRICES.get(build_type as usize) {
        #[allow(clippy::cast_possible_truncation)]
        Some(&price) => crate::economy_func::get_price(price as u32, 1, 0, 0),
        None => -1,
    }
}

/// `ScriptTile::PlantTreeRectangle` preconditions after the native company-mode
/// check; the end tile on success, else `INVALID_TILE`.
#[unsafe(no_mangle)]
#[allow(clippy::cast_possible_truncation)]
pub extern "C" fn openttd_rust_script_tile_plant_tree_rectangle(
    s: &Services,
    tile: u32,
    width: i64,
    height: i64,
) -> u32 {
    let refused = crate::tile_type::INVALID_TILE;
    if !enforce(valid(&world(s), tile), ERR_PRECONDITION_FAILED)
        || !enforce((1..=20).contains(&width), ERR_PRECONDITION_FAILED)
        || !enforce((1..=20).contains(&height), ERR_PRECONDITION_FAILED)
    {
        return refused;
    }
    let w = world(s);
    // SAFETY: the canonical setting outlives the entry; read where `TileAddWrap` reads it.
    let freeform = unsafe { w.shared.settings().freeform_edges.read() };
    let end = w
        .dimensions
        .tile_add_wrap(
            TileIndex(tile),
            (width - 1) as i32,
            (height - 1) as i32,
            w.size_y(),
            freeform,
        )
        .0;
    if enforce(valid(&w, end), ERR_PRECONDITION_FAILED) {
        end
    } else {
        refused
    }
}

/// `is_valid_tile` as a C byte.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_map_is_valid_tile(s: &Services, tile: u32) -> u8 {
    u8::from(is_valid_tile(s, tile))
}

/// `is_buildable` as a C byte.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tile_is_buildable(s: &Services, tile: u32) -> u8 {
    u8::from(is_buildable(s, tile))
}

/// `is_buildable_rectangle` as a C byte.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tile_is_buildable_rectangle(
    s: &Services,
    tile: u32,
    width: i64,
    height: i64,
) -> u8 {
    u8::from(is_buildable_rectangle(s, tile, width, height))
}

/// `is_sea` as a C byte.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tile_is_sea(s: &Services, tile: u32) -> u8 {
    u8::from(is_sea(s, tile))
}

/// `is_river` as a C byte.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tile_is_river(s: &Services, tile: u32) -> u8 {
    u8::from(is_river(s, tile))
}

/// `is_water` as a C byte.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tile_is_water(s: &Services, tile: u32) -> u8 {
    u8::from(is_water(s, tile))
}

/// `is_coast` as a C byte.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tile_is_coast(s: &Services, tile: u32) -> u8 {
    u8::from(is_coast(s, tile))
}

/// `is_station` as a C byte.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tile_is_station(s: &Services, tile: u32) -> u8 {
    u8::from(is_station(s, tile))
}

/// `script_is_steep_slope` as a C byte.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tile_is_steep_slope(slope: u32) -> u8 {
    u8::from(script_is_steep_slope(slope))
}

/// `script_is_halftile_slope` as a C byte.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tile_is_halftile_slope(slope: u32) -> u8 {
    u8::from(script_is_halftile_slope(slope))
}

/// `has_tree` as a C byte.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tile_has_tree(s: &Services, tile: u32) -> u8 {
    u8::from(has_tree(s, tile))
}

/// `is_farm` as a C byte.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tile_is_farm(s: &Services, tile: u32) -> u8 {
    u8::from(is_farm(s, tile))
}

/// `is_rock` as a C byte.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tile_is_rock(s: &Services, tile: u32) -> u8 {
    u8::from(is_rock(s, tile))
}

/// `is_rough` as a C byte.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tile_is_rough(s: &Services, tile: u32) -> u8 {
    u8::from(is_rough(s, tile))
}

/// `is_snow` as a C byte.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tile_is_snow(s: &Services, tile: u32) -> u8 {
    u8::from(is_snow(s, tile))
}

/// `is_desert` as a C byte.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tile_is_desert(s: &Services, tile: u32) -> u8 {
    u8::from(is_desert(s, tile))
}

/// `is_house` as a C byte.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tile_is_house(s: &Services, tile: u32) -> u8 {
    u8::from(is_house(s, tile))
}

/// `has_transport_type` as a C byte.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tile_has_transport_type(
    s: &Services,
    tile: u32,
    transport_type: u32,
) -> u8 {
    u8::from(has_transport_type(s, tile, transport_type))
}

/// The `ScriptTileList` rectangle and single-tile edits: valid corners only, and
/// the `OrthogonalTileArea` tiles in row-major order.
/// # Safety
/// `owner` is the live list of the calling object.
unsafe fn edit_tiles(
    s: &Services,
    owner: *mut crate::script_list::List,
    t1: u32,
    t2: u32,
    add: bool,
) {
    let w = world(s);
    if !valid(&w, t1) || !valid(&w, t2) {
        return;
    }
    let check = |ok| w.check(ok);
    let area = crate::tilearea::TileArea::from_corners(
        w.dimensions,
        w.types.count(),
        TileIndex(t1),
        TileIndex(t2),
        check,
    );
    for tile in area.tiles_raw(w.dimensions) {
        // SAFETY: caller contract.
        unsafe {
            if add {
                (*owner).add(i64::from(tile), 0);
            } else {
                (*owner).remove(i64::from(tile));
            }
        }
    }
}

/// `ScriptTileList::AddRectangle` (`add`) and `RemoveRectangle`.
/// # Safety
/// `owner` is the live list of the calling object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_script_tile_list_rectangle(
    s: &Services,
    owner: *mut crate::script_list::List,
    t1: u32,
    t2: u32,
    add: u8,
) {
    // SAFETY: caller contract.
    unsafe { edit_tiles(s, owner, t1, t2, add != 0) };
}

/// `ScriptTileList::AddTile` (`add`) and `RemoveTile`.
/// # Safety
/// `owner` is the live list of the calling object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_script_tile_list_tile(
    s: &Services,
    owner: *mut crate::script_list::List,
    tile: u32,
    add: u8,
) {
    if !valid(&world(s), tile) {
        return;
    }
    // SAFETY: caller contract.
    unsafe {
        if add != 0 {
            (*owner).add(i64::from(tile), 0);
        } else {
            (*owner).remove(i64::from(tile));
        }
    }
}
