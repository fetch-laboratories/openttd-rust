/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Tile movement policy: the `get_slope_z_proc`, `get_foundation_proc`,
//! `get_tile_track_status_proc` and `vehicle_enter_tile_proc` bodies of the rail,
//! road, station, water and tunnel/bridge tile types, with `GetRailFoundation`,
//! `GetRoadFoundation` and `TicksToLeaveDepot`. Map fields are loaded in place
//! through the canonical descriptor; pool fields remain typed services. No map
//! reference spans a callback.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names,
    clippy::if_not_else,
    clippy::too_many_lines,
    clippy::collapsible_match,
    clippy::cast_sign_loss
)]
use crate::direction_func::{
    axis_to_diag_dir, diag_dir_to_axis, diag_dir_to_dir, dir_to_diag_dir, reverse_diag_dir,
    reverse_dir,
};
use crate::direction_type::{Axis, DiagDirection, Direction};
use crate::map_func::World;
use crate::math::is_inside_bs;
use crate::rail_map::{RailGroundType, RailTileType};
use crate::road_map::RoadTileType;
use crate::road_type::{DisallowedRoadDirections, RoadBits, RoadTramType};
use crate::roadveh::{RVC_DEPOT_STOP_FRAME, RVSB_IN_DEPOT, RVSB_IN_ROAD_STOP, RVSB_WORMHOLE};
use crate::services::{FoundationSlope, Services};
use crate::slope_func::{
    flattening_foundation, get_highest_slope_corner, halftile_foundation,
    is_slope_with_one_corner_raised, is_slope_with_three_corners_raised, is_steep_slope,
    opposite_corner, slope_with_one_corner_raised, slope_with_three_corners_raised,
    special_rail_foundation,
};
use crate::slope_type::{Corner, Foundation, Slope};
use crate::station_type::StationID;
use crate::tile_cmd::{VehicleEnterTileState, VehicleEnterTileStates};
use crate::tile_type::{TILE_HEIGHT, TILE_SIZE, TILE_UNIT_MASK, TileIndex, TileType};
use crate::track_func::{
    axis_to_track_bits, combine_track_status, diag_dir_to_diag_track_bits,
    diag_dir_to_diag_trackdir, is_reversing_road_trackdir, track_bits_to_trackdir_bits,
};
use crate::track_type::{Track, TrackBits, TrackStatus, TrackdirBits};
use crate::transport_type::TransportType;
use crate::tunnelbridge::{TUNNEL_SOUND_FRAME, TUNNEL_VISIBILITY};
use crate::vehicle::VehicleRef;
use crate::vehicle_base::{VehState, VehStates};
use crate::vehicle_type::VehicleType;
use crate::water_map::WaterTileType;
use crate::witness as w;
use std::cell::{Cell, OnceCell};

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
    /// `GetTileTrackStatus(tile, TRANSPORT_ROAD, GetRoadTramType(v->roadtype))`.
    pub(crate) fn get_road_vehicle_track_status(
        &self,
        v: VehicleRef,
        tile: TileIndex,
    ) -> TrackStatus {
        if self.audited {
            let s = self.observed();
            let rtt = (s.road_kind)(v);
            s.get_tile_track_status(
                tile,
                TransportType::TRANSPORT_ROAD,
                u32::from(rtt.0),
                DiagDirection::INVALID_DIAGDIR,
            )
        } else {
            let s = self.get();
            let rtt = (s.road_kind)(v);
            s.get_tile_track_status(
                tile,
                TransportType::TRANSPORT_ROAD,
                u32::from(rtt.0),
                DiagDirection::INVALID_DIAGDIR,
            )
        }
    }
    /// Temporary raw adapter of [`Self::get_road_vehicle_track_status`] for
    /// `road.rs` and `ground_vehicle.rs` until their passes.
    pub(crate) fn road_tracks(&self, v: VehicleRef, tile: u32) -> u32 {
        self.get_road_vehicle_track_status(v, TileIndex(tile))
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
    fn has_tile_road_type(&self, tile: TileIndex, rtt: RoadTramType) -> bool {
        self.world
            .asserted(|m| crate::road_map::has_tile_road_type(m, tile.0, rtt))
    }
}

