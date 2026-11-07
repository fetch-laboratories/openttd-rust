/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Industry construction, funding and prospecting transaction. The result is a native stack
//! `CommandCost`; specs and layouts are immutable.
use super::{
    Builder, ConstructionRequest, ConstructionServices, GenerationServices, Industry,
    IndustryCommandServices, IndustryCost, IndustryCostServices, LayoutHandle, LifecycleServices,
    Services, ShellHandle, SpecHandle, TilePolicyServices, TileServices, TownHandle,
};
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company_type::{CompanyID, Owner};
use crate::direction_type::{DIAGDIR_NE, DIAGDIR_NW, DIAGDIR_SE, DIAGDIR_SW};
use crate::industry_type::{IT_INVALID, NUM_INDUSTRYTYPES};
use crate::industrytype::{self, CheckProc, IndustryBehaviour, IndustryBehaviours};
use crate::landscape_type::ARCTIC;
use crate::newgrf_callbacks::{
    CALLBACK_FAILED, IndustryCallbackMask, IndustryCallbackMasks, IndustryTileCallbackMask,
    IndustryTileCallbackMasks,
};
use crate::newgrf_industries::IndustryAvailabilityCallType;
use crate::slope_type::{
    SLOPE_E, SLOPE_ELEVATED, SLOPE_FLAT, SLOPE_N, SLOPE_NE, SLOPE_NW, SLOPE_S, SLOPE_SE,
    SLOPE_STEEP, SLOPE_SW, SLOPE_W,
};
use crate::strings_type::INVALID_STRING_ID;
use crate::tile_type::{
    INVALID_TILE, MP_CLEAR, MP_HOUSE, MP_TREES, MP_VOID, TROPICZONE_DESERT, TROPICZONE_RAINFOREST,
};
use std::{ffi::c_void, ptr};

