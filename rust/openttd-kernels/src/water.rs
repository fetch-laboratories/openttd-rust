/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Flooding, including live vehicle-hash traversal and nested command results. Native
//! traversal advances after each callback.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::collapsible_match
)]
use crate::company_base::CompanyInfrastructure;
use crate::company_type::CompanyID;
use crate::services::TileTypeAccess;
use crate::slope_func::{
    is_inclined_slope_raw, is_slope_with_one_corner_raised_raw,
    is_slope_with_three_corners_raised_raw, is_steep_slope_raw,
};
use crate::tile_type::TileIndex;
use crate::tile_type::TileType;

/// `RIVER_OFFSET_DESERT_DISTANCE`: diameter of the non-desert search around a river tile.
pub const RIVER_OFFSET_DESERT_DISTANCE: u32 = 5;
use crate::{
    ground::{Call, ClearServices, CostServices, RailServices},
    services::{
        self, ClearGroundType, Foundation, RailGroundType, Services, Slope, Track, TreeGround,
    },
    vehicle::{VehicleRef, VehicleServices},
};

#[repr(C)]
pub struct AirportGround {
    pub station: *const crate::station::Station,
    pub tile: u32,
    pub width: u16,
    pub height: u16,
}
#[repr(C)]
pub struct WaterServices {
    pub rail: *const RailServices,
    pub clear: *const ClearServices,
    pub vehicles: *const VehicleServices,
    pub costs: *const CostServices,
    pub company: *const crate::company::Services,
    pub signal_context: extern "C" fn() -> crate::signal::Context,
    pub set_non_flooding: extern "C" fn(u32, bool),
    pub set_tree_ground: extern "C" fn(u32, TreeGround, u32),
    pub make_sea: extern "C" fn(u32),
    pub buoy_or_dock: extern "C" fn(u32) -> bool,
    pub water_regions: extern "C" fn() -> *mut crate::water_regions::Regions,
    pub remove_rail: unsafe extern "C" fn(*mut services::CommandCost, u32, Track),
    pub airport_tile: extern "C" fn(u32) -> bool,
    pub airport: extern "C" fn(u32) -> AirportGround,
    pub airport_member: unsafe extern "C" fn(*const crate::station::Station, u32) -> bool,
    pub airport_z: extern "C" fn(*const std::ffi::c_void) -> i32,
    pub crash: extern "C" fn(VehicleRef) -> u32,
    pub ai_event: extern "C" fn(VehicleRef, u32),
    pub game_event: extern "C" fn(VehicleRef, u32),
    pub news: extern "C" fn(VehicleRef, u32),
}
impl WaterServices {
    /// `InvalidateWaterRegion`.
    /// # Safety
    /// The service returns the current map's region owner, exclusively for this call.
    pub(crate) unsafe fn invalidate_region(&self, tile: u32, s: &Services) {
        let regions = (self.water_regions)();
        crate::water_regions::invalidate_water_region(
            unsafe { &mut *regions },
            s,
            crate::tile_type::TileIndex(tile),
        );
    }
}
/// Native packed-map/storage leaves for water class restoration.
#[repr(C)]
pub struct RestorationServices {
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
    let map = crate::map_func::World::new(s);
    let mut class = map.water_class(tile);
    /* Autoslope might turn an originally canal or river tile into land */
    let (slope, z) = map.tile_slope_z(TileIndex(tile));
    if slope != crate::slope_type::Slope::SLOPE_FLAT {
        if class == 1 {
            /* If we clear the canal, we have to remove it from the infrastructure count as well. */
            if (s.company_valid)(owner) {
                let infrastructure = CompanyInfrastructure::get(s, CompanyID(owner));
                unsafe {
                    CompanyInfrastructure::add_water(infrastructure, u32::MAX);
                };
                // SAFETY: Shared command tables are immutable and live for this call.
                unsafe { ((*(*(s.station_commands)()).command).infrastructure_dirty)(owner) };
            }
            /* Sloped canals are locks and no natural water remains whatever the slope direction */
            class = 3;
        }
        /* Only river water should be restored on appropriate slopes. Other water would be invalid on slopes */
        if class != crate::water_map::WaterClass::River.0 || !is_inclined_slope_raw(slope.0) {
            class = 3;
        }
    }
    if class == 0 && z > 0 {
        /* Update company infrastructure count. */
        if (s.company_valid)(owner) {
            let infrastructure = CompanyInfrastructure::get(s, CompanyID(owner));
            unsafe { CompanyInfrastructure::add_water(infrastructure, 1) };
            // SAFETY: Shared command tables are immutable and live for this call.
            unsafe { ((*(*(s.station_commands)()).command).infrastructure_dirty)(owner) };
        }
        class = 1;
    }
    crate::ground::do_clear_square_raw(tile, s);
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
pub(crate) const FLOOD_FROM: [u8; 15] = [
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
        self.types.read(TileIndex(tile)).0
    }
    /// Original `IsValidTile`: `tile < Map::Size() && !IsTileType(tile, MP_VOID)`.
    /// Legal map dimensions are powers of two whose product fits in u32.
    fn valid(&self, tile: u32) -> bool {
        tile < self.size_x * self.size_y && self.tile_type(tile) != 7
    }
    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn behaviour(&self, tile: u32) -> u8 {
        match self.tile_type(tile) {
            6 => {
                if self
                    .types
                    .asserted(self.s, |m| crate::water_map::is_coast(m, tile))
                {
                    return if is_slope_with_one_corner_raised_raw(
                        (self.types.asserted(self.s, |m| {
                            crate::tile_map::get_tile_slope(
                                m,
                                TileIndex(tile),
                                self.size_x,
                                self.size_y,
                            )
                        }))
                        .0,
                    ) {
                        1
                    } else {
                        3
                    };
                }
                u8::from(
                    self.types
                        .asserted(self.s, |m| crate::water_map::get_water_class(m, tile))
                        == 0,
                )
            }
            5 | 8 | 10 => u8::from(
                self.types
                    .asserted(self.s, |m| crate::water_map::get_water_class(m, tile))
                    == 0,
            ),
            1 => {
                if self
                    .types
                    .asserted(self.s, |m| crate::rail_map::get_rail_ground_type(m, tile))
                    == 13
                {
                    if is_slope_with_one_corner_raised_raw(
                        (self.types.asserted(self.s, |m| {
                            crate::tile_map::get_tile_slope(
                                m,
                                TileIndex(tile),
                                self.size_x,
                                self.size_y,
                            )
                        }))
                        .0,
                    ) {
                        1
                    } else {
                        3
                    }
                } else {
                    0
                }
            }
            4 => {
                if self
                    .types
                    .asserted(self.s, |m| crate::tree_map::get_tree_ground_raw(m, tile))
                    == 3
                {
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
        // SAFETY: The live result is not read again, ending its native lifetime.
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
        if self
            .types
            .asserted(self.s, |m| crate::rail_map::get_rail_ground_type(m, tile))
            == 13
        {
            return false;
        }
        let slope = self.types.asserted(self.s, |m| {
            crate::tile_map::get_tile_slope(m, TileIndex(tile), self.size_x, self.size_y)
        });
        let mut tracks = (self.r().tracks)(tile).0;
        if is_slope_with_one_corner_raised_raw(slope.0) {
            let corner = (slope.0.trailing_zeros() ^ 2) as usize;
            let lower = crate::track_func::corner_to_track_bits(corner as u8, |ok| {
                crate::movement::native_assert(self.s, ok);
            });
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
            if (5..10).contains(&crate::movement::rail_foundation(slope.0, tracks, |ok| {
                crate::movement::native_assert(self.s, ok);
            })) {
                flooded = true;
                (self.r().set_ground)(tile, RailGroundType(13));
                (self.s.mark_dirty)(tile);
                crate::ground::hit(crate::ground::Witness::HalfRail);
            }
        } else {
            let foundation = (self.s.apply_foundation)(
                Foundation(crate::movement::rail_foundation(slope.0, tracks, |ok| {
                    crate::movement::native_assert(self.s, ok);
                })),
                Slope(slope.0),
            );
            if foundation.z == 0
                && (is_steep_slope_raw(foundation.slope.0)
                    || is_slope_with_three_corners_raised_raw(foundation.slope.0))
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
        crate::effect::openttd_rust_effect_create_rel(
            v.0.as_ptr(),
            4,
            4,
            8,
            crate::effectvehicle_func::EV_EXPLOSION_LARGE,
        );
        if (self.s.disaster_sound)() {
            (self.v().play_sound)(v, 16);
        }
        crate::ground::hit(crate::ground::Witness::VehicleCrash);
    }
    fn victim(&self, v: VehicleRef, z: i32) {
        // Fields are read at the source's points; no callback intervenes before Crash.
        if v.status() & crate::vehicle_base::VS_CRASHED != 0 {
            crate::ground::hit(crate::ground::Witness::CrashedSkip);
            return;
        }
        match v.vehicle_type() {
            3 => {
                let tile = v.tile();
                if !(self.w.airport_tile)(tile) || (self.s.tile_max_z)(tile) != 0 {
                    return;
                }
                if v.subtype() == crate::aircraft::AIR_SHADOW {
                    return;
                }
                if v.z()
                    != (self.w.airport_z)(crate::newgrf_airport::station_fta(
                        crate::station_map::station_by_tile(tile, crate::newgrf_airport::runtime()),
                        crate::newgrf_airport::runtime(),
                    ))
                {
                    return;
                }
                self.crash(v);
            }
            0 | 1 => {
                if v.z() <= z {
                    // Source dereferences First only after this z test.
                    self.crash(v.first());
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
        let z = crate::bridge_map::get_bridge_pixel_height(tile, self.s);
        crate::ground::hit(crate::ground::Witness::BridgeStart);
        self.vehicles_on_tile(tile, z);
        crate::ground::hit(crate::ground::Witness::BridgeEnd);
        self.vehicles_on_tile(end, z);
    }
    /// `IsCanal(t) || IsRiver(t)`, with the canal test first.
    fn canal_or_river(&self, t: u32) -> bool {
        self.types
            .asserted(self.s, |m| crate::water_map::is_canal(m, t))
            || self
                .types
                .asserted(self.s, |m| crate::water_map::is_river(m, t))
    }
    fn dirty_neighbours(&self, tile: u32) {
        for direction in 0..8 {
            let target = offset(tile, direction, self.size_x);
            if self.valid(target) && self.tile_type(target) == 6 && self.canal_or_river(target) {
                (self.s.mark_dirty)(target);
            }
        }
    }
    fn flood(&self, tile: u32, call: *mut Call) {
        let company = (self.s.current_company)();
        (self.co().finance.set_current_company)(17);
        let slope = self.types.asserted(self.s, |m| {
            crate::tile_map::get_tile_slope(m, TileIndex(tile), self.size_x, self.size_y)
        });
        let flooded = if slope == crate::slope_type::Slope::SLOPE_FLAT {
            /* Flood vehicles */
            self.flood_vehicles(tile);
            /* flood flat tile */
            if self.clear(tile, call) {
                (self.w.make_sea)(tile);
                (self.s.mark_dirty)(tile);
                true
            } else {
                false
            }
        } else {
            /* make coast.. */
            match self.tile_type(tile) {
                crate::tile_type::MP_RAILWAY => {
                    if (self.s.is_plain_rail)(tile) {
                        self.flood_vehicles(tile);
                        self.half_flood(tile, call)
                    } else {
                        false
                    }
                }
                crate::tile_type::MP_TREES if !is_slope_with_one_corner_raised_raw(slope.0) => {
                    (self.w.set_tree_ground)(tile, TreeGround::TREE_GROUND_SHORE, 3);
                    (self.s.mark_dirty)(tile);
                    true
                }
                crate::tile_type::MP_CLEAR | crate::tile_type::MP_TREES => {
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
        };
        if flooded {
            self.dirty_neighbours(tile);
            crate::signal::acquire_context(self.w.signal_context).flush();
            let stations = (self.s.station_commands)();
            // SAFETY: the getter supplies the borrowed process-lifetime table;
            // docking runs synchronously without a live owner borrow.
            unsafe { crate::station_cmd::check_possible_docking_raw(stations, tile) };
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
                    if kind == 4
                        && self
                            .types
                            .asserted(self.s, |m| crate::tree_map::get_tree_ground_raw(m, target))
                            == 3
                    {
                        continue;
                    }
                    let foundation = crate::movement::foundation_slope(self.s, target);
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
                let mask = FLOOD_FROM
                    [usize::from(crate::movement::foundation_slope(self.s, tile).slope.0 & 15)];
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
    call: *mut Call,
    w: *const WaterServices,
    s: *const Services,
) {
    // SAFETY: Same synchronous table contract as the in-crate scheduler entry;
    // the descriptor ends before this synchronous native entry returns.
    unsafe {
        let shared = &*s;
        let map = crate::map_func::World::new(shared);
        let types = &map.types;
        let tile_type = types.read(TileIndex(tile));
        let ambient = tile_type == TileType::MP_WATER && crate::newgrf_generic::ambient_enabled();
        water_tile_loop(
            tile,
            tile_type.0,
            ambient,
            size_x,
            size_y,
            call,
            w,
            s,
            types,
        );
    };
}
/// Original `TileLoop_Water` after its caller has read `GetTileType(tile)`; `ambient` is
/// the caller's observation of the `AmbientSoundEffect` gate, used only on water tiles.
#[allow(clippy::inline_always, clippy::too_many_arguments)]
#[inline(always)]
pub(crate) unsafe fn water_tile_loop(
    tile: u32,
    tile_type: u8,
    ambient: bool,
    size_x: u32,
    size_y: u32,
    call: *mut Call,
    w: *const WaterServices,
    s: *const Services,
    types: &TileTypeAccess,
) {
    if tile_type == 6 {
        // SAFETY: Live immutable tables; no owner access spans ambient sound.
        crate::newgrf_generic::ambient(ambient, tile);
        if types.asserted(unsafe { &*s }, |m| {
            crate::water_map::is_non_flooding_water_tile(m, tile)
        }) {
            return;
        }
    }
    // SAFETY: Resolve only the tables reached after the original early return.
    unsafe { World::new(w, s, types, size_x, size_y) }.tile_loop(tile, call);
}
/// # Safety
/// Same table/context contract; original native plain-rail assertion is retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_flood_halftile_abi(
    tile: u32,
    call: *mut Call,
    w: *const WaterServices,
    s: *const Services,
) -> u8 {
    // SAFETY: Serialized native entry; no borrowed state crosses commands. Half
    // flooding uses the canonical descriptor and original map dimensions.
    let map = crate::map_func::World::new(unsafe { &*s });
    u8::from(
        unsafe { World::new(w, s, &map.types, map.dimensions.size_x(), map.size().1) }
            .half_flood(tile, call),
    )
}
/// # Safety
/// Live immutable service tables; valid original tile argument.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_flooding_behaviour(
    tile: u32,
    w: *const WaterServices,
    s: *const Services,
) -> FloodingBehaviour {
    // SAFETY: Serialized native entry; scalar reads end before callbacks.
    let map = crate::map_func::World::new(unsafe { &*s });
    FloodingBehaviour(
        unsafe { World::new(w, s, &map.types, map.dimensions.size_x(), map.size().1) }
            .behaviour(tile),
    )
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
    let map = crate::map_func::World::new(s);
    unsafe { World::new(w, s, &map.types, size_x, size_y) }.dirty_neighbours(tile);
}
/// `ClearNeighbourNonFloodingStates`, over the map dimensions `size_x` by `size_y`.
/// # Safety
/// Live immutable service tables for the synchronous no-unwind invocation.
pub(crate) unsafe fn clear_neighbour_non_flooding_states(
    tile: crate::tile_type::TileIndex,
    size_x: u32,
    size_y: u32,
    w: *const WaterServices,
    s: *const Services,
) {
    // SAFETY: Serialized native entry; no map references are formed.
    let shared = unsafe { &*s };
    let map = crate::map_func::World::new(shared);
    let world = unsafe { World::new(w, s, &map.types, size_x, size_y) };
    for direction in 0..8 {
        let target = offset(tile.0, direction, size_x);
        if world.valid(target) && world.tile_type(target) == 6 {
            (world.w.set_non_flooding)(target, false);
            crate::ground::hit(crate::ground::Witness::Wakeup);
        }
    }
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
    // SAFETY: The caller's contract is the function's.
    unsafe {
        clear_neighbour_non_flooding_states(
            crate::tile_type::TileIndex(tile),
            size_x,
            size_y,
            w,
            s,
        );
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
        openttd_rust_water_tile_loop(tile, size_x, size_y, call, (s.water_services)(), s);
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDAirportGround" ["landscape"] => AirportGround {
        station,
        tile,
        width,
        height,
    };
    "OpenTTDWaterGroundServices" ["landscape"] => WaterServices {
        rail,
        clear,
        vehicles,
        costs,
        company,
        signal_context,
        set_non_flooding,
        set_tree_ground,
        make_sea,
        buoy_or_dock,
        water_regions,
        remove_rail,
        airport_tile,
        airport,
        airport_member,
        airport_z,
        crash,
        ai_event,
        game_event,
        news,
    };
    "OpenTTDWaterRestorationServices" ["common"] => RestorationServices {
        make_canal,
        make_river,
    };
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
        set_non_flooding: unused2,
        set_tree_ground: unused3,
        make_sea: unused1,
        buoy_or_dock: unused1,
        water_regions: fixture_water_regions,
        remove_rail: unused3,
        airport_tile: unused1,
        airport: unused1,
        airport_member: unused2,
        airport_z: unused1,
        crash: unused1,
        ai_event: unused2,
        game_event: unused2,
        news: unused2,
    }
}

/// The invalidation's `IsValidTile` reads the fixture map, so a fixture that
/// invalidates a valid tile reaches an owner for that map: one per thread,
/// replaced when the installed map's dimensions change.
#[cfg(test)]
extern "C" fn fixture_water_regions() -> *mut crate::water_regions::Regions {
    use std::cell::Cell;
    thread_local! {
        static REGIONS: Cell<(u32, u32, *mut crate::water_regions::Regions)> =
            const { Cell::new((0, 0, std::ptr::null_mut())) };
    }
    // SAFETY: This thread's fixture statics address its installed map.
    let (allocation, map) = unsafe { (*crate::map_func::fixture::statics()).load() };
    let (size_x, size_y) = (map.size_x(), allocation.count / map.size_x());
    let (x, y, regions) = REGIONS.get();
    if !regions.is_null() && (x, y) == (size_x, size_y) {
        return regions;
    }
    // A replaced owner is leaked: a caller may still hold it for this thread's test.
    let regions = Box::into_raw(Box::new(crate::water_regions::Regions::new(size_x, size_y)));
    REGIONS.set((size_x, size_y, regions));
    regions
}

/// Native `FloodingBehaviour` representation; every byte pattern is retained.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FloodingBehaviour(pub u8);

impl FloodingBehaviour {
    /// The tile does not flood neighboured tiles.
    pub const FLOOD_NONE: Self = Self(0);
    /// The tile floods neighboured tiles.
    pub const FLOOD_ACTIVE: Self = Self(1);
    /// The tile does not actively flood neighboured tiles, but it prevents them from drying up.
    pub const FLOOD_PASSIVE: Self = Self(2);
    /// The tile drys up if it is not constantly flooded from neighboured tiles.
    pub const FLOOD_DRYUP: Self = Self(3);
}