#[cfg(feature = "native-test-support")]
#[path = "native_test_support/movement.rs"]
pub(crate) mod native_test_support;

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
    pub resolve: extern "C" fn(crate::vehicle::VehicleRef) -> MovementVehicle,
    pub depot_invalidate: extern "C" fn(crate::vehicle::VehicleRef),
    pub native_assert: extern "C" fn(bool),
    pub objects: *const crate::objects::ObjectServices,
    pub town: *const crate::town::Leaves,
    pub industry_tiles: *const crate::industry::TileServices,
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
    /// The native `assert` policy as a header helper's precondition check.
    #[inline]
    fn check(&self) -> impl Fn(bool) + Copy + '_ {
        move |ok| (self.native_assert)(ok)
    }

    nested_tables! {
        pub(crate) fn shared -> Services;
        pub(crate) fn train -> crate::train::Leaves;
        pub(crate) fn vehicle -> crate::vehicle::VehicleServices;
        pub(crate) fn tunnel_bridge -> crate::tunnelbridge_cmd::TunnelBridgeServices;
        fn objects -> crate::objects::ObjectServices;
        fn town -> crate::town::Leaves;
        fn industry_tiles -> crate::industry::TileServices;
    }
}
impl Context<'_> {
    /// `_tile_type_procs[GetTileType(tile)]->get_foundation_proc(tile, tileh)`.
    fn get_foundation(&self, tile: TileIndex, tileh: Slope) -> Foundation {
        w::hit(w::MOVE_FOUNDATION);
        let kind = self.world.kind(tile);
        match kind {
            // `GetFoundation_Track`.
            TileType::MP_RAILWAY => {
                if self.world.is_plain_rail(tile.0) {
                    get_rail_foundation(tileh, TrackBits(self.world.tracks(tile.0)), self.check())
                } else {
                    flattening_foundation(tileh)
                }
            }
            // `GetFoundation_Road`.
            TileType::MP_ROAD => {
                if self.world.is_normal_road(tile.0) {
                    get_road_foundation(
                        tileh,
                        RoadBits(self.world.all_road_bits(tile.0)),
                        self.check(),
                    )
                } else {
                    flattening_foundation(tileh)
                }
            }
            // `GetFoundation_Station`.
            TileType::MP_STATION => flattening_foundation(tileh),
            // `GetFoundation_TunnelBridge`.
            TileType::MP_TUNNELBRIDGE => {
                if self.world.tunnel(tile.0) {
                    Foundation::FOUNDATION_NONE
                } else {
                    Foundation(crate::bridge::get_bridge_foundation(
                        tileh.0,
                        diag_dir_to_axis(
                            DiagDirection(self.world.tunnel_bridge_dir(tile.0)),
                            self.check(),
                        )
                        .0,
                    ))
                }
            }
            TileType::MP_HOUSE => {
                crate::town::house::get_foundation_town(self.town(), self.shared(), tile, tileh)
            }
            TileType::MP_INDUSTRY => crate::industry::get_foundation_industry(
                self.shared(),
                self.industry_tiles(),
                tile,
                tileh,
            ),
            TileType::MP_OBJECT => {
                crate::objects::get_foundation_object(self.objects(), tile, tileh)
            }
            // `GetFoundation_Clear`, `_Trees`, `_Water` and `_Void`.
            _ => Foundation::FOUNDATION_NONE,
        }
    }
    /// `GetSlopePixelZ`: the `get_slope_z_proc` of the tile at `TileVirtXY(x, y)`.
    #[inline]
    pub(crate) fn get_slope_pixel_z(&self, x: i32, y: i32, ground_vehicle: bool) -> i32 {
        let tile = self
            .world
            .dimensions
            .tile_virt_xy(x.cast_unsigned(), y.cast_unsigned());
        let kind = self.world.kind(tile);
        // One named body for constant Rust callers and the dynamic native entry.
        // Observe the live tile type before selecting the ground specialization.
        if ground_vehicle {
            self.get_slope_pixel_z_proc::<true>(tile, x as u32, y as u32, kind)
        } else {
            self.get_slope_pixel_z_proc::<false>(tile, x as u32, y as u32, kind)
        }
    }
    /// `_tile_type_procs[kind]->get_slope_z_proc(tile, x, y, ground_vehicle)`.
    #[inline]
    fn get_slope_pixel_z_proc<const GROUND_VEHICLE: bool>(
        &self,
        tile: TileIndex,
        x: u32,
        y: u32,
        kind: TileType,
    ) -> i32 {
        w::hit(w::MOVE_PIXEL_Z);
        // `GetSlopePixelZ_Town`, `_Industry`, `_Station`, and the non-plain rail,
        // non-normal road and non-owned-land object branches.
        if matches!(
            kind,
            TileType::MP_HOUSE | TileType::MP_INDUSTRY | TileType::MP_STATION
        ) || (kind == TileType::MP_RAILWAY && !self.world.is_plain_rail(tile.0))
            || (kind == TileType::MP_ROAD && !self.world.is_normal_road(tile.0))
            || (kind == TileType::MP_OBJECT
                && !unsafe {
                    ((*(*(*(self.shared().station_commands)()).command)
                        .rail_map
                        .ground)
                        .owned_land)(tile.0)
                })
        {
            return self.world.tile_max_pixel_z(tile);
        }
        let (mut tileh, mut z) = if kind == TileType::MP_VOID {
            /* This function may be called on tiles outside the map, don't assume
             * that 'tile' is a valid tile index. See GetSlopePixelZOutsideMap. */
            self.world
                .tile_pixel_slope_outside_map((x >> 4).cast_signed(), (y >> 4).cast_signed())
        } else {
            self.world.tile_pixel_slope(tile)
        };
        if matches!(kind, TileType::MP_RAILWAY | TileType::MP_ROAD) {
            if tileh == Slope::SLOPE_FLAT {
                return z;
            }
            let f = if kind == TileType::MP_RAILWAY {
                get_rail_foundation(tileh, TrackBits(self.world.tracks(tile.0)), self.check())
            } else {
                get_road_foundation(
                    tileh,
                    RoadBits(self.world.all_road_bits(tile.0)),
                    self.check(),
                )
            };
            z = z.wrapping_add(
                crate::landscape::apply_pixel_foundation_to_slope(self.shared(), f, &mut tileh)
                    .cast_signed(),
            );
        } else if kind == TileType::MP_TUNNELBRIDGE {
            if self.world.tunnel(tile.0) {
                /* In the tunnel entrance? */
                if GROUND_VEHICLE {
                    return z;
                }
            } else {
                // IsBridge(tile)
                let dir = DiagDirection(self.world.tunnel_bridge_dir(tile.0));
                let f = Foundation(crate::bridge::get_bridge_foundation(
                    tileh.0,
                    diag_dir_to_axis(dir, self.check()).0,
                ));
                z = z.wrapping_add(
                    crate::landscape::apply_pixel_foundation_to_slope(self.shared(), f, &mut tileh)
                        .cast_signed(),
                );

                /* On the bridge ramp? */
                if GROUND_VEHICLE {
                    if tileh != Slope::SLOPE_FLAT {
                        return z.wrapping_add(TILE_HEIGHT.cast_signed());
                    }

                    tileh = match dir {
                        DiagDirection::DIAGDIR_NE => Slope::SLOPE_NE,
                        DiagDirection::DIAGDIR_SE => Slope::SLOPE_SE,
                        DiagDirection::DIAGDIR_SW => Slope::SLOPE_SW,
                        DiagDirection::DIAGDIR_NW => Slope::SLOPE_NW,
                        _ => crate::services_assert::not_reached(),
                    };
                }
            }
        }
        z.wrapping_add(
            crate::landscape::partial_pixel_z((x & 0xF) as i32, (y & 0xF) as i32, tileh.0)
                .unwrap_or_else(|| crate::services_assert::not_reached())
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
    /// `GetSlopePixelZ`.
    #[inline]
    pub(crate) fn get_slope_pixel_z(&self, x: i32, y: i32, ground_vehicle: bool) -> i32 {
        self.with_context(|c| c.get_slope_pixel_z(x, y, ground_vehicle))
    }
    /// `GetSlopePixelZOutsideMap`: return the world Z coordinate at a point,
    /// which may lie outside the map.
    pub(crate) fn get_slope_pixel_z_outside_map(&self, x: i32, y: i32) -> i32 {
        w::hit(w::MOVE_OUTSIDE_Z);
        let inside = {
            let world = World::new(self.shared());
            is_inside_bs(
                x as usize,
                0,
                (world.dimensions.size_x() * TILE_SIZE) as usize,
            ) && is_inside_bs(y as usize, 0, (world.size_y() * TILE_SIZE) as usize)
        };
        if inside {
            self.get_slope_pixel_z(x, y, false)
        } else {
            self.with_context(|c| {
                c.get_slope_pixel_z_proc::<false>(
                    TileIndex::INVALID,
                    x as u32,
                    y as u32,
                    TileType::MP_VOID,
                )
            })
        }
    }
    /// `GetFoundationSlope`: the slope on top of the foundation and the z of the
    /// foundation slope.
    pub(crate) fn get_foundation_slope(&self, tile: TileIndex) -> (Slope, i32) {
        let (tileh, z, f) = self.with_context(|c| {
            let (tileh, z) = c.world.tile_slope_z(tile);
            (tileh, z, c.get_foundation(tile, tileh))
        });
        let applied = (self.shared().apply_foundation)(f, tileh);
        (applied.slope, z.wrapping_add(applied.z))
    }
    /// `GetTileTrackStatus`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn get_tile_track_status(
        &self,
        tile: TileIndex,
        mode: TransportType,
        sub_mode: u32,
        side: DiagDirection,
    ) -> TrackStatus {
        self.with_context(|c| c.get_tile_track_status(tile, mode, sub_mode, side))
    }
    /// `GetTileTrackStatus` within an enclosing synchronous entry's descriptor,
    /// loading fields at each query.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn get_tile_track_status_in_world(
        &self,
        world: &World<'_>,
        tile: TileIndex,
        mode: TransportType,
        sub_mode: u32,
        side: DiagDirection,
    ) -> TrackStatus {
        Context { s: self, world }.get_tile_track_status(tile, mode, sub_mode, side)
    }
    /// Temporary raw adapter of [`Self::get_tile_track_status`] for `train.rs`
    /// until its pass.
    #[inline]
    pub(crate) fn track_status(&self, tile: u32, mode: u8, sub: u32, side: u8) -> u32 {
        self.get_tile_track_status(
            TileIndex(tile),
            TransportType(mode),
            sub,
            DiagDirection(side),
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
    unsafe { &*s }.get_slope_pixel_z(x, y, ground_vehicle)
}
/// # Safety
/// Same as the in-map entry; outside coordinates never access an invalid tile.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_movement_outside_z(
    s: *const MovementServices,
    x: i32,
    y: i32,
) -> i32 {
    unsafe { &*s }.get_slope_pixel_z_outside_map(x, y)
}
/// # Safety
/// Immutable table, live tile; callbacks may reenter without a world borrow.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_movement_foundation_slope(
    s: *const MovementServices,
    tile: u32,
) -> FoundationSlope {
    let (slope, z) = unsafe { &*s }.get_foundation_slope(TileIndex(tile));
    FoundationSlope { z, slope }
}

/// `_valid_tracks_without_foundation`: valid `TrackBits` on a specific (non-steep)
/// slope without foundation.
const VALID_TRACKS_WITHOUT_FOUNDATION: [TrackBits; 15] = [
    TrackBits::TRACK_BIT_ALL,
    TrackBits::TRACK_BIT_RIGHT,
    TrackBits::TRACK_BIT_UPPER,
    TrackBits::TRACK_BIT_X,
    TrackBits::TRACK_BIT_LEFT,
    TrackBits::TRACK_BIT_NONE,
    TrackBits::TRACK_BIT_Y,
    TrackBits::TRACK_BIT_LOWER,
    TrackBits::TRACK_BIT_LOWER,
    TrackBits::TRACK_BIT_Y,
    TrackBits::TRACK_BIT_NONE,
    TrackBits::TRACK_BIT_LEFT,
    TrackBits::TRACK_BIT_X,
    TrackBits::TRACK_BIT_UPPER,
    TrackBits::TRACK_BIT_RIGHT,
];