#[repr(C)]
pub struct ConflictKinds {
    pub kinds: [u8; 3],
}
#[repr(C)]
pub struct Allocation {
    pub shell: *mut ShellHandle,
    pub owner: *mut Industry,
    pub id: u16,
}
#[repr(C)]
pub struct ConstructionErrors {
    pub site: u32,
    pub forest: u32,
    pub positioned: u32,
    pub desert: u32,
    pub rainforest: u32,
    pub lowland: u32,
    pub one_per_town: u32,
    pub town: u32,
    pub population: u32,
    pub near_town: u32,
    pub conflict: u32,
    pub too_many: u32,
}
#[repr(C)]
pub struct LocationResult {
    pub callback: u16,
    pub registers: [i32; 16],
}
#[repr(C)]
pub struct TransactionServices {
    pub raw_setting: unsafe extern "C" fn() -> u8,
    pub multiple_per_town: unsafe extern "C" fn() -> bool,
    pub town: unsafe extern "C" fn(*mut ShellHandle) -> *const TownHandle,
    pub population: unsafe extern "C" fn(*const TownHandle) -> u32,
    pub town_position: unsafe extern "C" fn(*const TownHandle) -> u32,
    pub industry: unsafe extern "C" fn(u16) -> *mut ShellHandle,
    pub edge_distance: unsafe extern "C" fn(u32, u8) -> u32,
    pub refinery_limit: unsafe extern "C" fn() -> u8,
    pub tile_height: unsafe extern "C" fn(u32) -> u32,
    pub set_company: unsafe extern "C" fn(u8),
    pub temporary_shell: unsafe extern "C" fn(
        *mut Industry,
        *mut c_void,
        unsafe extern "C" fn(*mut c_void, *mut ShellHandle),
    ),
    pub resolve_location:
        unsafe extern "C" fn(*mut ShellHandle, u32, u8, u32, u8) -> LocationResult,
    pub resolve_shape: unsafe extern "C" fn(*mut ShellHandle, u32, u16, u32) -> LocationResult,
    pub location_error:
        unsafe extern "C" fn(*mut IndustryCost, *const LocationResult, *const SpecHandle),
    pub shape_error:
        unsafe extern "C" fn(*mut IndustryCost, *const LocationResult, *const super::TileSpec),
    pub terragenesis: unsafe extern "C" fn() -> bool,
    pub ignore_restrictions: unsafe extern "C" fn() -> bool,
    pub platform: unsafe extern "C" fn() -> u8,
    pub can_allocate: unsafe extern "C" fn() -> bool,
    pub allocate: unsafe extern "C" fn(u32) -> Allocation,
    pub local_company: unsafe extern "C" fn() -> bool,
    pub prospect_no_place: unsafe extern "C" fn(),
    pub prospect_unlucky: unsafe extern "C" fn(),
}
#[repr(C)]
pub struct TransactionContext {
    pub map: super::Map,
    pub builder: *mut Builder,
    pub production: *const Services,
    pub generation: *const GenerationServices,
    pub construction: *const ConstructionServices,
    pub lifecycle: *const LifecycleServices,
    pub tiles: *const TileServices,
    pub tile_policy: *const TilePolicyServices,
    pub commands: *const IndustryCommandServices,
    pub command_checks: *const crate::services::CommandServices,
    pub costs: *const IndustryCostServices,
    pub transaction: *const TransactionServices,
    pub errors: *const ConstructionErrors,
}
struct Engine<'a> {
    context: &'a TransactionContext,
    s: &'a Services,
    g: &'a GenerationServices,
    c: &'a ConstructionServices,
    t: &'a TileServices,
    p: &'a TilePolicyServices,
    m: &'a IndustryCommandServices,
    cost: &'a IndustryCostServices,
    x: &'a TransactionServices,
    e: &'a ConstructionErrors,
    out: *mut IndustryCost,
}
impl Engine<'_> {
    fn fail(&self, message: u32) -> bool {
        unsafe { (self.cost.error)(self.out, message) };
        false
    }
    fn success(&self) -> bool {
        unsafe { (self.cost.zero)(self.out) };
        true
    }
    fn succeeded(&self) -> bool {
        unsafe { (self.cost.success)(self.out) }
    }
    fn company(&self) -> Owner {
        CompanyID(unsafe { (self.p.current_company)() })
    }
    fn swap_company(&self, company: Owner) -> Owner {
        let original = self.company();
        unsafe { (self.x.set_company)(company.0) };
        original
    }
    fn map(&self) -> super::Map {
        self.context.map
    }
    fn tile_add_wrap(&self, tile: u32, x: i32, y: i32) -> u32 {
        let map = self.map();
        map.coordinates().tile_add_wrap(
            tile,
            x,
            y,
            map.size_y,
            (self.g.environment.freeform_edges)(),
        )
    }
    fn distance(&self, a: u32, b: u32) -> u32 {
        let map = self.map();
        let coordinates = map.coordinates();
        coordinates
            .tile_x(a)
            .abs_diff(coordinates.tile_x(b))
            .max(coordinates.tile_y(a).abs_diff(coordinates.tile_y(b)))
    }
    fn offset(&self, tile: u32, x: i32, y: i32) -> u32 {
        tile.wrapping_add(self.map().coordinates().diff(x, y))
    }
    fn edge(&self, tile: u32, limit: u32) -> bool {
        let map = self.map();
        let x = if map.size_x > 256 {
            limit.wrapping_mul(map.size_x / 256)
        } else {
            limit
        };
        let y = if map.size_y > 256 {
            limit.wrapping_mul(map.size_y / 256)
        } else {
            limit
        };
        unsafe {
            (self.x.edge_distance)(tile, DIAGDIR_NE) < x
                || (self.x.edge_distance)(tile, DIAGDIR_NW) < y
                || (self.x.edge_distance)(tile, DIAGDIR_SW) < x
                || (self.x.edge_distance)(tile, DIAGDIR_SE) < y
        }
    }
    fn builtin_location(&self, tile: u32, spec: *const SpecHandle) -> bool {
        match CheckProc(unsafe { (*spec).check_proc }) {
            CheckProc::CHECK_FOREST => {
                if self.map().landscape == ARCTIC
                    && (self.s.shared().tile_z)(tile)
                        < i32::from(crate::snow_line::highest(self.s.shared())) + 2
                {
                    return self.fail(self.e.forest);
                }
            }
            CheckProc::CHECK_REFINERY => {
                if !(self.g.environment.editor)()
                    && !self.edge(
                        self.offset(tile, 1, 1),
                        u32::from(unsafe { (self.x.refinery_limit)() }),
                    )
                {
                    return self.fail(self.e.positioned);
                }
            }
            CheckProc::CHECK_FARM => {
                if self.map().landscape == ARCTIC
                    && (self.s.shared().tile_z)(tile).wrapping_add(2)
                        >= i32::from(crate::snow_line::highest(self.s.shared()))
                {
                    return self.fail(self.e.site);
                }
            }
            CheckProc::CHECK_PLANTATION
                if (self.s.shared().tropic_zone)(tile) == TROPICZONE_DESERT =>
            {
                return self.fail(self.e.site);
            }
            CheckProc::CHECK_WATER if (self.s.shared().tropic_zone)(tile) != TROPICZONE_DESERT => {
                return self.fail(self.e.desert);
            }
            CheckProc::CHECK_LUMBERMILL
                if (self.s.shared().tropic_zone)(tile) != TROPICZONE_RAINFOREST =>
            {
                return self.fail(self.e.rainforest);
            }
            CheckProc::CHECK_BUBBLEGEN if (self.s.shared().tile_z)(tile) > 4 => {
                return self.fail(self.e.lowland);
            }
            CheckProc::CHECK_OIL_RIG => {
                if (self.g.environment.editor)() && unsafe { (self.x.ignore_restrictions)() } {
                    return self.success();
                }
                if unsafe { (self.x.tile_height)(tile) } != 0
                    || !self.edge(
                        self.offset(tile, 1, 1),
                        u32::from(unsafe { (self.x.refinery_limit)() }),
                    )
                {
                    return self.fail(self.e.positioned);
                }
            }
            _ => {}
        }
        self.success()
    }
    fn registry(&self, kind: u8) -> *const Vec<u16> {
        unsafe {
            ptr::addr_of!((*self.context.builder).registry)
                .cast::<Vec<u16>>()
                .add(usize::from(kind))
        }
    }
    fn registry_count(&self, kind: u8) -> usize {
        unsafe { (&*self.registry(kind)).len() }
    }
    fn registry_id(&self, kind: u8, n: usize) -> u16 {
        unsafe { ptr::read((&*self.registry(kind)).as_ptr().add(n)) }
    }
    fn conflict(&self, tile: u32, kind: u8) -> bool {
        let spec = industrytype::industry(u16::from(kind));
        let conflicting = unsafe { (*spec).conflicting };
        for other in conflicting {
            if other == IT_INVALID {
                continue;
            }
            for n in 0..self.registry_count(other) {
                let shell = unsafe { (self.x.industry)(self.registry_id(other, n)) };
                let owner = unsafe { (self.m.owner)(shell) };
                let position = unsafe { (*owner).metadata.tile };
                if self.distance(tile, position) <= 14 {
                    return self.fail(self.e.conflict);
                }
            }
        }
        self.success()
    }
    fn find_town(&self, tile: u32, kind: u8) -> Option<*const TownHandle> {
        let town = crate::town::spatial::closest(unsafe { &*self.s.shared }, tile, u32::MAX);
        if !unsafe { (self.x.multiple_per_town)() } {
            for n in 0..self.registry_count(kind) {
                let shell = unsafe { (self.x.industry)(self.registry_id(kind, n)) };
                if unsafe { (self.x.town)(shell) } == town {
                    self.fail(self.e.one_per_town);
                    return None;
                }
            }
        }
        self.success();
        Some(town)
    }
    fn allowed(&self, tile: u32, kind: u8, town: *const TownHandle) -> bool {
        let spec = industrytype::industry(u16::from(kind));
        if IndustryBehaviours(unsafe { (*spec).behaviour }).test(IndustryBehaviour::Town1200More)
            && unsafe { (self.x.population)(town) } < 1200
        {
            return self.fail(self.e.population);
        }
        let spec = industrytype::industry(u16::from(kind));
        if IndustryBehaviours(unsafe { (*spec).behaviour }).test(IndustryBehaviour::OnlyNearTown)
            && self.distance(unsafe { (self.x.town_position)(town) }, tile) > 9
        {
            return self.fail(self.e.near_town);
        }
        self.success()
    }
    fn free(&self, tile: u32, layout: *const LayoutHandle, kind: u8) -> bool {
        let behaviour = unsafe { (*industrytype::industry(u16::from(kind))).behaviour };
        for n in 0..unsafe { (*layout).len() } {
            let part = super::layout_tile(layout, n);
            let gfx = unsafe { (self.c.translated_gfx)(part.gfx) };
            let current = self.tile_add_wrap(tile, i32::from(part.x), i32::from(part.y));
            if current == INVALID_TILE {
                return self.fail(self.e.site);
            }
            if gfx == crate::industry_map::GFX_WATERTILE_SPECIALCHECK {
                if !(self.s.shared().is_water_tile)(current)
                    || crate::map_func::World::new(self.s.shared()).tile_slope(current)
                        != SLOPE_FLAT
                {
                    return self.fail(self.e.site);
                }
            } else {
                unsafe { (*self.context.command_checks).no_vehicle(self.out.cast(), current) };
                if !self.succeeded() {
                    return false;
                }
                if crate::map_func::World::new(self.s.shared()).bridge_above(current) {
                    return self.fail(self.e.site);
                }
                let spec = industrytype::tile(gfx);
                if unsafe { (*spec).slopes_refused } & 32 == 0
                    && ((self.s.shared().has_water_class)(current)
                        && unsafe { (self.t.on_water)(current) })
                        != (IndustryBehaviours(behaviour).test(IndustryBehaviour::BuiltOnWater))
                {
                    return self.fail(self.e.site);
                }
                if behaviour
                    & (IndustryBehaviours::from_element(IndustryBehaviour::OnlyInTown).0
                        | IndustryBehaviours::from_element(IndustryBehaviour::Town1200More).0)
                    != 0
                    || (IndustryBehaviours(behaviour).test(IndustryBehaviour::OnlyNearTown)
                        && (self.s.shared().tile_type)(current) == MP_HOUSE)
                {
                    if (self.s.shared().tile_type)(current) != MP_HOUSE {
                        return self.fail(self.e.town);
                    }
                    let company = self.swap_company(Owner::TOWN);
                    unsafe {
                        crate::landscape_cmd::clear_command(
                            &*self.context.command_checks,
                            self.out.cast(),
                            current,
                            0,
                        );
                        (self.x.set_company)(company.0);
                    };
                    if !self.succeeded() {
                        return false;
                    }
                } else {
                    unsafe {
                        crate::landscape_cmd::clear_command(
                            &*self.context.command_checks,
                            self.out.cast(),
                            current,
                            DoCommandFlag::AUTO
                                | DoCommandFlag::NO_TEST_TOWN_RATING
                                | DoCommandFlag::NO_MODIFY_TOWN_RATING,
                        );
                    };
                    if !self.succeeded() {
                        return false;
                    }
                }
            }
        }
        self.success()
    }
    #[allow(clippy::too_many_arguments)]
    fn slopes(
        &self,
        tile: u32,
        layout: *const LayoutHandle,
        layout_index: usize,
        kind: u8,
        bits: u16,
        founder: u8,
        creation: u8,
        custom: &mut bool,
    ) -> bool {
        let mut refused = false;
        let mut custom_shape = false;
        for n in 0..unsafe { (*layout).len() } {
            let part = super::layout_tile(layout, n);
            let gfx = unsafe { (self.c.translated_gfx)(part.gfx) };
            let current = self.tile_add_wrap(tile, i32::from(part.x), i32::from(part.y));
            if gfx != crate::industry_map::GFX_WATERTILE_SPECIALCHECK {
                let spec = industrytype::tile(gfx);
                if unsafe {
                    IndustryTileCallbackMasks((*spec).callback_mask)
                        .test(IndustryTileCallbackMask::ShapeCheck)
                } {
                    custom_shape = true;
                    self.shape_check(
                        tile,
                        current,
                        spec,
                        kind,
                        gfx,
                        layout_index,
                        bits,
                        founder,
                        creation,
                    );
                    if !self.succeeded() {
                        return false;
                    }
                } else {
                    refused |= slope_refused(
                        crate::map_func::World::new(self.s.shared()).tile_slope(current),
                        unsafe { (*spec).slopes_refused },
                    );
                }
            }
        }
        *custom = custom_shape;
        // Fully flat land is almost impossible in TerraGenesis. Check whether
        // the platform can be levelled later (CheckIfCanLevelIndustryPlatform).
        if !refused
            || (unsafe { (self.x.terragenesis)() }
                && unsafe { (self.c.generating)() }
                && !custom_shape
                && !unsafe { (self.x.ignore_restrictions)() })
        {
            return self.success();
        }
        self.fail(self.e.site)
    }
    fn surrounding(&self, tile: u32, height: u32, internal: i32) -> bool {
        let map = self.map();
        if tile.is_multiple_of(map.size_x)
            || tile / map.size_x == 0
            || (self.s.shared().tile_type)(tile) == MP_VOID
        {
            return false;
        }
        for y in 0..2 {
            for x in 0..2 {
                let walk = self.offset(tile, x - 1, y - 1);
                let current = unsafe { (self.x.tile_height)(walk) };
                if (self.s.shared().tile_type)(walk) != MP_CLEAR
                    && (self.s.shared().tile_type)(walk) != MP_TREES
                {
                    return false;
                }
                if internal != 0 && current.abs_diff(height) > 1 {
                    return false;
                }
                if internal == 0
                    && current != height
                    && (walk.is_multiple_of(map.size_x)
                        || walk / map.size_x == 0
                        || !self.surrounding(
                            self.offset(walk, -1, -1),
                            height,
                            internal.wrapping_add(1),
                        ))
                {
                    return false;
                }
            }
        }
        true
    }
    fn level(&self, tile: u32, flags: u16, layout: *const LayoutHandle) -> bool {
        let (mut max_x, mut max_y) = (0_i32, 0_i32);
        for n in 0..unsafe { (*layout).len() } {
            let part = super::layout_tile(layout, n);
            if part.gfx == crate::industry_map::GFX_WATERTILE_SPECIALCHECK {
                continue;
            }
            max_x = max_x.max(i32::from(part.x));
            max_y = max_y.max(i32::from(part.y));
        }
        let height = unsafe { (self.x.tile_height)(tile) };
        let map = self.map();
        let platform = u32::from(unsafe { (self.x.platform)() });
        if tile % map.size_x <= platform.wrapping_add(1)
            || tile / map.size_x <= platform.wrapping_add(1)
        {
            return false;
        }
        // Abort instead of clamping when the platform cannot expand within the map.
        let origin = self.offset(tile, -(platform as i32), -(platform as i32));
        let width = (max_x.wrapping_add(2).wrapping_add((2 * platform) as i32)) as u16;
        let height_area = (max_y.wrapping_add(2).wrapping_add((2 * platform) as i32)) as u16;
        if (origin % map.size_x).wrapping_add(u32::from(width)) >= map.size_x - 1
            || (origin / map.size_x).wrapping_add(u32::from(height_area)) >= map.size_y - 1
        {
            return false;
        }
        // Terraform as OWNER_TOWN to disable autoslope and town ratings, regardless
        // of whether the founder is a company or this is random/editor construction.
        let company = self.swap_company(Owner::TOWN);
        for y in 0..u32::from(height_area) {
            for x in 0..u32::from(width) {
                let walk = self.offset(origin, x as i32, y as i32);
                let current = unsafe { (self.x.tile_height)(walk) };
                if current != height {
                    if !self.surrounding(walk, height, 0) {
                        unsafe { (self.x.set_company)(company.0) };
                        return false;
                    }
                    // This test cannot account fully for height differences over one
                    // without modifying the map.
                    unsafe {
                        crate::landscape_cmd::terraform_command(
                            &*self.context.command_checks,
                            self.out.cast(),
                            walk,
                            crate::slope_func::SLOPE_N,
                            current <= height,
                            flags & !DoCommandFlag::EXECUTE,
                        )
                    };
                    if !self.succeeded() {
                        unsafe { (self.x.set_company)(company.0) };
                        return false;
                    }
                }
            }
        }
        if DoCommandFlags(flags).test(DoCommandFlag::Execute) {
            for y in 0..u32::from(height_area) {
                for x in 0..u32::from(width) {
                    let walk = self.offset(origin, x as i32, y as i32);
                    let mut current = unsafe { (self.x.tile_height)(walk) };
                    while current != height {
                        // Terraform for free: the test pass cannot calculate the
                        // exact cost, and mismatched test/execute costs would assert.
                        unsafe {
                            crate::landscape_cmd::terraform_command(
                                &*self.context.command_checks,
                                self.out.cast(),
                                walk,
                                crate::slope_func::SLOPE_N,
                                current <= height,
                                flags,
                            )
                        };
                        current = if current > height {
                            current.wrapping_sub(1)
                        } else {
                            current.wrapping_add(1)
                        };
                    }
                }
            }
        }
        unsafe { (self.x.set_company)(company.0) };
        true
    }
    #[allow(clippy::too_many_arguments)]
    fn location_check(
        &self,
        tile: u32,
        kind: u8,
        layout_index: usize,
        random: u32,
        bits: u16,
        founder: u8,
        creation: u8,
    ) {
        let spec = industrytype::industry(u16::from(kind));
        let mut owner = Industry::default();
        let mut call = LocationCall {
            engine: self,
            spec,
            tile,
            kind,
            layout_index,
            random,
            bits,
            founder,
            creation,
        };
        // The owner is a stack value. Only raw pointers cross the synchronous
        // native ctor/visit/resolver/dtor scope; none survives that scope.
        unsafe { (self.x.temporary_shell)(&raw mut owner, (&raw mut call).cast(), location_visit) };
    }
    #[allow(clippy::too_many_arguments)]
    fn shape_check(
        &self,
        tile: u32,
        current: u32,
        spec: *const super::TileSpec,
        kind: u8,
        gfx: u16,
        layout_index: usize,
        bits: u16,
        founder: u8,
        creation: u8,
    ) {
        let mut owner = Industry::default();
        let mut call = ShapeCall {
            engine: self,
            tile,
            current,
            spec,
            kind,
            gfx,
            layout_index,
            bits,
            founder,
            creation,
        };
        unsafe { (self.x.temporary_shell)(&raw mut owner, (&raw mut call).cast(), shape_visit) };
    }
    #[allow(clippy::too_many_arguments)]
    fn create(
        &self,
        tile: u32,
        kind: u8,
        flags: u16,
        spec: *const SpecHandle,
        layout_index: usize,
        random: u32,
        bits: u16,
        founder: u8,
        creation: u8,
        industry: *mut *mut ShellHandle,
    ) -> bool {
        let layout = unsafe { &raw const (*spec).layouts.as_slice()[layout_index] };
        unsafe { *industry = ptr::null_mut() };
        if !self.conflict(tile, kind) {
            return false;
        }
        let Some(town) = self.find_town(tile, kind) else {
            return false;
        };
        if !self.allowed(tile, kind, town) {
            return false;
        }
        let scratch = crate::objects::snapshot_areas();
        let free = self.free(tile, layout, kind);
        crate::objects::restore_areas(scratch);
        if !free {
            return false;
        }
        if IndustryCallbackMasks(unsafe {
            (*industrytype::industry(u16::from(kind))).callback_mask
        })
        .test(IndustryCallbackMask::Location)
        {
            self.location_check(tile, kind, layout_index, random, bits, founder, creation);
        } else {
            self.builtin_location(tile, spec);
        }
        if !self.succeeded() {
            return false;
        }
        let mut custom = false;
        if !self.slopes(
            tile,
            layout,
            layout_index,
            kind,
            bits,
            founder,
            creation,
            &mut custom,
        ) {
            return false;
        }
        if !custom
            && unsafe { (self.x.terragenesis)() }
            && unsafe { (self.c.generating)() }
            && !unsafe { (self.x.ignore_restrictions)() }
            && !self.level(tile, 8, layout)
        {
            return self.fail(self.e.site);
        }
        if !unsafe { (self.x.can_allocate)() } {
            return self.fail(self.e.too_many);
        }
        if DoCommandFlags(flags).test(DoCommandFlag::Execute) {
            let allocation = unsafe { (self.x.allocate)(tile) };
            unsafe { *industry = allocation.shell };
            if !custom {
                self.level(tile, 8 | 1, layout);
            }
            let request = ConstructionRequest {
                layout,
                town,
                layout_index,
                tile,
                size_x: self.map().size_x,
                size_y: self.map().size_y,
                id: allocation.id,
                random_bits: bits,
                kind,
                founder,
            };
            unsafe {
                super::openttd_rust_industry_initialize(
                    allocation.shell.cast(),
                    allocation.owner,
                    self.context.builder,
                    &raw const request,
                    self.context.production,
                    self.context.generation,
                    self.context.lifecycle,
                    self.context.construction,
                );
            };
        }
        self.success()
    }
}
struct LocationCall<'a, 'b> {
    engine: &'a Engine<'b>,
    spec: *const SpecHandle,
    tile: u32,
    kind: u8,
    layout_index: usize,
    random: u32,
    bits: u16,
    founder: u8,
    creation: u8,
}
unsafe extern "C" fn location_visit(context: *mut c_void, shell: *mut ShellHandle) {
    let call = unsafe { &*context.cast::<LocationCall<'_, '_>>() };
    let e = call.engine;
    let owner = unsafe { (e.m.owner)(shell) };
    unsafe {
        (*owner).metadata.tile = call.tile;
        (*owner).metadata.width = 0;
        (*owner).metadata.kind = call.kind;
        (*owner).metadata.selected_layout = call.layout_index as u8;
    }
    let town = crate::town::spatial::closest(unsafe { &*e.s.shared }, call.tile, u32::MAX);
    unsafe {
        (e.c.set_town)(shell.cast(), town);
        (*owner).metadata.random = call.bits;
        (*owner).metadata.founder = call.founder;
    }
    let result =
        unsafe { (e.x.resolve_location)(shell, call.tile, call.kind, call.random, call.creation) };
    if result.callback == CALLBACK_FAILED {
        e.success();
    } else {
        unsafe { (e.x.location_error)(e.out, &raw const result, call.spec) };
    }
}
struct ShapeCall<'a, 'b> {
    engine: &'a Engine<'b>,
    tile: u32,
    current: u32,
    spec: *const super::TileSpec,
    kind: u8,
    gfx: u16,
    layout_index: usize,
    bits: u16,
    founder: u8,
    creation: u8,
}
unsafe extern "C" fn shape_visit(context: *mut c_void, shell: *mut ShellHandle) {
    let call = unsafe { &*context.cast::<ShapeCall<'_, '_>>() };
    let e = call.engine;
    let owner = unsafe { (e.m.owner)(shell) };
    unsafe {
        (*owner).metadata.tile = call.tile;
        (*owner).metadata.width = 0;
        (*owner).metadata.kind = call.kind;
        (*owner).metadata.random = call.bits;
        (*owner).metadata.founder = call.founder;
    }
    let parameter = (u32::from(call.creation) << 8) | call.layout_index as u32;
    let result = unsafe { (e.x.resolve_shape)(shell, call.current, call.gfx, parameter) };
    if result.callback == CALLBACK_FAILED {
        if !slope_refused(
            crate::map_func::World::new(e.s.shared()).tile_slope(call.current),
            unsafe { (*call.spec).slopes_refused },
        ) {
            e.success();
        } else {
            e.fail(e.e.site);
        }
    } else if (e.s.shared().grf_file_version)(unsafe { (*call.spec).grf_prop.grffile }) < 7 {
        if result.callback != 0 {
            e.success();
        } else {
            e.fail(e.e.site);
        }
    } else {
        unsafe { (e.x.shape_error)(e.out, &raw const result, call.spec) };
    }
}
fn slope_refused(current: u8, refused: u8) -> bool {
    if current & SLOPE_STEEP != 0 {
        return true;
    }
    if current != SLOPE_FLAT {
        if refused & SLOPE_STEEP != 0 {
            return true;
        }
        let complement = current ^ SLOPE_ELEVATED;
        if refused & SLOPE_W != 0 && complement & SLOPE_NW != 0 {
            return true;
        }
        if refused & SLOPE_S != 0 && complement & SLOPE_NE != 0 {
            return true;
        }
        if refused & SLOPE_E != 0 && complement & SLOPE_SW != 0 {
            return true;
        }
        if refused & SLOPE_N != 0 && complement & SLOPE_SE != 0 {
            return true;
        }
    }
    false
}
unsafe fn engine<'a>(context: *const TransactionContext, out: *mut IndustryCost) -> Engine<'a> {
    let context = unsafe { &*context };
    unsafe {
        Engine {
            context,
            s: &*context.production,
            g: &*context.generation,
            c: &*context.construction,
            t: &*context.tiles,
            p: &*context.tile_policy,
            m: &*context.commands,
            cost: &*context.costs,
            x: &*context.transaction,
            e: &*context.errors,
            out,
        }
    }
}
/// # Safety
/// Live native stack cost/output and synchronous services. Borrowed specs/layouts
/// remain immutable. Commands, allocation and resolvers receive no owner borrow.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn openttd_rust_industry_build(
    out: *mut IndustryCost,
    tile: u32,
    kind: u8,
    flags: u16,
    first_layout: u32,
    fund: bool,
    seed: u32,
    context: *const TransactionContext,
) {
    let e = unsafe { engine(context, out) };
    if kind >= NUM_INDUSTRYTYPES {
        e.fail(INVALID_STRING_ID);
        return;
    }
    // Check that this industry is available for the climate.
    let spec = industrytype::industry(u16::from(kind));
    if !unsafe { (*spec).enabled } || unsafe { (*spec).layouts.is_empty() } {
        e.fail(INVALID_STRING_ID);
        return;
    }
    // Raw-material industry construction requires the corresponding setting.
    if !(e.g.environment.editor)()
        && e.company() != Owner::DEITY
        && unsafe { (e.x.raw_setting)() } == 0
        && unsafe { super::spec_raw((*spec).life_type) }
    {
        e.fail(INVALID_STRING_ID);
        return;
    }
    if !(e.g.environment.editor)()
        && unsafe {
            super::resolver::openttd_rust_industry_probability(
                kind,
                if e.company() == Owner::DEITY {
                    IndustryAvailabilityCallType::IACT_RANDOMCREATION.0
                } else {
                    IndustryAvailabilityCallType::IACT_USERCREATION.0
                },
                1,
                e.s.resolver,
            )
        } == 0
    {
        e.fail(INVALID_STRING_ID);
        return;
    }
    // The local Randomizer is seeded twice with seed and Next is called once.
    // Its returned second word is rotr(seed, 3) - 1; the updated first word is unused.
    let bits = seed as u16;
    let random = seed.rotate_right(3).wrapping_sub(1);
    let count = unsafe { (*spec).layouts.len() };
    e.fail(e.e.site);
    let deity_prospect = e.company() == Owner::DEITY && !fund;
    let mut industry = ptr::null_mut();
    if deity_prospect
        || (!(e.g.environment.editor)()
            && e.company() != Owner::DEITY
            && unsafe { (e.x.raw_setting)() } == 2
            && unsafe { super::spec_raw((*spec).life_type) })
    {
        if DoCommandFlags(flags).test(DoCommandFlag::Execute) {
            // A fuller map lowers the effective prospecting chance because a
            // successful prospect still may find no buildable location.
            let succeeded =
                deity_prospect || e.s.shared().random() <= unsafe { (*spec).prospecting_chance };
            if succeeded {
                let creation = if e.company() == Owner::DEITY {
                    IndustryAvailabilityCallType::IACT_RANDOMCREATION.0
                } else {
                    IndustryAvailabilityCallType::IACT_PROSPECTCREATION.0
                };
                // Build as OWNER_TOWN so the founder's owned land is not treated specially.
                let company = e.swap_company(Owner::TOWN);
                for _ in 0..5000 {
                    let map = e.map();
                    // Keep random draws in separate statements: C++ parameter
                    // evaluation order does not specify their order.
                    let candidate =
                        e.s.shared().random() & map.size_x.wrapping_mul(map.size_y).wrapping_sub(1);
                    // Start with a random layout, then check each layout in turn.
                    let mut layout = e.s.shared().random_range(count as u32) as usize;
                    for _ in 0..count {
                        layout = layout.wrapping_add(1) % count;
                        if e.create(
                            candidate,
                            kind,
                            flags,
                            spec,
                            layout,
                            random,
                            bits,
                            company.0,
                            creation,
                            &raw mut industry,
                        ) {
                            break;
                        }
                    }
                    if e.succeeded() {
                        break;
                    }
                }
                unsafe { (e.x.set_company)(company.0) };
            }
            if !e.succeeded() && unsafe { (e.x.local_company)() } {
                if succeeded {
                    unsafe { (e.x.prospect_no_place)() };
                } else {
                    unsafe { (e.x.prospect_unlucky)() };
                }
            }
        }
    } else {
        // Check layouts successively, starting with the given layout.
        let mut layout = first_layout as usize;
        if layout >= count {
            e.fail(INVALID_STRING_ID);
            return;
        }
        for _ in 0..count {
            layout = layout.wrapping_add(1) % count;
            if e.create(
                tile,
                kind,
                flags,
                spec,
                layout,
                random,
                bits,
                e.company().0,
                if e.company() == Owner::DEITY {
                    IndustryAvailabilityCallType::IACT_RANDOMCREATION.0
                } else {
                    IndustryAvailabilityCallType::IACT_USERCREATION.0
                },
                &raw mut industry,
            ) {
                break;
            }
        }
        if !e.succeeded() {
            return;
        }
    }
    if DoCommandFlags(flags).test(DoCommandFlag::Execute)
        && !industry.is_null()
        && !(e.g.environment.editor)()
    {
        super::advertise(industry.cast(), e.s);
    }
    let construction = unsafe {
        super::construction_cost(
            (*spec).cost_multiplier,
            (*spec).life_type,
            (e.x.raw_setting)(),
        )
    };
    unsafe { (e.cost.expense_money)(out, crate::economy_type::EXPENSES_OTHER, construction) };
}
/// # Safety
/// Live stack result/output; source preconditions require a nonempty layout list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_create(
    out: *mut IndustryCost,
    industry: *mut *mut ShellHandle,
    tile: u32,
    kind: u8,
    creation: u8,
    context: *const TransactionContext,
) {
    let e = unsafe { engine(context, out) };
    let spec = industrytype::industry(u16::from(kind));
    let random = e.s.shared().random();
    let second = e.s.shared().random();
    let count = unsafe { (*spec).layouts.len() };
    let layout = e.s.shared().random_range(count as u32) as usize;
    e.create(
        tile,
        kind,
        1,
        spec,
        layout,
        random,
        second as u16,
        16,
        creation,
        industry,
    );
}

/// # Safety
/// Actual native slope bytes; pure original slope-refusal policy, no callbacks.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_slope_refused_abi(current: u8, refused: u8) -> u8 {
    u8::from(slope_refused(current, refused))
}

/// # Safety
/// Native stack cost and immutable tables. Temporary shell construction/destruction
/// is synchronous and no temporary owner or resolver pointer may escape the visit.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn openttd_rust_industry_location_check(
    out: *mut IndustryCost,
    tile: u32,
    kind: u8,
    layout: usize,
    random: u32,
    bits: u16,
    founder: u8,
    creation: u8,
    context: *const TransactionContext,
) {
    unsafe { engine(context, out) }
        .location_check(tile, kind, layout, random, bits, founder, creation);
}
/// # Safety
/// Same temporary shell/stack cost contract as the native location callback.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn openttd_rust_industry_shape_check(
    out: *mut IndustryCost,
    tile: u32,
    current: u32,
    spec: *const super::TileSpec,
    kind: u8,
    gfx: u16,
    layout: usize,
    bits: u16,
    founder: u8,
    creation: u8,
    context: *const TransactionContext,
) {
    unsafe { engine(context, out) }.shape_check(
        tile, current, spec, kind, gfx, layout, bits, founder, creation,
    );
}
