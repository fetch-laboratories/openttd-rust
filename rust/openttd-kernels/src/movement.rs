/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete tile movement policy. Native map/pools remain canonical; services
//! are immutable and only copied scalars cross the synchronous boundary. No
//! native borrow survives a callback, including `NewGRF` foundation callbacks.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names,
    clippy::if_not_else,
    clippy::items_after_statements,
    clippy::too_many_lines,
    clippy::collapsible_match,
    clippy::verbose_bit_mask,
    clippy::decimal_bitwise_operands,
    clippy::cast_sign_loss
)]
use crate::direction_func::diag_dir_to_axis;
use crate::ground::{RailServices, RoadServices};
use crate::rail_map::{
    RAIL_GROUND_HALF_TILE_WATER, RAIL_TILE_DEPOT, RAIL_TILE_NORMAL, RAIL_TILE_SIGNALS,
};
use crate::road_map::{ROAD_TILE_CROSSING, ROAD_TILE_DEPOT, ROAD_TILE_NORMAL};
use crate::services::{Foundation, FoundationSlope, RoadBits, Services, Slope, TrackBits};
use crate::slope_func::{
    FOUNDATION_INCLINED_X, FOUNDATION_INCLINED_Y, FOUNDATION_INVALID, FOUNDATION_LEVELED,
    FOUNDATION_NONE, FOUNDATION_STEEP_BOTH, FOUNDATION_STEEP_LOWER, SLOPE_E, SLOPE_FLAT, SLOPE_N,
    SLOPE_NE, SLOPE_NW, SLOPE_S, SLOPE_SE, SLOPE_SW, SLOPE_W, flattening_foundation,
    get_highest_slope_corner, halftile_foundation, is_slope_with_one_corner_raised,
    is_slope_with_three_corners_raised, is_steep_slope, opposite_corner,
    slope_with_one_corner_raised, slope_with_three_corners_raised, special_rail_foundation,
};
use crate::tile_cmd::{CANNOT_ENTER, ENTERED_STATION, ENTERED_WORMHOLE};
use crate::tile_type::{
    INVALID_TILE, MP_HOUSE, MP_INDUSTRY, MP_OBJECT, MP_RAILWAY, MP_ROAD, MP_STATION,
    MP_TUNNELBRIDGE, MP_VOID, MP_WATER,
};
use crate::vehicle_type::{VEH_ROAD, VEH_SHIP, VEH_TRAIN};
use crate::witness as w;
use std::cell::Cell;
use std::ffi::c_void;

/// Entry-local lazy reuse of the immutable registry, never map or owner state.
#[derive(Clone)]
pub(crate) struct CachedTable<'a> {
    shared: &'a Services,
    resolved: Cell<Option<&'a MovementServices>>,
}
impl<'a> CachedTable<'a> {
    pub(crate) fn new(shared: &'a Services) -> Self {
        Self {
            shared,
            resolved: Cell::new(None),
        }
    }
    pub(crate) fn get(&self) -> &'a MovementServices {
        // Copy before the getter: no Cell borrow spans a callback/reentry.
        if let Some(table) = self.resolved.get() {
            return table;
        }
        let table = table(self.shared);
        self.resolved.set(Some(table));
        table
    }
}

