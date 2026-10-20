/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Object lifecycle owner. Native pools, specs, map and save records remain
//! canonical. Counts and cleared-area command scratch have one process-wide
//! Rust owner, serialized by the game-state mutex. No state borrow spans a
//! reentrant service. Panics abort; native exceptions never unwind through Rust.
#![allow(
    clippy::similar_names,
    clippy::if_not_else,
    clippy::manual_midpoint,
    unsafe_code,
    missing_docs,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::fn_params_excessive_bools
)]
use std::cell::UnsafeCell;

struct State {
    counts: [u16; 64000],
    areas: Vec<ClearArea>,
}
struct Global(UnsafeCell<State>);
// SAFETY: native callers serialize every entry under the game-state mutex.
// An access ends before any service can reenter; the owner is process-wide.
unsafe impl Sync for Global {}
static STATE: Global = Global(UnsafeCell::new(State {
    counts: [0; 64000],
    areas: Vec::new(),
}));

/// Value-only command scratch entry. Indices, never pointers, survive append.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ClearArea {
    pub first_tile: u32,
    pub tile: u32,
    pub width: u16,
    pub height: u16,
}
/// Industry transaction backup at the original copy point.
pub(crate) fn snapshot_areas() -> Vec<ClearArea> {
    // SAFETY: caller holds the game-state mutex; no callback in this scope.
    unsafe { (*STATE.0.get()).areas.clone() }
}
/// Industry transaction restore at the original move point.
pub(crate) fn restore_areas(areas: Vec<ClearArea>) {
    // SAFETY: caller holds the game-state mutex; no callback in this scope.
    unsafe { (*STATE.0.get()).areas = areas };
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_counts_reset() {
    // SAFETY: serialized native entry.
    unsafe { (*STATE.0.get()).counts.fill(0) };
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_count_increment(kind: u16) {
    // SAFETY: serialized entry; native source asserts kind < NUM_OBJECTS.
    let count = unsafe { &mut (*STATE.0.get()).counts[usize::from(kind)] };
    *count = count.wrapping_add(1);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_count_decrement(kind: u16) {
    // SAFETY: serialized entry; native source asserts kind < NUM_OBJECTS.
    let count = unsafe { &mut (*STATE.0.get()).counts[usize::from(kind)] };
    *count = count.wrapping_sub(1);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_count(kind: u16) -> u16 {
    // SAFETY: serialized native entry; original kind precondition.
    unsafe { (*STATE.0.get()).counts[usize::from(kind)] }
}
pub(crate) fn clear_areas() {
    // SAFETY: serialized native entry.
    unsafe { (*STATE.0.get()).areas.clear() };
}
fn openttd_rust_object_areas_find(tile: u32, log_x: u32) -> usize {
    let mask = (1 << log_x) - 1;
    let x = tile & mask;
    let y = tile >> log_x;
    // SAFETY: serialized entry, no callbacks while reading.
    unsafe { &(*STATE.0.get()).areas }
        .iter()
        .position(|a| {
            if a.width == 0 {
                return false;
            }
            assert!(a.height != 0);
            let left = a.tile & mask;
            let top = a.tile >> log_x;
            x >= left && x < left + u32::from(a.width) && y >= top && y < top + u32::from(a.height)
        })
        .unwrap_or(usize::MAX)
}
fn openttd_rust_object_areas_get(index: usize) -> ClearArea {
    // SAFETY: serialized entry; index was returned by find/append.
    unsafe { (&(*STATE.0.get()).areas)[index] }
}
fn openttd_rust_object_areas_append(area: ClearArea) -> usize {
    // SAFETY: serialized entry; allocation failure aborts.
    let areas = unsafe { &mut (*STATE.0.get()).areas };
    areas.push(area);
    areas.len() - 1
}
fn openttd_rust_object_areas_set_first(index: usize, first: u32) {
    // SAFETY: serialized entry; append preserves indices returned by find/append.
    unsafe { (&mut (*STATE.0.get()).areas)[index].first_tile = first };
}

/// Stable tunnel scratch token; append never invalidates its index.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HiddenArea {
    pub index: usize,
    pub first_tile: u32,
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_indirectly_cleared(tile: u32, log_x: u32) -> bool {
    let i = openttd_rust_object_areas_find(tile, log_x);
    i != usize::MAX && openttd_rust_object_areas_get(i).first_tile != tile
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_tunnel_hide(tile: u32, log_x: u32) -> HiddenArea {
    let mut index = openttd_rust_object_areas_find(tile, log_x);
    if index == usize::MAX {
        index = openttd_rust_object_areas_append(ClearArea {
            first_tile: tile,
            tile,
            width: 1,
            height: 1,
        });
    }
    let first_tile = openttd_rust_object_areas_get(index).first_tile;
    openttd_rust_object_areas_set_first(index, u32::MAX);
    HiddenArea { index, first_tile }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_restore_first(hidden: HiddenArea) {
    openttd_rust_object_areas_set_first(hidden.index, hidden.first_tile);
}

use crate::services::{
    CommandCost, CommandCostServices, CommandServices, Services, TownHandle, WaterClass,
};
use std::ptr;

#[repr(C)]
pub struct Price {
    pub factor: u32,
    pub shift: i32,
}
impl Price {
    fn money(self, index: u32) -> i64 {
        crate::company::openttd_rust_economy_price(index, self.factor, self.shift)
    }
}

#[repr(C)]
pub struct Spec {
    _private: [u8; 0],
}
#[repr(C)]
pub struct Object {
    _private: [u8; 0],
}
#[repr(C)]
pub struct LoopFrame {
    _private: [u8; 0],
}
#[repr(C)]
pub struct GenerationFrame {
    _private: [u8; 0],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Area {
    pub tile: u32,
    pub width: u16,
    pub height: u16,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Identity {
    pub object: *mut Object,
    pub area: Area,
    pub kind: u16,
}
#[repr(C)]
pub struct Reference {
    pub object: *mut Object,
    pub kind: u16,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Allocation {
    pub object: *mut Object,
    pub id: u32,
}
#[repr(C)]
pub struct Next {
    pub object: *mut Object,
    pub id: u32,
    pub tile: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LoopPolicy {
    pub spec: *const Spec,
    pub object: *mut Object,
    pub north: u32,
    pub flags: u16,
    pub kind: u16,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Policy {
    pub flags: u16,
    pub callbacks: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Footprint {
    pub flags: u16,
    pub callbacks: u8,
    pub size: u8,
}
#[repr(C)]
pub struct Clearance {
    pub flags: u16,
    pub height: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Admission {
    pub views: u8,
    pub height: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AreaShape {
    pub views: u8,
    pub size: u8,
}
#[repr(C)]
pub struct Generation {
    pub flags: u16,
    pub amount: u8,
    pub views: u8,
    pub ever: bool,
}
#[repr(C)]
pub struct SlopeResult {
    pub callback: u16,
    pub registers: [i32; 16],
    pub version: u8,
}
#[repr(C)]
pub struct Strings {
    pub too_many: u32,
    pub no_town: u32,
    pub edge_sub: u32,
    pub water: u32,
    pub edge: u32,
    pub on_water: u32,
    pub bridge: u32,
    pub flat: u32,
    pub own: u32,
    pub built: u32,
    pub limit: u32,
    pub hq: u32,
    pub object: u32,
}
#[repr(C)]
pub struct ObjectServices {
    pub shared: *const Services,
    pub cost: *const CommandCostServices,
    pub command: *const CommandServices,
    pub company: *const crate::company::Services,
    pub cargo: *const crate::cargo_storage::Services,
    pub town: *const crate::town::Leaves,
    pub founding: *const crate::town::founding::FoundingServices,
    pub industry_tiles: *const crate::industry::TileServices,
    pub strings: Strings,
    pub spec_count: unsafe extern "C" fn() -> usize,
    pub spec: unsafe extern "C" fn(u16) -> *const Spec,
    pub callbacks: unsafe extern "C" fn(*const Spec) -> u8,
    pub colour_policy: unsafe extern "C" fn(*mut Object) -> Policy,
    pub flags: unsafe extern "C" fn(*const Spec) -> u16,
    pub footprint: unsafe extern "C" fn(*const Spec) -> Footprint,
    pub clearance: unsafe extern "C" fn(*const Spec) -> Clearance,
    pub available: unsafe extern "C" fn(*const Spec) -> bool,
    pub ever: unsafe extern "C" fn(*const Spec) -> bool,
    pub area_shape: unsafe extern "C" fn(*const Spec) -> AreaShape,
    pub admission: unsafe extern "C" fn(*const Spec) -> Admission,
    pub generation: unsafe extern "C" fn(*const Spec) -> Generation,
    pub can_allocate: unsafe extern "C" fn() -> bool,
    pub allocate: unsafe extern "C" fn(u16, *const TownHandle, Area, u8) -> Allocation,
    pub nearest_town: unsafe extern "C" fn(u32) -> *const TownHandle,
    pub object_type: unsafe extern "C" fn(u32) -> u16,
    pub loop_policy: unsafe extern "C" fn(u32) -> LoopPolicy,
    pub reference: unsafe extern "C" fn(u32) -> Reference,
    pub identity: unsafe extern "C" fn(u32) -> Identity,
    pub location: unsafe extern "C" fn(u32) -> Area,
    pub next: unsafe extern "C" fn(u32) -> Next,
    pub set_colour: unsafe extern "C" fn(*mut Object, u8) -> (),
    pub recolour: unsafe extern "C" fn(u8, bool) -> u8,
    pub colour_callback: unsafe extern "C" fn(*const Spec, *mut Object, u8, u32) -> u16,
    pub colour_error: unsafe extern "C" fn(*const Spec, u16) -> (),
    pub make: unsafe extern "C" fn(u32, u8, u32, WaterClass, u32) -> (),
    pub delete_object: unsafe extern "C" fn(*mut Object) -> (),
    pub trigger: unsafe extern "C" fn(*mut Object, u8, *const Spec) -> (),
    pub trigger_tile: unsafe extern "C" fn(*mut Object, u32, *const Spec) -> (),
    pub slope_callback:
        unsafe extern "C" fn(*const Spec, u32, crate::services::Slope, u32, u8) -> SlopeResult,
    pub slope_error: unsafe extern "C" fn(*mut CommandCost, *const SlopeResult, *const Spec) -> (),
    pub buildable: unsafe extern "C" fn(*mut CommandCost, u32, *mut i32) -> (),
    pub build_price: unsafe extern "C" fn(*const Spec) -> Price,
    pub clear_price: unsafe extern "C" fn(*const Spec) -> Price,
    pub set_hq: unsafe extern "C" fn(u8, u32) -> (),
    pub statue_town: unsafe extern "C" fn(*mut Object) -> crate::town::TownRef,
    pub hq_begin: unsafe extern "C" fn(*mut LoopFrame, u32, u32) -> (),
    pub hq_deliver: unsafe extern "C" fn(*mut LoopFrame, u8, u32, u8) -> (),
    pub hq_end: unsafe extern "C" fn(*mut LoopFrame) -> (),
    pub autoslope_callback: unsafe extern "C" fn(*const Spec, *mut Object, u32) -> u16,
    pub boolean_callback: unsafe extern "C" fn(*const Spec, u16, u16) -> bool,
    pub enabled: unsafe extern "C" fn(*const Spec) -> bool,
    pub bridge_top: unsafe extern "C" fn(u32) -> i32,
    pub close_inspect: unsafe extern "C" fn(u32) -> (),
    pub progress_set: unsafe extern "C" fn(*mut GenerationFrame, u32) -> bool,
    pub progress_increase: unsafe extern "C" fn(*mut GenerationFrame) -> bool,
    pub normal: unsafe extern "C" fn() -> bool,
    pub freeform: unsafe extern "C" fn() -> bool,
    pub bulldozer: unsafe extern "C" fn() -> bool,
    pub water_class: unsafe extern "C" fn(u32) -> WaterClass,
    pub restoration: *const crate::water::RestorationServices,
    pub set_owner: unsafe extern "C" fn(u32, u8) -> (),
    pub scale_size_1d: unsafe extern "C" fn(u32) -> u32,
    pub passengers: unsafe extern "C" fn() -> u8,
    pub mail: unsafe extern "C" fn() -> u8,
    pub scale_cargo: unsafe extern "C" fn(u32) -> u32,
}

const NONE: u8 = 16;
const TOWN: u8 = 15;
const WATER: u8 = 17;
const DEITY: u8 = 18;
const INVALID_OWNER: u8 = 255;
const INVALID_TILE: u32 = u32::MAX;
const HQ: u16 = 4;
const OWNED: u16 = 3;
const STATUE: u16 = 2;
const LIGHTHOUSE: u16 = 1;
const TRANSMITTER: u16 = 0;
const EXECUTE: u16 = 1;
const AUTO: u16 = 2;
const NO_WATER: u16 = 8;
const CONSTRUCTION: u8 = 0;
const FAILED: u16 = 0xffff;
const SCENEDIT: u16 = 1;
const CANNOT_REMOVE: u16 = 1 << 1;
const AUTOREMOVE: u16 = 1 << 2;
const BUILT_ON_WATER: u16 = 1 << 3;
const CLEAR_INCOME: u16 = 1 << 4;
const ANIMATION: u16 = 1 << 6;
const IN_GAME: u16 = 1 << 7;
const TWO_CC: u16 = 1 << 8;
const NOT_ON_LAND: u16 = 1 << 9;
const UNDER_BRIDGE: u16 = 1 << 11;
const SCALE_WATER: u16 = 1 << 13;

#[derive(Clone, Copy)]
struct Cost<'a>(*mut CommandCost, &'a CommandCostServices);
impl Cost<'_> {
    fn zero(self) {
        (self.1.zero)(self.0);
    }
    fn set(self, money: i64) {
        (self.1.expense_money)(self.0, CONSTRUCTION, money);
    }
    fn fail(self, error: u32) {
        (self.1.error)(self.0, error);
    }
    fn error(self) {
        self.fail(0xffff);
    }
    fn add(self, cost: Self) {
        (self.1.add)(self.0, cost.0);
    }
    fn money(self) -> i64 {
        (self.1.money)(self.0)
    }
    fn amount(self, money: i64) {
        (self.1.amount)(self.0, money);
    }
    fn ok(self) -> bool {
        (self.1.success)(self.0)
    }
    fn take(self, cost: Self) {
        (self.1.move_cost)(self.0, cost.0);
    }
}
struct World<'a> {
    s: &'a ObjectServices,
    shared: &'a Services,
    map: crate::services::Map,
}
impl<'a> World<'a> {
    unsafe fn new<const NEEDS_MAP: bool>(s: &'a ObjectServices) -> Self {
        // SAFETY: immutable native tables live throughout the entry.
        let shared = unsafe { &*s.shared };
        Self {
            s,
            shared,
            map: crate::services::Map {
                log_x: if NEEDS_MAP {
                    (shared.map_log_x)() as u8
                } else {
                    0
                },
            },
        }
    }
    fn commands(&self) -> &CommandServices {
        // SAFETY: entry provides the immutable table when commands are used.
        unsafe { &*self.s.command }
    }
    fn company(&self) -> &crate::company::Services {
        // SAFETY: entry provides the immutable table when company services are used.
        unsafe { &*self.s.company }
    }
    fn tiles(&self, a: Area) -> impl Iterator<Item = u32> {
        let stride = self.map.size_x();
        (0..u32::from(a.height)).flat_map(move |y| {
            (0..u32::from(a.width))
                .map(move |x| a.tile.wrapping_add(y.wrapping_mul(stride)).wrapping_add(x))
        })
    }
    fn footprint(tile: u32, shape: Footprint, view: u8) -> Area {
        let (x, y) = if view & 1 == 0 {
            (shape.size & 15, shape.size >> 4)
        } else {
            (shape.size >> 4, shape.size & 15)
        };
        Area {
            tile,
            width: u16::from(x),
            height: u16::from(y),
        }
    }
    fn is_type(&self, tile: u32, kind: u16) -> bool {
        (self.shared.tile_type)(tile) == 10 && unsafe { (self.s.object_type)(tile) == kind }
    }
    fn owner(&self, tile: u32) -> u8 {
        (self.shared.tile_owner)(tile)
    }
    fn current(&self) -> u8 {
        (self.shared.current_company)()
    }
    fn finance(&self, owner: u8) -> *mut crate::company::Finances {
        (self.company().finance.company_finances)(owner)
    }
    fn object_limit(finance: *const crate::company::Finances) -> u32 {
        if finance.is_null() {
            return i32::MAX as u32;
        }
        // SAFETY: immediate field copy; no borrow survives services.
        unsafe { (*finance).object >> 16 }
    }
    fn water_loop(&self, tile: u32, call: *mut crate::ground::Call) {
        // SAFETY: own live native result context and immutable shared tables.
        unsafe {
            crate::water::tree_industry_water_loop(tile, call, self.shared);
        }
    }
    fn build(&self, kind: u16, tile: u32, owner: u8, town: *const TownHandle, view: u8) {
        unsafe {
            let spec = (self.s.spec)(kind);
            let shape = (self.s.footprint)(spec);
            let area = Self::footprint(tile, shape, view);
            let town = if town.is_null() {
                (self.s.nearest_town)(tile)
            } else {
                town
            };
            let object = (self.s.allocate)(kind, town, area, view);
            let mut colour = if owner == NONE {
                self.shared.random() as u8
            } else {
                (self.s.recolour)(owner, true)
            };
            if shape.flags & TWO_CC == 0 {
                colour &= 15;
            }
            (self.s.set_colour)(object.object, colour);
            if shape.callbacks & 8 != 0 {
                let result = (self.s.colour_callback)(spec, object.object, colour, tile);
                if result != FAILED {
                    if result >= 256 {
                        (self.s.colour_error)(spec, result);
                    }
                    (self.s.set_colour)(object.object, result as u8);
                }
            }
            for t in self.tiles(area) {
                if (self.shared.is_water_tile)(t) {
                    crate::water::openttd_rust_clear_neighbour_flooding(
                        t,
                        self.map.size_x(),
                        (self.shared.map_max_y)() + 1,
                        (self.shared.water_services)(),
                        self.shared,
                    );
                }
                if (self.shared.has_water_ground)(t) {
                    (&*(self.shared.water_services)()).invalidate_region(t, self.shared);
                }
                let wc = if (self.shared.is_water_tile)(t) {
                    (self.s.water_class)(t)
                } else {
                    WaterClass(3)
                };
                if wc.0 == 1 && owner != NONE && matches!(self.owner(t), NONE | WATER) {
                    ((*self.s.restoration).infrastructure)(owner, 1);
                    ((*self.s.restoration).infrastructure_dirty)(owner);
                }
                let remove = (self.shared.clear_square_services.docking)(t);
                (self.s.make)(t, owner, object.id, wc, self.shared.random());
                if remove {
                    (self.shared.clear_square_services.remove_docking)(t);
                }
                (self.shared.mark_dirty)(t);
            }
            openttd_rust_object_count_increment(kind);
            if shape.flags & ANIMATION != 0 {
                (self.s.trigger)(object.object, 0, spec);
            }
        }
        crate::witness::hit(crate::witness::OBJECT_BUILD);
    }
    fn update_hq(&self, tile: u32, score: u32) {
        if tile == INVALID_TILE {
            return;
        }
        let target = [170, 350, 520, 720]
            .into_iter()
            .filter(|&n| score >= n)
            .count() as u8;
        unsafe {
            while ((*self.s.industry_tiles).frame)(tile) < target {
                let area = (self.s.location)(tile);
                for t in self.tiles(area) {
                    ((*self.s.industry_tiles).set_frame)(
                        t,
                        ((*self.s.industry_tiles).frame)(t).wrapping_add(1),
                    );
                    (self.shared.mark_dirty)(t);
                }
                crate::witness::hit(crate::witness::OBJECT_HQ_GROW);
            }
        }
    }
    fn really_clear(&self, object: Identity) {
        openttd_rust_object_count_decrement(object.kind);
        unsafe {
            for tile in self.tiles(object.area) {
                (self.s.close_inspect)(tile);
                crate::water::restore(
                    tile,
                    self.owner(tile),
                    self.shared,
                    &*(self.shared.water_services)(),
                    &*self.s.restoration,
                );
            }
            (self.s.delete_object)(object.object);
        }
        crate::witness::hit(crate::witness::OBJECT_REMOVE);
    }
    fn clear(&self, out: Cost<'_>, tmp: Cost<'_>, tile: u32, flags: u16) {
        unsafe {
            let object = (self.s.identity)(tile);
            let spec = (self.s.spec)(object.kind);
            let flags_spec = (self.s.flags)(spec);
            let price = (self.s.clear_price)(spec)
                .money(51)
                .saturating_mul(i64::from(object.area.width))
                .saturating_mul(i64::from(object.area.height))
                / 5;
            out.set(if flags_spec & CLEAR_INCOME == 0 {
                price
            } else {
                negate(price)
            });
            let company = self.current();
            if company == TOWN {
                out.error();
                return;
            }
            if company != WATER {
                if flags & NO_WATER != 0 && ((*self.s.industry_tiles).on_water)(tile) {
                    out.fail(self.s.strings.water);
                    return;
                }
                if flags_spec & AUTOREMOVE == 0 && flags & AUTO != 0 {
                    out.fail(if object.kind == HQ {
                        self.s.strings.hq
                    } else {
                        self.s.strings.object
                    });
                    return;
                }
                if ((*self.s.town).editor)() {
                } else if self.owner(tile) == NONE {
                    if !(self.s.bulldozer)() && flags_spec & CANNOT_REMOVE != 0 {
                        out.error();
                        return;
                    }
                } else {
                    (self.commands().tile_ownership)(tmp.0, tile);
                    if !tmp.ok() {
                        out.take(tmp);
                        return;
                    }
                    if flags_spec & CANNOT_REMOVE != 0 && flags_spec & AUTOREMOVE == 0 {
                        if !(self.s.bulldozer)() {
                            if object.kind == HQ {
                                out.fail(self.s.strings.hq);
                            } else {
                                out.error();
                            }
                            return;
                        }
                        out.set(out.money().saturating_mul(25));
                    }
                }
            } else if flags_spec & (BUILT_ON_WATER | NOT_ON_LAND) != 0 {
                out.error();
                return;
            }
            if object.kind == HQ {
                let owner = self.owner(tile);
                if flags & EXECUTE != 0 {
                    (self.s.set_hq)(owner, INVALID_TILE);
                    crate::cargo_storage::openttd_rust_cargo_invalidate(
                        self.s.cargo,
                        u16::from(owner),
                        2,
                    );
                }
                out.set(
                    crate::company::openttd_rust_company_value(
                        self.company(),
                        self.finance(owner),
                        owner,
                        true,
                    ) / 100,
                );
            } else if object.kind == STATUE && flags & EXECUTE != 0 {
                let town = (self.s.statue_town)(object.object);
                let fields = crate::town::openttd_rust_town_simulation(town.state);
                (*fields).statues &= !((1_u64 << self.owner(tile)) as u16);
                ((*self.s.town).dirty_authority)(u32::from(town.id.0));
            }
            openttd_rust_object_areas_append(ClearArea {
                first_tile: tile,
                tile: object.area.tile,
                width: object.area.width,
                height: object.area.height,
            });
            if flags & EXECUTE != 0 {
                self.really_clear(object);
            }
        }
        crate::witness::hit(crate::witness::OBJECT_CLEAR);
    }
    fn build_command(
        &self,
        out: Cost<'_>,
        tmp: Cost<'_>,
        aux: Cost<'_>,
        flags: u16,
        tile: u32,
        kind: u16,
        view: u8,
    ) {
        unsafe {
            out.set(0);
            if usize::from(kind) >= (self.s.spec_count)() {
                out.error();
                return;
            }
            let spec = (self.s.spec)(kind);
            let admission = (self.s.admission)(spec);
            if (self.s.normal)()
                && !(self.s.available)(spec)
                && !((*self.s.town).generating_world)()
            {
                out.error();
                return;
            }
            if (((*self.s.town).editor)() || ((*self.s.town).generating_world)())
                && !(self.s.ever)(spec)
            {
                out.error();
                return;
            }
            let shape = (self.s.footprint)(spec);
            if shape.flags & SCENEDIT != 0
                && ((!((*self.s.town).generating_world)() && !((*self.s.town).editor)())
                    || self.current() != NONE)
            {
                out.error();
                return;
            }
            if shape.flags & IN_GAME != 0
                && (((*self.s.town).generating_world)()
                    || !(self.s.normal)()
                    || self.current() > 15)
            {
                out.error();
                return;
            }
            if view >= admission.views {
                out.error();
                return;
            }
            if !(self.s.can_allocate)() {
                out.fail(self.s.strings.too_many);
                return;
            }
            if ((*self.s.founding).town_count)() == 0 {
                out.fail(self.s.strings.no_town);
                return;
            }
            let area = Self::footprint(tile, shape, view);
            for t in self.tiles(area) {
                if !(self.shared.tile_valid)(t) {
                    out.fail(self.s.strings.edge_sub);
                    return;
                }
            }
            if kind == OWNED {
                (self.commands().landscape_clear)(tmp.0, tile, flags);
                out.add(tmp);
            } else {
                let water = shape.flags & (BUILT_ON_WATER | NOT_ON_LAND) != 0;
                let ground = shape.flags & NOT_ON_LAND == 0;
                for t in self.tiles(area) {
                    if (self.shared.has_water_ground)(t) {
                        if !water {
                            out.fail(self.s.strings.water);
                            return;
                        }
                        if !(self.shared.is_water_tile)(t) {
                            (self.commands().landscape_clear)(
                                tmp.0,
                                t,
                                flags & !(NO_WATER | EXECUTE),
                            );
                            out.add(tmp);
                        } else {
                            let owner = self.owner(t);
                            if owner != NONE && owner != WATER {
                                (self.commands().ownership_at)(tmp.0, owner, t);
                                out.add(tmp);
                            }
                            if !(self.s.freeform)()
                                && (self.map.tile_x(t) < 1
                                    || self.map.tile_x(t) >= (self.shared.map_max_x)() - 1
                                    || self.map.tile_y(t) < 1
                                    || self.map.tile_y(t) >= (self.shared.map_max_y)() - 1)
                            {
                                out.fail(self.s.strings.edge);
                                return;
                            }
                            (self.commands().no_vehicle)(tmp.0, t);
                            out.add(tmp);
                        }
                    } else {
                        if !ground {
                            out.fail(self.s.strings.on_water);
                            return;
                        }
                        if !(kind == HQ
                            && (self.shared.tile_type)(t) == 10
                            && self.owner(t) == self.current()
                            && self.is_type(t, HQ))
                        {
                            (self.commands().landscape_clear)(tmp.0, t, flags & !EXECUTE);
                            out.add(tmp);
                        }
                    }
                }
                let mut z = (self.shared.tile_z)(tile);
                if (self.shared.tile_slope)(tile) != 0 {
                    z += 1;
                }
                for t in self.tiles(area) {
                    let mut result = SlopeResult {
                        callback: FAILED,
                        registers: [0; 16],
                        version: 0,
                    };
                    if shape.callbacks & 1 != 0 {
                        let diff = t.wrapping_sub(tile);
                        result = (self.s.slope_callback)(
                            spec,
                            t,
                            crate::services::Slope((self.shared.tile_slope)(t)),
                            self.map.tile_y(diff) << 4 | self.map.tile_x(diff),
                            view,
                        );
                        crate::witness::hit(crate::witness::OBJECT_SLOPE_CALLBACK);
                    }
                    if result.callback == FAILED {
                        (self.s.buildable)(tmp.0, t, &raw mut z);
                        out.add(tmp);
                    } else {
                        if result.version < 8 {
                            result.callback ^= 1 << 10;
                        }
                        (self.s.slope_error)(tmp.0, &raw const result, spec);
                        if !tmp.ok() {
                            out.take(tmp);
                            return;
                        }
                    }
                }
                if flags & EXECUTE != 0 {
                    for t in self.tiles(area) {
                        if (self.shared.has_water_ground)(t) {
                            if !(self.shared.is_water_tile)(t) {
                                (self.commands().landscape_clear)(tmp.0, t, flags & !NO_WATER);
                            }
                        } else {
                            (self.commands().landscape_clear)(tmp.0, t, flags);
                        }
                    }
                }
            }
            if !out.ok() {
                return;
            }
            for t in self.tiles(area) {
                if (self.shared.bridge_above)(t)
                    && (shape.flags & UNDER_BRIDGE == 0
                        || (self.shared.tile_max_z)(t) + i32::from(admission.height)
                            >= (self.s.bridge_top)(t))
                {
                    out.fail(self.s.strings.bridge);
                    return;
                }
            }
            let mut score = 0;
            let mut size = 1_u32;
            match kind {
                TRANSMITTER | LIGHTHOUSE => {
                    if (self.shared.tile_slope)(tile) != 0 {
                        out.fail(self.s.strings.flat);
                        return;
                    }
                }
                OWNED => {
                    if (self.shared.tile_type)(tile) == 10
                        && self.owner(tile) == self.current()
                        && self.is_type(tile, OWNED)
                    {
                        out.fail(self.s.strings.own);
                        return;
                    }
                }
                HQ => {
                    let company = self.current();
                    let old = (self.company().hq_location)(company);
                    if old != INVALID_TILE {
                        if old == tile {
                            out.fail(self.s.strings.built);
                            return;
                        }
                        (self.company().finance.set_current_company)(WATER);
                        self.clear(tmp, aux, old, flags);
                        (self.company().finance.set_current_company)(company);
                        out.add(tmp);
                        crate::witness::hit(crate::witness::OBJECT_HQ_RELOCATE);
                    }
                    if flags & EXECUTE != 0 {
                        score = crate::company::openttd_rust_company_update_rating(
                            self.company(),
                            self.finance(company),
                            company,
                            false,
                        ) as u32;
                        (self.s.set_hq)(company, tile);
                    }
                }
                STATUE => {
                    out.error();
                    return;
                }
                _ => {
                    size = u32::from(area.width) * u32::from(area.height);
                }
            }
            let finance = self.finance(self.current());
            if !finance.is_null() && Self::object_limit(finance) < size {
                out.fail(self.s.strings.limit);
                return;
            }
            if flags & EXECUTE != 0 {
                self.build(
                    kind,
                    tile,
                    if self.current() == DEITY {
                        NONE
                    } else {
                        self.current()
                    },
                    ptr::null(),
                    view,
                );
                if kind == HQ {
                    self.update_hq(tile, score);
                }
                if !finance.is_null() {
                    // SAFETY: immediate canonical scalar access, ended before callbacks.
                    (*finance).object = (*finance).object.wrapping_sub(size << 16);
                }
            }
            out.amount(
                (self.s.build_price)(spec)
                    .money(50)
                    .saturating_mul(i64::from(size)),
            );
        }
        crate::witness::hit(crate::witness::OBJECT_BUILD_COMMAND);
    }
}
fn negate(money: i64) -> i64 {
    if money == i64::MIN { i64::MAX } else { -money }
}

impl World<'_> {
    fn do_build(
        &self,
        out: Cost<'_>,
        tmp: Cost<'_>,
        aux: Cost<'_>,
        test: Cost<'_>,
        flags: u16,
        tile: u32,
        kind: u16,
        view: u8,
    ) {
        if tile != 0
            && (tile >= self.map.size_x() * ((self.shared.map_max_y)() + 1)
                || (!(self.shared.tile_valid)(tile) && flags & 256 == 0))
        {
            out.error();
            return;
        }
        unsafe {
            let protocol = self.commands();
            let top = (protocol.enter)();
            if top || flags & EXECUTE == 0 {
                crate::command::before(protocol, top, true);
                self.build_command(test, tmp, aux, flags & !EXECUTE, tile, kind, view);
                crate::command::after(protocol, &*self.s.cost, test.0, flags, top, true);
                if !test.ok() || flags & EXECUTE == 0 {
                    out.take(test);
                    (protocol.leave)();
                    return;
                }
            }
            crate::command::before(protocol, top, false);
            self.build_command(out, tmp, aux, flags, tile, kind, view);
            crate::command::after(protocol, &*self.s.cost, out.0, flags, top, false);
            (protocol.leave)();
        }
    }
    fn build_area(
        &self,
        out: Cost<'_>,
        ret: Cost<'_>,
        tmp: Cost<'_>,
        aux: Cost<'_>,
        test: Cost<'_>,
        last: Cost<'_>,
        flags: u16,
        tile: u32,
        start: u32,
        kind: u16,
        view: u8,
        diagonal: bool,
    ) {
        unsafe {
            let map_y = (self.shared.map_max_y)() + 1;
            if start >= self.map.size_x() * map_y || usize::from(kind) >= (self.s.spec_count)() {
                out.error();
                return;
            }
            let spec = (self.s.spec)(kind);
            let shape = (self.s.area_shape)(spec);
            if view >= shape.views || shape.size != 0x11 {
                out.error();
                return;
            }
            let mut money = crate::command::available_money(self.commands());
            out.set(0);
            last.error();
            let mut success = false;
            let finance = self.finance(self.current());
            let mut limit = Self::object_limit(finance) as i32;
            let mut iterator = Drag::new(self.map, map_y, tile, start, diagonal);
            while iterator.tile != INVALID_TILE {
                let t = iterator.tile;
                self.do_build(ret, tmp, aux, test, flags & !EXECUTE, t, kind, view);
                if !finance.is_null() {
                    let prior = limit;
                    limit -= 1;
                    if prior <= 0 {
                        break;
                    }
                }
                if !ret.ok() {
                    last.take(ret);
                    iterator.advance();
                    continue;
                }
                success = true;
                let cost = ret.money();
                if flags & EXECUTE != 0 {
                    money = money.saturating_sub(cost);
                    if cost > 0 && money < 0 {
                        break;
                    }
                    self.do_build(ret, tmp, aux, test, flags, t, kind, view);
                }
                out.amount(cost);
                iterator.advance();
            }
            if !success {
                out.take(last);
            }
        }
        crate::witness::hit(crate::witness::OBJECT_AREA);
    }
    fn terraform(&self, out: Cost<'_>, tile: u32, flags: u16, z: i32, slope: u8) {
        unsafe {
            let object = (self.s.reference)(tile);
            if object.kind == OWNED {
                (self.commands().tile_ownership)(out.0, tile);
                if out.ok() {
                    out.zero();
                    return;
                }
            } else if (self.shared.autoslope)()
                && object.kind != TRANSMITTER
                && object.kind != LIGHTHOUSE
            {
                let old_slope = (self.shared.tile_slope)(tile);
                let new_max = if slope == 0 {
                    0
                } else if slope & 16 != 0 {
                    2
                } else {
                    1
                };
                if old_slope & 16 == 0
                    && slope & 16 == 0
                    && (self.shared.tile_max_z)(tile) == z + new_max
                {
                    let spec = (self.s.spec)(object.kind);
                    let callbacks = (self.s.callbacks)(spec);
                    if if callbacks & 32 != 0 {
                        {
                            let result = (self.s.autoslope_callback)(spec, object.object, tile);
                            result == FAILED || !(self.s.boolean_callback)(spec, 0x15D, result)
                        }
                    } else {
                        (self.s.enabled)(spec)
                    } {
                        out.set(crate::company::price(57));
                        crate::witness::hit(crate::witness::OBJECT_AUTOSLOPE);
                        return;
                    }
                }
            }
            (self.commands().landscape_clear)(out.0, tile, flags);
        }
    }
    fn build_above(&self, out: Cost<'_>, tile: u32, flags: u16, height: i32) {
        crate::witness::hit(crate::witness::OBJECT_BUILD_ABOVE);
        unsafe {
            let spec = (self.s.spec)((self.s.object_type)(tile));
            let clearance = (self.s.clearance)(spec);
            if clearance.flags & UNDER_BRIDGE != 0
                && (self.shared.tile_max_z)(tile) + i32::from(clearance.height) <= height
            {
                out.zero();
                return;
            }
            (self.commands().landscape_clear)(out.0, tile, flags);
        }
    }
    fn change_owner(&self, tile: u32, old: u8, new: u8) {
        if self.owner(tile) != old {
            return;
        }
        unsafe {
            let object = (self.s.identity)(tile);
            let mut clear = false;
            if (object.kind == OWNED || object.kind >= 5) && new != INVALID_OWNER {
                (self.s.set_owner)(tile, new);
                if (self.s.water_class)(tile).0 == 1 {
                    ((*self.s.restoration).infrastructure)(old, u32::MAX);
                    ((*self.s.restoration).infrastructure)(new, 1);
                }
            } else if object.kind == STATUE {
                let town = (self.s.statue_town)(object.object);
                let fields = crate::town::openttd_rust_town_simulation(town.state);
                (*fields).statues &= !((1_u64 << old) as u16);
                if new != INVALID_OWNER && (*fields).statues & ((1_u64 << new) as u16) == 0 {
                    (*fields).statues |= (1_u64 << new) as u16;
                    (self.s.set_owner)(tile, new);
                } else {
                    clear = true;
                }
                ((*self.s.town).dirty_authority)(u32::from(town.id.0));
            } else {
                clear = true;
            }
            if clear {
                self.really_clear(object);
                (self.company().change_tile_owner)(tile, old, new);
            }
        }
        crate::witness::hit(crate::witness::OBJECT_OWNER);
    }
    fn colours(&self, company: u8) {
        unsafe {
            let mut from = 0;
            loop {
                let object = (self.s.next)(from);
                if object.object.is_null() {
                    break;
                }
                from = object.id.wrapping_add(1);
                if self.owner(object.tile) != company {
                    continue;
                }
                let shape = (self.s.colour_policy)(object.object);
                if shape.callbacks & 8 != 0 {
                    continue;
                }
                (self.s.set_colour)(
                    object.object,
                    (self.s.recolour)(company, shape.flags & TWO_CC != 0),
                );
            }
        }
        crate::witness::hit(crate::witness::OBJECT_COLOURS);
    }
    fn cargo(&self, tile: u32, cargo: *mut u32, accepted: *mut u64) {
        if unsafe { (self.s.object_type)(tile) } != HQ {
            return;
        }
        unsafe {
            let level = if accepted.is_null() {
                1
            } else {
                u32::from(((*self.s.industry_tiles).frame)(tile)) + 1
            };
            let pass = (self.s.passengers)();
            if pass < 64 {
                let entry = cargo.add(usize::from(pass));
                *entry = (*entry).wrapping_add(if accepted.is_null() { 1 } else { level.max(1) });
                if !accepted.is_null() {
                    *accepted |= 1_u64 << pass;
                }
            }
            let mail = (self.s.mail)();
            if mail < 64 {
                let entry = cargo.add(usize::from(mail));
                *entry = (*entry).wrapping_add(if accepted.is_null() {
                    1
                } else {
                    (level / 2).max(1)
                });
                if !accepted.is_null() {
                    *accepted |= 1_u64 << mail;
                }
            }
        }
        crate::witness::hit(crate::witness::OBJECT_CARGO);
    }
    fn tile_loop(&self, tile: u32, frame: *mut LoopFrame, call: *mut crate::ground::Call) {
        unsafe {
            let policy = (self.s.loop_policy)(tile);
            if policy.flags & ANIMATION != 0 {
                (self.s.trigger_tile)(policy.object, tile, policy.spec);
                if policy.north == tile {
                    (self.s.trigger)(policy.object, 2, policy.spec);
                }
            }
            let water = ((*self.s.industry_tiles).on_water)(tile);
            if water {
                self.water_loop(tile, call);
            }
            // Only animation/water services can mutate the type before this check.
            let kind = if water || policy.flags & ANIMATION != 0 {
                (self.s.object_type)(tile)
            } else {
                policy.kind
            };
            if kind != HQ {
                return;
            }
            let level = u32::from(((*self.s.industry_tiles).frame)(tile)) + 1;
            (self.s.hq_begin)(frame, tile, level);
            let random = self.shared.random();
            let pass = (self.s.passengers)();
            if pass < 64 && random & 255 < 256 / 4 / (6_u32.wrapping_sub(level)) {
                let mut amount = (random & 255) / 8 / 4 + 1;
                if crate::company::in_recession() {
                    amount = (amount + 1) >> 1;
                }
                amount = (self.s.scale_cargo)(amount);
                (self.s.hq_deliver)(frame, pass, amount, self.owner(tile));
                crate::witness::hit(crate::witness::OBJECT_HQ_PASS);
            }
            let mail = (self.s.mail)();
            if mail < 64 && (random >> 8) & 255 < 196 / 4 / (6_u32.wrapping_sub(level)) {
                let mut amount = ((random >> 8) & 255) / 8 / 4 + 1;
                if crate::company::in_recession() {
                    amount = (amount + 1) >> 1;
                }
                amount = (self.s.scale_cargo)(amount);
                (self.s.hq_deliver)(frame, mail, amount, self.owner(tile));
                crate::witness::hit(crate::witness::OBJECT_HQ_MAIL);
            }
            (self.s.hq_end)(frame);
        }
        crate::witness::hit(crate::witness::OBJECT_LOOP);
    }
    fn near(&self, tile: u32, kind: u16) -> bool {
        let size_x = self.map.size_x();
        let size_y = (self.shared.map_max_y)() + 1;
        let mut it = crate::spiral::square(
            self.map.tile_x(tile),
            self.map.tile_y(tile),
            9,
            size_x,
            size_y,
        );
        while !it.is_end() {
            if self.is_type(it.x + it.y * size_x, kind) {
                return true;
            }
            it = it.advance(size_x, size_y);
        }
        false
    }
    fn lighthouse(&self) -> bool {
        let maxx = (self.shared.map_max_x)();
        let maxy = (self.shared.map_max_y)();
        let random = self.shared.random();
        let mut perimeter = ((random >> 16) % (2 * (maxx + maxy))).wrapping_sub(maxy) as i32;
        let mut dir = 0_u8;
        while perimeter > 0 {
            perimeter -= if dir & 1 == 0 { maxx } else { maxy } as i32;
            dir += 1;
        }
        let (x, y) = match dir {
            1 => (random % maxx, 1),
            2 => (1, random % maxy),
            3 => (random % maxx, maxy - 1),
            _ => (maxx - 1, random % maxy),
        };
        let mut tile = x + y * self.map.size_x();
        unsafe {
            if (self.shared.tile_type)(tile) != 6 || (self.s.water_class)(tile).0 != 0 {
                return false;
            }
            for _ in 0..19 {
                if (self.shared.tile_type)(tile) == 0
                    && (self.shared.tile_slope)(tile) == 0
                    && (self.shared.tile_z)(tile) <= 2
                    && !(self.shared.bridge_above)(tile)
                {
                    if self.near(tile, LIGHTHOUSE) {
                        return false;
                    }
                    self.build(LIGHTHOUSE, tile, NONE, ptr::null(), 0);
                    crate::witness::hit(crate::witness::OBJECT_LIGHTHOUSE);
                    return true;
                }
                tile = self.map.add_diag(tile, dir);
                if !(self.shared.tile_valid)(tile) {
                    return false;
                }
            }
        }
        false
    }
    fn transmitter(&self) -> bool {
        let tile = self.shared.random() & (self.map.size_x() * ((self.shared.map_max_y)() + 1) - 1);
        if (self.shared.tile_type)(tile) == 0
            && (self.shared.tile_slope)(tile) == 0
            && (self.shared.tile_z)(tile) >= 4
            && !(self.shared.bridge_above)(tile)
        {
            if self.near(tile, TRANSMITTER) {
                return false;
            }
            self.build(TRANSMITTER, tile, NONE, ptr::null(), 0);
            crate::witness::hit(crate::witness::OBJECT_TRANSMITTER);
            return true;
        }
        false
    }
    fn generate(
        &self,
        frame: *mut GenerationFrame,
        out: Cost<'_>,
        tmp: Cost<'_>,
        aux: Cost<'_>,
    ) -> bool {
        unsafe {
            if !(self.s.progress_set)(frame, (self.s.spec_count)() as u32) {
                return false;
            }
            let maxx = (self.shared.map_max_x)();
            let maxy = (self.shared.map_max_y)();
            let mut water_tiles = 0_u32;
            if (self.s.freeform)() {
                for x in 0..maxx {
                    if (self.shared.tile_type)(x + self.map.size_x()) == 6 {
                        water_tiles += 1;
                    }
                    if (self.shared.tile_type)(x + (maxy - 1) * self.map.size_x()) == 6 {
                        water_tiles += 1;
                    }
                }
                for y in 1..maxy - 1 {
                    if (self.shared.tile_type)(1 + y * self.map.size_x()) == 6 {
                        water_tiles += 1;
                    }
                    if (self.shared.tile_type)(maxx - 1 + y * self.map.size_x()) == 6 {
                        water_tiles += 1;
                    }
                }
            }
            for kind in 0..(self.s.spec_count)() {
                let spec = (self.s.spec)(kind as u16);
                let generation = (self.s.generation)(spec);
                if !generation.ever || generation.amount == 0 {
                    continue;
                }
                let mut amount = if generation.flags & SCALE_WATER != 0 && (self.s.freeform)() {
                    (self.s.scale_size_1d)(u32::from(generation.amount) * water_tiles)
                        / (2 * maxy + 2 * maxx - 6)
                } else if generation.flags & SCALE_WATER != 0 {
                    (self.s.scale_size_1d)(u32::from(generation.amount))
                } else {
                    ((*self.s.founding).scale_size)(u32::from(generation.amount))
                } as u16;
                let mut retry = ((*self.s.founding).scale_size)(1000);
                while retry != 0 && amount != 0 && (self.s.can_allocate)() {
                    let success = match kind as u16 {
                        TRANSMITTER => self.transmitter(),
                        LIGHTHOUSE => self.lighthouse(),
                        _ => {
                            let view = self.shared.random_range(u32::from(generation.views)) as u8;
                            let tile = self.shared.random() & (self.map.size_x() * (maxy + 1) - 1);
                            self.build_command(
                                out,
                                tmp,
                                aux,
                                EXECUTE | AUTO | 16 | 512,
                                tile,
                                kind as u16,
                                view,
                            );
                            out.ok()
                        }
                    };
                    if success {
                        amount = amount.wrapping_sub(1);
                    }
                    retry -= 1;
                }
                if !(self.s.progress_increase)(frame) {
                    return false;
                }
            }
        }
        crate::witness::hit(crate::witness::OBJECT_GENERATE);
        true
    }
}

// Same traversal and clipping as TileIterator::Create, without a boundary
// allocation. The rotated a/b coordinates preserve the source's signed halves.
struct Drag {
    map: crate::services::Map,
    size_y: u32,
    tile: u32,
    diagonal: bool,
    x: u32,
    y: u32,
    width: u32,
    left: u32,
    rows: u32,
    a: i32,
    b: i32,
    max_a: i32,
    max_b: i32,
}
impl Drag {
    fn new(map: crate::services::Map, size_y: u32, tile: u32, end: u32, diagonal: bool) -> Self {
        let (x, y) = (map.tile_x(tile), map.tile_y(tile));
        let (ex, ey) = (map.tile_x(end), map.tile_y(end));
        let mut a = ey as i32 + ex as i32 - y as i32 - x as i32;
        let mut b = ey as i32 - ex as i32 - y as i32 + x as i32;
        a += if a > 0 { 1 } else { -1 };
        b += if b > 0 { 1 } else { -1 };
        let width = x.abs_diff(ex) + 1;
        Self {
            map,
            size_y,
            tile: if diagonal {
                tile
            } else {
                x.min(ex) + y.min(ey) * map.size_x()
            },
            diagonal,
            x,
            y,
            width,
            left: width,
            rows: y.abs_diff(ey) + 1,
            a: 0,
            b: 0,
            max_a: a,
            max_b: b,
        }
    }
    fn advance(&mut self) {
        if !self.diagonal {
            self.left -= 1;
            if self.left > 0 {
                self.tile = self.tile.wrapping_add(1);
            } else {
                self.rows -= 1;
                if self.rows == 0 {
                    self.tile = INVALID_TILE;
                } else {
                    self.left = self.width;
                    self.tile = self
                        .tile
                        .wrapping_add(self.map.size_x())
                        .wrapping_add(1)
                        .wrapping_sub(self.width);
                }
            }
            return;
        }
        loop {
            if self.max_a.abs() == 1 {
                self.a = 0;
                self.b = if self.max_b > 0 {
                    (self.b + 2).min(self.max_b)
                } else {
                    (self.b - 2).max(self.max_b)
                };
            } else {
                self.a += if self.max_a > 0 { 2 } else { -2 };
                if if self.max_a > 0 {
                    self.a >= self.max_a
                } else {
                    self.a <= self.max_a
                } {
                    self.a = if self.a.abs() % 2 != 0 {
                        0
                    } else if self.max_a > 0 {
                        1
                    } else {
                        -1
                    };
                    self.b += if self.max_b > 0 { 1 } else { -1 };
                }
            }
            let x = self.x.wrapping_add(((self.a - self.b) / 2) as u32);
            let y = self.y.wrapping_add(((self.b + self.a) / 2) as u32);
            self.tile = if x >= self.map.size_x() || y >= self.size_y {
                INVALID_TILE
            } else {
                x + y * self.map.size_x()
            };
            if self.tile <= self.map.size_x() * self.size_y || self.max_b == self.b {
                break;
            }
        }
        if self.max_b == self.b {
            self.tile = INVALID_TILE;
        }
    }
}

/// Synchronous object owner entry.
/// # Safety
/// Immutable tables, live native cost/frame/array storage and original tile/ID
/// preconditions; serialized with reset/save/load. No native pointer is retained.
/// Services may reenter after all state accesses end. Native cancellation is
/// returned before C++ rethrows; other leaves are noexcept. Panics abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_build(
    s: *const ObjectServices,
    kind: u16,
    tile: u32,
    owner: u8,
    town: *const TownHandle,
    view: u8,
) {
    // SAFETY: native tables and stack storage outlive the synchronous call.
    let w = unsafe { World::new::<true>(&*s) };
    w.build(kind, tile, owner, town, view);
}

/// Synchronous object owner entry.
/// # Safety
/// Immutable tables, live native cost/frame/array storage and original tile/ID
/// preconditions; serialized with reset/save/load. No native pointer is retained.
/// Services may reenter after all state accesses end. Native cancellation is
/// returned before C++ rethrows; other leaves are noexcept. Panics abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_update_hq(
    s: *const ObjectServices,
    tile: u32,
    score: u32,
) {
    // SAFETY: native tables and stack storage outlive the synchronous call.
    let w = unsafe { World::new::<true>(&*s) };
    w.update_hq(tile, score);
}

/// Synchronous object owner entry.
/// # Safety
/// Immutable tables, live native cost/frame/array storage and original tile/ID
/// preconditions; serialized with reset/save/load. No native pointer is retained.
/// Services may reenter after all state accesses end. Native cancellation is
/// returned before C++ rethrows; other leaves are noexcept. Panics abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_colours(s: *const ObjectServices, company: u8) {
    // SAFETY: native tables and stack storage outlive the synchronous call.
    let w = unsafe { World::new::<false>(&*s) };
    w.colours(company);
}

/// Synchronous object owner entry.
/// # Safety
/// Immutable tables, live native cost/frame/array storage and original tile/ID
/// preconditions; serialized with reset/save/load. No native pointer is retained.
/// Services may reenter after all state accesses end. Native cancellation is
/// returned before C++ rethrows; other leaves are noexcept. Panics abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_build_command(
    s: *const ObjectServices,
    out: *mut CommandCost,
    tmp: *mut CommandCost,
    aux: *mut CommandCost,
    flags: u16,
    tile: u32,
    kind: u16,
    view: u8,
) {
    // SAFETY: native tables and stack storage outlive the synchronous call.
    let w = unsafe { World::new::<true>(&*s) };
    let costs = unsafe { &*(*s).cost };
    let out = Cost(out, costs);
    let tmp = Cost(tmp, costs);
    let aux = Cost(aux, costs);
    w.build_command(out, tmp, aux, flags, tile, kind, view);
}

/// Synchronous object owner entry.
/// # Safety
/// Immutable tables, live native cost/frame/array storage and original tile/ID
/// preconditions; serialized with reset/save/load. No native pointer is retained.
/// Services may reenter after all state accesses end. Native cancellation is
/// returned before C++ rethrows; other leaves are noexcept. Panics abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_build_area(
    s: *const ObjectServices,
    out: *mut CommandCost,
    ret: *mut CommandCost,
    tmp: *mut CommandCost,
    aux: *mut CommandCost,
    test: *mut CommandCost,
    last: *mut CommandCost,
    flags: u16,
    tile: u32,
    start: u32,
    kind: u16,
    view: u8,
    diagonal: bool,
) {
    // SAFETY: native tables and stack storage outlive the synchronous call.
    let w = unsafe { World::new::<true>(&*s) };
    let costs = unsafe { &*(*s).cost };
    let out = Cost(out, costs);
    let ret = Cost(ret, costs);
    let tmp = Cost(tmp, costs);
    let aux = Cost(aux, costs);
    let test = Cost(test, costs);
    let last = Cost(last, costs);
    w.build_area(
        out, ret, tmp, aux, test, last, flags, tile, start, kind, view, diagonal,
    );
}

/// Synchronous object owner entry.
/// # Safety
/// Immutable tables, live native cost/frame/array storage and original tile/ID
/// preconditions; serialized with reset/save/load. No native pointer is retained.
/// Services may reenter after all state accesses end. Native cancellation is
/// returned before C++ rethrows; other leaves are noexcept. Panics abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_clear(
    s: *const ObjectServices,
    out: *mut CommandCost,
    tmp: *mut CommandCost,
    tile: u32,
    flags: u16,
) {
    // SAFETY: native tables and stack storage outlive the synchronous call.
    let w = unsafe { World::new::<true>(&*s) };
    let costs = unsafe { &*(*s).cost };
    let out = Cost(out, costs);
    let tmp = Cost(tmp, costs);
    w.clear(out, tmp, tile, flags);
}

/// Synchronous object owner entry.
/// # Safety
/// Immutable tables, live native cost/frame/array storage and original tile/ID
/// preconditions; serialized with reset/save/load. No native pointer is retained.
/// Services may reenter after all state accesses end. Native cancellation is
/// returned before C++ rethrows; other leaves are noexcept. Panics abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_cargo(
    s: *const ObjectServices,
    tile: u32,
    cargo: *mut u32,
    accepted: *mut u64,
) {
    // SAFETY: native tables and stack storage outlive the synchronous call.
    let w = unsafe { World::new::<false>(&*s) };
    w.cargo(tile, cargo, accepted);
}

/// Synchronous object owner entry.
/// # Safety
/// Immutable tables, live native cost/frame/array storage and original tile/ID
/// preconditions; serialized with reset/save/load. No native pointer is retained.
/// Services may reenter after all state accesses end. Native cancellation is
/// returned before C++ rethrows; other leaves are noexcept. Panics abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_tile_loop(
    s: *const ObjectServices,
    frame: *mut LoopFrame,
    call: *mut crate::ground::Call,
    tile: u32,
) {
    // SAFETY: native tables and stack storage outlive the synchronous call.
    let w = unsafe { World::new::<false>(&*s) };
    w.tile_loop(tile, frame, call);
}

/// Synchronous object owner entry.
/// # Safety
/// Immutable tables, live native cost/frame/array storage and original tile/ID
/// preconditions; serialized with reset/save/load. No native pointer is retained.
/// Services may reenter after all state accesses end. Native cancellation is
/// returned before C++ rethrows; other leaves are noexcept. Panics abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_generate(
    s: *const ObjectServices,
    out: *mut CommandCost,
    tmp: *mut CommandCost,
    aux: *mut CommandCost,
    frame: *mut GenerationFrame,
) -> bool {
    // SAFETY: native tables and stack storage outlive the synchronous call.
    let w = unsafe { World::new::<true>(&*s) };
    let costs = unsafe { &*(*s).cost };
    let out = Cost(out, costs);
    let tmp = Cost(tmp, costs);
    let aux = Cost(aux, costs);
    w.generate(frame, out, tmp, aux)
}

/// Synchronous object owner entry.
/// # Safety
/// Immutable tables, live native cost/frame/array storage and original tile/ID
/// preconditions; serialized with reset/save/load. No native pointer is retained.
/// Services may reenter after all state accesses end. Native cancellation is
/// returned before C++ rethrows; other leaves are noexcept. Panics abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_change_owner(
    s: *const ObjectServices,
    tile: u32,
    old: u8,
    new_owner: u8,
) {
    // SAFETY: native tables and stack storage outlive the synchronous call.
    let w = unsafe { World::new::<true>(&*s) };
    w.change_owner(tile, old, new_owner);
}

