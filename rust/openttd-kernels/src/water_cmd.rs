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
use crate::services::{
    CommandCost, CommandCostServices, CommandServices, FoundationSlope, Services, WaterClass,
};
use crate::{water::WaterServices, witness as w};

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
    pub water_type: extern "C" fn(u32) -> WaterTileType,
    pub lock_part: extern "C" fn(u32) -> LockPart,
    pub lock_direction: extern "C" fn(u32) -> crate::services::DiagDirection,
    pub other_depot_tile: extern "C" fn(u32) -> u32,
    pub slope_z: extern "C" fn(u32) -> FoundationSlope,
    pub environment: crate::services::EnvironmentServices,
    pub infrastructure: extern "C" fn(u8, u32),
    pub make_depot: extern "C" fn(u32, u8, u16, DepotPart, Axis, WaterClass),
    pub make_lock_tile:
        extern "C" fn(u32, u8, LockPart, crate::services::DiagDirection, WaterClass),
    pub make_canal: extern "C" fn(u32, u8, u8),
    pub make_river: extern "C" fn(u32, u8),
    pub bridge_height_above: extern "C" fn(u32) -> i32,
    pub area_tile: unsafe extern "C" fn(*mut Area) -> u32,
    pub area_next: unsafe extern "C" fn(*mut Area),
    pub build_depot_price: extern "C" fn() -> i64,
    pub clear_depot_price: extern "C" fn() -> i64,
    pub build_lock_price: extern "C" fn() -> i64,
    pub clear_lock_price: extern "C" fn() -> i64,
    pub build_canal_price: extern "C" fn() -> i64,
    pub clear_canal_price: extern "C" fn() -> i64,
    pub clear_water_price: extern "C" fn() -> i64,
    pub clear_rough_price: extern "C" fn() -> i64,
    pub strings: Strings,
}
const EXEC: u16 = 1;
const AUTO: u16 = 2;
const NO_WATER: u16 = 8;
const BANKRUPT: u16 = 32;
const FORCE: u16 = 1024;
const NONE: u8 = 16;
const WATER: u8 = 17;
const INVALID_OWNER: u8 = 255;
const FACTOR: u32 = 2;
const INVALID_STRING: u32 = 0xffff;

