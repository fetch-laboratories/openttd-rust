/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete selected road command policy. Shared map and pool storage remain
//! native. Calls retain only copied identities; no Rust world borrow spans a
//! command, town rating update, vehicle callback or other reentry. Native costs
//! are stack-owned opaque objects, and panics/environmental exceptions abort.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::if_not_else,
    clippy::fn_params_excessive_bools
)]
use crate::command_type::DoCommandFlag;
use crate::company::COMPANIES as MAX_COMPANIES;
use crate::company_type::{INVALID_OWNER, OWNER_DEITY, OWNER_NONE, OWNER_TOWN, OWNER_WATER};
use crate::rail_cmd::{
    CROSSING_SLOPES, ConvertFrame as Area, CrossingRoad, Foundation, LEVELCROSSING_TRACKBIT_FACTOR,
    RailCommandServices, TUNNELBRIDGE_TRACKBIT_FACTOR,
};
use crate::services::{
    CommandCost, CommandCostServices, CommandServices, Map, RailMapServices, Services, Slope,
    TownHandle,
};
use crate::slope_func::{get_slope_max_z, is_slope_with_one_corner_raised, is_steep_slope};
use crate::strings_type::INVALID_STRING_ID;
use crate::tile_type::{MP_RAILWAY, MP_ROAD, MP_STATION, MP_TUNNELBRIDGE};
use crate::witness as w;
/// Opaque caller-owned native affected-road-vehicle list.
#[repr(C)]
pub struct ConvertFrame {
    _private: [u8; 0],
}
/// Caller-owned native stack results and lazy conversion cursor. Never dereferenced.
#[repr(C)]
pub struct CommandScratch {
    _private: [u8; 0],
}
/// Caller-owned native slot that holds a context only once a road vehicle builds.
#[repr(C)]
pub struct CommandSlot {
    _private: [u8; 0],
}
/// Borrowed native stack context supplied by the outer command/controller frame.
#[repr(C)]
pub struct CommandContext {
    pub services: *const RoadCommandServices,
    pub scratch: *mut CommandScratch,
    pub cost: *mut CommandCost,
    pub ret: *mut CommandCost,
}
impl CommandContext {
    /// # Safety
    /// All context pointers denote live exclusive caller-owned stack storage.
    pub(crate) unsafe fn build(&self, flags: u16, tile: u32, bits: u8, rt: u8, town: u16) -> bool {
        // SAFETY: immutable native table and exclusive stack slots outlive the call.
        let s = unsafe { &*self.services };
        unsafe {
            (s.do_build)(
                self.services,
                self.scratch,
                self.cost,
                self.ret,
                flags,
                tile,
                bits,
                rt,
                0,
                town,
            );
        }
        let costs = unsafe { &*(*s.rail_commands).cost };
        (costs.success)(self.cost)
    }
}
/// Only the native stack objects needed by a nested conversion.
#[repr(C)]
pub struct ConversionScratch {
    pub frame: *mut ConvertFrame,
    pub area: *mut Area,
    pub error: *mut CommandCost,
    pub ret: *mut CommandCost,
}
/// Only the three original predicates reached by the nearest-town map scan.
#[repr(C)]
pub struct NearestTownTile {
    pub road: bool,
    pub depot: bool,
    pub town_owned: bool,
}
/// Fields read together only in the deity town-boundary branch.
#[repr(C)]
pub struct TownEdge {
    pub tile: u32,
    pub radius: u32,
}
/// Fields needed to advance the original ascending effect iterator.
#[repr(C)]
pub struct WorksEffect {
    pub id: u32,
    pub tile: u32,
}
/// Native `StringID` constants.
#[repr(C)]
pub struct Strings {
    pub no_road: u32,
    pub no_tram: u32,
    pub junction: u32,
    pub remove_road: u32,
    pub no_suitable_road: u32,
    pub no_suitable_tram: u32,
    pub authority: u32,
}
/// Native immutable table; pointer callbacks require live stack costs/frames.
#[repr(C)]
pub struct RoadCommandServices {
    /// Native `rail_commands` service.
    /// Direct Rust owner entry; production binds `openttd_rust_road_do_build`.
    pub do_build: unsafe extern "C" fn(
        *const RoadCommandServices,
        *mut CommandScratch,
        *mut CommandCost,
        *mut CommandCost,
        u16,
        u32,
        u8,
        u8,
        u8,
        u16,
    ),
    pub rail_commands: *const RailCommandServices,
    /// Native `town` service.
    pub town: *const crate::town::Leaves,
    /// Native `authority` service.
    pub authority: *const crate::town::AuthorityServices,
    /// Native `strings` service.
    pub strings: Strings,
    /// Native `town_valid` service.
    pub nearest_tile: extern "C" fn(u32) -> NearestTownTile,
    pub town_valid: extern "C" fn(u16) -> bool,
    /// Native `town_edge` service.
    pub town_edge: extern "C" fn(*const TownHandle) -> TownEdge,
    /// Native `set_town` service.
    pub set_town: extern "C" fn(u32, u16),
    /// Native `foundation` service.
    pub foundation: extern "C" fn(u8, u8) -> Foundation,
    /// Native `extra_dynamite` service.
    pub extra_dynamite: extern "C" fn() -> bool,
    /// Native `depot_slope` service.
    pub depot_slope: extern "C" fn(u8, u8) -> bool,
    /// Native `make_depot` service.
    pub make_depot: extern "C" fn(u32, u8, u16, u8, u8),
    /// Native `depot_exit` service.
    pub depot_exit: extern "C" fn(u32, u8),
    /// Next effect in native pool order; `u32::MAX` denotes end.
    pub next_effect: extern "C" fn(u32) -> WorksEffect,
    /// Delete the source-valid effect shell and its owner.
    pub delete_effect: extern "C" fn(u32),
    /// Native encoded owned-by-town result.
    pub owned_by_town: unsafe extern "C" fn(*mut CommandCost, u32),
    /// One extra native result for nested build or low-money remove test.
    pub scratch_cost: unsafe extern "C" fn(*mut CommandScratch) -> *mut CommandCost,
    /// Lazily construct the nested conversion stack objects, resetting prior use.
    pub conversion_scratch: unsafe extern "C" fn(*mut CommandScratch) -> ConversionScratch,
    /// Native `collect_vehicles` service.
    pub collect_vehicles: unsafe extern "C" fn(*mut ConvertFrame, u32),
    /// Native `cargo_changed` service.
    pub cargo_changed: unsafe extern "C" fn(*mut ConvertFrame),
}
use crate::road_type::INVALID_ROADTYPE as INVALID;
use crate::services::Cost;
const INVALID_TOWN: u16 = u16::MAX;
const RATING_ROAD_DOWN_STEP_EDGE: i32 = -18;
const RATING_ROAD_DOWN_STEP_INNER: i32 = -50;
const RATING_ROAD_MINIMUM: i32 = -100;
const ROAD_DEPOT_TRACKBIT_FACTOR: u32 = 2;
const ROAD_STOP_TRACKBIT_FACTOR: u32 = 2;
const INVALID_SLOPES: [[u8; 15]; 2] = [
    [0, 12, 9, 8, 3, 0, 1, 0, 6, 4, 0, 0, 2, 0, 0],
    [0, 0, 0, 5, 0, 15, 10, 15, 0, 10, 15, 15, 5, 15, 15],
];
fn mirror(bits: u8) -> u8 {
    ((bits << 2) | (bits >> 2)) & 15
}
fn straight(bits: u8) -> bool {
    bits == 10 || bits == 5
}
fn axis_bits(axis: u8) -> u8 {
    if axis == 0 { 10 } else { 5 }
}
fn dir_bit(dir: u8) -> u8 {
    8 >> dir
}
fn simple_slope(slope: u8) -> u8 {
    if !is_steep_slope(slope) {
        return slope;
    }
    // Steep W/S/E/N have every corner except the opposite corner raised.
    match slope & 15 {
        11 => 1,
        7 => 2,
        14 => 4,
        13 => 8,
        _ => unreachable!(),
    }
}
fn price(index: usize) -> i64 {
    crate::company::price(index)
}
/// Original `CanConvertUnownedRoadType` permission helper.
fn can_convert_unowned_road_type(owner: u8, rtt: u8) -> bool {
    owner == OWNER_NONE || (owner == OWNER_TOWN && rtt == 0)
}
struct Ctx<'a> {
    s: &'a RoadCommandServices,
    r: &'a RailCommandServices,
    shared: &'a Services,
    rail: &'a RailMapServices,
    ground: &'a crate::ground::RailServices,
    costs: &'a CommandCostServices,
    commands: &'a CommandServices,
    town: &'a crate::town::Leaves,
    authority: &'a crate::town::AuthorityServices,
    map: Map,
}
impl<'a> Ctx<'a> {
    /// # Safety
    /// Native immutable process-lifetime tables and nested tables are valid.
    unsafe fn new(s: *const RoadCommandServices) -> Self {
        w::hit(w::ROAD_COMMANDS_ENTRIES);
        // SAFETY: caller supplies immutable process-lifetime tables.
        let s = unsafe { &*s };
        let r = unsafe { &*s.rail_commands };
        let shared = unsafe { &*r.shared };
        let rail = unsafe { &*r.rail };
        Self {
            s,
            r,
            shared,
            rail,
            ground: unsafe { &*rail.ground },
            costs: unsafe { &*r.cost },
            commands: unsafe { &*r.command },
            town: unsafe { &*s.town },
            authority: unsafe { &*s.authority },
            map: Map {
                log_x: (shared.map_log_x)() as u8,
            },
        }
    }
    fn command(&self, cost: Cost<'_>, flags: u16, tile: u32, body: impl FnMut(u16) -> i64) -> i64 {
        crate::command::nested(self.commands, self.shared, cost, tile, flags, body)
    }

