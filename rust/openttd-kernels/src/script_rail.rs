/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `ScriptRail` (`script_rail.cpp`): rail tile queries, rail type availability
//! and the preconditions and arguments of the multi-check command wrappers.
//! The module contract is `script_tile.rs`'s. Command wrappers keep their native
//! company-mode macro first and call these entries for the remaining checks.
#![allow(unsafe_code)] // Exported symbol attributes only.

use crate::map_func::World;
use crate::script_host::{self, ERR_PRECONDITION_INVALID_COMPANY, Services};
use crate::script_tile::{
    ERR_PRECONDITION_FAILED, deity_or_company_mode_valid, enforce, has_transport_type, valid, world,
};
use crate::track_type::{
    INVALID_TRACK, INVALID_TRACKDIR, TRACK_BIT_ALL, TRACK_LEFT, TRACK_LOWER, TRACK_RIGHT,
    TRACK_UPPER, TRACK_X, TRACK_Y, TRACKDIR_LEFT_N, TRACKDIR_LEFT_S, TRACKDIR_LOWER_E,
    TRACKDIR_LOWER_W, TRACKDIR_RIGHT_N, TRACKDIR_RIGHT_S, TRACKDIR_UPPER_E, TRACKDIR_UPPER_W,
    TRACKDIR_X_NE, TRACKDIR_X_SW, TRACKDIR_Y_NW, TRACKDIR_Y_SE,
};

/// `ScriptRail::RAILTRACK_*`, the `TrackBits` of each track.
const RAILTRACK_NE_SW: u32 = 1;
const RAILTRACK_NW_SE: u32 = 2;
const RAILTRACK_NW_NE: u32 = 4;
const RAILTRACK_SW_SE: u32 = 8;
const RAILTRACK_NW_SW: u32 = 16;
const RAILTRACK_NE_SE: u32 = 32;
const RAILTRACK_INVALID: u32 = 0xFF;
/// `ScriptRail::RAILTYPE_INVALID`.
const RAILTYPE_INVALID: u32 = u32::MAX;
/// `ScriptRail::SIGNALTYPE_*`.
const SIGNALTYPE_NORMAL: i32 = 0;
const SIGNALTYPE_PBS: i32 = 4;
const SIGNALTYPE_PBS_ONEWAY: i32 = 5;
const SIGNALTYPE_TWOWAY: i32 = 8;
const SIGNALTYPE_NORMAL_TWOWAY: i32 = 8;
const SIGNALTYPE_COMBO_TWOWAY: i32 = 11;
const SIGNALTYPE_NONE: u32 = 0xFF;
/// `ScriptTile::TRANSPORT_RAIL`.
const TRANSPORT_RAIL: u32 = 0;
/// The refused result of the byte-valued command entries.
const REFUSED: u8 = 0xFF;

/// Native assertion of the shared map view.
fn check<'a>(w: &'a World<'a>) -> impl Fn(bool) + Copy + 'a {
    move |ok| w.check(ok)
}

/// `ScriptRail::IsRailTypeAvailable`; `(::RailType)` truncates to the `uint8_t` enum.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn rail_type_available(s: &Services, rail_type: u32) -> bool {
    if !enforce(
        deity_or_company_mode_valid(s),
        ERR_PRECONDITION_INVALID_COMPANY,
    ) {
        return false;
    }
    let rail_type = rail_type as u8;
    if rail_type >= crate::rail_type::RAILTYPE_END {
        return false;
    }
    let company = script_host::active_company();
    company == crate::company_type::OWNER_DEITY
        || crate::rail::has_rail_type(
            crate::type_availability::services(s.command.shared),
            company,
            rail_type,
        )
}

/// `ScriptRail::GetCurrentRailType`.
pub(crate) fn current_rail_type() -> u32 {
    u32::from(script_host::active_rail_type())
}