struct World<'a> {
    a: &'a WaterCommandServices,
    s: &'a Services,
    c: &'a CommandCostServices,
    cmd: &'a CommandServices,
    water: &'a WaterServices,
    size_x: u32,
    size_y: u32,
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
        Self {
            a,
            s,
            c,
            cmd,
            water,
            size_x: (s.map_max_x)() + 1,
            size_y: (s.map_max_y)() + 1,
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
        (self.cmd.landscape_clear)(ret, tile, flags);
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
    fn kind(&self, t: u32) -> u8 {
        (self.a.water_type)(t).0
    }
    fn flat(&self, t: u32) -> bool {
        (self.s.tile_slope)(t) == 0
    }
    fn canal(&self, t: u32) -> bool {
        self.kind(t) == 0 && self.class(t) == 1
    }
    fn delta(&self, dir: u8) -> u32 {
        match dir {
            0 => u32::MAX,
            1 => self.size_x,
            2 => 1,
            3 => 0u32.wrapping_sub(self.size_x),
            _ => unreachable!(),
        }
    }
    fn inclined(slope: u8) -> u8 {
        match slope {
            12 => 0,
            6 => 1,
            3 => 2,
            9 => 3,
            _ => 255,
        }
    }
    fn edge(&self, t: u32) -> bool {
        let x = t % self.size_x;
        let y = t / self.size_x;
        !(1..self.size_x - 2).contains(&x) || !(1..self.size_y - 2).contains(&y)
    }
    fn infra(&self, owner: u8, delta: u32, dirty: bool) {
        (self.a.infrastructure)(owner, delta);
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
            crate::water::openttd_rust_dirty_water_neighbours(
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
        let mut wc = self.class(tile);
        let slope = (self.a.slope_z)(tile);
        if slope.slope.0 != 0 {
            if wc == 1 {
                if (self.s.company_valid)(owner) {
                    self.infra(owner, u32::MAX, true);
                }
                wc = 3;
            }
            if wc != 2 || Self::inclined(slope.slope.0) == 255 {
                wc = 3;
            }
        }
        if wc == 0 && slope.z > 0 {
            if (self.s.company_valid)(owner) {
                self.infra(owner, 1, true);
            }
            wc = 1;
        }
        (self.s.clear_square)(tile);
        match wc {
            0 => (self.water.make_sea)(tile),
            1 => (self.a.make_canal)(tile, owner, self.random()),
            2 => (self.a.make_river)(tile, self.random()),
            _ => {}
        }
        if wc != 3 {
            (self.water.check_docking)(tile);
        }
        (self.s.mark_dirty)(tile);
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
            self.error(out, INVALID_STRING);
            return;
        }
        let tile2 = tile.wrapping_add(if axis.0 == 0 { 1 } else { self.size_x });
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
        if !(self.cmd.depot_available)() {
            self.error(out, INVALID_STRING);
            return;
        }
        let wc1 = self.class(tile);
        let wc2 = self.class(tile2);
        self.expense(out, (self.a.build_depot_price)());
        for t in [tile, tile2] {
            let add = !(self.s.is_water_tile)(t);
            self.clear_command(ret, t, flags | AUTO);
            if self.take_failure(out, ret) {
                return;
            }
            if add {
                (self.c.amount)(out, (self.c.money)(ret));
            }
        }
        if flags & EXEC != 0 {
            let id = (self.cmd.new_depot)(tile);
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
            (self.water.check_docking)(tile);
            (self.water.check_docking)(tile2);
            (self.s.mark_dirty)(tile);
            (self.s.mark_dirty)(tile2);
            (self.cmd.depot_name)(id);
            w::hit(w::WATER_CMD_DEPOT);
        }
    }
    fn remove_depot(&self, out: *mut CommandCost, ret: *mut CommandCost, flags: u16, tile: u32) {
        if self.kind(tile) != 3 {
            self.error(out, INVALID_STRING);
            return;
        }
        self.ownership(ret, tile);
        if self.take_failure(out, ret) {
            return;
        }
        let tile2 = (self.a.other_depot_tile)(tile);
        if flags & BANKRUPT == 0 {
            self.no_vehicle(ret, tile);
            if !self.failed(ret) {
                self.no_vehicle(ret, tile2);
            }
            if self.take_failure(out, ret) {
                return;
            }
        }
        let force = flags & FORCE != 0;
        if flags & EXEC != 0 {
            (self.cmd.delete_depot)(tile);
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
        self.expense(out, (self.a.clear_depot_price)());
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
        let delta = self.delta(dir);
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
                (self.c.amount)(out, (self.a.build_canal_price)());
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
                && (self.a.bridge_height_above)(t) < (self.s.tile_max_z)(t) + [2, 3, 2][part]
            {
                // The original error deliberately uses the MIDDLE tile height.
                let diff = ((self.s.tile_max_z)(tile) + [2, 3, 2][part]
                    - (self.a.bridge_height_above)(t))
                    * 50;
                (self.c.error_param)(out, self.a.strings.low_bridge, diff as u64);
                return;
            }
        }
        if flags & EXEC != 0 {
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
            (self.water.check_docking)(lower);
            (self.water.check_docking)(upper);
            for t in [tile, lower, upper] {
                (self.s.mark_dirty)(t);
            }
            self.dirty_neighbours(lower);
            self.dirty_neighbours(upper);
            (self.water.invalidate_region)(lower);
            (self.water.invalidate_region)(upper);
            w::hit(w::WATER_CMD_LOCK);
        }
        (self.c.amount)(out, (self.a.build_lock_price)());
    }
    fn build_lock(&self, out: *mut CommandCost, ret: *mut CommandCost, flags: u16, tile: u32) {
        let dir = Self::inclined((self.s.tile_slope)(tile));
        if dir == 255 {
            self.error(out, self.a.strings.sloped_wrong);
            return;
        }
        let lower = tile.wrapping_sub(self.delta(dir));
        if !(self.a.environment.freeform_edges)() && self.edge(lower) {
            self.error(out, self.a.strings.edge);
            return;
        }
        self.do_build_lock(out, ret, flags, tile, dir);
    }
    fn remove_lock(&self, out: *mut CommandCost, ret: *mut CommandCost, flags: u16, tile: u32) {
        if (self.s.tile_owner)(tile) != NONE {
            self.ownership(ret, tile);
            if self.take_failure(out, ret) {
                return;
            }
        }
        let delta = self.delta((self.a.lock_direction)(tile).0);
        let upper = tile.wrapping_add(delta);
        let lower = tile.wrapping_sub(delta);
        for t in [tile, upper, lower] {
            self.no_vehicle(ret, t);
            if self.take_failure(out, ret) {
                return;
            }
        }
        if flags & EXEC != 0 {
            let owner = (self.s.tile_owner)(tile);
            if (self.s.company_valid)(owner) {
                self.infra(owner, 0u32.wrapping_sub(3 * FACTOR), true);
            }
            if self.class(tile) == 2 {
                (self.a.make_river)(tile, self.random());
            } else {
                (self.s.clear_square)(tile);
                self.wake_neighbours(tile);
            }
            self.restore(upper, (self.s.tile_owner)(upper));
            self.restore(lower, (self.s.tile_owner)(lower));
            for t in [tile, lower, upper] {
                self.dirty_neighbours(t);
            }
            w::hit(w::WATER_CMD_REMOVE_LOCK);
        }
        self.expense(out, (self.a.clear_lock_price)());
    }
    fn desert(&self, tile: u32) {
        let mut iter = crate::spiral::square(
            tile % self.size_x,
            tile / self.size_x,
            5,
            self.size_x,
            self.size_y,
        );
        while !iter.is_end() {
            let t = iter.y * self.size_x + iter.x;
            if (self.s.tropic_zone)(t) == 1 {
                (self.s.set_tropic_zone)(t, 0);
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
        start: u32,
        wc: WaterClass,
    ) {
        if start >= self.size_x * self.size_y
            || wc.0 >= 3
            || (wc.0 != 1 && !(self.a.environment.editor)())
        {
            self.error(out, INVALID_STRING);
            return;
        }
        self.expense(out, 0);
        // SAFETY: The facade supplies a live, independent stack area iterator.
        let mut tile = unsafe { (self.a.area_tile)(area) };
        while tile != u32::MAX {
            let slope = (self.s.tile_slope)(tile);
            if slope != 0 && (wc.0 != 2 || Self::inclined(slope) == 255) {
                self.error(out, self.a.strings.flat);
                return;
            }
            let water = (self.s.is_water_tile)(tile);
            if !(water
                && self.canal(tile)
                && !(self.a.environment.editor)()
                && ((self.s.tile_owner)(tile) == (self.s.current_company)()
                    || (self.s.tile_owner)(tile) == NONE))
            {
                self.clear_command(ret, tile, flags);
                if self.take_failure(out, ret) {
                    return;
                }
                if !water {
                    (self.c.amount)(out, (self.c.money)(ret));
                }
                if flags & EXEC != 0 {
                    if (self.s.tile_type)(tile) == 6 && self.canal(tile) {
                        let owner = (self.s.tile_owner)(tile);
                        if (self.s.company_valid)(owner) {
                            self.infra(owner, u32::MAX, true);
                        }
                    }
                    match wc.0 {
                        2 => {
                            (self.a.make_river)(tile, self.random());
                            if (self.a.environment.editor)() {
                                self.desert(tile);
                            }
                        }
                        0 if (self.s.tile_height)(tile) == 0 => (self.water.make_sea)(tile),
                        _ => {
                            let owner = (self.s.current_company)();
                            (self.a.make_canal)(tile, owner, self.random());
                            if (self.s.company_valid)(owner) {
                                self.infra(owner, 1, true);
                            }
                        }
                    }
                    (self.s.mark_dirty)(tile);
                    self.dirty_neighbours(tile);
                    (self.water.check_docking)(tile);
                    w::hit(w::WATER_CMD_CANAL);
                }
                (self.c.amount)(out, (self.a.build_canal_price)());
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
                if flags & NO_WATER != 0 {
                    self.error(out, self.a.strings.build_on_water);
                    return;
                }
                let price = if self.canal(tile) {
                    (self.a.clear_canal_price)()
                } else {
                    (self.a.clear_water_price)()
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
                if owner != WATER && owner != NONE {
                    self.ownership(ret, tile);
                    if self.take_failure(out, ret) {
                        return;
                    }
                }
                if flags & EXEC != 0 {
                    if self.canal(tile) && (self.s.company_valid)(owner) {
                        self.infra(owner, u32::MAX, true);
                    }
                    (self.s.clear_square)(tile);
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
                if flags & EXEC != 0 {
                    (self.s.clear_square)(tile);
                    self.dirty_neighbours(tile);
                    self.wake_neighbours(tile);
                }
                self.expense(
                    out,
                    if matches!(slope, 1 | 2 | 4 | 8) {
                        (self.a.clear_water_price)()
                    } else {
                        (self.a.clear_rough_price)()
                    },
                );
            }
            2 => {
                if flags & AUTO != 0 {
                    self.error(out, self.a.strings.demolish_building);
                    return;
                }
                if (self.s.current_company)() == WATER {
                    self.error(out, INVALID_STRING);
                    return;
                }
                let part = (self.a.lock_part)(tile).0;
                let delta = self.delta((self.a.lock_direction)(tile).0);
                let middle = match part {
                    0 => tile,
                    1 => tile.wrapping_add(delta),
                    2 => tile.wrapping_sub(delta),
                    _ => unreachable!(),
                };
                self.remove_lock(out, ret, flags, middle);
            }
            3 => {
                if flags & AUTO != 0 {
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
        let middle = self.kind(tile) == 2 && (self.a.lock_part)(tile).0 == 0;
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
            self.clear_command(ret, tile, EXEC | BANKRUPT);
        }
        if (self.s.tile_owner)(tile) == old {
            if self.class(tile) == 1 && !middle {
                self.infra(old, u32::MAX, false);
            }
            (self.s.set_tile_owner)(tile, NONE);
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
entry!(openttd_rust_water_build_canal(out:*mut CommandCost,ret:*mut CommandCost,area:*mut Area,flags:u16,start:u32,wc:WaterClass)
|world:&World<'_>| { // SAFETY: The facade owns the iterator and native result slots.
    unsafe { world.build_canal(out,ret,area,flags,start,wc); }
});
entry!(openttd_rust_water_restore(tile:u32,owner:u8) |world:&World<'_>| world.restore(tile,owner));
entry!(openttd_rust_water_make_river(tile:u32) |world:&World<'_>| {
    (world.a.make_river)(tile,world.random()); (world.s.mark_dirty)(tile); world.desert(tile);
});
entry!(openttd_rust_water_clear(out:*mut CommandCost,ret:*mut CommandCost,flags:u16,tile:u32)
    |world:&World<'_>| world.clear(out,ret,flags,tile));
entry!(openttd_rust_water_change_owner(ret:*mut CommandCost,tile:u32,old:u8,new:u8)
    |world:&World<'_>| world.owner(ret,tile,old,new));
entry!(openttd_rust_water_terraform(out:*mut CommandCost,flags:u16,tile:u32) |world:&World<'_>| {
    if (world.s.is_water_tile)(tile) && world.canal(tile) { world.error(out,world.a.strings.demolish_canal); }
    else { world.clear_command(out,tile,flags); }
});
entry!(openttd_rust_water_build_above(out:*mut CommandCost,flags:u16,tile:u32,height:i32) |world:&World<'_>| {
    match world.kind(tile) {
        0|1=>(world.c.zero)(out),
        2=>{
            let required=(world.s.tile_max_z)(tile)+[2,3,2][usize::from((world.a.lock_part)(tile).0)];
            if required<=height { (world.c.zero)(out); }
            else { (world.c.error_param)(out,world.a.strings.low_bridge,((required-height)*50) as u64); }
        },
        _=>world.clear_command(out,tile,flags),
    }
});
