/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Tile movement policy. Map fields are loaded in place through the canonical
//! descriptor; pool fields remain typed services. No map reference spans a callback.
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
use crate::direction_func::diag_dir_to_axis_raw;
use crate::map_func::World;
use crate::rail_map::{
    RAIL_GROUND_HALF_TILE_WATER, RAIL_TILE_DEPOT, RAIL_TILE_NORMAL, RAIL_TILE_SIGNALS,
};
use crate::road_map::{ROAD_TILE_CROSSING, ROAD_TILE_DEPOT, ROAD_TILE_NORMAL};
use crate::services::{Foundation, FoundationSlope, RoadBits, Services, Slope, TrackBits};
use crate::slope_func::{
    FOUNDATION_INCLINED_X, FOUNDATION_INCLINED_Y, FOUNDATION_INVALID, FOUNDATION_LEVELED,
    FOUNDATION_NONE, FOUNDATION_STEEP_BOTH, FOUNDATION_STEEP_LOWER, SLOPE_E, SLOPE_FLAT, SLOPE_N,
    SLOPE_NE, SLOPE_NW, SLOPE_S, SLOPE_SE, SLOPE_SW, SLOPE_W, flattening_foundation_raw,
    get_highest_slope_corner_raw, halftile_foundation_raw, is_slope_with_one_corner_raised_raw,
    is_slope_with_three_corners_raised_raw, is_steep_slope_raw, opposite_corner_raw,
    slope_with_one_corner_raised_raw, slope_with_three_corners_raised_raw,
    special_rail_foundation_raw,
};
use crate::tile_cmd::{CANNOT_ENTER, ENTERED_STATION, ENTERED_WORMHOLE};
use crate::tile_type::TileIndex;
use crate::tile_type::TileType;
use crate::tile_type::{
    INVALID_TILE, MP_HOUSE, MP_INDUSTRY, MP_OBJECT, MP_RAILWAY, MP_ROAD, MP_STATION,
    MP_TUNNELBRIDGE, MP_VOID,
};
use crate::vehicle_type::{VEH_ROAD, VEH_SHIP, VEH_TRAIN};
use crate::witness as w;
use std::cell::{Cell, OnceCell};
use std::ffi::c_void;

/// Entry-local lazy registry reuse, with map metadata only for audited entries.
/// Native `GetRustMovementServices` is static const and never reset, even on load.
/// Fixtures retain their tables until entry return; reentry cannot replace either table.
pub(crate) struct CachedTable<'a> {
    shared: &'a Services,
    resolved: Cell<Option<&'a MovementServices>>,
    audited: bool,
    world: OnceCell<World<'a>>,
}
impl Clone for CachedTable<'_> {
    fn clone(&self) -> Self {
        Self {
            shared: self.shared,
            resolved: self.resolved.clone(),
            audited: self.audited,
            world: OnceCell::new(),
        }
    }
}
impl<'a> CachedTable<'a> {
    pub(crate) fn new(shared: &'a Services) -> Self {
        Self {
            shared,
            resolved: Cell::new(None),
            audited: false,
            world: OnceCell::new(),
        }
    }
    /// # Safety
    /// Road entries exclude map reset/load and mutex yield through all
    /// synchronous orders, GRF/sound, movement, crash and viewport callbacks.
    pub(crate) unsafe fn audited(shared: &'a Services) -> Self {
        Self {
            audited: true,
            ..Self::new(shared)
        }
    }
    #[inline]
    fn observed(&self) -> Context<'_> {
        let s = self.get();
        // The audited constructor bounds the descriptor to this entry.
        let world = self.world.get_or_init(|| World::new(s.shared()));
        Context { s, world }
    }
    pub(crate) fn road_tracks(&self, v: VehicleRef, tile: u32) -> u32 {
        if self.audited {
            let s = self.observed();
            let kind = (s.road_kind)(v);
            s.track_status(tile, TRANSPORT_ROAD, u32::from(kind.0), INVALID_DIAGDIR)
        } else {
            let s = self.get();
            let kind = (s.road_kind)(v);
            s.track_status(tile, TRANSPORT_ROAD, u32::from(kind.0), INVALID_DIAGDIR)
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

/// Keeps only allocation metadata; each original observation loads fresh scalars.
struct Context<'a> {
    s: &'a MovementServices,
    world: &'a World<'a>,
}
impl std::ops::Deref for Context<'_> {
    type Target = MovementServices;
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.s
    }
}
impl Context<'_> {
    #[inline]
    fn has_tile_road_type(&self, tile: u32, kind: crate::services::RoadTramType) -> bool {
        self.world
            .asserted(|m| crate::road_map::has_tile_road_type(m, tile, kind))
    }
}

