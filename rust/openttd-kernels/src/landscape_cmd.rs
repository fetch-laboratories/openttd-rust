/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Whole landscape clear, terraform and level transactions. Ordered command-local
//! scratch matches `std::map/set`; canonical map writes and tile owners stay shared.
//! No borrow of shared scratch or finance state survives a reentrant command.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::semicolon_if_nothing_returned,
    clippy::if_not_else
)]
use crate::command_type::DoCommandFlag as F;
use crate::map_type::{LM_LEVEL, LM_LOWER, LM_RAISE};
use crate::services::{CommandCost, CommandCostServices, CommandServices, Cost, Map, Services};
use crate::slope_func::{SLOPE_E, SLOPE_FLAT, SLOPE_N, SLOPE_S, SLOPE_STEEP, SLOPE_W};
use crate::strings_type::INVALID_STRING_ID;
use crate::tile_type::{
    INVALID_TILE, MP_CLEAR, MP_HOUSE, MP_INDUSTRY, MP_OBJECT, MP_RAILWAY, MP_ROAD, MP_STATION,
    MP_TREES, MP_VOID, MP_WATER,
};
use crate::water_type::WATER_CLASS_CANAL;
use std::collections::{BTreeMap, BTreeSet};
use std::ptr::{addr_of, addr_of_mut};