/// Synchronous object owner entry.
/// # Safety
/// Immutable tables, live native cost/frame/array storage and original tile/ID
/// preconditions; serialized with reset/save/load. No native pointer is retained.
/// Services may reenter after all state accesses end. Native cancellation is
/// returned before C++ rethrows; other leaves are noexcept. Panics abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_terraform(
    s: *const ObjectServices,
    out: *mut CommandCost,
    tile: u32,
    flags: u16,
    z: i32,
    slope: u8,
) {
    // SAFETY: native tables and stack storage outlive the synchronous call.
    let w = unsafe { World::new::<false>(&*s) };
    let costs = unsafe { &*(*s).cost };
    let out = Cost(out, costs);
    w.terraform(out, tile, flags, z, slope);
}

/// Synchronous object owner entry.
/// # Safety
/// Immutable tables, live native cost/frame/array storage and original tile/ID
/// preconditions; serialized with reset/save/load. No native pointer is retained.
/// Services may reenter after all state accesses end. Native cancellation is
/// returned before C++ rethrows; other leaves are noexcept. Panics abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_build_above(
    s: *const ObjectServices,
    out: *mut CommandCost,
    tile: u32,
    flags: u16,
    height: i32,
) {
    // SAFETY: native tables and stack storage outlive the synchronous call.
    let w = unsafe { World::new::<false>(&*s) };
    let costs = unsafe { &*(*s).cost };
    let out = Cost(out, costs);
    w.build_above(out, tile, flags, height);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_counts_and_nested_industry_tunnel_scratch_follow_source_scope() {
        openttd_rust_object_counts_reset();
        openttd_rust_object_count_decrement(63999);
        assert_eq!(openttd_rust_object_count(63999), u16::MAX);
        openttd_rust_object_count_increment(63999);
        assert_eq!(openttd_rust_object_count(63999), 0);
        openttd_rust_object_count_increment(3);
        std::thread::spawn(|| {
            assert_eq!(openttd_rust_object_count(3), 1);
            openttd_rust_object_count_increment(3);
        })
        .join()
        .unwrap();
        assert_eq!(openttd_rust_object_count(3), 2);

        clear_areas();
        let entry = ClearArea {
            first_tile: 10,
            tile: 10,
            width: 2,
            height: 2,
        };
        let index = openttd_rust_object_areas_append(entry);
        let industry = snapshot_areas();
        // Tunnel hides its first tile while nested terraform appends/reallocates.
        openttd_rust_object_areas_set_first(index, u32::MAX);
        for _ in 0..4096 {
            openttd_rust_object_areas_append(entry);
        }
        openttd_rust_object_areas_set_first(index, 10);
        assert_eq!(openttd_rust_object_areas_find(11, 6), index);
        // Industry's original copy/restore point restores the whole ordered list.
        restore_areas(industry);
        assert_eq!(snapshot_areas().len(), 1);
        assert_eq!(openttd_rust_object_areas_get(index).first_tile, 10);
        assert_eq!(openttd_rust_object_areas_find(75, 6), index);
        assert_eq!(openttd_rust_object_areas_find(76, 6), usize::MAX);
        clear_areas();
        openttd_rust_object_counts_reset();
    }
}
