/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `ScriptRoad` (`script_road.cpp`): road tile queries, road type availability,
//! connected road part lookups and the preconditions and arguments of the
//! multi-check command wrappers. The module contract is `script_tile.rs`'s.
//! Command wrappers keep their native mode macro first.
#![allow(unsafe_code)] // Exported symbol attributes and the borrowed tile span.

use crate::direction_type::DiagDirection;
use crate::map_func::World;
use crate::road_type::{
    DRD_BOTH, DRD_NONE, DRD_NORTHBOUND, DRD_SOUTHBOUND, ROAD_E, ROAD_N, ROAD_NE, ROAD_NONE,
    ROAD_NW, ROAD_S, ROAD_SE, ROAD_SW, ROAD_X, ROAD_Y,
};
use crate::script_host::{self, ERR_PRECONDITION_INVALID_COMPANY, Services};
use crate::script_tile::{
    ERR_PRECONDITION_FAILED, deity_or_company_mode_valid, enforce, valid, world,
};
use crate::slope_func::{
    SLOPE_EW, SLOPE_FLAT, SLOPE_NE, SLOPE_NW, SLOPE_SE, SLOPE_SW, SLOPE_W, SLOPE_WSE,
};
use crate::tile_type::TileIndex;
use crate::tile_type::TileType;

/// `ScriptRoad::ROADVEHTYPE_BUS` and `ROADVEHTYPE_TRUCK`.
const ROADVEHTYPE_BUS: u32 = 0;
const ROADVEHTYPE_TRUCK: u32 = 1;
/// The refused result of the byte-valued command entries.
const REFUSED: u8 = 0xFF;

fn check<'a>(w: &'a World<'a>) -> impl Fn(bool) + Copy + 'a {
    move |ok| w.check(ok)
}

/// `ScriptRoad::IsRoadTypeAvailable`; `(::RoadType)` truncates to the `uint8_t` enum.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn road_type_available(s: &Services, road_type: u32) -> bool {
    if !enforce(
        deity_or_company_mode_valid(s),
        ERR_PRECONDITION_INVALID_COMPANY,
    ) {
        return false;
    }
    let road_type = road_type as u8;
    let shared = s.command.shared;
    road_type < crate::road_type::ROADTYPE_END
        && crate::road::availability::has_road_type(
            shared,
            crate::type_availability::services(shared),
            script_host::active_company(),
            road_type,
        )
}

/// `ScriptRoad::GetCurrentRoadType`.
pub(crate) fn current_road_type() -> u32 {
    u32::from(script_host::active_road_type())
}

/// `ScriptRoad::RoadVehHasPowerOnRoad` and its `RoadVehCanRunOnRoad` forwarder.
#[allow(clippy::cast_possible_truncation)]
fn road_veh_has_power(s: &Services, engine_type: u32, road_type: u32) -> bool {
    road_type_available(s, engine_type)
        && road_type_available(s, road_type)
        && (s.command.shared.road_powered)(engine_type as u8, road_type as u8)
}

/// `ScriptRoad::GetMaxSpeed`.
#[unsafe(no_mangle)]
#[allow(clippy::cast_possible_truncation)]
pub extern "C" fn openttd_rust_script_road_max_speed(s: &Services, road_type: u32) -> i64 {
    if !road_type_available(s, road_type) {
        return -1;
    }
    i64::from((crate::movement::table(s.command.shared).road_type_speed)(
        crate::services::RoadType(road_type as u8),
    ))
}

/// `ScriptRoad::GetBuildCost`.
#[unsafe(no_mangle)]
#[allow(clippy::cast_possible_truncation)]
pub extern "C" fn openttd_rust_script_road_build_cost(
    s: &Services,
    road_type: u32,
    build_type: u32,
) -> i64 {
    use crate::economy_type::{PR_BUILD_DEPOT_ROAD, PR_BUILD_STATION_BUS, PR_BUILD_STATION_TRUCK};
    if !road_type_available(s, road_type) {
        return -1;
    }
    let price = match build_type {
        0 => return crate::road_cost::build_cost(s.command.shared, road_type as u8),
        1 => PR_BUILD_DEPOT_ROAD,
        2 => PR_BUILD_STATION_BUS,
        3 => PR_BUILD_STATION_TRUCK,
        _ => return -1,
    };
    crate::economy_func::get_price(price as u32, 1, 0, 0)
}

