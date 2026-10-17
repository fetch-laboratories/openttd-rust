/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Ground evolution for clear, railway and tunnel/bridge tiles. Native storage is
//! accessed through consumed-field records; no map or owner borrow spans calls.
//! Entries are synchronous and serialized by the game-state mutex. Native leaves
//! are noexcept and all Rust panics abort, including impossible original states.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::if_not_else,
    clippy::too_many_arguments
)]
use crate::services::{
    ClearGroundType, DiagDirection, Foundation, FoundationSlope, LandscapeType, RailGroundType,
    RoadBits, RoadType, Roadside, Services, Slope, TrackBits, WaterClass,
};

#[repr(usize)]
pub(crate) enum Witness {
    SnowStart,
    SnowChange,
    SnowClear,
    Desert,
    FieldFence,
    FieldCycle,
    FieldOrphan,
    RailGround,
    Roadside,
    RoadWorkStart,
    RoadWorkEnd,
    RoadRemove,
    TunnelGround,
    FloodSuccess,
    FloodFail,
    DryCoast,
    DryRail,
    DryTree,
    Wakeup,
    HalfRailRemove,
    HalfRail,
    BridgeStart,
    BridgeEnd,
    AirportMember,
    VehicleCrash,
    CrashedSkip,
}
#[inline]
pub(crate) fn hit(branch: Witness) {
    crate::witness::hit(crate::witness::LANDSCAPE + branch as usize);
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FieldFences {
    pub ne: u32,
    pub se: u32,
    pub sw: u32,
    pub nw: u32,
}
#[repr(C)]
pub struct ClearSurface {
    pub ground: ClearGroundType,
    pub density: u8,
    pub snow: bool,
}
#[repr(C)]
pub struct FieldGround {
    pub industry: u16,
    pub field_type: u32,
}
#[repr(C)]
pub struct ClearServices {
    pub field_tile: extern "C" fn(u32) -> bool,
    pub density: extern "C" fn(u32) -> u32,
    pub counter: extern "C" fn(u32) -> u32,
    pub snow: extern "C" fn(u32) -> bool,
    pub surface: extern "C" fn(u32) -> ClearSurface,
    pub set_ground: extern "C" fn(u32, ClearGroundType, u32),
    pub set_density: extern "C" fn(u32, u32),
    pub set_counter: extern "C" fn(u32, u32),
    pub clear_snow: extern "C" fn(u32),
    pub make_clear: extern "C" fn(u32, ClearGroundType, u32),
    pub fences: extern "C" fn(u32) -> FieldFences,
    pub field: extern "C" fn(u32) -> FieldGround,
    pub set_field: extern "C" fn(u32, u32),
    pub water_class: extern "C" fn(u32) -> WaterClass,
    pub ambient: extern "C" fn(u32),
}
#[repr(C)]
pub struct RailSnowGround {
    pub z: i32,
    pub slope: Slope,
    pub tracks: TrackBits,
    pub plain: bool,
}
#[repr(C)]
pub struct RailFenceGround {
    pub tracks: TrackBits,
    pub owner: u8,
    pub plain: bool,
}
#[repr(C)]
pub struct RailServices {
    pub ground: extern "C" fn(u32) -> RailGroundType,
    pub set_ground: extern "C" fn(u32, RailGroundType),
    pub snow: extern "C" fn(u32) -> RailSnowGround,
    pub fences: extern "C" fn(u32) -> RailFenceGround,
    pub foundation: extern "C" fn(Slope, TrackBits) -> Foundation,
    pub owned_land: extern "C" fn(u32) -> bool,
}
#[repr(C)]
pub struct TunnelGround {
    pub z: i32,
    pub snow: bool,
}
#[repr(C)]
pub struct TunnelServices {
    pub snow: extern "C" fn(u32) -> TunnelGround,
    pub snow_or_desert: extern "C" fn(u32) -> bool,
    pub set_snow_or_desert: extern "C" fn(u32, bool),
}
fn diagonal(tile: u32, direction: u8, size_x: u32) -> u32 {
    match direction {
        0 => tile.wrapping_sub(1),
        1 => tile.wrapping_add(size_x),
        2 => tile.wrapping_add(1),
        _ => tile.wrapping_sub(size_x),
    }
}
fn three_corners(slope: u8) -> bool {
    matches!(slope, 7 | 11 | 13 | 14)
}
fn steep(slope: u8) -> bool {
    slope & 16 != 0
}

#[allow(clippy::inline_always)]
#[inline(always)]
fn field_fence(
    tile: u32,
    size_x: u32,
    direction: u8,
    fence: u32,
    c: &ClearServices,
    s: &Services,
) -> bool {
    if fence != 0 || (c.field_tile)(diagonal(tile, direction, size_x)) {
        return false;
    }
    (s.set_fence)(tile, DiagDirection(direction), 3);
    true
}

/// # Safety
/// Tables are live for this serialized synchronous entry; callbacks do not unwind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_clear_tile_loop(
    tile: u32,
    climate: LandscapeType,
    editor: bool,
    ambient_callback: bool,
    size_x: u32,
    clear: *const ClearServices,
    shared: *const Services,
) {
    // SAFETY: Same synchronous table contract as the in-crate scheduler entry.
    unsafe {
        clear_tile_loop(
            tile,
            climate,
            editor,
            ambient_callback,
            size_x,
            clear,
            shared,
        );
    };
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) unsafe fn clear_tile_loop(
    tile: u32,
    climate: LandscapeType,
    editor: bool,
    ambient_callback: bool,
    size_x: u32,
    clear: *const ClearServices,
    shared: *const Services,
) {
    let climate = climate.0;
    // SAFETY: Caller supplies immutable live tables. No state borrow crosses a call.
    let (c, s) = unsafe { (&*clear, &*shared) };
    if ambient_callback {
        (c.ambient)(tile);
    }
    if climate == 1 {
        let k = (s.tile_z)(tile) - i32::from((s.snow_line)()) + 1;
        if !(c.snow)(tile) {
            if k >= 0 {
                (s.make_snow)(tile, 0);
                hit(Witness::SnowStart);
                (s.mark_dirty)(tile);
            }
        } else {
            let density = (c.density)(tile);
            let required = k.clamp(0, 3) as u32;
            if density == required {
                if k < 0 {
                    (c.clear_snow)(tile);
                    hit(Witness::SnowClear);
                    (s.mark_dirty)(tile);
                }
            } else {
                (c.set_density)(
                    tile,
                    if density < required {
                        density + 1
                    } else {
                        density - 1
                    },
                );
                hit(Witness::SnowChange);
                (s.mark_dirty)(tile);
            }
        }
    } else if climate == 2 {
        let current = if (s.clear_ground)(tile).0 == 5 {
            (c.density)(tile)
        } else {
            0
        };
        let mut expected = 0;
        if (s.tropic_zone)(tile) == 1 {
            let normal = (0..4).any(|direction| {
                let t = diagonal(tile, direction, size_x);
                (s.tile_valid)(t)
                    && ((s.tropic_zone)(t) != 1
                        || ((s.has_water_class)(t) && (c.water_class)(t).0 == 0))
            });
            expected = if normal { 1 } else { 3 };
        }
        if current != expected {
            (c.set_ground)(
                tile,
                ClearGroundType(if expected == 0 { 0 } else { 5 }),
                if expected == 0 { 3 } else { expected },
            );
            (s.mark_dirty)(tile);
            hit(Witness::Desert);
        }
    }
    let surface = (c.surface)(tile);
    if surface.snow {
        return;
    }
    match surface.ground.0 {
        0 => {
            let density = u32::from(surface.density);
            if density == 3 {
                return;
            }
            if editor {
                (c.set_ground)(
                    tile,
                    ClearGroundType(u8::from((s.random)(s.context) & 255 <= 21)),
                    3,
                );
            } else {
                let counter = (c.counter)(tile);
                if counter < 7 {
                    (c.set_counter)(tile, counter + 1);
                    return;
                }
                (c.set_counter)(tile, 0);
                (c.set_density)(tile, density + 1);
            }
        }
        3 => {
            // Four ordered steps keep each guarded live neighbour read and write.
            // SetFence's fields are disjoint; dirtying remains after all four.
            let fences = (c.fences)(tile);
            let mut dirty = field_fence(tile, size_x, 0, fences.ne, c, s);
            dirty |= field_fence(tile, size_x, 1, fences.se, c, s);
            dirty |= field_fence(tile, size_x, 2, fences.sw, c, s);
            dirty |= field_fence(tile, size_x, 3, fences.nw, c, s);
            if dirty {
                (s.mark_dirty)(tile);
                hit(Witness::FieldFence);
            }
            if editor {
                return;
            }
            let counter = (c.counter)(tile);
            if counter < 7 {
                (c.set_counter)(tile, counter + 1);
                return;
            }
            (c.set_counter)(tile, 0);
            let field = (c.field)(tile);
            if field.industry == u16::MAX && field.field_type >= 7 {
                (c.make_clear)(tile, ClearGroundType(0), 2);
                hit(Witness::FieldOrphan);
            } else {
                hit(Witness::FieldCycle);
                (c.set_field)(
                    tile,
                    if field.field_type < 8 {
                        field.field_type + 1
                    } else {
                        0
                    },
                );
            }
        }
        _ => return,
    }
    (s.mark_dirty)(tile);
}