fn rail_tile(s: &Services, tile: u32) -> bool {
    let w = world(s);
    if !valid(&w, tile) {
        return false;
    }
    (w.kind(tile) == crate::tile_type::MP_RAILWAY && !w.rail_depot(tile))
        || (w.station_rail_raw(tile) && !w.station_blocked(tile))
        || w.level_crossing_tile(tile)
}

fn level_crossing_tile(s: &Services, tile: u32) -> bool {
    let w = world(s);
    valid(&w, tile) && w.level_crossing_tile(tile)
}

fn rail_depot_tile(s: &Services, tile: u32) -> bool {
    let w = world(s);
    valid(&w, tile) && w.rail_depot(tile)
}

fn rail_station_tile(s: &Services, tile: u32) -> bool {
    let w = world(s);
    valid(&w, tile) && w.rail_station(tile)
}

fn rail_waypoint_tile(s: &Services, tile: u32) -> bool {
    let w = world(s);
    valid(&w, tile) && w.rail_waypoint_tile(tile)
}

/// `ScriptRail::GetRailTracks`.
fn rail_tracks(s: &Services, tile: u32) -> u32 {
    use crate::track_func::{axis_to_track, axis_to_track_bits, track_to_track_bits};
    if !rail_tile(s, tile) {
        return RAILTRACK_INVALID;
    }
    let w = world(s);
    let c = check(&w);
    if rail_station_tile(s, tile) || rail_waypoint_tile(s, tile) {
        let track = axis_to_track(crate::services::Axis(w.station_axis(tile)), c).0;
        return u32::from(track_to_track_bits(track, c));
    }
    if level_crossing_tile(s, tile) {
        let axis = crate::direction_func::other_axis_raw(w.crossing_road_axis(tile), c);
        return u32::from(axis_to_track_bits(axis, c));
    }
    if rail_depot_tile(s, tile) {
        return 0;
    }
    u32::from(w.tracks(tile))
}

/// `ScriptRail::GetRailTracks` as the C `uint`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_rail_tracks(s: &Services, tile: u32) -> u32 {
    rail_tracks(s, tile)
}

/// `ScriptRail::GetCurrentRailType`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_rail_current_type() -> u32 {
    current_rail_type()
}

/// `ScriptRail::SetCurrentRailType`.
#[unsafe(no_mangle)]
#[allow(clippy::cast_possible_truncation)]
pub extern "C" fn openttd_rust_script_rail_set_current_type(s: &Services, rail_type: u32) {
    if !rail_type_available(s, rail_type) {
        return;
    }
    script_host::set_active_rail_type(rail_type as u8);
}

/// `ScriptRail::TrainCanRunOnRail`.
#[allow(clippy::cast_possible_truncation)]
fn train_can_run_on_rail(s: &Services, engine: u32, track: u32) -> bool {
    if !rail_type_available(s, engine) || !rail_type_available(s, track) {
        return false;
    }
    crate::rail::is_compatible_rail(s.command.shared, engine as u8, track as u8)
}

/// `ScriptRail::TrainHasPowerOnRail`, using the integrated rail-command table.
#[unsafe(no_mangle)]
#[allow(clippy::cast_possible_truncation)]
pub extern "C" fn openttd_rust_script_rail_train_has_power(
    s: &Services,
    rail: &crate::rail_cmd::RailCommandServices,
    engine: u32,
    track: u32,
) -> u8 {
    u8::from(
        rail_type_available(s, engine)
            && rail_type_available(s, track)
            && (rail.powered_rail)(engine as u8, track as u8),
    )
}

/// `ScriptRail::GetMaxSpeed`, using the integrated ground-physics table.
#[unsafe(no_mangle)]
#[allow(clippy::cast_possible_truncation)]
pub extern "C" fn openttd_rust_script_rail_max_speed(
    s: &Services,
    physics: &crate::ground_vehicle::Services,
    rail_type: u32,
) -> i64 {
    if !rail_type_available(s, rail_type) {
        return -1;
    }
    i64::from((physics.rail_speed)(crate::services::RailType(
        rail_type as u8,
    )))
}