/// `GetPresentRoadTypes(tile).Test(road_type)`.
fn has_present(w: &World<'_>, tile: u32, road_type: u8) -> bool {
    crate::road_map::get_present_road_types(w, tile) & (1_u64 << road_type) != 0
}

/// The shared prefix of the current-road-type tile predicates.
#[allow(clippy::cast_possible_truncation)]
fn current_road_tile(s: &Services, tile: u32, read: impl Fn(&World<'_>, u8) -> bool) -> bool {
    let w = world(s);
    if !valid(&w, tile) {
        return false;
    }
    if !road_type_available(s, current_road_type()) {
        return false;
    }
    read(&w, current_road_type() as u8)
}

fn road_depot_tile(s: &Services, tile: u32) -> bool {
    current_road_tile(s, tile, |w, rt| {
        w.kind(TileIndex(tile)) == TileType::MP_ROAD
            && w.road_tile_type(tile).0 == crate::road_map::ROAD_TILE_DEPOT
            && has_present(w, tile, rt)
    })
}

fn road_station_tile(s: &Services, tile: u32) -> bool {
    current_road_tile(s, tile, |w, rt| {
        w.station_road_stop(tile) && has_present(w, tile, rt)
    })
}

fn drive_through_station_tile(s: &Services, tile: u32) -> bool {
    current_road_tile(s, tile, |w, rt| {
        w.drive_through_stop_tile(tile) && has_present(w, tile, rt)
    })
}

fn road_tile(s: &Services, tile: u32) -> bool {
    let w = world(s);
    if !valid(&w, tile) {
        return false;
    }
    (w.kind(TileIndex(tile)) == TileType::MP_ROAD
        && w.road_tile_type(tile).0 != crate::road_map::ROAD_TILE_DEPOT)
        || drive_through_station_tile(s, tile)
}

/// `ScriptRoad::GetCurrentRoadType`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_road_current_type() -> u32 {
    current_road_type()
}

/// `ScriptRoad::SetCurrentRoadType`.
#[unsafe(no_mangle)]
#[allow(clippy::cast_possible_truncation)]
pub extern "C" fn openttd_rust_script_road_set_current_type(s: &Services, road_type: u32) {
    if !road_type_available(s, road_type) {
        return;
    }
    script_host::set_active_road_type(road_type as u8);
}

/// `ScriptRoad::HasRoadType`; `::RoadType(road_type)` truncates to the `uint8_t` enum.
#[allow(clippy::cast_possible_truncation)]
fn has_road_type(s: &Services, tile: u32, road_type: u32) -> bool {
    let w = world(s);
    if !valid(&w, tile) {
        return false;
    }
    if !road_type_available(s, road_type) {
        return false;
    }
    w.asserted(|m| crate::road_map::may_have_road(m, crate::tile_type::TileIndex(tile)))
        && has_present(&w, tile, road_type as u8)
}

/// `GetAnyRoadBits(tile, rtt)` through the shared native road-bits binding.
fn any_road_bits(s: &Services, tile: u32, tram: bool) -> u8 {
    (s.command.shared.road_bits)(tile, u8::from(tram), false)
}

