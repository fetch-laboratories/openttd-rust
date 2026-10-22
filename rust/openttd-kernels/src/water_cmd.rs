/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Water construction, clearing, restoration and tile policy (#294). Map/pool
//! identities and `CommandCost` storage remain native. The world mutex serializes
//! each synchronous entry. No native reference is formed or retained over a
//! callback; nested clear commands may reenter any owner. Native leaves are
//! noexcept (no ordinary-play throwing service); panics abort.
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
use crate::command_type::DoCommandFlag;
use crate::company_type::{INVALID_OWNER, OWNER_NONE, OWNER_WATER};
use crate::direction_type::INVALID_DIAGDIR;
use crate::services::{
    CommandCost, CommandCostServices, CommandServices, Map, Services, WaterClass,
};
use crate::slope_func::{
    get_inclined_slope_direction, is_inclined_slope, is_slope_with_one_corner_raised,
};
use crate::strings_type::INVALID_STRING_ID;
use crate::tile_type::INVALID_TILE;
use crate::viewport_func::TILE_HEIGHT_STEP;
use crate::water::{RIVER_OFFSET_DESERT_DISTANCE, WaterServices};
use crate::witness as w;

macro_rules! carrier {
    ($($name:ident),+) => {$(#[repr(transparent)] #[derive(Clone, Copy)] pub struct $name(pub u8);)+};
}
carrier!(WaterTileType, LockPart, DepotPart, Axis);
#[repr(C)]
pub struct Area {
    _private: [u8; 0],
}
#[repr(C)]
pub struct Strings {
    pub built_on_water: u32,
    pub demolish_bridge: u32,
    pub unsuitable: u32,
    pub sloped_wrong: u32,
    pub edge: u32,
    pub low_bridge: u32,
    pub flat: u32,
    pub already_built: u32,
    pub build_on_water: u32,
    pub demolish_building: u32,
    pub demolish_canal: u32,
}
#[repr(C)]
pub struct WaterCommandServices {
    pub shared: *const Services,
    pub cost: *const CommandCostServices,
    pub command: *const CommandServices,
    pub water: *const WaterServices,
    pub rail: *const crate::rail_cmd::RailCommandServices,
    pub water_type: extern "C" fn(u32) -> WaterTileType,
    pub lock_part: extern "C" fn(u32) -> LockPart,
    pub lock_direction: extern "C" fn(u32) -> crate::services::DiagDirection,
    pub other_depot_tile: extern "C" fn(u32) -> u32,
    pub restoration: crate::water::RestorationServices,
    pub environment: crate::services::EnvironmentServices,
    pub make_depot: extern "C" fn(u32, u8, u16, DepotPart, Axis, WaterClass),
    pub make_lock_tile:
        extern "C" fn(u32, u8, LockPart, crate::services::DiagDirection, WaterClass),
    pub area_begin: unsafe extern "C" fn(*mut Area, u32, u32, bool),
    pub area_tile: unsafe extern "C" fn(*mut Area) -> u32,
    pub area_next: unsafe extern "C" fn(*mut Area),
    pub strings: Strings,
}
const FACTOR: u32 = 2;

/// `GetLockPartMinimalBridgeHeight`.
fn get_lock_part_minimal_bridge_height(lock_part: usize) -> i32 {
    const MINIMAL_BRIDGE_HEIGHT: [u8; 3] = [
        2, // LockPart::Middle
        3, // LockPart::Lower
        2, // LockPart::Upper
    ];
    i32::from(MINIMAL_BRIDGE_HEIGHT[lock_part])
}

struct World<'a> {
    a: &'a WaterCommandServices,
    s: &'a Services,
    c: &'a CommandCostServices,
    cmd: &'a CommandServices,
    water: &'a WaterServices,
    size_x: u32,
    size_y: u32,
    map: Map,
}
impl World<'_> {
    unsafe fn new(a: *const WaterCommandServices) -> Self {
        // SAFETY: Each facade supplies immutable process-lifetime tables under
        // the game-state mutex. Only these tables are borrowed, never world state.
        let (a, s, c, cmd, water) = unsafe {
            (
                &*a,
                &*(*a).shared,
                &*(*a).cost,
                &*(*a).command,
                &*(*a).water,
            )
        };
        let size_x = (s.map_max_x)() + 1;
        Self {
            a,
            s,
            c,
            cmd,
            water,
            size_x,
            size_y: (s.map_max_y)() + 1,
            map: Map {
                log_x: size_x.trailing_zeros() as u8,
            },
        }
    }
    fn error(&self, out: *mut CommandCost, string: u32) {
        (self.c.error)(out, string);
    }
    fn expense(&self, out: *mut CommandCost, money: i64) {
        (self.c.expense_money)(out, 0, money);
    }
    fn failed(&self, out: *const CommandCost) -> bool {
        !(self.c.success)(out)
    }
    fn take_failure(&self, out: *mut CommandCost, ret: *mut CommandCost) -> bool {
        if self.failed(ret) {
            (self.c.move_cost)(out, ret);
            true
        } else {
            false
        }
    }
    fn clear_command(&self, ret: *mut CommandCost, tile: u32, flags: u16) {
        crate::landscape_cmd::clear_command(self.cmd, ret, tile, flags);
    }
    fn ownership(&self, ret: *mut CommandCost, tile: u32) {
        // SAFETY: The facade owns this live native stack slot across reentry.
        unsafe {
            (self.cmd.tile_ownership)(ret, tile);
        }
    }
    fn no_vehicle(&self, ret: *mut CommandCost, tile: u32) {
        // SAFETY: The facade owns this live native stack slot across reentry.
        unsafe {
            (self.cmd.no_vehicle)(ret, tile);
        }
    }
    fn class(&self, t: u32) -> u8 {
        unsafe { ((*self.water.clear).water_class)(t).0 }
    }
    fn rail(&self) -> &crate::rail_cmd::RailCommandServices {
        // SAFETY: native getter supplies an immutable process-lifetime table.
        unsafe { &*self.a.rail }
    }
    fn bridge_height(&self, tile: u32) -> i32 {
        // SAFETY: canonical immutable station services; no owner state is borrowed.
        let stations = unsafe { &*(self.s.station_commands)() };
        (stations.bridge_height)((stations.southern_bridge_end)(tile))
    }
    fn check_docking(&self, tile: u32) {
        // SAFETY: the immutable station table is live; docking borrows no water owner.
        unsafe {
            crate::station_cmd::openttd_rust_station_check_docking(
                (self.s.station_commands)(),
                tile,
            );
        }
    }
    fn kind(&self, t: u32) -> u8 {
        (self.a.water_type)(t).0
    }
    fn flat(&self, t: u32) -> bool {
        (self.s.tile_slope)(t) == 0
    }
    fn canal(&self, t: u32) -> bool {
        self.kind(t) == 0 && self.class(t) == 1
    }
    fn edge(&self, t: u32) -> bool {
        let x = t % self.size_x;
        let y = t / self.size_x;
        !(1..self.size_x - 2).contains(&x) || !(1..self.size_y - 2).contains(&y)
    }
    fn infra(&self, owner: u8, delta: u32, dirty: bool) {
        (self.a.restoration.infrastructure)(owner, delta);
        if dirty {
            (self.cmd.infrastructure_dirty)(owner);
        }
    }
    fn random(&self) -> u8 {
        (self.s.random)(self.s.context) as u8
    }
    fn dirty_neighbours(&self, tile: u32) {
        // SAFETY: Direct call to the integrated Rust owner with the same tables.
        unsafe {
            crate::water::dirty_water_neighbours(
                tile,
                self.size_x,
                self.size_y,
                self.water,
                self.s,
            );
        }
    }
    fn wake_neighbours(&self, tile: u32) {
        // SAFETY: Direct call; no borrow of native map data spans this entry.
        unsafe {
            crate::water::openttd_rust_clear_neighbour_flooding(
                tile,
                self.size_x,
                self.size_y,
                self.water,
                self.s,
            );
        }
    }
    fn restore(&self, tile: u32, owner: u8) {
        // SAFETY: All tables are immutable and world operations are serialized.
        unsafe {
            crate::water::restore(tile, owner, self.s, self.water, &self.a.restoration);
        }
        w::hit(w::WATER_CMD_RESTORE);
    }
    fn build_depot(
        &self,
        out: *mut CommandCost,
        ret: *mut CommandCost,
        flags: u16,
        tile: u32,
        axis: Axis,
    ) {
        if axis.0 >= 2 {
            self.error(out, INVALID_STRING_ID);
            return;
        }
        let tile2 = tile.wrapping_add(self.map.offs_by_axis(axis.0));
        if !(self.s.has_water_ground)(tile) || !(self.s.has_water_ground)(tile2) {
            self.error(out, self.a.strings.built_on_water);
            return;
        }
        if (self.s.bridge_above)(tile) || (self.s.bridge_above)(tile2) {
            self.error(out, self.a.strings.demolish_bridge);
            return;
        }
        if !self.flat(tile) || !self.flat(tile2) {
            self.error(out, self.a.strings.unsuitable);
            return;
        }
        if !(self.rail().depot_available)() {
            self.error(out, INVALID_STRING_ID);
            return;
        }
        let wc1 = self.class(tile);
        let wc2 = self.class(tile2);
        self.expense(out, crate::company::price(7));
        for t in [tile, tile2] {
            let add = !(self.s.is_water_tile)(t);
            self.clear_command(ret, t, flags | DoCommandFlag::AUTO);
            if self.take_failure(out, ret) {
                return;
            }
            if add {
                (self.c.amount)(out, (self.c.money)(ret));
            }
        }
        if flags & DoCommandFlag::EXECUTE != 0 {
            let id = (self.rail().new_depot)(tile);
            let owner = (self.s.current_company)();
            let mut infra = 2 * FACTOR;
            for (t, wc) in [(tile, wc1), (tile2, wc2)] {
                if wc == 1
                    && !((self.s.has_water_class)(t)
                        && self.class(t) == 1
                        && (self.s.tile_owner)(t) == owner)
                {
                    infra += 1;
                }
            }
            self.infra(owner, infra, true);
            (self.a.make_depot)(tile, owner, id, DepotPart(0), axis, WaterClass(wc1));
            (self.a.make_depot)(tile2, owner, id, DepotPart(1), axis, WaterClass(wc2));
            self.check_docking(tile);
            self.check_docking(tile2);
            (self.s.mark_dirty)(tile);
            (self.s.mark_dirty)(tile2);
            (self.rail().depot_name)(id);
            w::hit(w::WATER_CMD_DEPOT);
        }
    }
    fn remove_depot(&self, out: *mut CommandCost, ret: *mut CommandCost, flags: u16, tile: u32) {
        if self.kind(tile) != 3 {
            self.error(out, INVALID_STRING_ID);
            return;
        }
        self.ownership(ret, tile);
        if self.take_failure(out, ret) {
            return;
        }
        let tile2 = (self.a.other_depot_tile)(tile);
        if flags & DoCommandFlag::BANKRUPT == 0 {
            self.no_vehicle(ret, tile);
            if !self.failed(ret) {
                self.no_vehicle(ret, tile2);
            }
            if self.take_failure(out, ret) {
                return;
            }
        }
        let force = flags & DoCommandFlag::FORCE_CLEAR_TILE != 0;
        if flags & DoCommandFlag::EXECUTE != 0 {
            (self.rail().delete_depot)(tile);
            let owner = (self.s.tile_owner)(tile);
            if (self.s.company_valid)(owner) {
                let mut amount = 0u32.wrapping_sub(2 * FACTOR);
                if force && self.class(tile) == 1 {
                    amount = amount.wrapping_sub(1);
                }
                self.infra(owner, amount, true);
            }
            if !force {
                self.restore(tile, (self.s.tile_owner)(tile));
            }
            self.restore(tile2, (self.s.tile_owner)(tile2));
            w::hit(w::WATER_CMD_REMOVE_DEPOT);
            if force {
                w::hit(w::WATER_CMD_FORCE);
            }
        }
        self.expense(out, crate::company::price(32));
    }
    fn do_build_lock(
        &self,
        out: *mut CommandCost,
        ret: *mut CommandCost,
        flags: u16,
        tile: u32,
        dir: u8,
    ) {
        self.expense(out, 0);
        let delta = self.map.offs_by_diag_dir(dir);
        let lower = tile.wrapping_sub(delta);
        let upper = tile.wrapping_add(delta);
        for t in [tile, upper, lower] {
            self.no_vehicle(ret, t);
            if self.take_failure(out, ret) {
                return;
            }
        }
        let middle_class = if (self.s.has_water_ground)(tile) {
            self.class(tile)
        } else {
            1
        };
        self.clear_command(ret, tile, flags);
        if self.take_failure(out, ret) {
            return;
        }
        (self.c.amount)(out, (self.c.money)(ret));
        let mut classes = [middle_class, 0, 0];
        for (index, t) in [(1, lower), (2, upper)] {
            if !(self.s.is_water_tile)(t) {
                self.clear_command(ret, t, flags);
                if self.take_failure(out, ret) {
                    return;
                }
                (self.c.amount)(out, (self.c.money)(ret));
                (self.c.amount)(out, crate::company::price(60));
            }
            if !self.flat(t) {
                self.error(out, self.a.strings.sloped_wrong);
                return;
            }
            classes[index] = if (self.s.is_water_tile)(t) {
                self.class(t)
            } else {
                1
            };
        }
        for (part, t) in [tile, lower, upper].into_iter().enumerate() {
            if (self.s.bridge_above)(t)
                && self.bridge_height(t)
                    < (self.s.tile_max_z)(t) + get_lock_part_minimal_bridge_height(part)
            {
                // The original error deliberately uses the MIDDLE tile height.
                let diff = ((self.s.tile_max_z)(tile) + get_lock_part_minimal_bridge_height(part))
                    .wrapping_sub(self.bridge_height(t))
                    .wrapping_mul(TILE_HEIGHT_STEP);
                (self.c.error_param)(out, self.a.strings.low_bridge, diff as u64);
                return;
            }
        }
        if flags & DoCommandFlag::EXECUTE != 0 {
            let owner = (self.s.current_company)();
            if (self.s.company_valid)(owner) {
                if !(self.s.is_water_tile)(lower) {
                    self.infra(owner, 1, false);
                }
                if !(self.s.is_water_tile)(upper) {
                    self.infra(owner, 1, false);
                }
                self.infra(owner, 3 * FACTOR, true);
            }
            for (part, t) in [tile, lower, upper].into_iter().enumerate() {
                let o = if part != 0 && (self.s.is_water_tile)(t) {
                    (self.s.tile_owner)(t)
                } else {
                    owner
                };
                (self.a.make_lock_tile)(
                    t,
                    o,
                    LockPart(part as u8),
                    crate::services::DiagDirection(dir),
                    WaterClass(classes[part]),
                );
            }
            self.check_docking(lower);
            self.check_docking(upper);
            for t in [tile, lower, upper] {
                (self.s.mark_dirty)(t);
            }
            self.dirty_neighbours(lower);
            self.dirty_neighbours(upper);
            unsafe {
                self.water.invalidate_region(lower, self.s);
            }
            unsafe {
                self.water.invalidate_region(upper, self.s);
            }
            w::hit(w::WATER_CMD_LOCK);
        }
        (self.c.amount)(out, crate::company::price(64));
    }
    fn build_lock(&self, out: *mut CommandCost, ret: *mut CommandCost, flags: u16, tile: u32) {
        let dir = get_inclined_slope_direction((self.s.tile_slope)(tile));
        if dir == INVALID_DIAGDIR {
            self.error(out, self.a.strings.sloped_wrong);
            return;
        }
        let lower = tile.wrapping_sub(self.map.offs_by_diag_dir(dir));
        if !(self.a.environment.freeform_edges)() && self.edge(lower) {
            self.error(out, self.a.strings.edge);
            return;
        }
        self.do_build_lock(out, ret, flags, tile, dir);
    }
    fn remove_lock(&self, out: *mut CommandCost, ret: *mut CommandCost, flags: u16, tile: u32) {
        if (self.s.tile_owner)(tile) != OWNER_NONE {
            self.ownership(ret, tile);
            if self.take_failure(out, ret) {
                return;
            }
        }
        let delta = self.map.offs_by_diag_dir((self.a.lock_direction)(tile).0);
        let upper = tile.wrapping_add(delta);
        let lower = tile.wrapping_sub(delta);
        for t in [tile, upper, lower] {
            self.no_vehicle(ret, t);
            if self.take_failure(out, ret) {
                return;
            }
        }
        if flags & DoCommandFlag::EXECUTE != 0 {
            let owner = (self.s.tile_owner)(tile);
            if (self.s.company_valid)(owner) {
                self.infra(owner, 0u32.wrapping_sub(3 * FACTOR), true);
            }
            if self.class(tile) == 2 {
                (self.a.restoration.make_river)(tile, self.random());
            } else {
                crate::ground::clear_square(tile, self.s);
                self.wake_neighbours(tile);
            }
            self.restore(upper, (self.s.tile_owner)(upper));
            self.restore(lower, (self.s.tile_owner)(lower));
            for t in [tile, lower, upper] {
                self.dirty_neighbours(t);
            }
            w::hit(w::WATER_CMD_REMOVE_LOCK);
        }
        self.expense(out, crate::company::price(65));
    }
    fn desert(&self, tile: u32) {
        let mut iter = crate::spiral::square(
            tile % self.size_x,
            tile / self.size_x,
            RIVER_OFFSET_DESERT_DISTANCE,
            self.size_x,
            self.size_y,
        );
        while !iter.is_end() {
            let t = iter.y * self.size_x + iter.x;
            if (self.s.tropic_zone)(t) == 1 {
                (self.s.set_tropic_zone)(t, 0);
                w::hit(w::WATER_CMD_DESERT);
            }
            iter = iter.advance(self.size_x, self.size_y);
        }
    }
    unsafe fn build_canal(
        &self,
        out: *mut CommandCost,
        ret: *mut CommandCost,
        area: *mut Area,
        flags: u16,
        end: u32,
        start: u32,
        wc: WaterClass,
        diagonal: bool,
    ) {
        if start >= self.size_x * self.size_y
            || wc.0 >= 3
            || (wc.0 != 1 && !(self.a.environment.editor)())
        {
            self.error(out, INVALID_STRING_ID);
            return;
        }
        self.expense(out, 0);
        // SAFETY: The facade supplies a live, independent stack area iterator.
        let mut tile = unsafe {
            (self.a.area_begin)(area, end, start, diagonal);
            (self.a.area_tile)(area)
        };
        while tile != INVALID_TILE {
            let slope = (self.s.tile_slope)(tile);
            if slope != 0 && (wc.0 != 2 || !is_inclined_slope(slope)) {
                self.error(out, self.a.strings.flat);
                return;
            }
            let water = (self.s.is_water_tile)(tile);
            if !(water
                && self.canal(tile)
                && !(self.a.environment.editor)()
                && ((self.s.tile_owner)(tile) == (self.s.current_company)()
                    || (self.s.tile_owner)(tile) == OWNER_NONE))
            {
                self.clear_command(ret, tile, flags);
                if self.take_failure(out, ret) {
                    return;
                }
                if !water {
                    (self.c.amount)(out, (self.c.money)(ret));
                }
                if flags & DoCommandFlag::EXECUTE != 0 {
                    if (self.s.tile_type)(tile) == 6 && self.canal(tile) {
                        let owner = (self.s.tile_owner)(tile);
                        if (self.s.company_valid)(owner) {
                            self.infra(owner, u32::MAX, true);
                        }
                    }
                    match wc.0 {
                        2 => {
                            (self.a.restoration.make_river)(tile, self.random());
                            if (self.a.environment.editor)() {
                                self.desert(tile);
                            }
                        }
                        0 if (self.s.tile_height)(tile) == 0 => (self.water.make_sea)(tile),
                        _ => {
                            let owner = (self.s.current_company)();
                            (self.a.restoration.make_canal)(tile, owner, self.random());
                            if (self.s.company_valid)(owner) {
                                self.infra(owner, 1, true);
                            }
                        }
                    }
                    (self.s.mark_dirty)(tile);
                    self.dirty_neighbours(tile);
                    self.check_docking(tile);
                    w::hit(w::WATER_CMD_CANAL);
                }
                (self.c.amount)(out, crate::company::price(60));
            } else {
                w::hit(w::WATER_CMD_SKIP);
            }
            // SAFETY: Iterator remains live, and callbacks cannot replace it.
            unsafe {
                (self.a.area_next)(area);
                tile = (self.a.area_tile)(area);
            }
        }
        if (self.c.money)(out) == 0 {
            self.error(out, self.a.strings.already_built);
        }
    }
    fn clear(&self, out: *mut CommandCost, ret: *mut CommandCost, flags: u16, tile: u32) {
        match self.kind(tile) {
            0 => {
                if flags & DoCommandFlag::NO_WATER != 0 {
                    self.error(out, self.a.strings.build_on_water);
                    return;
                }
                let price = if self.canal(tile) {
                    crate::company::price(61)
                } else {
                    crate::company::price(34)
                };
                if !(self.a.environment.freeform_edges)() && self.edge(tile) {
                    self.error(out, self.a.strings.edge);
                    return;
                }
                self.no_vehicle(ret, tile);
                if self.take_failure(out, ret) {
                    return;
                }
                let owner = (self.s.tile_owner)(tile);
                if owner != OWNER_WATER && owner != OWNER_NONE {
                    self.ownership(ret, tile);
                    if self.take_failure(out, ret) {
                        return;
                    }
                }
                if flags & DoCommandFlag::EXECUTE != 0 {
                    if self.canal(tile) && (self.s.company_valid)(owner) {
                        self.infra(owner, u32::MAX, true);
                    }
                    crate::ground::clear_square(tile, self.s);
                    self.dirty_neighbours(tile);
                    self.wake_neighbours(tile);
                    w::hit(w::WATER_CMD_CLEAR);
                }
                self.expense(out, price);
            }
            1 => {
                let slope = (self.s.tile_slope)(tile);
                self.no_vehicle(ret, tile);
                if self.take_failure(out, ret) {
                    return;
                }
                if flags & DoCommandFlag::EXECUTE != 0 {
                    crate::ground::clear_square(tile, self.s);
                    self.dirty_neighbours(tile);
                    self.wake_neighbours(tile);
                }
                self.expense(
                    out,
                    if is_slope_with_one_corner_raised(slope) {
                        crate::company::price(34)
                    } else {
                        crate::company::price(23)
                    },
                );
            }
            2 => {
                if flags & DoCommandFlag::AUTO != 0 {
                    self.error(out, self.a.strings.demolish_building);
                    return;
                }
                if (self.s.current_company)() == OWNER_WATER {
                    self.error(out, INVALID_STRING_ID);
                    return;
                }
                let part = (self.a.lock_part)(tile).0;
                let delta = self.map.offs_by_diag_dir((self.a.lock_direction)(tile).0);
                let middle = match part {
                    0 => tile,
                    1 => tile.wrapping_add(delta),
                    2 => tile.wrapping_sub(delta),
                    _ => unreachable!(),
                };
                self.remove_lock(out, ret, flags, middle);
            }
            3 => {
                if flags & DoCommandFlag::AUTO != 0 {
                    self.error(out, self.a.strings.demolish_building);
                    return;
                }
                self.remove_depot(out, ret, flags, tile);
            }
            _ => unreachable!("original water subtype"),
        }
    }
    fn owner(&self, ret: *mut CommandCost, tile: u32, old: u8, new: u8) {
        if (self.s.tile_owner)(tile) != old {
            return;
        }
        let middle = (self.s.water_middle_lock)(tile);
        if middle {
            self.infra(old, 0u32.wrapping_sub(3 * FACTOR), false);
        }
        if new != INVALID_OWNER {
            if middle {
                self.infra(new, 3 * FACTOR, false);
            }
            if self.class(tile) == 1 && !middle {
                self.infra(old, u32::MAX, false);
                self.infra(new, 1, false);
            }
            if self.kind(tile) == 3 {
                self.infra(old, 0u32.wrapping_sub(FACTOR), false);
                self.infra(new, FACTOR, false);
            }
            (self.s.set_tile_owner)(tile, new);
            return;
        }
        if self.kind(tile) == 3 {
            self.clear_command(ret, tile, DoCommandFlag::EXECUTE | DoCommandFlag::BANKRUPT);
        }
        if (self.s.tile_owner)(tile) == old {
            if self.class(tile) == 1 && !middle {
                self.infra(old, u32::MAX, false);
            }
            (self.s.set_tile_owner)(tile, OWNER_NONE);
        }
        w::hit(w::WATER_CMD_OWNER);
    }
}

// All exported entries share this contract: native stack result slots are live,
// independent and never dereferenced by Rust; tables are immutable for the call.
// No owner/map borrow spans a command, docking call or clear-square mutation.
macro_rules! entry {
    ($name:ident($($arg:ident:$ty:ty),*) $body:expr) => {
        /// # Safety
        /// Live immutable native tables, independent constructed result/iterator
        /// storage and original tile preconditions under the game-state mutex.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(a:*const WaterCommandServices,$($arg:$ty),*) {
            // SAFETY: The native facade upholds the common synchronous contract.
            let world=unsafe { World::new(a) };
            $body(&world);
        }
    };
}
entry!(openttd_rust_water_build_depot(out:*mut CommandCost,ret:*mut CommandCost,flags:u16,tile:u32,axis:Axis)
    |world:&World<'_>| world.build_depot(out,ret,flags,tile,axis));
entry!(openttd_rust_water_build_lock(out:*mut CommandCost,ret:*mut CommandCost,flags:u16,tile:u32)
    |world:&World<'_>| world.build_lock(out,ret,flags,tile));
entry!(openttd_rust_water_build_canal(out:*mut CommandCost,ret:*mut CommandCost,area:*mut Area,flags:u16,end:u32,start:u32,wc:WaterClass,diagonal:bool)
|world:&World<'_>| { // SAFETY: The facade owns the iterator and native result slots.
    unsafe { world.build_canal(out,ret,area,flags,end,start,wc,diagonal); }
});
entry!(openttd_rust_water_make_river(tile:u32) |world:&World<'_>| {
    (world.a.restoration.make_river)(tile,world.random()); (world.s.mark_dirty)(tile); world.desert(tile);
});
entry!(openttd_rust_water_clear(out:*mut CommandCost,ret:*mut CommandCost,flags:u16,tile:u32)
    |world:&World<'_>| world.clear(out,ret,flags,tile));
entry!(openttd_rust_water_terraform(out:*mut CommandCost,flags:u16,tile:u32) |world:&World<'_>| {
    if (world.s.is_water_tile)(tile) && world.canal(tile) { world.error(out,world.a.strings.demolish_canal); }
    else { world.clear_command(out,tile,flags); }
});
entry!(openttd_rust_water_build_above(out:*mut CommandCost,flags:u16,tile:u32,height:i32) |world:&World<'_>| {
    match world.kind(tile) {
        0|1=>(world.c.zero)(out),
        2=>{
            let required=(world.s.tile_max_z)(tile)+get_lock_part_minimal_bridge_height(usize::from((world.a.lock_part)(tile).0));
            if required<=height { (world.c.zero)(out); }
            else { (world.c.error_param)(out,world.a.strings.low_bridge,required.wrapping_sub(height).wrapping_mul(TILE_HEIGHT_STEP) as u64); }
        },
        _=>world.clear_command(out,tile,flags),
    }
});
/// Original `ChangeTileOwner_Water`, called by the ownership dispatcher.
/// # Safety
/// As for the native entries: live immutable tables and an independent result.
pub(crate) unsafe fn openttd_rust_water_change_owner(
    a: *const WaterCommandServices,
    ret: *mut CommandCost,
    tile: u32,
    old: u8,
    new: u8,
) {
    // SAFETY: The ownership dispatcher upholds the common synchronous contract.
    let world = unsafe { World::new(a) };
    world.owner(ret, tile, old, new);
}
