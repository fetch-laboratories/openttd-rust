/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Flooding ownership, including live vehicle-hash traversal and nested command
//! results. Native leaves are noexcept. No Rust borrow denotes a native pool/map
//! object across calls; shared native traversal advances after each callback.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::collapsible_match
)]
use crate::services::TileTypeAccess;
use crate::slope_func::{
    is_inclined_slope, is_slope_with_one_corner_raised, is_slope_with_three_corners_raised,
    is_steep_slope,
};

/// `RIVER_OFFSET_DESERT_DISTANCE`: diameter of the non-desert search around a river tile.
pub const RIVER_OFFSET_DESERT_DISTANCE: u32 = 5;
use crate::{
    ground::{Call, ClearServices, CostServices, RailServices},
    services::{
        self, ClearGroundType, FloodingBehaviour, Foundation, FoundationSlope, RailGroundType,
        Services, Slope, Track, TrackBits, TreeGround,
    },
    vehicle::{VehicleRef, VehicleServices, VehicleType},
};

#[repr(C)]
pub struct AirportGround {
    pub station: *const crate::station::Station,
    pub tile: u32,
    pub width: u16,
    pub height: u16,
}
#[repr(C)]
pub struct FloodVictim {
    pub status: u8,
    pub kind: VehicleType,
}
#[repr(C)]
pub struct AircraftFloodPose {
    pub tile: u32,
    pub z: i32,
    pub subtype: u8,
}
#[repr(C)]
pub struct GroundFloodPose {
    pub z: i32,
    pub first: *mut crate::vehicle::OpenTTDVehicle,
}
#[repr(C)]
pub struct WaterServices {
    pub rail: *const RailServices,
    pub clear: *const ClearServices,
    pub vehicles: *const VehicleServices,
    pub costs: *const CostServices,
    pub company: *const crate::company::Services,
    pub signal_context: extern "C" fn() -> crate::signal::Context,
    pub non_flooding: extern "C" fn(u32) -> bool,
    pub set_non_flooding: extern "C" fn(u32, bool),
    pub coast: extern "C" fn(u32) -> bool,
    pub set_tree_ground: extern "C" fn(u32, TreeGround, u32),
    pub make_sea: extern "C" fn(u32),
    pub buoy_or_dock: extern "C" fn(u32) -> bool,
    pub canal_or_river: extern "C" fn(u32) -> bool,
    pub region_context: extern "C" fn() -> crate::water_regions::Context,
    pub remove_rail: unsafe extern "C" fn(*mut services::CommandCost, u32, Track),
    pub apply_foundation: extern "C" fn(Foundation, Slope) -> FoundationSlope,
    pub airport_tile: extern "C" fn(u32) -> bool,
    pub airport: extern "C" fn(u32) -> AirportGround,
    pub airport_member: unsafe extern "C" fn(*const crate::station::Station, u32) -> bool,
    pub airport_z: extern "C" fn(u32) -> i32,
    pub victim: extern "C" fn(VehicleRef) -> FloodVictim,
    pub aircraft_pose: extern "C" fn(VehicleRef) -> AircraftFloodPose,
    pub ground_pose: extern "C" fn(VehicleRef) -> GroundFloodPose,
    pub crash: extern "C" fn(VehicleRef) -> u32,
    pub ai_event: extern "C" fn(VehicleRef, u32),
    pub game_event: extern "C" fn(VehicleRef, u32),
    pub news: extern "C" fn(VehicleRef, u32),
}
impl WaterServices {
    /// # Safety
    /// The context resolves the current live map-lifetime region owner and leaves.
    pub(crate) unsafe fn invalidate_region(&self, tile: u32, s: &Services) {
        if !(s.tile_valid)(tile) {
            return;
        }
        let context = (self.region_context)();
        unsafe {
            crate::water_regions::openttd_rust_water_invalidate(
                context.owner,
                context.leaves,
                tile,
            );
        }
    }
}
/// Native packed-map/storage leaves for water class restoration.
#[repr(C)]
pub struct RestorationServices {
    pub infrastructure: extern "C" fn(u8, u32),
    pub infrastructure_dirty: extern "C" fn(u8),
    pub make_canal: extern "C" fn(u32, u8, u8),
    pub make_river: extern "C" fn(u32, u8),
}
/// # Safety
/// All tables refer to the current live serialized map; callbacks retain no owner borrow.
pub(crate) unsafe fn restore(
    tile: u32,
    owner: u8,
    s: &Services,
    water: &WaterServices,
    r: &RestorationServices,
) {
    let mut class = unsafe { (&*water.clear).water_class }(tile).0;
    let slope_z = (s.tile_slope_z)(tile);
    if slope_z.slope.0 != 0 {
        if class == 1 {
            if (s.company_valid)(owner) {
                (r.infrastructure)(owner, u32::MAX);
                (r.infrastructure_dirty)(owner);
            }
            class = 3;
        }
        if class != 2 || !is_inclined_slope(slope_z.slope.0) {
            class = 3;
        }
    }
    if class == 0 && slope_z.z > 0 {
        if (s.company_valid)(owner) {
            (r.infrastructure)(owner, 1);
            (r.infrastructure_dirty)(owner);
        }
        class = 1;
    }
    crate::ground::clear_square(tile, s);
    match class {
        0 => (water.make_sea)(tile),
        1 => (r.make_canal)(tile, owner, s.random() as u8),
        2 => (r.make_river)(tile, s.random() as u8),
        _ => (),
    }
    if class != 3 {
        unsafe {
            crate::station_cmd::openttd_rust_station_check_docking((s.station_commands)(), tile);
        }
    }
    (s.mark_dirty)(tile);
}
/// # Safety
/// Tables and map are live throughout this serialized native call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_water_restore(
    tile: u32,
    owner: u8,
    s: *const Services,
    water: *const WaterServices,
    r: *const RestorationServices,
) {
    unsafe {
        restore(tile, owner, &*s, &*water, &*r);
    }
    crate::witness::hit(crate::witness::WATER_CMD_RESTORE);
}
const OFFSETS: [(i32, i32); 8] = [
    (-1, -1),
    (-1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
    (1, 0),
    (1, -1),
    (0, -1),
];
const FLOOD_FROM: [u8; 15] = [
    0b1010_1010,
    0b0000_1010,
    0b1000_0010,
    0b0000_0010,
    0b1010_0000,
    0,
    0b1000_0000,
    0b1000_0011,
    0b0010_1000,
    0b0000_1000,
    0,
    0b0000_1110,
    0b0010_0000,
    0b0011_1000,
    0b1110_0000,
];
fn offset(tile: u32, direction: usize, size_x: u32) -> u32 {
    let (dx, dy) = OFFSETS[direction];
    tile.wrapping_add_signed(dx)
        .wrapping_add(size_x.wrapping_mul(dy as u32))
}
fn wrap(tile: u32, direction: usize, size_x: u32, size_y: u32) -> u32 {
    let (dx, dy) = OFFSETS[direction];
    // Native map dimensions are powers of two (TileX/TileY use mask/shift).
    let x = (tile & (size_x - 1)).wrapping_add_signed(dx);
    let y = (tile >> size_x.trailing_zeros()).wrapping_add_signed(dy);
    if x >= size_x || y >= size_y {
        u32::MAX
    } else {
        y * size_x + x
    }
}
struct World<'a> {
    w: &'a WaterServices,
    s: &'a Services,
    types: &'a TileTypeAccess,
    size_x: u32,
    size_y: u32,
}
impl<'a> World<'a> {
    unsafe fn new(
        w: *const WaterServices,
        s: *const Services,
        types: &'a TileTypeAccess,
        size_x: u32,
        size_y: u32,
    ) -> Self {
        // SAFETY: Entry contract supplies immutable live nested tables.
        unsafe {
            Self {
                w: &*w,
                s: &*s,
                types,
                size_x,
                size_y,
            }
        }
    }
    fn r(&self) -> &RailServices {
        // SAFETY: Entry supplies this immutable nested table when its branch consumes it.
        unsafe { &*self.w.rail }
    }
    fn c(&self) -> &ClearServices {
        // SAFETY: Entry supplies this immutable nested table when its branch consumes it.
        unsafe { &*self.w.clear }
    }
    fn v(&self) -> &VehicleServices {
        // SAFETY: Entry supplies this immutable nested table when its branch consumes it.
        unsafe { &*self.w.vehicles }
    }
    fn costs(&self) -> &CostServices {
        // SAFETY: Entry supplies this immutable nested table when its branch consumes it.
        unsafe { &*self.w.costs }
    }
    fn co(&self) -> &crate::company::Services {
        // SAFETY: Entry supplies this immutable nested table when its branch consumes it.
        unsafe { &*self.w.company }
    }
    fn tile_type(&self, tile: u32) -> u8 {
        self.types.read(tile)
    }
    /// Original `IsValidTile`: `tile < Map::Size() && !IsTileType(tile, MP_VOID)`.
    /// Legal map dimensions are powers of two whose product fits in u32.
    fn valid(&self, tile: u32) -> bool {
        tile < self.size_x * self.size_y && self.tile_type(tile) != 7
    }
    fn behaviour(&self, tile: u32) -> u8 {
        match self.tile_type(tile) {
            6 => {
                if (self.w.coast)(tile) {
                    return if is_slope_with_one_corner_raised((self.s.tile_slope)(tile)) {
                        1
                    } else {
                        3
                    };
                }
                u8::from((self.c().water_class)(tile).0 == 0)
            }
            5 | 8 | 10 => u8::from((self.c().water_class)(tile).0 == 0),
            1 => {
                if (self.r().ground)(tile).0 == 13 {
                    if is_slope_with_one_corner_raised((self.s.tile_slope)(tile)) {
                        1
                    } else {
                        3
                    }
                } else {
                    0
                }
            }
            4 => {
                if (self.s.tree_ground)(tile).0 == 3 {
                    3
                } else {
                    0
                }
            }
            7 => 1,
            _ => 0,
        }
    }
    fn result(&self, call: *mut Call, out: *const services::CommandCost) -> bool {
        // SAFETY: prepare constructed this result; release ends its native lifetime.
        let success = unsafe { ((*self.costs().costs).success)(out) };
        // SAFETY: The live result is no longer read, ending its native lifetime.
        unsafe { (self.costs().release)(call) };
        success
    }
    fn clear(&self, tile: u32, call: *mut Call) -> bool {
        // SAFETY: Each native entry owns this live independent stack context.
        let out = unsafe { (self.costs().prepare)(call) };
        // SAFETY: Immutable shared table; native storage outlives command/reentry.
        unsafe {
            crate::landscape_cmd::clear_command(&*self.costs().commands, out, tile, 1);
        }
        self.result(call, out)
    }
    fn half_flood(&self, tile: u32, call: *mut Call) -> bool {
        let mut flooded = false;
        if (self.r().ground)(tile).0 == 13 {
            return false;
        }
        let slope = (self.s.tile_slope)(tile);
        let mut tracks = (self.r().tracks)(tile).0;
        if is_slope_with_one_corner_raised(slope) {
            let corner = (slope.trailing_zeros() ^ 2) as usize;
            let lower = [16, 8, 32, 4][corner];
            let remove = lower & tracks;
            if remove != 0 {
                let company = (self.s.current_company)();
                (self.co().finance.set_current_company)(17);
                // SAFETY: Each native entry owns this live independent stack context.
                let out = unsafe { (self.costs().prepare)(call) };
                // SAFETY: prepare constructed out, and no owner borrow spans the command.
                unsafe { (self.w.remove_rail)(out, tile, Track(remove.trailing_zeros() as u8)) };
                flooded = self.result(call, out);
                (self.co().finance.set_current_company)(company);
                if !flooded {
                    return false;
                }
                crate::ground::hit(crate::ground::Witness::HalfRailRemove);
                tracks &= !remove;
                if tracks == 0 {
                    (self.s.make_shore)(tile);
                    (self.s.mark_dirty)(tile);
                    return true;
                }
            }
            if (5..10).contains(&(self.r().foundation)(Slope(slope), TrackBits(tracks)).0) {
                flooded = true;
                (self.r().set_ground)(tile, RailGroundType(13));
                (self.s.mark_dirty)(tile);
                crate::ground::hit(crate::ground::Witness::HalfRail);
            }
        } else {
            let foundation = (self.w.apply_foundation)(
                (self.r().foundation)(Slope(slope), TrackBits(tracks)),
                Slope(slope),
            );
            if foundation.z == 0
                && (is_steep_slope(foundation.slope.0)
                    || is_slope_with_three_corners_raised(foundation.slope.0))
            {
                flooded = true;
                (self.r().set_ground)(tile, RailGroundType(13));
                (self.s.mark_dirty)(tile);
                crate::ground::hit(crate::ground::Witness::HalfRail);
            }
        }
        flooded
    }
    fn crash(&self, v: VehicleRef) {
        let victims = (self.w.crash)(v);
        (self.w.ai_event)(v, victims);
        (self.w.game_event)(v, victims);
        (self.w.news)(v, victims);
        (self.v().large_explosion)(v);
        if (self.s.disaster_sound)() {
            (self.v().play_sound)(v, 16);
        }
        crate::ground::hit(crate::ground::Witness::VehicleCrash);
    }
    fn victim(&self, v: VehicleRef, z: i32) {
        // Pose reads are separated only by pure map tests in the source. No
        // mutating callback intervenes, and no pose is reused after Crash.
        let inputs = (self.w.victim)(v);
        if inputs.status & 128 != 0 {
            crate::ground::hit(crate::ground::Witness::CrashedSkip);
            return;
        }
        match inputs.kind.0 {
            3 => {
                let pose = (self.w.aircraft_pose)(v);
                let tile = pose.tile;
                if !(self.w.airport_tile)(tile) || (self.s.tile_max_z)(tile) != 0 {
                    return;
                }
                if pose.subtype == 4 {
                    return;
                }
                if pose.z != (self.w.airport_z)(tile) {
                    return;
                }
                self.crash(v);
            }
            0 | 1 => {
                let pose = (self.w.ground_pose)(v);
                if pose.z <= z {
                    // SAFETY: Source dereferences First only after this z test.
                    // Vehicle construction/afterload establishes a nonnull head
                    // before hash insertion; the raw record adds no precondition
                    // to parts rejected by altitude. No callback intervenes.
                    self.crash(VehicleRef(unsafe {
                        std::ptr::NonNull::new_unchecked(pose.first)
                    }));
                }
            }
            _ => {}
        }
    }
    fn vehicles_on_tile(&self, tile: u32, z: i32) {
        // Original traversal reads hash_tile_next only after Crash, event queues,
        // news and explosion creation return. Crash preserves the current shell.
        self.v().for_each_on_tile(tile, |v| self.victim(v, z));
    }
    fn flood_vehicles(&self, tile: u32) {
        if (self.w.airport_tile)(tile) {
            let airport = (self.w.airport)(tile);
            for y in 0..u32::from(airport.height) {
                for x in 0..u32::from(airport.width) {
                    let target = airport.tile.wrapping_add(y * self.size_x).wrapping_add(x);
                    // SAFETY: Flood/crash callbacks cannot delete this live station or reset pools.
                    if unsafe { (self.w.airport_member)(airport.station, target) } {
                        crate::ground::hit(crate::ground::Witness::AirportMember);
                        self.vehicles_on_tile(target, 0);
                    }
                }
            }
            return;
        }
        if !(self.s.is_bridge)(tile) {
            self.vehicles_on_tile(tile, 0);
            return;
        }
        let end = (self.s.other_tunnel_end)(tile);
        let z = crate::bridge_map::get_bridge_pixel_height(tile, self.s, |f, h| {
            (self.w.apply_foundation)(Foundation(f), Slope(h)).z
        });
        crate::ground::hit(crate::ground::Witness::BridgeStart);
        self.vehicles_on_tile(tile, z);
        crate::ground::hit(crate::ground::Witness::BridgeEnd);
        self.vehicles_on_tile(end, z);
    }
    fn dirty_neighbours(&self, tile: u32) {
        for direction in 0..8 {
            let target = offset(tile, direction, self.size_x);
            if self.valid(target) && self.tile_type(target) == 6 && (self.w.canal_or_river)(target)
            {
                (self.s.mark_dirty)(target);
            }
        }
    }
    fn flood(&self, tile: u32, call: *mut Call) {
        let company = (self.s.current_company)();
        (self.co().finance.set_current_company)(17);
        let slope = (self.s.tile_slope)(tile);
        let flooded = if slope != 0 {
            match self.tile_type(tile) {
                1 => {
                    if (self.s.is_plain_rail)(tile) {
                        self.flood_vehicles(tile);
                        self.half_flood(tile, call)
                    } else {
                        false
                    }
                }
                4 if !is_slope_with_one_corner_raised(slope) => {
                    (self.w.set_tree_ground)(tile, TreeGround(3), 3);
                    (self.s.mark_dirty)(tile);
                    true
                }
                0 | 4 => {
                    if self.clear(tile, call) {
                        (self.s.make_shore)(tile);
                        (self.s.mark_dirty)(tile);
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            }
        } else {
            self.flood_vehicles(tile);
            if self.clear(tile, call) {
                (self.w.make_sea)(tile);
                (self.s.mark_dirty)(tile);
                true
            } else {
                false
            }
        };
        if flooded {
            self.dirty_neighbours(tile);
            crate::signal::acquire_context(self.w.signal_context).flush();
            let stations = (self.s.station_commands)();
            // SAFETY: the getter supplies the borrowed process-lifetime table;
            // docking runs synchronously without a live owner borrow.
            unsafe { crate::station_cmd::check_possible_docking(stations, tile) };
            unsafe {
                self.w.invalidate_region(tile, self.s);
            }
            crate::ground::hit(crate::ground::Witness::FloodSuccess);
        } else {
            crate::ground::hit(crate::ground::Witness::FloodFail);
        }
        (self.co().finance.set_current_company)(company);
    }
    fn dry(&self, tile: u32, call: *mut Call) {
        let company = (self.s.current_company)();
        (self.co().finance.set_current_company)(17);
        match self.tile_type(tile) {
            1 => {
                let ground = match (self.r().tracks)(tile).0 {
                    4 => 10,
                    8 => 11,
                    16 => 8,
                    32 => 9,
                    _ => unreachable!("original drying rail track"),
                };
                (self.r().set_ground)(tile, RailGroundType(ground));
                (self.s.mark_dirty)(tile);
                crate::ground::hit(crate::ground::Witness::DryRail);
            }
            4 => {
                (self.w.set_tree_ground)(tile, TreeGround(0), 3);
                (self.s.mark_dirty)(tile);
                crate::ground::hit(crate::ground::Witness::DryTree);
            }
            6 => {
                if self.clear(tile, call) {
                    (self.c().make_clear)(tile, ClearGroundType(0), 3);
                    (self.s.mark_dirty)(tile);
                    crate::ground::hit(crate::ground::Witness::DryCoast);
                }
            }
            _ => unreachable!("original drying tile type"),
        }
        (self.co().finance.set_current_company)(company);
    }
    fn tile_loop(&self, tile: u32, call: *mut Call) {
        match self.behaviour(tile) {
            1 => {
                let mut continue_flooding = false;
                for direction in 0..8 {
                    let target = wrap(tile, direction, self.size_x, self.size_y);
                    if !self.valid(target) {
                        continue;
                    }
                    let kind = self.tile_type(target);
                    if kind == 6 {
                        continue;
                    }
                    if kind == 5 && (self.w.buoy_or_dock)(target) {
                        continue;
                    }
                    continue_flooding = true;
                    if kind == 4 && (self.s.tree_ground)(target).0 == 3 {
                        continue;
                    }
                    let foundation = (self.s.foundation)(target);
                    if foundation.z > 0 {
                        continue;
                    }
                    let mask = FLOOD_FROM[usize::from(foundation.slope.0 & 15)];
                    if mask & (1 << ((direction + 4) & 7)) == 0 {
                        continue;
                    }
                    self.flood(target, call);
                }
                if !continue_flooding && self.tile_type(tile) == 6 {
                    (self.w.set_non_flooding)(tile, true);
                }
            }
            3 => {
                let mask = FLOOD_FROM[usize::from((self.s.foundation)(tile).slope.0 & 15)];
                for direction in 0..8 {
                    if mask & (1 << direction) == 0 {
                        continue;
                    }
                    let target = wrap(tile, direction, self.size_x, self.size_y);
                    if target == u32::MAX {
                        continue;
                    }
                    if matches!(self.behaviour(target), 1 | 2) {
                        return;
                    }
                }
                self.dry(tile, call);
            }
            _ => {}
        }
    }
}
/// # Safety
/// Live immutable service tables and an independent native lazy-result context.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_water_tile_loop(
    tile: u32,
    size_x: u32,
    size_y: u32,
    ambient_callback: bool,
    call: *mut Call,
    w: *const WaterServices,
    s: *const Services,
) {
    // SAFETY: Same synchronous table contract as the in-crate scheduler entry;
    // this native entry keeps the original callback reader.
    unsafe {
        let types = TileTypeAccess::callback((*s).tile_type);
        let tile_type = types.read(tile);
        water_tile_loop(
            tile,
            tile_type,
            size_x,
            size_y,
            ambient_callback,
            call,
            w,
            s,
            &types,
        );
    };
}
/// Original `TileLoop_Water` after its caller has read `GetTileType(tile)`.
#[allow(clippy::inline_always, clippy::too_many_arguments)]
#[inline(always)]
pub(crate) unsafe fn water_tile_loop(
    tile: u32,
    tile_type: u8,
    size_x: u32,
    size_y: u32,
    ambient_callback: bool,
    call: *mut Call,
    w: *const WaterServices,
    s: *const Services,
    types: &TileTypeAccess,
) {
    if tile_type == 6 {
        // SAFETY: Live immutable tables; no owner access spans ambient sound.
        let water = unsafe { &*w };
        if ambient_callback {
            // SAFETY: The water table's clear table is live for this entry.
            (unsafe { &*water.clear }.ambient)(tile);
        }
        if (water.non_flooding)(tile) {
            return;
        }
    }
    // SAFETY: Resolve only the tables reached after the original early return.
    unsafe { World::new(w, s, types, size_x, size_y) }.tile_loop(tile, call);
}
/// # Safety
/// Same table/context contract; original native plain-rail assertion is retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_flood_halftile(
    tile: u32,
    call: *mut Call,
    w: *const WaterServices,
    s: *const Services,
) -> bool {
    // SAFETY: Serialized native entry; no borrowed state crosses commands. Half
    // flooding reads neither map size nor tile type.
    let types = TileTypeAccess::callback(unsafe { (*s).tile_type });
    unsafe { World::new(w, s, &types, 0, 0) }.half_flood(tile, call)
}
/// # Safety
/// Live immutable service tables; valid original tile argument.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_flooding_behaviour(
    tile: u32,
    w: *const WaterServices,
    s: *const Services,
) -> FloodingBehaviour {
    // SAFETY: Serialized native entry, observations are copied. Behaviour reads
    // no map size.
    let types = TileTypeAccess::callback(unsafe { (*s).tile_type });
    FloodingBehaviour(unsafe { World::new(w, s, &types, 0, 0) }.behaviour(tile))
}
/// # Safety
/// Live immutable service tables for the synchronous no-unwind invocation.
pub(crate) unsafe fn dirty_water_neighbours(
    tile: u32,
    size_x: u32,
    size_y: u32,
    w: &WaterServices,
    s: &Services,
) {
    // SAFETY: Serialized native entry; no map references are formed.
    let types = TileTypeAccess::callback(s.tile_type);
    unsafe { World::new(w, s, &types, size_x, size_y) }.dirty_neighbours(tile);
}
/// # Safety
/// Live immutable service tables for the synchronous no-unwind invocation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_clear_neighbour_flooding(
    tile: u32,
    size_x: u32,
    size_y: u32,
    w: *const WaterServices,
    s: *const Services,
) {
    // SAFETY: Serialized native entry; no map references are formed.
    let types = TileTypeAccess::callback(unsafe { (*s).tile_type });
    let world = unsafe { World::new(w, s, &types, size_x, size_y) };
    for direction in 0..8 {
        let target = offset(tile, direction, size_x);
        if world.valid(target) && world.tile_type(target) == 6 {
            (world.w.set_non_flooding)(target, false);
            crate::ground::hit(crate::ground::Witness::Wakeup);
        }
    }
}