#[cfg(feature = "native-test-support")]
#[path = "native_test_support/movement.rs"]
pub(crate) mod native_test_support;

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
pub struct MovementServices {
    pub shared: *const Services,
    pub orders: *const crate::orders::Leaves,
    pub train: *const crate::train::Leaves,
    pub vehicle: *const crate::vehicle::VehicleServices,
    pub tunnel_bridge: *const crate::tunnelbridge_cmd::TunnelBridgeServices,
    pub road_compatible: extern "C" fn(VehicleRef) -> u64,
    pub road_type_speed: extern "C" fn(crate::services::RoadType) -> u16,
    pub road_kind: extern "C" fn(VehicleRef) -> crate::services::RoadTramType,
    pub road_vehicle: *const crate::road::Leaves,
    pub resolve: extern "C" fn(crate::vehicle::VehicleRef) -> MovementVehicle,
    pub depot_invalidate: extern "C" fn(crate::vehicle::VehicleRef),
    pub native_assert: extern "C" fn(bool),
    pub object_foundation: extern "C" fn(u32) -> ObjectFoundation,
    pub house_foundation: extern "C" fn(u32) -> CallbackFoundation,
    pub industry_foundation: extern "C" fn(u32) -> CallbackFoundation,
    pub industry_callback: extern "C" fn(u32, u16) -> u16,
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
    pub(crate) fn native_assert_call(&self, ok: bool) {
        if !ok {
            (self.native_assert)(ok);
        }
    }