    fn do_build(
        &self,
        nested: *mut CommandScratch,
        cost: Cost<'_>,
        ret: Cost<'_>,
        flags: u16,
        tile: u32,
        bits: u8,
        rt: u8,
        drd: u8,
        town: u16,
    ) {
        self.command(cost, flags, tile, |f| {
            self.build(nested, cost, ret, f, tile, bits, rt, drd, town);
            0
        });
    }
    fn do_convert(
        &self,
        nested: *mut CommandScratch,
        cost: Cost<'_>,
        flags: u16,
        end: u32,
        start: u32,
        rt: u8,
        diagonal: bool,
    ) {
        self.command(cost, flags, end, |f| {
            // SAFETY: native caller owns the stack frame. Reset for each actual
            // proc call, just as the native test and execute have separate locals.
            let scratch = unsafe { (self.s.conversion_scratch)(nested) };
            self.convert(
                scratch.frame,
                scratch.area,
                cost,
                self.c(scratch.error),
                self.c(scratch.ret),
                f,
                end,
                start,
                rt,
                diagonal,
            );
            0
        });
    }
    fn c(&self, p: *mut CommandCost) -> Cost<'a> {
        Cost(p, self.costs)
    }
    fn size(&self) -> u32 {
        self.map
            .size_x()
            .wrapping_mul((self.shared.map_max_y)().wrapping_add(1))
    }
    fn tile_type(&self, tile: u32) -> u8 {
        (self.shared.tile_type)(tile)
    }
    fn kind(&self, tile: u32) -> u8 {
        (self.shared.road_tile_kind)(tile).0
    }
    fn rt(&self, tile: u32, rtt: u8) -> u8 {
        if rtt == 0 {
            (self.shared.road_type)(tile)
        } else {
            (self.shared.tram_type)(tile).0
        }
    }
    fn owner(&self, tile: u32, rtt: u8) -> u8 {
        if rtt == 0 {
            (self.shared.road_owner)(tile)
        } else {
            (self.shared.tram_owner)(tile)
        }
    }
    fn bits(&self, tile: u32, rtt: u8) -> u8 {
        (self.shared.normal_road_bits)(tile, rtt).0
    }
    fn any_bits(&self, tile: u32, rtt: u8) -> u8 {
        (self.shared.road_bits)(tile, rtt, false)
    }
    fn may_have(&self, tile: u32) -> bool {
        let present = (self.shared.may_have_road)(tile);
        if !present {
            w::hit(w::ROAD_COMMANDS_NON_ROAD);
        }
        present
    }
    fn no_road(&self, c: Cost<'_>, rtt: u8) {
        c.error(if rtt == 1 {
            self.s.strings.no_tram
        } else {
            self.s.strings.no_road
        });
    }
    fn clear(&self, c: Cost<'_>, flags: u16, tile: u32) {
        crate::landscape_cmd::clear_command(self.commands, c.0, tile, flags);
    }
    fn no_vehicle(&self, c: Cost<'_>, tile: u32) {
        unsafe {
            (self.commands.no_vehicle)(c.0, tile);
        }
    }
    fn ownership(&self, c: Cost<'_>, owner: u8, tile: u32) {
        unsafe {
            (self.commands.ownership)(c.0, owner, tile);
        }
    }
    fn tile_ownership(&self, c: Cost<'_>, tile: u32) {
        unsafe {
            (self.commands.tile_ownership)(c.0, tile);
        }
    }
    fn free(&self, c: Cost<'_>, tile: u32, end: u32) {
        unsafe {
            (self.commands.wormhole_free)(c.0, tile, end, core::ptr::null());
        }
    }
    fn infrastructure(&self, rt: u8, owner: u8, count: i32) {
        // SAFETY: canonical immutable table; this entry holds no world borrow.
        unsafe {
            update_company_road_infrastructure(self.s, rt, owner, count);
        }
    }

    fn update_town(&self, tile: u32) {
        let town = (self.shared.calc_closest_town)(tile, u32::MAX);
        let id = if town.is_null() {
            INVALID_TOWN
        } else {
            (self.authority.town_index)(town)
        };
        (self.s.set_town)(tile, id);
    }
    fn rating(&self, c: Cost<'_>, town: *const TownHandle, flags: u16, kind: u8) {
        // SAFETY: live native town identity (or null), immutable tables and stack cost.
        unsafe {
            crate::town::openttd_rust_town_authority_remove(
                self.s.authority,
                self.r.shared,
                self.r.cost,
                c.0,
                town,
                flags & 16 != 0,
                kind,
            );
        }
    }
    fn allow_remove(
        &self,
        c: Cost<'_>,
        tile: u32,
        remove: u8,
        owner: u8,
        rtt: u8,
        flags: u16,
        town_check: bool,
    ) {
        c.zero();
        if (self.town.environment.editor)() || remove == 0 {
            return;
        }
        let company = (self.shared.current_company)();
        if company == OWNER_WATER || (rtt == 0 && !(self.shared.company_valid)(company)) {
            return;
        }
        if owner != OWNER_TOWN {
            if owner != OWNER_NONE {
                self.ownership(c, owner, 0);
            }
            return;
        }
        if !town_check || (self.authority.bulldozer)() {
            return;
        }
        let town = (self.shared.closest_town)(tile, u32::MAX);
        if town.is_null() {
            return;
        }
        w::hit(w::ROAD_COMMANDS_TOWN_REMOVE);
        self.rating(c, town, flags, 0);
        if !c.ok() {
            return;
        }
        let present = self.any_bits(tile, rtt);
        let mut neighbours = 0;
        for dir in 0..4 {
            if present & dir_bit(dir) != 0
                && self.any_bits(self.map.add_diag(tile, dir), rtt) & dir_bit(dir ^ 2) != 0
            {
                neighbours |= dir_bit(dir);
            }
        }
        let mut decrease = RATING_ROAD_DOWN_STEP_EDGE;
        if neighbours & neighbours.wrapping_sub(1) != 0 && neighbours & remove != 0 {
            if !(self.s.extra_dynamite)() {
                (self.costs.error_param)(
                    c.0,
                    self.s.strings.authority,
                    u64::from((self.authority.town_index)(town)),
                );
                return;
            }
            decrease = RATING_ROAD_DOWN_STEP_INNER;
        }
        // SAFETY: owner access resolves here and ends before the reentrant dirty-window service.
        unsafe {
            crate::town::openttd_rust_town_change_rating(
                self.s.town,
                self.r.shared,
                (self.shared.town_owner)(town),
                u32::from((self.authority.town_index)(town)),
                town as usize,
                company,
                decrease,
                RATING_ROAD_MINIMUM,
                flags & 512 != 0,
                (self.authority.bulldozer)(),
            );
        }
    }
    fn remove(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        tile: u32,
        flags: u16,
        mut pieces: u8,
        rtt: u8,
        town_check: bool,
    ) {
        debug_assert!(pieces != 0);
        w::hit(w::ROAD_COMMANDS_REMOVE);
        let rt = if self.may_have(tile) {
            self.rt(tile, rtt)
        } else {
            INVALID
        };
        if rt == INVALID {
            self.no_road(cost, rtt);
            return;
        }
        match self.tile_type(tile) {
            MP_ROAD => self.no_vehicle(cost, tile),
            MP_STATION => {
                if !(self.shared.is_drive_through_stop)(tile) {
                    cost.error(INVALID_STRING_ID);
                    return;
                }
                self.no_vehicle(cost, tile);
            }
            MP_TUNNELBRIDGE => {
                if (self.shared.tunnel_transport)(tile) != 1 {
                    cost.error(INVALID_STRING_ID);
                    return;
                }
                self.free(cost, tile, (self.rail.other_tunnel_bridge_end)(tile));
            }
            _ => {
                cost.error(INVALID_STRING_ID);
                return;
            }
        }
        if !cost.ok() {
            return;
        }
        self.allow_remove(
            cost,
            tile,
            pieces,
            self.owner(tile, rtt),
            rtt,
            flags,
            town_check,
        );
        if !cost.ok() {
            return;
        }
        if self.tile_type(tile) != MP_ROAD {
            if self.rt(tile, rtt ^ 1) == INVALID {
                self.clear(cost, flags, tile);
                return;
            }
            cost.set(0);
            if self.tile_type(tile) == MP_TUNNELBRIDGE {
                w::hit(w::ROAD_COMMANDS_REMOVE_WORMHOLE);
                if axis_bits((self.shared.tunnel_bridge_direction)(tile) & 1) & pieces == 0 {
                    self.no_road(cost, rtt);
                    return;
                }
                let end = (self.rail.other_tunnel_bridge_end)(tile);
                let len = self.length(tile, end).wrapping_add(2);
                cost.add_money(
                    crate::road_cost::clear_cost(self.shared, rt)
                        .saturating_mul(i64::from(len.wrapping_mul(2))),
                );
                if flags & DoCommandFlag::EXECUTE != 0 {
                    self.infrastructure(
                        rt,
                        self.owner(tile, rtt),
                        len.wrapping_mul(2)
                            .wrapping_mul(TUNNELBRIDGE_TRACKBIT_FACTOR)
                            .wrapping_neg() as i32,
                    );
                    (self.shared.set_road_type)(end, rtt, INVALID);
                    (self.shared.set_road_type)(tile, rtt, INVALID);
                    let other_owner = self.owner(tile, rtt ^ 1);
                    let owner = (self.shared.tile_owner)(tile);
                    if owner != other_owner && owner != OWNER_TOWN {
                        (self.shared.set_tile_owner)(tile, other_owner);
                        (self.shared.set_tile_owner)(end, other_owner);
                    }
                    self.dirty_wormhole(tile, end);
                }
            } else {
                w::hit(w::ROAD_COMMANDS_REMOVE_STOP);
                cost.add_money(crate::road_cost::clear_cost(self.shared, rt).saturating_mul(2));
                if flags & DoCommandFlag::EXECUTE != 0 {
                    self.infrastructure(rt, self.owner(tile, rtt), -2);
                    (self.shared.set_road_type)(tile, rtt, INVALID);
                    (self.shared.mark_dirty)(tile);
                }
            }
            return;
        }
        match self.kind(tile) {
            0 => {
                let slope = simple_slope((self.shared.tile_slope)(tile));
                let mut present = self.bits(tile, rtt);
                let other = self.bits(tile, rtt ^ 1);
                let foundation = (self.s.foundation)(slope, present).0;
                if (self.shared.has_road_works)(tile)
                    && (self.shared.current_company)() != OWNER_WATER
                {
                    cost.error(self.r.strings.road_works);
                    return;
                }
                if (straight(other) && other & INVALID_SLOPES[0][usize::from(slope & 15)] != 0)
                    || (slope != 0 && !(self.shared.build_on_slopes)())
                {
                    pieces |= mirror(pieces);
                }
                pieces &= present;
                if pieces == 0 {
                    self.no_road(cost, rtt);
                    return;
                }
                present ^= pieces;
                if slope != 0
                    && present != 0
                    && present & INVALID_SLOPES[0][usize::from(slope & 15)] == present
                {
                    cost.error(INVALID_STRING_ID);
                    return;
                }
                if flags & DoCommandFlag::EXECUTE != 0 {
                    if (self.shared.has_road_works)(tile) {
                        debug_assert_eq!((self.shared.current_company)(), OWNER_WATER);
                        w::hit(w::ROAD_COMMANDS_FLOOD_WORKS);
                        let mut from = 0;
                        loop {
                            let effect = (self.s.next_effect)(from);
                            if effect.id == u32::MAX {
                                break;
                            }
                            from = effect.id.wrapping_add(1);
                            if effect.tile == tile {
                                (self.s.delete_effect)(effect.id);
                            }
                        }
                    }
                    self.infrastructure(rt, self.owner(tile, rtt), -(pieces.count_ones() as i32));
                    if present == 0 {
                        if self.rt(tile, rtt ^ 1) == INVALID {
                            crate::ground::clear_square(tile, self.shared);
                        } else {
                            if rtt == 0 && self.owner(tile, rtt) == OWNER_TOWN {
                                self.update_town(tile);
                            }
                            if rtt == 0 {
                                (self.shared.set_road_directions)(tile, 0);
                            }
                            (self.shared.set_road_bits)(tile, 0, rtt);
                            (self.shared.set_road_type)(tile, rtt, INVALID);
                            (self.shared.mark_dirty)(tile);
                        }
                    } else {
                        if rtt == 0 {
                            (self.shared.set_road_directions)(tile, 0);
                        }
                        (self.shared.set_road_bits)(tile, present, rtt);
                        (self.shared.mark_dirty)(tile);
                    }
                }
                cost.set(
                    crate::road_cost::clear_cost(self.shared, rt)
                        .saturating_mul(i64::from(pieces.count_ones())),
                );
                if foundation == 0 && (self.s.foundation)(slope, present).0 != 0 {
                    cost.add_money(price(57));
                }
            }
            1 => {
                w::hit(w::ROAD_COMMANDS_REMOVE_CROSSING);
                if pieces & ((self.shared.crossing_road_bits)(tile).0 ^ 15) != 0 {
                    cost.error(INVALID_STRING_ID);
                    return;
                }
                if flags & DoCommandFlag::EXECUTE != 0 {
                    (self.r.crossing_removed)(tile, (self.rail.crossing_road_axis)(tile).0);
                    self.infrastructure(rt, self.owner(tile, rtt), -2);
                    let track = crate::ground::crossing_rail_track(tile, self.rail).0;
                    if self.rt(tile, rtt ^ 1) == INVALID {
                        let tracks = (self.rail.crossing_rail_bits)(tile).0;
                        let reserved = (self.shared.crossing_reserved)(tile);
                        (self.rail.make_rail_normal)(
                            tile,
                            (self.shared.tile_owner)(tile),
                            tracks,
                            (self.rail.rail_type)(tile).0,
                        );
                        if reserved {
                            (self.rail.set_track_reservation)(tile, tracks);
                        }
                        let owner = (self.shared.tile_owner)(tile);
                        if (self.shared.company_valid)(owner) {
                            (self.r.rail_infrastructure)(
                                owner,
                                (self.rail.rail_type)(tile).0,
                                (LEVELCROSSING_TRACKBIT_FACTOR - 1).wrapping_neg(),
                            );
                            (self.commands.infrastructure_dirty)(owner);
                        }
                    } else {
                        (self.shared.set_road_type)(tile, rtt, INVALID);
                    }
                    (self.shared.mark_dirty)(tile);
                    (self.r.layout_changed)(tile, track);
                }
                cost.set(crate::road_cost::clear_cost(self.shared, rt).saturating_mul(2));
            }
            _ => cost.error(INVALID_STRING_ID),
        }
        // ret is a separate native slot reserved by callers for nested command paths.
        let _ = ret;
    }
    fn slope(&self, cost: Cost<'_>, mut slope: u8, pieces: &mut u8, existing: u8, other: u8) {
        *pieces &= !existing;
        if *pieces == 0 {
            cost.error(INVALID_STRING_ID);
            return;
        }
        cost.zero();
        if slope == 0 {
            return;
        }
        w::hit(w::ROAD_COMMANDS_SLOPE);
        slope = simple_slope(slope);
        let mut type_bits = existing | *pieces;
        if (self.shared.build_on_slopes)()
            && INVALID_SLOPES[0][usize::from(slope)] & (other | type_bits) == 0
        {
            if other | existing == 0 {
                cost.set(price(57));
            }
            return;
        }
        *pieces |= mirror(*pieces);
        type_bits = existing | *pieces;
        if straight(type_bits)
            && (other == type_bits || other == 0)
            && INVALID_SLOPES[1][usize::from(slope)] & (other | type_bits) == 0
        {
            if is_slope_with_one_corner_raised(slope) {
                if (self.shared.build_on_slopes)() {
                    if other | existing == 0 {
                        cost.set(price(57));
                    }
                    return;
                }
            } else {
                if existing.is_power_of_two() && (self.s.foundation)(slope, existing).0 == 0 {
                    cost.set(price(57));
                }
                return;
            }
        }
        cost.error(INVALID_STRING_ID);
    }
    fn length(&self, tile: u32, end: u32) -> u32 {
        let a = (self.map.tile_x(tile) + self.map.tile_y(tile)) as i32;
        let b = (self.map.tile_x(end) + self.map.tile_y(end)) as i32;
        a.abs_diff(b).wrapping_sub(1)
    }
    fn dirty_wormhole(&self, tile: u32, end: u32) {
        if (self.shared.is_bridge)(tile) {
            (self.shared.mark_bridge_dirty)(tile);
        } else {
            (self.shared.mark_dirty)(tile);
            (self.shared.mark_dirty)(end);
        }
    }
    fn build(
        &self,
        nested: *mut CommandScratch,
        cost: Cost<'_>,
        ret: Cost<'_>,
        flags: u16,
        tile: u32,
        mut pieces: u8,
        mut rt: u8,
        toggle: u8,
        mut town_id: u16,
    ) {
        w::hit(w::ROAD_COMMANDS_BUILD);
        let mut company = (self.shared.current_company)();
        cost.set(0);
        let mut existing = 0;
        let mut other = 0;
        if ((self.shared.company_valid)(company) && town_id != INVALID_TOWN)
            || (company == OWNER_TOWN && !(self.s.town_valid)(town_id))
            || (company == OWNER_DEITY && town_id != INVALID_TOWN)
        {
            cost.error(INVALID_STRING_ID);
            return;
        }
        if company != OWNER_TOWN {
            let town = (self.shared.calc_closest_town)(tile, u32::MAX);
            town_id = if town.is_null() {
                INVALID_TOWN
            } else {
                (self.authority.town_index)(town)
            };
            if company == OWNER_DEITY {
                company = OWNER_TOWN;
                if town.is_null() {
                    company = OWNER_NONE;
                } else {
                    let edge = (self.s.town_edge)(town);
                    if self.map.distance_square(tile, edge.tile) > edge.radius {
                        company = OWNER_NONE;
                    }
                }
            }
        }
        if pieces == 0 || pieces & !15 != 0 || toggle >= 4 || !(self.shared.road_type_valid)(rt) {
            cost.error(INVALID_STRING_ID);
            return;
        }
        let slope = (self.shared.tile_slope)(tile);
        let rtt = u8::from((self.shared.road_tram_type)(rt).0);
        let mut clear = false;
        match self.tile_type(tile) {
            MP_ROAD => match self.kind(tile) {
                0 => {
                    if (self.shared.has_road_works)(tile) {
                        cost.error(self.r.strings.road_works);
                        return;
                    }
                    other = self.bits(tile, rtt ^ 1);
                    if self.rt(tile, rtt) != INVALID {
                        existing = self.bits(tile, rtt);
                        if rtt == 0
                            && ((self.shared.disallowed_road_directions)(tile) != 0 || toggle != 0)
                            && !straight(existing | pieces)
                        {
                            cost.error(self.s.strings.junction);
                            return;
                        }
                        if existing & pieces == pieces {
                            if toggle != 0 && rtt == 0 {
                                w::hit(w::ROAD_COMMANDS_ONEWAY);
                                let owner = self.owner(tile, rtt);
                                if owner != OWNER_NONE {
                                    self.ownership(ret, owner, tile);
                                    if !ret.ok() {
                                        cost.take(ret);
                                        return;
                                    }
                                }
                                let old = (self.shared.disallowed_road_directions)(tile);
                                let new = old ^ toggle;
                                if old.count_ones() <= new.count_ones() {
                                    self.no_vehicle(ret, tile);
                                    if !ret.ok() {
                                        cost.take(ret);
                                        return;
                                    }
                                }
                                if flags & DoCommandFlag::EXECUTE != 0 && straight(existing) {
                                    (self.shared.set_road_directions)(tile, new);
                                    (self.shared.mark_dirty)(tile);
                                }
                                cost.zero();
                                return;
                            }
                            cost.error(self.r.strings.already_built);
                            return;
                        }
                        if rtt == 1 && existing.is_power_of_two() {
                            let owner = self.owner(tile, rtt);
                            if (self.shared.company_valid)(owner) {
                                self.ownership(ret, owner, 0);
                                if !ret.ok() {
                                    cost.take(ret);
                                    return;
                                }
                            }
                        }
                    }
                }
                1 => {
                    if (self.r.road_no_crossing)(rt) {
                        cost.error(self.r.strings.crossing_road);
                        return;
                    }
                    other = (self.shared.crossing_road_bits)(tile).0;
                    if pieces & (other ^ 15) != 0 {
                        clear = true;
                    } else {
                        pieces = other;
                        if self.rt(tile, rtt) != INVALID {
                            cost.error(self.r.strings.already_built);
                            return;
                        }
                    }
                }
                2 => {
                    if self.any_bits(tile, rtt) & pieces == pieces {
                        cost.error(self.r.strings.already_built);
                        return;
                    }
                    clear = true;
                }
                _ => unreachable!(),
            },
            MP_RAILWAY => {
                if is_steep_slope(slope) || CROSSING_SLOPES & (1 << slope) == 0 {
                    cost.error(self.r.strings.sloped_wrong);
                    return;
                }
                if !(self.r.crossing_with_competitor)()
                    && company != OWNER_TOWN
                    && company != OWNER_DEITY
                {
                    self.tile_ownership(ret, tile);
                    if !ret.ok() {
                        cost.take(ret);
                        return;
                    }
                }
                if (self.rail.rail_tile_type)(tile).0 != 0 {
                    clear = true;
                } else {
                    if (self.r.road_no_crossing)(rt) {
                        cost.error(self.r.strings.crossing_road);
                        return;
                    }
                    if (self.r.rail_no_crossings)((self.rail.rail_type)(tile).0) {
                        cost.error(self.r.strings.crossing_rail);
                        return;
                    }
                    let tracks = (self.ground.tracks)(tile).0;
                    let axis = match tracks {
                        1 if pieces & 0b1010 == 0 => 1,
                        2 if pieces & 5 == 0 => 0,
                        _ => {
                            clear = true;
                            0
                        }
                    };
                    if !clear {
                        self.no_vehicle(ret, tile);
                        if !ret.ok() {
                            cost.take(ret);
                            return;
                        }
                        w::hit(w::ROAD_COMMANDS_BUILD_CROSSING);
                        if flags & DoCommandFlag::EXECUTE != 0 {
                            let track = axis ^ 1;
                            (self.r.layout_changed)(tile, track);
                            self.infrastructure(rt, company, 2);
                            let owner = (self.shared.tile_owner)(tile);
                            if (self.shared.company_valid)(owner) {
                                (self.r.rail_infrastructure)(
                                    owner,
                                    (self.rail.rail_type)(tile).0,
                                    LEVELCROSSING_TRACKBIT_FACTOR - 1,
                                );
                                (self.commands.infrastructure_dirty)(owner);
                            }
                            let reserved = (self.rail.reserved_tracks)(tile).0 & (1 << track) != 0;
                            (self.r.make_crossing)(
                                tile,
                                company,
                                company,
                                (self.shared.tile_owner)(tile),
                                axis,
                                (self.rail.rail_type)(tile).0,
                                if rtt == 0 { rt } else { INVALID },
                                if rtt == 1 { rt } else { INVALID },
                                town_id,
                            );
                            (self.shared.set_crossing_reserved)(tile, reserved);
                            (self.shared.update_level_crossing)(tile, false);
                            (self.r.mark_crossings)(tile, (self.rail.crossing_road_axis)(tile).0);
                            (self.shared.mark_dirty)(tile);
                        }
                        cost.set(crate::road_cost::build_cost(self.shared, rt).saturating_mul(2));
                        return;
                    }
                }
            }
            MP_STATION => {
                if self.any_bits(tile, rtt) & pieces == pieces {
                    cost.error(self.r.strings.already_built);
                    return;
                }
                if !(self.shared.is_drive_through_stop)(tile) {
                    clear = true;
                } else {
                    let bits = axis_bits((self.shared.drive_through_axis)(tile));
                    if pieces & !bits != 0 {
                        clear = true;
                    } else {
                        pieces = bits;
                        if self.rt(tile, rtt) != INVALID {
                            cost.error(self.r.strings.already_built);
                            return;
                        }
                    }
                }
            }
            MP_TUNNELBRIDGE => {
                if (self.shared.tunnel_transport)(tile) != 1
                    || mirror(dir_bit((self.shared.tunnel_bridge_direction)(tile))) != pieces
                {
                    clear = true;
                } else {
                    if self.rt(tile, rtt) != INVALID {
                        cost.error(self.r.strings.already_built);
                        return;
                    }
                    self.free(ret, tile, (self.rail.other_tunnel_bridge_end)(tile));
                    if !ret.ok() {
                        cost.take(ret);
                        return;
                    }
                }
            }
            _ => clear = true,
        }
        if clear {
            w::hit(w::ROAD_COMMANDS_BUILD_CLEAR);
            self.clear(ret, flags, tile);
            if !ret.ok() {
                cost.take(ret);
                return;
            }
            cost.add_money(ret.money());
        }
        if other != pieces {
            self.slope(ret, slope, &mut pieces, existing, other);
            if !ret.ok() || (ret.money() != 0 && !(self.shared.build_on_slopes)()) {
                cost.error(self.r.strings.sloped_wrong);
                return;
            }
            cost.add_money(ret.money());
        }
        if !clear {
            if self.tile_type(tile) == MP_ROAD {
                pieces &= existing ^ 15;
                if (self.shared.is_normal_road)(tile) {
                    let slope = (self.shared.tile_slope)(tile);
                    let new = (self.s.foundation)(slope, pieces | existing).0;
                    let bits = self.bits(tile, rtt ^ 1);
                    if bits != 0 && (self.s.foundation)(slope, bits).0 != new {
                        cost.error(self.r.strings.sloped_wrong);
                        return;
                    }
                }
            }
            self.no_vehicle(ret, tile);
            if !ret.ok() {
                cost.take(ret);
                return;
            }
            if self.tile_type(tile) == MP_ROAD && (self.shared.is_normal_road)(tile) {
                let old = self.rt(tile, rtt);
                if old != INVALID && old != rt {
                    if (self.shared.road_powered)(rt, old) {
                        rt = old;
                    } else if (self.shared.road_powered)(old, rt) {
                        self.do_convert(nested, ret, flags, tile, tile, rt, false);
                        if !ret.ok() {
                            cost.take(ret);
                            return;
                        }
                        cost.add_money(ret.money());
                    } else {
                        cost.error(INVALID_STRING_ID);
                        return;
                    }
                }
            }
        }
        let mut num = if !clear && self.tile_type(tile) == MP_TUNNELBRIDGE {
            self.length(tile, (self.rail.other_tunnel_bridge_end)(tile))
                .wrapping_add(2)
                .wrapping_mul(2)
        } else {
            pieces.count_ones()
        };
        cost.add_money(
            crate::road_cost::build_cost(self.shared, rt).saturating_mul(i64::from(num)),
        );
        if flags & DoCommandFlag::EXECUTE != 0 {
            match self.tile_type(tile) {
                MP_ROAD => {
                    let kind = self.kind(tile);
                    if existing == 0 || kind == 1 {
                        (self.shared.set_road_type)(tile, rtt, rt);
                        (self.shared.set_road_owner)(tile, rtt, company);
                        if rtt == 0 {
                            (self.s.set_town)(tile, town_id);
                        }
                    }
                    if kind != 1 {
                        (self.shared.set_road_bits)(tile, existing | pieces, rtt);
                    }
                }
                MP_TUNNELBRIDGE => {
                    w::hit(w::ROAD_COMMANDS_BUILD_WORMHOLE);
                    let end = (self.rail.other_tunnel_bridge_end)(tile);
                    (self.shared.set_road_type)(end, rtt, rt);
                    (self.shared.set_road_type)(tile, rtt, rt);
                    (self.shared.set_road_owner)(end, rtt, company);
                    (self.shared.set_road_owner)(tile, rtt, company);
                    if (self.shared.is_bridge)(tile) {
                        (self.shared.mark_bridge_dirty)(tile);
                    } else {
                        (self.shared.mark_dirty)(end);
                        (self.shared.mark_dirty)(tile);
                    }
                }
                MP_STATION => {
                    w::hit(w::ROAD_COMMANDS_BUILD_STOP);
                    debug_assert!((self.shared.is_drive_through_stop)(tile));
                    (self.shared.set_road_type)(tile, rtt, rt);
                    (self.shared.set_road_owner)(tile, rtt, company);
                }
                _ => (self.r.make_road)(
                    tile,
                    CrossingRoad {
                        bits: pieces,
                        road_type: if rtt == 0 { rt } else { INVALID },
                        tram_type: if rtt == 1 { rt } else { INVALID },
                        town: town_id,
                        road_owner: company,
                        tram_owner: company,
                    },
                ),
            }
            if self.tile_type(tile) == MP_TUNNELBRIDGE {
                num = num.wrapping_mul(TUNNELBRIDGE_TRACKBIT_FACTOR);
            }
            self.infrastructure(rt, self.owner(tile, rtt), num as i32);
            if rtt == 0 && self.tile_type(tile) == MP_ROAD && (self.shared.is_normal_road)(tile) {
                existing |= pieces;
                (self.shared.set_road_directions)(
                    tile,
                    if straight(existing) {
                        (self.shared.disallowed_road_directions)(tile) ^ toggle
                    } else {
                        0
                    },
                );
            }
            (self.shared.mark_dirty)(tile);
        }
    }
    fn connect(&self, tile: u32, rt: u8, dir: u8) -> bool {
        let tile = self.map.add_diag(tile, dir);
        if !(self.shared.tile_valid)(tile) || !self.may_have(tile) {
            return false;
        }
        let rtt = u8::from((self.shared.road_tram_type)(rt).0);
        let old = self.rt(tile, rtt);
        if old == INVALID
            || (!(self.shared.road_powered)(old, rt) && !(self.shared.road_powered)(rt, old))
        {
            return false;
        }
        (self.shared.road_bits)(tile, rtt, false) & dir_bit(dir ^ 2) != 0
    }
    fn valid_drag(&self, start: u32, end: u32, rt: u8, axis: u8) -> bool {
        start < self.size()
            && (self.shared.road_type_valid)(rt)
            && axis < 2
            && (if axis == 0 {
                self.map.tile_y(start) == self.map.tile_y(end)
            } else {
                self.map.tile_x(start) == self.map.tile_x(end)
            })
    }
    fn build_long(
        &self,
        nested: *mut CommandScratch,
        cost: Cost<'_>,
        last: Cost<'_>,
        ret: Cost<'_>,
        flags: u16,
        end: u32,
        start: u32,
        rt: u8,
        axis: u8,
        mut drd: u8,
        mut start_half: bool,
        mut end_half: bool,
        ai: bool,
    ) {
        w::hit(w::ROAD_COMMANDS_BUILD_LONG);
        if !self.valid_drag(start, end, rt, axis) || drd >= 4 {
            cost.error(INVALID_STRING_ID);
            return;
        }
        let mut dir = if axis == 0 { 2 } else { 1 };
        if start > end || (start == end && start_half) {
            dir ^= 2;
            start_half = !start_half;
            end_half = !end_half;
            if drd == 1 || drd == 2 {
                drd ^= 3;
            }
        }
        if (drd == 1 || drd == 2) && (axis == 1) == (start == end && start_half == end_half) {
            drd ^= 3;
        }
        cost.set(0);
        last.error(INVALID_STRING_ID);
        let mut tile = start;
        let (mut bridge, mut tunnel, mut success) = (false, false, false);
        // SAFETY: the native extra result is distinct and retains its address
        // through this synchronous drag, including nested conversion/reentry.
        let build_ret = self.c(unsafe { (self.s.scratch_cost)(nested) });
        loop {
            let mut bits = axis_bits(axis);
            if !ai && start != end {
                if tile == end && !self.connect(tile, rt, dir) {
                    bits = dir_bit(dir ^ 2);
                } else if tile == start && !self.connect(tile, rt, dir ^ 2) {
                    bits = dir_bit(dir);
                }
            } else {
                if tile == end && !end_half {
                    bits &= dir_bit(dir ^ 2);
                }
                if tile == start && start_half {
                    bits &= dir_bit(dir);
                }
            }
            self.do_build(
                nested,
                ret,
                build_ret,
                flags,
                tile,
                bits,
                rt,
                drd,
                INVALID_TOWN,
            );
            if !ai && ret.message() == self.r.strings.already_built {
                success = true;
            }
            if !ret.ok() {
                last.take(ret);
                if last.message() != self.r.strings.already_built {
                    if ai {
                        cost.take(last);
                        return;
                    }
                    if success {
                        w::hit(w::ROAD_COMMANDS_DRAG_OBSTACLE);
                        break;
                    }
                }
            } else {
                success = true;
                if self.tile_type(tile) == MP_TUNNELBRIDGE {
                    if (self.shared.is_bridge)(tile) {
                        if !bridge || (self.shared.tunnel_bridge_direction)(tile) == dir {
                            cost.add_money(ret.money());
                        }
                        bridge = true;
                    } else {
                        if !tunnel || (self.shared.tunnel_bridge_direction)(tile) == dir {
                            cost.add_money(ret.money());
                        }
                        tunnel = true;
                    }
                } else {
                    cost.add_money(ret.money());
                }
            }
            if tile == end {
                break;
            }
            tile = self.map.add_diag(tile, dir);
        }
        if !success {
            cost.take(last);
        }
    }
    fn remove_long(
        &self,
        nested: *mut CommandScratch,
        cost: Cost<'_>,
        last: Cost<'_>,
        ret: Cost<'_>,
        scratch: Cost<'_>,
        flags: u16,
        mut end: u32,
        mut start: u32,
        rt: u8,
        axis: u8,
        mut start_half: bool,
        mut end_half: bool,
    ) -> i64 {
        w::hit(w::ROAD_COMMANDS_REMOVE_LONG);
        cost.set(0);
        if !self.valid_drag(start, end, rt, axis) {
            cost.error(INVALID_STRING_ID);
            return 0;
        }
        if start > end || (start == end && start_half) {
            core::mem::swap(&mut start, &mut end);
            core::mem::swap(&mut start_half, &mut end_half);
        }
        let available = crate::command::available_money(self.commands);
        let mut spent = 0_i64;
        let mut tile = start;
        last.error(INVALID_STRING_ID);
        let mut success = false;
        loop {
            let mut bits = axis_bits(axis);
            if tile == end && !end_half {
                bits &= 1 | 8;
            }
            if tile == start && start_half {
                bits &= 4 | 2;
            }
            if bits != 0 {
                let rtt = u8::from((self.shared.road_tram_type)(rt).0);
                self.remove(
                    ret,
                    scratch,
                    tile,
                    flags & !DoCommandFlag::EXECUTE,
                    bits,
                    rtt,
                    true,
                );
                if ret.ok() {
                    if flags & DoCommandFlag::EXECUTE != 0 {
                        spent = spent.saturating_add(ret.money());
                        if spent > 0 && spent > available {
                            w::hit(w::ROAD_COMMANDS_REMOVE_MONEY);
                            // Parent cost retains its partial executed result. The
                            // other three costs can be reused: we return immediately.
                            // SAFETY: native extra slot is distinct from parent slots.
                            let nested_ret = self.c(unsafe { (self.s.scratch_cost)(nested) });
                            self.command(ret, flags & !DoCommandFlag::EXECUTE, end, |f| {
                                self.remove_long(
                                    nested, ret, last, nested_ret, scratch, f, end, start, rt,
                                    axis, start_half, end_half,
                                )
                            });
                            return ret.money();
                        }
                        self.remove(scratch, last, tile, flags, bits, rtt, false);
                    }
                    cost.add_money(ret.money());
                    success = true;
                } else if last.message() != self.r.strings.owned_by
                    && last.message() != self.s.strings.authority
                {
                    last.take(ret);
                }
            }
            if tile == end {
                break;
            }
            tile = tile.wrapping_add(if axis == 0 { 1 } else { self.map.size_x() });
        }
        if !success {
            cost.take(last);
        }
        0
    }
    fn build_depot(&self, cost: Cost<'_>, ret: Cost<'_>, flags: u16, tile: u32, rt: u8, dir: u8) {
        w::hit(w::ROAD_COMMANDS_DEPOT_BUILD);
        if !(self.shared.road_type_valid)(rt) || dir >= 4 {
            cost.error(INVALID_STRING_ID);
            return;
        }
        cost.set(0);
        let slope = (self.shared.tile_slope)(tile);
        if slope != 0 {
            if !(self.shared.build_on_slopes)() || !(self.s.depot_slope)(dir, slope) {
                cost.error(self.r.strings.flat_land);
                return;
            }
            cost.add_money(price(57));
        }
        let mut rotate = false;
        if self.tile_type(tile) == MP_ROAD
            && (self.shared.is_road_depot)(tile)
            && rt
                == if self.rt(tile, 1) != INVALID {
                    self.rt(tile, 1)
                } else {
                    self.rt(tile, 0)
                }
        {
            self.tile_ownership(ret, tile);
            if !ret.ok() {
                cost.take(ret);
                return;
            }
            if dir == (self.shared.road_depot_direction)(tile) {
                cost.zero();
                return;
            }
            self.no_vehicle(ret, tile);
            if !ret.ok() {
                cost.take(ret);
                return;
            }
            rotate = true;
        }
        if !rotate {
            self.clear(ret, flags, tile);
            cost.add(ret);
            if !cost.ok() {
                return;
            }
            if (self.shared.bridge_above)(tile) {
                cost.error(self.r.strings.demolish_bridge);
                return;
            }
            if !(self.r.depot_available)() {
                cost.error(INVALID_STRING_ID);
                return;
            }
        }
        if flags & DoCommandFlag::EXECUTE != 0 {
            if rotate {
                w::hit(w::ROAD_COMMANDS_DEPOT_ROTATE);
                (self.s.depot_exit)(tile, dir);
            } else {
                let depot = (self.r.new_depot)(tile);
                (self.s.make_depot)(tile, (self.shared.current_company)(), depot, dir, rt);
                (self.r.depot_name)(depot);
                self.infrastructure(
                    rt,
                    (self.shared.current_company)(),
                    ROAD_DEPOT_TRACKBIT_FACTOR as i32,
                );
            }
            (self.shared.mark_dirty)(tile);
        }
        cost.add_money(price(6));
    }
    fn remove_depot(&self, cost: Cost<'_>, flags: u16, tile: u32) {
        w::hit(w::ROAD_COMMANDS_DEPOT_REMOVE);
        if (self.shared.current_company)() != OWNER_WATER {
            self.tile_ownership(cost, tile);
            if !cost.ok() {
                return;
            }
        }
        self.no_vehicle(cost, tile);
        if !cost.ok() {
            return;
        }
        if flags & DoCommandFlag::EXECUTE != 0 {
            let owner = (self.shared.tile_owner)(tile);
            if (self.shared.company_valid)(owner) {
                let mut rt = self.rt(tile, 0);
                if rt == INVALID {
                    rt = self.rt(tile, 1);
                }
                // The original skips INVALID_ROADTYPE only for the public update helper;
                // a depot is guaranteed to have one valid type here.
                (self.r.road_infrastructure)(owner, rt, ROAD_DEPOT_TRACKBIT_FACTOR.wrapping_neg());
                (self.commands.infrastructure_dirty)(owner);
            }
            (self.r.delete_depot)(tile);
            crate::ground::clear_square(tile, self.shared);
        }
        cost.set(price(31));
    }
    fn clear_tile(&self, cost: Cost<'_>, ret: Cost<'_>, scratch: Cost<'_>, flags: u16, tile: u32) {
        w::hit(w::ROAD_COMMANDS_CLEAR);
        match self.kind(tile) {
            0 => {
                let bits = self.bits(tile, 0) | self.bits(tile, 1);
                if (bits.is_power_of_two() && self.bits(tile, 1) == 0)
                    || flags & DoCommandFlag::AUTO == 0
                {
                    cost.set(0);
                    for rtt in [0, 1] {
                        if !self.may_have(tile) || self.rt(tile, rtt) == INVALID {
                            continue;
                        }
                        self.remove(ret, scratch, tile, flags, self.bits(tile, rtt), rtt, true);
                        if !ret.ok() {
                            cost.take(ret);
                            return;
                        }
                        cost.add_money(ret.money());
                    }
                } else {
                    cost.error(self.s.strings.remove_road);
                }
            }
            1 => {
                cost.set(0);
                if flags & DoCommandFlag::AUTO != 0 {
                    cost.error(self.s.strings.remove_road);
                    return;
                }
                for rtt in [1, 0] {
                    if !self.may_have(tile) || self.rt(tile, rtt) == INVALID {
                        continue;
                    }
                    self.remove(
                        ret,
                        scratch,
                        tile,
                        flags,
                        (self.shared.crossing_road_bits)(tile).0,
                        rtt,
                        true,
                    );
                    if !ret.ok() {
                        cost.take(ret);
                        return;
                    }
                    cost.add_money(ret.money());
                }
                if flags & DoCommandFlag::EXECUTE != 0 {
                    self.clear(ret, flags, tile);
                }
            }
            _ => {
                if flags & DoCommandFlag::AUTO != 0 {
                    cost.error(self.r.strings.demolish_building);
                    return;
                }
                self.remove_depot(cost, flags, tile);
            }
        }
    }
    fn nearest_towns(&self, invalidate: bool) {
        for tile in 0..self.size() {
            let read = (self.s.nearest_tile)(tile);
            if read.road && !read.depot && !read.town_owned {
                if invalidate {
                    (self.s.set_town)(tile, INVALID_TOWN);
                } else {
                    self.update_town(tile);
                }
            }
        }
    }
    fn change_owner(&self, ret: Cost<'_>, scratch: Cost<'_>, tile: u32, old: u8, new: u8) {
        w::hit(w::ROAD_COMMANDS_OWNER);
        if (self.shared.is_road_depot)(tile) {
            if (self.shared.tile_owner)(tile) == old {
                if new == INVALID_OWNER {
                    self.clear(ret, DoCommandFlag::EXECUTE | DoCommandFlag::BANKRUPT, tile);
                } else {
                    let mut rt = self.rt(tile, 0);
                    if rt == INVALID {
                        rt = self.rt(tile, 1);
                    }
                    (self.r.road_infrastructure)(
                        old,
                        rt,
                        ROAD_DEPOT_TRACKBIT_FACTOR.wrapping_neg(),
                    );
                    (self.r.road_infrastructure)(new, rt, ROAD_DEPOT_TRACKBIT_FACTOR);
                    (self.shared.set_tile_owner)(tile, new);
                    for rtt in [0, 1] {
                        if self.owner(tile, rtt) == old {
                            (self.shared.set_road_owner)(tile, rtt, new);
                        }
                    }
                }
            }
            return;
        }
        for rtt in [0, 1] {
            if self.owner(tile, rtt) == old {
                let rt = self.rt(tile, rtt);
                if rt != INVALID {
                    let num = if (self.shared.is_level_crossing)(tile) {
                        2
                    } else {
                        self.bits(tile, rtt).count_ones()
                    };
                    (self.r.road_infrastructure)(old, rt, num.wrapping_neg());
                    if new != INVALID_OWNER {
                        (self.r.road_infrastructure)(new, rt, num);
                    }
                }
                (self.shared.set_road_owner)(
                    tile,
                    rtt,
                    if new == INVALID_OWNER {
                        OWNER_NONE
                    } else {
                        new
                    },
                );
            }
        }
        if (self.shared.is_level_crossing)(tile) && (self.shared.tile_owner)(tile) == old {
            if new == INVALID_OWNER {
                let track = crate::ground::crossing_rail_track(tile, self.rail).0;
                self.command(
                    ret,
                    DoCommandFlag::EXECUTE | DoCommandFlag::BANKRUPT,
                    tile,
                    |flags| {
                        // SAFETY: the outer owner-change frame owns both native results.
                        unsafe {
                            crate::rail_cmd::openttd_rust_rail_remove_single(
                                self.s.rail_commands,
                                ret.0,
                                scratch.0,
                                flags,
                                tile,
                                track,
                            );
                        }
                        0
                    },
                );
            } else {
                let rt = (self.rail.rail_type)(tile).0;
                (self.r.rail_infrastructure)(old, rt, LEVELCROSSING_TRACKBIT_FACTOR.wrapping_neg());
                (self.r.rail_infrastructure)(new, rt, LEVELCROSSING_TRACKBIT_FACTOR);
                (self.shared.set_tile_owner)(tile, new);
            }
        }
    }
    fn terraform(
        &self,
        cost: Cost<'_>,
        scratch: Cost<'_>,
        flags: u16,
        tile: u32,
        mut z: i32,
        mut slope: u8,
    ) {
        w::hit(w::ROAD_COMMANDS_TERRAFORM);
        if (self.shared.build_on_slopes)() && (self.shared.autoslope)() {
            let valid = match self.kind(tile) {
                1 => {
                    !is_steep_slope(slope)
                        && (self.shared.tile_max_z)(tile) == z + get_slope_max_z(slope)
                        && CROSSING_SLOPES & (1 << slope) != 0
                }
                2 => {
                    (self.r.entrance_edge)(tile, z, slope, (self.shared.road_depot_direction)(tile))
                }
                0 => {
                    let bits = self.bits(tile, 0) | self.bits(tile, 1);
                    let mut copy = bits;
                    self.slope(scratch, slope, &mut copy, 0, 0);
                    if scratch.ok() && bits == copy {
                        let mut old_slope = (self.shared.tile_slope)(tile);
                        let mut old_z = (self.shared.tile_z)(tile);
                        let old = (self.shared.apply_foundation)(
                            (self.s.foundation)(old_slope, bits),
                            Slope(old_slope),
                        );
                        old_z += old.z;
                        old_slope = old.slope.0;
                        let new = (self.shared.apply_foundation)(
                            (self.s.foundation)(slope, bits),
                            Slope(slope),
                        );
                        z += new.z;
                        slope = new.slope.0;
                        old_z == z && old_slope == slope
                    } else {
                        false
                    }
                }
                _ => unreachable!(),
            };
            if valid {
                w::hit(w::ROAD_COMMANDS_AUTOSLOPE);
                cost.set(price(57));
                return;
            }
        }
        self.clear(cost, flags, tile);
    }
    fn convert_owner(&self, tile: u32, num: u32, owner: u8, from: u8, to: u8) {
        let company = (self.shared.current_company)();
        if u32::from(company) >= MAX_COMPANIES
            || (u32::from(owner) >= MAX_COMPANIES && owner != OWNER_NONE)
        {
            return;
        }
        if owner == OWNER_NONE {
            (self.shared.set_road_owner)(
                tile,
                u8::from((self.shared.road_tram_type)(to).0),
                company,
            );
            self.infrastructure(to, company, num as i32);
        } else {
            (self.r.road_infrastructure)(owner, from, num.wrapping_neg());
            (self.r.road_infrastructure)(owner, to, num);
            (self.commands.infrastructure_dirty)(owner);
        }
    }
    fn convert(
        &self,
        frame: *mut ConvertFrame,
        area: *mut Area,
        cost: Cost<'_>,
        error: Cost<'_>,
        ret: Cost<'_>,
        flags: u16,
        end: u32,
        start: u32,
        to: u8,
        diagonal: bool,
    ) {
        w::hit(w::ROAD_COMMANDS_CONVERT);
        if !(self.shared.road_type_valid)(to) || start >= self.size() {
            cost.error(INVALID_STRING_ID);
            return;
        }
        let rtt = u8::from((self.shared.road_tram_type)(to).0);
        cost.set(0);
        error.error(if rtt == 1 {
            self.s.strings.no_suitable_tram
        } else {
            self.s.strings.no_suitable_road
        });
        let mut found = false;
        unsafe {
            (self.r.area_begin)(area, start, end, diagonal);
        }
        loop {
            let tile = unsafe { (self.r.area_tile)(area) };
            if tile == u32::MAX {
                break;
            }
            // Advance at the bottom even when this tile fails, preserving the native iterator order.
            self.convert_tile(
                frame, cost, error, ret, flags, tile, start, end, to, rtt, diagonal, &mut found,
            );
            unsafe {
                (self.r.area_next)(area);
            }
        }
        if flags & DoCommandFlag::EXECUTE != 0 {
            unsafe {
                (self.s.cargo_changed)(frame);
            }
        }
        if !found {
            cost.take(error);
        }
    }
    fn convert_tile(
        &self,
        frame: *mut ConvertFrame,
        cost: Cost<'_>,
        error: Cost<'_>,
        ret: Cost<'_>,
        flags: u16,
        tile: u32,
        start: u32,
        end: u32,
        to: u8,
        rtt: u8,
        diagonal: bool,
        found: &mut bool,
    ) {
        if !self.may_have(tile) {
            return;
        }
        let from = self.rt(tile, rtt);
        if from == INVALID || from == to {
            return;
        }
        let tt = self.tile_type(tile);
        match tt {
            MP_STATION => {
                if !(self.shared.is_any_road_stop)(tile) {
                    return;
                }
            }
            MP_ROAD => {
                if (self.shared.is_level_crossing)(tile) && (self.r.road_no_crossing)(to) {
                    error.make_error(self.r.strings.crossing_road);
                    return;
                }
            }
            MP_TUNNELBRIDGE => {
                if (self.shared.tunnel_transport)(tile) != 1 {
                    return;
                }
            }
            _ => return,
        }
        let owner = self.owner(tile, rtt);
        if !can_convert_unowned_road_type(owner, rtt) {
            self.ownership(ret, owner, tile);
            if !ret.ok() {
                error.take(ret);
                return;
            }
        }
        if owner == OWNER_TOWN {
            let town =
                (self.shared.closest_town)(tile, u32::from((self.authority.authority_distance)()));
            self.rating(ret, town, 0, u8::from(tt == MP_TUNNELBRIDGE));
            if !ret.ok() {
                error.take(ret);
                return;
            }
        }
        if tt != MP_TUNNELBRIDGE {
            if !(self.shared.road_powered)(from, to) {
                self.no_vehicle(ret, tile);
                if !ret.ok() {
                    error.take(ret);
                    return;
                }
                if rtt == 0 && owner == OWNER_TOWN {
                    unsafe {
                        (self.s.owned_by_town)(error.0, tile);
                    }
                    return;
                }
            }
            let mut num = self.any_bits(tile, rtt).count_ones();
            if tt == MP_STATION && (self.shared.is_bay_stop)(tile) {
                num = num.wrapping_mul(ROAD_STOP_TRACKBIT_FACTOR);
            } else if tt == MP_ROAD && (self.shared.is_road_depot)(tile) {
                num = num.wrapping_mul(ROAD_DEPOT_TRACKBIT_FACTOR);
            }
            *found = true;
            cost.add_money(
                crate::road_cost::convert_cost(self.shared, from, to)
                    .saturating_mul(i64::from(num)),
            );
            if flags & DoCommandFlag::EXECUTE != 0 {
                w::hit(w::ROAD_COMMANDS_CONVERT_TILE);
                if owner == (self.shared.current_company)() {
                    self.convert_owner(tile, num, owner, from, to);
                }
                (self.shared.set_road_type)(tile, rtt, to);
                (self.shared.mark_dirty)(tile);
                unsafe {
                    (self.s.collect_vehicles)(frame, tile);
                }
                if self.tile_type(tile) == MP_ROAD && (self.shared.is_road_depot)(tile) {
                    (self.r.depot_windows)(tile);
                }
            }
        } else {
            let other = (self.rail.other_tunnel_bridge_end)(tile);
            if other < tile && (self.r.area_contains)(start, end, diagonal, other) {
                return;
            }
            if !(self.shared.road_powered)(from, to) {
                self.free(ret, tile, other);
                if !ret.ok() {
                    error.take(ret);
                    return;
                }
                if rtt == 0 && owner == OWNER_TOWN {
                    unsafe {
                        (self.s.owned_by_town)(error.0, tile);
                    }
                    return;
                }
            }
            let num = self.length(tile, other).wrapping_add(2).wrapping_mul(2);
            *found = true;
            cost.add_money(
                crate::road_cost::convert_cost(self.shared, from, to)
                    .saturating_mul(i64::from(num)),
            );
            if flags & DoCommandFlag::EXECUTE != 0 {
                w::hit(w::ROAD_COMMANDS_CONVERT_WORMHOLE);
                if owner == (self.shared.current_company)() {
                    self.convert_owner(
                        tile,
                        num.wrapping_mul(TUNNELBRIDGE_TRACKBIT_FACTOR),
                        owner,
                        from,
                        to,
                    );
                    crate::ground::set_tunnel_bridge_owner(
                        tile,
                        other,
                        (self.shared.current_company)(),
                        self.shared,
                    );
                }
                (self.shared.set_road_type)(tile, rtt, to);
                (self.shared.set_road_type)(other, rtt, to);
                unsafe {
                    (self.s.collect_vehicles)(frame, tile);
                    (self.s.collect_vehicles)(frame, other);
                }
                self.dirty_wormhole(tile, other);
            }
        }
    }
    fn build_above(&self, cost: Cost<'_>, flags: u16, tile: u32) {
        cost.zero();
        if (self.shared.is_road_depot)(tile) {
            w::hit(w::ROAD_COMMANDS_BUILD_ABOVE);
            self.clear(cost, flags, tile);
        }
    }
}