/// `GetRailFoundation`: checks if a track combination is valid on a specific slope
/// and returns the needed foundation.
pub(crate) fn get_rail_foundation(
    tileh: Slope,
    bits: TrackBits,
    check: impl Fn(bool) + Copy,
) -> Foundation {
    if bits == TrackBits::TRACK_BIT_NONE {
        return Foundation::FOUNDATION_NONE;
    }

    if is_steep_slope(tileh) {
        /* Test for inclined foundations */
        if bits == TrackBits::TRACK_BIT_X {
            return Foundation::FOUNDATION_INCLINED_X;
        }
        if bits == TrackBits::TRACK_BIT_Y {
            return Foundation::FOUNDATION_INCLINED_Y;
        }

        /* Get higher track */
        let highest_corner =
            get_highest_slope_corner(tileh, || crate::services_assert::not_reached());
        let higher_track = TrackBits(crate::track_func::corner_to_track_bits(
            highest_corner.0,
            check,
        ));

        /* Only higher track? */
        if bits == higher_track {
            return halftile_foundation(highest_corner, check);
        }

        /* Overlap with higher track? */
        if crate::track_func::tracks_overlap((bits | higher_track).0) {
            return Foundation::FOUNDATION_INVALID;
        }

        /* either lower track or both higher and lower track */
        if bits & higher_track != TrackBits::TRACK_BIT_NONE {
            Foundation::FOUNDATION_STEEP_BOTH
        } else {
            Foundation::FOUNDATION_STEEP_LOWER
        }
    } else {
        if !VALID_TRACKS_WITHOUT_FOUNDATION[usize::from(tileh.0)] & bits
            == TrackBits::TRACK_BIT_NONE
        {
            return Foundation::FOUNDATION_NONE;
        }

        let valid_on_leveled = !TrackBits(crate::rail_cmd::LEVELED_TRACKS[usize::from(tileh.0)])
            & bits
            == TrackBits::TRACK_BIT_NONE;
        let leveled = if valid_on_leveled {
            Foundation::FOUNDATION_LEVELED
        } else {
            Foundation::FOUNDATION_INVALID
        };

        let track_corner = match bits {
            TrackBits::TRACK_BIT_LEFT => Corner::CORNER_W,
            TrackBits::TRACK_BIT_LOWER => Corner::CORNER_S,
            TrackBits::TRACK_BIT_RIGHT => Corner::CORNER_E,
            TrackBits::TRACK_BIT_UPPER => Corner::CORNER_N,

            TrackBits::TRACK_BIT_HORZ => {
                if tileh == Slope::SLOPE_N {
                    return halftile_foundation(Corner::CORNER_N, check);
                }
                if tileh == Slope::SLOPE_S {
                    return halftile_foundation(Corner::CORNER_S, check);
                }
                return leveled;
            }

            TrackBits::TRACK_BIT_VERT => {
                if tileh == Slope::SLOPE_W {
                    return halftile_foundation(Corner::CORNER_W, check);
                }
                if tileh == Slope::SLOPE_E {
                    return halftile_foundation(Corner::CORNER_E, check);
                }
                return leveled;
            }

            TrackBits::TRACK_BIT_X => {
                if is_slope_with_one_corner_raised(tileh) {
                    return Foundation::FOUNDATION_INCLINED_X;
                }
                return leveled;
            }

            TrackBits::TRACK_BIT_Y => {
                if is_slope_with_one_corner_raised(tileh) {
                    return Foundation::FOUNDATION_INCLINED_Y;
                }
                return leveled;
            }

            _ => return leveled,
        };
        /* Single diagonal track */

        /* Track must be at least valid on leveled foundation */
        if !valid_on_leveled {
            return Foundation::FOUNDATION_INVALID;
        }

        /* If slope has three raised corners, build leveled foundation */
        if is_slope_with_three_corners_raised(tileh) {
            return Foundation::FOUNDATION_LEVELED;
        }

        /* If neighboured corners of track_corner are lowered, build halftile foundation */
        if tileh & slope_with_three_corners_raised(opposite_corner(track_corner), check)
            == slope_with_one_corner_raised(track_corner, check)
        {
            return halftile_foundation(track_corner, check);
        }

        /* else special anti-zig-zag foundation */
        special_rail_foundation(track_corner, check)
    }
}
/// `GetRoadFoundation`: get the foundation type of a `RoadBits` slope combination.
pub(crate) fn get_road_foundation(
    mut tileh: Slope,
    bits: RoadBits,
    check: impl Fn(bool) + Copy,
) -> Foundation {
    /* Flat land and land without a road doesn't require a foundation */
    if tileh == Slope::SLOPE_FLAT || bits == RoadBits::ROAD_NONE {
        return Foundation::FOUNDATION_NONE;
    }

    /* Steep slopes behave the same as slopes with one corner raised. */
    if is_steep_slope(tileh) {
        tileh = slope_with_one_corner_raised(
            get_highest_slope_corner(tileh, || crate::services_assert::not_reached()),
            check,
        );
    }

    /* Leveled RoadBits on a slope */
    if RoadBits(crate::road_cmd::INVALID_SLOPES[0][usize::from(tileh.0)]) & bits
        == RoadBits::ROAD_NONE
    {
        return Foundation::FOUNDATION_LEVELED;
    }

    /* Straight roads without foundation on a slope */
    if !is_slope_with_one_corner_raised(tileh)
        && RoadBits(crate::road_cmd::INVALID_SLOPES[1][usize::from(tileh.0)]) & bits
            == RoadBits::ROAD_NONE
    {
        return Foundation::FOUNDATION_NONE;
    }

    /* Roads on steep Slopes or on Slopes with one corner raised */
    if bits == RoadBits::ROAD_X {
        Foundation::FOUNDATION_INCLINED_X
    } else {
        Foundation::FOUNDATION_INCLINED_Y
    }
}
/// Temporary raw adapter of [`get_rail_foundation`] for `rail_cmd.rs`, `water.rs`
/// and `ground.rs` until their passes.
pub(crate) fn rail_foundation(s: u8, b: u8, check: impl Fn(bool) + Copy) -> u8 {
    get_rail_foundation(Slope(s), TrackBits(b), check).0
}
/// Temporary raw adapter of [`get_road_foundation`] for `road_cmd.rs` until its pass.
pub(crate) fn road_foundation(s: u8, b: u8, check: impl Fn(bool) + Copy) -> u8 {
    get_road_foundation(Slope(s), RoadBits(b), check).0
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_foundation(
    tileh: Slope,
    bits: TrackBits,
    check: extern "C" fn(bool),
) -> Foundation {
    get_rail_foundation(tileh, bits, |ok| check(ok))
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_foundation(
    tileh: Slope,
    bits: RoadBits,
    check: extern "C" fn(bool),
) -> Foundation {
    get_road_foundation(tileh, bits, |ok| check(ok))
}

/// # Safety
/// The table and native tile remain valid for the synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_movement_foundation(
    s: *const MovementServices,
    tile: u32,
    tileh: Slope,
) -> Foundation {
    unsafe { &*s }.with_context(|c| c.get_foundation(TileIndex(tile), tileh))
}

/// `_road_trackbits`: converts `RoadBits` to `TrackBits`.
const ROAD_TRACKBITS: [TrackBits; 16] = [
    TrackBits::TRACK_BIT_NONE,  // ROAD_NONE
    TrackBits::TRACK_BIT_NONE,  // ROAD_NW
    TrackBits::TRACK_BIT_NONE,  // ROAD_SW
    TrackBits::TRACK_BIT_LEFT,  // ROAD_W
    TrackBits::TRACK_BIT_NONE,  // ROAD_SE
    TrackBits::TRACK_BIT_Y,     // ROAD_Y
    TrackBits::TRACK_BIT_LOWER, // ROAD_S
    TrackBits(
        TrackBits::TRACK_BIT_LEFT.0 | TrackBits::TRACK_BIT_LOWER.0 | TrackBits::TRACK_BIT_Y.0,
    ), // ROAD_Y | ROAD_SW
    TrackBits::TRACK_BIT_NONE,  // ROAD_NE
    TrackBits::TRACK_BIT_UPPER, // ROAD_N
    TrackBits::TRACK_BIT_X,     // ROAD_X
    TrackBits(
        TrackBits::TRACK_BIT_LEFT.0 | TrackBits::TRACK_BIT_UPPER.0 | TrackBits::TRACK_BIT_X.0,
    ), // ROAD_X | ROAD_NW
    TrackBits::TRACK_BIT_RIGHT, // ROAD_E
    TrackBits(
        TrackBits::TRACK_BIT_RIGHT.0 | TrackBits::TRACK_BIT_UPPER.0 | TrackBits::TRACK_BIT_Y.0,
    ), // ROAD_Y | ROAD_NE
    TrackBits(
        TrackBits::TRACK_BIT_RIGHT.0 | TrackBits::TRACK_BIT_LOWER.0 | TrackBits::TRACK_BIT_X.0,
    ), // ROAD_X | ROAD_SE
    TrackBits::TRACK_BIT_ALL,   // ROAD_ALL
];
/// `CombineTrackStatus(TrackBitsToTrackdirBits(bits), red_signals)`.
#[inline]
const fn track_bits_status(bits: TrackBits, red_signals: TrackdirBits) -> TrackStatus {
    combine_track_status(track_bits_to_trackdir_bits(bits.0), red_signals.0)
}
impl Context<'_> {
    /// `_tile_type_procs[GetTileType(tile)]->get_tile_track_status_proc(tile,
    /// mode, sub_mode, side)`.
    // Keep one policy body visible to constant propagation at literal Rust callers.
    // The native facade below still supplies dynamic transport/subtype/side values.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn get_tile_track_status(
        &self,
        tile: TileIndex,
        mode: TransportType,
        sub_mode: u32,
        side: DiagDirection,
    ) -> TrackStatus {
        match self.world.kind(tile) {
            TileType::MP_RAILWAY => self.get_tile_track_status_track(tile, mode, side),
            TileType::MP_ROAD => self.get_tile_track_status_road(tile, mode, sub_mode, side),
            TileType::MP_STATION => self.get_tile_track_status_station(tile, mode, sub_mode, side),
            TileType::MP_WATER => self.get_tile_track_status_water(tile, mode),
            TileType::MP_TUNNELBRIDGE => {
                self.get_tile_track_status_tunnel_bridge(tile, mode, sub_mode, side)
            }
            // `GetTileTrackStatus_Clear`, `_Town`, `_Trees`, `_Industry`, `_Object`
            // and `_Void`.
            _ => 0,
        }
    }
    /// `GetTileTrackStatus_Track`.
    #[inline]
    fn get_tile_track_status_track(
        &self,
        tile: TileIndex,
        mode: TransportType,
        side: DiagDirection,
    ) -> TrackStatus {
        w::hit(w::MOVE_TRACK_RAIL);
        /* Case of half tile slope with water. */
        if mode == TransportType::TRANSPORT_WATER
            && self.world.is_plain_rail(tile.0)
            && RailGroundType(self.world.rail_ground(tile.0)) == RailGroundType::HalfTileWater
            && is_slope_with_one_corner_raised(self.world.tile_slope(tile))
        {
            w::hit(w::MOVE_HALF_WATER);
            let tb = match TrackBits(self.world.tracks(tile.0)) {
                TrackBits::TRACK_BIT_UPPER => TrackBits::TRACK_BIT_LOWER,
                TrackBits::TRACK_BIT_LOWER => TrackBits::TRACK_BIT_UPPER,
                TrackBits::TRACK_BIT_LEFT => TrackBits::TRACK_BIT_RIGHT,
                TrackBits::TRACK_BIT_RIGHT => TrackBits::TRACK_BIT_LEFT,
                _ => crate::services_assert::not_reached(),
            };
            return track_bits_status(tb, TrackdirBits::TRACKDIR_BIT_NONE);
        }

        if mode != TransportType::TRANSPORT_RAIL {
            return 0;
        }

        let mut trackbits = TrackBits::TRACK_BIT_NONE;
        let mut red_signals = TrackdirBits::TRACKDIR_BIT_NONE;

        match RailTileType(self.world.rail_tile_type(tile.0)) {
            RailTileType::Normal => trackbits = TrackBits(self.world.tracks(tile.0)),

            RailTileType::Signals => {
                trackbits = TrackBits(self.world.tracks(tile.0));
                w::hit(w::MOVE_SIGNAL_DIRECTIONS);
                let a = self.world.present_signals(tile.0);
                let mut b = u32::from(self.world.signal_states(tile.0));

                b &= u32::from(a);

                /* When signals are not present (in neither direction),
                 * we pretend them to be green. Otherwise, it depends on
                 * the signal type. For signals that are only active from
                 * one side, we set the missing signals explicitly to
                 * `green'. Otherwise, they implicitly become `red'. */
                let upper = crate::signal_func::signal_on_track(Track::TRACK_UPPER);
                if !self.world.oneway_signal(tile.0, Track::TRACK_UPPER.0) || a & upper == 0 {
                    b |= u32::from(!a & upper);
                }
                let lower = crate::signal_func::signal_on_track(Track::TRACK_LOWER);
                if !self.world.oneway_signal(tile.0, Track::TRACK_LOWER.0) || a & lower == 0 {
                    b |= u32::from(!a & lower);
                }

                if b & 0x8 == 0 {
                    red_signals |= TrackdirBits::TRACKDIR_BIT_LEFT_N
                        | TrackdirBits::TRACKDIR_BIT_X_NE
                        | TrackdirBits::TRACKDIR_BIT_Y_SE
                        | TrackdirBits::TRACKDIR_BIT_UPPER_E;
                }
                if b & 0x4 == 0 {
                    red_signals |= TrackdirBits::TRACKDIR_BIT_LEFT_S
                        | TrackdirBits::TRACKDIR_BIT_X_SW
                        | TrackdirBits::TRACKDIR_BIT_Y_NW
                        | TrackdirBits::TRACKDIR_BIT_UPPER_W;
                }
                if b & 0x2 == 0 {
                    red_signals |=
                        TrackdirBits::TRACKDIR_BIT_RIGHT_N | TrackdirBits::TRACKDIR_BIT_LOWER_E;
                }
                if b & 0x1 == 0 {
                    red_signals |=
                        TrackdirBits::TRACKDIR_BIT_RIGHT_S | TrackdirBits::TRACKDIR_BIT_LOWER_W;
                }
            }

            RailTileType::Depot => {
                let dir = DiagDirection(self.world.rail_depot_dir(tile.0));

                if side == DiagDirection::INVALID_DIAGDIR || side == dir {
                    trackbits = TrackBits(diag_dir_to_diag_track_bits(dir.0, self.check()));
                }
            }

            _ => crate::services_assert::not_reached(),
        }

        track_bits_status(trackbits, red_signals)
    }
    /// `GetTileTrackStatus_Road`.
    #[inline]
    fn get_tile_track_status_road(
        &self,
        tile: TileIndex,
        mode: TransportType,
        sub_mode: u32,
        side: DiagDirection,
    ) -> TrackStatus {
        w::hit(w::MOVE_TRACK_ROAD);
        let mut trackdirbits = TrackdirBits::TRACKDIR_BIT_NONE;
        let mut red_signals = TrackdirBits::TRACKDIR_BIT_NONE; // crossing barred
        match mode {
            TransportType::TRANSPORT_RAIL => {
                if self.world.level_crossing(tile.0) {
                    trackdirbits = TrackdirBits(track_bits_to_trackdir_bits(
                        self.world.crossing_rail_bits(tile.0),
                    ));
                }
            }

            TransportType::TRANSPORT_ROAD => {
                let rtt = RoadTramType(sub_mode != 0);
                if !self.has_tile_road_type(tile, rtt) {
                    return 0;
                }
                match self.world.road_tile_type(tile.0) {
                    RoadTileType::Normal => {
                        const DRD_TO_MULTIPLIER: [u16; DisallowedRoadDirections::DRD_END.0
                            as usize] = [0x101, 0x100, 0x1, 0x0];
                        let bits = RoadBits(self.world.road_bits(tile.0, rtt.0));

                        /* no roadbit at this side of tile, return 0 */
                        if side != DiagDirection::INVALID_DIAGDIR
                            && crate::road_func::diag_dir_to_road_bits(side, self.check()) & bits
                                == RoadBits::ROAD_NONE
                        {
                            return 0;
                        }

                        let drd = if rtt == RoadTramType::RTT_TRAM {
                            DisallowedRoadDirections::DRD_NONE
                        } else {
                            DisallowedRoadDirections(self.world.disallowed_road_directions(tile.0))
                        };
                        let multiplier = DRD_TO_MULTIPLIER[usize::from(drd.0)];
                        if self.world.has_road_works(tile.0) {
                            w::hit(w::MOVE_ROAD_WORKS);
                        } else {
                            trackdirbits = TrackdirBits(
                                u16::from(ROAD_TRACKBITS[usize::from(bits.0)].0) * multiplier,
                            );
                        }
                    }

                    RoadTileType::Crossing => {
                        let axis = Axis(self.world.crossing_road_axis(tile.0));

                        if side != DiagDirection::INVALID_DIAGDIR
                            && axis != diag_dir_to_axis(side, self.check())
                        {
                            return 0;
                        }

                        trackdirbits = TrackdirBits(track_bits_to_trackdir_bits(
                            axis_to_track_bits(axis.0, self.check()),
                        ));
                        if self.world.crossing_barred(tile.0) {
                            w::hit(w::MOVE_CROSSING_BARRED);
                            red_signals = trackdirbits;
                            if crate::train::train_on_crossing(self.train(), self.shared(), tile) {
                                return combine_trackdir_status(trackdirbits, red_signals);
                            }

                            let mask_red_signal_bits_if_crossing_barred =
                                |t: TileIndex,
                                 mask: TrackdirBits,
                                 red_signals: &mut TrackdirBits| {
                                    if self.world.level_crossing_tile(t.0)
                                        && self.world.crossing_barred(t.0)
                                    {
                                        w::hit(w::MOVE_CROSSING_ESCAPE);
                                        *red_signals &= mask;
                                    }
                                };
                            let map = self.world.dimensions;
                            let south = axis_to_diag_dir(axis, self.check());
                            /* Check for blocked adjacent crossing to south, keep only southbound red signal trackdirs, allow northbound traffic */
                            mask_red_signal_bits_if_crossing_barred(
                                map.tile_add_by_diag_dir(
                                    tile,
                                    south,
                                    crate::map_func::native_check,
                                ),
                                TrackdirBits::TRACKDIR_BIT_X_SW | TrackdirBits::TRACKDIR_BIT_Y_SE,
                                &mut red_signals,
                            );
                            /* Check for blocked adjacent crossing to north, keep only northbound red signal trackdirs, allow southbound traffic */
                            mask_red_signal_bits_if_crossing_barred(
                                map.tile_add_by_diag_dir(
                                    tile,
                                    reverse_diag_dir(
                                        axis_to_diag_dir(axis, self.check()),
                                        self.check(),
                                    ),
                                    crate::map_func::native_check,
                                ),
                                TrackdirBits::TRACKDIR_BIT_X_NE | TrackdirBits::TRACKDIR_BIT_Y_NW,
                                &mut red_signals,
                            );
                        }
                    }

                    _ => {
                        // RoadTileType::Depot
                        let dir = DiagDirection(self.world.road_depot_dir(tile.0));

                        if side == DiagDirection::INVALID_DIAGDIR || side == dir {
                            trackdirbits = TrackdirBits(track_bits_to_trackdir_bits(
                                diag_dir_to_diag_track_bits(dir.0, self.check()),
                            ));
                        }
                    }
                }
            }

            _ => {}
        }
        combine_trackdir_status(trackdirbits, red_signals)
    }
    /// `GetTileTrackStatus_Station`.
    #[inline]
    fn get_tile_track_status_station(
        &self,
        tile: TileIndex,
        mode: TransportType,
        sub_mode: u32,
        side: DiagDirection,
    ) -> TrackStatus {
        w::hit(w::MOVE_TRACK_STATION);
        let mut trackbits = TrackBits::TRACK_BIT_NONE;

        match mode {
            TransportType::TRANSPORT_RAIL => {
                if self.world.has_station_rail(tile) && !self.world.is_station_tile_blocked(tile) {
                    trackbits = TrackBits(crate::track_func::track_to_track_bits(
                        self.world.get_rail_station_track(tile).0,
                        self.check(),
                    ));
                }
            }

            TransportType::TRANSPORT_WATER => {
                /* buoy is coded as a station, it is always on open water */
                if self.world.is_buoy(tile) {
                    trackbits = TrackBits::TRACK_BIT_ALL;
                    let map = self.world.dimensions;
                    /* remove tracks that connect NE map edge */
                    if map.tile_x(tile) == 0 {
                        w::hit(w::MOVE_WATER_EDGE);
                        trackbits &= !(TrackBits::TRACK_BIT_X
                            | TrackBits::TRACK_BIT_UPPER
                            | TrackBits::TRACK_BIT_RIGHT);
                    }
                    /* remove tracks that connect NW map edge */
                    if map.tile_y(tile) == 0 {
                        w::hit(w::MOVE_WATER_EDGE);
                        trackbits &= !(TrackBits::TRACK_BIT_Y
                            | TrackBits::TRACK_BIT_LEFT
                            | TrackBits::TRACK_BIT_UPPER);
                    }
                }
            }

            TransportType::TRANSPORT_ROAD => {
                if self
                    .world
                    .asserted(|m| crate::station_map::is_any_road_stop(m, tile))
                {
                    let rtt = RoadTramType(sub_mode != 0);
                    if !self.has_tile_road_type(tile, rtt) {
                        return 0;
                    }

                    if self.world.is_bay_road_stop_tile(tile) {
                        let dir = self.world.get_bay_road_stop_dir(tile);
                        if side == DiagDirection::INVALID_DIAGDIR || dir == side {
                            trackbits = TrackBits(diag_dir_to_diag_track_bits(dir.0, self.check()));
                        }
                    } else {
                        let axis = self.world.get_drive_through_stop_axis(tile);
                        if side == DiagDirection::INVALID_DIAGDIR
                            || axis == diag_dir_to_axis(side, self.check())
                        {
                            trackbits = TrackBits(axis_to_track_bits(axis.0, self.check()));
                        }
                    }
                }
            }

            _ => {}
        }

        track_bits_status(trackbits, TrackdirBits::TRACKDIR_BIT_NONE)
    }
    /// `GetTileTrackStatus_Water`.
    #[inline]
    fn get_tile_track_status_water(&self, tile: TileIndex, mode: TransportType) -> TrackStatus {
        const COAST_TRACKS: [TrackBits; 16] = [
            TrackBits::TRACK_BIT_NONE,
            TrackBits::TRACK_BIT_RIGHT,
            TrackBits::TRACK_BIT_UPPER,
            TrackBits::TRACK_BIT_NONE,
            TrackBits::TRACK_BIT_LEFT,
            TrackBits::TRACK_BIT_NONE,
            TrackBits::TRACK_BIT_NONE,
            TrackBits::TRACK_BIT_NONE,
            TrackBits::TRACK_BIT_LOWER,
            TrackBits::TRACK_BIT_NONE,
            TrackBits::TRACK_BIT_NONE,
            TrackBits::TRACK_BIT_NONE,
            TrackBits::TRACK_BIT_NONE,
            TrackBits::TRACK_BIT_NONE,
            TrackBits::TRACK_BIT_NONE,
            TrackBits::TRACK_BIT_NONE,
        ];

        w::hit(w::MOVE_TRACK_WATER);
        if mode != TransportType::TRANSPORT_WATER {
            return 0;
        }

        let mut ts = match WaterTileType(self.world.water_tile_type(tile.0)) {
            WaterTileType::Clear => {
                if self.world.tile_flat(tile) {
                    TrackBits::TRACK_BIT_ALL
                } else {
                    TrackBits::TRACK_BIT_NONE
                }
            }
            WaterTileType::Coast => {
                w::hit(w::MOVE_COAST);
                COAST_TRACKS[usize::from(self.world.tile_slope(tile).0 & 0xF)]
            }
            WaterTileType::Lock => {
                w::hit(w::MOVE_LOCK);
                TrackBits(diag_dir_to_diag_track_bits(
                    self.world.lock_direction(tile.0),
                    self.check(),
                ))
            }
            WaterTileType::Depot => {
                w::hit(w::MOVE_SHIP_DEPOT);
                TrackBits(axis_to_track_bits(
                    self.world.ship_depot_axis(tile.0),
                    self.check(),
                ))
            }
            _ => return 0,
        };
        let map = self.world.dimensions;
        if map.tile_x(tile) == 0 {
            w::hit(w::MOVE_WATER_EDGE);
            /* NE border: remove tracks that connects NE tile edge */
            ts &=
                !(TrackBits::TRACK_BIT_X | TrackBits::TRACK_BIT_UPPER | TrackBits::TRACK_BIT_RIGHT);
        }
        if map.tile_y(tile) == 0 {
            w::hit(w::MOVE_WATER_EDGE);
            /* NW border: remove tracks that connects NW tile edge */
            ts &=
                !(TrackBits::TRACK_BIT_Y | TrackBits::TRACK_BIT_LEFT | TrackBits::TRACK_BIT_UPPER);
        }
        track_bits_status(ts, TrackdirBits::TRACKDIR_BIT_NONE)
    }
    /// `GetTileTrackStatus_TunnelBridge`.
    #[inline]
    fn get_tile_track_status_tunnel_bridge(
        &self,
        tile: TileIndex,
        mode: TransportType,
        sub_mode: u32,
        side: DiagDirection,
    ) -> TrackStatus {
        w::hit(w::MOVE_TRACK_WORMHOLE);
        let transport_type = TransportType(self.world.tunnel_bridge_transport(tile.0));
        if transport_type != mode
            || (transport_type == TransportType::TRANSPORT_ROAD
                && !self.has_tile_road_type(tile, RoadTramType(sub_mode != 0)))
        {
            return 0;
        }

        let dir = DiagDirection(self.world.tunnel_bridge_dir(tile.0));
        if side != DiagDirection::INVALID_DIAGDIR && side != reverse_diag_dir(dir, self.check()) {
            return 0;
        }
        track_bits_status(
            TrackBits(diag_dir_to_diag_track_bits(dir.0, self.check())),
            TrackdirBits::TRACKDIR_BIT_NONE,
        )
    }
}
/// `CombineTrackStatus`.
#[inline]
const fn combine_trackdir_status(
    trackdirbits: TrackdirBits,
    red_signals: TrackdirBits,
) -> TrackStatus {
    combine_track_status(trackdirbits.0, red_signals.0)
}
/// # Safety
/// Native tables and map stay live and serialized for the call. No map reference
/// survives the already-owned train traversal, which is invoked directly.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_movement_track_status(
    s: *const MovementServices,
    tile: u32,
    mode: TransportType,
    sub_mode: u32,
    side: DiagDirection,
) -> u32 {
    unsafe { &*s }.get_tile_track_status(TileIndex(tile), mode, sub_mode, side)
}