/// `ScriptRail::GetBuildCost`, retaining availability before build-type validation.
#[unsafe(no_mangle)]
#[allow(clippy::cast_possible_truncation)]
pub extern "C" fn openttd_rust_script_rail_build_cost(
    s: &Services,
    rail: &crate::rail_cmd::RailCommandServices,
    rail_type: u32,
    build_type: u32,
) -> i64 {
    use crate::economy_type::{
        PR_BUILD_DEPOT_TRAIN, PR_BUILD_SIGNALS, PR_BUILD_STATION_RAIL,
        PR_BUILD_STATION_RAIL_LENGTH, PR_BUILD_WAYPOINT_RAIL,
    };
    if !rail_type_available(s, rail_type) {
        return -1;
    }
    let price = |index| crate::economy_func::get_price(index as u32, 1, 0, 0);
    match build_type {
        0 => (rail.rail_build_cost)(rail_type as u8),
        1 => price(PR_BUILD_SIGNALS),
        2 => price(PR_BUILD_DEPOT_TRAIN),
        3 => price(PR_BUILD_STATION_RAIL).wrapping_add(price(PR_BUILD_STATION_RAIL_LENGTH)),
        4 => price(PR_BUILD_WAYPOINT_RAIL),
        _ => -1,
    }
}

/// `ScriptRail::GetRailType`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_rail_type(s: &Services, tile: u32) -> u32 {
    if !has_transport_type(s, tile, TRANSPORT_RAIL) {
        return RAILTYPE_INVALID;
    }
    u32::from(world(s).rail_type(tile))
}

/// `ScriptRail::GetRailDepotFrontTile`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_rail_depot_front_tile(s: &Services, depot: u32) -> u32 {
    if !rail_depot_tile(s, depot) {
        return crate::tile_type::INVALID_TILE;
    }
    let w = world(s);
    depot.wrapping_add(
        w.dimensions
            .tile_offs_by_diag_dir_raw(w.rail_depot_dir(depot)),
    )
}

/// `ScriptRail::GetRailStationDirection`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_rail_station_direction(s: &Services, tile: u32) -> u32 {
    if !rail_station_tile(s, tile) {
        return RAILTRACK_INVALID;
    }
    let w = world(s);
    u32::from(crate::track_func::axis_to_track_bits(
        w.station_axis(tile),
        check(&w),
    ))
}

/// `ScriptMap::DistanceManhattan`, which validates both tiles.
fn script_distance(s: &Services, a: u32, b: u32) -> i64 {
    crate::script_tile::openttd_rust_script_map_distance_manhattan(s, a, b)
}

/// `ScriptRail::AreTilesConnected`; tile differences are unsigned `TileIndex` arithmetic.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn tiles_connected(s: &Services, from: u32, tile: u32, to: u32) -> bool {
    if !rail_tile(s, tile) {
        return false;
    }
    if from == to || script_distance(s, from, tile) != 1 || script_distance(s, tile, to) != 1 {
        return false;
    }
    let (from, to) = if to < from { (to, from) } else { (from, to) };
    let size_x = crate::script_tile::openttd_rust_script_map_size_x(s) as i32 as u32;
    if tile.wrapping_sub(from) == 1 {
        if to.wrapping_sub(tile) == 1 {
            return rail_tracks(s, tile) & RAILTRACK_NE_SW != 0;
        }
        if to.wrapping_sub(tile) == size_x {
            return rail_tracks(s, tile) & RAILTRACK_NE_SE != 0;
        }
    } else if tile.wrapping_sub(from) == size_x {
        if tile.wrapping_sub(to) == 1 {
            return rail_tracks(s, tile) & RAILTRACK_NW_NE != 0;
        }
        if to.wrapping_sub(tile) == 1 {
            return rail_tracks(s, tile) & RAILTRACK_NW_SW != 0;
        }
        if to.wrapping_sub(tile) == size_x {
            return rail_tracks(s, tile) & RAILTRACK_NW_SE != 0;
        }
    } else {
        return rail_tracks(s, tile) & RAILTRACK_SW_SE != 0;
    }
    (s.command.shared.not_reached)();
    false
}