#[repr(C)]
pub struct Strings {
    pub canal: u32,
    pub clear_limit: u32,
    pub water: u32,
    pub sea_level: u32,
    pub high: u32,
    pub edge: u32,
    pub bridge: u32,
    pub bridge_high: u32,
    pub tunnel: u32,
    pub terraform_limit: u32,
    pub levelled: u32,
}
#[repr(C)]
pub struct LandscapeServices {
    pub shared: *const Services,
    pub costs: *const CommandCostServices,
    pub commands: *const CommandServices,
    pub strings: Strings,
    pub tile_procs: *const TileServices,
    pub water: *const crate::water_cmd::WaterCommandServices,
    pub on_water: extern "C" fn(u32) -> bool,
    pub coast: extern "C" fn(u32) -> bool,
    pub buoy: extern "C" fn(u32) -> bool,
    pub canal: extern "C" fn(u32) -> bool,
    pub height_limit: extern "C" fn() -> u8,
    pub max_bridge_height: extern "C" fn() -> u8,
    pub freeform_edges: extern "C" fn() -> bool,
    pub editor: extern "C" fn() -> bool,
    pub exchange_generating: extern "C" fn(bool) -> bool,
    pub tunnel_in_way: extern "C" fn(u32, i32) -> bool,
    pub mark_dirty_height: extern "C" fn(u32, i32),
    pub set_height: extern "C" fn(u32, u32),
    pub paused: extern "C" fn() -> bool,
    pub explosion: extern "C" fn(u32, bool),
}
/// Immutable canonical tile-owner tables, resolved without copying world records.
#[repr(C)]
pub struct TileServices {
    pub clear: extern "C" fn(*mut CommandCost, u32, u16),
    pub terraform: extern "C" fn(*mut CommandCost, u32, u16, i32, u8),
    pub owners: *const crate::tile_owner::Services,
    pub trees: *const crate::trees::TreeServices,
    pub industry_policy: *const crate::industry::TilePolicyServices,
    pub industry_costs: *const crate::industry::IndustryCostServices,
    pub industry_generation: *const crate::industry::GenerationServices,
}
struct World<'a> {
    a: &'a LandscapeServices,
    s: &'a Services,
    c: &'a CommandCostServices,
    cmd: &'a CommandServices,
    map: Map,
    size: u32,
}
impl<'a> World<'a> {
    unsafe fn new(a: &'a LandscapeServices) -> Self {
        let (s, c, cmd) = unsafe { (&*a.shared, &*a.costs, &*a.commands) };
        let map = s.map();
        Self {
            a,
            s,
            c,
            cmd,
            map,
            size: ((s.map_max_y)() + 1) << map.log_x,
        }
    }
    fn cost(&self, out: *mut CommandCost) -> Cost<'_> {
        Cost(out, self.c)
    }
    fn finance(&self, flags: u16, clear: bool) -> *mut crate::company::Finances {
        if clear && flags & (F::AUTO | F::BANKRUPT) != 0 {
            return std::ptr::null_mut();
        }
        (self.cmd.finance.company_finances)((self.cmd.finance.current_company)())
    }
    fn clear_limit(c: *mut crate::company::Finances) -> i32 {
        if c.is_null() {
            i32::MAX
        } else {
            (unsafe { addr_of!((*c).clear).read() >> 16 }) as i32
        }
    }
    fn terraform_limit(c: *mut crate::company::Finances) -> i32 {
        if c.is_null() {
            i32::MAX
        } else {
            (unsafe { addr_of!((*c).terraform).read() >> 16 }) as i32
        }
    }
    fn indirectly_cleared(&self, t: u32) -> bool {
        crate::objects::openttd_rust_object_indirectly_cleared(t, u32::from(self.map.log_x))
    }
    fn area<T>(
        &self,
        area: *mut crate::water_cmd::Area,
        tile: u32,
        start: u32,
        diagonal: bool,
        mut visit: impl FnMut(u32) -> Option<T>,
    ) -> Option<T> {
        let water = unsafe { &*self.a.water };
        unsafe {
            (water.area_begin)(area, tile, start, diagonal);
        }
        loop {
            let t = unsafe { (water.area_tile)(area) };
            if t == INVALID_TILE {
                return None;
            }
            if let Some(result) = visit(t) {
                return Some(result);
            }
            unsafe {
                (water.area_next)(area);
            }
        }
    }
    fn clear_tile(&self, out: Cost<'_>, tile: u32, flags: u16) {
        let kind = (self.s.tile_type)(tile);
        if matches!(
            kind,
            MP_RAILWAY | MP_ROAD | MP_STATION | MP_WATER | MP_OBJECT
        ) {
            unsafe {
                /* SAFETY: synchronous costs never escape or outlive this call. */
                crate::command::with_cost(self.c, |ret| {
                    if matches!(kind, MP_ROAD | MP_STATION) {
                        crate::command::with_cost(self.c, |error| {
                            self.clear_tile_with(out, tile, flags, kind, ret.0, error.0)
                        });
                    } else {
                        self.clear_tile_with(out, tile, flags, kind, ret.0, std::ptr::null_mut());
                    }
                })
            };
        } else {
            self.clear_tile_with(
                out,
                tile,
                flags,
                kind,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        }
    }
    fn clear_tile_with(
        &self,
        out: Cost<'_>,
        tile: u32,
        flags: u16,
        kind: u8,
        ret: *mut CommandCost,
        error: *mut CommandCost,
    ) {
        let t = unsafe { &*self.a.tile_procs };
        let owners = unsafe { &*t.owners };
        // SAFETY: the matching caller supplies exactly the independent native
        // result slots used by this tile kind. Other cases do not read the slots.
        unsafe {
            match kind {
                MP_RAILWAY => crate::rail_cmd::openttd_rust_rail_clear_tile(
                    owners.rail,
                    out.0,
                    ret,
                    flags,
                    tile,
                ),
                MP_ROAD => crate::road_cmd::openttd_rust_road_clear(
                    (self.s.road_commands)(),
                    out.0,
                    ret,
                    error,
                    flags,
                    tile,
                ),
                MP_HOUSE => crate::town::house::openttd_rust_house_clear(
                    (*owners.objects).town,
                    self.s,
                    out.0,
                    tile,
                    flags,
                ),
                MP_TREES => out.set(crate::trees::clear_tile(&*t.trees, self.s, tile, flags)),
                MP_STATION => crate::station_cmd::openttd_rust_station_clear(
                    owners.station,
                    out.0,
                    ret,
                    error,
                    tile,
                    flags,
                ),
                MP_WATER => crate::water_cmd::openttd_rust_water_clear(
                    owners.water,
                    out.0,
                    ret,
                    flags,
                    tile,
                ),
                MP_INDUSTRY => crate::industry::openttd_rust_industry_tile_clear(
                    out.0.cast(),
                    tile,
                    flags,
                    owners.industry,
                    t.industry_policy,
                    t.industry_costs,
                    t.industry_generation,
                ),
                MP_OBJECT => crate::objects::openttd_rust_object_clear(
                    owners.objects,
                    out.0,
                    ret,
                    tile,
                    flags,
                ),
                _ => (t.clear)(out.0, tile, flags),
            }
        }
    }
    fn terraform_tile(&self, out: Cost<'_>, tile: u32, flags: u16, z: i32, slope: u8) {
        let t = unsafe { &*self.a.tile_procs };
        let owners = unsafe { &*t.owners };
        unsafe {
            match (self.s.tile_type)(tile) {
                MP_RAILWAY => crate::rail_cmd::openttd_rust_rail_terraform(
                    owners.rail,
                    out.0,
                    flags,
                    tile,
                    z,
                    slope,
                ),
                MP_ROAD => {
                    /* SAFETY: synchronous costs never escape or outlive this call. */
                    crate::command::with_cost(self.c, |ret| {
                        crate::road_cmd::openttd_rust_road_terraform(
                            (self.s.road_commands)(),
                            out.0,
                            ret.0,
                            flags,
                            tile,
                            z,
                            slope,
                        )
                    })
                }
                MP_HOUSE => crate::town::house::openttd_rust_house_terraform(
                    (*owners.objects).town,
                    self.s,
                    out.0,
                    tile,
                    flags,
                    z,
                    slope,
                ),
                MP_CLEAR | MP_TREES => self.clear_do(out, tile, flags),
                MP_STATION => crate::station_cmd::openttd_rust_station_terraform(
                    owners.station,
                    out.0,
                    tile,
                    flags,
                    z,
                    slope,
                ),
                MP_WATER => {
                    crate::water_cmd::openttd_rust_water_terraform(owners.water, out.0, flags, tile)
                }
                MP_INDUSTRY => crate::industry::openttd_rust_industry_tile_terraform(
                    out.0.cast(),
                    tile,
                    flags,
                    z,
                    slope,
                    self.s,
                    owners.industry,
                    t.industry_policy,
                    t.industry_costs,
                ),
                MP_OBJECT => crate::objects::openttd_rust_object_terraform(
                    owners.objects,
                    out.0,
                    tile,
                    flags,
                    z,
                    slope,
                ),
                _ => (t.terraform)(out.0, tile, flags, z, slope),
            }
        }
    }
    fn clear_do(&self, out: Cost<'_>, tile: u32, flags: u16) {
        crate::command::nested(self.cmd, self.s, out, tile, flags, |f| {
            self.clear(out, tile, f)
        });
    }
    fn clear(&self, out: Cost<'_>, tile: u32, flags: u16) {
        out.set(0);
        let water = unsafe { &*(self.s.water_services)() };
        let native = unsafe { &*water.clear };
        let mut do_clear = false;
        if flags & F::FORCE_CLEAR_TILE != 0
            && (self.s.has_water_class)(tile)
            && (self.a.on_water)(tile)
            && !(self.s.is_water_tile)(tile)
            && !(self.a.coast)(tile)
        {
            let class = (native.water_class)(tile);
            if flags & F::AUTO != 0 && class == WATER_CLASS_CANAL {
                out.error(self.a.strings.canal);
                return;
            }
            if !(self.a.buoy)(tile) || (self.s.tile_owner)(tile) == (self.s.current_company)() {
                do_clear = true;
                out.add_money(crate::company::price(if class == WATER_CLASS_CANAL {
                    crate::economy_type::PR_CLEAR_CANAL
                } else {
                    crate::economy_type::PR_CLEAR_WATER
                }));
            }
        }
        let company = self.finance(flags, true);
        if Self::clear_limit(company) < 1 {
            out.error(self.a.strings.clear_limit);
            return;
        }
        if self.indirectly_cleared(tile) {
            if flags & F::NO_WATER != 0 && (self.s.has_water_class)(tile) && (self.a.on_water)(tile)
            {
                out.error(self.a.strings.water);
                return;
            }
        } else {
            unsafe {
                /* SAFETY: synchronous costs never escape or outlive this call. */
                crate::command::with_cost(self.c, |ret| {
                    self.clear_tile(ret, tile, flags);
                    out.add(ret);
                })
            };
        }
        // The original performs these effects even when the tile handler failed.
        if flags & F::EXECUTE != 0 {
            if !company.is_null() {
                unsafe {
                    let p = addr_of_mut!((*company).clear);
                    p.write(p.read().wrapping_sub(1 << 16));
                }
            }
            if do_clear {
                if (self.s.is_water_tile)(tile) && (self.a.canal)(tile) {
                    let owner = (self.s.tile_owner)(tile);
                    if (self.s.company_valid)(owner) {
                        let w = unsafe { &*self.a.water };
                        (w.restoration.infrastructure)(owner, u32::MAX);
                        (self.cmd.infrastructure_dirty)(owner);
                    }
                }
                crate::ground::clear_square(tile, self.s);
                unsafe {
                    crate::water::openttd_rust_clear_neighbour_flooding(
                        tile,
                        self.map.size_x(),
                        (self.s.map_max_y)() + 1,
                        water,
                        self.s,
                    );
                }
            }
        }
    }
    fn clear_area(
        &self,
        out: Cost<'_>,
        last: Cost<'_>,
        ret: Cost<'_>,
        tile: u32,
        start: u32,
        diagonal: bool,
        mut flags: u16,
        area: *mut crate::water_cmd::Area,
    ) -> i64 {
        if start >= self.size {
            out.error(INVALID_STRING_ID);
            return 0;
        }
        let mut money = crate::command::available_money(self.cmd);
        out.set(0);
        last.error(INVALID_STRING_ID);
        let mut success = false;
        let company = self.finance(flags, true);
        let mut limit = Self::clear_limit(company);
        if tile != start {
            flags |= F::FORCE_CLEAR_TILE;
        }
        let extra = self.area(area, tile, start, diagonal, |t| {
            self.clear_do(ret, t, flags & !F::EXECUTE);
            if !ret.ok() {
                if success {
                    crate::witness::hit(crate::witness::LANDSCAPE_CMD_PARTIAL);
                }
                last.take(ret);
                return if !company.is_null() && Self::clear_limit(company) < 1 {
                    Some(0)
                } else {
                    None
                };
            }
            success = true;
            let amount = ret.money();
            if flags & F::EXECUTE != 0 {
                money = money.saturating_sub(amount);
                if amount > 0 && money < 0 {
                    return Some(amount);
                }
                unsafe {
                    /* SAFETY: synchronous costs never escape or outlive this call. */
                    crate::command::with_cost(self.c, |ignored| self.clear_do(ignored, t, flags))
                };
                if (t == tile || t == start) && !(self.a.paused)() {
                    (self.a.explosion)(t, tile == start);
                }
            } else if amount != 0 {
                limit -= 1;
                if limit <= 0 {
                    return Some(0);
                }
            }
            out.add_money(amount);
            None
        });
        if let Some(amount) = extra
            && amount != 0
        {
            return amount;
        }
        if !success {
            out.take(last);
        }
        0
    }
    fn terraform_do(&self, out: Cost<'_>, tile: u32, slope: u8, up: bool, flags: u16) -> u32 {
        crate::command::nested(self.cmd, self.s, out, tile, flags, |f| {
            self.terraform(out, tile, slope, up, f)
        })
    }
    fn terraform(&self, out: Cost<'_>, tile: u32, slope: u8, up: bool, flags: u16) -> u32 {
        out.set(0);
        let direction = if up { 1 } else { -1 };
        let mut ts = TerraformerState::default();
        for (corner, x, y) in [
            (SLOPE_W, 1, 0),
            (SLOPE_S, 1, 1),
            (SLOPE_E, 0, 1),
            (SLOPE_N, 0, 0),
        ] {
            let t = tile.wrapping_add(self.map.diff(x, y));
            if slope & corner == 0 || (corner != SLOPE_N && t >= self.size) {
                continue;
            }
            let result = unsafe {
                /* SAFETY: synchronous costs never escape or outlive this call. */
                crate::command::with_cost(self.c, |ret| {
                    let error = self.tile_height(
                        &mut ts,
                        ret,
                        t,
                        (self.s.tile_height)(t) as i32 + direction,
                    );
                    if ret.ok() {
                        out.add_money(ret.money());
                        None
                    } else {
                        out.take(ret);
                        Some(error)
                    }
                })
            };
            if let Some(error) = result {
                return error;
            }
        }
        for pass in 0..2 {
            if pass == 1 {
                crate::witness::hit(crate::witness::LANDSCAPE_CMD_TWO_PASS);
            }
            for &t in &ts.dirty {
                if (self.s.tile_type)(t) == MP_VOID {
                    continue;
                }
                let heights = [
                    ts.height(self, t),
                    ts.height(self, t.wrapping_add(self.map.diff(1, 0))),
                    ts.height(self, t.wrapping_add(self.map.diff(1, 1))),
                    ts.height(self, t.wrapping_add(self.map.diff(0, 1))),
                ];
                let low = *heights.iter().min().unwrap();
                let high = *heights.iter().max().unwrap();
                let mut new_slope = if high > low + 1 {
                    SLOPE_STEEP
                } else {
                    SLOPE_FLAT
                };
                for (height, corner) in heights
                    .into_iter()
                    .zip([SLOPE_N, SLOPE_W, SLOPE_S, SLOPE_E])
                {
                    if height > low {
                        new_slope |= corner;
                    }
                }
                if pass == 0 {
                    if (self.s.bridge_above)(t) {
                        let station = unsafe { &*(self.s.station_commands)() };
                        let height = crate::bridge_map::get_bridge_height(
                            (station.southern_bridge_end)(t),
                            self.s,
                        );
                        if direction == 1 && height <= high {
                            out.error(self.a.strings.bridge);
                            return t;
                        }
                        if direction == -1 && height > low + i32::from((self.a.max_bridge_height)())
                        {
                            out.error(self.a.strings.bridge_high);
                            return t;
                        }
                    }
                    if direction == -1 && (self.a.tunnel_in_way)(t, low) {
                        out.error(self.a.strings.tunnel);
                        return t;
                    }
                }
                let indirect = self.indirectly_cleared(t);
                let old = (self.s.clear_square_services.generating_world)();
                if (self.a.editor)() {
                    (self.a.exchange_generating)(true);
                }
                let mut tile_flags = flags | F::AUTO | F::FORCE_CLEAR_TILE;
                if pass == 0 {
                    tile_flags = (tile_flags & !F::EXECUTE) | F::NO_MODIFY_TOWN_RATING;
                }
                let ok = unsafe {
                    /* SAFETY: synchronous costs never escape or outlive this call. */
                    crate::command::with_cost(self.c, |ret| {
                        if indirect {
                            crate::witness::hit(crate::witness::LANDSCAPE_CMD_INDIRECT);
                            self.clear_do(ret, t, tile_flags);
                        } else {
                            self.terraform_tile(ret, t, tile_flags, low, new_slope);
                        }
                        (self.a.exchange_generating)(old);
                        if !ret.ok() {
                            out.take(ret);
                            false
                        } else {
                            if pass == 1 {
                                out.add_money(ret.money());
                            }
                            true
                        }
                    })
                };
                if !ok {
                    return t;
                }
            }
        }
        let company = self.finance(flags, false);
        if !company.is_null() && (Self::terraform_limit(company) as usize) < ts.heights.len() {
            out.error(self.a.strings.terraform_limit);
            return INVALID_TILE;
        }
        if flags & F::EXECUTE != 0 {
            for &t in &ts.dirty {
                (self.s.mark_dirty)(t);
                if let Some(&height) = ts.heights.get(&t) {
                    (self.a.mark_dirty_height)(t, height);
                }
            }
            for (&t, &height) in &ts.heights {
                (self.a.set_height)(t, height as u32);
            }
            if !company.is_null() {
                unsafe {
                    let p = addr_of_mut!((*company).terraform);
                    p.write(
                        p.read()
                            .wrapping_sub((ts.heights.len() as u32).wrapping_shl(16)),
                    );
                }
            }
        }
        if out.ok() { tile } else { INVALID_TILE }
    }
    fn tile_height(&self, ts: &mut TerraformerState, out: Cost<'_>, tile: u32, height: i32) -> u32 {
        if height < 0 {
            out.error(self.a.strings.sea_level);
            return INVALID_TILE;
        }
        if height > i32::from((self.a.height_limit)()) {
            out.error(self.a.strings.high);
            return INVALID_TILE;
        }
        if height == ts.height(self, tile) {
            out.error(INVALID_STRING_ID);
            return INVALID_TILE;
        }
        let mut x = self.map.tile_x(tile);
        let mut y = self.map.tile_y(tile);
        if !(self.a.freeform_edges)()
            && (x <= 1 || y <= 1 || x >= (self.s.map_max_x)() - 1 || y >= (self.s.map_max_y)() - 1)
        {
            if x == 1 {
                x = 0;
            }
            if y == 1 {
                y = 0;
            }
            out.error(self.a.strings.edge);
            return y * self.map.size_x() + x;
        }
        if self.map.tile_y(tile) >= 1 {
            ts.dirty.insert(tile.wrapping_add(self.map.diff(0, -1)));
            if self.map.tile_x(tile) >= 1 {
                ts.dirty.insert(tile.wrapping_add(self.map.diff(-1, -1)));
            }
        }
        if self.map.tile_x(tile) >= 1 {
            ts.dirty.insert(tile.wrapping_add(self.map.diff(-1, 0)));
        }
        ts.dirty.insert(tile);
        ts.heights.insert(tile, height);
        out.set(crate::company::price(crate::economy_type::PR_TERRAFORM));
        for dir in 0..4 {
            let neighbour = self
                .map
                .add_diff_c_wrap(tile, dir, (self.s.map_max_y)() + 1);
            if neighbour == INVALID_TILE {
                continue;
            }
            let r = ts.height(self, neighbour);
            let mut difference = height - r;
            if difference.abs() > 1 {
                crate::witness::hit(crate::witness::LANDSCAPE_CMD_RECURSIVE);
                difference += if difference < 0 { 1 } else { -1 };
                let failed = unsafe {
                    /* SAFETY: synchronous costs never escape or outlive this call. */
                    crate::command::with_cost(self.c, |ret| {
                        let error = self.tile_height(ts, ret, neighbour, r + difference);
                        if ret.ok() {
                            out.add_money(ret.money());
                            None
                        } else {
                            out.take(ret);
                            Some(error)
                        }
                    })
                };
                if let Some(error) = failed {
                    return error;
                }
            }
        }
        INVALID_TILE
    }
    fn level(
        &self,
        out: Cost<'_>,
        last: Cost<'_>,
        ret: Cost<'_>,
        tile: u32,
        start: u32,
        diagonal: bool,
        mode: u8,
        flags: u16,
        area: *mut crate::water_cmd::Area,
    ) -> (i64, u32) {
        if start >= self.size {
            out.error(INVALID_STRING_ID);
            return (0, INVALID_TILE);
        }
        let old = (self.s.tile_height)(start);
        let h = match mode {
            LM_LEVEL => old,
            LM_RAISE => old.wrapping_add(1),
            LM_LOWER => old.wrapping_sub(1),
            _ => {
                out.error(INVALID_STRING_ID);
                return (0, INVALID_TILE);
            }
        };
        if h > u32::from((self.a.height_limit)()) {
            out.error(if old == 0 {
                self.a.strings.sea_level
            } else {
                self.a.strings.high
            });
            return (0, INVALID_TILE);
        }
        let mut money = crate::command::available_money(self.cmd);
        out.set(0);
        last.error(if mode == LM_LEVEL {
            self.a.strings.levelled
        } else {
            INVALID_STRING_ID
        });
        let mut success = false;
        let company = self.finance(flags, false);
        let mut limit = Self::terraform_limit(company);
        if limit == 0 {
            out.error(self.a.strings.terraform_limit);
            return (0, INVALID_TILE);
        }
        let mut error = INVALID_TILE;
        let extra = self.area(area, tile, start, diagonal, |t| {
            let mut current = (self.s.tile_height)(t);
            while current != h {
                error = self.terraform_do(ret, t, SLOPE_N, current <= h, flags & !F::EXECUTE);
                if !ret.ok() {
                    last.take(ret);
                    if last.message() == self.a.strings.terraform_limit {
                        limit = 0;
                    }
                    break;
                }
                let amount = ret.money();
                if flags & F::EXECUTE != 0 {
                    money = money.saturating_sub(amount);
                    if money < 0 {
                        return Some((amount, true));
                    }
                    unsafe {
                        /* SAFETY: synchronous costs never escape or outlive this call. */
                        crate::command::with_cost(self.c, |ignored| {
                            self.terraform_do(ignored, t, SLOPE_N, current <= h, flags);
                        })
                    };
                } else {
                    limit -= 1;
                    if limit <= 0 {
                        success = true;
                        break;
                    }
                }
                out.add_money(amount);
                current = current.wrapping_add(if current > h { u32::MAX } else { 1 });
                success = true;
            }
            if limit <= 0 { Some((0, false)) } else { None }
        });
        if let Some((amount, true)) = extra {
            return (amount, error);
        }
        if !success {
            out.take(last);
        }
        (0, if out.ok() { tile } else { error })
    }
}
#[derive(Default)]
struct TerraformerState {
    dirty: BTreeSet<u32>,
    heights: BTreeMap<u32, i32>,
}
impl TerraformerState {
    fn height(&self, w: &World<'_>, tile: u32) -> i32 {
        self.heights
            .get(&tile)
            .copied()
            .unwrap_or_else(|| (w.s.tile_height)(tile) as i32)
    }
}