    nested_tables! {
        pub(crate) fn shared -> Services;
        pub(crate) fn train -> crate::train::Leaves;
        pub(crate) fn vehicle -> crate::vehicle::VehicleServices;
        pub(crate) fn tunnel_bridge -> crate::tunnelbridge_cmd::TunnelBridgeServices;
        fn road_vehicle -> crate::road::Leaves;
    }
}
impl Context<'_> {
    fn foundation(&self, tile: u32, slope: u8) -> u8 {
        w::hit(w::MOVE_FOUNDATION);
        let kind = self.world.kind(TileIndex(tile));
        match kind {
            TileType::MP_RAILWAY => {
                if self.world.is_plain_rail(tile) {
                    rail_foundation(slope, self.world.tracks(tile), |ok| {
                        (self.native_assert)(ok);
                    })
                } else {
                    flattening_foundation_raw(slope)
                }
            }
            TileType::MP_ROAD => {
                if self.world.is_normal_road(tile) {
                    road_foundation(slope, self.world.all_road_bits(tile), |ok| {
                        (self.native_assert)(ok);
                    })
                } else {
                    flattening_foundation_raw(slope)
                }
            }
            TileType::MP_STATION => flattening_foundation_raw(slope),
            TileType::MP_TUNNELBRIDGE => {
                if self.world.tunnel(tile) {
                    FOUNDATION_NONE
                } else {
                    crate::bridge::get_bridge_foundation(
                        slope,
                        diag_dir_to_axis_raw(self.world.tunnel_bridge_dir(tile), |ok| {
                            (self.native_assert)(ok);
                        }),
                    )
                }
            }
            TileType::MP_HOUSE | TileType::MP_INDUSTRY => {
                let house = kind == TileType::MP_HOUSE;
                let p = if house {
                    (self.house_foundation)(tile)
                } else {
                    (self.industry_foundation)(tile)
                };
                if p.callback {
                    let result = if house {
                        unsafe { crate::town::resolver::foundation(TileIndex(tile), p.kind) }
                    } else {
                        (self.industry_callback)(tile, p.kind)
                    };
                    if result != crate::newgrf_callbacks::CALLBACK_FAILED
                        && !(self.shared().grf_boolean_callback)(
                            if house {
                                (self.shared().specs().house().file)(p.spec.cast())
                            } else {
                                unsafe {
                                    (*p.spec.cast::<crate::industrytype::IndustryTileSpec>())
                                        .grf_prop
                                        .grffile
                                }
                            },
                            if house {
                                crate::newgrf_callbacks::CBID_HOUSE_DRAW_FOUNDATIONS.0
                            } else {
                                crate::newgrf_callbacks::CBID_INDTILE_DRAW_FOUNDATIONS.0
                            },
                            result,
                        )
                    {
                        return FOUNDATION_NONE;
                    }
                }
                flattening_foundation_raw(slope)
            }
            TileType::MP_OBJECT => {
                let p = (self.object_foundation)(tile);
                if p.enabled && p.no_foundation {
                    FOUNDATION_NONE
                } else {
                    flattening_foundation_raw(slope)
                }
            }
            _ => FOUNDATION_NONE,
        }
    }
    #[inline]
    pub(crate) fn pixel_z_at(&self, x: i32, y: i32, ground: bool) -> i32 {
        let tile = self
            .world
            .dimensions
            .tile_virt_xy(x.cast_unsigned(), y.cast_unsigned())
            .0;
        let kind = self.world.kind(TileIndex(tile));
        // One named body for constant Rust callers and the dynamic native entry.
        // Observe the live tile type before selecting the ground specialization.
        if ground {
            self.pixel_z::<true>(tile, x as u32, y as u32, kind.0)
        } else {
            self.pixel_z::<false>(tile, x as u32, y as u32, kind.0)
        }
    }
    #[inline]
    fn pixel_z<const GROUND_VEHICLE: bool>(&self, tile: u32, x: u32, y: u32, kind: u8) -> i32 {
        w::hit(w::MOVE_PIXEL_Z);
        if matches!(kind, MP_HOUSE | MP_INDUSTRY | MP_STATION)
            || (kind == MP_RAILWAY && !self.world.is_plain_rail(tile))
            || (kind == MP_ROAD && !self.world.is_normal_road(tile))
            || (kind == MP_OBJECT
                && !unsafe {
                    ((*(*(*(self.shared().station_commands)()).command)
                        .rail_map
                        .ground)
                        .owned_land)(tile)
                })
        {
            return self.world.tile_max_pixel_z(tile);
        }
        let (slope, z) = if kind == MP_VOID {
            self.world
                .tile_pixel_slope_outside_map((x >> 4).cast_signed(), (y >> 4).cast_signed())
        } else {
            self.world.tile_pixel_slope(tile)
        };
        let mut p = FoundationSlope {
            z,
            slope: Slope(slope),
        };
        if matches!(kind, MP_RAILWAY | MP_ROAD) {
            if p.slope.0 == SLOPE_FLAT {
                return p.z;
            }
            let f = if kind == MP_RAILWAY {
                rail_foundation(p.slope.0, self.world.tracks(tile), |ok| {
                    (self.native_assert)(ok);
                })
            } else {
                road_foundation(p.slope.0, self.world.all_road_bits(tile), |ok| {
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
            if self.world.tunnel(tile) {
                if GROUND_VEHICLE {
                    return p.z;
                }
            } else {
                let dir = self.world.tunnel_bridge_dir(tile);
                let z = crate::landscape::apply_pixel_foundation_to_slope(
                    self.shared(),
                    Foundation(crate::bridge::get_bridge_foundation(
                        p.slope.0,
                        diag_dir_to_axis_raw(dir, |ok| (self.native_assert)(ok)),
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
impl MovementServices {
    #[inline]
    fn with_context<R>(&self, f: impl FnOnce(Context<'_>) -> R) -> R {
        // Standalone terrain and track-status calls end the descriptor before return.
        let world = World::new(self.shared());
        f(Context {
            s: self,
            world: &world,
        })
    }
    #[inline]
    pub(crate) fn pixel_z_at(&self, x: i32, y: i32, ground: bool) -> i32 {
        self.with_context(|c| c.pixel_z_at(x, y, ground))
    }
    #[inline]
    fn pixel_z<const GROUND: bool>(&self, tile: u32, x: u32, y: u32, kind: u8) -> i32 {
        self.with_context(|c| c.pixel_z::<GROUND>(tile, x, y, kind))
    }
    #[inline]
    fn foundation(&self, tile: u32, slope: u8) -> u8 {
        self.with_context(|c| c.foundation(tile, slope))
    }
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn track_status(&self, tile: u32, mode: u8, sub: u32, side: u8) -> u32 {
        self.with_context(|c| c.track_status(tile, mode, sub, side))
    }
    /// Use an enclosing synchronous entry's descriptor, loading fields at each query.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn track_status_in_world(
        &self,
        world: &World<'_>,
        tile: u32,
        mode: u8,
        sub: u32,
        side: u8,
    ) -> u32 {
        Context { s: self, world }.track_status(tile, mode, sub, side)
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
    let (slope, z, foundation) = s.with_context(|c| {
        let (slope, z) = c.world.tile_slope_z(tile);
        (slope, z, c.foundation(tile, slope))
    });
    let a = (s.shared().apply_foundation)(Foundation(foundation), Slope(slope));
    FoundationSlope {
        z: z.wrapping_add(a.z),
        slope: a.slope,
    }
}
pub(crate) fn rail_foundation(s: u8, b: u8, check: impl Fn(bool) + Copy) -> u8 {
    if b == TRACK_BIT_NONE {
        return FOUNDATION_NONE;
    }
    if is_steep_slope_raw(s) {
        if b == TRACK_BIT_X {
            return FOUNDATION_INCLINED_X;
        }
        if b == TRACK_BIT_Y {
            return FOUNDATION_INCLINED_Y;
        }
        let corner =
            get_highest_slope_corner_raw(s, || unreachable!("native highest slope corner"));
        let higher = crate::track_func::corner_to_track_bits(corner, check);
        if b == higher {
            return halftile_foundation_raw(corner, check);
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
                return halftile_foundation_raw(CORNER_N, check);
            }
            if s == SLOPE_S {
                return halftile_foundation_raw(CORNER_S, check);
            }
            return leveled;
        }
        TRACK_BIT_VERT => {
            if s == SLOPE_W {
                return halftile_foundation_raw(CORNER_W, check);
            }
            if s == SLOPE_E {
                return halftile_foundation_raw(CORNER_E, check);
            }
            return leveled;
        }
        TRACK_BIT_X => {
            return if is_slope_with_one_corner_raised_raw(s) {
                FOUNDATION_INCLINED_X
            } else {
                leveled
            };
        }
        TRACK_BIT_Y => {
            return if is_slope_with_one_corner_raised_raw(s) {
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
    if is_slope_with_three_corners_raised_raw(s) {
        return FOUNDATION_LEVELED;
    }
    if s & slope_with_three_corners_raised_raw(opposite_corner_raw(corner), check)
        == slope_with_one_corner_raised_raw(corner, check)
    {
        return halftile_foundation_raw(corner, check);
    }
    special_rail_foundation_raw(corner, check)
}
pub(crate) fn road_foundation(mut s: u8, b: u8, check: impl Fn(bool) + Copy) -> u8 {
    if s == SLOPE_FLAT || b == 0 {
        return FOUNDATION_NONE;
    }
    if is_steep_slope_raw(s) {
        s = slope_with_one_corner_raised_raw(
            get_highest_slope_corner_raw(s, || unreachable!("native highest slope corner")),
            check,
        );
    }
    if crate::road_cmd::INVALID_SLOPES[0][usize::from(s)] & b == 0 {
        return FOUNDATION_LEVELED;
    }
    if !is_slope_with_one_corner_raised_raw(s)
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

use crate::direction_func::{axis_to_diag_dir_raw, reverse_diag_dir_raw};
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
impl Context<'_> {
    fn water_edges(&self, tile: u32, mut bits: u8) -> u8 {
        let map = self.world.dimensions;
        if map.tile_x(TileIndex(tile)) == 0 {
            w::hit(w::MOVE_WATER_EDGE);
            bits &= !(TRACK_BIT_X | TRACK_BIT_UPPER | TRACK_BIT_RIGHT);
        }
        if map.tile_y(TileIndex(tile)) == 0 {
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
        match self.world.kind(TileIndex(tile)) {
            TileType::MP_RAILWAY => {
                w::hit(w::MOVE_TRACK_RAIL);
                if mode == TRANSPORT_WATER
                    && self.world.is_plain_rail(tile)
                    && self.world.rail_ground(tile) == RAIL_GROUND_HALF_TILE_WATER
                    && is_slope_with_one_corner_raised_raw(self.world.tile_slope(tile))
                {
                    w::hit(w::MOVE_HALF_WATER);
                    bits = match self.world.tracks(tile) {
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
                match self.world.rail_tile_type(tile) {
                    RAIL_TILE_NORMAL => bits = self.world.tracks(tile),
                    RAIL_TILE_SIGNALS => {
                        bits = self.world.tracks(tile);
                        w::hit(w::MOVE_SIGNAL_DIRECTIONS);
                        let a = self.world.present_signals(tile);
                        let mut b = u32::from(self.world.signal_states(tile) & a);
                        if !self.world.oneway_signal(tile, TRACK_UPPER)
                            || a & crate::signal_func::signal_on_track(TRACK_UPPER) == 0
                        {
                            b |= u32::from(!a & crate::signal_func::signal_on_track(TRACK_UPPER));
                        }
                        if !self.world.oneway_signal(tile, TRACK_LOWER)
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
                        let dir = self.world.rail_depot_dir(tile);
                        if side == INVALID_DIAGDIR || side == dir {
                            bits = diag_dir_to_diag_track_bits(dir, |ok| (self.native_assert)(ok));
                        }
                    }
                    _ => unreachable!("native rail tile type"),
                }
            }
            TileType::MP_ROAD => {
                w::hit(w::MOVE_TRACK_ROAD);
                match mode {
                    TRANSPORT_RAIL => {
                        if self.world.level_crossing(tile) {
                            bits = self.world.crossing_rail_bits(tile);
                        }
                    }
                    TRANSPORT_ROAD => {
                        let tram = sub != 0;
                        if !self.has_tile_road_type(tile, crate::services::RoadTramType(tram)) {
                            return 0;
                        }
                        match self.world.road_tile_type(tile).0 {
                            ROAD_TILE_NORMAL => {
                                let road = self.world.road_bits(tile, tram);
                                if side != INVALID_DIAGDIR
                                    && crate::road_func::diag_dir_to_road_bits_raw(side, |ok| {
                                        (self.native_assert)(ok);
                                    }) & road
                                        == 0
                                {
                                    return 0;
                                }
                                let (disallowed, road_works) = if tram {
                                    (0, self.world.has_road_works(tile))
                                } else {
                                    (
                                        self.world.disallowed_road_directions(tile),
                                        self.world.has_road_works(tile),
                                    )
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
                                let axis = self.world.crossing_road_axis(tile);
                                if side != INVALID_DIAGDIR
                                    && axis
                                        != diag_dir_to_axis_raw(side, |ok| (self.native_assert)(ok))
                                {
                                    return 0;
                                }
                                bits = axis_to_track_bits(axis, |ok| (self.native_assert)(ok));
                                if self.world.crossing_barred(tile) {
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
                                                axis_to_diag_dir_raw(axis, |ok| {
                                                    (self.native_assert)(ok);
                                                }),
                                                TRACKDIR_BIT_X_SW | TRACKDIR_BIT_Y_SE,
                                            ),
                                            (
                                                reverse_diag_dir_raw(
                                                    axis_to_diag_dir_raw(axis, |ok| {
                                                        (self.native_assert)(ok);
                                                    }),
                                                    |ok| (self.native_assert)(ok),
                                                ),
                                                TRACKDIR_BIT_X_NE | TRACKDIR_BIT_Y_NW,
                                            ),
                                        ] {
                                            let t = self
                                                .world
                                                .dimensions
                                                .tile_add_by_diag_dir_raw(tile, dir);
                                            if self.world.level_crossing_tile(t)
                                                && self.world.crossing_barred(t)
                                            {
                                                w::hit(w::MOVE_CROSSING_ESCAPE);
                                                red &= mask;
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {
                                let dir = self.world.road_depot_dir(tile);
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
            TileType::MP_STATION => {
                w::hit(w::MOVE_TRACK_STATION);
                match mode {
                    TRANSPORT_RAIL => {
                        if self.world.station_rail_raw(tile) && !self.world.station_blocked(tile) {
                            bits = crate::track_func::track_to_track_bits(
                                self.world.station_track(tile),
                                |ok| (self.native_assert)(ok),
                            );
                        }
                    }
                    TRANSPORT_WATER => {
                        if self.world.buoy(tile) {
                            bits = self.water_edges(tile, TRACK_BIT_ALL);
                        }
                    }
                    TRANSPORT_ROAD => {
                        if self.world.any_road_stop_tile(tile) {
                            if !self
                                .has_tile_road_type(tile, crate::services::RoadTramType(sub != 0))
                            {
                                return 0;
                            }
                            if self.world.bay_stop(tile) {
                                let dir = self.world.bay_dir(tile);
                                if side == INVALID_DIAGDIR || side == dir {
                                    bits = diag_dir_to_diag_track_bits(dir, |ok| {
                                        (self.native_assert)(ok);
                                    });
                                }
                            } else {
                                let axis = self.world.drive_through_axis(tile);
                                if side == INVALID_DIAGDIR
                                    || axis
                                        == diag_dir_to_axis_raw(side, |ok| (self.native_assert)(ok))
                                {
                                    bits = axis_to_track_bits(axis, |ok| (self.native_assert)(ok));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            TileType::MP_WATER => {
                w::hit(w::MOVE_TRACK_WATER);
                if mode != TRANSPORT_WATER {
                    return 0;
                }
                bits = match self.world.water_tile_type(tile) {
                    0 => {
                        if self.world.tile_flat(tile) {
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
                        ][usize::from(self.world.tile_slope(tile) & 15)]
                    }
                    2 => {
                        w::hit(w::MOVE_LOCK);
                        diag_dir_to_diag_track_bits(self.world.lock_direction(tile), |ok| {
                            (self.native_assert)(ok);
                        })
                    }
                    3 => {
                        w::hit(w::MOVE_SHIP_DEPOT);
                        axis_to_track_bits(self.world.ship_depot_axis(tile), |ok| {
                            (self.native_assert)(ok);
                        })
                    }
                    _ => return 0,
                };
                bits = self.water_edges(tile, bits);
            }
            TileType::MP_TUNNELBRIDGE => {
                w::hit(w::MOVE_TRACK_WORMHOLE);
                let transport = self.world.tunnel_bridge_transport(tile);
                if transport != mode
                    || (transport == TRANSPORT_ROAD
                        && !self.has_tile_road_type(tile, crate::services::RoadTramType(sub != 0)))
                {
                    return 0;
                }
                let dir = self.world.tunnel_bridge_dir(tile);
                if side != INVALID_DIAGDIR
                    && side != reverse_diag_dir_raw(dir, |ok| (self.native_assert)(ok))
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

use crate::direction_func::{diag_dir_to_dir_raw, dir_to_diag_dir_raw, reverse_dir_raw};
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
    fn hidden(v: VehicleRef, hidden: bool) {
        let status = (v).status();
        (v).write_status(if hidden {
            status | crate::vehicle_base::VS_HIDDEN
        } else {
            status & !crate::vehicle_base::VS_HIDDEN
        });
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
        let world = World::new(self.shared());
        if h.kind.0 != VEH_TRAIN || !world.rail_depot(tile) {
            return 0;
        }
        let dir = world.rail_depot_dir(tile);
        let fract = ((x & 15) + ((y & 15) << 4)) as u8;
        if FRACT_BEHIND[usize::from(dir)] == fract {
            return CANNOT_ENTER;
        }
        let direction = (v).direction();
        if direction == diag_dir_to_dir_raw(dir, |ok| (self.native_assert)(ok)) {
            let length = crate::train::next_offset(h.train);
            let enter = FRACT_ENTER[usize::from(dir)];
            let point = (i32::from(enter & 15) + (length + 1) * LEAVE_OFFSET[usize::from(dir)])
                .wrapping_add(
                    (i32::from(enter >> 4) + (length + 1) * LEAVE_OFFSET[usize::from(dir) + 4])
                        .wrapping_shl(4),
                ) as u8;
            if point == fract {
                w::hit(w::MOVE_TRAIN_DEPOT_LEAVE_0 + usize::from(dir));
                let next = (v).next();
                if let Some(next) = next {
                    Self::hidden(next, false);
                    let n = (self.resolve)(next);
                    n.train
                        .set_track(u64::from(diag_dir_to_diag_track_bits(dir, |ok| {
                            (self.native_assert)(ok);
                        })));
                }
            }
        } else if FRACT_ENTER[usize::from(dir)] == fract {
            w::hit(w::MOVE_TRAIN_DEPOT_ENTER_0 + usize::from(dir));
            (self.native_assert)(
                diag_dir_to_dir_raw(
                    reverse_diag_dir_raw(dir, |ok| (self.native_assert)(ok)),
                    |ok| {
                        (self.native_assert)(ok);
                    },
                ) == direction,
            );
            h.train.set_track(u64::from(TRACK_BIT_DEPOT));
            Self::hidden(v, true);
            (v).write_direction(reverse_dir_raw(direction, |ok| (self.native_assert)(ok)));
            if (v).next().is_none() {
                self.vehicle().enter_depot(v.first());
            }
            (v).write_tile(tile);
            (self.depot_invalidate)(v);
            return ENTERED_WORMHOLE;
        }
        0
    }
    unsafe fn enter_road(&self, h: &MovementVehicle, v: VehicleRef, tile: u32) -> u8 {
        let world = World::new(self.shared());
        if world.road_tile_type(tile).0 != ROAD_TILE_DEPOT || h.kind.0 != VEH_ROAD {
            return 0;
        }
        if unsafe { crate::road::get_frame(h.road) } == RVC_DEPOT_STOP_FRAME {
            let dir = world.road_depot_dir(tile);
            if ROAD_ENTER_DEPOT[usize::from(dir)] == unsafe { crate::road::get_state(h.road) } {
                unsafe {
                    w::hit(w::MOVE_ROAD_DEPOT_ENTER_0 + usize::from(dir));
                    crate::road::set_state(h.road, RVSB_IN_DEPOT);
                }
                Self::hidden(v, true);
                (v).write_direction(reverse_dir_raw((v).direction(), |ok| {
                    (self.native_assert)(ok);
                }));
                if (v).next().is_none() {
                    self.vehicle().enter_depot(v.first());
                }
                (v).write_tile(tile);
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
            if !World::new(self.shared()).is_rail_station_raw(tile)
                || !crate::vehicle_base::is_front_engine(
                    crate::vehicle_base::is_ground_vehicle(h.kind.0),
                    v.subtype(),
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
            let dir = dir_to_diag_dir_raw((v).direction(), |ok| (self.native_assert)(ok));
            x &= 15;
            y &= 15;
            if diag_dir_to_axis_raw(dir, |ok| (self.native_assert)(ok)) != AXIS_X {
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
                    (v).write_status((v).status() | crate::vehicle_base::VS_TRAIN_SLOWING);
                    let speed = ((stop - x) * 20 - 15).max(0) as u16;
                    if speed < (v).speed() {
                        (v).write_speed(speed);
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
                    v.subtype(),
                )
            {
                if unsafe { crate::road_stop::enter_by_tile_raw(self.shared(), tile, v, h.road) } {
                    w::hit(w::MOVE_ROAD_STOP_ADMIT);
                    return 0;
                }
                w::hit(w::MOVE_ROAD_STOP_REFUSE);
                return CANNOT_ENTER;
            }
        }
        0
    }
    fn prepare_bridge(v: VehicleRef) {
        let z = v.z();
        let ground = v.ground();
        let flags = crate::raw_field::read_field!(ground, flags);
        if flags & (1 << crate::ground_vehicle::GVF_GOINGUP_BIT) != 0 {
            w::hit(w::MOVE_BRIDGE_UP);
            (v).write_z(z.wrapping_add(1));
            crate::raw_field::write_field!(
                ground,
                flags,
                flags & !(1 << crate::ground_vehicle::GVF_GOINGUP_BIT)
            );
        } else {
            w::hit(w::MOVE_BRIDGE_DOWN);
            crate::raw_field::write_field!(
                ground,
                flags,
                flags & !(1 << crate::ground_vehicle::GVF_GOINGDOWN_BIT)
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
        let vehicle_z = v.z();
        let direction = v.direction();
        let z = slope.wrapping_sub(vehicle_z);
        if z.wrapping_abs() > 2 {
            return CANNOT_ENTER;
        }
        let world = World::new(self.shared());
        let dir = world.tunnel_bridge_dir(tile);
        let vdir = dir_to_diag_dir_raw(direction, |ok| (self.native_assert)(ok));
        let pos = (if diag_dir_to_axis_raw(vdir, |ok| (self.native_assert)(ok)) == AXIS_X {
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
        if world.tunnel(tile) {
            if h.kind.0 == VEH_TRAIN {
                let track = h.train.track();
                if track != TRACK_BIT_WORMHOLE && dir == vdir {
                    if crate::vehicle_base::is_front_engine(
                        crate::vehicle_base::is_ground_vehicle(h.kind.0),
                        v.subtype(),
                    ) && frame == TUNNEL_SOUND_FRAME
                    {
                        w::hit(w::MOVE_TRAIN_TUNNEL_SOUND);
                        if !(self.vehicle().play_vehicle_sound)(v, crate::newgrf_sound::VSE_TUNNEL)
                            && (self.vehicle().engine)(v).train_class() == 0
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
                        (v).write_tile(tile);
                        h.train.set_track(u64::from(TRACK_BIT_WORMHOLE));
                        Self::hidden(v, true);
                        return ENTERED_WORMHOLE;
                    }
                }
                if dir == reverse_diag_dir_raw(vdir, |ok| (self.native_assert)(ok))
                    && frame == TILE_SIZE as u8 - TUNNEL_VISIBILITY[usize::from(dir)]
                    && z == 0
                {
                    (v).write_tile(tile);
                    let track = diag_dir_to_diag_track_bits(vdir, |ok| (self.native_assert)(ok));
                    h.train.set_track(u64::from(track));
                    (self.native_assert)(track != 0);
                    w::hit(w::MOVE_TRAIN_TUNNEL_EXIT);
                    w::hit(w::MOVE_TRAIN_TUNNEL_EXIT_0 + usize::from(dir));
                    Self::hidden(v, false);
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
                        (v).write_tile(tile);
                        unsafe {
                            crate::road::set_state(h.road, RVSB_WORMHOLE);
                        }
                        Self::hidden(v, true);
                        return ENTERED_WORMHOLE;
                    }
                    return 0;
                }
                if dir == reverse_diag_dir_raw(vdir, |ok| (self.native_assert)(ok))
                    && frame == TILE_SIZE as u8 - TUNNEL_VISIBILITY[usize::from(dir)]
                    && z == 0
                {
                    (v).write_tile(tile);
                    unsafe {
                        crate::road::set_state(
                            h.road,
                            diag_dir_to_diag_trackdir(vdir, |ok| (self.native_assert)(ok)),
                        );
                        crate::road::set_frame(h.road, frame);
                    }
                    w::hit(w::MOVE_ROAD_TUNNEL_EXIT);
                    Self::hidden(v, false);
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
                let first = v.first();
                let first_speed = first.speed();
                first.write_speed(first_speed.min(speed));
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
                        h.train.set_track(u64::from(TRACK_BIT_WORMHOLE));
                        Self::prepare_bridge(v);
                    }
                    VEH_ROAD => {
                        unsafe {
                            crate::road::set_state(h.road, RVSB_WORMHOLE);
                        }
                        Self::prepare_bridge(v);
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
            } else if vdir == reverse_diag_dir_raw(dir, |ok| (self.native_assert)(ok)) {
                (v).write_tile(tile);
                match h.kind.0 {
                    VEH_TRAIN => {
                        if h.train.track() == TRACK_BIT_WORMHOLE {
                            h.train
                                .set_track(u64::from(diag_dir_to_diag_track_bits(vdir, |ok| {
                                    (self.native_assert)(ok);
                                })));
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
    let tile = v.vehicle().tile();
    let x = v.vehicle().x();
    let y = v.vehicle().y();
    let dir = World::new(s.shared()).rail_depot_dir(tile);
    let length = crate::train::next_offset(v);
    let enter = FRACT_ENTER[usize::from(dir)];
    match dir {
        0 => (x & 15) - (i32::from(enter & 15) - (length + 1)),
        1 => -((y & 15) - (i32::from(enter >> 4) + (length + 1))),
        2 => -((x & 15) - (i32::from(enter & 15) + (length + 1))),
        3 => (y & 15) - (i32::from(enter >> 4) - (length + 1)),
        _ => unreachable!("native depot direction"),
    }
}

/// Resolve the native immutable movement registry. Production callers hold the
/// game-state mutex; no map or owner borrow is formed by resolving this table.
#[inline]
pub(crate) fn table(shared: &Services) -> &MovementServices {
    // SAFETY: The shared table holds the immutable process-lifetime movement table.
    unsafe { &*shared.movement }
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
    let last = (v).last_station();
    // SAFETY: Both native leaves resolve the same serialized surviving shell.
    // The Order is owned by Rust and its facade aliases this stable allocation.
    unsafe {
        let owner = ((*s.orders).vehicle)(v.shell().cast());
        crate::orders::openttd_rust_order_should_stop(
            std::ptr::addr_of_mut!((*owner).current).cast(),
            last,
            station,
        ) != 0
    }
}

/// Lazy handler for header preconditions. Valid inline helpers never resolve the
/// native registry; failed assertions follow the native build's assertion policy.
#[inline]
pub(crate) fn native_assert(shared: &Services, condition: bool) {
    if !condition {
        (table(shared).native_assert)(condition);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn successful_header_preconditions_need_no_native_registry() {
        extern "C" fn random(_: *mut std::ffi::c_void) -> u32 {
            0
        }
        extern "C" fn industry(_: i32, _: i32, _: *mut u32) -> u32 {
            0
        }
        // The fixture has no movement registry, as before native first use.
        let shared = crate::services::fixture(std::ptr::null_mut(), random, industry);
        super::native_assert(&shared, true);
    }
}

crate::abi::layouts! {
    fn abi_layout;
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
    "OpenTTDMovementServices" ["landscape"] => MovementServices {
        shared,
        orders,
        train,
        vehicle,
        tunnel_bridge,
        road_compatible,
        road_type_speed,
        road_kind,
        road_vehicle,
        resolve,
        depot_invalidate,
        native_assert,
        object_foundation,
        house_foundation,
        industry_foundation,
        industry_callback,
    };
}

#[cfg(test)]
#[path = "movement/tests.rs"]
mod evidence_tests;