/// `ScriptRoad::AreRoadTilesConnected`.
#[allow(clippy::cast_possible_truncation)]
fn tiles_connected(s: &Services, t1: u32, t2: u32) -> bool {
    let w = world(s);
    if !valid(&w, t1) || !valid(&w, t2) {
        return false;
    }
    if !road_type_available(s, current_road_type()) {
        return false;
    }
    let m = w.dimensions;
    /* Tiles not neighbouring */
    if m.distance_manhattan(TileIndex(t1), TileIndex(t2)) != 1 {
        return false;
    }
    let tram = crate::road::availability::get_road_tram_type(script_host::active_road_type()).0;
    let r1 = any_road_bits(s, t1, tram);
    let r2 = any_road_bits(s, t2, tram);
    #[allow(clippy::comparison_chain)]
    let dir_1: u8 = if m.tile_x(TileIndex(t1)) == m.tile_x(TileIndex(t2)) {
        if m.tile_y(TileIndex(t1)) < m.tile_y(TileIndex(t2)) {
            2
        } else {
            0
        }
    } else if m.tile_x(TileIndex(t1)) < m.tile_x(TileIndex(t2)) {
        1
    } else {
        3
    };
    let dir_2 = 2 ^ dir_1;
    let drd2 = if w.is_normal_road_tile(t2) {
        w.asserted(|m| crate::road_map::get_disallowed_road_directions(m, t2))
    } else {
        DRD_NONE
    };
    let against = if dir_1 > dir_2 {
        DRD_SOUTHBOUND
    } else {
        DRD_NORTHBOUND
    };
    r1 & (1 << dir_1) != 0 && r2 & (1 << dir_2) != 0 && drd2 != DRD_BOTH && drd2 != against
}

/// `RoadPartOrientation`: `NW`, `NE`, `SW`, `SE`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Part {
    Nw,
    Ne,
    Sw,
    Se,
}

/// `IsStraight`.
fn is_straight(start: Part, end: Part) -> bool {
    match start {
        Part::Nw => end == Part::Se,
        Part::Ne => end == Part::Sw,
        Part::Sw => end == Part::Ne,
        Part::Se => end == Part::Nw,
    }
}

/// `CheckAutoExpandedRoadBits`.
fn check_auto_expanded_road_bits(existing: &[Part], start: Part, end: Part) -> bool {
    is_straight(start, end)
        && existing
            .first()
            .is_none_or(|&first| first == start || first == end)
}

/// `LookupWithoutBuildOnSlopes`.
fn lookup_without_build_on_slopes(slope: u8, existing: &[Part], start: Part, end: Part) -> i32 {
    let expanded = |sw: bool| {
        let auto = if existing.is_empty() { 2 } else { 1 };
        if check_auto_expanded_road_bits(existing, start, end) && sw {
            auto
        } else {
            0
        }
    };
    match slope {
        /* Flat slopes can always be build. */
        SLOPE_FLAT => 1,
        /* Only 4 of the slopes can be build upon. */
        SLOPE_NE | SLOPE_SW => expanded(start == Part::Sw || end == Part::Sw),
        SLOPE_SE | SLOPE_NW => expanded(start != Part::Sw && end != Part::Sw),
        /* Any other tile cannot be built on. */
        _ => 0,
    }
}

/// `RotateNeighbour`.
fn rotate_neighbour(neighbour: Part) -> Part {
    match neighbour {
        Part::Nw => Part::Ne,
        Part::Ne => Part::Se,
        Part::Se => Part::Sw,
        Part::Sw => Part::Nw,
    }
}

/// `NeighbourToRoadBits`.
fn neighbour_to_road_bits(neighbour: Part) -> u8 {
    match neighbour {
        Part::Nw => ROAD_NW,
        Part::Ne => ROAD_NE,
        Part::Se => ROAD_SE,
        Part::Sw => ROAD_SW,
    }
}