/// Original road `allow_remove` entry.
/// # Safety
/// Immutable tables and live native cost/frame slots outlive this synchronous
/// invocation. Slots are distinct; no native borrow survives reentrant calls.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_allow_remove(
    services: *const RoadCommandServices,
    out: *mut CommandCost,
    tile: u32,
    bits: u8,
    owner: u8,
    rtt: u8,
    flags: u16,
    town_check: bool,
) {
    // SAFETY: caller supplies valid immutable native tables.
    let c = unsafe { Ctx::new(services) };
    c.allow_remove(c.c(out), tile, bits, owner, rtt, flags, town_check);
}

/// Original road `build` entry.
/// # Safety
/// Immutable tables and live native cost/frame slots outlive this synchronous
/// invocation. Slots are distinct; no native borrow survives reentrant calls.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_build(
    services: *const RoadCommandServices,
    nested: *mut CommandScratch,
    out: *mut CommandCost,
    ret: *mut CommandCost,
    flags: u16,
    tile: u32,
    bits: u8,
    rt: u8,
    drd: u8,
    town: u16,
) {
    // SAFETY: caller supplies valid immutable native tables.
    let c = unsafe { Ctx::new(services) };
    c.build(nested, c.c(out), c.c(ret), flags, tile, bits, rt, drd, town);
}