/// `abs(abs(dx) - abs(dy))` of `ScriptRail::BuildRail`, `RemoveRail` and `SimulateDrag`.
#[allow(clippy::cast_possible_wrap)]
fn diag_offset(w: &World<'_>, to: u32, tile: u32) -> i32 {
    let m = w.dimensions;
    let dx = (m.tile_x(to) as i32).wrapping_sub(m.tile_x(tile) as i32);
    let dy = (m.tile_y(to) as i32).wrapping_sub(m.tile_y(tile) as i32);
    dx.wrapping_abs()
        .wrapping_sub(dy.wrapping_abs())
        .wrapping_abs()
}

/// `SimulateDrag`: the track and the adjusted end tile of a dragged build or removal.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
fn simulate_drag(s: &Services, from: u32, tile: u32, mut to: u32) -> (u8, u32) {
    let map = world(s);
    let dims = map.dimensions;
    let size_x = crate::script_tile::openttd_rust_script_map_size_x(s);
    let x = |t: u32| dims.tile_x(t) as i32;
    let y = |t: u32| dims.tile_y(t) as i32;
    let offset = diag_offset(&map, to, tile);
    let step_x = |to: u32| to.wrapping_sub(x(to).wrapping_sub(x(tile)).clamp(-1, 1) as u32);
    let step_y = |to: u32| {
        let step = size_x.wrapping_mul(i64::from(y(to).wrapping_sub(y(tile)).clamp(-1, 1)));
        to.wrapping_sub(step as u32)
    };
    let mut track = TRACK_X;
    if y(from) == y(to) {
        track = TRACK_X;
        to = step_x(to);
    } else if x(from) == x(to) {
        track = TRACK_Y;
        to = step_y(to);
    } else if y(from) < y(tile) {
        track = if x(to) < x(tile) {
            TRACK_UPPER
        } else {
            TRACK_LEFT
        };
        to = if offset != 0 { step_x(to) } else { step_y(to) };
    } else if y(from) > y(tile) {
        track = if x(to) < x(tile) {
            TRACK_RIGHT
        } else {
            TRACK_LOWER
        };
        to = if offset != 0 { step_x(to) } else { step_y(to) };
    } else if x(from) < x(tile) {
        track = if y(to) < y(tile) {
            TRACK_UPPER
        } else {
            TRACK_RIGHT
        };
        to = if offset == 0 { step_x(to) } else { step_y(to) };
    } else if x(from) > x(tile) {
        track = if y(to) < y(tile) {
            TRACK_LEFT
        } else {
            TRACK_LOWER
        };
        to = if offset == 0 { step_x(to) } else { step_y(to) };
    }
    (track, to)
}

/// A dragged rail command: whether it runs, its track and adjusted end tile.
#[repr(C)]
pub struct DragPlan {
    pub run: u8,
    pub track: u8,
    pub to: u32,
}

/// `ScriptRail::BuildRail` (`build`) and `RemoveRail` preconditions after the
/// native company-mode check, then `SimulateDrag`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_rail_drag(
    s: &Services,
    from: u32,
    tile: u32,
    to: u32,
    build: u8,
) -> DragPlan {
    let refused = DragPlan {
        run: 0,
        track: INVALID_TRACK,
        to,
    };
    let w = world(s);
    let m = w.dimensions;
    if !enforce(valid(&w, from), ERR_PRECONDITION_FAILED)
        || !enforce(valid(&w, tile), ERR_PRECONDITION_FAILED)
        || !enforce(valid(&w, to), ERR_PRECONDITION_FAILED)
        || !enforce(
            m.distance_manhattan(from, tile) == 1,
            ERR_PRECONDITION_FAILED,
        )
        || !enforce(m.distance_manhattan(tile, to) >= 1, ERR_PRECONDITION_FAILED)
        || (build != 0
            && !enforce(
                rail_type_available(s, current_rail_type()),
                ERR_PRECONDITION_FAILED,
            ))
    {
        return refused;
    }
    let straight = (m.tile_x(from) == m.tile_x(tile) && m.tile_x(tile) == m.tile_x(to))
        || (m.tile_y(from) == m.tile_y(tile) && m.tile_y(tile) == m.tile_y(to));
    if !enforce(
        diag_offset(&w, to, tile) <= 1 || straight,
        ERR_PRECONDITION_FAILED,
    ) {
        return refused;
    }
    let (track, to) = simulate_drag(s, from, tile, to);
    DragPlan { run: 1, track, to }
}