/// `LookupWithBuildOnSlopes`.
fn lookup_with_build_on_slopes(
    s: &Services,
    mut slope: u8,
    existing: &[Part],
    mut start: Part,
    mut end: Part,
) -> i32 {
    use crate::slope_func::{
        get_highest_slope_corner_raw, is_steep_slope_raw, slope_with_one_corner_raised_raw,
    };
    const BASE_SLOPES: [u8; 15] = [
        SLOPE_FLAT, SLOPE_W, SLOPE_W, SLOPE_SW, SLOPE_W, SLOPE_EW, SLOPE_SW, SLOPE_WSE, SLOPE_W,
        SLOPE_SW, SLOPE_EW, SLOPE_WSE, SLOPE_SW, SLOPE_WSE, SLOPE_WSE,
    ];
    const BASE_ROTATES: [u8; 15] = [0, 0, 1, 0, 2, 0, 1, 0, 3, 3, 2, 3, 2, 2, 1];
    /* Steep slopes behave the same as slopes with one corner raised. */
    if is_steep_slope_raw(slope) {
        let assert = |ok| crate::movement::native_assert(s.command.shared, ok);
        slope = slope_with_one_corner_raised_raw(
            get_highest_slope_corner_raw(slope, || (s.command.shared.not_reached)()),
            assert,
        );
    }
    /* The slope is not steep. Rotate the data to the uniform base slopes. */
    let Some(&base_slope) = BASE_SLOPES.get(usize::from(slope)) else {
        /* This slope is an invalid slope, so ignore it. */
        return -1;
    };
    let base_rotate = BASE_ROTATES[usize::from(slope)];
    match base_slope {
        /* Flat slopes can always be build; SLOPE_EW and SLOPE_WSE always cause
         * foundations, which makes them accessible from all sides. */
        SLOPE_FLAT | SLOPE_EW | SLOPE_WSE => return 1,
        SLOPE_W | SLOPE_SW => {}
        _ => return -1,
    }
    let rotate = |mut part: Part| {
        for _ in 0..base_rotate {
            part = rotate_neighbour(part);
        }
        part
    };
    start = rotate(start);
    end = rotate(end);
    let start_roadbits = neighbour_to_road_bits(start);
    let new_roadbits = start_roadbits | neighbour_to_road_bits(end);
    let existing_roadbits = existing.iter().fold(ROAD_NONE, |bits, &part| {
        bits | neighbour_to_road_bits(rotate(part))
    });
    if base_slope == SLOPE_W {
        return match new_roadbits {
            /* Cannot build anything with a turn from the low side. */
            ROAD_N | ROAD_E | ROAD_S => 0,
            ROAD_X | ROAD_Y => {
                if (existing_roadbits | new_roadbits) != new_roadbits {
                    return 0;
                }
                /* If the start is in the low part, it is automatically
                 * building the second part too. */
                if start_roadbits & ROAD_E != 0 && existing_roadbits & crate::road_type::ROAD_W == 0
                {
                    2
                } else {
                    1
                }
            }
            _ => {
                if (existing_roadbits | new_roadbits) == new_roadbits {
                    return 1;
                }
                i32::from(existing_roadbits & ROAD_E == 0)
            }
        };
    }
    /* A slope similar to a SLOPE_SW. */
    match new_roadbits {
        ROAD_N | ROAD_E => 0,
        ROAD_X => {
            if (existing_roadbits | new_roadbits) != new_roadbits {
                return 0;
            }
            if start_roadbits & ROAD_NE != 0 && existing_roadbits & ROAD_SW == 0 {
                2
            } else {
                1
            }
        }
        _ => i32::from(existing_roadbits & ROAD_NE == 0),
    }
}

/// `ToRoadPartOrientation` against `ScriptMap::GetTileIndex` offsets.
fn to_part(s: &Services, tile: u32) -> Option<Part> {
    let index = |x, y| crate::script_tile::openttd_rust_script_map_tile_index(s, x, y);
    if tile == index(0, -1) {
        return Some(Part::Nw);
    }
    if tile == index(1, 0) {
        return Some(Part::Sw);
    }
    if tile == index(0, 1) {
        return Some(Part::Se);
    }
    if tile == index(-1, 0) {
        return Some(Part::Ne);
    }
    None
}