/// `_fractcoords_behind`.
const FRACTCOORDS_BEHIND: [u8; 4] = [0x8F, 0x8, 0x80, 0xF8];
/// `_fractcoords_enter`.
const FRACTCOORDS_ENTER: [u8; 4] = [0x8A, 0x48, 0x84, 0xA8];
/// `_deltacoord_leaveoffset`.
const DELTACOORD_LEAVEOFFSET: [i8; 8] = [-1, 0, 1, 0 /* x */, 0, 1, 0, -1 /* y */];
/// `_roadveh_enter_depot_dir`: given the direction the road depot is pointing,
/// this is the direction the vehicle should be travelling in in order to enter
/// the depot.
const ROADVEH_ENTER_DEPOT_DIR: [u8; 4] = [
    crate::track_type::Trackdir::TRACKDIR_X_SW.0,
    crate::track_type::Trackdir::TRACKDIR_Y_NW.0,
    crate::track_type::Trackdir::TRACKDIR_X_NE.0,
    crate::track_type::Trackdir::TRACKDIR_Y_SE.0,
];
impl MovementServices {
    /// `v->vehstatus.Set(VehState::Hidden)` or `.Reset(VehState::Hidden)`.
    fn set_hidden(v: VehicleRef, hidden: bool) {
        let mask = VehStates::from_element(VehState::Hidden).0;
        let status = v.status();
        v.write_status(if hidden {
            status | mask
        } else {
            status & !mask
        });
    }
    /// `VehicleEnter_Track`: tile callback routine when vehicle enters tile.
    // All owner accesses are scalar loads/stores and end before calling native.
    // Depot entry queues autoreplace later in the tick; it preserves this shell
    // and every chain shell used by the original subsequent tile/window stores.
    unsafe fn vehicle_enter_track(
        &self,
        h: &MovementVehicle,
        u: VehicleRef,
        tile: TileIndex,
        x: i32,
        y: i32,
    ) -> VehicleEnterTileStates {
        let world = World::new(self.shared());
        /* This routine applies only to trains in depot tiles. */
        if h.kind != VehicleType::VEH_TRAIN || !world.rail_depot(tile.0) {
            return VehicleEnterTileStates(0);
        }

        /* Depot direction. */
        let dir = DiagDirection(world.rail_depot_dir(tile.0));

        let fract_coord = ((x & 0xF) + ((y & 0xF) << 4)) as u8;

        /* Make sure a train is not entering the tile from behind. */
        if FRACTCOORDS_BEHIND[usize::from(dir.0)] == fract_coord {
            return VehicleEnterTileStates::from_element(VehicleEnterTileState::CannotEnter);
        }

        let v = u;
        let direction = Direction(v.direction());

        /* Leaving depot? */
        if direction == diag_dir_to_dir(dir, self.check()) {
            /* Calculate the point where the following wagon should be activated. */
            let length = crate::train::calc_next_vehicle_offset(h.train);

            let enter = FRACTCOORDS_ENTER[usize::from(dir.0)];
            let fract_coord_leave = (i32::from(enter & 0x0F) // x
                + (length + 1) * i32::from(DELTACOORD_LEAVEOFFSET[usize::from(dir.0)]))
            .wrapping_add(
                (i32::from(enter >> 4) // y
                    + (length + 1) * i32::from(DELTACOORD_LEAVEOFFSET[usize::from(dir.0) + 4]))
                .wrapping_shl(4),
            ) as u8;

            if fract_coord_leave == fract_coord {
                /* Leave the depot. */
                w::hit(w::MOVE_TRAIN_DEPOT_LEAVE_0 + usize::from(dir.0));
                if let Some(next) = v.next() {
                    Self::set_hidden(next, false);
                    let track = if diag_dir_to_axis(dir, self.check()) == Axis::AXIS_X {
                        TrackBits::TRACK_BIT_X
                    } else {
                        TrackBits::TRACK_BIT_Y
                    };
                    (self.resolve)(next).train.set_track(u64::from(track.0));
                }
            }
        } else if FRACTCOORDS_ENTER[usize::from(dir.0)] == fract_coord {
            /* Entering depot. */
            w::hit(w::MOVE_TRAIN_DEPOT_ENTER_0 + usize::from(dir.0));
            self.native_assert_call(
                diag_dir_to_dir(reverse_diag_dir(dir, self.check()), self.check()) == direction,
            );
            h.train.set_track(u64::from(TrackBits::TRACK_BIT_DEPOT.0));
            Self::set_hidden(v, true);
            v.write_direction(reverse_dir(direction, self.check()).0);
            if v.next().is_none() {
                self.vehicle().enter_depot(v.first());
            }
            v.write_tile(tile.0);

            (self.depot_invalidate)(v);
            return VehicleEnterTileStates::from_element(VehicleEnterTileState::EnteredWormhole);
        }

        VehicleEnterTileStates(0)
    }
    /// `VehicleEnter_Road`.
    unsafe fn vehicle_enter_road(
        &self,
        h: &MovementVehicle,
        v: VehicleRef,
        tile: TileIndex,
    ) -> VehicleEnterTileStates {
        let world = World::new(self.shared());
        if world.road_tile_type(tile.0) == RoadTileType::Depot {
            if h.kind != VehicleType::VEH_ROAD {
                return VehicleEnterTileStates(0);
            }

            let rv = h.road;
            if unsafe { crate::road::get_frame(rv) } == RVC_DEPOT_STOP_FRAME {
                let dir = world.road_depot_dir(tile.0);
                if ROADVEH_ENTER_DEPOT_DIR[usize::from(dir)]
                    == unsafe { crate::road::get_state(rv) }
                {
                    w::hit(w::MOVE_ROAD_DEPOT_ENTER_0 + usize::from(dir));
                    unsafe {
                        crate::road::set_state(rv, RVSB_IN_DEPOT);
                    }
                    Self::set_hidden(v, true);
                    v.write_direction(reverse_dir(Direction(v.direction()), self.check()).0);
                    if v.next().is_none() {
                        self.vehicle().enter_depot(v.first());
                    }
                    v.write_tile(tile.0);

                    (self.depot_invalidate)(v);
                    return VehicleEnterTileStates::from_element(
                        VehicleEnterTileState::EnteredWormhole,
                    );
                }
            }
        }
        VehicleEnterTileStates(0)
    }
    /// `VehicleEnter_Station`.
    unsafe fn vehicle_enter_station(
        &self,
        h: &MovementVehicle,
        v: VehicleRef,
        tile: TileIndex,
        mut x: i32,
        mut y: i32,
    ) -> VehicleEnterTileStates {
        if h.kind == VehicleType::VEH_TRAIN {
            let station_id = World::new(self.shared()).get_station_index(tile);
            if !should_stop_at_station(self.shared(), v, station_id) {
                return VehicleEnterTileStates(0);
            }
            if !World::new(self.shared()).is_rail_station(tile) || !v.is_front_engine() {
                return VehicleEnterTileStates(0);
            }

            let (mut stop, station_ahead, station_length) = crate::train::get_train_stop_location(
                self.train(),
                self.shared(),
                h.train,
                station_id,
                tile,
            );

            /* Stop whenever that amount of station ahead + the distance from the
             * begin of the platform to the stop location is longer than the length
             * of the platform. Station ahead 'includes' the current tile where the
             * vehicle is on, so we need to subtract that. */
            if stop
                .wrapping_add(station_ahead)
                .wrapping_sub(TILE_SIZE.cast_signed())
                >= station_length
            {
                return VehicleEnterTileStates(0);
            }

            let dir = dir_to_diag_dir(Direction(v.direction()), self.check());

            x &= 0xF;
            y &= 0xF;

            if diag_dir_to_axis(dir, self.check()) != Axis::AXIS_X {
                std::mem::swap(&mut x, &mut y);
            }
            if y == TILE_SIZE.cast_signed() / 2 {
                if dir != DiagDirection::DIAGDIR_SE && dir != DiagDirection::DIAGDIR_SW {
                    x = TILE_SIZE.cast_signed() - 1 - x;
                }
                stop &= TILE_SIZE.cast_signed() - 1;

                if x == stop {
                    w::hit(w::MOVE_STATION_STOP);
                    return VehicleEnterTileStates::from_element(
                        VehicleEnterTileState::EnteredStation,
                    ); // enter station
                } else if x < stop {
                    w::hit(w::MOVE_STATION_SLOW);
                    v.write_status(v.status() | VehStates::from_element(VehState::TrainSlowing).0);
                    let spd = ((stop - x) * 20 - 15).max(0) as u16;
                    if spd < v.speed() {
                        v.write_speed(spd);
                    }
                }
            }
        } else if h.kind == VehicleType::VEH_ROAD {
            let rv = h.road;
            let state = unsafe { crate::road::get_state(rv) };
            if state < RVSB_IN_ROAD_STOP
                && !is_reversing_road_trackdir(state, self.check())
                && unsafe { crate::road::get_frame(rv) } == 0
                && World::new(self.shared())
                    .asserted(|m| crate::station_map::is_station_road_stop(m, tile))
                && v.is_front_engine()
            {
                /* Attempt to allocate a parking bay in a road stop */
                if unsafe { crate::road_stop::enter_by_tile(self.shared(), tile, v, rv) } {
                    w::hit(w::MOVE_ROAD_STOP_ADMIT);
                    return VehicleEnterTileStates(0);
                }
                w::hit(w::MOVE_ROAD_STOP_REFUSE);
                return VehicleEnterTileStates::from_element(VehicleEnterTileState::CannotEnter);
            }
        }

        VehicleEnterTileStates(0)
    }
    /// `PrepareToEnterBridge`: clears the ground vehicle's slope flags and, when
    /// it was going up, updates the z coordinate that `UpdateInclination` will not.
    fn prepare_to_enter_bridge(gv: VehicleRef) {
        let ground = gv.ground();
        let mut flags: u16 = crate::raw_field::read_field!(ground, flags);
        if crate::bitmath_func::has_bit(flags, crate::ground_vehicle::GVF_GOINGUP_BIT) {
            w::hit(w::MOVE_BRIDGE_UP);
            gv.write_z(gv.z().wrapping_add(1));
            crate::bitmath_func::clr_bit16(&mut flags, crate::ground_vehicle::GVF_GOINGUP_BIT);
        } else {
            w::hit(w::MOVE_BRIDGE_DOWN);
            crate::bitmath_func::clr_bit16(&mut flags, crate::ground_vehicle::GVF_GOINGDOWN_BIT);
        }
        crate::raw_field::write_field!(ground, flags, flags);
    }
    /// `VehicleEnter_TunnelBridge`.
    unsafe fn vehicle_enter_tunnel_bridge(
        &self,
        h: &MovementVehicle,
        v: VehicleRef,
        tile: TileIndex,
        x: i32,
        y: i32,
    ) -> VehicleEnterTileStates {
        let z = self.get_slope_pixel_z(x, y, true).wrapping_sub(v.z());

        if z.wrapping_abs() > 2 {
            return VehicleEnterTileStates::from_element(VehicleEnterTileState::CannotEnter);
        }
        let world = World::new(self.shared());
        /* Direction into the wormhole */
        let dir = DiagDirection(world.tunnel_bridge_dir(tile.0));
        /* Direction of the vehicle */
        let vdir = dir_to_diag_dir(Direction(v.direction()), self.check());
        /* New position of the vehicle on the tile */
        let pos = (if diag_dir_to_axis(vdir, self.check()) == Axis::AXIS_X {
            x
        } else {
            y
        }) as u8
            & TILE_UNIT_MASK as u8;
        /* Number of units moved by the vehicle since entering the tile */
        let frame = if vdir == DiagDirection::DIAGDIR_NE || vdir == DiagDirection::DIAGDIR_NW {
            TILE_SIZE as u8 - 1 - pos
        } else {
            pos
        };
        let visibility_frame = TUNNEL_VISIBILITY[usize::from(dir.0)];

        if world.tunnel(tile.0) {
            if h.kind == VehicleType::VEH_TRAIN {
                let t = h.train;

                if TrackBits(t.track()) != TrackBits::TRACK_BIT_WORMHOLE && dir == vdir {
                    if v.is_front_engine() && frame == TUNNEL_SOUND_FRAME {
                        w::hit(w::MOVE_TRAIN_TUNNEL_SOUND);
                        if !(self.vehicle().play_vehicle_sound)(v, crate::newgrf_sound::VSE_TUNNEL)
                            && crate::engine_type::EngineClass(
                                (self.vehicle().engine)(v).train_class(),
                            ) == crate::engine_type::EngineClass::EC_STEAM
                        {
                            (self.vehicle().play_sound)(
                                v,
                                crate::sound_type::SND_05_TRAIN_THROUGH_TUNNEL,
                            );
                        }
                        return VehicleEnterTileStates(0);
                    }
                    if frame == visibility_frame {
                        w::hit(w::MOVE_TRAIN_TUNNEL_ENTER);
                        w::hit(w::MOVE_TRAIN_TUNNEL_ENTER_0 + usize::from(dir.0));
                        v.write_tile(tile.0);
                        t.set_track(u64::from(TrackBits::TRACK_BIT_WORMHOLE.0));
                        Self::set_hidden(v, true);
                        return VehicleEnterTileStates::from_element(
                            VehicleEnterTileState::EnteredWormhole,
                        );
                    }
                }

                if dir == reverse_diag_dir(vdir, self.check())
                    && frame == TILE_SIZE as u8 - visibility_frame
                    && z == 0
                {
                    /* We're at the tunnel exit ?? */
                    v.write_tile(tile.0);
                    t.set_track(u64::from(diag_dir_to_diag_track_bits(vdir.0, self.check())));
                    self.native_assert_call(TrackBits(t.track()) != TrackBits::TRACK_BIT_NONE);
                    w::hit(w::MOVE_TRAIN_TUNNEL_EXIT);
                    w::hit(w::MOVE_TRAIN_TUNNEL_EXIT_0 + usize::from(dir.0));
                    Self::set_hidden(v, false);
                    return VehicleEnterTileStates::from_element(
                        VehicleEnterTileState::EnteredWormhole,
                    );
                }
            } else if h.kind == VehicleType::VEH_ROAD {
                let rv = h.road;

                /* Enter tunnel? */
                if unsafe { crate::road::get_state(rv) } != RVSB_WORMHOLE && dir == vdir {
                    if frame == visibility_frame {
                        w::hit(w::MOVE_ROAD_TUNNEL_ENTER);
                        /* Frame should be equal to the next frame number in the RV's movement */
                        self.native_assert_call(
                            i32::from(frame)
                                == i32::from(unsafe { crate::road::get_frame(rv) }) + 1,
                        );
                        v.write_tile(tile.0);
                        unsafe {
                            crate::road::set_state(rv, RVSB_WORMHOLE);
                        }
                        Self::set_hidden(v, true);
                        return VehicleEnterTileStates::from_element(
                            VehicleEnterTileState::EnteredWormhole,
                        );
                    }
                    return VehicleEnterTileStates(0);
                }

                /* We're at the tunnel exit ?? */
                if dir == reverse_diag_dir(vdir, self.check())
                    && frame == TILE_SIZE as u8 - visibility_frame
                    && z == 0
                {
                    v.write_tile(tile.0);
                    unsafe {
                        crate::road::set_state(rv, diag_dir_to_diag_trackdir(vdir.0, self.check()));
                        crate::road::set_frame(rv, frame);
                    }
                    w::hit(w::MOVE_ROAD_TUNNEL_EXIT);
                    Self::set_hidden(v, false);
                    return VehicleEnterTileStates::from_element(
                        VehicleEnterTileState::EnteredWormhole,
                    );
                }
            }
        } else {
            // IsBridge(tile)
            if h.kind != VehicleType::VEH_SHIP {
                /* modify speed of vehicle */
                let mut spd = (self.shared().bridge_speed)(tile.0);

                if h.kind == VehicleType::VEH_ROAD {
                    if spd > u16::MAX / 2 {
                        w::hit(w::MOVE_BRIDGE_SPEED_NARROW);
                    }
                    spd = spd.wrapping_mul(2);
                }
                let first = v.first();
                first.write_speed(first.speed().min(spd));
            }

            if vdir == dir {
                /* Vehicle enters bridge at the last frame inside this tile. */
                if frame != TILE_SIZE as u8 - 1 {
                    return VehicleEnterTileStates(0);
                }
                match h.kind {
                    VehicleType::VEH_TRAIN => {
                        w::hit(w::MOVE_TRAIN_BRIDGE_ENTER);
                        h.train
                            .set_track(u64::from(TrackBits::TRACK_BIT_WORMHOLE.0));
                        Self::prepare_to_enter_bridge(v);
                    }

                    VehicleType::VEH_ROAD => {
                        w::hit(w::MOVE_ROAD_BRIDGE_ENTER);
                        unsafe {
                            crate::road::set_state(h.road, RVSB_WORMHOLE);
                        }
                        Self::prepare_to_enter_bridge(v);
                    }

                    VehicleType::VEH_SHIP => unsafe {
                        w::hit(w::MOVE_SHIP_BRIDGE_ENTER);
                        crate::ship_control::set_state(h.ship, TrackBits::TRACK_BIT_WORMHOLE);
                    },

                    _ => crate::services_assert::not_reached(),
                }
                return VehicleEnterTileStates::from_element(
                    VehicleEnterTileState::EnteredWormhole,
                );
            } else if vdir == reverse_diag_dir(dir, self.check()) {
                v.write_tile(tile.0);
                match h.kind {
                    VehicleType::VEH_TRAIN => {
                        if TrackBits(h.train.track()) == TrackBits::TRACK_BIT_WORMHOLE {
                            h.train.set_track(u64::from(diag_dir_to_diag_track_bits(
                                vdir.0,
                                self.check(),
                            )));
                            w::hit(w::MOVE_TRAIN_BRIDGE_EXIT);
                            return VehicleEnterTileStates::from_element(
                                VehicleEnterTileState::EnteredWormhole,
                            );
                        }
                    }

                    VehicleType::VEH_ROAD => {
                        if unsafe { crate::road::get_state(h.road) } == RVSB_WORMHOLE {
                            unsafe {
                                crate::road::set_state(
                                    h.road,
                                    diag_dir_to_diag_trackdir(vdir.0, self.check()),
                                );
                                crate::road::set_frame(h.road, 0);
                            }
                            w::hit(w::MOVE_ROAD_BRIDGE_EXIT);
                            return VehicleEnterTileStates::from_element(
                                VehicleEnterTileState::EnteredWormhole,
                            );
                        }
                    }

                    VehicleType::VEH_SHIP => {
                        if unsafe { crate::ship_control::get_state(h.ship) }
                            == TrackBits::TRACK_BIT_WORMHOLE
                        {
                            unsafe {
                                crate::ship_control::set_state(
                                    h.ship,
                                    TrackBits(diag_dir_to_diag_track_bits(vdir.0, self.check())),
                                );
                            }
                            w::hit(w::MOVE_SHIP_BRIDGE_EXIT);
                            return VehicleEnterTileStates::from_element(
                                VehicleEnterTileState::EnteredWormhole,
                            );
                        }
                    }

                    _ => crate::services_assert::not_reached(),
                }
            }
        }
        VehicleEnterTileStates(0)
    }
    /// `VehicleEnterTile`: call the tile callback function for a vehicle entering
    /// a tile. Only the rail, road, station and tunnel/bridge procedures act; the
    /// others return no state.
    ///
    /// # Safety
    /// Tables/vehicle/map stay live and serialized. Selected callbacks preserve the
    /// current shell; all scalar owner accesses end before every callback/reentry.
    pub(crate) unsafe fn vehicle_enter_tile(
        &self,
        v: VehicleRef,
        tile: TileIndex,
        x: i32,
        y: i32,
    ) -> VehicleEnterTileStates {
        let kind = World::new(self.shared()).kind(tile);
        if !matches!(
            kind,
            TileType::MP_RAILWAY
                | TileType::MP_ROAD
                | TileType::MP_STATION
                | TileType::MP_TUNNELBRIDGE
        ) {
            return VehicleEnterTileStates(0);
        }
        let h = (self.resolve)(v);
        unsafe {
            match kind {
                TileType::MP_RAILWAY => self.vehicle_enter_track(&h, v, tile, x, y),
                TileType::MP_ROAD => self.vehicle_enter_road(&h, v, tile),
                TileType::MP_STATION => self.vehicle_enter_station(&h, v, tile, x, y),
                _ => self.vehicle_enter_tunnel_bridge(&h, v, tile, x, y),
            }
        }
    }
    /// `TicksToLeaveDepot`: compute number of ticks when next wagon will leave a
    /// depot. Negative means next wagon should have left depot n ticks before.
    pub(crate) fn ticks_to_leave_depot(&self, v: crate::train::Handle) -> i32 {
        w::hit(w::MOVE_DEPOT_TICKS);
        let dir = DiagDirection(World::new(self.shared()).rail_depot_dir(v.vehicle().tile()));
        let length = crate::train::calc_next_vehicle_offset(v);
        let enter = FRACTCOORDS_ENTER[usize::from(dir.0)];
        let x_pos = v.vehicle().x();
        let y_pos = v.vehicle().y();

        match dir {
            DiagDirection::DIAGDIR_NE => (x_pos & 0x0F) - (i32::from(enter & 0x0F) - (length + 1)),
            DiagDirection::DIAGDIR_SE => -((y_pos & 0x0F) - (i32::from(enter >> 4) + (length + 1))),
            DiagDirection::DIAGDIR_SW => {
                -((x_pos & 0x0F) - (i32::from(enter & 0x0F) + (length + 1)))
            }
            DiagDirection::DIAGDIR_NW => (y_pos & 0x0F) - (i32::from(enter >> 4) - (length + 1)),
            _ => crate::services_assert::not_reached(),
        }
    }
}
/// # Safety
/// As [`MovementServices::vehicle_enter_tile`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_movement_enter(
    s: *const MovementServices,
    v: VehicleRef,
    tile: u32,
    x: i32,
    y: i32,
) -> u8 {
    unsafe { (*s).vehicle_enter_tile(v, TileIndex(tile), x, y) }.0
}
/// # Safety
/// Live serialized train and immutable native tables throughout this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_movement_depot_ticks(
    s: *const MovementServices,
    v: crate::train::Handle,
) -> i32 {
    unsafe { &*s }.ticks_to_leave_depot(v)
}