/// # Safety
/// Tables are live for this serialized synchronous entry; callbacks do not unwind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_tile_loop(
    tile: u32,
    climate: LandscapeType,
    ambient_callback: bool,
    size_x: u32,
    size_y: u32,
    call: *mut Call,
    water: *const crate::water::WaterServices,
    rail: *const RailServices,
    shared: *const Services,
) {
    // SAFETY: Same synchronous table contract; this native entry keeps the
    // original callback tile-type reader.
    unsafe {
        let types = crate::services::TileTypeAccess::callback((*shared).tile_type);
        rail_tile_loop(
            tile,
            climate,
            ambient_callback,
            size_x,
            size_y,
            call,
            water,
            rail,
            shared,
            &types,
        );
    }
}
/// Original `TileLoop_Track`; `types` is the caller's tile-type reader.
pub(crate) unsafe fn rail_tile_loop(
    tile: u32,
    climate: LandscapeType,
    ambient_callback: bool,
    size_x: u32,
    size_y: u32,
    call: *mut Call,
    water: *const crate::water::WaterServices,
    rail: *const RailServices,
    shared: *const Services,
    types: &crate::services::TileTypeAccess,
) {
    let climate = climate.0;
    // SAFETY: Caller supplies immutable live tables; observations are copied.
    let (r, s) = unsafe { (&*rail, &*shared) };
    let old = (r.ground)(tile).0;
    if old == 13 {
        // SAFETY: The same serialized facade/batch owns this independent native
        // result context; no result is live before this source early return.
        // `TileLoop_Water` observes this railway tile's type.
        unsafe {
            crate::water::water_tile_loop(
                tile,
                1,
                size_x,
                size_y,
                ambient_callback,
                call,
                water,
                shared,
                types,
            );
        };
        return;
    }
    let mut new = None;
    if climate == 1 {
        let mut ground = (r.snow)(tile);
        let mut half = false;
        if ground.plain {
            let foundation = (r.foundation)(ground.slope, ground.tracks).0;
            match foundation {
                0 => {
                    if three_corners(ground.slope.0) {
                        ground.z += 1;
                    }
                }
                2 | 3 => {
                    if steep(ground.slope.0) {
                        ground.z += 1;
                    }
                }
                4 => ground.z += 1,
                _ => {
                    if steep(ground.slope.0) {
                        ground.z += 1;
                    }
                    ground.z += 1;
                }
            }
            half = (5..10).contains(&foundation);
        } else if ground.slope.0 != 0 {
            ground.z += 1;
        }
        let snow = i32::from((s.snow_line)());
        if ground.z > snow {
            new = Some(if half && ground.z - snow == 1 { 14 } else { 12 });
        }
    } else if climate == 2 && (s.tropic_zone)(tile) == 1 {
        new = Some(12);
    }
    let new = new.unwrap_or_else(|| {
        if old == 0 {
            return 1;
        }
        let ground = (r.fences)(tile);
        if !ground.plain {
            return 1;
        }
        let mut fences = 0_u8;
        for (direction, tracks) in [37, 42, 25, 22].into_iter().enumerate() {
            if ground.tracks.0 & tracks != 0 {
                continue;
            }
            let neighbour = diagonal(tile, direction as u8, size_x);
            let fence = if (s.tile_valid)(neighbour) {
                let kind = (s.tile_type)(neighbour);
                matches!(kind, 2 | 3 | 8 | 9)
                    || (kind == 10 && !(r.owned_land)(neighbour))
                    || (s.tile_owner)(neighbour) != ground.owner
            } else {
                true
            };
            if fence {
                fences |= 1 << direction;
            }
        }
        match fences {
            0 => 1,
            1 => 5,
            2 => 3,
            4 => 6,
            8 => 2,
            5 => 7,
            10 => 4,
            3 => 8,
            9 => 11,
            6 => 10,
            12 => 9,
            _ => unreachable!("original rail fence mask"),
        }
    });
    if old != new {
        (r.set_ground)(tile, RailGroundType(new));
        hit(Witness::RailGround);
        (s.mark_dirty)(tile);
    }
}