/// Shared ordinary road-build command for native callers, including town growth.
/// # Safety
/// Same caller-owned distinct native slots and synchronous lifetime as `road_build`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_do_build(
    services: *const RoadCommandServices,
    nested: *mut CommandScratch,
    out: *mut CommandCost,
    ret: *mut CommandCost,
    flags: u16,
    tile: u32,
    bits: u8,
    rt: u8,
    drd: u8,
    town: u16,
) {
    // SAFETY: native caller supplies the immutable tables and live stack slots.
    let c = unsafe { Ctx::new(services) };
    c.do_build(nested, c.c(out), c.c(ret), flags, tile, bits, rt, drd, town);
}

/// Original road `build_long` entry.
/// # Safety
/// Immutable tables and live native cost/frame slots outlive this synchronous
/// invocation. Slots are distinct; no native borrow survives reentrant calls.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_build_long(
    services: *const RoadCommandServices,
    nested: *mut CommandScratch,
    out: *mut CommandCost,
    last: *mut CommandCost,
    ret: *mut CommandCost,
    flags: u16,
    end: u32,
    start: u32,
    rt: u8,
    axis: u8,
    drd: u8,
    start_half: bool,
    end_half: bool,
    ai: bool,
) {
    // SAFETY: caller supplies valid immutable native tables.
    let c = unsafe { Ctx::new(services) };
    c.build_long(
        nested,
        c.c(out),
        c.c(last),
        c.c(ret),
        flags,
        end,
        start,
        rt,
        axis,
        drd,
        start_half,
        end_half,
        ai,
    );
}