/// `ScriptRoad::CanBuildConnectedRoadParts`; `(::Slope)` truncates to the `uint8_t` enum.
#[allow(clippy::cast_possible_truncation)]
fn can_build_parts(s: &Services, slope: u32, existing: &[u32], start: u32, end: u32) -> i64 {
    let slope = slope as u8;
    /* The start tile and end tile cannot be the same tile either. */
    if start == end {
        return -1;
    }
    let mut parts = Vec::with_capacity(existing.len());
    for &tile in existing {
        let Some(part) = to_part(s, tile) else {
            return -1;
        };
        parts.push(part);
    }
    let (Some(start), Some(end)) = (to_part(s, start), to_part(s, end)) else {
        return -1;
    };
    i64::from(if (s.command.shared.build_on_slopes)() {
        lookup_with_build_on_slopes(s, slope, &parts, start, end)
    } else {
        lookup_without_build_on_slopes(slope, &parts, start, end)
    })
}

/// `ScriptRoad::CanBuildConnectedRoadParts` over the converted script array.
/// # Safety
/// `existing` addresses `count` initialized tiles that stay live for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_script_road_can_build_parts(
    s: &Services,
    slope: u32,
    existing: *const crate::tile_type::TileIndex,
    count: usize,
    start: u32,
    end: u32,
) -> i64 {
    let tiles = if count == 0 {
        &[][..]
    } else {
        // SAFETY: caller contract; `TileIndex` is a transparent `u32`.
        unsafe { std::slice::from_raw_parts(existing.cast::<u32>(), count) }
    };
    can_build_parts(s, slope, tiles, start, end)
}

/// `ScriptRoad::CanBuildConnectedRoadPartsHere`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_road_can_build_parts_here(
    s: &Services,
    tile: u32,
    start: u32,
    end: u32,
) -> i64 {
    let w = world(s);
    if !valid(&w, tile) || !valid(&w, start) || !valid(&w, end) {
        return -1;
    }
    let m = w.dimensions;
    if m.distance_manhattan(TileIndex(tile), TileIndex(start)) != 1
        || m.distance_manhattan(TileIndex(tile), TileIndex(end)) != 1
    {
        return -1;
    }
    let index = |x, y| crate::script_tile::openttd_rust_script_map_tile_index(s, x, y);
    /* ROAD_NW, ROAD_SW, ROAD_SE, ROAD_NE */
    let neighbours = [index(0, -1), index(1, 0), index(0, 1), index(-1, 0)];
    let rb = if w.is_normal_road_tile(tile) {
        let bits = |tram| {
            w.asserted(|m| {
                crate::road_map::get_road_bits(m, tile, crate::services::RoadTramType(tram))
            })
            .0
        };
        bits(false) | bits(true)
    } else {
        any_road_bits(s, tile, false) | any_road_bits(s, tile, true)
    };
    let existing: Vec<u32> = (0..4)
        .filter(|&i| rb & (1 << i) != 0)
        .map(|i| neighbours[i])
        .collect();
    let slope = crate::script_tile::openttd_rust_script_tile_slope(s, tile);
    can_build_parts(
        s,
        slope,
        &existing,
        start.wrapping_sub(tile),
        end.wrapping_sub(tile),
    )
}

/// `NeighbourHasReachableRoad`.
fn neighbour_has_reachable_road(w: &World<'_>, rt: u8, start_tile: u32, neighbour: u8) -> bool {
    let neighbour_tile = w
        .dimensions
        .tile_add_by_diag_dir(
            TileIndex(start_tile),
            DiagDirection(neighbour),
            crate::map_func::native_check,
        )
        .0;
    if !has_present(w, neighbour_tile, rt) {
        return false;
    }
    match w.kind(TileIndex(neighbour_tile)) {
        TileType::MP_ROAD => w.road_tile_type(neighbour_tile).0 != crate::road_map::ROAD_TILE_DEPOT,
        TileType::MP_STATION => {
            w.drive_through_stop_tile(neighbour_tile)
                && crate::direction_func::diag_dir_to_axis_raw(neighbour, check(w))
                    == w.drive_through_axis(neighbour_tile)
        }
        _ => false,
    }
}