/// # Safety
/// Tables are live for this serialized synchronous entry; callbacks do not unwind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_tunnel_tile_loop(
    tile: u32,
    climate: LandscapeType,
    tunnel: *const TunnelServices,
    shared: *const Services,
) {
    let climate = climate.0;
    // SAFETY: Caller supplies immutable live tables; no mutable borrow retained.
    let (t, s) = unsafe { (&*tunnel, &*shared) };
    if climate == 1 {
        let ground = (t.snow)(tile);
        let snow = ground.z > i32::from((s.snow_line)());
        if ground.snow != snow {
            (t.set_snow_or_desert)(tile, !ground.snow);
            hit(Witness::TunnelGround);
            (s.mark_dirty)(tile);
        }
    } else if climate == 2 && (s.tropic_zone)(tile) == 1 && !(t.snow_or_desert)(tile) {
        (t.set_snow_or_desert)(tile, true);
        hit(Witness::TunnelGround);
        (s.mark_dirty)(tile);
    }
}

#[repr(C)]
pub struct Call {
    _private: [u8; 0],
}
#[repr(C)]
pub struct CostServices {
    pub commands: *const crate::services::CommandServices,
    pub costs: *const crate::services::CommandCostServices,
    // Caller provides the facade's live context; release follows its last result access.
    pub prepare: unsafe extern "C" fn(*mut Call) -> *mut crate::services::CommandCost,
    pub release: unsafe extern "C" fn(*mut Call),
}
#[repr(C)]
pub struct RoadSnowGround {
    pub z: i32,
    pub snow: bool,
}
#[repr(C)]
pub struct RoadServices {
    pub snow: extern "C" fn(u32) -> RoadSnowGround,
    pub snow_or_desert: extern "C" fn(u32) -> bool,
    pub toggle_snow_or_desert: extern "C" fn(u32),
    pub roadside: extern "C" fn(u32) -> Roadside,
    pub set_roadside: extern "C" fn(u32, Roadside),
    pub work_counter: extern "C" fn(u32) -> u32,
    pub set_work_counter: extern "C" fn(u32, u32),
    pub all_bits: extern "C" fn(u32) -> RoadBits,
    pub bulldozer: extern "C" fn(u32),
    pub cleanup: extern "C" fn(u32, RoadBits) -> RoadBits,
    pub remove: extern "C" fn(u32, RoadBits),
    pub town_road_type: extern "C" fn() -> RoadType,
    pub set_road_type: extern "C" fn(u32, RoadType),
}