/// Direct water call from a tree/industry tile loop. Both callers have just read
/// their own tile type through pure observations, so the water-only ambient and
/// non-flooding prelude is inactive. Nested commands use the native caller's stack.
/// # Safety
/// Live immutable tables and independent lazy-result context, serialized with map reset.
pub(crate) unsafe fn tree_industry_water_loop(tile: u32, call: *mut Call, s: &Services) {
    let size_x = (s.map_max_x)().wrapping_add(1);
    let size_y = (s.map_max_y)().wrapping_add(1);
    // SAFETY: Same serialized table/context contract as the native water entry.
    unsafe {
        openttd_rust_water_tile_loop(tile, size_x, size_y, false, call, (s.water_services)(), s);
    }
}

#[cfg(test)]
pub(crate) fn fixture() -> WaterServices {
    extern "C" fn unused0<R>() -> R {
        panic!("unexpected water fixture service");
    }
    extern "C" fn unused1<A, R>(_: A) -> R {
        panic!("unexpected water fixture service");
    }
    extern "C" fn unused2<A, B, R>(_: A, _: B) -> R {
        panic!("unexpected water fixture service");
    }
    extern "C" fn unused3<A, B, C, R>(_: A, _: B, _: C) -> R {
        panic!("unexpected water fixture service");
    }
    WaterServices {
        rail: std::ptr::null(),
        clear: std::ptr::null(),
        vehicles: std::ptr::null(),
        costs: std::ptr::null(),
        company: std::ptr::null(),
        signal_context: unused0,
        non_flooding: unused1,
        set_non_flooding: unused2,
        coast: unused1,
        set_tree_ground: unused3,
        make_sea: unused1,
        buoy_or_dock: unused1,
        canal_or_river: unused1,
        region_context: unused_region_context,
        remove_rail: unused3,
        apply_foundation: unused2,
        airport_tile: unused1,
        airport: unused1,
        airport_member: unused2,
        airport_z: unused1,
        victim: unused1,
        aircraft_pose: unused1,
        ground_pose: unused1,
        crash: unused1,
        ai_event: unused2,
        game_event: unused2,
        news: unused2,
    }
}

#[cfg(test)]
extern "C" fn unused_region_context() -> crate::water_regions::Context {
    panic!("unused water region context")
}