#[repr(C)]
pub struct NormalRoadStatusTail {
    pub disallowed: u8,
    pub road_works: bool,
}
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
    pub shared: *const Services,
    pub orders: *const crate::orders::Leaves,
    pub rail: *const RailServices,
    pub road: *const RoadServices,
    pub rail_map: *const crate::services::RailMapServices,
    pub train: *const crate::train::Leaves,
    pub vehicle: *const crate::vehicle::VehicleServices,
    pub road_kind: extern "C" fn(VehicleRef) -> crate::services::RoadTramType,
    pub road_vehicle: *const crate::road::Leaves,
    pub resolve: extern "C" fn(crate::vehicle::VehicleRef) -> MovementVehicle,
    pub pose: extern "C" fn(crate::vehicle::VehicleRef) -> MovementPose,
    pub depot_position: extern "C" fn(crate::train::Handle) -> DepotPosition,
    pub incline: extern "C" fn(crate::vehicle::VehicleRef) -> Incline,
    pub first: extern "C" fn(VehicleRef) -> VehicleRef,
    pub first_speed: extern "C" fn(crate::vehicle::VehicleRef) -> FirstSpeed,
    pub write_ground_flags: extern "C" fn(crate::vehicle::VehicleRef, u16),
    pub train_engine_class: extern "C" fn(crate::vehicle::VehicleRef) -> u8,
    pub depot_invalidate: extern "C" fn(crate::vehicle::VehicleRef),
    pub native_assert: extern "C" fn(bool),
    pub normal_road_status_tail: extern "C" fn(u32) -> NormalRoadStatusTail,
    pub pixel_location: extern "C" fn(i32, i32) -> crate::services::PixelLocation,
    pub plain_rail: extern "C" fn(u32) -> bool,
    pub normal_road: extern "C" fn(u32) -> bool,
    pub is_tunnel: extern "C" fn(u32) -> bool,
    pub rail_station: extern "C" fn(u32) -> bool,
    pub road_crossing: extern "C" fn(u32) -> bool,
    pub tile_flat: extern "C" fn(u32) -> bool,
    pub add_by_diag_dir: extern "C" fn(u32, u8) -> u32,
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
    pub owned_land: extern "C" fn(u32) -> bool,
    pub object_foundation: extern "C" fn(u32) -> ObjectFoundation,
    pub house_foundation: extern "C" fn(u32) -> CallbackFoundation,
    pub house_boolean: extern "C" fn(*const c_void, u16) -> bool,
    pub industry_foundation: extern "C" fn(u32) -> CallbackFoundation,
    pub industry_callback: extern "C" fn(u32, u16) -> u16,
    pub industry_boolean: extern "C" fn(*const c_void, u16) -> bool,
}
/// One accessor per nested table pointer, named after its field.
macro_rules! nested_tables {
    ($($vis:vis fn $name:ident -> $ty:ty;)*) => {$(
        $vis fn $name(&self) -> &$ty {
            // SAFETY: Only resolved at branches whose native/fixture table supplies this immutable callee.
            unsafe { &*self.$name }
        }
    )*};
}
impl MovementServices {
    nested_tables! {
        pub(crate) fn shared -> Services;
        fn rail -> RailServices;
        fn road -> RoadServices;
        fn rail_map -> crate::services::RailMapServices;
        fn train -> crate::train::Leaves;
        fn vehicle -> crate::vehicle::VehicleServices;
        fn road_vehicle -> crate::road::Leaves;
    }
    fn foundation(&self, tile: u32, slope: u8) -> u8 {
        w::hit(w::MOVE_FOUNDATION);
        let kind = (self.shared().tile_type)(tile);
        match kind {
            MP_RAILWAY => {
                if (self.plain_rail)(tile) {
                    rail_foundation(slope, (self.rail().tracks)(tile).0, |ok| {
                        (self.native_assert)(ok);
                    })
                } else {
                    flattening_foundation(slope)
                }
            }
            MP_ROAD => {
                if (self.normal_road)(tile) {
                    road_foundation(slope, (self.road().all_bits)(tile).0, |ok| {
                        (self.native_assert)(ok);
                    })
                } else {
                    flattening_foundation(slope)
                }
            }
            MP_STATION => flattening_foundation(slope),
            MP_TUNNELBRIDGE => {
                if (self.is_tunnel)(tile) {
                    FOUNDATION_NONE
                } else {
                    crate::bridge::get_bridge_foundation(
                        slope,
                        diag_dir_to_axis((self.shared().tunnel_bridge_direction)(tile), |ok| {
                            (self.native_assert)(ok);
                        }),
                    )
                }
            }
            MP_HOUSE | MP_INDUSTRY => {
                let house = kind == MP_HOUSE;
                let p = if house {
                    (self.house_foundation)(tile)
                } else {
                    (self.industry_foundation)(tile)
                };
                if p.callback {
                    let result = if house {
                        unsafe { crate::town::resolver::foundation(tile, p.kind) }
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
    #[inline]
    pub(crate) fn pixel_z_at(&self, x: i32, y: i32, ground: bool) -> i32 {
        let p = (self.pixel_location)(x, y);
        // One named body for constant Rust callers and the dynamic native entry.
        // Resolve the native location before selecting the ground specialization.
        if ground {
            self.pixel_z::<true>(p.tile, x as u32, y as u32, p.kind)
        } else {
            self.pixel_z::<false>(p.tile, x as u32, y as u32, p.kind)
        }
    }
    #[inline]
    fn pixel_z<const GROUND_VEHICLE: bool>(&self, tile: u32, x: u32, y: u32, kind: u8) -> i32 {
        w::hit(w::MOVE_PIXEL_Z);
        if matches!(kind, MP_HOUSE | MP_INDUSTRY | MP_STATION)
            || (kind == MP_RAILWAY && !(self.plain_rail)(tile))
            || (kind == MP_ROAD && !(self.normal_road)(tile))
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
                rail_foundation(p.slope.0, (self.rail().tracks)(tile).0, |ok| {
                    (self.native_assert)(ok);
                })
            } else {
                road_foundation(p.slope.0, (self.road().all_bits)(tile).0, |ok| {
                    (self.native_assert)(ok);
                })
            };
            let z = crate::landscape::apply_pixel_foundation_to_slope(
                self.shared(),
                Foundation(f),
                &mut p.slope,
            );
            p.z = p.z.wrapping_add(z.cast_signed());
        } else if kind == MP_TUNNELBRIDGE {
            if (self.is_tunnel)(tile) {
                if GROUND_VEHICLE {
                    return p.z;
                }
            } else {
                let dir = (self.shared().tunnel_bridge_direction)(tile);
                let z = crate::landscape::apply_pixel_foundation_to_slope(
                    self.shared(),
                    Foundation(crate::bridge::get_bridge_foundation(
                        p.slope.0,
                        diag_dir_to_axis(dir, |ok| (self.native_assert)(ok)),
                    )),
                    &mut p.slope,
                );
                p.z = p.z.wrapping_add(z.cast_signed());
                if GROUND_VEHICLE {
                    if p.slope.0 != SLOPE_FLAT {
                        return p.z.wrapping_add(TILE_HEIGHT.cast_signed());
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
    unsafe { &*s }.pixel_z_at(x, y, ground_vehicle)
}
/// # Safety
/// Same as the in-map entry; outside coordinates never access an invalid tile.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_movement_outside_z(
    s: *const MovementServices,
    x: i32,
    y: i32,
) -> i32 {
    w::hit(w::MOVE_OUTSIDE_Z);
    let s = unsafe { &*s };
    if (x as u32) < ((s.shared().map_max_x)() + 1) * TILE_SIZE
        && (y as u32) < ((s.shared().map_max_y)() + 1) * TILE_SIZE
    {
        s.pixel_z_at(x, y, false)
    } else {
        s.pixel_z::<false>(INVALID_TILE, x as u32, y as u32, MP_VOID)
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
    let p = (s.shared().tile_slope_z)(tile);
    let a =
        (s.shared().apply_foundation)(Foundation(s.foundation(tile, p.slope.0)), Slope(p.slope.0));
    FoundationSlope {
        z: p.z.wrapping_add(a.z),
        slope: a.slope,
    }
}
pub(crate) fn rail_foundation(s: u8, b: u8, check: impl Fn(bool) + Copy) -> u8 {
    if b == TRACK_BIT_NONE {
        return FOUNDATION_NONE;
    }
    if is_steep_slope(s) {
        if b == TRACK_BIT_X {
            return FOUNDATION_INCLINED_X;
        }
        if b == TRACK_BIT_Y {
            return FOUNDATION_INCLINED_Y;
        }
        let corner = get_highest_slope_corner(s);
        let higher = crate::track_func::corner_to_track_bits(corner, check);
        if b == higher {
            return halftile_foundation(corner, check);
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
    const WITHOUT: [u8; 15] = [
        TRACK_BIT_ALL,
        TRACK_BIT_RIGHT,
        TRACK_BIT_UPPER,
        TRACK_BIT_X,
        TRACK_BIT_LEFT,
        TRACK_BIT_NONE,
        TRACK_BIT_Y,
        TRACK_BIT_LOWER,
        TRACK_BIT_LOWER,
        TRACK_BIT_Y,
        TRACK_BIT_NONE,
        TRACK_BIT_LEFT,
        TRACK_BIT_X,
        TRACK_BIT_UPPER,
        TRACK_BIT_RIGHT,
    ];
    if !WITHOUT[usize::from(s)] & b == TRACK_BIT_NONE {
        return FOUNDATION_NONE;
    }
    let valid = !crate::rail_cmd::LEVELED_TRACKS[usize::from(s)] & b == 0;
    let leveled = if valid {
        FOUNDATION_LEVELED
    } else {
        FOUNDATION_INVALID
    };
    let corner = match b {
        TRACK_BIT_LEFT => CORNER_W,
        TRACK_BIT_LOWER => CORNER_S,
        TRACK_BIT_RIGHT => CORNER_E,
        TRACK_BIT_UPPER => CORNER_N,
        TRACK_BIT_HORZ => {
            if s == SLOPE_N {
                return halftile_foundation(CORNER_N, check);
            }
            if s == SLOPE_S {
                return halftile_foundation(CORNER_S, check);
            }
            return leveled;
        }
        TRACK_BIT_VERT => {
            if s == SLOPE_W {
                return halftile_foundation(CORNER_W, check);
            }
            if s == SLOPE_E {
                return halftile_foundation(CORNER_E, check);
            }
            return leveled;
        }
        TRACK_BIT_X => {
            return if is_slope_with_one_corner_raised(s) {
                FOUNDATION_INCLINED_X
            } else {
                leveled
            };
        }
        TRACK_BIT_Y => {
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
    if s & slope_with_three_corners_raised(opposite_corner(corner), check)
        == slope_with_one_corner_raised(corner, check)
    {
        return halftile_foundation(corner, check);
    }
    special_rail_foundation(corner, check)
}
pub(crate) fn road_foundation(mut s: u8, b: u8, check: impl Fn(bool) + Copy) -> u8 {
    if s == SLOPE_FLAT || b == 0 {
        return FOUNDATION_NONE;
    }
    if is_steep_slope(s) {
        s = slope_with_one_corner_raised(get_highest_slope_corner(s), check);
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
pub extern "C" fn openttd_rust_rail_foundation(
    s: Slope,
    b: TrackBits,
    check: extern "C" fn(bool),
) -> Foundation {
    Foundation(rail_foundation(s.0, b.0, |ok| check(ok)))
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_foundation(
    s: Slope,
    b: RoadBits,
    check: extern "C" fn(bool),
) -> Foundation {
    Foundation(road_foundation(s.0, b.0, |ok| check(ok)))
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
use crate::direction_type::{
    AXIS_X, DIAGDIR_NE, DIAGDIR_NW, DIAGDIR_SE, DIAGDIR_SW, INVALID_DIAGDIR,
};
use crate::slope_func::{CORNER_E, CORNER_N, CORNER_S, CORNER_W};
use crate::tile_type::{TILE_HEIGHT, TILE_SIZE, TILE_UNIT_MASK};
use crate::track_func::{
    axis_to_track_bits, combine_track_status, diag_dir_to_diag_track_bits,
    diag_dir_to_diag_trackdir, is_reversing_road_trackdir, track_bits_to_trackdir_bits,
};
use crate::track_type::{
    TRACK_BIT_ALL, TRACK_BIT_DEPOT, TRACK_BIT_HORZ, TRACK_BIT_LEFT, TRACK_BIT_LOWER,
    TRACK_BIT_NONE, TRACK_BIT_RIGHT, TRACK_BIT_UPPER, TRACK_BIT_VERT, TRACK_BIT_WORMHOLE,
    TRACK_BIT_X, TRACK_BIT_Y, TRACK_LOWER, TRACK_UPPER, TRACKDIR_BIT_LEFT_N, TRACKDIR_BIT_LEFT_S,
    TRACKDIR_BIT_LOWER_E, TRACKDIR_BIT_LOWER_W, TRACKDIR_BIT_RIGHT_N, TRACKDIR_BIT_RIGHT_S,
    TRACKDIR_BIT_UPPER_E, TRACKDIR_BIT_UPPER_W, TRACKDIR_BIT_X_NE, TRACKDIR_BIT_X_SW,
    TRACKDIR_BIT_Y_NW, TRACKDIR_BIT_Y_SE,
};
use crate::transport_type::{TRANSPORT_RAIL, TRANSPORT_ROAD, TRANSPORT_WATER};
impl MovementServices {
    fn water_edges(&self, tile: u32, mut bits: u8) -> u8 {
        let map = self.shared().map();
        if map.tile_x(tile) == 0 {
            w::hit(w::MOVE_WATER_EDGE);
            bits &= !(TRACK_BIT_X | TRACK_BIT_UPPER | TRACK_BIT_RIGHT);
        }
        if map.tile_y(tile) == 0 {
            w::hit(w::MOVE_WATER_EDGE);
            bits &= !(TRACK_BIT_Y | TRACK_BIT_LEFT | TRACK_BIT_UPPER);
        }
        bits
    }
    // Keep one policy body visible to constant propagation at literal Rust callers.
    // The native facade below still supplies dynamic transport/subtype/side values.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn track_status(&self, tile: u32, mode: u8, sub: u32, side: u8) -> u32 {
        let mut bits = TRACK_BIT_NONE;
        let mut red = 0;
        match (self.shared().tile_type)(tile) {
            MP_RAILWAY => {
                w::hit(w::MOVE_TRACK_RAIL);
                if mode == TRANSPORT_WATER
                    && (self.plain_rail)(tile)
                    && (self.rail().ground)(tile).0 == RAIL_GROUND_HALF_TILE_WATER
                    && is_slope_with_one_corner_raised((self.shared().tile_slope)(tile))
                {
                    w::hit(w::MOVE_HALF_WATER);
                    bits = match (self.rail().tracks)(tile).0 {
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
                match (self.rail_map().rail_tile_type)(tile).0 {
                    RAIL_TILE_NORMAL => bits = (self.rail().tracks)(tile).0,
                    RAIL_TILE_SIGNALS => {
                        bits = (self.rail().tracks)(tile).0;
                        w::hit(w::MOVE_SIGNAL_DIRECTIONS);
                        let a = (self.rail_map().present_signals)(tile) as u8;
                        let mut b = (self.rail_map().signal_states)(tile) & u32::from(a);
                        if !(self.oneway_signal)(tile, TRACK_UPPER)
                            || a & crate::signal_func::signal_on_track(TRACK_UPPER) == 0
                        {
                            b |= u32::from(!a & crate::signal_func::signal_on_track(TRACK_UPPER));
                        }
                        if !(self.oneway_signal)(tile, TRACK_LOWER)
                            || a & crate::signal_func::signal_on_track(TRACK_LOWER) == 0
                        {
                            b |= u32::from(!a & crate::signal_func::signal_on_track(TRACK_LOWER));
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
                    RAIL_TILE_DEPOT => {
                        let dir = (self.rail_map().rail_depot_direction)(tile).0;
                        if side == INVALID_DIAGDIR || side == dir {
                            bits = diag_dir_to_diag_track_bits(dir, |ok| (self.native_assert)(ok));
                        }
                    }
                    _ => unreachable!("native rail tile type"),
                }
            }
            MP_ROAD => {
                w::hit(w::MOVE_TRACK_ROAD);
                match mode {
                    TRANSPORT_RAIL => {
                        if (self.road_crossing)(tile) {
                            bits = (self.rail_map().crossing_rail_bits)(tile).0;
                        }
                    }
                    TRANSPORT_ROAD => {
                        let tram = sub != 0;
                        if !(self.has_tile_road_type)(tile, crate::services::RoadTramType(tram)) {
                            return 0;
                        }
                        match (self.shared().road_tile_kind)(tile).0 {
                            ROAD_TILE_NORMAL => {
                                let road = (self.shared().normal_road_bits)(tile, u8::from(tram)).0;
                                if side != INVALID_DIAGDIR
                                    && crate::road_func::diag_dir_to_road_bits(side, |ok| {
                                        (self.native_assert)(ok);
                                    }) & road
                                        == 0
                                {
                                    return 0;
                                }
                                let (disallowed, road_works) = if tram {
                                    (0, (self.shared().has_road_works)(tile))
                                } else {
                                    let tail = (self.normal_road_status_tail)(tile);
                                    (tail.disallowed, tail.road_works)
                                };
                                let multiplier = [0x101, 0x100, 1, 0][usize::from(disallowed)];
                                const ROAD_TRACKS: [u8; 16] = [
                                    TRACK_BIT_NONE,
                                    TRACK_BIT_NONE,
                                    TRACK_BIT_NONE,
                                    TRACK_BIT_LEFT,
                                    TRACK_BIT_NONE,
                                    TRACK_BIT_Y,
                                    TRACK_BIT_LOWER,
                                    TRACK_BIT_LEFT | TRACK_BIT_LOWER | TRACK_BIT_Y,
                                    TRACK_BIT_NONE,
                                    TRACK_BIT_UPPER,
                                    TRACK_BIT_X,
                                    TRACK_BIT_LEFT | TRACK_BIT_UPPER | TRACK_BIT_X,
                                    TRACK_BIT_RIGHT,
                                    TRACK_BIT_RIGHT | TRACK_BIT_UPPER | TRACK_BIT_Y,
                                    TRACK_BIT_RIGHT | TRACK_BIT_LOWER | TRACK_BIT_X,
                                    TRACK_BIT_ALL,
                                ];
                                let dirs = if road_works {
                                    w::hit(w::MOVE_ROAD_WORKS);
                                    0
                                } else {
                                    u16::from(ROAD_TRACKS[usize::from(road)]) * multiplier
                                };
                                return combine_track_status(dirs, 0);
                            }
                            ROAD_TILE_CROSSING => {
                                let axis = (self.rail_map().crossing_road_axis)(tile).0;
                                if side != INVALID_DIAGDIR
                                    && axis != diag_dir_to_axis(side, |ok| (self.native_assert)(ok))
                                {
                                    return 0;
                                }
                                bits = axis_to_track_bits(axis, |ok| (self.native_assert)(ok));
                                if (self.crossing_barred)(tile) {
                                    w::hit(w::MOVE_CROSSING_BARRED);
                                    red = track_bits_to_trackdir_bits(bits);
                                    // Already-owned train query executes directly in Rust.
                                    if unsafe {
                                        crate::train::openttd_rust_train_train_on_tile(
                                            tile,
                                            self.train(),
                                            self.shared(),
                                        )
                                    } == 0
                                    {
                                        for (dir, mask) in [
                                            (
                                                axis_to_diag_dir(axis, |ok| {
                                                    (self.native_assert)(ok);
                                                }),
                                                TRACKDIR_BIT_X_SW | TRACKDIR_BIT_Y_SE,
                                            ),
                                            (
                                                reverse_diag_dir(
                                                    axis_to_diag_dir(axis, |ok| {
                                                        (self.native_assert)(ok);
                                                    }),
                                                    |ok| (self.native_assert)(ok),
                                                ),
                                                TRACKDIR_BIT_X_NE | TRACKDIR_BIT_Y_NW,
                                            ),
                                        ] {
                                            let t = (self.add_by_diag_dir)(tile, dir);
                                            if (self.shared().is_level_crossing)(t)
                                                && (self.crossing_barred)(t)
                                            {
                                                w::hit(w::MOVE_CROSSING_ESCAPE);
                                                red &= mask;
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {
                                let dir = (self.shared().road_depot_direction)(tile);
                                if side == INVALID_DIAGDIR || side == dir {
                                    bits = diag_dir_to_diag_track_bits(dir, |ok| {
                                        (self.native_assert)(ok);
                                    });
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            MP_STATION => {
                w::hit(w::MOVE_TRACK_STATION);
                match mode {
                    TRANSPORT_RAIL => {
                        if (self.rail_map().has_station_rail)(tile)
                            && !(self.rail_map().station_blocked)(tile)
                        {
                            bits = crate::track_func::track_to_track_bits(
                                (self.rail_map().station_track)(tile).0,
                                |ok| (self.native_assert)(ok),
                            );
                        }
                    }
                    TRANSPORT_WATER => {
                        if (self.buoy)(tile) {
                            bits = self.water_edges(tile, TRACK_BIT_ALL);
                        }
                    }
                    TRANSPORT_ROAD => {
                        if (self.shared().is_any_road_stop)(tile) {
                            if !(self.has_tile_road_type)(
                                tile,
                                crate::services::RoadTramType(sub != 0),
                            ) {
                                return 0;
                            }
                            if (self.shared().is_bay_stop)(tile) {
                                let dir = (self.shared().bay_stop_direction)(tile);
                                if side == INVALID_DIAGDIR || side == dir {
                                    bits = diag_dir_to_diag_track_bits(dir, |ok| {
                                        (self.native_assert)(ok);
                                    });
                                }
                            } else {
                                let axis = (self.shared().drive_through_axis)(tile);
                                if side == INVALID_DIAGDIR
                                    || axis == diag_dir_to_axis(side, |ok| (self.native_assert)(ok))
                                {
                                    bits = axis_to_track_bits(axis, |ok| (self.native_assert)(ok));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            MP_WATER => {
                w::hit(w::MOVE_TRACK_WATER);
                if mode != TRANSPORT_WATER {
                    return 0;
                }
                bits = match (self.water_type)(tile).0 {
                    0 => {
                        if (self.tile_flat)(tile) {
                            TRACK_BIT_ALL
                        } else {
                            TRACK_BIT_NONE
                        }
                    }
                    1 => {
                        w::hit(w::MOVE_COAST);
                        [
                            TRACK_BIT_NONE,
                            TRACK_BIT_RIGHT,
                            TRACK_BIT_UPPER,
                            TRACK_BIT_NONE,
                            TRACK_BIT_LEFT,
                            TRACK_BIT_NONE,
                            TRACK_BIT_NONE,
                            TRACK_BIT_NONE,
                            TRACK_BIT_LOWER,
                            TRACK_BIT_NONE,
                            TRACK_BIT_NONE,
                            TRACK_BIT_NONE,
                            TRACK_BIT_NONE,
                            TRACK_BIT_NONE,
                            TRACK_BIT_NONE,
                            TRACK_BIT_NONE,
                        ][usize::from((self.shared().tile_slope)(tile) & 15)]
                    }
                    2 => {
                        w::hit(w::MOVE_LOCK);
                        diag_dir_to_diag_track_bits((self.lock_direction)(tile).0, |ok| {
                            (self.native_assert)(ok);
                        })
                    }
                    3 => {
                        w::hit(w::MOVE_SHIP_DEPOT);
                        axis_to_track_bits((self.ship_depot_axis)(tile).0, |ok| {
                            (self.native_assert)(ok);
                        })
                    }
                    _ => return 0,
                };
                bits = self.water_edges(tile, bits);
            }
            MP_TUNNELBRIDGE => {
                w::hit(w::MOVE_TRACK_WORMHOLE);
                let transport = (self.shared().tunnel_transport)(tile);
                if transport != mode
                    || (transport == TRANSPORT_ROAD
                        && !(self.has_tile_road_type)(
                            tile,
                            crate::services::RoadTramType(sub != 0),
                        ))
                {
                    return 0;
                }
                let dir = (self.shared().tunnel_bridge_direction)(tile);
                if side != INVALID_DIAGDIR
                    && side != reverse_diag_dir(dir, |ok| (self.native_assert)(ok))
                {
                    return 0;
                }
                bits = diag_dir_to_diag_track_bits(dir, |ok| (self.native_assert)(ok));
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
const ROAD_ENTER_DEPOT: [u8; 4] = [
    crate::track_type::TRACKDIR_X_SW,
    crate::track_type::TRACKDIR_Y_NW,
    crate::track_type::TRACKDIR_X_NE,
    crate::track_type::TRACKDIR_Y_SE,
];
use crate::tunnelbridge::{TUNNEL_SOUND_FRAME, TUNNEL_VISIBILITY};
impl MovementServices {
    fn hidden(&self, v: VehicleRef, hidden: bool) {
        let status = (self.vehicle().status)(v);
        (self.vehicle().write_status)(
            v,
            if hidden {
                status | crate::vehicle_base::VS_HIDDEN
            } else {
                status & !crate::vehicle_base::VS_HIDDEN
            },
        );
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
        if h.kind.0 != VEH_TRAIN || !(self.rail_depot_tile)(tile) {
            return 0;
        }
        let dir = (self.rail_map().rail_depot_direction)(tile).0;
        let fract = ((x & 15) + ((y & 15) << 4)) as u8;
        if FRACT_BEHIND[usize::from(dir)] == fract {
            return CANNOT_ENTER;
        }
        let direction = (self.vehicle().direction)(v);
        if direction == diag_dir_to_dir(dir, |ok| (self.native_assert)(ok)) {
            let length = crate::train::next_offset(self.train(), self.shared(), h.train);
            let enter = FRACT_ENTER[usize::from(dir)];
            let point = (i32::from(enter & 15) + (length + 1) * LEAVE_OFFSET[usize::from(dir)])
                .wrapping_add(
                    (i32::from(enter >> 4) + (length + 1) * LEAVE_OFFSET[usize::from(dir) + 4])
                        .wrapping_shl(4),
                ) as u8;
            if point == fract {
                w::hit(w::MOVE_TRAIN_DEPOT_LEAVE_0 + usize::from(dir));
                let next = (self.vehicle().next)(v);
                if let Some(next) = next {
                    self.hidden(next, false);
                    let n = (self.resolve)(next);
                    unsafe {
                        crate::train_state::openttd_rust_train_state_set_track(
                            n.train.owner,
                            diag_dir_to_diag_track_bits(dir, |ok| (self.native_assert)(ok)),
                        );
                    }
                }
            }
        } else if FRACT_ENTER[usize::from(dir)] == fract {
            w::hit(w::MOVE_TRAIN_DEPOT_ENTER_0 + usize::from(dir));
            (self.native_assert)(
                diag_dir_to_dir(reverse_diag_dir(dir, |ok| (self.native_assert)(ok)), |ok| {
                    (self.native_assert)(ok);
                }) == direction,
            );
            unsafe {
                crate::train_state::openttd_rust_train_state_set_track(
                    h.train.owner,
                    TRACK_BIT_DEPOT,
                );
            }
            self.hidden(v, true);
            (self.vehicle().write_direction)(
                v,
                reverse_dir(direction, |ok| (self.native_assert)(ok)),
            );
            if (self.vehicle().next)(v).is_none() {
                self.vehicle().enter_depot((self.first)(v));
            }
            (self.vehicle().write_tile)(v, tile);
            (self.depot_invalidate)(v);
            return ENTERED_WORMHOLE;
        }
        0
    }
    unsafe fn enter_road(&self, h: &MovementVehicle, v: VehicleRef, tile: u32) -> u8 {
        if (self.shared().road_tile_kind)(tile).0 != ROAD_TILE_DEPOT || h.kind.0 != VEH_ROAD {
            return 0;
        }
        if unsafe { crate::road::get_frame(h.road) } == RVC_DEPOT_STOP_FRAME {
            let dir = (self.shared().road_depot_direction)(tile);
            if ROAD_ENTER_DEPOT[usize::from(dir)] == unsafe { crate::road::get_state(h.road) } {
                unsafe {
                    w::hit(w::MOVE_ROAD_DEPOT_ENTER_0 + usize::from(dir));
                    crate::road::set_state(h.road, RVSB_IN_DEPOT);
                }
                self.hidden(v, true);
                (self.vehicle().write_direction)(
                    v,
                    reverse_dir((self.vehicle().direction)(v), |ok| (self.native_assert)(ok)),
                );
                if (self.vehicle().next)(v).is_none() {
                    self.vehicle().enter_depot((self.first)(v));
                }
                (self.vehicle().write_tile)(v, tile);
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
        if h.kind.0 == VEH_TRAIN {
            let station = (self.shared().station_index)(tile);
            if !should_stop(self.shared(), v, station) {
                return 0;
            }
            if !(self.rail_station)(tile)
                || !crate::vehicle_base::is_front_engine(
                    crate::vehicle_base::is_ground_vehicle(h.kind.0),
                    (self.vehicle().subtype)(v),
                )
            {
                return 0;
            }
            let (mut stop, ahead, length) =
                crate::train::stop_location(self.train(), self.shared(), h.train, station, tile);
            if stop
                .wrapping_add(ahead)
                .wrapping_sub(TILE_SIZE.cast_signed())
                >= length
            {
                return 0;
            }
            let dir = dir_to_diag_dir((self.vehicle().direction)(v), |ok| (self.native_assert)(ok));
            x &= 15;
            y &= 15;
            if diag_dir_to_axis(dir, |ok| (self.native_assert)(ok)) != AXIS_X {
                std::mem::swap(&mut x, &mut y);
            }
            if y == TILE_SIZE.cast_signed() / 2 {
                if dir != DIAGDIR_SE && dir != DIAGDIR_SW {
                    x = TILE_SIZE.cast_signed() - 1 - x;
                }
                stop &= TILE_SIZE.cast_signed() - 1;
                if x == stop {
                    w::hit(w::MOVE_STATION_STOP);
                    return ENTERED_STATION;
                }
                if x < stop {
                    w::hit(w::MOVE_STATION_SLOW);
                    (self.vehicle().write_status)(
                        v,
                        (self.vehicle().status)(v) | crate::vehicle_base::VS_TRAIN_SLOWING,
                    );
                    let speed = ((stop - x) * 20 - 15).max(0) as u16;
                    if speed < (self.vehicle().speed)(v) {
                        (self.vehicle().write_speed)(v, speed);
                    }
                }
            }
        } else if h.kind.0 == VEH_ROAD {
            let state = unsafe { crate::road::get_state(h.road) };
            if state < RVSB_IN_ROAD_STOP
                && !is_reversing_road_trackdir(state, |ok| (self.native_assert)(ok))
                && unsafe { crate::road::get_frame(h.road) } == 0
                && (self.road_vehicle().is_station_road_stop)(tile)
                && crate::vehicle_base::is_front_engine(
                    crate::vehicle_base::is_ground_vehicle(h.kind.0),
                    (self.vehicle().subtype)(v),
                )
            {
                if unsafe {
                    crate::road_stop::openttd_rust_road_stop_enter(self.shared(), tile, v, h.road)
                } {
                    w::hit(w::MOVE_ROAD_STOP_ADMIT);
                    return 0;
                }
                w::hit(w::MOVE_ROAD_STOP_REFUSE);
                return CANNOT_ENTER;
            }
        }
        0
    }
    fn prepare_bridge(&self, v: VehicleRef) {
        let p = (self.incline)(v);
        if p.flags & (1 << crate::ground_vehicle::GVF_GOINGUP_BIT) != 0 {
            w::hit(w::MOVE_BRIDGE_UP);
            (self.vehicle().write_z)(v, p.z.wrapping_add(1));
            (self.write_ground_flags)(v, p.flags & !(1 << crate::ground_vehicle::GVF_GOINGUP_BIT));
        } else {
            w::hit(w::MOVE_BRIDGE_DOWN);
            (self.write_ground_flags)(
                v,
                p.flags & !(1 << crate::ground_vehicle::GVF_GOINGDOWN_BIT),
            );
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
        let dir = (self.shared().tunnel_bridge_direction)(tile);
        let vdir = dir_to_diag_dir(p.direction, |ok| (self.native_assert)(ok));
        let pos = (if diag_dir_to_axis(vdir, |ok| (self.native_assert)(ok)) == AXIS_X {
            x
        } else {
            y
        }) as u8
            & TILE_UNIT_MASK as u8;
        let frame = if vdir == DIAGDIR_NE || vdir == DIAGDIR_NW {
            TILE_SIZE as u8 - 1 - pos
        } else {
            pos
        };
        if (self.is_tunnel)(tile) {
            if h.kind.0 == VEH_TRAIN {
                let track = unsafe {
                    crate::train_state::openttd_rust_train_state_get_track(h.train.owner)
                };
                if track != TRACK_BIT_WORMHOLE && dir == vdir {
                    if crate::vehicle_base::is_front_engine(
                        crate::vehicle_base::is_ground_vehicle(h.kind.0),
                        (self.vehicle().subtype)(v),
                    ) && frame == TUNNEL_SOUND_FRAME
                    {
                        w::hit(w::MOVE_TRAIN_TUNNEL_SOUND);
                        if !(self.vehicle().play_vehicle_sound)(v, crate::newgrf_sound::VSE_TUNNEL)
                            && (self.train_engine_class)(v) == 0
                        {
                            (self.vehicle().play_sound)(
                                v,
                                crate::sound_type::SND_05_TRAIN_THROUGH_TUNNEL,
                            );
                        }
                        return 0;
                    }
                    if frame == TUNNEL_VISIBILITY[usize::from(dir)] {
                        w::hit(w::MOVE_TRAIN_TUNNEL_ENTER);
                        w::hit(w::MOVE_TRAIN_TUNNEL_ENTER_0 + usize::from(dir));
                        (self.vehicle().write_tile)(v, tile);
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
                if dir == reverse_diag_dir(vdir, |ok| (self.native_assert)(ok))
                    && frame == TILE_SIZE as u8 - TUNNEL_VISIBILITY[usize::from(dir)]
                    && z == 0
                {
                    (self.vehicle().write_tile)(v, tile);
                    let track = diag_dir_to_diag_track_bits(vdir, |ok| (self.native_assert)(ok));
                    unsafe {
                        crate::train_state::openttd_rust_train_state_set_track(
                            h.train.owner,
                            track,
                        );
                    }
                    (self.native_assert)(track != 0);
                    w::hit(w::MOVE_TRAIN_TUNNEL_EXIT);
                    w::hit(w::MOVE_TRAIN_TUNNEL_EXIT_0 + usize::from(dir));
                    self.hidden(v, false);
                    return ENTERED_WORMHOLE;
                }
            } else if h.kind.0 == VEH_ROAD {
                if unsafe { crate::road::get_state(h.road) } != RVSB_WORMHOLE && dir == vdir {
                    if frame == TUNNEL_VISIBILITY[usize::from(dir)] {
                        w::hit(w::MOVE_ROAD_TUNNEL_ENTER);
                        (self.native_assert)(
                            u32::from(frame)
                                == u32::from(unsafe { crate::road::get_frame(h.road) }) + 1,
                        );
                        (self.vehicle().write_tile)(v, tile);
                        unsafe {
                            crate::road::set_state(h.road, RVSB_WORMHOLE);
                        }
                        self.hidden(v, true);
                        return ENTERED_WORMHOLE;
                    }
                    return 0;
                }
                if dir == reverse_diag_dir(vdir, |ok| (self.native_assert)(ok))
                    && frame == TILE_SIZE as u8 - TUNNEL_VISIBILITY[usize::from(dir)]
                    && z == 0
                {
                    (self.vehicle().write_tile)(v, tile);
                    unsafe {
                        crate::road::set_state(
                            h.road,
                            diag_dir_to_diag_trackdir(vdir, |ok| (self.native_assert)(ok)),
                        );
                        crate::road::set_frame(h.road, frame);
                    }
                    w::hit(w::MOVE_ROAD_TUNNEL_EXIT);
                    self.hidden(v, false);
                    return ENTERED_WORMHOLE;
                }
            }
        } else {
            if h.kind.0 != VEH_SHIP {
                let mut speed = (self.shared().bridge_speed)(tile);
                if h.kind.0 == VEH_ROAD {
                    if speed > u16::MAX / 2 {
                        w::hit(w::MOVE_BRIDGE_SPEED_NARROW);
                    }
                    speed = speed.wrapping_mul(2);
                }
                let first = (self.first_speed)(v);
                (self.vehicle().write_speed)(first.first, first.speed.min(speed));
            }
            if vdir == dir {
                if frame != TILE_SIZE as u8 - 1 {
                    return 0;
                }
                w::hit(match h.kind.0 {
                    VEH_TRAIN => w::MOVE_TRAIN_BRIDGE_ENTER,
                    VEH_ROAD => w::MOVE_ROAD_BRIDGE_ENTER,
                    VEH_SHIP => w::MOVE_SHIP_BRIDGE_ENTER,
                    _ => unreachable!("native bridge vehicle"),
                });
                match h.kind.0 {
                    VEH_TRAIN => {
                        unsafe {
                            crate::train_state::openttd_rust_train_state_set_track(
                                h.train.owner,
                                TRACK_BIT_WORMHOLE,
                            );
                        }
                        self.prepare_bridge(v);
                    }
                    VEH_ROAD => {
                        unsafe {
                            crate::road::set_state(h.road, RVSB_WORMHOLE);
                        }
                        self.prepare_bridge(v);
                    }
                    VEH_SHIP => unsafe {
                        crate::ship_control::openttd_rust_ship_set_state(
                            h.ship,
                            TRACK_BIT_WORMHOLE,
                        );
                    },
                    _ => unreachable!("native bridge vehicle type"),
                }
                return ENTERED_WORMHOLE;
            } else if vdir == reverse_diag_dir(dir, |ok| (self.native_assert)(ok)) {
                (self.vehicle().write_tile)(v, tile);
                match h.kind.0 {
                    VEH_TRAIN => {
                        if unsafe {
                            crate::train_state::openttd_rust_train_state_get_track(h.train.owner)
                        } == TRACK_BIT_WORMHOLE
                        {
                            unsafe {
                                crate::train_state::openttd_rust_train_state_set_track(
                                    h.train.owner,
                                    diag_dir_to_diag_track_bits(vdir, |ok| {
                                        (self.native_assert)(ok);
                                    }),
                                );
                            }
                            w::hit(w::MOVE_TRAIN_BRIDGE_EXIT);
                            return ENTERED_WORMHOLE;
                        }
                    }
                    VEH_ROAD => {
                        if unsafe { crate::road::get_state(h.road) } == RVSB_WORMHOLE {
                            unsafe {
                                crate::road::set_state(
                                    h.road,
                                    diag_dir_to_diag_trackdir(vdir, |ok| (self.native_assert)(ok)),
                                );
                                crate::road::set_frame(h.road, 0);
                            }
                            w::hit(w::MOVE_ROAD_BRIDGE_EXIT);
                            return ENTERED_WORMHOLE;
                        }
                    }
                    VEH_SHIP => {
                        if unsafe { crate::ship_control::openttd_rust_ship_get_state(h.ship) }
                            == TRACK_BIT_WORMHOLE
                        {
                            unsafe {
                                crate::ship_control::openttd_rust_ship_set_state(
                                    h.ship,
                                    diag_dir_to_diag_track_bits(vdir, |ok| {
                                        (self.native_assert)(ok);
                                    }),
                                );
                            }
                            w::hit(w::MOVE_SHIP_BRIDGE_EXIT);
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
    let kind = (s.shared().tile_type)(tile);
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
    w::hit(w::MOVE_DEPOT_TICKS);
    let p = (s.depot_position)(v);
    let dir = (s.rail_map().rail_depot_direction)(p.tile).0;
    let length = crate::train::next_offset(s.train(), s.shared(), v);
    let enter = FRACT_ENTER[usize::from(dir)];
    match dir {
        0 => (p.x & 15) - (i32::from(enter & 15) - (length + 1)),
        1 => -((p.y & 15) - (i32::from(enter >> 4) + (length + 1))),
        2 => -((p.x & 15) - (i32::from(enter & 15) + (length + 1))),
        3 => (p.y & 15) - (i32::from(enter >> 4) - (length + 1)),
        _ => unreachable!("native depot direction"),
    }
}

/// Resolve the native immutable movement registry. Production callers hold the
/// game-state mutex; no map or owner borrow is formed by resolving this table.
pub(crate) fn table(shared: &Services) -> &MovementServices {
    // SAFETY: Native registry returns its immutable process-lifetime table.
    unsafe { &*(shared.movement_services)() }
}
// Forward literal transport/side arguments through this registry facade too.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn track_status(shared: &Services, tile: u32, mode: u8, sub: u32, side: u8) -> u32 {
    table(shared).track_status(tile, mode, sub, side)
}
pub(crate) fn pixel_z(shared: &Services, x: i32, y: i32, ground: bool) -> i32 {
    table(shared).pixel_z_at(x, y, ground)
}
pub(crate) fn foundation_slope(shared: &Services, tile: u32) -> FoundationSlope {
    // SAFETY: Callers supply the serialized live native map and registry.
    unsafe { openttd_rust_movement_foundation_slope(table(shared), tile) }
}
/// # Safety
/// The vehicle/map stay live through the synchronous entry and its source callbacks.
pub(crate) unsafe fn enter(shared: &Services, v: VehicleRef, tile: u32, x: i32, y: i32) -> u8 {
    unsafe { openttd_rust_movement_enter(table(shared), v, tile, x, y) }
}

pub(crate) fn water_tracks(shared: &Services, tile: u32, side: u8) -> u32 {
    table(shared).water_tracks(tile, side)
}
impl MovementServices {
    pub(crate) fn water_tracks(&self, tile: u32, side: u8) -> u32 {
        u32::from(crate::track_func::track_status_to_track_bits(
            self.track_status(tile, TRANSPORT_WATER, 0, side),
        ))
    }
    pub(crate) fn road_tracks(&self, v: VehicleRef, tile: u32) -> u32 {
        let kind = (self.road_kind)(v);
        self.track_status(tile, TRANSPORT_ROAD, u32::from(kind.0), INVALID_DIAGDIR)
    }
}

pub(crate) fn depot_ticks(shared: &Services, v: crate::train::Handle) -> i32 {
    // SAFETY: Same serialized train as the original synchronous controller call.
    unsafe { openttd_rust_movement_depot_ticks(table(shared), v) }
}

/// Native afterload uses the same portal table as movement.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_movement_tunnel_visibility(direction: u8) -> u8 {
    TUNNEL_VISIBILITY[usize::from(direction)]
}

/// Call the existing order owner directly; acquire only its live raw alias and
/// the source last-station scalar. No owner reference spans either native leaf.
pub(crate) fn should_stop(shared: &Services, v: VehicleRef, station: u16) -> bool {
    let s = table(shared);
    let last = (s.vehicle().last_station)(v);
    // SAFETY: Both native leaves resolve the same serialized surviving shell.
    // The Order is owned by Rust and its facade aliases this stable allocation.
    unsafe {
        let owner = ((*s.orders).vehicle)(v.0.as_ptr().cast());
        crate::orders::openttd_rust_order_should_stop(
            std::ptr::addr_of_mut!((*owner).current).cast(),
            last,
            station,
        ) != 0
    }
}

/// Lazy handler for header preconditions. Valid inline helpers never resolve the
/// native registry; failed assertions follow the native build's assertion policy.
pub(crate) fn native_assert(shared: &Services, condition: bool) {
    (table(shared).native_assert)(condition);
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDMovementFirstSpeed" ["landscape"] => FirstSpeed {
        first,
        speed,
    };
    "OpenTTDMovementIncline" ["landscape"] => Incline {
        z,
        flags,
    };
    "OpenTTDMovementDepotPosition" ["landscape"] => DepotPosition {
        tile,
        x,
        y,
    };
    "OpenTTDMovementPose" ["landscape"] => MovementPose {
        z,
        direction,
    };
    "OpenTTDMovementVehicle" ["landscape"] => MovementVehicle {
        train,
        road,
        ship,
        kind,
    };
    "OpenTTDMovementObjectFoundation" ["landscape"] => ObjectFoundation {
        enabled,
        no_foundation,
    };
    "OpenTTDMovementFoundation" ["landscape"] => CallbackFoundation {
        spec,
        kind,
        callback,
    };
    "OpenTTDNormalRoadStatusTail" ["landscape"] => NormalRoadStatusTail {
        disallowed,
        road_works,
    };
    "OpenTTDMovementServices" ["landscape"] => MovementServices {
        shared,
        orders,
        rail,
        road,
        rail_map,
        train,
        vehicle,
        road_kind,
        road_vehicle,
        resolve,
        pose,
        depot_position,
        incline,
        first,
        first_speed,
        write_ground_flags,
        train_engine_class,
        depot_invalidate,
        native_assert,
        normal_road_status_tail,
        pixel_location,
        plain_rail,
        normal_road,
        is_tunnel,
        rail_station,
        road_crossing,
        tile_flat,
        add_by_diag_dir,
        rail_depot_tile,
        has_tile_road_type,
        oneway_signal,
        crossing_barred,
        buoy,
        water_type,
        lock_direction,
        ship_depot_axis,
        pixel_slope,
        outside_pixel_slope,
        max_pixel_z,
        owned_land,
        object_foundation,
        house_foundation,
        house_boolean,
        industry_foundation,
        industry_callback,
        industry_boolean,
    };
}