/// Original road `remove_long` entry.
/// # Safety
/// Immutable tables and live native cost/frame slots outlive this synchronous
/// invocation. Slots are distinct; no native borrow survives reentrant calls.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_remove_long(
    services: *const RoadCommandServices,
    nested: *mut CommandScratch,
    out: *mut CommandCost,
    last: *mut CommandCost,
    ret: *mut CommandCost,
    scratch: *mut CommandCost,
    flags: u16,
    end: u32,
    start: u32,
    rt: u8,
    axis: u8,
    start_half: bool,
    end_half: bool,
) -> i64 {
    // SAFETY: caller supplies valid immutable native tables.
    let c = unsafe { Ctx::new(services) };
    c.remove_long(
        nested,
        c.c(out),
        c.c(last),
        c.c(ret),
        c.c(scratch),
        flags,
        end,
        start,
        rt,
        axis,
        start_half,
        end_half,
    )
}

/// Original road `build_depot` entry.
/// # Safety
/// Immutable tables and live native cost/frame slots outlive this synchronous
/// invocation. Slots are distinct; no native borrow survives reentrant calls.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_build_depot(
    services: *const RoadCommandServices,
    out: *mut CommandCost,
    ret: *mut CommandCost,
    flags: u16,
    tile: u32,
    rt: u8,
    dir: u8,
) {
    // SAFETY: caller supplies valid immutable native tables.
    let c = unsafe { Ctx::new(services) };
    c.build_depot(c.c(out), c.c(ret), flags, tile, rt, dir);
}

