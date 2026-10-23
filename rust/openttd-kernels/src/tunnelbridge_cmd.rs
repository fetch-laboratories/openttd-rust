/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete tunnel/bridge construction, removal and tile ownership policy.
//! Native tables and result slots are borrowed synchronously under the game-state
//! mutex; no map/pool or owner borrow crosses a callback. Panics abort, and native
//! noexcept leaves have no ordinary-play throwing service.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::if_not_else
)]
use crate::bridge::{MAX_BRIDGES, calc_bridge_len_cost_factor, get_bridge_foundation};
use crate::command_type::DoCommandFlag;
use crate::company::price;
use crate::company_type::{INVALID_OWNER, OWNER_DEITY, OWNER_NONE, OWNER_TOWN, OWNER_WATER};
use crate::direction_func::{axis_to_diag_dir, diag_dir_to_axis, reverse_diag_dir};
use crate::direction_type::{DIAGDIR_NE, DIAGDIR_NW, DIAGDIR_SE, DIAGDIR_SW, INVALID_DIAGDIR};
use crate::economy_type::{
    PR_BUILD_AQUEDUCT, PR_BUILD_BRIDGE, PR_BUILD_FOUNDATION, PR_BUILD_TUNNEL, PR_CLEAR_AQUEDUCT,
    PR_CLEAR_BRIDGE, PR_CLEAR_TUNNEL,
};
use crate::services::{
    Axis, CommandCost, Cost, DiagDirection, Foundation, Map, RailType, RoadType, Services, Slope,
    TownHandle, Train, TransportType,
};
use crate::slope_func::{
    FOUNDATION_NONE, SLOPE_FLAT, SLOPE_NE, SLOPE_NW, SLOPE_SE, SLOPE_SW, complement_slope,
    get_inclined_slope_direction,
};
use crate::strings_type::INVALID_STRING_ID;
use crate::tile_type::{
    MP_CLEAR, MP_OBJECT, MP_RAILWAY, MP_ROAD, MP_STATION, MP_TUNNELBRIDGE, MP_WATER,
};
use crate::track_func::{axis_to_track, diag_dir_to_diag_track};
use crate::tunnelbridge::{TUNNELBRIDGE_TRACKBIT_FACTOR, get_tunnel_bridge_length};
use crate::witness as w;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::road_map::{ROADSIDE_PAVED, ROADSIDE_TREES};
use crate::road_type::INVALID_ROADTYPE;
use crate::town_type::{
    RATING_MAXIMUM, RATING_TUNNEL_BRIDGE_DOWN_STEP, RATING_TUNNEL_BRIDGE_MINIMUM,
    RATING_TUNNEL_BRIDGE_UP_STEP,
};
use crate::transport_type::{
    TRANSPORT_RAIL as RAIL, TRANSPORT_ROAD as ROAD, TRANSPORT_WATER as WATER,
};
// Process-wide hidden command output, also read from the GUI thread under the
// game-state mutex. Atomic storage preserves that scope without Rust references.
static END_TILE: AtomicU32 = AtomicU32::new(0);

