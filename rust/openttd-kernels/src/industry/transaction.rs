/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete industry construction, funding and prospecting transaction.
//! The native result is a stack `CommandCost`. Specs and layouts are immutable;
//! every owner/registry access ends before a command, resolver or allocation call.
use super::{
    Builder, ConstructionRequest, ConstructionServices, GenerationServices, Industry,
    IndustryCommandServices, IndustryCost, IndustryCostServices, LayoutHandle, LifecycleServices,
    Location, Services, ShellHandle, SpecHandle, TilePolicyServices, TileServices, TownHandle,
};
use std::{ffi::c_void, ptr, slice};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ClearArea {
    pub first_tile: u32,
    pub area: Location,
}
#[repr(C)]
pub struct ClearAreas {
    pub data: *const ClearArea,
    pub size: usize,
}
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
    pub enabled: unsafe extern "C" fn(*const SpecHandle) -> bool,
    pub layout_count: unsafe extern "C" fn(*const SpecHandle) -> usize,
    pub layout: unsafe extern "C" fn(*const SpecHandle, usize) -> *const LayoutHandle,
    pub raw: unsafe extern "C" fn(*const SpecHandle) -> bool,
    pub raw_setting: unsafe extern "C" fn() -> u8,
    pub check_proc: unsafe extern "C" fn(*const SpecHandle) -> u8,
    pub conflicting: unsafe extern "C" fn(*const SpecHandle) -> ConflictKinds,
    pub multiple_per_town: unsafe extern "C" fn() -> bool,
    pub closest_town: unsafe extern "C" fn(u32, u32) -> *const TownHandle,
    pub town: unsafe extern "C" fn(*mut ShellHandle) -> *const TownHandle,
    pub population: unsafe extern "C" fn(*const TownHandle) -> u32,
    pub town_position: unsafe extern "C" fn(*const TownHandle) -> u32,
    pub industry: unsafe extern "C" fn(u16) -> *mut ShellHandle,
    pub highest_snow_line: unsafe extern "C" fn() -> u8,
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
    pub tile_grf_version: unsafe extern "C" fn(*const super::TileSpec) -> u8,
    pub shape_callback: unsafe extern "C" fn(*const super::TileSpec) -> bool,
    pub terragenesis: unsafe extern "C" fn() -> bool,
    pub ignore_restrictions: unsafe extern "C" fn() -> bool,
    pub platform: unsafe extern "C" fn() -> u8,
    pub terraform: unsafe extern "C" fn(*mut IndustryCost, u32, u16, bool),
    pub can_allocate: unsafe extern "C" fn() -> bool,
    pub allocate: unsafe extern "C" fn(u32) -> Allocation,
    pub prospecting_chance: unsafe extern "C" fn(*const SpecHandle) -> u32,
    pub local_company: unsafe extern "C" fn() -> bool,
    pub prospect_no_place: unsafe extern "C" fn(),
    pub prospect_unlucky: unsafe extern "C" fn(),
    pub construction_cost: unsafe extern "C" fn(*const SpecHandle) -> i64,
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
    pub clear_areas: unsafe extern "C" fn() -> ClearAreas,
    pub restore_clear_areas: unsafe extern "C" fn(*const ClearArea, usize),
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
    fn company(&self) -> u8 {
        unsafe { (self.p.current_company)() }
    }
    fn swap_company(&self, company: u8) -> u8 {
        let original = self.company();
        unsafe { (self.x.set_company)(company) };
        original
    }
    fn map(&self) -> super::Map {
        self.context.map
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
            (self.x.edge_distance)(tile, 0) < x
                || (self.x.edge_distance)(tile, 3) < y
                || (self.x.edge_distance)(tile, 2) < x
                || (self.x.edge_distance)(tile, 1) < y
        }
    }
    fn builtin_location(&self, tile: u32, spec: *const SpecHandle) -> bool {
        match unsafe { (self.x.check_proc)(spec) } {
            1 => {
                if self.map().landscape == 1
                    && (self.s.shared().tile_z)(tile)
                        < i32::from(unsafe { (self.x.highest_snow_line)() }) + 2
                {
                    return self.fail(self.e.forest);
                }
            }
            2 => {
                if !unsafe { (self.g.editor)() }
                    && !self.edge(
                        self.offset(tile, 1, 1),
                        u32::from(unsafe { (self.x.refinery_limit)() }),
                    )
                {
                    return self.fail(self.e.positioned);
                }
            }
            3 => {
                if self.map().landscape == 1
                    && (self.s.shared().tile_z)(tile).wrapping_add(2)
                        >= i32::from(unsafe { (self.x.highest_snow_line)() })
                {
                    return self.fail(self.e.site);
                }
            }
            4 if (self.s.shared().tropic_zone)(tile) == 1 => return self.fail(self.e.site),
            5 if (self.s.shared().tropic_zone)(tile) != 1 => return self.fail(self.e.desert),
            6 if (self.s.shared().tropic_zone)(tile) != 2 => return self.fail(self.e.rainforest),
            7 if (self.s.shared().tile_z)(tile) > 4 => return self.fail(self.e.lowland),
            8 => {
                if unsafe { (self.g.editor)() } && unsafe { (self.x.ignore_restrictions)() } {
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
        let spec = unsafe { (self.g.spec)(kind) };
        let conflicting = unsafe { (self.x.conflicting)(spec) };
        for other in conflicting.kinds {
            if other == 255 {
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
        let town = unsafe { (self.x.closest_town)(tile, u32::MAX) };
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
        let spec = unsafe { (self.g.spec)(kind) };
        if unsafe { (self.g.spec_behaviour)(spec) } & 8 != 0
            && unsafe { (self.x.population)(town) } < 1200
        {
            return self.fail(self.e.population);
        }
        let spec = unsafe { (self.g.spec)(kind) };
        if unsafe { (self.g.spec_behaviour)(spec) } & 32 != 0
            && self.distance(unsafe { (self.x.town_position)(town) }, tile) > 9
        {
            return self.fail(self.e.near_town);
        }
        self.success()
    }
    fn free(&self, tile: u32, layout: *const LayoutHandle, kind: u8) -> bool {
        let behaviour = unsafe { (self.g.spec_behaviour)((self.g.spec)(kind)) };
        for n in 0..unsafe { (self.c.layout_size)(layout) } {
            let part = unsafe { (self.c.layout_tile)(layout, n) };
            let gfx = unsafe { (self.c.translated_gfx)(part.gfx) };
            let current =
                unsafe { (self.s.tile_add_wrap)(tile, i32::from(part.x), i32::from(part.y)) };
            if current == u32::MAX {
                return self.fail(self.e.site);
            }
            if gfx == 255 {
                if !(self.s.shared().is_water_tile)(current)
                    || (self.s.shared().tile_slope)(current) != 0
                {
                    return self.fail(self.e.site);
                }
            } else {
                unsafe { ((*self.context.command_checks).no_vehicle)(self.out.cast(), current) };
                if !self.succeeded() {
                    return false;
                }
                if (self.s.shared().bridge_above)(current) {
                    return self.fail(self.e.site);
                }
                let spec = unsafe { (self.t.spec)(gfx) };
                if unsafe { (self.p.slopes_refused)(spec) } & 32 == 0
                    && ((self.s.shared().has_water_class)(current)
                        && unsafe { (self.t.on_water)(current) })
                        != (behaviour & 4 != 0)
                {
                    return self.fail(self.e.site);
                }
                if behaviour & 0x18 != 0
                    || (behaviour & 32 != 0 && (self.s.shared().tile_type)(current) == 3)
                {
                    if (self.s.shared().tile_type)(current) != 3 {
                        return self.fail(self.e.town);
                    }
                    let company = self.swap_company(15);
                    unsafe {
                        (self.p.landscape_clear)(self.out, current, 0);
                        (self.x.set_company)(company);
                    };
                    if !self.succeeded() {
                        return false;
                    }
                } else {
                    unsafe { (self.p.landscape_clear)(self.out, current, 2 | 16 | 512) };
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
        for n in 0..unsafe { (self.c.layout_size)(layout) } {
            let part = unsafe { (self.c.layout_tile)(layout, n) };
            let gfx = unsafe { (self.c.translated_gfx)(part.gfx) };
            let current =
                unsafe { (self.s.tile_add_wrap)(tile, i32::from(part.x), i32::from(part.y)) };
            if gfx != 255 {
                let spec = unsafe { (self.t.spec)(gfx) };
                if unsafe { (self.x.shape_callback)(spec) } {
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
                    refused |= slope_refused((self.s.shared().tile_slope)(current), unsafe {
                        (self.p.slopes_refused)(spec)
                    });
                }
            }
        }
        *custom = custom_shape;
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
            || (self.s.shared().tile_type)(tile) == 7
        {
            return false;
        }
        for y in 0..2 {
            for x in 0..2 {
                let walk = self.offset(tile, x - 1, y - 1);
                let current = unsafe { (self.x.tile_height)(walk) };
                if (self.s.shared().tile_type)(walk) != 0 && (self.s.shared().tile_type)(walk) != 4
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
        for n in 0..unsafe { (self.c.layout_size)(layout) } {
            let part = unsafe { (self.c.layout_tile)(layout, n) };
            if part.gfx == 255 {
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
        let origin = self.offset(tile, -(platform as i32), -(platform as i32));
        let width = (max_x.wrapping_add(2).wrapping_add((2 * platform) as i32)) as u16;
        let height_area = (max_y.wrapping_add(2).wrapping_add((2 * platform) as i32)) as u16;
        if (origin % map.size_x).wrapping_add(u32::from(width)) >= map.size_x - 1
            || (origin / map.size_x).wrapping_add(u32::from(height_area)) >= map.size_y - 1
        {
            return false;
        }
        let company = self.swap_company(15);
        for y in 0..u32::from(height_area) {
            for x in 0..u32::from(width) {
                let walk = self.offset(origin, x as i32, y as i32);
                let current = unsafe { (self.x.tile_height)(walk) };
                if current != height {
                    if !self.surrounding(walk, height, 0) {
                        unsafe { (self.x.set_company)(company) };
                        return false;
                    }
                    unsafe { (self.x.terraform)(self.out, walk, flags & !1, current <= height) };
                    if !self.succeeded() {
                        unsafe { (self.x.set_company)(company) };
                        return false;
                    }
                }
            }
        }
        if flags & 1 != 0 {
            for y in 0..u32::from(height_area) {
                for x in 0..u32::from(width) {
                    let walk = self.offset(origin, x as i32, y as i32);
                    let mut current = unsafe { (self.x.tile_height)(walk) };
                    while current != height {
                        unsafe { (self.x.terraform)(self.out, walk, flags, current <= height) };
                        current = if current > height {
                            current.wrapping_sub(1)
                        } else {
                            current.wrapping_add(1)
                        };
                    }
                }
            }
        }
        unsafe { (self.x.set_company)(company) };
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
        let spec = unsafe { (self.g.spec)(kind) };
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
        let layout = unsafe { (self.x.layout)(spec, layout_index) };
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
        let areas = unsafe { (self.context.clear_areas)() };
        let scratch = if areas.size == 0 {
            Vec::new()
        } else {
            unsafe { slice::from_raw_parts(areas.data, areas.size) }.to_vec()
        };
        let free = self.free(tile, layout, kind);
        unsafe { (self.context.restore_clear_areas)(scratch.as_ptr(), scratch.len()) };
        if !free {
            return false;
        }
        if unsafe { (self.c.callback_mask)((self.g.spec)(kind)) } & 8 != 0 {
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
        if flags & 1 != 0 {
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
    let town = unsafe { (e.x.closest_town)(call.tile, u32::MAX) };
    unsafe {
        (e.c.set_town)(shell.cast(), town);
        (*owner).metadata.random = call.bits;
        (*owner).metadata.founder = call.founder;
    }
    let result =
        unsafe { (e.x.resolve_location)(shell, call.tile, call.kind, call.random, call.creation) };
    if result.callback == 0xffff {
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
    if result.callback == 0xffff {
        if !slope_refused((e.s.shared().tile_slope)(call.current), unsafe {
            (e.p.slopes_refused)(call.spec)
        }) {
            e.success();
        } else {
            e.fail(e.e.site);
        }
    } else if unsafe { (e.x.tile_grf_version)(call.spec) } < 7 {
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
    if current & 16 != 0 {
        return true;
    }
    if current != 0 {
        if refused & 16 != 0 {
            return true;
        }
        let complement = current ^ 15;
        if refused & 1 != 0 && complement & 9 != 0 {
            return true;
        }
        if refused & 2 != 0 && complement & 0x0C != 0 {
            return true;
        }
        if refused & 4 != 0 && complement & 3 != 0 {
            return true;
        }
        if refused & 8 != 0 && complement & 6 != 0 {
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
    if kind >= 240 {
        e.fail(0xffff);
        return;
    }
    let spec = unsafe { (e.g.spec)(kind) };
    if !unsafe { (e.x.enabled)(spec) } || unsafe { (e.x.layout_count)(spec) } == 0 {
        e.fail(0xffff);
        return;
    }
    if !unsafe { (e.g.editor)() }
        && e.company() != 18
        && unsafe { (e.x.raw_setting)() } == 0
        && unsafe { (e.x.raw)(spec) }
    {
        e.fail(0xffff);
        return;
    }
    if !unsafe { (e.g.editor)() }
        && unsafe { (e.s.probability_callback)(kind, if e.company() == 18 { 1 } else { 2 }, 1) }
            == 0
    {
        e.fail(0xffff);
        return;
    }
    // The local Randomizer is seeded twice with seed and Next is called once.
    // Its returned second word is rotr(seed, 3) - 1; the updated first word is unused.
    let bits = seed as u16;
    let random = seed.rotate_right(3).wrapping_sub(1);
    let count = unsafe { (e.x.layout_count)(spec) };
    e.fail(e.e.site);
    let deity_prospect = e.company() == 18 && !fund;
    let mut industry = ptr::null_mut();
    if deity_prospect
        || (!unsafe { (e.g.editor)() }
            && e.company() != 18
            && unsafe { (e.x.raw_setting)() } == 2
            && unsafe { (e.x.raw)(spec) })
    {
        if flags & 1 != 0 {
            let succeeded = deity_prospect
                || e.s.shared().random() <= unsafe { (e.x.prospecting_chance)(spec) };
            if succeeded {
                let creation = if e.company() == 18 { 1 } else { 3 };
                let company = e.swap_company(15);
                for _ in 0..5000 {
                    let map = e.map();
                    let candidate =
                        e.s.shared().random() & map.size_x.wrapping_mul(map.size_y).wrapping_sub(1);
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
                            company,
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
                unsafe { (e.x.set_company)(company) };
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
        let mut layout = first_layout as usize;
        if layout >= count {
            e.fail(0xffff);
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
                e.company(),
                if e.company() == 18 { 1 } else { 2 },
                &raw mut industry,
            ) {
                break;
            }
        }
        if !e.succeeded() {
            return;
        }
    }
    if flags & 1 != 0 && !industry.is_null() && !unsafe { (e.g.editor)() } {
        unsafe { (e.s.advertise)(industry.cast()) };
    }
    let construction = unsafe { (e.x.construction_cost)(spec) };
    unsafe { (e.cost.expense_money)(out, 12, construction) };
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
    let spec = unsafe { (e.g.spec)(kind) };
    let random = e.s.shared().random();
    let second = e.s.shared().random();
    let count = unsafe { (e.x.layout_count)(spec) };
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
pub unsafe extern "C" fn openttd_rust_industry_slope_refused(current: u8, refused: u8) -> bool {
    slope_refused(current, refused)
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
