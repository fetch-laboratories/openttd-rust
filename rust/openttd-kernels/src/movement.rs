/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete tile movement policy. Native map/pools remain canonical; services
//! are immutable and only copied scalars cross the synchronous boundary. No
//! native borrow survives a callback, including NewGRF foundation callbacks.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
use crate::direction_func::diag_dir_to_axis;
use crate::ground::{RailServices, RoadServices};
use crate::services::{Foundation, FoundationSlope, RoadBits, Services, Slope, TrackBits};
use crate::slope_func::*;
use crate::tile_type::*;
use std::ffi::c_void;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CallbackFoundation {
    pub spec: *const c_void,
    pub kind: u16,
    pub callback: bool,
}
#[repr(C)]
pub struct ObjectFoundation {
    pub enabled: bool,
    pub no_foundation: bool,
}
#[repr(C)]
pub struct MovementVehicle {
    pub train: crate::train::Handle,
    pub road: *mut crate::road::State,
    pub ship: *mut crate::ship_control::State,
    pub kind: crate::vehicle::VehicleType,
}
#[repr(C)]
pub struct MovementPose {
    pub z: i32,
    pub direction: u8,
}
#[repr(C)]
pub struct DepotPosition {
    pub tile: u32,
    pub x: i32,
    pub y: i32,
}
#[repr(C)]
pub struct Incline {
    pub z: i32,
    pub flags: u16,
}
#[repr(C)]
pub struct FirstSpeed {
    pub first: crate::vehicle::VehicleRef,
    pub speed: u16,
}
#[repr(C)]
pub struct MovementServices {
    pub shared: &'static Services,
    pub rail: &'static RailServices,
    pub road: &'static RoadServices,
    pub rail_map: &'static crate::services::RailMapServices,
    pub train: &'static crate::train::Leaves,
    pub vehicle: &'static crate::vehicle::VehicleServices,
    pub road_vehicle: &'static crate::road::Leaves,
    pub resolve: extern "C" fn(crate::vehicle::VehicleRef) -> MovementVehicle,
    pub pose: extern "C" fn(crate::vehicle::VehicleRef) -> MovementPose,
    pub depot_position: extern "C" fn(crate::train::Handle) -> DepotPosition,
    pub incline: extern "C" fn(crate::vehicle::VehicleRef) -> Incline,
    pub first: extern "C" fn(VehicleRef) -> VehicleRef,
    pub first_speed: extern "C" fn(crate::vehicle::VehicleRef) -> FirstSpeed,
    pub write_ground_flags: extern "C" fn(crate::vehicle::VehicleRef, u16),
    pub front_engine: extern "C" fn(crate::vehicle::VehicleRef) -> bool,
    pub train_engine_class: extern "C" fn(crate::vehicle::VehicleRef) -> u8,
    pub depot_invalidate: extern "C" fn(crate::vehicle::VehicleRef),
    pub native_assert: extern "C" fn(bool),
    pub rail_depot_tile: extern "C" fn(u32) -> bool,
    pub has_tile_road_type: extern "C" fn(u32, crate::services::RoadTramType) -> bool,
    pub oneway_signal: extern "C" fn(u32, u8) -> bool,
    pub crossing_barred: extern "C" fn(u32) -> bool,
    pub buoy: extern "C" fn(u32) -> bool,
    pub water_type: extern "C" fn(u32) -> crate::water_cmd::WaterTileType,
    pub lock_direction: extern "C" fn(u32) -> crate::services::DiagDirection,
    pub ship_depot_axis: extern "C" fn(u32) -> crate::services::Axis,
    pub pixel_slope: extern "C" fn(u32) -> FoundationSlope,
    pub outside_pixel_slope: extern "C" fn(u32, u32) -> FoundationSlope,
    pub max_pixel_z: extern "C" fn(u32) -> i32,
    pub apply_foundation: extern "C" fn(Foundation, Slope) -> FoundationSlope,
    pub owned_land: extern "C" fn(u32) -> bool,
    pub object_foundation: extern "C" fn(u32) -> ObjectFoundation,
    pub house_foundation: extern "C" fn(u32) -> CallbackFoundation,
    pub house_callback: extern "C" fn(u32, u16) -> u16,
    pub house_boolean: extern "C" fn(*const c_void, u16) -> bool,
    pub industry_foundation: extern "C" fn(u32) -> CallbackFoundation,
    pub industry_callback: extern "C" fn(u32, u16) -> u16,
    pub industry_boolean: extern "C" fn(*const c_void, u16) -> bool,
}
impl MovementServices {
    fn foundation(&self, tile: u32, slope: u8) -> u8 {
        match (self.shared.tile_type)(tile) {
            MP_RAILWAY => {
                if (self.shared.is_plain_rail)(tile) {
                    rail_foundation(slope, (self.rail.tracks)(tile).0)
                } else {
                    flattening_foundation(slope)
                }
            }
            MP_ROAD => {
                if (self.shared.is_normal_road)(tile) {
                    road_foundation(slope, (self.road.all_bits)(tile).0)
                } else {
                    flattening_foundation(slope)
                }
            }
            MP_STATION => flattening_foundation(slope),
            MP_TUNNELBRIDGE => {
                if (self.shared.is_bridge)(tile) {
                    crate::bridge::get_bridge_foundation(
                        slope,
                        diag_dir_to_axis((self.shared.tunnel_bridge_direction)(tile)),
                    )
                } else {
                    FOUNDATION_NONE
                }
            }
            MP_HOUSE | MP_INDUSTRY => {
                let house = (self.shared.tile_type)(tile) == MP_HOUSE;
                let p = if house {
                    (self.house_foundation)(tile)
                } else {
                    (self.industry_foundation)(tile)
                };
                if p.callback {
                    let result = if house {
                        (self.house_callback)(tile, p.kind)
                    } else {
                        (self.industry_callback)(tile, p.kind)
                    };
                    if result != crate::newgrf_callbacks::CALLBACK_FAILED
                        && !if house {
                            (self.house_boolean)(p.spec, result)
                        } else {
                            (self.industry_boolean)(p.spec, result)
                        }
                    {
                        return FOUNDATION_NONE;
                    }
                }
                flattening_foundation(slope)
            }
            MP_OBJECT => {
                let p = (self.object_foundation)(tile);
                if p.enabled && p.no_foundation {
                    FOUNDATION_NONE
                } else {
                    flattening_foundation(slope)
                }
            }
            _ => FOUNDATION_NONE,
        }
    }
    fn pixel_z(&self, tile: u32, x: u32, y: u32, ground_vehicle: bool, kind: u8) -> i32 {
        if matches!(kind, MP_HOUSE | MP_INDUSTRY | MP_STATION)
            || (kind == MP_RAILWAY && !(self.shared.is_plain_rail)(tile))
            || (kind == MP_ROAD && !(self.shared.is_normal_road)(tile))
            || (kind == MP_OBJECT && !(self.owned_land)(tile))
        {
            return (self.max_pixel_z)(tile);
        }
        let mut p = if kind == MP_VOID {
            (self.outside_pixel_slope)(x >> 4, y >> 4)
        } else {
            (self.pixel_slope)(tile)
        };
        if matches!(kind, MP_RAILWAY | MP_ROAD) {
            if p.slope.0 == SLOPE_FLAT {
                return p.z;
            }
            let f = if kind == MP_RAILWAY {
                rail_foundation(p.slope.0, (self.rail.tracks)(tile).0)
            } else {
                road_foundation(p.slope.0, (self.road.all_bits)(tile).0)
            };
            let a = (self.apply_foundation)(Foundation(f), Slope(p.slope.0));
            p.z = p.z.wrapping_add(a.z.wrapping_mul(8));
            p.slope = a.slope;
        } else if kind == MP_TUNNELBRIDGE {
            if !(self.shared.is_bridge)(tile) {
                if ground_vehicle {
                    return p.z;
                }
            } else {
                let dir = (self.shared.tunnel_bridge_direction)(tile);
                let a = (self.apply_foundation)(
                    Foundation(crate::bridge::get_bridge_foundation(
                        p.slope.0,
                        diag_dir_to_axis(dir),
                    )),
                    Slope(p.slope.0),
                );
                p.z = p.z.wrapping_add(a.z.wrapping_mul(8));
                p.slope = a.slope;
                if ground_vehicle {
                    if p.slope.0 != SLOPE_FLAT {
                        return p.z.wrapping_add(8);
                    }
                    p.slope.0 = [SLOPE_NE, SLOPE_SE, SLOPE_SW, SLOPE_NW][usize::from(dir)];
                }
            }
        }
        p.z.wrapping_add(
            crate::landscape::partial_pixel_z((x & 15) as i32, (y & 15) as i32, p.slope.0)
                .expect("native slope")
                .cast_signed(),
        )
    }
}
/// # Safety
/// Immutable native table; map remains live/serialized for this synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_movement_pixel_z(
    s: *const MovementServices,
    x: i32,
    y: i32,
    ground_vehicle: bool,
) -> i32 {
    let s = unsafe { &*s };
    let map = crate::services::Map {
        log_x: (s.shared.map_log_x)() as u8,
    };
    let tile = map.tile_xy((x as u32) >> 4, (y as u32) >> 4);
    s.pixel_z(
        tile,
        x as u32,
        y as u32,
        ground_vehicle,
        (s.shared.tile_type)(tile),
    )
}
/// # Safety
/// Same as the in-map entry; outside coordinates never access an invalid tile.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_movement_outside_z(
    s: *const MovementServices,
    x: i32,
    y: i32,
) -> i32 {
    let s = unsafe { &*s };
    if (x as u32) < ((s.shared.map_max_x)() + 1) * 16
        && (y as u32) < ((s.shared.map_max_y)() + 1) * 16
    {
        unsafe { openttd_rust_movement_pixel_z(s, x, y, false) }
    } else {
        s.pixel_z(INVALID_TILE, x as u32, y as u32, false, MP_VOID)
    }
}
/// # Safety
/// Immutable table, live tile; callbacks may reenter without a world borrow.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_movement_foundation_slope(
    s: *const MovementServices,
    tile: u32,
) -> FoundationSlope {
    let s = unsafe { &*s };
    let p = (s.shared.tile_slope_z)(tile);
    let a = (s.apply_foundation)(Foundation(s.foundation(tile, p.slope.0)), Slope(p.slope.0));
    FoundationSlope {
        z: p.z.wrapping_add(a.z),
        slope: a.slope,
    }
}
pub(crate) fn rail_foundation(s: u8, b: u8) -> u8 {
    if b == 0 {
        return FOUNDATION_NONE;
    }
    if is_steep_slope(s) {
        if b == 1 {
            return FOUNDATION_INCLINED_X;
        }
        if b == 2 {
            return FOUNDATION_INCLINED_Y;
        }
        let corner = get_highest_slope_corner(s);
        let higher = crate::track_func::corner_to_track_bits(corner);
        if b == higher {
            return halftile_foundation(corner);
        }
        if crate::track_func::tracks_overlap(b | higher) {
            return FOUNDATION_INVALID;
        }
        return if b & higher != 0 {
            FOUNDATION_STEEP_BOTH
        } else {
            FOUNDATION_STEEP_LOWER
        };
    }
    const WITHOUT: [u8; 15] = [63, 32, 4, 1, 16, 0, 2, 8, 8, 2, 0, 16, 1, 4, 32];
    const LEVELED: [u8; 15] = [
        0,
        16,
        8,
        2 | 8 | 16,
        32,
        63,
        1 | 8 | 32,
        63,
        4,
        1 | 4 | 16,
        63,
        63,
        2 | 4 | 32,
        63,
        63,
    ];
    if !WITHOUT[usize::from(s)] & b == 0 {
        return FOUNDATION_NONE;
    }
    let valid = !LEVELED[usize::from(s)] & b == 0;
    let leveled = if valid {
        FOUNDATION_LEVELED
    } else {
        FOUNDATION_INVALID
    };
    let corner = match b {
        16 => 0,
        8 => 1,
        32 => 2,
        4 => 3,
        12 => {
            if s == SLOPE_N {
                return halftile_foundation(3);
            }
            if s == SLOPE_S {
                return halftile_foundation(1);
            }
            return leveled;
        }
        48 => {
            if s == SLOPE_W {
                return halftile_foundation(0);
            }
            if s == SLOPE_E {
                return halftile_foundation(2);
            }
            return leveled;
        }
        1 => {
            return if is_slope_with_one_corner_raised(s) {
                FOUNDATION_INCLINED_X
            } else {
                leveled
            };
        }
        2 => {
            return if is_slope_with_one_corner_raised(s) {
                FOUNDATION_INCLINED_Y
            } else {
                leveled
            };
        }
        _ => return leveled,
    };
    if !valid {
        return FOUNDATION_INVALID;
    }
    if is_slope_with_three_corners_raised(s) {
        return FOUNDATION_LEVELED;
    }
    if s & slope_with_three_corners_raised(opposite_corner(corner))
        == slope_with_one_corner_raised(corner)
    {
        return halftile_foundation(corner);
    }
    special_rail_foundation(corner)
}
pub(crate) fn road_foundation(mut s: u8, b: u8) -> u8 {
    if s == SLOPE_FLAT || b == 0 {
        return FOUNDATION_NONE;
    }
    if is_steep_slope(s) {
        s = slope_with_one_corner_raised(get_highest_slope_corner(s));
    }
    if crate::road_cmd::INVALID_SLOPES[0][usize::from(s)] & b == 0 {
        return FOUNDATION_LEVELED;
    }
    if !is_slope_with_one_corner_raised(s)
        && crate::road_cmd::INVALID_SLOPES[1][usize::from(s)] & b == 0
    {
        return FOUNDATION_NONE;
    }
    if b == crate::road_type::ROAD_X {
        FOUNDATION_INCLINED_X
    } else {
        FOUNDATION_INCLINED_Y
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_foundation(s: Slope, b: TrackBits) -> Foundation {
    Foundation(rail_foundation(s.0, b.0))
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_foundation(s: Slope, b: RoadBits) -> Foundation {
    Foundation(road_foundation(s.0, b.0))
}

/// # Safety
/// The table and native tile remain valid for the synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_movement_foundation(
    s: *const MovementServices,
    tile: u32,
    slope: Slope,
) -> Foundation {
    Foundation(unsafe { &*s }.foundation(tile, slope.0))
}

use crate::direction_func::{axis_to_diag_dir, reverse_diag_dir};
use crate::direction_type::INVALID_DIAGDIR;
use crate::track_func::*;
use crate::track_type::*;
use crate::transport_type::*;
impl MovementServices {
    fn water_edges(&self, tile: u32, mut bits: u8) -> u8 {
        let map = crate::services::Map {
            log_x: (self.shared.map_log_x)() as u8,
        };
        if map.tile_x(tile) == 0 {
            bits &= !(TRACK_BIT_X | TRACK_BIT_UPPER | TRACK_BIT_RIGHT);
        }
        if map.tile_y(tile) == 0 {
            bits &= !(TRACK_BIT_Y | TRACK_BIT_LEFT | TRACK_BIT_UPPER);
        }
        bits
    }
    fn track_status(&self, tile: u32, mode: u8, sub: u32, side: u8) -> u32 {
        let mut bits = TRACK_BIT_NONE;
        let mut red = 0;
        let r = self.rail_map;
        match (self.shared.tile_type)(tile) {
            MP_RAILWAY => {
                if mode == TRANSPORT_WATER
                    && (self.shared.is_plain_rail)(tile)
                    && (self.rail.ground)(tile).0 == 13
                    && is_slope_with_one_corner_raised((self.shared.tile_slope)(tile))
                {
                    bits = match (self.rail.tracks)(tile).0 {
                        TRACK_BIT_UPPER => TRACK_BIT_LOWER,
                        TRACK_BIT_LOWER => TRACK_BIT_UPPER,
                        TRACK_BIT_LEFT => TRACK_BIT_RIGHT,
                        TRACK_BIT_RIGHT => TRACK_BIT_LEFT,
                        _ => unreachable!("native half-tile track"),
                    };
                    return combine_track_status(track_bits_to_trackdir_bits(bits), 0);
                }
                if mode != TRANSPORT_RAIL {
                    return 0;
                }
                match (r.rail_tile_type)(tile).0 {
                    0 => bits = (self.rail.tracks)(tile).0,
                    1 => {
                        bits = (self.rail.tracks)(tile).0;
                        let a = (r.present_signals)(tile) as u8;
                        let mut b = (r.signal_states)(tile) & u32::from(a);
                        if !(self.oneway_signal)(tile, TRACK_UPPER) || a & 12 == 0 {
                            b |= u32::from(!a & 12);
                        }
                        if !(self.oneway_signal)(tile, TRACK_LOWER) || a & 3 == 0 {
                            b |= u32::from(!a & 3);
                        }
                        if b & 8 == 0 {
                            red |= TRACKDIR_BIT_LEFT_N
                                | TRACKDIR_BIT_X_NE
                                | TRACKDIR_BIT_Y_SE
                                | TRACKDIR_BIT_UPPER_E;
                        }
                        if b & 4 == 0 {
                            red |= TRACKDIR_BIT_LEFT_S
                                | TRACKDIR_BIT_X_SW
                                | TRACKDIR_BIT_Y_NW
                                | TRACKDIR_BIT_UPPER_W;
                        }
                        if b & 2 == 0 {
                            red |= TRACKDIR_BIT_RIGHT_N | TRACKDIR_BIT_LOWER_E;
                        }
                        if b & 1 == 0 {
                            red |= TRACKDIR_BIT_RIGHT_S | TRACKDIR_BIT_LOWER_W;
                        }
                    }
                    3 => {
                        let dir = (r.rail_depot_direction)(tile).0;
                        if side == INVALID_DIAGDIR || side == dir {
                            bits = diag_dir_to_diag_track_bits(dir);
                        }
                    }
                    _ => unreachable!("native rail tile type"),
                }
            }
            MP_ROAD => match mode {
                TRANSPORT_RAIL => {
                    if (self.shared.is_level_crossing)(tile) {
                        bits = (r.crossing_rail_bits)(tile).0;
                    }
                }
                TRANSPORT_ROAD => {
                    let tram = sub != 0;
                    if !(self.has_tile_road_type)(tile, crate::services::RoadTramType(tram)) {
                        return 0;
                    }
                    match (self.shared.road_tile_kind)(tile).0 {
                        0 => {
                            let road = (self.shared.normal_road_bits)(tile, u8::from(tram)).0;
                            if side != INVALID_DIAGDIR
                                && crate::road_func::diag_dir_to_road_bits(side) & road == 0
                            {
                                return 0;
                            }
                            let multiplier = [0x101, 0x100, 1, 0][if tram {
                                0
                            } else {
                                usize::from((self.shared.disallowed_road_directions)(tile))
                            }];
                            const ROAD_TRACKS: [u8; 16] = [
                                0,
                                0,
                                0,
                                16,
                                0,
                                2,
                                8,
                                16 | 8 | 2,
                                0,
                                4,
                                1,
                                16 | 4 | 1,
                                32,
                                32 | 4 | 2,
                                32 | 8 | 1,
                                63,
                            ];
                            let dirs = if (self.shared.has_road_works)(tile) {
                                0
                            } else {
                                u16::from(ROAD_TRACKS[usize::from(road)]) * multiplier
                            };
                            return combine_track_status(dirs, 0);
                        }
                        1 => {
                            let axis = (r.crossing_road_axis)(tile).0;
                            if side != INVALID_DIAGDIR && axis != diag_dir_to_axis(side) {
                                return 0;
                            }
                            bits = axis_to_track_bits(axis);
                            if (self.crossing_barred)(tile) {
                                red = track_bits_to_trackdir_bits(bits);
                                // Already-owned train query executes directly in Rust.
                                if unsafe {
                                    crate::train::openttd_rust_train_train_on_tile(
                                        tile,
                                        self.train,
                                        self.shared,
                                    )
                                } == 0
                                {
                                    let map = crate::services::Map {
                                        log_x: (self.shared.map_log_x)() as u8,
                                    };
                                    for (dir, mask) in [
                                        (
                                            axis_to_diag_dir(axis),
                                            TRACKDIR_BIT_X_SW | TRACKDIR_BIT_Y_SE,
                                        ),
                                        (
                                            reverse_diag_dir(axis_to_diag_dir(axis)),
                                            TRACKDIR_BIT_X_NE | TRACKDIR_BIT_Y_NW,
                                        ),
                                    ] {
                                        let t = tile.wrapping_add(map.offs_by_diag_dir(dir));
                                        if (self.shared.is_level_crossing)(t)
                                            && (self.crossing_barred)(t)
                                        {
                                            red &= mask;
                                        }
                                    }
                                }
                            }
                        }
                        _ => {
                            let dir = (self.shared.road_depot_direction)(tile);
                            if side == INVALID_DIAGDIR || side == dir {
                                bits = diag_dir_to_diag_track_bits(dir);
                            }
                        }
                    }
                }
                _ => {}
            },
            MP_STATION => match mode {
                TRANSPORT_RAIL => {
                    if (r.has_station_rail)(tile) && !(r.station_blocked)(tile) {
                        bits = 1 << (r.station_track)(tile).0;
                    }
                }
                TRANSPORT_WATER => {
                    if (self.buoy)(tile) {
                        bits = self.water_edges(tile, TRACK_BIT_ALL);
                    }
                }
                TRANSPORT_ROAD => {
                    if (self.shared.is_any_road_stop)(tile) {
                        if !(self.has_tile_road_type)(tile, crate::services::RoadTramType(sub != 0))
                        {
                            return 0;
                        }
                        if (self.shared.is_bay_stop)(tile) {
                            let dir = (self.shared.bay_stop_direction)(tile);
                            if side == INVALID_DIAGDIR || side == dir {
                                bits = diag_dir_to_diag_track_bits(dir);
                            }
                        } else {
                            let axis = (self.shared.drive_through_axis)(tile);
                            if side == INVALID_DIAGDIR || axis == diag_dir_to_axis(side) {
                                bits = axis_to_track_bits(axis);
                            }
                        }
                    }
                }
                _ => {}
            },
            MP_WATER => {
                if mode != TRANSPORT_WATER {
                    return 0;
                }
                bits = match (self.water_type)(tile).0 {
                    0 => {
                        if (self.shared.tile_slope)(tile) == 0 {
                            TRACK_BIT_ALL
                        } else {
                            TRACK_BIT_NONE
                        }
                    }
                    1 => [0, 32, 4, 0, 16, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0]
                        [usize::from((self.shared.tile_slope)(tile) & 15)],
                    2 => diag_dir_to_diag_track_bits((self.lock_direction)(tile).0),
                    3 => axis_to_track_bits((self.ship_depot_axis)(tile).0),
                    _ => return 0,
                };
                bits = self.water_edges(tile, bits);
            }
            MP_TUNNELBRIDGE => {
                let transport = (self.shared.tunnel_transport)(tile);
                if transport != mode
                    || (transport == TRANSPORT_ROAD
                        && !(self.has_tile_road_type)(
                            tile,
                            crate::services::RoadTramType(sub != 0),
                        ))
                {
                    return 0;
                }
                let dir = (self.shared.tunnel_bridge_direction)(tile);
                if side != INVALID_DIAGDIR && side != reverse_diag_dir(dir) {
                    return 0;
                }
                bits = diag_dir_to_diag_track_bits(dir);
            }
            _ => {}
        }
        combine_track_status(track_bits_to_trackdir_bits(bits), red)
    }
}
/// # Safety
/// Native tables and map stay live and serialized for the call. No map reference
/// survives the already-owned train traversal, which is invoked directly.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_movement_track_status(
    s: *const MovementServices,
    tile: u32,
    mode: crate::services::TransportType,
    sub: u32,
    side: crate::services::DiagDirection,
) -> u32 {
    unsafe { &*s }.track_status(tile, mode.0, sub, side.0)
}

use crate::direction_func::{diag_dir_to_dir, dir_to_diag_dir, reverse_dir};
use crate::roadveh::{RVC_DEPOT_STOP_FRAME, RVSB_IN_DEPOT, RVSB_IN_ROAD_STOP, RVSB_WORMHOLE};
use crate::vehicle::VehicleRef;
const FRACT_BEHIND: [u8; 4] = [0x8f, 0x08, 0x80, 0xf8];
const FRACT_ENTER: [u8; 4] = [0x8a, 0x48, 0x84, 0xa8];
const LEAVE_OFFSET: [i32; 8] = [-1, 0, 1, 0, 0, 1, 0, -1];
const TUNNEL_VISIBILITY: [u8; 4] = [12, 8, 8, 12];
const ENTERED_STATION: u8 = 1;
const ENTERED_WORMHOLE: u8 = 2;
const CANNOT_ENTER: u8 = 4;
impl MovementServices {
    fn hidden(&self, v: VehicleRef, hidden: bool) {
        let status = (self.vehicle.status)(v);
        (self.vehicle.write_status)(v, if hidden { status | 1 } else { status & !1 });
    }
    // All owner accesses are scalar loads/stores and end before calling native.
    // Depot entry queues autoreplace later in the tick; it preserves this shell
    // and every chain shell used by the original subsequent tile/window stores.
    unsafe fn enter_rail(
        &self,
        h: &MovementVehicle,
        v: VehicleRef,
        tile: u32,
        x: i32,
        y: i32,
    ) -> u8 {
        if h.kind.0 != 0 || !(self.rail_depot_tile)(tile) {
            return 0;
        }
        let dir = (self.rail_map.rail_depot_direction)(tile).0;
        let fract = ((x & 15) + ((y & 15) << 4)) as u8;
        if FRACT_BEHIND[usize::from(dir)] == fract {
            return CANNOT_ENTER;
        }
        let direction = (self.vehicle.direction)(v);
        if direction == diag_dir_to_dir(dir) {
            let length = crate::train::next_offset(self.train, self.shared, h.train);
            let enter = FRACT_ENTER[usize::from(dir)];
            let point = (i32::from(enter & 15) + (length + 1) * LEAVE_OFFSET[usize::from(dir)])
                .wrapping_add(
                    (i32::from(enter >> 4) + (length + 1) * LEAVE_OFFSET[usize::from(dir) + 4])
                        .wrapping_shl(4),
                ) as u8;
            if point == fract {
                let next = (self.vehicle.next)(v);
                if let Some(next) = next {
                    self.hidden(next, false);
                    let n = (self.resolve)(next);
                    unsafe {
                        crate::train_state::openttd_rust_train_state_set_track(
                            n.train.owner,
                            diag_dir_to_diag_track_bits(dir),
                        );
                    }
                }
            }
        } else if FRACT_ENTER[usize::from(dir)] == fract {
            (self.native_assert)(diag_dir_to_dir(reverse_diag_dir(dir)) == direction);
            unsafe {
                crate::train_state::openttd_rust_train_state_set_track(
                    h.train.owner,
                    TRACK_BIT_DEPOT,
                );
            }
            self.hidden(v, true);
            (self.vehicle.write_direction)(v, reverse_dir(direction));
            if (self.vehicle.next)(v).is_none() {
                (self.vehicle.enter_depot)((self.first)(v));
            }
            (self.vehicle.write_tile)(v, tile);
            (self.depot_invalidate)(v);
            return ENTERED_WORMHOLE;
        }
        0
    }
    unsafe fn enter_road(&self, h: &MovementVehicle, v: VehicleRef, tile: u32) -> u8 {
        if (self.shared.road_tile_kind)(tile).0 != 2 || h.kind.0 != 1 {
            return 0;
        }
        if unsafe { crate::road::get_frame(h.road) } == RVC_DEPOT_STOP_FRAME {
            let dir = (self.shared.road_depot_direction)(tile);
            if [8, 9, 0, 1][usize::from(dir)] == unsafe { crate::road::get_state(h.road) } {
                unsafe {
                    crate::road::set_state(h.road, RVSB_IN_DEPOT);
                }
                self.hidden(v, true);
                (self.vehicle.write_direction)(v, reverse_dir((self.vehicle.direction)(v)));
                if (self.vehicle.next)(v).is_none() {
                    (self.vehicle.enter_depot)((self.first)(v));
                }
                (self.vehicle.write_tile)(v, tile);
                (self.depot_invalidate)(v);
                return ENTERED_WORMHOLE;
            }
        }
        0
    }
    unsafe fn enter_station(
        &self,
        h: &MovementVehicle,
        v: VehicleRef,
        tile: u32,
        mut x: i32,
        mut y: i32,
    ) -> u8 {
        if h.kind.0 == 0 {
            let station = (self.shared.station_index)(tile);
            if (self.train.order_stop)(h.train, station) == 0 {
                return 0;
            }
            if !(self.rail_map.is_rail_station)(tile) || !(self.front_engine)(v) {
                return 0;
            }
            let (mut stop, ahead, length) =
                crate::train::stop_location(self.train, self.shared, h.train, station, tile);
            if stop.wrapping_add(ahead).wrapping_sub(16) >= length {
                return 0;
            }
            let dir = dir_to_diag_dir((self.vehicle.direction)(v));
            x &= 15;
            y &= 15;
            if diag_dir_to_axis(dir) != 0 {
                std::mem::swap(&mut x, &mut y);
            }
            if y == 8 {
                if dir != 1 && dir != 2 {
                    x = 15 - x;
                }
                stop &= 15;
                if x == stop {
                    return ENTERED_STATION;
                }
                if x < stop {
                    (self.vehicle.write_status)(v, (self.vehicle.status)(v) | 16);
                    let speed = ((stop - x) * 20 - 15).max(0) as u16;
                    if speed < (self.vehicle.speed)(v) {
                        (self.vehicle.write_speed)(v, speed);
                    }
                }
            }
        } else if h.kind.0 == 1 {
            let state = unsafe { crate::road::get_state(h.road) };
            if state < RVSB_IN_ROAD_STOP
                && !is_reversing_road_trackdir(state)
                && unsafe { crate::road::get_frame(h.road) } == 0
                && (self.road_vehicle.is_station_road_stop)(tile)
                && (self.front_engine)(v)
            {
                if unsafe {
                    crate::road_stop::openttd_rust_road_stop_enter(self.shared, tile, v, h.road)
                } {
                    return 0;
                }
                return CANNOT_ENTER;
            }
        }
        0
    }
    fn prepare_bridge(&self, v: VehicleRef) {
        let p = (self.incline)(v);
        if p.flags & 1 != 0 {
            (self.vehicle.write_z)(v, p.z.wrapping_add(1));
            (self.write_ground_flags)(v, p.flags & !1);
        } else {
            (self.write_ground_flags)(v, p.flags & !2);
        }
    }
    unsafe fn enter_wormhole(
        &self,
        h: &MovementVehicle,
        v: VehicleRef,
        tile: u32,
        x: i32,
        y: i32,
    ) -> u8 {
        let slope = unsafe { openttd_rust_movement_pixel_z(self, x, y, true) };
        let p = (self.pose)(v);
        let z = slope.wrapping_sub(p.z);
        if z.wrapping_abs() > 2 {
            return CANNOT_ENTER;
        }
        let dir = (self.shared.tunnel_bridge_direction)(tile);
        let vdir = dir_to_diag_dir(p.direction);
        let pos = (if diag_dir_to_axis(vdir) == 0 { x } else { y }) as u8 & 15;
        let frame = if vdir == 0 || vdir == 3 {
            15 - pos
        } else {
            pos
        };
        if !(self.shared.is_bridge)(tile) {
            if h.kind.0 == 0 {
                let track = unsafe {
                    crate::train_state::openttd_rust_train_state_get_track(h.train.owner)
                };
                if track != TRACK_BIT_WORMHOLE && dir == vdir {
                    if (self.front_engine)(v) && frame == 1 {
                        if !(self.vehicle.play_vehicle_sound)(
                            v,
                            crate::newgrf_sound::VehicleSoundEvent(2),
                        ) && (self.train_engine_class)(v) == 0
                        {
                            (self.vehicle.play_sound)(v, 3);
                        }
                        return 0;
                    }
                    if frame == TUNNEL_VISIBILITY[usize::from(dir)] {
                        (self.vehicle.write_tile)(v, tile);
                        unsafe {
                            crate::train_state::openttd_rust_train_state_set_track(
                                h.train.owner,
                                TRACK_BIT_WORMHOLE,
                            );
                        }
                        self.hidden(v, true);
                        return ENTERED_WORMHOLE;
                    }
                }
                if dir == reverse_diag_dir(vdir)
                    && frame == 16 - TUNNEL_VISIBILITY[usize::from(dir)]
                    && z == 0
                {
                    (self.vehicle.write_tile)(v, tile);
                    let track = diag_dir_to_diag_track_bits(vdir);
                    unsafe {
                        crate::train_state::openttd_rust_train_state_set_track(
                            h.train.owner,
                            track,
                        );
                    }
                    (self.native_assert)(track != 0);
                    self.hidden(v, false);
                    return ENTERED_WORMHOLE;
                }
            } else if h.kind.0 == 1 {
                if unsafe { crate::road::get_state(h.road) } != RVSB_WORMHOLE && dir == vdir {
                    if frame == TUNNEL_VISIBILITY[usize::from(dir)] {
                        (self.native_assert)(
                            u32::from(frame)
                                == u32::from(unsafe { crate::road::get_frame(h.road) }) + 1,
                        );
                        (self.vehicle.write_tile)(v, tile);
                        unsafe {
                            crate::road::set_state(h.road, RVSB_WORMHOLE);
                        }
                        self.hidden(v, true);
                        return ENTERED_WORMHOLE;
                    }
                    return 0;
                }
                if dir == reverse_diag_dir(vdir)
                    && frame == 16 - TUNNEL_VISIBILITY[usize::from(dir)]
                    && z == 0
                {
                    (self.vehicle.write_tile)(v, tile);
                    unsafe {
                        crate::road::set_state(h.road, diag_dir_to_diag_trackdir(vdir));
                        crate::road::set_frame(h.road, frame);
                    }
                    self.hidden(v, false);
                    return ENTERED_WORMHOLE;
                }
            }
        } else {
            if h.kind.0 != 2 {
                let mut speed = (self.shared.bridge_speed)(tile);
                if h.kind.0 == 1 {
                    speed = speed.wrapping_mul(2);
                }
                let first = (self.first_speed)(v);
                (self.vehicle.write_speed)(first.first, first.speed.min(speed));
            }
            if vdir == dir {
                if frame != 15 {
                    return 0;
                }
                match h.kind.0 {
                    0 => {
                        unsafe {
                            crate::train_state::openttd_rust_train_state_set_track(
                                h.train.owner,
                                TRACK_BIT_WORMHOLE,
                            );
                        }
                        self.prepare_bridge(v);
                    }
                    1 => {
                        unsafe {
                            crate::road::set_state(h.road, RVSB_WORMHOLE);
                        }
                        self.prepare_bridge(v);
                    }
                    2 => unsafe {
                        crate::ship_control::openttd_rust_ship_set_state(
                            h.ship,
                            TRACK_BIT_WORMHOLE,
                        );
                    },
                    _ => unreachable!("native bridge vehicle type"),
                }
                return ENTERED_WORMHOLE;
            } else if vdir == reverse_diag_dir(dir) {
                (self.vehicle.write_tile)(v, tile);
                match h.kind.0 {
                    0 => {
                        if unsafe {
                            crate::train_state::openttd_rust_train_state_get_track(h.train.owner)
                        } == TRACK_BIT_WORMHOLE
                        {
                            unsafe {
                                crate::train_state::openttd_rust_train_state_set_track(
                                    h.train.owner,
                                    diag_dir_to_diag_track_bits(vdir),
                                );
                            }
                            return ENTERED_WORMHOLE;
                        }
                    }
                    1 => {
                        if unsafe { crate::road::get_state(h.road) } == RVSB_WORMHOLE {
                            unsafe {
                                crate::road::set_state(h.road, diag_dir_to_diag_trackdir(vdir));
                                crate::road::set_frame(h.road, 0);
                            }
                            return ENTERED_WORMHOLE;
                        }
                    }
                    2 => {
                        if unsafe { crate::ship_control::openttd_rust_ship_get_state(h.ship) }
                            == TRACK_BIT_WORMHOLE
                        {
                            unsafe {
                                crate::ship_control::openttd_rust_ship_set_state(
                                    h.ship,
                                    diag_dir_to_diag_track_bits(vdir),
                                );
                            }
                            return ENTERED_WORMHOLE;
                        }
                    }
                    _ => unreachable!("native bridge vehicle type"),
                }
            }
        }
        0
    }
}
/// # Safety
/// Tables/vehicle/map stay live and serialized. Selected callbacks preserve the
/// current shell; all scalar owner accesses end before every callback/reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_movement_enter(
    s: *const MovementServices,
    v: VehicleRef,
    tile: u32,
    x: i32,
    y: i32,
) -> u8 {
    let s = unsafe { &*s };
    let kind = (s.shared.tile_type)(tile);
    if !matches!(kind, MP_RAILWAY | MP_ROAD | MP_STATION | MP_TUNNELBRIDGE) {
        return 0;
    }
    let h = (s.resolve)(v);
    unsafe {
        match kind {
            MP_RAILWAY => s.enter_rail(&h, v, tile, x, y),
            MP_ROAD => s.enter_road(&h, v, tile),
            MP_STATION => s.enter_station(&h, v, tile, x, y),
            MP_TUNNELBRIDGE => s.enter_wormhole(&h, v, tile, x, y),
            _ => 0,
        }
    }
}
/// # Safety
/// Live serialized train and immutable native tables throughout this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_movement_depot_ticks(
    s: *const MovementServices,
    v: crate::train::Handle,
) -> i32 {
    let s = unsafe { &*s };
    let p = (s.depot_position)(v);
    let dir = (s.rail_map.rail_depot_direction)(p.tile).0;
    let length = crate::train::next_offset(s.train, s.shared, v);
    let enter = FRACT_ENTER[usize::from(dir)];
    match dir {
        0 => (p.x & 15) - (i32::from(enter & 15) - (length + 1)),
        1 => -((p.y & 15) - (i32::from(enter >> 4) + (length + 1))),
        2 => -((p.x & 15) - (i32::from(enter & 15) + (length + 1))),
        3 => (p.y & 15) - (i32::from(enter >> 4) - (length + 1)),
        _ => unreachable!("native depot direction"),
    }
}