#[repr(C)]
pub struct Availability {
    pub year: i32,
    pub min: u8,
    pub max: u16,
}
#[repr(C)]
pub struct Strings {
    pub bridge_border: u32,
    pub map_edge: u32,
    pub same_end: u32,
    pub straight: u32,
    pub too_long: u32,
    pub wrong_slope: u32,
    pub unequal_height: u32,
    pub demolish_bridge: u32,
    pub already_built: u32,
    pub authority: u32,
    pub other_owner: u32,
    pub too_low: u32,
    pub too_high: u32,
    pub tunnel_site: u32,
    pub water: u32,
    pub tunnel_border: u32,
    pub tunnel_way: u32,
    pub tunnel_long: u32,
    pub excavate: u32,
    pub demolish_tunnel: u32,
}
#[repr(C)]
pub struct TunnelBridgeServices {
    pub rail: *const crate::rail_cmd::RailCommandServices,
    pub road: *const crate::road_cmd::RoadCommandServices,
    pub road_ground: *const crate::ground::RoadServices,
    pub water: *const crate::water_cmd::WaterCommandServices,
    pub company: *const crate::company::Services,
    pub availability: extern "C" fn(u32) -> Availability,
    pub speed: extern "C" fn(u32) -> u16,
    pub price: extern "C" fn(u32) -> u16,
    pub max_bridge_length: extern "C" fn() -> u16,
    pub max_bridge_height: extern "C" fn() -> u8,
    pub max_tunnel_length: extern "C" fn() -> u16,
    pub crossing_tunnels: extern "C" fn() -> bool,
    pub other_bridge_end: extern "C" fn(u32) -> u32,
    pub northern_bridge_end: extern "C" fn(u32) -> u32,
    pub bridge_type: extern "C" fn(u32) -> u32,
    pub is_tunnel: extern "C" fn(u32) -> bool,
    pub tunnel_in_way: extern "C" fn(u32, i32, crate::services::DiagDirection) -> bool,
    pub set_middle: extern "C" fn(u32, crate::services::Axis),
    pub clear_middle: extern "C" fn(u32),
    pub set_reservation: extern "C" fn(u32, bool),
    pub make_rail_bridge:
        extern "C" fn(u32, u8, u32, crate::services::DiagDirection, crate::services::RailType),
    pub make_road_bridge: extern "C" fn(
        u32,
        u8,
        u8,
        u8,
        u32,
        crate::services::DiagDirection,
        crate::services::RoadType,
        crate::services::RoadType,
    ),
    pub make_aqueduct: extern "C" fn(u32, u8, crate::services::DiagDirection),
    pub make_rail_tunnel:
        extern "C" fn(u32, u8, crate::services::DiagDirection, crate::services::RailType),
    pub make_road_tunnel: extern "C" fn(
        u32,
        u8,
        crate::services::DiagDirection,
        crate::services::RoadType,
        crate::services::RoadType,
    ),
    pub mark_bridge: extern "C" fn(u32, u32, crate::services::DiagDirection, u32),
    pub mark_dirty_height: extern "C" fn(u32, i32),
    pub terraform: extern "C" fn(*mut CommandCost, u16, u32, u8),
    pub objects: *const crate::objects::ObjectServices,
    pub strings: Strings,
}
struct World<'a> {
    a: &'a TunnelBridgeServices,
    r: &'a crate::rail_cmd::RailCommandServices,
    s: &'a Services,
    rail: &'a crate::services::RailMapServices,
    c: &'a crate::services::CommandCostServices,
    cmd: &'a crate::services::CommandServices,
    road: &'a crate::road_cmd::RoadCommandServices,
    town: &'a crate::town::Leaves,
    authority: &'a crate::town::AuthorityServices,
    stations: &'a crate::station_cmd::CommandServicesTable,
    map: Map,
}
impl<'a> World<'a> {
    unsafe fn new(a: *const TunnelBridgeServices) -> Self {
        // SAFETY: the outer facade supplies immutable process-lifetime tables;
        // only tables are borrowed, never map/pool or command scratch state.
        unsafe {
            let a = &*a;
            let r = &*a.rail;
            let s = &*r.shared;
            let road = &*a.road;
            Self {
                a,
                r,
                s,
                rail: &*r.rail,
                c: &*r.cost,
                cmd: &*r.command,
                road,
                town: &*road.town,
                authority: &*road.authority,
                stations: &*(s.station_commands)(),
                map: Map {
                    log_x: (s.map_log_x)() as u8,
                },
            }
        }
    }
    fn cost(&self, p: *mut CommandCost) -> Cost<'a> {
        Cost(p, self.c)
    }
    fn error(out: Cost<'_>) {
        out.error(INVALID_STRING_ID);
    }
    fn length(&self, start: u32, end: u32) -> u32 {
        get_tunnel_bridge_length(self.map, start, end)
    }
    fn slope_z(&self, tile: u32) -> (u8, i32) {
        // SAFETY: immutable water table; no world borrow crosses the call.
        let value = (self.s.tile_slope_z)(tile);
        (value.slope.0, value.z)
    }
    fn editor(&self) -> bool {
        (self.town.environment.editor)()
    }
    fn bridge_height(&self, tile: u32) -> i32 {
        crate::bridge_map::get_bridge_height(tile, self.s)
    }
    fn clear_command(&self, out: Cost<'_>, tile: u32, flags: u16) {
        crate::landscape_cmd::clear_command(self.cmd, out.0, tile, flags);
    }
    fn ownership(&self, out: Cost<'_>, owner: u8, tile: u32) {
        // SAFETY: live native output slot, no world borrow.
        unsafe { (self.cmd.ownership)(out.0, owner, tile) };
    }
    fn signal(&self, tile: u32, direction: u8, owner: u8) {
        crate::signal::Context {
            shared: self.r.shared,
            rail: self.r.rail,
            native: self.r.signal,
            log_x: u32::from(self.map.log_x),
        }
        .add_side(tile, direction, owner);
    }
    fn check_docking(&self, tile: u32) {
        // SAFETY: direct integrated owner; all access scopes end before reentry.
        unsafe {
            crate::station_cmd::openttd_rust_station_check_docking(
                (self.s.station_commands)(),
                tile,
            );
        }
    }
    fn rating(&self, town: *const TownHandle, flags: u16, delta: i32, minimum: i32) {
        // SAFETY: shell identity is source-live. The rating owner ends access
        // before its native invalidation callbacks; no caller owner is borrowed.
        unsafe {
            crate::town::openttd_rust_town_change_rating(
                self.road.town,
                self.r.shared,
                if town.is_null() {
                    std::ptr::null_mut()
                } else {
                    (self.s.town_owner)(town)
                },
                if town.is_null() {
                    u32::from(u16::MAX)
                } else {
                    u32::from((self.authority.town_index)(town))
                },
                town as usize,
                (self.s.current_company)(),
                delta,
                minimum,
                flags & DoCommandFlag::NO_MODIFY_TOWN_RATING != 0,
                (self.authority.bulldozer)(),
            );
        }
    }
    fn rating_check(&self, out: Cost<'_>, town: *const TownHandle, flags: u16) {
        w::hit(w::TB_TOWN_RATING);
        // SAFETY: source permits a null town and the native slot is live.
        unsafe {
            crate::town::openttd_rust_town_authority_remove(
                self.road.authority,
                self.r.shared,
                self.r.cost,
                out.0,
                town,
                flags & DoCommandFlag::NO_TEST_TOWN_RATING != 0,
                1,
            );
        }
    }
    fn deity_owner(&self, tile: u32) -> u8 {
        w::hit(w::TB_DEITY);
        let town = (self.s.calc_closest_town)(tile, u32::MAX);
        if town.is_null() {
            return OWNER_NONE;
        }
        let edge = (self.road.town_edge)(town);
        if self.map.distance_square(tile, edge.tile) > edge.radius {
            OWNER_NONE
        } else {
            OWNER_TOWN
        }
    }
    fn available(&self, out: Cost<'_>, bridge_type: u32, length: u32, flags: u16) {
        out.zero();
        if flags & DoCommandFlag::QUERY_COST != 0 {
            if length > u32::from((self.a.max_bridge_length)()) {
                out.error(self.a.strings.too_long);
            }
            return;
        }
        if bridge_type >= MAX_BRIDGES {
            return Self::error(out);
        }
        let b = (self.a.availability)(bridge_type);
        if b.year > (self.s.calendar_year)() {
            return Self::error(out);
        }
        let maximum = b.max.min((self.a.max_bridge_length)());
        if u32::from(b.min) > length {
            return Self::error(out);
        }
        if length > u32::from(maximum) {
            out.error(self.a.strings.too_long);
        }
    }
    fn slope(&self, out: Cost<'_>, north: bool, axis: u8, slope: &mut u8, z: &mut i32) {
        let foundation = get_bridge_foundation(*slope, axis);
        let applied = (self.s.apply_foundation)(Foundation(foundation), Slope(*slope));
        *z = z.wrapping_add(applied.z);
        *slope = applied.slope.0;
        let valid = match (north, axis) {
            (true, 0) => SLOPE_NE,
            (true, _) => SLOPE_NW,
            (false, 0) => SLOPE_SW,
            (false, _) => SLOPE_SE,
        };
        if *slope != SLOPE_FLAT && *slope != valid {
            return Self::error(out);
        }
        if foundation == FOUNDATION_NONE {
            out.zero();
        } else {
            out.set(price(PR_BUILD_FOUNDATION));
        }
    }
    fn clear_cost(&self, tile: u32, base: usize) -> i64 {
        let mut result = price(base);
        match (self.s.tunnel_transport)(tile) {
            ROAD => {
                let road = (self.s.road_type)(tile);
                let tram = (self.s.tram_type)(tile).0;
                if road != INVALID_ROADTYPE {
                    result = result.saturating_add(
                        crate::road_cost::clear_cost(self.s, road).saturating_mul(2),
                    );
                }
                if tram != INVALID_ROADTYPE {
                    result = result.saturating_add(
                        crate::road_cost::clear_cost(self.s, tram).saturating_mul(2),
                    );
                }
            }
            RAIL => {
                result =
                    result.saturating_add((self.r.rail_clear_cost)((self.rail.rail_type)(tile).0));
            }
            WATER => result = price(PR_CLEAR_AQUEDUCT),
            _ => (),
        }
        result
    }
    fn build_above_tile(&self, out: Cost<'_>, tile: u32, flags: u16, axis: u8, height: i32) {
        // SAFETY: each named owner receives canonical tables and a live output;
        // callbacks may reenter without any Rust borrow of mutable world state.
        unsafe {
            match (self.s.tile_type)(tile) {
                MP_CLEAR => crate::ground::check_build_above_clear(out),
                MP_RAILWAY => {
                    crate::rail_cmd::openttd_rust_rail_build_above(self.a.rail, out.0, flags, tile);
                }
                MP_ROAD => {
                    crate::road_cmd::openttd_rust_road_build_above(self.a.road, out.0, flags, tile);
                }
                MP_STATION => crate::station_cmd::openttd_rust_station_build_above(
                    (self.s.station_commands)(),
                    out.0,
                    tile,
                    height,
                ),
                MP_WATER => crate::water_cmd::openttd_rust_water_build_above(
                    self.a.water,
                    out.0,
                    flags,
                    tile,
                    height,
                ),
                MP_TUNNELBRIDGE => self.build_above(out, tile, flags, axis, height),
                MP_OBJECT => crate::objects::openttd_rust_object_build_above(
                    self.a.objects,
                    out.0,
                    tile,
                    flags,
                    height,
                ),
                _ => self.clear_command(out, tile, flags),
            }
        }
    }
    fn build_bridge(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        north: Cost<'_>,
        south: Cost<'_>,
        flags: u16,
        mut end: u32,
        mut start: u32,
        transport: u8,
        bridge_type: u32,
        kind: u8,
    ) {
        w::hit(w::TB_BUILD_BRIDGE);
        let mut company = (self.s.current_company)();
        let mut railtype = (self.rail.invalid_rail_type)().0;
        let mut roadtype = INVALID_ROADTYPE;
        for tile in [start, end] {
            if !(self.s.tile_valid)(tile) {
                return cost.error(self.a.strings.bridge_border);
            }
            if self.map.tile_x(tile) == 0 || self.map.tile_y(tile) == 0 {
                return cost.error(if transport == WATER {
                    self.a.strings.bridge_border
                } else {
                    self.a.strings.map_edge
                });
            }
        }
        match transport {
            ROAD => {
                roadtype = kind;
                if !(self.s.road_type_valid)(roadtype) {
                    return Self::error(cost);
                }
            }
            RAIL => {
                railtype = kind;
                if !(self.r.rail_type_valid)(railtype) {
                    return Self::error(cost);
                }
            }
            WATER => (),
            _ => return Self::error(cost),
        }
        if company == OWNER_DEITY {
            if transport != ROAD {
                return Self::error(cost);
            }
            company = self.deity_owner(start);
        }
        if start == end {
            return cost.error(self.a.strings.same_end);
        }
        let axis = if self.map.tile_x(start) == self.map.tile_x(end) {
            1
        } else if self.map.tile_y(start) == self.map.tile_y(end) {
            0
        } else {
            return cost.error(self.a.strings.straight);
        };
        if end < start {
            std::mem::swap(&mut start, &mut end);
        }
        let mut length = self.length(start, end);
        if transport != WATER {
            self.available(ret, bridge_type, length, flags);
            if !ret.ok() {
                return cost.take(ret);
            }
        } else if length > u32::from((self.a.max_bridge_length)()) {
            return cost.error(self.a.strings.too_long);
        }
        length = length.wrapping_add(2);
        let (mut start_slope, mut start_z) = self.slope_z(start);
        let (mut end_slope, mut end_z) = self.slope_z(end);
        let mut reservation = false;
        self.slope(north, true, axis, &mut start_slope, &mut start_z);
        self.slope(south, false, axis, &mut end_slope, &mut end_z);
        if transport == WATER && (start_slope == SLOPE_FLAT || end_slope == SLOPE_FLAT) {
            return cost.error(self.a.strings.wrong_slope);
        }
        if start_z != end_z {
            return cost.error(self.a.strings.unequal_height);
        }
        cost.set(0);
        let mut owner;
        let new_owner;
        let mut road = INVALID_ROADTYPE;
        let mut tram = INVALID_ROADTYPE;
        if (self.s.is_bridge)(start)
            && (self.s.is_bridge)(end)
            && (self.a.other_bridge_end)(start) == end
            && (self.s.tunnel_transport)(start) == transport
        {
            w::hit(w::TB_REPLACE_BRIDGE);
            match transport {
                RAIL => reservation = (self.rail.wormhole_reserved)(start),
                ROAD => {
                    road = (self.s.road_type)(start);
                    tram = (self.s.tram_type)(start).0;
                }
                _ => (),
            }
            if transport == RAIL && (self.rail.rail_type)(start).0 != railtype {
                return cost.error(self.a.strings.demolish_bridge);
            }
            if transport == ROAD {
                let existing = if (self.s.road_type_is_road)(roadtype) {
                    road
                } else {
                    tram
                };
                if existing != roadtype && existing != INVALID_ROADTYPE {
                    return cost.error(self.a.strings.demolish_bridge);
                }
            }
            if flags & DoCommandFlag::QUERY_COST == 0 {
                if bridge_type == (self.a.bridge_type)(start)
                    && (transport != ROAD || road == roadtype || tram == roadtype)
                {
                    return cost.error(self.a.strings.already_built);
                }
                if (self.s.tile_owner)(start) == OWNER_TOWN && !self.editor() {
                    let town = (self.s.closest_town)(start, u32::MAX);
                    if town.is_null() {
                        return Self::error(cost);
                    }
                    if (self.a.speed)(bridge_type) < (self.a.speed)((self.a.bridge_type)(start)) {
                        return (self.c.error_param)(
                            cost.0,
                            self.a.strings.authority,
                            u64::from((self.authority.town_index)(town)),
                        );
                    }
                    self.rating(town, flags, RATING_TUNNEL_BRIDGE_UP_STEP, RATING_MAXIMUM);
                }
            }
            if (self.s.tile_owner)(start) != company
                && (self.s.tile_owner)(start) != OWNER_TOWN
                && (self.s.tile_owner)(start) != OWNER_NONE
            {
                return cost.error(self.a.strings.other_owner);
            }
            cost.add_money(
                self.clear_cost(start, PR_CLEAR_BRIDGE)
                    .saturating_mul(i64::from(length)),
            );
            owner = (self.s.tile_owner)(start);
            new_owner = owner == OWNER_NONE;
            if new_owner {
                owner = company;
            }
            let delta = if axis == 0 {
                self.map.diff(1, 0)
            } else {
                self.map.diff(0, 1)
            };
            let mut tile = start.wrapping_add(delta);
            while tile != end {
                self.build_above_tile(ret, tile, flags, axis, start_z.wrapping_add(1));
                if !ret.ok() {
                    return cost.take(ret);
                }
                cost.add_money(ret.money());
                tile = tile.wrapping_add(delta);
            }
        } else {
            w::hit(w::TB_NEW_BRIDGE);
            let allow_slopes = (self.s.build_on_slopes)() && transport != WATER;
            self.clear_command(ret, start, flags);
            if !ret.ok() {
                return cost.take(ret);
            }
            cost.take(ret);
            if !north.ok() || (north.money() != 0 && !allow_slopes) {
                return cost.error(self.a.strings.wrong_slope);
            }
            cost.add_money(north.money());
            self.clear_command(ret, end, flags);
            if !ret.ok() {
                return cost.take(ret);
            }
            cost.add_money(ret.money());
            if !south.ok() || (south.money() != 0 && !allow_slopes) {
                return cost.error(self.a.strings.wrong_slope);
            }
            cost.add_money(south.money());
            for tile in [start, end] {
                if !(self.s.bridge_above)(tile) {
                    continue;
                }
                if axis == (self.s.bridge_axis)(tile) {
                    return cost.error(self.a.strings.demolish_bridge);
                }
                if start_z.wrapping_add(1) == self.bridge_height((self.a.northern_bridge_end)(tile))
                {
                    return cost.error(self.a.strings.demolish_bridge);
                }
            }
            let delta = self.map.offs_by_axis(axis);
            let mut tile = start.wrapping_add(delta);
            while tile != end {
                if (self.s.tile_max_z)(tile) > start_z {
                    return cost.error(self.a.strings.too_low);
                }
                if start_z
                    >= (self.s.tile_z)(tile).wrapping_add(i32::from((self.a.max_bridge_height)()))
                {
                    return cost.error(self.a.strings.too_high);
                }
                if (self.s.bridge_above)(tile) {
                    return cost.error(self.a.strings.demolish_bridge);
                }
                self.build_above_tile(ret, tile, flags, axis, start_z.wrapping_add(1));
                if !ret.ok() {
                    return cost.take(ret);
                }
                cost.add_money(ret.money());
                if flags & DoCommandFlag::EXECUTE != 0 {
                    (self.a.set_middle)(tile, Axis(axis));
                }
                tile = tile.wrapping_add(delta);
            }
            owner = company;
            new_owner = true;
        }
        let mut hasroad = road != INVALID_ROADTYPE;
        let mut hastram = tram != INVALID_ROADTYPE;
        if transport == ROAD {
            if (self.s.road_type_is_road)(roadtype) {
                road = roadtype;
            }
            if (self.s.road_type_is_tram)(RoadType(roadtype)) {
                tram = roadtype;
            }
        }
        if flags & DoCommandFlag::EXECUTE != 0 {
            let dir = axis_to_diag_dir(axis, |ok| {
                crate::movement::native_assert(self.s, ok);
            });
            let valid_company = (self.s.company_valid)(company);
            let pieces = length.wrapping_mul(TUNNELBRIDGE_TRACKBIT_FACTOR);
            match transport {
                RAIL => {
                    if new_owner && valid_company {
                        (self.r.rail_infrastructure)(company, railtype, pieces);
                    }
                    (self.a.make_rail_bridge)(
                        start,
                        owner,
                        bridge_type,
                        DiagDirection(dir),
                        RailType(railtype),
                    );
                    (self.a.make_rail_bridge)(
                        end,
                        owner,
                        bridge_type,
                        DiagDirection(reverse_diag_dir(dir, |ok| {
                            crate::movement::native_assert(self.s, ok);
                        })),
                        RailType(railtype),
                    );
                    (self.a.set_reservation)(start, reservation);
                    (self.a.set_reservation)(end, reservation);
                }
                ROAD => {
                    if new_owner {
                        if hasroad && (self.s.road_owner)(start) == OWNER_NONE {
                            hasroad = false;
                        }
                        if hastram && (self.s.tram_owner)(start) == OWNER_NONE {
                            hastram = false;
                        }
                    }
                    if valid_company {
                        if !hasroad && road != INVALID_ROADTYPE {
                            (self.r.road_infrastructure)(company, road, pieces.wrapping_mul(2));
                        }
                        if !hastram && tram != INVALID_ROADTYPE {
                            (self.r.road_infrastructure)(company, tram, pieces.wrapping_mul(2));
                        }
                    }
                    let owner_road = if hasroad {
                        (self.s.road_owner)(start)
                    } else {
                        company
                    };
                    let owner_tram = if hastram {
                        (self.s.tram_owner)(start)
                    } else {
                        company
                    };
                    (self.a.make_road_bridge)(
                        start,
                        owner,
                        owner_road,
                        owner_tram,
                        bridge_type,
                        DiagDirection(dir),
                        RoadType(road),
                        RoadType(tram),
                    );
                    (self.a.make_road_bridge)(
                        end,
                        owner,
                        owner_road,
                        owner_tram,
                        bridge_type,
                        DiagDirection(reverse_diag_dir(dir, |ok| {
                            crate::movement::native_assert(self.s, ok);
                        })),
                        RoadType(road),
                        RoadType(tram),
                    );
                }
                WATER => {
                    if new_owner && valid_company {
                        (self.stations.water_infrastructure)(company, pieces);
                    }
                    (self.a.make_aqueduct)(start, owner, DiagDirection(dir));
                    (self.a.make_aqueduct)(
                        end,
                        owner,
                        DiagDirection(reverse_diag_dir(dir, |ok| {
                            crate::movement::native_assert(self.s, ok);
                        })),
                    );
                    self.check_docking(start);
                    self.check_docking(end);
                }
                _ => unreachable!(),
            }
            (self.a.mark_bridge)(
                start,
                end,
                DiagDirection(axis_to_diag_dir(axis, |ok| {
                    crate::movement::native_assert(self.s, ok);
                })),
                start_z as u32,
            );
            (self.cmd.infrastructure_dirty)(company);
        }
        if flags & DoCommandFlag::EXECUTE != 0 && transport == RAIL {
            self.signal(start, INVALID_DIAGDIR, company);
            (self.r.layout_changed)(
                start,
                axis_to_track(Axis(axis), |ok| {
                    crate::movement::native_assert(self.s, ok);
                })
                .0,
            );
        }
        let valid_company = (self.s.company_valid)(company);
        // SAFETY: immutable process-lifetime company table.
        let companies = unsafe { &*self.a.company };
        if flags & DoCommandFlag::QUERY_COST == 0
            || (valid_company
                && (companies.company_is_ai)(company)
                && company != (companies.local_company)())
        {
            if flags & DoCommandFlag::QUERY_COST != 0 {
                w::hit(w::TB_AI_QUERY);
            }
            match transport {
                ROAD => {
                    if road != INVALID_ROADTYPE {
                        cost.add_money(
                            crate::road_cost::build_cost(self.s, road)
                                .saturating_mul(i64::from(length.wrapping_mul(2))),
                        );
                    }
                    if tram != INVALID_ROADTYPE {
                        cost.add_money(
                            crate::road_cost::build_cost(self.s, tram)
                                .saturating_mul(i64::from(length.wrapping_mul(2))),
                        );
                    }
                }
                RAIL => cost.add_money(
                    (self.r.rail_build_cost)(railtype).saturating_mul(i64::from(length)),
                ),
                _ => (),
            }
            if valid_company {
                length = calc_bridge_len_cost_factor(length as i32) as u32;
            }
            if transport != WATER {
                cost.add_money(
                    i64::from(length)
                        .saturating_mul(price(PR_BUILD_BRIDGE))
                        .saturating_mul(i64::from((self.a.price)(bridge_type)))
                        >> 8,
                );
            } else {
                cost.add_money(i64::from(length).saturating_mul(price(PR_BUILD_AQUEDUCT)));
            }
        }
    }
    fn build_tunnel(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        flags: u16,
        start: u32,
        transport: u8,
        kind: u8,
    ) {
        w::hit(w::TB_BUILD_TUNNEL);
        let mut company = (self.s.current_company)();
        let mut railtype = (self.rail.invalid_rail_type)().0;
        let mut roadtype = INVALID_ROADTYPE;
        END_TILE.store(0, Ordering::Relaxed);
        match transport {
            RAIL => {
                railtype = kind;
                if !(self.r.rail_type_valid)(railtype) {
                    return Self::error(cost);
                }
            }
            ROAD => {
                roadtype = kind;
                if !(self.s.road_type_valid)(roadtype) {
                    return Self::error(cost);
                }
            }
            _ => return Self::error(cost),
        }
        if company == OWNER_DEITY {
            if transport != ROAD {
                return Self::error(cost);
            }
            company = self.deity_owner(start);
        }
        let (start_slope, start_z) = self.slope_z(start);
        let direction = get_inclined_slope_direction(start_slope);
        if direction == INVALID_DIAGDIR {
            return cost.error(self.a.strings.tunnel_site);
        }
        if (self.s.has_water_ground)(start) {
            return cost.error(self.a.strings.water);
        }
        self.clear_command(ret, start, flags);
        if !ret.ok() {
            return cost.take(ret);
        }
        let delta = self.map.offs_by_diag_dir(direction);
        // Both source branches deliberately compare against Map::MaxX().
        let in_way = if diag_dir_to_axis(direction, |ok| {
            crate::movement::native_assert(self.s, ok);
        }) == 1
        {
            if self.map.tile_x(start) < (self.map.size_x() - 1) / 2 {
                DIAGDIR_SW
            } else {
                DIAGDIR_NE
            }
        } else if self.map.tile_y(start) < (self.map.size_x() - 1) / 2 {
            DIAGDIR_SE
        } else {
            DIAGDIR_NW
        };
        let mut end = start;
        let mut coefficient = 3;
        let mut tiles: i32 = 0;
        let mut bump: i32 = 25;
        cost.set(0);
        let end_slope;
        loop {
            end = end.wrapping_add(delta);
            if !(self.s.tile_valid)(end) {
                return cost.error(self.a.strings.tunnel_border);
            }
            let (slope, z) = self.slope_z(end);
            if start_z == z {
                end_slope = slope;
                break;
            }
            if !(self.a.crossing_tunnels)()
                && (self.a.tunnel_in_way)(end, start_z, DiagDirection(in_way))
            {
                return cost.error(self.a.strings.tunnel_way);
            }
            tiles = tiles.wrapping_add(1);
            if tiles == bump {
                coefficient += 1;
                bump = bump.wrapping_mul(2);
            }
            cost.add_money(price(PR_BUILD_TUNNEL));
            cost.add_money(cost.money() >> coefficient);
        }
        cost.add_money(price(PR_BUILD_TUNNEL));
        cost.add_money(ret.money());
        END_TILE.store(end, Ordering::Relaxed);
        if tiles > i32::from((self.a.max_tunnel_length)()) {
            return cost.error(self.a.strings.tunnel_long);
        }
        if (self.s.has_water_ground)(end) {
            return cost.error(self.a.strings.water);
        }
        self.clear_command(ret, end, flags);
        if !ret.ok() {
            return cost.error(self.a.strings.excavate);
        }
        cost.add_money(ret.money());
        if end_slope != complement_slope(start_slope) {
            w::hit(w::TB_TUNNEL_TERRAFORM);
            // The canonical scratch owner returns a stable index, then restores
            // the first tile after recursive terraform may append/reallocate.
            let hidden =
                crate::objects::openttd_rust_object_tunnel_hide(end, u32::from(self.map.log_x));
            (self.a.terraform)(ret.0, flags, end, end_slope & start_slope);
            crate::objects::openttd_rust_object_restore_first(hidden);
            if !ret.ok() {
                return cost.error(self.a.strings.excavate);
            }
            cost.add_money(ret.money());
        }
        cost.add_money(price(PR_BUILD_TUNNEL));
        match transport {
            ROAD => cost.add_money(
                crate::road_cost::build_cost(self.s, roadtype)
                    .saturating_mul(i64::from(tiles.wrapping_add(2)))
                    .saturating_mul(2),
            ),
            RAIL => cost.add_money(
                (self.r.rail_build_cost)(railtype).saturating_mul(i64::from(tiles.wrapping_add(2))),
            ),
            _ => unreachable!(),
        }
        if flags & DoCommandFlag::EXECUTE != 0 {
            let valid = (self.s.company_valid)(company);
            let pieces = (tiles.wrapping_add(2) as u32).wrapping_mul(TUNNELBRIDGE_TRACKBIT_FACTOR);
            if transport == RAIL {
                if valid {
                    (self.r.rail_infrastructure)(company, railtype, pieces);
                }
                (self.a.make_rail_tunnel)(
                    start,
                    company,
                    DiagDirection(direction),
                    RailType(railtype),
                );
                (self.a.make_rail_tunnel)(
                    end,
                    company,
                    DiagDirection(reverse_diag_dir(direction, |ok| {
                        crate::movement::native_assert(self.s, ok);
                    })),
                    RailType(railtype),
                );
                self.signal(start, INVALID_DIAGDIR, company);
                (self.r.layout_changed)(
                    start,
                    diag_dir_to_diag_track(direction, |ok| {
                        crate::movement::native_assert(self.s, ok);
                    }),
                );
            } else {
                if valid {
                    (self.r.road_infrastructure)(company, roadtype, pieces.wrapping_mul(2));
                }
                let road = if (self.s.road_type_is_road)(roadtype) {
                    roadtype
                } else {
                    INVALID_ROADTYPE
                };
                let tram = if (self.s.road_type_is_tram)(RoadType(roadtype)) {
                    roadtype
                } else {
                    INVALID_ROADTYPE
                };
                (self.a.make_road_tunnel)(
                    start,
                    company,
                    DiagDirection(direction),
                    RoadType(road),
                    RoadType(tram),
                );
                (self.a.make_road_tunnel)(
                    end,
                    company,
                    DiagDirection(reverse_diag_dir(direction, |ok| {
                        crate::movement::native_assert(self.s, ok);
                    })),
                    RoadType(road),
                    RoadType(tram),
                );
            }
            (self.cmd.infrastructure_dirty)(company);
        }
    }
    fn allow_remove(&self, out: Cost<'_>, tile: u32) {
        out.zero();
        let company = (self.s.current_company)();
        if company == OWNER_WATER || self.editor() {
            return;
        }
        match (self.s.tunnel_transport)(tile) {
            ROAD => {
                let road = (self.s.road_type)(tile);
                let tram = (self.s.tram_type)(tile).0;
                let mut road_owner = company;
                let mut tram_owner = company;
                if road != INVALID_ROADTYPE {
                    road_owner = (self.s.road_owner)(tile);
                }
                if tram != INVALID_ROADTYPE {
                    tram_owner = (self.s.tram_owner)(tile);
                }
                if road_owner == OWNER_TOWN
                    && company != OWNER_TOWN
                    && !((self.road.extra_dynamite)() || (self.authority.bulldozer)())
                {
                    // SAFETY: native slot remains live across encoded-error construction.
                    unsafe { (self.cmd.tile_ownership)(out.0, tile) };
                    return;
                }
                if road_owner == OWNER_NONE || road_owner == OWNER_TOWN {
                    road_owner = company;
                }
                if tram_owner == OWNER_NONE {
                    tram_owner = company;
                }
                self.ownership(out, road_owner, tile);
                if out.ok() {
                    self.ownership(out, tram_owner, tile);
                }
            }
            RAIL => self.ownership(out, (self.s.tile_owner)(tile), 0),
            WATER => {
                let mut owner = (self.s.tile_owner)(tile);
                if owner == OWNER_NONE {
                    owner = company;
                }
                self.ownership(out, owner, 0);
            }
            _ => unreachable!(),
        }
    }
    fn update_road_infrastructure(&self, road: u8, owner: u8, count: i32) {
        // Original UpdateCompanyRoadInfrastructure helper, owned with roads.
        unsafe {
            crate::road_cmd::update_company_road_infrastructure(self.a.road, road, owner, count);
        }
    }
    fn clear(&self, cost: Cost<'_>, ret: Cost<'_>, tile: u32, flags: u16) {
        if (self.a.is_tunnel)(tile) {
            if flags & DoCommandFlag::AUTO != 0 {
                return cost.error(self.a.strings.demolish_tunnel);
            }
            self.do_clear_tunnel(cost, ret, tile, flags);
        } else {
            if flags & DoCommandFlag::AUTO != 0 {
                return cost.error(self.a.strings.demolish_bridge);
            }
            self.do_clear_bridge(cost, ret, tile, flags);
        }
    }
    fn do_clear_tunnel(&self, cost: Cost<'_>, ret: Cost<'_>, tile: u32, flags: u16) {
        w::hit(w::TB_CLEAR_TUNNEL);
        self.allow_remove(ret, tile);
        if !ret.ok() {
            return cost.take(ret);
        }
        let end = (self.s.other_tunnel_end)(tile);
        // SAFETY: live cost slot and original endpoints; no retained native reference.
        unsafe {
            (self.cmd.wormhole_free)(ret.0, tile, end, std::ptr::null());
        }
        if !ret.ok() {
            w::hit(w::TB_OCCUPIED);
            return cost.take(ret);
        }
        END_TILE.store(end, Ordering::Relaxed);
        let mut town = std::ptr::null();
        if (self.s.tile_owner)(tile) == OWNER_TOWN && !self.editor() {
            town = (self.s.closest_town)(tile, u32::MAX);
            self.rating_check(ret, town, flags);
            if !ret.ok() {
                return cost.take(ret);
            }
        }
        if (self.s.tile_owner)(tile) == OWNER_TOWN && !self.editor() {
            self.rating(
                town,
                flags,
                RATING_TUNNEL_BRIDGE_DOWN_STEP,
                RATING_TUNNEL_BRIDGE_MINIMUM,
            );
        }
        let base = self.clear_cost(tile, PR_CLEAR_TUNNEL);
        let length = self.length(tile, end).wrapping_add(2);
        if flags & DoCommandFlag::EXECUTE != 0 {
            if (self.s.tunnel_transport)(tile) == RAIL {
                let dir = (self.s.tunnel_bridge_direction)(tile);
                let track = diag_dir_to_diag_track(dir, |ok| {
                    crate::movement::native_assert(self.s, ok);
                });
                let owner = (self.s.tile_owner)(tile);
                let train = self.release_reservation(tile, track);
                if (self.s.company_valid)(owner) {
                    (self.r.rail_infrastructure)(
                        owner,
                        (self.rail.rail_type)(tile).0,
                        length
                            .wrapping_mul(TUNNELBRIDGE_TRACKBIT_FACTOR)
                            .wrapping_neg(),
                    );
                    (self.cmd.infrastructure_dirty)(owner);
                }
                crate::ground::clear_square(tile, self.s);
                crate::ground::clear_square(end, self.s);
                self.signal(
                    tile,
                    reverse_diag_dir(dir, |ok| crate::movement::native_assert(self.s, ok)),
                    owner,
                );
                self.signal(end, dir, owner);
                (self.r.layout_changed)(tile, track);
                (self.r.layout_changed)(end, track);
                if !train.is_null() {
                    unsafe {
                        (self.s.try_path_reserve)(train, false, false);
                    }
                }
            } else {
                self.update_road_infrastructure(
                    (self.s.road_type)(tile),
                    (self.s.road_owner)(tile),
                    length
                        .wrapping_mul(2)
                        .wrapping_mul(TUNNELBRIDGE_TRACKBIT_FACTOR)
                        .wrapping_neg() as i32,
                );
                self.update_road_infrastructure(
                    (self.s.tram_type)(tile).0,
                    (self.s.tram_owner)(tile),
                    length
                        .wrapping_mul(2)
                        .wrapping_mul(TUNNELBRIDGE_TRACKBIT_FACTOR)
                        .wrapping_neg() as i32,
                );
                crate::ground::clear_square(tile, self.s);
                crate::ground::clear_square(end, self.s);
            }
        }
        cost.set(base.saturating_mul(i64::from(length)));
    }
    fn do_clear_bridge(&self, cost: Cost<'_>, ret: Cost<'_>, tile: u32, flags: u16) {
        w::hit(w::TB_CLEAR_BRIDGE);
        self.allow_remove(ret, tile);
        if !ret.ok() {
            return cost.take(ret);
        }
        let end = (self.a.other_bridge_end)(tile);
        // SAFETY: live cost slot and original endpoints; no retained native reference.
        unsafe {
            (self.cmd.wormhole_free)(ret.0, tile, end, std::ptr::null());
        }
        if !ret.ok() {
            w::hit(w::TB_OCCUPIED);
            return cost.take(ret);
        }
        let direction = (self.s.tunnel_bridge_direction)(tile);
        let delta = self.map.offs_by_diag_dir(direction);
        let mut town = std::ptr::null();
        if (self.s.tile_owner)(tile) == OWNER_TOWN && !self.editor() {
            town = (self.s.closest_town)(tile, u32::MAX);
            self.rating_check(ret, town, flags);
            if !ret.ok() {
                return cost.take(ret);
            }
        }
        if (self.s.tile_owner)(tile) == OWNER_TOWN && !self.editor() {
            self.rating(
                town,
                flags,
                RATING_TUNNEL_BRIDGE_DOWN_STEP,
                RATING_TUNNEL_BRIDGE_MINIMUM,
            );
        }
        let base = self.clear_cost(tile, PR_CLEAR_BRIDGE);
        let length = self.length(tile, end).wrapping_add(2);
        if flags & DoCommandFlag::EXECUTE != 0 {
            let rail = (self.s.tunnel_transport)(tile) == RAIL;
            let owner = (self.s.tile_owner)(tile);
            let height = self.bridge_height(tile);
            let train = if rail {
                self.release_reservation(
                    tile,
                    diag_dir_to_diag_track(direction, |ok| {
                        crate::movement::native_assert(self.s, ok);
                    }),
                )
            } else {
                std::ptr::null_mut()
            };
            if rail {
                if (self.s.company_valid)(owner) {
                    (self.r.rail_infrastructure)(
                        owner,
                        (self.rail.rail_type)(tile).0,
                        length
                            .wrapping_mul(TUNNELBRIDGE_TRACKBIT_FACTOR)
                            .wrapping_neg(),
                    );
                }
            } else if (self.s.tunnel_transport)(tile) == ROAD {
                self.update_road_infrastructure(
                    (self.s.road_type)(tile),
                    (self.s.road_owner)(tile),
                    length
                        .wrapping_mul(2)
                        .wrapping_mul(TUNNELBRIDGE_TRACKBIT_FACTOR)
                        .wrapping_neg() as i32,
                );
                self.update_road_infrastructure(
                    (self.s.tram_type)(tile).0,
                    (self.s.tram_owner)(tile),
                    length
                        .wrapping_mul(2)
                        .wrapping_mul(TUNNELBRIDGE_TRACKBIT_FACTOR)
                        .wrapping_neg() as i32,
                );
            } else if (self.s.company_valid)(owner) {
                (self.stations.water_infrastructure)(
                    owner,
                    length
                        .wrapping_mul(TUNNELBRIDGE_TRACKBIT_FACTOR)
                        .wrapping_neg(),
                );
            }
            (self.cmd.infrastructure_dirty)(owner);
            crate::ground::clear_square(tile, self.s);
            crate::ground::clear_square(end, self.s);
            let mut middle = tile.wrapping_add(delta);
            // SAFETY: immutable table; only original scalar observations cross.
            let road_ground = unsafe { &*self.a.road_ground };
            while middle != end {
                if (self.s.is_normal_road)(middle)
                    && (road_ground.roadside)(middle).0 == ROADSIDE_TREES.0
                {
                    let min_z = (self.s.tile_max_z)(middle).wrapping_add(3);
                    if height < min_z {
                        (road_ground.set_roadside)(middle, ROADSIDE_PAVED);
                    }
                }
                (self.a.clear_middle)(middle);
                (self.a.mark_dirty_height)(
                    middle,
                    (height as u32).wrapping_sub((self.s.tile_height)(middle)) as i32,
                );
                middle = middle.wrapping_add(delta);
            }
            if rail {
                self.signal(
                    tile,
                    reverse_diag_dir(direction, |ok| {
                        crate::movement::native_assert(self.s, ok);
                    }),
                    owner,
                );
                self.signal(end, direction, owner);
                let track = diag_dir_to_diag_track(direction, |ok| {
                    crate::movement::native_assert(self.s, ok);
                });
                (self.r.layout_changed)(tile, track);
                (self.r.layout_changed)(end, track);
                if !train.is_null() {
                    unsafe {
                        (self.s.try_path_reserve)(train, true, false);
                    }
                }
            }
        }
        cost.set(base.saturating_mul(i64::from(length)));
    }
    fn release_reservation(&self, tile: u32, track: u8) -> *mut Train {
        if !(self.rail.wormhole_reserved)(tile) {
            return std::ptr::null_mut();
        }
        w::hit(w::TB_RESERVATION);
        let train = (self.r.reserving_train)(tile, track);
        if !train.is_null() {
            unsafe {
                (self.s.free_train_reservation)(train);
            }
        }
        train
    }
    fn change_owner(&self, ret: Cost<'_>, tile: u32, old: u8, new: u8) {
        w::hit(w::TB_OWNER);
        let end = (self.rail.other_tunnel_bridge_end)(tile);
        let pieces = if tile < end {
            self.length(tile, end)
                .wrapping_add(2)
                .wrapping_mul(TUNNELBRIDGE_TRACKBIT_FACTOR)
        } else {
            0
        };
        let transport = (self.s.tunnel_transport)(tile);
        if transport == ROAD {
            for rtt in [0, 1] {
                let owner = if rtt == 0 {
                    (self.s.road_owner)(tile)
                } else {
                    (self.s.tram_owner)(tile)
                };
                if owner == old {
                    let kind = if rtt == 0 {
                        (self.s.road_type)(tile)
                    } else {
                        (self.s.tram_type)(tile).0
                    };
                    if kind != INVALID_ROADTYPE {
                        (self.r.road_infrastructure)(
                            old,
                            kind,
                            pieces.wrapping_mul(2).wrapping_neg(),
                        );
                        if new != INVALID_OWNER {
                            (self.r.road_infrastructure)(new, kind, pieces.wrapping_mul(2));
                        }
                    }
                    (self.s.set_road_owner)(
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
        }
        if (self.s.tile_owner)(tile) != old {
            return;
        }
        if transport == RAIL {
            (self.r.rail_infrastructure)(old, (self.rail.rail_type)(tile).0, pieces.wrapping_neg());
            if new != INVALID_OWNER {
                (self.r.rail_infrastructure)(new, (self.rail.rail_type)(tile).0, pieces);
            }
        } else if transport == WATER {
            (self.stations.water_infrastructure)(old, pieces.wrapping_neg());
            if new != INVALID_OWNER {
                (self.stations.water_infrastructure)(new, pieces);
            }
        }
        if new != INVALID_OWNER {
            (self.s.set_tile_owner)(tile, new);
        } else if transport == RAIL {
            self.clear_command(ret, tile, DoCommandFlag::EXECUTE | DoCommandFlag::BANKRUPT);
            // Preserve native assertion policy without adding a release guard.
            let signal = unsafe { &*self.r.signal };
            if signal.assertions {
                (signal.assert_condition)(ret.ok());
            }
        } else {
            (self.s.set_tile_owner)(tile, OWNER_NONE);
        }
    }
    fn terraform(
        &self,
        out: Cost<'_>,
        ret: Cost<'_>,
        tile: u32,
        flags: u16,
        mut z_new: i32,
        mut slope_new: u8,
    ) {
        if (self.s.build_on_slopes)()
            && (self.s.autoslope)()
            && (self.s.is_bridge)(tile)
            && (self.s.tunnel_transport)(tile) != WATER
        {
            let direction = (self.s.tunnel_bridge_direction)(tile);
            let axis = diag_dir_to_axis(direction, |ok| {
                crate::movement::native_assert(self.s, ok);
            });
            let (mut slope_old, mut z_old) = self.slope_z(tile);
            let north = direction != DIAGDIR_NW && direction != DIAGDIR_NE;
            self.slope(ret, north, axis, &mut slope_old, &mut z_old);
            self.slope(ret, north, axis, &mut slope_new, &mut z_new);
            if ret.ok() && z_old == z_new && slope_old == slope_new {
                w::hit(w::TB_AUTOSLOPE);
                return out.set(price(PR_BUILD_FOUNDATION));
            }
        }
        self.clear_command(out, tile, flags);
    }
    fn build_above(&self, out: Cost<'_>, tile: u32, flags: u16, axis: u8, height: i32) {
        w::hit(w::TB_BUILD_ABOVE);
        if (self.a.is_tunnel)(tile)
            || (axis
                != diag_dir_to_axis((self.s.tunnel_bridge_direction)(tile), |ok| {
                    crate::movement::native_assert(self.s, ok);
                })
                && height > self.bridge_height(tile))
        {
            out.zero();
        } else {
            self.clear_command(out, tile, flags);
        }
    }
}
/// Process-wide command/GUI output from tunnel construction and removal.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_tunnel_endtile() -> u32 {
    END_TILE.load(Ordering::Relaxed)
}

macro_rules! entry {
    ($name:ident($($arg:ident:$ty:ty),*) $body:expr) => {
        /// # Safety
        /// Immutable tables and independent native result slots are live under
        /// the game-state mutex; original tile/pool preconditions hold.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(a:*const TunnelBridgeServices,$($arg:$ty),*) {
            // SAFETY: the outer facade supplies the synchronous entry contract.
            let world = unsafe { World::new(a) };
            $body(&world);
        }
    };
}
entry!(openttd_rust_bridge_available(out:*mut CommandCost,kind:u32,length:u32,flags:u16) |w:&World<'_>| {
    w.available(w.cost(out),kind,length,flags);
});
entry!(openttd_rust_build_bridge(out:*mut CommandCost,ret:*mut CommandCost,north:*mut CommandCost,south:*mut CommandCost,flags:u16,end:u32,start:u32,transport:TransportType,bridge:u32,kind:u8) |w:&World<'_>| {
    w.build_bridge(w.cost(out),w.cost(ret),w.cost(north),w.cost(south),flags,end,start,transport.0,bridge,kind);
});
entry!(openttd_rust_build_tunnel(out:*mut CommandCost,ret:*mut CommandCost,flags:u16,start:u32,transport:TransportType,kind:u8) |w:&World<'_>| {
    w.build_tunnel(w.cost(out),w.cost(ret),flags,start,transport.0,kind);
});
entry!(openttd_rust_tunnelbridge_clear(out:*mut CommandCost,ret:*mut CommandCost,tile:u32,flags:u16) |w:&World<'_>| {
    w.clear(w.cost(out),w.cost(ret),tile,flags);
});
entry!(openttd_rust_tunnelbridge_terraform(out:*mut CommandCost,ret:*mut CommandCost,tile:u32,flags:u16,z:i32,slope:u8) |w:&World<'_>| {
    w.terraform(w.cost(out),w.cost(ret),tile,flags,z,slope);
});
entry!(openttd_rust_tunnelbridge_build_above(out:*mut CommandCost,tile:u32,flags:u16,axis:Axis,height:i32) |w:&World<'_>| {
    w.build_above(w.cost(out),tile,flags,axis.0,height);
});
/// Direct shared ownership dispatch, reusing the outer frame's result slot.
/// # Safety
/// Same synchronous table/result-slot contract as the native facade.
pub(crate) unsafe fn change_owner(
    a: *const TunnelBridgeServices,
    ret: *mut CommandCost,
    tile: u32,
    old: u8,
    new: u8,
) {
    // SAFETY: the shared dispatcher provides the canonical tables and live slot.
    let w = unsafe { World::new(a) };
    w.change_owner(w.cost(ret), tile, old, new);
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDBridgeAvailability" ["probe"] => Availability { year, min, max, };
    "OpenTTDTunnelBridgeStrings" ["probe"] => Strings {
        bridge_border,
        map_edge,
        same_end,
        straight,
        too_long,
        wrong_slope,
        unequal_height,
        demolish_bridge,
        already_built,
        authority,
        other_owner,
        too_low,
        too_high,
        tunnel_site,
        water,
        tunnel_border,
        tunnel_way,
        tunnel_long,
        excavate,
        demolish_tunnel,
    };
    "OpenTTDTunnelBridgeServices" ["probe"] => TunnelBridgeServices {
        rail,
        road,
        road_ground,
        water,
        company,
        availability,
        speed,
        price,
        max_bridge_length,
        max_bridge_height,
        max_tunnel_length,
        crossing_tunnels,
        other_bridge_end,
        northern_bridge_end,
        bridge_type,
        is_tunnel,
        tunnel_in_way,
        set_middle,
        clear_middle,
        set_reservation,
        make_rail_bridge,
        make_road_bridge,
        make_aqueduct,
        make_rail_tunnel,
        make_road_tunnel,
        mark_bridge,
        mark_dirty_height,
        terraform,
        objects,
        strings,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tunnel_output_is_shared_with_the_gui_thread() {
        let before = END_TILE.swap(0x1234_5678, Ordering::Relaxed);
        let value = std::thread::spawn(|| {
            assert_eq!(openttd_rust_tunnel_endtile(), 0x1234_5678);
            END_TILE.store(0x8765_4321, Ordering::Relaxed);
        });
        value.join().expect("GUI reader thread");
        assert_eq!(openttd_rust_tunnel_endtile(), 0x8765_4321);
        END_TILE.store(before, Ordering::Relaxed);
    }
}