/// `ScriptRail::ConvertRailType` preconditions after the native company-mode check.
fn convert_rail_type(s: &Services, start: u32, end: u32, convert_to: u32) -> bool {
    let w = world(s);
    enforce(valid(&w, start), ERR_PRECONDITION_FAILED)
        && enforce(valid(&w, end), ERR_PRECONDITION_FAILED)
        && enforce(rail_type_available(s, convert_to), ERR_PRECONDITION_FAILED)
}

/// `ScriptRail::BuildRailDepot` preconditions after the native company-mode check;
/// the entrance `DiagDirection`, or `REFUSED`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_rail_depot(s: &Services, tile: u32, front: u32) -> u8 {
    use crate::direction_type::{DIAGDIR_NE, DIAGDIR_NW, DIAGDIR_SE, DIAGDIR_SW};
    let w = world(s);
    let m = w.dimensions;
    if !enforce(tile != front, ERR_PRECONDITION_FAILED)
        || !enforce(valid(&w, tile), ERR_PRECONDITION_FAILED)
        || !enforce(valid(&w, front), ERR_PRECONDITION_FAILED)
        || !enforce(
            m.tile_x(tile) == m.tile_x(front) || m.tile_y(tile) == m.tile_y(front),
            ERR_PRECONDITION_FAILED,
        )
        || !enforce(
            rail_type_available(s, current_rail_type()),
            ERR_PRECONDITION_FAILED,
        )
    {
        return REFUSED;
    }
    #[allow(clippy::if_same_then_else, clippy::comparison_chain)]
    if m.tile_x(tile) == m.tile_x(front) {
        if m.tile_y(tile) < m.tile_y(front) {
            DIAGDIR_SE
        } else {
            DIAGDIR_NW
        }
    } else if m.tile_x(tile) < m.tile_x(front) {
        DIAGDIR_SW
    } else {
        DIAGDIR_NE
    }
}

/// The checks `ScriptRail::BuildRailStation` and `BuildNewGRFRailStation` share
/// after the native company-mode check and before their station checks.
fn rail_station(s: &Services, tile: u32, direction: u32, platforms: i64, length: i64) -> bool {
    enforce(valid(&world(s), tile), ERR_PRECONDITION_FAILED)
        && enforce(
            direction == RAILTRACK_NW_SE || direction == RAILTRACK_NE_SW,
            ERR_PRECONDITION_FAILED,
        )
        && enforce((1..=0xFF).contains(&platforms), ERR_PRECONDITION_FAILED)
        && enforce((1..=0xFF).contains(&length), ERR_PRECONDITION_FAILED)
        && enforce(
            rail_type_available(s, current_rail_type()),
            ERR_PRECONDITION_FAILED,
        )
}

/// `ScriptRail::BuildRailWaypoint` preconditions after the native company-mode check;
/// the waypoint `Axis`, or `REFUSED`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_rail_waypoint(s: &Services, tile: u32) -> u8 {
    if !enforce(valid(&world(s), tile), ERR_PRECONDITION_FAILED)
        || !enforce(rail_tile(s, tile), ERR_PRECONDITION_FAILED)
        || !enforce(
            rail_tracks(s, tile) == RAILTRACK_NE_SW || rail_tracks(s, tile) == RAILTRACK_NW_SE,
            ERR_PRECONDITION_FAILED,
        )
        || !enforce(
            rail_type_available(s, current_rail_type()),
            ERR_PRECONDITION_FAILED,
        )
    {
        return REFUSED;
    }
    if rail_tracks(s, tile) == RAILTRACK_NE_SW {
        crate::direction_type::AXIS_X
    } else {
        crate::direction_type::AXIS_Y
    }
}