/// # Safety
/// All tables and native lazy result context outlive this serialized entry. No
/// Rust borrow denotes native command storage or a town across command callbacks.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_tile_loop(
    tile: u32,
    climate: LandscapeType,
    ambient: bool,
    rebuild: bool,
    size_x: u32,
    call: *mut Call,
    costs: *const CostServices,
    roads: *const RoadServices,
    shared: *const Services,
) {
    let climate = climate.0;
    // SAFETY: Live immutable tables; native storage is accessed only by its leaves.
    let (c, r, s) = unsafe { (&*costs, &*roads, &*shared) };
    if climate == 1 {
        let ground = (r.snow)(tile);
        if ground.snow != (ground.z > i32::from((s.snow_line)())) {
            (r.toggle_snow_or_desert)(tile);
            (s.mark_dirty)(tile);
        }
    } else if climate == 2 && (s.tropic_zone)(tile) == 1 && !(r.snow_or_desert)(tile) {
        (r.toggle_snow_or_desert)(tile);
        (s.mark_dirty)(tile);
    }
    if (s.is_road_depot)(tile) {
        return;
    }
    let town = (s.closest_town)(tile, u32::MAX);
    let side = (r.roadside)(tile).0;
    if side < 6 {
        let mut zone = 0;
        if !town.is_null() {
            let owner = (s.town_owner)(town);
            // SAFETY: The shared town owner belongs to the live closest-town shell.
            // Only copied inputs survive the following native services.
            let (months, group, distance) =
                unsafe { crate::town::road_context(owner, tile, size_x) };
            zone = group;
            if months != 0 && (distance < 8 || zone != 0) && (s.is_normal_road)(tile) {
                let bits = (r.all_bits)(tile);
                if bits.0.count_ones() > 1 && (s.foundation)(tile).slope.0 == 0 {
                    // SAFETY: The facade owns this live native stack context.
                    let out = unsafe { (c.prepare)(call) };
                    // SAFETY: prepare provides live native result storage.
                    unsafe { ((*c.commands).ensure_no_vehicle)(out, tile) };
                    // SAFETY: The result remains a live native object until release.
                    let empty = unsafe { ((*c.costs).success)(out) };
                    // SAFETY: The command/check has returned; no result access follows.
                    unsafe { (c.release)(call) };
                    if empty && crate::services::chance16_i(1, 40, (s.random)(s.context)) {
                        (r.set_roadside)(tile, Roadside(if side <= 1 { 6 } else { 7 }));
                        hit(Witness::RoadWorkStart);
                        if ambient {
                            (s.play_sound)(tile, 31);
                        }
                        (r.bulldozer)(tile);
                        (s.mark_dirty)(tile);
                        return;
                    }
                }
            }
        }
        let desired = match zone {
            0 => [1, 1],
            1 | 2 => {
                if climate == 3 && zone == 2 {
                    [3, 2]
                } else {
                    [2, 2]
                }
            }
            3 => {
                if climate == 3 {
                    [3, 2]
                } else {
                    [5, 5]
                }
            }
            _ => [3, 2],
        };
        if side == desired[0] {
            return;
        }
        (r.set_roadside)(
            tile,
            Roadside(if side == desired[1] {
                desired[0]
            } else if side == 0 {
                desired[1]
            } else {
                0
            }),
        );
        (s.mark_dirty)(tile);
        hit(Witness::Roadside);
    } else {
        let counter = ((r.work_counter)(tile) + 1) & 15;
        (r.set_work_counter)(tile, counter);
        if counter != 15 {
            return;
        }
        (r.set_roadside)(tile, Roadside(if side == 6 { 1 } else { 2 }));
        (r.set_work_counter)(tile, 0);
        hit(Witness::RoadWorkEnd);
        if rebuild {
            let old = RoadBits((s.road_bits)(tile, 0, false));
            let new = (r.cleanup)(tile, old);
            if old != new {
                (r.remove)(tile, RoadBits(old.0 ^ new.0));
                hit(Witness::RoadRemove);
                if new.0 == 0 {
                    (s.mark_dirty)(tile);
                    return;
                }
            }
        }
        if (s.road_owner)(tile) == 15 {
            let road_type = (r.town_road_type)();
            if road_type.0 != (s.road_type)(tile) {
                (r.set_road_type)(tile, road_type);
            }
        }
        (s.mark_dirty)(tile);
    }
}