/// `ScriptRoad::GetNeighbourRoadCount`.
#[unsafe(no_mangle)]
#[allow(clippy::cast_possible_truncation)]
pub extern "C" fn openttd_rust_script_road_neighbour_count(s: &Services, tile: u32) -> i64 {
    use crate::direction_type::{DIAGDIR_NE, DIAGDIR_NW, DIAGDIR_SE, DIAGDIR_SW};
    let w = world(s);
    if !valid(&w, tile) {
        return -1;
    }
    if !road_type_available(s, current_road_type()) {
        return -1;
    }
    let rt = current_road_type() as u8;
    let m = w.dimensions;
    let mut neighbour = 0;
    if m.tile_x(TileIndex(tile)) > 0 && neighbour_has_reachable_road(&w, rt, tile, DIAGDIR_NE) {
        neighbour += 1;
    }
    if neighbour_has_reachable_road(&w, rt, tile, DIAGDIR_SE) {
        neighbour += 1;
    }
    if neighbour_has_reachable_road(&w, rt, tile, DIAGDIR_SW) {
        neighbour += 1;
    }
    if m.tile_y(TileIndex(tile)) > 0 && neighbour_has_reachable_road(&w, rt, tile, DIAGDIR_NW) {
        neighbour += 1;
    }
    neighbour
}

/// `ScriptRoad::GetRoadDepotFrontTile`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_road_depot_front_tile(s: &Services, depot: u32) -> u32 {
    if !road_depot_tile(s, depot) {
        return crate::tile_type::INVALID_TILE;
    }
    let w = world(s);
    (TileIndex(depot)
        + w.dimensions.tile_offs_by_diag_dir(
            DiagDirection(w.road_depot_dir(depot)),
            crate::map_func::native_check,
        ))
    .0
}

/// `ScriptRoad::GetRoadStationFrontTile`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_road_station_front_tile(s: &Services, station: u32) -> u32 {
    if !road_station_tile(s, station) {
        return crate::tile_type::INVALID_TILE;
    }
    let w = world(s);
    if w.bay_stop(station) {
        return (TileIndex(station)
            + w.dimensions.tile_offs_by_diag_dir(
                DiagDirection(w.bay_dir(station)),
                crate::map_func::native_check,
            ))
        .0;
    }
    station.wrapping_sub(
        w.dimensions
            .offs_by_axis(
                crate::direction_type::Axis(w.drive_through_axis(station)),
                |ok| crate::services_assert::check(ok),
            )
            .cast_unsigned(),
    )
}

/// `ScriptRoad::GetDriveThroughBackTile`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_road_drive_through_back_tile(
    s: &Services,
    station: u32,
) -> u32 {
    if !drive_through_station_tile(s, station) {
        return crate::tile_type::INVALID_TILE;
    }
    let w = world(s);
    station.wrapping_add(
        w.dimensions
            .offs_by_axis(
                crate::direction_type::Axis(w.drive_through_axis(station)),
                |ok| crate::services_assert::check(ok),
            )
            .cast_unsigned(),
    )
}

/// `ScriptRoad::ConvertRoadType` preconditions after the native company-mode check.
fn convert_road_type(s: &Services, start: u32, end: u32, road_type: u32) -> bool {
    let w = world(s);
    enforce(valid(&w, start), ERR_PRECONDITION_FAILED)
        && enforce(valid(&w, end), ERR_PRECONDITION_FAILED)
        && enforce(road_type_available(s, road_type), ERR_PRECONDITION_FAILED)
}

/// `start != end`, both tiles valid and in one row or column: the leading
/// preconditions of the long-road, depot and stop wrappers.
fn straight_pair(s: &Services, start: u32, end: u32) -> bool {
    let w = world(s);
    let m = w.dimensions;
    enforce(start != end, ERR_PRECONDITION_FAILED)
        && enforce(valid(&w, start), ERR_PRECONDITION_FAILED)
        && enforce(valid(&w, end), ERR_PRECONDITION_FAILED)
        && enforce(
            m.tile_x(TileIndex(start)) == m.tile_x(TileIndex(end))
                || m.tile_y(TileIndex(start)) == m.tile_y(TileIndex(end)),
            ERR_PRECONDITION_FAILED,
        )
}