/// `KillFirstBit((uint)rail_track) == 0`.
fn single_bit(rail_track: u32) -> bool {
    crate::bitmath_func::kill_first_bit_u32(rail_track) == 0
}

/// `ScriptRail::BuildRailTrack` preconditions after the native company-mode check;
/// `FindFirstTrack((::TrackBits)rail_track)`, or `REFUSED`.
#[unsafe(no_mangle)]
#[allow(clippy::cast_possible_truncation)]
pub extern "C" fn openttd_rust_script_rail_build_track(
    s: &Services,
    tile: u32,
    rail_track: u32,
) -> u8 {
    if !enforce(valid(&world(s), tile), ERR_PRECONDITION_FAILED)
        || !enforce(rail_track != 0, ERR_PRECONDITION_FAILED)
        || !enforce(
            rail_track & !u32::from(TRACK_BIT_ALL) == 0,
            ERR_PRECONDITION_FAILED,
        )
        || !enforce(single_bit(rail_track), ERR_PRECONDITION_FAILED)
        || !enforce(
            rail_type_available(s, current_rail_type()),
            ERR_PRECONDITION_FAILED,
        )
    {
        return REFUSED;
    }
    crate::track_func::find_first_track(rail_track as u8)
}

/// `ScriptRail::RemoveRailTrack` preconditions after the native company-mode check;
/// `FindFirstTrack((::TrackBits)rail_track)`, or `REFUSED`.
#[unsafe(no_mangle)]
#[allow(clippy::cast_possible_truncation)]
pub extern "C" fn openttd_rust_script_rail_remove_track(
    s: &Services,
    tile: u32,
    rail_track: u32,
) -> u8 {
    let w = world(s);
    if !enforce(valid(&w, tile), ERR_PRECONDITION_FAILED)
        || !enforce(
            w.plain_rail(tile) || w.level_crossing_tile(tile),
            ERR_PRECONDITION_FAILED,
        )
        || !enforce(
            rail_tracks(s, tile) & rail_track != 0,
            ERR_PRECONDITION_FAILED,
        )
        || !enforce(single_bit(rail_track), ERR_PRECONDITION_FAILED)
    {
        return REFUSED;
    }
    crate::track_func::find_first_track(rail_track as u8)
}

/// `ScriptRailSignalData` (`_possible_trackdirs`): track, trackdir and signal cycles
/// by `2 + dx + 2 * dy` of the front tile.
const POSSIBLE_TRACKDIRS: [[(u8, u8, u8); 3]; 5] = [
    [
        (TRACK_UPPER, TRACKDIR_UPPER_E, 0),
        (TRACK_Y, TRACKDIR_Y_SE, 0),
        (TRACK_LEFT, TRACKDIR_LEFT_S, 1),
    ],
    [
        (TRACK_RIGHT, TRACKDIR_RIGHT_S, 1),
        (TRACK_X, TRACKDIR_X_SW, 1),
        (TRACK_UPPER, TRACKDIR_UPPER_W, 1),
    ],
    [
        (INVALID_TRACK, INVALID_TRACKDIR, 0),
        (INVALID_TRACK, INVALID_TRACKDIR, 0),
        (INVALID_TRACK, INVALID_TRACKDIR, 0),
    ],
    [
        (TRACK_LOWER, TRACKDIR_LOWER_E, 0),
        (TRACK_X, TRACKDIR_X_NE, 0),
        (TRACK_LEFT, TRACKDIR_LEFT_N, 0),
    ],
    [
        (TRACK_RIGHT, TRACKDIR_RIGHT_N, 0),
        (TRACK_Y, TRACKDIR_Y_NW, 1),
        (TRACK_LOWER, TRACKDIR_LOWER_W, 1),
    ],
];