/// Shared `Command<CMD_LANDSCAPE_CLEAR>::Do` path for every Rust owner.
pub(crate) fn clear_command(
    commands: &CommandServices,
    out: *mut CommandCost,
    tile: u32,
    flags: u16,
) {
    // SAFETY: the getter lends immutable process-lifetime tables; the calling
    // command owns its native result and serialized map for this invocation.
    let world = unsafe { World::new(&*(commands.landscape_commands)()) };
    world.clear_do(world.cost(out), tile, flags);
}

/// Direct native facade for the command body; the native dispatcher owns lifecycle.
/// # Safety
/// Immutable tables, exclusive native result slots and serialized map remain live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_landscape_clear(
    a: &LandscapeServices,
    out: *mut CommandCost,
    tile: u32,
    flags: u16,
) {
    let w = unsafe { World::new(a) };
    w.clear(w.cost(out), tile, flags);
}
/// # Safety
/// The area and three distinct native costs belong to this synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_landscape_clear_area(
    a: &LandscapeServices,
    out: *mut CommandCost,
    last: *mut CommandCost,
    ret: *mut CommandCost,
    tile: u32,
    start: u32,
    diagonal: bool,
    flags: u16,
    area: *mut crate::water_cmd::Area,
) -> i64 {
    let w = unsafe { World::new(a) };
    w.clear_area(
        w.cost(out),
        w.cost(last),
        w.cost(ret),
        tile,
        start,
        diagonal,
        flags,
        area,
    )
}
/// # Safety
/// Immutable tables, exclusive native result and serialized map remain live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_landscape_terraform(
    a: &LandscapeServices,
    out: *mut CommandCost,
    tile: u32,
    slope: u8,
    up: bool,
    flags: u16,
) -> u32 {
    let w = unsafe { World::new(a) };
    w.terraform(w.cost(out), tile, slope, up, flags)
}
/// # Safety
/// The area and three distinct native costs belong to this synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_landscape_level(
    a: &LandscapeServices,
    out: *mut CommandCost,
    last: *mut CommandCost,
    ret: *mut CommandCost,
    tile: u32,
    start: u32,
    diagonal: bool,
    mode: u8,
    flags: u16,
    area: *mut crate::water_cmd::Area,
    extra: &mut i64,
) -> u32 {
    let w = unsafe { World::new(a) };
    let (money, t) = w.level(
        w.cost(out),
        w.cost(last),
        w.cost(ret),
        tile,
        start,
        diagonal,
        mode,
        flags,
        area,
    );
    *extra = money;
    t
}

