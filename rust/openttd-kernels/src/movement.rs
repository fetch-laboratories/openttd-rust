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
    spec: *const c_void,
    kind: u16,
    callback: bool,
}
#[repr(C)]
pub struct ObjectFoundation {
    enabled: bool,
    no_foundation: bool,
}
#[repr(C)]
pub struct MovementServices {
    shared: &'static Services,
    rail: &'static RailServices,
    road: &'static RoadServices,
    pixel_slope: extern "C" fn(u32) -> FoundationSlope,
    outside_pixel_slope: extern "C" fn(u32, u32) -> FoundationSlope,
    max_pixel_z: extern "C" fn(u32) -> i32,
    apply_foundation: extern "C" fn(Foundation, Slope) -> FoundationSlope,
    owned_land: extern "C" fn(u32) -> bool,
    object_foundation: extern "C" fn(u32) -> ObjectFoundation,
    house_foundation: extern "C" fn(u32) -> CallbackFoundation,
    house_callback: extern "C" fn(u32, u16) -> u16,
    house_boolean: extern "C" fn(*const c_void, u16) -> bool,
    industry_foundation: extern "C" fn(u32) -> CallbackFoundation,
    industry_callback: extern "C" fn(u32, u16) -> u16,
    industry_boolean: extern "C" fn(*const c_void, u16) -> bool,
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
    if b == 10 {
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