/// Original road `clear` entry.
/// # Safety
/// Immutable tables and live native cost/frame slots outlive this synchronous
/// invocation. Slots are distinct; no native borrow survives reentrant calls.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_clear(
    services: *const RoadCommandServices,
    out: *mut CommandCost,
    ret: *mut CommandCost,
    scratch: *mut CommandCost,
    flags: u16,
    tile: u32,
) {
    // SAFETY: caller supplies valid immutable native tables.
    let c = unsafe { Ctx::new(services) };
    c.clear_tile(c.c(out), c.c(ret), c.c(scratch), flags, tile);
}

/// Original road `nearest_towns` entry.
/// # Safety
/// Immutable tables and live native cost/frame slots outlive this synchronous
/// invocation. Slots are distinct; no native borrow survives reentrant calls.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_nearest_towns(
    services: *const RoadCommandServices,
    invalidate: bool,
) {
    // SAFETY: caller supplies valid immutable native tables.
    let c = unsafe { Ctx::new(services) };
    c.nearest_towns(invalidate);
}

/// Original road `change_owner` entry.
/// # Safety
/// Immutable tables and live native cost/frame slots outlive this synchronous
/// invocation. Slots are distinct; no native borrow survives reentrant calls.
pub(crate) unsafe fn openttd_rust_road_change_owner(
    services: *const RoadCommandServices,
    ret: *mut CommandCost,
    scratch: *mut CommandCost,
    tile: u32,
    old: u8,
    new: u8,
) {
    // SAFETY: caller supplies valid immutable native tables.
    let c = unsafe { Ctx::new(services) };
    c.change_owner(c.c(ret), c.c(scratch), tile, old, new);
}