/// Actual record-return and enum-argument calls for every default native ABI.
#[repr(C)]
pub struct ABICalls {
    pub fences: extern "C" fn() -> FieldFences,
    pub surface: extern "C" fn() -> ClearSurface,
    pub field: extern "C" fn() -> FieldGround,
    pub rail_snow: extern "C" fn() -> RailSnowGround,
    pub rail_fence: extern "C" fn() -> RailFenceGround,
    pub foundation: extern "C" fn() -> FoundationSlope,
    pub road_snow: extern "C" fn() -> RoadSnowGround,
    pub tunnel: extern "C" fn() -> TunnelGround,
    pub airport: extern "C" fn() -> crate::water::AirportGround,
    pub victim: extern "C" fn() -> crate::water::FloodVictim,
    pub aircraft: extern "C" fn() -> crate::water::AircraftFloodPose,
    pub ground: extern "C" fn() -> crate::water::GroundFloodPose,
    pub enums: extern "C" fn(
        ClearGroundType,
        RailGroundType,
        Slope,
        TrackBits,
        crate::services::Track,
        Foundation,
        Roadside,
        RoadBits,
        RoadType,
        crate::services::TreeGround,
        WaterClass,
        crate::services::FloodingBehaviour,
        DiagDirection,
        LandscapeType,
        crate::vehicle::VehicleType,
    ) -> bool,
}
/// # Safety
/// A complete immutable fixture table. Returned opaque handles are marker values
/// for this ABI-only call; they are compared, never dereferenced or used by game services.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ground_abi_calls(calls: *const ABICalls) -> bool {
    // SAFETY: Caller provides actual typed native functions for this fixture.
    let c = unsafe { &*calls };
    let fences = (c.fences)();
    let surface = (c.surface)();
    let field = (c.field)();
    let snow = (c.rail_snow)();
    let fence = (c.rail_fence)();
    let foundation = (c.foundation)();
    let road = (c.road_snow)();
    let tunnel = (c.tunnel)();
    let airport = (c.airport)();
    let victim = (c.victim)();
    let aircraft = (c.aircraft)();
    let ground = (c.ground)();
    fences.ne == 0xfedc_ba98
        && fences.se == 0x89ab_cdef
        && fences.sw == 0x7654_3210
        && fences.nw == 0x1234_5678
        && surface.density == 0xcd
        && surface.ground.0 == 240
        && surface.snow
        && field.industry == 0xfedc
        && field.field_type == 0x9876_5432
        && snow.z == -42
        && snow.slope.0 == 238
        && snow.tracks.0 == 237
        && snow.plain
        && fence.tracks.0 == 236
        && fence.owner == 235
        && !fence.plain
        && foundation.z == 123_456
        && foundation.slope.0 == 234
        && road.z == -123_456
        && road.snow
        && tunnel.z == 789
        && !tunnel.snow
        && airport.station.is_null()
        && airport.tile == 0x1234_5678
        && airport.width == 0xabcd
        && airport.height == 0xdcba
        && victim.status == 233
        && victim.kind.0 == 226
        && aircraft.tile == 0x8765_4321
        && aircraft.z == -789
        && aircraft.subtype == 232
        && ground.z == 777
        && ground.first.addr() != 0
        && (c.enums)(
            ClearGroundType(240),
            RailGroundType(239),
            snow.slope,
            snow.tracks,
            crate::services::Track(227),
            Foundation(228),
            Roadside(229),
            RoadBits(230),
            RoadType(231),
            crate::services::TreeGround(232),
            WaterClass(233),
            crate::services::FloodingBehaviour(234),
            DiagDirection(235),
            LandscapeType(236),
            victim.kind,
        )
}