/// Resolve the native immutable movement registry. Production callers hold the
/// game-state mutex; no map or owner borrow is formed by resolving this table.
#[inline]
pub(crate) fn table(shared: &Services) -> &MovementServices {
    // SAFETY: The shared table holds the immutable process-lifetime movement table.
    unsafe { &*shared.movement }
}
/// `GetTileTrackStatus`, through the registry.
// Forward literal transport/side arguments through this registry facade too.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_tile_track_status(
    shared: &Services,
    tile: TileIndex,
    mode: TransportType,
    sub_mode: u32,
    side: DiagDirection,
) -> TrackStatus {
    table(shared).get_tile_track_status(tile, mode, sub_mode, side)
}
/// Temporary raw adapter of [`get_tile_track_status`] for the files no pass has
/// finished yet.
#[inline]
pub(crate) fn track_status(shared: &Services, tile: u32, mode: u8, sub: u32, side: u8) -> u32 {
    get_tile_track_status(
        shared,
        TileIndex(tile),
        TransportType(mode),
        sub,
        DiagDirection(side),
    )
}
/// `GetSlopePixelZ`, through the registry.
pub(crate) fn get_slope_pixel_z(shared: &Services, x: i32, y: i32, ground_vehicle: bool) -> i32 {
    table(shared).get_slope_pixel_z(x, y, ground_vehicle)
}
/// `GetFoundationSlope`, through the registry.
pub(crate) fn get_foundation_slope(shared: &Services, tile: TileIndex) -> (Slope, i32) {
    table(shared).get_foundation_slope(tile)
}
/// Temporary raw adapter of [`get_foundation_slope`] for `water.rs`, `ground.rs`
/// and `road_cleanup.rs` until their passes.
pub(crate) fn foundation_slope(shared: &Services, tile: u32) -> FoundationSlope {
    let (slope, z) = get_foundation_slope(shared, TileIndex(tile));
    FoundationSlope { z, slope }
}
/// `VehicleEnterTile`, through the registry.
///
/// # Safety
/// The vehicle/map stay live through the synchronous entry and its source callbacks.
pub(crate) unsafe fn vehicle_enter_tile(
    shared: &Services,
    v: VehicleRef,
    tile: TileIndex,
    x: i32,
    y: i32,
) -> VehicleEnterTileStates {
    unsafe { table(shared).vehicle_enter_tile(v, tile, x, y) }
}
/// Temporary raw adapter of [`vehicle_enter_tile`] for `ship_control.rs` until
/// its pass.
///
/// # Safety
/// As [`vehicle_enter_tile`].
pub(crate) unsafe fn enter(shared: &Services, v: VehicleRef, tile: u32, x: i32, y: i32) -> u8 {
    unsafe { vehicle_enter_tile(shared, v, TileIndex(tile), x, y) }.0
}