/// Original road `terraform` entry.
/// # Safety
/// Immutable tables and live native cost/frame slots outlive this synchronous
/// invocation. Slots are distinct; no native borrow survives reentrant calls.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_terraform(
    services: *const RoadCommandServices,
    out: *mut CommandCost,
    scratch: *mut CommandCost,
    flags: u16,
    tile: u32,
    z: i32,
    slope: u8,
) {
    // SAFETY: caller supplies valid immutable native tables.
    let c = unsafe { Ctx::new(services) };
    c.terraform(c.c(out), c.c(scratch), flags, tile, z, slope);
}

/// Original road `convert` entry.
/// # Safety
/// Immutable tables and live native cost/frame slots outlive this synchronous
/// invocation. Slots are distinct; no native borrow survives reentrant calls.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_convert(
    services: *const RoadCommandServices,
    frame: *mut ConvertFrame,
    area: *mut Area,
    out: *mut CommandCost,
    error: *mut CommandCost,
    ret: *mut CommandCost,
    flags: u16,
    end: u32,
    start: u32,
    rt: u8,
    diagonal: bool,
) {
    // SAFETY: caller supplies valid immutable native tables.
    let c = unsafe { Ctx::new(services) };
    c.convert(
        frame,
        area,
        c.c(out),
        c.c(error),
        c.c(ret),
        flags,
        end,
        start,
        rt,
        diagonal,
    );
}

