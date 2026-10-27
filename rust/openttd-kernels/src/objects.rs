/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Object lifecycle owner; native pools, specs, map and save records remain canonical.
//! Export contract: tables and native cost, frame and array storage are live for the
//! synchronous call, arguments meet the original's tile and ID preconditions, and calls are
//! serialized with reset, save and load. No native pointer is retained, services reenter only
//! after state accesses end, native cancellation returns before C++ rethrows; panics abort.
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
use crate::command_type::DoCommandFlag;
use crate::company_type::{
    INVALID_OWNER, MAX_COMPANIES, OWNER_DEITY, OWNER_NONE, OWNER_TOWN, OWNER_WATER,
};
use crate::economy_func::get_price;
use crate::economy_type::PR_BUILD_FOUNDATION;
use crate::game_state::GameState;
use crate::newgrf_callbacks::CALLBACK_FAILED;
use crate::slope_func::{get_slope_max_z, is_steep_slope};
use crate::strings_type::INVALID_STRING_ID;
use crate::tile_type::{INVALID_TILE, MP_CLEAR, MP_OBJECT, MP_WATER};
use crate::tilearea::TileArea;

struct State {
    counts: [u16; 64000],
    areas: Vec<ClearArea>,
}
static STATE: GameState<State> = GameState::new(State {
    counts: [0; 64000],
    areas: Vec::new(),
});

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
    unsafe { (*STATE.get()).areas.clone() }
}
/// Industry transaction restore at the original move point.
pub(crate) fn restore_areas(areas: Vec<ClearArea>) {
    // SAFETY: caller holds the game-state mutex; no callback in this scope.
    unsafe { (*STATE.get()).areas = areas };
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_counts_reset() {
    // SAFETY: serialized native entry.
    unsafe { (*STATE.get()).counts.fill(0) };
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_count_increment(kind: u16) {
    // SAFETY: serialized entry; native source asserts kind < NUM_OBJECTS.
    let count = unsafe { &mut (*STATE.get()).counts[usize::from(kind)] };
    *count = count.wrapping_add(1);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_count_decrement(kind: u16) {
    // SAFETY: serialized entry; native source asserts kind < NUM_OBJECTS.
    let count = unsafe { &mut (*STATE.get()).counts[usize::from(kind)] };
    *count = count.wrapping_sub(1);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_count(kind: u16) -> u16 {
    // SAFETY: serialized native entry; original kind precondition.
    unsafe { (*STATE.get()).counts[usize::from(kind)] }
}
pub(crate) fn clear_areas() {
    // SAFETY: serialized native entry.
    unsafe { (*STATE.get()).areas.clear() };
}
fn openttd_rust_object_areas_find(tile: u32, log_x: u32) -> usize {
    let map = crate::services::Map::from_log_x(log_x);
    let tile_area = TileArea { tile, w: 1, h: 1 };
    // SAFETY: serialized entry, no callbacks while reading.
    unsafe { &(*STATE.get()).areas }
        .iter()
        .position(|a| {
            TileArea {
                tile: a.tile,
                w: a.width,
                h: a.height,
            }
            .intersects(tile_area, map)
        })
        .unwrap_or(usize::MAX)
}
fn openttd_rust_object_areas_get(index: usize) -> ClearArea {
    // SAFETY: serialized entry; index was returned by find/append.
    unsafe { (&(*STATE.get()).areas)[index] }
}
fn openttd_rust_object_areas_append(area: ClearArea) -> usize {
    // SAFETY: serialized entry; allocation failure aborts.
    let areas = unsafe { &mut (*STATE.get()).areas };
    areas.push(area);
    areas.len() - 1
}
fn openttd_rust_object_areas_set_first(index: usize, first: u32) {
    // SAFETY: serialized entry; append preserves indices returned by find/append.
    unsafe { (&mut (*STATE.get()).areas)[index].first_tile = first };
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
    CommandCost, CommandCostServices, CommandServices, Cost, Services, TownHandle, WaterClass,
};
use std::ptr;

#[repr(C)]
pub struct Price {
    pub factor: u32,
    /// The spec GRF's `price_base_multipliers[price]`, zero without a GRF file.
    pub shift: i32,
}

/// Native `ObjectAnimationTrigger : uint8_t` carrier.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct AnimationTrigger(pub u8);
impl AnimationTrigger {
    const BUILT: Self = Self(0);
    const TILE_LOOP: Self = Self(1);
    const TILE_LOOP_NORTH: Self = Self(2);
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
pub struct GenerationFrame {
    _private: [u8; 0],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Identity {
    pub object: *mut Object,
    pub area: TileArea,
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
    pub clearance: unsafe extern "C" fn(u32) -> Clearance,
    pub area_shape: unsafe extern "C" fn(*const Spec) -> AreaShape,
    pub admission: unsafe extern "C" fn(*const Spec) -> Admission,
    pub generation: unsafe extern "C" fn(*const Spec) -> Generation,
    pub can_allocate: unsafe extern "C" fn() -> bool,
    pub allocate: unsafe extern "C" fn(u16, *const TownHandle, TileArea, u8) -> Allocation,
    pub object_type: unsafe extern "C" fn(u32) -> u16,
    pub loop_policy: unsafe extern "C" fn(u32) -> LoopPolicy,
    pub identity: unsafe extern "C" fn(u32) -> Identity,
    pub location: unsafe extern "C" fn(*mut Object) -> TileArea,
    pub next: unsafe extern "C" fn(u32) -> Next,
    pub set_colour: unsafe extern "C" fn(*mut Object, u8) -> (),
    pub recolour: unsafe extern "C" fn(u8, bool) -> u8,
    pub colour_error: unsafe extern "C" fn(*const Spec, u16) -> (),
    pub make: unsafe extern "C" fn(u32, u8, u32, WaterClass, u32) -> (),
    pub delete_object: unsafe extern "C" fn(*mut Object) -> (),
    pub query: *const QueryServices,
    pub resolver: *const ResolverServices,
    pub slope_error: unsafe extern "C" fn(*mut CommandCost, *const SlopeResult, *const Spec) -> (),
    pub build_price: unsafe extern "C" fn(*const Spec) -> Price,
    pub clear_price: unsafe extern "C" fn(*const Spec) -> Price,
    pub set_hq: unsafe extern "C" fn(u8, u32) -> (),
    pub statue_town: unsafe extern "C" fn(*mut Object) -> crate::town::TownRef,
    pub boolean_callback: unsafe extern "C" fn(*const Spec, u16, u16) -> bool,
    pub close_inspect: unsafe extern "C" fn(u32) -> (),
    pub progress_set: unsafe extern "C" fn(*mut GenerationFrame, u32) -> bool,
    pub progress_increase: unsafe extern "C" fn(*mut GenerationFrame) -> bool,
    pub normal: unsafe extern "C" fn() -> bool,
    pub bulldozer: unsafe extern "C" fn() -> bool,
    pub water_class: unsafe extern "C" fn(u32) -> WaterClass,
    pub restoration: *const crate::water::RestorationServices,
    pub set_owner: unsafe extern "C" fn(u32, u8) -> (),
    pub scale_size_1d: unsafe extern "C" fn(u32) -> u32,
    pub scale_cargo: unsafe extern "C" fn(u32) -> u32,
}

const HQ: u16 = 4;
const OWNED: u16 = 3;
use crate::object_type::OBJECT_STATUE as STATUE;
const LIGHTHOUSE: u16 = 1;
const TRANSMITTER: u16 = 0;
const NEW_OBJECT_OFFSET: u16 = 5;
const OBJECT_SIZE_1X1: u8 = 0x11;
const INVALID_WATER_CLASS: WaterClass = WaterClass(3);
const SOURCE_HEADQUARTERS: u8 = 2;
const CBID_OBJECT_AUTOSLOPE: u16 = 0x15D;
const PR_BUILD_OBJECT: u32 = 50;
const PR_CLEAR_OBJECT: u32 = 51;
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

#[repr(C)]
pub struct Resolver {
    _private: [u8; 0],
}
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct HouseZone(u8);
#[repr(C)]
pub struct Definition {
    pub grfid: u32,
    pub local_id: u16,
    pub view: u8,
    pub has_file: bool,
}
#[repr(C)]
pub struct QueryServices {
    pub random_bits: unsafe extern "C" fn(u32) -> u8,
    pub index: unsafe extern "C" fn(u32) -> u32,
    pub by_tile: unsafe extern "C" fn(u32) -> *mut Object,
    pub definition: unsafe extern "C" fn(*mut Object) -> Definition,
    pub object_index: unsafe extern "C" fn(*mut Object) -> u32,
    pub kind: unsafe extern "C" fn(*mut Object) -> u16,
    pub north: unsafe extern "C" fn(*mut Object) -> u32,
    pub town: unsafe extern "C" fn(*mut Object) -> *const TownHandle,
    pub town_tile: unsafe extern "C" fn(*const TownHandle) -> u32,
    pub date: unsafe extern "C" fn(*mut Object) -> i32,
    pub colour: unsafe extern "C" fn(*mut Object) -> u8,
    pub view: unsafe extern "C" fn(*mut Object) -> u8,
    pub nearby: unsafe extern "C" fn(u8, u32) -> u32,
    pub nearby_info: unsafe extern "C" fn(u32, bool) -> u32,
    pub terrain: unsafe extern "C" fn(u32) -> u32,
    pub town_zone: unsafe extern "C" fn(*const TownHandle, u32) -> HouseZone,
    pub badges: unsafe extern "C" fn(*const Resolver, *const Spec, u32) -> u32,
    pub grfid: unsafe extern "C" fn(*const Resolver) -> u32,
    pub version8: unsafe extern "C" fn(*const Resolver) -> bool,
    pub override_id: unsafe extern "C" fn(u16, u32) -> u16,
    pub manhattan: unsafe extern "C" fn(u32, u32) -> u32,
    pub square: unsafe extern "C" fn(u32, u32) -> u32,
    pub triggers: unsafe extern "C" fn(*const Spec) -> u16,

    pub unhandled: unsafe extern "C" fn(u8),
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
            map: if NEEDS_MAP {
                shared.map()
            } else {
                crate::services::Map::from_log_x(0)
            },
        }
    }
    unsafe fn callback(
        &self,
        callback: u16,
        param1: u32,
        param2: u32,
        spec: *const Spec,
        object: *mut Object,
        tile: u32,
        registers: *mut i32,
        count: usize,
        view: u8,
    ) -> u16 {
        unsafe {
            openttd_rust_object_callback(
                self.s, callback, param1, param2, spec, object, tile, registers, count, view,
            )
        }
    }
    fn query(&self) -> &QueryServices {
        // SAFETY: immutable native services live for the synchronous entry.
        unsafe { &*self.s.query }
    }
    fn random_bits(&self, tile: u32) -> u32 {
        if (self.shared.tile_valid)(tile) && (self.shared.tile_type)(tile) == MP_OBJECT {
            unsafe { u32::from((self.query().random_bits)(tile)) }
        } else {
            0
        }
    }
    fn id_at_offset(&self, tile: u32, grfid: u32) -> u32 {
        if (self.shared.tile_type)(tile) != MP_OBJECT {
            return 0xffff;
        }
        // SAFETY: object tile owns a live native object and specification.
        let d = unsafe { (self.query().definition)((self.query().by_tile)(tile)) };
        if d.has_file && d.grfid == grfid {
            u32::from(d.local_id) | u32::from(d.view) << 16
        } else {
            0xfffe
        }
    }
    fn nearby_info(&self, parameter: u8, mut tile: u32, index: u32, version8: bool) -> u32 {
        let q = self.query();
        // SAFETY: native nearby helper wraps to the map, as in the original.
        unsafe {
            if parameter != 0 {
                tile = (q.nearby)(parameter, tile);
            }
            let same = (self.shared.tile_type)(tile) == MP_OBJECT && (q.index)(tile) == index;
            (q.nearby_info)(tile, version8) | u32::from(same) << 8
        }
    }
    fn closest(&self, tile: u32, kind: u16, current: *mut Object) -> u32 {
        let mut best = u32::MAX;
        let mut from = 0;
        // SAFETY: pool iteration returns live pointers; no pointer is dereferenced in Rust.
        unsafe {
            loop {
                let o = (self.s.next)(from);
                if o.object.is_null() {
                    break;
                }
                from = o.id.wrapping_add(1);
                if (self.query().kind)(o.object) != kind || o.object == current {
                    continue;
                }
                best = best.min((self.query().manhattan)(tile, o.tile));
            }
        }
        best
    }
    fn count_distance(&self, local_id: u8, grfid: u32, tile: u32, current: *mut Object) -> u32 {
        let q = self.query();
        // SAFETY: resolver and override manager are native and remain live.
        let kind = unsafe {
            match crate::newgrf_storage::openttd_rust_storage_temp_get(0x100) as u32 {
                0 => u16::from(local_id),
                u32::MAX => (q.override_id)(u16::from(local_id), grfid),
                other => (q.override_id)(u16::from(local_id), other),
            }
        };
        if kind >= 64000 {
            return 0xffff;
        }
        u32::from(openttd_rust_object_count(kind)) << 16
            | self.closest(tile, kind, current).min(u32::from(u16::MAX))
    }
    fn variable(
        &self,
        ro: *const Resolver,
        obj: *mut Object,
        spec: *const Spec,
        tile: u32,
        view: u8,
        variable: u8,
        parameter: u32,
        available: &mut bool,
    ) -> u32 {
        let q = self.query();
        let mut town = std::ptr::null();
        // SAFETY: native entry supplies its live resolver/spec/object. Null-object
        // queries take the original availability path before any object access.
        unsafe {
            if obj.is_null() {
                match variable {
                    0x41 | 0x60 | 0x61 | 0x62 | 0x64 => {}
                    0x45 | 0x46 if (self.shared.tile_valid)(tile) => {
                        town = (self.shared.closest_town)(tile, u32::MAX);
                    }
                    0x42 => return (self.shared.calendar_date)() as u32,
                    0x44 => return u32::from((self.shared.current_company)()),
                    0x48 => return u32::from(view),
                    0x7a => return (q.badges)(ro, spec, parameter),
                    _ => return self.unhandled(variable, available),
                }
                if !(self.shared.tile_valid)(tile) {
                    return self.unhandled(variable, available);
                }
            } else if matches!(variable, 0x45 | 0x46) {
                town = (q.town)(obj);
            }
            match variable {
                0x40 => {
                    let offset = tile.wrapping_sub((q.north)(obj));
                    let x = self.map.tile_x(offset);
                    let y = self.map.tile_y(offset);
                    y << 20 | x << 16 | y << 8 | x
                }
                0x41 => u32::from((self.shared.tile_slope)(tile)) << 8 | (q.terrain)(tile),
                0x42 => (q.date)(obj) as u32,
                0x43 => u32::from(((*self.s.industry_tiles).frame)(tile)),
                0x44 => u32::from((self.shared.tile_owner)(tile)),
                0x45 => {
                    u32::from((q.town_zone)(town, tile).0) << 16
                        | (q.manhattan)(tile, (q.town_tile)(town)).min(u32::from(u16::MAX))
                }
                0x46 => (q.square)(tile, (q.town_tile)(town)),
                0x47 => u32::from((q.colour)(obj)),
                0x48 => u32::from((q.view)(obj)),
                0x60 => self.id_at_offset((q.nearby)(parameter as u8, tile), (q.grfid)(ro)),
                0x61 | 0x63 => {
                    let near = (q.nearby)(parameter as u8, tile);
                    if (self.shared.tile_type)(near) == MP_OBJECT && (q.by_tile)(near) == obj {
                        if variable == 0x61 {
                            u32::from((q.random_bits)(near))
                        } else {
                            u32::from(((*self.s.industry_tiles).frame)(near))
                        }
                    } else {
                        0
                    }
                }
                0x62 => self.nearby_info(
                    parameter as u8,
                    tile,
                    if obj.is_null() {
                        u32::MAX
                    } else {
                        (q.object_index)(obj)
                    },
                    (q.version8)(ro),
                ),
                0x64 => self.count_distance(parameter as u8, (q.grfid)(ro), tile, obj),
                0x7a => (q.badges)(ro, spec, parameter),
                _ => self.unhandled(variable, available),
            }
        }
    }
    fn unhandled(&self, variable: u8, available: &mut bool) -> u32 {
        unsafe { (self.query().unhandled)(variable) };
        *available = false;
        u32::MAX
    }
    fn do_trigger(
        &self,
        obj: *mut Object,
        tile: u32,
        trigger: AnimationTrigger,
        spec: *const Spec,
        random: u32,
    ) -> bool {
        // SAFETY: only immutable service tables are borrowed across the callback.
        // Native object/spec pointers are opaque; no owner-state borrow survives it.
        unsafe {
            if (self.query().triggers)(spec) & (1_u16 << trigger.0) == 0 {
                return false;
            }
            crate::animated_tile::object_change(spec, obj, tile, random, u32::from(trigger.0));
        }
        true
    }
    fn trigger_tile(
        &self,
        obj: *mut Object,
        tile: u32,
        trigger: AnimationTrigger,
        spec: *const Spec,
    ) -> bool {
        let random = self.shared.random();
        self.do_trigger(obj, tile, trigger, spec, random)
    }
    fn trigger(&self, obj: *mut Object, trigger: AnimationTrigger, spec: *const Spec) -> bool {
        // SAFETY: native spec and object are live; original area is captured before
        // callbacks and no borrow into either record crosses a callback.
        unsafe {
            if (self.query().triggers)(spec) & (1_u16 << trigger.0) == 0 {
                return false;
            }
            let mut ret = true;
            let mut random = self.shared.random();
            let area = (self.s.location)(obj);
            // The tile-loop entry has no captured map. Acquire the actual stride
            // only when the original whole-object iterator is constructed.
            let map = self.shared.map();
            for tile in area.tiles(map) {
                if self.do_trigger(obj, tile, trigger, spec, random) {
                    // SB also ORs high bits from the unmasked draw.
                    crate::bitmath_func::sb(&mut random, 0, 16, self.shared.random());
                } else {
                    ret = false;
                }
            }
            ret
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
    fn tiles(&self, a: TileArea) -> impl Iterator<Item = u32> {
        a.tiles(self.map)
    }
    fn footprint(tile: u32, shape: Footprint, view: u8) -> TileArea {
        let (x, y) = if view & 1 == 0 {
            (shape.size & 15, shape.size >> 4)
        } else {
            (shape.size >> 4, shape.size & 15)
        };
        TileArea {
            tile,
            w: u16::from(x),
            h: u16::from(y),
        }
    }
    fn is_type(&self, tile: u32, kind: u16) -> bool {
        (self.shared.tile_type)(tile) == MP_OBJECT && self.object_type_is(tile, kind)
    }
    /// Original `IsObjectType`; callers uphold its object-tile precondition.
    fn object_type_is(&self, tile: u32, kind: u16) -> bool {
        unsafe { (self.s.object_type)(tile) == kind }
    }
    fn owner(&self, tile: u32) -> u8 {
        (self.shared.tile_owner)(tile)
    }
    /// Original `ObjectSpec::GetBuildCost`; native storage supplies its factors.
    unsafe fn get_build_cost(&self, spec: *const Spec) -> i64 {
        let price = unsafe { (self.s.build_price)(spec) };
        get_price(PR_BUILD_OBJECT, price.factor, price.shift, 0)
    }
    /// Original `ObjectSpec::GetClearCost`; native storage supplies its factors.
    unsafe fn get_clear_cost(&self, spec: *const Spec) -> i64 {
        let price = unsafe { (self.s.clear_price)(spec) };
        get_price(PR_CLEAR_OBJECT, price.factor, price.shift, 0)
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
                (self.shared.calc_closest_town)(tile, u32::MAX)
            } else {
                town
            };
            let object = (self.s.allocate)(kind, town, area, view);
            let mut colour = if owner == OWNER_NONE {
                self.shared.random() as u8
            } else {
                (self.s.recolour)(owner, true)
            };
            if shape.flags & TWO_CC == 0 {
                colour &= 15;
            }
            (self.s.set_colour)(object.object, colour);
            if shape.callbacks & 8 != 0 {
                let result = self.callback(
                    crate::newgrf_callbacks::CBID_OBJECT_COLOUR.0,
                    u32::from(colour),
                    0,
                    spec,
                    object.object,
                    tile,
                    ptr::null_mut(),
                    0,
                    0,
                );
                if result != CALLBACK_FAILED {
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
                    INVALID_WATER_CLASS
                };
                if wc.0 == 1
                    && owner != OWNER_NONE
                    && (crate::tile_map::is_tile_owner(self.shared, t, OWNER_NONE)
                        || crate::tile_map::is_tile_owner(self.shared, t, OWNER_WATER))
                {
                    ((*self.s.restoration).infrastructure)(owner, 1);
                    ((*self.s.restoration).infrastructure_dirty)(owner);
                }
                let remove = (self.shared.clear_square_services.docking)(t);
                (self.s.make)(t, owner, object.id, wc, self.shared.random());
                if remove {
                    crate::station_cmd::openttd_rust_station_remove_docking_tile(
                        (self.shared.station_commands)(),
                        t,
                    );
                }
                (self.shared.mark_dirty)(t);
            }
            openttd_rust_object_count_increment(kind);
            if shape.flags & ANIMATION != 0 {
                self.trigger(object.object, AnimationTrigger::BUILT, spec);
            }
        }
        crate::witness::hit(crate::witness::OBJECT_BUILD);
    }
    /// Original `GetCompanyHQSize`: the size is stored in the animation frame.
    fn get_company_hq_size(&self, tile: u32) -> u8 {
        unsafe { ((*self.s.industry_tiles).frame)(tile) }
    }
    /// Original `IncreaseCompanyHQSize`, including each frame write and redraw.
    fn increase_company_hq_size(&self, tile: u32) {
        unsafe {
            let area = (self.s.location)((self.query().by_tile)(tile));
            for t in self.tiles(area) {
                ((*self.s.industry_tiles).set_frame)(
                    t,
                    ((*self.s.industry_tiles).frame)(t).wrapping_add(1),
                );
                (self.shared.mark_dirty)(t);
            }
        }
        crate::witness::hit(crate::witness::OBJECT_HQ_GROW);
    }
    fn update_hq(&self, tile: u32, score: u32) {
        if tile == INVALID_TILE {
            return;
        }
        let target = [170, 350, 520, 720]
            .into_iter()
            .filter(|&n| score >= n)
            .count() as u8;
        while self.get_company_hq_size(tile) < target {
            self.increase_company_hq_size(tile);
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
            let price = self
                .get_clear_cost(spec)
                .saturating_mul(i64::from(object.area.w))
                .saturating_mul(i64::from(object.area.h))
                / 5;
            out.set(if flags_spec & CLEAR_INCOME == 0 {
                price
            } else {
                crate::company::neg(price)
            });
            let company = self.current();
            if company == OWNER_TOWN {
                out.error(INVALID_STRING_ID);
                return;
            }
            if company != OWNER_WATER {
                if flags & DoCommandFlag::NO_WATER != 0 && ((*self.s.industry_tiles).on_water)(tile)
                {
                    out.error(self.s.strings.water);
                    return;
                }
                if flags_spec & AUTOREMOVE == 0 && flags & DoCommandFlag::AUTO != 0 {
                    out.error(if object.kind == HQ {
                        self.s.strings.hq
                    } else {
                        self.s.strings.object
                    });
                    return;
                }
                if ((*self.s.town).environment.editor)() {
                } else if self.owner(tile) == OWNER_NONE {
                    if !(self.s.bulldozer)() && flags_spec & CANNOT_REMOVE != 0 {
                        out.error(INVALID_STRING_ID);
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
                                out.error(self.s.strings.hq);
                            } else {
                                out.error(INVALID_STRING_ID);
                            }
                            return;
                        }
                        out.set(out.money().saturating_mul(25));
                    }
                }
            } else if flags_spec & (BUILT_ON_WATER | NOT_ON_LAND) != 0 {
                out.error(INVALID_STRING_ID);
                return;
            }
            if object.kind == HQ {
                let owner = self.owner(tile);
                if flags & DoCommandFlag::EXECUTE != 0 {
                    (self.s.set_hq)(owner, INVALID_TILE);
                    crate::cargo_storage::openttd_rust_cargo_invalidate(
                        self.s.cargo,
                        u16::from(owner),
                        SOURCE_HEADQUARTERS,
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
            } else if object.kind == STATUE && flags & DoCommandFlag::EXECUTE != 0 {
                let town = (self.s.statue_town)(object.object);
                let fields = crate::town::openttd_rust_town_simulation(town.state);
                (*fields).statues &= !((1_u64 << self.owner(tile)) as u16);
                ((*self.s.town).dirty_authority)(u32::from(town.id.0));
            }
            openttd_rust_object_areas_append(ClearArea {
                first_tile: tile,
                tile: object.area.tile,
                width: object.area.w,
                height: object.area.h,
            });
            if flags & DoCommandFlag::EXECUTE != 0 {
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
                out.error(INVALID_STRING_ID);
                return;
            }
            let spec = (self.s.spec)(kind);
            let admission = (self.s.admission)(spec);
            if (self.s.normal)() && !available(self.s, spec) && !((*self.s.town).generating_world)()
            {
                out.error(INVALID_STRING_ID);
                return;
            }
            if (((*self.s.town).environment.editor)() || ((*self.s.town).generating_world)())
                && !was_available(self.s, spec)
            {
                out.error(INVALID_STRING_ID);
                return;
            }
            let shape = (self.s.footprint)(spec);
            if shape.flags & SCENEDIT != 0
                && ((!((*self.s.town).generating_world)()
                    && !((*self.s.town).environment.editor)())
                    || self.current() != OWNER_NONE)
            {
                out.error(INVALID_STRING_ID);
                return;
            }
            if shape.flags & IN_GAME != 0
                && (((*self.s.town).generating_world)()
                    || !(self.s.normal)()
                    || self.current() > MAX_COMPANIES)
            {
                out.error(INVALID_STRING_ID);
                return;
            }
            if view >= admission.views {
                out.error(INVALID_STRING_ID);
                return;
            }
            if !(self.s.can_allocate)() {
                out.error(self.s.strings.too_many);
                return;
            }
            if (self.shared.town_count)() == 0 {
                out.error(self.s.strings.no_town);
                return;
            }
            let area = Self::footprint(tile, shape, view);
            for t in self.tiles(area) {
                if !(self.shared.tile_valid)(t) {
                    out.error(self.s.strings.edge_sub);
                    return;
                }
            }
            if kind == OWNED {
                crate::landscape_cmd::clear_command(self.commands(), tmp.0, tile, flags);
                out.add(tmp);
            } else {
                let water = shape.flags & (BUILT_ON_WATER | NOT_ON_LAND) != 0;
                let ground = shape.flags & NOT_ON_LAND == 0;
                for t in self.tiles(area) {
                    if (self.shared.has_water_ground)(t) {
                        if !water {
                            out.error(self.s.strings.water);
                            return;
                        }
                        if !(self.shared.is_water_tile)(t) {
                            crate::landscape_cmd::clear_command(
                                self.commands(),
                                tmp.0,
                                t,
                                flags & !(DoCommandFlag::NO_WATER | DoCommandFlag::EXECUTE),
                            );
                            out.add(tmp);
                        } else {
                            let owner = self.owner(t);
                            if owner != OWNER_NONE && owner != OWNER_WATER {
                                (self.commands().ownership)(tmp.0, owner, t);
                                out.add(tmp);
                            }
                            if !((*self.s.town).environment.freeform_edges)()
                                && (!crate::math::is_inside_mm(
                                    self.map.tile_x(t) as usize,
                                    1,
                                    ((self.shared.map_max_x)() - 1) as usize,
                                ) || !crate::math::is_inside_mm(
                                    self.map.tile_y(t) as usize,
                                    1,
                                    ((self.shared.map_max_y)() - 1) as usize,
                                ))
                            {
                                out.error(self.s.strings.edge);
                                return;
                            }
                            self.commands().no_vehicle(tmp.0, t);
                            out.add(tmp);
                        }
                    } else {
                        if !ground {
                            out.error(self.s.strings.on_water);
                            return;
                        }
                        if !(kind == HQ
                            && (self.shared.tile_type)(t) == MP_OBJECT
                            && crate::tile_map::is_tile_owner(self.shared, t, self.current())
                            && self.object_type_is(t, HQ))
                        {
                            crate::landscape_cmd::clear_command(
                                self.commands(),
                                tmp.0,
                                t,
                                flags & !DoCommandFlag::EXECUTE,
                            );
                            out.add(tmp);
                        }
                    }
                }
                let slope_z = (self.shared.tile_slope_z)(tile);
                let mut z = slope_z.z;
                if slope_z.slope.0 != 0 {
                    z += 1;
                }
                for t in self.tiles(area) {
                    let mut result = SlopeResult {
                        callback: CALLBACK_FAILED,
                        registers: [0; 16],
                        version: 0,
                    };
                    if shape.callbacks & 1 != 0 {
                        let diff = t.wrapping_sub(tile);
                        result.callback = self.callback(
                            crate::newgrf_callbacks::CBID_OBJECT_LAND_SLOPE_CHECK.0,
                            u32::from((self.shared.tile_slope)(t)),
                            self.map.tile_y(diff) << 4 | self.map.tile_x(diff),
                            spec,
                            ptr::null_mut(),
                            t,
                            result.registers.as_mut_ptr(),
                            result.registers.len(),
                            view,
                        );
                        if result.callback != CALLBACK_FAILED {
                            let r = resolver_services(self.s);
                            result.version = (r.version)((r.file)(spec));
                        }
                        crate::witness::hit(crate::witness::OBJECT_SLOPE_CALLBACK);
                    }
                    if result.callback == CALLBACK_FAILED {
                        crate::station_cmd::openttd_rust_station_check_buildable(
                            (self.shared.station_commands)(),
                            tmp.0,
                            t,
                            0,
                            &raw mut z,
                            false,
                            false,
                        );
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
                if flags & DoCommandFlag::EXECUTE != 0 {
                    for t in self.tiles(area) {
                        if (self.shared.has_water_ground)(t) {
                            if !(self.shared.is_water_tile)(t) {
                                crate::landscape_cmd::clear_command(
                                    self.commands(),
                                    tmp.0,
                                    t,
                                    flags & !DoCommandFlag::NO_WATER,
                                );
                            }
                        } else {
                            crate::landscape_cmd::clear_command(self.commands(), tmp.0, t, flags);
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
                        || (self.shared.tile_max_z)(t) + i32::from(admission.height) >= {
                            let station = &*(self.shared.station_commands)();
                            crate::bridge_map::get_bridge_height(
                                (station.southern_bridge_end)(t),
                                self.shared,
                            )
                        })
                {
                    out.error(self.s.strings.bridge);
                    return;
                }
            }
            let mut score = 0;
            let mut size = 1_u32;
            match kind {
                TRANSMITTER | LIGHTHOUSE => {
                    if !((*self.s.founding).flat)(crate::town::founding::Tile(tile)) {
                        out.error(self.s.strings.flat);
                        return;
                    }
                }
                OWNED => {
                    if (self.shared.tile_type)(tile) == MP_OBJECT
                        && crate::tile_map::is_tile_owner(self.shared, tile, self.current())
                        && self.object_type_is(tile, OWNED)
                    {
                        out.error(self.s.strings.own);
                        return;
                    }
                }
                HQ => {
                    let company = self.current();
                    let old = (self.company().hq_location)(company);
                    if old != INVALID_TILE {
                        if old == tile {
                            out.error(self.s.strings.built);
                            return;
                        }
                        (self.company().finance.set_current_company)(OWNER_WATER);
                        self.clear(tmp, aux, old, flags);
                        (self.company().finance.set_current_company)(company);
                        out.add(tmp);
                        crate::witness::hit(crate::witness::OBJECT_HQ_RELOCATE);
                    }
                    if flags & DoCommandFlag::EXECUTE != 0 {
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
                    out.error(INVALID_STRING_ID);
                    return;
                }
                _ => {
                    size = u32::from(area.w) * u32::from(area.h);
                }
            }
            let finance = self.finance(self.current());
            if !finance.is_null() && Self::object_limit(finance) < size {
                out.error(self.s.strings.limit);
                return;
            }
            if flags & DoCommandFlag::EXECUTE != 0 {
                self.build(
                    kind,
                    tile,
                    if self.current() == OWNER_DEITY {
                        OWNER_NONE
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
            out.add_money(self.get_build_cost(spec).saturating_mul(i64::from(size)));
        }
        crate::witness::hit(crate::witness::OBJECT_BUILD_COMMAND);
    }
}

impl World<'_> {
    fn do_build(
        &self,
        out: Cost<'_>,
        tmp: Cost<'_>,
        aux: Cost<'_>,
        flags: u16,
        tile: u32,
        kind: u16,
        view: u8,
    ) {
        crate::command::nested(self.commands(), self.shared, out, tile, flags, |flags| {
            self.build_command(out, tmp, aux, flags, tile, kind, view);
        });
    }
    fn build_area(
        &self,
        out: Cost<'_>,
        ret: Cost<'_>,
        tmp: Cost<'_>,
        aux: Cost<'_>,
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
                out.error(INVALID_STRING_ID);
                return;
            }
            let spec = (self.s.spec)(kind);
            let shape = (self.s.area_shape)(spec);
            if view >= shape.views || shape.size != OBJECT_SIZE_1X1 {
                out.error(INVALID_STRING_ID);
                return;
            }
            let mut money = crate::command::available_money(self.commands());
            out.set(0);
            last.error(INVALID_STRING_ID);
            let mut success = false;
            let finance = self.finance(self.current());
            let mut limit = Self::object_limit(finance) as i32;
            let mut iterator = Drag::new(self.map, map_y, tile, start, diagonal);
            while iterator.tile != INVALID_TILE {
                let t = iterator.tile;
                self.do_build(
                    ret,
                    tmp,
                    aux,
                    flags & !DoCommandFlag::EXECUTE,
                    t,
                    kind,
                    view,
                );
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
                if flags & DoCommandFlag::EXECUTE != 0 {
                    money = money.saturating_sub(cost);
                    if cost > 0 && money < 0 {
                        break;
                    }
                    self.do_build(ret, tmp, aux, flags, t, kind, view);
                }
                out.add_money(cost);
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
            let kind = (self.s.object_type)(tile);
            if kind == OWNED {
                (self.commands().tile_ownership)(out.0, tile);
                if out.ok() {
                    out.zero();
                    return;
                }
            } else if (self.shared.autoslope)() && kind != TRANSMITTER && kind != LIGHTHOUSE {
                let old_slope = (self.shared.tile_slope)(tile);
                if !is_steep_slope(old_slope)
                    && !is_steep_slope(slope)
                    && (self.shared.tile_max_z)(tile) == z + get_slope_max_z(slope)
                {
                    let spec = (self.s.spec)(kind);
                    let callbacks = (self.s.callbacks)(spec);
                    if if callbacks & 32 != 0 {
                        {
                            let result = self.callback(
                                CBID_OBJECT_AUTOSLOPE,
                                0,
                                0,
                                spec,
                                (self.query().by_tile)(tile),
                                tile,
                                ptr::null_mut(),
                                0,
                                0,
                            );
                            result == CALLBACK_FAILED
                                || !(self.s.boolean_callback)(spec, CBID_OBJECT_AUTOSLOPE, result)
                        }
                    } else {
                        enabled(self.s, spec)
                    } {
                        out.set(crate::company::price(PR_BUILD_FOUNDATION));
                        crate::witness::hit(crate::witness::OBJECT_AUTOSLOPE);
                        return;
                    }
                }
            }
            crate::landscape_cmd::clear_command(self.commands(), out.0, tile, flags);
        }
    }
    fn build_above(&self, out: Cost<'_>, tile: u32, flags: u16, height: i32) {
        crate::witness::hit(crate::witness::OBJECT_BUILD_ABOVE);
        unsafe {
            let clearance = (self.s.clearance)(tile);
            if clearance.flags & UNDER_BRIDGE != 0
                && (self.shared.tile_max_z)(tile) + i32::from(clearance.height) <= height
            {
                out.zero();
                return;
            }
            crate::landscape_cmd::clear_command(self.commands(), out.0, tile, flags);
        }
    }
    fn change_owner(&self, ownership: &crate::tile_owner::Context, tile: u32, old: u8, new: u8) {
        if !crate::tile_map::is_tile_owner(self.shared, tile, old) {
            return;
        }
        unsafe {
            let kind = (self.s.object_type)(tile);
            let mut clear = false;
            if (kind == OWNED || kind >= NEW_OBJECT_OFFSET) && new != INVALID_OWNER {
                (self.s.set_owner)(tile, new);
                if (self.s.water_class)(tile).0 == 1 {
                    ((*self.s.restoration).infrastructure)(old, u32::MAX);
                    ((*self.s.restoration).infrastructure)(new, 1);
                }
            } else if kind == STATUE {
                let town = (self.s.statue_town)((self.query().by_tile)(tile));
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
                self.really_clear((self.s.identity)(tile));
                // When clearing objects, they may turn into canal, which may require transferring ownership.
                crate::tile_owner::openttd_rust_tile_change_owner(ownership, tile, old, new);
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
                u32::from(self.get_company_hq_size(tile)) + 1
            };
            let pass = crate::cargo_registry::openttd_rust_cargo_by_label(
                crate::cargo_type::CT_PASSENGERS,
            );
            if crate::cargo_type::is_valid_cargo_type(pass) {
                let entry = cargo.add(usize::from(pass));
                *entry = (*entry).wrapping_add(if accepted.is_null() { 1 } else { level.max(1) });
                if !accepted.is_null() {
                    *accepted |= 1_u64 << pass;
                }
            }
            let mail =
                crate::cargo_registry::openttd_rust_cargo_by_label(crate::cargo_type::CT_MAIL);
            if crate::cargo_type::is_valid_cargo_type(mail) {
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
    fn tile_loop(&self, tile: u32, call: *mut crate::ground::Call) {
        unsafe {
            let policy = (self.s.loop_policy)(tile);
            if policy.flags & ANIMATION != 0 {
                self.trigger_tile(
                    policy.object,
                    tile,
                    AnimationTrigger::TILE_LOOP,
                    policy.spec,
                );
                if policy.north == tile {
                    self.trigger(
                        policy.object,
                        AnimationTrigger::TILE_LOOP_NORTH,
                        policy.spec,
                    );
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
            let level = u32::from(self.get_company_hq_size(tile)) + 1;
            crate::movement::native_assert(self.shared, level < 6);
            let mut stations =
                crate::station::StationFinder::new(crate::tilearea::TileArea { tile, w: 2, h: 2 });
            let catchment = std::cell::OnceCell::new();
            let mut deliver = |cargo: u8, amount: u32| {
                let c = catchment.get_or_init(|| crate::station::Ctx::from_shared(self.shared));
                let list = std::ptr::from_ref(stations.get_stations(c));
                let source = crate::cargo_storage::Source {
                    id: u16::from(self.owner(tile)),
                    source_type: crate::source_type::Source::HEADQUARTERS,
                };
                // The call-local finder's set outlives the delivery and no
                // service can reach it.
                crate::station::move_goods_to_station(
                    cargo,
                    amount,
                    source,
                    list,
                    crate::company_type::INVALID_OWNER,
                    c,
                );
            };
            let random = self.shared.random();
            let pass = crate::cargo_registry::openttd_rust_cargo_by_label(
                crate::cargo_type::CT_PASSENGERS,
            );
            if crate::cargo_type::is_valid_cargo_type(pass)
                && random & 255 < 256 / 4 / (6_u32.wrapping_sub(level))
            {
                let mut amount = (random & 255) / 8 / 4 + 1;
                if (self.shared.recession)() {
                    amount = (amount + 1) >> 1;
                }
                amount = (self.s.scale_cargo)(amount);
                deliver(pass, amount);
                crate::witness::hit(crate::witness::OBJECT_HQ_PASS);
            }
            let mail =
                crate::cargo_registry::openttd_rust_cargo_by_label(crate::cargo_type::CT_MAIL);
            if crate::cargo_type::is_valid_cargo_type(mail)
                && (random >> 8) & 255 < 196 / 4 / (6_u32.wrapping_sub(level))
            {
                let mut amount = ((random >> 8) & 255) / 8 / 4 + 1;
                if (self.shared.recession)() {
                    amount = (amount + 1) >> 1;
                }
                amount = (self.s.scale_cargo)(amount);
                deliver(mail, amount);
                crate::witness::hit(crate::witness::OBJECT_HQ_MAIL);
            }
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
            if (self.shared.tile_type)(tile) != MP_WATER || (self.s.water_class)(tile).0 != 0 {
                return false;
            }
            for _ in 0..19 {
                if (self.shared.tile_type)(tile) == MP_CLEAR
                    && ((*self.s.founding).flat)(crate::town::founding::Tile(tile))
                    && (self.shared.tile_z)(tile) <= 2
                    && !(self.shared.bridge_above)(tile)
                {
                    if self.near(tile, LIGHTHOUSE) {
                        return false;
                    }
                    self.build(LIGHTHOUSE, tile, OWNER_NONE, ptr::null(), 0);
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
    fn random_tile(&self) -> u32 {
        self.shared.random() & (self.map.size_x() * ((self.shared.map_max_y)() + 1) - 1)
    }
    fn transmitter(&self) -> bool {
        let tile = self.random_tile();
        if (self.shared.tile_type)(tile) == MP_CLEAR
            && unsafe { ((*self.s.founding).flat)(crate::town::founding::Tile(tile)) }
            && (self.shared.tile_z)(tile) >= 4
            && !(self.shared.bridge_above)(tile)
        {
            if self.near(tile, TRANSMITTER) {
                return false;
            }
            self.build(TRANSMITTER, tile, OWNER_NONE, ptr::null(), 0);
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
            if ((*self.s.town).environment.freeform_edges)() {
                for x in 0..maxx {
                    if (self.shared.tile_type)(x + self.map.size_x()) == MP_WATER {
                        water_tiles += 1;
                    }
                    if (self.shared.tile_type)(x + (maxy - 1) * self.map.size_x()) == MP_WATER {
                        water_tiles += 1;
                    }
                }
                for y in 1..maxy - 1 {
                    if (self.shared.tile_type)(1 + y * self.map.size_x()) == MP_WATER {
                        water_tiles += 1;
                    }
                    if (self.shared.tile_type)(maxx - 1 + y * self.map.size_x()) == MP_WATER {
                        water_tiles += 1;
                    }
                }
            }
            for kind in 0..(self.s.spec_count)() {
                let spec = (self.s.spec)(kind as u16);
                let generation = (self.s.generation)(spec);
                if !was_available(self.s, spec) || generation.amount == 0 {
                    continue;
                }
                let mut amount = if generation.flags & SCALE_WATER != 0
                    && ((*self.s.town).environment.freeform_edges)()
                {
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
                            let tile = self.random_tile();
                            self.build_command(
                                out,
                                tmp,
                                aux,
                                DoCommandFlag::EXECUTE
                                    | DoCommandFlag::AUTO
                                    | DoCommandFlag::NO_TEST_TOWN_RATING
                                    | DoCommandFlag::NO_MODIFY_TOWN_RATING,
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

/// Stack context owned by the object invocation. Successful parent lookup is
/// cached for this lifetime; a null result leaves the next lookup free to retry.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ObjectScope {
    pub object: *mut Object,
    pub spec: *const Spec,
    pub parent: *const TownHandle,
    pub tile: u32,
    pub view: u8,
}
#[repr(C)]
pub struct Eligibility {
    pub flags: u16,
    pub climate: u8,
    pub views: u8,
}
#[repr(C)]
pub struct Lifetime {
    pub introduction: i32,
    pub end: i32,
}
#[repr(C)]
pub struct ResolverServices {
    pub file: unsafe extern "C" fn(*const Spec) -> *const std::ffi::c_void,
    pub root: unsafe extern "C" fn(*const Spec, bool) -> *const std::ffi::c_void,
    pub local_id: unsafe extern "C" fn(*const Spec) -> u16,
    pub version: unsafe extern "C" fn(*const std::ffi::c_void) -> u8,
    pub views: unsafe extern "C" fn(*const Spec) -> u8,
    pub eligibility: unsafe extern "C" fn(*const Spec) -> Eligibility,
    pub introduction: unsafe extern "C" fn(*const Spec) -> i32,
    pub lifetime: unsafe extern "C" fn(*const Spec) -> Lifetime,
    pub town_scope:
        unsafe extern "C" fn(*mut Resolver, *const TownHandle, bool) -> *mut std::ffi::c_void,
    pub base_scope: unsafe extern "C" fn(*mut Resolver, u8, u8) -> *mut std::ffi::c_void,
    pub evaluate: unsafe extern "C" fn(*const ObjectScope, u16, u32, u32, *mut i32, usize) -> u16,
}
static INVOCATIONS: GameState<*const ObjectServices> = GameState::new(ptr::null());
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_services_exchange(
    s: *const ObjectServices,
) -> *const ObjectServices {
    INVOCATIONS.exchange(s)
}
fn invocation_services<'a>() -> &'a ObjectServices {
    // SAFETY: ResetObjects installs the immutable process-live native table
    // before use. No Rust reference into a context or world record escapes.
    unsafe { INVOCATIONS.table() }
}
fn resolver_services(s: &ObjectServices) -> &ResolverServices {
    // SAFETY: native table lives for the serialized entry.
    unsafe { &*s.resolver }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_scope_init(
    object: *mut Object,
    spec: *const Spec,
    tile: u32,
    view: u8,
) -> ObjectScope {
    ObjectScope {
        object,
        spec,
        parent: ptr::null(),
        tile,
        view,
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_file(spec: *const Spec) -> *const std::ffi::c_void {
    unsafe { (resolver_services(invocation_services()).file)(spec) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_root(
    context: *const ObjectScope,
) -> *const std::ffi::c_void {
    let ObjectScope { object, spec, .. } = unsafe { *context };
    unsafe { (resolver_services(invocation_services()).root)(spec, !object.is_null()) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_debug_id(context: *const ObjectScope) -> u32 {
    let spec = unsafe { (*context).spec };
    unsafe { u32::from((resolver_services(invocation_services()).local_id)(spec)) }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_feature() -> u8 {
    0x0f
}
unsafe fn object_town(context: *mut ObjectScope, ro: *mut Resolver) -> *mut std::ffi::c_void {
    let s = invocation_services();
    let ObjectScope {
        object,
        parent,
        tile,
        ..
    } = unsafe { *context };
    let town = if parent.is_null() {
        if object.is_null() {
            // ClosestTownFromTile has no invalid-tile guard in this source body.
            (unsafe { &*s.shared }.closest_town)(tile, u32::MAX)
        } else {
            unsafe { ((*s.query).town)(object) }
        }
    } else {
        parent
    };
    if town.is_null() {
        return ptr::null_mut();
    }
    // End the context access before constructing the native virtual facade,
    // which uses the integrated town/storage owner and can reenter it.
    unsafe {
        (*context).parent = town;
    }
    unsafe { (resolver_services(s).town_scope)(ro, town, object.is_null()) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_scope(
    context: *mut ObjectScope,
    ro: *mut Resolver,
    own: *mut std::ffi::c_void,
    scope: u8,
    relative: u8,
) -> *mut std::ffi::c_void {
    use crate::newgrf_spritegroup::{VSG_SCOPE_PARENT, VSG_SCOPE_SELF};
    if scope == VSG_SCOPE_SELF {
        return own;
    }
    if scope == VSG_SCOPE_PARENT {
        let town = unsafe { object_town(context, ro) };
        if !town.is_null() {
            return town;
        }
    }
    unsafe { (resolver_services(invocation_services()).base_scope)(ro, scope, relative) }
}
/// Direct object owner entry. Native evaluation constructs only its stack
/// virtual facade for the directly called common Action2 owner.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_callback(
    s: *const ObjectServices,
    callback: u16,
    param1: u32,
    param2: u32,
    spec: *const Spec,
    object: *mut Object,
    tile: u32,
    registers: *mut i32,
    count: usize,
    view: u8,
) -> u16 {
    let context = openttd_rust_object_scope_init(object, spec, tile, view);
    unsafe {
        (resolver_services(&*s).evaluate)(
            &raw const context,
            callback,
            param1,
            param2,
            registers,
            count,
        )
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_animation_callback(
    callback: u16,
    param1: u32,
    param2: u32,
    spec: *const Spec,
    object: *mut Object,
    tile: u32,
    _extra: i32,
) -> u16 {
    unsafe {
        openttd_rust_object_callback(
            invocation_services(),
            callback,
            param1,
            param2,
            spec,
            object,
            tile,
            ptr::null_mut(),
            0,
            0,
        )
    }
}
fn is_enabled(views: u8) -> bool {
    views > 0
}
fn enabled(s: &ObjectServices, spec: *const Spec) -> bool {
    is_enabled(unsafe { (resolver_services(s).views)(spec) })
}
fn ever_available(s: &ObjectServices, spec: *const Spec) -> bool {
    let policy = unsafe { (resolver_services(s).eligibility)(spec) };
    is_enabled(policy.views)
        && policy.climate & (1 << (unsafe { &*s.town }.climate)()) != 0
        && policy.flags
            & if !(unsafe { &*s.town }.environment.editor)()
                && !(unsafe { &*s.town }.generating_world)()
            {
                SCENEDIT
            } else {
                IN_GAME
            }
            == 0
}
fn was_available(s: &ObjectServices, spec: *const Spec) -> bool {
    ever_available(s, spec)
        && (unsafe { &*s.shared }.calendar_date)()
            > unsafe { (resolver_services(s).introduction)(spec) }
}
fn available(s: &ObjectServices, spec: *const Spec) -> bool {
    if !was_available(s, spec) {
        return false;
    }
    let dates = unsafe { (resolver_services(s).lifetime)(spec) };
    (unsafe { &*s.shared }.calendar_date)() < dates.end
        || dates.end < dates.introduction.wrapping_add(365)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_enabled(
    s: *const ObjectServices,
    spec: *const Spec,
) -> bool {
    enabled(unsafe { &*s }, spec)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_ever_available(
    s: *const ObjectServices,
    spec: *const Spec,
) -> bool {
    ever_available(unsafe { &*s }, spec)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_was_available(
    s: *const ObjectServices,
    spec: *const Spec,
) -> bool {
    was_available(unsafe { &*s }, spec)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_available(
    s: *const ObjectServices,
    spec: *const Spec,
) -> bool {
    available(unsafe { &*s }, spec)
}

/// Synchronous `NewGRF` query/animation entries. Native records stay opaque.
///
/// # Safety
/// The caller holds the game-state mutex and supplies live tables, resolver,
/// object/spec pointers (object may be null for variable queries), and availability
/// storage. Calls are synchronous; native leaves are noexcept. Panics abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_variable(
    s: *const ObjectServices,
    ro: *const Resolver,
    obj: *mut Object,
    spec: *const Spec,
    tile: u32,
    view: u8,
    variable: u8,
    parameter: u32,
    available: *mut bool,
) -> u32 {
    unsafe {
        World::new::<true>(&*s).variable(
            ro,
            obj,
            spec,
            tile,
            view,
            variable,
            parameter,
            &mut *available,
        )
    }
}
/// # Safety
/// Same serialized live-table contract as `openttd_rust_object_variable`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_random_bits(
    s: *const ObjectServices,
    tile: u32,
) -> u32 {
    unsafe { World::new::<false>(&*s).random_bits(tile) }
}
/// # Safety
/// Same serialized live-table/object/spec contract as the variable entry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_trigger_tile(
    s: *const ObjectServices,
    obj: *mut Object,
    tile: u32,
    trigger: AnimationTrigger,
    spec: *const Spec,
) -> bool {
    unsafe { World::new::<false>(&*s).trigger_tile(obj, tile, trigger, spec) }
}
/// # Safety
/// Same serialized live-table/object/spec contract as the variable entry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_trigger(
    s: *const ObjectServices,
    obj: *mut Object,
    trigger: AnimationTrigger,
    spec: *const Spec,
) -> bool {
    unsafe { World::new::<false>(&*s).trigger(obj, trigger, spec) }
}

/// Synchronous object owner entry.
/// # Safety
/// The module's export contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_build(
    s: *const ObjectServices,
    kind: u16,
    tile: u32,
    owner: u8,
    town: *const TownHandle,
    view: u8,
) {
    unsafe { build(&*s, kind, tile, owner, town, view) };
}

/// Original `BuildObject`, shared by Rust component owners.
/// # Safety
/// Live immutable services and native town shell, under the game-state mutex.
/// No caller borrow may span the object's reentrant callbacks.
pub(crate) unsafe fn build(
    s: &ObjectServices,
    kind: u16,
    tile: u32,
    owner: u8,
    town: *const TownHandle,
    view: u8,
) {
    // SAFETY: Caller supplies the canonical live service tables.
    let w = unsafe { World::new::<true>(s) };
    w.build(kind, tile, owner, town, view);
}

/// Synchronous object owner entry.
/// # Safety
/// The module's export contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_update_hq(
    s: *const ObjectServices,
    tile: u32,
    score: u32,
) {
    let w = unsafe { World::new::<true>(&*s) };
    w.update_hq(tile, score);
}

/// Synchronous object owner entry.
/// # Safety
/// The module's export contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_colours(s: *const ObjectServices, company: u8) {
    let w = unsafe { World::new::<false>(&*s) };
    w.colours(company);
}

/// Synchronous object owner entry.
/// # Safety
/// The module's export contract.
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
    let w = unsafe { World::new::<true>(&*s) };
    let costs = unsafe { &*(*s).cost };
    let out = Cost(out, costs);
    let tmp = Cost(tmp, costs);
    let aux = Cost(aux, costs);
    w.build_command(out, tmp, aux, flags, tile, kind, view);
}

/// Synchronous object owner entry.
/// # Safety
/// The module's export contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_build_area(
    s: *const ObjectServices,
    out: *mut CommandCost,
    ret: *mut CommandCost,
    tmp: *mut CommandCost,
    aux: *mut CommandCost,
    last: *mut CommandCost,
    flags: u16,
    tile: u32,
    start: u32,
    kind: u16,
    view: u8,
    diagonal: bool,
) {
    let w = unsafe { World::new::<true>(&*s) };
    let costs = unsafe { &*(*s).cost };
    let out = Cost(out, costs);
    let ret = Cost(ret, costs);
    let tmp = Cost(tmp, costs);
    let aux = Cost(aux, costs);
    let last = Cost(last, costs);
    w.build_area(
        out, ret, tmp, aux, last, flags, tile, start, kind, view, diagonal,
    );
}

/// Synchronous object owner entry.
/// # Safety
/// The module's export contract.
pub(crate) unsafe fn openttd_rust_object_clear(
    s: *const ObjectServices,
    out: *mut CommandCost,
    tmp: *mut CommandCost,
    tile: u32,
    flags: u16,
) {
    let w = unsafe { World::new::<true>(&*s) };
    let costs = unsafe { &*(*s).cost };
    let out = Cost(out, costs);
    let tmp = Cost(tmp, costs);
    w.clear(out, tmp, tile, flags);
}

/// Synchronous object owner entry.
/// # Safety
/// The module's export contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_cargo(
    s: *const ObjectServices,
    tile: u32,
    cargo: *mut u32,
    accepted: *mut u64,
) {
    let w = unsafe { World::new::<false>(&*s) };
    w.cargo(tile, cargo, accepted);
}

/// Synchronous object owner entry.
/// # Safety
/// The module's export contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_tile_loop(
    s: *const ObjectServices,
    call: *mut crate::ground::Call,
    tile: u32,
) {
    let w = unsafe { World::new::<false>(&*s) };
    w.tile_loop(tile, call);
}

/// Synchronous object owner entry.
/// # Safety
/// The module's export contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_generate(
    s: *const ObjectServices,
    out: *mut CommandCost,
    tmp: *mut CommandCost,
    aux: *mut CommandCost,
    frame: *mut GenerationFrame,
) -> bool {
    let w = unsafe { World::new::<true>(&*s) };
    let costs = unsafe { &*(*s).cost };
    let out = Cost(out, costs);
    let tmp = Cost(tmp, costs);
    let aux = Cost(aux, costs);
    w.generate(frame, out, tmp, aux)
}

/// `ChangeTileOwner_Object`, entered only from the shared ownership dispatcher.
/// # Safety
/// Immutable tables and the dispatcher's live native scratch outlive the
/// synchronous call; original tile/ID preconditions. No native pointer is
/// retained. Services may reenter after all state accesses end. Panics abort.
pub(crate) unsafe fn openttd_rust_object_change_owner(
    s: *const ObjectServices,
    ownership: &crate::tile_owner::Context,
    tile: u32,
    old: u8,
    new_owner: u8,
) {
    let w = unsafe { World::new::<true>(&*s) };
    w.change_owner(ownership, tile, old, new_owner);
}

/// Synchronous object owner entry.
/// # Safety
/// The module's export contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_terraform(
    s: *const ObjectServices,
    out: *mut CommandCost,
    tile: u32,
    flags: u16,
    z: i32,
    slope: u8,
) {
    let w = unsafe { World::new::<false>(&*s) };
    let costs = unsafe { &*(*s).cost };
    let out = Cost(out, costs);
    w.terraform(out, tile, flags, z, slope);
}

/// Synchronous object owner entry.
/// # Safety
/// The module's export contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_build_above(
    s: *const ObjectServices,
    out: *mut CommandCost,
    tile: u32,
    flags: u16,
    height: i32,
) {
    let w = unsafe { World::new::<false>(&*s) };
    let costs = unsafe { &*(*s).cost };
    let out = Cost(out, costs);
    w.build_above(out, tile, flags, height);
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDObjectHiddenArea" ["landscape"] => HiddenArea {
        index,
        first_tile,
    };
    "OpenTTDObjectPrice" ["landscape"] => Price {
        factor,
        shift,
    };
    "OpenTTDObjectArea" ["landscape"] => TileArea {
        tile,
        w => width,
        h => height,
    };
    "OpenTTDObjectIdentity" ["landscape"] => Identity {
        object,
        area,
        kind,
    };
    "OpenTTDObjectAllocation" ["landscape"] => Allocation {
        object,
        id,
    };
    "OpenTTDObjectNext" ["landscape"] => Next {
        object,
        id,
        tile,
    };
    "OpenTTDObjectLoopPolicy" ["landscape"] => LoopPolicy {
        spec,
        object,
        north,
        flags,
        kind,
    };
    "OpenTTDObjectPolicy" ["landscape"] => Policy {
        flags,
        callbacks,
    };
    "OpenTTDObjectFootprint" ["landscape"] => Footprint {
        flags,
        callbacks,
        size,
    };
    "OpenTTDObjectClearance" ["landscape"] => Clearance {
        flags,
        height,
    };
    "OpenTTDObjectAdmission" ["landscape"] => Admission {
        views,
        height,
    };
    "OpenTTDObjectAreaShape" ["landscape"] => AreaShape {
        views,
        size,
    };
    "OpenTTDObjectGeneration" ["landscape"] => Generation {
        flags,
        amount,
        views,
    };
    "OpenTTDObjectSlopeResult" ["landscape"] => SlopeResult {
        callback,
        registers,
        version,
    };
    "OpenTTDObjectStrings" ["landscape"] => Strings {
        too_many,
        no_town,
        edge_sub,
        water,
        edge,
        on_water,
        bridge,
        flat,
        own,
        built,
        limit,
        hq,
        object,
    };
    "OpenTTDObjectDefinition" ["landscape"] => Definition {
        grfid,
        local_id,
        view,
        has_file,
    };
    "OpenTTDObjectQueryServices" ["landscape"] => QueryServices {
        random_bits,
        index,
        by_tile,
        definition,
        object_index,
        kind,
        north,
        town,
        town_tile,
        date,
        colour,
        view,
        nearby,
        nearby_info,
        terrain,
        town_zone,
        badges,
        grfid,
        version8,
        override_id,
        manhattan,
        square,
        triggers,

        unhandled,
    };
    "OpenTTDObjectScope" ["landscape"] => ObjectScope { object, spec, parent, tile, view };
    "OpenTTDObjectEligibility" ["landscape"] => Eligibility { flags, climate, views };
    "OpenTTDObjectLifetime" ["landscape"] => Lifetime { introduction, end };
    "OpenTTDObjectResolverServices" ["landscape"] => ResolverServices { file, root, local_id, version, views, eligibility, introduction, lifetime, town_scope, base_scope, evaluate };
    "OpenTTDObjectServices" ["landscape"] => ObjectServices {
        shared,
        cost,
        command,
        company,
        cargo,
        town,
        founding,
        industry_tiles,
        strings,
        spec_count,
        spec,
        callbacks,
        colour_policy,
        flags,
        footprint,
        clearance,
        area_shape,
        admission,
        generation,
        can_allocate,
        allocate,
        object_type,
        loop_policy,
        identity,
        location,
        next,
        set_colour,
        recolour,
        colour_error,
        make,
        delete_object,
        query,
        resolver,
        slope_error,
        build_price,
        clear_price,
        set_hq,
        statue_town,
        boolean_callback,
        close_inspect,
        progress_set,
        progress_increase,
        normal,
        bulldozer,
        water_class,
        restoration,
        set_owner,
        scale_size_1d,
        scale_cargo,
    };
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