/// `ScriptRoad::_BuildRoadInternal` preconditions after the native mode check;
/// the road `Axis`, or `REFUSED`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_road_build(
    s: &Services,
    start: u32,
    end: u32,
    one_way: u8,
) -> u8 {
    if !straight_pair(s, start, end)
        || !enforce(
            one_way == 0
                || crate::road::availability::road_type_is_road(script_host::active_road_type()),
            ERR_PRECONDITION_FAILED,
        )
        || !enforce(
            road_type_available(s, current_road_type()),
            ERR_PRECONDITION_FAILED,
        )
    {
        return REFUSED;
    }
    let m = world(s).dimensions;
    if m.tile_y(TileIndex(start)) == m.tile_y(TileIndex(end)) {
        crate::direction_type::AXIS_X
    } else {
        crate::direction_type::AXIS_Y
    }
}

/// `ScriptRoad::RemoveRoad` and `RemoveRoadFull` preconditions after the native
/// company-mode check.
fn remove_road(s: &Services, start: u32, end: u32) -> bool {
    straight_pair(s, start, end)
        && enforce(
            road_type_available(s, current_road_type()),
            ERR_PRECONDITION_FAILED,
        )
}

/// The entrance `DiagDirection` of `ScriptRoad::BuildRoadDepot`.
fn entrance_dir(w: &World<'_>, tile: u32, front: u32) -> u8 {
    use crate::direction_type::{DIAGDIR_NE, DIAGDIR_NW, DIAGDIR_SE, DIAGDIR_SW};
    let m = w.dimensions;
    #[allow(clippy::if_same_then_else, clippy::comparison_chain)]
    if m.tile_x(TileIndex(tile)) == m.tile_x(TileIndex(front)) {
        if m.tile_y(TileIndex(tile)) < m.tile_y(TileIndex(front)) {
            DIAGDIR_SE
        } else {
            DIAGDIR_NW
        }
    } else if m.tile_x(TileIndex(tile)) < m.tile_x(TileIndex(front)) {
        DIAGDIR_SW
    } else {
        DIAGDIR_NE
    }
}

/// `ScriptRoad::BuildRoadDepot` preconditions after the native company-mode check;
/// the entrance `DiagDirection`, or `REFUSED`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_road_depot(s: &Services, tile: u32, front: u32) -> u8 {
    if !straight_pair(s, tile, front)
        || !enforce(
            road_type_available(s, current_road_type()),
            ERR_PRECONDITION_FAILED,
        )
    {
        return REFUSED;
    }
    entrance_dir(&world(s), tile, front)
}

/// `ScriptRoad::_BuildRoadStationInternal` checks after the native station check;
/// `DiagdirBetweenTiles(tile, front)`, or `REFUSED`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_road_station(
    s: &Services,
    tile: u32,
    front: u32,
    road_veh_type: u32,
) -> u8 {
    if !enforce(
        road_veh_type == ROADVEHTYPE_BUS || road_veh_type == ROADVEHTYPE_TRUCK,
        ERR_PRECONDITION_FAILED,
    ) || !enforce(
        road_type_available(s, current_road_type()),
        ERR_PRECONDITION_FAILED,
    ) {
        return REFUSED;
    }
    let w = world(s);
    w.dimensions
        .diagdir_between_tiles(TileIndex(tile), TileIndex(front))
        .0
}