/// Temporary raw adapter for `ship_control.rs`, `disaster.rs` and
/// `water_regions.rs` until their passes:
/// `TrackStatusToTrackBits(GetTileTrackStatus(tile, TRANSPORT_WATER, 0, side))`.
pub(crate) fn water_tracks(shared: &Services, tile: u32, side: u8) -> u32 {
    table(shared).water_tracks(tile, side)
}
impl MovementServices {
    /// Temporary raw adapter: see [`water_tracks`].
    pub(crate) fn water_tracks(&self, tile: u32, side: u8) -> u32 {
        u32::from(crate::track_func::track_status_to_track_bits(
            self.get_tile_track_status(
                TileIndex(tile),
                TransportType::TRANSPORT_WATER,
                0,
                DiagDirection(side),
            ),
        ))
    }
}

/// `TicksToLeaveDepot`, through the registry.
pub(crate) fn ticks_to_leave_depot(shared: &Services, v: crate::train::Handle) -> i32 {
    table(shared).ticks_to_leave_depot(v)
}

/// Native afterload uses the same portal table as movement.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_movement_tunnel_visibility(direction: u8) -> u8 {
    TUNNEL_VISIBILITY[usize::from(direction)]
}

/// `v->current_order.ShouldStopAtStation(v, station)`. Acquires only the order
/// owner's live alias and the source last-station scalar; no owner reference spans
/// either native leaf.
pub(crate) fn should_stop_at_station(shared: &Services, v: VehicleRef, station: StationID) -> bool {
    let s = table(shared);
    let last_station_visited = StationID(v.last_station());
    // SAFETY: Both native leaves resolve the same serialized surviving shell.
    // The Order is owned by Rust and its facade aliases this stable allocation.
    unsafe {
        let owner = ((*s.orders).vehicle)(v.shell().cast());
        crate::orders::should_stop_at_station(
            std::ptr::addr_of!((*owner).current).cast(),
            last_station_visited,
            station,
        )
    }
}
/// Temporary raw adapter of [`should_stop_at_station`] for `train.rs`,
/// `train_reservation.rs` and `vehicle_effect.rs` until their passes.
pub(crate) fn should_stop(shared: &Services, v: VehicleRef, station: u16) -> bool {
    should_stop_at_station(shared, v, StationID(station))
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
    "OpenTTDMovementServices" ["landscape"] => MovementServices {
        shared,
        orders,
        train,
        vehicle,
        tunnel_bridge,
        road_compatible,
        road_type_speed,
        road_kind,
        resolve,
        depot_invalidate,
        native_assert,
        objects,
        town,
        industry_tiles,
    };
}

#[cfg(test)]
#[path = "movement/tests.rs"]
mod evidence_tests;