/// Original road `build_above` entry.
/// # Safety
/// Immutable tables and live native cost/frame slots outlive this synchronous
/// invocation. Slots are distinct; no native borrow survives reentrant calls.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_build_above(
    services: *const RoadCommandServices,
    out: *mut CommandCost,
    flags: u16,
    tile: u32,
) {
    // SAFETY: caller supplies valid immutable native tables.
    let c = unsafe { Ctx::new(services) };
    c.build_above(c.c(out), flags, tile);
}

/// Removal entry also used by the native road tile loop pending #212.
/// # Safety
/// Immutable tables and distinct live cost slots outlive this synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_remove(
    services: *const RoadCommandServices,
    out: *mut CommandCost,
    ret: *mut CommandCost,
    flags: u16,
    tile: u32,
    pieces: u8,
    rtt: u8,
    town_check: bool,
) {
    // SAFETY: caller supplies valid tables and source-valid map inputs.
    let c = unsafe { Ctx::new(services) };
    c.remove(c.c(out), c.c(ret), tile, flags, pieces, rtt, town_check);
}

/// Shared `UpdateCompanyRoadInfrastructure`, also called directly by bridge policy.
/// # Safety
/// Native immutable services are live under the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_update_road_infrastructure(
    s: *const RoadCommandServices,
    rt: u8,
    owner: u8,
    count: i32,
) {
    // SAFETY: native callers supply the process-lifetime canonical table.
    unsafe {
        update_company_road_infrastructure(s, rt, owner, count);
    }
}
pub(crate) unsafe fn update_company_road_infrastructure(
    s: *const RoadCommandServices,
    rt: u8,
    owner: u8,
    count: i32,
) {
    // SAFETY: only immutable tables are borrowed; no map/pool borrow survives calls.
    let s = unsafe { &*s };
    let rail = unsafe { &*s.rail_commands };
    let shared = unsafe { &*rail.shared };
    if rt == INVALID || !(shared.company_valid)(owner) {
        return;
    }
    (rail.road_infrastructure)(owner, rt, count as u32);
    let commands = unsafe { &*rail.command };
    (commands.infrastructure_dirty)(owner);
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRoadNearestTownTile" ["probe"] => NearestTownTile { road, depot, town_owned, };
    "OpenTTDRoadCommandContext" ["probe"] => CommandContext {
        services,
        scratch,
        cost,
        ret,
    };
    "OpenTTDRoadCommandServices" ["probe"] => RoadCommandServices {
        do_build,
        rail_commands,
        town,
        authority,
        strings,
        nearest_tile,
        town_valid,
        town_edge,
        set_town,
        foundation,
        extra_dynamite,
        depot_slope,
        make_depot,
        depot_exit,
        next_effect,
        delete_effect,
        owned_by_town,
        scratch_cost,
        conversion_scratch,
        collect_vehicles,
        cargo_changed,
    };
    "OpenTTDRoadConversionScratch" ["probe"] => ConversionScratch {
        frame,
        area,
        error,
        ret,
    };
    "OpenTTDRoadTownEdge" ["probe"] => TownEdge {
        tile,
        radius,
    };
    "OpenTTDRoadWorksEffect" ["probe"] => WorksEffect {
        id,
        tile,
    };
    "OpenTTDRoadCommandStrings" ["probe"] => Strings {
        no_road,
        no_tram,
        junction,
        remove_road,
        no_suitable_road,
        no_suitable_tram,
        authority,
    };
}