/// `ScriptRoad::RemoveRoadDepot` preconditions after the native company-mode check.
fn remove_road_depot(s: &Services, tile: u32) -> bool {
    let w = world(s);
    enforce(valid(&w, tile), ERR_PRECONDITION_FAILED)
        && enforce(
            w.kind(TileIndex(tile)) == TileType::MP_ROAD,
            ERR_PRECONDITION_FAILED,
        )
        && enforce(
            w.road_tile_type(tile).0 == crate::road_map::ROAD_TILE_DEPOT,
            ERR_PRECONDITION_FAILED,
        )
}

/// `ScriptRoad::RemoveRoadStation` preconditions after the native company-mode check;
/// `GetRoadStopType(tile)`, or `REFUSED`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_road_remove_station(s: &Services, tile: u32) -> u8 {
    let w = world(s);
    if !enforce(valid(&w, tile), ERR_PRECONDITION_FAILED)
        || !enforce(
            w.kind(TileIndex(tile)) == TileType::MP_STATION,
            ERR_PRECONDITION_FAILED,
        )
        || !enforce(w.station_road_stop(tile), ERR_PRECONDITION_FAILED)
    {
        return REFUSED;
    }
    w.road_stop_type(tile)
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
    /// `ScriptRoad::RoadVehHasPowerOnRoad` and `RoadVehCanRunOnRoad`.
    openttd_rust_script_road_powered => road_veh_has_power(engine_type: u32, road_type: u32);
    /// `ScriptRoad::IsRoadTile`.
    openttd_rust_script_road_is_road_tile => road_tile(tile: u32);
    /// `ScriptRoad::IsRoadDepotTile`.
    openttd_rust_script_road_is_depot_tile => road_depot_tile(tile: u32);
    /// `ScriptRoad::IsRoadStationTile`.
    openttd_rust_script_road_is_station_tile => road_station_tile(tile: u32);
    /// `ScriptRoad::IsDriveThroughRoadStationTile`.
    openttd_rust_script_road_is_drive_through_tile => drive_through_station_tile(tile: u32);
    /// `ScriptRoad::IsRoadTypeAvailable`.
    openttd_rust_script_road_is_type_available => road_type_available(road_type: u32);
    /// `ScriptRoad::HasRoadType`.
    openttd_rust_script_road_has_type => has_road_type(tile: u32, road_type: u32);
    /// `ScriptRoad::AreRoadTilesConnected`.
    openttd_rust_script_road_tiles_connected => tiles_connected(t1: u32, t2: u32);
    /// `ScriptRoad::ConvertRoadType` preconditions after the native company-mode check.
    openttd_rust_script_road_convert => convert_road_type(start: u32, end: u32, road_type: u32);
    /// `ScriptRoad::RemoveRoad` and `RemoveRoadFull` preconditions.
    openttd_rust_script_road_remove => remove_road(start: u32, end: u32);
    /// `ScriptRoad::_BuildRoadStationInternal` checks before its native station check.
    openttd_rust_script_road_station_tiles => straight_pair(tile: u32, front: u32);
    /// `ScriptRoad::RemoveRoadDepot` preconditions.
    openttd_rust_script_road_remove_depot => remove_road_depot(tile: u32);
}

/// `ScriptRoadTypeList`: the road types of the requested road/tram kinds the
/// company may use, in type order; an invalid company mode records its error.
/// # Safety
/// `owner` is the live list of the constructing object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_script_road_type_list(
    s: &Services,
    owner: *mut crate::script_list::List,
    rtts: u32,
) {
    if !enforce(
        deity_or_company_mode_valid(s),
        ERR_PRECONDITION_INVALID_COMPANY,
    ) {
        return;
    }
    let owner_company = script_host::active_company();
    let shared = s.command.shared;
    for rt in 0..crate::road_type::ROADTYPE_END {
        let kind = crate::road::availability::get_road_tram_type(rt).0;
        if rtts & (1 << u32::from(kind)) == 0 {
            continue;
        }
        let types = crate::type_availability::services(shared);
        if crate::road::availability::has_road_type(shared, types, owner_company, rt) {
            // SAFETY: caller contract.
            unsafe { (*owner).add(i64::from(rt), 0) };
        }
    }
}