/// `2 + (TileX(front) - TileX(tile)) + 2 * (TileY(front) - TileY(tile))` in `uint`, as `int`.
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
fn data_index(w: &World<'_>, tile: u32, front: u32) -> usize {
    let m = w.dimensions;
    let dx = m.tile_x(front).wrapping_sub(m.tile_x(tile));
    let dy = m.tile_y(front).wrapping_sub(m.tile_y(tile));
    2_u32.wrapping_add(dx).wrapping_add(dy.wrapping_mul(2)) as i32 as usize
}

/// The first `_possible_trackdirs` entry whose track the tile has.
fn signal_entry(s: &Services, tile: u32, front: u32) -> Option<(u8, u8, u8)> {
    let w = world(s);
    POSSIBLE_TRACKDIRS[data_index(&w, tile, front)]
        .into_iter()
        .find(|&(track, _, _)| {
            u32::from(crate::track_func::track_to_track_bits(track, check(&w)))
                & rail_tracks(s, tile)
                != 0
        })
}

/// `ScriptRail::GetSignalType`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_rail_signal_type(s: &Services, tile: u32, front: u32) -> u32 {
    if script_distance(s, tile, front) != 1 {
        return SIGNALTYPE_NONE;
    }
    let w = world(s);
    if w.kind(tile) != crate::tile_type::MP_RAILWAY || !w.has_signals(tile) {
        return SIGNALTYPE_NONE;
    }
    for (track, trackdir, _) in POSSIBLE_TRACKDIRS[data_index(&w, tile, front)] {
        if u32::from(crate::track_func::track_to_track_bits(track, check(&w)))
            & rail_tracks(s, tile)
            == 0
        {
            continue;
        }
        if !w.has_signal_on_track(tile, track) || !w.has_signal_on_trackdir(tile, trackdir) {
            continue;
        }
        let mut signal = u32::from(w.signal_type(tile, track));
        let reverse = crate::track_func::reverse_trackdir(trackdir);
        if w.has_signal_on_trackdir(tile, reverse) {
            signal |= SIGNALTYPE_TWOWAY as u32;
        }
        return signal;
    }
    SIGNALTYPE_NONE
}

/// `IsValidSignalType` of `script_rail.cpp`.
fn is_valid_signal_type(signal: i32) -> bool {
    if !(SIGNALTYPE_NORMAL..=SIGNALTYPE_COMBO_TWOWAY).contains(&signal) {
        return false;
    }
    !(signal > SIGNALTYPE_PBS_ONEWAY && signal < SIGNALTYPE_NORMAL_TWOWAY)
}

/// A single-signal command: whether it runs, its track, `SignalType` and cycles.
#[repr(C)]
pub struct SignalPlan {
    pub run: u8,
    pub track: u8,
    pub signal: u8,
    pub cycles: u8,
}

/// `ScriptRail::BuildSignal` preconditions after the native company-mode check, then
/// its track, cycle count and in-game signal type.
#[unsafe(no_mangle)]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub extern "C" fn openttd_rust_script_rail_build_signal(
    s: &Services,
    tile: u32,
    front: u32,
    signal: u32,
) -> SignalPlan {
    let refused = SignalPlan {
        run: 0,
        track: INVALID_TRACK,
        signal: 0,
        cycles: 0,
    };
    let signal = signal.cast_signed();
    if !enforce(
        script_distance(s, tile, front) == 1,
        ERR_PRECONDITION_FAILED,
    ) || !enforce(world(s).plain_rail(tile), ERR_PRECONDITION_FAILED)
        || !enforce(is_valid_signal_type(signal), ERR_PRECONDITION_FAILED)
    {
        return refused;
    }
    let Some((track, _, mut cycles)) = signal_entry(s, tile, front) else {
        enforce(false, ERR_PRECONDITION_FAILED);
        return refused;
    };
    if signal < SIGNALTYPE_TWOWAY {
        if signal != SIGNALTYPE_PBS && signal != SIGNALTYPE_PBS_ONEWAY {
            cycles += 1;
        }
    } else {
        cycles = 0;
    }
    let kind = if signal >= SIGNALTYPE_TWOWAY {
        signal ^ SIGNALTYPE_TWOWAY
    } else {
        signal
    };
    SignalPlan {
        run: 1,
        track,
        signal: kind as u8,
        cycles,
    }
}