/// Shared `Command<CMD_TERRAFORM_LAND>::Do`, including its additional error tile.
pub(crate) fn terraform_command(
    commands: &CommandServices,
    out: *mut CommandCost,
    tile: u32,
    slope: u8,
    up: bool,
    flags: u16,
) -> u32 {
    // SAFETY: the getter supplies canonical immutable tables for the native caller.
    let world = unsafe { World::new(&*(commands.landscape_commands)()) };
    world.terraform_do(world.cost(out), tile, slope, up, flags)
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDLandscapeCommandStrings" ["landscape"] => crate::landscape_cmd::Strings {
        canal, clear_limit, water, sea_level, high, edge, bridge, bridge_high,
        tunnel, terraform_limit, levelled,
    };
    "OpenTTDLandscapeCommandServices" ["landscape"] => crate::landscape_cmd::LandscapeServices {
        shared, costs, commands, strings, tile_procs, water, on_water, coast, buoy, canal,
        height_limit, max_bridge_height, freeform_edges, editor, exchange_generating,
        tunnel_in_way, mark_dirty_height, set_height, paused, explosion,
    };
    "OpenTTDLandscapeTileServices" ["landscape"] => crate::landscape_cmd::TileServices {
        clear, terraform, owners, trees, industry_policy, industry_costs, industry_generation,
    };
}