/// `ScriptRail::RemoveSignal` preconditions after the native company-mode check;
/// the signal's track, or `REFUSED`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_rail_remove_signal(
    s: &Services,
    tile: u32,
    front: u32,
) -> u8 {
    if !enforce(
        script_distance(s, tile, front) == 1,
        ERR_PRECONDITION_FAILED,
    ) || !enforce(
        openttd_rust_script_rail_signal_type(s, tile, front) != SIGNALTYPE_NONE,
        ERR_PRECONDITION_FAILED,
    ) {
        return REFUSED;
    }
    let Some((track, _, _)) = signal_entry(s, tile, front) else {
        enforce(false, ERR_PRECONDITION_FAILED);
        return REFUSED;
    };
    track
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
    /// `ScriptRail::IsRailTile`.
    openttd_rust_script_rail_is_rail_tile => rail_tile(tile: u32);
    /// `ScriptRail::IsLevelCrossingTile`.
    openttd_rust_script_rail_is_level_crossing_tile => level_crossing_tile(tile: u32);
    /// `ScriptRail::IsRailDepotTile`.
    openttd_rust_script_rail_is_depot_tile => rail_depot_tile(tile: u32);
    /// `ScriptRail::IsRailStationTile`.
    openttd_rust_script_rail_is_station_tile => rail_station_tile(tile: u32);
    /// `ScriptRail::IsRailWaypointTile`.
    openttd_rust_script_rail_is_waypoint_tile => rail_waypoint_tile(tile: u32);
    /// `ScriptRail::IsRailTypeAvailable`.
    openttd_rust_script_rail_is_type_available => rail_type_available(rail_type: u32);
    /// `ScriptRail::TrainCanRunOnRail`.
    openttd_rust_script_rail_train_can_run => train_can_run_on_rail(engine: u32, track: u32);
    /// `ScriptRail::AreTilesConnected`.
    openttd_rust_script_rail_tiles_connected => tiles_connected(from: u32, tile: u32, to: u32);
    /// `ScriptRail::ConvertRailType` preconditions after the native company-mode check.
    openttd_rust_script_rail_convert => convert_rail_type(start: u32, end: u32, convert_to: u32);
    /// Shared `ScriptRail::BuildRailStation` and `BuildNewGRFRailStation` preconditions.
    openttd_rust_script_rail_station => rail_station(tile: u32, direction: u32, platforms: i64, length: i64);
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDScriptRailDrag" ["script"] => DragPlan { run, track, to };
    "OpenTTDScriptRailSignal" ["script"] => SignalPlan { run, track, signal, cycles };
}

/// `ScriptRailTypeList`: the rail types a deity or the company may use, in type
/// order; an invalid company mode records its error and leaves the list empty.
/// # Safety
/// `owner` is the live list of the constructing object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_script_rail_type_list(
    s: &Services,
    owner: *mut crate::script_list::List,
) {
    if !enforce(
        deity_or_company_mode_valid(s),
        ERR_PRECONDITION_INVALID_COMPANY,
    ) {
        return;
    }
    let owner_company = script_host::active_company();
    let is_deity = owner_company == crate::company_type::OWNER_DEITY;
    let types = crate::type_availability::services(s.command.shared);
    for rt in 0..crate::rail_type::RAILTYPE_END {
        if is_deity || crate::rail::has_rail_type(types, owner_company, rt) {
            // SAFETY: caller contract.
            unsafe { (*owner).add(i64::from(rt), 0) };
        }
    }
}
